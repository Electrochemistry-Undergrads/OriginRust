//! Regression with life data: accelerated failure time models (Origin 2026b's "regression with life data"; not in
//! Origin 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 48).
//!
//! `ln T = b0 + b1 x1 + ... + sigma W`, with `W` the standard smallest extreme value (Weibull; shape `1 / sigma`), the
//! standard normal (lognormal) or the extreme value with `sigma = 1` (exponential). With `z = (ln t - x'b) / sigma` a
//! failure adds `-ln sigma - ln t + ln g(z)` and a right-censored time `ln G(z)` (`g` and `G` the density and survival
//! function of `W`), so the log-likelihood is on the scale of the distribution fits of section 37. It is maximized by
//! Newton's method on `(b, ln sigma)` with the analytic gradient, a central-difference Hessian and step halving, from
//! the least-squares line of the failures' log times. Standard errors come from the inverse of the observed
//! information; the scale's from the delta method. Checked against scipy in `crates/calc/tests/lifereg_reference.rs`
//! (made by `tools/lifereg-ref.py`).

use crate::lifedata::LifeDist;
use crate::stability::invert;
use statrs::distribution::{Continuous, ContinuousCDF, Normal};

/// A fitted accelerated failure time model.
#[derive(Debug, Clone, PartialEq)]
pub struct LifeRegression {
    pub dist: LifeDist,
    /// Intercept, then one coefficient per covariate (on log time).
    pub coefficients: Vec<f64>,
    pub se: Vec<f64>,
    pub z: Vec<f64>,
    pub p: Vec<f64>,
    /// `sigma` (1 for the exponential) and its standard error (NaN for the exponential).
    pub scale: f64,
    pub scale_se: f64,
    pub log_likelihood: f64,
    pub n: usize,
    pub failures: usize,
}

/// Why the model could not be fitted.
#[derive(Debug, Clone, PartialEq)]
pub enum LifeRegError {
    Ragged,
    Missing(usize),
    NotPositive(usize),
    TooFewFailures(usize),
    NormalNotAllowed,
    NoConvergence,
}

impl std::fmt::Display for LifeRegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifeRegError::Ragged => f.write_str("the covariates, times and censor flags differ in length"),
            LifeRegError::Missing(r) => write!(f, "row {} has a missing value", r + 1),
            LifeRegError::NotPositive(r) => write!(f, "the time in row {} is not above 0", r + 1),
            LifeRegError::TooFewFailures(k) => write!(f, "{k} failures are too few for this model"),
            LifeRegError::NormalNotAllowed => {
                f.write_str("life-data regression takes Weibull, lognormal or exponential")
            }
            LifeRegError::NoConvergence => f.write_str("the fit did not converge (the information matrix is singular)"),
        }
    }
}

impl std::error::Error for LifeRegError {}

fn std_normal() -> Normal {
    Normal::new(0.0, 1.0).expect("the standard normal")
}

/// `ln g(z)`, `ln G(z)` and their derivatives `d ln g / dz`, `d ln G / dz`.
fn terms(dist: LifeDist, z: f64) -> (f64, f64, f64, f64) {
    match dist {
        LifeDist::Lognormal => {
            let n = std_normal();
            let s = n.sf(z);
            let mills = if s > 1e-300 { n.pdf(z) / s } else { z + 1.0 / z };
            (n.ln_pdf(z), s.max(f64::MIN_POSITIVE).ln(), -z, -mills)
        }
        // Smallest extreme value: g = exp(z - e^z), G = exp(-e^z).
        _ => {
            let ez = z.exp();
            (z - ez, -ez, 1.0 - ez, -ez)
        }
    }
}

struct Data<'a> {
    x: &'a [Vec<f64>],
    lt: Vec<f64>,
    failed: &'a [bool],
    dist: LifeDist,
}

impl Data<'_> {
    fn sigma_free(&self) -> bool {
        self.dist != LifeDist::Exponential
    }

    /// Log-likelihood and gradient at `theta = (b, ln sigma?)`.
    fn eval(&self, theta: &[f64]) -> (f64, Vec<f64>) {
        let k = self.x[0].len() + 1;
        let (ls, s) = if self.sigma_free() { (theta[k], theta[k].exp()) } else { (0.0, 1.0) };
        let mut ll = 0.0;
        let mut g = vec![0.0; theta.len()];
        for ((row, lt), failed) in self.x.iter().zip(&self.lt).zip(self.failed) {
            let eta = theta[0] + row.iter().zip(&theta[1..k]).map(|(a, b)| a * b).sum::<f64>();
            let z = (lt - eta) / s;
            let (lg, lsv, dlg, dls) = terms(self.dist, z);
            let d = if *failed {
                ll += -ls - lt + lg;
                dlg
            } else {
                ll += lsv;
                dls
            };
            // dz/db_j = -x_j / s; dz/d ln s = -z.
            g[0] -= d / s;
            for (j, a) in row.iter().enumerate() {
                g[j + 1] -= d * a / s;
            }
            if self.sigma_free() {
                g[k] += -d * z - if *failed { 1.0 } else { 0.0 };
            }
        }
        (ll, g)
    }

    fn hessian(&self, theta: &[f64]) -> Vec<Vec<f64>> {
        let m = theta.len();
        let mut h = vec![vec![0.0; m]; m];
        for j in 0..m {
            let e = 1e-5 * theta[j].abs().max(1.0);
            let mut up = theta.to_vec();
            let mut dn = theta.to_vec();
            up[j] += e;
            dn[j] -= e;
            let (gu, gd) = (self.eval(&up).1, self.eval(&dn).1);
            for i in 0..m {
                h[i][j] = (gu[i] - gd[i]) / (2.0 * e);
            }
        }
        for i in 0..m {
            for j in 0..i {
                let v = 0.5 * (h[i][j] + h[j][i]);
                h[i][j] = v;
                h[j][i] = v;
            }
        }
        h
    }
}

/// Fits the accelerated failure time model of `dist` to times `t` with covariate rows `x`; `failed[i]` is false
/// for a right-censored time.
pub fn life_regression(
    x: &[Vec<f64>],
    t: &[f64],
    failed: &[bool],
    dist: LifeDist,
) -> Result<LifeRegression, LifeRegError> {
    if dist == LifeDist::Normal {
        return Err(LifeRegError::NormalNotAllowed);
    }
    let n = t.len();
    if x.len() != n || failed.len() != n || n == 0 || x.iter().any(|r| r.len() != x[0].len()) {
        return Err(LifeRegError::Ragged);
    }
    if let Some(r) = (0..n).find(|&r| !t[r].is_finite() || x[r].iter().any(|v| !v.is_finite())) {
        return Err(LifeRegError::Missing(r));
    }
    if let Some(r) = (0..n).find(|&r| t[r] <= 0.0) {
        return Err(LifeRegError::NotPositive(r));
    }
    let p = x[0].len();
    let k = p + 1;
    let failures = failed.iter().filter(|f| **f).count();
    let free = if dist == LifeDist::Exponential { k } else { k + 1 };
    if failures < free {
        return Err(LifeRegError::TooFewFailures(failures));
    }
    let data = Data { x, lt: t.iter().map(|v| v.ln()).collect(), failed, dist };
    // Start: least squares of the failures' log times (all rows when that is singular).
    let rows: Vec<usize> = (0..n).filter(|&r| failed[r]).collect();
    let ls_fit = |rows: &[usize]| -> Option<(Vec<f64>, f64)> {
        let mut xtx = vec![vec![0.0; k]; k];
        let mut xty = vec![0.0; k];
        for &r in rows {
            let xr: Vec<f64> = std::iter::once(1.0).chain(x[r].iter().copied()).collect();
            for i in 0..k {
                xty[i] += xr[i] * data.lt[r];
                for j in 0..k {
                    xtx[i][j] += xr[i] * xr[j];
                }
            }
        }
        let inv = invert(&xtx)?;
        let b: Vec<f64> = (0..k).map(|i| (0..k).map(|j| inv[i][j] * xty[j]).sum()).collect();
        let ss: f64 = rows
            .iter()
            .map(|&r| (data.lt[r] - b[0] - x[r].iter().zip(&b[1..]).map(|(a, c)| a * c).sum::<f64>()).powi(2))
            .sum();
        Some((b, (ss / rows.len() as f64).sqrt().max(0.1)))
    };
    let all: Vec<usize> = (0..n).collect();
    let (b0, s0) = ls_fit(&rows).or_else(|| ls_fit(&all)).ok_or(LifeRegError::NoConvergence)?;
    let mut theta = b0;
    if data.sigma_free() {
        theta.push(s0.ln());
    }
    let (mut ll, mut g) = data.eval(&theta);
    for _ in 0..500 {
        let gnorm = g.iter().map(|v| v.abs()).fold(0.0, f64::max);
        if gnorm < 1e-10 * (n as f64) {
            break;
        }
        let h = data.hessian(&theta);
        let neg: Vec<Vec<f64>> = h.iter().map(|r| r.iter().map(|v| -v).collect()).collect();
        let newton: Option<Vec<f64>> = invert(&neg)
            .map(|inv| (0..free).map(|i| (0..free).map(|j| inv[i][j] * g[j]).sum()).collect::<Vec<f64>>())
            // Away from the maximum the Hessian need not be negative definite: only an uphill Newton step is used.
            .filter(|s| s.iter().zip(&g).map(|(a, b)| a * b).sum::<f64>() > 0.0);
        // The gradient direction, scaled to a step of at most 1 in any parameter.
        let gradient: Vec<f64> = g.iter().map(|v| v / gnorm.max(1.0)).collect();
        let mut moved = false;
        for step in newton.iter().chain(std::iter::once(&gradient)) {
            let mut scale = 1.0;
            for _ in 0..60 {
                let cand: Vec<f64> = theta.iter().zip(step).map(|(a, d)| a + scale * d).collect();
                let (lc, gc) = data.eval(&cand);
                if lc.is_finite() && lc > ll {
                    theta = cand;
                    ll = lc;
                    g = gc;
                    moved = true;
                    break;
                }
                scale *= 0.5;
            }
            if moved {
                break;
            }
        }
        if !moved {
            break;
        }
    }
    if !ll.is_finite() {
        return Err(LifeRegError::NoConvergence);
    }
    let h = data.hessian(&theta);
    let neg: Vec<Vec<f64>> = h.iter().map(|r| r.iter().map(|v| -v).collect()).collect();
    let cov = invert(&neg).ok_or(LifeRegError::NoConvergence)?;
    let se: Vec<f64> = (0..k).map(|i| cov[i][i].max(0.0).sqrt()).collect();
    let z: Vec<f64> = theta[..k].iter().zip(&se).map(|(b, s)| b / s).collect();
    let p = z.iter().map(|v| 2.0 * std_normal().sf(v.abs())).collect();
    let (scale, scale_se) = if data.sigma_free() {
        let s = theta[k].exp();
        (s, s * cov[k][k].max(0.0).sqrt())
    } else {
        (1.0, f64::NAN)
    };
    Ok(LifeRegression {
        dist,
        coefficients: theta[..k].to_vec(),
        se,
        z,
        p,
        scale,
        scale_se,
        log_likelihood: ll,
        n,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifedata::fit_right_censored;

    #[test]
    fn a_constant_covariate_model_is_the_distribution_fit() {
        // With one covariate whose coefficient is not needed the intercept matches the plain fit's log scale.
        let t = [2.0, 4.0, 6.0, 8.0, 9.0, 3.0, 5.0];
        let failed = [true, true, false, true, false, true, true];
        let x: Vec<Vec<f64>> = (0..7).map(|i| vec![f64::from(i % 2)]).collect();
        let fit = life_regression(&x, &t, &failed, LifeDist::Weibull).unwrap();
        assert!(fit.log_likelihood.is_finite() && fit.scale > 0.0);
        // Intercept only (a zero covariate column would be singular): compare with the distribution fit.
        let none: Vec<Vec<f64>> = (0..7).map(|_| Vec::new()).collect();
        let r = life_regression(&none, &t, &failed, LifeDist::Weibull).unwrap();
        let plain = fit_right_censored(&t, &failed.map(|f| !f), LifeDist::Weibull).unwrap();
        assert!((r.coefficients[0] - plain.params[1].ln()).abs() < 1e-7, "{:?} {:?}", r.coefficients, plain.params);
        assert!((1.0 / r.scale - plain.params[0]).abs() < 1e-6, "shape");
        assert!((r.log_likelihood - plain.log_likelihood).abs() < 1e-8);
        let e = life_regression(&none, &t, &failed, LifeDist::Exponential).unwrap();
        let pe = fit_right_censored(&t, &failed.map(|f| !f), LifeDist::Exponential).unwrap();
        assert!((e.coefficients[0] - pe.params[0].ln()).abs() < 1e-7 && e.scale == 1.0, "{:?} {:?}", e, pe.params);
    }

    #[test]
    fn bad_input_is_refused() {
        let x = vec![vec![1.0], vec![2.0]];
        assert_eq!(life_regression(&x, &[1.0], &[true], LifeDist::Weibull), Err(LifeRegError::Ragged));
        assert_eq!(
            life_regression(&x, &[1.0, 0.0], &[true, true], LifeDist::Weibull),
            Err(LifeRegError::NotPositive(1))
        );
        assert_eq!(
            life_regression(&x, &[1.0, 2.0], &[true, false], LifeDist::Weibull),
            Err(LifeRegError::TooFewFailures(1))
        );
        assert_eq!(
            life_regression(&x, &[1.0, 2.0], &[true, true], LifeDist::Normal),
            Err(LifeRegError::NormalNotAllowed)
        );
    }
}
