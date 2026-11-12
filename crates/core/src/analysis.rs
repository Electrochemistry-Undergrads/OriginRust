//! Analysis tools applied to worksheet columns: integrate, differentiate,
//! smooth, normalize, subtract a baseline, interpolate, find peaks and
//! descriptive statistics, plus sorting and filtering of worksheet rows.
//!
//! The numerics are in the `sheaf-calc` crate (checked against Origin, see
//! `docs/spec/analysis-tools.md`). This module connects them to worksheets:
//! it finds the X column of each chosen Y column, restricts the rows, and
//! puts the results where the user can see and plot them.
//!
//! Where results go:
//!
//! - Tools that give one value per input row (derivative, smoothed curve,
//!   normalized curve, baseline-subtracted curve, running integral) add one
//!   new Y column directly after its source column, so the new column is
//!   plotted against the same X column. Rows outside the chosen row range
//!   are missing values.
//! - Tools that give a few numbers per input column (integration results,
//!   statistics) add a new sheet to the workbook with one row per input
//!   column, and return the same table in the [`Report`].
//! - The statistics tools (t-tests, ANOVA, nonparametric tests, correlation, normality tests, histogram) put their tables on
//!   a new sheet of their own; see the `statistics` submodule and `docs/spec/statistics.md`.
//! - Tools whose result has its own length (interpolated curve, list of
//!   peaks) add a new sheet with an X and a Y column per input column.

use crate::column::{Column, ColumnData, Designation};
use crate::ids::{ColumnId, WindowId};
use crate::project::Project;
use crate::worksheet::Worksheet;
use sheaf_calc as calc;
use sheaf_expr::Expr;

pub use sheaf_calc::{
    AreaType, Boundary, CurveOp, DiffOptions, InterpMethod, NormalizeMethod, PeakDirection, PeakFilter, PeakOptions,
    SmoothMethod, SmoothOptions, SplineBoundary,
};
pub use sheaf_calc::{BinSize, FreqOptions, MeansComparison, Tail};
pub use sheaf_calc::{ChainBaseline, ChainOptions, FindMethod, FindOptions, LimitMode};
pub use sheaf_fit::multipeak::{FitBaseline, PeakShape};

/// The multi-peak fit of [`PeakChainSpec`]: a sum of `shape` peaks on `baseline`, one peak for
/// every peak the search found, started at the found centre, width and height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakFitSpec {
    pub shape: PeakShape,
    pub baseline: FitBaseline,
}

/// What [`Tool::PeakChain`] does: baseline, peak search, per-peak integration and (optionally)
/// a multi-peak fit, in one run with one result sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct PeakChainSpec {
    pub chain: ChainOptions,
    /// Add the integration columns (area, area percent, FWHM, limits).
    pub integrate: bool,
    /// Fit all found peaks together and add the fit columns.
    pub fit: Option<PeakFitSpec>,
}

/// Which rows of the chosen columns a tool works on.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RowRange {
    #[default]
    All,
    /// Rows `from..=to`, counted from 1 as the worksheet shows them.
    Rows(usize, usize),
    /// The first unbroken run of rows whose X value lies between the two
    /// limits (in either order). For data whose X rises or falls steadily
    /// this is simply "X from .. to"; for a cyclic sweep that passes the
    /// interval twice it is the first pass.
    XBetween(f64, f64),
}

/// What [`Tool::Integrate`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntegrateSpec {
    pub area_type: AreaType,
    /// Subtract the straight line through the first and the last point of
    /// the row range before integrating (the area of a peak above a sloping
    /// background).
    pub end_point_baseline: bool,
    /// Also add a column with the running integral.
    pub cumulative: bool,
}

impl Default for IntegrateSpec {
    fn default() -> Self {
        IntegrateSpec { area_type: AreaType::Mathematical, end_point_baseline: false, cumulative: true }
    }
}

/// The baseline that [`Tool::SubtractBaseline`] removes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaselineSpec {
    /// A constant level.
    Constant(f64),
    /// The smallest value of the row range.
    Minimum,
    /// The straight line through the first and the last point of the row range.
    EndPoints,
    /// The straight line through two given points `(x, y)`.
    Line((f64, f64), (f64, f64)),
}

/// What [`Tool::Interpolate`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterpSpec {
    pub method: InterpMethod,
    pub boundary: SplineBoundary,
    /// Number of equally spaced points of the result.
    pub npts: usize,
    /// X range of the result; `None` is the X range of the data.
    pub range: Option<(f64, f64)>,
    /// Smoothing factor of the B-spline method: the allowed sum of squared residuals (Origin's
    /// `sf`). 0 gives the interpolating spline, which differs from Origin's automatic factor.
    pub smoothing: f64,
}

impl Default for InterpSpec {
    fn default() -> Self {
        InterpSpec {
            method: InterpMethod::Linear,
            boundary: SplineBoundary::NotAKnot,
            npts: 100,
            range: None,
            smoothing: 0.0,
        }
    }
}

/// What [`Tool::IntegrateRanges`] does: the integral of each of several ranges of every chosen
/// column, in one results table.
#[derive(Debug, Clone, PartialEq)]
pub struct IntegrateRangesSpec {
    /// The ranges to integrate, one table row each per input column. [`RowRange::XBetween`] takes
    /// the first unbroken run of rows inside the X interval, [`RowRange::Rows`] counts worksheet
    /// rows. The request's own row range, if any, is applied first: X intervals are searched
    /// inside it and row ranges are cut to it.
    pub ranges: Vec<RowRange>,
    /// Subtract the straight line through the first and last point of each range first.
    pub end_point_baseline: bool,
}

/// The second operand of [`Tool::CurveMath`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveOperand {
    Constant(f64),
    /// A reference curve: this Y column, with the X column Sheaf finds for it (the nearest X
    /// column to its left, row numbers if there is none). It may be in any sheet of the project.
    Reference(ColumnId),
}

/// What [`Tool::CurveMath`] does: `y op operand` for every chosen Y column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveMathSpec {
    pub op: CurveOp,
    pub operand: CurveOperand,
    /// Reference only: restrict the result to the X range both curves cover (the data and the
    /// reference are then evaluated on the merged X values of both and the result is a new sheet
    /// with its own X column). Off: one value per data row, new column beside the data.
    pub common_only: bool,
}

/// Which t-test [`TTestSpec`] runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TTestKind {
    /// Each chosen column is tested on its own against a hypothesized mean.
    OneSample { mean: f64 },
    /// Exactly two chosen columns; the difference of their means (first minus second) against
    /// `difference`. Without `equal_variance` the Welch test is used.
    TwoSample { equal_variance: bool, difference: f64 },
    /// Exactly two chosen columns; the rows where both have a number are paired.
    Paired { difference: f64 },
}

/// What [`Tool::TTest`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TTestSpec {
    pub kind: TTestKind,
    pub tail: Tail,
    /// Significance level, 0 to 1 (Origin's default is 0.05).
    pub alpha: f64,
}

impl Default for TTestSpec {
    fn default() -> Self {
        TTestSpec { kind: TTestKind::OneSample { mean: 0.0 }, tail: Tail::Two, alpha: 0.05 }
    }
}

/// What [`Tool::Anova`] does: one-way ANOVA with each chosen column as a group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnovaSpec {
    pub alpha: f64,
    /// Means comparison of all pairs of groups; `None` leaves it out.
    pub comparison: Option<MeansComparison>,
}

impl Default for AnovaSpec {
    fn default() -> Self {
        AnovaSpec { alpha: 0.05, comparison: Some(MeansComparison::Tukey) }
    }
}

/// What [`Tool::Histogram`] does: frequency counts of each chosen column.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HistogramSpec {
    pub freq: FreqOptions,
    /// Also make a graph with a column plot of the counts.
    pub graph: bool,
}

/// What [`Tool::Normality`] does: all the normality tests on each chosen column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalitySpec {
    pub alpha: f64,
}

impl Default for NormalitySpec {
    fn default() -> Self {
        NormalitySpec { alpha: 0.05 }
    }
}

/// How [`TwoWayAnovaSpec`] finds the two factors in the chosen columns.
#[derive(Debug, Clone, PartialEq)]
pub enum TwoWayLayout {
    /// Origin's "Indexed Data": three columns, the level of factor A, the level of factor B and the data of
    /// each row, in this order. The factor columns may hold text or numbers; a row with an empty factor
    /// cell or no number is left out.
    Indexed,
    /// Origin's "Raw Data": every chosen column holds the data of one cell of the design; `levels` gives
    /// the level of factor A and of factor B of each column, in the order of the columns.
    Raw { levels: Vec<(String, String)> },
}

/// What [`Tool::TwoWayAnova`] does.
#[derive(Debug, Clone, PartialEq)]
pub struct TwoWayAnovaSpec {
    pub layout: TwoWayLayout,
    /// Fit the interaction of the two factors (Origin's "Interactions" check box).
    pub interaction: bool,
    /// Means comparison of the levels of each factor (and of the cells when there is an interaction).
    pub comparison: Option<MeansComparison>,
    pub alpha: f64,
}

impl Default for TwoWayAnovaSpec {
    fn default() -> Self {
        TwoWayAnovaSpec {
            layout: TwoWayLayout::Indexed,
            interaction: true,
            comparison: Some(MeansComparison::Tukey),
            alpha: 0.05,
        }
    }
}

/// What [`Tool::MannWhitney`] does: two chosen columns, two independent samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MannWhitneySpec {
    /// The alternative about the first column: `Upper` is "tends to be larger".
    pub tail: Tail,
    /// Exact p-value (permutation distribution of the rank sum) instead of the normal approximation.
    pub exact: bool,
    pub alpha: f64,
}

impl Default for MannWhitneySpec {
    fn default() -> Self {
        MannWhitneySpec { tail: Tail::Two, exact: false, alpha: 0.05 }
    }
}

/// Which signed rank test [`SignedRankSpec`] runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignedRankKind {
    /// Each chosen column against a hypothesized median.
    OneSample { median: f64 },
    /// Exactly two chosen columns; the rows where both have a number are paired and the first minus the
    /// second is tested.
    Paired,
}

/// What [`Tool::SignedRank`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignedRankSpec {
    pub kind: SignedRankKind,
    pub tail: Tail,
    /// Decide with the exact p-value where there is one (up to 80 non-zero differences, as Origin reports
    /// it); the asymptotic one is always in the table.
    pub exact: bool,
    pub alpha: f64,
}

impl Default for SignedRankSpec {
    fn default() -> Self {
        SignedRankSpec { kind: SignedRankKind::OneSample { median: 0.0 }, tail: Tail::Two, exact: true, alpha: 0.05 }
    }
}

/// What [`Tool::KruskalWallis`] does: every chosen column is a group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KruskalWallisSpec {
    pub alpha: f64,
}

impl Default for KruskalWallisSpec {
    fn default() -> Self {
        KruskalWallisSpec { alpha: 0.05 }
    }
}

/// What [`Tool::Correlation`] does: the coefficients of every pair of the chosen columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorrelationSpec {
    pub pearson: bool,
    pub spearman: bool,
    pub kendall: bool,
    /// Leave out every row where any column has no number (default: pair by pair).
    pub listwise: bool,
    /// Level for the sentence that names the significant pairs.
    pub alpha: f64,
}

impl Default for CorrelationSpec {
    fn default() -> Self {
        CorrelationSpec { pearson: true, spearman: false, kendall: false, listwise: false, alpha: 0.05 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tool {
    Integrate(IntegrateSpec),
    IntegrateRanges(IntegrateRangesSpec),
    CurveMath(CurveMathSpec),
    Differentiate(DiffOptions),
    Smooth(SmoothOptions),
    Normalize(NormalizeMethod),
    SubtractBaseline(BaselineSpec),
    Interpolate(InterpSpec),
    FindPeaks(PeakOptions),
    PeakChain(PeakChainSpec),
    Statistics,
    TTest(TTestSpec),
    Anova(AnovaSpec),
    Histogram(HistogramSpec),
    Normality(NormalitySpec),
    TwoWayAnova(TwoWayAnovaSpec),
    MannWhitney(MannWhitneySpec),
    SignedRank(SignedRankSpec),
    KruskalWallis(KruskalWallisSpec),
    Correlation(CorrelationSpec),
}

impl Tool {
    /// The name used for the undo step and for result sheets.
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Integrate(_) => "Integrate",
            Tool::IntegrateRanges(_) => "Integrate ranges",
            Tool::CurveMath(spec) => match (spec.operand, spec.op) {
                (CurveOperand::Reference(_), CurveOp::Subtract) => "Subtract reference",
                _ => "Curve math",
            },
            Tool::Differentiate(_) => "Differentiate",
            Tool::Smooth(_) => "Smooth",
            Tool::Normalize(_) => "Normalize",
            Tool::SubtractBaseline(_) => "Subtract baseline",
            Tool::Interpolate(_) => "Interpolate",
            Tool::FindPeaks(_) => "Find peaks",
            Tool::PeakChain(_) => "Analyze peaks",
            Tool::Statistics => "Statistics",
            Tool::TTest(spec) => match spec.kind {
                TTestKind::OneSample { .. } => "One-sample t-test",
                TTestKind::TwoSample { .. } => "Two-sample t-test",
                TTestKind::Paired { .. } => "Paired t-test",
            },
            Tool::Anova(_) => "One-way ANOVA",
            Tool::Histogram(_) => "Histogram",
            Tool::Normality(_) => "Normality test",
            Tool::TwoWayAnova(_) => "Two-way ANOVA",
            Tool::MannWhitney(_) => "Mann-Whitney test",
            Tool::SignedRank(spec) => match spec.kind {
                SignedRankKind::OneSample { .. } => "One-sample Wilcoxon signed rank test",
                SignedRankKind::Paired => "Paired Wilcoxon signed rank test",
            },
            Tool::KruskalWallis(_) => "Kruskal-Wallis test",
            Tool::Correlation(_) => "Correlation coefficients",
        }
    }

    fn sheet_prefix(&self) -> &'static str {
        match self {
            Tool::Integrate(_) | Tool::IntegrateRanges(_) => "Integrate",
            Tool::CurveMath(_) => "Math",
            Tool::Interpolate(_) => "Interpolate",
            Tool::FindPeaks(_) => "Peaks",
            Tool::PeakChain(_) => "PeakAnalysis",
            Tool::Statistics => "Statistics",
            Tool::TTest(_) => "TTest",
            Tool::Anova(_) => "ANOVA",
            Tool::Histogram(_) => "Histogram",
            Tool::Normality(_) => "Normality",
            Tool::TwoWayAnova(_) => "ANOVA2Way",
            Tool::MannWhitney(_) => "MannWhitney",
            Tool::SignedRank(_) => "SignedRank",
            Tool::KruskalWallis(_) => "KruskalWallis",
            Tool::Correlation(_) => "Correlation",
            _ => "Result",
        }
    }
}

/// One run of a tool: the Y columns to work on (all in one sheet), the rows
/// and the tool with its options.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub columns: Vec<ColumnId>,
    pub rows: RowRange,
    pub tool: Tool,
}

/// A table of results: one row per input column.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResultTable {
    /// Header of each numeric column.
    pub headers: Vec<String>,
    /// Unit of each numeric column when every row has the same unit there; empty when the rows
    /// differ (see [`ResultTable::mixed_units`] and `row_units`).
    pub units: Vec<String>,
    /// A label (which input column) and one number per header.
    pub rows: Vec<(String, Vec<f64>)>,
    /// The unit of every number, row by row. Input columns can have different units (the A and
    /// mA columns of a CH Instruments import), so one unit per table column is not enough.
    pub row_units: Vec<Vec<String>>,
}

impl ResultTable {
    fn new(headers: &[&str]) -> Self {
        ResultTable { headers: headers.iter().map(|h| h.to_string()).collect(), ..Default::default() }
    }

    /// Adds a row and updates the shared units.
    fn push_row(&mut self, label: String, values: Vec<f64>, units: Vec<String>) {
        self.rows.push((label, values));
        self.row_units.push(units);
        // Kept up to date row by row (a rescan of all rows here made big tables quadratic).
        let last = self.rows.len() - 1;
        if last == 0 {
            self.units = (0..self.headers.len()).map(|i| self.unit(0, i).to_string()).collect();
        } else {
            for i in 0..self.headers.len() {
                if self.unit(last, i) != self.unit(0, i) {
                    self.units[i].clear();
                }
            }
        }
    }

    /// The unit of the number in row `row`, column `col`.
    pub fn unit(&self, row: usize, col: usize) -> &str {
        self.row_units.get(row).and_then(|u| u.get(col)).map_or("", String::as_str)
    }

    /// True if the rows of column `col` do not all have the same unit.
    pub fn mixed_units(&self, col: usize) -> bool {
        (1..self.rows.len()).any(|r| self.unit(r, col) != self.unit(0, col))
    }
}

/// What a tool did.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub book: WindowId,
    /// Index of the sheet that holds the input columns.
    pub sheet: usize,
    /// Columns that were added, in the order of the input columns.
    pub new_columns: Vec<ColumnId>,
    /// Index of the sheet that was added for the results, if any.
    pub new_sheet: Option<usize>,
    /// The graph that was added (a histogram's column plot), if any.
    pub new_graph: Option<WindowId>,
    /// The numbers per input column (integration, statistics, peak counts).
    pub table: Option<ResultTable>,
    /// One sentence for a status line, for example "Integrate: 3 columns, results in sheet Integrate1".
    pub message: String,
    /// Further plain-English lines, for example the goodness of fit of a multi-peak fit.
    pub notes: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AnalysisError {
    #[error("no columns were chosen")]
    NoColumns,
    #[error("there is no column with id {0:?}")]
    NoSuchColumn(ColumnId),
    #[error("the chosen columns are not all in the same sheet")]
    DifferentSheets,
    #[error("column {0} holds text; this tool needs numbers")]
    NotNumeric(String),
    #[error("column {0} is empty")]
    Empty(String),
    #[error("rows {0} to {1} are not a valid range for column {2}, which has {3} rows")]
    BadRows(usize, usize, String, usize),
    #[error("column {0} has no row whose X value lies between {1} and {2}")]
    NoRowsInXRange(String, f64, f64),
    #[error("column {0} has fewer than two rows with both an X and a Y value in the chosen range")]
    TooFewPoints(String),
    #[error("{0}")]
    BadOption(String),
    #[error("there is no such sheet")]
    NoSuchSheet,
    #[error("{0}")]
    Condition(String),
}

/// A column to add next to its source column.
struct NewColumn {
    after: ColumnId,
    long_name: String,
    units: String,
    comments: String,
    data: Vec<f64>,
}

/// Columns for a new result sheet (an X and Y pair per input column).
struct SheetColumn {
    designation: Designation,
    long_name: String,
    units: String,
    comments: String,
    data: ColumnData,
}

/// The data one input column contributes to a tool run.
struct Input<'a> {
    id: ColumnId,
    /// Short name, for error messages.
    short: String,
    /// "B: Current, Scan 2" style label for result tables.
    label: String,
    /// Long name or short name, for the long names of result columns.
    name: String,
    units: String,
    x_name: String,
    x_units: String,
    /// X values, as long as `y` (row numbers when the sheet has no X column).
    x: Vec<f64>,
    y: &'a [f64],
    /// First and last row of the range, 0-based inclusive.
    first: usize,
    last: usize,
}

impl Input<'_> {
    fn xs(&self) -> &[f64] {
        &self.x[self.first..=self.last]
    }

    fn ys(&self) -> &[f64] {
        &self.y[self.first..=self.last]
    }

    /// Puts values computed for the row range back at their rows; other rows are missing.
    fn aligned(&self, values: &[f64]) -> Vec<f64> {
        let mut out = vec![f64::NAN; self.y.len()];
        let n = values.len().min(self.last + 1 - self.first);
        out[self.first..self.first + n].copy_from_slice(&values[..n]);
        out
    }

    /// "rows 11 to 61" when a part of the column was used, otherwise empty.
    fn range_note(&self) -> String {
        if self.first == 0 && self.last + 1 == self.y.len() {
            String::new()
        } else {
            format!(", rows {} to {}", self.first + 1, self.last + 1)
        }
    }
}

pub(crate) fn column_label(c: &Column) -> String {
    let detail = match (c.long_name.is_empty(), c.comments.lines().next().unwrap_or_default()) {
        (true, "") => String::new(),
        (true, comment) => comment.to_string(),
        (false, "") => c.long_name.clone(),
        (false, comment) => format!("{}, {}", c.long_name, comment),
    };
    if detail.is_empty() { c.short_name.clone() } else { format!("{}: {}", c.short_name, detail) }
}

/// Unit of `y` times `x`. Current times time is written as charge.
fn unit_product(y: &str, x: &str) -> String {
    match (y, x) {
        ("", _) => x.to_string(),
        (_, "") => y.to_string(),
        ("A", "s") => "C".to_string(),
        ("mA", "s") => "mC".to_string(),
        ("uA" | "\u{b5}A" | "\u{3bc}A", "s") => "\u{b5}C".to_string(),
        ("nA", "s") => "nC".to_string(),
        _ => format!("{y}\u{b7}{x}"),
    }
}

/// Unit of the `order`-th derivative of `y` with respect to `x`.
fn unit_ratio(y: &str, x: &str, order: usize) -> String {
    let below = match (x, order) {
        ("", _) => String::new(),
        (_, 1) => x.to_string(),
        _ => format!("{x}^{order}"),
    };
    match (y, below.as_str()) {
        (_, "") => y.to_string(),
        ("", b) => format!("1/{b}"),
        (y, b) => format!("{y}/{b}"),
    }
}

pub(crate) fn resolve_rows(rows: RowRange, short: &str, x: &[f64], n: usize) -> Result<(usize, usize), AnalysisError> {
    match rows {
        RowRange::All => Ok((0, n - 1)),
        RowRange::Rows(from, to) => {
            if from < 1 || to < from || from > n {
                return Err(AnalysisError::BadRows(from, to, short.to_string(), n));
            }
            Ok((from - 1, to.min(n) - 1))
        }
        RowRange::XBetween(a, b) => {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let inside = |v: f64| v >= lo && v <= hi;
            let first = x
                .iter()
                .position(|v| inside(*v))
                .ok_or_else(|| AnalysisError::NoRowsInXRange(short.to_string(), lo, hi))?;
            // The run continues over missing X values and ends at the first X outside the interval.
            let mut last = first;
            for (i, v) in x.iter().enumerate().skip(first + 1) {
                if inside(*v) {
                    last = i;
                } else if !v.is_nan() {
                    break;
                }
            }
            Ok((first, last))
        }
    }
}

/// A sheet name that the workbook does not use yet: `prefix` followed by the smallest free number.
pub(crate) fn free_sheet_name(sheets: &[Worksheet], prefix: &str) -> String {
    (1..)
        .map(|n| format!("{prefix}{n}"))
        .find(|name| !sheets.iter().any(|s| s.name.eq_ignore_ascii_case(name)))
        .expect("the name sequence is unbounded")
}

/// The peak chain for every input column: one table row per peak, a baseline column beside each
/// source and, with a fit, the fitted curve.
fn run_peak_chain(
    spec: &PeakChainSpec,
    inputs: &[Input<'_>],
    beside: &mut Vec<NewColumn>,
    notes: &mut Vec<String>,
) -> Result<ResultTable, AnalysisError> {
    let mut headers: Vec<&str> = vec!["Row", "Center", "Height"];
    if spec.integrate {
        headers.extend(["Area", "Area %", "FWHM", "Left limit", "Right limit"]);
    }
    if spec.fit.is_some() {
        headers.extend(["Fit center", "Fit height", "Fit FWHM", "Fit area", "Fit area %"]);
    }
    let mut t = ResultTable::new(&headers);
    for inp in inputs {
        let (xs, ys) = (inp.xs(), inp.ys());
        let result = calc::analyze_peaks(xs, ys, &spec.chain).map_err(|e| match e {
            calc::ChainError::NoBaseline => AnalysisError::BadOption(format!(
                "column {}: the baseline could not be made (anchor points need at least two different X values, and the data at least two points)",
                inp.short
            )),
            calc::ChainError::UnsortedX => AnalysisError::BadOption(format!(
                "column {}: X is not sorted; sort the rows along X before analysing peaks",
                inp.short
            )),
        })?;
        if result.peaks.is_empty() {
            return Err(AnalysisError::BadOption(format!(
                "no peaks were found in column {}; lower the height filter or change the search method",
                inp.short
            )));
        }
        if !matches!(spec.chain.baseline, ChainBaseline::None) {
            beside.push(NewColumn {
                after: inp.id,
                long_name: format!("Baseline of {}", inp.name),
                units: inp.units.clone(),
                comments: format!("Baseline of {}{}", inp.short, inp.range_note()),
                data: inp.aligned(&result.baseline),
            });
        }
        // The multi-peak fit, started at the found peaks.
        let mut fitted_peaks: Vec<[f64; 5]> = Vec::new();
        if let Some(fs) = spec.fit {
            let n = xs.len();
            let (y0, m) = match (fs.baseline, &spec.chain.baseline) {
                (_, ChainBaseline::None) => {
                    (ys.iter().copied().filter(|v| v.is_finite()).fold(f64::INFINITY, f64::min), 0.0)
                }
                (FitBaseline::Constant, _) => {
                    let v: Vec<f64> = result.baseline.iter().copied().filter(|b| b.is_finite()).collect();
                    (v.iter().sum::<f64>() / v.len().max(1) as f64, 0.0)
                }
                (FitBaseline::Linear, _) => {
                    let ok: Vec<usize> =
                        (0..n).filter(|&i| result.baseline[i].is_finite() && xs[i].is_finite()).collect();
                    let (f, l) = (ok[0], ok[ok.len() - 1]);
                    let (b0, b1) = (result.baseline[f], result.baseline[l]);
                    let m = if xs[l] == xs[f] { 0.0 } else { (b1 - b0) / (xs[l] - xs[f]) };
                    (b0 - m * xs[f], m)
                }
            };
            let finite_x = xs.iter().copied().filter(|v| v.is_finite());
            let span = finite_x.clone().fold(f64::NEG_INFINITY, f64::max) - finite_x.fold(f64::INFINITY, f64::min);
            let starts: Vec<sheaf_fit::multipeak::PeakStart> = result
                .peaks
                .iter()
                .map(|p| sheaf_fit::multipeak::PeakStart {
                    center: p.center,
                    fwhm: if p.fwhm > 0.0 { p.fwhm } else { span / 20.0 },
                    height: ys[p.index] - (y0 + m * p.center),
                })
                .collect();
            let mut func =
                sheaf_fit::multipeak::multi_peak_function(fs.shape, fs.baseline, &starts).with_initial("y0", y0);
            if fs.baseline == FitBaseline::Linear {
                func = func.with_initial("m", m);
            }
            let report = sheaf_fit::nlsf::fit(&func, xs, ys, &sheaf_fit::nlsf::FitOptions::default()).map_err(|e| {
                AnalysisError::BadOption(format!("column {}: the multi-peak fit failed: {e}", inp.short))
            })?;
            let get = |name: String| report.param(&name).map_or(f64::NAN, |p| p.value);
            let mut total = 0.0;
            for k in 1..=starts.len() {
                let (xc, w, a) = (get(format!("xc{k}")), get(format!("w{k}")), get(format!("A{k}")));
                total += a;
                fitted_peaks.push([xc, fs.shape.height(w, a), fs.shape.fwhm(w), a, 0.0]);
            }
            for p in fitted_peaks.iter_mut() {
                p[4] = if total != 0.0 { 100.0 * p[3] / total } else { f64::NAN };
            }
            notes.push(format!(
                "{}: {} fit of {} peaks, R^2 = {:.6}, reduced chi-square = {:.6e}, {} iterations{}",
                inp.label,
                fs.shape.name(),
                starts.len(),
                report.stats.r_squared,
                report.stats.reduced_chi_sqr,
                report.stats.iterations.unwrap_or(0),
                if report.warnings.is_empty() { String::new() } else { format!(" ({})", report.warnings.join("; ")) }
            ));
            beside.push(NewColumn {
                after: inp.id,
                long_name: format!("Peak fit of {}", inp.name),
                units: inp.units.clone(),
                comments: format!("{} multi-peak fit of {}{}", fs.shape.name(), inp.short, inp.range_note()),
                data: inp.aligned(&report.fitted),
            });
        }
        let xu = &inp.x_units;
        let area_unit = unit_product(&inp.units, xu);
        for (i, p) in result.peaks.iter().enumerate() {
            let mut v = vec![(p.index + inp.first + 1) as f64, p.center, p.height];
            let mut u = vec![String::new(), xu.clone(), inp.units.clone()];
            if spec.integrate {
                v.extend([p.area, p.area_percent, p.fwhm, p.left_x, p.right_x]);
                u.extend([area_unit.clone(), "%".to_string(), xu.clone(), xu.clone(), xu.clone()]);
            }
            if spec.fit.is_some() {
                v.extend(fitted_peaks[i]);
                u.extend([xu.clone(), inp.units.clone(), xu.clone(), area_unit.clone(), "%".to_string()]);
            }
            t.push_row(format!("Peak {} of {}", i + 1, inp.label), v, u);
        }
    }
    Ok(t)
}

mod statistics;

impl Project {
    /// A new graph with a column plot of each (bin centre, count) pair of columns.
    fn histogram_graph(&mut self, ids: &[ColumnId], plots: &[(usize, usize)]) -> WindowId {
        let long_name = ids.first().and_then(|c| self.locate(*c)).map(|(b, _, _)| b.long_name.clone());
        let graph = self.add_graph(crate::template::Template::Scatter);
        for &(x, y) in plots {
            self.add_plot_xy(graph, 0, Some(ids[x]), ids[y], None).expect("the graph and the new columns exist");
        }
        let count_title = plots.first().and_then(|&(_, y)| self.column(ids[y])).map(|c| c.display_name().to_string());
        if let Some(g) = self.graph_mut(graph) {
            if let Some(l) = g.layers.first_mut() {
                for p in &mut l.plots {
                    p.kind = crate::graph::PlotKind::Column;
                }
                l.apply_palette();
                l.y_axis.title = count_title;
                // The bars start at zero.
                l.y_axis.min = Some(0.0);
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        graph
    }

    /// Runs an analysis tool on worksheet columns. See the module text for
    /// where the results go. On error nothing is changed.
    pub fn analyze(&mut self, request: &Request) -> Result<Report, AnalysisError> {
        let first = *request.columns.first().ok_or(AnalysisError::NoColumns)?;
        let (book, _, _) = self.locate(first).ok_or(AnalysisError::NoSuchColumn(first))?;
        let book_id = book.id;
        let sheet_index = book
            .sheets
            .iter()
            .position(|s| s.column(first).is_some())
            .expect("the column was located in this workbook");
        let sheet = &book.sheets[sheet_index];

        let mut inputs = Vec::with_capacity(request.columns.len());
        // The two-way ANOVA reads its columns itself: the factor columns may hold text.
        let own_columns = matches!(request.tool, Tool::TwoWayAnova(_));
        for id in request.columns.iter().filter(|_| !own_columns) {
            let index = sheet.column_index(*id).ok_or_else(|| {
                if self.column(*id).is_some() {
                    AnalysisError::DifferentSheets
                } else {
                    AnalysisError::NoSuchColumn(*id)
                }
            })?;
            let c = &sheet.columns[index];
            let y = c.numeric().ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
            if y.is_empty() {
                return Err(AnalysisError::Empty(c.short_name.clone()));
            }
            let x_col = sheet.x_for(index).filter(|x| x.id != c.id && x.data.is_numeric());
            let x: Vec<f64> = match x_col.and_then(Column::numeric) {
                Some(v) => (0..y.len()).map(|i| v.get(i).copied().unwrap_or(f64::NAN)).collect(),
                None => (1..=y.len()).map(|i| i as f64).collect(),
            };
            let (first, last) = resolve_rows(request.rows, &c.short_name, &x, y.len())?;
            inputs.push(Input {
                id: c.id,
                short: c.short_name.clone(),
                label: column_label(c),
                name: c.display_name().to_string(),
                units: c.units.clone(),
                x_name: x_col.map_or_else(|| "Row".to_string(), |x| x.display_name().to_string()),
                x_units: x_col.map_or_else(String::new, |x| x.units.clone()),
                x,
                y,
                first,
                last,
            });
        }

        let mut beside: Vec<NewColumn> = Vec::new();
        let mut on_sheet: Vec<SheetColumn> = Vec::new();
        let mut table: Option<ResultTable> = None;
        let mut notes: Vec<String> = Vec::new();
        // Plots of a new graph: indices into `on_sheet` of the X and the Y column.
        let mut graph_plots: Vec<(usize, usize)> = Vec::new();

        match &request.tool {
            Tool::TwoWayAnova(spec) => {
                let out = statistics::two_way_anova(spec, sheet, &request.columns, request.rows)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
            }
            Tool::TTest(_)
            | Tool::Anova(_)
            | Tool::Histogram(_)
            | Tool::Normality(_)
            | Tool::MannWhitney(_)
            | Tool::SignedRank(_)
            | Tool::KruskalWallis(_)
            | Tool::Correlation(_) => {
                let out = statistics::run(&request.tool, &inputs)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
                graph_plots = out.plots;
            }
            Tool::PeakChain(spec) => {
                table = Some(run_peak_chain(spec, &inputs, &mut beside, &mut notes)?);
            }
            Tool::Integrate(spec) => {
                let mut t =
                    ResultTable::new(&["Area", "X start", "X end", "Peak X", "Peak height", "Width at half height"]);
                for inp in &inputs {
                    let options = calc::IntegrateOptions {
                        area_type: spec.area_type,
                        end_point_baseline: spec.end_point_baseline,
                        rows: None,
                    };
                    let r = calc::integrate(inp.xs(), inp.ys(), &options)
                        .ok_or_else(|| AnalysisError::TooFewPoints(inp.short.clone()))?;
                    let area_unit = unit_product(&inp.units, &inp.x_units);
                    let xu = &inp.x_units;
                    t.push_row(
                        inp.label.clone(),
                        vec![r.area, r.x1, r.x2, r.peak_x, r.peak_height, r.width],
                        vec![area_unit.clone(), xu.clone(), xu.clone(), xu.clone(), inp.units.clone(), xu.clone()],
                    );
                    if spec.cumulative {
                        let baseline = if spec.end_point_baseline { ", end-point baseline subtracted" } else { "" };
                        let kind = if spec.area_type == AreaType::Absolute { "Absolute integral" } else { "Integral" };
                        beside.push(NewColumn {
                            after: inp.id,
                            long_name: format!("Integral of {}", inp.name),
                            units: area_unit,
                            comments: format!("{kind} of {}{}{baseline}", inp.short, inp.range_note()),
                            data: inp.aligned(&r.cumulative),
                        });
                    }
                }
                table = Some(t);
            }
            Tool::IntegrateRanges(spec) => {
                if spec.ranges.is_empty() {
                    return Err(AnalysisError::BadOption("give at least one range to integrate".into()));
                }
                let mut t = ResultTable::new(&[
                    "First row",
                    "Last row",
                    "X start",
                    "X end",
                    "Area",
                    "Absolute area",
                    "Peak X",
                    "Peak height",
                    "Width at half height",
                ]);
                for inp in &inputs {
                    for (k, range) in spec.ranges.iter().enumerate() {
                        let (f, l) = match *range {
                            RowRange::All => (inp.first, inp.last),
                            RowRange::Rows(from, to) => {
                                let (f, l) = resolve_rows(*range, &inp.short, &inp.x, inp.y.len())?;
                                let (f, l) = (f.max(inp.first), l.min(inp.last));
                                if f > l {
                                    return Err(AnalysisError::BadRows(from, to, inp.short.clone(), inp.y.len()));
                                }
                                (f, l)
                            }
                            RowRange::XBetween(..) => {
                                let (f, l) = resolve_rows(*range, &inp.short, inp.xs(), inp.last + 1 - inp.first)?;
                                (f + inp.first, l + inp.first)
                            }
                        };
                        let (xs, ys) = (&inp.x[f..=l], &inp.y[f..=l]);
                        let options = |area_type| calc::IntegrateOptions {
                            area_type,
                            end_point_baseline: spec.end_point_baseline,
                            rows: None,
                        };
                        // Worksheet rows (1-based) of the first and last point that is actually used:
                        // points with a missing X or Y are skipped by the integration.
                        let usable = |i: &usize| xs[*i].is_finite() && ys[*i].is_finite();
                        let first_used = (0..xs.len()).find(usable).map_or(f + 1, |i| f + i + 1);
                        let last_used = (0..xs.len()).rev().find(usable).map_or(f + 1, |i| f + i + 1);
                        let math = calc::integrate(xs, ys, &options(AreaType::Mathematical))
                            .ok_or_else(|| AnalysisError::TooFewPoints(inp.short.clone()))?;
                        let abs = calc::integrate(xs, ys, &options(AreaType::Absolute))
                            .ok_or_else(|| AnalysisError::TooFewPoints(inp.short.clone()))?;
                        let area_unit = unit_product(&inp.units, &inp.x_units);
                        let xu = &inp.x_units;
                        t.push_row(
                            format!("{}, range {}", inp.label, k + 1),
                            vec![
                                first_used as f64,
                                last_used as f64,
                                math.x1,
                                math.x2,
                                math.area,
                                abs.area,
                                math.peak_x,
                                math.peak_height,
                                math.width,
                            ],
                            vec![
                                String::new(),
                                String::new(),
                                xu.clone(),
                                xu.clone(),
                                area_unit.clone(),
                                area_unit,
                                xu.clone(),
                                inp.units.clone(),
                                xu.clone(),
                            ],
                        );
                    }
                }
                table = Some(t);
            }
            Tool::CurveMath(spec) => {
                let reference = match spec.operand {
                    CurveOperand::Constant(c) => {
                        if !c.is_finite() {
                            return Err(AnalysisError::BadOption(format!("{c} is not a usable operand")));
                        }
                        if spec.op == CurveOp::Divide && c == 0.0 {
                            return Err(AnalysisError::BadOption("cannot divide by 0".into()));
                        }
                        None
                    }
                    CurveOperand::Reference(id) => {
                        let (_, rsheet, rcol) = self.locate(id).ok_or(AnalysisError::NoSuchColumn(id))?;
                        let ry = rcol.numeric().ok_or_else(|| AnalysisError::NotNumeric(rcol.short_name.clone()))?;
                        let at = rsheet.column_index(id).expect("the column was located in this sheet");
                        let x_col = rsheet.x_for(at).filter(|x| x.id != id && x.data.is_numeric());
                        let rx: Vec<f64> = match x_col.and_then(Column::numeric) {
                            Some(v) => (0..ry.len()).map(|i| v.get(i).copied().unwrap_or(f64::NAN)).collect(),
                            None => (1..=ry.len()).map(|i| i as f64).collect(),
                        };
                        Some((rcol.short_name.clone(), rcol.display_name().to_string(), rx, ry))
                    }
                };
                let word = match spec.op {
                    CurveOp::Add => "plus",
                    CurveOp::Subtract => "minus",
                    CurveOp::Multiply => "times",
                    CurveOp::Divide => "divided by",
                    CurveOp::Power => "to the power of",
                };
                for inp in &inputs {
                    let units = match (spec.op, reference.is_some()) {
                        (CurveOp::Add | CurveOp::Subtract, _) => inp.units.clone(),
                        (CurveOp::Multiply | CurveOp::Divide, false) => inp.units.clone(),
                        _ => String::new(),
                    };
                    match (&reference, spec.operand) {
                        (None, CurveOperand::Constant(c)) => {
                            beside.push(NewColumn {
                                after: inp.id,
                                long_name: format!("{} {word} {c}", inp.name),
                                units,
                                comments: format!("{} {word} {c}{}", inp.short, inp.range_note()),
                                data: inp.aligned(&calc::curve_math_const(inp.ys(), spec.op, c)),
                            });
                        }
                        (Some((rshort, rname, rx, ry)), _) => {
                            let valid =
                                rx.iter().zip(ry.iter()).filter(|(a, b)| a.is_finite() && b.is_finite()).count();
                            if valid < 2 {
                                return Err(AnalysisError::TooFewPoints(rshort.clone()));
                            }
                            let r = calc::curve_math_ref(inp.xs(), inp.ys(), rx, ry, spec.op, spec.common_only)
                                .ok_or_else(|| {
                                    AnalysisError::BadOption(format!(
                                        "column {} and the reference column {rshort} share no X range",
                                        inp.short
                                    ))
                                })?;
                            let long_name = format!("{} {word} {rname}", inp.name);
                            let comments = format!("{} {word} reference {rshort}{}", inp.short, inp.range_note());
                            if spec.common_only {
                                on_sheet.push(SheetColumn {
                                    designation: Designation::X,
                                    long_name: inp.x_name.clone(),
                                    units: inp.x_units.clone(),
                                    comments: String::new(),
                                    data: ColumnData::numeric(r.x),
                                });
                                on_sheet.push(SheetColumn {
                                    designation: Designation::Y,
                                    long_name,
                                    units,
                                    comments: format!("{comments}, common X range only"),
                                    data: ColumnData::numeric(r.y),
                                });
                            } else {
                                beside.push(NewColumn {
                                    after: inp.id,
                                    long_name,
                                    units,
                                    comments,
                                    data: inp.aligned(&r.y),
                                });
                            }
                        }
                        (None, CurveOperand::Reference(_)) => unreachable!("the reference was read above"),
                    }
                }
            }
            Tool::Differentiate(options) => {
                if !(1..=9).contains(&options.order) {
                    return Err(AnalysisError::BadOption("the derivative order must be between 1 and 9".into()));
                }
                if options.smooth {
                    // Otherwise the local polynomial fit is impossible and the column comes out empty.
                    let window = 2 * (options.npts / 2) + 1;
                    if options.poly >= window {
                        return Err(AnalysisError::BadOption(format!(
                            "the polynomial order ({}) of the smoothing derivative must be smaller than its \
                             window of {window} points",
                            options.poly
                        )));
                    }
                    if options.order > options.poly {
                        return Err(AnalysisError::BadOption(format!(
                            "a derivative of order {} needs a smoothing polynomial of at least that order (it is {})",
                            options.order, options.poly
                        )));
                    }
                }
                for inp in &inputs {
                    let d = calc::differentiate(inp.xs(), inp.ys(), options);
                    let which = if options.order == 1 { String::new() } else { format!(" (order {})", options.order) };
                    beside.push(NewColumn {
                        after: inp.id,
                        long_name: format!("Derivative{which} of {}", inp.name),
                        units: unit_ratio(&inp.units, &inp.x_units, options.order),
                        comments: format!("Derivative{which} of {}{}", inp.short, inp.range_note()),
                        data: inp.aligned(&d),
                    });
                }
            }
            Tool::Smooth(options) => {
                if options.npts == 0 {
                    return Err(AnalysisError::BadOption("the smoothing window needs at least one point".into()));
                }
                let how = match options.method {
                    SmoothMethod::AdjacentAveraging { weighted: false } => "adjacent averaging".to_string(),
                    SmoothMethod::AdjacentAveraging { weighted: true } => "weighted adjacent averaging".to_string(),
                    SmoothMethod::SavitzkyGolay { poly_order } => format!("Savitzky-Golay, order {poly_order}"),
                    SmoothMethod::Percentile { percent } => format!("percentile filter, {percent} %"),
                    SmoothMethod::FftFilter => "FFT filter".to_string(),
                    #[allow(unreachable_patterns)]
                    _ => "smoothing".to_string(),
                };
                for inp in &inputs {
                    let s = calc::smooth_xy(inp.xs(), inp.ys(), options);
                    beside.push(NewColumn {
                        after: inp.id,
                        long_name: format!("Smoothed {}", inp.name),
                        units: inp.units.clone(),
                        comments: format!("{} smoothed: {how}, {} points{}", inp.short, options.npts, inp.range_note()),
                        data: inp.aligned(&s),
                    });
                }
            }
            Tool::Normalize(method) => {
                if let NormalizeMethod::Divide(d) = method
                    && (*d == 0.0 || !d.is_finite())
                {
                    return Err(AnalysisError::BadOption(format!(
                        "cannot divide by {d}; give a finite value other than 0"
                    )));
                }
                for inp in &inputs {
                    let v = calc::normalize(inp.ys(), *method);
                    let units =
                        if matches!(method, NormalizeMethod::Divide(_)) { inp.units.clone() } else { String::new() };
                    beside.push(NewColumn {
                        after: inp.id,
                        long_name: format!("Normalized {}", inp.name),
                        units,
                        comments: format!("{} normalized{}", inp.short, inp.range_note()),
                        data: inp.aligned(&v),
                    });
                }
            }
            Tool::SubtractBaseline(spec) => {
                for inp in &inputs {
                    let (xs, ys) = (inp.xs(), inp.ys());
                    let (v, what) = match *spec {
                        BaselineSpec::Constant(level) => (calc::subtract_constant(ys, level), format!("minus {level}")),
                        BaselineSpec::Minimum => {
                            let min = ys.iter().copied().filter(|v| v.is_finite()).fold(f64::INFINITY, f64::min);
                            if !min.is_finite() {
                                return Err(AnalysisError::TooFewPoints(inp.short.clone()));
                            }
                            (calc::subtract_constant(ys, min), "minus its minimum".to_string())
                        }
                        BaselineSpec::EndPoints => {
                            let valid = |i: &usize| xs[*i].is_finite() && ys[*i].is_finite();
                            let a = (0..ys.len()).find(valid);
                            let b = (0..ys.len()).rev().find(valid);
                            match (a, b) {
                                (Some(a), Some(b)) if a != b && xs[a] != xs[b] => (
                                    calc::subtract_line(xs, ys, (xs[a], ys[a]), (xs[b], ys[b])),
                                    "minus the line through its end points".to_string(),
                                ),
                                _ => return Err(AnalysisError::TooFewPoints(inp.short.clone())),
                            }
                        }
                        BaselineSpec::Line(p1, p2) => {
                            if p1.0 == p2.0 {
                                return Err(AnalysisError::BadOption(
                                    "the two baseline points need different X values".into(),
                                ));
                            }
                            (
                                calc::subtract_line(xs, ys, p1, p2),
                                format!("minus the line through ({}, {}) and ({}, {})", p1.0, p1.1, p2.0, p2.1),
                            )
                        }
                    };
                    beside.push(NewColumn {
                        after: inp.id,
                        long_name: format!("{} minus baseline", inp.name),
                        units: inp.units.clone(),
                        comments: format!("{} {what}{}", inp.short, inp.range_note()),
                        data: inp.aligned(&v),
                    });
                }
            }
            Tool::Interpolate(spec) => {
                if spec.npts < 2 {
                    return Err(AnalysisError::BadOption("interpolation needs at least two result points".into()));
                }
                if spec.method == InterpMethod::BSpline && !(spec.smoothing >= 0.0 && spec.smoothing.is_finite()) {
                    return Err(AnalysisError::BadOption(
                        "the B-spline smoothing factor must be a number of at least 0".into(),
                    ));
                }
                let options = calc::InterpOptions {
                    method: spec.method,
                    boundary: spec.boundary,
                    extrapolation: calc::Extrapolation::Extrapolate,
                    smoothing: spec.smoothing,
                };
                let needed = if spec.method == InterpMethod::BSpline { 4 } else { 2 };
                for inp in &inputs {
                    let valid = inp.xs().iter().zip(inp.ys()).filter(|(a, b)| a.is_finite() && b.is_finite()).count();
                    if valid < needed {
                        return Err(if needed == 2 {
                            AnalysisError::TooFewPoints(inp.short.clone())
                        } else {
                            AnalysisError::BadOption(format!(
                                "column {} has fewer than four rows with both an X and a Y value; the B-spline needs four",
                                inp.short
                            ))
                        });
                    }
                    let (lo, hi) = match spec.range {
                        Some((a, b)) => (Some(a.min(b)), Some(a.max(b))),
                        None => (None, None),
                    };
                    let (gx, gy) = calc::interp1xy(inp.xs(), inp.ys(), spec.npts, lo, hi, &options);
                    on_sheet.push(SheetColumn {
                        designation: Designation::X,
                        long_name: inp.x_name.clone(),
                        units: inp.x_units.clone(),
                        comments: String::new(),
                        data: ColumnData::numeric(gx),
                    });
                    on_sheet.push(SheetColumn {
                        designation: Designation::Y,
                        long_name: inp.name.clone(),
                        units: inp.units.clone(),
                        comments: format!("{} interpolated to {} points{}", inp.label, spec.npts, inp.range_note()),
                        data: ColumnData::numeric(gy),
                    });
                }
            }
            Tool::FindPeaks(options) => {
                let mut t = ResultTable::new(&["Peaks found"]);
                for inp in &inputs {
                    let found = calc::find_peaks(inp.xs(), inp.ys(), options);
                    t.push_row(inp.label.clone(), vec![found.x.len() as f64], vec![String::new()]);
                    on_sheet.push(SheetColumn {
                        designation: Designation::X,
                        long_name: format!("Peak position ({})", inp.x_name),
                        units: inp.x_units.clone(),
                        comments: inp.label.clone(),
                        data: ColumnData::numeric(found.x),
                    });
                    on_sheet.push(SheetColumn {
                        designation: Designation::Y,
                        long_name: format!("Peak height ({})", inp.name),
                        units: inp.units.clone(),
                        comments: inp.label.clone(),
                        data: ColumnData::numeric(found.y),
                    });
                    on_sheet.push(SheetColumn {
                        designation: Designation::Disregard,
                        long_name: "Row".to_string(),
                        units: String::new(),
                        comments: inp.label.clone(),
                        data: ColumnData::numeric(found.rows.iter().map(|r| (r + inp.first) as f64).collect()),
                    });
                }
                table = Some(t);
            }
            Tool::Statistics => {
                let mut t = ResultTable::new(&[
                    "N", "Missing", "Mean", "SD", "SEM", "Sum", "Minimum", "Q1", "Median", "Q3", "Maximum",
                ]);
                for inp in &inputs {
                    let s = calc::stats(inp.ys());
                    let units = (0..t.headers.len()).map(|i| if i < 2 { String::new() } else { inp.units.clone() });
                    t.push_row(
                        inp.label.clone(),
                        vec![
                            s.n as f64,
                            s.missing as f64,
                            s.mean,
                            s.sd,
                            s.sem,
                            s.sum,
                            s.min,
                            s.q1,
                            s.median,
                            s.q3,
                            s.max,
                        ],
                        units.collect(),
                    );
                }
                table = Some(t);
            }
        }

        // A table becomes a sheet too: a text column that names the input, then one column per number.
        if let Some(t) = table.as_ref().filter(|_| on_sheet.is_empty()) {
            on_sheet.push(SheetColumn {
                designation: Designation::Label,
                long_name: "Data".to_string(),
                units: String::new(),
                comments: String::new(),
                data: ColumnData::text(t.rows.iter().map(|(label, _)| label.clone()).collect()),
            });
            for (i, header) in t.headers.iter().enumerate() {
                on_sheet.push(SheetColumn {
                    designation: Designation::Y,
                    long_name: header.clone(),
                    units: t.units.get(i).cloned().unwrap_or_default(),
                    comments: String::new(),
                    data: ColumnData::numeric(t.rows.iter().map(|(_, v)| v[i]).collect()),
                });
                // Rows with different units: the unit of each row goes into a text column beside it.
                if t.mixed_units(i) {
                    on_sheet.push(SheetColumn {
                        designation: Designation::Label,
                        long_name: format!("{header} unit"),
                        units: String::new(),
                        comments: String::new(),
                        data: ColumnData::text((0..t.rows.len()).map(|r| t.unit(r, i).to_string()).collect()),
                    });
                }
            }
        }

        // Everything was computed; now change the project.
        let count = request.columns.len();
        drop(inputs);
        let mut new_columns = Vec::with_capacity(beside.len());
        let mut added_names: Vec<String> = Vec::new();
        for nc in beside {
            let id = self.new_column_id();
            let ws = &mut self.workbook_mut(book_id).expect("workbook exists").sheets[sheet_index];
            let source = ws.column_index(nc.after).expect("source column exists");
            // Stay behind the error and label columns that belong to the source column, and behind
            // result columns added for it earlier in this run.
            let mut at = source + 1;
            while at < ws.columns.len()
                && (matches!(
                    ws.columns[at].designation,
                    Designation::YError | Designation::XError | Designation::Label
                ) || new_columns.contains(&ws.columns[at].id))
            {
                at += 1;
            }
            let name = ws.next_short_name();
            added_names.push(format!("{name} \"{}\"", nc.long_name));
            let column = Column::new(id, name, Designation::Y, ColumnData::numeric(nc.data)).with_labels(
                &nc.long_name,
                &nc.units,
                &nc.comments,
            );
            ws.columns.insert(at, column);
            new_columns.push(id);
        }
        let mut new_sheet = None;
        let mut new_graph = None;
        let mut sheet_name = String::new();
        let mut sheet_column_ids: Vec<ColumnId> = Vec::new();
        if !on_sheet.is_empty() {
            let mut ws = Worksheet::new(free_sheet_name(
                &self.workbook(book_id).expect("workbook exists").sheets,
                request.tool.sheet_prefix(),
            ));
            for sc in on_sheet {
                let id = self.new_column_id();
                let name = ws.next_short_name();
                ws.columns.push(Column::new(id, name, sc.designation, sc.data).with_labels(
                    &sc.long_name,
                    &sc.units,
                    &sc.comments,
                ));
                new_columns.push(id);
                sheet_column_ids.push(id);
            }
            sheet_name = ws.name.clone();
            let b = self.workbook_mut(book_id).expect("workbook exists");
            b.sheets.push(ws);
            new_sheet = Some(b.sheets.len() - 1);
        }
        if !graph_plots.is_empty() {
            new_graph = Some(self.histogram_graph(&sheet_column_ids, &graph_plots));
        }
        let columns = if count == 1 { "1 column".to_string() } else { format!("{count} columns") };
        // Name the added columns, so that the result can be found without opening the sheet.
        const SHOWN: usize = 6;
        let mut added = added_names.iter().take(SHOWN).cloned().collect::<Vec<_>>().join(", ");
        if added_names.len() > SHOWN {
            added.push_str(&format!(" and {} more", added_names.len() - SHOWN));
        }
        let message = match (new_sheet, added.is_empty()) {
            (Some(_), true) => format!("{}: {columns}, results in sheet {sheet_name}", request.tool.name()),
            (Some(_), false) => {
                format!("{}: {columns}, results in sheet {sheet_name} and in new columns {added}", request.tool.name())
            }
            (None, _) => format!("{}: {columns}, results in new columns {added}", request.tool.name()),
        };
        let message = match new_graph.and_then(|g| self.graph(g)) {
            Some(g) => format!("{message}; graph {}", g.name),
            None => message,
        };
        Ok(Report { book: book_id, sheet: sheet_index, new_columns, new_sheet, new_graph, table, message, notes })
    }

    /// Sorts all rows of a sheet by the values of one column. Rows keep
    /// together across all columns; rows with a missing key go last; rows
    /// with equal keys keep their order. Text keys sort by character order,
    /// ignoring case.
    pub fn sort_sheet(
        &mut self,
        book: WindowId,
        sheet: usize,
        key: ColumnId,
        descending: bool,
    ) -> Result<(), AnalysisError> {
        let ws = self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(AnalysisError::NoSuchSheet)?;
        let key_col = ws.column(key).ok_or(AnalysisError::NoSuchColumn(key))?;
        let n = ws.row_count();
        let mut order: Vec<usize> = (0..n).collect();
        match &key_col.data {
            ColumnData::Numeric(v) => {
                let value = |i: usize| v.get(i).copied().unwrap_or(f64::NAN);
                order.sort_by(|a, b| {
                    let (x, y) = (value(*a), value(*b));
                    match (x.is_nan(), y.is_nan()) {
                        (true, true) => std::cmp::Ordering::Equal,
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        (false, false) => {
                            let o = x.partial_cmp(&y).expect("neither is NaN");
                            if descending { o.reverse() } else { o }
                        }
                    }
                });
            }
            ColumnData::Text(v) => {
                let value = |i: usize| v.get(i).map(|s| s.to_lowercase()).unwrap_or_default();
                let keys: Vec<String> = (0..n).map(value).collect();
                order.sort_by(|a, b| match (keys[*a].is_empty(), keys[*b].is_empty()) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    (false, false) => {
                        let o = keys[*a].cmp(&keys[*b]);
                        if descending { o.reverse() } else { o }
                    }
                });
            }
        }
        let ws = &mut self.workbook_mut(book).expect("checked above").sheets[sheet];
        for c in &mut ws.columns {
            c.data = take_rows(&c.data, &order, n);
        }
        Ok(())
    }

    /// Keeps only the rows of a sheet for which a condition holds, for
    /// example `col(B) > 0.5 && col(A) < 10`. The condition is written in
    /// the column-formula language; a row is kept when the value is neither
    /// zero nor missing.
    ///
    /// With `to_new_sheet` the sheet stays as it is and the kept rows are
    /// copied to a new sheet of the same workbook (whose index is returned);
    /// otherwise the other rows are removed from the sheet itself.
    /// Returns the number of rows kept and the index of the sheet that holds them.
    pub fn filter_sheet(
        &mut self,
        book: WindowId,
        sheet: usize,
        condition: &str,
        to_new_sheet: bool,
    ) -> Result<(usize, usize), AnalysisError> {
        let expr = Expr::parse(condition).map_err(|e| AnalysisError::Condition(e.to_string()))?;
        let ws = self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(AnalysisError::NoSuchSheet)?;
        let n = ws.row_count();
        let values =
            crate::formula::eval_on_sheet(&expr, ws, n).map_err(|e| AnalysisError::Condition(e.to_string()))?;
        let keep: Vec<usize> = (0..n).filter(|i| values[*i] != 0.0 && !values[*i].is_nan()).collect();
        let kept = keep.len();
        if kept == 0 && n > 0 && !to_new_sheet {
            // Filtering in place would empty the whole sheet; that is almost always a mistyped condition.
            return Err(AnalysisError::Condition(format!(
                "no row matches '{condition}', so filtering in place would delete all {n} rows; nothing was changed"
            )));
        }
        if to_new_sheet {
            let mut copy =
                Worksheet::new(free_sheet_name(&self.workbook(book).expect("checked above").sheets, "Filter"));
            let source = self.workbook(book).expect("checked above").sheets[sheet].columns.clone();
            for mut c in source {
                c.id = self.new_column_id();
                c.data = take_rows(&c.data, &keep, n);
                c.formula = None;
                c.recalc = crate::column::RecalcMode::None;
                c.formula_state = crate::column::FormulaState::Current;
                c.formula_rows = None;
                copy.columns.push(c);
            }
            let b = self.workbook_mut(book).expect("checked above");
            b.sheets.push(copy);
            Ok((kept, b.sheets.len() - 1))
        } else {
            let ws = &mut self.workbook_mut(book).expect("checked above").sheets[sheet];
            for c in &mut ws.columns {
                c.data = take_rows(&c.data, &keep, n);
            }
            Ok((kept, sheet))
        }
    }
}

/// The rows `order` of a column, in that order. A column shorter than the
/// sheet (`rows` long) counts as padded with missing values; missing values
/// at the end of the result are dropped again so that short columns stay short.
fn take_rows(data: &ColumnData, order: &[usize], rows: usize) -> ColumnData {
    debug_assert!(order.iter().all(|i| *i < rows));
    match data {
        ColumnData::Numeric(v) => {
            let mut out: Vec<f64> = order.iter().map(|i| v.get(*i).copied().unwrap_or(f64::NAN)).collect();
            if v.len() < rows {
                while out.last().is_some_and(|x| x.is_nan()) {
                    out.pop();
                }
            }
            ColumnData::numeric(out)
        }
        ColumnData::Text(v) => {
            let mut out: Vec<String> = order.iter().map(|i| v.get(*i).cloned().unwrap_or_default()).collect();
            if v.len() < rows {
                while out.last().is_some_and(String::is_empty) {
                    out.pop();
                }
            }
            ColumnData::text(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sheet with X = 0, 1, .., 10 (s) and two Y columns in A: y = 2 and y = x.
    fn sample() -> (Project, WindowId, ColumnId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("demo");
        let x: Vec<f64> = (0..=10).map(f64::from).collect();
        let xc = p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
        let flat = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0; 11])).unwrap();
        let ramp = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x)).unwrap();
        p.column_mut(xc).unwrap().units = "s".into();
        p.column_mut(xc).unwrap().long_name = "Time".into();
        for (c, comment) in [(flat, "Scan 1"), (ramp, "Scan 2")] {
            let col = p.column_mut(c).unwrap();
            col.units = "A".into();
            col.long_name = "Current".into();
            col.comments = comment.into();
        }
        (p, book, flat, ramp)
    }

    fn request(columns: &[ColumnId], tool: Tool) -> Request {
        Request { columns: columns.to_vec(), rows: RowRange::All, tool }
    }

    /// A CH Instruments import has the current in A and in mA side by side. Integrating both must
    /// label each row with its own unit, not with the first column's (a factor of 1000).
    #[test]
    fn result_tables_keep_each_rows_unit() {
        let (mut p, book, flat, ramp) = sample();
        p.column_mut(ramp).unwrap().units = "mA".into();
        let r = p.analyze(&request(&[flat, ramp], Tool::Integrate(IntegrateSpec::default()))).unwrap();
        let t = r.table.as_ref().unwrap();
        assert_eq!((t.unit(0, 0), t.unit(1, 0)), ("C", "mC"));
        assert!(t.mixed_units(0));
        assert_eq!(t.units[0], "", "no single unit for the Area column");
        assert_eq!(t.units[1], "s", "both rows share the X unit");
        // The result sheet carries the units of the rows in a text column beside the numbers.
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let names: Vec<&str> = rs.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(&names[..4], ["Data", "Area", "Area unit", "X start"]);
        assert_eq!(rs.columns[2].text().unwrap(), ["C", "mC"]);
        // Statistics too.
        let s = p.analyze(&request(&[flat, ramp], Tool::Statistics)).unwrap();
        let t = s.table.unwrap();
        assert_eq!((t.unit(0, 2), t.unit(1, 2)), ("A", "mA"));
        assert_eq!(t.units[0], "", "N has no unit");
        assert!(!t.mixed_units(0));
    }

    #[test]
    fn options_that_would_give_an_empty_or_infinite_column_are_refused() {
        let (mut p, _, flat, _) = sample();
        let smooth_diff = |poly, npts, order| Tool::Differentiate(DiffOptions { order, smooth: true, poly, npts });
        for (tool, why) in [
            (smooth_diff(2, 1, 1), "window of 1 point"),
            (smooth_diff(5, 3, 1), "polynomial order not below the window"),
            (smooth_diff(2, 20, 3), "derivative order above the polynomial order"),
            (Tool::Normalize(NormalizeMethod::Divide(0.0)), "divide by zero"),
            (Tool::Normalize(NormalizeMethod::Divide(f64::INFINITY)), "divide by infinity"),
        ] {
            let before = p.clone();
            let err = p.analyze(&request(&[flat], tool)).unwrap_err();
            assert!(matches!(err, AnalysisError::BadOption(_)), "{why}: {err}");
            assert_eq!(p, before, "{why}: nothing may change");
        }
        // A usable smoothing derivative still works and names the column it added.
        let r = p.analyze(&request(&[flat], smooth_diff(2, 5, 1))).unwrap();
        assert!(r.message.contains("\"Derivative of Current\""), "{}", r.message);
    }

    #[test]
    fn filtering_in_place_never_empties_the_sheet() {
        let (mut p, book, _, _) = sample();
        let before = p.clone();
        let err = p.filter_sheet(book, 0, "col(A) > 100", false).unwrap_err();
        assert!(err.to_string().contains("nothing was changed"), "{err}");
        assert_eq!(p, before);
        // Into a new sheet an empty result is fine: the source is untouched.
        let (kept, _) = p.filter_sheet(book, 0, "col(A) > 100", true).unwrap();
        assert_eq!(kept, 0);
    }

    #[test]
    fn integrate_gives_a_table_a_result_sheet_and_running_integrals_next_to_the_sources() {
        let (mut p, book, flat, ramp) = sample();
        let r = p.analyze(&request(&[flat, ramp], Tool::Integrate(IntegrateSpec::default()))).unwrap();
        let t = r.table.as_ref().unwrap();
        assert_eq!(t.rows[0].0, "B: Current, Scan 1");
        assert_eq!(t.rows[0].1[0], 20.0, "area of y = 2 over 0..10");
        assert_eq!(t.rows[1].1[0], 50.0, "area of y = x over 0..10");
        assert_eq!(t.units[0], "C", "current times time is charge");
        // Sheet order is now A(X), B, its integral, C, its integral.
        let ws = &p.workbook(book).unwrap().sheets[0];
        let names: Vec<&str> = ws.columns.iter().map(|c| c.short_name.as_str()).collect();
        assert_eq!(names, ["A", "B", "D", "C", "E"]);
        assert_eq!(ws.columns[2].id, r.new_columns[0]);
        assert_eq!(ws.columns[2].numeric().unwrap()[10], 20.0);
        assert_eq!(ws.columns[4].numeric().unwrap()[10], 50.0);
        assert_eq!((ws.columns[2].long_name.as_str(), ws.columns[2].units.as_str()), ("Integral of Current", "C"));
        // The result sheet repeats the table.
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(rs.name, "Integrate1");
        assert_eq!(rs.columns[0].text().unwrap(), ["B: Current, Scan 1", "C: Current, Scan 2"]);
        assert_eq!(rs.columns[1].numeric().unwrap(), [20.0, 50.0]);
        assert_eq!(rs.columns[1].long_name, "Area");
        assert!(r.message.contains("Integrate1"));
        // A second run gets the next free sheet name.
        let r2 = p
            .analyze(&request(&[flat], Tool::Integrate(IntegrateSpec { cumulative: false, ..Default::default() })))
            .unwrap();
        assert_eq!(p.workbook(book).unwrap().sheets[r2.new_sheet.unwrap()].name, "Integrate2");
        assert_eq!(r2.new_columns.len(), 7, "only the columns of the result sheet");
    }

    #[test]
    fn a_row_range_limits_the_calculation_and_leaves_other_rows_missing() {
        let (mut p, _, _, ramp) = sample();
        let spec = IntegrateSpec { end_point_baseline: false, ..Default::default() };
        let r = p
            .analyze(&Request { columns: vec![ramp], rows: RowRange::Rows(3, 5), tool: Tool::Integrate(spec) })
            .unwrap();
        // Rows 3 to 5 are x = 2, 3, 4: the area under y = x is 6.
        assert_eq!(r.table.unwrap().rows[0].1[..3], [6.0, 2.0, 4.0]);
        let run = p.numeric(r.new_columns[0]).unwrap();
        assert!(run[1].is_nan() && run[5].is_nan());
        assert_eq!(run[2..5], [0.0, 2.5, 6.0]);
        assert!(p.column(r.new_columns[0]).unwrap().comments.contains("rows 3 to 5"));
        // The same rows chosen by their X values.
        let by_x = Request { columns: vec![ramp], rows: RowRange::XBetween(4.0, 2.0), tool: Tool::Integrate(spec) };
        assert_eq!(p.analyze(&by_x).unwrap().table.unwrap().rows[0].1[0], 6.0);
        // With the end-point baseline the area between a straight line and itself is zero.
        let spec = IntegrateSpec { end_point_baseline: true, ..spec };
        let r = p
            .analyze(&Request { columns: vec![ramp], rows: RowRange::Rows(3, 5), tool: Tool::Integrate(spec) })
            .unwrap();
        assert_eq!(r.table.unwrap().rows[0].1[0], 0.0);
    }

    #[test]
    fn x_range_takes_the_first_pass_of_a_cyclic_sweep() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let x = vec![0.0, 1.0, 2.0, 3.0, 2.0, 1.0, 0.0];
        p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 7])).unwrap();
        let r = p
            .analyze(&Request {
                columns: vec![y],
                rows: RowRange::XBetween(1.0, 2.0),
                tool: Tool::Normalize(NormalizeMethod::Max),
            })
            .unwrap();
        let v = p.numeric(r.new_columns[0]).unwrap();
        assert_eq!(v[1..3], [1.0, 1.0]);
        assert!(v[0].is_nan() && v[3].is_nan() && v[4].is_nan());
    }

    #[test]
    fn per_row_tools_add_a_column_with_a_fitting_name_and_unit() {
        let (mut p, book, _, ramp) = sample();
        let d = p.analyze(&request(&[ramp], Tool::Differentiate(DiffOptions::default()))).unwrap();
        let c = p.column(d.new_columns[0]).unwrap();
        assert_eq!((c.long_name.as_str(), c.units.as_str()), ("Derivative of Current", "A/s"));
        assert!(c.numeric().unwrap().iter().all(|v| (v - 1.0).abs() < 1e-12));
        let s = p.analyze(&request(&[ramp], Tool::Smooth(SmoothOptions::default()))).unwrap();
        assert_eq!(p.column(s.new_columns[0]).unwrap().long_name, "Smoothed Current");
        let n = p.analyze(&request(&[ramp], Tool::Normalize(NormalizeMethod::Max))).unwrap();
        assert_eq!(p.numeric(n.new_columns[0]).unwrap()[10], 1.0);
        assert_eq!(p.column(n.new_columns[0]).unwrap().units, "");
        let b = p.analyze(&request(&[ramp], Tool::SubtractBaseline(BaselineSpec::EndPoints))).unwrap();
        assert!(p.numeric(b.new_columns[0]).unwrap().iter().all(|v| v.abs() < 1e-12));
        let m = p.analyze(&request(&[ramp], Tool::SubtractBaseline(BaselineSpec::Constant(1.0)))).unwrap();
        assert_eq!(p.numeric(m.new_columns[0]).unwrap()[0], -1.0);
        // Every result column sits between its source and the next original column, so it shares the X column.
        let ws = &p.workbook(book).unwrap().sheets[0];
        let source = ws.column_index(ramp).unwrap();
        assert_eq!(ws.columns.len(), source + 6);
        assert!(d.new_sheet.is_none());
    }

    #[test]
    fn the_fft_filter_and_the_b_spline_are_accepted_and_bad_factors_are_refused() {
        let (mut p, book, _, ramp) = sample();
        let fft = SmoothOptions { method: SmoothMethod::FftFilter, npts: 3, ..SmoothOptions::default() };
        let r = p.analyze(&request(&[ramp], Tool::Smooth(fft))).unwrap();
        let col = p.column(r.new_columns[0]).unwrap();
        assert!(col.comments.contains("FFT filter"), "{}", col.comments);
        assert!(p.numeric(r.new_columns[0]).unwrap().iter().all(|v| v.is_finite()));
        let spec = InterpSpec { method: InterpMethod::BSpline, npts: 21, ..Default::default() };
        let r = p.analyze(&request(&[ramp], Tool::Interpolate(spec))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.columns.len(), 2);
        // a positive smoothing factor is accepted; a negative one is refused
        let smoothing = InterpSpec { smoothing: 0.5, ..spec };
        let r = p.analyze(&request(&[ramp], Tool::Interpolate(smoothing))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(ws.columns[1].numeric().unwrap().iter().all(|v| v.is_finite()));
        let negative = InterpSpec { smoothing: -0.5, ..spec };
        let err = p.analyze(&request(&[ramp], Tool::Interpolate(negative))).unwrap_err();
        assert!(err.to_string().contains("smoothing factor"), "{err}");
    }

    #[test]
    fn interpolate_peaks_and_statistics_write_a_new_sheet() {
        let (mut p, book, flat, ramp) = sample();
        let spec = InterpSpec { npts: 21, ..Default::default() };
        let r = p.analyze(&request(&[ramp], Tool::Interpolate(spec))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.name, "Interpolate1");
        assert_eq!(ws.columns[0].designation, Designation::X);
        assert_eq!(ws.columns[0].numeric().unwrap()[1], 0.5);
        assert_eq!(ws.columns[1].numeric().unwrap()[1], 0.5);
        assert_eq!((ws.columns[0].long_name.as_str(), ws.columns[1].units.as_str()), ("Time", "A"));

        // A triangle has one peak, at x = 3.
        let tri = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0., 1., 2., 3., 2., 1., 0., 0., 0., 0., 0.]))
            .unwrap();
        let r = p.analyze(&request(&[tri], Tool::FindPeaks(PeakOptions::default()))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.name, "Peaks1");
        assert_eq!(ws.columns[0].numeric().unwrap(), [3.0]);
        assert_eq!(ws.columns[1].numeric().unwrap(), [3.0]);
        assert_eq!(ws.columns[2].numeric().unwrap(), [4.0], "the worksheet row of the peak");
        assert_eq!(r.table.unwrap().rows[0].1, [1.0]);

        let r = p.analyze(&request(&[flat, ramp], Tool::Statistics)).unwrap();
        let t = r.table.unwrap();
        assert_eq!(t.headers[2], "Mean");
        assert_eq!((t.rows[0].1[0], t.rows[0].1[2], t.rows[1].1[2], t.rows[1].1[10]), (11.0, 2.0, 5.0, 10.0));
        assert_eq!(p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()].name, "Statistics1");
    }

    #[test]
    fn a_sheet_without_x_column_uses_row_numbers() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.0, 1.0])).unwrap();
        let r = p.analyze(&request(&[y], Tool::Integrate(IntegrateSpec::default()))).unwrap();
        assert_eq!(r.table.unwrap().rows[0].1[..3], [2.0, 1.0, 3.0]);
    }

    #[test]
    fn errors_change_nothing() {
        let (mut p, book, flat, _) = sample();
        let text = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["a".into()])).unwrap();
        let with_text = p.clone();
        let run = |p: &mut Project, columns: Vec<ColumnId>, rows| {
            p.analyze(&Request { columns, rows, tool: Tool::Integrate(IntegrateSpec::default()) })
        };
        assert_eq!(run(&mut p, vec![], RowRange::All), Err(AnalysisError::NoColumns));
        assert_eq!(run(&mut p, vec![flat, text], RowRange::All), Err(AnalysisError::NotNumeric("D".into())));
        assert_eq!(run(&mut p, vec![flat], RowRange::Rows(5, 2)), Err(AnalysisError::BadRows(5, 2, "B".into(), 11)));
        assert_eq!(run(&mut p, vec![flat], RowRange::Rows(4, 4)), Err(AnalysisError::TooFewPoints("B".into())));
        assert!(matches!(
            run(&mut p, vec![flat], RowRange::XBetween(50.0, 60.0)),
            Err(AnalysisError::NoRowsInXRange(..))
        ));
        assert_eq!(run(&mut p, vec![ColumnId(999)], RowRange::All), Err(AnalysisError::NoSuchColumn(ColumnId(999))));
        let other = p.add_workbook("");
        let far = p.add_column(other, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        assert_eq!(run(&mut p, vec![flat, far], RowRange::All), Err(AnalysisError::DifferentSheets));
        p.remove_window(other);
        assert_eq!(p.windows, with_text.windows);
    }

    #[test]
    fn result_columns_stay_behind_the_error_column_of_their_source() {
        let (mut p, book, flat, _) = sample();
        // Give B an error column directly after it: A(X) B err C.
        let err = p.new_column_id();
        let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
        ws.columns.insert(2, Column::new(err, "E", Designation::YError, ColumnData::numeric(vec![0.1; 11])));
        let r = p.analyze(&request(&[flat], Tool::Normalize(NormalizeMethod::Max))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!(ws.column_index(r.new_columns[0]), Some(3));
        assert_eq!(ws.y_error_for(1).map(|c| c.id), Some(err), "B keeps its error column");
    }

    #[test]
    fn sort_keeps_rows_together_and_puts_missing_keys_last() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let key =
            p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![3.0, f64::NAN, 1.0, 2.0, 1.0])).unwrap();
        let tag = p
            .add_column(
                book,
                0,
                Designation::Y,
                ColumnData::text(["c", "n", "a1", "b", "a2"].map(String::from).to_vec()),
            )
            .unwrap();
        let short = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![30.0, 0.0, 10.0])).unwrap();
        p.sort_sheet(book, 0, key, false).unwrap();
        assert_eq!(p.column(tag).unwrap().text().unwrap(), ["a1", "a2", "b", "c", "n"]);
        let k = p.numeric(key).unwrap();
        assert_eq!(k[..4], [1.0, 1.0, 2.0, 3.0]);
        assert!(k[4].is_nan());
        // The short column was padded with missing values for the sort; row "c" keeps its 30.
        let s = p.numeric(short).unwrap();
        assert_eq!((s[0], s[3], s[4]), (10.0, 30.0, 0.0));
        assert!(s[1].is_nan() && s[2].is_nan());
        p.sort_sheet(book, 0, key, true).unwrap();
        assert_eq!(p.column(tag).unwrap().text().unwrap(), ["c", "b", "a1", "a2", "n"]);
        // Sorting by a text column ignores case and puts empty cells last.
        p.sort_sheet(book, 0, tag, false).unwrap();
        assert_eq!(p.column(tag).unwrap().text().unwrap(), ["a1", "a2", "b", "c", "n"]);
        assert_eq!(p.sort_sheet(book, 3, key, false), Err(AnalysisError::NoSuchSheet));
    }

    #[test]
    fn filter_keeps_matching_rows_in_place_or_in_a_new_sheet() {
        let (mut p, book, flat, ramp) = sample();
        let (kept, sheet) = p.filter_sheet(book, 0, "col(C) >= 3 && col(A) < 6", true).unwrap();
        assert_eq!((kept, sheet), (3, 1));
        let b = p.workbook(book).unwrap();
        assert_eq!(b.sheets[1].name, "Filter1");
        assert_eq!(b.sheets[1].columns[2].numeric().unwrap(), [3.0, 4.0, 5.0]);
        assert_eq!(b.sheets[1].columns[2].long_name, "Current");
        assert_ne!(b.sheets[1].columns[2].id, ramp, "the copy has its own column ids");
        assert_eq!(p.numeric(ramp).unwrap().len(), 11, "the source sheet is untouched");
        let (kept, sheet) = p.filter_sheet(book, 0, "col(C) > 8", false).unwrap();
        assert_eq!((kept, sheet), (2, 0));
        assert_eq!(p.numeric(ramp).unwrap(), [9.0, 10.0]);
        assert_eq!(p.numeric(flat).unwrap(), [2.0, 2.0]);
        assert!(matches!(p.filter_sheet(book, 0, "col(Nope) > 1", false), Err(AnalysisError::Condition(_))));
        assert!(matches!(p.filter_sheet(book, 0, "1 +", false), Err(AnalysisError::Condition(_))));
    }

    /// A second workbook with a reference curve X = 2, 4, 6 and Y = 1, 2, 3 (that is x / 2).
    fn add_reference(p: &mut Project) -> ColumnId {
        let book = p.add_workbook("blank");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![2.0, 4.0, 6.0])).unwrap();
        p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap()
    }

    fn curve_math(op: CurveOp, operand: CurveOperand, common_only: bool) -> Tool {
        Tool::CurveMath(CurveMathSpec { op, operand, common_only })
    }

    #[test]
    fn subtracting_a_reference_from_another_workbook_adds_a_column_beside_the_data() {
        let (mut p, book, _, ramp) = sample();
        let reference = add_reference(&mut p);
        let tool = curve_math(CurveOp::Subtract, CurveOperand::Reference(reference), false);
        let r = p.analyze(&request(&[ramp], tool)).unwrap();
        // y = x minus the reference x / 2, also outside 2..6 where its end segments are continued
        let out = p.numeric(r.new_columns[0]).unwrap();
        for (i, v) in out.iter().enumerate() {
            assert!((v - i as f64 / 2.0).abs() < 1e-12, "row {i}: {v}");
        }
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!(ws.columns[3].id, r.new_columns[0], "right behind its source column");
        assert_eq!(ws.columns[3].units, "A");
        assert_eq!(ws.columns[3].long_name, "Current minus B");
        assert!(r.message.starts_with("Subtract reference"), "{}", r.message);
        assert!(r.new_sheet.is_none());
    }

    #[test]
    fn common_range_only_puts_the_result_on_a_new_sheet_with_its_own_x() {
        let (mut p, book, _, ramp) = sample();
        let reference = add_reference(&mut p);
        let tool = curve_math(CurveOp::Divide, CurveOperand::Reference(reference), true);
        let r = p.analyze(&request(&[ramp], tool)).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(rs.columns[0].numeric().unwrap(), [2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(rs.columns[0].units, "s");
        assert_eq!(rs.columns[1].numeric().unwrap(), [2.0; 5], "x divided by x / 2");
        assert_eq!(rs.columns[1].units, "", "a ratio of two curves has no unit");
        // The data sheet is untouched.
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns.len(), 3);
    }

    #[test]
    fn constants_work_and_bad_operands_change_nothing() {
        let (mut p, _, flat, ramp) = sample();
        let r = p
            .analyze(&request(&[flat, ramp], curve_math(CurveOp::Multiply, CurveOperand::Constant(3.0), false)))
            .unwrap();
        assert_eq!(p.numeric(r.new_columns[0]).unwrap(), [6.0; 11]);
        assert_eq!(p.numeric(r.new_columns[1]).unwrap()[10], 30.0);
        assert_eq!(p.column(r.new_columns[1]).unwrap().units, "A", "scaling keeps the unit");
        let before = p.clone();
        for (tool, why) in [
            (curve_math(CurveOp::Divide, CurveOperand::Constant(0.0), false), "divide by 0"),
            (curve_math(CurveOp::Add, CurveOperand::Constant(f64::NAN), false), "NaN operand"),
        ] {
            assert!(matches!(p.analyze(&request(&[ramp], tool)), Err(AnalysisError::BadOption(_))), "{why}");
            assert_eq!(p, before, "{why}");
        }
        // A reference with one point, a column that is gone, and a reference that does not overlap.
        let one = {
            let b = p.add_workbook("one");
            p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap()
        };
        let tool = curve_math(CurveOp::Add, CurveOperand::Reference(one), false);
        assert!(matches!(p.analyze(&request(&[ramp], tool)), Err(AnalysisError::TooFewPoints(_))));
        let far = {
            let b = p.add_workbook("far");
            p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![50.0, 60.0])).unwrap();
            p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap()
        };
        let tool = curve_math(CurveOp::Add, CurveOperand::Reference(far), true);
        assert!(matches!(p.analyze(&request(&[ramp], tool)), Err(AnalysisError::BadOption(_))));
    }

    fn ranges(list: &[(f64, f64)], end_point_baseline: bool) -> Tool {
        Tool::IntegrateRanges(IntegrateRangesSpec {
            ranges: list.iter().map(|&(a, b)| RowRange::XBetween(a, b)).collect(),
            end_point_baseline,
        })
    }

    #[test]
    fn several_ranges_give_one_table_row_each_per_column() {
        let (mut p, book, flat, ramp) = sample();
        let r = p.analyze(&request(&[flat, ramp], ranges(&[(0.0, 4.0), (4.0, 10.0)], false))).unwrap();
        let t = r.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 4);
        assert_eq!(t.rows[0].0, "B: Current, Scan 1, range 1");
        assert_eq!(t.rows[3].0, "C: Current, Scan 2, range 2");
        let col = |name: &str| t.headers.iter().position(|h| h == name).unwrap();
        let area: Vec<f64> = t.rows.iter().map(|r| r.1[col("Area")]).collect();
        assert_eq!(area, [8.0, 12.0, 8.0, 42.0]);
        assert_eq!(t.rows[3].1[col("First row")], 5.0);
        assert_eq!(t.rows[3].1[col("Last row")], 11.0);
        assert_eq!(t.rows[3].1[col("Peak X")], 10.0);
        assert_eq!(t.rows[3].1[col("Peak height")], 10.0);
        assert_eq!(t.unit(0, col("Area")), "C");
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(rs.name, "Integrate1");
        assert_eq!(rs.columns[0].text().unwrap().len(), 4);
        assert!(r.new_columns.len() > 4 && p.workbook(book).unwrap().sheets[0].columns.len() == 3);
    }

    /// First row and Last row are worksheet rows, also when a missing value sits inside the range
    /// or at its start.
    #[test]
    fn range_rows_are_worksheet_rows_with_missing_values() {
        let (mut p, _, _, ramp) = sample();
        let col = p.column_mut(ramp).unwrap();
        let mut y: Vec<f64> = (0..=10).map(f64::from).collect();
        y[3] = f64::NAN; // row 4
        y[4] = f64::NAN; // row 5
        col.data = ColumnData::numeric(y);
        let rows = |tool: Tool, p: &mut Project| {
            let t = p.analyze(&request(&[ramp], tool)).unwrap().table.unwrap();
            (t.rows[0].1[0], t.rows[0].1[1])
        };
        let by_rows = |a, b| {
            Tool::IntegrateRanges(IntegrateRangesSpec { ranges: vec![RowRange::Rows(a, b)], end_point_baseline: false })
        };
        assert_eq!(rows(by_rows(1, 5), &mut p), (1.0, 3.0), "the range ends in missing values");
        assert_eq!(rows(by_rows(4, 8), &mut p), (6.0, 8.0), "the range starts in missing values");
        assert_eq!(rows(by_rows(1, 8), &mut p), (1.0, 8.0), "a gap in the middle changes nothing");
        assert_eq!(rows(ranges(&[(0.0, 7.0)], false), &mut p), (1.0, 8.0));
    }

    #[test]
    fn ranges_report_absolute_area_and_honour_the_baseline() {
        let (mut p, _, _, ramp) = sample();
        // y = x - 5 over 0..10 has equal positive and negative parts.
        let col = p.column_mut(ramp).unwrap();
        col.data = ColumnData::numeric((0..=10).map(|i| f64::from(i) - 5.0).collect());
        let r = p.analyze(&request(&[ramp], ranges(&[(0.0, 10.0), (5.0, 10.0)], false))).unwrap();
        let t = r.table.unwrap();
        let (area, abs) =
            (t.headers.iter().position(|h| h == "Area"), t.headers.iter().position(|h| h == "Absolute area"));
        let (area, abs) = (area.unwrap(), abs.unwrap());
        assert_eq!(t.rows[0].1[area], 0.0);
        assert_eq!(t.rows[0].1[abs], 25.0);
        assert_eq!(t.rows[1].1[area], 12.5);
        let r = p.analyze(&request(&[ramp], ranges(&[(0.0, 10.0)], true))).unwrap();
        assert_eq!(r.table.unwrap().rows[0].1[area], 0.0, "a straight line minus itself");
    }

    #[test]
    fn range_lists_respect_the_row_range_and_refuse_bad_input() {
        let (mut p, _, _, ramp) = sample();
        // Row ranges, cut to the request's rows 3..=9 (x = 2..8).
        let tool = Tool::IntegrateRanges(IntegrateRangesSpec {
            ranges: vec![RowRange::Rows(1, 5), RowRange::XBetween(6.0, 20.0)],
            end_point_baseline: false,
        });
        let r = p.analyze(&Request { columns: vec![ramp], rows: RowRange::Rows(3, 9), tool }).unwrap();
        let t = r.table.unwrap();
        assert_eq!(t.rows[0].1[4], 6.0, "rows 3 to 5, x = 2..4");
        assert_eq!(t.rows[1].1[4], 14.0, "x = 6..8, the end of the chosen rows");
        let before = p.clone();
        for (tool, why) in [
            (Tool::IntegrateRanges(IntegrateRangesSpec { ranges: vec![], end_point_baseline: false }), "no ranges"),
            (ranges(&[(20.0, 30.0)], false), "no row inside the interval"),
            (ranges(&[(4.2, 4.4)], false), "one point only"),
        ] {
            assert!(p.analyze(&request(&[ramp], tool)).is_err(), "{why}");
            assert_eq!(p, before, "{why}");
        }
    }

    /// Two Gauss peaks on a slope, in a sheet with X in s and Y in A.
    fn two_peaks() -> (Project, WindowId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("peaks");
        let x: Vec<f64> = (0..400).map(|i| i as f64 * 0.05).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&x| {
                0.1 + 0.01 * x
                    + (-(x - 6.0).powi(2) / (2.0 * 0.4 * 0.4)).exp()
                    + 0.6 * (-(x - 13.0).powi(2) / (2.0 * 0.5 * 0.5)).exp()
            })
            .collect();
        let xc = p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
        let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        p.column_mut(xc).unwrap().units = "s".into();
        p.column_mut(yc).unwrap().units = "A".into();
        (p, book, yc)
    }

    fn chain(integrate: bool, fit: Option<PeakFitSpec>) -> Tool {
        Tool::PeakChain(PeakChainSpec {
            chain: ChainOptions {
                baseline: ChainBaseline::AutoMinima(4),
                find: FindOptions { filter: PeakFilter::Height(0.3), ..FindOptions::default() },
                limits: LimitMode::Baseline,
            },
            integrate,
            fit,
        })
    }

    #[test]
    fn peak_chain_gives_a_row_per_peak_with_area_and_baseline_column() {
        let (mut p, book, y) = two_peaks();
        let r = p.analyze(&request(&[y], chain(true, None))).unwrap();
        let t = r.table.unwrap();
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.headers[3], "Area");
        let exact = 0.4 * (2.0 * std::f64::consts::PI).sqrt();
        assert!((t.rows[0].1[3] - exact).abs() < 0.02 * exact, "{}", t.rows[0].1[3]);
        assert!((t.rows[0].1[1] - 6.0).abs() < 1e-9);
        assert_eq!(t.unit(0, 3), "C");
        assert!((t.rows[0].1[4] + t.rows[1].1[4] - 100.0).abs() < 1e-9);
        let ws = &p.workbook(book).unwrap().sheets;
        assert_eq!(ws.len(), 2);
        assert!(ws[1].name.starts_with("PeakAnalysis"));
        assert_eq!(r.new_columns.len(), 1 + t.headers.len() + 1, "baseline column, label, and numbers");
        assert_eq!(ws[0].columns.len(), 3, "baseline column beside the source");
    }

    #[test]
    fn peak_chain_fit_reports_each_peak() {
        let (mut p, _, y) = two_peaks();
        let fit = PeakFitSpec { shape: PeakShape::Gauss, baseline: FitBaseline::Linear };
        let r = p.analyze(&request(&[y], chain(true, Some(fit)))).unwrap();
        let t = r.table.unwrap();
        let h = |n: &str| t.headers.iter().position(|h| h == n).unwrap();
        assert!((t.rows[0].1[h("Fit center")] - 6.0).abs() < 1e-4);
        assert!((t.rows[1].1[h("Fit center")] - 13.0).abs() < 1e-4);
        assert!((t.rows[0].1[h("Fit FWHM")] - 0.4 * 2.354_820).abs() < 1e-3);
        let exact = 0.4 * (2.0 * std::f64::consts::PI).sqrt();
        assert!((t.rows[0].1[h("Fit area")] - exact).abs() < 1e-3);
        assert!((t.rows[0].1[h("Fit area %")] + t.rows[1].1[h("Fit area %")] - 100.0).abs() < 1e-9);
        assert_eq!(r.notes.len(), 1);
        assert!(r.notes[0].contains("R^2"));
    }

    #[test]
    fn peak_chain_errors_change_nothing() {
        let (mut p, _, y) = two_peaks();
        let before = p.clone();
        let none = Tool::PeakChain(PeakChainSpec {
            chain: ChainOptions {
                find: FindOptions { filter: PeakFilter::Height(50.0), ..FindOptions::default() },
                ..ChainOptions::default()
            },
            integrate: true,
            fit: None,
        });
        assert!(matches!(p.analyze(&request(&[y], none)), Err(AnalysisError::BadOption(m)) if m.contains("no peaks")));
        let bad = Tool::PeakChain(PeakChainSpec {
            chain: ChainOptions {
                baseline: ChainBaseline::Anchors(vec![(1.0, 0.0)], InterpMethod::Linear),
                ..ChainOptions::default()
            },
            integrate: false,
            fit: None,
        });
        assert!(p.analyze(&request(&[y], bad)).is_err());
        assert_eq!(p, before);
    }

    #[test]
    fn peak_chain_survives_a_missing_end_point_and_refuses_unsorted_x() {
        let (mut p, book, y) = two_peaks();
        let xcol = p.workbook(book).unwrap().sheets[0].columns[0].id;
        let y0 = p.column_mut(y).unwrap().numeric_mut().unwrap();
        y0[0] = f64::NAN;
        y0[120] = f64::NAN; // inside the first peak
        let end = Tool::PeakChain(PeakChainSpec {
            chain: ChainOptions {
                baseline: ChainBaseline::EndPoints,
                find: FindOptions { filter: PeakFilter::Height(0.3), ..FindOptions::default() },
                ..ChainOptions::default()
            },
            integrate: true,
            fit: Some(PeakFitSpec { shape: PeakShape::Gauss, baseline: FitBaseline::Linear }),
        });
        let t = p.analyze(&request(&[y], end.clone())).unwrap().table.unwrap();
        assert_eq!(t.rows.len(), 2);
        for r in &t.rows {
            assert!(r.1.iter().all(|v| v.is_finite()), "{:?}", r.1);
        }
        assert!((t.rows[0].1[4] + t.rows[1].1[4] - 100.0).abs() < 1e-9);
        // Unsorted X: refused, nothing changes.
        let mut q = p.clone();
        q.column_mut(xcol).unwrap().numeric_mut().unwrap().swap(10, 300);
        let snapshot = q.clone();
        assert!(matches!(q.analyze(&request(&[y], end)), Err(AnalysisError::BadOption(m)) if m.contains("not sorted")));
        assert_eq!(q, snapshot);
    }

    #[test]
    fn a_big_result_table_is_built_in_linear_time() {
        let mut t = ResultTable::new(&["a"]);
        for i in 0..50_000 {
            t.push_row(format!("r{i}"), vec![i as f64], vec![if i == 40_000 { "s".into() } else { "A".into() }]);
        }
        assert_eq!(t.units, vec![String::new()], "mixed units give an empty shared unit");
        let mut u = ResultTable::new(&["a"]);
        u.push_row("r".into(), vec![1.0], vec!["A".into()]);
        u.push_row("r".into(), vec![2.0], vec!["A".into()]);
        assert_eq!(u.units, vec!["A".to_string()]);
    }
}
