//! Cox proportional hazards regression (for Origin 2026b's reliability and survival analysis; not in Origin 2016, so
//! nothing here is measured in Origin; `docs/spec/statistics.md` section 52).
//!
//! The hazard of a case with covariates `x` is `h0(t) exp(x'b)`. With Breslow's handling of tied times, the partial
//! log-likelihood is `sum over failures i of [x_i'b - ln sum_{j: t_j >= t_i} exp(x_j'b)]`. It is maximized by
//! Newton's method from `b = 0` with step halving; the standard errors come from the inverse of the information
//! matrix, the hazard ratios are `exp(b)` with intervals `exp(b -+ z SE)`, and the likelihood-ratio, Wald and score
//! tests of `b = 0` use the chi-square distribution on as many degrees of freedom as covariates. The baseline
//! cumulative hazard is Breslow's `H0(t) = sum over failure times <= t of d / sum_{risk set} exp(x'b)`, and the survival
//! at the covariate means is `exp(-H0(t) exp(mean x'b))`. Checked against an independent numpy/scipy fit in
//! `crates/calc/tests/cox_reference.rs` (made by `tools/cox-ref.py`).
//!
//! **Efron's ties** (T445): at a time with `d` tied failures `D` the risk-set sum `S` is replaced, for `l = 0..d-1`, by
//! `S - (l/d) sum_{j in D} exp(x_j'b)` (and likewise in the gradient and information sums), so the tied failures leave
//! the risk set gradually; the baseline hazard increment is `sum_l 1 / (S - (l/d) sum_D exp(x'b))`. Without ties the
//! two rules agree.

use crate::stability::invert;
use statrs::distribution::{ChiSquared, ContinuousCDF, Normal};

/// A fitted Cox model.
#[derive(Debug, Clone, PartialEq)]
pub struct Cox {
    pub coefficients: Vec<f64>,
    pub se: Vec<f64>,
    pub z: Vec<f64>,
    pub p: Vec<f64>,
    pub hazard_ratio: Vec<f64>,
    pub hr_low: Vec<f64>,
    pub hr_high: Vec<f64>,
    pub log_likelihood: f64,
    pub log_likelihood_null: f64,
    /// (statistic, p) of the likelihood-ratio, Wald and score tests.
    pub lr: (f64, f64),
    pub wald: (f64, f64),
    pub score: (f64, f64),
    pub n: usize,
    pub events: usize,
    /// Distinct failure times and the survival at the covariate means just after each.
    pub times: Vec<f64>,
    pub survival: Vec<f64>,
}

/// Why the model could not be fitted.
#[derive(Debug, Clone, PartialEq)]
pub enum CoxError {
    Ragged,
    Missing(usize),
    NoCovariates,
    TooFewEvents(usize),
    BadLevel,
    NoConvergence,
}

impl std::fmt::Display for CoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoxError::Ragged => f.write_str("the covariates, times and censor flags differ in length"),
            CoxError::Missing(r) => write!(f, "row {} has a missing value", r + 1),
            CoxError::NoCovariates => f.write_str("give at least one covariate column"),
            CoxError::TooFewEvents(k) => write!(f, "{k} failures are too few for this model"),
            CoxError::BadLevel => f.write_str("the confidence level must lie between 0 and 1"),
            CoxError::NoConvergence => {
                f.write_str("the fit did not converge (a covariate may separate the failures completely)")
            }
        }
    }
}

impl std::error::Error for CoxError {}

/// How tied failure times enter the partial likelihood (T445).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ties {
    /// Every tied failure sees the whole risk set.
    #[default]
    Breslow,
    /// The tied failures leave the risk set in equal steps.
    Efron,
}

/// The partial log-likelihood, its gradient and the information matrix at `b`; `order` sorts the cases by time and
/// `start[k]` is the first sorted index whose time equals that of sorted case `k` (the start of its risk set).
fn evaluate(
    x: &[Vec<f64>],
    failed: &[bool],
    order: &[usize],
    start: &[usize],
    b: &[f64],
    ties: Ties,
) -> (f64, Vec<f64>, Vec<Vec<f64>>) {
    let n = order.len();
    let p = b.len();
    // Suffix sums over the sorted cases of w, w x and w x x'.
    let mut s0 = vec![0.0; n + 1];
    let mut s1 = vec![vec![0.0; p]; n + 1];
    let mut s2 = vec![vec![vec![0.0; p]; p]; n + 1];
    for k in (0..n).rev() {
        let row = &x[order[k]];
        let eta: f64 = row.iter().zip(b).map(|(a, c)| a * c).sum();
        let w = eta.exp();
        s0[k] = s0[k + 1] + w;
        for i in 0..p {
            s1[k][i] = s1[k + 1][i] + w * row[i];
            for j in 0..p {
                s2[k][i][j] = s2[k + 1][i][j] + w * row[i] * row[j];
            }
        }
    }
    let mut ll = 0.0;
    let mut grad = vec![0.0; p];
    let mut info = vec![vec![0.0; p]; p];
    let mut k = 0;
    while k < n {
        // The cases tied at this time: k..end, the risk set from k on.
        let mut end = k;
        while end < n && start[end] == start[k] {
            end += 1;
        }
        let tied: Vec<usize> = order[k..end].iter().copied().filter(|&c| failed[c]).collect();
        let d = tied.len();
        if d > 0 {
            // The failures' own sums, taken out of the risk set in steps of 1/d under Efron's rule.
            let (mut a0, mut a1, mut a2) = (0.0, vec![0.0; p], vec![vec![0.0; p]; p]);
            for &c in &tied {
                let row = &x[c];
                let eta: f64 = row.iter().zip(b).map(|(a, cc)| a * cc).sum();
                let w = eta.exp();
                ll += eta;
                a0 += w;
                for i in 0..p {
                    grad[i] += row[i];
                    a1[i] += w * row[i];
                    for j in 0..p {
                        a2[i][j] += w * row[i] * row[j];
                    }
                }
            }
            for l in 0..d {
                let f = if ties == Ties::Efron { l as f64 / d as f64 } else { 0.0 };
                let den = s0[k] - f * a0;
                ll -= den.ln();
                let mean: Vec<f64> = (0..p).map(|i| (s1[k][i] - f * a1[i]) / den).collect();
                for i in 0..p {
                    grad[i] -= mean[i];
                    for j in 0..p {
                        info[i][j] += (s2[k][i][j] - f * a2[i][j]) / den - mean[i] * mean[j];
                    }
                }
            }
        }
        k = end;
    }
    (ll, grad, info)
}

/// Fits the Cox model to times `t` with covariate rows `x`; `failed[i]` is false for a censored case. Breslow ties.
pub fn cox(x: &[Vec<f64>], t: &[f64], failed: &[bool], confidence: f64) -> Result<Cox, CoxError> {
    cox_with(x, t, failed, confidence, Ties::Breslow)
}

/// [`cox`] with the tie rule `ties` (T445).
pub fn cox_with(x: &[Vec<f64>], t: &[f64], failed: &[bool], confidence: f64, ties: Ties) -> Result<Cox, CoxError> {
    let n = t.len();
    if x.len() != n || failed.len() != n || x.iter().any(|r| r.len() != x.first().map_or(0, Vec::len)) {
        return Err(CoxError::Ragged);
    }
    let p = x.first().map_or(0, Vec::len);
    if p == 0 {
        return Err(CoxError::NoCovariates);
    }
    if !(confidence > 0.0 && confidence < 1.0) {
        return Err(CoxError::BadLevel);
    }
    if let Some(r) = (0..n).find(|&r| !t[r].is_finite() || x[r].iter().any(|v| !v.is_finite())) {
        return Err(CoxError::Missing(r));
    }
    let events = failed.iter().filter(|f| **f).count();
    if events <= p {
        return Err(CoxError::TooFewEvents(events));
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| t[a].total_cmp(&t[b]).then(a.cmp(&b)));
    let mut start = vec![0; n];
    for k in 1..n {
        start[k] = if t[order[k]] == t[order[k - 1]] { start[k - 1] } else { k };
    }
    let zero = vec![0.0; p];
    let (ll0, g0, i0) = evaluate(x, failed, &order, &start, &zero, ties);
    let mut b = zero.clone();
    let (mut ll, mut g, mut info) = (ll0, g0.clone(), i0.clone());
    for _ in 0..100 {
        let inv = invert(&info).ok_or(CoxError::NoConvergence)?;
        let step: Vec<f64> = (0..p).map(|i| (0..p).map(|j| inv[i][j] * g[j]).sum()).collect();
        let mut scale = 1.0;
        let mut moved = false;
        for _ in 0..40 {
            let cand: Vec<f64> = b.iter().zip(&step).map(|(a, d)| a + scale * d).collect();
            let (lc, gc, ic) = evaluate(x, failed, &order, &start, &cand, ties);
            if lc.is_finite() && lc >= ll - 1e-12 * ll.abs().max(1.0) {
                moved = lc > ll + 1e-15 * ll.abs().max(1.0);
                b = cand;
                ll = lc;
                g = gc;
                info = ic;
                break;
            }
            scale *= 0.5;
        }
        let small = step.iter().map(|v| v.abs()).fold(0.0, f64::max) < 1e-12;
        if !moved || small {
            break;
        }
    }
    // A coefficient that moves the hazard by more than e^25 across a covariate's range means the failures are
    // separated: the likelihood has no maximum and Newton only stopped because it stopped gaining.
    let separated = (0..p).any(|i| {
        let (lo, hi) = x.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, c), r| (a.min(r[i]), c.max(r[i])));
        !b[i].is_finite() || b[i].abs() * (hi - lo) > 25.0
    });
    if separated {
        return Err(CoxError::NoConvergence);
    }
    let cov = invert(&info).ok_or(CoxError::NoConvergence)?;
    let se: Vec<f64> = (0..p).map(|i| cov[i][i].max(0.0).sqrt()).collect();
    let nrm = Normal::new(0.0, 1.0).expect("the standard normal");
    let z: Vec<f64> = b.iter().zip(&se).map(|(c, s)| c / s).collect();
    let pv: Vec<f64> = z.iter().map(|v| 2.0 * nrm.sf(v.abs())).collect();
    let zq = nrm.inverse_cdf(0.5 + confidence / 2.0);
    let chi = |s: f64| ChiSquared::new(p as f64).map_or(f64::NAN, |d| d.sf(s.max(0.0)));
    let lr = 2.0 * (ll - ll0);
    let wald: f64 = (0..p).map(|i| (0..p).map(|j| b[i] * info[i][j] * b[j]).sum::<f64>()).sum();
    let inv0 = invert(&i0).ok_or(CoxError::NoConvergence)?;
    let score: f64 = (0..p).map(|i| (0..p).map(|j| g0[i] * inv0[i][j] * g0[j]).sum::<f64>()).sum();
    // The baseline at the covariate means (Breslow's or Efron's increments).
    let means: Vec<f64> = (0..p).map(|i| x.iter().map(|r| r[i]).sum::<f64>() / n as f64).collect();
    let eta_mean: f64 = means.iter().zip(&b).map(|(a, c)| a * c).sum();
    let mut times = Vec::new();
    let mut survival = Vec::new();
    let mut h0 = 0.0;
    let mut k = 0;
    while k < n {
        let tk = t[order[k]];
        let mut end = k;
        let mut d = 0usize;
        let mut tied = 0.0;
        let weight = |c: usize| x[c].iter().zip(&b).map(|(a, cc)| a * cc).sum::<f64>().exp();
        while end < n && t[order[end]] == tk {
            if failed[order[end]] {
                d += 1;
                tied += weight(order[end]);
            }
            end += 1;
        }
        if d > 0 {
            let risk: f64 = order[k..].iter().map(|&c| weight(c)).sum();
            h0 += match ties {
                Ties::Breslow => d as f64 / risk,
                Ties::Efron => (0..d).map(|l| 1.0 / (risk - l as f64 / d as f64 * tied)).sum(),
            };
            times.push(tk);
            survival.push((-h0 * eta_mean.exp()).exp());
        }
        k = end;
    }
    Ok(Cox {
        hazard_ratio: b.iter().map(|v| v.exp()).collect(),
        hr_low: b.iter().zip(&se).map(|(c, s)| (c - zq * s).exp()).collect(),
        hr_high: b.iter().zip(&se).map(|(c, s)| (c + zq * s).exp()).collect(),
        coefficients: b,
        se,
        z,
        p: pv,
        log_likelihood: ll,
        log_likelihood_null: ll0,
        lr: (lr, chi(lr)),
        wald: (wald, chi(wald)),
        score: (score, chi(score)),
        n,
        events,
        times,
        survival,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binary_covariate_without_ties_matches_the_hand_likelihood() {
        // Times 1..6, all failures; group 1 fails first. At b the partial likelihood has a closed form.
        let t = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let x: Vec<Vec<f64>> = [1.0, 1.0, 0.0, 1.0, 0.0, 0.0].iter().map(|v| vec![*v]).collect();
        let failed = [true; 6];
        let c = cox(&x, &t, &failed, 0.95).unwrap();
        let ll_at = |b: f64| {
            let w: Vec<f64> = x.iter().map(|r| (r[0] * b).exp()).collect();
            (0..6).map(|i| x[i][0] * b - w[i..].iter().sum::<f64>().ln()).sum::<f64>()
        };
        assert!((c.log_likelihood - ll_at(c.coefficients[0])).abs() < 1e-12);
        let h = 1e-5;
        let slope = (ll_at(c.coefficients[0] + h) - ll_at(c.coefficients[0] - h)) / (2.0 * h);
        assert!(slope.abs() < 1e-7, "the maximum: {slope}");
        assert!(c.coefficients[0] > 0.0, "group 1 fails sooner");
        assert!((c.hazard_ratio[0] - c.coefficients[0].exp()).abs() < 1e-15);
        assert_eq!(c.times.len(), 6);
        assert!(c.survival.windows(2).all(|w| w[1] < w[0]));
    }

    #[test]
    fn efron_equals_breslow_without_ties_and_differs_with_them() {
        let t = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let x: Vec<Vec<f64>> = [1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0].iter().map(|v| vec![*v]).collect();
        let failed = [true, true, true, false, true, true, true];
        let a = cox_with(&x, &t, &failed, 0.95, Ties::Breslow).unwrap();
        let b = cox_with(&x, &t, &failed, 0.95, Ties::Efron).unwrap();
        assert!(
            (a.coefficients[0] - b.coefficients[0]).abs() < 1e-12
                && (a.log_likelihood - b.log_likelihood).abs() < 1e-12
        );
        assert!((a.survival[2] - b.survival[2]).abs() < 1e-12);
        // Two failures at the same time: Efron's likelihood is higher at b = 0 (its denominators are smaller).
        let tied = [1.0, 1.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let a = cox_with(&x, &tied, &failed, 0.95, Ties::Breslow).unwrap();
        let b = cox_with(&x, &tied, &failed, 0.95, Ties::Efron).unwrap();
        assert!(b.log_likelihood_null > a.log_likelihood_null);
        // By hand at b = 0: risk sets 7, 5, 4, 2, 1 (the censored 4 leaves no failure); Efron's pair: 7 and 6.
        let breslow0 = -(2.0 * 7f64.ln() + 5f64.ln() + 3f64.ln() + 2f64.ln() + 1f64.ln());
        let efron0 = -(7f64.ln() + 6f64.ln() + 5f64.ln() + 3f64.ln() + 2f64.ln() + 1f64.ln());
        assert!((a.log_likelihood_null - breslow0).abs() < 1e-12, "{}", a.log_likelihood_null);
        assert!((b.log_likelihood_null - efron0).abs() < 1e-12, "{}", b.log_likelihood_null);
    }

    #[test]
    fn bad_input_is_refused() {
        assert_eq!(cox(&[vec![1.0]], &[1.0, 2.0], &[true, true], 0.95), Err(CoxError::Ragged));
        assert_eq!(cox(&[vec![], vec![]], &[1.0, 2.0], &[true, true], 0.95), Err(CoxError::NoCovariates));
        assert_eq!(cox(&[vec![1.0], vec![0.0]], &[1.0, 2.0], &[true, false], 0.95), Err(CoxError::TooFewEvents(1)));
        // Complete separation: every case with x = 1 fails before every case with x = 0.
        let x: Vec<Vec<f64>> = [1.0, 1.0, 1.0, 0.0, 0.0, 0.0].iter().map(|v| vec![*v]).collect();
        assert_eq!(cox(&x, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[true; 6], 0.95), Err(CoxError::NoConvergence));
    }
}
