//! Straight-line fit with errors in x and y (Analysis > Fitting > Linear Fit with X Error).
//!
//! The line minimises `S = sum (y_i - a - b x_i)^2 / (sy_i^2 + b^2 sx_i^2)`, which is the
//! orthogonal-distance (errors-in-variables) fit with weights `1/sx^2` and `1/sy^2`. The solution is
//! found with York's iteration (York 1966, in the form of York et al. 2004 without correlated
//! errors); the standard errors are York's `sb = 1/sqrt(sum W u^2)`, `sa = sqrt(1/sum W + xbar^2
//! sb^2)`, multiplied by `sqrt(S / (n - 2))` unless the scaling is switched off.
//!
//! Origin's own routine could not be driven by script (`docs/spec/fitting.md`, section 12), so
//! this has been checked against the published York data and against the weighted straight-line
//! fit for zero x errors, not against Origin.

use crate::dist::{f_upper_p, t_critical, t_two_sided_p};
use crate::report::{Anova, FitError, FitReport, FitStatus, Parameter, Statistics};

#[derive(Debug, Clone, PartialEq)]
pub struct XErrOptions {
    pub confidence_level: f64,
    /// Multiply the variances by the reduced chi-square (default, as the other fits).
    pub scale_errors: bool,
}

impl Default for XErrOptions {
    fn default() -> Self {
        XErrOptions { confidence_level: 0.95, scale_errors: true }
    }
}

/// Fits `y = Intercept + Slope * x` to points with the standard errors `sx` of x and `sy` of y. A
/// point with a missing x, y or error is left out.
pub fn fit_linear_xerr(
    x: &[f64],
    y: &[f64],
    sx: &[f64],
    sy: &[f64],
    opts: &XErrOptions,
) -> Result<FitReport, FitError> {
    let n_all = x.len();
    if y.len() != n_all || sx.len() != n_all || sy.len() != n_all {
        return Err(FitError::LengthMismatch { x: n_all, y: y.len().min(sx.len()).min(sy.len()) });
    }
    if !(opts.confidence_level > 0.0 && opts.confidence_level < 1.0) {
        return Err(FitError::Failed(format!(
            "the confidence level must be strictly between 0 and 1, not {}",
            opts.confidence_level
        )));
    }
    let used: Vec<usize> = (0..n_all)
        .filter(|&i| x[i].is_finite() && y[i].is_finite() && sx[i].is_finite() && sy[i].is_finite())
        .collect();
    let n = used.len();
    if n < 2 {
        return Err(FitError::TooFewPoints { have: n, need: 2 });
    }
    if used.iter().any(|&i| sx[i] < 0.0 || sy[i] < 0.0) {
        return Err(FitError::Failed("errors must not be negative".into()));
    }
    if used.iter().any(|&i| sx[i] == 0.0 && sy[i] == 0.0) {
        return Err(FitError::Failed("a point with an error of zero in both x and y cannot be weighted".into()));
    }
    let px: Vec<f64> = used.iter().map(|&i| x[i]).collect();
    let py: Vec<f64> = used.iter().map(|&i| y[i]).collect();
    let psx: Vec<f64> = used.iter().map(|&i| sx[i]).collect();
    let psy: Vec<f64> = used.iter().map(|&i| sy[i]).collect();

    // Start from the unweighted least-squares slope.
    let (mx, my) = (px.iter().sum::<f64>() / n as f64, py.iter().sum::<f64>() / n as f64);
    let sxx: f64 = px.iter().map(|v| (v - mx).powi(2)).sum();
    if sxx == 0.0 {
        return Err(FitError::Failed("all x values are equal, so the slope is not determined".into()));
    }
    let mut b = px.iter().zip(&py).map(|(a, c)| (a - mx) * (c - my)).sum::<f64>() / sxx;
    let mut converged = false;
    let (mut w, mut beta) = (vec![0.0; n], vec![0.0; n]);
    for _ in 0..500 {
        for i in 0..n {
            w[i] = 1.0 / (psy[i] * psy[i] + b * b * psx[i] * psx[i]);
        }
        let sw: f64 = w.iter().sum();
        let xbar = (0..n).map(|i| w[i] * px[i]).sum::<f64>() / sw;
        let ybar = (0..n).map(|i| w[i] * py[i]).sum::<f64>() / sw;
        let (mut num, mut den) = (0.0, 0.0);
        for i in 0..n {
            let (u, v) = (px[i] - xbar, py[i] - ybar);
            beta[i] = w[i] * (u * psy[i] * psy[i] + b * v * psx[i] * psx[i]);
            num += w[i] * beta[i] * v;
            den += w[i] * beta[i] * u;
        }
        if den == 0.0 {
            return Err(FitError::Failed("the iteration for the slope broke down".into()));
        }
        let nb = num / den;
        let done = (nb - b).abs() <= 1e-15 * nb.abs().max(1e-300);
        b = nb;
        if done {
            converged = true;
            break;
        }
    }
    // Final weights and sums at the converged slope.
    for i in 0..n {
        w[i] = 1.0 / (psy[i] * psy[i] + b * b * psx[i] * psx[i]);
    }
    let sw: f64 = w.iter().sum();
    let xbar = (0..n).map(|i| w[i] * px[i]).sum::<f64>() / sw;
    let ybar = (0..n).map(|i| w[i] * py[i]).sum::<f64>() / sw;
    for i in 0..n {
        let (u, v) = (px[i] - xbar, py[i] - ybar);
        beta[i] = w[i] * (u * psy[i] * psy[i] + b * v * psx[i] * psx[i]);
    }
    let a = ybar - b * xbar;
    let xadj: Vec<f64> = (0..n).map(|i| xbar + beta[i]).collect();
    let xadj_bar = (0..n).map(|i| w[i] * xadj[i]).sum::<f64>() / sw;
    let su: f64 = (0..n).map(|i| w[i] * (xadj[i] - xadj_bar).powi(2)).sum();
    let var_b = 1.0 / su;
    let var_a = 1.0 / sw + xadj_bar * xadj_bar * var_b;
    let cov_ab = -xadj_bar * var_b;

    let s: f64 = (0..n).map(|i| w[i] * (py[i] - a - b * px[i]).powi(2)).sum();
    let dof = n - 2;
    let reduced = if dof > 0 { s / dof as f64 } else { f64::NAN };
    let scale = if opts.scale_errors && dof > 0 { reduced } else { 1.0 };
    let (se_a, se_b) = ((var_a * scale).sqrt(), (var_b * scale).sqrt());
    let tss: f64 = (0..n).map(|i| w[i] * (py[i] - ybar).powi(2)).sum();
    let r2 = if tss > 0.0 { 1.0 - s / tss } else { f64::NAN };
    let adj = if dof > 0 && tss > 0.0 { 1.0 - reduced / (tss / (n - 1) as f64) } else { f64::NAN };
    let tcrit = if dof > 0 { t_critical(opts.confidence_level, dof) } else { f64::NAN };
    let param = |name: &str, value: f64, se: f64| -> Parameter {
        let t = value / se;
        Parameter {
            name: name.into(),
            value,
            std_error: if dof > 0 { se } else { f64::NAN },
            fixed: false,
            t_value: if dof > 0 { t } else { f64::NAN },
            prob_t: if dof > 0 { t_two_sided_p(t, dof) } else { f64::NAN },
            lcl: value - tcrit * se,
            ucl: value + tcrit * se,
            dependency: f64::NAN,
        }
    };
    let mut warnings = Vec::new();
    if n < n_all {
        warnings.push(format!("{} point(s) with a missing value were left out", n_all - n));
    }
    if !converged {
        warnings.push("the iteration for the slope did not converge in 500 steps".into());
    }
    if dof == 0 {
        warnings.push("there are no degrees of freedom left; standard errors and probabilities are not defined".into());
    }
    let anova = (dof > 0).then(|| {
        let model_ss = tss - s;
        let f = model_ss / (s / dof as f64);
        Anova {
            model_df: 1,
            model_ss,
            model_ms: model_ss,
            error_df: dof,
            error_ss: s,
            error_ms: reduced,
            total_df: n - 1,
            total_ss: tss,
            uncorrected_total_ss: f64::NAN,
            f_value: f,
            prob_f: f_upper_p(f, 1, dof),
        }
    });
    let mut fitted = vec![f64::NAN; n_all];
    let mut residuals = vec![f64::NAN; n_all];
    for &i in &used {
        fitted[i] = a + b * x[i];
        residuals[i] = y[i] - fitted[i];
    }
    Ok(FitReport {
        model: "Linear with X Error".into(),
        equation: "y = Intercept + Slope*x".into(),
        params: vec![param("Intercept", a, se_a), param("Slope", b, se_b)],
        stats: Statistics {
            n_points: n,
            dof,
            rss: s,
            reduced_chi_sqr: reduced,
            r_squared: r2,
            adj_r_squared: adj,
            pearson_r: if r2 >= 0.0 { Some(b.signum() * r2.sqrt()) } else { None },
            iterations: None,
        },
        anova,
        covariance: vec![var_a * scale, cov_ab * scale, cov_ab * scale, var_b * scale],
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
    use crate::linear::{LinearOptions, fit_linear};
    use crate::report::Weighting;

    /// Pearson's data with York's weights (York 1966; York et al. 2004, Table 1): the slope is
    /// -0.4805 and the intercept 5.4799.
    #[test]
    fn york_data_give_the_published_line() {
        let x = [0.0, 0.9, 1.8, 2.6, 3.3, 4.4, 5.2, 6.1, 6.5, 7.4];
        let y = [5.9, 5.4, 4.4, 4.6, 3.5, 3.7, 2.8, 2.8, 2.4, 1.5];
        let wx = [1000.0, 1000.0, 500.0, 800.0, 200.0, 80.0, 60.0, 20.0, 1.8, 1.0];
        let wy = [1.0, 1.8, 4.0, 8.0, 20.0, 20.0, 70.0, 70.0, 100.0, 500.0];
        let sx: Vec<f64> = wx.iter().map(|w| 1.0 / f64::sqrt(*w)).collect();
        let sy: Vec<f64> = wy.iter().map(|w| 1.0 / f64::sqrt(*w)).collect();
        let r = fit_linear_xerr(&x, &y, &sx, &sy, &XErrOptions::default()).unwrap();
        assert!((r.param("Slope").unwrap().value + 0.48053).abs() < 2e-5, "{}", r.param("Slope").unwrap().value);
        assert!((r.param("Intercept").unwrap().value - 5.47991).abs() < 2e-5);
        // York et al. 2004: reduced chi-square 1.4833 (sum of 11.866 over 8 degrees of freedom).
        assert!((r.stats.reduced_chi_sqr - 1.4833).abs() < 2e-3, "{}", r.stats.reduced_chi_sqr);
    }

    #[test]
    fn without_x_errors_it_is_the_weighted_straight_line() {
        let x: Vec<f64> = (1..=10).map(f64::from).collect();
        let y: Vec<f64> = x.iter().map(|v| 2.0 + 0.5 * v + 0.3 * (3.0 * v).sin()).collect();
        let sy: Vec<f64> = x.iter().map(|v| 0.1 + 0.05 * v).collect();
        let sx = vec![0.0; x.len()];
        let r = fit_linear_xerr(&x, &y, &sx, &sy, &XErrOptions::default()).unwrap();
        let lin =
            fit_linear(&x, &y, &LinearOptions { weighting: Weighting::Instrumental(sy.clone()), ..Default::default() })
                .unwrap();
        for name in ["Intercept", "Slope"] {
            let (a, b) = (r.param(name).unwrap(), lin.param(name).unwrap());
            assert!((a.value - b.value).abs() < 1e-10 * b.value.abs().max(1.0), "{name}");
            assert!((a.std_error - b.std_error).abs() < 1e-9 * b.std_error, "{name} error");
        }
        assert!((r.stats.rss - lin.stats.rss).abs() < 1e-9 * lin.stats.rss);
    }

    #[test]
    fn x_errors_flatten_the_slope_less_than_ignoring_them_would() {
        // Noisy x: the x-error fit is steeper than the ordinary fit of y on x (regression dilution).
        let x = [1.0, 2.2, 2.7, 4.3, 4.6, 6.4, 6.6, 8.4, 8.5, 10.4];
        let y: Vec<f64> = (1..=10).map(|i| 1.0 + 0.8 * i as f64).collect();
        let (sx, sy) = (vec![1.0; 10], vec![0.5; 10]);
        let r = fit_linear_xerr(&x, &y, &sx, &sy, &XErrOptions::default()).unwrap();
        let plain = fit_linear(&x, &y, &LinearOptions::default()).unwrap();
        assert!(r.param("Slope").unwrap().value > plain.param("Slope").unwrap().value);
    }

    #[test]
    fn refuses_points_without_any_error() {
        let r = fit_linear_xerr(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.5], &[0.0; 3], &[0.0; 3], &XErrOptions::default());
        assert!(r.is_err());
    }
}
