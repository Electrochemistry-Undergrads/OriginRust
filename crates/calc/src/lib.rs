//! Numerical analysis tools of Sheaf: integration, differentiation, smoothing, normalisation,
//! interpolation, baseline subtraction, peak finding and descriptive statistics.
//!
//! Every tool is a plain function over `&[f64]` slices with an options struct and a result struct.
//! Missing values are NaN. The behaviour follows what Origin 2016 computes for the same input; the
//! rules are written down in `docs/spec/analysis-tools.md`, and the tests compare against numbers
//! taken from Origin (see `tools/origin-ref/`).
//!
//! Row numbers in options and results are 1-based, as in a worksheet.

pub mod anova;
pub mod baseline;
pub mod curvemath;
pub mod dierckx;
pub mod differentiate;
pub mod freq;
pub mod hypothesis;
pub mod integrate;
pub mod interp;
pub mod normality;
pub mod normalize;
pub mod peakchain;
pub mod peaks;
pub mod smooth;
pub mod stats;
mod util;

pub use anova::{Anova, AnovaError, AnovaRow, Comparison, MeansComparison, anova_one_way, ptukey, qtukey};
pub use baseline::{BaselineMethod, subtract_baseline, subtract_constant, subtract_line};
pub use curvemath::{CurveMathResult, CurveOp, curve_math_const, curve_math_ref};
pub use differentiate::{DiffOptions, differentiate};
pub use freq::{BinSize, Freq, FreqOptions, FreqRow, auto_bin_range, auto_bin_size, freq_counts};
pub use hypothesis::{SampleStats, TTest, Tail, ttest_one, ttest_paired, ttest_two};
pub use integrate::{AreaType, IntegrateOptions, IntegrateResult, charge, integrate};
pub use interp::{Extrapolation, InterpMethod, InterpOptions, SplineBoundary, interp1, interp1xy};
pub use normality::{
    DAgostino, TestResult, anderson_darling, dagostino, kolmogorov_cdf, kolmogorov_smirnov, ks_distance, lilliefors,
    shapiro_wilk,
};
pub use normalize::{NormalizeMethod, normalize};
pub use peakchain::{
    ChainBaseline, ChainError, ChainOptions, ChainResult, FindMethod, FindOptions, LimitMode, PeakInfo, analyze_peaks,
    auto_minima_anchors, find_peak_indices, integrate_peaks,
};
pub use peaks::{PeakDirection, PeakFilter, PeakOptions, PeakResult, find_peaks};
pub use smooth::{Boundary, SmoothMethod, SmoothOptions, smooth, smooth_xy};
pub use stats::{Stats, quantile_origin, stats};
