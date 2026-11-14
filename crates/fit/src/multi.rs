//! Multiple linear regression `y = b0 + b1 x1 + ... + bp xp` by ordinary least squares
//! (Origin's `fitMR`). Definitions, all confirmed against `fitMR` (`docs/spec/fitting.md`, section 12):
//!
//! - A row with a missing value in y or any x is left out. Degrees of freedom `n - p` with `p` the
//!   free coefficients (a fixed intercept is not counted).
//! - Standard errors `sqrt(diag((X'X)^-1) * rss / dof)`, t-values and `Prob>|t|` with `dof`.
//! - Free intercept: total sum of squares about the mean of y, `n - 1` degrees of freedom. Fixed
//!   intercept (at any value): total about zero (`sum y^2`), `n` degrees of freedom, as in the straight
//!   line fit (`fitting-linear.md`).
//! - R-square `1 - rss/tss`, adjusted `1 - (rss/dof)/(tss/total_df)`, R is its square root.
//! - ANOVA: model `tss - rss` with `total_df - dof` degrees of freedom, error `rss`, `F = MS(model) /
//!   MS(error)`.

use nalgebra::{DMatrix, DVector};

use crate::dist::{f_upper_p, t_critical, t_two_sided_p};
use crate::report::{Anova, FitError, FitReport, FitStatus, Parameter, Statistics};

#[derive(Debug, Clone, PartialEq)]
pub struct MultiOptions {
    /// Hold the intercept at this value instead of fitting it.
    pub fix_intercept: Option<f64>,
    pub confidence_level: f64,
}

impl Default for MultiOptions {
    fn default() -> Self {
        MultiOptions { fix_intercept: None, confidence_level: 0.95 }
    }
}

/// Fits `y` on the columns `xs` (one slice per independent variable, all as long as `y`). `names`
/// names the variables (`x1`, `x2`... when shorter than `xs`).
pub fn fit_multiple(xs: &[Vec<f64>], names: &[String], y: &[f64], opts: &MultiOptions) -> Result<FitReport, FitError> {
    let p = xs.len();
    if p == 0 {
        return Err(FitError::Failed("a multiple regression needs at least one independent variable".into()));
    }
    for x in xs {
        if x.len() != y.len() {
            return Err(FitError::LengthMismatch { x: x.len(), y: y.len() });
        }
    }
    if !(opts.confidence_level > 0.0 && opts.confidence_level < 1.0) {
        return Err(FitError::Failed(format!(
            "the confidence level must be strictly between 0 and 1, not {}",
            opts.confidence_level
        )));
    }
    let n_all = y.len();
    let used: Vec<usize> = (0..n_all).filter(|&i| y[i].is_finite() && xs.iter().all(|x| x[i].is_finite())).collect();
    let n = used.len();
    let fixed = opts.fix_intercept;
    let free_intercept = fixed.is_none();
    let ncoef = p + usize::from(free_intercept);
    if n < ncoef {
        return Err(FitError::TooFewPoints { have: n, need: ncoef });
    }
    let c0 = fixed.unwrap_or(0.0);

    // Design matrix, columns scaled to unit largest entry for the solve.
    let mut a = DMatrix::<f64>::zeros(n, ncoef);
    let off = usize::from(free_intercept);
    for (r, &i) in used.iter().enumerate() {
        if free_intercept {
            a[(r, 0)] = 1.0;
        }
        for j in 0..p {
            a[(r, off + j)] = xs[j][i];
        }
    }
    let rhs = DVector::from_iterator(n, used.iter().map(|&i| y[i] - c0));
    let scale: Vec<f64> =
        (0..ncoef).map(|j| a.column(j).iter().fold(0.0f64, |m, v| m.max(v.abs())).max(f64::MIN_POSITIVE)).collect();
    for j in 0..ncoef {
        for r in 0..n {
            a[(r, j)] /= scale[j];
        }
    }
    let qr = a.clone().qr();
    let rmat = qr.r();
    let diag_max = (0..ncoef).map(|j| rmat[(j, j)].abs()).fold(0.0, f64::max);
    if (0..ncoef).any(|j| rmat[(j, j)].abs() <= 1e-12 * diag_max) {
        return Err(FitError::Failed(
            "the independent variables are linearly dependent (or one is constant), so the coefficients are not determined"
                .into(),
        ));
    }
    let qtb = qr.q().transpose() * &rhs;
    let beta_scaled = rmat
        .solve_upper_triangular(&qtb.rows(0, ncoef).into_owned())
        .ok_or_else(|| FitError::Failed("the least-squares problem is singular".into()))?;
    let rinv =
        rmat.clone().try_inverse().ok_or_else(|| FitError::Failed("the least-squares problem is singular".into()))?;
    let xtx_inv = &rinv * rinv.transpose();

    let mut beta = vec![0.0; ncoef];
    for j in 0..ncoef {
        beta[j] = beta_scaled[j] / scale[j];
    }
    let predict = |r: usize| -> f64 {
        let mut s = c0;
        if free_intercept {
            s += beta[0];
        }
        for j in 0..p {
            s += beta[off + j] * xs[j][used[r]];
        }
        s
    };
    let rss: f64 = (0..n).map(|r| (y[used[r]] - predict(r)).powi(2)).sum();
    let dof = n - ncoef;
    let mse = if dof > 0 { rss / dof as f64 } else { f64::NAN };
    let ybar = used.iter().map(|&i| y[i]).sum::<f64>() / n as f64;
    let (tss, total_df) = if free_intercept {
        (used.iter().map(|&i| (y[i] - ybar).powi(2)).sum::<f64>(), n - 1)
    } else {
        (used.iter().map(|&i| y[i] * y[i]).sum::<f64>(), n)
    };
    let mut warnings = Vec::new();
    if n < n_all {
        warnings.push(format!("{} row(s) with a missing value were left out", n_all - n));
    }
    let r2 = if tss > 0.0 && tss.is_finite() { 1.0 - rss / tss } else { f64::NAN };
    let adj = if dof > 0 && total_df > 0 && tss > 0.0 { 1.0 - mse / (tss / total_df as f64) } else { f64::NAN };
    if !r2.is_finite() {
        warnings.push("y does not vary, so R-square is undefined".into());
    }
    if dof == 0 {
        warnings.push("there are no degrees of freedom left; standard errors and probabilities are not defined".into());
    }
    let tcrit = if dof > 0 { t_critical(opts.confidence_level, dof) } else { f64::NAN };

    let var_name = |j: usize| names.get(j).cloned().filter(|s| !s.is_empty()).unwrap_or_else(|| format!("x{}", j + 1));
    let mut params = Vec::with_capacity(ncoef + usize::from(!free_intercept));
    let mut cov = vec![0.0; (p + 1) * (p + 1)];
    let mut cov_at = |row: usize, col: usize, v: f64| cov[row * (p + 1) + col] = v;
    // The report always lists the intercept first, then the variables.
    let mut push = |name: String, value: f64, k: Option<usize>| {
        let (se, t, prob, lcl, ucl, fixed_flag) = match k {
            Some(k) if dof > 0 => {
                let se = (xtx_inv[(k, k)] / (scale[k] * scale[k]) * mse).sqrt();
                let t = value / se;
                (se, t, t_two_sided_p(t, dof), value - tcrit * se, value + tcrit * se, false)
            }
            Some(_) => (f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, false),
            None => (0.0, f64::NAN, f64::NAN, f64::NAN, f64::NAN, true),
        };
        params.push(Parameter {
            name,
            value,
            std_error: se,
            fixed: fixed_flag,
            t_value: t,
            prob_t: prob,
            lcl,
            ucl,
            dependency: f64::NAN,
        });
    };
    match fixed {
        Some(c) => push("Intercept".into(), c, None),
        None => push("Intercept".into(), beta[0], Some(0)),
    }
    for j in 0..p {
        push(var_name(j), beta[off + j], Some(off + j));
    }
    if dof > 0 {
        // Covariance in report order (intercept row and column are zero when fixed).
        let idx: Vec<Option<usize>> =
            std::iter::once(if free_intercept { Some(0) } else { None }).chain((0..p).map(|j| Some(off + j))).collect();
        for (ri, rk) in idx.iter().enumerate() {
            for (ci, ck) in idx.iter().enumerate() {
                if let (Some(a), Some(b)) = (*rk, *ck) {
                    cov_at(ri, ci, xtx_inv[(a, b)] / (scale[a] * scale[b]) * mse);
                }
            }
        }
    }

    let model_df = total_df - dof;
    let anova = if dof > 0 && model_df > 0 {
        let model_ss = tss - rss;
        let model_ms = model_ss / model_df as f64;
        let f = model_ms / mse;
        Some(Anova {
            model_df,
            model_ss,
            model_ms,
            error_df: dof,
            error_ss: rss,
            error_ms: mse,
            total_df,
            total_ss: tss,
            uncorrected_total_ss: f64::NAN,
            f_value: f,
            prob_f: f_upper_p(f, model_df, dof),
        })
    } else {
        None
    };

    let mut fitted = vec![f64::NAN; n_all];
    let mut residuals = vec![f64::NAN; n_all];
    for (r, &i) in used.iter().enumerate() {
        let f = predict(r);
        fitted[i] = f;
        residuals[i] = y[i] - f;
    }
    let equation = (0..p).fold("y = Intercept".to_string(), |s, j| format!("{s} + b{}*{}", j + 1, var_name(j)));
    Ok(FitReport {
        model: "Multiple Linear Regression".into(),
        equation,
        params,
        stats: Statistics {
            n_points: n,
            dof,
            rss,
            reduced_chi_sqr: mse,
            r_squared: r2,
            adj_r_squared: adj,
            pearson_r: if r2 >= 0.0 { Some(r2.sqrt()) } else { None },
            iterations: None,
        },
        anova,
        covariance: cov,
        confidence_level: opts.confidence_level,
        fitted,
        residuals,
        status: FitStatus::Exact,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_plane_is_recovered() {
        let x1: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let x2: Vec<f64> = (0..10).map(|i| ((i * 7) % 5) as f64).collect();
        let y: Vec<f64> = x1.iter().zip(&x2).map(|(a, b)| 1.0 + 2.0 * a - 3.0 * b).collect();
        let r = fit_multiple(&[x1, x2], &[], &y, &MultiOptions::default()).unwrap();
        assert!((r.params[0].value - 1.0).abs() < 1e-10);
        assert!((r.params[1].value - 2.0).abs() < 1e-10);
        assert!((r.params[2].value + 3.0).abs() < 1e-10);
        assert_eq!(r.params[2].name, "x2");
    }

    #[test]
    fn dependent_columns_are_refused() {
        let x1: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let x2: Vec<f64> = x1.iter().map(|v| 2.0 * v).collect();
        let y: Vec<f64> = x1.iter().map(|v| v * v).collect();
        assert!(fit_multiple(&[x1, x2], &[], &y, &MultiOptions::default()).is_err());
    }
}
