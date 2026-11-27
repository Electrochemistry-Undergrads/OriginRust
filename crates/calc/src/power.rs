//! Power and sample size of the t tests and the one-way ANOVA (Origin's Statistics > Power and Sample Size).
//!
//! Power is the textbook one: the probability that the test statistic lands in the rejection region when it
//! follows the noncentral distribution that belongs to the effect. The sample size is the smallest whole number
//! of observations (per group) whose power reaches the wanted power. The rules and the numbers Origin gave are
//! in `docs/spec/statistics.md` section 12.
//!
//! **Proportions and variances** (T415; Sheaf's own, not measured in Origin): the tests Sheaf runs in section 21.
//! One proportion, the normal-approximation z test with the standard error at the hypothesized proportion: two-sided
//! power `Phi((|p - p0| sqrt n - z sqrt(p0 q0)) / sqrt(p q)) + Phi((-|p - p0| sqrt n - z sqrt(p0 q0)) / sqrt(p q))`
//! with `z` the `1 - alpha/2` normal point, one-sided only the first term with `z` at `1 - alpha`. Two proportions of
//! `n` each, the unpooled z test: the same with `sqrt(p1 q1 + p2 q2)` in both places. One variance, the chi-square
//! test with `n - 1` df: under the alternative `(n - 1) s^2 / sigma0^2` is `r` times a chi-square, `r = sigma^2 /
//! sigma0^2`, so the power is `F(c_lo / r) + 1 - F(c_hi / r)` at the test's critical points. Two variances of `n`
//! each, the F test with `(n - 1, n - 1)` df and `r = sigma1^2 / sigma2^2`, the same way. One-sided tests look in
//! the direction of the effect, as for the t tests.
//!
//! **Options** (T500, [`RateOptions`]): unequal groups, `n2 = ratio n1` (`n` is the first group's size; not rounded),
//! for the two-sample tests: unpooled `se = sqrt(p1 q1 / n1 + p2 q2 / n2)` in both places, F with `(n1 - 1, n2 - 1)`
//! df. The pooled two-proportion z test takes the standard error under the null from the pooled proportion `pbar =
//! (n1 p1 + n2 p2) / (n1 + n2)`: power `Phi((|d| - z se0) / se1) [+ Phi((-|d| - z se0) / se1)]` with `se0 = sqrt(pbar
//! qbar (1/n1 + 1/n2))` and `se1` the unpooled one (statsmodels' `power_proportions_2indep`). The exact one-proportion
//! test (the binomial test of section 21: two-sided, every count no more likely than the observed one under `p0`,
//! within a relative 1e-7; one-sided, the tail in the direction of the effect) has the power `sum_x P(x | n, p)` over
//! the counts it rejects at `alpha` (p-value at most `alpha`); it is not monotone in `n`, so its sample size is the
//! first `n` from 2 whose power reaches the target (scipy's `binomtest` rule).

use statrs::distribution::{ChiSquared, ContinuousCDF, FisherSnedecor, Normal, StudentsT};
use statrs::function::beta::beta_reg;
use statrs::function::gamma::ln_gamma;

/// Which t test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerTest {
    /// One sample: the mean against a hypothesized value; `n` observations, `n - 1` degrees of freedom.
    OneSample,
    /// Two samples of the same size `n` with a pooled standard deviation; `2 n - 2` degrees of freedom.
    TwoSample,
    /// Paired samples: the standard deviation is that of the differences, so it is the one-sample test.
    Paired,
}

/// Why a power or sample size could not be computed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerError {
    /// A sample size below 2, a standard deviation that is not positive, or a level outside (0, 1).
    BadInput,
    /// The means (or the treatment sum of squares) are equal: no sample size reaches a power above alpha.
    NoEffect,
    /// The wanted power is not above the power of the smallest sample.
    BelowLowest { lowest: f64 },
    /// The wanted power is outside (0, 1).
    BadPower,
}

impl std::fmt::Display for PowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerError::BadInput => {
                write!(f, "the sample sizes, standard deviation and significance level must be valid")
            }
            PowerError::NoEffect => write!(f, "the effect is zero, so no sample size reaches the wanted power"),
            PowerError::BelowLowest { lowest } => {
                write!(f, "the wanted power is not above the power of the smallest sample ({lowest})")
            }
            PowerError::BadPower => write!(f, "the wanted power must lie between 0 and 1"),
        }
    }
}

/// Distribution function of the noncentral t distribution (Lenth's algorithm AS 243).
pub fn pnt(t: f64, df: f64, ncp: f64) -> f64 {
    if t.is_nan() || df.is_nan() || ncp.is_nan() || df <= 0.0 {
        return f64::NAN;
    }
    if ncp == 0.0 {
        return StudentsT::new(0.0, 1.0, df).map(|d| d.cdf(t)).unwrap_or(f64::NAN);
    }
    let (negdel, tt, del) = if t >= 0.0 { (false, t, ncp) } else { (true, -t, -ncp) };
    let norm = Normal::new(0.0, 1.0).expect("standard normal");
    if df > 4e5 || del * del > 2.0 * std::f64::consts::LN_2 * 1021.0 {
        // Large df or ncp: the normal approximation of Abramowitz & Stegun 26.7.10.
        let s = 1.0 / (4.0 * df);
        let v = norm.cdf((tt * (1.0 - s) - del) / (1.0 + tt * tt * 2.0 * s).sqrt());
        return if negdel { 1.0 - v } else { v };
    }
    let x = tt * tt / (tt * tt + df);
    let mut tnc;
    if x > 0.0 {
        let lambda = del * del;
        let mut p = 0.5 * (-0.5 * lambda).exp();
        let mut q = (2.0 / std::f64::consts::PI).sqrt() * p * del;
        let mut s = 0.5 - p;
        if s < 1e-7 {
            s = -0.5 * (-0.5 * lambda).exp_m1();
        }
        let mut a = 0.5;
        let b = 0.5 * df;
        let rxb = (1.0 - x).powf(b);
        let albeta = 0.5 * std::f64::consts::PI.ln() + ln_gamma(b) - ln_gamma(0.5 + b);
        let mut xodd = beta_reg(a, b, x);
        let mut godd = 2.0 * rxb * (a * x.ln() - albeta).exp();
        tnc = b * x;
        let mut xeven = if tnc < f64::EPSILON { tnc } else { 1.0 - rxb };
        let mut geven = tnc * rxb;
        tnc = p * xodd + q * xeven;
        for it in 1..=2000 {
            a += 1.0;
            xodd -= godd;
            xeven -= geven;
            godd *= x * (a + b - 1.0) / a;
            geven *= x * (a + b - 0.5) / (a + 0.5);
            p *= lambda / (2.0 * it as f64);
            q *= lambda / (2.0 * it as f64 + 1.0);
            tnc += p * xodd + q * xeven;
            s -= p;
            if s < -1e-10 {
                break;
            }
            if s <= 0.0 && it > 1 {
                break;
            }
            let errbd = 2.0 * s * (xodd - godd);
            if errbd.abs() < 1e-14 {
                break;
            }
        }
    } else {
        tnc = 0.0;
    }
    tnc += norm.cdf(-del);
    let v = if negdel { 1.0 - tnc } else { tnc };
    v.clamp(0.0, 1.0)
}

/// Distribution function of the noncentral F distribution with numerator and denominator degrees of freedom
/// `df1`, `df2` and noncentrality `ncp`: a Poisson mixture of regularized incomplete beta functions.
pub fn pnf(f: f64, df1: f64, df2: f64, ncp: f64) -> f64 {
    if f.is_nan() || df1 <= 0.0 || df2 <= 0.0 || ncp < 0.0 {
        return f64::NAN;
    }
    if f <= 0.0 {
        return 0.0;
    }
    if ncp == 0.0 {
        return FisherSnedecor::new(df1, df2).map(|d| d.cdf(f)).unwrap_or(f64::NAN);
    }
    let x = df1 * f / (df1 * f + df2);
    let half = ncp / 2.0;
    // Start at the mode of the Poisson weights and sum outwards.
    let mode = half.floor();
    let term = |j: f64| (j * half.ln() - half - ln_gamma(j + 1.0)).exp() * beta_reg(df1 / 2.0 + j, df2 / 2.0, x);
    let mut sum = term(mode);
    let mut j = mode + 1.0;
    while j < mode + 100_000.0 {
        let t = term(j);
        sum += t;
        if t < 1e-17 * sum.max(1e-300) && j > half {
            break;
        }
        j += 1.0;
    }
    let mut j = mode - 1.0;
    while j >= 0.0 {
        let t = term(j);
        sum += t;
        if t < 1e-17 * sum.max(1e-300) && j < half {
            break;
        }
        j -= 1.0;
    }
    sum.clamp(0.0, 1.0)
}

/// Power of a t test with `n` observations (per group for the two-sample test).
///
/// `effect` is the difference of the means (or of the mean and the hypothesized value), `sd` the standard
/// deviation (pooled for two samples, of the differences for paired samples). The one-sided test is the upper
/// one.
pub fn t_power(test: PowerTest, n: f64, effect: f64, sd: f64, alpha: f64, two_sided: bool) -> Result<f64, PowerError> {
    if n.is_nan() || n < 2.0 || sd.is_nan() || sd <= 0.0 || !(alpha > 0.0 && alpha < 1.0) || !effect.is_finite() {
        return Err(PowerError::BadInput);
    }
    let (df, ncp) = match test {
        PowerTest::OneSample | PowerTest::Paired => (n - 1.0, effect / sd * n.sqrt()),
        PowerTest::TwoSample => (2.0 * n - 2.0, effect / sd * (n / 2.0).sqrt()),
    };
    let dist = StudentsT::new(0.0, 1.0, df).map_err(|_| PowerError::BadInput)?;
    Ok(if two_sided {
        let tc = dist.inverse_cdf(1.0 - alpha / 2.0);
        (1.0 - pnt(tc, df, ncp)) + pnt(-tc, df, ncp)
    } else {
        let tc = dist.inverse_cdf(1.0 - alpha);
        1.0 - pnt(tc, df, ncp)
    })
}

/// Power of the one-way ANOVA F test with `n` observations in each of `levels` groups; `css` is the corrected
/// sum of squares of the group means (the sum of squared deviations of the means from their mean) and `sd` the
/// root of the error mean square. The noncentrality is `n css / sd^2`.
pub fn anova_power(n: f64, levels: f64, css: f64, sd: f64, alpha: f64) -> Result<f64, PowerError> {
    if n.is_nan()
        || n < 2.0
        || levels.is_nan()
        || levels < 2.0
        || sd.is_nan()
        || sd <= 0.0
        || !(alpha > 0.0 && alpha < 1.0)
        || css.is_nan()
        || css < 0.0
    {
        return Err(PowerError::BadInput);
    }
    let (df1, df2) = (levels - 1.0, levels * (n - 1.0));
    let fc = FisherSnedecor::new(df1, df2).map_err(|_| PowerError::BadInput)?.inverse_cdf(1.0 - alpha);
    Ok(1.0 - pnf(fc, df1, df2, n * css / (sd * sd)))
}

/// Powers of a t test for a list of sample sizes (per group for two samples).
///
/// The sizes must be whole numbers (Origin refuses 12.5); a size below 2 gets no power (NaN, Origin leaves the
/// cell empty). The direction of the effect does not matter: Origin's one-sided test has the power of the
/// upper test for the size of the difference.
pub fn t_powers(
    test: PowerTest,
    sizes: &[f64],
    effect: f64,
    sd: f64,
    alpha: f64,
    two_sided: bool,
) -> Result<Vec<f64>, PowerError> {
    if sizes.is_empty() || sizes.iter().any(|n| n.is_nan() || n.fract() != 0.0) {
        return Err(PowerError::BadInput);
    }
    sizes
        .iter()
        .map(|&n| {
            if n < 2.0 {
                check_t(effect, sd, alpha).map(|()| f64::NAN)
            } else {
                t_power(test, n, effect.abs(), sd, alpha, two_sided)
            }
        })
        .collect()
}

fn check_t(effect: f64, sd: f64, alpha: f64) -> Result<(), PowerError> {
    if sd.is_nan() || sd <= 0.0 || !(alpha > 0.0 && alpha < 1.0) || !effect.is_finite() {
        return Err(PowerError::BadInput);
    }
    Ok(())
}

/// Powers of the one-way ANOVA for a list of group sizes. The number of groups must be a whole number of at
/// least 2 and `css` positive; the sizes may be fractions, and a size of 1 or less has power 1 (Origin's value
/// for one observation per group, where there are no error degrees of freedom).
pub fn anova_powers(sizes: &[f64], levels: f64, css: f64, sd: f64, alpha: f64) -> Result<Vec<f64>, PowerError> {
    if sizes.is_empty()
        || sizes.iter().any(|n| n.is_nan())
        || levels.is_nan()
        || levels.fract() != 0.0
        || levels < 2.0
        || css.is_nan()
        || css <= 0.0
    {
        return Err(PowerError::BadInput);
    }
    sizes
        .iter()
        .map(|&n| if n <= 1.0 { check_anova(sd, alpha).map(|()| 1.0) } else { anova_power(n, levels, css, sd, alpha) })
        .collect()
}

fn check_anova(sd: f64, alpha: f64) -> Result<(), PowerError> {
    if sd.is_nan() || sd <= 0.0 || !(alpha > 0.0 && alpha < 1.0) {
        return Err(PowerError::BadInput);
    }
    Ok(())
}

/// Sample sizes for wanted powers.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleSizes {
    /// The smallest whole number of observations (per group) that reaches each wanted power. Where the wanted
    /// power is not above the power of two observations (`below_lowest`) it is 2 for the two-sample test and
    /// NaN for the one-sample and paired tests and the ANOVA, as in Origin.
    pub sizes: Vec<f64>,
    /// The wanted powers that two observations already exceed.
    pub below_lowest: Vec<bool>,
    /// The power of the smallest sample (two observations); `None` for the ANOVA, where Origin gives none.
    pub lowest: Option<f64>,
}

/// The largest sample size searched for.
const MAX_SIZE: f64 = 1e9;

fn smallest_size(target: f64, power_at: &dyn Fn(f64) -> f64) -> Result<f64, PowerError> {
    // Powers rise with the sample size: double until the target is reached, then bisect on whole numbers.
    let mut hi = 4.0;
    while power_at(hi) < target {
        hi *= 2.0;
        if hi > MAX_SIZE {
            return Err(PowerError::NoEffect);
        }
    }
    let mut lo = hi / 2.0;
    if lo < 2.0 {
        lo = 2.0;
    }
    if power_at(lo) >= target {
        return Ok(lo);
    }
    while hi - lo > 1.0 {
        let mid = ((lo + hi) / 2.0).floor();
        if power_at(mid) >= target {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Ok(hi)
}

fn check_powers(powers: &[f64]) -> Result<(), PowerError> {
    if powers.is_empty() || powers.iter().any(|p| p.is_nan() || *p <= 0.0 || *p >= 1.0) {
        return Err(PowerError::BadPower);
    }
    Ok(())
}

/// Smallest sample sizes of a t test that reach each wanted power.
pub fn t_sample_sizes(
    test: PowerTest,
    powers: &[f64],
    effect: f64,
    sd: f64,
    alpha: f64,
    two_sided: bool,
) -> Result<SampleSizes, PowerError> {
    check_t(effect, sd, alpha)?;
    let effect = effect.abs();
    if effect == 0.0 {
        return Err(PowerError::NoEffect);
    }
    check_powers(powers)?;
    let power_at = |n: f64| t_power(test, n, effect, sd, alpha, two_sided).unwrap_or(f64::NAN);
    let lowest = power_at(2.0);
    let mut out = SampleSizes { sizes: Vec::new(), below_lowest: Vec::new(), lowest: Some(lowest) };
    for &target in powers {
        if target <= lowest {
            out.sizes.push(if test == PowerTest::TwoSample { 2.0 } else { f64::NAN });
            out.below_lowest.push(true);
        } else {
            out.sizes.push(smallest_size(target, &power_at)?);
            out.below_lowest.push(false);
        }
    }
    Ok(out)
}

/// Smallest group sizes of the one-way ANOVA that reach each wanted power (at least 2 per group).
pub fn anova_sample_sizes(
    powers: &[f64],
    levels: f64,
    css: f64,
    sd: f64,
    alpha: f64,
) -> Result<SampleSizes, PowerError> {
    if levels.is_nan() || levels.fract() != 0.0 || levels < 2.0 || css.is_nan() || css <= 0.0 {
        return Err(PowerError::BadInput);
    }
    check_anova(sd, alpha)?;
    check_powers(powers)?;
    let power_at = |n: f64| anova_power(n, levels, css, sd, alpha).unwrap_or(f64::NAN);
    let mut out = SampleSizes { sizes: Vec::new(), below_lowest: Vec::new(), lowest: None };
    // Origin gives no lowest power for the ANOVA, but a wanted power that two observations per group already
    // reach is refused all the same (and its cell stays empty).
    let at_two = power_at(2.0);
    for &target in powers {
        if target <= at_two {
            out.sizes.push(f64::NAN);
            out.below_lowest.push(true);
        } else {
            out.sizes.push(smallest_size(target, &power_at)?);
            out.below_lowest.push(false);
        }
    }
    Ok(out)
}

/// A test on proportions or variances (T415).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateTest {
    /// One proportion `a` against the hypothesized `b` (normal approximation).
    OneProportion,
    /// Two proportions `a` and `b` with `n` each (unpooled normal approximation).
    TwoProportions,
    /// One variance `a` against the hypothesized `b` (chi-square).
    OneVariance,
    /// Two variances `a` and `b` with `n` each (F).
    TwoVariances,
}

impl RateTest {
    fn check(self, a: f64, b: f64, alpha: f64) -> Result<(), PowerError> {
        let ok = match self {
            RateTest::OneProportion | RateTest::TwoProportions => a > 0.0 && a < 1.0 && b > 0.0 && b < 1.0,
            RateTest::OneVariance | RateTest::TwoVariances => a > 0.0 && b > 0.0 && a.is_finite() && b.is_finite(),
        };
        if !ok || !(alpha > 0.0 && alpha < 1.0) {
            return Err(PowerError::BadInput);
        }
        Ok(())
    }
}

/// Variants of the proportion and variance tests (T500; see the module notes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RateOptions {
    /// The second group's size over the first's (two-sample tests); 1 for equal groups.
    pub ratio: f64,
    /// The pooled two-proportion z test.
    pub pooled: bool,
    /// The exact binomial test of one proportion.
    pub exact: bool,
}

impl Default for RateOptions {
    fn default() -> Self {
        RateOptions { ratio: 1.0, pooled: false, exact: false }
    }
}

/// Largest sample the exact one-proportion power is computed for, and searched up to.
pub const MAX_EXACT_POWER_SIZE: f64 = 20_000.0;

/// The power of the exact binomial test of `p0` with `n` trials when the proportion is `p` (T500).
fn exact_binomial_power(n: usize, p: f64, p0: f64, alpha: f64, two_sided: bool) -> f64 {
    let ln_choose = |k: usize| ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0);
    let pmf = |q: f64| -> Vec<f64> {
        let (lq, lr) = (q.ln(), (1.0 - q).ln());
        (0..=n).map(|k| (ln_choose(k) + k as f64 * lq + (n - k) as f64 * lr).exp()).collect()
    };
    let (null, alt) = (pmf(p0), pmf(p));
    let rejects: Vec<bool> = if two_sided {
        let mut sorted = null.clone();
        sorted.sort_by(f64::total_cmp);
        let mut cum = Vec::with_capacity(sorted.len());
        let mut s = 0.0;
        for v in &sorted {
            s += v;
            cum.push(s);
        }
        null.iter()
            .map(|&f| {
                let j = sorted.partition_point(|v| *v <= f * (1.0 + 1e-7));
                j > 0 && cum[j - 1].min(1.0) <= alpha
            })
            .collect()
    } else if p > p0 {
        // Upper tail: P(X >= x).
        let mut out = vec![false; n + 1];
        let mut s = 0.0;
        for x in (0..=n).rev() {
            s += null[x];
            out[x] = s.min(1.0) <= alpha;
        }
        out
    } else {
        let mut out = vec![false; n + 1];
        let mut s = 0.0;
        for x in 0..=n {
            s += null[x];
            out[x] = s.min(1.0) <= alpha;
        }
        out
    };
    alt.iter().zip(&rejects).filter(|(_, r)| **r).map(|(v, _)| v).sum::<f64>().min(1.0)
}

/// The chances that `r` times a variable of `d` falls below its `tail` point and above its `1 - tail` point.
fn beyond<D: ContinuousCDF<f64, f64>>(d: &D, tail: f64, r: f64) -> (f64, f64) {
    (d.cdf(d.inverse_cdf(tail) / r), d.sf(d.inverse_cdf(1.0 - tail) / r))
}

/// The power of a proportion or variance test with `n` observations (per group), `n` at least 2.
pub fn rate_power(test: RateTest, n: f64, a: f64, b: f64, alpha: f64, two_sided: bool) -> Result<f64, PowerError> {
    rate_power_with(test, n, a, b, alpha, two_sided, RateOptions::default())
}

/// [`rate_power`] with the options of `o` (T500): `n` is the first group's size.
pub fn rate_power_with(
    test: RateTest,
    n: f64,
    a: f64,
    b: f64,
    alpha: f64,
    two_sided: bool,
    o: RateOptions,
) -> Result<f64, PowerError> {
    test.check(a, b, alpha)?;
    if n.is_nan() || n < 2.0 || !(o.ratio.is_finite() && o.ratio > 0.0) {
        return Err(PowerError::BadInput);
    }
    let n2 = n * o.ratio;
    if matches!(test, RateTest::TwoProportions | RateTest::TwoVariances) && n2 < 2.0 {
        return Err(PowerError::BadInput);
    }
    let tail = if two_sided { alpha / 2.0 } else { alpha };
    match test {
        RateTest::OneProportion if o.exact => {
            if n > MAX_EXACT_POWER_SIZE || n.fract() != 0.0 {
                return Err(PowerError::BadInput);
            }
            Ok(exact_binomial_power(n as usize, a, b, alpha, two_sided))
        }
        RateTest::OneProportion | RateTest::TwoProportions => {
            let norm = Normal::new(0.0, 1.0).expect("standard normal");
            let z = norm.inverse_cdf(1.0 - tail);
            // The standard errors of the difference under the null and the alternative.
            let (se0, se1) = if test == RateTest::OneProportion {
                ((b * (1.0 - b) / n).sqrt(), (a * (1.0 - a) / n).sqrt())
            } else {
                let se1 = (a * (1.0 - a) / n + b * (1.0 - b) / n2).sqrt();
                let se0 = if o.pooled {
                    let pbar = (n * a + n2 * b) / (n + n2);
                    (pbar * (1.0 - pbar) * (1.0 / n + 1.0 / n2)).sqrt()
                } else {
                    se1
                };
                (se0, se1)
            };
            let d = (a - b).abs();
            let upper = norm.cdf((d - z * se0) / se1);
            Ok(if two_sided { upper + norm.cdf((-d - z * se0) / se1) } else { upper })
        }
        RateTest::OneVariance | RateTest::TwoVariances => {
            let r = a / b;
            let (below, above) = if test == RateTest::OneVariance {
                beyond(&ChiSquared::new(n - 1.0).map_err(|_| PowerError::BadInput)?, tail, r)
            } else {
                beyond(&FisherSnedecor::new(n - 1.0, n2 - 1.0).map_err(|_| PowerError::BadInput)?, tail, r)
            };
            Ok(if two_sided {
                below + above
            } else if r >= 1.0 {
                above
            } else {
                below
            })
        }
    }
}

/// Powers of a proportion or variance test for a list of sample sizes (per group for two samples): whole numbers;
/// a size below 2 gets no power (NaN), as for the t tests.
pub fn rate_powers(
    test: RateTest,
    sizes: &[f64],
    a: f64,
    b: f64,
    alpha: f64,
    two_sided: bool,
) -> Result<Vec<f64>, PowerError> {
    rate_powers_with(test, sizes, a, b, alpha, two_sided, RateOptions::default())
}

/// [`rate_powers`] with the options of `o` (T500); a size whose second group falls below 2 gets no power.
pub fn rate_powers_with(
    test: RateTest,
    sizes: &[f64],
    a: f64,
    b: f64,
    alpha: f64,
    two_sided: bool,
    o: RateOptions,
) -> Result<Vec<f64>, PowerError> {
    if sizes.is_empty() || sizes.iter().any(|n| n.is_nan() || n.fract() != 0.0) {
        return Err(PowerError::BadInput);
    }
    test.check(a, b, alpha)?;
    let two = matches!(test, RateTest::TwoProportions | RateTest::TwoVariances);
    sizes
        .iter()
        .map(|&n| {
            if n < 2.0 || (two && n * o.ratio < 2.0) {
                Ok(f64::NAN)
            } else {
                rate_power_with(test, n, a, b, alpha, two_sided, o)
            }
        })
        .collect()
}

/// Smallest sample sizes (per group) of a proportion or variance test that reach each wanted power; a wanted power
/// that two observations already reach gets no size (NaN), and the power of two observations is given.
pub fn rate_sample_sizes(
    test: RateTest,
    powers: &[f64],
    a: f64,
    b: f64,
    alpha: f64,
    two_sided: bool,
) -> Result<SampleSizes, PowerError> {
    rate_sample_sizes_with(test, powers, a, b, alpha, two_sided, RateOptions::default())
}

/// [`rate_sample_sizes`] with the options of `o` (T500): the sizes are the first group's; the smallest first group
/// is the one whose second group has at least 2.
pub fn rate_sample_sizes_with(
    test: RateTest,
    powers: &[f64],
    a: f64,
    b: f64,
    alpha: f64,
    two_sided: bool,
    o: RateOptions,
) -> Result<SampleSizes, PowerError> {
    test.check(a, b, alpha)?;
    if a == b {
        return Err(PowerError::NoEffect);
    }
    if !(o.ratio.is_finite() && o.ratio > 0.0) {
        return Err(PowerError::BadInput);
    }
    check_powers(powers)?;
    let two = matches!(test, RateTest::TwoProportions | RateTest::TwoVariances);
    let first = if two { (2.0 / o.ratio).ceil().max(2.0) } else { 2.0 };
    let power_at = |n: f64| rate_power_with(test, n, a, b, alpha, two_sided, o).unwrap_or(f64::NAN);
    let lowest = power_at(first);
    if test == RateTest::OneProportion && o.exact {
        // Not monotone: the first n from 2 whose power reaches the target.
        let mut out = SampleSizes { sizes: Vec::new(), below_lowest: Vec::new(), lowest: Some(lowest) };
        for &target in powers {
            if target <= lowest {
                out.sizes.push(f64::NAN);
                out.below_lowest.push(true);
                continue;
            }
            let mut n = 2.0;
            while n <= MAX_EXACT_POWER_SIZE && power_at(n) < target {
                n += 1.0;
            }
            if n > MAX_EXACT_POWER_SIZE {
                return Err(PowerError::NoEffect);
            }
            out.sizes.push(n);
            out.below_lowest.push(false);
        }
        return Ok(out);
    }
    let mut out = SampleSizes { sizes: Vec::new(), below_lowest: Vec::new(), lowest: Some(lowest) };
    for &target in powers {
        if target <= lowest {
            out.sizes.push(f64::NAN);
            out.below_lowest.push(true);
        } else {
            out.sizes.push(smallest_size(target, &power_at)?);
            out.below_lowest.push(false);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_pooled_and_unequal_proportion_powers_match_scipy_and_statsmodels() {
        // T500: scipy's binomtest rejection regions and statsmodels' power_proportions_2indep
        // (re-workspace/scratch/t480/power_ref.py).
        let exact = RateOptions { exact: true, ..Default::default() };
        for (n, p, p0, two, want) in [
            (20.0, 0.7, 0.5, true, 0.4164137694694344),
            (50.0, 0.3, 0.5, true, 0.7821932227280467),
            (37.0, 0.62, 0.5, false, 0.4301589873936447),
            (60.0, 0.4, 0.5, false, 0.4511063893843983),
            (200.0, 0.55, 0.5, true, 0.26198288668465874),
        ] {
            let got = rate_power_with(RateTest::OneProportion, n, p, p0, 0.05, two, exact).unwrap();
            assert!((got - want).abs() < 1e-10, "{n} {p} {p0} {two}: {got} vs {want}");
        }
        let s = rate_sample_sizes_with(RateTest::OneProportion, &[0.8], 0.7, 0.5, 0.05, true, exact).unwrap();
        assert_eq!(s.sizes, [49.0], "the first n whose exact power reaches 0.8");
        for (p1, p2, n1, ratio, two, want) in [
            (0.6, 0.4, 50.0, 1.0, true, 0.5163234210273413),
            (0.6, 0.4, 40.0, 2.0, true, 0.5447098732588368),
            (0.3, 0.45, 80.0, 0.5, false, 0.49181339901936466),
            (0.55, 0.4, 100.0, 1.5, false, 0.7554877374735751),
        ] {
            let o = RateOptions { ratio, pooled: true, exact: false };
            let got = rate_power_with(RateTest::TwoProportions, n1, p1, p2, 0.05, two, o).unwrap();
            assert!((got - want).abs() < 1e-12, "{p1} {p2} {n1} {ratio}: {got} vs {want}");
        }
        // Equal groups by default reproduce the T415 powers; a second group below 2 has none.
        let plain = rate_power(RateTest::TwoProportions, 50.0, 0.6, 0.4, 0.05, true).unwrap();
        let same =
            rate_power_with(RateTest::TwoProportions, 50.0, 0.6, 0.4, 0.05, true, RateOptions::default()).unwrap();
        assert!((plain - same).abs() < 1e-15);
        let tiny = RateOptions { ratio: 0.1, ..Default::default() };
        assert!(
            rate_powers_with(RateTest::TwoVariances, &[10.0, 40.0], 2.0, 1.0, 0.05, true, tiny).unwrap()[0].is_nan()
        );
        let s = rate_sample_sizes_with(
            RateTest::TwoProportions,
            &[0.8],
            0.6,
            0.4,
            0.05,
            true,
            RateOptions { ratio: 2.0, pooled: true, exact: false },
        )
        .unwrap();
        let n = s.sizes[0];
        let o = RateOptions { ratio: 2.0, pooled: true, exact: false };
        assert!(rate_power_with(RateTest::TwoProportions, n, 0.6, 0.4, 0.05, true, o).unwrap() >= 0.8);
        assert!(rate_power_with(RateTest::TwoProportions, n - 1.0, 0.6, 0.4, 0.05, true, o).unwrap() < 0.8);
    }

    #[test]
    fn rate_tests_without_an_effect_have_the_level_as_power_and_refuse_bad_input() {
        for test in [RateTest::OneProportion, RateTest::TwoProportions, RateTest::OneVariance, RateTest::TwoVariances] {
            for two in [true, false] {
                let p = rate_power(test, 20.0, 0.4, 0.4, 0.05, two).unwrap();
                assert!((p - 0.05).abs() < 1e-9, "{test:?} {two}: {p}");
            }
            assert_eq!(rate_powers(test, &[1.0, 2.5], 0.4, 0.5, 0.05, true), Err(PowerError::BadInput));
            assert!(rate_powers(test, &[1.0], 0.4, 0.5, 0.05, true).unwrap()[0].is_nan());
            assert_eq!(rate_powers(test, &[10.0], 0.4, 0.0, 0.05, true), Err(PowerError::BadInput));
            assert_eq!(rate_sample_sizes(test, &[0.8], 0.4, 0.5, 0.05, true).unwrap().sizes.len(), 1);
            assert_eq!(rate_sample_sizes(test, &[1.0], 0.4, 0.5, 0.05, true), Err(PowerError::BadPower));
        }
        assert_eq!(rate_powers(RateTest::OneProportion, &[10.0], 1.0, 0.5, 0.05, true), Err(PowerError::BadInput));
        // A variance above 1 is fine, a proportion is not.
        assert!(rate_powers(RateTest::OneVariance, &[10.0], 3.0, 1.0, 0.05, true).is_ok());
    }

    #[test]
    #[allow(clippy::excessive_precision)] // the digits as Origin printed them
    fn powers_agree_with_numbers_origin_gave() {
        // Two samples, means 5 and 6, sd 1.5, alpha 0.05, n = 10: Origin 0.29211006608063178.
        let p = t_power(PowerTest::TwoSample, 10.0, 1.0, 1.5, 0.05, true).unwrap();
        assert!((p - 0.29211006608063178).abs() < 1e-5, "{p}");
        let p = t_power(PowerTest::OneSample, 20.0, 1.0, 1.5, 0.05, true).unwrap();
        assert!((p - 0.80729166078418058).abs() < 1e-5, "{p}");
        let p = anova_power(10.0, 3.0, 2.0, 1.0, 0.05).unwrap();
        assert!((p - 0.97325526836930554).abs() < 1e-5, "{p}");
    }
}
