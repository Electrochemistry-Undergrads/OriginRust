//! The windows of the Analysis menu (integrate, differentiate, smooth,
//! normalize, subtract baseline, interpolate, find peaks, statistics) and the
//! "Filter Rows" window of the Worksheet menu.
//!
//! One window type serves all analysis tools: it shows which columns the
//! tool will work on, the tool's own options, and the row range. OK runs the
//! tool as one undoable step.

use super::frame::{
    DialogButton, description_row, dialog_theme_row, ok_cancel_apply, preview_area, read_only_box, tree_node,
};
use eframe::egui::{self, DragValue, RichText};
use sheaf_core::analysis::{
    AnovaSpec, BinSize, CorrelationSpec, FreqOptions, FriedmanSpec, HistogramSpec, KruskalWallisSpec, KsTwoSampleSpec,
    MannWhitneySpec, MeansComparison, MedianTestSpec, NormalitySpec, OutlierSpec, RowStatsSpec, SignTestSpec,
    SignedRankKind, SignedRankSpec, StatisticsSpec, TTestKind, TTestSpec, Tail, TwoWayAnovaSpec, TwoWayLayout,
};
use sheaf_core::analysis::{
    AreaType, BaselineSpec, Boundary, CurveMathSpec, CurveOp, CurveOperand, DiffOptions, IntegrateRangesSpec,
    IntegrateSpec, InterpMethod, InterpSpec, LocalSpan, NormalizeMethod, OutputTarget, PeakDirection, PeakFilter,
    PeakOptions, Report, Request, RowRange, SmoothMethod, SmoothOptions, SplineBoundary, Tool,
};
use sheaf_core::analysis::{
    ChainBaseline, ChainOptions, FindMethod, FindOptions, FitBaseline, LimitMode, PeakChainSpec, PeakFitSpec, PeakShape,
};
use sheaf_core::column::{Designation, RecalcMode};
use sheaf_core::{ColumnId, Document, Project, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisKind {
    Integrate,
    /// Integrate several X ranges into one table.
    IntegrateRanges,
    Differentiate,
    Smooth,
    Normalize,
    Baseline,
    /// Subtract a reference curve.
    SubtractRef,
    /// Simple Math: add, subtract, multiply, divide or power with a constant or a reference.
    CurveMath,
    Interpolate,
    Peaks,
    /// Baseline, peak search, per-peak integration and an optional multi-peak fit.
    PeakChain,
    Statistics,
    /// The Statistics menu: t-tests, one-way ANOVA, histogram and normality tests.
    TTest,
    Anova,
    Histogram,
    Normality,
    /// Statistics > ANOVA > Two-Way ANOVA.
    Anova2,
    /// Statistics > Nonparametric Tests and Descriptive Statistics > Correlation Coefficient.
    MannWhitney,
    SignedRank,
    /// The same window, starting with the paired test.
    SignedRankPaired,
    KruskalWallis,
    Correlation,
    /// Statistics > Descriptive Statistics > Statistics on Rows.
    RowStatistics,
    /// Statistics > Descriptive Statistics > Grubbs Test and Dixon's Q-test.
    Grubbs,
    Dixon,
    /// Statistics > Nonparametric Tests: more tests.
    SignTest,
    KsTwoSample,
    MedianTest,
    Friedman,
    /// Analysis > Mathematics: Average Multiple Curves, Intersect and Polygon Area.
    AverageCurves,
    Intersect,
    PolygonArea,
    /// Data Manipulation (options in `manip_options.rs`) and Mathematics > Median Filter.
    SubtractLine,
    ReduceX,
    TranslateX,
    TranslateY,
    MedianFilter,
}

impl AnalysisKind {
    pub const ALL: [AnalysisKind; 12] = [
        AnalysisKind::Integrate,
        AnalysisKind::IntegrateRanges,
        AnalysisKind::Differentiate,
        AnalysisKind::Smooth,
        AnalysisKind::Normalize,
        AnalysisKind::Baseline,
        AnalysisKind::SubtractRef,
        AnalysisKind::CurveMath,
        AnalysisKind::Interpolate,
        AnalysisKind::Peaks,
        AnalysisKind::PeakChain,
        AnalysisKind::Statistics,
    ];

    /// The text of the menu entry.
    pub fn menu_label(self) -> &'static str {
        match self {
            AnalysisKind::Integrate => "Integrate...",
            AnalysisKind::IntegrateRanges => "Integrate Ranges...",
            AnalysisKind::Differentiate => "Differentiate...",
            AnalysisKind::Smooth => "Smooth...",
            AnalysisKind::Normalize => "Normalize...",
            AnalysisKind::Baseline => "Subtract Baseline...",
            AnalysisKind::SubtractRef => "Subtract Reference Data...",
            AnalysisKind::CurveMath => "Simple Math...",
            AnalysisKind::Interpolate => "Interpolate...",
            AnalysisKind::Peaks => "Find Peaks...",
            AnalysisKind::PeakChain => "Peak Analyzer...",
            AnalysisKind::Statistics => "Statistics on Columns...",
            AnalysisKind::TTest => "t-Test...",
            AnalysisKind::Anova => "One-Way ANOVA...",
            AnalysisKind::Histogram => "Histogram...",
            AnalysisKind::Normality => "Normality Test...",
            AnalysisKind::Anova2 => "Two-Way ANOVA...",
            AnalysisKind::MannWhitney => "Mann-Whitney Test...",
            AnalysisKind::SignedRank => "One-Sample Wilcoxon Signed Rank Test...",
            AnalysisKind::SignedRankPaired => "Paired Sample Wilcoxon Signed Rank Test...",
            AnalysisKind::KruskalWallis => "Kruskal-Wallis ANOVA...",
            AnalysisKind::Correlation => "Correlation Coefficient...",
            AnalysisKind::RowStatistics => "Statistics on Rows...",
            AnalysisKind::Grubbs => "Grubbs Test...",
            AnalysisKind::Dixon => "Dixon's Q-test...",
            AnalysisKind::SignTest => "Paired Sample Sign Test...",
            AnalysisKind::KsTwoSample => "Two Sample Kolmogorov-Smirnov Test...",
            AnalysisKind::MedianTest => "Mood's Median Test...",
            AnalysisKind::Friedman => "Friedman ANOVA...",
            AnalysisKind::AverageCurves => "Average Multiple Curves...",
            AnalysisKind::Intersect => "Intersect...",
            AnalysisKind::PolygonArea => "Polygon Area...",
            AnalysisKind::SubtractLine => "Subtract Straight Line...",
            AnalysisKind::ReduceX => "Reduce to Evenly Spaced X...",
            AnalysisKind::TranslateX => "Curve Translate X...",
            AnalysisKind::TranslateY => "Curve Translate Y...",
            AnalysisKind::MedianFilter => "Median Filter...",
        }
    }

    fn title(self) -> &'static str {
        self.menu_label().trim_end_matches("...")
    }

    /// The name used on the command line with `--show analysis:NAME`.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "integrate" => AnalysisKind::Integrate,
            "integrate-ranges" => AnalysisKind::IntegrateRanges,
            "differentiate" => AnalysisKind::Differentiate,
            "smooth" => AnalysisKind::Smooth,
            "normalize" => AnalysisKind::Normalize,
            "baseline" => AnalysisKind::Baseline,
            "subtract-ref" => AnalysisKind::SubtractRef,
            "math" => AnalysisKind::CurveMath,
            "interpolate" => AnalysisKind::Interpolate,
            "peaks" => AnalysisKind::Peaks,
            "peak-chain" => AnalysisKind::PeakChain,
            "statistics" => AnalysisKind::Statistics,
            "ttest" => AnalysisKind::TTest,
            "anova" => AnalysisKind::Anova,
            "histogram" => AnalysisKind::Histogram,
            "normality" => AnalysisKind::Normality,
            "anova2" => AnalysisKind::Anova2,
            "mannwhitney" => AnalysisKind::MannWhitney,
            "signrank" => AnalysisKind::SignedRank,
            "signrank-paired" => AnalysisKind::SignedRankPaired,
            "kruskal" => AnalysisKind::KruskalWallis,
            "corr" => AnalysisKind::Correlation,
            "rowstats" => AnalysisKind::RowStatistics,
            "grubbs" => AnalysisKind::Grubbs,
            "dixon" => AnalysisKind::Dixon,
            "sign" => AnalysisKind::SignTest,
            "ks2" => AnalysisKind::KsTwoSample,
            "mediantest" => AnalysisKind::MedianTest,
            "friedman" => AnalysisKind::Friedman,
            "average-curves" => AnalysisKind::AverageCurves,
            "intersect" => AnalysisKind::Intersect,
            "polygon-area" => AnalysisKind::PolygonArea,
            "subtract-line" => AnalysisKind::SubtractLine,
            "reduce-x" => AnalysisKind::ReduceX,
            "translate-x" => AnalysisKind::TranslateX,
            "translate-y" => AnalysisKind::TranslateY,
            "median-filter" => AnalysisKind::MedianFilter,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowMode {
    All,
    Rows,
    XRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmoothKind {
    SavitzkyGolay,
    Average,
    Weighted,
    Percentile,
    Fft,
    Lowess,
    Loess,
    Binomial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaselineKind {
    Minimum,
    Constant,
    EndPoints,
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainBaselineKind {
    None,
    EndPoints,
    Minima,
}

const FIND_METHODS: [(&str, FindMethod); 4] = [
    ("Local maximum", FindMethod::LocalMax),
    ("Window search", FindMethod::Window),
    ("First derivative", FindMethod::FirstDerivative),
    ("Second derivative", FindMethod::SecondDerivative),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeakKeep {
    /// At least 20 % of the largest magnitude.
    Default,
    Height,
    Count,
    All,
}

const NORMALIZE_CHOICES: [(&str, NormalizeMethod); 12] = [
    ("Divide by the maximum", NormalizeMethod::Max),
    ("To the range 0 to 1", NormalizeMethod::ZeroToOne),
    ("To the range 0 to 100", NormalizeMethod::ZeroToHundred),
    ("Z scores (mean 0, standard deviation 1)", NormalizeMethod::ZScore),
    ("Divide by the mean", NormalizeMethod::Mean),
    ("Divide by the median", NormalizeMethod::Median),
    ("Divide by the sum", NormalizeMethod::Sum),
    ("Divide by the minimum", NormalizeMethod::Min),
    ("Divide by the standard deviation", NormalizeMethod::Sd),
    ("Divide by the norm", NormalizeMethod::Norm),
    ("Divide by the mode", NormalizeMethod::Mode),
    ("Divide by a value", NormalizeMethod::Divide(1.0)),
];

/// The options of the Statistics menu windows.
struct StatsOptions {
    /// 0 one-sample, 1 two-sample, 2 paired.
    ttest: usize,
    mean: f64,
    difference: f64,
    equal_variance: bool,
    tail: Tail,
    alpha: f64,
    /// Index into `COMPARISONS`.
    comparison: usize,
    /// 0 automatic, 1 bin size, 2 number of bins.
    bins: usize,
    bin_size: f64,
    bin_count: usize,
    own_range: bool,
    bin_min: f64,
    bin_max: f64,
    graph: bool,
    /// Two-way ANOVA: raw layout (one column per cell) instead of indexed.
    two_way_raw: bool,
    two_way_interaction: bool,
    /// Indexes into the sheet's columns: factor A, factor B and the data.
    two_way_columns: [usize; 3],
    /// Raw layout: the levels of factor A and B of each chosen column.
    two_way_levels: Vec<(String, String)>,
    /// Mann-Whitney: exact p-value.
    exact: bool,
    /// Signed rank: paired instead of one-sample, and the hypothesized median.
    paired: bool,
    median: f64,
    /// Signed rank: decide with the exact p-value where there is one.
    decide_exact: bool,
    /// Correlation: Pearson, Spearman, Kendall, and listwise deletion.
    pearson: bool,
    spearman: bool,
    kendall: bool,
    listwise: bool,
    /// Statistics on Columns: which quantities to write.
    desc: StatisticsSpec,
    /// Statistics on Rows: which quantities to add.
    rows: RowStatsSpec,
}

impl Default for StatsOptions {
    fn default() -> Self {
        StatsOptions {
            ttest: 1,
            mean: 0.0,
            difference: 0.0,
            equal_variance: true,
            tail: Tail::Two,
            alpha: 0.05,
            comparison: 0,
            bins: 0,
            bin_size: 1.0,
            bin_count: 10,
            own_range: false,
            bin_min: 0.0,
            bin_max: 10.0,
            graph: true,
            two_way_raw: false,
            two_way_interaction: true,
            two_way_columns: [0, 1, 2],
            two_way_levels: Vec::new(),
            exact: false,
            paired: false,
            median: 0.0,
            decide_exact: true,
            pearson: true,
            spearman: false,
            kendall: false,
            listwise: false,
            desc: StatisticsSpec::default(),
            rows: RowStatsSpec::default(),
        }
    }
}

const COMPARISONS: [(&str, Option<MeansComparison>); 6] = [
    ("Tukey", Some(MeansComparison::Tukey)),
    ("Bonferroni", Some(MeansComparison::Bonferroni)),
    ("Sidak", Some(MeansComparison::Sidak)),
    ("Fisher LSD", Some(MeansComparison::FisherLsd)),
    ("Scheffe", Some(MeansComparison::Scheffe)),
    ("None", None),
];

pub enum AnalysisOutcome {
    Open,
    Closed,
    /// OK: the tool ran; the window closes.
    Done(Box<Report>),
    /// Apply: the tool ran and the window stays open.
    Applied(Box<Report>),
    /// Auto Apply: an option changed and the tool ran again; the window stays open.
    AutoApplied(Box<Report>),
}

/// Where the one-value-per-row results go (the Output node).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputChoice {
    NextToInput,
    NewSheet,
    Existing,
}

/// Undo steps made by one window share a token, so one Undo reverts all its Apply runs.
fn next_undo_token() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub struct AnalysisDialog {
    kind: AnalysisKind,
    columns: Vec<ColumnId>,
    /// "B, C, D" for the first line of the window.
    column_names: String,
    rows_in_sheet: usize,
    row_mode: RowMode,
    from: usize,
    to: usize,
    x_from: f64,
    x_to: f64,
    integrate: IntegrateSpec,
    diff: DiffOptions,
    smooth_kind: SmoothKind,
    smooth_points: usize,
    smooth_poly: usize,
    smooth_percent: f64,
    /// Lowess and Loess: share of the points in each local fit.
    smooth_span: f64,
    /// Binomial filter: passes of the 1 2 1 / 4 filter.
    smooth_order: usize,
    boundary: Boundary,
    normalize: usize,
    divisor: f64,
    baseline: BaselineKind,
    level: f64,
    line: [f64; 4],
    interp: InterpSpec,
    interp_own_range: bool,
    /// Interpolate onto the values of this column (index into `sheet_columns`) instead of a grid.
    interp_at: Option<usize>,
    /// The column that `interp_at` starts at when the X values are taken from a column.
    interp_at_default: usize,
    peaks: PeakOptions,
    peak_keep: PeakKeep,
    peak_height: f64,
    peak_count: usize,
    chain_baseline: ChainBaselineKind,
    chain_minima: usize,
    find_method: usize,
    chain_half_height: bool,
    chain_integrate: bool,
    chain_fit: Option<PeakShape>,
    chain_fit_linear: bool,
    /// X intervals of "Integrate Ranges".
    ranges: Vec<(f64, f64)>,
    curve_op: CurveOp,
    /// Simple Math: use a reference curve (otherwise the constant).
    use_reference: bool,
    constant: f64,
    /// Columns that can serve as reference: every other numeric Y column of the project, with a
    /// description ("Book2 / Sheet1: B  up").
    reference_choices: Vec<(ColumnId, String)>,
    reference: usize,
    common_only: bool,
    stats: StatsOptions,
    curves: super::math_curves::CurveOptions,
    /// Options of the Data Manipulation windows and the median filter.
    manip: super::manip_options::ManipOptions,
    /// Every column of the sheet with a description, for the factor and data choices of the two-way ANOVA
    /// (the factor columns may hold text).
    sheet_columns: Vec<(ColumnId, String)>,
    problem: Option<String>,
    /// The "Select Input Columns" window: one tick per column of the sheet while it is open.
    picker: Option<Vec<bool>>,
    /// The preview pane is shown (the "<<" button hides it).
    show_preview: bool,
    auto_preview: bool,
    preview: Option<Result<PreviewData, String>>,
    /// The request the preview shows, so that Auto Preview computes again only after a change.
    preview_request: Option<Request>,
    /// The Recalculate choice (None, Manual, Auto); Manual is the default of Origin.
    recalc: RecalcMode,
    output_choice: OutputChoice,
    /// For [`OutputChoice::Existing`]: the target column (index into `sheet_columns`) of each input.
    output_existing: Vec<usize>,
    auto_apply: bool,
    /// The columns that hold the result of this window's last run: Apply and Auto Apply replace them.
    applied: Option<Vec<ColumnId>>,
    /// What the last run used, so that Auto Apply runs again only after a change.
    applied_key: Option<(Request, OutputTarget, RecalcMode)>,
    applied_report: Option<Report>,
    /// The options of the last Auto Apply run that failed, so that it is not retried every frame.
    failed_key: Option<(Request, OutputTarget, RecalcMode)>,
    undo_token: u64,
}

const CURVE_OPS: [(&str, CurveOp); 5] = [
    ("Add", CurveOp::Add),
    ("Subtract", CurveOp::Subtract),
    ("Multiply", CurveOp::Multiply),
    ("Divide", CurveOp::Divide),
    ("Power", CurveOp::Power),
];

/// Every numeric Y column outside `exclude`, for the reference list.
fn reference_choices(project: &Project, exclude: &[ColumnId]) -> Vec<(ColumnId, String)> {
    let mut out = Vec::new();
    for book in project.workbooks() {
        for sheet in &book.sheets {
            for c in &sheet.columns {
                if c.designation == Designation::Y && c.data.is_numeric() && !exclude.contains(&c.id) {
                    let detail = if c.long_name.is_empty() { String::new() } else { format!("  {}", c.long_name) };
                    out.push((c.id, format!("{} / {}: {}{detail}", book.name, sheet.name, c.short_name)));
                }
            }
        }
    }
    out
}

impl AnalysisDialog {
    /// The columns the window runs on (for tests).
    #[cfg(test)]
    pub fn columns(&self) -> &[ColumnId] {
        &self.columns
    }

    /// A window for running `kind` on `columns`. `None` when a column no longer exists.
    /// Chooses the interpolation method (the Mathematics menu's Linear, Spline and B-Spline items).
    pub fn set_interp_method(&mut self, method: InterpMethod) {
        self.interp.method = method;
    }

    /// Mathematics > Interpolate/Extrapolate Y from X: the X values come from a column, at first the
    /// first column of the sheet that is neither an input nor an input's X column.
    pub fn set_interp_from_column(&mut self) {
        self.interp_at = Some(self.interp_at_default);
    }

    #[cfg(test)]
    pub fn interp_method(&self) -> InterpMethod {
        self.interp.method
    }

    pub fn new(project: &Project, kind: AnalysisKind, columns: Vec<ColumnId>) -> Option<Self> {
        let (_, sheet, _) = project.locate(*columns.first()?)?;
        let names: Vec<&str> =
            columns.iter().filter_map(|id| sheet.column(*id)).map(|c| c.short_name.as_str()).collect();
        if names.len() != columns.len() {
            return None;
        }
        let rows_in_sheet = sheet.row_count().max(1);
        // Start the X limits at the X range of the first column's X data.
        let x = sheet.column_index(columns[0]).and_then(|i| sheet.x_for(i)).and_then(|c| c.numeric());
        let (x_lo, x_hi) = match x {
            Some(v) => {
                let finite = v.iter().copied().filter(|v| v.is_finite());
                (finite.clone().fold(f64::INFINITY, f64::min), finite.fold(f64::NEG_INFINITY, f64::max))
            }
            None => (1.0, rows_in_sheet as f64),
        };
        let (x_lo, x_hi) = if x_lo <= x_hi { (x_lo, x_hi) } else { (0.0, 1.0) };
        let reference_choices = reference_choices(project, &columns);
        let sheet_columns: Vec<(ColumnId, String)> = sheet
            .columns
            .iter()
            .map(|c| {
                let detail = if c.long_name.is_empty() { String::new() } else { format!("  {}", c.long_name) };
                (c.id, format!("{}{detail}", c.short_name))
            })
            .collect();
        // Interpolate/Extrapolate Y from X: the X values come at first from the first column that is
        // neither an input nor the X column of an input (that would give the data back).
        let input_xs: Vec<ColumnId> = columns
            .iter()
            .filter_map(|id| sheet.column_index(*id).and_then(|i| sheet.x_for(i)).map(|c| c.id))
            .collect();
        let interp_at_default = sheet
            .columns
            .iter()
            .position(|c| !columns.contains(&c.id) && !input_xs.contains(&c.id))
            .or_else(|| sheet.columns.iter().position(|c| !columns.contains(&c.id)))
            .unwrap_or(0);
        // The two-way ANOVA starts with the chosen columns as factor A, factor B and data, in this order.
        let mut stats = StatsOptions { paired: kind == AnalysisKind::SignedRankPaired, ..StatsOptions::default() };
        if kind == AnalysisKind::Anova2 {
            let position = |id: &ColumnId| sheet_columns.iter().position(|(c, _)| c == id);
            let chosen: Vec<usize> = columns.iter().filter_map(position).collect();
            stats.two_way_columns = if chosen.len() >= 3 {
                [chosen[0], chosen[1], chosen[2]]
            } else {
                let last = sheet_columns.len().saturating_sub(1);
                [0, 1.min(last), 2.min(last)]
            };
            stats.two_way_levels = vec![(String::new(), String::new()); columns.len()];
        }
        Some(AnalysisDialog {
            kind,
            column_names: names.join(", "),
            ranges: vec![(x_lo, x_hi)],
            curve_op: CurveOp::Add,
            use_reference: kind == AnalysisKind::SubtractRef || !reference_choices.is_empty(),
            constant: 1.0,
            reference_choices,
            reference: 0,
            common_only: false,
            columns,
            rows_in_sheet,
            row_mode: RowMode::All,
            from: 1,
            to: rows_in_sheet,
            x_from: x_lo,
            x_to: x_hi,
            integrate: IntegrateSpec::default(),
            diff: DiffOptions::default(),
            smooth_kind: SmoothKind::SavitzkyGolay,
            smooth_points: 5,
            smooth_poly: 2,
            smooth_percent: 50.0,
            smooth_span: 0.1,
            smooth_order: 1,
            boundary: Boundary::None,
            normalize: 0,
            divisor: 1.0,
            baseline: BaselineKind::Minimum,
            level: 0.0,
            line: [x_lo, 0.0, x_hi, 0.0],
            interp: InterpSpec { range: Some((x_lo, x_hi)), ..InterpSpec::default() },
            interp_own_range: false,
            interp_at: None,
            interp_at_default,
            peaks: PeakOptions::default(),
            peak_keep: PeakKeep::Default,
            peak_height: 0.0,
            peak_count: 1,
            chain_baseline: ChainBaselineKind::Minima,
            chain_minima: 5,
            find_method: 0,
            chain_half_height: false,
            chain_integrate: true,
            chain_fit: None,
            chain_fit_linear: true,
            stats,
            curves: Default::default(),
            manip: super::manip_options::ManipOptions::new(x_lo, x_hi),
            sheet_columns,
            problem: None,
            picker: None,
            show_preview: true,
            auto_preview: false,
            preview: None,
            preview_request: None,
            recalc: RecalcMode::Manual,
            output_choice: OutputChoice::NextToInput,
            output_existing: Vec::new(),
            auto_apply: false,
            applied: None,
            applied_key: None,
            applied_report: None,
            failed_key: None,
            undo_token: next_undo_token(),
        })
    }

    fn tool(&self) -> Result<Tool, String> {
        Ok(match self.kind {
            AnalysisKind::Integrate => Tool::Integrate(self.integrate),
            AnalysisKind::IntegrateRanges => Tool::IntegrateRanges(IntegrateRangesSpec {
                ranges: self.ranges.iter().map(|&(a, b)| RowRange::XBetween(a, b)).collect(),
                end_point_baseline: self.integrate.end_point_baseline,
            }),
            AnalysisKind::SubtractRef | AnalysisKind::CurveMath => {
                let reference = || {
                    self.reference_choices.get(self.reference).map(|(id, _)| CurveOperand::Reference(*id)).ok_or_else(
                        || "there is no other column with numbers to use as the reference curve".to_string(),
                    )
                };
                let (op, operand) = if self.kind == AnalysisKind::SubtractRef {
                    (CurveOp::Subtract, reference()?)
                } else if self.use_reference {
                    (self.curve_op, reference()?)
                } else {
                    (self.curve_op, CurveOperand::Constant(self.constant))
                };
                let common_only = self.common_only && matches!(operand, CurveOperand::Reference(_));
                Tool::CurveMath(CurveMathSpec { op, operand, common_only })
            }
            AnalysisKind::SubtractLine
            | AnalysisKind::ReduceX
            | AnalysisKind::TranslateX
            | AnalysisKind::TranslateY
            | AnalysisKind::MedianFilter => self.manip.tool(self.kind),
            AnalysisKind::Differentiate => Tool::Differentiate(self.diff),
            AnalysisKind::AverageCurves => self.curves.average_tool(),
            AnalysisKind::Intersect => self.curves.intersect_tool(),
            AnalysisKind::PolygonArea => Tool::PolygonArea,
            AnalysisKind::Smooth => Tool::Smooth(SmoothOptions {
                method: match self.smooth_kind {
                    SmoothKind::SavitzkyGolay => SmoothMethod::SavitzkyGolay { poly_order: self.smooth_poly },
                    SmoothKind::Average => SmoothMethod::AdjacentAveraging { weighted: false },
                    SmoothKind::Weighted => SmoothMethod::AdjacentAveraging { weighted: true },
                    SmoothKind::Percentile => SmoothMethod::Percentile { percent: self.smooth_percent },
                    SmoothKind::Fft => SmoothMethod::FftFilter,
                    SmoothKind::Lowess => SmoothMethod::Lowess { span: LocalSpan::Proportion(self.smooth_span) },
                    SmoothKind::Loess => SmoothMethod::Loess { span: LocalSpan::Proportion(self.smooth_span) },
                    SmoothKind::Binomial => SmoothMethod::Binomial { order: self.smooth_order },
                },
                npts: self.smooth_points,
                boundary: self.boundary,
            }),
            AnalysisKind::Normalize => Tool::Normalize(match NORMALIZE_CHOICES[self.normalize].1 {
                NormalizeMethod::Divide(_) => NormalizeMethod::Divide(self.divisor),
                other => other,
            }),
            AnalysisKind::Baseline => Tool::SubtractBaseline(match self.baseline {
                BaselineKind::Minimum => BaselineSpec::Minimum,
                BaselineKind::Constant => BaselineSpec::Constant(self.level),
                BaselineKind::EndPoints => BaselineSpec::EndPoints,
                BaselineKind::Line => BaselineSpec::Line((self.line[0], self.line[1]), (self.line[2], self.line[3])),
            }),
            AnalysisKind::Interpolate => Tool::Interpolate(InterpSpec {
                range: if self.interp_own_range { self.interp.range } else { None },
                at: self.interp_at.and_then(|i| self.sheet_columns.get(i)).map(|(id, _)| *id),
                ..self.interp
            }),
            AnalysisKind::Peaks => Tool::FindPeaks(PeakOptions {
                filter: match self.peak_keep {
                    PeakKeep::Default => PeakFilter::Height(0.0),
                    PeakKeep::Height => PeakFilter::Height(self.peak_height),
                    PeakKeep::Count => PeakFilter::Number(self.peak_count),
                    PeakKeep::All => PeakFilter::None,
                },
                ..self.peaks
            }),
            AnalysisKind::PeakChain => Tool::PeakChain(PeakChainSpec {
                chain: ChainOptions {
                    baseline: match self.chain_baseline {
                        ChainBaselineKind::None => ChainBaseline::None,
                        ChainBaselineKind::EndPoints => ChainBaseline::EndPoints,
                        ChainBaselineKind::Minima => ChainBaseline::AutoMinima(self.chain_minima),
                    },
                    find: FindOptions {
                        method: FIND_METHODS[self.find_method].1,
                        npts: self.peaks.npts,
                        direction: self.peaks.direction,
                        filter: match self.peak_keep {
                            PeakKeep::Default => PeakFilter::Height(0.0),
                            PeakKeep::Height => PeakFilter::Height(self.peak_height),
                            PeakKeep::Count => PeakFilter::Number(self.peak_count),
                            PeakKeep::All => PeakFilter::None,
                        },
                        ..FindOptions::default()
                    },
                    limits: if self.chain_half_height { LimitMode::HalfHeight } else { LimitMode::Baseline },
                },
                integrate: self.chain_integrate,
                fit: self.chain_fit.map(|shape| PeakFitSpec {
                    shape,
                    baseline: if self.chain_fit_linear { FitBaseline::Linear } else { FitBaseline::Constant },
                }),
            }),
            AnalysisKind::Statistics => Tool::Statistics(self.stats.desc),
            AnalysisKind::TTest => Tool::TTest(TTestSpec {
                kind: match self.stats.ttest {
                    0 => TTestKind::OneSample { mean: self.stats.mean },
                    1 => TTestKind::TwoSample {
                        equal_variance: self.stats.equal_variance,
                        difference: self.stats.difference,
                    },
                    _ => TTestKind::Paired { difference: self.stats.difference },
                },
                tail: self.stats.tail,
                alpha: self.stats.alpha,
            }),
            AnalysisKind::Anova => {
                Tool::Anova(AnovaSpec { alpha: self.stats.alpha, comparison: COMPARISONS[self.stats.comparison].1 })
            }
            AnalysisKind::Histogram => Tool::Histogram(HistogramSpec {
                freq: FreqOptions {
                    min: self.stats.own_range.then_some(self.stats.bin_min),
                    max: self.stats.own_range.then_some(self.stats.bin_max),
                    size: match self.stats.bins {
                        1 => BinSize::Width(self.stats.bin_size),
                        2 => BinSize::Count(self.stats.bin_count),
                        _ => BinSize::Auto,
                    },
                    below: false,
                    above: false,
                },
                graph: self.stats.graph,
            }),
            AnalysisKind::Normality => Tool::Normality(NormalitySpec { alpha: self.stats.alpha }),
            AnalysisKind::Anova2 => {
                let layout = if self.stats.two_way_raw {
                    if let Some(i) =
                        self.stats.two_way_levels.iter().position(|(a, b)| a.trim().is_empty() || b.trim().is_empty())
                    {
                        let name = self.column_names.split(", ").nth(i).unwrap_or("");
                        return Err(format!("give the level of factor A and of factor B for column {name}"));
                    }
                    TwoWayLayout::Raw { levels: self.stats.two_way_levels.clone() }
                } else {
                    TwoWayLayout::Indexed
                };
                Tool::TwoWayAnova(TwoWayAnovaSpec {
                    layout,
                    interaction: self.stats.two_way_interaction,
                    comparison: COMPARISONS[self.stats.comparison].1,
                    alpha: self.stats.alpha,
                })
            }
            AnalysisKind::MannWhitney => Tool::MannWhitney(MannWhitneySpec {
                tail: self.stats.tail,
                exact: self.stats.exact,
                alpha: self.stats.alpha,
            }),
            AnalysisKind::SignedRank | AnalysisKind::SignedRankPaired => Tool::SignedRank(SignedRankSpec {
                kind: if self.stats.paired {
                    SignedRankKind::Paired
                } else {
                    SignedRankKind::OneSample { median: self.stats.median }
                },
                tail: self.stats.tail,
                exact: self.stats.decide_exact,
                alpha: self.stats.alpha,
            }),
            AnalysisKind::KruskalWallis => Tool::KruskalWallis(KruskalWallisSpec { alpha: self.stats.alpha }),
            AnalysisKind::RowStatistics => Tool::RowStatistics(self.stats.rows),
            AnalysisKind::Grubbs => Tool::Grubbs(OutlierSpec { alpha: self.stats.alpha }),
            AnalysisKind::Dixon => Tool::Dixon(OutlierSpec { alpha: self.stats.alpha }),
            AnalysisKind::SignTest => Tool::SignTest(SignTestSpec { tail: self.stats.tail, alpha: self.stats.alpha }),
            AnalysisKind::KsTwoSample => {
                Tool::KsTwoSample(KsTwoSampleSpec { tail: self.stats.tail, alpha: self.stats.alpha })
            }
            AnalysisKind::MedianTest => Tool::MedianTest(MedianTestSpec { alpha: self.stats.alpha }),
            AnalysisKind::Friedman => Tool::Friedman(FriedmanSpec { alpha: self.stats.alpha }),
            AnalysisKind::Correlation => Tool::Correlation(CorrelationSpec {
                pearson: self.stats.pearson,
                spearman: self.stats.spearman,
                kendall: self.stats.kendall,
                listwise: self.stats.listwise,
                alpha: self.stats.alpha,
            }),
        })
    }

    fn request(&self) -> Result<Request, String> {
        let rows = match self.row_mode {
            RowMode::All => RowRange::All,
            RowMode::Rows => RowRange::Rows(self.from, self.to),
            RowMode::XRange => RowRange::XBetween(self.x_from, self.x_to),
        };
        let tool = self.tool()?;
        let columns = if self.kind == AnalysisKind::Anova2 && !self.stats.two_way_raw {
            let [a, b, y] = self.stats.two_way_columns;
            let pick = |i: usize| self.sheet_columns.get(i).map(|(id, _)| *id);
            match (pick(a), pick(b), pick(y)) {
                (Some(a), Some(b), Some(y)) => vec![a, b, y],
                _ => return Err("the sheet needs three columns: factor A, factor B and the data".to_string()),
            }
        } else {
            self.columns.clone()
        };
        Ok(Request { columns, rows, tool })
    }

    /// The reference list and the options that belong to it.
    fn reference_options(&mut self, ui: &mut egui::Ui) {
        let text = match self.reference_choices.get(self.reference) {
            Some((_, label)) => label.as_str(),
            None => "(no other column with numbers)",
        };
        ui.horizontal(|ui| {
            ui.label("Reference");
            let mut chosen = self.reference;
            egui::ComboBox::from_id_salt("reference_column").selected_text(text).width(300.0).show_ui(ui, |ui| {
                for (i, (_, label)) in self.reference_choices.iter().enumerate() {
                    ui.selectable_value(&mut chosen, i, label);
                }
            });
            self.reference = chosen;
        });
        ui.checkbox(&mut self.common_only, "Common X range only").on_hover_text(
            "Only where data and reference both have X values; the result goes to a new sheet. Otherwise the \
             reference is continued as a straight line outside its range.",
        );
    }

    fn options(&mut self, ui: &mut egui::Ui) {
        match self.kind {
            AnalysisKind::IntegrateRanges => {
                let mut remove = None;
                for (i, (from, to)) in self.ranges.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Range {}: X from", i + 1));
                        ui.add(DragValue::new(from).speed(0.01));
                        ui.label("to");
                        ui.add(DragValue::new(to).speed(0.01));
                        if ui.small_button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove.filter(|_| self.ranges.len() > 1) {
                    self.ranges.remove(i);
                }
                if ui.button("Add range").clicked() {
                    let last = self.ranges.last().copied().unwrap_or((0.0, 1.0));
                    self.ranges.push(last);
                }
                ui.checkbox(
                    &mut self.integrate.end_point_baseline,
                    "Subtract the line through the first and last point of each range",
                );
            }
            AnalysisKind::SubtractLine
            | AnalysisKind::ReduceX
            | AnalysisKind::TranslateX
            | AnalysisKind::TranslateY
            | AnalysisKind::MedianFilter => self.manip.options(ui, self.kind),
            AnalysisKind::SubtractRef => {
                self.reference_options(ui);
                ui.weak("The reference is interpolated onto the X values of the data.");
            }
            AnalysisKind::CurveMath => {
                ui.horizontal(|ui| {
                    ui.label("Operator");
                    egui::ComboBox::from_id_salt("curve_op")
                        .selected_text(CURVE_OPS.iter().find(|o| o.1 == self.curve_op).map_or("", |o| o.0))
                        .show_ui(ui, |ui| {
                            for (label, op) in CURVE_OPS {
                                ui.selectable_value(&mut self.curve_op, op, label);
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Operand");
                    ui.radio_value(&mut self.use_reference, false, "Constant");
                    ui.add_enabled(!self.use_reference, DragValue::new(&mut self.constant).speed(0.1));
                });
                ui.radio_value(&mut self.use_reference, true, "Reference data");
                ui.add_enabled_ui(self.use_reference, |ui| self.reference_options(ui));
            }
            AnalysisKind::Integrate => {
                ui.horizontal(|ui| {
                    ui.label("Area");
                    ui.radio_value(&mut self.integrate.area_type, AreaType::Mathematical, "Signed");
                    ui.radio_value(&mut self.integrate.area_type, AreaType::Absolute, "Absolute");
                });
                ui.checkbox(
                    &mut self.integrate.end_point_baseline,
                    "Subtract the line through the first and last point",
                )
                .on_hover_text("For the area of a peak above a sloping background: choose the rows around the peak.");
                ui.checkbox(&mut self.integrate.cumulative, "Add a column with the running integral");
            }
            AnalysisKind::Differentiate => {
                ui.horizontal(|ui| {
                    ui.label("Derivative order");
                    ui.add(DragValue::new(&mut self.diff.order).range(1..=9));
                });
                ui.checkbox(&mut self.diff.smooth, "Smooth with a local polynomial fit");
                ui.add_enabled_ui(self.diff.smooth, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Polynomial order");
                        ui.add(DragValue::new(&mut self.diff.poly).range(1..=9));
                        ui.label(if self.smooth_kind == SmoothKind::Fft {
                            "Points of window (cutoff)"
                        } else {
                            "Points of window"
                        });
                        ui.add(DragValue::new(&mut self.diff.npts).range(3..=1001));
                    });
                });
            }
            AnalysisKind::Smooth => {
                ui.horizontal(|ui| {
                    ui.label("Method");
                    egui::ComboBox::from_id_salt("smooth_method")
                        .selected_text(match self.smooth_kind {
                            SmoothKind::SavitzkyGolay => "Savitzky-Golay",
                            SmoothKind::Average => "Adjacent averaging",
                            SmoothKind::Weighted => "Weighted adjacent averaging",
                            SmoothKind::Percentile => "Percentile filter",
                            SmoothKind::Fft => "FFT filter",
                            SmoothKind::Lowess => "Lowess",
                            SmoothKind::Loess => "Loess",
                            SmoothKind::Binomial => "Binomial",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.smooth_kind, SmoothKind::SavitzkyGolay, "Savitzky-Golay");
                            ui.selectable_value(&mut self.smooth_kind, SmoothKind::Average, "Adjacent averaging");
                            ui.selectable_value(
                                &mut self.smooth_kind,
                                SmoothKind::Weighted,
                                "Weighted adjacent averaging",
                            );
                            ui.selectable_value(&mut self.smooth_kind, SmoothKind::Percentile, "Percentile filter");
                            ui.selectable_value(&mut self.smooth_kind, SmoothKind::Fft, "FFT filter");
                            ui.selectable_value(&mut self.smooth_kind, SmoothKind::Lowess, "Lowess");
                            ui.selectable_value(&mut self.smooth_kind, SmoothKind::Loess, "Loess");
                            ui.selectable_value(&mut self.smooth_kind, SmoothKind::Binomial, "Binomial");
                        });
                });
                ui.horizontal(|ui| {
                    let local = matches!(self.smooth_kind, SmoothKind::Lowess | SmoothKind::Loess);
                    if local {
                        ui.label("Span");
                        ui.add(DragValue::new(&mut self.smooth_span).speed(0.01).range(0.0..=1.0));
                    } else if self.smooth_kind == SmoothKind::Binomial {
                        ui.label("Order");
                        ui.add(DragValue::new(&mut self.smooth_order).range(1..=1000));
                    } else {
                        ui.label("Points of window");
                        ui.add(DragValue::new(&mut self.smooth_points).range(1..=100_001));
                    }
                    match self.smooth_kind {
                        SmoothKind::SavitzkyGolay => {
                            ui.label("Polynomial order");
                            ui.add(DragValue::new(&mut self.smooth_poly).range(1..=9));
                        }
                        SmoothKind::Percentile => {
                            ui.label("Percentile");
                            ui.add(DragValue::new(&mut self.smooth_percent).range(0.0..=100.0));
                        }
                        _ => {}
                    }
                });
                if self.smooth_kind == SmoothKind::Fft {
                    ui.weak("The FFT filter keeps frequencies below 1 / (2 dx points); larger values smooth more.");
                }
                if matches!(self.smooth_kind, SmoothKind::Lowess | SmoothKind::Loess) {
                    ui.weak("The span is the share of the points in each local fit (0 to 1); Lowess fits lines, Loess parabolas.");
                }
                let ends = !matches!(self.smooth_kind, SmoothKind::Fft | SmoothKind::Lowess | SmoothKind::Loess);
                ui.add_enabled_ui(ends, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("At the ends");
                        egui::ComboBox::from_id_salt("smooth_boundary")
                            .selected_text(boundary_label(self.boundary))
                            .show_ui(ui, |ui| {
                                for b in [
                                    Boundary::None,
                                    Boundary::Reflect,
                                    Boundary::Repeat,
                                    Boundary::Periodic,
                                    Boundary::Extrapolate,
                                ] {
                                    ui.selectable_value(&mut self.boundary, b, boundary_label(b));
                                }
                            });
                    });
                });
            }
            AnalysisKind::Normalize => {
                ui.horizontal(|ui| {
                    ui.label("Method");
                    egui::ComboBox::from_id_salt("normalize_method")
                        .selected_text(NORMALIZE_CHOICES[self.normalize].0)
                        .width(260.0)
                        .show_ui(ui, |ui| {
                            for (i, (label, _)) in NORMALIZE_CHOICES.iter().enumerate() {
                                ui.selectable_value(&mut self.normalize, i, *label);
                            }
                        });
                });
                if matches!(NORMALIZE_CHOICES[self.normalize].1, NormalizeMethod::Divide(_)) {
                    ui.horizontal(|ui| {
                        ui.label("Value");
                        ui.add(DragValue::new(&mut self.divisor).speed(0.1));
                    });
                }
            }
            AnalysisKind::Baseline => {
                ui.radio_value(&mut self.baseline, BaselineKind::Minimum, "The smallest value");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.baseline, BaselineKind::Constant, "A constant");
                    ui.add_enabled(self.baseline == BaselineKind::Constant, DragValue::new(&mut self.level).speed(0.1));
                });
                ui.radio_value(
                    &mut self.baseline,
                    BaselineKind::EndPoints,
                    "The line through the first and last point",
                );
                ui.radio_value(&mut self.baseline, BaselineKind::Line, "The line through two points");
                ui.add_enabled_ui(self.baseline == BaselineKind::Line, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("X1");
                        ui.add(DragValue::new(&mut self.line[0]).speed(0.01));
                        ui.label("Y1");
                        ui.add(DragValue::new(&mut self.line[1]).speed(0.01));
                        ui.label("X2");
                        ui.add(DragValue::new(&mut self.line[2]).speed(0.01));
                        ui.label("Y2");
                        ui.add(DragValue::new(&mut self.line[3]).speed(0.01));
                    });
                });
            }
            AnalysisKind::Interpolate => {
                ui.horizontal(|ui| {
                    ui.label("Method");
                    ui.radio_value(&mut self.interp.method, InterpMethod::Linear, "Linear");
                    ui.radio_value(&mut self.interp.method, InterpMethod::CubicSpline, "Cubic spline");
                    ui.radio_value(&mut self.interp.method, InterpMethod::Akima, "Akima spline");
                    ui.radio_value(&mut self.interp.method, InterpMethod::BSpline, "Cubic B-spline");
                });
                if self.interp.method == InterpMethod::BSpline {
                    ui.horizontal(|ui| {
                        ui.label("Smoothing factor");
                        ui.add(DragValue::new(&mut self.interp.smoothing).speed(0.01).range(0.0..=1.0e12));
                    });
                    ui.weak(
                        "The factor is the allowed sum of squared residuals, as in Origin's sf. 0 gives the                          interpolating cubic spline, which is not Origin's automatic smoothing for 0.",
                    );
                }
                if self.interp.method == InterpMethod::CubicSpline {
                    ui.horizontal(|ui| {
                        ui.label("Spline ends");
                        ui.radio_value(&mut self.interp.boundary, SplineBoundary::NotAKnot, "Not-a-knot");
                        ui.radio_value(&mut self.interp.boundary, SplineBoundary::Natural, "Natural");
                    });
                }
                let mut from_column = self.interp_at.is_some();
                if ui.checkbox(&mut from_column, "Interpolate at the X values of a column").changed() {
                    if from_column {
                        self.set_interp_from_column();
                    } else {
                        self.interp_at = None;
                    }
                }
                if let Some(at) = &mut self.interp_at {
                    let names = &self.sheet_columns;
                    egui::ComboBox::from_id_salt("interp_at_column")
                        .selected_text(names.get(*at).map_or("", |(_, n)| n.as_str()))
                        .show_ui(ui, |ui| {
                            for (k, (_, name)) in names.iter().enumerate() {
                                ui.selectable_value(at, k, name);
                            }
                        });
                }
                let grid = self.interp_at.is_none();
                ui.add_enabled_ui(grid, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Number of points");
                        ui.add(DragValue::new(&mut self.interp.npts).range(2..=10_000_000));
                    });
                    ui.checkbox(&mut self.interp_own_range, "Choose the X range of the result");
                });
                ui.add_enabled_ui(grid && self.interp_own_range, |ui| {
                    let (mut lo, mut hi) = self.interp.range.unwrap_or((0.0, 1.0));
                    ui.horizontal(|ui| {
                        ui.label("X from");
                        ui.add(DragValue::new(&mut lo).speed(0.01));
                        ui.label("to");
                        ui.add(DragValue::new(&mut hi).speed(0.01));
                    });
                    self.interp.range = Some((lo, hi));
                });
            }
            AnalysisKind::Peaks | AnalysisKind::PeakChain => {
                let chain = self.kind == AnalysisKind::PeakChain;
                if chain {
                    ui.horizontal(|ui| {
                        ui.label("Baseline");
                        ui.radio_value(&mut self.chain_baseline, ChainBaselineKind::None, "None");
                        ui.radio_value(&mut self.chain_baseline, ChainBaselineKind::EndPoints, "End points");
                        ui.radio_value(&mut self.chain_baseline, ChainBaselineKind::Minima, "Lowest points of");
                        ui.add_enabled(
                            self.chain_baseline == ChainBaselineKind::Minima,
                            DragValue::new(&mut self.chain_minima).range(2..=1000),
                        );
                        ui.label("stretches");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Search method");
                        egui::ComboBox::from_id_salt("peak_find_method")
                            .selected_text(FIND_METHODS[self.find_method].0)
                            .show_ui(ui, |ui| {
                                for (i, (name, _)) in FIND_METHODS.iter().enumerate() {
                                    ui.selectable_value(&mut self.find_method, i, *name);
                                }
                            });
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Direction");
                    ui.radio_value(&mut self.peaks.direction, PeakDirection::Both, "Both");
                    ui.radio_value(&mut self.peaks.direction, PeakDirection::Positive, "Positive");
                    ui.radio_value(&mut self.peaks.direction, PeakDirection::Negative, "Negative");
                });
                ui.horizontal(|ui| {
                    ui.label("Neighbours compared on each side");
                    ui.add(DragValue::new(&mut self.peaks.npts).range(1..=10_000));
                });
                ui.radio_value(&mut self.peak_keep, PeakKeep::Default, "Keep peaks of at least 20 % of the largest");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.peak_keep, PeakKeep::Height, "Keep peaks of at least this height");
                    ui.add_enabled(
                        self.peak_keep == PeakKeep::Height,
                        DragValue::new(&mut self.peak_height).speed(0.01),
                    );
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.peak_keep, PeakKeep::Count, "Keep only the highest");
                    ui.add_enabled(
                        self.peak_keep == PeakKeep::Count,
                        DragValue::new(&mut self.peak_count).range(1..=10_000),
                    );
                });
                ui.radio_value(&mut self.peak_keep, PeakKeep::All, "Keep every local extremum");
                if !chain {
                    ui.horizontal(|ui| {
                        ui.label("Smoothing points before the search");
                        ui.add(DragValue::new(&mut self.peaks.smooth_points).range(0..=1001));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Foot height");
                        ui.add(DragValue::new(&mut self.peaks.foot_height).speed(0.01).range(0.0..=f64::MAX));
                    });
                    ui.horizontal(|ui| {
                        let mut limited = self.peaks.max_half_width.is_some();
                        ui.checkbox(&mut limited, "Limit the distance of the feet from the peak");
                        let mut w = self.peaks.max_half_width.unwrap_or(1.0);
                        ui.add_enabled(limited, DragValue::new(&mut w).speed(0.01).range(0.0..=f64::MAX));
                        self.peaks.max_half_width = limited.then_some(w);
                    });
                }
                if chain {
                    ui.checkbox(&mut self.chain_half_height, "Integrate only the upper half of each peak");
                    ui.checkbox(&mut self.chain_integrate, "Integrate every peak (area, area %, FWHM, limits)");
                    ui.horizontal(|ui| {
                        ui.label("Fit all peaks together");
                        ui.radio_value(&mut self.chain_fit, None, "No fit");
                        ui.radio_value(&mut self.chain_fit, Some(PeakShape::Gauss), "Gauss");
                        ui.radio_value(&mut self.chain_fit, Some(PeakShape::Lorentz), "Lorentz");
                    });
                    ui.add_enabled(
                        self.chain_fit.is_some(),
                        egui::Checkbox::new(&mut self.chain_fit_linear, "Sloped baseline in the fit"),
                    );
                }
            }
            AnalysisKind::Statistics => {
                let d = &mut self.stats.desc;
                ui.label("Quantities");
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut d.n, "N");
                    ui.checkbox(&mut d.missing, "Missing");
                    ui.checkbox(&mut d.mean, "Mean");
                    ui.checkbox(&mut d.sd, "SD");
                    ui.checkbox(&mut d.sem, "SEM");
                    ui.checkbox(&mut d.sum, "Sum");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut d.min, "Minimum");
                    ui.checkbox(&mut d.q1, "Q1");
                    ui.checkbox(&mut d.median, "Median");
                    ui.checkbox(&mut d.q3, "Q3");
                    ui.checkbox(&mut d.max, "Maximum");
                    ui.checkbox(&mut d.iqr, "IQR");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut d.variance, "Variance");
                    ui.checkbox(&mut d.skewness, "Skewness");
                    ui.checkbox(&mut d.kurtosis, "Kurtosis");
                    ui.checkbox(&mut d.cv, "Coefficient of variation");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.confidence_limits, "Confidence limits of the mean, level");
                    ui.add_enabled(d.confidence_limits, DragValue::new(&mut d.level).speed(0.001).range(0.5..=0.9999));
                });
            }
            AnalysisKind::TTest => {
                let st = &mut self.stats;
                ui.horizontal(|ui| {
                    ui.label("Test");
                    ui.radio_value(&mut st.ttest, 0, "One sample");
                    ui.radio_value(&mut st.ttest, 1, "Two sample");
                    ui.radio_value(&mut st.ttest, 2, "Paired");
                });
                if st.ttest == 0 {
                    ui.horizontal(|ui| {
                        ui.label("Hypothesized mean");
                        ui.add(DragValue::new(&mut st.mean).speed(0.01));
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Hypothesized difference (first minus second)");
                        ui.add(DragValue::new(&mut st.difference).speed(0.01));
                    });
                    ui.label("Needs exactly two columns.");
                }
                ui.add_enabled(
                    st.ttest == 1,
                    egui::Checkbox::new(&mut st.equal_variance, "Assume equal variances (otherwise Welch)"),
                );
                ui.horizontal(|ui| {
                    ui.label("Alternative");
                    ui.radio_value(&mut st.tail, Tail::Two, "Two-tailed");
                    ui.radio_value(&mut st.tail, Tail::Upper, "Upper tail");
                    ui.radio_value(&mut st.tail, Tail::Lower, "Lower tail");
                });
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            AnalysisKind::Anova => {
                let st = &mut self.stats;
                ui.label("Every chosen column is a group.");
                ui.horizontal(|ui| {
                    ui.label("Means comparison");
                    egui::ComboBox::from_id_salt("anova_comparison")
                        .selected_text(COMPARISONS[st.comparison].0)
                        .show_ui(ui, |ui| {
                            for (i, (label, _)) in COMPARISONS.iter().enumerate() {
                                ui.selectable_value(&mut st.comparison, i, *label);
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            AnalysisKind::Anova2 => {
                let st = &mut self.stats;
                ui.horizontal(|ui| {
                    ui.label("Input data");
                    ui.radio_value(&mut st.two_way_raw, false, "Indexed");
                    ui.radio_value(&mut st.two_way_raw, true, "Raw");
                });
                if st.two_way_raw {
                    ui.weak(
                        "Every chosen column is one cell of the design: give its level of factor A and of factor B.",
                    );
                    let names: Vec<String> = self.column_names.split(", ").map(str::to_string).collect();
                    egui::Grid::new("two_way_levels").num_columns(3).show(ui, |ui| {
                        ui.label("Column");
                        ui.label("Factor A level");
                        ui.label("Factor B level");
                        ui.end_row();
                        for (name, (a, b)) in names.iter().zip(st.two_way_levels.iter_mut()) {
                            ui.label(name);
                            ui.add(egui::TextEdit::singleline(a).desired_width(90.0));
                            ui.add(egui::TextEdit::singleline(b).desired_width(90.0));
                            ui.end_row();
                        }
                    });
                } else {
                    for (i, label) in ["Factor A", "Factor B", "Data"].into_iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(label);
                            let current = self.sheet_columns.get(st.two_way_columns[i]).map_or("", |c| c.1.as_str());
                            egui::ComboBox::from_id_salt(("two_way_column", i))
                                .selected_text(current)
                                .width(220.0)
                                .show_ui(ui, |ui| {
                                    for (j, (_, text)) in self.sheet_columns.iter().enumerate() {
                                        ui.selectable_value(&mut st.two_way_columns[i], j, text);
                                    }
                                });
                        });
                    }
                    ui.weak("The factor columns may hold text or numbers; one row of the sheet is one observation.");
                }
                ui.checkbox(&mut st.two_way_interaction, "Interaction of the two factors");
                ui.horizontal(|ui| {
                    ui.label("Means comparison");
                    egui::ComboBox::from_id_salt("anova2_comparison")
                        .selected_text(COMPARISONS[st.comparison].0)
                        .show_ui(ui, |ui| {
                            for (i, (label, _)) in COMPARISONS.iter().enumerate() {
                                ui.selectable_value(&mut st.comparison, i, *label);
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            AnalysisKind::MannWhitney => {
                let st = &mut self.stats;
                ui.label("Needs exactly two columns (two independent samples).");
                ui.horizontal(|ui| {
                    ui.label("Alternative (first column)");
                    ui.radio_value(&mut st.tail, Tail::Two, "Differs");
                    ui.radio_value(&mut st.tail, Tail::Upper, "Larger");
                    ui.radio_value(&mut st.tail, Tail::Lower, "Smaller");
                });
                ui.checkbox(&mut st.exact, "Exact p-value (otherwise the normal approximation)");
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            AnalysisKind::SignedRank | AnalysisKind::SignedRankPaired => {
                let st = &mut self.stats;
                ui.horizontal(|ui| {
                    ui.label("Test");
                    ui.radio_value(&mut st.paired, false, "One sample");
                    ui.radio_value(&mut st.paired, true, "Paired (two columns)");
                });
                ui.add_enabled_ui(!st.paired, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Hypothesized median");
                        ui.add(DragValue::new(&mut st.median).speed(0.01));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Alternative");
                    ui.radio_value(&mut st.tail, Tail::Two, "Two-tailed");
                    ui.radio_value(&mut st.tail, Tail::Upper, "Upper tail");
                    ui.radio_value(&mut st.tail, Tail::Lower, "Lower tail");
                });
                ui.checkbox(
                    &mut st.decide_exact,
                    "Decide with the exact p-value where there is one (up to 80 differences)",
                );
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            AnalysisKind::KruskalWallis | AnalysisKind::MedianTest | AnalysisKind::Friedman => {
                let st = &mut self.stats;
                ui.label(match self.kind {
                    AnalysisKind::Friedman => "Every chosen column is a treatment, every row a block.",
                    _ => "Every chosen column is a group.",
                });
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            AnalysisKind::SignTest | AnalysisKind::KsTwoSample => {
                let st = &mut self.stats;
                ui.label("Needs exactly two columns.");
                ui.horizontal(|ui| {
                    ui.label("Alternative (first column)");
                    ui.radio_value(&mut st.tail, Tail::Two, "Two-tailed");
                    ui.radio_value(&mut st.tail, Tail::Upper, "F(x) > G(y)");
                    ui.radio_value(&mut st.tail, Tail::Lower, "F(x) < G(y)");
                });
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            AnalysisKind::Grubbs | AnalysisKind::Dixon => {
                let st = &mut self.stats;
                ui.label("One test for each chosen column.");
                if self.kind == AnalysisKind::Dixon {
                    ui.horizontal(|ui| {
                        ui.label("Significance level");
                        ui.radio_value(&mut st.alpha, 0.1, "0.1");
                        ui.radio_value(&mut st.alpha, 0.05, "0.05");
                        ui.radio_value(&mut st.alpha, 0.01, "0.01");
                    });
                    ui.label("Dixon's table covers 3 to 10 numbers.");
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Significance level");
                        ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                    });
                }
            }
            AnalysisKind::RowStatistics => {
                let r = &mut self.stats.rows;
                ui.label("Statistics of each row across the chosen columns, added as new columns.");
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut r.mean, "Mean");
                    ui.checkbox(&mut r.sd, "Standard deviation");
                    ui.checkbox(&mut r.sem, "SE of mean");
                    ui.checkbox(&mut r.min, "Minimum");
                    ui.checkbox(&mut r.max, "Maximum");
                    ui.checkbox(&mut r.n, "Number of points");
                    ui.checkbox(&mut r.sum, "Sum");
                });
            }
            AnalysisKind::Correlation => {
                let st = &mut self.stats;
                ui.horizontal(|ui| {
                    ui.label("Coefficients");
                    ui.checkbox(&mut st.pearson, "Pearson");
                    ui.checkbox(&mut st.spearman, "Spearman");
                    ui.checkbox(&mut st.kendall, "Kendall");
                });
                ui.horizontal(|ui| {
                    ui.label("Missing values");
                    ui.radio_value(&mut st.listwise, false, "Pairwise");
                    ui.radio_value(&mut st.listwise, true, "Listwise");
                });
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            AnalysisKind::Histogram => {
                let st = &mut self.stats;
                ui.horizontal(|ui| {
                    ui.label("Bins");
                    ui.radio_value(&mut st.bins, 0, "Automatic");
                    ui.radio_value(&mut st.bins, 1, "Bin size");
                    ui.add_enabled(st.bins == 1, DragValue::new(&mut st.bin_size).speed(0.01).range(1e-12..=1e12));
                    ui.radio_value(&mut st.bins, 2, "Number of bins");
                    ui.add_enabled(st.bins == 2, DragValue::new(&mut st.bin_count).range(1..=10_000));
                });
                ui.checkbox(&mut st.own_range, "Set the range of the bins");
                ui.add_enabled_ui(st.own_range, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("From");
                        ui.add(DragValue::new(&mut st.bin_min).speed(0.01));
                        ui.label("to");
                        ui.add(DragValue::new(&mut st.bin_max).speed(0.01));
                    });
                });
                ui.checkbox(&mut st.graph, "Make a graph with a column plot of the counts");
            }
            AnalysisKind::AverageCurves => self.curves.average_ui(ui, self.columns.len()),
            AnalysisKind::Intersect => self.curves.intersect_ui(ui),
            AnalysisKind::PolygonArea => {
                ui.weak(
                    "Each input column is a closed curve: its last point is joined to its first. The area is \
                     reported with its sign, so a curve that runs counterclockwise is positive.",
                );
            }
            AnalysisKind::Normality => {
                let st = &mut self.stats;
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
        }
    }

    /// Ticks "Auto Preview" (for `--show analysis-preview:NAME`).
    pub fn tick_auto_preview(&mut self) {
        self.auto_preview = true;
    }

    /// The result curve of the preview, X then Y (for tests).
    #[cfg(test)]
    pub fn preview_result(&self) -> Option<(Vec<f64>, Vec<f64>)> {
        self.preview.as_ref()?.as_ref().ok().map(|d| (d.rx.clone(), d.ry.clone()))
    }

    /// Sets the Curve Translate shift as the dialog's field would (for tests).
    #[cfg(test)]
    pub fn set_translate_by(&mut self, by: f64) {
        self.manip.set_by(by);
    }

    /// How many points the preview holds, or the reason there is none (for tests).
    #[cfg(test)]
    pub fn preview_state(&self) -> Option<Result<usize, String>> {
        self.preview.as_ref().map(|p| p.as_ref().map(|d| d.ry.len()).map_err(Clone::clone))
    }

    /// The X-Function name that Origin shows after the title ("Smooth: smooth"); empty for the tools whose
    /// window is titled with the tool's name alone.
    fn xf_name(&self) -> &'static str {
        match self.kind {
            AnalysisKind::Integrate | AnalysisKind::IntegrateRanges => "integ1",
            AnalysisKind::Differentiate => "differentiate",
            AnalysisKind::Smooth => "smooth",
            AnalysisKind::Normalize => "normalize",
            AnalysisKind::Baseline => "baseline",
            AnalysisKind::SubtractRef | AnalysisKind::CurveMath => "curvemath",
            AnalysisKind::Interpolate => "interp1",
            AnalysisKind::Peaks => "pkFind",
            AnalysisKind::PeakChain => "",
            AnalysisKind::Statistics => "DescStats",
            AnalysisKind::TTest => "ttest",
            AnalysisKind::Anova => "anova1",
            AnalysisKind::Histogram => "freqcounts",
            AnalysisKind::Normality => "normtest",
            AnalysisKind::Anova2 => "anova2",
            AnalysisKind::MannWhitney => "mannwhitney",
            AnalysisKind::SignedRank | AnalysisKind::SignedRankPaired => "signrank",
            AnalysisKind::KruskalWallis => "kwanova",
            AnalysisKind::Correlation => "corrcoef",
            AnalysisKind::RowStatistics => "rowstats",
            AnalysisKind::Grubbs => "grubbs",
            AnalysisKind::Dixon => "qtest",
            AnalysisKind::SignTest => "sign2",
            AnalysisKind::KsTwoSample => "kstest2",
            AnalysisKind::MedianTest => "mediantest",
            AnalysisKind::Friedman => "friedman",
            AnalysisKind::AverageCurves => "avecurves",
            AnalysisKind::Intersect => "intersect",
            AnalysisKind::PolygonArea => "polyarea",
            // Origin's X-Function names for these were not looked up; the title shows the tool alone.
            AnalysisKind::SubtractLine
            | AnalysisKind::ReduceX
            | AnalysisKind::TranslateX
            | AnalysisKind::TranslateY
            | AnalysisKind::MedianFilter => "",
        }
    }

    /// The window title.
    fn window_title(&self) -> String {
        match self.xf_name() {
            "" => self.kind.title().to_string(),
            name => format!("{}: {name}", self.kind.title()),
        }
    }

    /// One line for the Description row of the window.
    fn description(&self) -> &'static str {
        match self.kind {
            AnalysisKind::Integrate => "Integrate the input curves by the trapezoidal rule",
            AnalysisKind::IntegrateRanges => "Integrate the input curves over several X ranges",
            AnalysisKind::Differentiate => "Differentiate the input curves",
            AnalysisKind::Smooth => "Smooth the input curves",
            AnalysisKind::Normalize => "Normalize the input columns",
            AnalysisKind::Baseline => "Subtract a baseline from the input curves",
            AnalysisKind::SubtractRef => "Subtract a reference curve from the input curves",
            AnalysisKind::CurveMath => "Add, subtract, multiply, divide or raise to a power with a constant or a curve",
            AnalysisKind::Interpolate => "Interpolate the input curves at evenly spaced X values",
            AnalysisKind::Peaks => "Find the peaks of the input curves",
            AnalysisKind::PeakChain => "Baseline, peak search, integration and fit of the peaks",
            AnalysisKind::Statistics => "Descriptive statistics of the input columns",
            AnalysisKind::TTest => "Test the means of one or two samples",
            AnalysisKind::Anova => "One-way analysis of variance; every column is a group",
            AnalysisKind::Histogram => "Count the values of the input columns in bins",
            AnalysisKind::Normality => "Test whether the input columns come from a normal distribution",
            AnalysisKind::Anova2 => "Two-way analysis of variance",
            AnalysisKind::MannWhitney => "Compare two independent samples by their ranks",
            AnalysisKind::SignedRank => "Test the median of one sample by signed ranks",
            AnalysisKind::SignedRankPaired => "Test the difference of two paired samples by signed ranks",
            AnalysisKind::KruskalWallis => "Compare several samples by their ranks",
            AnalysisKind::RowStatistics => "Descriptive statistics of each row across the chosen columns",
            AnalysisKind::Grubbs => "Test the value farthest from the mean for being an outlier",
            AnalysisKind::Dixon => "Test the lowest or highest value for being an outlier (3 to 10 numbers)",
            AnalysisKind::SignTest => "Test two paired samples by the signs of their differences",
            AnalysisKind::KsTwoSample => "Compare two samples by their distribution functions",
            AnalysisKind::MedianTest => "Compare the medians of several samples (Mood's median test)",
            AnalysisKind::Friedman => "Compare treatments in repeated blocks by ranks within each row",
            AnalysisKind::AverageCurves => "Average several curves on a common set of X values",
            AnalysisKind::Intersect => "Find the points where two curves meet",
            AnalysisKind::PolygonArea => "Area, centroid and perimeter of each input curve taken as a closed polygon",
            AnalysisKind::Correlation => "Correlation coefficients of the input columns",
            AnalysisKind::SubtractLine => "Subtract the straight line through two points from the input curves",
            AnalysisKind::ReduceX => "Reduce the input curves to evenly spaced X values",
            AnalysisKind::TranslateX => "Shift the input curves horizontally",
            AnalysisKind::TranslateY => "Shift the input curves vertically",
            AnalysisKind::MedianFilter => "Replace each point by the median of the points around it",
        }
    }

    /// Where the results go, for the Output group.
    fn output_text(&self) -> &'static str {
        match self.kind {
            AnalysisKind::Integrate => "Area, peak position, height and width in a new sheet",
            AnalysisKind::IntegrateRanges => {
                "One row per range and column (area, absolute area, peak, width) in a new sheet"
            }
            AnalysisKind::Differentiate
            | AnalysisKind::Normalize
            | AnalysisKind::Baseline
            | AnalysisKind::Smooth
            | AnalysisKind::SubtractRef
            | AnalysisKind::CurveMath => "New columns beside the input",
            AnalysisKind::Interpolate => "The interpolated curves in a new sheet",
            AnalysisKind::Peaks => "Peak positions and heights in a new sheet",
            AnalysisKind::PeakChain => "Peaks, areas and fit results in a new sheet; the baseline beside the data",
            AnalysisKind::Statistics => {
                "The chosen quantities (count, mean, SD, SEM, quartiles, variance, skewness, confidence limits, ...)                  in a new sheet"
            }
            AnalysisKind::TTest => "The statistic, p-value and confidence interval in a new sheet",
            AnalysisKind::Anova => "Group statistics, the ANOVA table and the comparisons in a new sheet",
            AnalysisKind::Histogram => "Bin centres, counts, cumulative counts and percentages in a new sheet",
            AnalysisKind::Normality => {
                "Shapiro-Wilk, Kolmogorov-Smirnov, Lilliefors, Anderson-Darling and D'Agostino-Pearson in a new sheet"
            }
            AnalysisKind::Anova2 => {
                "The tables (statistics, ANOVA, comparisons) in a new sheet; sums of squares are adjusted for the \
                 other terms, as in Origin"
            }
            AnalysisKind::MannWhitney => "Ranks, U, Z and the p-values in a new sheet",
            AnalysisKind::SignedRank | AnalysisKind::SignedRankPaired => "Both p-values in a new sheet",
            AnalysisKind::KruskalWallis => "Mean ranks, the chi-square statistic and its p-value in a new sheet",
            AnalysisKind::RowStatistics => "New columns behind the last chosen column",
            AnalysisKind::Grubbs | AnalysisKind::Dixon => {
                "The suspect value, its row, the statistic and the verdict in a new sheet"
            }
            AnalysisKind::SignTest | AnalysisKind::KsTwoSample => {
                "The counts or distance, the statistic and the p-value in a new sheet"
            }
            AnalysisKind::MedianTest => {
                "Counts around the median, the chi-square statistic and its p-value in a new sheet"
            }
            AnalysisKind::Friedman => "Mean ranks, the chi-square statistic and its p-value in a new sheet",
            AnalysisKind::AverageCurves => "X, the mean, its standard deviation and the count in a new sheet",
            AnalysisKind::Intersect => "X and Y of each intersection in a new sheet",
            AnalysisKind::PolygonArea => {
                "Area, signed area, centroid, perimeter and a self-intersection flag in a new sheet"
            }
            AnalysisKind::Correlation => "The matrices of coefficients and p-values in a new sheet",
            AnalysisKind::SubtractLine | AnalysisKind::TranslateY | AnalysisKind::MedianFilter => {
                "New columns beside the input"
            }
            AnalysisKind::ReduceX | AnalysisKind::TranslateX => "The curves in a new sheet with their own X column",
        }
    }

    /// Whether a preview of the result can be computed cheaply: the tools that write columns of
    /// the same length as the input, or a resampled curve.
    fn has_preview(&self) -> bool {
        matches!(
            self.kind,
            AnalysisKind::Integrate
                | AnalysisKind::Differentiate
                | AnalysisKind::Smooth
                | AnalysisKind::Normalize
                | AnalysisKind::Baseline
                | AnalysisKind::Interpolate
                | AnalysisKind::SubtractRef
                | AnalysisKind::CurveMath
        ) || super::manip_options::handles(self.kind)
    }

    /// Chooses other input columns of the sheet.
    fn set_columns(&mut self, project: &Project, columns: Vec<ColumnId>) {
        let Some((_, sheet, _)) = columns.first().and_then(|c| project.locate(*c)) else {
            return;
        };
        let names: Vec<&str> =
            columns.iter().filter_map(|id| sheet.column(*id)).map(|c| c.short_name.as_str()).collect();
        if names.len() != columns.len() {
            return;
        }
        self.column_names = names.join(", ");
        self.reference_choices = reference_choices(project, &columns);
        self.reference = self.reference.min(self.reference_choices.len().saturating_sub(1));
        if self.kind == AnalysisKind::CurveMath {
            self.use_reference = self.use_reference && !self.reference_choices.is_empty();
        }
        if self.kind == AnalysisKind::Anova2 {
            self.stats.two_way_levels.resize(columns.len(), (String::new(), String::new()));
        }
        self.columns = columns;
        self.preview_request = None;
        // A new choice of input starts a new set of results.
        self.applied = None;
        self.applied_key = None;
        self.output_existing.clear();
    }

    /// True for the tools whose results can go to a new sheet or to existing columns.
    fn has_output_choice(&self) -> bool {
        matches!(
            self.kind,
            AnalysisKind::Differentiate
                | AnalysisKind::Normalize
                | AnalysisKind::Baseline
                | AnalysisKind::Smooth
                | AnalysisKind::SubtractRef
                | AnalysisKind::CurveMath
                | AnalysisKind::SubtractLine
                | AnalysisKind::TranslateY
                | AnalysisKind::MedianFilter
        )
    }

    /// The target of the results as the Output node has it set.
    fn output_target(&self) -> Result<OutputTarget, String> {
        if !self.has_output_choice() {
            return Ok(OutputTarget::NextToInput);
        }
        Ok(match self.output_choice {
            OutputChoice::NextToInput => OutputTarget::NextToInput,
            OutputChoice::NewSheet => OutputTarget::NewSheet,
            OutputChoice::Existing => {
                let mut ids = Vec::new();
                for i in 0..self.columns.len() {
                    let at = self.output_existing.get(i).copied().unwrap_or_else(|| self.default_existing(i));
                    ids.push(self.sheet_columns.get(at).map(|(id, _)| *id).ok_or("choose the column for the result")?);
                }
                OutputTarget::Existing(ids)
            }
        })
    }

    /// The column after the input column, as the first suggestion for an existing target.
    fn default_existing(&self, input: usize) -> usize {
        let at = self.columns.get(input).and_then(|c| self.sheet_columns.iter().position(|(id, _)| id == c));
        at.map_or(0, |a| (a + 1).min(self.sheet_columns.len().saturating_sub(1)))
    }

    /// Whether Recalculate Manual and Auto apply to the tool as it is set.
    fn linkable(&self) -> bool {
        self.tool().is_ok_and(|t| Project::can_link(&t))
    }

    /// The Output node of the tree.
    fn output_node(&mut self, ui: &mut egui::Ui) {
        if !self.has_output_choice() {
            ui.add_enabled_ui(false, |ui| {
                ui.label(self.output_text());
            });
            return;
        }
        let label = |c: OutputChoice| match c {
            OutputChoice::NextToInput => "New columns beside the input",
            OutputChoice::NewSheet => "A new sheet",
            OutputChoice::Existing => "Existing columns",
        };
        ui.horizontal(|ui| {
            ui.label("Output to");
            egui::ComboBox::from_id_salt("analysis_output_choice").selected_text(label(self.output_choice)).show_ui(
                ui,
                |ui| {
                    for c in [OutputChoice::NextToInput, OutputChoice::NewSheet, OutputChoice::Existing] {
                        ui.selectable_value(&mut self.output_choice, c, label(c));
                    }
                },
            );
        });
        if self.output_choice == OutputChoice::Existing {
            if self.output_existing.len() != self.columns.len() {
                self.output_existing = (0..self.columns.len()).map(|i| self.default_existing(i)).collect();
            }
            let names: Vec<String> = self.column_names.split(", ").map(str::to_string).collect();
            for i in 0..self.columns.len() {
                ui.horizontal(|ui| {
                    ui.label(format!("Result for {}", names.get(i).map_or("", String::as_str)));
                    let at = &mut self.output_existing[i];
                    let shown = self.sheet_columns.get(*at).map_or("", |(_, n)| n.as_str());
                    egui::ComboBox::from_id_salt(("analysis_output_existing", i)).selected_text(shown).show_ui(
                        ui,
                        |ui| {
                            for (k, (_, name)) in self.sheet_columns.iter().enumerate() {
                                ui.selectable_value(at, k, name);
                            }
                        },
                    );
                });
            }
            ui.weak("The values and labels of the chosen columns are replaced.");
        }
    }

    /// Runs the tool for OK, Apply and Auto Apply. A later run of the same window replaces the
    /// columns of the earlier one; all runs of the window are one undo step.
    fn run(&mut self, doc: &mut Document) -> Result<Report, String> {
        let request = self.request()?;
        let output = self.output_target()?;
        let mode = if Project::can_link(&request.tool) { self.recalc } else { RecalcMode::None };
        // A histogram makes a graph that plots the new columns, which must stay.
        let previous = if self.kind == AnalysisKind::Histogram { None } else { self.applied.clone() };
        let key = (request.clone(), output.clone(), mode);
        let label = request.tool.name();
        let result = doc.apply_merging(label, self.undo_token, |p| {
            p.analyze_linked_into(previous.as_deref(), &request, &output, mode)
        });
        match result {
            Ok((report, outputs)) => {
                self.applied = Some(outputs);
                self.applied_key = Some(key);
                self.applied_report = Some(report.clone());
                self.problem = None;
                Ok(report)
            }
            Err(e) => {
                self.failed_key = Some(key);
                Err(e.to_string())
            }
        }
    }

    /// The range of the input in Origin's text form: `[Book1]Sheet1!(A"t",B"up")`.
    fn range_text(&self, project: &Project) -> String {
        let Some((book, sheet, first)) = self.columns.first().and_then(|c| project.locate(*c)) else {
            return String::new();
        };
        let part = |c: &sheaf_core::Column| {
            if c.long_name.is_empty() { c.short_name.clone() } else { format!("{}\"{}\"", c.short_name, c.long_name) }
        };
        let mut parts: Vec<String> = Vec::new();
        if let Some(x) = sheet.column_index(first.id).and_then(|i| sheet.x_for(i))
            && !self.columns.contains(&x.id)
        {
            parts.push(part(x));
        }
        parts.extend(self.columns.iter().filter_map(|id| sheet.column(*id)).map(part));
        format!("[{}]{}!({})", book.name, sheet.name, parts.join(","))
    }

    /// Runs the tool on a copy of the project and keeps the first input column and the first result
    /// column for the preview pane.
    fn compute_preview(&self, project: &Project) -> Result<PreviewData, String> {
        if !self.has_preview() {
            return Err("No preview for this tool".to_string());
        }
        let cells: usize = project.workbooks().flat_map(|b| &b.sheets).flat_map(|s| &s.columns).map(|c| c.len()).sum();
        if cells > PREVIEW_MAX_CELLS {
            return Err("The project is too large for a preview".to_string());
        }
        let request = self.request()?;
        let first = *request.columns.first().ok_or("no input column")?;
        let mut copy = project.clone();
        let report = copy.analyze(&request).map_err(|e| e.to_string())?;
        let (_, sheet, input) = copy.locate(first).ok_or("the input column is gone")?;
        let index = sheet.column_index(first).ok_or("the input column is gone")?;
        let y = input.numeric().ok_or("the input holds text")?;
        let x: Vec<f64> = match sheet.x_for(index).filter(|x| x.id != first).and_then(|c| c.numeric()) {
            Some(v) => (0..y.len()).map(|i| v.get(i).copied().unwrap_or(f64::NAN)).collect(),
            None => (1..=y.len()).map(|i| i as f64).collect(),
        };
        // A tool that writes a new sheet puts its X column first; the preview of those is the sheet's X
        // and Y pair, not the X column plotted against the input's X.
        let result_sheet = report.new_sheet.and_then(|s| copy.workbook(report.book)?.sheets.get(s));
        let on_result_sheet = |id: &ColumnId| result_sheet.is_some_and(|s| s.column(*id).is_some());
        let (rx, ry) = if let Some(s) = result_sheet.filter(|_| report.new_columns.first().is_some_and(on_result_sheet))
        {
            let col = |i: usize| s.columns.get(i).and_then(|c| c.numeric()).map(<[f64]>::to_vec);
            (col(0).ok_or("no result")?, col(1).ok_or("no result")?)
        } else if let Some(id) = report.new_columns.first() {
            let c = copy.column(*id).and_then(|c| c.numeric()).ok_or("the result has no numbers")?;
            ((0..c.len()).map(|i| x.get(i).copied().unwrap_or(f64::NAN)).collect(), c.to_vec())
        } else if let Some(s) = result_sheet {
            let col = |i: usize| s.columns.get(i).and_then(|c| c.numeric()).map(<[f64]>::to_vec);
            (col(0).ok_or("no result")?, col(1).ok_or("no result")?)
        } else {
            return Err("No preview for these options".to_string());
        };
        Ok(PreviewData { label: input.short_name.clone(), x, y: y.to_vec(), rx, ry })
    }

    /// Draws the Input node: the range with its picker, and the rows.
    fn input_node(&mut self, ui: &mut egui::Ui, project: &Project) {
        let text = self.range_text(project);
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 34.0).max(120.0);
            read_only_box(ui, &text, width);
            if ui.button("...").on_hover_text("Choose the columns of the sheet").clicked() {
                self.picker = Some(self.sheet_columns_selected());
            }
        });
        ui.horizontal(|ui| {
            ui.label("Rows");
            ui.radio_value(&mut self.row_mode, RowMode::All, "All");
            ui.radio_value(&mut self.row_mode, RowMode::Rows, "From row");
            ui.add_enabled_ui(self.row_mode == RowMode::Rows, |ui| {
                ui.add(DragValue::new(&mut self.from).range(1..=self.rows_in_sheet));
                ui.label("to");
                ui.add(DragValue::new(&mut self.to).range(1..=self.rows_in_sheet));
            });
        });
        ui.horizontal(|ui| {
            ui.label("    ");
            ui.radio_value(&mut self.row_mode, RowMode::XRange, "X from")
                .on_hover_text("The first unbroken run of rows whose X value lies in the interval.");
            ui.add_enabled_ui(self.row_mode == RowMode::XRange, |ui| {
                ui.add(DragValue::new(&mut self.x_from).speed(0.01));
                ui.label("to");
                ui.add(DragValue::new(&mut self.x_to).speed(0.01));
            });
        });
    }

    /// One tick per column of the sheet, set for the columns that are inputs now.
    fn sheet_columns_selected(&self) -> Vec<bool> {
        self.sheet_columns.iter().map(|(id, _)| self.columns.contains(id)).collect()
    }

    /// The small "Select Input Columns" window opened by the "..." button of the Input row.
    fn picker_window(&mut self, ctx: &egui::Context, project: &Project) {
        let Some(mut ticks) = self.picker.take() else {
            return;
        };
        let mut open = true;
        let mut done: Option<bool> = None;
        egui::Window::new("Select Input Columns")
            .id(egui::Id::new("analysis_input_picker"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for ((_, text), on) in self.sheet_columns.iter().zip(ticks.iter_mut()) {
                        ui.checkbox(on, text);
                    }
                });
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        done = Some(false);
                    }
                    if ui.add_enabled(ticks.iter().any(|t| *t), egui::Button::new("OK")).clicked() {
                        done = Some(true);
                    }
                });
            });
        match done {
            Some(true) => {
                let ids: Vec<ColumnId> =
                    self.sheet_columns.iter().zip(&ticks).filter(|(_, on)| **on).map(|((id, _), _)| *id).collect();
                self.set_columns(project, ids);
            }
            Some(false) => {}
            None if open => self.picker = Some(ticks),
            None => {}
        }
    }

    /// Draws the window. OK runs the tool and closes it; Apply runs it and keeps the window.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> AnalysisOutcome {
        if self.columns.iter().any(|c| doc.project().column(*c).is_none()) {
            return AnalysisOutcome::Closed;
        }
        let mut open = true;
        let mut pressed: Option<DialogButton> = None;
        let mut preview_now = false;
        let with_pane = self.show_preview && self.has_preview();
        let avail = ctx.available_rect();
        let width = if with_pane { 800.0 } else { 520.0 };
        let pos = (avail.center() - egui::vec2(width, 520.0) / 2.0).max(avail.min);
        let title = self.window_title();
        egui::Window::new(title)
            .id(egui::Id::new("analysis"))
            .open(&mut open)
            .default_pos(pos)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(width - 14.0);
                dialog_theme_row(ui);
                description_row(ui, self.description());
                ui.horizontal(|ui| {
                    ui.label("Recalculate");
                    let linkable = self.linkable();
                    let shown = if linkable { self.recalc.label() } else { RecalcMode::None.label() };
                    egui::ComboBox::from_id_salt("analysis_recalculate")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            if !linkable {
                                let _ = ui.selectable_label(true, "None");
                            } else {
                                ui.selectable_value(&mut self.recalc, RecalcMode::None, "None");
                            }
                            ui.add_enabled_ui(linkable, |ui| {
                                ui.selectable_value(&mut self.recalc, RecalcMode::Manual, "Manual")
                                    .on_hover_text("Mark the results when the input changes; recalculate by hand");
                                ui.selectable_value(&mut self.recalc, RecalcMode::Auto, "Auto")
                                    .on_hover_text("Run the tool again whenever the input changes");
                            });
                        })
                        .response
                        .on_hover_text("Smooth, Integrate and Find Peaks can follow their input");
                });
                ui.add_space(2.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(if with_pane { 480.0 } else { 500.0 });
                        egui::ScrollArea::vertical()
                            .id_salt("analysis_tree")
                            .max_height(380.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                tree_node(ui, "analysis_input", "Input", |ui| self.input_node(ui, doc.project()));
                                let title = self.kind.title();
                                tree_node(ui, "analysis_options", title, |ui| self.options(ui));
                                tree_node(ui, "analysis_output", "Output", |ui| self.output_node(ui));
                            });
                    });
                    if with_pane {
                        self.preview_pane(ui);
                    }
                });
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Right to left: the arrow that hides the preview is the rightmost control.
                    let arrow = egui::Button::new(if self.show_preview { "<<" } else { ">>" });
                    if ui.add_enabled(self.has_preview(), arrow).on_hover_text("Hide or show the preview").clicked() {
                        self.show_preview = !self.show_preview;
                    }
                    pressed = ok_cancel_apply(ui, true);
                    if ui.add_enabled(self.has_preview(), egui::Button::new("Preview")).clicked() {
                        preview_now = true;
                    }
                    ui.add_enabled(self.has_preview(), egui::Checkbox::new(&mut self.auto_preview, "Auto Preview"));
                    ui.checkbox(&mut self.auto_apply, "Auto Apply")
                        .on_hover_text("Run the tool again at once whenever an option changes");
                });
            });
        self.picker_window(ctx, doc.project());

        // The preview follows the options while Auto Preview is ticked, or when Preview is pressed.
        if self.has_preview() && self.show_preview {
            let current = self.request().ok();
            if preview_now || (self.auto_preview && current != self.preview_request) {
                self.preview = Some(self.compute_preview(doc.project()));
                self.preview_request = current;
            }
        }

        let current_key = match (self.request(), self.output_target()) {
            (Ok(r), Ok(o)) => {
                let mode = if Project::can_link(&r.tool) { self.recalc } else { RecalcMode::None };
                Some((r, o, mode))
            }
            _ => None,
        };
        if matches!(pressed, Some(DialogButton::Ok | DialogButton::Apply)) {
            // OK right after an Auto Apply run with the same options: the results are in place.
            if pressed == Some(DialogButton::Ok)
                && self.auto_apply
                && current_key.is_some()
                && current_key == self.applied_key
                && let Some(report) = self.applied_report.clone()
            {
                return AnalysisOutcome::Done(Box::new(report));
            }
            match self.run(doc) {
                Ok(report) => {
                    if pressed == Some(DialogButton::Apply) {
                        self.preview_request = None;
                        return AnalysisOutcome::Applied(Box::new(report));
                    }
                    return AnalysisOutcome::Done(Box::new(report));
                }
                Err(e) => self.problem = Some(e),
            }
        } else if self.auto_apply
            && pressed != Some(DialogButton::Cancel)
            && let Some(key) = current_key
            && Some(&key) != self.applied_key.as_ref()
            && Some(&key) != self.failed_key.as_ref()
        {
            match self.run(doc) {
                Ok(report) => return AnalysisOutcome::AutoApplied(Box::new(report)),
                Err(e) => self.problem = Some(e),
            }
        }
        if open && pressed != Some(DialogButton::Cancel) { AnalysisOutcome::Open } else { AnalysisOutcome::Closed }
    }

    /// The preview at the right: the first input column in grey, the result in red.
    fn preview_pane(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_width(270.0);
            let rect = preview_area(ui, egui::vec2(270.0, 250.0));
            let grey = egui::Color32::from_rgb(120, 120, 120);
            match &self.preview {
                Some(Ok(data)) => draw_preview(ui, rect, data),
                Some(Err(text)) => {
                    let font = egui::FontId::proportional(12.0);
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, font, grey);
                }
                None => {
                    let big = egui::FontId::proportional(20.0);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "No Preview",
                        big,
                        egui::Color32::BLACK,
                    );
                    let small = egui::FontId::proportional(11.0);
                    let at = rect.center() + egui::vec2(0.0, 24.0);
                    ui.painter().text(
                        at,
                        egui::Align2::CENTER_CENTER,
                        "Tick Auto Preview or press Preview",
                        small,
                        grey,
                    );
                }
            }
        });
    }
}

/// The curves of the preview pane.
struct PreviewData {
    label: String,
    x: Vec<f64>,
    y: Vec<f64>,
    rx: Vec<f64>,
    ry: Vec<f64>,
}

/// Most cells the project may hold for a preview to copy it.
const PREVIEW_MAX_CELLS: usize = 400_000;

/// Draws both curves of the preview, scaled to their common range.
fn draw_preview(ui: &egui::Ui, rect: egui::Rect, data: &PreviewData) {
    let finite = |v: &[f64]| v.iter().copied().filter(|v| v.is_finite()).collect::<Vec<_>>();
    let xs: Vec<f64> = finite(&data.x).into_iter().chain(finite(&data.rx)).collect();
    let ys: Vec<f64> = finite(&data.y).into_iter().chain(finite(&data.ry)).collect();
    if xs.is_empty() || ys.is_empty() {
        return;
    }
    let range = |v: &[f64]| {
        (v.iter().copied().fold(f64::INFINITY, f64::min), v.iter().copied().fold(f64::NEG_INFINITY, f64::max))
    };
    let ((x0, x1), (y0, y1)) = (range(&xs), range(&ys));
    let (dx, dy) = ((x1 - x0).max(1e-300), (y1 - y0).max(1e-300));
    let inner = rect.shrink2(egui::vec2(8.0, 14.0));
    let at = |x: f64, y: f64| {
        egui::pos2(
            inner.left() + ((x - x0) / dx) as f32 * inner.width(),
            inner.bottom() - ((y - y0) / dy) as f32 * inner.height(),
        )
    };
    let painter = ui.painter_at(rect);
    let line = |x: &[f64], y: &[f64], color: egui::Color32| {
        let n = x.len().min(y.len());
        let step = (n / 1500).max(1);
        let stroke = egui::Stroke::new(1.2_f32, color);
        let mut points = Vec::new();
        for i in (0..n).step_by(step) {
            if x[i].is_finite() && y[i].is_finite() {
                points.push(at(x[i], y[i]));
            } else {
                if points.len() > 1 {
                    painter.add(egui::Shape::line(std::mem::take(&mut points), stroke));
                }
                points.clear();
            }
        }
        if points.len() > 1 {
            painter.add(egui::Shape::line(points, stroke));
        }
    };
    line(&data.x, &data.y, egui::Color32::from_rgb(130, 130, 130));
    line(&data.rx, &data.ry, egui::Color32::from_rgb(220, 0, 0));
    let small = egui::FontId::proportional(9.0);
    let ink = egui::Color32::from_rgb(70, 70, 70);
    let (lb, rb, lt) = (rect.left_bottom(), rect.right_bottom(), rect.left_top());
    painter.text(lb + egui::vec2(4.0, -1.0), egui::Align2::LEFT_BOTTOM, format!("{x0:.4}"), small.clone(), ink);
    painter.text(rb + egui::vec2(-4.0, -1.0), egui::Align2::RIGHT_BOTTOM, format!("{x1:.4}"), small.clone(), ink);
    let head = format!("{} grey, result red   y {y0:.4} .. {y1:.4}", data.label);
    painter.text(lt + egui::vec2(4.0, 1.0), egui::Align2::LEFT_TOP, head, small, ink);
}

fn boundary_label(b: Boundary) -> &'static str {
    match b {
        Boundary::None => "Shrink the window",
        Boundary::Reflect => "Mirror the data",
        Boundary::Repeat => "Repeat the end value",
        Boundary::Periodic => "Wrap around",
        Boundary::Extrapolate => "Continue the end line",
    }
}

pub enum FilterOutcome {
    Open,
    Closed,
    /// Rows kept, and the index of the sheet that holds them.
    Done(usize, usize),
}

/// The "Filter Rows" window: keep the rows of a sheet for which a condition holds.
pub struct FilterDialog {
    pub book: WindowId,
    sheet: usize,
    condition: String,
    to_new_sheet: bool,
    problem: Option<String>,
    focus: bool,
}

impl FilterDialog {
    pub fn new(project: &Project, book: WindowId) -> Option<Self> {
        let b = project.workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        Some(FilterDialog { book, sheet, condition: String::new(), to_new_sheet: true, problem: None, focus: true })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> FilterOutcome {
        let Some(sheet) = doc.project().workbook(self.book).and_then(|b| b.sheets.get(self.sheet)) else {
            return FilterOutcome::Closed;
        };
        let columns: Vec<(String, String)> =
            sheet.columns.iter().map(|c| (c.short_name.clone(), c.axis_title())).collect();
        let mut open = true;
        let mut run = false;
        let mut cancel = false;
        egui::Window::new("Filter rows")
            .id(egui::Id::new("filter_rows"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Keep the rows for which this condition holds:");
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.condition)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(420.0)
                        .hint_text("for example  col(B) > 0.5 && col(A) < 10"),
                );
                if self.focus {
                    edit.request_focus();
                    self.focus = false;
                }
                ui.horizontal_wrapped(|ui| {
                    ui.weak("Insert:");
                    for (short, label) in &columns {
                        if ui.small_button(short).on_hover_text(label).clicked() {
                            self.condition.push_str(&format!("col({short})"));
                        }
                    }
                });
                ui.weak("Comparisons: <  <=  >  >=  ==  !=    Combine with && (and), || (or), ! (not).");
                ui.add_space(4.0);
                ui.radio_value(&mut self.to_new_sheet, true, "Copy the kept rows to a new sheet");
                ui.radio_value(&mut self.to_new_sheet, false, "Remove the other rows from this sheet");
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), RichText::new(p));
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    run = ui.button("OK").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if run {
            let (book, sheet, condition, new_sheet) =
                (self.book, self.sheet, self.condition.clone(), self.to_new_sheet);
            match doc.edit("Filter rows", move |p| p.filter_sheet(book, sheet, &condition, new_sheet)) {
                Ok((kept, at)) => return FilterOutcome::Done(kept, at),
                Err(e) => self.problem = Some(e.to_string()),
            }
        }
        if open && !cancel { FilterOutcome::Open } else { FilterOutcome::Closed }
    }
}
