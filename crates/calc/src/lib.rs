//! Numerical analysis tools of Sheaf: integration, differentiation, smoothing, normalisation,
//! interpolation, baseline subtraction, peak finding and descriptive statistics.
//!
//! Every tool is a plain function over `&[f64]` slices with an options struct and a result struct.
//! Missing values are NaN. The behaviour follows what Origin computes for the same input; the
//! rules are written down in `docs/spec/analysis-tools.md`, and the tests compare against numbers
//! taken from Origin (see `tools/origin-ref/`).
//!
//! Row numbers in options and results are 1-based, as in a worksheet.

pub mod anova;
pub mod anova2;
pub mod baseline;
pub mod bin2d;
pub mod correlation;
pub mod crosstab;
pub mod curvemath;
pub mod curves;
pub mod dierckx;
pub mod differentiate;
pub mod discfreq;
pub mod distfit;
pub mod dunnett;
pub mod envelope;
pub mod fft;
pub mod freq;
pub mod hypothesis;
pub mod integrate;
pub mod interp;
pub mod levelcross;
pub mod manip;
pub mod nonparam;
pub mod normality;
pub mod normalize;
pub mod nptests;
pub mod outlier;
pub mod peakchain;
pub mod peaks;
pub mod power;
pub mod signal;
pub mod smooth;
pub mod stats;
mod util;

pub use anova::{
    Anova, AnovaError, AnovaRow, Comparison, MeansComparison, anova_one_way, compare_means, compare_to_control, ptukey,
    qtukey,
};
pub use anova2::{TwoWayAnova, TwoWayError, TwoWayRow, anova_two_way};
pub use baseline::{BaselineMethod, subtract_baseline, subtract_constant, subtract_line};
pub use bin2d::{AxisBins, AxisLayout, Bin, Bin2d, Bin2dOptions, Bin2dStat, axis_layout, bin2d};
pub use correlation::{Corr, CorrKind, CorrMatrix, correlation, correlation_matrix};
pub use crosstab::{ChiSquareTest, Crosstab, CrosstabError, Fisher};
pub use curvemath::{CurveMathResult, CurveOp, column_math, curve_math_const, curve_math_ref};
pub use curves::{
    AverageMethod, AverageOptions, AverageResult, CommonX, CurveError, Intersection, MeetKind, PolygonArea, XRange,
    average_curves, intersections, polygon_area,
};
pub use differentiate::{DiffOptions, differentiate};
pub use discfreq::{Category, DiscSort, DiscreteFreq, DiscreteRow, discrete_frequency};
pub use distfit::{
    DistFit, Family, FitError, ProbPlot, cdf as dist_cdf, estimate as dist_estimate, fit as dist_fit,
    log_likelihood as dist_log_likelihood, plotting_positions, probplot, standard_quantile,
};
pub use dunnett::{dunnett_cdf, dunnett_critical, dunnett_loadings, dunnett_p};
pub use envelope::{Envelope, EnvelopeError, EnvelopeOptions, EnvelopeType, envelope, extreme_points, natural_spline};
pub use fft::{
    FftFilterOptions, FftFilterType, FftOptions, FftResult, FftWindow, IfftOptions, IfftResult, PowerNorm,
    WindowCorrection, fft_filter, fft_spectrum, fft_window, ifft_spectrum,
};
pub use freq::{BinSize, Freq, FreqOptions, FreqRow, auto_bin_range, auto_bin_size, freq_counts};
pub use hypothesis::{SampleStats, TTest, Tail, ttest_one, ttest_paired, ttest_two};
pub use integrate::{AreaType, IntegrateOptions, IntegrateResult, charge, integrate};
pub use interp::{Extrapolation, InterpMethod, InterpOptions, SplineBoundary, interp1, interp1xy};
pub use levelcross::{Crossing, CrossingIndex, level_crossings};
pub use manip::{
    DUP_TOL, DupMethod, EvenGrid, LinePoint, MedianEdge, REDUCE_AUTO_FACTOR, REDUCE_EVEN_TOLERANCE_PCT, ReduceGroup,
    ReduceMethod, ReduceOptions, Reduced, even_grid, median_filter, reduce_duplicates, reduce_evenly, shift_to_land,
    subtract_line_through, translate, value_at,
};
pub use nonparam::{
    KruskalWallis, MannWhitney, RankGroup, RankTestError, SIGNED_RANK_EXACT_MAX_N, SignedRank, kruskal_wallis,
    mann_whitney, midranks, signed_rank, signed_rank_one, signed_rank_paired,
};
pub use normality::{
    DAgostino, KS_EXACT_MAX_N, TestResult, anderson_darling, dagostino, kolmogorov_cdf, kolmogorov_smirnov,
    ks_distance, lilliefors, shapiro_wilk,
};
pub use normalize::{NormalizeMethod, normalize};
pub use nptests::{
    Friedman, FriedmanError, FriedmanGroup, KS2_EXACT_MAX_PAIRS, KsTwoSample, MedianGroup, MedianTest,
    SIGN_EXACT_MAX_N, SignTest, friedman, ks_two_sample, median_test, sign_test,
};
pub use outlier::{DixonQ, Grubbs, OutlierError, OutlierSide, dixon_q, grubbs};
pub use peakchain::{
    BaselineStat, ChainBaseline, ChainError, ChainOptions, ChainResult, FindMethod, FindOptions, LimitMode, PeakInfo,
    analyze_peaks, auto_minima_anchors, find_peak_indices, integrate_peaks,
};
pub use peaks::{PeakDirection, PeakFilter, PeakOptions, PeakResult, find_peaks};
pub use power::{
    PowerError, PowerTest, SampleSizes, anova_power, anova_powers, anova_sample_sizes, pnf, pnt, t_power, t_powers,
    t_sample_sizes,
};
pub use signal::{convolve, correlate, hilbert};
pub use smooth::{Boundary, LocalSpan, SmoothMethod, SmoothOptions, smooth, smooth_xy};
pub use stats::{RowStats, Stats, mean_confidence_limits, quantile_origin, row_stats, stats};
