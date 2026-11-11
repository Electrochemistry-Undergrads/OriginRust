//! The result of a fit, shared by linear, polynomial and nonlinear fitting.
//!
//! The fields mirror the tables of Origin's fit reports: "Parameters", "Statistics" and "ANOVA".
//! A value Origin leaves blank (for example the standard error of a fixed parameter) is NaN here.

use serde::{Deserialize, Serialize};

/// One parameter of a fitted model, as one row of the "Parameters" table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Name as in the model, for example `Intercept`, `Slope`, `B2`, `xc`, `w`.
    pub name: String,
    pub value: f64,
    /// Standard error. NaN for a fixed parameter.
    pub std_error: f64,
    /// True if the parameter was held at its value and not fitted.
    pub fixed: bool,
    /// `value / std_error`. NaN for a fixed parameter.
    pub t_value: f64,
    /// Two-sided probability of a t-value at least this large if the true value were zero
    /// ("Prob>|t|"). NaN for a fixed parameter.
    pub prob_t: f64,
    /// Lower and upper confidence limits at [`FitReport::confidence_level`] ("LCL", "UCL").
    pub lcl: f64,
    pub ucl: f64,
    /// Origin's "Dependency" column: `1 - 1 / (C_ii * (C^-1)_ii)` over the free parameters, where
    /// `C` is the covariance matrix. NaN where not computed (linear fits, fixed parameters).
    pub dependency: f64,
}

/// The "Statistics" table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Statistics {
    /// Points used (rows with a missing x, y or weight are left out).
    pub n_points: usize,
    /// Degrees of freedom: points minus free parameters.
    pub dof: usize,
    /// Residual sum of squares, weighted if the fit is weighted.
    pub rss: f64,
    /// `rss / dof`.
    pub reduced_chi_sqr: f64,
    /// Coefficient of determination (COD).
    pub r_squared: f64,
    /// Adjusted R-square.
    pub adj_r_squared: f64,
    /// Pearson's r. Only for a straight-line fit; `None` otherwise.
    pub pearson_r: Option<f64>,
    /// Iterations used by an iterative fit; `None` for linear and polynomial fits.
    pub iterations: Option<usize>,
}

/// The "ANOVA" table. Sums of squares are weighted if the fit is weighted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anova {
    /// "Model" (or "Regression") row.
    pub model_df: usize,
    pub model_ss: f64,
    pub model_ms: f64,
    /// "Error" (or "Residual") row.
    pub error_df: usize,
    pub error_ss: f64,
    pub error_ms: f64,
    /// "Total" (corrected total) row.
    pub total_df: usize,
    pub total_ss: f64,
    /// "Uncorrected Total" sum of squares where Origin shows it (nonlinear fits); NaN otherwise.
    pub uncorrected_total_ss: f64,
    pub f_value: f64,
    /// "Prob>F".
    pub prob_f: f64,
}

/// How the fit ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitStatus {
    /// Converged within the tolerance (Origin: "Succeeded(100)").
    Converged,
    /// Stopped at the iteration limit before converging.
    MaxIterations,
    /// Closed-form fit (linear, polynomial): no iterations.
    Exact,
    /// Something else Origin would report; the text says what.
    Other(String),
}

/// How points are weighted. Origin's names are in brackets.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum Weighting {
    /// Every point counts the same ("No Weighting").
    #[default]
    None,
    /// `w = 1 / err^2` from a y-error column ("Instrumental").
    Instrumental(Vec<f64>),
    /// `w = 1 / y` ("Statistical").
    Statistical,
    /// `w` given directly ("Direct Weighting").
    Direct(Vec<f64>),
}

/// The complete result of one fit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FitReport {
    /// What was fitted, for example `Linear`, `Polynomial (order 3)`, `Gauss`.
    pub model: String,
    /// The model written as mathematics, for example `y = y0 + A*exp(-x/t)`.
    pub equation: String,
    pub params: Vec<Parameter>,
    pub stats: Statistics,
    pub anova: Option<Anova>,
    /// Covariance matrix of all parameters in `params` order, row-major (`params.len()` squared).
    /// Rows and columns of fixed parameters are zero.
    pub covariance: Vec<f64>,
    /// Confidence level of `lcl` and `ucl`, for example 0.95.
    pub confidence_level: f64,
    /// The fitted values at the x of every input point, in input order. NaN where the input
    /// point was left out.
    pub fitted: Vec<f64>,
    /// `y - fitted` for every input point, in input order. NaN where the point was left out.
    pub residuals: Vec<f64>,
    pub status: FitStatus,
    /// Plain-English notes, for example "parameter w reached its lower bound".
    pub warnings: Vec<String>,
}

impl FitReport {
    /// The parameter with this name.
    pub fn param(&self, name: &str) -> Option<&Parameter> {
        self.params.iter().find(|p| p.name == name)
    }
}

/// Why a fit could not be done.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FitError {
    #[error("not enough points: {have} usable points for {need} free parameters")]
    TooFewPoints { have: usize, need: usize },
    #[error("x and y have different lengths ({x} and {y})")]
    LengthMismatch { x: usize, y: usize },
    #[error("the fit function could not be used: {0}")]
    BadFunction(String),
    #[error("the fit failed: {0}")]
    Failed(String),
}
