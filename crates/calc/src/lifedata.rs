//! Parametric life-data fits to right-censored times, for the Distribution Overview plot (Origin 2026b; not in
//! Origin 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 37).
//!
//! Maximum likelihood with right censoring: a failure at `t` adds `ln f(t)`, a case censored at `t` adds `ln S(t)`.
//!
//! - Exponential: `theta = sum t / r` (`r` failures).
//! - Weibull (shape `k`, scale `lambda`): `k` solves `sum t^k ln t / sum t^k - 1 / k - mean of ln t over the
//!   failures = 0` (all times in the sums), `lambda = (sum t^k / r)^(1/k)`.
//! - Normal and lognormal (on `ln t`; `mu`, `sigma`): Newton's method on the score in `(mu, ln sigma)` from the
//!   failures' estimates, with step halving.
//!
//! The plotting positions are the modified Kaplan-Meier ones: at each failure time the mean of the Kaplan-Meier
//! failure probability just before and just after it.
//!
//! **Arbitrary censoring** (T384): each row is exact, right-censored (failed after a time), left-censored (failed
//! before a time) or interval-censored (failed between two times), adding `ln f(t)`, `ln S(t)`, `ln F(t)` or `ln(F(b) -
//! F(a))`. The likelihood is maximized by Nelder-Mead on `(ln shape, ln scale)`, `(mu, ln sigma)` or `ln theta`, from
//! the right-censored fit of a pseudo sample (exact times, interval midpoints, left times halved, right times censored),
//! restarted from its best point until it stops improving. Checked against scipy in
//! `crates/calc/tests/lifedata_reference.rs` (made by `tools/lifedata-ref.py`).

use statrs::distribution::{Continuous, ContinuousCDF, Normal};
use statrs::function::gamma::gamma;

/// The distribution fitted to the times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LifeDist {
    #[default]
    Weibull,
    Lognormal,
    Exponential,
    Normal,
}

impl LifeDist {
    pub fn name(self) -> &'static str {
        match self {
            LifeDist::Weibull => "Weibull",
            LifeDist::Lognormal => "lognormal",
            LifeDist::Exponential => "exponential",
            LifeDist::Normal => "normal",
        }
    }

    /// The names of the parameters.
    pub fn param_names(self) -> &'static [&'static str] {
        match self {
            LifeDist::Weibull => &["Shape", "Scale"],
            LifeDist::Lognormal => &["Log mean (mu)", "Log SD (sigma)"],
            LifeDist::Exponential => &["Mean (theta)"],
            LifeDist::Normal => &["Mean (mu)", "SD (sigma)"],
        }
    }
}

/// A fitted distribution.
#[derive(Debug, Clone, PartialEq)]
pub struct LifeFit {
    pub dist: LifeDist,
    pub params: Vec<f64>,
    pub log_likelihood: f64,
    pub n: usize,
    pub failures: usize,
    pub mean: f64,
    pub median: f64,
}

/// Why the fit failed.
#[derive(Debug, Clone, PartialEq)]
pub enum LifeError {
    LengthMismatch,
    /// Fewer failures than the distribution needs (1 for the exponential, 2 otherwise).
    TooFewFailures(usize),
    NotPositive(f64),
    NoSpread,
    NoConvergence,
}

impl std::fmt::Display for LifeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifeError::LengthMismatch => f.write_str("the time and censor lists differ in length"),
            LifeError::TooFewFailures(r) => write!(f, "{r} failure(s) are too few for this distribution"),
            LifeError::NotPositive(t) => write!(f, "the time {t} is not above 0 (needed by this distribution)"),
            LifeError::NoSpread => f.write_str("the failure times do not vary"),
            LifeError::NoConvergence => f.write_str("the fit did not converge"),
        }
    }
}

impl std::error::Error for LifeError {}

fn std_normal() -> Normal {
    Normal::new(0.0, 1.0).expect("the standard normal")
}

impl LifeFit {
    /// The distribution function at `t`.
    pub fn cdf(&self, t: f64) -> f64 {
        1.0 - self.survival(t)
    }

    /// The survival function at `t`.
    pub fn survival(&self, t: f64) -> f64 {
        let p = &self.params;
        match self.dist {
            LifeDist::Weibull if t <= 0.0 => 1.0,
            LifeDist::Weibull => (-(t / p[1]).powf(p[0])).exp(),
            LifeDist::Exponential if t <= 0.0 => 1.0,
            LifeDist::Exponential => (-t / p[0]).exp(),
            LifeDist::Lognormal if t <= 0.0 => 1.0,
            LifeDist::Lognormal => std_normal().sf((t.ln() - p[0]) / p[1]),
            LifeDist::Normal => std_normal().sf((t - p[0]) / p[1]),
        }
    }

    /// The density at `t`.
    pub fn pdf(&self, t: f64) -> f64 {
        let p = &self.params;
        match self.dist {
            LifeDist::Weibull if t <= 0.0 => 0.0,
            LifeDist::Weibull => p[0] / p[1] * (t / p[1]).powf(p[0] - 1.0) * (-(t / p[1]).powf(p[0])).exp(),
            LifeDist::Exponential if t < 0.0 => 0.0,
            LifeDist::Exponential => (-t / p[0]).exp() / p[0],
            LifeDist::Lognormal if t <= 0.0 => 0.0,
            LifeDist::Lognormal => std_normal().pdf((t.ln() - p[0]) / p[1]) / (p[1] * t),
            LifeDist::Normal => std_normal().pdf((t - p[0]) / p[1]) / p[1],
        }
    }

    /// The hazard `f / S` at `t`.
    pub fn hazard(&self, t: f64) -> f64 {
        let p = &self.params;
        match self.dist {
            LifeDist::Weibull if t <= 0.0 => 0.0,
            LifeDist::Weibull => p[0] / p[1] * (t / p[1]).powf(p[0] - 1.0),
            LifeDist::Exponential => 1.0 / p[0],
            _ => self.pdf(t) / self.survival(t),
        }
    }

    /// The time at failure probability `q`.
    pub fn quantile(&self, q: f64) -> f64 {
        let p = &self.params;
        match self.dist {
            LifeDist::Weibull => p[1] * (-(-q).ln_1p()).powf(1.0 / p[0]),
            LifeDist::Exponential => -p[0] * (-q).ln_1p(),
            LifeDist::Lognormal => (p[0] + p[1] * std_normal().inverse_cdf(q)).exp(),
            LifeDist::Normal => p[0] + p[1] * std_normal().inverse_cdf(q),
        }
    }
}

/// The log-likelihood of `params` for the right-censored times.
pub fn log_likelihood(dist: LifeDist, params: &[f64], t: &[f64], censored: &[bool]) -> f64 {
    let fit = LifeFit { dist, params: params.to_vec(), log_likelihood: 0.0, n: 0, failures: 0, mean: 0.0, median: 0.0 };
    t.iter().zip(censored).map(|(t, c)| if *c { fit.survival(*t).ln() } else { fit.pdf(*t).ln() }).sum()
}

/// `phi(z) / (1 - Phi(z))`, the normal hazard, stable for large `z`.
fn mills(z: f64) -> f64 {
    let n = std_normal();
    let s = n.sf(z);
    if s > 1e-300 { n.pdf(z) / s } else { z + 1.0 / z }
}

/// Fits `dist` by maximum likelihood to `times`, `censored[i]` marking a right-censored case.
pub fn fit_right_censored(times: &[f64], censored: &[bool], dist: LifeDist) -> Result<LifeFit, LifeError> {
    if times.len() != censored.len() {
        return Err(LifeError::LengthMismatch);
    }
    let (t, c): (Vec<f64>, Vec<bool>) =
        times.iter().zip(censored).filter(|(t, _)| t.is_finite()).map(|(t, c)| (*t, *c)).unzip();
    if dist != LifeDist::Normal
        && let Some(bad) = t.iter().find(|t| **t <= 0.0)
    {
        return Err(LifeError::NotPositive(*bad));
    }
    let fails: Vec<f64> = t.iter().zip(&c).filter(|(_, c)| !**c).map(|(t, _)| *t).collect();
    let r = fails.len();
    let need = if dist == LifeDist::Exponential { 1 } else { 2 };
    if r < need {
        return Err(LifeError::TooFewFailures(r));
    }
    if dist != LifeDist::Exponential && fails.iter().all(|v| *v == fails[0]) {
        return Err(LifeError::NoSpread);
    }
    let rf = r as f64;
    let params = match dist {
        LifeDist::Exponential => vec![t.iter().sum::<f64>() / rf],
        LifeDist::Weibull => {
            // Times over their largest, so that t^k cannot overflow; the scale is put back at the end.
            let top = t.iter().copied().fold(0.0, f64::max);
            let u: Vec<f64> = t.iter().map(|v| v / top).collect();
            let lu: Vec<f64> = u.iter().map(|v| v.ln()).collect();
            let mean_fail_ln = lu.iter().zip(&c).filter(|(_, c)| !**c).map(|(l, _)| *l).sum::<f64>() / rf;
            let g = |k: f64| {
                let (mut a, mut b) = (0.0, 0.0);
                for (v, l) in u.iter().zip(&lu) {
                    let p = v.powf(k);
                    a += p * l;
                    b += p;
                }
                a / b - 1.0 / k - mean_fail_ln
            };
            // g increases with k: bracket the root, then bisect to the last bit.
            let (mut lo, mut hi) = (1e-3, 1.0);
            while g(hi) < 0.0 {
                hi *= 2.0;
                if hi > 1e6 {
                    return Err(LifeError::NoConvergence);
                }
            }
            while g(lo) > 0.0 {
                lo /= 2.0;
                if lo < 1e-12 {
                    return Err(LifeError::NoConvergence);
                }
            }
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if mid <= lo || mid >= hi {
                    break;
                }
                if g(mid) < 0.0 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let k = 0.5 * (lo + hi);
            let scale = (u.iter().map(|v| v.powf(k)).sum::<f64>() / rf).powf(1.0 / k) * top;
            vec![k, scale]
        }
        LifeDist::Normal | LifeDist::Lognormal => {
            let y: Vec<f64> = if dist == LifeDist::Lognormal { t.iter().map(|v| v.ln()).collect() } else { t.clone() };
            normal_censored(&y, &c)?
        }
    };
    let ll = log_likelihood(dist, &params, &t, &c);
    let fit = LifeFit { dist, params, log_likelihood: ll, n: t.len(), failures: r, mean: 0.0, median: 0.0 };
    let p = &fit.params;
    let mean = match dist {
        LifeDist::Weibull => p[1] * gamma(1.0 + 1.0 / p[0]),
        LifeDist::Exponential => p[0],
        LifeDist::Lognormal => (p[0] + 0.5 * p[1] * p[1]).exp(),
        LifeDist::Normal => p[0],
    };
    let median = fit.quantile(0.5);
    if !ll.is_finite() {
        return Err(LifeError::NoConvergence);
    }
    Ok(LifeFit { mean, median, ..fit })
}

/// Normal maximum likelihood of `y` with right censoring: `[mu, sigma]`.
fn normal_censored(y: &[f64], censored: &[bool]) -> Result<Vec<f64>, LifeError> {
    let fails: Vec<f64> = y.iter().zip(censored).filter(|(_, c)| !**c).map(|(v, _)| *v).collect();
    let m0 = fails.iter().sum::<f64>() / fails.len() as f64;
    let s0 = (fails.iter().map(|v| (v - m0).powi(2)).sum::<f64>() / fails.len() as f64).sqrt();
    if s0 <= 0.0 {
        return Err(LifeError::NoSpread);
    }
    // Log-likelihood (up to a constant) and score in (mu, ln sigma).
    let ll = |mu: f64, ls: f64| -> f64 {
        let s = ls.exp();
        y.iter()
            .zip(censored)
            .map(|(v, c)| {
                let z = (v - mu) / s;
                if *c { std_normal().sf(z).max(f64::MIN_POSITIVE).ln() } else { -ls - 0.5 * z * z }
            })
            .sum()
    };
    let score = |mu: f64, ls: f64| -> [f64; 2] {
        let s = ls.exp();
        let (mut a, mut b) = (0.0, 0.0);
        for (v, c) in y.iter().zip(censored) {
            let z = (v - mu) / s;
            if *c {
                let h = mills(z);
                a += h / s;
                b += z * h;
            } else {
                a += z / s;
                b += z * z - 1.0;
            }
        }
        [a, b]
    };
    let (mut mu, mut ls) = (m0, s0.ln());
    for _ in 0..200 {
        let g = score(mu, ls);
        if g[0].abs() * ls.exp() < 1e-13 * y.len() as f64 && g[1].abs() < 1e-13 * y.len() as f64 {
            break;
        }
        // The Hessian by central differences of the analytic score.
        let e = 1e-6;
        let (gm1, gm2) = (score(mu + e * ls.exp(), ls), score(mu - e * ls.exp(), ls));
        let (gl1, gl2) = (score(mu, ls + e), score(mu, ls - e));
        let hmm = (gm1[0] - gm2[0]) / (2.0 * e * ls.exp());
        let hml = (gl1[0] - gl2[0]) / (2.0 * e);
        let hll = (gl1[1] - gl2[1]) / (2.0 * e);
        let hlm = (gm1[1] - gm2[1]) / (2.0 * e * ls.exp());
        let hoff = 0.5 * (hml + hlm);
        let det = hmm * hll - hoff * hoff;
        let (mut dm, mut dl) = if det > 0.0 && hmm < 0.0 {
            ((-hll * g[0] + hoff * g[1]) / det, (hoff * g[0] - hmm * g[1]) / det)
        } else {
            // Not concave here: a small gradient step.
            (g[0] * ls.exp().powi(2) / y.len() as f64, g[1] / y.len() as f64)
        };
        let base = ll(mu, ls);
        let mut ok = false;
        for _ in 0..60 {
            if ll(mu + dm, ls + dl) >= base - 1e-12 * base.abs() {
                ok = true;
                break;
            }
            dm *= 0.5;
            dl *= 0.5;
        }
        if !ok {
            break;
        }
        mu += dm;
        ls += dl;
    }
    let g = score(mu, ls);
    if !(mu.is_finite() && ls.is_finite())
        || g[0].abs() * ls.exp() > 1e-6 * y.len() as f64
        || g[1].abs() > 1e-6 * y.len() as f64
    {
        return Err(LifeError::NoConvergence);
    }
    Ok(vec![mu, ls.exp()])
}

/// Modified Kaplan-Meier plotting positions: for each failure (in increasing time, ties kept apart as in the
/// Kaplan-Meier product), `(t, F)` with `F` the mean of the Kaplan-Meier failure probability just before and just
/// after the failure. Censored cases at the same time as a failure are counted as at risk at that failure.
pub fn plotting_positions(times: &[f64], censored: &[bool]) -> Vec<(f64, f64)> {
    let mut cases: Vec<(f64, bool)> =
        times.iter().zip(censored).filter(|(t, _)| t.is_finite()).map(|(t, c)| (*t, *c)).collect();
    // Failures before censorings at the same time.
    cases.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut at_risk = cases.len() as f64;
    let mut s = 1.0;
    let mut out = Vec::new();
    for (t, c) in cases {
        if !c {
            let before = s;
            s *= 1.0 - 1.0 / at_risk;
            out.push((t, 1.0 - 0.5 * (before + s)));
        }
        at_risk -= 1.0;
    }
    out
}

/// One row of arbitrarily censored life data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Obs {
    Exact(f64),
    /// Still working at this time.
    Right(f64),
    /// Failed before this time.
    Left(f64),
    /// Failed between the two times.
    Interval(f64, f64),
}

impl Obs {
    /// The row from a start and an end column: equal times are exact, a missing end is right-censored at the start, a
    /// missing start is left-censored at the end, and otherwise the row is an interval. `None` when both are missing
    /// or the start is after the end.
    pub fn from_columns(start: f64, end: f64) -> Option<Obs> {
        match (start.is_finite(), end.is_finite()) {
            (true, true) if start == end => Some(Obs::Exact(start)),
            (true, true) if start < end => Some(Obs::Interval(start, end)),
            (true, false) => Some(Obs::Right(start)),
            (false, true) => Some(Obs::Left(end)),
            _ => None,
        }
    }

    fn times(&self) -> [f64; 2] {
        match *self {
            Obs::Exact(t) | Obs::Right(t) | Obs::Left(t) => [t, t],
            Obs::Interval(a, b) => [a, b],
        }
    }
}

fn fit_of(dist: LifeDist, params: Vec<f64>) -> LifeFit {
    LifeFit { dist, params, log_likelihood: 0.0, n: 0, failures: 0, mean: 0.0, median: 0.0 }
}

/// The log-likelihood of `params` for arbitrarily censored rows.
pub fn log_likelihood_arbitrary(dist: LifeDist, params: &[f64], obs: &[Obs]) -> f64 {
    let f = fit_of(dist, params.to_vec());
    obs.iter()
        .map(|o| match *o {
            Obs::Exact(t) => f.pdf(t).ln(),
            Obs::Right(t) => f.survival(t).ln(),
            Obs::Left(t) => f.cdf(t).ln(),
            Obs::Interval(a, b) => {
                // The difference from the nearer tail keeps its digits.
                let p = if f.cdf(a) > 0.5 { f.survival(a) - f.survival(b) } else { f.cdf(b) - f.cdf(a) };
                p.ln()
            }
        })
        .sum()
}

/// Nelder-Mead minimum of `g` from `start`.
fn simplex(g: &dyn Fn(&[f64]) -> f64, start: &[f64], step: f64) -> (Vec<f64>, f64) {
    let k = start.len();
    let mut pts: Vec<Vec<f64>> = vec![start.to_vec()];
    for i in 0..k {
        let mut q = start.to_vec();
        q[i] += step;
        pts.push(q);
    }
    let mut vals: Vec<f64> = pts.iter().map(|q| g(q)).collect();
    for _ in 0..10_000 {
        let mut ord: Vec<usize> = (0..=k).collect();
        ord.sort_by(|&i, &j| vals[i].total_cmp(&vals[j]));
        pts = ord.iter().map(|&i| pts[i].clone()).collect();
        vals = ord.iter().map(|&i| vals[i]).collect();
        let spread = pts.iter().flat_map(|q| q.iter().zip(&pts[0]).map(|(a, b)| (a - b).abs())).fold(0.0, f64::max);
        if spread < 1e-13 && (vals[k] - vals[0]).abs() <= 1e-15 * vals[0].abs().max(1e-300) {
            break;
        }
        let c: Vec<f64> = (0..k).map(|d| pts[..k].iter().map(|q| q[d]).sum::<f64>() / k as f64).collect();
        let along = |t: f64| c.iter().zip(&pts[k]).map(|(ci, w)| ci + t * (ci - w)).collect::<Vec<f64>>();
        let r = along(1.0);
        let fr = g(&r);
        if fr < vals[0] {
            let e = along(2.0);
            let fe = g(&e);
            if fe < fr {
                pts[k] = e;
                vals[k] = fe;
            } else {
                pts[k] = r;
                vals[k] = fr;
            }
        } else if fr < vals[k - 1] {
            pts[k] = r;
            vals[k] = fr;
        } else {
            let cc = if fr < vals[k] { along(0.5) } else { along(-0.5) };
            let fc = g(&cc);
            if fc < vals[k].min(fr) {
                pts[k] = cc;
                vals[k] = fc;
            } else {
                for i in 1..=k {
                    pts[i] = pts[i].iter().zip(&pts[0]).map(|(q, b)| b + 0.5 * (q - b)).collect();
                    vals[i] = g(&pts[i]);
                }
            }
        }
    }
    let best = (0..=k).min_by(|&i, &j| vals[i].total_cmp(&vals[j])).unwrap_or(0);
    (pts[best].clone(), vals[best])
}

/// Fits `dist` by maximum likelihood to arbitrarily censored rows.
pub fn fit_arbitrary(obs: &[Obs], dist: LifeDist) -> Result<LifeFit, LifeError> {
    if obs.is_empty() {
        return Err(LifeError::TooFewFailures(0));
    }
    if dist != LifeDist::Normal
        && let Some(t) = obs.iter().flat_map(|o| o.times()).find(|t| *t < 0.0 || (*t == 0.0))
    {
        // A left time of 0 or an interval starting at 0 is fine for the start; exact and right times must be above 0.
        let ok = obs.iter().all(|o| match *o {
            Obs::Exact(t) | Obs::Right(t) | Obs::Left(t) => t > 0.0,
            Obs::Interval(a, b) => a >= 0.0 && b > 0.0,
        });
        if !ok {
            return Err(LifeError::NotPositive(t));
        }
    }
    let failures = obs.iter().filter(|o| !matches!(o, Obs::Right(_))).count();
    let need = if dist == LifeDist::Exponential { 1 } else { 2 };
    if failures < need {
        return Err(LifeError::TooFewFailures(failures));
    }
    // A start from the right-censored fit of a pseudo sample.
    let (pt, pc): (Vec<f64>, Vec<bool>) = obs
        .iter()
        .map(|o| match *o {
            Obs::Exact(t) => (t, false),
            Obs::Right(t) => (t, true),
            Obs::Left(t) => (0.5 * t, false),
            Obs::Interval(a, b) => (0.5 * (a + b), false),
        })
        .unzip();
    let start = fit_right_censored(&pt, &pc, dist).map(|f| f.params).or_else(|_| {
        // Pseudo failures that do not vary: spread them by the data's range.
        let lo = pt.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = pt.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mid = 0.5 * (lo + hi).max(1e-9);
        Ok::<Vec<f64>, LifeError>(match dist {
            LifeDist::Weibull => vec![1.5, mid],
            LifeDist::Lognormal => vec![mid.ln(), 0.5],
            LifeDist::Exponential => vec![mid],
            LifeDist::Normal => vec![mid, (hi - lo).max(1e-9) / 4.0],
        })
    })?;
    let to_free = |p: &[f64]| -> Vec<f64> {
        match dist {
            LifeDist::Weibull => vec![p[0].ln(), p[1].ln()],
            LifeDist::Lognormal | LifeDist::Normal => vec![p[0], p[1].ln()],
            LifeDist::Exponential => vec![p[0].ln()],
        }
    };
    let from_free = |z: &[f64]| -> Vec<f64> {
        match dist {
            LifeDist::Weibull => vec![z[0].exp(), z[1].exp()],
            LifeDist::Lognormal | LifeDist::Normal => vec![z[0], z[1].exp()],
            LifeDist::Exponential => vec![z[0].exp()],
        }
    };
    let neg = |z: &[f64]| {
        let v = -log_likelihood_arbitrary(dist, &from_free(z), obs);
        if v.is_finite() { v } else { f64::INFINITY }
    };
    let mut z = to_free(&start);
    let mut best = neg(&z);
    let mut step = 0.2;
    for _ in 0..20 {
        let (zn, v) = simplex(&neg, &z, step);
        let improved = v < best - 1e-13 * best.abs().max(1.0);
        z = zn;
        best = best.min(v);
        step *= 0.3;
        if !improved && step < 1e-4 {
            break;
        }
    }
    if !best.is_finite() {
        return Err(LifeError::NoConvergence);
    }
    let params = from_free(&z);
    let mut fit = fit_of(dist, params);
    let p = fit.params.clone();
    fit.log_likelihood = -best;
    fit.n = obs.len();
    fit.failures = failures;
    fit.mean = match dist {
        LifeDist::Weibull => p[1] * gamma(1.0 + 1.0 / p[0]),
        LifeDist::Exponential => p[0],
        LifeDist::Lognormal => (p[0] + 0.5 * p[1] * p[1]).exp(),
        LifeDist::Normal => p[0],
    };
    fit.median = fit.quantile(0.5);
    Ok(fit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_and_uncensored_weibull() {
        let t = [2.0, 4.0, 6.0, 8.0];
        let f = fit_right_censored(&t, &[false, false, true, false], LifeDist::Exponential).unwrap();
        assert!((f.params[0] - 20.0 / 3.0).abs() < 1e-12 && f.failures == 3 && f.n == 4);
        assert!((f.median - 20.0 / 3.0 * 2f64.ln()).abs() < 1e-12);
        assert!((f.hazard(5.0) - 0.15).abs() < 1e-12);
        // Uncensored normal: the maximum likelihood mean and divisor-n deviation.
        let f = fit_right_censored(&t, &[false; 4], LifeDist::Normal).unwrap();
        assert!((f.params[0] - 5.0).abs() < 1e-9 && (f.params[1] - 5f64.sqrt()).abs() < 1e-9, "{:?}", f.params);
        let w = fit_right_censored(&t, &[false; 4], LifeDist::Weibull).unwrap();
        assert!((w.cdf(w.quantile(0.3)) - 0.3).abs() < 1e-12);
        assert!((w.hazard(3.0) - w.pdf(3.0) / w.survival(3.0)).abs() < 1e-12);
    }

    #[test]
    fn plotting_positions_follow_kaplan_meier() {
        // Failures at 1 and 3, a censoring at 2: KM 0.75 after 1, 0.375 after 3.
        let p = plotting_positions(&[3.0, 1.0, 2.0, 5.0], &[false, false, true, true]);
        assert_eq!(p.len(), 2);
        assert!((p[0].1 - 0.125).abs() < 1e-15 && (p[1].1 - (1.0 - 0.5 * (0.75 + 0.375))).abs() < 1e-15);
    }

    #[test]
    fn arbitrary_censoring_reduces_to_the_right_censored_fit() {
        let t = [2.0, 4.0, 6.0, 8.0, 9.0];
        let c = [false, false, true, false, true];
        let obs: Vec<Obs> = t.iter().zip(&c).map(|(t, c)| if *c { Obs::Right(*t) } else { Obs::Exact(*t) }).collect();
        for dist in [LifeDist::Weibull, LifeDist::Lognormal, LifeDist::Exponential, LifeDist::Normal] {
            let a = fit_right_censored(&t, &c, dist).unwrap();
            let b = fit_arbitrary(&obs, dist).unwrap();
            for (x, y) in a.params.iter().zip(&b.params) {
                assert!((x - y).abs() < 1e-6 * x.abs().max(1.0), "{dist:?}: {:?} vs {:?}", a.params, b.params);
            }
            assert!((a.log_likelihood - b.log_likelihood).abs() < 1e-9, "{dist:?}");
        }
        assert_eq!(Obs::from_columns(3.0, f64::NAN), Some(Obs::Right(3.0)));
        assert_eq!(Obs::from_columns(f64::NAN, 3.0), Some(Obs::Left(3.0)));
        assert_eq!(Obs::from_columns(2.0, 3.0), Some(Obs::Interval(2.0, 3.0)));
        assert_eq!(Obs::from_columns(3.0, 2.0), None);
        let ok = fit_arbitrary(&[Obs::Interval(0.0, 2.0), Obs::Interval(1.0, 3.0), Obs::Right(4.0)], LifeDist::Weibull);
        assert!(ok.is_ok(), "{ok:?}");
    }

    #[test]
    fn bad_input_is_refused() {
        assert_eq!(fit_right_censored(&[1.0], &[false, true], LifeDist::Weibull), Err(LifeError::LengthMismatch));
        assert_eq!(
            fit_right_censored(&[1.0, 2.0], &[false, true], LifeDist::Weibull),
            Err(LifeError::TooFewFailures(1))
        );
        assert_eq!(
            fit_right_censored(&[0.0, 2.0], &[false, false], LifeDist::Lognormal),
            Err(LifeError::NotPositive(0.0))
        );
        assert_eq!(fit_right_censored(&[2.0, 2.0], &[false, false], LifeDist::Normal), Err(LifeError::NoSpread));
        assert!(fit_right_censored(&[-1.0, 2.0], &[false, false], LifeDist::Normal).is_ok());
    }
}
