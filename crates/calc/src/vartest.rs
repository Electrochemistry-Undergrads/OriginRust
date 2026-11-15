//! Tests for variance and proportions: Origin's Statistics > Hypothesis Testing > One-Sample Test for
//! Variance (`vartest1`), Two-Sample Test for Variance (`vartest2`), One-Sample Test for Proportion and
//! Two-Sample Test for Proportions.
//!
//! Measured against OriginPro 2016 (docs/spec/statistics.md, section 17 (Variance and proportion tests); fixtures in
//! `tests/origin_vartest.rs`). What Origin does, as far as these functions copy it:
//!
//! - one sample, variance: `chi2 = (n - 1) s^2 / sigma0^2` with `n - 1` degrees of freedom; the interval is
//!   that of the variance, `[(n - 1) s^2 / chi2(1 - a/2), (n - 1) s^2 / chi2(a/2)]`;
//! - two samples, variance: `F = (s1^2 / s2^2) / ratio0`; the interval is that of the ratio of the
//!   variances, `[r / F(1 - a/2), r / F(a/2)]`;
//! - the two-sided p-value of these asymmetric statistics is `2 min(P(X <= x), P(X >= x))`;
//! - the `vartest1`/`vartest2` X-functions report, for a one-sided test, the two-sided interval of level
//!   `1 - 2 alpha` ([`VarTest::lcl`]); the dialogs' report gives, per confidence level, one finite limit
//!   ([`variance_interval`], [`ratio_interval`]);
//! - one proportion: see [`proportion_one`] (normal approximation with the quirk that z is 0 below `n p0`,
//!   exact binomial test whose two-sided p sums every count no more likely than the observed one, Wald and
//!   Clopper-Pearson intervals);
//! - two proportions: see [`proportion_two`] (only the pooled z, the unpooled Wald interval of the
//!   difference and the layout were measured; see the spec for what was not).

use statrs::distribution::{ChiSquared, ContinuousCDF, FisherSnedecor, Normal};
use statrs::function::beta::beta_reg;
use statrs::function::gamma::ln_gamma;

use crate::hypothesis::{SampleStats, Tail};

/// What a test of one variance or of the ratio of two variances reports.
#[derive(Debug, Clone, PartialEq)]
pub struct VarTest {
    /// One sample, or two for the ratio test.
    pub samples: Vec<SampleStats>,
    /// The sample variance `s^2` (one sample) or the ratio `s1^2 / s2^2` (two samples).
    pub estimate: f64,
    /// The hypothesized variance, or the hypothesized ratio of the variances (usually 1).
    pub hypothesized: f64,
    /// Chi-square (one sample) or F (two samples).
    pub statistic: f64,
    /// Degrees of freedom; the second is NaN for the one-sample test.
    pub df: (f64, f64),
    pub p: f64,
    /// Confidence interval of the estimate (the variance, or the ratio of the variances).
    pub lcl: f64,
    pub ucl: f64,
    /// Confidence level of that interval, for example 0.95.
    pub level: f64,
    pub tail: Tail,
    pub alpha: f64,
}

impl VarTest {
    /// True if the null hypothesis is rejected at the significance level `alpha`.
    pub fn rejected(&self) -> bool {
        self.p < self.alpha
    }
}

fn level_of(tail: Tail, alpha: f64) -> f64 {
    if tail == Tail::Two { 1.0 - alpha } else { 1.0 - 2.0 * alpha }
}

fn p_of(below: f64, above: f64, tail: Tail) -> f64 {
    match tail {
        Tail::Two => (2.0 * below.min(above)).min(1.0),
        Tail::Upper => above,
        Tail::Lower => below,
    }
}

/// Origin accepted every significance level below 1 for the two-sided variance tests (0.6 gave the 40 %
/// interval). A one-sided test with `alpha >= 0.5` has no interval of level `1 - 2 alpha` (NaN limits; not
/// measured in Origin).
fn alpha_ok(alpha: f64) -> bool {
    alpha > 0.0 && alpha < 1.0
}

/// One-sample chi-square test of the variance of `data` against `var0`. NaN entries are ignored. Needs at
/// least two numbers and `var0 > 0`. A sample without spread gives chi-square 0, 0 degrees of freedom, p 0 and
/// no interval (Origin's `vartest1` did exactly that and left the limits unset).
pub fn vartest_one(data: &[f64], var0: f64, tail: Tail, alpha: f64) -> Option<VarTest> {
    let s = SampleStats::of(data);
    if s.n < 2 || !(var0 > 0.0 && var0.is_finite()) || !alpha_ok(alpha) {
        return None;
    }
    let level = level_of(tail, alpha);
    let var = s.sd * s.sd;
    if var == 0.0 {
        return Some(VarTest {
            samples: vec![s],
            estimate: 0.0,
            hypothesized: var0,
            statistic: 0.0,
            df: (0.0, f64::NAN),
            p: 0.0,
            lcl: f64::NAN,
            ucl: f64::NAN,
            level,
            tail,
            alpha,
        });
    }
    let df = s.n as f64 - 1.0;
    let chi = df * var / var0;
    if !chi.is_finite() {
        return None;
    }
    let dist = ChiSquared::new(df).ok()?;
    let p = p_of(dist.cdf(chi), dist.sf(chi), tail);
    let half = (1.0 - level) / 2.0;
    let (lcl, ucl) = if level > 0.0 {
        (df * var / dist.inverse_cdf(1.0 - half), df * var / dist.inverse_cdf(half))
    } else {
        (f64::NAN, f64::NAN)
    };
    Some(VarTest {
        samples: vec![s],
        estimate: var,
        hypothesized: var0,
        statistic: chi,
        df: (df, f64::NAN),
        p,
        lcl,
        ucl,
        level,
        tail,
        alpha,
    })
}

/// Two-sample F test of `var(a) / var(b)` against `ratio0` (Origin's dialog only tests 1, equal variances).
/// NaN entries are ignored. Needs two numbers in each sample and `ratio0 > 0`. A flat first sample gives a
/// ratio of 0 and a flat second one an infinite ratio; Origin left the statistic and p-value unset then and
/// reported the limits 0 and 0 (first flat) or infinity and infinity (second flat), as here (NaN).
pub fn vartest_two(a: &[f64], b: &[f64], ratio0: f64, tail: Tail, alpha: f64) -> Option<VarTest> {
    let (s1, s2) = (SampleStats::of(a), SampleStats::of(b));
    if s1.n < 2 || s2.n < 2 || !(ratio0 > 0.0 && ratio0.is_finite()) || !alpha_ok(alpha) {
        return None;
    }
    let (df1, df2) = (s1.n as f64 - 1.0, s2.n as f64 - 1.0);
    let (v1, v2) = (s1.sd * s1.sd, s2.sd * s2.sd);
    let ratio = v1 / v2;
    let level = level_of(tail, alpha);
    let make = |statistic, p, lcl, ucl| VarTest {
        samples: vec![s1, s2],
        estimate: ratio,
        hypothesized: ratio0,
        statistic,
        df: (df1, df2),
        p,
        lcl,
        ucl,
        level,
        tail,
        alpha,
    };
    if v2 == 0.0 {
        return (v1 > 0.0).then(|| make(f64::NAN, f64::NAN, f64::INFINITY, f64::INFINITY));
    }
    if v1 == 0.0 {
        return Some(make(f64::NAN, f64::NAN, 0.0, 0.0));
    }
    let f = ratio / ratio0;
    let dist = FisherSnedecor::new(df1, df2).ok()?;
    let p = p_of(dist.cdf(f), dist.sf(f), tail);
    let half = (1.0 - level) / 2.0;
    let (lcl, ucl) = if level > 0.0 {
        (ratio / dist.inverse_cdf(1.0 - half), ratio / dist.inverse_cdf(half))
    } else {
        (f64::NAN, f64::NAN)
    };
    Some(make(f, p, lcl, ucl))
}

/// The confidence interval of a variance (`n` numbers, sample variance `var`) as Origin's dialog reports it:
/// two-sided `[(n - 1) var / chi2(1 - a/2), (n - 1) var / chi2(a/2)]` with `a = 1 - level`; for the upper-tailed
/// test only a lower limit `(n - 1) var / chi2(level)` (the upper one is NaN, Origin printed "--"); for the
/// lower-tailed test only an upper limit `(n - 1) var / chi2(1 - level)` and the lower limit 0. (The X-function
/// `vartest1` instead gives the two-sided interval of level `1 - 2 alpha` for a one-sided test; that is
/// [`VarTest::lcl`].) NaN where the sample has no spread.
pub fn variance_interval(n: usize, var: f64, tail: Tail, level: f64) -> (f64, f64) {
    if n < 2 || !(var > 0.0) || !(level > 0.0 && level < 1.0) {
        return (f64::NAN, f64::NAN);
    }
    let df = n as f64 - 1.0;
    let Ok(dist) = ChiSquared::new(df) else { return (f64::NAN, f64::NAN) };
    let a = 1.0 - level;
    match tail {
        Tail::Two => (df * var / dist.inverse_cdf(1.0 - a / 2.0), df * var / dist.inverse_cdf(a / 2.0)),
        Tail::Upper => (df * var / dist.inverse_cdf(level), f64::NAN),
        Tail::Lower => (0.0, df * var / dist.inverse_cdf(a)),
    }
}

/// The confidence interval of the ratio of two variances `ratio = var1 / var2` (samples of `n1` and `n2`
/// numbers), one-sided in the way of [`variance_interval`]: two-sided `[r / F(1 - a/2), r / F(a/2)]`, upper
/// `[r / F(level), NaN]`, lower `[0, r / F(1 - level)]`.
pub fn ratio_interval(n1: usize, n2: usize, ratio: f64, tail: Tail, level: f64) -> (f64, f64) {
    if n1 < 2 || n2 < 2 || !(ratio > 0.0 && ratio.is_finite()) || !(level > 0.0 && level < 1.0) {
        return (f64::NAN, f64::NAN);
    }
    let Ok(dist) = FisherSnedecor::new(n1 as f64 - 1.0, n2 as f64 - 1.0) else { return (f64::NAN, f64::NAN) };
    let a = 1.0 - level;
    match tail {
        Tail::Two => (ratio / dist.inverse_cdf(1.0 - a / 2.0), ratio / dist.inverse_cdf(a / 2.0)),
        Tail::Upper => (ratio / dist.inverse_cdf(level), f64::NAN),
        Tail::Lower => (0.0, ratio / dist.inverse_cdf(a)),
    }
}

/// What the test of one proportion reports (Origin's two sections: normal approximation and binomial test).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OneProportion {
    pub successes: u64,
    pub trials: u64,
    /// The sample proportion `x / n`.
    pub estimate: f64,
    pub hypothesized: f64,
    /// Normal approximation: `(x - n p0) / sqrt(n p0 (1 - p0))`, but 0 when `x < n p0` (Origin's numbers;
    /// see the spec), and its p-value (two-sided `2 (1 - Phi(z))`, upper `1 - Phi(z)`, lower `Phi(z)`).
    pub z: f64,
    pub p_normal: f64,
    /// Exact binomial p-value (two-sided: the sum of the probabilities of every count no more likely than the
    /// observed one).
    pub p_exact: f64,
    pub tail: Tail,
}

/// Origin shows a warning under the normal approximation when fewer than 10 successes or fewer than 10 failures
/// were seen.
pub fn normal_approximation_doubtful(successes: u64, trials: u64) -> bool {
    successes < 10 || trials.saturating_sub(successes) < 10
}

fn std_normal() -> Normal {
    Normal::new(0.0, 1.0).expect("standard normal")
}

fn z_p(z: f64, tail: Tail) -> f64 {
    let n = std_normal();
    p_of(n.cdf(z), n.sf(z), tail)
}

fn ln_choose(n: u64, k: u64) -> f64 {
    ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0)
}

/// Exact binomial p-value of `x` successes in `n` trials against the proportion `p0` (`0 < p0 < 1`).
fn binomial_p(x: u64, n: u64, p0: f64, tail: Tail) -> f64 {
    let (lp, lq) = (p0.ln(), (1.0 - p0).ln());
    let pmf: Vec<f64> = (0..=n).map(|k| (ln_choose(n, k) + k as f64 * lp + (n - k) as f64 * lq).exp()).collect();
    let x = x as usize;
    let below: f64 = pmf[..=x].iter().sum::<f64>().min(1.0);
    let above: f64 = pmf[x..].iter().sum::<f64>().min(1.0);
    match tail {
        Tail::Upper => above,
        Tail::Lower => below,
        Tail::Two => {
            let observed = pmf[x];
            pmf.iter().filter(|p| **p <= observed * (1.0 + 1e-7)).sum::<f64>().min(1.0)
        }
    }
}

/// The `p` in `[0, 1]` where the increasing function `f` crosses `target` (bisection).
fn solve_increasing(f: impl Fn(f64) -> f64, target: f64) -> f64 {
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if f(mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-15 {
            break;
        }
    }
    0.5 * (lo + hi)
}

/// Lower limit of the Clopper-Pearson interval of `x / n` with `a` in each tail.
fn cp_lower(x: u64, n: u64, a: f64) -> f64 {
    if x == 0 {
        return 0.0;
    }
    let (xf, nf) = (x as f64, n as f64);
    // P(X >= x | p) = I_p(x, n - x + 1) grows with p; the limit is where it reaches `a`.
    solve_increasing(|p| beta_reg(xf, nf - xf + 1.0, p), a)
}

/// Upper limit of the Clopper-Pearson interval of `x / n` with `a` in the tail.
fn cp_upper(x: u64, n: u64, a: f64) -> f64 {
    if x == n {
        return 1.0;
    }
    let (xf, nf) = (x as f64, n as f64);
    // P(X <= x | p) = 1 - I_p(x + 1, n - x) falls with p; the limit is where it falls to `a`.
    solve_increasing(|p| beta_reg(xf + 1.0, nf - xf, p), 1.0 - a)
}

/// Test of the proportion of `successes` in `trials` against `p0`. Needs `trials > 0`, `successes <= trials`
/// and `0 < p0 < 1`.
pub fn proportion_one(successes: u64, trials: u64, p0: f64, tail: Tail) -> Option<OneProportion> {
    if trials == 0 || successes > trials || !(p0 > 0.0 && p0 < 1.0) {
        return None;
    }
    let (x, n) = (successes as f64, trials as f64);
    let z = ((x - n * p0) / (n * p0 * (1.0 - p0)).sqrt()).max(0.0);
    Some(OneProportion {
        successes,
        trials,
        estimate: x / n,
        hypothesized: p0,
        z,
        p_normal: z_p(z, tail),
        p_exact: binomial_p(successes, trials, p0, tail),
        tail,
    })
}

/// The Wald (normal approximation) interval `p +- z sqrt(p (1 - p) / n)` of the confidence level `level`
/// (0 to 1), cut to `[0, 1]`. One-sided: the upper-tailed test gives `[lower, 1]` and the lower-tailed test
/// `[0, upper]`, each with the whole `1 - level` in its one tail (so the 95 % limit of a one-sided test is the
/// 90 % two-sided one only in name: it is 1.645 standard errors from the estimate).
pub fn wald_interval(successes: u64, trials: u64, tail: Tail, level: f64) -> (f64, f64) {
    let p = successes as f64 / trials as f64;
    let half_tail = if tail == Tail::Two { (1.0 - level) / 2.0 } else { 1.0 - level };
    let width = std_normal().inverse_cdf(1.0 - half_tail) * (p * (1.0 - p) / trials as f64).sqrt();
    match tail {
        Tail::Two => ((p - width).max(0.0), (p + width).min(1.0)),
        Tail::Upper => ((p - width).max(0.0), 1.0),
        Tail::Lower => (0.0, (p + width).min(1.0)),
    }
}

/// The Clopper-Pearson (exact binomial) interval of the confidence level `level`, one-sided as for
/// [`wald_interval`]. The two-sided interval of all successes has no upper limit and of no successes no lower
/// limit (NaN; Origin printed "--" for the first, the second was not measured: its dialog does not accept 0
/// successes).
pub fn exact_interval(successes: u64, trials: u64, tail: Tail, level: f64) -> (f64, f64) {
    let (x, n) = (successes, trials);
    match tail {
        Tail::Two => {
            let a = (1.0 - level) / 2.0;
            (if x == 0 { f64::NAN } else { cp_lower(x, n, a) }, if x == n { f64::NAN } else { cp_upper(x, n, a) })
        }
        Tail::Upper => (cp_lower(x, n, 1.0 - level), 1.0),
        Tail::Lower => (0.0, cp_upper(x, n, 1.0 - level)),
    }
}

/// What the test of two proportions reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoProportions {
    /// `(successes, trials)` of each sample.
    pub first: (u64, u64),
    pub second: (u64, u64),
    /// The estimate `p1 - p2`.
    pub estimate: f64,
    pub hypothesized: f64,
    /// Normal approximation `(p1 - p2 - d0) / se`; `se` is the pooled one (`sqrt(p (1 - p) (1/n1 + 1/n2))`,
    /// `p = (x1 + x2) / (n1 + n2)`) when `pooled` and the hypothesized difference is 0, else the unpooled
    /// `sqrt(p1 (1 - p1) / n1 + p2 (1 - p2) / n2)`. 0 when that is 0 (equal all-or-nothing samples).
    pub z: f64,
    pub p_normal: f64,
    /// Fisher's exact p-value (hypergeometric); two-sided: every table no more likely than the observed one.
    /// Only given for the hypothesized difference 0, `None` otherwise.
    pub p_fisher: Option<f64>,
    pub tail: Tail,
}

/// Test of the difference of two proportions, `(successes, trials)` each, against `difference` (first minus
/// second). Needs both sample sizes above 0, no more successes than trials and `|difference| < 1`.
/// Measured against Origin: the pooled z and the two-sided p, the unpooled Wald interval of the difference
/// ([`wald_difference_interval`]). Not measured: the unpooled z, a hypothesized difference other than 0, a
/// negative z (the one-sample test clips it to 0; here it is kept), the one-sided p-values and Fisher's test.
pub fn proportion_two(
    first: (u64, u64),
    second: (u64, u64),
    difference: f64,
    pooled: bool,
    tail: Tail,
) -> Option<TwoProportions> {
    let ((x1, n1), (x2, n2)) = (first, second);
    if n1 == 0 || n2 == 0 || x1 > n1 || x2 > n2 || !(difference.abs() < 1.0) {
        return None;
    }
    let (f1, f2, g1, g2) = (x1 as f64, x2 as f64, n1 as f64, n2 as f64);
    let (p1, p2) = (f1 / g1, f2 / g2);
    let se = if pooled && difference == 0.0 {
        let p = (f1 + f2) / (g1 + g2);
        (p * (1.0 - p) * (1.0 / g1 + 1.0 / g2)).sqrt()
    } else {
        (p1 * (1.0 - p1) / g1 + p2 * (1.0 - p2) / g2).sqrt()
    };
    let diff = p1 - p2;
    let z = if se > 0.0 { (diff - difference) / se } else { 0.0 };
    Some(TwoProportions {
        first,
        second,
        estimate: diff,
        hypothesized: difference,
        z,
        p_normal: z_p(z, tail),
        p_fisher: (difference == 0.0).then(|| fisher_p(first, second, tail)),
        tail,
    })
}

/// Fisher's exact p-value of the 2 x 2 table with `x1` of `n1` and `x2` of `n2` successes: the number of
/// successes of the first sample is hypergeometric given the margins; upper tail = the first sample has more.
fn fisher_p(first: (u64, u64), second: (u64, u64), tail: Tail) -> f64 {
    let ((x1, n1), (x2, n2)) = (first, second);
    let (k, total) = (x1 + x2, n1 + n2);
    let (lo, hi) = (k.saturating_sub(n2), k.min(n1));
    let ln_total = ln_choose(total, k);
    let pmf: Vec<f64> = (lo..=hi).map(|i| (ln_choose(n1, i) + ln_choose(n2, k - i) - ln_total).exp()).collect();
    let at = (x1 - lo) as usize;
    let below: f64 = pmf[..=at].iter().sum::<f64>().min(1.0);
    let above: f64 = pmf[at..].iter().sum::<f64>().min(1.0);
    match tail {
        Tail::Upper => above,
        Tail::Lower => below,
        Tail::Two => {
            let observed = pmf[at];
            pmf.iter().filter(|p| **p <= observed * (1.0 + 1e-7)).sum::<f64>().min(1.0)
        }
    }
}

/// The Wald interval of the difference `p1 - p2`, `diff +- z sqrt(p1 (1 - p1) / n1 + p2 (1 - p2) / n2)`,
/// level `level` (0 to 1), one-sided as [`wald_interval`] (`[lower, 1]` for the upper-tailed test, `[-1, upper]`
/// for the lower-tailed one; the one-sided limits are not measured), cut to `[-1, 1]`.
pub fn wald_difference_interval(first: (u64, u64), second: (u64, u64), tail: Tail, level: f64) -> (f64, f64) {
    let ((x1, n1), (x2, n2)) = (first, second);
    let (p1, p2) = (x1 as f64 / n1 as f64, x2 as f64 / n2 as f64);
    let half_tail = if tail == Tail::Two { (1.0 - level) / 2.0 } else { 1.0 - level };
    let se = (p1 * (1.0 - p1) / n1 as f64 + p2 * (1.0 - p2) / n2 as f64).sqrt();
    let width = std_normal().inverse_cdf(1.0 - half_tail) * se;
    let d = p1 - p2;
    match tail {
        Tail::Two => ((d - width).max(-1.0), (d + width).min(1.0)),
        Tail::Upper => ((d - width).max(-1.0), 1.0),
        Tail::Lower => (-1.0, (d + width).min(1.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs().max(1e-300)
    }

    /// df = 2: chi-square cdf is `1 - exp(-x/2)`, so every number has a closed form.
    #[test]
    fn one_sample_variance_with_two_degrees_of_freedom() {
        let r = vartest_one(&[1.0, 2.0, 3.0], 0.5, Tail::Two, 0.05).unwrap();
        assert!(close(r.estimate, 1.0, 1e-12) && close(r.statistic, 4.0, 1e-12) && r.df.0 == 2.0);
        assert!(close(r.p, 2.0 * (-2.0_f64).exp(), 1e-9), "{}", r.p);
        let (q_hi, q_lo) = (-2.0 * 0.025_f64.ln(), -2.0 * 0.975_f64.ln());
        assert!(close(r.lcl, 2.0 / q_hi, 1e-6), "{}", r.lcl);
        assert!(close(r.ucl, 2.0 / q_lo, 1e-6), "{}", r.ucl);
        let up = vartest_one(&[1.0, 2.0, 3.0], 0.5, Tail::Upper, 0.05).unwrap();
        let low = vartest_one(&[1.0, 2.0, 3.0], 0.5, Tail::Lower, 0.05).unwrap();
        assert!(close(up.p, (-2.0_f64).exp(), 1e-9) && close(up.p + low.p, 1.0, 1e-12));
        assert!(close(up.level, 0.9, 1e-12));
    }

    /// F(2, 2) has cdf `x / (1 + x)`.
    #[test]
    fn two_sample_variance_with_two_and_two_degrees_of_freedom() {
        let r = vartest_two(&[1.0, 2.0, 3.0], &[1.0, 3.0, 5.0], 1.0, Tail::Two, 0.05).unwrap();
        assert!(close(r.statistic, 0.25, 1e-12) && r.df == (2.0, 2.0));
        assert!(close(r.p, 0.4, 1e-9), "{}", r.p);
        assert!(close(r.lcl, 0.25 / 39.0, 1e-6) && close(r.ucl, 0.25 * 39.0, 1e-6), "{} {}", r.lcl, r.ucl);
        let low = vartest_two(&[1.0, 2.0, 3.0], &[1.0, 3.0, 5.0], 1.0, Tail::Lower, 0.05).unwrap();
        assert!(close(low.p, 0.2, 1e-9));
    }

    #[test]
    fn swapping_the_samples_inverts_f_and_keeps_the_two_sided_p() {
        let a = [3.1, 4.7, 2.2, 5.9, 4.0, 3.3];
        let b = [10.0, 7.5, 12.25, 9.0, 8.0, 11.0, 6.5];
        let ab = vartest_two(&a, &b, 1.0, Tail::Two, 0.05).unwrap();
        let ba = vartest_two(&b, &a, 1.0, Tail::Two, 0.05).unwrap();
        assert!(close(ab.statistic * ba.statistic, 1.0, 1e-12));
        assert!(close(ab.p, ba.p, 1e-9));
        assert!(close(ab.lcl * ba.ucl, 1.0, 1e-6) && close(ab.ucl * ba.lcl, 1.0, 1e-6));
    }

    #[test]
    fn variance_tests_refuse_bad_input_without_panicking() {
        assert!(vartest_one(&[1.0], 1.0, Tail::Two, 0.05).is_none());
        assert!(vartest_one(&[1.0, 2.0], 0.0, Tail::Two, 0.05).is_none());
        assert!(vartest_one(&[1.0, 2.0], 1.0, Tail::Two, 1.0).is_none());
        assert!(vartest_two(&[3.0, 3.0], &[3.0, 3.0], 1.0, Tail::Two, 0.05).is_none());
        let flat_second = vartest_two(&[1.0, 2.0], &[3.0, 3.0], 1.0, Tail::Two, 0.05).unwrap();
        assert!(flat_second.statistic.is_nan() && flat_second.lcl == f64::INFINITY);
        let flat = vartest_one(&[2.0, 2.0, f64::NAN, 2.0], 1.0, Tail::Lower, 0.05).unwrap();
        assert_eq!((flat.samples[0].n, flat.statistic), (3, 0.0));
    }

    #[test]
    fn one_sided_intervals_have_one_finite_limit() {
        let up = variance_interval(3, 1.0, Tail::Upper, 0.9);
        assert!(up.1.is_nan() && close(up.0, 2.0 / (-2.0 * 0.1_f64.ln()), 1e-6), "{up:?}");
        let low = variance_interval(3, 1.0, Tail::Lower, 0.9);
        assert!(low.0 == 0.0 && close(low.1, 2.0 / (-2.0 * 0.9_f64.ln()), 1e-6), "{low:?}");
        let two = variance_interval(3, 1.0, Tail::Two, 0.8);
        assert!(close(two.0, up.0, 1e-9) && close(two.1, low.1, 1e-9));
    }

    #[test]
    fn one_proportion_closed_forms() {
        // 10 of 10 against 1/2: exact upper p = 2^-10, two-sided twice that.
        let up = proportion_one(10, 10, 0.5, Tail::Upper).unwrap();
        assert!(close(up.p_exact, 1.0 / 1024.0, 1e-9), "{}", up.p_exact);
        let two = proportion_one(10, 10, 0.5, Tail::Two).unwrap();
        assert!(close(two.p_exact, 2.0 / 1024.0, 1e-9));
        // Clopper-Pearson at the ends: 0.025^(1/n) and 1 - that.
        let cp = exact_interval(10, 10, Tail::Two, 0.95);
        assert!(close(cp.0, 0.025_f64.powf(0.1), 1e-9) && cp.1.is_nan(), "{cp:?}");
        let zero = exact_interval(0, 10, Tail::Two, 0.95);
        assert!(zero.0.is_nan() && close(zero.1, 1.0 - 0.025_f64.powf(0.1), 1e-9), "{zero:?}");
        // 60 of 100 against 1/2: z = 2; below n p0 the z is 0.
        let z = proportion_one(60, 100, 0.5, Tail::Two).unwrap();
        assert!(close(z.z, 2.0, 1e-12) && close(z.p_normal, 0.045_500_263_896_358_4, 1e-8), "{}", z.p_normal);
        let w = 1.959_963_984_540_054 * (0.24_f64 / 100.0).sqrt();
        let wald = wald_interval(60, 100, Tail::Two, 0.95);
        assert!(close(wald.0, 0.6 - w, 1e-9) && close(wald.1, 0.6 + w, 1e-9));
        let below = proportion_one(40, 100, 0.5, Tail::Upper).unwrap();
        assert_eq!((below.z, below.p_normal), (0.0, 0.5));
    }

    #[test]
    fn clopper_pearson_brackets_the_estimate_and_matches_the_tails() {
        let (x, n) = (7_u64, 23_u64);
        let (lo, hi) = exact_interval(x, n, Tail::Two, 0.95);
        assert!(lo < 7.0 / 23.0 && 7.0 / 23.0 < hi);
        assert!(close(binomial_p(x, n, lo, Tail::Upper), 0.025, 1e-6));
        assert!(close(binomial_p(x, n, hi, Tail::Lower), 0.025, 1e-6));
    }

    #[test]
    fn two_proportions_pooled_and_separate() {
        let pooled = proportion_two((30, 50), (20, 50), 0.0, true, Tail::Two).unwrap();
        assert!(close(pooled.z, 2.0, 1e-12) && close(pooled.p_normal, 0.045_500_263_896_358_4, 1e-8));
        let separate = proportion_two((30, 50), (20, 50), 0.0, false, Tail::Two).unwrap();
        assert!(close(separate.z, 0.2 / 0.0096_f64.sqrt(), 1e-12));
        let fisher = pooled.p_fisher.unwrap();
        assert!(fisher > 0.0 && fisher < 1.0);
        let up = proportion_two((30, 50), (20, 50), 0.0, true, Tail::Upper).unwrap().p_fisher.unwrap();
        assert!(up < fisher);
        assert!(proportion_two((30, 50), (20, 50), 0.1, true, Tail::Two).unwrap().p_fisher.is_none());
        assert!(proportion_two((0, 0), (0, 10), 0.0, true, Tail::Two).is_none());
        assert!(proportion_two((11, 10), (0, 10), 0.0, true, Tail::Two).is_none());
    }

    /// Fisher on the table [[3, 1], [1, 3]]: the counts of the first sample are hypergeometric
    /// (1, 16, 36, 16, 1) / 70.
    #[test]
    fn fisher_exact_p_of_a_small_table() {
        let t = proportion_two((3, 4), (1, 4), 0.0, true, Tail::Two).unwrap();
        assert!(close(t.p_fisher.unwrap(), 34.0 / 70.0, 1e-9), "{:?}", t.p_fisher);
        let up = proportion_two((3, 4), (1, 4), 0.0, true, Tail::Upper).unwrap();
        assert!(close(up.p_fisher.unwrap(), 17.0 / 70.0, 1e-9));
    }
}
