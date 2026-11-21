//! One- and two-sample Poisson rate tests (Origin 2026b's One-Sample and Two-Sample Poisson Rate Test,
//! T267). OriginLab documents the dialogs (Summarized input: sample size, total occurrence, exposure; Exact
//! and Normal Approximation; the pooled rate for a test difference of 0) but prints no formulas, and the
//! reference install (Origin 2016) has no such test, so these are the textbook formulas, not measured:
//!
//! - one sample, `x` occurrences in `n` observations of exposure `t` each: the rate is `x / (n t)`; the
//!   normal approximation `z = (x - mu0) / sqrt(mu0)` with `mu0 = rate0 n t`; the exact p-values are Poisson
//!   tails (two-sided: twice the smaller tail, at most 1); the exact interval is the chi-square
//!   (Garwood) one, the normal interval `rate +- z sqrt(rate / (n t))`;
//! - two samples: `z = (r1 - r2 - d) / se` with the unpooled `se = sqrt(r1 / T1 + r2 / T2)` (`T = n t`) or,
//!   for `d = 0` and `pooled`, `sqrt(r (1 / T1 + 1 / T2))` with `r = (x1 + x2) / (T1 + T2)`; the exact test
//!   (only for `d = 0`) is the conditional binomial test of `x1` among `x1 + x2` with `p = T1 / (T1 + T2)`.

use crate::hypothesis::Tail;
use statrs::distribution::{ChiSquared, ContinuousCDF, Normal};
use statrs::function::beta::beta_reg;
use statrs::function::gamma::{gamma_lr, gamma_ur};

fn std_normal() -> Normal {
    Normal::new(0.0, 1.0).expect("standard normal")
}

fn z_p(z: f64, tail: Tail) -> f64 {
    if z.is_nan() {
        return f64::NAN;
    }
    let n = std_normal();
    match tail {
        Tail::Two => (2.0 * n.sf(z.abs())).min(1.0),
        Tail::Upper => n.sf(z),
        Tail::Lower => n.cdf(z),
    }
}

/// `(P(X <= x), P(X >= x))` for a Poisson count with mean `mu`.
fn poisson_tails(x: u64, mu: f64) -> (f64, f64) {
    let xf = x as f64;
    let below = gamma_ur(xf + 1.0, mu);
    let above = if x == 0 { 1.0 } else { gamma_lr(xf, mu) };
    (below.min(1.0), above.min(1.0))
}

/// `(P(X <= k), P(X >= k))` for a binomial count of `n` trials with success probability `p`.
fn binomial_tails(k: u64, n: u64, p: f64) -> (f64, f64) {
    let (kf, nf) = (k as f64, n as f64);
    let below = if k >= n { 1.0 } else { beta_reg(nf - kf, kf + 1.0, 1.0 - p) };
    let above = if k == 0 { 1.0 } else { beta_reg(kf, nf - kf + 1.0, p) };
    (below.min(1.0), above.min(1.0))
}

/// The exact p-value from the two tails: the upper or lower one, or twice the smaller (at most 1).
fn exact_p((below, above): (f64, f64), tail: Tail) -> f64 {
    match tail {
        Tail::Upper => above,
        Tail::Lower => below,
        Tail::Two => (2.0 * below.min(above)).min(1.0),
    }
}

/// What the test of one Poisson rate reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnePoisson {
    pub total: u64,
    pub size: u64,
    pub exposure: f64,
    /// The estimated rate `x / (n t)`.
    pub rate: f64,
    pub hypothesized: f64,
    pub z: f64,
    pub p_normal: f64,
    pub p_exact: f64,
    pub tail: Tail,
}

/// Test of the rate of `total` occurrences in `size` observations of `exposure` each against `rate0`. Needs
/// `size > 0`, `exposure > 0` and `rate0 > 0`.
pub fn poisson_one(total: u64, size: u64, exposure: f64, rate0: f64, tail: Tail) -> Option<OnePoisson> {
    if size == 0 || !(exposure > 0.0 && exposure.is_finite()) || !(rate0 > 0.0 && rate0.is_finite()) {
        return None;
    }
    let t = size as f64 * exposure;
    let mu0 = rate0 * t;
    let z = (total as f64 - mu0) / mu0.sqrt();
    Some(OnePoisson {
        total,
        size,
        exposure,
        rate: total as f64 / t,
        hypothesized: rate0,
        z,
        p_normal: z_p(z, tail),
        p_exact: exact_p(poisson_tails(total, mu0), tail),
        tail,
    })
}

/// The share of `1 - level` in each tail of an interval: half for a two-sided one.
fn tail_share(tail: Tail, level: f64) -> f64 {
    if tail == Tail::Two { (1.0 - level) / 2.0 } else { 1.0 - level }
}

/// The exact (chi-square) interval of the rate at `level` (0 to 1). One-sided: the upper-tailed test has
/// no upper limit and the lower-tailed test a lower limit of 0. A limit that does not exist is NaN.
pub fn poisson_exact_interval(total: u64, size: u64, exposure: f64, tail: Tail, level: f64) -> (f64, f64) {
    let t2 = 2.0 * size as f64 * exposure;
    let a = tail_share(tail, level);
    let x = total as f64;
    let lower = || if total == 0 { 0.0 } else { ChiSquared::new(2.0 * x).expect("df > 0").inverse_cdf(a) / t2 };
    let upper = || ChiSquared::new(2.0 * x + 2.0).expect("df > 0").inverse_cdf(1.0 - a) / t2;
    match tail {
        Tail::Two => (lower(), upper()),
        Tail::Upper => (lower(), f64::NAN),
        Tail::Lower => (0.0, upper()),
    }
}

/// The normal-approximation interval `rate +- z sqrt(rate / (n t))`, the lower limit at least 0.
pub fn poisson_normal_interval(total: u64, size: u64, exposure: f64, tail: Tail, level: f64) -> (f64, f64) {
    let t = size as f64 * exposure;
    let rate = total as f64 / t;
    let width = std_normal().inverse_cdf(1.0 - tail_share(tail, level)) * (rate / t).sqrt();
    match tail {
        Tail::Two => ((rate - width).max(0.0), rate + width),
        Tail::Upper => ((rate - width).max(0.0), f64::NAN),
        Tail::Lower => (0.0, rate + width),
    }
}

/// What the test of two Poisson rates reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoPoisson {
    /// `(total occurrence, sample size)` of each sample.
    pub first: (u64, u64),
    pub second: (u64, u64),
    pub exposures: (f64, f64),
    pub rates: (f64, f64),
    /// The estimate `rate1 - rate2`.
    pub estimate: f64,
    pub hypothesized: f64,
    /// NaN, with a NaN p-value, when the standard error is 0 (no occurrences in either sample).
    pub z: f64,
    pub p_normal: f64,
    /// The conditional binomial test, only for a hypothesized difference of 0.
    pub p_exact: Option<f64>,
    pub tail: Tail,
}

/// Test of the difference of two rates (first minus second) against `difference`. Needs both sample sizes
/// and exposures above 0.
pub fn poisson_two(
    first: (u64, u64),
    second: (u64, u64),
    exposures: (f64, f64),
    difference: f64,
    pooled: bool,
    tail: Tail,
) -> Option<TwoPoisson> {
    let ((x1, n1), (x2, n2)) = (first, second);
    let ok = |e: f64| e > 0.0 && e.is_finite();
    if n1 == 0 || n2 == 0 || !ok(exposures.0) || !ok(exposures.1) || !difference.is_finite() {
        return None;
    }
    let (t1, t2) = (n1 as f64 * exposures.0, n2 as f64 * exposures.1);
    let (r1, r2) = (x1 as f64 / t1, x2 as f64 / t2);
    let se = if pooled && difference == 0.0 {
        let r = (x1 + x2) as f64 / (t1 + t2);
        (r * (1.0 / t1 + 1.0 / t2)).sqrt()
    } else {
        (r1 / t1 + r2 / t2).sqrt()
    };
    let z = if se > 0.0 { (r1 - r2 - difference) / se } else { f64::NAN };
    let p_exact = (difference == 0.0).then(|| exact_p(binomial_tails(x1, x1 + x2, t1 / (t1 + t2)), tail));
    Some(TwoPoisson {
        first,
        second,
        exposures,
        rates: (r1, r2),
        estimate: r1 - r2,
        hypothesized: difference,
        z,
        p_normal: z_p(z, tail),
        p_exact,
        tail,
    })
}

/// The normal-approximation interval of `rate1 - rate2`, `+- z sqrt(r1 / T1 + r2 / T2)`; the missing side
/// of a one-sided interval is NaN.
pub fn poisson_difference_interval(
    first: (u64, u64),
    second: (u64, u64),
    exposures: (f64, f64),
    tail: Tail,
    level: f64,
) -> (f64, f64) {
    let (t1, t2) = (first.1 as f64 * exposures.0, second.1 as f64 * exposures.1);
    let (r1, r2) = (first.0 as f64 / t1, second.0 as f64 / t2);
    let d = r1 - r2;
    let width = std_normal().inverse_cdf(1.0 - tail_share(tail, level)) * (r1 / t1 + r2 / t2).sqrt();
    match tail {
        Tail::Two => (d - width, d + width),
        Tail::Upper => (d - width, f64::NAN),
        Tail::Lower => (f64::NAN, d + width),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * b.abs().max(1e-300)
    }

    #[test]
    fn one_sample_against_independent_values() {
        // 12 occurrences in 5 observations of exposure 2 (rate 1.2) against 0.8: mu0 = 8.
        let r = poisson_one(12, 5, 2.0, 0.8, Tail::Two).unwrap();
        assert!(close(r.rate, 1.2, 1e-15));
        assert!(close(r.z, 4.0 / 8f64.sqrt(), 1e-15));
        // scipy: 2 * min(poisson.cdf(12, 8), poisson.sf(11, 8)) = 2 * 0.111924...
        assert!(close(r.p_exact, 2.0 * 0.111_924_001_018_518_58, 1e-10), "{}", r.p_exact);
        let up = poisson_one(12, 5, 2.0, 0.8, Tail::Upper).unwrap();
        assert!(close(up.p_exact, 0.111_924_001_018_518_58, 1e-10));
        assert!(close(up.p_normal, 0.078_649_603_525_142_58, 1e-9), "{}", up.p_normal);
        let low = poisson_one(12, 5, 2.0, 0.8, Tail::Lower).unwrap();
        assert!(close(low.p_exact, 0.936_202_803_263_438_2, 1e-10), "{}", low.p_exact);
        // Exact 95 % interval: chi2.ppf(.025, 24)/20, chi2.ppf(.975, 26)/20.
        let (lo, hi) = poisson_exact_interval(12, 5, 2.0, Tail::Two, 0.95);
        assert!(close(lo, 0.620_057_510_872_221_7, 1e-9) && close(hi, 2.096_158_504_817_696, 1e-9), "{lo} {hi}");
        assert_eq!(poisson_exact_interval(0, 5, 2.0, Tail::Two, 0.95).0, 0.0);
        assert!(poisson_exact_interval(3, 5, 2.0, Tail::Upper, 0.95).1.is_nan());
        let (lo, hi) = poisson_normal_interval(12, 5, 2.0, Tail::Two, 0.95);
        let w = 1.959_963_984_540_054 * (1.2f64 / 10.0).sqrt();
        assert!(close(lo, 1.2 - w, 1e-12) && close(hi, 1.2 + w, 1e-12));
        assert!(poisson_one(1, 0, 1.0, 1.0, Tail::Two).is_none());
        assert!(poisson_one(1, 1, 1.0, 0.0, Tail::Two).is_none());
    }

    #[test]
    fn two_samples_pooled_unpooled_and_exact() {
        // 20 in 10 x 1 against 8 in 8 x 1: rates 2 and 1.
        let r = poisson_two((20, 10), (8, 8), (1.0, 1.0), 0.0, true, Tail::Two).unwrap();
        assert!(close(r.estimate, 1.0, 1e-15));
        let pooled: f64 = 28.0 / 18.0;
        assert!(close(r.z, 1.0 / (pooled * (0.1 + 0.125)).sqrt(), 1e-14), "{}", r.z);
        let u = poisson_two((20, 10), (8, 8), (1.0, 1.0), 0.0, false, Tail::Two).unwrap();
        assert!(close(u.z, 1.0 / (0.2f64 + 0.125).sqrt(), 1e-14));
        // Conditional binomial: x1 = 20 of 28 with p = 10/18; scipy binom.sf(19, 28, 10/18) = 0.064851...
        let up = poisson_two((20, 10), (8, 8), (1.0, 1.0), 0.0, true, Tail::Upper).unwrap();
        assert!(close(up.p_exact.unwrap(), 0.064_850_909_012_253_38, 1e-9), "{:?}", up.p_exact);
        assert!(poisson_two((20, 10), (8, 8), (1.0, 1.0), 0.5, true, Tail::Two).unwrap().p_exact.is_none());
        let none = poisson_two((0, 10), (0, 8), (1.0, 1.0), 0.0, true, Tail::Two).unwrap();
        assert!(none.z.is_nan() && none.p_normal.is_nan());
        let (lo, hi) = poisson_difference_interval((20, 10), (8, 8), (1.0, 1.0), Tail::Two, 0.95);
        let w = 1.959_963_984_540_054 * (0.2f64 + 0.125).sqrt();
        assert!(close(lo, 1.0 - w, 1e-12) && close(hi, 1.0 + w, 1e-12));
    }
}
