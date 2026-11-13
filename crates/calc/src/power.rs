//! Power and sample size of the one-sample, two-sample and paired t-tests and of one-way ANOVA.
//!
//! The power of a t-test is the probability that the statistic falls in the rejection region when the
//! true standardized effect is `d`: the statistic then has a noncentral t distribution with
//! `ncp = d sqrt(n)` (one sample and pairs, `n` pairs) or `ncp = d / sqrt(1/n1 + 1/n2)` (two samples,
//! equal variances), and `n - 1` or `n1 + n2 - 2` degrees of freedom. The one-way ANOVA F statistic has a
//! noncentral F distribution with `ncp = f^2 N` (`f` the effect size of Cohen: the standard deviation of
//! the group means about their mean over the common standard deviation, `N` the total sample size). The
//! formulas are the textbook ones (see `docs/spec/statistics.md`, section 12); nothing here was measured
//! against Origin.
//!
//! A sample size is the smallest integer whose power reaches the wanted power (the power is increasing in
//! `n`, found by doubling and bisection).

use crate::hypothesis::Tail;
use crate::noncentral::{ncf_sf, nct_cdf, nct_sf};
use statrs::distribution::{ContinuousCDF, FisherSnedecor, StudentsT};

/// Largest sample size the search goes to.
pub const MAX_SAMPLE_SIZE: usize = 10_000_000;

#[derive(Debug, Clone, PartialEq)]
pub enum PowerError {
    /// A level (significance or power) outside 0 to 1.
    BadLevel(&'static str),
    /// The effect size must be a positive number.
    BadEffect,
    /// Fewer observations than the test needs.
    TooSmall { needed: usize },
    /// The wanted power is not reached below [`MAX_SAMPLE_SIZE`].
    TooLarge,
    /// The ratio of the sample sizes must be positive.
    BadRatio,
    /// One-way ANOVA needs at least two groups.
    TooFewGroups,
    /// The group means have no spread, or the standard deviation is not positive.
    BadSpread,
}

impl std::fmt::Display for PowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerError::BadLevel(what) => write!(f, "the {what} must lie between 0 and 1"),
            PowerError::BadEffect => write!(f, "the effect size must be a positive number"),
            PowerError::TooSmall { needed } => write!(f, "the sample size must be at least {needed}"),
            PowerError::TooLarge => write!(f, "the wanted power is not reached with {MAX_SAMPLE_SIZE} observations"),
            PowerError::BadRatio => write!(f, "the ratio of the sample sizes must be positive"),
            PowerError::TooFewGroups => write!(f, "a one-way ANOVA needs at least two groups"),
            PowerError::BadSpread => write!(f, "the standard deviation must be positive and the means must differ"),
        }
    }
}

/// Which t-test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TTestDesign {
    /// One sample against a hypothesized mean.
    OneSample,
    /// Pairs: a one-sample test of the differences (the standard deviation is that of the differences).
    Paired,
    /// Two independent samples with equal variances; `ratio` is `n2 / n1`.
    TwoSample { ratio: f64 },
}

/// Power of a t-test and the numbers it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TPower {
    pub n1: usize,
    /// The second sample size (equal to `n1` for the other designs, where it is not used).
    pub n2: usize,
    pub df: f64,
    pub ncp: f64,
    /// Critical value of the t statistic (the upper one for a two-sided test).
    pub critical: f64,
    pub power: f64,
}

fn check_level(v: f64, what: &'static str) -> Result<(), PowerError> {
    if v > 0.0 && v < 1.0 { Ok(()) } else { Err(PowerError::BadLevel(what)) }
}

fn second_size(design: TTestDesign, n1: usize) -> Result<usize, PowerError> {
    match design {
        TTestDesign::TwoSample { ratio } => {
            if !(ratio > 0.0 && ratio.is_finite()) {
                return Err(PowerError::BadRatio);
            }
            Ok(((n1 as f64 * ratio).ceil() as usize).max(2))
        }
        _ => Ok(n1),
    }
}

/// Power of a t-test with `n1` observations (pairs, or the first sample) for the standardized effect
/// `effect` (the difference to detect over the standard deviation, taken as positive; the one-sided tails
/// assume the true difference lies on the side of the alternative).
pub fn t_power(design: TTestDesign, effect: f64, n1: usize, alpha: f64, tail: Tail) -> Result<TPower, PowerError> {
    check_level(alpha, "significance level")?;
    if !(effect > 0.0 && effect.is_finite()) {
        return Err(PowerError::BadEffect);
    }
    let n2 = second_size(design, n1)?;
    let (df, ncp, needed) = match design {
        TTestDesign::OneSample | TTestDesign::Paired => (n1 as f64 - 1.0, effect * (n1 as f64).sqrt(), 2),
        TTestDesign::TwoSample { .. } => {
            ((n1 + n2) as f64 - 2.0, effect / (1.0 / n1 as f64 + 1.0 / n2 as f64).sqrt(), 2)
        }
    };
    if n1 < needed || df < 1.0 {
        return Err(PowerError::TooSmall { needed });
    }
    let t = StudentsT::new(0.0, 1.0, df).expect("df is positive");
    let (critical, power) = match tail {
        Tail::Two => {
            let c = t.inverse_cdf(1.0 - alpha / 2.0);
            (c, nct_sf(c, df, ncp) + nct_cdf(-c, df, ncp))
        }
        Tail::Upper | Tail::Lower => {
            let c = t.inverse_cdf(1.0 - alpha);
            (c, nct_sf(c, df, ncp))
        }
    };
    Ok(TPower { n1, n2, df, ncp, critical, power: power.clamp(0.0, 1.0) })
}

/// Smallest sample size (pairs, or the size of the first sample) whose power reaches `power`.
pub fn t_sample_size(
    design: TTestDesign,
    effect: f64,
    power: f64,
    alpha: f64,
    tail: Tail,
) -> Result<TPower, PowerError> {
    check_level(power, "power")?;
    search(2, |n| t_power(design, effect, n, alpha, tail), power)
}

/// Smallest `n >= start` whose power reaches `target`; `start` is the smallest size the test allows.
fn search<T: HasPower + Copy>(
    start: usize,
    f: impl Fn(usize) -> Result<T, PowerError>,
    target: f64,
) -> Result<T, PowerError> {
    let first = f(start)?;
    if first.power() >= target {
        return Ok(first);
    }
    let mut lo = start;
    let mut hi = start * 2;
    loop {
        if hi > MAX_SAMPLE_SIZE {
            return Err(PowerError::TooLarge);
        }
        if f(hi)?.power() >= target {
            break;
        }
        lo = hi;
        hi *= 2;
    }
    // lo does not reach the power, hi does.
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if f(mid)?.power() >= target {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    f(hi)
}

trait HasPower {
    fn power(&self) -> f64;
}

impl HasPower for TPower {
    fn power(&self) -> f64 {
        self.power
    }
}

impl HasPower for AnovaPower {
    fn power(&self) -> f64 {
        self.power
    }
}

/// Power of a one-way ANOVA and the numbers it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnovaPower {
    pub groups: usize,
    /// Observations per group (balanced design).
    pub n: usize,
    pub total: usize,
    pub df1: f64,
    pub df2: f64,
    pub ncp: f64,
    /// Critical value of F.
    pub critical: f64,
    pub power: f64,
}

/// Cohen's effect size `f` of group means with a common standard deviation `sd`: the standard deviation of
/// the means about their mean (divisor `k`) over `sd`.
pub fn anova_effect_size(means: &[f64], sd: f64) -> Result<f64, PowerError> {
    if means.len() < 2 {
        return Err(PowerError::TooFewGroups);
    }
    if !(sd > 0.0 && sd.is_finite()) {
        return Err(PowerError::BadSpread);
    }
    let k = means.len() as f64;
    let grand = means.iter().sum::<f64>() / k;
    let f = (means.iter().map(|m| (m - grand).powi(2)).sum::<f64>() / k).sqrt() / sd;
    if f > 0.0 { Ok(f) } else { Err(PowerError::BadSpread) }
}

/// Power of a balanced one-way ANOVA of `groups` groups with `n` observations each and effect size `f`.
pub fn anova_power(groups: usize, effect_f: f64, n: usize, alpha: f64) -> Result<AnovaPower, PowerError> {
    check_level(alpha, "significance level")?;
    if groups < 2 {
        return Err(PowerError::TooFewGroups);
    }
    if !(effect_f > 0.0 && effect_f.is_finite()) {
        return Err(PowerError::BadEffect);
    }
    if n < 2 {
        return Err(PowerError::TooSmall { needed: 2 });
    }
    let total = groups * n;
    let df1 = groups as f64 - 1.0;
    let df2 = (total - groups) as f64;
    let ncp = effect_f * effect_f * total as f64;
    let critical = FisherSnedecor::new(df1, df2).expect("positive degrees of freedom").inverse_cdf(1.0 - alpha);
    let power = ncf_sf(critical, df1, df2, ncp);
    Ok(AnovaPower { groups, n, total, df1, df2, ncp, critical, power: power.clamp(0.0, 1.0) })
}

/// Smallest group size whose power reaches `power`.
pub fn anova_sample_size(groups: usize, effect_f: f64, power: f64, alpha: f64) -> Result<AnovaPower, PowerError> {
    check_level(power, "power")?;
    search(2, |n| anova_power(groups, effect_f, n, alpha), power)
}
