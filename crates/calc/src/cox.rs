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
//!
//! **Strata** (T451): with a stratum for each case the risk sets are taken within each stratum only; the partial
//! log-likelihood, its gradient and information are the sums over the strata, the coefficients are shared, and each
//! stratum has its own baseline (a survival curve at the covariate means per stratum).
//!
//! **Residuals and predictions** (T457): the martingale residual of case `i` is `d_i - H0(t_i) exp(x_i'b)` with its
//! stratum's baseline cumulative hazard (at `x = 0`) up to and including `t_i`; the deviance residual is `sign(M)
//! sqrt(-2 (M + d ln(d - M)))`; the Schoenfeld residual of a failure is `x_i` minus the risk-set mean of `x` at its
//! time (Efron: the mean of the `d` stepped means of its tie group). The survival at covariates `x` is the curve at
//! the means raised to `exp((x - mean)'b)`.
//!
//! **Proportional-hazards test** (T458): Grambsch and Therneau's test on the scaled Schoenfeld residuals against `g`,
//! the rank of the failure time among the failures (ties share their mean rank). With `U = sum (g_k - mean g) r_k`
//! over the `d` failures, `V` the covariance of `b` and `S = sum (g_k - mean g)^2`: covariate `j` has `d (V U)_j^2 /
//! (V_jj S)` on 1 degree of freedom, all together `d U' V U / S` on `p`. Other time scales `g` (T503, R's `cox.zph`
//! transforms): `km`, one minus the left-continuous Kaplan-Meier estimate of all the cases (strata pooled, no
//! covariates) just before the failure time; `identity`, the time itself; `log`, its logarithm.

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
    /// Distinct failure times and the survival at the covariate means just after each (the first stratum's).
    pub times: Vec<f64>,
    pub survival: Vec<f64>,
    /// The `(times, survival)` curve of each stratum, in stratum order (one curve without strata; T451).
    pub curves: Vec<(Vec<f64>, Vec<f64>)>,
    /// The covariate means the curves are drawn at (T457).
    pub means: Vec<f64>,
    /// Per case, in input order: martingale and deviance residuals (T457).
    pub martingale: Vec<f64>,
    pub deviance: Vec<f64>,
    /// Per failure, in time order within strata: the case index and its Schoenfeld residuals (one per covariate).
    pub schoenfeld: Vec<(usize, Vec<f64>)>,
    /// The failure time of each Schoenfeld residual (T458).
    pub schoenfeld_times: Vec<f64>,
    /// The covariance of the coefficients, the inverse information (T458).
    pub cov: Vec<Vec<f64>>,
    /// The Kaplan-Meier estimate of all the cases just before each Schoenfeld residual's failure time (T503).
    pub km_before: Vec<f64>,
}

/// The time scale of the proportional-hazards test (T503).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhTransform {
    /// The rank of the failure time among the failures (ties share their mean rank).
    #[default]
    Rank,
    /// One minus the left-continuous Kaplan-Meier estimate (R's default).
    Km,
    /// The failure time.
    Identity,
    /// The logarithm of the failure time.
    Log,
}

/// The proportional-hazards test (T458): `(chi-square, p)` per covariate and for all of them.
#[derive(Debug, Clone, PartialEq)]
pub struct PhTest {
    pub per_covariate: Vec<(f64, f64)>,
    pub global: (f64, f64),
}

impl Cox {
    /// Grambsch and Therneau's proportional-hazards test against the rank of the failure times (T458).
    pub fn ph_test(&self) -> PhTest {
        self.ph_test_with(PhTransform::Rank)
    }

    /// The time scale `g` of each Schoenfeld residual under `transform` (T503).
    pub fn ph_times(&self, transform: PhTransform) -> Vec<f64> {
        let d = self.schoenfeld.len();
        match transform {
            PhTransform::Km => self.km_before.iter().map(|s| 1.0 - s).collect(),
            PhTransform::Identity => self.schoenfeld_times.clone(),
            PhTransform::Log => self.schoenfeld_times.iter().map(|t| t.ln()).collect(),
            PhTransform::Rank => {
                let mut idx: Vec<usize> = (0..d).collect();
                idx.sort_by(|&a, &b| self.schoenfeld_times[a].total_cmp(&self.schoenfeld_times[b]));
                let mut g = vec![0.0; d];
                let mut k = 0;
                while k < d {
                    let mut end = k;
                    while end < d && self.schoenfeld_times[idx[end]] == self.schoenfeld_times[idx[k]] {
                        end += 1;
                    }
                    let rank = (k + end + 1) as f64 / 2.0;
                    for &i in &idx[k..end] {
                        g[i] = rank;
                    }
                    k = end;
                }
                g
            }
        }
    }

    /// [`Cox::ph_test`] on the time scale `transform` (T503).
    pub fn ph_test_with(&self, transform: PhTransform) -> PhTest {
        let p = self.coefficients.len();
        let d = self.schoenfeld.len();
        let g = self.ph_times(transform);
        let mean = g.iter().sum::<f64>() / d as f64;
        let s: f64 = g.iter().map(|v| (v - mean).powi(2)).sum();
        let mut u = vec![0.0; p];
        for ((_, r), gk) in self.schoenfeld.iter().zip(&g) {
            for j in 0..p {
                u[j] += (gk - mean) * r[j];
            }
        }
        let vu: Vec<f64> = (0..p).map(|i| (0..p).map(|j| self.cov[i][j] * u[j]).sum()).collect();
        let df = d as f64;
        let chi = |stat: f64, k: usize| ChiSquared::new(k as f64).map_or(f64::NAN, |c| c.sf(stat.max(0.0)));
        let per_covariate = (0..p)
            .map(|j| {
                let stat = df * vu[j] * vu[j] / (self.cov[j][j] * s);
                (stat, chi(stat, 1))
            })
            .collect();
        let global = df * u.iter().zip(&vu).map(|(a, b)| a * b).sum::<f64>() / s;
        PhTest { per_covariate, global: (global, chi(global, p)) }
    }

    /// The survival curve of each stratum at covariates `x` (T457): the curve at the means to the power `exp((x -
    /// mean)'b)`.
    pub fn survival_at(&self, x: &[f64]) -> Vec<(Vec<f64>, Vec<f64>)> {
        let shift: f64 = x.iter().zip(&self.means).zip(&self.coefficients).map(|((a, m), b)| (a - m) * b).sum();
        let power = shift.exp();
        self.curves.iter().map(|(t, s)| (t.clone(), s.iter().map(|v| v.powf(power)).collect())).collect()
    }
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
    cox_stratified(x, t, failed, &vec![0; t.len()], confidence, ties)
}

/// The sums of [`evaluate`] over the strata.
fn evaluate_all(
    x: &[Vec<f64>],
    failed: &[bool],
    groups: &[(Vec<usize>, Vec<usize>)],
    b: &[f64],
    ties: Ties,
) -> (f64, Vec<f64>, Vec<Vec<f64>>) {
    let p = b.len();
    let (mut ll, mut grad, mut info) = (0.0, vec![0.0; p], vec![vec![0.0; p]; p]);
    for (order, start) in groups {
        let (l, g, h) = evaluate(x, failed, order, start, b, ties);
        ll += l;
        for i in 0..p {
            grad[i] += g[i];
            for j in 0..p {
                info[i][j] += h[i][j];
            }
        }
    }
    (ll, grad, info)
}

/// [`cox_with`] with a stratum number for each case (T451): risk sets within strata, one baseline per stratum.
pub fn cox_stratified(
    x: &[Vec<f64>],
    t: &[f64],
    failed: &[bool],
    strata: &[usize],
    confidence: f64,
    ties: Ties,
) -> Result<Cox, CoxError> {
    let n = t.len();
    if strata.len() != n {
        return Err(CoxError::Ragged);
    }
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
    // The cases of each stratum sorted by time, and the start of each case's tie group within its stratum.
    let count = strata.iter().copied().max().map_or(0, |m| m + 1);
    let groups: Vec<(Vec<usize>, Vec<usize>)> = (0..count)
        .map(|s| {
            let mut order: Vec<usize> = (0..n).filter(|&c| strata[c] == s).collect();
            order.sort_by(|&a, &b| t[a].total_cmp(&t[b]).then(a.cmp(&b)));
            let mut start = vec![0; order.len()];
            for k in 1..order.len() {
                start[k] = if t[order[k]] == t[order[k - 1]] { start[k - 1] } else { k };
            }
            (order, start)
        })
        .filter(|(order, _)| !order.is_empty())
        .collect();
    let zero = vec![0.0; p];
    let (ll0, g0, i0) = evaluate_all(x, failed, &groups, &zero, ties);
    let mut b = zero.clone();
    let (mut ll, mut g, mut info) = (ll0, g0.clone(), i0.clone());
    for _ in 0..100 {
        let inv = invert(&info).ok_or(CoxError::NoConvergence)?;
        let step: Vec<f64> = (0..p).map(|i| (0..p).map(|j| inv[i][j] * g[j]).sum()).collect();
        let mut scale = 1.0;
        let mut moved = false;
        for _ in 0..40 {
            let cand: Vec<f64> = b.iter().zip(&step).map(|(a, d)| a + scale * d).collect();
            let (lc, gc, ic) = evaluate_all(x, failed, &groups, &cand, ties);
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
    let weight = |c: usize| x[c].iter().zip(&b).map(|(a, cc)| a * cc).sum::<f64>().exp();
    let mut curves = Vec::new();
    let mut martingale = vec![0.0; n];
    let mut schoenfeld = Vec::new();
    for (order, _) in &groups {
        let m = order.len();
        let (mut times, mut survival) = (Vec::new(), Vec::new());
        let mut h0 = 0.0;
        let mut k = 0;
        while k < m {
            let tk = t[order[k]];
            let mut end = k;
            let mut d = 0usize;
            let mut tied = 0.0;
            let mut tied_x = vec![0.0; p];
            while end < m && t[order[end]] == tk {
                if failed[order[end]] {
                    d += 1;
                    let w = weight(order[end]);
                    tied += w;
                    for (a, v) in tied_x.iter_mut().zip(&x[order[end]]) {
                        *a += w * v;
                    }
                }
                end += 1;
            }
            if d > 0 {
                let risk: f64 = order[k..].iter().map(|&c| weight(c)).sum();
                let mut risk_x = vec![0.0; p];
                for &c in &order[k..] {
                    let w = weight(c);
                    for (a, v) in risk_x.iter_mut().zip(&x[c]) {
                        *a += w * v;
                    }
                }
                h0 += match ties {
                    Ties::Breslow => d as f64 / risk,
                    Ties::Efron => (0..d).map(|l| 1.0 / (risk - l as f64 / d as f64 * tied)).sum(),
                };
                // The risk-set mean of x at this time (Efron: the mean of the stepped means).
                let mean_x: Vec<f64> = (0..p)
                    .map(|i| match ties {
                        Ties::Breslow => risk_x[i] / risk,
                        Ties::Efron => {
                            (0..d)
                                .map(|l| {
                                    let f = l as f64 / d as f64;
                                    (risk_x[i] - f * tied_x[i]) / (risk - f * tied)
                                })
                                .sum::<f64>()
                                / d as f64
                        }
                    })
                    .collect();
                for &c in &order[k..end] {
                    if failed[c] {
                        schoenfeld.push((c, x[c].iter().zip(&mean_x).map(|(a, mu)| a - mu).collect()));
                    }
                }
                times.push(tk);
                survival.push((-h0 * eta_mean.exp()).exp());
            }
            // Every case at this time has the cumulative hazard up to and including it.
            for &c in &order[k..end] {
                martingale[c] = f64::from(u8::from(failed[c])) - h0 * weight(c);
            }
            k = end;
        }
        curves.push((times, survival));
    }
    let deviance: Vec<f64> = martingale
        .iter()
        .zip(failed)
        .map(|(mr, f)| {
            let d = f64::from(u8::from(*f));
            let inner = if *f { mr + d * (d - mr).ln() } else { *mr };
            mr.signum() * (-2.0 * inner).max(0.0).sqrt()
        })
        .collect();
    let (times, survival) = curves.first().cloned().unwrap_or_default();
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
        curves,
        means,
        martingale,
        deviance,
        km_before: {
            // T503: the Kaplan-Meier estimate of all the cases just before each failure's time.
            let km = |at: f64| -> f64 {
                let mut times: Vec<f64> =
                    t.iter().zip(failed).filter(|(ti, f)| **f && **ti < at).map(|(ti, _)| *ti).collect();
                times.sort_by(f64::total_cmp);
                times.dedup();
                times
                    .iter()
                    .map(|&tau| {
                        let at_risk = t.iter().filter(|ti| **ti >= tau).count() as f64;
                        let died = t.iter().zip(failed).filter(|(ti, f)| **f && **ti == tau).count() as f64;
                        1.0 - died / at_risk
                    })
                    .product()
            };
            schoenfeld.iter().map(|(c, _)| km(t[*c])).collect()
        },
        schoenfeld_times: schoenfeld.iter().map(|(c, _)| t[*c]).collect(),
        schoenfeld,
        cov,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ph_test_takes_rs_time_transforms() {
        // T503: failures at 1, 2, 3 and 5; the Kaplan-Meier estimate just before them is 1, 5/6, 4/6 and 4/9
        // (6 at risk at 1, 5 at 2, 3 at 3, 1 at 5).
        let x = vec![vec![0.5], vec![1.2], vec![0.3], vec![2.0], vec![0.7], vec![1.5]];
        let t = [1.0, 2.0, 2.0, 3.0, 4.0, 5.0];
        let failed = [true, true, false, true, false, true];
        let m = cox(&x, &t, &failed, 0.95).unwrap();
        assert_eq!(m.schoenfeld_times, [1.0, 2.0, 3.0, 5.0]);
        let km = m.ph_times(PhTransform::Km);
        for (g, want) in km.iter().zip([0.0, 1.0 / 6.0, 1.0 / 3.0, 5.0 / 9.0]) {
            assert!((g - want).abs() < 1e-12, "{km:?}");
        }
        assert_eq!(m.ph_times(PhTransform::Identity), [1.0, 2.0, 3.0, 5.0]);
        assert_eq!(m.ph_times(PhTransform::Rank), [1.0, 2.0, 3.0, 4.0]);
        assert!((m.ph_times(PhTransform::Log)[3] - 5f64.ln()).abs() < 1e-15);
        // Rank stays the default; the scales give their own statistics.
        assert_eq!(m.ph_test_with(PhTransform::Rank), m.ph_test());
        let (a, b) = (m.ph_test_with(PhTransform::Km).global, m.ph_test_with(PhTransform::Identity).global);
        assert!(a.0.is_finite() && b.0.is_finite() && a.0 != b.0, "{a:?} {b:?}");
    }

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
    fn one_stratum_is_the_plain_model_and_strata_split_the_risk_sets() {
        let t = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let x: Vec<Vec<f64>> = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0].iter().map(|v| vec![*v]).collect();
        let failed = [true, true, true, false, true, true, true, true];
        let plain = cox_with(&x, &t, &failed, 0.95, Ties::Breslow).unwrap();
        let one = cox_stratified(&x, &t, &failed, &[0; 8], 0.95, Ties::Breslow).unwrap();
        assert_eq!(plain, one);
        assert_eq!(plain.curves.len(), 1);
        // Two strata: at b = 0 each failure's risk set counts only its own stratum.
        let strata = [0, 0, 0, 0, 1, 1, 1, 1];
        let s = cox_stratified(&x, &t, &failed, &strata, 0.95, Ties::Breslow).unwrap();
        // Stratum 0 failures at 1, 2, 3 with risk sets 4, 3, 2; stratum 1 at 5, 6, 7, 8 with 4, 3, 2, 1.
        let null = -(4f64.ln() + 3f64.ln() + 2f64.ln() + 4f64.ln() + 3f64.ln() + 2f64.ln() + 1f64.ln());
        assert!((s.log_likelihood_null - null).abs() < 1e-12, "{}", s.log_likelihood_null);
        assert_eq!(s.curves.len(), 2);
        assert_eq!(s.curves[1].0, [5.0, 6.0, 7.0, 8.0]);
        assert!(cox_stratified(&x, &t, &failed, &[0; 3], 0.95, Ties::Breslow).is_err());
    }

    #[test]
    fn residuals_sum_as_they_should_and_curves_follow_the_covariates() {
        let t = [1.0, 2.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let x: Vec<Vec<f64>> = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0].iter().map(|v| vec![*v]).collect();
        let failed = [true, true, true, false, true, true, false, true];
        for ties in [Ties::Breslow, Ties::Efron] {
            let c = cox_with(&x, &t, &failed, 0.95, ties).unwrap();
            // At the maximum the Schoenfeld residuals sum to the score, 0.
            let s: f64 = c.schoenfeld.iter().map(|(_, r)| r[0]).sum();
            assert!(s.abs() < 1e-8, "{ties:?}: {s}");
            assert_eq!(c.schoenfeld.len(), c.events);
            // With Breslow's baseline the martingale residuals sum to 0 (each increment is d / risk).
            if ties == Ties::Breslow {
                let m: f64 = c.martingale.iter().sum();
                assert!(m.abs() < 1e-12, "{m}");
            }
            assert!(c.deviance.iter().zip(&c.martingale).all(|(d, m)| d.signum() == m.signum() || *m == 0.0));
        }
        let c = cox(&x, &t, &failed, 0.95).unwrap();
        // At the means the curve is the curve; at x = 1 it is the means' curve to exp((1 - mean) b).
        assert_eq!(c.survival_at(&c.means.clone())[0].1, c.survival);
        let one = c.survival_at(&[1.0]);
        let power = ((1.0 - c.means[0]) * c.coefficients[0]).exp();
        assert!((one[0].1[2] - c.survival[2].powf(power)).abs() < 1e-15);
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
