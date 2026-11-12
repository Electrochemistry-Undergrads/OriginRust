//! Numerical analysis tools of Sheaf: integration, differentiation, smoothing, normalisation,
//! interpolation, baseline subtraction, peak finding and descriptive statistics.
//!
//! Every tool is a plain function over `&[f64]` slices with an options struct and a result struct.
//! Missing values are NaN. The behaviour follows what Origin 2016 computes for the same input; the
//! rules are written down in `docs/spec/analysis-tools.md`, and the tests compare against numbers
//! taken from Origin (see `tools/origin-ref/`).
//!
//! Row numbers in options and results are 1-based, as in a worksheet.

pub mod baseline;
pub mod curvemath;
pub mod dierckx;
pub mod differentiate;
pub mod integrate;
pub mod interp;
pub mod normalize;
pub mod peaks;
pub mod smooth;
pub mod stats;
mod util;

pub use baseline::{BaselineMethod, subtract_baseline, subtract_constant, subtract_line};
pub use curvemath::{CurveMathResult, CurveOp, curve_math_const, curve_math_ref};
pub use differentiate::{DiffOptions, differentiate};
pub use integrate::{AreaType, IntegrateOptions, IntegrateResult, charge, integrate};
pub use interp::{Extrapolation, InterpMethod, InterpOptions, SplineBoundary, interp1, interp1xy};
pub use normalize::{NormalizeMethod, normalize};
pub use peaks::{PeakDirection, PeakFilter, PeakOptions, PeakResult, find_peaks};
pub use smooth::{Boundary, SmoothMethod, SmoothOptions, smooth, smooth_xy};
pub use stats::{Stats, quantile_origin, stats};
