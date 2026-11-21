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
    AnovaSpec, BinSize, CorrelationSpec, CrosstabSpec, DiscSort, DiscreteFreqSpec, FreqOptions, FriedmanSpec,
    GraphArrangement, GraphSummarySpec, HistogramSpec, KruskalWallisSpec, KsTwoSampleSpec, MannWhitneySpec,
    MeansComparison, MedianTestSpec, NormalitySpec, OutlierSpec, PoissonTestKind, PoissonTestSpec, PropPowerSpec,
    PropTestKind, PropTestSpec, RowStatsSpec, SignTestSpec, SignedRankKind, SignedRankSpec, StatisticsSpec, TTestKind,
    TTestSpec, Tail, TwoWayAnovaSpec, TwoWayLayout, VarTestKind, VarTestSpec,
};
use sheaf_core::analysis::{
    AreaType, BaselineSpec, Boundary, ColumnMathSpec, ConvolutionSpec, CrossingIndex, CurveMathSpec, CurveOp,
    CurveOperand, DeconvolutionSpec, DiffOptions, IntegrateRangesSpec, IntegrateSpec, InterpMethod, InterpSpec,
    LevelCrossingSpec, LocalSpan, NormalizeMethod, OutputTarget, PeakDirection, PeakFilter, PeakOptions, Report,
    Request, RowRange, SignalCorrelationSpec, SmoothMethod, SmoothOptions, SplineBoundary, StftOutput, StftSpec, Tool,
};
use sheaf_core::analysis::{
    BaselineStat, ChainBaseline, ChainOptions, FindMethod, FindOptions, FitBaseline, LimitMode, PeakBaselineSpec,
    PeakChainSpec, PeakFitSpec, PeakShape,
};
use sheaf_core::analysis::{
    CoherenceOptions, CoherenceSpec, CoherenceWindow, DecimateFilter, DecimateSpec, EnvelopeOptions, EnvelopeSpec,
    EnvelopeType, FftFilterOptions, FftFilterSpec, FftFilterType, FftSpec, FftWindow, IfftSpec, PowerNorm,
    TraceInterpSpec, TraceMethod, WindowCorrection,
};
use sheaf_core::analysis::{NormalizeRef, RefStat};
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
    /// Simple Column Math (Origin's `vmathtool`): a column with a constant or another column, row by row.
    ColumnMath,
    /// Signal Processing > Convolution (Origin's `conv`).
    Convolution,
    /// Signal Processing > Deconvolution (Origin's `deconv`, T57).
    Deconvolution,
    /// Signal Processing > Correlation (Origin's `corr1`).
    SignalCorrelation,
    /// Level Crossing (Origin's `levelcrossing` X-Function, which has no menu item there).
    LevelCrossing,
    /// Signal Processing > Hilbert Transform (Origin's `hilbert`).
    Hilbert,
    /// Signal Processing > FFT > FFT (Origin's `fft1`).
    Fft,
    /// Signal Processing > STFT (Origin's `stft`, T57).
    Stft,
    /// Signal Processing > FFT > IFFT (Origin's `ifft1`).
    Ifft,
    /// Signal Processing > FFT Filters (Origin's `fft_filters`).
    FftFilter,
    /// Signal Processing > IIR Filter (Origin's `dfilter`, OriginPro).
    IirFilter,
    /// Signal Processing > FIR Filter (Origin 2027's FIR Filter app).
    FirFilter,
    /// Signal Processing > Envelope (Origin's `envelope`).
    Envelope,
    /// Signal Processing > Coherence (Origin's `cohere`).
    Coherence,
    /// Signal Processing > Decimation (Origin's `decimate`).
    Decimate,
    /// Signal Processing > Wavelet > Decompose (Origin's `dwt`; options in `wavelet.rs`).
    WaveletDecompose,
    /// Signal Processing > Wavelet > Reconstruction (Origin's `idwt`).
    WaveletReconstruct,
    /// Signal Processing > Wavelet > Denoise (Origin's `wtdenoise`).
    WaveletDenoise,
    /// Signal Processing > Wavelet > Smooth (Origin's `wtsmooth`).
    WaveletSmooth,
    /// Signal Processing > Wavelet > Continuous Wavelet (Origin's `cwt`).
    ContinuousWavelet,
    /// Mathematics > Trace Interpolation (Origin's `interp1trace`).
    TraceInterp,
    Interpolate,
    Peaks,
    /// Baseline, peak search, per-peak integration and an optional multi-peak fit.
    PeakChain,
    Statistics,
    /// The Statistics menu: t-tests, one-way ANOVA, histogram and normality tests.
    TTest,
    /// Statistics > Hypothesis Testing > One-Sample Test for Variance.
    VarTestOne,
    /// Statistics > Hypothesis Testing > Two-Sample Test for Variance.
    VarTestTwo,
    /// Statistics > Hypothesis Testing > One-Sample Proportion Test (counts typed in, no data).
    PropTestOne,
    /// Statistics > Hypothesis Testing > Two-Sample Proportion Test.
    PropTestTwo,
    /// Statistics > Hypothesis Testing > One-Sample Poisson Rate Test (Origin 2026b, T267; counts typed in).
    PoissonOne,
    /// Statistics > Hypothesis Testing > Two-Sample Poisson Rate Test (T267).
    PoissonTwo,
    Anova,
    Histogram,
    /// Statistics > Descriptive Statistics > Discrete Frequency.
    DiscreteFreq,
    /// Statistics > Descriptive Statistics > 2D Frequency Counts/Binning (options in `bin2d_options.rs`).
    Binning2D,
    Normality,
    /// Statistics > ANOVA > Two-Way ANOVA.
    Anova2,
    /// Statistics > ANOVA > Three-Way ANOVA (options in `stats3_options.rs`).
    Anova3,
    /// Statistics > ANOVA > One-Way Repeated Measures ANOVA (options in `rm_options.rs`).
    RmAnova1,
    /// Statistics > ANOVA > Two-Way Repeated Measures ANOVA.
    RmAnova2,
    /// Statistics > Descriptive Statistics > Partial Correlation Coefficient (options in `stats3_options.rs`).
    PartialCorr,
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
    /// Statistics > Descriptive Statistics > Distribution Fit, Probability Plot and Q-Q Plot
    /// (options in `dist_options.rs`).
    DistFit,
    ProbPlot,
    QQPlot,
    /// Statistics > Multivariate Analysis and ROC Curve (options in `multivariate_options.rs`).
    Pca,
    Roc,
    /// Statistics > Survival Analysis > Kaplan-Meier Estimator (options in `multivariate_options.rs`).
    KaplanMeier,
    /// Statistics > Multivariate Analysis > K-Means Cluster Analysis (options in `multivariate_options.rs`).
    Kmeans,
    /// Statistics > Descriptive Statistics > Cross Tabulation.
    Crosstab,
    /// Statistics > Descriptive Statistics > Graphical Summary (T40).
    GraphSummary,
    /// Analysis > Mathematics: Average Multiple Curves, Intersect and Polygon Area.
    AverageCurves,
    Intersect,
    PolygonArea,
    /// Analysis > Mathematics > XYZ Surface Area (T91; options in `xyz_options.rs`).
    XyzArea,
    /// Data Manipulation (options in `manip_options.rs`) and Mathematics > Median Filter.
    SubtractLine,
    ReduceDup,
    ReduceX,
    TranslateX,
    TranslateY,
    MedianFilter,
}

impl AnalysisKind {
    pub const ALL: [AnalysisKind; 33] = [
        AnalysisKind::Integrate,
        AnalysisKind::IntegrateRanges,
        AnalysisKind::Differentiate,
        AnalysisKind::Smooth,
        AnalysisKind::Normalize,
        AnalysisKind::Baseline,
        AnalysisKind::SubtractRef,
        AnalysisKind::CurveMath,
        AnalysisKind::ColumnMath,
        AnalysisKind::Convolution,
        AnalysisKind::Deconvolution,
        AnalysisKind::SignalCorrelation,
        AnalysisKind::LevelCrossing,
        AnalysisKind::Hilbert,
        AnalysisKind::Fft,
        AnalysisKind::Stft,
        AnalysisKind::Ifft,
        AnalysisKind::FftFilter,
        AnalysisKind::IirFilter,
        AnalysisKind::FirFilter,
        AnalysisKind::Envelope,
        AnalysisKind::Coherence,
        AnalysisKind::Decimate,
        AnalysisKind::WaveletDecompose,
        AnalysisKind::WaveletReconstruct,
        AnalysisKind::WaveletDenoise,
        AnalysisKind::WaveletSmooth,
        AnalysisKind::ContinuousWavelet,
        AnalysisKind::TraceInterp,
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
            AnalysisKind::ColumnMath => "Simple Column Math...",
            AnalysisKind::Convolution => "Convolution...",
            AnalysisKind::Deconvolution => "Deconvolution...",
            AnalysisKind::Stft => "STFT...",
            AnalysisKind::SignalCorrelation => "Correlation...",
            AnalysisKind::LevelCrossing => "Level Crossing...",
            AnalysisKind::Hilbert => "Hilbert Transform...",
            AnalysisKind::Fft => "FFT...",
            AnalysisKind::Ifft => "IFFT...",
            AnalysisKind::FftFilter => "FFT Filters...",
            AnalysisKind::IirFilter => "IIR Filter...",
            AnalysisKind::FirFilter => "FIR Filter...",
            AnalysisKind::Envelope => "Envelope...",
            AnalysisKind::Coherence => "Coherence...",
            AnalysisKind::Decimate => "Decimation...",
            AnalysisKind::WaveletDecompose => "Decompose...",
            AnalysisKind::WaveletReconstruct => "Reconstruction...",
            AnalysisKind::WaveletDenoise => "Denoise...",
            AnalysisKind::WaveletSmooth => "Smooth...",
            AnalysisKind::ContinuousWavelet => "Continuous Wavelet...",
            AnalysisKind::TraceInterp => "Trace Interpolation...",
            AnalysisKind::Interpolate => "Interpolate...",
            AnalysisKind::Peaks => "Find Peaks...",
            AnalysisKind::PeakChain => "Peak Analyzer...",
            AnalysisKind::Statistics => "Statistics on Columns...",
            AnalysisKind::TTest => "t-Test...",
            AnalysisKind::VarTestOne => "One-Sample Test for Variance...",
            AnalysisKind::VarTestTwo => "Two-Sample Test for Variance...",
            AnalysisKind::PropTestOne => "One-Sample Proportion Test...",
            AnalysisKind::PropTestTwo => "Two-Sample Proportion Test...",
            AnalysisKind::PoissonOne => "One-Sample Poisson Rate Test...",
            AnalysisKind::PoissonTwo => "Two-Sample Poisson Rate Test...",
            AnalysisKind::Anova => "One-Way ANOVA...",
            AnalysisKind::Histogram => "Histogram...",
            AnalysisKind::Binning2D => "2D Frequency Counts/Binning...",
            AnalysisKind::DiscreteFreq => "Discrete Frequency...",
            AnalysisKind::Normality => "Normality Test...",
            AnalysisKind::Anova2 => "Two-Way ANOVA...",
            AnalysisKind::Anova3 => "Three-Way ANOVA...",
            AnalysisKind::RmAnova1 => "One-Way Repeated Measures ANOVA...",
            AnalysisKind::RmAnova2 => "Two-Way Repeated Measures ANOVA...",
            AnalysisKind::PartialCorr => "Partial Correlation Coefficient...",
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
            AnalysisKind::DistFit => "Distribution Fit...",
            AnalysisKind::ProbPlot => "Probability Plot...",
            AnalysisKind::QQPlot => "Q-Q Plot...",
            AnalysisKind::Pca => "Principal Component Analysis...",
            AnalysisKind::Roc => "ROC Curve...",
            AnalysisKind::KaplanMeier => "Kaplan-Meier Estimator...",
            AnalysisKind::Kmeans => "K-Means Cluster Analysis...",
            AnalysisKind::Crosstab => "Cross Tabulation...",
            AnalysisKind::GraphSummary => "Graphical Summary...",
            AnalysisKind::AverageCurves => "Average Multiple Curves...",
            AnalysisKind::Intersect => "Intersect...",
            AnalysisKind::PolygonArea => "Polygon Area...",
            AnalysisKind::XyzArea => "XYZ Surface Area...",
            AnalysisKind::SubtractLine => "Subtract Straight Line...",
            AnalysisKind::ReduceDup => "Reduce Duplicate X Data...",
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
            "column-math" => AnalysisKind::ColumnMath,
            "convolution" => AnalysisKind::Convolution,
            "deconvolution" => AnalysisKind::Deconvolution,
            "stft" => AnalysisKind::Stft,
            "signal-correlation" => AnalysisKind::SignalCorrelation,
            "level-crossing" => AnalysisKind::LevelCrossing,
            "hilbert" => AnalysisKind::Hilbert,
            "fft" => AnalysisKind::Fft,
            "ifft" => AnalysisKind::Ifft,
            "fft-filter" => AnalysisKind::FftFilter,
            "iir-filter" => AnalysisKind::IirFilter,
            "fir-filter" => AnalysisKind::FirFilter,
            "envelope" => AnalysisKind::Envelope,
            "coherence" => AnalysisKind::Coherence,
            "decimate" => AnalysisKind::Decimate,
            "wavelet-decompose" => AnalysisKind::WaveletDecompose,
            "wavelet-reconstruct" => AnalysisKind::WaveletReconstruct,
            "wavelet-denoise" => AnalysisKind::WaveletDenoise,
            "wavelet-smooth" => AnalysisKind::WaveletSmooth,
            "cwt" => AnalysisKind::ContinuousWavelet,
            "trace-interp" => AnalysisKind::TraceInterp,
            "interpolate" => AnalysisKind::Interpolate,
            "peaks" => AnalysisKind::Peaks,
            "peak-chain" => AnalysisKind::PeakChain,
            "statistics" => AnalysisKind::Statistics,
            "ttest" => AnalysisKind::TTest,
            "vartest1" => AnalysisKind::VarTestOne,
            "vartest2" => AnalysisKind::VarTestTwo,
            "proptest1" => AnalysisKind::PropTestOne,
            "proptest2" => AnalysisKind::PropTestTwo,
            "poissontest1" => AnalysisKind::PoissonOne,
            "poissontest2" => AnalysisKind::PoissonTwo,
            "anova" => AnalysisKind::Anova,
            "histogram" => AnalysisKind::Histogram,
            "bin2d" => AnalysisKind::Binning2D,
            "discfreq" => AnalysisKind::DiscreteFreq,
            "normality" => AnalysisKind::Normality,
            "anova2" => AnalysisKind::Anova2,
            "anova3" => AnalysisKind::Anova3,
            "rmanova1" => AnalysisKind::RmAnova1,
            "rmanova2" => AnalysisKind::RmAnova2,
            "partial-corr" => AnalysisKind::PartialCorr,
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
            "distfit" => AnalysisKind::DistFit,
            "probplot" => AnalysisKind::ProbPlot,
            "qqplot" => AnalysisKind::QQPlot,
            "pca" => AnalysisKind::Pca,
            "roc" => AnalysisKind::Roc,
            "kaplanmeier" => AnalysisKind::KaplanMeier,
            "kmeans" => AnalysisKind::Kmeans,
            "crosstab" => AnalysisKind::Crosstab,
            "graphsummary" => AnalysisKind::GraphSummary,
            "average-curves" => AnalysisKind::AverageCurves,
            "intersect" => AnalysisKind::Intersect,
            "polygon-area" => AnalysisKind::PolygonArea,
            "xyz-area" => AnalysisKind::XyzArea,
            "subtract-line" => AnalysisKind::SubtractLine,
            "reduce-dup" => AnalysisKind::ReduceDup,
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

/// Origin's Baseline Modes of the Peak Analyzer that Sheaf offers (the others are listed greyed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainBaselineKind {
    /// Origin's default: Y is a value of the data (minimum by default) or a custom number.
    Constant,
    /// None (Y=0).
    None,
    /// Straight Line: through the first and the last point (Sheaf's reading of the mode).
    EndPoints,
    /// User Defined, with Sheaf's anchors: the lowest points of n stretches.
    Minima,
    /// User Defined with Origin's 2nd Derivative anchor finding (Origin's default method).
    SecondDerivative,
}

/// The Goal page of Origin's Peak Analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainGoal {
    Integrate,
    CreateBaseline,
    SubtractBaseline,
    Find,
    Fit,
}

impl ChainGoal {
    fn name(self) -> &'static str {
        match self {
            ChainGoal::Integrate => "Integrate Peaks",
            ChainGoal::CreateBaseline => "Create Baseline",
            ChainGoal::SubtractBaseline => "Subtract Baseline",
            ChainGoal::Find => "Find Peaks",
            ChainGoal::Fit => "Fit Peaks",
        }
    }

    /// Origin's pages after Goal for this goal (measured, analysis-tools.md 11.1).
    fn pages(self) -> &'static [&'static str] {
        match self {
            ChainGoal::Integrate => &["Baseline Mode", "Baseline Treatment", "Find Peaks", "Integrate Peaks", "Finish"],
            ChainGoal::Find => &["Baseline Mode", "Baseline Treatment", "Find Peaks", "Finish"],
            ChainGoal::Fit => &["Baseline Mode", "Baseline Treatment", "Find Peaks", "Fit Peaks", "Finish"],
            ChainGoal::CreateBaseline => &["Baseline Mode", "Create Baseline", "Finish"],
            ChainGoal::SubtractBaseline => &["Baseline Mode", "Subtract Baseline", "Finish"],
        }
    }
}

/// Two columns of labels and controls, as the analysis windows lay them out.
fn grid_rows(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([14.0, 6.0]).show(ui, add);
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

/// The Normalize choices after [`NORMALIZE_CHOICES`] that need a reference column (T260): Origin's
/// `ref` at this index and `cell` right after it.
const NORMALIZE_MATCH: usize = NORMALIZE_CHOICES.len();
const NORMALIZE_REF_LABELS: [&str; 2] = ["Match a statistic of a reference column", "Divide by a reference cell"];

/// Confidence levels in percent typed as numbers separated by spaces or commas.
fn parse_levels(text: &str) -> Result<Vec<f64>, String> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .map(|w| w.parse::<f64>().map_err(|_| format!("\"{w}\" is not a confidence level")))
        .collect()
}

/// The Hypothetical Sample Size(s) box: whole numbers separated by spaces or commas.
fn parse_sizes(text: &str) -> Result<Vec<u64>, String> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .map(|w| w.parse::<u64>().map_err(|_| format!("\"{w}\" is not a sample size")))
        .collect()
}

/// The Alternate Hypothesis radios of the variance and proportion tests: `Variance <> 1`, `> 1`, `< 1`.
fn alternative_radios(ui: &mut egui::Ui, tail: &mut Tail, what: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label("Alternate Hypothesis");
        ui.radio_value(tail, Tail::Two, format!("{what} <> {value}"));
        ui.radio_value(tail, Tail::Upper, format!("{what} > {value}"));
        ui.radio_value(tail, Tail::Lower, format!("{what} < {value}"));
    });
}

/// Significance Level, the Confidence Interval(s) tick and the Confidence Level(s) box.
fn significance_and_intervals(ui: &mut egui::Ui, st: &mut StatsOptions) {
    ui.horizontal(|ui| {
        ui.label("Significance Level");
        ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.4999));
    });
    ui.checkbox(&mut st.intervals, "Confidence Interval(s)");
    ui.add_enabled_ui(st.intervals, |ui| {
        ui.horizontal(|ui| {
            ui.label("Confidence Level(s) in %");
            ui.add(egui::TextEdit::singleline(&mut st.levels_text).desired_width(120.0));
        });
    });
}

/// The options of the Statistics menu windows.
struct StatsOptions {
    /// 0 one-sample, 1 two-sample, 2 paired.
    ttest: usize,
    mean: f64,
    difference: f64,
    equal_variance: bool,
    /// Test of variance: the hypothesized variance or ratio of variances.
    var_hyp: f64,
    /// Proportion tests: counts of each sample, the test proportion and difference.
    successes: [u64; 2],
    sample_size: [u64; 2],
    prop_hyp: f64,
    prop_diff: f64,
    /// Poisson rate tests (T267): total occurrence, sample size and exposure of each sample, the test rate
    /// and difference, and the two methods (Origin: Exact and Normal Approximation).
    occurrences: [u64; 2],
    poisson_size: [u64; 2],
    exposure: [f64; 2],
    rate_hyp: f64,
    rate_diff: f64,
    poisson_exact: bool,
    poisson_normal: bool,
    poisson_pooled: bool,
    /// Proportion tests: normal approximation and exact (binomial) test.
    normal_method: bool,
    binomial_method: bool,
    /// Two-sample proportion test: "Use pooled p for test" (Origin: off).
    pooled_p: bool,
    /// Proportion tests, Power Analysis: Actual Power, Hypothetical Power, their significance level and the
    /// hypothetical sample sizes as typed (Origin's defaults: off, 0.05, "50 100 200").
    power_actual: bool,
    power_hypothetical: bool,
    power_alpha: f64,
    power_sizes_text: String,
    /// Confidence intervals of the variance and proportion tests, and their levels in percent as typed.
    intervals: bool,
    levels_text: String,
    tail: Tail,
    alpha: f64,
    comparison: Option<MeansComparison>,
    /// Dunnett's control group: index among the chosen columns.
    control: usize,
    /// Cross tabulation: indexes into the sheet's columns, row variable and column variable.
    pair_columns: [usize; 2],
    cross: CrosstabSpec,
    /// Graphical Summary: level and arrangement, and the group columns (indexes into the sheet's columns).
    summary: GraphSummarySpec,
    summary_groups: Vec<usize>,
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
            var_hyp: 1.0,
            successes: [5, 5],
            sample_size: [10, 10],
            prop_hyp: 0.5,
            prop_diff: 0.0,
            occurrences: [10, 10],
            poisson_size: [10, 10],
            exposure: [1.0, 1.0],
            rate_hyp: 1.0,
            rate_diff: 0.0,
            poisson_exact: true,
            poisson_normal: true,
            poisson_pooled: true,
            normal_method: true,
            binomial_method: false,
            pooled_p: false,
            power_actual: false,
            power_hypothetical: false,
            power_alpha: 0.05,
            power_sizes_text: "50 100 200".to_string(),
            intervals: false,
            levels_text: "90 95 99".to_string(),
            tail: Tail::Two,
            alpha: 0.05,
            comparison: Some(MeansComparison::Tukey),
            control: 0,
            pair_columns: [0, 1],
            cross: CrosstabSpec::default(),
            summary: GraphSummarySpec::default(),
            summary_groups: Vec::new(),
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

/// The means comparisons of the one-way ANOVA (Dunnett's is checked against scipy, not Origin), then "None".
const COMPARISONS_ONE_WAY: [Option<MeansComparison>; 9] = [
    Some(MeansComparison::Tukey),
    Some(MeansComparison::Bonferroni),
    Some(MeansComparison::Sidak),
    Some(MeansComparison::FisherLsd),
    Some(MeansComparison::Scheffe),
    Some(MeansComparison::HolmBonferroni),
    Some(MeansComparison::HolmSidak),
    Some(MeansComparison::Dunnett),
    None,
];

/// The means comparisons of the two-way ANOVA (no Holm methods there), then "None".
const COMPARISONS_TWO_WAY: [Option<MeansComparison>; 6] = [
    Some(MeansComparison::Tukey),
    Some(MeansComparison::Bonferroni),
    Some(MeansComparison::Sidak),
    Some(MeansComparison::FisherLsd),
    Some(MeansComparison::Scheffe),
    None,
];

fn comparison_label(c: Option<MeansComparison>) -> &'static str {
    c.map_or("None", MeansComparison::label)
}

fn comparison_combo(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut Option<MeansComparison>,
    list: &[Option<MeansComparison>],
) {
    egui::ComboBox::from_id_salt(id).selected_text(comparison_label(*current)).show_ui(ui, |ui| {
        for c in list {
            ui.selectable_value(current, *c, comparison_label(*c));
        }
    });
}

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
pub(crate) fn next_undo_token() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub struct AnalysisDialog {
    /// The Peak Analyzer themes file (T169); `None` hides Save as Theme.
    pub peak_themes: Option<std::path::PathBuf>,
    /// The name typed for Save as Theme and what the last save did.
    theme_name: String,
    theme_note: Option<String>,
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
    /// T260: the statistic matched to the reference column's, and the reference cell's row (from 1).
    normalize_stat: RefStat,
    normalize_row: usize,
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
    /// User Defined, 2nd Derivative: Origin's Number of Points to Find (8).
    chain_points: usize,
    /// Create Baseline page: Origin's Interpolation Method (Line, Spline, BSpline).
    chain_interp: InterpMethod,
    /// Constant baseline: a value of the data, or `None` for the custom number.
    chain_const: Option<BaselineStat>,
    chain_custom_y: f64,
    find_method: usize,
    chain_half_height: bool,
    chain_goal: ChainGoal,
    /// The wizard page shown: 0 is Goal, then the pages of the goal.
    chain_page: usize,
    chain_fit: PeakShape,
    /// The function of each peak in the order found (`None`: the Function of `chain_fit`).
    chain_fit_shapes: Vec<Option<PeakShape>>,
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
    /// Convolution and Correlation: circular instead of linear, normalized, the X step (`None`:
    /// Convolution's automatic step from the input's X).
    signal_circular: bool,
    signal_normalize: bool,
    signal_interval: Option<f64>,
    level_crossing: LevelCrossingSpec,
    fft: FftSpec,
    /// STFT: window, lengths, overlap and output.
    stft: StftSpec,
    ifft: IfftSpec,
    fft_filter: FftFilterSpec,
    /// IIR Filter and FIR Filter (options in `filter_options.rs`).
    filters: super::filter_options::FilterOptions,
    envelope: EnvelopeSpec,
    /// Coherence: window, lengths, overlap and interval of Origin's `cohere`.
    coherence: CoherenceOptions,
    decimate: DecimateSpec,
    /// The Wavelet tools: options of Decompose, Reconstruction, Denoise and Smooth.
    wavelet: super::wavelet::WaveletOptions,
    trace_interp: TraceInterpSpec,
    stats: StatsOptions,
    dist: super::dist_options::DistOptions,
    multi: super::multivariate_options::MultiOptions,
    curves: super::math_curves::CurveOptions,
    bin2d: super::bin2d_options::Bin2dState,
    /// Partial correlation and three-way ANOVA windows.
    partcorr: super::stats3_options::PartCorrState,
    anova3: super::stats3_options::Anova3State,
    xyz: super::xyz_options::XyzState,
    rm: super::rm_options::RmState,
    discfreq: DiscreteFreqSpec,
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
/// A frequency or threshold that is Origin's automatic value until "Auto" is unticked.
fn auto_value(ui: &mut egui::Ui, label: &str, value: &mut Option<f64>, start: f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut auto = value.is_none();
        if ui.checkbox(&mut auto, "Auto").changed() {
            *value = if auto { None } else { Some(start) };
        }
        if let Some(v) = value {
            ui.add(DragValue::new(v).speed(0.001).range(0.0..=f64::MAX));
        }
    });
}

/// The options of Signal Processing > FFT Filters, with Origin's names.
fn fft_filter_options(ui: &mut egui::Ui, spec: &mut FftFilterSpec) {
    const KINDS: [(&str, FftFilterType); 6] = [
        ("Low Pass", FftFilterType::LowPass),
        ("High Pass", FftFilterType::HighPass),
        ("Band Pass", FftFilterType::BandPass),
        ("Band Block", FftFilterType::BandBlock),
        ("Threshold", FftFilterType::Threshold),
        ("Low Pass Parabolic", FftFilterType::LowPassParabolic),
    ];
    let o = &mut spec.options;
    ui.horizontal(|ui| {
        ui.label("Filter Type");
        egui::ComboBox::from_id_salt("fft_filter_type")
            .selected_text(KINDS.iter().find(|k| k.1 == o.kind).map_or("", |k| k.0))
            .show_ui(ui, |ui| {
                for (label, k) in KINDS {
                    ui.selectable_value(&mut o.kind, k, label);
                }
            });
    });
    auto_value(ui, "Sampling interval", &mut spec.interval, 1.0);
    match o.kind {
        FftFilterType::LowPass | FftFilterType::HighPass => auto_value(ui, "Cutoff Frequency", &mut o.cutoff, 0.1),
        FftFilterType::BandPass | FftFilterType::BandBlock => {
            auto_value(ui, "Lower Cutoff Frequency", &mut o.freq1, 0.1);
            auto_value(ui, "Upper Cutoff Frequency", &mut o.freq2, 0.3);
        }
        FftFilterType::Threshold => auto_value(ui, "Threshold", &mut o.threshold, 1.0),
        FftFilterType::LowPassParabolic => {
            auto_value(ui, "Pass Frequency", &mut o.pass, 0.1);
            auto_value(ui, "Stop Frequency", &mut o.stop, 0.3);
        }
    }
    let offset = matches!(o.kind, FftFilterType::HighPass | FftFilterType::BandPass | FftFilterType::BandBlock);
    ui.add_enabled(offset, egui::Checkbox::new(&mut o.keep_offset, "Keep DC offset"));
    ui.weak("Auto: cutoffs at a quarter and three quarters of the Nyquist frequency, the threshold at a tenth of the largest magnitude.");
}

/// The options of Signal Processing > Envelope, with Origin's names.
fn envelope_options(ui: &mut egui::Ui, spec: &mut EnvelopeSpec) {
    const KINDS: [(&str, EnvelopeType); 3] = [
        ("Upper Envelope", EnvelopeType::Upper),
        ("Lower Envelope", EnvelopeType::Lower),
        ("Both Envelopes", EnvelopeType::Both),
    ];
    let o: &mut EnvelopeOptions = &mut spec.options;
    ui.horizontal(|ui| {
        ui.label("Type");
        egui::ComboBox::from_id_salt("envelope_type")
            .selected_text(KINDS.iter().find(|k| k.1 == o.kind).map_or("", |k| k.0))
            .show_ui(ui, |ui| {
                for (label, k) in KINDS {
                    ui.selectable_value(&mut o.kind, k, label);
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Method");
        ui.add_enabled(false, egui::Label::new("By Spline of Extreme Pts"));
    });
    ui.horizontal(|ui| {
        ui.label("Points on each side");
        ui.add(DragValue::new(&mut o.npts).speed(0.1).range(0.0..=1.0e6));
    });
    ui.weak("An extreme is the first largest (smallest) value among this many points on each side; a natural cubic spline goes through the extremes.");
}

/// The options of Signal Processing > Coherence, with Origin's names.
fn coherence_options(ui: &mut egui::Ui, o: &mut CoherenceOptions) {
    ui.horizontal(|ui| {
        ui.label("Sampling interval");
        ui.add(DragValue::new(&mut o.interval).speed(0.01).range(1e-300..=f64::MAX));
    })
    .response
    .on_hover_text("The frequency step is 1 / (FFT length * interval). Origin does not take it from the X column.");
    ui.horizontal(|ui| {
        ui.label("FFT Length");
        egui::ComboBox::from_id_salt("coherence_fftlen").selected_text(o.fft_len.to_string()).show_ui(ui, |ui| {
            for n in [256, 512, 1024] {
                ui.selectable_value(&mut o.fft_len, n, n.to_string());
            }
        });
        ui.add(DragValue::new(&mut o.fft_len).speed(1).range(2..=1 << 24));
    });
    o.win_len = o.win_len.clamp(2, o.fft_len);
    ui.horizontal(|ui| {
        ui.label("Window Length");
        ui.add(DragValue::new(&mut o.win_len).speed(1).range(2..=o.fft_len))
            .on_hover_text("Each segment is zero padded to the FFT length; it cannot be longer.");
    });
    o.overlap = o.overlap.min(o.win_len - 1);
    ui.horizontal(|ui| {
        ui.label("Overlap");
        ui.add(DragValue::new(&mut o.overlap).speed(1).range(0..=o.win_len - 1))
            .on_hover_text("Points shared by neighbouring segments; less than the window length.");
    });
    let name = |w: CoherenceWindow| match w {
        CoherenceWindow::Rectangle => "Rectangle",
        CoherenceWindow::Welch => "Welch",
        CoherenceWindow::Triangular => "Triangular",
        CoherenceWindow::Bartlett => "Bartlett",
        CoherenceWindow::Hanning => "Hanning",
        CoherenceWindow::Hamming => "Hamming",
        CoherenceWindow::Blackman => "Blackman",
        CoherenceWindow::Gaussian { .. } => "Gaussian",
        CoherenceWindow::Kaiser { .. } => "Kaiser",
    };
    let choices = [
        CoherenceWindow::Rectangle,
        CoherenceWindow::Welch,
        CoherenceWindow::Triangular,
        CoherenceWindow::Bartlett,
        CoherenceWindow::Hanning,
        CoherenceWindow::Hamming,
        CoherenceWindow::Blackman,
        CoherenceWindow::Gaussian { alpha: 0.0 },
        CoherenceWindow::Kaiser { beta: 0.0 },
    ];
    ui.horizontal(|ui| {
        ui.label("Window");
        egui::ComboBox::from_id_salt("coherence_window").selected_text(name(o.window)).show_ui(ui, |ui| {
            for w in choices {
                let same = std::mem::discriminant(&w) == std::mem::discriminant(&o.window);
                if ui.selectable_label(same, name(w)).clicked() && !same {
                    o.window = w;
                }
            }
        });
    });
    match &mut o.window {
        CoherenceWindow::Gaussian { alpha } => {
            ui.horizontal(|ui| {
                ui.label("Alpha");
                ui.add(DragValue::new(alpha).speed(0.01).range(0.0..=f64::MAX));
            });
        }
        CoherenceWindow::Kaiser { beta } => {
            ui.horizontal(|ui| {
                ui.label("Beta");
                ui.add(DragValue::new(beta).speed(0.01).range(0.0..=f64::MAX));
            });
        }
        _ => {}
    }
    ui.weak(
        "Segments of the window length, no detrending; missing values and the end of the shorter signal count as 0.",
    );
}

/// The options of Signal Processing > Decimation, with Origin's names.
fn decimate_options(ui: &mut egui::Ui, spec: &mut DecimateSpec) {
    let o = &mut spec.options;
    ui.horizontal(|ui| {
        ui.label("Resample Factor");
        ui.add(DragValue::new(&mut o.factor).range(1..=100_000));
    });
    ui.horizontal(|ui| {
        ui.label("Filter Type");
        egui::ComboBox::from_id_salt("decimate_filter")
            .selected_text(match o.filter {
                DecimateFilter::None => "No Filter",
                DecimateFilter::MovingAverage => "Moving Average Filter",
                DecimateFilter::Fir => "FIR Filter",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut o.filter, DecimateFilter::None, "No Filter");
                ui.selectable_value(&mut o.filter, DecimateFilter::MovingAverage, "Moving Average Filter");
                ui.selectable_value(&mut o.filter, DecimateFilter::Fir, "FIR Filter");
            });
    });
    ui.add_enabled_ui(o.filter == DecimateFilter::Fir, |ui| {
        ui.horizontal(|ui| {
            ui.label("Order");
            ui.add(DragValue::new(&mut o.order).range(8..=60));
        });
    });
    ui.weak("A factor of 3 or more needs ten times as many points as the factor.");
}

/// The options of Mathematics > Trace Interpolation, with Origin's names.
fn trace_interp_options(ui: &mut egui::Ui, spec: &mut TraceInterpSpec) {
    const METHODS: [(&str, TraceMethod); 3] = [
        ("Linear", TraceMethod::Linear),
        ("Cubic Spline", TraceMethod::CubicSpline),
        ("Cubic B-Spline", TraceMethod::CubicBSpline),
    ];
    ui.horizontal(|ui| {
        ui.label("Method");
        egui::ComboBox::from_id_salt("trace_interp_method")
            .selected_text(METHODS.iter().find(|m| m.1 == spec.method).map_or("", |m| m.0))
            .show_ui(ui, |ui| {
                for (label, m) in METHODS {
                    ui.selectable_value(&mut spec.method, m, label);
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Number of Points");
        ui.add(DragValue::new(&mut spec.points).range(2..=1_000_000));
    });
    ui.weak("The points are evenly spaced along the path of the curve. Neighbouring points with the same X, then with the same Y, are first replaced by their mean.");
}

/// The options of Signal Processing > FFT > FFT, with Origin's names.
const WINDOWS: [(&str, FftWindow); 7] = [
    ("Rectangle", FftWindow::Rectangle),
    ("Welch", FftWindow::Welch),
    ("Triangular", FftWindow::Triangular),
    ("Bartlett", FftWindow::Bartlett),
    ("Hanning", FftWindow::Hanning),
    ("Hamming", FftWindow::Hamming),
    ("Blackman", FftWindow::Blackman),
];

/// STFT's options (T57; Origin's `stft` is documented, not measured).
fn stft_options(ui: &mut egui::Ui, spec: &mut StftSpec) {
    let o = &mut spec.options;
    ui.horizontal(|ui| {
        ui.label("Sampling interval");
        ui.checkbox(&mut spec.auto_interval, "Auto")
            .on_hover_text("The X step of the input column; 1 without an X column.");
        ui.add_enabled(!spec.auto_interval, DragValue::new(&mut o.interval).speed(0.01).range(1e-300..=f64::MAX));
    });
    ui.horizontal(|ui| {
        ui.label("Window");
        egui::ComboBox::from_id_salt("stft_window")
            .selected_text(WINDOWS.iter().find(|w| w.1 == o.window).map_or("", |w| w.0))
            .show_ui(ui, |ui| {
                for (label, w) in WINDOWS {
                    ui.selectable_value(&mut o.window, w, label);
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Window Length");
        ui.add(DragValue::new(&mut o.window_len).range(2..=1_048_576));
    });
    ui.horizontal(|ui| {
        ui.label("FFT Length");
        ui.add(DragValue::new(&mut o.fft_len).range(2..=1_048_576));
    })
    .response
    .on_hover_text("At least the window length; each segment is padded with zeros to it.");
    ui.horizontal(|ui| {
        ui.label("Overlap");
        ui.add(DragValue::new(&mut o.overlap).range(0..=1_048_575));
    })
    .response
    .on_hover_text("Points shared by neighbouring segments.");
    o.fft_len = o.fft_len.max(o.window_len);
    o.overlap = o.overlap.min(o.window_len.saturating_sub(1));
    ui.horizontal(|ui| {
        ui.label("Output");
        ui.radio_value(&mut o.output, StftOutput::Amplitude, "Amplitude");
        ui.radio_value(&mut o.output, StftOutput::Db, "dB");
    });
    ui.weak("Origin writes a matrix; Sheaf writes a frequency column and one column per segment.");
}

fn fft_options(ui: &mut egui::Ui, spec: &mut FftSpec) {
    let o = &mut spec.options;
    ui.horizontal(|ui| {
        ui.label("Sampling interval");
        ui.checkbox(&mut spec.auto_interval, "Auto")
            .on_hover_text("The X step of the input column; 1 without an X column.");
        ui.add_enabled(!spec.auto_interval, DragValue::new(&mut o.interval).speed(0.01).range(1e-300..=f64::MAX));
    });
    ui.horizontal(|ui| {
        ui.label("Window");
        egui::ComboBox::from_id_salt("fft_window")
            .selected_text(WINDOWS.iter().find(|w| w.1 == o.window).map_or("", |w| w.0))
            .show_ui(ui, |ui| {
                for (label, w) in WINDOWS {
                    ui.selectable_value(&mut o.window, w, label);
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Window Correction");
        ui.radio_value(&mut o.correction, WindowCorrection::None, "None");
        ui.radio_value(&mut o.correction, WindowCorrection::Amplitude, "Amplitude");
        ui.radio_value(&mut o.correction, WindowCorrection::Power, "Power");
    });
    ui.checkbox(&mut o.normalize, "Normalize Real, Imaginary and Magnitude");
    ui.horizontal(|ui| {
        ui.label("Spectrum Type");
        ui.radio_value(&mut o.two_sided, false, "One-sided");
        ui.radio_value(&mut o.two_sided, true, "Two-sided");
    });
    ui.add_enabled(o.two_sided, egui::Checkbox::new(&mut o.shift, "Shift (zero frequency in the middle)"));
    ui.checkbox(&mut o.unwrap, "Unwrap Phase");
    ui.horizontal(|ui| {
        ui.label("Factor");
        ui.radio_value(&mut o.science, false, "-1 (Electrical Engineering)");
        ui.radio_value(&mut o.science, true, "+1 (Science)");
    });
    ui.horizontal(|ui| {
        ui.label("Power");
        ui.radio_value(&mut o.power, PowerNorm::Msa, "MSA");
        ui.radio_value(&mut o.power, PowerNorm::Ssa, "SSA");
        ui.radio_value(&mut o.power, PowerNorm::Tisa, "TISA");
    })
    .response
    .on_hover_text("Mean square, sum square or time-interval square amplitude");
}

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
    /// Which tool the window runs (tests).
    #[cfg(test)]
    pub fn kind(&self) -> AnalysisKind {
        self.kind
    }

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
        // Convolution and Correlation may use an input itself (autocorrelation, as Origin).
        let signal = matches!(
            kind,
            AnalysisKind::Convolution
                | AnalysisKind::Deconvolution
                | AnalysisKind::SignalCorrelation
                | AnalysisKind::Ifft
                | AnalysisKind::Coherence
                | AnalysisKind::WaveletReconstruct
                | AnalysisKind::ContinuousWavelet
        );
        let reference_choices = reference_choices(project, if signal { &[] } else { &columns });
        // IFFT: the column right after the (first) input is the imaginary part, as an FFT sheet has it.
        let next_column = columns.first().and_then(|c| {
            let (_, sheet, _) = project.locate(*c)?;
            let at = sheet.column_index(*c)?;
            sheet.columns.get(at + 1).map(|n| n.id)
        });
        let first_reference = match (kind, next_column) {
            (AnalysisKind::Ifft, Some(id)) => reference_choices.iter().position(|(c, _)| *c == id).unwrap_or(0),
            _ => 0,
        };
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
        if kind == AnalysisKind::Crosstab {
            let position = |id: &ColumnId| sheet_columns.iter().position(|(c, _)| c == id);
            let chosen: Vec<usize> = columns.iter().filter_map(position).collect();
            let last = sheet_columns.len().saturating_sub(1);
            stats.pair_columns = if chosen.len() >= 2 { [chosen[0], chosen[1]] } else { [0, 1.min(last)] };
        }
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
        let anova3 = if kind == AnalysisKind::Anova3 {
            let chosen: Vec<usize> =
                columns.iter().filter_map(|id| sheet_columns.iter().position(|(c, _)| c == id)).collect();
            super::stats3_options::Anova3State::with_columns(&chosen, sheet_columns.len())
        } else {
            Default::default()
        };
        let xyz = {
            let chosen: Vec<usize> =
                columns.iter().filter_map(|id| sheet_columns.iter().position(|(c, _)| c == id)).collect();
            let at = |id: ColumnId| sheet_columns.iter().position(|(c, _)| *c == id);
            let default =
                sheaf_core::analysis::xyz_default_columns(sheet).and_then(|[x, y, z]| Some([at(x)?, at(y)?, at(z)?]));
            super::xyz_options::XyzState::new(&chosen, default, sheet_columns.len())
        };
        let rm = {
            let chosen: Vec<usize> =
                columns.iter().filter_map(|id| sheet_columns.iter().position(|(c, _)| c == id)).collect();
            super::rm_options::RmState::new(kind == AnalysisKind::RmAnova2, &chosen, sheet_columns.len())
        };
        Some(AnalysisDialog {
            peak_themes: None,
            theme_name: String::new(),
            theme_note: None,
            kind,
            column_names: names.join(", "),
            ranges: vec![(x_lo, x_hi)],
            curve_op: CurveOp::Add,
            // Origin's Simple Column Math starts on Constant 10.
            use_reference: kind == AnalysisKind::SubtractRef
                || (kind != AnalysisKind::ColumnMath && !reference_choices.is_empty()),
            constant: if kind == AnalysisKind::ColumnMath { 10.0 } else { 1.0 },
            reference_choices,
            reference: first_reference,
            common_only: false,
            signal_circular: false,
            signal_normalize: false,
            signal_interval: if kind == AnalysisKind::SignalCorrelation { Some(1.0) } else { None },
            level_crossing: LevelCrossingSpec { level: 0.0, index: CrossingIndex::Nearest, min_gap: 1, details: false },
            fft: FftSpec { auto_interval: true, ..Default::default() },
            stft: StftSpec::default(),
            ifft: IfftSpec { auto_interval: true, ..Default::default() },
            fft_filter: FftFilterSpec {
                options: FftFilterOptions { keep_offset: true, ..Default::default() },
                interval: None,
            },
            filters: super::filter_options::FilterOptions::default(),
            envelope: EnvelopeSpec::default(),
            coherence: CoherenceOptions::default(),
            decimate: DecimateSpec::default(),
            wavelet: super::wavelet::WaveletOptions::default(),
            trace_interp: TraceInterpSpec::default(),
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
            normalize_stat: RefStat::Mean,
            normalize_row: 1,
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
            chain_baseline: ChainBaselineKind::Constant,
            chain_minima: 5,
            chain_points: 8,
            chain_interp: InterpMethod::Linear,
            chain_const: Some(BaselineStat::Minimum),
            chain_custom_y: 0.0,
            find_method: 0,
            chain_half_height: false,
            chain_goal: ChainGoal::Integrate,
            chain_page: 0,
            chain_fit: PeakShape::Gauss,
            chain_fit_shapes: Vec::new(),
            chain_fit_linear: true,
            stats,
            dist: Default::default(),
            multi: Default::default(),
            curves: Default::default(),
            bin2d: Default::default(),
            partcorr: Default::default(),
            anova3,
            xyz,
            rm,
            discfreq: Default::default(),
            manip: super::manip_options::ManipOptions::new(x_lo, x_hi),
            sheet_columns,
            problem: None,
            picker: None,
            show_preview: true,
            auto_preview: false,
            preview: None,
            preview_request: None,
            recalc: RecalcMode::Manual,
            // Origin writes Simple Column Math back into its input column.
            output_choice: if kind == AnalysisKind::ColumnMath {
                OutputChoice::Existing
            } else {
                OutputChoice::NextToInput
            },
            output_existing: Vec::new(),
            auto_apply: false,
            applied: None,
            applied_key: None,
            applied_report: None,
            failed_key: None,
            undo_token: next_undo_token(),
        })
    }

    /// The peak search controls of Find Peaks and of the Peak Analyzer's Find Peaks page.
    fn peak_search_controls(&mut self, ui: &mut egui::Ui, chain: bool) {
        if chain {
            ui.horizontal(|ui| {
                ui.label("Method");
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
            ui.add_enabled(self.peak_keep == PeakKeep::Height, DragValue::new(&mut self.peak_height).speed(0.01));
        });
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.peak_keep, PeakKeep::Count, "Keep only the highest");
            ui.add_enabled(self.peak_keep == PeakKeep::Count, DragValue::new(&mut self.peak_count).range(1..=10_000));
        });
        ui.radio_value(&mut self.peak_keep, PeakKeep::All, "Keep every local extremum");
    }

    /// Origin's Peak Analyzer as a wizard: the page list of the goal, Prev and Next, and the page
    /// (analysis-tools.md 11.1). OK runs it (Origin's Finish).
    /// Save as Theme (T169): the window's settings under a name, for Batch Peak Analysis Using Theme.
    fn peak_theme_row(&mut self, ui: &mut egui::Ui) {
        let Some(path) = self.peak_themes.clone() else { return };
        ui.horizontal(|ui| {
            ui.label("Theme");
            ui.add(
                egui::TextEdit::singleline(&mut self.theme_name)
                    .desired_width(140.0)
                    .id(egui::Id::new("peak_theme_name")),
            );
            if ui.button("Save as Theme").clicked() {
                self.theme_note = Some(match self.tool() {
                    Ok(Tool::PeakChain(spec)) => match sheaf_io::peak_themes::save(&path, &self.theme_name, &spec) {
                        Ok((name, true)) => format!("Replaced the theme {name}"),
                        Ok((name, false)) => format!("Saved the theme {name}"),
                        Err(e) => e,
                    },
                    Ok(_) => "Only Peak Analyzer settings can be saved as a theme".to_string(),
                    Err(e) => e,
                });
            }
        });
        if let Some(note) = &self.theme_note {
            ui.weak(note.as_str());
        }
    }

    fn peak_wizard(&mut self, ui: &mut egui::Ui) {
        let pages = self.chain_goal.pages();
        self.chain_page = self.chain_page.min(pages.len());
        let current = if self.chain_page == 0 { "Goal" } else { pages[self.chain_page - 1] };
        egui::Frame::new().fill(egui::Color32::BLACK).inner_margin(6.0).show(ui, |ui| {
            ui.set_min_width(260.0);
            ui.vertical(|ui| {
                let mut go = None;
                let goal = format!("Goal - {}", self.chain_goal.name());
                let items = std::iter::once(goal.as_str()).chain(pages.iter().copied());
                for (k, name) in items.enumerate() {
                    let here = k == self.chain_page;
                    let marker = if here {
                        "\u{25A0}"
                    } else if k < self.chain_page {
                        "\u{25A3}"
                    } else {
                        "\u{25A1}"
                    };
                    let indent = if k == 0 { "" } else { "    " };
                    let color = if here { egui::Color32::WHITE } else { egui::Color32::YELLOW };
                    let mut text = egui::RichText::new(format!("{indent}{marker} {name}")).color(color);
                    if here {
                        text = text.strong();
                    }
                    if ui.add(egui::Label::new(text).sense(egui::Sense::click())).clicked() {
                        go = Some(k);
                    }
                }
                if let Some(k) = go {
                    self.chain_page = k;
                }
            });
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(self.chain_page > 0, egui::Button::new("Prev")).clicked() {
                self.chain_page -= 1;
            }
            // Next stops on the last page before Finish, as in Origin.
            if ui.add_enabled(self.chain_page + 1 < pages.len(), egui::Button::new("Next")).clicked() {
                self.chain_page += 1;
            }
            ui.weak("OK runs the analysis (Origin's Finish).");
        });
        self.peak_theme_row(ui);
        ui.separator();
        match current {
            "Goal" => {
                ui.label(egui::RichText::new("Define a baseline, find and integrate peaks").strong());
                ui.horizontal(|ui| {
                    ui.label("Goal");
                    ui.vertical(|ui| {
                        for g in [
                            ChainGoal::Integrate,
                            ChainGoal::CreateBaseline,
                            ChainGoal::SubtractBaseline,
                            ChainGoal::Find,
                            ChainGoal::Fit,
                        ] {
                            let before = self.chain_goal;
                            ui.radio_value(&mut self.chain_goal, g, g.name());
                            // Origin's baseline goals start on User Defined, the peak goals on Constant.
                            let baseline_goal =
                                |g: ChainGoal| matches!(g, ChainGoal::CreateBaseline | ChainGoal::SubtractBaseline);
                            if before != self.chain_goal && baseline_goal(before) != baseline_goal(self.chain_goal) {
                                self.chain_baseline = if baseline_goal(self.chain_goal) {
                                    ChainBaselineKind::SecondDerivative
                                } else {
                                    ChainBaselineKind::Constant
                                };
                            }
                        }
                    });
                });
            }
            "Baseline Mode" => {
                let baseline_goal = matches!(self.chain_goal, ChainGoal::CreateBaseline | ChainGoal::SubtractBaseline);
                // Origin's list for the baseline goals (measured).
                const BASELINE_GOAL_MODES: [(&str, Option<ChainBaselineKind>); 4] = [
                    ("User Defined", Some(ChainBaselineKind::SecondDerivative)),
                    ("XPS", None),
                    ("End Points Weighted", None),
                    ("Asymmetric Least Squares Smoothing", None),
                ];
                const MODES: [(&str, Option<ChainBaselineKind>); 8] = [
                    ("Constant", Some(ChainBaselineKind::Constant)),
                    ("User Defined", Some(ChainBaselineKind::SecondDerivative)),
                    ("Use Existing Dataset", None),
                    ("XPS", None),
                    ("None (Y=0)", Some(ChainBaselineKind::None)),
                    ("End Points Weighted", None),
                    ("Straight Line", Some(ChainBaselineKind::EndPoints)),
                    ("Asymmetric Least Squares Smoothing", None),
                ];
                let modes: &[(&str, Option<ChainBaselineKind>)] =
                    if baseline_goal { &BASELINE_GOAL_MODES } else { &MODES };
                // Both ways of finding anchors are Origin's User Defined.
                let shown = match self.chain_baseline {
                    ChainBaselineKind::Minima => ChainBaselineKind::SecondDerivative,
                    k => k,
                };
                let name = modes.iter().find(|m| m.1 == Some(shown)).map_or("", |m| m.0);
                ui.horizontal(|ui| {
                    ui.label("Baseline Mode");
                    egui::ComboBox::from_id_salt("pa_basemode").selected_text(name).width(240.0).show_ui(ui, |ui| {
                        for &(label, kind) in modes {
                            match kind {
                                Some(k) => {
                                    ui.selectable_value(&mut self.chain_baseline, k, label);
                                }
                                None => {
                                    ui.add_enabled(false, egui::Button::selectable(false, label));
                                }
                            }
                        }
                    });
                });
                match self.chain_baseline {
                    ChainBaselineKind::Constant => {
                        ui.horizontal(|ui| {
                            ui.label("Y =");
                            for (label, stat) in [
                                ("Minimum", Some(BaselineStat::Minimum)),
                                ("Maximum", Some(BaselineStat::Maximum)),
                                ("Mean", Some(BaselineStat::Mean)),
                                ("Median", Some(BaselineStat::Median)),
                                ("Custom", None),
                            ] {
                                ui.radio_value(&mut self.chain_const, stat, label);
                            }
                            ui.add_enabled(
                                self.chain_const.is_none(),
                                DragValue::new(&mut self.chain_custom_y).speed(0.01),
                            );
                        });
                    }
                    ChainBaselineKind::Minima | ChainBaselineKind::SecondDerivative => {
                        ui.label(egui::RichText::new("Anchor Points Finding").strong());
                        ui.horizontal_top(|ui| {
                            ui.label("Method");
                            ui.vertical(|ui| {
                                ui.radio_value(
                                    &mut self.chain_baseline,
                                    ChainBaselineKind::SecondDerivative,
                                    "2nd Derivative",
                                );
                                for greyed in [
                                    "1st Derivative and 2nd Derivative",
                                    "Use Script to Search",
                                    "Use Existing Dataset",
                                ] {
                                    ui.add_enabled(false, egui::RadioButton::new(false, greyed));
                                }
                                ui.radio_value(
                                    &mut self.chain_baseline,
                                    ChainBaselineKind::Minima,
                                    "Lowest point of each stretch (Sheaf's own)",
                                );
                            });
                        });
                        if self.chain_baseline == ChainBaselineKind::SecondDerivative {
                            grid_rows(ui, "pa_anchor_find", |ui| {
                                ui.label("Smoothing Window Size");
                                ui.add_enabled(false, egui::Label::new("Auto"));
                                ui.end_row();
                                ui.label("Threshold");
                                ui.add_enabled(false, egui::Label::new("Auto"));
                                ui.end_row();
                                ui.label("Number of Points to Find");
                                ui.add(DragValue::new(&mut self.chain_points).range(2..=1000));
                                ui.end_row();
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label("Anchor points: the lowest point of each of");
                                ui.add(DragValue::new(&mut self.chain_minima).range(2..=1000));
                                ui.label("stretches");
                            });
                        }
                    }
                    ChainBaselineKind::EndPoints => {
                        ui.weak("The line through the first and the last point.");
                    }
                    ChainBaselineKind::None => {}
                }
            }
            "Create Baseline" => {
                grid_rows(ui, "pa_basecreate", |ui| {
                    ui.label("Connect by");
                    ui.add_enabled(false, egui::Button::selectable(true, "Interpolation"));
                    ui.end_row();
                    ui.label("Number of Baseline Points");
                    ui.add_enabled(false, egui::Checkbox::new(&mut true, "Same as Input"));
                    ui.end_row();
                    ui.label("Interpolation Method");
                    ui.vertical(|ui| {
                        ui.radio_value(&mut self.chain_interp, InterpMethod::Linear, "Line");
                        ui.radio_value(&mut self.chain_interp, InterpMethod::CubicSpline, "Spline");
                        ui.radio_value(&mut self.chain_interp, InterpMethod::BSpline, "BSpline");
                    });
                    ui.end_row();
                });
                ui.weak("The baseline goes beside the data as Baseline_Data Y1, Y2, ...");
            }
            "Subtract Baseline" => {
                ui.weak(
                    "The data minus the baseline go beside the data as Subtracted_Data Y1, followed by the \
                     baseline as Baseline_Data Y1.",
                );
            }
            "Baseline Treatment" => {
                ui.add_enabled(false, egui::Checkbox::new(&mut false, "Auto Subtract Baseline"));
                ui.add_enabled(false, egui::Checkbox::new(&mut false, "Auto Rescale"));
                ui.weak(
                    "The peaks are measured from the baseline; a column with the baseline is written beside the data.",
                );
            }
            "Find Peaks" => self.peak_search_controls(ui, true),
            "Integrate Peaks" => {
                ui.horizontal(|ui| {
                    ui.label("Integrate From");
                    ui.add_enabled(false, egui::Button::selectable(true, "Baseline"));
                });
                ui.checkbox(&mut self.chain_half_height, "Integrate only the upper half of each peak (Sheaf's own)");
                ui.weak(
                    "Quantities: Peak Area, Percent Area (of the whole curve's area), Row Index, Beginning X, \
                     Ending X, Peak Center, Peak Height, FWHM.",
                );
            }
            "Fit Peaks" => {
                ui.horizontal(|ui| {
                    ui.label("Function");
                    ui.radio_value(&mut self.chain_fit, PeakShape::Gauss, "Gauss");
                    ui.radio_value(&mut self.chain_fit, PeakShape::Lorentz, "Lorentz");
                    ui.radio_value(&mut self.chain_fit, PeakShape::Voigt, "Voigt");
                });
                ui.checkbox(&mut self.chain_fit_linear, "Sloped baseline in the fit");
                // A function per peak, in the order found; peaks beyond the list take the Function
                // above (Sheaf's own layout; Origin's Fit Control window was not opened).
                ui.label("Function per peak (in the order found; the others use the Function above)");
                let mut remove = None;
                for (k, choice) in self.chain_fit_shapes.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let text = choice.map_or("Same as above", PeakShape::name);
                        egui::ComboBox::from_id_salt(("peak_fit_shape", k))
                            .selected_text(text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(choice, None, "Same as above");
                                for s in [PeakShape::Gauss, PeakShape::Lorentz, PeakShape::Voigt] {
                                    ui.selectable_value(choice, Some(s), s.name());
                                }
                            })
                            .response
                            .on_hover_text(format!("Peak {}", k + 1));
                        ui.label(format!("Peak {}", k + 1));
                        if ui.small_button("Remove").clicked() {
                            remove = Some(k);
                        }
                    });
                }
                if let Some(k) = remove {
                    self.chain_fit_shapes.remove(k);
                }
                if ui.button("Add Peak Function").clicked() {
                    self.chain_fit_shapes.push(None);
                }
            }
            _ => {
                ui.label(format!("{}: press OK to run it.", self.chain_goal.name()));
            }
        }
    }

    /// The baseline the Peak Analyzer's Baseline Mode page describes.
    fn chain_baseline_spec(&self) -> ChainBaseline {
        match self.chain_baseline {
            ChainBaselineKind::Constant => match self.chain_const {
                Some(stat) => ChainBaseline::ConstantStat(stat),
                None => ChainBaseline::Constant(self.chain_custom_y),
            },
            ChainBaselineKind::None => ChainBaseline::None,
            ChainBaselineKind::EndPoints => ChainBaseline::EndPoints,
            ChainBaselineKind::Minima => ChainBaseline::AutoMinima(self.chain_minima),
            ChainBaselineKind::SecondDerivative => {
                // The baseline goals join the anchors with the Create Baseline page's method; the
                // peak goals with straight lines.
                let method = match self.chain_goal {
                    ChainGoal::CreateBaseline | ChainGoal::SubtractBaseline => self.chain_interp,
                    _ => InterpMethod::Linear,
                };
                ChainBaseline::SecondDerivative { points: self.chain_points, method }
            }
        }
    }

    /// Chooses Normalize's reference methods (T260): the statistic match, or the cell when `stat` is
    /// `None`. Tests use it; the window has the Method list.
    #[cfg(test)]
    pub fn choose_normalize_reference(&mut self, stat: Option<RefStat>, row: usize) {
        match stat {
            Some(s) => (self.normalize, self.normalize_stat) = (NORMALIZE_MATCH, s),
            None => (self.normalize, self.normalize_row) = (NORMALIZE_MATCH + 1, row),
        }
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
            AnalysisKind::LevelCrossing => Tool::LevelCrossing(self.level_crossing),
            AnalysisKind::Hilbert => Tool::Hilbert,
            AnalysisKind::Fft => Tool::Fft(self.fft),
            AnalysisKind::Stft => Tool::Stft(self.stft),
            AnalysisKind::FftFilter => Tool::FftFilter(self.fft_filter),
            AnalysisKind::IirFilter => self.filters.iir_tool(),
            AnalysisKind::FirFilter => self.filters.fir_tool(),
            AnalysisKind::Envelope => Tool::Envelope(self.envelope),
            AnalysisKind::Coherence => {
                let other = self
                    .reference_choices
                    .get(self.reference)
                    .map(|(id, _)| *id)
                    .ok_or("there is no other column with numbers to use as the second signal")?;
                Tool::Coherence(CoherenceSpec { other, options: self.coherence })
            }
            AnalysisKind::Decimate => Tool::Decimate(self.decimate),
            AnalysisKind::WaveletDecompose => self.wavelet.decompose_tool(),
            AnalysisKind::WaveletReconstruct => {
                let detail = self
                    .reference_choices
                    .get(self.reference)
                    .map(|(id, _)| *id)
                    .ok_or("there is no other column with numbers to use as the detail coefficients")?;
                self.wavelet.reconstruct_tool(detail)
            }
            AnalysisKind::WaveletDenoise => self.wavelet.denoise_tool(),
            AnalysisKind::WaveletSmooth => self.wavelet.smooth_tool(),
            AnalysisKind::ContinuousWavelet => {
                let scales = self
                    .reference_choices
                    .get(self.reference)
                    .map(|(id, _)| *id)
                    .ok_or("there is no other column with numbers to use as the scales")?;
                self.wavelet.cwt_tool(scales)
            }
            AnalysisKind::TraceInterp => Tool::TraceInterp(self.trace_interp),
            AnalysisKind::Ifft => {
                let imaginary = if self.use_reference {
                    Some(
                        self.reference_choices
                            .get(self.reference)
                            .map(|(id, _)| *id)
                            .ok_or("there is no other column with numbers to use as the imaginary part")?,
                    )
                } else {
                    None
                };
                Tool::Ifft(IfftSpec { imaginary, ..self.ifft })
            }
            AnalysisKind::Convolution | AnalysisKind::Deconvolution | AnalysisKind::SignalCorrelation => {
                let other = self
                    .reference_choices
                    .get(self.reference)
                    .map(|(id, _)| *id)
                    .ok_or("there is no other column with numbers to use as the second signal")?;
                let (circular, normalize) = (self.signal_circular, self.signal_normalize);
                if self.kind == AnalysisKind::Deconvolution {
                    Tool::Deconvolution(DeconvolutionSpec {
                        response: other,
                        normalize,
                        interval: self.signal_interval,
                    })
                } else if self.kind == AnalysisKind::Convolution {
                    Tool::Convolution(ConvolutionSpec {
                        response: other,
                        circular,
                        normalize,
                        interval: self.signal_interval,
                    })
                } else {
                    let interval = self.signal_interval.unwrap_or(1.0);
                    Tool::SignalCorrelation(SignalCorrelationSpec { other, circular, normalize, interval })
                }
            }
            AnalysisKind::ColumnMath => {
                let operand = if self.use_reference {
                    self.reference_choices
                        .get(self.reference)
                        .map(|(id, _)| CurveOperand::Reference(*id))
                        .ok_or("there is no other column with numbers to use as the operand")?
                } else {
                    CurveOperand::Constant(self.constant)
                };
                Tool::ColumnMath(ColumnMathSpec { op: self.curve_op, operand })
            }
            AnalysisKind::SubtractLine
            | AnalysisKind::ReduceDup
            | AnalysisKind::ReduceX
            | AnalysisKind::TranslateX
            | AnalysisKind::TranslateY
            | AnalysisKind::MedianFilter => self.manip.tool(self.kind),
            AnalysisKind::Differentiate => Tool::Differentiate(self.diff),
            AnalysisKind::DistFit | AnalysisKind::ProbPlot | AnalysisKind::QQPlot => self.dist.tool(self.kind)?,
            AnalysisKind::Pca | AnalysisKind::Roc | AnalysisKind::KaplanMeier | AnalysisKind::Kmeans => {
                self.multi.tool(self.kind)?
            }
            AnalysisKind::AverageCurves => self.curves.average_tool(),
            AnalysisKind::Intersect => self.curves.intersect_tool(),
            AnalysisKind::PolygonArea => Tool::PolygonArea,
            AnalysisKind::XyzArea => Tool::XyzArea,
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
            AnalysisKind::Normalize if self.normalize >= NORMALIZE_MATCH => {
                let column = self
                    .reference_choices
                    .get(self.reference)
                    .map(|(id, _)| *id)
                    .ok_or_else(|| "there is no other column with numbers to use as the reference".to_string())?;
                Tool::NormalizeRef(if self.normalize == NORMALIZE_MATCH {
                    NormalizeRef::Column { column, stat: self.normalize_stat }
                } else {
                    NormalizeRef::Cell { column, row: self.normalize_row.saturating_sub(1) }
                })
            }
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
            AnalysisKind::PeakChain
                if matches!(self.chain_goal, ChainGoal::CreateBaseline | ChainGoal::SubtractBaseline) =>
            {
                Tool::PeakBaseline(PeakBaselineSpec {
                    baseline: self.chain_baseline_spec(),
                    subtract: self.chain_goal == ChainGoal::SubtractBaseline,
                })
            }
            AnalysisKind::PeakChain => Tool::PeakChain(PeakChainSpec {
                centers: Vec::new(),
                chain: ChainOptions {
                    baseline: self.chain_baseline_spec(),
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
                integrate: self.chain_goal == ChainGoal::Integrate,
                fit: (self.chain_goal == ChainGoal::Fit).then(|| PeakFitSpec {
                    shape: self.chain_fit,
                    baseline: if self.chain_fit_linear { FitBaseline::Linear } else { FitBaseline::Constant },
                    shapes: self.chain_fit_shapes.iter().map(|s| s.unwrap_or(self.chain_fit)).collect(),
                    peak_curves: false,
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
            AnalysisKind::VarTestOne | AnalysisKind::VarTestTwo => {
                let st = &self.stats;
                Tool::VarTest(VarTestSpec {
                    kind: if self.kind == AnalysisKind::VarTestOne {
                        VarTestKind::OneSample { variance: st.var_hyp }
                    } else {
                        VarTestKind::TwoSample { ratio: 1.0 }
                    },
                    tail: st.tail,
                    alpha: st.alpha,
                    intervals: st.intervals,
                    levels: parse_levels(&st.levels_text)?,
                })
            }
            AnalysisKind::PropTestOne | AnalysisKind::PropTestTwo => {
                let st = &self.stats;
                Tool::PropTest(PropTestSpec {
                    kind: if self.kind == AnalysisKind::PropTestOne {
                        PropTestKind::OneSample {
                            successes: st.successes[0],
                            size: st.sample_size[0],
                            proportion: st.prop_hyp,
                        }
                    } else {
                        PropTestKind::TwoSample {
                            first: (st.successes[0], st.sample_size[0]),
                            second: (st.successes[1], st.sample_size[1]),
                            difference: st.prop_diff,
                        }
                    },
                    tail: st.tail,
                    alpha: st.alpha,
                    intervals: st.intervals,
                    levels: parse_levels(&st.levels_text)?,
                    normal: st.normal_method,
                    exact: st.binomial_method,
                    pooled: st.pooled_p,
                    power: PropPowerSpec {
                        actual: st.power_actual,
                        hypothetical: st.power_hypothetical,
                        alpha: st.power_alpha,
                        sizes: if st.power_hypothetical { parse_sizes(&st.power_sizes_text)? } else { Vec::new() },
                    },
                })
            }
            AnalysisKind::PoissonOne | AnalysisKind::PoissonTwo => {
                let st = &self.stats;
                Tool::PoissonTest(PoissonTestSpec {
                    kind: if self.kind == AnalysisKind::PoissonOne {
                        PoissonTestKind::OneSample {
                            total: st.occurrences[0],
                            size: st.poisson_size[0],
                            exposure: st.exposure[0],
                            rate: st.rate_hyp,
                        }
                    } else {
                        PoissonTestKind::TwoSample {
                            first: (st.occurrences[0], st.poisson_size[0]),
                            second: (st.occurrences[1], st.poisson_size[1]),
                            exposures: (st.exposure[0], st.exposure[1]),
                            difference: st.rate_diff,
                        }
                    },
                    tail: st.tail,
                    alpha: st.alpha,
                    intervals: st.intervals,
                    levels: parse_levels(&st.levels_text)?,
                    normal: st.poisson_normal,
                    exact: st.poisson_exact,
                    pooled: st.poisson_pooled,
                })
            }
            AnalysisKind::Anova => Tool::Anova(AnovaSpec {
                alpha: self.stats.alpha,
                comparison: self.stats.comparison,
                control: self.stats.control,
            }),
            AnalysisKind::Crosstab => Tool::Crosstab(CrosstabSpec { alpha: self.stats.alpha, ..self.stats.cross }),
            AnalysisKind::GraphSummary => {
                let groups = self
                    .stats
                    .summary_groups
                    .iter()
                    .filter_map(|&i| self.sheet_columns.get(i).map(|(id, _)| *id))
                    .filter(|id| !self.columns.contains(id))
                    .collect();
                Tool::GraphSummary(GraphSummarySpec { groups, ..self.stats.summary })
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
            AnalysisKind::Binning2D => Tool::Binning2D(self.bin2d.options()),
            AnalysisKind::DiscreteFreq => Tool::DiscreteFrequency(self.discfreq),
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
                    comparison: self.stats.comparison,
                    alpha: self.stats.alpha,
                })
            }
            AnalysisKind::Anova3 => Tool::ThreeWayAnova(self.anova3.spec()?),
            AnalysisKind::RmAnova1 | AnalysisKind::RmAnova2 => Tool::RmAnova(self.rm.spec()?),
            AnalysisKind::PartialCorr => Tool::PartialCorrelation(self.partcorr.spec(&self.sheet_columns)?),
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
        } else if self.kind == AnalysisKind::Anova3 {
            self.anova3.picked(&self.sheet_columns)?
        } else if self.kind == AnalysisKind::XyzArea {
            self.xyz.picked(&self.sheet_columns)?
        } else if matches!(self.kind, AnalysisKind::RmAnova1 | AnalysisKind::RmAnova2) {
            self.rm.picked(&self.sheet_columns)?
        } else if self.kind == AnalysisKind::Crosstab {
            let pick = |i: usize| self.sheet_columns.get(i).map(|(id, _)| *id);
            match (pick(self.stats.pair_columns[0]), pick(self.stats.pair_columns[1])) {
                (Some(a), Some(b)) if a != b => vec![a, b],
                (Some(_), Some(_)) => return Err("choose two different columns".to_string()),
                _ => return Err("the sheet needs two columns: the row variable and the column variable".to_string()),
            }
        } else {
            self.columns.clone()
        };
        Ok(Request { columns, rows, tool })
    }

    /// The reference list and the options that belong to it.
    fn reference_options(&mut self, ui: &mut egui::Ui) {
        let column_math = self.kind == AnalysisKind::ColumnMath;
        let label = match self.kind {
            AnalysisKind::ColumnMath => "Column",
            AnalysisKind::Convolution | AnalysisKind::Deconvolution => "Response",
            AnalysisKind::SignalCorrelation | AnalysisKind::Coherence => "Second signal",
            AnalysisKind::WaveletReconstruct => "Detail Coefficients",
            AnalysisKind::ContinuousWavelet => "Scale",
            AnalysisKind::Ifft => "Imaginary",
            _ => "Reference",
        };
        let text = match self.reference_choices.get(self.reference) {
            Some((_, label)) => label.as_str(),
            None => "(no other column with numbers)",
        };
        ui.horizontal(|ui| {
            ui.label(label);
            let mut chosen = self.reference;
            egui::ComboBox::from_id_salt("reference_column").selected_text(text).width(300.0).show_ui(ui, |ui| {
                for (i, (_, label)) in self.reference_choices.iter().enumerate() {
                    ui.selectable_value(&mut chosen, i, label);
                }
            });
            self.reference = chosen;
        });
        if column_math {
            ui.weak("Row by row, without X values; rows past the end of the column count as 0.");
            return;
        }
        if matches!(
            self.kind,
            AnalysisKind::Normalize
                | AnalysisKind::Convolution
                | AnalysisKind::Deconvolution
                | AnalysisKind::SignalCorrelation
                | AnalysisKind::Ifft
                | AnalysisKind::Coherence
                | AnalysisKind::WaveletReconstruct
                | AnalysisKind::ContinuousWavelet
        ) {
            return;
        }
        ui.checkbox(&mut self.common_only, "Common X range only").on_hover_text(
            "Only where data and reference both have X values; the result goes to a new sheet. Otherwise the \
             reference is continued as a straight line outside its range.",
        );
    }

    fn options(&mut self, ui: &mut egui::Ui, filter_fs: Option<f64>) {
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
            | AnalysisKind::ReduceDup
            | AnalysisKind::ReduceX
            | AnalysisKind::TranslateX
            | AnalysisKind::TranslateY
            | AnalysisKind::MedianFilter => self.manip.options(ui, self.kind),
            AnalysisKind::SubtractRef => {
                self.reference_options(ui);
                ui.weak("The reference is interpolated onto the X values of the data.");
            }
            AnalysisKind::Fft => fft_options(ui, &mut self.fft),
            AnalysisKind::Stft => stft_options(ui, &mut self.stft),
            AnalysisKind::FftFilter => fft_filter_options(ui, &mut self.fft_filter),
            AnalysisKind::IirFilter => super::filter_options::iir_options(ui, &mut self.filters.iir, filter_fs),
            AnalysisKind::FirFilter => super::filter_options::fir_options(ui, &mut self.filters.fir, filter_fs),
            AnalysisKind::Envelope => envelope_options(ui, &mut self.envelope),
            AnalysisKind::Coherence => {
                self.reference_options(ui);
                coherence_options(ui, &mut self.coherence);
            }
            AnalysisKind::Decimate => decimate_options(ui, &mut self.decimate),
            AnalysisKind::WaveletDecompose => super::wavelet::decompose_options(ui, &mut self.wavelet.decompose),
            AnalysisKind::WaveletReconstruct => {
                self.reference_options(ui);
                super::wavelet::reconstruct_options(ui, &mut self.wavelet.reconstruct);
            }
            AnalysisKind::WaveletDenoise => super::wavelet::denoise_options(ui, &mut self.wavelet.denoise),
            AnalysisKind::WaveletSmooth => super::wavelet::smooth_options(ui, &mut self.wavelet.smooth),
            AnalysisKind::ContinuousWavelet => {
                self.reference_options(ui);
                super::wavelet::cwt_options(ui, &mut self.wavelet.cwt);
            }
            AnalysisKind::TraceInterp => trace_interp_options(ui, &mut self.trace_interp),
            AnalysisKind::Ifft => {
                ui.add_enabled_ui(!self.reference_choices.is_empty(), |ui| {
                    ui.checkbox(&mut self.use_reference, "The input is the real part; imaginary part from");
                });
                ui.add_enabled_ui(self.use_reference, |ui| self.reference_options(ui));
                let spec = &mut self.ifft;
                ui.horizontal(|ui| {
                    ui.label("Sampling interval");
                    ui.checkbox(&mut spec.auto_interval, "Auto")
                        .on_hover_text("The X step of the input column (its frequency step); 1 without an X column.");
                    ui.add_enabled(
                        !spec.auto_interval,
                        DragValue::new(&mut spec.options.interval).speed(0.01).range(1e-300..=f64::MAX),
                    );
                });
                ui.checkbox(&mut spec.options.shift, "Shift (the input has the zero frequency in the middle)");
                ui.checkbox(&mut spec.options.unwrap, "Unwrap Phase");
                ui.horizontal(|ui| {
                    ui.label("Factor");
                    ui.radio_value(&mut spec.options.science, false, "-1 (Electrical Engineering)");
                    ui.radio_value(&mut spec.options.science, true, "+1 (Science)");
                });
            }
            AnalysisKind::Hilbert => {
                ui.weak(
                    "Writes the Hilbert transform of each input column. Origin also writes the analytic signal as a \
                     complex column, whose real part is the input and whose imaginary part is this transform.",
                );
            }
            AnalysisKind::LevelCrossing => {
                let spec = &mut self.level_crossing;
                ui.horizontal(|ui| {
                    ui.label("Level");
                    ui.add(DragValue::new(&mut spec.level).speed(0.01));
                });
                ui.checkbox(&mut spec.details, "Also output directions, X indices and Y crossing values");
                ui.horizontal(|ui| {
                    ui.label("Row of a crossing between rows");
                    ui.radio_value(&mut spec.index, CrossingIndex::Nearest, "Any");
                    ui.radio_value(&mut spec.index, CrossingIndex::Left, "Left");
                    ui.radio_value(&mut spec.index, CrossingIndex::Right, "Right");
                });
                ui.horizontal(|ui| {
                    ui.label("Minimum rows between a pair");
                    ui.add(DragValue::new(&mut spec.min_gap).range(1..=1_000_000));
                })
                .response
                .on_hover_text("Both crossings of a pair closer than this are dropped (Origin's pts).");
            }
            AnalysisKind::Convolution | AnalysisKind::Deconvolution | AnalysisKind::SignalCorrelation => {
                self.reference_options(ui);
                if self.kind == AnalysisKind::Deconvolution {
                    ui.weak(
                        "Divides the spectrum of each input by the response's (FFT over the input's points), so it \
                         undoes a circular convolution, and a linear one followed by zeros.",
                    );
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Type");
                        ui.radio_value(&mut self.signal_circular, false, "Linear");
                        ui.radio_value(&mut self.signal_circular, true, "Circular");
                    });
                }
                ui.checkbox(&mut self.signal_normalize, "Normalize");
                ui.horizontal(|ui| {
                    ui.label("Sampling interval");
                    if self.kind != AnalysisKind::SignalCorrelation {
                        let mut auto = self.signal_interval.is_none();
                        if ui.checkbox(&mut auto, "Auto").changed() {
                            self.signal_interval = if auto { None } else { Some(1.0) };
                        }
                    }
                    if let Some(d) = &mut self.signal_interval {
                        ui.add(DragValue::new(d).speed(0.01).range(1e-300..=f64::MAX));
                    } else {
                        ui.weak("the X step of the input");
                    }
                });
            }
            AnalysisKind::CurveMath | AnalysisKind::ColumnMath => {
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
                    let shown = NORMALIZE_CHOICES
                        .get(self.normalize)
                        .map(|c| c.0)
                        .unwrap_or_else(|| NORMALIZE_REF_LABELS[self.normalize - NORMALIZE_MATCH]);
                    egui::ComboBox::from_id_salt("normalize_method").selected_text(shown).width(260.0).show_ui(
                        ui,
                        |ui| {
                            for (i, (label, _)) in NORMALIZE_CHOICES.iter().enumerate() {
                                ui.selectable_value(&mut self.normalize, i, *label);
                            }
                            for (k, label) in NORMALIZE_REF_LABELS.iter().enumerate() {
                                ui.selectable_value(&mut self.normalize, NORMALIZE_MATCH + k, *label);
                            }
                        },
                    );
                });
                // T260: Origin's ref and cell methods.
                if self.normalize >= NORMALIZE_MATCH {
                    self.reference_options(ui);
                    if self.normalize == NORMALIZE_MATCH {
                        ui.horizontal(|ui| {
                            ui.label("Statistic");
                            egui::ComboBox::from_id_salt("normalize_stat")
                                .selected_text(self.normalize_stat.label())
                                .show_ui(ui, |ui| {
                                    for s in RefStat::ALL {
                                        ui.selectable_value(&mut self.normalize_stat, s, s.label());
                                    }
                                });
                        });
                        ui.weak("Each column is scaled so that this statistic equals the reference column's.");
                    } else {
                        ui.horizontal(|ui| {
                            ui.label("Row");
                            ui.add(DragValue::new(&mut self.normalize_row).range(1..=usize::MAX).speed(1.0));
                        });
                    }
                } else if matches!(NORMALIZE_CHOICES[self.normalize].1, NormalizeMethod::Divide(_)) {
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
                        "The factor is the allowed sum of squared residuals, as in Origin's sf. 0 chooses it from the data, as Origin does.",
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
                    ui.label("Outside the data");
                    use sheaf_core::analysis::Extrapolation as E;
                    ui.radio_value(&mut self.interp.extrapolation, E::Extrapolate, "Extrapolate");
                    ui.radio_value(&mut self.interp.extrapolation, E::Missing, "Missing value");
                    ui.radio_value(&mut self.interp.extrapolation, E::Repeat, "Repeat the end value");
                });
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
            AnalysisKind::PeakChain => self.peak_wizard(ui),
            AnalysisKind::Peaks => {
                self.peak_search_controls(ui, false);
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
            AnalysisKind::VarTestOne | AnalysisKind::VarTestTwo => {
                let one = self.kind == AnalysisKind::VarTestOne;
                let st = &mut self.stats;
                // Origin's two-sample dialog has no box for the ratio: its null hypothesis is fixed at 1.
                if one {
                    ui.horizontal(|ui| {
                        ui.label("Test Variance");
                        ui.add(DragValue::new(&mut st.var_hyp).speed(0.01).range(1e-12..=f64::MAX));
                    });
                }
                let (what, shown) =
                    if one { ("Variance", format!("{}", st.var_hyp)) } else { ("Variance 1/Variance 2", "1".into()) };
                ui.horizontal(|ui| {
                    ui.label("Null Hypothesis");
                    read_only_box(ui, &format!("{what} = {shown}"), 140.0);
                });
                alternative_radios(ui, &mut st.tail, what, &shown);
                if !one {
                    ui.weak("Needs exactly two columns; the first over the second.");
                }
                significance_and_intervals(ui, st);
            }
            AnalysisKind::PropTestOne | AnalysisKind::PropTestTwo => {
                let one = self.kind == AnalysisKind::PropTestOne;
                let st = &mut self.stats;
                let samples = if one { 1 } else { 2 };
                ui.label("Input Data: Summarized");
                for i in 0..samples {
                    let tag = if one { String::new() } else { format!(" of Sample{}", i + 1) };
                    ui.horizontal(|ui| {
                        ui.label(format!("Number of Success{tag}"));
                        ui.add(DragValue::new(&mut st.successes[i]).range(0..=u64::from(u32::MAX)));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("Sample Size{tag}"));
                        ui.add(DragValue::new(&mut st.sample_size[i]).range(1..=u64::from(u32::MAX)));
                    });
                }
                ui.label("Proportion Test");
                let (what, value) = if one {
                    ui.horizontal(|ui| {
                        ui.label("Test Proportion");
                        ui.add(DragValue::new(&mut st.prop_hyp).speed(0.01).range(0.0001..=0.9999));
                    });
                    ("Proportion", st.prop_hyp)
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Test Difference");
                        ui.add(DragValue::new(&mut st.prop_diff).speed(0.01).range(-0.9999..=0.9999));
                    });
                    ("Proportion1 - Proportion2", st.prop_diff)
                };
                let shown = format!("{value}");
                ui.horizontal(|ui| {
                    ui.label("Null Hypothesis");
                    read_only_box(ui, &format!("{what} = {shown}"), 180.0);
                });
                alternative_radios(ui, &mut st.tail, what, &shown);
                significance_and_intervals(ui, st);
                ui.label("Test Method");
                ui.checkbox(&mut st.normal_method, "Normal Approximation");
                if !one {
                    ui.checkbox(&mut st.pooled_p, "Use pooled p for test");
                }
                ui.checkbox(&mut st.binomial_method, if one { "Binomial Test" } else { "Fisher's Exact Test" });
                ui.label("Power Analysis");
                ui.checkbox(&mut st.power_actual, "Actual Power");
                ui.add_enabled_ui(st.power_actual || st.power_hypothetical, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Significance Level");
                        ui.add(DragValue::new(&mut st.power_alpha).speed(0.001).range(0.0001..=0.4999));
                    });
                });
                ui.checkbox(&mut st.power_hypothetical, "Hypothetical Power");
                ui.add_enabled_ui(st.power_hypothetical, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Hypothetical Sample Size(s)");
                        ui.add(egui::TextEdit::singleline(&mut st.power_sizes_text).desired_width(120.0));
                    });
                });
            }
            AnalysisKind::PoissonOne | AnalysisKind::PoissonTwo => {
                let one = self.kind == AnalysisKind::PoissonOne;
                let st = &mut self.stats;
                ui.label("Input Data: Summarized");
                for i in 0..if one { 1 } else { 2 } {
                    let tag = if one { String::new() } else { format!(" of Sample{}", i + 1) };
                    ui.horizontal(|ui| {
                        ui.label(format!("Sample Size{tag}"));
                        ui.add(DragValue::new(&mut st.poisson_size[i]).range(1..=u64::from(u32::MAX)));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("Total Occurrence{tag}"));
                        ui.add(DragValue::new(&mut st.occurrences[i]).range(0..=u64::from(u32::MAX)));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("Exposure{tag}"));
                        ui.add(DragValue::new(&mut st.exposure[i]).speed(0.1).range(1e-9..=1e12));
                    });
                }
                ui.label("Poisson Rate Test");
                let (what, value) = if one {
                    ui.horizontal(|ui| {
                        ui.label("Test Rate");
                        ui.add(DragValue::new(&mut st.rate_hyp).speed(0.01).range(1e-9..=1e12));
                    });
                    ("Rate", st.rate_hyp)
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Test Difference");
                        ui.add(DragValue::new(&mut st.rate_diff).speed(0.01));
                    });
                    ("Rate1 - Rate2", st.rate_diff)
                };
                let shown = format!("{value}");
                ui.horizontal(|ui| {
                    ui.label("Null Hypothesis");
                    read_only_box(ui, &format!("{what} = {shown}"), 180.0);
                });
                alternative_radios(ui, &mut st.tail, what, &shown);
                significance_and_intervals(ui, st);
                ui.label("Test Method");
                ui.checkbox(&mut st.poisson_exact, "Exact");
                ui.checkbox(&mut st.poisson_normal, "Normal Approximation");
                if !one {
                    ui.checkbox(&mut st.poisson_pooled, "Use pooled p for test");
                }
                ui.weak("Origin 2026b's test; its formulas are not published, so the textbook ones are used.");
            }
            AnalysisKind::Anova => {
                let st = &mut self.stats;
                ui.label("Every chosen column is a group.");
                ui.horizontal(|ui| {
                    ui.label("Means comparison");
                    comparison_combo(ui, "anova_comparison", &mut st.comparison, &COMPARISONS_ONE_WAY);
                });
                if st.comparison == Some(MeansComparison::Dunnett) {
                    let names: Vec<&str> = self.column_names.split(", ").collect();
                    st.control = st.control.min(names.len().saturating_sub(1));
                    ui.horizontal(|ui| {
                        ui.label("Control group");
                        egui::ComboBox::from_id_salt("anova_control")
                            .selected_text(names.get(st.control).copied().unwrap_or(""))
                            .show_ui(ui, |ui| {
                                for (i, name) in names.iter().enumerate() {
                                    ui.selectable_value(&mut st.control, i, *name);
                                }
                            });
                    });
                    ui.weak("Every other group is compared with the control; p-values and limits are adjusted for all of them.");
                } else if st.comparison.is_some_and(MeansComparison::is_holm) {
                    ui.weak("Step-down adjustment of all pairs; it gives adjusted p-values but no confidence limits.");
                }
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
                if st.comparison.is_some_and(MeansComparison::is_holm) {
                    st.comparison = Some(MeansComparison::Tukey);
                }
                ui.horizontal(|ui| {
                    ui.label("Means comparison");
                    comparison_combo(ui, "anova2_comparison", &mut st.comparison, &COMPARISONS_TWO_WAY);
                });
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut st.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            AnalysisKind::Anova3 => self.anova3.ui(ui, &self.sheet_columns),
            AnalysisKind::XyzArea => self.xyz.ui(ui, &self.sheet_columns),
            AnalysisKind::RmAnova1 | AnalysisKind::RmAnova2 => self.rm.ui(ui, &self.sheet_columns),
            AnalysisKind::PartialCorr => self.partcorr.ui(ui, &self.sheet_columns, &self.columns),
            AnalysisKind::GraphSummary => {
                let st = &mut self.stats;
                ui.label(
                    "Every chosen column is a variable; each group of the group columns is summarized on its own.",
                );
                ui.horizontal(|ui| {
                    ui.label("Groups");
                    let chosen: Vec<&str> = st
                        .summary_groups
                        .iter()
                        .filter_map(|&i| self.sheet_columns.get(i))
                        .map(|(_, n)| n.split("  ").next().unwrap_or(""))
                        .collect();
                    let text = if chosen.is_empty() { "(none)".to_string() } else { chosen.join(", ") };
                    egui::ComboBox::from_id_salt("summary_groups").selected_text(text).width(220.0).show_ui(ui, |ui| {
                        for (j, (id, name)) in self.sheet_columns.iter().enumerate() {
                            if self.columns.contains(id) {
                                continue;
                            }
                            let mut on = st.summary_groups.contains(&j);
                            if ui.checkbox(&mut on, name).changed() {
                                if on {
                                    st.summary_groups.push(j);
                                } else {
                                    st.summary_groups.retain(|k| *k != j);
                                }
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Confidence Level (%)");
                    ui.add(DragValue::new(&mut st.summary.level).speed(0.1).range(0.01..=99.99));
                });
                ui.horizontal(|ui| {
                    ui.label("Graph Arrangement");
                    egui::ComboBox::from_id_salt("summary_arrangement")
                        .selected_text(st.summary.arrangement.label())
                        .show_ui(ui, |ui| {
                            for a in GraphArrangement::ALL {
                                let enabled = a.implemented();
                                let r = ui.add_enabled(
                                    enabled,
                                    egui::Button::selectable(st.summary.arrangement == a, a.label()),
                                );
                                if enabled && r.clicked() {
                                    st.summary.arrangement = a;
                                }
                                if !enabled {
                                    r.on_disabled_hover_text("Not available yet; Sheaf makes Separate Graphs only.");
                                }
                            }
                        });
                });
                ui.weak(
                    "For each variable and group: Description Statistics, confidence intervals of the mean, median and \
                     standard deviation, and the Anderson-Darling normality test, with a graph of a histogram and \
                     normal curve, an interval plot and a normal probability plot.",
                );
            }
            AnalysisKind::Crosstab => {
                let st = &mut self.stats;
                for (i, label) in ["Row variable", "Column variable"].into_iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        let current = self.sheet_columns.get(st.pair_columns[i]).map_or("", |c| c.1.as_str());
                        egui::ComboBox::from_id_salt(("crosstab_column", i))
                            .selected_text(current)
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for (j, (_, text)) in self.sheet_columns.iter().enumerate() {
                                    ui.selectable_value(&mut st.pair_columns[i], j, text);
                                }
                            });
                    });
                }
                ui.weak(
                    "Each column holds the levels (text or numbers) of one variable; one row of the sheet is one case.",
                );
                ui.checkbox(&mut st.cross.row_percent, "Row percent");
                ui.checkbox(&mut st.cross.column_percent, "Column percent");
                ui.checkbox(&mut st.cross.total_percent, "Total percent");
                ui.checkbox(&mut st.cross.expected, "Expected counts");
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
            AnalysisKind::DistFit | AnalysisKind::ProbPlot | AnalysisKind::QQPlot => self.dist.options(ui, self.kind),
            AnalysisKind::Pca | AnalysisKind::Roc | AnalysisKind::KaplanMeier | AnalysisKind::Kmeans => {
                self.multi.options(ui, self.kind)
            }
            AnalysisKind::DiscreteFreq => {
                let d = &mut self.discfreq;
                egui::CollapsingHeader::new("Quantities to Compute").default_open(true).show(ui, |ui| {
                    ui.checkbox(&mut d.count, "Count");
                    ui.checkbox(&mut d.relative, "Relative Frequency");
                    ui.checkbox(&mut d.cumulative, "Cumulative Frequency");
                    ui.add_enabled_ui(d.relative || d.cumulative, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Frequency in");
                            ui.radio_value(&mut d.percent, false, "Fraction");
                            ui.radio_value(&mut d.percent, true, "Percent");
                        });
                    });
                });
                ui.checkbox(&mut d.case_sensitive, "Case Sensitive");
                ui.horizontal(|ui| {
                    ui.label("Sort Data by");
                    egui::ComboBox::from_id_salt("discfreq-sort").selected_text(d.sort.label()).show_ui(ui, |ui| {
                        for s in DiscSort::ALL {
                            ui.selectable_value(&mut d.sort, s, s.label());
                        }
                    });
                });
            }
            AnalysisKind::Binning2D => {
                let mut names = self.column_names.split(", ");
                let (x, y) = match (names.next(), names.next()) {
                    (Some(a), Some(b)) if self.columns.len() >= 2 => (format!("{a}(X)"), format!("{b}(Y)")),
                    (Some(a), _) => ("X".to_string(), format!("{a}(Y)")),
                    _ => ("X".to_string(), "Y".to_string()),
                };
                self.bin2d.ui(ui, &x, &y);
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

    /// Sets the Level Crossing level as the dialog's field would (for tests).
    #[cfg(test)]
    pub fn set_crossing_level(&mut self, level: f64) {
        self.level_crossing.level = level;
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
            AnalysisKind::ColumnMath => "vmathtool",
            AnalysisKind::Convolution => "conv",
            AnalysisKind::Deconvolution => "deconv",
            AnalysisKind::Stft => "stft",
            AnalysisKind::SignalCorrelation => "corr1",
            AnalysisKind::LevelCrossing => "levelcrossing",
            AnalysisKind::Hilbert => "hilbert",
            AnalysisKind::Fft => "fft1",
            AnalysisKind::FftFilter => "fft_filters",
            AnalysisKind::IirFilter => "dfilter",
            AnalysisKind::FirFilter => "",
            AnalysisKind::Envelope => "envelope",
            AnalysisKind::Coherence => "cohere",
            AnalysisKind::Decimate => "decimate",
            AnalysisKind::WaveletDecompose => "dwt",
            AnalysisKind::WaveletReconstruct => "idwt",
            AnalysisKind::WaveletDenoise => "wtdenoise",
            AnalysisKind::WaveletSmooth => "wtsmooth",
            AnalysisKind::ContinuousWavelet => "cwt",
            AnalysisKind::TraceInterp => "interp1trace",
            AnalysisKind::Ifft => "ifft1",
            AnalysisKind::Interpolate => "interp1",
            AnalysisKind::Peaks => "pkFind",
            AnalysisKind::PeakChain => "",
            AnalysisKind::Statistics => "DescStats",
            AnalysisKind::TTest => "ttest",
            AnalysisKind::VarTestOne => "OneSampleTestVar",
            AnalysisKind::VarTestTwo => "TwoSampleTestVar",
            AnalysisKind::PropTestOne => "OneSampleProportionTest",
            AnalysisKind::PropTestTwo => "TwoSampleProportionTest",
            AnalysisKind::PoissonOne => "OneSamplePoissonRateTest",
            AnalysisKind::PoissonTwo => "TwoSamplePoissonRateTest",
            AnalysisKind::Anova => "anova1",
            AnalysisKind::Histogram => "freqcounts",
            AnalysisKind::Binning2D => "twoDBinning",
            AnalysisKind::DiscreteFreq => "discfreqs",
            AnalysisKind::Normality => "normtest",
            AnalysisKind::Anova2 => "anova2",
            AnalysisKind::Anova3 => "ANOVAThreeWay",
            AnalysisKind::RmAnova1 => "ANOVAOneWayRM",
            AnalysisKind::RmAnova2 => "ANOVATwoWayRM",
            AnalysisKind::PartialCorr => "pcorrcoef",
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
            AnalysisKind::KaplanMeier => "kaplanmeier",
            AnalysisKind::Kmeans => "kmeans",
            AnalysisKind::AverageCurves => "avecurves",
            AnalysisKind::Intersect => "intersect",
            AnalysisKind::PolygonArea => "polyarea",
            AnalysisKind::XyzArea => "xyzarea",
            AnalysisKind::ReduceDup => "reducedup",
            // Origin's X-Function names for these were not looked up; the title shows the tool alone.
            AnalysisKind::Crosstab
            | AnalysisKind::GraphSummary
            | AnalysisKind::Pca
            | AnalysisKind::Roc
            | AnalysisKind::SubtractLine
            | AnalysisKind::ReduceX
            | AnalysisKind::TranslateX
            | AnalysisKind::TranslateY
            | AnalysisKind::MedianFilter
            | AnalysisKind::DistFit
            | AnalysisKind::ProbPlot
            | AnalysisKind::QQPlot => "",
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
            AnalysisKind::ColumnMath => {
                "Add, subtract, multiply, divide or raise to a power with a constant or a column, row by row"
            }
            AnalysisKind::Convolution => "Convolve the input signals with a response",
            AnalysisKind::Deconvolution => "Deconvolve the input signals by a response",
            AnalysisKind::Stft => "The short-time frequency spectrum of the input signals",
            AnalysisKind::SignalCorrelation => "Correlate the input signals with a second signal",
            AnalysisKind::LevelCrossing => "Find where the input curves cross a level",
            AnalysisKind::Hilbert => "The Hilbert transform of the input signals",
            AnalysisKind::Fft => "The frequency spectrum of the input signals",
            AnalysisKind::FftFilter => "Remove frequencies from the input signals",
            AnalysisKind::IirFilter => "Design an IIR filter and apply it to the input signals",
            AnalysisKind::FirFilter => "Design an FIR filter and apply it to the input signals",
            AnalysisKind::Envelope => "The upper and lower envelope of the input curves",
            AnalysisKind::Coherence => "The magnitude-squared coherence of the input signals with a second signal",
            AnalysisKind::Decimate => "Keep every n-th point of the input signals",
            AnalysisKind::WaveletDecompose => "One level of the discrete wavelet transform of the input signals",
            AnalysisKind::WaveletReconstruct => "Rebuild a signal from approximation and detail coefficients",
            AnalysisKind::WaveletDenoise => "Remove noise from the input signals by thresholding wavelet coefficients",
            AnalysisKind::WaveletSmooth => "Smooth the input signals by dropping wavelet detail coefficients",
            AnalysisKind::ContinuousWavelet => {
                "The continuous wavelet transform of the input signals at the given scales"
            }
            AnalysisKind::TraceInterp => {
                "Resample curves whose X is not monotonic at evenly spaced points along the path"
            }
            AnalysisKind::Ifft => "The signal of a spectrum given as real and imaginary parts",
            AnalysisKind::Interpolate => "Interpolate the input curves at evenly spaced X values",
            AnalysisKind::Peaks => "Find the peaks of the input curves",
            AnalysisKind::PeakChain => "Baseline, peak search, integration and fit of the peaks",
            AnalysisKind::Statistics => "Descriptive statistics of the input columns",
            AnalysisKind::TTest => "Test the means of one or two samples",
            AnalysisKind::VarTestOne => "Test the variance of a sample against a value (chi-square test)",
            AnalysisKind::VarTestTwo => "Test the ratio of the variances of two samples (F test)",
            AnalysisKind::PropTestOne => "Test a proportion of successes against a value, from counts you type in",
            AnalysisKind::PropTestTwo => "Compare the proportions of two samples, from counts you type in",
            AnalysisKind::PoissonOne => "Test the rate of a Poisson count against a value, from counts you type in",
            AnalysisKind::PoissonTwo => "Compare the rates of two Poisson counts, from counts you type in",
            AnalysisKind::Anova => "One-way analysis of variance; every column is a group",
            AnalysisKind::Histogram => "Count the values of the input columns in bins",
            AnalysisKind::Binning2D => "Count the points of two variables in a grid of X and Y bins",
            AnalysisKind::DiscreteFreq => "Count how often each distinct value, text or number, occurs",
            AnalysisKind::Normality => "Test whether the input columns come from a normal distribution",
            AnalysisKind::Anova2 => "Two-way analysis of variance",
            AnalysisKind::Anova3 => "Three-way analysis of variance with type III sums of squares",
            AnalysisKind::RmAnova1 => "One-way analysis of variance of repeated measurements of the same subjects",
            AnalysisKind::RmAnova2 => {
                "Two-way analysis of variance of repeated measurements, both factors within subjects"
            }
            AnalysisKind::PartialCorr => {
                "Partial correlation coefficients of the input columns, controlling for other columns"
            }
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
            AnalysisKind::DistFit => "Fit distributions to the input columns by maximum likelihood and test the fit",
            AnalysisKind::ProbPlot => "Plot the data against the plotting positions on a probability axis",
            AnalysisKind::QQPlot => "Plot the quantiles of a distribution against the ordered data",
            AnalysisKind::KaplanMeier => {
                "Estimate the survival function of censored times, with quartiles, the mean and tests of groups"
            }
            AnalysisKind::Kmeans => "Group the rows into a chosen number of clusters of similar values",
            AnalysisKind::Roc => "Judge how well a test variable separates the two states of a state column",
            AnalysisKind::Pca => "Describe the variation of several columns by uncorrelated components",
            AnalysisKind::Crosstab => {
                "Count the cases of every combination of two categorical variables and test their independence"
            }
            AnalysisKind::GraphSummary => {
                "Descriptive statistics, confidence intervals and a normality test with a histogram, an interval plot and a probability plot"
            }
            AnalysisKind::AverageCurves => "Average several curves on a common set of X values",
            AnalysisKind::Intersect => "Find the points where two curves meet",
            AnalysisKind::PolygonArea => "Area, centroid and perimeter of each input curve taken as a closed polygon",
            AnalysisKind::XyzArea => "Area of the surface through the points of three columns X, Y and Z",
            AnalysisKind::Correlation => "Correlation coefficients of the input columns",
            AnalysisKind::SubtractLine => "Subtract the straight line through two points from the input curves",
            AnalysisKind::ReduceDup => "Replace the points that share an X value by one point",
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
            | AnalysisKind::CurveMath
            | AnalysisKind::ColumnMath
            | AnalysisKind::Hilbert
            | AnalysisKind::FftFilter
            | AnalysisKind::IirFilter
            | AnalysisKind::FirFilter => "New columns beside the input",
            AnalysisKind::Envelope => "The envelopes of each input, each with its X, in a new sheet",
            AnalysisKind::Convolution => "The convolution of each input, with its X, in a new sheet",
            AnalysisKind::Deconvolution => "The deconvolution of each input, with its X, in a new sheet",
            AnalysisKind::Stft => "A frequency column and one spectrum column per segment in a new sheet",
            AnalysisKind::Decimate => "The kept points of each input, with their X, in a new sheet",
            AnalysisKind::WaveletDecompose => "The approximation and detail coefficients of each input in a new sheet",
            AnalysisKind::WaveletReconstruct => "The rebuilt signal in a new sheet",
            AnalysisKind::ContinuousWavelet => {
                "One column of coefficients per scale (rows are the samples) in a new sheet"
            }
            AnalysisKind::WaveletDenoise | AnalysisKind::WaveletSmooth => {
                "The processed signal of each input, with its X, in a new sheet"
            }
            AnalysisKind::SignalCorrelation => "The correlation of each input against the lag in a new sheet",
            AnalysisKind::LevelCrossing => "The crossing X values in a new column beside each input (CrossingX1, ...)",
            AnalysisKind::Fft => "Eight spectrum columns per input in a new sheet",
            AnalysisKind::Coherence => "The coherence of each input against the frequency in a new sheet",
            AnalysisKind::TraceInterp => "The interpolated X and Y of each input in a new sheet",
            AnalysisKind::Ifft => "Time, real, imaginary, amplitude and phase in a new sheet",
            AnalysisKind::Interpolate => "The interpolated curves in a new sheet",
            AnalysisKind::Peaks => "Peak positions and heights in a new sheet",
            AnalysisKind::PeakChain => "Peaks, areas and fit results in a new sheet; the baseline beside the data",
            AnalysisKind::Statistics => {
                "The chosen quantities (count, mean, SD, SEM, quartiles, variance, skewness, confidence limits, ...)                  in a new sheet"
            }
            AnalysisKind::TTest => "The statistic, p-value and confidence interval in a new sheet",
            AnalysisKind::VarTestOne | AnalysisKind::VarTestTwo => {
                "The statistic, p-value and the confidence intervals asked for, in a new sheet"
            }
            AnalysisKind::PropTestOne | AnalysisKind::PropTestTwo => {
                "The z statistic, the p-values of the ticked methods and the intervals, in a new sheet"
            }
            AnalysisKind::PoissonOne | AnalysisKind::PoissonTwo => {
                "The rates, the exact and normal-approximation p-values and the intervals, in a new sheet"
            }
            AnalysisKind::Anova => "Group statistics, the ANOVA table and the comparisons in a new sheet",
            AnalysisKind::Histogram => "Bin centres, counts, cumulative counts and percentages in a new sheet",
            AnalysisKind::Binning2D => "One row per X bin and one column per Y bin in a new sheet",
            AnalysisKind::DiscreteFreq => "The values and their counts and frequencies in a new sheet",
            AnalysisKind::Normality => {
                "Shapiro-Wilk, Kolmogorov-Smirnov, Lilliefors, Anderson-Darling and D'Agostino-Pearson in a new sheet"
            }
            AnalysisKind::Anova2 => {
                "The tables (statistics, ANOVA, comparisons) in a new sheet; sums of squares are adjusted for the \
                 other terms, as in Origin"
            }
            AnalysisKind::RmAnova1 | AnalysisKind::RmAnova2 => {
                "The tables (within and between subjects, sphericity, descriptive statistics) in a new sheet"
            }
            AnalysisKind::Anova3 => {
                "The tables (statistics, ANOVA, comparisons, Levene, power) in a new sheet; sums of squares are adjusted for the other terms, as in Origin"
            }
            AnalysisKind::PartialCorr => "The partial correlations, p-values and degrees of freedom in a new sheet",
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
            AnalysisKind::DistFit => {
                "Parameters, log-likelihood, AIC, BIC and the Kolmogorov-Smirnov and Anderson-Darling statistics in a new sheet"
            }
            AnalysisKind::ProbPlot | AnalysisKind::QQPlot => {
                "The plotted points and the reference line in a new sheet, and a graph"
            }
            AnalysisKind::KaplanMeier => {
                "The survival table, quartile and mean survival times and tests of groups in a new sheet, and a graph of the curves"
            }
            AnalysisKind::Kmeans => {
                "Cluster centres, sizes and distances, the analysis of variance and the cluster of every row in a new sheet, and a scatter graph"
            }
            AnalysisKind::Roc => {
                "Area under the curve with its standard error, the curve points and the best cut-off in a new sheet, and a graph"
            }
            AnalysisKind::Pca => {
                "Eigenvalues, percent of variance, loadings and scores in a new sheet, and a scree plot"
            }
            AnalysisKind::Crosstab => {
                "Counts, percentages, expected counts and the chi-square and Fisher tests in a new sheet"
            }
            AnalysisKind::GraphSummary => {
                "Description Statistics, Confidence Intervals and Normality Test tables in a new sheet, and a graph for each variable and group"
            }
            AnalysisKind::AverageCurves => "X, the mean, its standard deviation and the count in a new sheet",
            AnalysisKind::Intersect => "X and Y of each intersection in a new sheet",
            AnalysisKind::PolygonArea => {
                "Area, signed area, centroid, perimeter and a self-intersection flag in a new sheet"
            }
            AnalysisKind::XyzArea => "The surface area with the points and triangles used in a new sheet",
            AnalysisKind::Correlation => "The matrices of coefficients and p-values in a new sheet",
            AnalysisKind::SubtractLine | AnalysisKind::TranslateY | AnalysisKind::MedianFilter => {
                "New columns beside the input"
            }
            AnalysisKind::ReduceDup | AnalysisKind::ReduceX | AnalysisKind::TranslateX => {
                "The curves in a new sheet with their own X column"
            }
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
                | AnalysisKind::ColumnMath
                | AnalysisKind::WaveletDenoise
                | AnalysisKind::WaveletSmooth
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
        let signal = matches!(
            self.kind,
            AnalysisKind::Convolution
                | AnalysisKind::Deconvolution
                | AnalysisKind::SignalCorrelation
                | AnalysisKind::Ifft
                | AnalysisKind::Coherence
                | AnalysisKind::WaveletReconstruct
                | AnalysisKind::ContinuousWavelet
        );
        self.reference_choices = reference_choices(project, if signal { &[] } else { &columns });
        self.reference = self.reference.min(self.reference_choices.len().saturating_sub(1));
        if matches!(self.kind, AnalysisKind::CurveMath | AnalysisKind::ColumnMath) {
            self.use_reference = self.use_reference && !self.reference_choices.is_empty();
        }
        if self.kind == AnalysisKind::Anova2 {
            self.stats.two_way_levels.resize(columns.len(), (String::new(), String::new()));
        }
        if self.kind == AnalysisKind::PartialCorr {
            self.partcorr.drop_variables(&self.sheet_columns, &columns);
        }
        // These tools run on the columns of their own boxes, not on the Input range: the picker sets the boxes.
        if matches!(
            self.kind,
            AnalysisKind::Crosstab
                | AnalysisKind::Anova2
                | AnalysisKind::Anova3
                | AnalysisKind::XyzArea
                | AnalysisKind::RmAnova1
                | AnalysisKind::RmAnova2
        ) {
            let chosen: Vec<usize> =
                columns.iter().filter_map(|id| self.sheet_columns.iter().position(|(c, _)| c == id)).collect();
            if self.kind == AnalysisKind::Crosstab && chosen.len() >= 2 {
                self.stats.pair_columns = [chosen[0], chosen[1]];
            }
            if self.kind == AnalysisKind::Anova2 && chosen.len() >= 3 {
                self.stats.two_way_columns = [chosen[0], chosen[1], chosen[2]];
            }
            if self.kind == AnalysisKind::XyzArea && chosen.len() >= 3 {
                self.xyz.columns = [chosen[0], chosen[1], chosen[2]];
            }
            if self.kind == AnalysisKind::Anova3 && chosen.len() >= 4 {
                self.anova3.columns = [chosen[0], chosen[1], chosen[2], chosen[3]];
            }
            let need = if self.kind == AnalysisKind::RmAnova2 { 4 } else { 3 };
            if matches!(self.kind, AnalysisKind::RmAnova1 | AnalysisKind::RmAnova2) && chosen.len() >= need {
                self.rm.columns[..need].copy_from_slice(&chosen[..need]);
            }
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
                | AnalysisKind::ColumnMath
                | AnalysisKind::Hilbert
                | AnalysisKind::FftFilter
                | AnalysisKind::IirFilter
                | AnalysisKind::FirFilter
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

    /// The column after the input column, as the first suggestion for an existing target (the input
    /// column itself for Simple Column Math, as Origin).
    fn default_existing(&self, input: usize) -> usize {
        let at = self.columns.get(input).and_then(|c| self.sheet_columns.iter().position(|(id, _)| id == c));
        if self.kind == AnalysisKind::ColumnMath {
            return at.unwrap_or(0);
        }
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
        let previous = if matches!(
            self.kind,
            AnalysisKind::Histogram | AnalysisKind::ProbPlot | AnalysisKind::QQPlot | AnalysisKind::GraphSummary
        ) {
            None
        } else {
            self.applied.clone()
        };
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

    /// The sample frequency Auto gives the IIR and FIR filters for the first input: 1 / the mean X step (the
    /// row number is the X without an X column), as the tools take it when they run.
    fn auto_sample_frequency(&self, project: &Project) -> Option<f64> {
        if !matches!(self.kind, AnalysisKind::IirFilter | AnalysisKind::FirFilter) {
            return None;
        }
        let (_, sheet, first) = self.columns.first().and_then(|c| project.locate(*c))?;
        let Some(x) = sheet.column_index(first.id).and_then(|i| sheet.x_for(i)).and_then(|x| x.numeric()) else {
            return Some(1.0);
        };
        let a = x.iter().position(|v| v.is_finite())?;
        let b = x.iter().rposition(|v| v.is_finite())?;
        let step = (x[b] - x[a]) / (b.checked_sub(a).filter(|n| *n > 0)? as f64);
        (step.is_finite() && step > 0.0).then(|| 1.0 / step).or(Some(1.0))
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
        // The tools that take their variables from factor boxes have no X column to limit.
        if matches!(
            self.kind,
            AnalysisKind::Crosstab
                | AnalysisKind::Anova2
                | AnalysisKind::Anova3
                | AnalysisKind::XyzArea
                | AnalysisKind::RmAnova1
                | AnalysisKind::RmAnova2
                | AnalysisKind::PartialCorr
                | AnalysisKind::GraphSummary
        ) {
            return;
        }
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
        let filter_fs = self.auto_sample_frequency(doc.project());
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
                                if !matches!(
                                    self.kind,
                                    AnalysisKind::PropTestOne
                                        | AnalysisKind::PropTestTwo
                                        | AnalysisKind::PoissonOne
                                        | AnalysisKind::PoissonTwo
                                ) {
                                    tree_node(ui, "analysis_input", "Input", |ui| self.input_node(ui, doc.project()));
                                }
                                let title = self.kind.title();
                                tree_node(ui, "analysis_options", title, |ui| self.options(ui, filter_fs));
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
                    // The probability and Q-Q plots make a new sheet and a new graph on every run, so running them
                    // at each option change would pile those up; they run on OK and Apply only.
                    let allowed = !matches!(
                        self.kind,
                        AnalysisKind::ProbPlot | AnalysisKind::QQPlot | AnalysisKind::GraphSummary
                    );
                    if !allowed {
                        self.auto_apply = false;
                    }
                    ui.add_enabled(allowed, egui::Checkbox::new(&mut self.auto_apply, "Auto Apply")).on_hover_text(
                        if allowed {
                            "Run the tool again at once whenever an option changes"
                        } else {
                            "This tool makes a new sheet and graph each time it runs; use Apply"
                        },
                    );
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

#[cfg(test)]
mod filter_fs_tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation};

    #[test]
    fn auto_sample_frequency_is_one_over_the_x_step() {
        let mut p = Project::new();
        let book = p.add_workbook("Book1");
        let x: Vec<f64> = (0..50).map(|i| f64::from(i) * 0.01).collect();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 50])).unwrap();
        let d = AnalysisDialog::new(&p, AnalysisKind::IirFilter, vec![y]).unwrap();
        assert!((d.auto_sample_frequency(&p).unwrap() - 100.0).abs() < 1e-9);
        let other = AnalysisDialog::new(&p, AnalysisKind::Smooth, vec![y]).unwrap();
        assert_eq!(other.auto_sample_frequency(&p), None);
    }
}
