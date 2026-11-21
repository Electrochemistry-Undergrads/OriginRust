//! The result of a fit, shared by linear, polynomial and nonlinear fitting.
//!
//! The fields mirror the tables of Origin's fit reports: "Parameters", "Statistics" and "ANOVA".
//! A value Origin leaves blank (for example the t-value of a fixed parameter) is NaN here.

use serde::{Deserialize, Serialize};

/// One parameter of a fitted model, as one row of the "Parameters" table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Name as in the model, for example `Intercept`, `Slope`, `B2`, `xc`, `w`.
    pub name: String,
    pub value: f64,
    /// Standard error. 0 for a fixed parameter (as in Origin's report sheet).
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
    /// `C` is the covariance matrix. NaN where not computed (linear fits); 0 for a fixed parameter of a nonlinear fit.
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
    /// `w = 1 / abs(y)` ("Statistical"); a y of zero is refused.
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

    /// Origin's "Correlation Matrix": `C_ij / sqrt(C_ii * C_jj)` from [`FitReport::covariance`],
    /// row-major in `params` order. 1 on the diagonal of a free parameter; NaN (blank) in the
    /// rows and columns of a fixed parameter, whose variance is zero. Empty when the covariance
    /// has the wrong size.
    pub fn correlation(&self) -> Vec<f64> {
        let n = self.params.len();
        if self.covariance.len() != n * n {
            return Vec::new();
        }
        let c = &self.covariance;
        let sd: Vec<f64> = (0..n)
            .map(|i| if self.params[i].fixed || c[i * n + i] <= 0.0 { f64::NAN } else { c[i * n + i].sqrt() })
            .collect();
        let mut out = vec![f64::NAN; n * n];
        for i in 0..n {
            for j in 0..n {
                if sd[i].is_finite() && sd[j].is_finite() {
                    out[i * n + j] = if i == j { 1.0 } else { c[i * n + j] / (sd[i] * sd[j]) };
                }
            }
        }
        out
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
    #[error("{0}")]
    BadSimulation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(name: &str, fixed: bool) -> Parameter {
        let nan = f64::NAN;
        Parameter {
            name: name.into(),
            value: 1.0,
            std_error: 0.0,
            fixed,
            t_value: nan,
            prob_t: nan,
            lcl: nan,
            ucl: nan,
            dependency: nan,
        }
    }

    fn report(params: Vec<Parameter>, covariance: Vec<f64>) -> FitReport {
        FitReport {
            model: "m".into(),
            equation: String::new(),
            params,
            stats: Statistics {
                n_points: 3,
                dof: 1,
                rss: 0.0,
                reduced_chi_sqr: 0.0,
                r_squared: 1.0,
                adj_r_squared: 1.0,
                pearson_r: None,
                iterations: None,
            },
            anova: None,
            covariance,
            confidence_level: 0.95,
            fitted: Vec::new(),
            residuals: Vec::new(),
            status: FitStatus::Exact,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn correlation_divides_by_the_standard_deviations_and_blanks_fixed_parameters() {
        let r = report(
            vec![param("a", false), param("b", true), param("c", false)],
            vec![4.0, 0.0, -3.0, 0.0, 0.0, 0.0, -3.0, 0.0, 9.0],
        );
        let c = r.correlation();
        assert_eq!((c[0], c[8]), (1.0, 1.0));
        assert_eq!((c[2], c[6]), (-0.5, -0.5));
        assert!(c[1].is_nan() && c[3].is_nan() && c[4].is_nan() && c[5].is_nan() && c[7].is_nan());
        assert!(report(vec![param("a", false)], vec![]).correlation().is_empty());
    }
}
