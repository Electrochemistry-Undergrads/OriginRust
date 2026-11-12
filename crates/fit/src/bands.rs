//! Confidence and prediction bands of a fitted curve (`docs/spec/fitting-bands.md`).
//!
//! At a point `x` the fitted curve `f(x; p)` has the variance `g' C g`, where `g` is the gradient
//! of `f` with respect to the parameters and `C` their covariance matrix scaled by the reduced
//! chi-square. With `t` the two-sided Student critical value for the chosen confidence level and
//! the degrees of freedom of the fit:
//!
//! - the confidence band is `f(x) +- t sqrt(g' C g)` and holds the true curve with the chosen
//!   probability;
//! - the prediction band is `f(x) +- t sqrt(s2 + g' C g)`, with `s2` the reduced chi-square, and
//!   holds a new observation with the chosen probability.
//!
//! For a straight line this is Origin's closed form `RMSE sqrt(1/W + (x - xbar)^2 / Sxx)` (and
//! with a leading `1` under the root for the prediction band), where `W` is the sum of weights.

use crate::dist::t_critical;
use crate::nlsf::FitFunction;
use crate::report::{FitError, FitReport};

/// Bands of one fitted curve at the given x values. All vectors have the length of `x`; a value is
/// NaN where it is undefined (no degrees of freedom, singular covariance).
#[derive(Debug, Clone, PartialEq)]
pub struct Bands {
    /// Confidence level the bands belong to, for example 0.95.
    pub level: f64,
    /// The student-t value that was used.
    pub t: f64,
    pub x: Vec<f64>,
    /// The fitted curve at `x`.
    pub fit: Vec<f64>,
    pub conf_lower: Vec<f64>,
    pub conf_upper: Vec<f64>,
    pub pred_lower: Vec<f64>,
    pub pred_upper: Vec<f64>,
}

impl Bands {
    /// Builds the bands from the curve values and the variance of the curve at every x
    /// (`g' C g`, covariance already scaled by the reduced chi-square); `s2` is the reduced
    /// chi-square, the variance of a single new observation about the curve.
    pub fn from_variances(x: &[f64], fit: &[f64], var: &[f64], s2: f64, dof: usize, level: f64) -> Bands {
        let t = t_critical(level, dof);
        let half = |extra: f64| -> Vec<f64> {
            var.iter().map(|v| if *v >= 0.0 { t * (v + extra).sqrt() } else { f64::NAN }).collect()
        };
        let conf = half(0.0);
        let pred = half(s2);
        let minus = |h: &[f64]| fit.iter().zip(h).map(|(f, h)| f - h).collect::<Vec<f64>>();
        let plus = |h: &[f64]| fit.iter().zip(h).map(|(f, h)| f + h).collect::<Vec<f64>>();
        Bands {
            level,
            t,
            x: x.to_vec(),
            fit: fit.to_vec(),
            conf_lower: minus(&conf),
            conf_upper: plus(&conf),
            pred_lower: minus(&pred),
            pred_upper: plus(&pred),
        }
    }
}

/// Bands of a nonlinear fit at the given x values. `report` is the result of
/// [`crate::nlsf::fit`] for `func`. The gradient of the model is taken by central differences.
///
/// `covariance_scaled` says whether `report.covariance` was scaled by the reduced chi-square
/// (`FitOptions::scale_errors` of `None` or `Some(true)`); the bands always use the scaled
/// covariance, so an unscaled one is multiplied by the reduced chi-square here.
pub fn nl_bands(
    func: &FitFunction,
    report: &FitReport,
    xs: &[f64],
    level: f64,
    covariance_scaled: bool,
) -> Result<Bands, FitError> {
    if !(level > 0.0 && level < 1.0) {
        return Err(FitError::Failed(format!("the confidence level must be between 0 and 1, not {level}")));
    }
    let np = report.params.len();
    if report.covariance.len() != np * np {
        return Err(FitError::Failed("the fit report has no covariance matrix".to_string()));
    }
    let p: Vec<f64> = report.params.iter().map(|q| q.value).collect();
    let s2 = report.stats.reduced_chi_sqr;
    let factor = if covariance_scaled { 1.0 } else { s2 };
    let mut fit = Vec::with_capacity(xs.len());
    let mut var = Vec::with_capacity(xs.len());
    let mut q = p.clone();
    for &x in xs {
        fit.push(func.eval(&p, x));
        let mut g = vec![0.0; np];
        for j in 0..np {
            if report.params[j].fixed {
                continue;
            }
            let h = 6e-6 * p[j].abs().max(1e-3);
            q[j] = p[j] + h;
            let up = func.eval(&q, x);
            q[j] = p[j] - h;
            let down = func.eval(&q, x);
            q[j] = p[j];
            g[j] = (up - down) / (2.0 * h);
        }
        let mut v = 0.0;
        for i in 0..np {
            for j in 0..np {
                v += g[i] * report.covariance[i * np + j] * g[j];
            }
        }
        var.push(v * factor);
    }
    Ok(Bands::from_variances(xs, &fit, &var, s2, report.stats.dof, level))
}
