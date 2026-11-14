//! Power and sample size of the t tests and the one-way ANOVA (Origin's Statistics > Power and Sample Size).
//!
//! Power is the textbook one: the probability that the test statistic lands in the rejection region when it
//! follows the noncentral distribution that belongs to the effect. The sample size is the smallest whole number
//! of observations (per group) whose power reaches the wanted power. The rules and the numbers Origin gave are
//! in `docs/spec/statistics.md` section 12.

use statrs::distribution::{ContinuousCDF, FisherSnedecor, Normal, StudentsT};
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

#[cfg(test)]
mod tests {
    use super::*;

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
