//! Curve fitting for Sheaf: linear and polynomial regression, nonlinear least squares
//! (Levenberg-Marquardt) with bounds and fixed parameters, a library of built-in fit functions,
//! user-defined functions and import of Origin `.FDF` fit definitions.
//!
//! Every fit returns the same [`FitReport`]: parameter values with standard errors, t-values and
//! confidence limits, goodness-of-fit statistics, an analysis-of-variance table where Origin shows
//! one, and the fitted curve with its residuals. The numbers follow what Origin 2016 reports for
//! the same input; the rules are in `docs/spec/fitting.md` and `docs/spec/fitting-linear.md`, and
//! the tests compare against numbers produced by the installed Origin (see `tools/origin-ref/`).
//!
//! Missing values (NaN) in x, y or the weights drop that point from the fit.

pub mod dist;
pub mod fdf;
pub mod library;
pub mod linear;
pub mod multipeak;
pub mod nlsf;
pub mod report;

pub use report::{Anova, FitError, FitReport, FitStatus, Parameter, Statistics, Weighting};
