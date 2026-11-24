//! Numerical analysis tools of Sheaf: integration, differentiation, smoothing, normalisation,
//! interpolation, baseline subtraction, peak finding and descriptive statistics.
//!
//! Every tool is a plain function over `&[f64]` slices with an options struct and a result struct.
//! Missing values are NaN. The behaviour follows what Origin computes for the same input; the
//! rules are written down in `docs/spec/analysis-tools.md`, and the tests compare against numbers
//! taken from Origin (see `tools/origin-ref/`).
//!
//! Row numbers in options and results are 1-based, as in a worksheet.

pub mod acceptance;
pub mod acf;
pub mod anova;
pub mod anova2;
pub mod anova3;
pub mod arima;
pub mod baseline;
pub mod bin2d;
pub mod binscatter;
pub mod capability;
pub mod changepoint;
pub mod chord;
pub mod coherence;
pub mod correlation;
pub mod crosstab;
pub mod curvemath;
pub mod curves;
pub mod cwt;
pub mod decimate;
pub mod decompose;
pub mod dierckx;
pub mod differentiate;
pub mod discfreq;
pub mod distfit;
pub mod doe;
pub mod dunnett;
pub mod envelope;
pub mod expsmooth;
pub mod fft;
pub mod fft2;
pub mod fir;
pub mod forest;
pub mod freq;
pub mod fsd;
pub mod graph_summary;
pub mod hexbin;
pub mod hypothesis;
pub mod iir;
pub mod integrate;
pub mod interp;
pub mod kde;
pub mod kmeans;
pub mod levelcross;
pub mod lifedata;
pub mod lifereg;
pub mod manip;
pub mod minverse;
pub mod mixture;
pub mod montecarlo;
pub mod network;
pub mod nonparam;
pub mod normality;
pub mod normalize;
pub mod nptests;
pub mod outlier;
pub mod partcorr;
pub mod pca;
pub mod peakchain;
pub mod peaks;
pub mod poisson;
pub mod power;
pub mod qc;
pub mod qpcr;
pub mod quantreg;
pub mod rarechart;
pub mod risetime;
pub mod rmanova;
pub mod roc;
pub mod sankey;
pub mod signal;
pub mod smooth;
pub mod stability;
pub mod stationarity;
pub mod stats;
pub mod stemleaf;
pub mod stft;
pub mod sunburst;
pub mod survival;
pub mod trace;
pub mod treemap;
pub mod treemap_tree;
pub mod trend;
pub mod upset;
mod util;
pub mod vartest;
pub mod wavelet;
pub mod xyzarea;

pub use anova::{
    Anova, AnovaError, AnovaRow, Comparison, MeansComparison, anova_one_way, compare_means, compare_to_control, ptukey,
    ptukey_upper, qtukey,
};
pub use anova2::{TwoWayAnova, TwoWayError, TwoWayRow, anova_two_way};
pub use anova3::{
    Effect as ThreeWayEffect, EffectPower, Levene, MAX_THREE_WAY_LEVELS, ThreeWayAnova, ThreeWayError, ThreeWayTerms,
    anova_three_way,
};
pub use baseline::{AlsOptions, BaselineMethod, als_baseline, subtract_baseline, subtract_constant, subtract_line};
pub use bin2d::{AxisBins, AxisLayout, Bin, Bin2d, Bin2dOptions, Bin2dStat, axis_layout, bin2d};
pub use coherence::{CoherenceOptions, CoherenceResult, CoherenceWindow, coherence, coherence_window};
pub use correlation::{Corr, CorrKind, CorrMatrix, correlation, correlation_matrix};
pub use crosstab::{ChiSquareTest, Crosstab, CrosstabError, Fisher};
pub use curvemath::{CurveMathResult, CurveOp, column_math, curve_math_const, curve_math_ref};
pub use curves::{
    AverageMethod, AverageOptions, AverageResult, CommonX, CurveError, Intersection, MeetKind, PolygonArea, XRange,
    average_curves, intersections, polygon_area,
};
pub use cwt::{CwtError, CwtWavelet};
pub use decimate::{DecimateError, DecimateFilter, DecimateOptions, decimate};
pub use differentiate::{DiffOptions, differentiate};
pub use discfreq::{Category, DiscSort, DiscreteFreq, DiscreteRow, discrete_frequency};
pub use distfit::{
    DistFit, Family, FitError, ProbPlot, Scores as PlotScores, cdf as dist_cdf, estimate as dist_estimate,
    fit as dist_fit, log_likelihood as dist_log_likelihood, plotting_positions, probplot, probplot_with,
    standard_quantile,
};
pub use dunnett::{dunnett_cdf, dunnett_critical, dunnett_loadings, dunnett_p};
pub use envelope::{Envelope, EnvelopeError, EnvelopeOptions, EnvelopeType, envelope, extreme_points, natural_spline};
pub use fft::{
    FftFilterOptions, FftFilterType, FftOptions, FftResult, FftWindow, IfftOptions, IfftResult, PowerNorm,
    WindowCorrection, fft_filter, fft_spectrum, fft_window, ifft_spectrum,
};
pub use fir::{
    FirFilter, FirMethod, FirSpec, FirWindow, MAX_FIR_ORDER, MagnitudeUnit, firls, firwin, kaiser_beta, kaiserord,
    window as fir_window,
};
pub use freq::{BinSize, Freq, FreqOptions, FreqRow, auto_bin_range, auto_bin_size, freq_counts};
pub use fsd::{Apodization, FsdOptions, fourier_self_deconvolution};
pub use graph_summary::{SampleSummary, graph_summary, median_confidence_limits, sd_confidence_limits};
pub use hypothesis::{SampleStats, TTest, Tail, ttest_one, ttest_paired, ttest_two};
pub use iir::{
    FilterError, FilterResponse, FreqUnit, IirFilter, IirMethod, IirSpec, MAX_ORDER as IIR_MAX_ORDER, iir_filter,
    iir_order, sosfilt, sosfiltfilt,
};
pub use integrate::{AreaType, CvSegment, IntegrateOptions, IntegrateResult, charge, cv_charge, integrate};
pub use interp::{Extrapolation, InterpMethod, InterpOptions, SplineBoundary, automatic_smoothing, interp1, interp1xy};
pub use kde::{Bandwidth, Density, GRID_POINTS, bandwidth, density_at, kernel_density};
pub use kmeans::{Cluster, Kmeans, KmeansError, KmeansOptions, VariableAnova, kmeans};
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
pub use normalize::{NormalizeMethod, RefStat, normalize, ref_stat};
pub use nptests::{
    Friedman, FriedmanError, FriedmanGroup, KS2_EXACT_MAX_PAIRS, KsTwoSample, MedianGroup, MedianTest,
    SIGN_EXACT_MAX_N, SignTest, friedman, ks_two_sample, median_test, sign_test,
};
pub use outlier::{DixonQ, Grubbs, OutlierError, OutlierSide, dixon_q, grubbs};
pub use partcorr::{PartialCorr, PartialCorrError, partial_correlation};
pub use pca::{EigenTest, Pca, PcaError, PcaOptions, pca};
pub use peakchain::{
    BaselineStat, ChainBaseline, ChainError, ChainOptions, ChainResult, FindMethod, FindOptions, LimitMode, PeakInfo,
    analyze_peaks, analyze_peaks_at, auto_minima_anchors, find_peak_indices, integrate_peaks,
    second_derivative_anchors,
};
pub use peaks::{PeakDirection, PeakFilter, PeakOptions, PeakResult, find_peaks};
pub use poisson::{
    OnePoisson, TwoPoisson, poisson_difference_interval, poisson_exact_interval, poisson_normal_interval, poisson_one,
    poisson_two,
};
pub use power::{
    PowerError, PowerTest, SampleSizes, anova_power, anova_powers, anova_sample_sizes, pnf, pnt, t_power, t_powers,
    t_sample_sizes,
};
pub use rmanova::{
    Correction as RmCorrection, Multivariate as RmMultivariate, RmAnova, RmBetween, RmEffect, RmError, RmLevel, RmRow,
    RmTerm, Sphericity as RmSphericity, rm_anova,
};
pub use roc::{Roc, RocError, RocOptions, RocPoint, roc};
pub use signal::{convolve, correlate, deconvolve, hilbert};
pub use smooth::{Boundary, LocalSpan, SmoothMethod, SmoothOptions, smooth, smooth_xy};
pub use stats::{RowStats, Stats, mean_confidence_limits, quantile_origin, row_stats, stats};
pub use stemleaf::{StemLeaf, StemLeafOptions, StemLine, auto_leaf_unit, stem_and_leaf};
pub use stft::{StftOptions, StftOutput, StftResult, stft};
pub use survival::{
    KaplanMeier, Quartile, RankResult, RankTest, SurvivalError, SurvivalRow, SurvivalStep, kaplan_meier, rank_test,
};
pub use trace::{TraceMethod, trace_interp, trace_points};
pub use vartest::{
    OneProportion, TwoProportions, VarTest, exact_interval, power_one_proportion, power_two_proportions,
    proportion_one, proportion_two, ratio_interval, two_sample_normal_doubtful, variance_interval, vartest_one,
    vartest_two, wald_difference_interval, wald_interval,
};
pub use wavelet::{Extension as WaveletExtension, Threshold as WaveletThreshold, Wavelet, WaveletError};
pub use xyzarea::{SurfaceArea, XyzAreaError, delaunay, xyz_surface_area};
