//! The windows of the Analysis menu (integrate, differentiate, smooth,
//! normalize, subtract baseline, interpolate, find peaks, statistics) and the
//! "Filter Rows" window of the Worksheet menu.
//!
//! One window type serves all analysis tools: it shows which columns the
//! tool will work on, the tool's own options, and the row range. OK runs the
//! tool as one undoable step.

use eframe::egui::{self, DragValue, RichText};
use sheaf_core::analysis::{
    AnovaSpec, BinSize, CorrelationSpec, FreqOptions, HistogramSpec, KruskalWallisSpec, MannWhitneySpec,
    MeansComparison, NormalitySpec, SignedRankKind, SignedRankSpec, TTestKind, TTestSpec, Tail, TwoWayAnovaSpec,
    TwoWayLayout,
};
use sheaf_core::analysis::{
    AreaType, BaselineSpec, Boundary, CurveMathSpec, CurveOp, CurveOperand, DiffOptions, IntegrateRangesSpec,
    IntegrateSpec, InterpMethod, InterpSpec, NormalizeMethod, PeakDirection, PeakFilter, PeakOptions, Report, Request,
    RowRange, SmoothMethod, SmoothOptions, SplineBoundary, Tool,
};
use sheaf_core::analysis::{
    ChainBaseline, ChainOptions, FindMethod, FindOptions, FitBaseline, LimitMode, PeakChainSpec, PeakFitSpec, PeakShape,
};
use sheaf_core::column::Designation;
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
    /// The tool ran; the window closes.
    Done(Box<Report>),
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
    boundary: Boundary,
    normalize: usize,
    divisor: f64,
    baseline: BaselineKind,
    level: f64,
    line: [f64; 4],
    interp: InterpSpec,
    interp_own_range: bool,
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
    /// Every column of the sheet with a description, for the factor and data choices of the two-way ANOVA
    /// (the factor columns may hold text).
    sheet_columns: Vec<(ColumnId, String)>,
    problem: Option<String>,
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
            boundary: Boundary::None,
            normalize: 0,
            divisor: 1.0,
            baseline: BaselineKind::Minimum,
            level: 0.0,
            line: [x_lo, 0.0, x_hi, 0.0],
            interp: InterpSpec { range: Some((x_lo, x_hi)), ..InterpSpec::default() },
            interp_own_range: false,
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
            sheet_columns,
            problem: None,
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
            AnalysisKind::Differentiate => Tool::Differentiate(self.diff),
            AnalysisKind::Smooth => Tool::Smooth(SmoothOptions {
                method: match self.smooth_kind {
                    SmoothKind::SavitzkyGolay => SmoothMethod::SavitzkyGolay { poly_order: self.smooth_poly },
                    SmoothKind::Average => SmoothMethod::AdjacentAveraging { weighted: false },
                    SmoothKind::Weighted => SmoothMethod::AdjacentAveraging { weighted: true },
                    SmoothKind::Percentile => SmoothMethod::Percentile { percent: self.smooth_percent },
                    SmoothKind::Fft => SmoothMethod::FftFilter,
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
            AnalysisKind::Statistics => Tool::Statistics,
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
                ui.weak("One row per range and column (area, absolute area, peak, width) goes to a new sheet.");
            }
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
                ui.weak("Area, peak position, height and width go to a new sheet.");
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
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Points of window");
                    ui.add(DragValue::new(&mut self.smooth_points).range(1..=100_001));
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
                ui.add_enabled_ui(self.smooth_kind != SmoothKind::Fft, |ui| {
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
                ui.horizontal(|ui| {
                    ui.label("Number of points");
                    ui.add(DragValue::new(&mut self.interp.npts).range(2..=10_000_000));
                });
                ui.checkbox(&mut self.interp_own_range, "Choose the X range of the result");
                ui.add_enabled_ui(self.interp_own_range, |ui| {
                    let (mut lo, mut hi) = self.interp.range.unwrap_or((0.0, 1.0));
                    ui.horizontal(|ui| {
                        ui.label("X from");
                        ui.add(DragValue::new(&mut lo).speed(0.01));
                        ui.label("to");
                        ui.add(DragValue::new(&mut hi).speed(0.01));
                    });
                    self.interp.range = Some((lo, hi));
                });
                ui.weak("The interpolated curves go to a new sheet.");
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
                    ui.weak("Peaks, areas and fit results go to a new sheet; the baseline goes beside the data.");
                } else {
                    ui.weak("Peak positions and heights go to a new sheet.");
                }
            }
            AnalysisKind::Statistics => {
                ui.weak("Count, mean, standard deviation, sum, minimum, quartiles and maximum go to a new sheet.");
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
                ui.weak("The statistic, p-value and confidence interval go to a new sheet.");
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
                ui.weak("Group statistics, the ANOVA table and the comparisons go to a new sheet.");
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
                ui.weak(
                    "The tables (statistics, ANOVA, comparisons) go to a new sheet. Sums of squares are adjusted for the \
                     other terms, as in Origin.",
                );
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
                ui.weak("Ranks, U, Z and the p-values go to a new sheet.");
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
                ui.weak("Both p-values go to a new sheet.");
            }
            AnalysisKind::KruskalWallis => {
                let st = &mut self.stats;
                ui.label("Every chosen column is a group.");
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
                ui.weak("Mean ranks, the chi-square statistic and its p-value go to a new sheet.");
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
                ui.weak("The matrices of coefficients and p-values go to a new sheet.");
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
                ui.weak("Bin centres, counts, cumulative counts and percentages go to a new sheet.");
            }
            AnalysisKind::Normality => {
                let st = &mut self.stats;
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
                ui.weak("Shapiro-Wilk, Kolmogorov-Smirnov, Lilliefors, Anderson-Darling and D'Agostino-Pearson.");
            }
        }
    }

    /// Draws the window and runs the tool when OK is pressed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> AnalysisOutcome {
        if self.columns.iter().any(|c| doc.project().column(*c).is_none()) {
            return AnalysisOutcome::Closed;
        }
        let mut open = true;
        let mut run = false;
        let mut cancel = false;
        egui::Window::new(self.kind.title())
            .id(egui::Id::new("analysis"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let count = self.columns.len();
                let what = if count == 1 { "Column".to_string() } else { format!("{count} columns:") };
                ui.label(format!("{what} {}", self.column_names));
                ui.separator();
                self.options(ui);
                ui.separator();
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
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    run = ui.button("OK").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });

        if run {
            match self.request() {
                Ok(request) => match doc.edit(request.tool.name(), |p| p.analyze(&request)) {
                    Ok(report) => return AnalysisOutcome::Done(Box::new(report)),
                    Err(e) => self.problem = Some(e.to_string()),
                },
                Err(e) => self.problem = Some(e),
            }
        }
        if open && !cancel { AnalysisOutcome::Open } else { AnalysisOutcome::Closed }
    }
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
