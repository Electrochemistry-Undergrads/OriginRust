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
use serde::{Deserialize, Serialize};
use sheaf_calc as calc;
use sheaf_expr::Expr;
use std::borrow::Cow;
use std::cmp::Ordering;

pub use sheaf_calc::{
    AreaType, Boundary, CurveOp, DiffOptions, Extrapolation, InterpMethod, LocalSpan, NormalizeMethod, PeakDirection,
    PeakFilter, PeakOptions, SmoothMethod, SmoothOptions, SplineBoundary,
};
pub use sheaf_calc::{AxisBins, Bin2dOptions, Bin2dStat, BinSize, DiscSort, FreqOptions, MeansComparison, Tail};
pub use sheaf_calc::{BaselineStat, ChainBaseline, ChainOptions, CrossingIndex, FindMethod, FindOptions, LimitMode};
pub use sheaf_calc::{
    CoherenceOptions, CoherenceWindow, DecimateFilter, DecimateOptions, FftFilterOptions, FftFilterType, FftOptions,
    FftWindow, IfftOptions, PowerNorm, TraceMethod, WindowCorrection,
};
pub use sheaf_calc::{EnvelopeOptions, EnvelopeType};
pub use sheaf_calc::{
    FilterError, FilterResponse, FirMethod, FirSpec, FirWindow, FreqUnit, IIR_MAX_ORDER, IirMethod, IirSpec,
    MAX_FIR_ORDER as FIR_MAX_ORDER, MagnitudeUnit,
};
pub use sheaf_fit::multipeak::{FitBaseline, PeakShape};

pub use curves::{AverageSpec, IntersectSpec};
pub use dist::{DistFitSpec, ProbPlotKind, ProbPlotSpec};
pub use multivariate::{KmeansSpec, PcaSpec};
pub use roc::RocSpec;
pub use sheaf_calc::Family as DistFamily;
pub use sheaf_calc::{AverageMethod, AverageOptions, CommonX, XRange};
pub use sheaf_fit::simulate::Spacing as SimulateSpacing;
pub use simulate::{SimulateCurveSpec, SimulateX};
pub use summary::{GraphArrangement, GraphSummarySpec};
pub use survival::KaplanMeierSpec;

/// The multi-peak fit of [`PeakChainSpec`]: a sum of peaks on `baseline`, one peak for every peak
/// the search found, started at the found centre, width and height. Peak `k` (0-based) has the
/// shape `shapes[k]`; peaks beyond the list have the shape `shape`.
#[derive(Debug, Clone, PartialEq)]
pub struct PeakFitSpec {
    pub shape: PeakShape,
    pub baseline: FitBaseline,
    /// A function per peak, in the order found (empty: every peak is `shape`).
    pub shapes: Vec<PeakShape>,
}

impl PeakFitSpec {
    /// Every peak of one shape.
    pub fn of(shape: PeakShape, baseline: FitBaseline) -> PeakFitSpec {
        PeakFitSpec { shape, baseline, shapes: Vec::new() }
    }

    /// The shape of each of `n` peaks.
    pub fn shapes_for(&self, n: usize) -> Vec<PeakShape> {
        (0..n).map(|k| self.shapes.get(k).copied().unwrap_or(self.shape)).collect()
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    /// `sf`). 0 chooses the factor from the data as Origin does (`sheaf_calc::automatic_smoothing`).
    pub smoothing: f64,
    /// Interpolate onto the values of this column instead of an even grid (Mathematics >
    /// Interpolate/Extrapolate Y from X, Origin's `interp1` with `ix` given); `npts` and `range` are
    /// then not used.
    pub at: Option<ColumnId>,
    /// Values outside the X range of the data: continue the end segment (Origin's default),
    /// missing, or repeat the end value (Origin's `option` extrap, miss, repeat).
    pub extrapolation: calc::Extrapolation,
}

impl Default for InterpSpec {
    fn default() -> Self {
        InterpSpec {
            method: InterpMethod::Linear,
            boundary: SplineBoundary::NotAKnot,
            npts: 100,
            range: None,
            smoothing: 0.0,
            at: None,
            extrapolation: calc::Extrapolation::Extrapolate,
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

/// What [`Tool::ColumnMath`] does (Origin's Simple Column Math, `vmathtool`): `y op operand` row by
/// row. A [`CurveOperand::Reference`] is used as a plain column here: its row `i` goes with row `i`
/// of the data, with no X values and no interpolation, and rows past its end count as 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnMathSpec {
    pub op: CurveOp,
    pub operand: CurveOperand,
}

/// What [`Tool::Convolution`] does (Origin's Signal Processing > Convolution, `conv`, measured;
/// analysis-tools.md 13): each input column convolved with the response column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConvolutionSpec {
    pub response: ColumnId,
    /// Circular instead of linear (Origin's `circular`).
    pub circular: bool,
    /// Divide the response by its Euclidean norm first (Origin's `norm`).
    pub normalize: bool,
    /// Sampling interval of the result's X; `None` is Origin's `<auto>`, the X step of the input.
    pub interval: Option<f64>,
}

/// What [`Tool::SignalCorrelation`] does (Origin's Signal Processing > Correlation, `corr1`,
/// measured; analysis-tools.md 13): each input column correlated with the other column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalCorrelationSpec {
    pub other: ColumnId,
    pub circular: bool,
    /// Divide by the square root of the product of the two sums of squares.
    pub normalize: bool,
    /// The lag step of the result's X (Origin: 1 unless given).
    pub interval: f64,
}

/// What [`Tool::LevelCrossing`] does (Origin's `levelcrossing` X-Function, measured;
/// analysis-tools.md 14).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelCrossingSpec {
    pub level: f64,
    /// The row given for a crossing between two rows (Origin's `type`).
    pub index: CrossingIndex,
    /// Drop both crossings of a pair whose rows are closer than this (Origin's `pts`, 1: keep all).
    pub min_gap: usize,
    /// Also write the direction, the row and the Y of each crossing (Origin's optional outputs);
    /// Origin's default writes only the X values.
    pub details: bool,
}

/// The number of the next `CrossingX<n>` column of a sheet, as Origin numbers them: one more than
/// the largest in use.
fn next_crossing_number(sheet: &Worksheet) -> usize {
    sheet
        .columns
        .iter()
        .filter_map(|c| c.long_name.strip_prefix("CrossingX")?.parse::<usize>().ok())
        .max()
        .map_or(1, |n| n + 1)
}

/// What [`Tool::Fft`] does (Origin's Signal Processing > FFT > FFT, `fft1`, measured;
/// analysis-tools.md 15).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FftSpec {
    pub options: FftOptions,
    /// Take the sampling interval from the input's X column (its mean step; 1 without X) instead of
    /// `options.interval`.
    pub auto_interval: bool,
}

/// What [`Tool::Ifft`] does (Origin's Signal Processing > FFT > IFFT, `ifft1`, measured;
/// analysis-tools.md 15.1): the input columns are real parts of a spectrum.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IfftSpec {
    /// The imaginary part, row by row with the input; none: 0.
    pub imaginary: Option<ColumnId>,
    pub options: IfftOptions,
    /// Take the sampling interval (the frequency step) from the input's X column.
    pub auto_interval: bool,
}

/// What [`Tool::Decimate`] does (Origin's Signal Processing > Decimation, `decimate`, measured;
/// analysis-tools.md 13.4): every `factor`-th point of each input, with its X, in a new sheet.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DecimateSpec {
    pub options: DecimateOptions,
}

/// What [`Tool::FftFilter`] does (Origin's Signal Processing > FFT Filters, `fft_filters`,
/// measured; analysis-tools.md 15.2).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FftFilterSpec {
    pub options: FftFilterOptions,
    /// Sampling interval; `None`: the X step of the input (1 without an X column).
    pub interval: Option<f64>,
}

/// What [`Tool::Envelope`] does (Origin's Signal Processing > Envelope, `envelope`, measured;
/// analysis-tools.md 13.2).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EnvelopeSpec {
    pub options: EnvelopeOptions,
}

/// What [`Tool::Coherence`] does (Origin's Signal Processing > Coherence, `cohere`, measured;
/// analysis-tools.md 13.3): the magnitude-squared coherence of each input column with the other
/// column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoherenceSpec {
    /// The second signal.
    pub other: ColumnId,
    /// Window, segment and FFT lengths, overlap and the sampling interval (Origin: 1 unless given,
    /// not taken from the X column).
    pub options: CoherenceOptions,
}

/// The most points [`Tool::TraceInterp`] makes; more are refused.
pub const MAX_TRACE_POINTS: usize = 10_000_000;

/// What [`Tool::TraceInterp`] does (Origin's Mathematics > Trace Interpolation, `interp1trace`,
/// measured; analysis-tools.md 16): the curve resampled at evenly spaced points along its path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceInterpSpec {
    pub method: TraceMethod,
    /// Number of points of the result (Origin's `npts`, default 100).
    pub points: usize,
}

impl Default for TraceInterpSpec {
    fn default() -> Self {
        Self { method: TraceMethod::Linear, points: 100 }
    }
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

/// Which test of variance [`VarTestSpec`] runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VarTestKind {
    /// Each chosen column is tested on its own against a hypothesized variance (chi-square test).
    OneSample { variance: f64 },
    /// Exactly two chosen columns; the ratio of their variances (first over second) against `ratio` (F test).
    TwoSample { ratio: f64 },
}

/// What [`Tool::VarTest`] does (Origin's One-Sample and Two-Sample Test for Variance).
#[derive(Debug, Clone, PartialEq)]
pub struct VarTestSpec {
    pub kind: VarTestKind,
    pub tail: Tail,
    /// Significance level, 0 to 1 (default 0.05).
    pub alpha: f64,
    /// Report the confidence intervals of `levels` (Origin's "Confidence Interval(s)", off by default).
    pub intervals: bool,
    /// Confidence levels in percent (Origin's default "90 95 99").
    pub levels: Vec<f64>,
}

impl Default for VarTestSpec {
    fn default() -> Self {
        VarTestSpec {
            kind: VarTestKind::OneSample { variance: 1.0 },
            tail: Tail::Two,
            alpha: 0.05,
            intervals: false,
            levels: vec![90.0, 95.0, 99.0],
        }
    }
}

/// Which test of proportions [`PropTestSpec`] runs. The counts are typed in (Origin's "Summarized" input).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropTestKind {
    /// `successes` of `size` against the proportion `proportion`.
    OneSample { successes: u64, size: u64, proportion: f64 },
    /// `(successes, size)` of each sample; the difference of the proportions (first minus second) against
    /// `difference`.
    TwoSample { first: (u64, u64), second: (u64, u64), difference: f64 },
}

/// What [`Tool::PropTest`] does (Origin's One-Sample and Two-Sample Proportion Test). It reads no data: the
/// chosen column only says which workbook the result sheet goes to.
#[derive(Debug, Clone, PartialEq)]
pub struct PropTestSpec {
    pub kind: PropTestKind,
    pub tail: Tail,
    pub alpha: f64,
    /// Report confidence intervals of `levels` percent.
    pub intervals: bool,
    pub levels: Vec<f64>,
    /// Normal approximation (default on).
    pub normal: bool,
    /// Exact (binomial; Fisher for two samples) test (default off).
    pub exact: bool,
    /// Two samples: the standard error of the z statistic from the pooled proportion (when the hypothesized
    /// difference is 0).
    pub pooled: bool,
    /// The "Power Analysis" group of Origin's dialogs.
    pub power: PropPowerSpec,
}

/// The "Power Analysis" group of the proportion dialogs: the power of the test at the observed sample sizes
/// ("Actual Power") and at typed sample sizes ("Hypothetical Power"), each at its own significance level.
#[derive(Debug, Clone, PartialEq)]
pub struct PropPowerSpec {
    pub actual: bool,
    pub hypothetical: bool,
    /// The significance level of the power analysis (Origin keeps it apart from the test's own level).
    pub alpha: f64,
    /// Hypothetical sample sizes (per sample for two samples).
    pub sizes: Vec<u64>,
}

impl Default for PropPowerSpec {
    fn default() -> Self {
        PropPowerSpec { actual: false, hypothetical: false, alpha: 0.05, sizes: vec![50, 100, 200] }
    }
}

impl Default for PropTestSpec {
    fn default() -> Self {
        PropTestSpec {
            kind: PropTestKind::OneSample { successes: 0, size: 0, proportion: 0.5 },
            tail: Tail::Two,
            alpha: 0.05,
            intervals: false,
            levels: vec![90.0, 95.0, 99.0],
            normal: true,
            exact: false,
            pooled: true,
            power: PropPowerSpec::default(),
        }
    }
}

/// What [`Tool::Anova`] does: one-way ANOVA with each chosen column as a group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnovaSpec {
    pub alpha: f64,
    /// Means comparison of all pairs of groups (of every group against the control for Dunnett); `None`
    /// leaves it out.
    pub comparison: Option<MeansComparison>,
    /// Dunnett's control group, counted from 0 in the order of the chosen columns; ignored by the other methods.
    pub control: usize,
}

impl Default for AnovaSpec {
    fn default() -> Self {
        AnovaSpec { alpha: 0.05, comparison: Some(MeansComparison::Tukey), control: 0 }
    }
}

/// What [`Tool::Histogram`] does: frequency counts of each chosen column.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HistogramSpec {
    pub freq: FreqOptions,
    /// Also make a graph with a column plot of the counts.
    pub graph: bool,
}

/// What [`Tool::Crosstab`] does: the first chosen column holds the levels of the row variable, the second
/// those of the column variable (text or numbers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrosstabSpec {
    /// Add each count's percentage of its row total.
    pub row_percent: bool,
    pub column_percent: bool,
    pub total_percent: bool,
    /// Add the counts expected under independence.
    pub expected: bool,
    /// Level for the sentence about the tests.
    pub alpha: f64,
}

impl Default for CrosstabSpec {
    fn default() -> Self {
        CrosstabSpec { row_percent: true, column_percent: true, total_percent: true, expected: true, alpha: 0.05 }
    }
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

/// What [`Tool::DiscreteFrequency`] does (Origin's Discrete Frequency, `discfreqs`): the distinct values of each
/// chosen column, text or numbers, with the quantities ticked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscreteFreqSpec {
    pub count: bool,
    pub relative: bool,
    pub cumulative: bool,
    /// Relative and cumulative frequency in percent (else as a fraction).
    pub percent: bool,
    pub case_sensitive: bool,
    pub sort: DiscSort,
}

impl Default for DiscreteFreqSpec {
    /// Origin's defaults: Count only, case-insensitive, by descending counts.
    fn default() -> Self {
        DiscreteFreqSpec {
            count: true,
            relative: false,
            cumulative: false,
            percent: false,
            case_sensitive: false,
            sort: DiscSort::CountDesc,
        }
    }
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

/// What [`Tool::Statistics`] (Origin's Statistics on Columns) writes: one flag per column of the result
/// table. The default is the original eleven columns; the others are the quantities that `sheaf_calc::stats`
/// computes as well and the confidence limits of the mean.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatisticsSpec {
    pub n: bool,
    pub missing: bool,
    pub mean: bool,
    pub sd: bool,
    pub sem: bool,
    pub sum: bool,
    pub min: bool,
    pub q1: bool,
    pub median: bool,
    pub q3: bool,
    pub max: bool,
    pub iqr: bool,
    pub variance: bool,
    pub skewness: bool,
    pub kurtosis: bool,
    /// Coefficient of variation (standard deviation divided by the mean).
    pub cv: bool,
    /// Lower and upper confidence limit of the mean at `level` (the interval of the one-sample t-test).
    pub confidence_limits: bool,
    /// Confidence level of the limits, for example 0.95.
    pub level: f64,
}

impl Default for StatisticsSpec {
    fn default() -> Self {
        StatisticsSpec {
            n: true,
            missing: true,
            mean: true,
            sd: true,
            sem: true,
            sum: true,
            min: true,
            q1: true,
            median: true,
            q3: true,
            max: true,
            iqr: false,
            variance: false,
            skewness: false,
            kurtosis: false,
            cv: false,
            confidence_limits: false,
            level: 0.95,
        }
    }
}

impl StatisticsSpec {
    /// Every quantity on.
    pub fn all() -> Self {
        StatisticsSpec {
            iqr: true,
            variance: true,
            skewness: true,
            kurtosis: true,
            cv: true,
            confidence_limits: true,
            ..Self::default()
        }
    }
}

/// What [`Tool::RowStatistics`] (Origin's Statistics on Rows, `rowstats`) adds: one new column per chosen
/// quantity, behind the last chosen column, with one number per worksheet row computed across the chosen
/// columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStatsSpec {
    pub mean: bool,
    pub sd: bool,
    pub sem: bool,
    pub min: bool,
    pub max: bool,
    pub n: bool,
    pub sum: bool,
}

impl Default for RowStatsSpec {
    /// Origin creates the mean and the standard deviation; the others are optional outputs of `rowstats`.
    fn default() -> Self {
        RowStatsSpec { mean: true, sd: true, sem: false, min: false, max: false, n: false, sum: false }
    }
}

/// What [`Tool::SignTest`] does: the paired sign test of exactly two columns. `Upper` is Origin's
/// "F(x) > G(y)".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignTestSpec {
    pub tail: Tail,
    pub alpha: f64,
}

impl Default for SignTestSpec {
    fn default() -> Self {
        SignTestSpec { tail: Tail::Two, alpha: 0.05 }
    }
}

/// What [`Tool::KsTwoSample`] does: two columns, two independent samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KsTwoSampleSpec {
    pub tail: Tail,
    pub alpha: f64,
}

impl Default for KsTwoSampleSpec {
    fn default() -> Self {
        KsTwoSampleSpec { tail: Tail::Two, alpha: 0.05 }
    }
}

/// What [`Tool::MedianTest`] does: every chosen column is a group (at least two).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MedianTestSpec {
    pub alpha: f64,
}

impl Default for MedianTestSpec {
    fn default() -> Self {
        MedianTestSpec { alpha: 0.05 }
    }
}

/// What [`Tool::Friedman`] does: every chosen column is a treatment, every row a block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FriedmanSpec {
    pub alpha: f64,
}

impl Default for FriedmanSpec {
    fn default() -> Self {
        FriedmanSpec { alpha: 0.05 }
    }
}

/// What [`Tool::Grubbs`] and [`Tool::Dixon`] do: one outlier test per chosen column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlierSpec {
    /// Significance level. Dixon's table has 0.1, 0.05 and 0.01 only.
    pub alpha: f64,
}

impl Default for OutlierSpec {
    fn default() -> Self {
        OutlierSpec { alpha: 0.05 }
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
    ColumnMath(ColumnMathSpec),
    Convolution(ConvolutionSpec),
    SignalCorrelation(SignalCorrelationSpec),
    LevelCrossing(LevelCrossingSpec),
    /// Signal Processing > Hilbert Transform (Origin's `hilbert`, measured; analysis-tools.md 13).
    Hilbert,
    Fft(FftSpec),
    Ifft(IfftSpec),
    FftFilter(FftFilterSpec),
    /// Signal Processing > IIR Filter (Origin's `dfilter`, OriginPro; documented, not measured; analysis-tools.md 17).
    IirFilter(IirSpec),
    /// Signal Processing > FIR Filter (Origin 2027's FIR Filter app; documented, not measured; analysis-tools.md 17).
    FirFilter(FirSpec),
    Envelope(EnvelopeSpec),
    Coherence(CoherenceSpec),
    Decimate(DecimateSpec),
    /// Mathematics > Trace Interpolation (Origin's `interp1trace`, measured; analysis-tools.md 16).
    TraceInterp(TraceInterpSpec),
    /// Data Manipulation: subtract the straight line through two points.
    SubtractLine(SubtractLineSpec),
    /// Data Manipulation: reduce duplicate X data.
    ReduceDup(ReduceDupSpec),
    /// Data Manipulation: reduce to evenly spaced X.
    ReduceX(ReduceOptions),
    /// Data Manipulation: curve translate X or Y.
    Translate(TranslateSpec),
    /// Mathematics: running median.
    MedianFilter(MedianFilterSpec),
    Differentiate(DiffOptions),
    Smooth(SmoothOptions),
    Normalize(NormalizeMethod),
    SubtractBaseline(BaselineSpec),
    Interpolate(InterpSpec),
    FindPeaks(PeakOptions),
    PeakChain(PeakChainSpec),
    Statistics(StatisticsSpec),
    TTest(TTestSpec),
    VarTest(VarTestSpec),
    PropTest(PropTestSpec),
    Anova(AnovaSpec),
    Histogram(HistogramSpec),
    /// Statistics > Descriptive Statistics > 2D Frequency Counts/Binning (Origin's `twoDBinning`): the
    /// points of a Y column against its X column, or of two chosen columns (X, then Y), counted in a grid
    /// of bins. A third chosen column is ignored, as in Origin.
    Binning2D(Bin2dOptions),
    /// Statistics > Descriptive Statistics > Discrete Frequency.
    DiscreteFrequency(DiscreteFreqSpec),
    Normality(NormalitySpec),
    Crosstab(CrosstabSpec),
    TwoWayAnova(TwoWayAnovaSpec),
    MannWhitney(MannWhitneySpec),
    SignedRank(SignedRankSpec),
    KruskalWallis(KruskalWallisSpec),
    Correlation(CorrelationSpec),
    RowStatistics(RowStatsSpec),
    SignTest(SignTestSpec),
    KsTwoSample(KsTwoSampleSpec),
    MedianTest(MedianTestSpec),
    Friedman(FriedmanSpec),
    Grubbs(OutlierSpec),
    Dixon(OutlierSpec),
    /// Statistics > Descriptive Statistics > Distribution Fit (T14, `docs/spec/statistics.md` section 16).
    DistFit(DistFitSpec),
    /// Probability plot and Q-Q plot against a distribution family; makes a graph.
    ProbPlot(ProbPlotSpec),
    /// Statistics > Multivariate Analysis > Principal Component Analysis (T29, `docs/spec/statistics.md` section 17).
    Pca(PcaSpec),
    /// Statistics > Multivariate Analysis > K-Means Cluster Analysis (T29, `docs/spec/statistics.md` section 18).
    Kmeans(KmeansSpec),
    /// Statistics > ROC Curve (T29, `docs/spec/statistics.md` section 19): test variables, then the state column.
    Roc(RocSpec),
    /// Statistics > Survival Analysis > Kaplan-Meier Estimator (T29, `docs/spec/statistics.md` section 20): time,
    /// censor and, optionally, group columns.
    KaplanMeier(KaplanMeierSpec),
    /// Statistics > Descriptive Statistics > Graphical Summary (T40; `docs/spec/statistics.md` section 22).
    GraphSummary(GraphSummarySpec),
    AverageCurves(AverageSpec),
    Intersect(IntersectSpec),
    PolygonArea,
    /// Analysis > Fitting > Simulate Curve (T47, `docs/spec/fitting.md` section 13): a built-in fit function at typed
    /// parameters, as an X and a Y column on a new sheet. Reads no data except an optional X column.
    SimulateCurve(SimulateCurveSpec),
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
            Tool::ColumnMath(_) => "Column math",
            Tool::Convolution(_) => "Convolution",
            Tool::SignalCorrelation(_) => "Signal correlation",
            Tool::LevelCrossing(_) => "Level crossing",
            Tool::Hilbert => "Hilbert transform",
            Tool::Fft(_) => "FFT",
            Tool::Ifft(_) => "IFFT",
            Tool::FftFilter(_) => "FFT filter",
            Tool::IirFilter(_) => "IIR filter",
            Tool::FirFilter(_) => "FIR filter",
            Tool::Envelope(_) => "Envelope",
            Tool::Coherence(_) => "Coherence",
            Tool::Decimate(_) => "Decimation",
            Tool::TraceInterp(_) => "Trace interpolation",
            Tool::SubtractLine(_) => "Subtract straight line",
            Tool::ReduceDup(_) => "Reduce duplicate X data",
            Tool::ReduceX(_) => "Reduce to evenly spaced X",
            Tool::Translate(spec) => match spec.axis {
                TranslateAxis::X => "Translate X",
                TranslateAxis::Y => "Translate Y",
            },
            Tool::MedianFilter(_) => "Median filter",
            Tool::Differentiate(_) => "Differentiate",
            Tool::Smooth(_) => "Smooth",
            Tool::Normalize(_) => "Normalize",
            Tool::SubtractBaseline(_) => "Subtract baseline",
            Tool::Interpolate(_) => "Interpolate",
            Tool::FindPeaks(_) => "Find peaks",
            Tool::PeakChain(_) => "Analyze peaks",
            Tool::Statistics(_) => "Statistics",
            Tool::TTest(spec) => match spec.kind {
                TTestKind::OneSample { .. } => "One-sample t-test",
                TTestKind::TwoSample { .. } => "Two-sample t-test",
                TTestKind::Paired { .. } => "Paired t-test",
            },
            Tool::Anova(_) => "One-way ANOVA",
            Tool::Histogram(_) => "Histogram",
            Tool::Binning2D(_) => "2D frequency counts",
            Tool::DiscreteFrequency(_) => "Discrete frequency",
            Tool::Normality(_) => "Normality test",
            Tool::Crosstab(_) => "Cross tabulation",
            Tool::TwoWayAnova(_) => "Two-way ANOVA",
            Tool::MannWhitney(_) => "Mann-Whitney test",
            Tool::SignedRank(spec) => match spec.kind {
                SignedRankKind::OneSample { .. } => "One-sample Wilcoxon signed rank test",
                SignedRankKind::Paired => "Paired Wilcoxon signed rank test",
            },
            Tool::KruskalWallis(_) => "Kruskal-Wallis test",
            Tool::Correlation(_) => "Correlation coefficients",
            Tool::RowStatistics(_) => "Statistics on rows",
            Tool::SignTest(_) => "Sign test",
            Tool::KsTwoSample(_) => "Two-sample Kolmogorov-Smirnov test",
            Tool::MedianTest(_) => "Median test",
            Tool::Friedman(_) => "Friedman ANOVA",
            Tool::Grubbs(_) => "Grubbs test",
            Tool::Dixon(_) => "Dixon's Q test",
            Tool::VarTest(spec) => match spec.kind {
                VarTestKind::OneSample { .. } => "One-sample test for variance",
                VarTestKind::TwoSample { .. } => "Two-sample test for variance",
            },
            Tool::PropTest(spec) => match spec.kind {
                PropTestKind::OneSample { .. } => "One-sample proportion test",
                PropTestKind::TwoSample { .. } => "Two-sample proportion test",
            },
            Tool::DistFit(_) => "Distribution fit",
            Tool::ProbPlot(spec) => match spec.kind {
                ProbPlotKind::Probability => "Probability plot",
                ProbPlotKind::QQ => "Q-Q plot",
            },
            Tool::Pca(_) => "Principal component analysis",
            Tool::Kmeans(_) => "K-means cluster analysis",
            Tool::Roc(_) => "ROC curve",
            Tool::KaplanMeier(_) => "Kaplan-Meier estimator",
            Tool::GraphSummary(_) => "Graphical summary",
            Tool::AverageCurves(_) => "Average multiple curves",
            Tool::Intersect(_) => "Intersect",
            Tool::PolygonArea => "Polygon area",
            Tool::SimulateCurve(_) => "Simulate curve",
        }
    }

    fn sheet_prefix(&self) -> &'static str {
        match self {
            Tool::Integrate(_) | Tool::IntegrateRanges(_) => "Integrate",
            Tool::CurveMath(_) | Tool::ColumnMath(_) => "Math",
            Tool::Convolution(_) => "Convolution",
            Tool::SignalCorrelation(_) => "Correlation",
            Tool::LevelCrossing(_) => "LevelCrossing",
            Tool::Hilbert => "Hilbert",
            Tool::Fft(_) => "FFT",
            Tool::Ifft(_) => "IFFT",
            Tool::FftFilter(_) => "FFTFilter",
            Tool::IirFilter(_) => "IIRFilter",
            Tool::FirFilter(_) => "FIRFilter",
            Tool::Envelope(_) => "Envelope",
            Tool::Coherence(_) => "Coherence",
            Tool::Decimate(_) => "Decimate",
            Tool::TraceInterp(_) => "TraceInterp",
            Tool::ReduceX(_) => "Reduced",
            Tool::ReduceDup(_) => "ReduceDup",
            Tool::Translate(_) => "Translate",
            Tool::Interpolate(_) => "Interpolate",
            Tool::FindPeaks(_) => "Peaks",
            Tool::PeakChain(_) => "PeakAnalysis",
            Tool::Statistics(_) => "Statistics",
            Tool::TTest(_) => "TTest",
            Tool::VarTest(_) => "VarTest",
            Tool::PropTest(_) => "PropTest",
            Tool::Anova(_) => "ANOVA",
            Tool::Histogram(_) => "Histogram",
            Tool::Binning2D(_) => "Binning2D",
            Tool::DiscreteFrequency(_) => "DiscretFreq",
            Tool::Normality(_) => "Normality",
            Tool::Crosstab(_) => "Crosstab",
            Tool::TwoWayAnova(_) => "ANOVA2Way",
            Tool::MannWhitney(_) => "MannWhitney",
            Tool::SignedRank(_) => "SignedRank",
            Tool::KruskalWallis(_) => "KruskalWallis",
            Tool::Correlation(_) => "Correlation",
            Tool::SignTest(_) => "SignTest",
            Tool::KsTwoSample(_) => "KolmogorovSmirnov",
            Tool::MedianTest(_) => "MedianTest",
            Tool::Friedman(_) => "Friedman",
            Tool::Grubbs(_) => "Grubbs",
            Tool::Dixon(_) => "Dixon",
            Tool::DistFit(_) => "DistFit",
            Tool::ProbPlot(spec) => match spec.kind {
                ProbPlotKind::Probability => "ProbPlot",
                ProbPlotKind::QQ => "QQPlot",
            },
            Tool::Pca(_) => "PCA",
            Tool::Kmeans(_) => "KMeans",
            Tool::Roc(_) => "ROC",
            Tool::KaplanMeier(_) => "KaplanMeier",
            Tool::GraphSummary(_) => "GraphSummary",
            Tool::AverageCurves(_) => "AverageResult",
            Tool::Intersect(_) => "Intersect",
            Tool::PolygonArea => "PolygonArea",
            Tool::SimulateCurve(_) => "Simulate",
            _ => "Result",
        }
    }
}

/// Where the one-value-per-row results of a tool go (the Output node of the analysis windows). Tools
/// that write a table of numbers always use a new sheet.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OutputTarget {
    /// New columns directly after each input column (ADR 0005).
    #[default]
    NextToInput,
    /// A new sheet in the same workbook, with an X and a Y column per input column.
    NewSheet,
    /// Existing columns of the input sheet, one per result column in order; their values and
    /// labels are replaced.
    Existing(Vec<ColumnId>),
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

/// Applies the looks of a planned graph (fixed limits, lines through zero, colours, labels) to its first layer.
fn apply_look(l: &mut crate::graph::Layer, look: &dist::Look, ids: &[ColumnId]) {
    if let Some((a, b)) = look.x_range {
        l.x_axis.min = Some(a);
        l.x_axis.max = Some(b);
    }
    if let Some((a, b)) = look.y_range {
        l.y_axis.min = Some(a);
        l.y_axis.max = Some(b);
    }
    if let Some(a) = look.y_min {
        l.y_axis.min = Some(a);
    }
    l.x_axis.major_step = look.x_step.or(l.x_axis.major_step);
    l.y_axis.major_step = look.y_step.or(l.y_axis.major_step);
    if look.no_legend {
        l.legend.visible = false;
    }
    if look.cross_lines {
        l.x_axis.additional_line = Some(0.0);
        l.y_axis.additional_line = Some(0.0);
    }
    if look.box_frame {
        for axis in [&mut l.x_axis, &mut l.y_axis] {
            axis.show_opposite = true;
            axis.opposite_ticks = false;
        }
    }
    for &(i, color) in &look.colors {
        if let Some(p) = l.plots.get_mut(i) {
            p.line.color = color;
            p.symbol.color = color;
            p.auto_color = false;
        }
    }
    for &(i, shape, size, color) in &look.symbols {
        if let Some(p) = l.plots.get_mut(i) {
            p.symbol.shape = shape;
            p.symbol.size_pt = size;
            if let Some(c) = color {
                p.symbol.color = c;
                p.auto_color = false;
            }
        }
    }
    for &i in &look.dotted {
        if let Some(p) = l.plots.get_mut(i) {
            p.line.dash = crate::graph::Dash::Dot;
        }
    }
    for &(i, col, color) in &look.labels {
        if let Some(p) = l.plots.get_mut(i) {
            let mut font = crate::graph::Font::new("Arial", 18.0);
            font.color = color;
            p.decor.labels = crate::graph::DataLabels {
                enable: true,
                form: crate::graph::LabelForm::Column { column: ids[col] },
                position: crate::graph::LabelPosition::Right,
                font,
                ..Default::default()
            };
        }
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
    /// Further graphs that the tool added (principal component analysis draws up to three), in drawing order.
    pub more_graphs: Vec<WindowId>,
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

fn local_span_text(span: calc::LocalSpan, npts: usize) -> String {
    match span {
        calc::LocalSpan::Proportion(p) => format!("span {p}"),
        calc::LocalSpan::Points => format!("{npts} points"),
    }
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
/// The values of a signal for Convolution and Correlation: trailing missing values dropped, a
/// missing value inside refused.
fn signal_values(v: &[f64], name: &str) -> Result<Vec<f64>, AnalysisError> {
    let end = v.iter().rposition(|x| !x.is_nan()).map_or(0, |i| i + 1);
    let v = &v[..end];
    if v.is_empty() {
        return Err(AnalysisError::TooFewPoints(name.to_string()));
    }
    if v.iter().any(|x| !x.is_finite()) {
        return Err(AnalysisError::BadOption(format!("column {name} has missing values")));
    }
    Ok(v.to_vec())
}

/// "2nd", "3rd", "12th" for the comment of a decimated column.
fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix} point")
}

/// Origin's `<auto>` sampling interval: the mean X step of the input, 1 without a usable X.
fn auto_interval(x: &[f64]) -> f64 {
    let first = x.iter().position(|v| v.is_finite());
    let last = x.iter().rposition(|v| v.is_finite());
    match (first, last) {
        (Some(a), Some(b)) if b > a => {
            let d = (x[b] - x[a]) / (b - a) as f64;
            if d.is_finite() && d > 0.0 { d } else { 1.0 }
        }
        _ => 1.0,
    }
}

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
    /// Short name of the X column ("Row" without one).
    x_short: String,
    x_units: String,
    /// X values, as long as `y` (row numbers when the sheet has no X column).
    x: Vec<f64>,
    y: Cow<'a, [f64]>,
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

/// The table of Statistics on Columns: one row per input column, the columns chosen in `spec`.
fn statistics_table(spec: &StatisticsSpec, inputs: &[Input<'_>]) -> Result<ResultTable, AnalysisError> {
    if spec.confidence_limits && !(spec.level > 0.0 && spec.level < 1.0) {
        return Err(AnalysisError::BadOption(format!(
            "the confidence level must be between 0 and 1, not {}",
            spec.level
        )));
    }
    let percent = format!("{}", (spec.level * 100.0 * 1e6).round() / 1e6);
    // (header, is a count, value of one column)
    type Pick = fn(&calc::Stats) -> f64;
    let mut picks: Vec<(String, bool, Pick)> = Vec::new();
    let mut add = |on: bool, header: &str, count: bool, f: Pick| {
        if on {
            picks.push((header.to_string(), count, f));
        }
    };
    add(spec.n, "N", true, |s| s.n as f64);
    add(spec.missing, "Missing", true, |s| s.missing as f64);
    add(spec.mean, "Mean", false, |s| s.mean);
    add(spec.sd, "SD", false, |s| s.sd);
    add(spec.sem, "SEM", false, |s| s.sem);
    add(spec.sum, "Sum", false, |s| s.sum);
    add(spec.min, "Minimum", false, |s| s.min);
    add(spec.q1, "Q1", false, |s| s.q1);
    add(spec.median, "Median", false, |s| s.median);
    add(spec.q3, "Q3", false, |s| s.q3);
    add(spec.max, "Maximum", false, |s| s.max);
    add(spec.iqr, "IQR", false, |s| s.iqr);
    add(spec.variance, "Variance", false, |s| s.variance);
    add(spec.skewness, "Skewness", true, |s| s.skewness);
    add(spec.kurtosis, "Kurtosis", true, |s| s.kurtosis);
    add(spec.cv, "CV", true, |s| s.cv);
    let headers: Vec<String> = picks
        .iter()
        .map(|p| p.0.clone())
        .chain(
            spec.confidence_limits
                .then(|| [format!("Lower {percent}% CL of Mean"), format!("Upper {percent}% CL of Mean")])
                .into_iter()
                .flatten(),
        )
        .collect();
    let mut t = ResultTable { headers, ..Default::default() };
    for inp in inputs {
        let s = calc::stats(inp.ys());
        let mut values: Vec<f64> = picks.iter().map(|p| (p.2)(&s)).collect();
        // The unit goes with the numbers that carry the data's unit: not the counts, skewness, kurtosis
        // and the coefficient of variation, which have none, and not the variance (unit squared).
        let mut units: Vec<String> =
            picks.iter().map(|p| if p.1 || p.0 == "Variance" { String::new() } else { inp.units.clone() }).collect();
        if spec.confidence_limits {
            let (lcl, ucl) = calc::mean_confidence_limits(&s, spec.level);
            values.extend([lcl, ucl]);
            units.extend([inp.units.clone(), inp.units.clone()]);
        }
        t.push_row(inp.label.clone(), values, units);
    }
    Ok(t)
}

/// Statistics on Rows: new columns behind the last input column, one number per worksheet row.
fn row_statistics(
    spec: &RowStatsSpec,
    inputs: &[Input<'_>],
    beside: &mut Vec<NewColumn>,
    notes: &mut Vec<String>,
) -> Result<(), AnalysisError> {
    type Pick = fn(&calc::RowStats) -> f64;
    let wanted: [(bool, &str, bool, Pick); 7] = [
        (spec.mean, "Mean", false, |r| r.mean),
        (spec.sd, "Standard Deviation", false, |r| r.sd),
        (spec.sem, "SE of Mean", false, |r| r.sem),
        (spec.min, "Minimum", false, |r| r.min),
        (spec.max, "Maximum", false, |r| r.max),
        (spec.n, "Number of Points", true, |r| r.n),
        (spec.sum, "Sum", false, |r| r.sum),
    ];
    if !wanted.iter().any(|w| w.0) {
        return Err(AnalysisError::BadOption("choose at least one quantity for Statistics on Rows".to_string()));
    }
    if inputs.len() < 2 {
        notes.push("Only one column was chosen: every row has one number, so there is no spread.".to_string());
    }
    let rows = inputs.iter().map(|i| i.y.len()).max().unwrap_or(0);
    let mut cells = vec![Vec::with_capacity(inputs.len()); rows];
    for (r, row) in cells.iter_mut().enumerate() {
        for inp in inputs {
            let inside = r >= inp.first && r <= inp.last;
            row.push(if inside { inp.y.get(r).copied().unwrap_or(f64::NAN) } else { f64::NAN });
        }
    }
    let stats: Vec<calc::RowStats> = cells.iter().map(|row| calc::row_stats(row)).collect();
    let units = match inputs.first() {
        Some(f) if inputs.iter().all(|i| i.units == f.units) => f.units.clone(),
        _ => String::new(),
    };
    let last = inputs.last().expect("at least one input").id;
    for (on, name, count, pick) in wanted {
        if on {
            beside.push(NewColumn {
                after: last,
                long_name: name.to_string(),
                units: if count { String::new() } else { units.clone() },
                comments: format!("across {} columns", inputs.len()),
                data: stats.iter().map(pick).collect(),
            });
        }
    }
    Ok(())
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
        if let Some(fs) = &spec.fit {
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
            let shapes = fs.shapes_for(starts.len());
            let mut func =
                sheaf_fit::multipeak::multi_peak_function_of(&shapes, fs.baseline, &starts).with_initial("y0", y0);
            if fs.baseline == FitBaseline::Linear {
                func = func.with_initial("m", m);
            }
            let report = sheaf_fit::nlsf::fit(&func, xs, ys, &sheaf_fit::nlsf::FitOptions::default()).map_err(|e| {
                AnalysisError::BadOption(format!("column {}: the multi-peak fit failed: {e}", inp.short))
            })?;
            let get = |name: String| report.param(&name).map_or(f64::NAN, |p| p.value);
            let mut total = 0.0;
            for (k, shape) in shapes.iter().enumerate() {
                let p: Vec<f64> = shape.param_names(k + 1).into_iter().map(get).collect();
                let peak = shape.summary(&p);
                total += peak.area;
                fitted_peaks.push([peak.center, peak.height, peak.fwhm, peak.area, 0.0]);
            }
            for p in fitted_peaks.iter_mut() {
                p[4] = if total != 0.0 { 100.0 * p[3] / total } else { f64::NAN };
            }
            notes.push(format!(
                "{}: {} fit of {} peaks, R^2 = {:.6}, reduced chi-square = {:.6e}, {} iterations{}",
                inp.label,
                sheaf_fit::multipeak::shapes_name(&shapes, "+"),
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
                comments: format!(
                    "{} multi-peak fit of {}{}",
                    sheaf_fit::multipeak::shapes_name(&shapes, "+"),
                    inp.short,
                    inp.range_note()
                ),
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

mod categorical;
mod curves;
mod dist;
mod filters;
mod manip;
mod multivariate;
mod power;
mod roc;
mod simulate;
mod statistics;
mod summary;
mod survival;
pub use manip::{
    MedianFilterSpec, ReduceDupSpec, Shift, SubtractLineSpec, TranslateAxis, TranslateSpec, dup_method_name,
};
pub use power::{PowerGoal, PowerKind, PowerOutput, PowerSpec, power_analysis};
pub use sheaf_calc::{
    DUP_TOL, DupMethod, EvenGrid, LinePoint, MedianEdge, REDUCE_AUTO_FACTOR, ReduceGroup, ReduceMethod, ReduceOptions,
};

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

    /// A new graph with the scatter and line plots of a probability plot; the plan names columns of the
    /// new result sheet by position.
    fn xy_graph(&mut self, ids: &[ColumnId], plan: &dist::XyGraph) -> WindowId {
        let long_name = ids.first().and_then(|c| self.locate(*c)).map(|(b, _, _)| b.long_name.clone());
        let graph = self.add_graph(crate::template::Template::Scatter);
        for p in &plan.plots {
            self.add_plot_xy(graph, 0, Some(ids[p.x]), ids[p.y], None).expect("the graph and the new columns exist");
        }
        if let Some(g) = self.graph_mut(graph) {
            if let Some(l) = g.layers.first_mut() {
                for (plot, spec) in l.plots.iter_mut().zip(&plan.plots) {
                    plot.kind = spec.kind;
                    plot.label = Some(spec.label.clone());
                }
                l.apply_palette();
                // A reference line takes the colour of the points it belongs to.
                for (i, spec) in plan.plots.iter().enumerate() {
                    if let Some(of) = spec.line_of {
                        let colour = l.plots[of].symbol.color;
                        l.plots[i].line.color = colour;
                        l.plots[i].symbol.color = colour;
                        l.plots[i].auto_color = false;
                    }
                }
                l.x_axis.scale = plan.x_scale;
                l.y_axis.scale = plan.y_scale;
                l.x_axis.title = Some(plan.x_title.clone());
                l.y_axis.title = Some(plan.y_title.clone());
                apply_look(l, &plan.look, ids);
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
            if let Some(title) = &plan.look.title {
                g.long_name = title.clone();
            }
        }
        if let Some(ov) = &plan.look.overlay {
            self.overlay_layer(graph, ids, ov);
        }
        graph
    }

    /// The second layer of a graph that has plots of its own against axes on the top and right edges (the
    /// loading vectors of a biplot).
    fn overlay_layer(&mut self, graph: WindowId, ids: &[ColumnId], ov: &dist::Overlay) {
        use crate::graph::{AxisSide, LayerLink, ScaleLink};
        let Some(parent) = self.graph(graph).and_then(|g| g.layers.first()).cloned() else { return };
        let mut layer = crate::template::right_y_layer(&parent, 0);
        layer.link = Some(LayerLink { parent: 0, x: ScaleLink::None, y: ScaleLink::None });
        layer.x_axis.side = AxisSide::Opposite;
        layer.x_axis.min = Some(ov.x_range.0);
        layer.x_axis.max = Some(ov.x_range.1);
        layer.y_axis.min = Some(ov.y_range.0);
        layer.y_axis.max = Some(ov.y_range.1);
        layer.x_axis.major_step = None;
        layer.y_axis.major_step = None;
        layer.x_axis.title = Some(String::new());
        layer.y_axis.title = Some(String::new());
        layer.x_axis.show_title = false;
        layer.y_axis.show_title = false;
        for axis in [&mut layer.x_axis, &mut layer.y_axis] {
            axis.color = ov.color;
            axis.tick_font.color = ov.color;
            axis.additional_line = None;
        }
        if let Some(g) = self.graph_mut(graph) {
            g.layers.push(layer);
        }
        for p in &ov.plots {
            self.add_plot_xy(graph, 1, Some(ids[p.x]), ids[p.y], None).expect("the graph and the new columns exist");
        }
        if let Some(l) = self.graph_mut(graph).and_then(|g| g.layers.get_mut(1)) {
            for (plot, spec) in l.plots.iter_mut().zip(&ov.plots) {
                plot.kind = spec.kind;
                plot.label = Some(spec.label.clone());
                plot.line.color = ov.color;
                plot.line.width_pt = 0.5;
                plot.symbol.color = ov.color;
                plot.auto_color = false;
            }
            for &(i, col) in &ov.labels {
                if let Some(plot) = l.plots.get_mut(i) {
                    plot.decor.labels = crate::graph::DataLabels {
                        enable: true,
                        form: crate::graph::LabelForm::Column { column: ids[col] },
                        position: crate::graph::LabelPosition::Right,
                        font: {
                            let mut f = crate::graph::Font::new("Arial", 18.0);
                            f.color = ov.color;
                            f
                        },
                        ..Default::default()
                    };
                }
            }
        }
    }

    /// Runs an analysis tool on worksheet columns. See the module text for
    /// where the results go. On error nothing is changed.
    pub fn analyze(&mut self, request: &Request) -> Result<Report, AnalysisError> {
        self.analyze_to(request, &OutputTarget::NextToInput)
    }

    /// Like [`Project::analyze`], with a choice of where the one-value-per-row results go.
    pub fn analyze_to(&mut self, request: &Request, output: &OutputTarget) -> Result<Report, AnalysisError> {
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
        let own_columns = matches!(
            request.tool,
            Tool::TwoWayAnova(_)
                | Tool::Crosstab(_)
                | Tool::DiscreteFrequency(_)
                | Tool::PropTest(_)
                | Tool::Roc(_)
                | Tool::KaplanMeier(_)
                | Tool::GraphSummary(_)
                | Tool::SimulateCurve(_)
        );
        for id in request.columns.iter().filter(|_| !own_columns) {
            let index = sheet.column_index(*id).ok_or_else(|| {
                if self.column(*id).is_some() {
                    AnalysisError::DifferentSheets
                } else {
                    AnalysisError::NoSuchColumn(*id)
                }
            })?;
            let c = &sheet.columns[index];
            let y = sheet.values_of(c).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
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
                x_short: x_col.map_or_else(|| "Row".to_string(), |x| x.short_name.clone()),
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
        // A graph of scatter and line plots (probability plots).
        let mut xy_graph: Option<dist::XyGraph> = None;
        // Further graphs of the same tool (loading plot, biplot).
        let mut more_xy_graphs: Vec<dist::XyGraph> = Vec::new();
        // The graphs of a graphical summary (three layers each).
        let mut summary_graphs: Vec<summary::SummaryGraph> = Vec::new();

        match &request.tool {
            Tool::TwoWayAnova(spec) => {
                let out = statistics::two_way_anova(spec, sheet, &request.columns, request.rows)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
            }
            Tool::Crosstab(spec) => {
                let out = categorical::crosstab(spec, sheet, &request.columns, request.rows)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
            }
            Tool::DiscreteFrequency(spec) => {
                let out = statistics::discrete_frequency(spec, sheet, &request.columns, request.rows)?;
                on_sheet = out.on_sheet;
                notes = out.notes;
            }
            Tool::PropTest(spec) => {
                let out = statistics::prop_test(spec)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
            }
            Tool::SimulateCurve(spec) => {
                let out = simulate::run(spec, self)?;
                on_sheet = out.on_sheet;
                notes = out.notes;
                xy_graph = out.graph;
            }
            Tool::TTest(_)
            | Tool::VarTest(_)
            | Tool::Anova(_)
            | Tool::Histogram(_)
            | Tool::Binning2D(_)
            | Tool::Normality(_)
            | Tool::MannWhitney(_)
            | Tool::SignedRank(_)
            | Tool::KruskalWallis(_)
            | Tool::Correlation(_)
            | Tool::SignTest(_)
            | Tool::KsTwoSample(_)
            | Tool::MedianTest(_)
            | Tool::Friedman(_)
            | Tool::Grubbs(_)
            | Tool::Dixon(_) => {
                let out = statistics::run(&request.tool, &inputs)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
                graph_plots = out.plots;
            }
            Tool::DistFit(_) | Tool::ProbPlot(_) => {
                let out = dist::run(&request.tool, &inputs)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
                xy_graph = out.graph;
            }
            Tool::Roc(spec) => {
                let out = roc::run(spec, sheet, &request.columns, request.rows)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
                xy_graph = out.graph;
            }
            Tool::KaplanMeier(spec) => {
                let out = survival::run(spec, sheet, &request.columns, request.rows)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
                xy_graph = out.graph;
            }
            Tool::GraphSummary(spec) => {
                let out = summary::run(spec, sheet, &request.columns, request.rows)?;
                table = Some(out.table);
                on_sheet = out.on_sheet;
                notes = out.notes;
                summary_graphs = out.graphs;
            }
            Tool::Pca(_) | Tool::Kmeans(_) => {
                let out = multivariate::run(&request.tool, &inputs)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
                xy_graph = out.graph;
                more_xy_graphs = out.more_graphs;
            }
            Tool::AverageCurves(_) | Tool::Intersect(_) | Tool::PolygonArea => {
                let out = curves::run(&request.tool, &inputs)?;
                table = out.table;
                on_sheet = out.on_sheet;
                notes = out.notes;
            }
            Tool::SubtractLine(_)
            | Tool::ReduceDup(_)
            | Tool::ReduceX(_)
            | Tool::Translate(_)
            | Tool::MedianFilter(_) => {
                manip::run(&request.tool, &inputs, &mut beside, &mut on_sheet)?;
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
            Tool::Convolution(ConvolutionSpec { response: other, circular, normalize, interval: _ })
            | Tool::SignalCorrelation(SignalCorrelationSpec { other, circular, normalize, interval: _ }) => {
                let conv = matches!(request.tool, Tool::Convolution(_));
                let (_, _, ocol) = self.locate(*other).ok_or(AnalysisError::NoSuchColumn(*other))?;
                let oshort = ocol.short_name.clone();
                let ovalues =
                    signal_values(ocol.numeric().ok_or_else(|| AnalysisError::NotNumeric(oshort.clone()))?, &oshort)?;
                for inp in &inputs {
                    let y = signal_values(inp.ys(), &inp.short)?;
                    let (x, v, x_name, what) = if conv {
                        let Tool::Convolution(spec) = &request.tool else { unreachable!() };
                        let step = match spec.interval {
                            Some(d) if d.is_finite() && d > 0.0 => d,
                            Some(d) => return Err(AnalysisError::BadOption(format!("{d} is not a usable interval"))),
                            None => auto_interval(inp.xs()),
                        };
                        let v = calc::convolve(&y, &ovalues, *circular, *normalize);
                        let x = (0..v.len()).map(|k| k as f64 * step).collect();
                        (x, v, inp.x_name.clone(), "convolved with")
                    } else {
                        let Tool::SignalCorrelation(spec) = &request.tool else { unreachable!() };
                        if !(spec.interval.is_finite() && spec.interval > 0.0) {
                            return Err(AnalysisError::BadOption(format!(
                                "{} is not a usable interval",
                                spec.interval
                            )));
                        }
                        let (lags, v) = calc::correlate(&y, &ovalues, *circular, *normalize);
                        (lags.iter().map(|l| l * spec.interval).collect(), v, "Lag".to_string(), "correlated with")
                    };
                    let x_units = if conv { inp.x_units.clone() } else { String::new() };
                    on_sheet.push(SheetColumn {
                        designation: Designation::X,
                        long_name: x_name,
                        units: x_units,
                        comments: String::new(),
                        data: ColumnData::numeric(x),
                    });
                    on_sheet.push(SheetColumn {
                        designation: Designation::Y,
                        long_name: format!("{} {what} {}", inp.name, ocol.display_name()),
                        units: String::new(),
                        comments: format!(
                            "{} {what} {oshort}{}{}",
                            inp.short,
                            if *circular { ", circular" } else { "" },
                            if *normalize { ", normalized" } else { "" }
                        ),
                        data: ColumnData::numeric(v),
                    });
                }
            }
            Tool::Fft(spec) => {
                for inp in &inputs {
                    let y = signal_values(inp.ys(), &inp.short)?;
                    let mut o = spec.options;
                    if spec.auto_interval {
                        o.interval = auto_interval(inp.xs());
                    }
                    if !(o.interval.is_finite() && o.interval > 0.0) {
                        return Err(AnalysisError::BadOption(format!("{} is not a usable interval", o.interval)));
                    }
                    let r = calc::fft_spectrum(&y, &o);
                    let note = format!("FFT of {}{}", inp.short, inp.range_note());
                    let mut col = |designation, name: &str, units: &str, data: Vec<f64>| {
                        on_sheet.push(SheetColumn {
                            designation,
                            long_name: name.into(),
                            units: units.into(),
                            comments: note.clone(),
                            data: ColumnData::numeric(data),
                        })
                    };
                    col(Designation::X, "Frequency", "", r.frequency);
                    col(Designation::Y, "Real", "", r.real);
                    col(Designation::Y, "Imaginary", "", r.imaginary);
                    col(Designation::Y, "Magnitude", "", r.magnitude);
                    col(Designation::Y, "Amplitude", &inp.units, r.amplitude);
                    col(Designation::Y, "Phase", "Degrees", r.phase);
                    col(Designation::Y, "Power", "", r.power);
                    col(Designation::Y, "dB", "dB", r.db);
                }
            }
            Tool::FftFilter(spec) => {
                for inp in &inputs {
                    let y = signal_values(inp.ys(), &inp.short)?;
                    let dt = spec.interval.unwrap_or_else(|| auto_interval(inp.xs()));
                    if !(dt.is_finite() && dt > 0.0) {
                        return Err(AnalysisError::BadOption(format!("{dt} is not a usable interval")));
                    }
                    let word = match spec.options.kind {
                        FftFilterType::LowPass => "low pass",
                        FftFilterType::HighPass => "high pass",
                        FftFilterType::BandPass => "band pass",
                        FftFilterType::BandBlock => "band block",
                        FftFilterType::Threshold => "threshold",
                        FftFilterType::LowPassParabolic => "parabolic low pass",
                    };
                    beside.push(NewColumn {
                        after: inp.id,
                        long_name: format!("{} FFT filtered", inp.name),
                        units: inp.units.clone(),
                        comments: format!("FFT {word} filter of {}{}", inp.short, inp.range_note()),
                        data: inp.aligned(&calc::fft_filter(&y, dt, &spec.options)),
                    });
                }
            }
            Tool::IirFilter(spec) => filters::run_iir(spec, &inputs, &mut beside)?,
            Tool::FirFilter(spec) => filters::run_fir(spec, &inputs, &mut beside)?,
            Tool::Envelope(spec) => {
                for inp in &inputs {
                    // Rows without an X or a Y value drop out before the extremes are searched.
                    let (x, y): (Vec<f64>, Vec<f64>) =
                        inp.xs().iter().zip(inp.ys()).filter(|(x, y)| x.is_finite() && y.is_finite()).unzip();
                    let env = calc::envelope(&x, &y, &spec.options).map_err(|e| {
                        AnalysisError::BadOption(match e {
                            calc::EnvelopeError::BadNpts => "Points on each side cannot be negative".to_string(),
                            calc::EnvelopeError::TooFewExtremes => format!(
                                "column {} has fewer than two upper or two lower extreme points for {} points on each side",
                                inp.short, spec.options.npts
                            ),
                            calc::EnvelopeError::RepeatedX => {
                                format!("column {} has extreme points with the same X", inp.short)
                            }
                        })
                    })?;
                    let quoted = if inp.name == inp.short { inp.short.clone() } else { format!("\"{}\"", inp.name) };
                    for (k, (word, values)) in [("Upper", env.upper), ("Lower", env.lower)].into_iter().enumerate() {
                        let Some(values) = values else { continue };
                        let comments = format!("{word} Envelope of {quoted}{}", inp.range_note());
                        on_sheet.push(SheetColumn {
                            designation: Designation::X,
                            long_name: format!("Envelope X {}", k + 1),
                            units: String::new(),
                            comments: comments.clone(),
                            data: ColumnData::numeric(x.clone()),
                        });
                        on_sheet.push(SheetColumn {
                            designation: Designation::Y,
                            long_name: format!("Envelope Y {}", k + 1),
                            units: String::new(),
                            comments,
                            data: ColumnData::numeric(values),
                        });
                    }
                }
            }
            Tool::Coherence(spec) => {
                let (_, _, ocol) = self.locate(spec.other).ok_or(AnalysisError::NoSuchColumn(spec.other))?;
                let oshort = ocol.short_name.clone();
                let ovalues = ocol.numeric().ok_or_else(|| AnalysisError::NotNumeric(oshort.clone()))?;
                for inp in &inputs {
                    // Origin: the shorter signal is padded with zeros and a missing value is 0.
                    let trim = |v: &[f64]| {
                        let end = v.iter().rposition(|a| !a.is_nan()).map_or(0, |i| i + 1);
                        v[..end].to_vec()
                    };
                    let x = trim(inp.ys());
                    let y_end = (inp.last + 1).min(ovalues.len());
                    let y = trim(ovalues.get(inp.first.min(y_end)..y_end).unwrap_or(&[]));
                    if x.is_empty() {
                        return Err(AnalysisError::TooFewPoints(inp.short.clone()));
                    }
                    if y.is_empty() {
                        return Err(AnalysisError::TooFewPoints(oshort.clone()));
                    }
                    let r = calc::coherence(&x, &y, &spec.options).map_err(AnalysisError::BadOption)?;
                    on_sheet.push(SheetColumn {
                        designation: Designation::X,
                        long_name: "Frequency".into(),
                        units: String::new(),
                        comments: String::new(),
                        data: ColumnData::numeric(r.frequency),
                    });
                    on_sheet.push(SheetColumn {
                        designation: Designation::Y,
                        long_name: format!("{} coherence with {}", inp.name, ocol.display_name()),
                        units: String::new(),
                        comments: format!("Coherence of {} and {oshort}{}", inp.short, inp.range_note()),
                        data: ColumnData::numeric(r.coherence),
                    });
                }
            }
            Tool::Decimate(spec) => {
                for inp in &inputs {
                    let y = signal_values(inp.ys(), &inp.short)?;
                    let x = &inp.xs()[..y.len()];
                    if x.iter().step_by(spec.options.factor.max(1)).any(|v| !v.is_finite()) {
                        return Err(AnalysisError::BadOption(format!("column {}: X has missing values", inp.short)));
                    }
                    let (dx, dy) = calc::decimate(x, &y, &spec.options)
                        .map_err(|e| AnalysisError::BadOption(format!("column {}: {e}", inp.short)))?;
                    let o = &spec.options;
                    let how = match o.filter {
                        DecimateFilter::None => String::new(),
                        DecimateFilter::MovingAverage => ", moving average filter".to_string(),
                        DecimateFilter::Fir => format!(", FIR filter of order {}", o.order),
                    };
                    on_sheet.push(SheetColumn {
                        designation: Designation::X,
                        long_name: format!("Decimated {}", inp.x_name),
                        units: inp.x_units.clone(),
                        comments: String::new(),
                        data: ColumnData::numeric(dx),
                    });
                    on_sheet.push(SheetColumn {
                        designation: Designation::Y,
                        long_name: format!("Decimated {}", inp.name),
                        units: inp.units.clone(),
                        comments: format!("Every {} of {}{}{how}", ordinal(o.factor), inp.short, inp.range_note()),
                        data: ColumnData::numeric(dy),
                    });
                }
            }
            Tool::TraceInterp(spec) => {
                if spec.points < 2 {
                    return Err(AnalysisError::BadOption(format!(
                        "{} points are too few, a trace needs at least 2",
                        spec.points
                    )));
                }
                if spec.points > MAX_TRACE_POINTS {
                    return Err(AnalysisError::BadOption(format!(
                        "{} points are too many, the most is {MAX_TRACE_POINTS}",
                        spec.points
                    )));
                }
                let word = match spec.method {
                    TraceMethod::Linear => "linear",
                    TraceMethod::CubicSpline => "cubic spline",
                    TraceMethod::CubicBSpline => "cubic B-spline",
                };
                for inp in &inputs {
                    let (x, y) = calc::trace_interp(inp.xs(), inp.ys(), spec.method, spec.points).ok_or_else(|| {
                        let need = if spec.method == TraceMethod::CubicBSpline { 4 } else { 2 };
                        AnalysisError::BadOption(format!(
                            "{} needs at least {need} points that differ in X and in Y next to each other",
                            inp.short
                        ))
                    })?;
                    on_sheet.push(SheetColumn {
                        designation: Designation::X,
                        long_name: inp.x_name.clone(),
                        units: inp.x_units.clone(),
                        comments: String::new(),
                        data: ColumnData::numeric(x),
                    });
                    on_sheet.push(SheetColumn {
                        designation: Designation::Y,
                        long_name: format!("{} trace", inp.name),
                        units: inp.units.clone(),
                        comments: format!("{word} trace interpolation of {}{}", inp.short, inp.range_note()),
                        data: ColumnData::numeric(y),
                    });
                }
            }
            Tool::Ifft(spec) => {
                let imaginary = match spec.imaginary {
                    Some(id) => {
                        let (_, _, c) = self.locate(id).ok_or(AnalysisError::NoSuchColumn(id))?;
                        let v = c.numeric().ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
                        Some((c.short_name.clone(), v.to_vec()))
                    }
                    None => None,
                };
                for inp in &inputs {
                    let re = signal_values(inp.ys(), &inp.short)?;
                    let im: Option<Vec<f64>> = match &imaginary {
                        Some((name, v)) => {
                            let rows = v.get(inp.first..).unwrap_or(&[]);
                            let part: Vec<f64> = (0..re.len()).map(|i| rows.get(i).copied().unwrap_or(0.0)).collect();
                            if part.iter().any(|x| !x.is_finite()) {
                                return Err(AnalysisError::BadOption(format!("column {name} has missing values")));
                            }
                            Some(part)
                        }
                        None => None,
                    };
                    let mut o = spec.options;
                    if spec.auto_interval {
                        o.interval = auto_interval(inp.xs());
                    }
                    if !(o.interval.is_finite() && o.interval > 0.0) {
                        return Err(AnalysisError::BadOption(format!("{} is not a usable interval", o.interval)));
                    }
                    let r = calc::ifft_spectrum(&re, im.as_deref(), &o);
                    let note = match &imaginary {
                        Some((name, _)) => format!("IFFT of {} + i {name}{}", inp.short, inp.range_note()),
                        None => format!("IFFT of {}{}", inp.short, inp.range_note()),
                    };
                    let mut col = |designation, name: &str, units: &str, data: Vec<f64>| {
                        on_sheet.push(SheetColumn {
                            designation,
                            long_name: name.into(),
                            units: units.into(),
                            comments: note.clone(),
                            data: ColumnData::numeric(data),
                        })
                    };
                    col(Designation::X, "Time", "", r.time);
                    col(Designation::Y, "Real", "", r.real);
                    col(Designation::Y, "Imaginary", "", r.imaginary);
                    col(Designation::Y, "Amplitude", "", r.amplitude);
                    col(Designation::Y, "Phase", "Degrees", r.phase);
                }
            }
            Tool::Hilbert => {
                for inp in &inputs {
                    let y = signal_values(inp.ys(), &inp.short)?;
                    beside.push(NewColumn {
                        after: inp.id,
                        long_name: format!("Hilbert transform of {}", inp.name),
                        units: inp.units.clone(),
                        comments: format!("Hilbert transform of {}{}", inp.short, inp.range_note()),
                        data: inp.aligned(&calc::hilbert(&y)),
                    });
                }
            }
            Tool::LevelCrossing(spec) => {
                if !spec.level.is_finite() {
                    return Err(AnalysisError::BadOption(format!("{} is not a usable level", spec.level)));
                }
                // Origin's default output (measured, analysis-tools.md 14): a column CrossingX<n> of
                // the crossing X values, without units, written even when there is no crossing.
                let level = spec.level;
                let first_number = next_crossing_number(sheet);
                for (k, inp) in inputs.iter().enumerate() {
                    let n = first_number + k;
                    let found = calc::level_crossings(inp.xs(), inp.ys(), level, spec.index, spec.min_gap);
                    let column = |long_name: String, units: &str, comments: String, data: Vec<f64>| NewColumn {
                        after: inp.id,
                        long_name,
                        units: units.to_string(),
                        comments,
                        data,
                    };
                    beside.push(column(
                        format!("CrossingX{n}"),
                        "",
                        format!("X values of Y= {level} Level Crossing of {}{}", inp.short, inp.range_note()),
                        found.iter().map(|c| c.x).collect(),
                    ));
                    if spec.details {
                        let about = format!("Level Crossing of {} at Y= {level}", inp.short);
                        beside.push(column(
                            format!("Directions{n}"),
                            "",
                            format!("{about}: 1 up, -1 down, 0 on the level without passing through"),
                            found.iter().map(|c| f64::from(c.direction)).collect(),
                        ));
                        beside.push(column(
                            format!("X Indices{n}"),
                            "",
                            format!("{about}: the row of each crossing"),
                            // Rows of the sheet, not of the chosen range.
                            found.iter().map(|c| (c.row + inp.first) as f64).collect(),
                        ));
                        beside.push(column(
                            format!("Y Crossing Values{n}"),
                            &inp.units,
                            about,
                            vec![level; found.len()],
                        ));
                    }
                    notes.push(match found.len() {
                        0 => format!("{} does not reach Y = {level}", inp.short),
                        1 => format!("{} reaches Y = {level} once", inp.short),
                        c => format!("{} reaches Y = {level} {c} times", inp.short),
                    });
                }
            }
            Tool::ColumnMath(spec) => {
                let operand: Option<(String, String, Vec<f64>)> = match spec.operand {
                    CurveOperand::Constant(c) => {
                        if !c.is_finite() {
                            return Err(AnalysisError::BadOption(format!("{c} is not a usable operand")));
                        }
                        // Origin refuses to divide a column by the constant 0.
                        if spec.op == CurveOp::Divide && c == 0.0 {
                            return Err(AnalysisError::BadOption("cannot divide by 0".into()));
                        }
                        None
                    }
                    CurveOperand::Reference(id) => {
                        let (_, _, rcol) = self.locate(id).ok_or(AnalysisError::NoSuchColumn(id))?;
                        let r = rcol.numeric().ok_or_else(|| AnalysisError::NotNumeric(rcol.short_name.clone()))?;
                        Some((rcol.short_name.clone(), rcol.display_name().to_string(), r.to_vec()))
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
                    let units = match (spec.op, operand.is_some()) {
                        (CurveOp::Add | CurveOp::Subtract, _) => inp.units.clone(),
                        (CurveOp::Multiply | CurveOp::Divide, false) => inp.units.clone(),
                        _ => String::new(),
                    };
                    let (long_name, comments, values) = match (&operand, spec.operand) {
                        (Some((rshort, rname, r)), _) => {
                            // Row i of the data goes with row i of the operand column.
                            let rows = r.get(inp.first..).unwrap_or(&[]);
                            (
                                format!("{} {word} {rname}", inp.name),
                                format!("{} {word} column {rshort}{}", inp.short, inp.range_note()),
                                calc::column_math(inp.ys(), spec.op, rows),
                            )
                        }
                        (None, CurveOperand::Constant(c)) => (
                            format!("{} {word} {c}", inp.name),
                            format!("{} {word} {c}{}", inp.short, inp.range_note()),
                            calc::curve_math_const(inp.ys(), spec.op, c),
                        ),
                        (None, CurveOperand::Reference(_)) => unreachable!("the operand column was read above"),
                    };
                    beside.push(NewColumn { after: inp.id, long_name, units, comments, data: inp.aligned(&values) });
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
                    SmoothMethod::Lowess { span } => format!("Lowess, {}", local_span_text(span, options.npts)),
                    SmoothMethod::Loess { span } => format!("Loess, {}", local_span_text(span, options.npts)),
                    SmoothMethod::Binomial { order } => format!("binomial, order {order}"),
                };
                let window = if matches!(
                    options.method,
                    SmoothMethod::Lowess { .. } | SmoothMethod::Loess { .. } | SmoothMethod::Binomial { .. }
                ) {
                    String::new()
                } else {
                    format!(", {} points", options.npts)
                };
                for inp in &inputs {
                    let s = calc::smooth_xy(inp.xs(), inp.ys(), options);
                    beside.push(NewColumn {
                        after: inp.id,
                        long_name: format!("Smoothed {}", inp.name),
                        units: inp.units.clone(),
                        comments: format!("{} smoothed: {how}{window}{}", inp.short, inp.range_note()),
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
                let at_values: Option<Vec<f64>> = match spec.at {
                    Some(c) => Some(self.numeric(c).ok_or(AnalysisError::NoSuchColumn(c))?.to_vec()),
                    None => None,
                };
                if at_values.is_none() && spec.npts < 2 {
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
                    extrapolation: spec.extrapolation,
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
                    let (gx, gy) = match &at_values {
                        Some(at) => (at.clone(), calc::interp1(inp.xs(), inp.ys(), at, &options)),
                        None => calc::interp1xy(inp.xs(), inp.ys(), spec.npts, lo, hi, &options),
                    };
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
                        comments: match &at_values {
                            Some(at) => format!("{} at {} given X values{}", inp.label, at.len(), inp.range_note()),
                            None => format!("{} interpolated to {} points{}", inp.label, spec.npts, inp.range_note()),
                        },
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
                    for (name, rows) in [("Left foot", &found.left), ("Right foot", &found.right)] {
                        on_sheet.push(SheetColumn {
                            designation: Designation::Disregard,
                            long_name: format!("{name} ({})", inp.x_name),
                            units: inp.x_units.clone(),
                            comments: inp.label.clone(),
                            data: ColumnData::numeric(rows.iter().map(|&r| inp.xs()[r - 1]).collect()),
                        });
                    }
                }
                table = Some(t);
            }
            Tool::Statistics(spec) => {
                table = Some(statistics_table(spec, &inputs)?);
            }
            Tool::RowStatistics(spec) => {
                row_statistics(spec, &inputs, &mut beside, &mut notes)?;
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

        match output {
            OutputTarget::NextToInput => {}
            // The crossings are not one value per input row: on a new sheet they go as they are.
            OutputTarget::NewSheet if matches!(request.tool, Tool::LevelCrossing(_)) => {
                for nc in std::mem::take(&mut beside) {
                    on_sheet.push(SheetColumn {
                        designation: Designation::Y,
                        long_name: nc.long_name,
                        units: nc.units,
                        comments: nc.comments,
                        data: ColumnData::numeric(nc.data),
                    });
                }
            }
            OutputTarget::NewSheet => {
                for nc in std::mem::take(&mut beside) {
                    let inp = inputs.iter().find(|i| i.id == nc.after).expect("every result column has a source");
                    on_sheet.push(SheetColumn {
                        designation: Designation::X,
                        long_name: inp.x_name.clone(),
                        units: inp.x_units.clone(),
                        comments: String::new(),
                        data: ColumnData::numeric(inp.x.clone()),
                    });
                    on_sheet.push(SheetColumn {
                        designation: Designation::Y,
                        long_name: nc.long_name,
                        units: nc.units,
                        comments: nc.comments,
                        data: ColumnData::numeric(nc.data),
                    });
                }
            }
            OutputTarget::Existing(targets) => {
                if beside.is_empty() {
                    return Err(AnalysisError::BadOption(
                        "this tool writes a table of numbers, which always goes to a new sheet".into(),
                    ));
                }
                if targets.len() != beside.len() {
                    return Err(AnalysisError::BadOption(format!(
                        "choose {} existing column(s) for the result, not {}",
                        beside.len(),
                        targets.len()
                    )));
                }
                for t in targets {
                    if sheet.column(*t).is_none() {
                        return Err(AnalysisError::BadOption(
                            "the existing columns for the result must be in the sheet of the input".into(),
                        ));
                    }
                }
            }
        }
        let existing: Option<&Vec<ColumnId>> = if let OutputTarget::Existing(t) = output { Some(t) } else { None };

        // Everything was computed; now change the project.
        let count = request.columns.len();
        drop(inputs);
        let mut new_columns = Vec::with_capacity(beside.len());
        let mut added_names: Vec<String> = Vec::new();
        for (k, nc) in beside.into_iter().enumerate() {
            if let Some(targets) = existing {
                let ws = &mut self.workbook_mut(book_id).expect("workbook exists").sheets[sheet_index];
                let c = ws.column_mut(targets[k]).expect("checked above");
                c.data = ColumnData::numeric(nc.data);
                c.long_name = nc.long_name.clone();
                c.units = nc.units;
                c.comments = nc.comments;
                added_names.push(format!("{} \"{}\"", c.short_name, nc.long_name));
                new_columns.push(targets[k]);
                continue;
            }
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
        if let Some(plan) = &xy_graph {
            new_graph = Some(self.xy_graph(&sheet_column_ids, plan));
        }
        let mut more_graphs = Vec::new();
        for plan in &more_xy_graphs {
            more_graphs.push(self.xy_graph(&sheet_column_ids, plan));
        }
        for plan in &summary_graphs {
            let id = self.summary_graph(&sheet_column_ids, plan);
            if new_graph.is_none() {
                new_graph = Some(id);
            } else {
                more_graphs.push(id);
            }
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
            (None, _) if existing.is_some() => {
                format!("{}: {columns}, results in existing columns {added}", request.tool.name())
            }
            (None, _) => format!("{}: {columns}, results in new columns {added}", request.tool.name()),
        };
        let message = match new_graph.and_then(|g| self.graph(g)) {
            Some(g) => {
                let names: Vec<String> = std::iter::once(g.name.clone())
                    .chain(more_graphs.iter().filter_map(|id| self.graph(*id).map(|g| g.name.clone())))
                    .collect();
                format!("{message}; graph{} {}", if names.len() == 1 { "" } else { "s" }, names.join(", "))
            }
            None => message,
        };
        Ok(Report {
            book: book_id,
            sheet: sheet_index,
            new_columns,
            new_sheet,
            new_graph,
            more_graphs,
            table,
            message,
            notes,
        })
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
        self.sort_range(book, sheet, key, &[], descending)
    }

    /// Sorts the rows of the given columns of a sheet (all columns when `columns` is empty) by
    /// the values of `key`, as [`Project::sort_sheet`] does; columns that are not listed keep
    /// their rows (Worksheet > Sort Range on whole columns). `key` must be one of the columns
    /// when `columns` is not empty.
    pub fn sort_range(
        &mut self,
        book: WindowId,
        sheet: usize,
        key: ColumnId,
        columns: &[ColumnId],
        descending: bool,
    ) -> Result<(), AnalysisError> {
        self.sort_range_in_rows(book, sheet, key, columns, descending, 0..usize::MAX)
    }

    /// Sorts only the selected rows, keeping all other rows and unlisted columns in place.
    pub fn sort_range_in_rows(
        &mut self,
        book: WindowId,
        sheet: usize,
        key: ColumnId,
        columns: &[ColumnId],
        descending: bool,
        rows: std::ops::Range<usize>,
    ) -> Result<(), AnalysisError> {
        let ws = self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(AnalysisError::NoSuchSheet)?;
        let key_col = ws.column(key).ok_or(AnalysisError::NoSuchColumn(key))?;
        for c in columns {
            ws.column(*c).ok_or(AnalysisError::NoSuchColumn(*c))?;
        }
        let n = ws.row_count();
        let order = sort_order_in_rows(&key_col.data, n, descending, rows);
        let ws = &mut self.workbook_mut(book).expect("checked above").sheets[sheet];
        for c in &mut ws.columns {
            if columns.is_empty() || columns.contains(&c.id) {
                c.reorder_rows(&order, n);
            }
        }
        ws.notes_reorder_rows(&|c| columns.is_empty() || columns.contains(&c), &order, n);
        // The rows moved, so a filter is applied again to the new order.
        let _ = self.reapply_row_filter(book, sheet);
        Ok(())
    }

    /// Sorts each of the given columns on its own by its values (Worksheet > Sort Columns);
    /// the other columns and the rows of the other columns do not move.
    pub fn sort_columns(
        &mut self,
        book: WindowId,
        sheet: usize,
        columns: &[ColumnId],
        descending: bool,
    ) -> Result<(), AnalysisError> {
        self.sort_columns_in_rows(book, sheet, columns, descending, 0..usize::MAX)
    }

    /// Sorts each listed column independently, restricted to the selected rows.
    pub fn sort_columns_in_rows(
        &mut self,
        book: WindowId,
        sheet: usize,
        columns: &[ColumnId],
        descending: bool,
        rows: std::ops::Range<usize>,
    ) -> Result<(), AnalysisError> {
        let ws = self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(AnalysisError::NoSuchSheet)?;
        for c in columns {
            ws.column(*c).ok_or(AnalysisError::NoSuchColumn(*c))?;
        }
        let n = ws.row_count();
        let ws = &mut self.workbook_mut(book).expect("checked above").sheets[sheet];
        let mut orders = Vec::new();
        for c in &mut ws.columns {
            if columns.contains(&c.id) {
                let order = sort_order_in_rows(&c.data, n, descending, rows.clone());
                c.reorder_rows(&order, n);
                orders.push((c.id, order));
            }
        }
        for (id, order) in orders {
            ws.notes_reorder_rows(&|c| c == id, &order, n);
        }
        // The rows moved, so a filter is applied again to the new order.
        let _ = self.reapply_row_filter(book, sheet);
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
                c.reorder_rows(&keep, n);
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
                c.reorder_rows(&keep, n);
            }
            ws.notes_reorder_rows(&|_| true, &keep, n);
            Ok((kept, sheet))
        }
    }
}

/// How two rows compare by one key column: a missing value counts as smaller than every value, as
/// in Origin (measured with `wsort`), so it comes first in an ascending sort and last in a
/// descending one; text compares without regard to case and the empty text is the smallest.
pub(crate) fn row_comparator(
    key: &ColumnData,
    n: usize,
    descending: bool,
) -> Box<dyn Fn(usize, usize) -> Ordering + '_> {
    let direct = move |o: Ordering| if descending { o.reverse() } else { o };
    match key {
        ColumnData::Numeric(v) | ColumnData::Mixed { numbers: v, .. } => {
            let value = move |i: usize| v.get(i).copied().unwrap_or(f64::NAN);
            Box::new(move |a, b| {
                let (x, y) = (value(a), value(b));
                direct(match (x.is_nan(), y.is_nan()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    (false, false) => x.partial_cmp(&y).expect("neither is NaN"),
                })
            })
        }
        ColumnData::Text(v) => {
            let keys: Vec<String> = (0..n).map(|i| v.get(i).map(|s| s.to_lowercase()).unwrap_or_default()).collect();
            Box::new(move |a, b| direct(keys[a].cmp(&keys[b])))
        }
    }
}

/// Equal keys keep their order; entries outside the selected interval keep their positions.
fn sort_order_in_rows(key: &ColumnData, n: usize, descending: bool, rows: std::ops::Range<usize>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    let cmp = row_comparator(key, n, descending);
    let end = rows.end.min(n);
    let start = rows.start.min(end);
    order[start..end].sort_by(|a, b| cmp(*a, *b));
    order
}

/// The rows `order` of a column, in that order. A column shorter than the
/// sheet (`rows` long) counts as padded with missing values; missing values
/// at the end of the result are dropped again so that short columns stay short.
pub(crate) fn take_rows(data: &ColumnData, order: &[usize], rows: usize) -> ColumnData {
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
        ColumnData::Mixed { numbers, texts } => {
            let mut out: Vec<f64> = order.iter().map(|i| numbers.get(*i).copied().unwrap_or(f64::NAN)).collect();
            let moved: std::collections::BTreeMap<usize, String> =
                order.iter().enumerate().filter_map(|(k, i)| texts.get(i).map(|t| (k, t.clone()))).collect();
            if numbers.len() < rows {
                while out.last().is_some_and(|x| x.is_nan()) && !moved.contains_key(&(out.len() - 1)) {
                    out.pop();
                }
            }
            ColumnData::mixed(out, moved)
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
        let s = p.analyze(&request(&[flat, ramp], Tool::Statistics(StatisticsSpec::default()))).unwrap();
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
    fn interpolation_onto_the_values_of_a_column() {
        let (mut p, book, _, ramp) = sample();
        let at = p.add_column(book, 0, Designation::Disregard, ColumnData::numeric(vec![0.5, 2.25, 12.0])).unwrap();
        let spec = InterpSpec { at: Some(at), ..Default::default() };
        let r = p.analyze(&request(&[ramp], Tool::Interpolate(spec))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.columns[0].numeric().unwrap(), [0.5, 2.25, 12.0], "X is the given column");
        assert_eq!(ws.columns[1].numeric().unwrap(), [0.5, 2.25, 12.0], "y = x, extrapolated past 10");
        assert!(ws.columns[1].comments.contains("3 given X values"), "{}", ws.columns[1].comments);
        let missing = InterpSpec { at: Some(ColumnId(9999)), ..Default::default() };
        assert!(p.analyze(&request(&[ramp], Tool::Interpolate(missing))).is_err());
        // Outside the data (X 0..10): missing, or the end value repeated.
        for (extrapolation, want) in [(Extrapolation::Missing, f64::NAN), (Extrapolation::Repeat, 10.0)] {
            let spec = InterpSpec { at: Some(at), extrapolation, ..Default::default() };
            let r = p.analyze(&request(&[ramp], Tool::Interpolate(spec))).unwrap();
            let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
            let y = ws.columns[1].numeric().unwrap();
            assert_eq!(&y[..2], [0.5, 2.25], "{extrapolation:?}: inside the data as before");
            assert!(y[2] == want || (want.is_nan() && y[2].is_nan()), "{extrapolation:?}: {}", y[2]);
        }
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

        let r = p.analyze(&request(&[flat, ramp], Tool::Statistics(StatisticsSpec::default()))).unwrap();
        let t = r.table.unwrap();
        assert_eq!(t.headers[2], "Mean");
        assert_eq!((t.rows[0].1[0], t.rows[0].1[2], t.rows[1].1[2], t.rows[1].1[10]), (11.0, 2.0, 5.0, 10.0));
        assert_eq!(p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()].name, "Statistics1");
    }

    #[test]
    fn statistics_on_columns_can_add_the_other_quantities_and_confidence_limits() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let data = vec![2.0, 3.5, 1.0, 6.0, 4.5, 3.0, f64::NAN, 8.0];
        let col = p.add_column(book, 0, Designation::Y, ColumnData::numeric(data.clone())).unwrap();
        // Default: the original eleven columns.
        let d = p.analyze(&request(&[col], Tool::Statistics(StatisticsSpec::default()))).unwrap();
        assert_eq!(d.table.unwrap().headers.len(), 11);
        let r = p.analyze(&request(&[col], Tool::Statistics(StatisticsSpec::all()))).unwrap();
        let t = r.table.unwrap();
        assert_eq!(t.headers.len(), 11 + 5 + 2);
        let at = |name: &str| t.headers.iter().position(|h| h == name).unwrap_or_else(|| panic!("{name}"));
        let row = &t.rows[0].1;
        let s = calc::stats(&data);
        assert_eq!(row[at("Variance")], s.variance);
        assert_eq!(row[at("IQR")], s.q3 - s.q1);
        assert_eq!((row[at("Skewness")], row[at("Kurtosis")], row[at("CV")]), (s.skewness, s.kurtosis, s.cv));
        // The limits are the interval of the one-sample t-test.
        let tt = calc::ttest_one(&data, 0.0, calc::Tail::Two, 0.05).unwrap();
        assert!((row[at("Lower 95% CL of Mean")] - tt.lcl).abs() < 1e-12);
        assert!((row[at("Upper 95% CL of Mean")] - tt.ucl).abs() < 1e-12);
        let spec = StatisticsSpec { confidence_limits: true, level: 0.9, ..StatisticsSpec::default() };
        let r = p.analyze(&request(&[col], Tool::Statistics(spec))).unwrap();
        assert!(r.table.unwrap().headers.iter().any(|h| h == "Lower 90% CL of Mean"));
        let bad = StatisticsSpec { confidence_limits: true, level: 1.5, ..StatisticsSpec::default() };
        assert!(p.analyze(&request(&[col], Tool::Statistics(bad))).is_err());
    }

    #[test]
    fn masked_cells_are_left_out_of_the_statistics() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let col = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 100.0])).unwrap();
        let mean = |p: &mut Project| {
            let r = p.analyze(&request(&[col], Tool::Statistics(StatisticsSpec::default()))).unwrap();
            let t = r.table.unwrap();
            let at = t.headers.iter().position(|h| h == "Mean").unwrap();
            (t.rows[0].1[at], t.headers.iter().position(|h| h == "N").map(|i| t.rows[0].1[i]))
        };
        assert_eq!(mean(&mut p).0, 26.5);
        p.column_mut(col).unwrap().mask.insert(3);
        let (m, n) = mean(&mut p);
        assert_eq!(m, 2.0);
        assert!(n.is_none_or(|n| n == 3.0));
    }

    #[test]
    fn statistics_on_rows_add_columns_behind_the_last_chosen_column() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, f64::NAN, f64::NAN])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![3.0, 6.0, 5.0, f64::NAN])).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![9.0; 4])).unwrap();
        let spec = RowStatsSpec { sem: true, n: true, sum: true, min: true, max: true, ..Default::default() };
        let r = p.analyze(&request(&[a, b], Tool::RowStatistics(spec))).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[0];
        let names: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
        // The new columns sit between the chosen columns and the untouched one.
        assert_eq!(names[..3], ["", "", "Mean"], "{names:?}");
        assert_eq!(names[ws.columns.len() - 1], "", "{names:?}");
        assert_eq!(ws.columns.len(), 3 + 7);
        assert_eq!(r.new_columns.len(), 7);
        let col = |name: &str| ws.columns.iter().find(|c| c.long_name == name).unwrap().numeric().unwrap().to_vec();
        assert_eq!(col("Mean")[..3], [2.0, 4.0, 5.0]);
        assert!(col("Mean")[3].is_nan() && col("Number of Points")[3].is_nan());
        assert!((col("Standard Deviation")[0] - 2.0_f64.sqrt()).abs() < 1e-12);
        assert!(col("Standard Deviation")[2].is_nan(), "one number has no spread");
        assert_eq!(col("Sum")[..3], [4.0, 8.0, 5.0]);
        assert_eq!(col("Number of Points")[..3], [2.0, 2.0, 1.0]);
        assert_eq!((col("Minimum")[1], col("Maximum")[1]), (2.0, 6.0));
        assert_eq!(ws.columns.last().unwrap().id, c);
        // Nothing chosen is an error.
        let none = RowStatsSpec { mean: false, sd: false, ..Default::default() };
        assert!(p.analyze(&request(&[a, b], Tool::RowStatistics(none))).is_err());
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
    fn sort_keeps_rows_together_and_treats_a_missing_key_as_the_smallest_value() {
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
        // Origin (wsort): a missing key is the smallest value, so it comes first when ascending.
        assert_eq!(p.column(tag).unwrap().text().unwrap(), ["n", "a1", "a2", "b", "c"]);
        let k = p.numeric(key).unwrap();
        assert!(k[0].is_nan());
        assert_eq!(k[1..], [1.0, 1.0, 2.0, 3.0]);
        // The short column was padded with missing values for the sort; row "c" keeps its 30.
        let s = p.numeric(short).unwrap();
        assert_eq!((s[0], s[1], s[4]), (0.0, 10.0, 30.0));
        assert!(s[2].is_nan() && s[3].is_nan());
        p.sort_sheet(book, 0, key, true).unwrap();
        assert_eq!(
            p.column(tag).unwrap().text().unwrap(),
            ["c", "b", "a1", "a2", "n"],
            "missing is last when descending"
        );
        // Sorting by a text column ignores case; an empty cell is the smallest.
        p.sort_sheet(book, 0, tag, false).unwrap();
        assert_eq!(p.column(tag).unwrap().text().unwrap(), ["a1", "a2", "b", "c", "n"]);
        assert_eq!(p.sort_sheet(book, 3, key, false), Err(AnalysisError::NoSuchSheet));
    }

    #[test]
    fn sort_range_moves_only_the_listed_columns_and_sort_columns_sorts_each_alone() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![3.0, 1.0, 2.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![30.0, 10.0, 20.0])).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![7.0, 8.0, 9.0])).unwrap();
        p.sort_range(book, 0, a, &[a, b], false).unwrap();
        assert_eq!(p.numeric(a).unwrap(), [1.0, 2.0, 3.0]);
        assert_eq!(p.numeric(b).unwrap(), [10.0, 20.0, 30.0]);
        assert_eq!(p.numeric(c).unwrap(), [7.0, 8.0, 9.0], "column C is not in the range");
        let d = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0, 9.0, 1.0])).unwrap();
        p.sort_columns(book, 0, &[c, d], true).unwrap();
        assert_eq!(p.numeric(c).unwrap(), [9.0, 8.0, 7.0]);
        assert_eq!(p.numeric(d).unwrap(), [9.0, 5.0, 1.0]);
        assert_eq!(p.numeric(a).unwrap(), [1.0, 2.0, 3.0], "column A keeps its order");
        assert_eq!(p.sort_columns(book, 0, &[ColumnId(9999)], false), Err(AnalysisError::NoSuchColumn(ColumnId(9999))));
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

    #[test]
    fn column_math_goes_row_by_row_and_can_replace_its_input_as_origin() {
        let (mut p, book, _, ramp) = sample();
        let reference = add_reference(&mut p);
        let tool =
            Tool::ColumnMath(ColumnMathSpec { op: CurveOp::Subtract, operand: CurveOperand::Reference(reference) });
        let r = p.analyze(&request(&[ramp], tool)).unwrap();
        // Rows 1-3 minus 1, 2, 3 (no X, no interpolation); the rows past the operand's end minus 0.
        let mut want: Vec<f64> = (0..11).map(f64::from).collect();
        for (w, d) in want.iter_mut().zip([1.0, 2.0, 3.0]) {
            *w -= d;
        }
        assert_eq!(p.numeric(r.new_columns[0]).unwrap(), want);
        assert_eq!(p.column(r.new_columns[0]).unwrap().comments, "C minus column B");
        assert!(r.message.starts_with("Column math"), "{}", r.message);
        // Origin's default output is the input column itself.
        let tool = Tool::ColumnMath(ColumnMathSpec { op: CurveOp::Add, operand: CurveOperand::Constant(10.0) });
        let before = p.workbook(book).unwrap().sheets[0].columns.len();
        p.analyze_to(&request(&[ramp], tool), &OutputTarget::Existing(vec![ramp])).unwrap();
        assert_eq!(p.numeric(ramp).unwrap()[..3], [10.0, 11.0, 12.0]);
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns.len(), before);
        let zero = Tool::ColumnMath(ColumnMathSpec { op: CurveOp::Divide, operand: CurveOperand::Constant(0.0) });
        assert!(matches!(p.analyze(&request(&[ramp], zero)), Err(AnalysisError::BadOption(_))));
    }

    #[test]
    fn convolution_and_correlation_write_x_and_y_to_a_new_sheet() {
        let (mut p, book, flat, ramp) = sample();
        let response = add_reference(&mut p);
        let spec = ConvolutionSpec { response, circular: false, normalize: false, interval: None };
        let r = p.analyze(&request(&[ramp], Tool::Convolution(spec))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(rs.name.starts_with("Convolution"), "{}", rs.name);
        // 11 + 3 - 1 values at the X step of the input (1 s).
        let x = rs.columns[0].numeric().unwrap();
        assert_eq!(x.len(), 13);
        assert_eq!((x[12], rs.columns[0].units.as_str()), (12.0, "s"));
        assert_eq!(rs.columns[1].numeric().unwrap()[..3], [0.0, 1.0, 4.0]);
        assert_eq!(rs.columns[1].comments, "C convolved with B");
        let spec = SignalCorrelationSpec { other: flat, circular: false, normalize: false, interval: 0.5 };
        let r = p.analyze(&request(&[ramp], Tool::SignalCorrelation(spec))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let lags = rs.columns[0].numeric().unwrap();
        assert_eq!((lags.len(), lags[0], lags[20]), (21, -5.0, 5.0));
        // Lag 0: the dot product of 0..10 with 2.
        assert_eq!(rs.columns[1].numeric().unwrap()[10], 110.0);
        assert_eq!(r.message.split(':').next(), Some("Signal correlation"));
        // A missing value inside a signal is refused.
        let mut holes = vec![1.0; 5];
        holes[2] = f64::NAN;
        let gappy = p.add_column(book, 0, Designation::Y, ColumnData::numeric(holes)).unwrap();
        let spec = SignalCorrelationSpec { other: gappy, circular: true, normalize: true, interval: 1.0 };
        assert!(matches!(
            p.analyze(&request(&[ramp], Tool::SignalCorrelation(spec))),
            Err(AnalysisError::BadOption(_))
        ));
    }

    #[test]
    fn coherence_writes_frequency_and_coherence_to_a_new_sheet() {
        let (mut p, book, flat, ramp) = sample();
        let options = CoherenceOptions { fft_len: 8, win_len: 8, overlap: 4, interval: 0.5, ..Default::default() };
        let r = p.analyze(&request(&[ramp], Tool::Coherence(CoherenceSpec { other: flat, options }))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(rs.name.starts_with("Coherence"), "{}", rs.name);
        assert_eq!(rs.columns.len(), 2);
        let f = rs.columns[0].numeric().unwrap();
        assert_eq!((f.len(), f[1], f[4]), (5, 0.25, 1.0));
        assert_eq!(rs.columns[1].comments, "Coherence of C and B");
        assert_eq!(r.message.split(':').next(), Some("Coherence"));
        // A window longer than the FFT is refused.
        let options = CoherenceOptions { win_len: 16, ..options };
        assert!(matches!(
            p.analyze(&request(&[ramp], Tool::Coherence(CoherenceSpec { other: flat, options }))),
            Err(AnalysisError::BadOption(_))
        ));
    }

    #[test]
    fn coherence_cuts_both_signals_to_the_row_range() {
        let (mut p, book, _, ramp) = sample();
        let sq: Vec<f64> = (0..11).map(|i| f64::from(i * i % 7)).collect();
        let flat = p.add_column(book, 0, Designation::Y, ColumnData::numeric(sq)).unwrap();
        let options = CoherenceOptions { fft_len: 8, win_len: 4, overlap: 0, ..Default::default() };
        let mut req = request(&[ramp], Tool::Coherence(CoherenceSpec { other: flat, options }));
        req.rows = RowRange::Rows(3, 6);
        let r = p.analyze(&req).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let got = rs.columns[1].numeric().unwrap().to_vec();
        let x = p.locate(ramp).unwrap().2.numeric().unwrap()[2..=5].to_vec();
        let y = p.locate(flat).unwrap().2.numeric().unwrap()[2..=5].to_vec();
        let want = calc::coherence(&x, &y, &options).unwrap().coherence;
        assert_eq!(got.len(), want.len());
        for (a, b) in got.iter().zip(&want) {
            assert!((a.is_nan() && b.is_nan()) || (a - b).abs() < 1e-12);
        }
    }

    #[test]
    fn the_fft_writes_origins_eight_columns_with_frequencies_from_the_x_step() {
        let (mut p, book, _, ramp) = sample();
        let r = p.analyze(&request(&[ramp], Tool::Fft(FftSpec { auto_interval: true, ..Default::default() }))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(rs.name.starts_with("FFT"), "{}", rs.name);
        let names: Vec<&str> = rs.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(names, ["Frequency", "Real", "Imaginary", "Magnitude", "Amplitude", "Phase", "Power", "dB"]);
        // 11 points one second apart: frequencies k / 11 up to 5 / 11.
        let f = rs.columns[0].numeric().unwrap();
        assert_eq!(f.len(), 6);
        assert!((f[5] - 5.0 / 11.0).abs() < 1e-12);
        // The mean of 0..10 is 5.
        assert!((rs.columns[4].numeric().unwrap()[0] - 5.0).abs() < 1e-12);
        let bad = FftSpec { options: FftOptions { interval: 0.0, ..Default::default() }, auto_interval: false };
        assert!(matches!(p.analyze(&request(&[ramp], Tool::Fft(bad))), Err(AnalysisError::BadOption(_))));
    }

    #[test]
    fn the_inverse_fft_of_an_fft_gives_the_signal_back() {
        let (mut p, book, _, ramp) = sample();
        let two_sided = FftOptions { two_sided: true, ..Default::default() };
        let r = p.analyze(&request(&[ramp], Tool::Fft(FftSpec { options: two_sided, auto_interval: true }))).unwrap();
        let sheet = r.new_sheet.unwrap();
        let (real, imag) = {
            let rs = &p.workbook(book).unwrap().sheets[sheet];
            (rs.columns[1].id, rs.columns[2].id)
        };
        let spec = IfftSpec { imaginary: Some(imag), options: IfftOptions::default(), auto_interval: true };
        let r = p.analyze(&request(&[real], Tool::Ifft(spec))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let names: Vec<&str> = rs.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(names, ["Time", "Real", "Imaginary", "Amplitude", "Phase"]);
        // The frequency step 1/11 Hz gives back the time step of 1 s and the ramp.
        for (i, (t, v)) in rs.columns[0].numeric().unwrap().iter().zip(rs.columns[1].numeric().unwrap()).enumerate() {
            assert!((t - i as f64).abs() < 1e-9 && (v - i as f64).abs() < 1e-9, "row {i}: {t} {v}");
        }
        assert!(r.message.starts_with("IFFT"), "{}", r.message);
    }

    #[test]
    fn decimation_writes_every_dth_point_with_its_x_to_a_new_sheet() {
        let (mut p, book, _, ramp) = sample();
        let options = DecimateOptions { factor: 2, ..Default::default() };
        let r = p.analyze(&request(&[ramp], Tool::Decimate(DecimateSpec { options }))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(rs.name.starts_with("Decimate"), "{}", rs.name);
        assert_eq!(rs.columns[0].numeric().unwrap(), [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!((rs.columns[0].units.as_str(), rs.columns[0].long_name.as_str()), ("s", "Decimated Time"));
        assert_eq!(rs.columns[1].numeric().unwrap(), [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(rs.columns[1].comments, "Every 2nd point of C");
        assert!(r.message.starts_with("Decimation"), "{}", r.message);
        // Origin refuses a factor of 3 on 11 points.
        let big = DecimateOptions { factor: 3, ..Default::default() };
        assert!(matches!(
            p.analyze(&request(&[ramp], Tool::Decimate(DecimateSpec { options: big }))),
            Err(AnalysisError::BadOption(_))
        ));
    }

    #[test]
    fn decimation_refuses_a_missing_x_in_a_kept_row_only() {
        let mut p = Project::new();
        let book = p.add_workbook("gaps");
        let mut x: Vec<f64> = (0..40).map(f64::from).collect();
        x[4] = f64::NAN;
        p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 40])).unwrap();
        let run = |p: &mut Project, factor| {
            let options = DecimateOptions { factor, ..Default::default() };
            p.analyze(&request(&[y], Tool::Decimate(DecimateSpec { options })))
        };
        // Row 4 is kept by a factor of 2 but skipped by a factor of 3.
        assert!(matches!(run(&mut p, 2), Err(AnalysisError::BadOption(_))));
        assert!(run(&mut p, 3).is_ok());
    }

    #[test]
    fn an_fft_filter_goes_beside_its_column() {
        let (mut p, book, flat, ramp) = sample();
        // The automatic low pass keeps a constant as it is.
        let r = p.analyze(&request(&[flat, ramp], Tool::FftFilter(FftFilterSpec::default()))).unwrap();
        let out = p.numeric(r.new_columns[0]).unwrap();
        assert!(out.iter().all(|v| (v - 2.0).abs() < 1e-12), "{out:?}");
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!(ws.columns[2].comments, "FFT low pass filter of B");
        assert_eq!(ws.columns[2].units, "A");
        // A high pass without the offset removes the mean of the constant entirely.
        let options = FftFilterOptions { kind: FftFilterType::HighPass, ..Default::default() };
        let r = p.analyze(&request(&[flat], Tool::FftFilter(FftFilterSpec { options, interval: None }))).unwrap();
        assert!(p.numeric(r.new_columns[0]).unwrap().iter().all(|v| v.abs() < 1e-12));
        assert!(r.message.starts_with("FFT filter"), "{}", r.message);
    }

    #[test]
    fn an_iir_filter_goes_beside_its_column() {
        let (mut p, book, flat, ramp) = sample();
        // X runs 0 to 10 in steps of 1: Fs = 1 Hz, Nyquist 0.5 Hz. A forward-backward low pass keeps a constant.
        let spec = IirSpec { order: Some(2), cutoff: Some(0.2), forward_backward: true, ..IirSpec::default() };
        let r = p.analyze(&request(&[flat, ramp], Tool::IirFilter(spec))).unwrap();
        assert_eq!(r.new_columns.len(), 2);
        let out = p.numeric(r.new_columns[0]).unwrap();
        assert!(out.iter().all(|v| (v - 2.0).abs() < 1e-9), "{out:?}");
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!(
            ws.columns[2].comments,
            "Butterworth low pass IIR filter of B (order 2, Fc 0.2 Hz, forward-backward)"
        );
        assert_eq!(ws.columns[2].units, "A");
        assert!(ws.columns[2].long_name.ends_with("IIR filtered"), "{}", ws.columns[2].long_name);
        assert!(r.message.starts_with("IIR filter"), "{}", r.message);
        // a ramp is a straight line for a zero-phase low pass with odd extension
        let ramp_out = p.numeric(r.new_columns[1]).unwrap();
        let ramp_in = p.numeric(ramp).unwrap();
        for (a, b) in ramp_out.iter().zip(ramp_in.iter()) {
            assert!((a - b).abs() < 1e-3, "{a} vs {b}");
        }
    }

    #[test]
    fn iir_and_fir_filters_refuse_what_cannot_be_designed_or_applied() {
        let (mut p, _, flat, _) = sample();
        // 0.7 Hz is above the Nyquist frequency of 0.5 Hz
        let bad = IirSpec { cutoff: Some(0.7), ..IirSpec::default() };
        let e = p.analyze(&request(&[flat], Tool::IirFilter(bad))).unwrap_err();
        assert!(e.to_string().contains("Nyquist"), "{e}");
        // 11 points are too few for forward-backward filtering at order 10
        let short = IirSpec { forward_backward: true, cutoff: Some(0.1), ..IirSpec::default() };
        let e = p.analyze(&request(&[flat], Tool::IirFilter(short))).unwrap_err();
        assert!(e.to_string().contains("more than"), "{e}");
        let odd =
            FirSpec { response: FilterResponse::HighPass, order: Some(5), cutoff: Some(0.2), ..FirSpec::default() };
        let e = p.analyze(&request(&[flat], Tool::FirFilter(odd))).unwrap_err();
        assert!(e.to_string().contains("even order"), "{e}");
    }

    #[test]
    fn a_fir_filter_goes_beside_its_column_and_keeps_a_constant() {
        let (mut p, book, flat, _) = sample();
        let spec = FirSpec { order: Some(4), cutoff: Some(0.2), ..FirSpec::default() };
        let r = p.analyze(&request(&[flat], Tool::FirFilter(spec))).unwrap();
        let out = p.numeric(r.new_columns[0]).unwrap();
        assert!(out.iter().all(|v| (v - 2.0).abs() < 1e-9), "{out:?}");
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!(ws.columns[2].comments, "Hamming window low pass FIR filter of B (order 4, Fc 0.2 Hz)");
        let ls = FirSpec {
            method: FirMethod::LeastSquares,
            order: Some(4),
            pass: Some(0.1),
            stop: Some(0.3),
            ..FirSpec::default()
        };
        let r = p.analyze(&request(&[flat], Tool::FirFilter(ls))).unwrap();
        let out = p.numeric(r.new_columns[0]).unwrap();
        assert!(out.iter().all(|v| (v - 2.0).abs() < 1e-9), "{out:?}");
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert!(
            ws.columns[2]
                .comments
                .starts_with("least squares low pass FIR filter of B (order 4, pass 0.1 Hz, stop 0.3 Hz"),
            "{}",
            ws.columns[2].comments
        );
    }

    #[test]
    fn an_envelope_goes_to_a_new_sheet_with_its_own_x() {
        let (mut p, book, _, _) = sample();
        let y = [0.0, 2.0, 1.0, 3.0, 0.5, 4.0, 0.2, 2.0, 1.0, 5.0, 4.0, 3.0];
        let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
        let xc = p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
        let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y.to_vec())).unwrap();
        let options = EnvelopeOptions { kind: EnvelopeType::Both, npts: 1.0 };
        let r = p.analyze(&request(&[yc], Tool::Envelope(EnvelopeSpec { options }))).unwrap();
        assert!(r.message.starts_with("Envelope"), "{}", r.message);
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let names: Vec<_> = rs.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(names, ["Envelope X 1", "Envelope Y 1", "Envelope X 2", "Envelope Y 2"]);
        assert_eq!(rs.columns[1].comments, "Upper Envelope of E");
        let upper = rs.columns[1].numeric().unwrap();
        // Origin printed 1.6138392857142858 at the first row and passes through the extremes.
        assert!((upper[0] - 1.6138392857142858).abs() < 1e-12 && (upper[9] - 5.0).abs() < 1e-12, "{upper:?}");
        let _ = xc;
        // A missing value drops its row (Origin: 11 rows, extremes searched without it).
        let mut with_gap = y.to_vec();
        with_gap[5] = f64::NAN;
        let gc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(with_gap)).unwrap();
        let r = p.analyze(&request(&[gc], Tool::Envelope(EnvelopeSpec { options }))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(rs.columns[1].numeric().unwrap().len(), 11);
        // Too few extremes are refused with a message.
        let tiny = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        assert!(p.analyze(&request(&[tiny], Tool::Envelope(EnvelopeSpec { options }))).is_err());
    }

    #[test]
    fn a_trace_interpolation_goes_to_a_new_sheet_with_its_own_x() {
        let mut p = Project::new();
        let book = p.add_workbook("loop");
        let x = [0.0, 2.0, 3.0, 2.0, 0.0, -1.0];
        let y = [0.0, 0.0, 1.0, 2.0, 2.0, 1.0];
        let xc = p.add_column(book, 0, Designation::X, ColumnData::numeric(x.to_vec())).unwrap();
        let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y.to_vec())).unwrap();
        p.column_mut(xc).unwrap().units = "V".into();
        p.column_mut(yc).unwrap().units = "A".into();
        let spec = TraceInterpSpec { method: TraceMethod::Linear, points: 10 };
        let r = p.analyze(&request(&[yc], Tool::TraceInterp(spec))).unwrap();
        let rs = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(rs.name.starts_with("TraceInterp"), "{}", rs.name);
        // Origin's numbers for this loop (analysis-tools.md 16): the diamond (1,0) (3,1) (1,2) (-1,1)
        let (ox, oy) = (rs.columns[0].numeric().unwrap(), rs.columns[1].numeric().unwrap());
        assert_eq!((ox.len(), oy.len()), (10, 10));
        assert_eq!((ox[0], oy[0], ox[3], oy[3], ox[9], oy[9]), (1.0, 0.0, 3.0, 1.0, -1.0, 1.0));
        assert_eq!((rs.columns[0].units.as_str(), rs.columns[1].units.as_str()), ("V", "A"));
        assert_eq!(rs.columns[1].comments, "linear trace interpolation of B");
        assert!(r.message.starts_with("Trace interpolation"), "{}", r.message);
        // The B-spline needs four points after merging; the diamond has four.
        let spline = TraceInterpSpec { method: TraceMethod::CubicBSpline, points: 10 };
        assert!(p.analyze(&request(&[yc], Tool::TraceInterp(spline))).is_ok());
        // A trace of fewer than two points, or with too few distinct points, is refused.
        let one = TraceInterpSpec { points: 1, ..spec };
        assert!(matches!(p.analyze(&request(&[yc], Tool::TraceInterp(one))), Err(AnalysisError::BadOption(_))));
        let huge = TraceInterpSpec { points: MAX_TRACE_POINTS + 1, ..spec };
        assert!(matches!(p.analyze(&request(&[yc], Tool::TraceInterp(huge))), Err(AnalysisError::BadOption(_))));
        let flat = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 6])).unwrap();
        assert!(matches!(p.analyze(&request(&[flat], Tool::TraceInterp(spec))), Err(AnalysisError::BadOption(_))));
    }

    #[test]
    fn the_hilbert_transform_goes_beside_its_column() {
        let (mut p, book, _, ramp) = sample();
        let r = p.analyze(&request(&[ramp], Tool::Hilbert)).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!(ws.columns[3].id, r.new_columns[0], "right behind its source column");
        assert_eq!(ws.columns[3].comments, "Hilbert transform of C");
        let h = p.numeric(r.new_columns[0]).unwrap();
        let want = sheaf_calc::hilbert(&(0..=10).map(f64::from).collect::<Vec<_>>());
        assert_eq!(h, want);
        assert!(r.message.starts_with("Hilbert transform"), "{}", r.message);
    }

    #[test]
    fn level_crossing_writes_origins_crossing_column_beside_the_input() {
        let (mut p, book, _, ramp) = sample();
        let spec = LevelCrossingSpec { level: 4.5, index: CrossingIndex::Nearest, min_gap: 1, details: false };
        let r = p.analyze(&request(&[ramp], Tool::LevelCrossing(spec))).unwrap();
        assert!(r.new_sheet.is_none());
        let col = p.column(r.new_columns[0]).unwrap();
        assert_eq!(col.long_name, "CrossingX1");
        assert_eq!(col.comments, "X values of Y= 4.5 Level Crossing of C");
        assert_eq!(col.units, "", "Origin gives the crossings no units");
        assert_eq!(p.numeric(r.new_columns[0]).unwrap(), [4.5]);
        assert_eq!(r.notes, ["C reaches Y = 4.5 once"]);
        // The optional outputs, numbered on: direction, row (of the sheet within rows 7 to 11) and Y.
        let mut req = request(&[ramp], Tool::LevelCrossing(LevelCrossingSpec { level: 8.0, details: true, ..spec }));
        req.rows = RowRange::Rows(7, 11);
        let r = p.analyze(&req).unwrap();
        let names: Vec<&str> = r.new_columns.iter().map(|&id| p.column(id).unwrap().long_name.as_str()).collect();
        assert_eq!(names, ["CrossingX2", "Directions2", "X Indices2", "Y Crossing Values2"]);
        let values: Vec<&[f64]> = r.new_columns.iter().map(|&id| p.numeric(id).unwrap()).collect();
        // 8 is the X of row 9, a point on the level: its own row, direction up.
        assert_eq!(values, [&[8.0][..], &[1.0], &[9.0], &[8.0]]);
        // No crossing: Origin still adds the empty column.
        let r = p.analyze(&request(&[ramp], Tool::LevelCrossing(LevelCrossingSpec { level: -3.0, ..spec }))).unwrap();
        assert_eq!(p.column(r.new_columns[0]).unwrap().long_name, "CrossingX3");
        assert!(p.numeric(r.new_columns[0]).unwrap().is_empty());
        // On a new sheet the crossings go as they are, without the X of the input.
        let r = p.analyze_to(&request(&[ramp], Tool::LevelCrossing(spec)), &OutputTarget::NewSheet).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.columns.len(), 1);
        assert_eq!(ws.columns[0].numeric().unwrap(), [4.5]);
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
        // Area % is of the whole corrected curve (Origin's AreaIntgP): the two peaks hold nearly all of it.
        assert!((t.rows[0].1[4] + t.rows[1].1[4] - 100.0).abs() < 1.0);
        let ws = &p.workbook(book).unwrap().sheets;
        assert_eq!(ws.len(), 2);
        assert!(ws[1].name.starts_with("PeakAnalysis"));
        assert_eq!(r.new_columns.len(), 1 + t.headers.len() + 1, "baseline column, label, and numbers");
        assert_eq!(ws[0].columns.len(), 3, "baseline column beside the source");
    }

    #[test]
    fn peak_chain_fit_reports_each_peak() {
        let (mut p, _, y) = two_peaks();
        let fit = PeakFitSpec::of(PeakShape::Gauss, FitBaseline::Linear);
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
    fn peak_chain_voigt_fit_of_gauss_peaks_gives_their_center_width_and_area() {
        // Gaussian peaks: the Voigt's Lorentzian width goes to its bound 0 and the Voigt FWHM
        // (0.5346 wL + sqrt(0.2166 wL^2 + wG^2)) is the Gaussian one.
        let (mut p, _, y) = two_peaks();
        let fit = PeakFitSpec::of(PeakShape::Voigt, FitBaseline::Linear);
        let r = p.analyze(&request(&[y], chain(true, Some(fit)))).unwrap();
        let t = r.table.unwrap();
        let h = |n: &str| t.headers.iter().position(|h| h == n).unwrap();
        assert!((t.rows[0].1[h("Fit center")] - 6.0).abs() < 1e-4);
        assert!((t.rows[1].1[h("Fit center")] - 13.0).abs() < 1e-4);
        assert!((t.rows[0].1[h("Fit FWHM")] - 0.4 * 2.354_820).abs() < 1e-3, "{}", t.rows[0].1[h("Fit FWHM")]);
        let exact = 0.4 * (2.0 * std::f64::consts::PI).sqrt();
        assert!((t.rows[0].1[h("Fit area")] - exact).abs() < 1e-3);
        assert!((t.rows[0].1[h("Fit height")] - 1.0).abs() < 1e-3, "{}", t.rows[0].1[h("Fit height")]);
        assert!(r.notes[0].contains("Voigt fit of 2 peaks"), "{}", r.notes[0]);
    }

    #[test]
    fn peak_chain_fit_takes_a_function_per_peak() {
        // Peak 1 Gauss, peak 2 (beyond the list) the spec's Voigt: the note and the parameters
        // follow each peak's function, and both Gaussian peaks keep their centre and area.
        let (mut p, _, y) = two_peaks();
        let fit =
            PeakFitSpec { shape: PeakShape::Voigt, baseline: FitBaseline::Linear, shapes: vec![PeakShape::Gauss] };
        assert_eq!(fit.shapes_for(3), [PeakShape::Gauss, PeakShape::Voigt, PeakShape::Voigt]);
        let r = p.analyze(&request(&[y], chain(true, Some(fit)))).unwrap();
        let t = r.table.unwrap();
        let h = |n: &str| t.headers.iter().position(|h| h == n).unwrap();
        assert!(r.notes[0].contains("Gauss+Voigt fit of 2 peaks"), "{}", r.notes[0]);
        assert!((t.rows[0].1[h("Fit center")] - 6.0).abs() < 1e-4);
        assert!((t.rows[1].1[h("Fit center")] - 13.0).abs() < 1e-4);
        let exact = 0.6 * 0.5 * (2.0 * std::f64::consts::PI).sqrt();
        assert!((t.rows[1].1[h("Fit area")] - exact).abs() < 1e-3, "{}", t.rows[1].1[h("Fit area")]);
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
            fit: Some(PeakFitSpec::of(PeakShape::Gauss, FitBaseline::Linear)),
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
