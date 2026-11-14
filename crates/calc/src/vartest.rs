//! Tests for variance and proportions: Origin's Statistics > Hypothesis Testing > One-Sample Test for
//! Variance (`vartest1`), Two-Sample Test for Variance (`vartest2`), One-Sample Test for Proportion and
//! Two-Sample Test for Proportions.
//!
//! NOT YET MEASURED AGAINST ORIGIN (written on 2026-11-14 in a session that could neither build nor run
//! Origin; see docs/tasks/T26-variance-proportion-tests.md). The formulas are the textbook ones:
//!
//! - one sample, variance: `chi2 = (n - 1) s^2 / sigma0^2` with `n - 1` degrees of freedom; the interval of
//!   the variance is `[(n - 1) s^2 / chi2(1 - a/2), (n - 1) s^2 / chi2(a/2)]`;
//! - two samples, variance: `F = (s1^2 / s2^2) / ratio0` with `(n1 - 1, n2 - 1)` degrees of freedom; the
//!   interval of the ratio is `[r / F(1 - a/2), r / F(a/2)]`;
//! - a two-sided p-value of these asymmetric statistics is `2 min(P(X <= x), P(X >= x))`, at most 1;
//! - one proportion: `z = (x - n p0) / sqrt(n p0 (1 - p0))`, the exact binomial p-value (two-sided: the
//!   probability of every count no more likely than the observed one, as R's `binom.test`), the Wald
//!   interval `p +- z sqrt(p (1 - p) / n)` and the Clopper-Pearson (exact) interval;
//! - two proportions: `z = (p1 - p2 - d0) / se`, `se` from the pooled proportion (only when `d0 = 0`) or
//!   from the two separate ones, Fisher's exact p-value of the 2 x 2 table, the Wald interval of `p1 - p2`.
//!
//! As in [`crate::hypothesis`], a one-sided test gives the interval of level `1 - 2 alpha` (Origin's rule
//! for `ttest1`, assumed here until measured).

use statrs::distribution::{ChiSquared, ContinuousCDF, FisherSnedecor, Normal};
use statrs::function::beta::beta_reg;
use statrs::function::gamma::ln_gamma;

use crate::crosstab::Crosstab;
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

fn alpha_ok(alpha: f64) -> bool {
    alpha > 0.0 && alpha < 0.5
}

/// One-sample chi-square test of the variance of `data` against `var0`. NaN entries are ignored. Needs at
/// least two numbers, `var0 > 0` and `0 < alpha < 0.5`.
pub fn vartest_one(data: &[f64], var0: f64, tail: Tail, alpha: f64) -> Option<VarTest> {
    let s = SampleStats::of(data);
    if s.n < 2 || !(var0 > 0.0 && var0.is_finite()) || !alpha_ok(alpha) {
        return None;
    }
    let df = s.n as f64 - 1.0;
    let var = s.sd * s.sd;
    let chi = df * var / var0;
    if !chi.is_finite() {
        return None;
    }
    let dist = ChiSquared::new(df).ok()?;
    let p = p_of(dist.cdf(chi), dist.sf(chi), tail);
    let level = level_of(tail, alpha);
    let half = (1.0 - level) / 2.0;
    let (lcl, ucl) = (df * var / dist.inverse_cdf(1.0 - half), df * var / dist.inverse_cdf(half));
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

/// Two-sample F test of `var(a) / var(b)` against `ratio0` (1 tests equal variances). NaN entries are
/// ignored. Needs two numbers in each sample, a second sample with some spread, `ratio0 > 0` and
/// `0 < alpha < 0.5`.
pub fn vartest_two(a: &[f64], b: &[f64], ratio0: f64, tail: Tail, alpha: f64) -> Option<VarTest> {
    let (s1, s2) = (SampleStats::of(a), SampleStats::of(b));
    if s1.n < 2 || s2.n < 2 || !(ratio0 > 0.0 && ratio0.is_finite()) || !alpha_ok(alpha) {
        return None;
    }
    let (df1, df2) = (s1.n as f64 - 1.0, s2.n as f64 - 1.0);
    let ratio = (s1.sd * s1.sd) / (s2.sd * s2.sd);
    let f = ratio / ratio0;
    if !f.is_finite() {
        return None;
    }
    let dist = FisherSnedecor::new(df1, df2).ok()?;
    let p = p_of(dist.cdf(f), dist.sf(f), tail);
    let level = level_of(tail, alpha);
    let half = (1.0 - level) / 2.0;
    let (lcl, ucl) = (ratio / dist.inverse_cdf(1.0 - half), ratio / dist.inverse_cdf(half));
    Some(VarTest {
        samples: vec![s1, s2],
        estimate: ratio,
        hypothesized: ratio0,
        statistic: f,
        df: (df1, df2),
        p,
        lcl,
        ucl,
        level,
        tail,
        alpha,
    })
}

/// What a test of one proportion, or of the difference of two, reports. Both the normal-approximation
/// and the exact p-value are given; which of them Origin shows by default is still to be measured.
#[derive(Debug, Clone, PartialEq)]
pub struct ProportionTest {
    /// Successes and trials of each sample (one or two).
    pub successes: Vec<u64>,
    pub trials: Vec<u64>,
    /// The sample proportion `x / n`, or the difference `x1 / n1 - x2 / n2`.
    pub estimate: f64,
    /// The hypothesized proportion, or the hypothesized difference.
    pub hypothesized: f64,
    /// Normal approximation: the z statistic and its p-value.
    pub z: f64,
    pub p_normal: f64,
    /// The exact p-value: binomial (one sample) or Fisher's (two samples; `None` where the 2 x 2 table has an
    /// empty margin, or the hypothesized difference is not 0).
    pub p_exact: Option<f64>,
    /// Wald interval of the estimate.
    pub wald: (f64, f64),
    /// Clopper-Pearson interval of the proportion (one sample only).
    pub exact_interval: Option<(f64, f64)>,
    pub level: f64,
    pub tail: Tail,
    pub alpha: f64,
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

/// Clopper-Pearson interval of `x / n` at confidence level `level`.
fn clopper_pearson(x: u64, n: u64, level: f64) -> (f64, f64) {
    let half = (1.0 - level) / 2.0;
    let (xf, nf) = (x as f64, n as f64);
    // P(X >= x | p) = I_p(x, n - x + 1) grows with p; the lower limit is where it reaches `half`.
    let lower = if x == 0 { 0.0 } else { solve_increasing(|p| beta_reg(xf, nf - xf + 1.0, p), half) };
    // P(X <= x | p) = 1 - I_p(x + 1, n - x) falls with p; the upper limit is where it falls to `half`.
    let upper = if x == n { 1.0 } else { solve_increasing(|p| beta_reg(xf + 1.0, nf - xf, p), 1.0 - half) };
    (lower, upper)
}

/// Test of the proportion of `successes` in `trials` against `p0`. No successes and all successes are
/// allowed; needs `trials > 0`, `successes <= trials`, `0 < p0 < 1` and `0 < alpha < 0.5`.
pub fn proportion_one(successes: u64, trials: u64, p0: f64, tail: Tail, alpha: f64) -> Option<ProportionTest> {
    if trials == 0 || successes > trials || !(p0 > 0.0 && p0 < 1.0) || !alpha_ok(alpha) {
        return None;
    }
    let (x, n) = (successes as f64, trials as f64);
    let phat = x / n;
    let z = (x - n * p0) / (n * p0 * (1.0 - p0)).sqrt();
    let level = level_of(tail, alpha);
    let crit = std_normal().inverse_cdf(1.0 - (1.0 - level) / 2.0);
    let half_width = crit * (phat * (1.0 - phat) / n).sqrt();
    Some(ProportionTest {
        successes: vec![successes],
        trials: vec![trials],
        estimate: phat,
        hypothesized: p0,
        z,
        p_normal: z_p(z, tail),
        p_exact: Some(binomial_p(successes, trials, p0, tail)),
        wald: (phat - half_width, phat + half_width),
        exact_interval: Some(clopper_pearson(successes, trials, level)),
        level,
        tail,
        alpha,
    })
}

/// Test of `x1 / n1 - x2 / n2` against `diff0`, each sample given as `(successes, trials)`. With `pooled`
/// (and `diff0 = 0`) the standard error of the z statistic comes from the pooled proportion
/// `(x1 + x2) / (n1 + n2)`; otherwise from the two separate proportions. The Wald interval always uses the
/// separate proportions.
pub fn proportion_two(
    first: (u64, u64),
    second: (u64, u64),
    diff0: f64,
    pooled: bool,
    tail: Tail,
    alpha: f64,
) -> Option<ProportionTest> {
    let ((x1, n1), (x2, n2)) = (first, second);
    if n1 == 0 || n2 == 0 || x1 > n1 || x2 > n2 || !(diff0 > -1.0 && diff0 < 1.0) || !alpha_ok(alpha) {
        return None;
    }
    let (p1, p2) = (x1 as f64 / n1 as f64, x2 as f64 / n2 as f64);
    let (m1, m2) = (n1 as f64, n2 as f64);
    let separate = (p1 * (1.0 - p1) / m1 + p2 * (1.0 - p2) / m2).sqrt();
    let se = if pooled && diff0 == 0.0 {
        let pp = (x1 + x2) as f64 / (m1 + m2);
        (pp * (1.0 - pp) * (1.0 / m1 + 1.0 / m2)).sqrt()
    } else {
        separate
    };
    let z = (p1 - p2 - diff0) / se;
    if z.is_nan() {
        return None;
    }
    let level = level_of(tail, alpha);
    let crit = std_normal().inverse_cdf(1.0 - (1.0 - level) / 2.0);
    let p_exact = if diff0 == 0.0 {
        Crosstab::new(vec![vec![x1 as usize, (n1 - x1) as usize], vec![x2 as usize, (n2 - x2) as usize]])
            .ok()
            .and_then(|t| t.fisher())
            .map(|f| match tail {
                Tail::Two => f.p_two_sided,
                Tail::Upper => f.p_greater,
                Tail::Lower => f.p_less,
            })
    } else {
        None
    };
    let d = p1 - p2;
    Some(ProportionTest {
        successes: vec![x1, x2],
        trials: vec![n1, n2],
        estimate: d,
        hypothesized: diff0,
        z,
        p_normal: z_p(z, tail),
        p_exact,
        wald: (d - crit * separate, d + crit * separate),
        exact_interval: None,
        level,
        tail,
        alpha,
    })
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
        assert!(vartest_one(&[1.0, 2.0], 1.0, Tail::Two, 0.7).is_none());
        assert!(vartest_two(&[1.0, 2.0], &[3.0, 3.0], 1.0, Tail::Two, 0.05).is_none());
        let flat = vartest_one(&[2.0, 2.0, f64::NAN, 2.0], 1.0, Tail::Lower, 0.05).unwrap();
        assert_eq!((flat.samples[0].n, flat.statistic), (3, 0.0));
    }

    #[test]
    fn one_proportion_closed_forms() {
        // 10 of 10 against 1/2: exact upper p = 2^-10, two-sided twice that.
        let r = proportion_one(10, 10, 0.5, Tail::Upper, 0.05).unwrap();
        assert!(close(r.p_exact.unwrap(), 1.0 / 1024.0, 1e-9), "{:?}", r.p_exact);
        let two = proportion_one(10, 10, 0.5, Tail::Two, 0.05).unwrap();
        assert!(close(two.p_exact.unwrap(), 2.0 / 1024.0, 1e-9));
        // Clopper-Pearson at the ends: (0.025)^(1/n) and 1 - that.
        let cp = two.exact_interval.unwrap();
        assert!(close(cp.0, 0.025_f64.powf(0.1), 1e-9) && cp.1 == 1.0, "{cp:?}");
        let zero = proportion_one(0, 10, 0.5, Tail::Two, 0.05).unwrap().exact_interval.unwrap();
        assert!(zero.0 == 0.0 && close(zero.1, 1.0 - 0.025_f64.powf(0.1), 1e-9), "{zero:?}");
        // 60 of 100 against 1/2: z = 2.
        let z = proportion_one(60, 100, 0.5, Tail::Two, 0.05).unwrap();
        assert!(close(z.z, 2.0, 1e-12) && close(z.p_normal, 0.045_500_263_896_358_4, 1e-8), "{}", z.p_normal);
        let w = 1.959_963_984_540_054 * (0.24_f64 / 100.0).sqrt();
        assert!(close(z.wald.0, 0.6 - w, 1e-9) && close(z.wald.1, 0.6 + w, 1e-9));
    }

    #[test]
    fn clopper_pearson_brackets_the_estimate_and_matches_the_tails() {
        let (x, n) = (7_u64, 23_u64);
        let (lo, hi) = clopper_pearson(x, n, 0.95);
        assert!(lo < 7.0 / 23.0 && 7.0 / 23.0 < hi);
        assert!(close(binomial_p(x, n, lo, Tail::Upper), 0.025, 1e-6));
        assert!(close(binomial_p(x, n, hi, Tail::Lower), 0.025, 1e-6));
    }

    #[test]
    fn two_proportions_pooled_and_separate() {
        let pooled = proportion_two((30, 50), (20, 50), 0.0, true, Tail::Two, 0.05).unwrap();
        assert!(close(pooled.z, 2.0, 1e-12) && close(pooled.p_normal, 0.045_500_263_896_358_4, 1e-8));
        let separate = proportion_two((30, 50), (20, 50), 0.0, false, Tail::Two, 0.05).unwrap();
        assert!(close(separate.z, 0.2 / 0.0096_f64.sqrt(), 1e-12));
        let fisher = pooled.p_exact.unwrap();
        assert!(fisher > 0.0 && fisher < 1.0);
        let up = proportion_two((30, 50), (20, 50), 0.0, true, Tail::Upper, 0.05).unwrap().p_exact.unwrap();
        assert!(up < fisher);
        assert!(proportion_two((30, 50), (20, 50), 0.1, true, Tail::Two, 0.05).unwrap().p_exact.is_none());
        assert!(proportion_two((0, 10), (0, 10), 0.0, true, Tail::Two, 0.05).is_none());
        assert!(proportion_two((11, 10), (0, 10), 0.0, true, Tail::Two, 0.05).is_none());
    }
}
