//! t-tests: one sample, two samples (pooled or Welch) and paired samples.
//!
//! The rules follow what Origin 2016 reports (`ttest1`, `ttest2`, `ttestpair`; see
//! `docs/spec/statistics.md`): the t statistic is `(estimate - hypothesized value) / standard error`,
//! the p-value follows the chosen tail, and the confidence interval belongs to the estimate itself
//! (the mean, the difference of the means, the mean of the pair differences). A one-sided test gives
//! the interval of confidence level `1 - 2 alpha`, the two-sided interval of the same alpha, as Origin does.

use statrs::distribution::{ContinuousCDF, StudentsT};

/// Which alternative hypothesis a test uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tail {
    /// The estimate differs from the hypothesized value in either direction.
    Two,
    /// The estimate is larger than the hypothesized value.
    Upper,
    /// The estimate is smaller than the hypothesized value.
    Lower,
}

/// Size, mean, standard deviation and standard error of one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleStats {
    pub n: usize,
    pub mean: f64,
    pub sd: f64,
    pub sem: f64,
}

impl SampleStats {
    /// Statistics of the numbers in `data`; NaN entries are ignored.
    pub fn of(data: &[f64]) -> SampleStats {
        let v: Vec<f64> = data.iter().copied().filter(|x| !x.is_nan()).collect();
        let n = v.len();
        let mean = if n > 0 { v.iter().sum::<f64>() / n as f64 } else { f64::NAN };
        let sd = if n > 1 {
            (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0)).sqrt()
        } else {
            f64::NAN
        };
        SampleStats { n, mean, sd, sem: sd / (n as f64).sqrt() }
    }
}

/// What a t-test reports.
#[derive(Debug, Clone, PartialEq)]
pub struct TTest {
    /// The samples that went into the test (one, or two for the two-sample and the paired test).
    pub samples: Vec<SampleStats>,
    /// The mean (one sample), the difference of the means `mean1 - mean2` (two samples) or the mean of
    /// the differences `x1 - x2` (paired).
    pub estimate: f64,
    /// Standard error of the estimate.
    pub se: f64,
    /// The value the estimate is compared with (the null hypothesis).
    pub hypothesized: f64,
    pub t: f64,
    pub df: f64,
    /// Probability of a statistic at least as extreme as `t` under the null hypothesis.
    pub p: f64,
    /// Confidence interval of the estimate.
    pub lcl: f64,
    pub ucl: f64,
    /// Confidence level of that interval, for example 0.95.
    pub level: f64,
    pub tail: Tail,
    pub alpha: f64,
}

impl TTest {
    /// True if the null hypothesis is rejected at the significance level `alpha`.
    pub fn rejected(&self) -> bool {
        self.p < self.alpha
    }
}

fn finish(
    samples: Vec<SampleStats>,
    estimate: f64,
    se: f64,
    df: f64,
    hypothesized: f64,
    tail: Tail,
    alpha: f64,
) -> Option<TTest> {
    if !(alpha > 0.0 && alpha < 1.0) || !df.is_finite() || df <= 0.0 || !se.is_finite() {
        return None;
    }
    let dist = StudentsT::new(0.0, 1.0, df).ok()?;
    let t = (estimate - hypothesized) / se;
    if t.is_nan() {
        return None;
    }
    let p = match tail {
        Tail::Two => 2.0 * dist.cdf(-t.abs()),
        Tail::Upper => dist.cdf(-t),
        Tail::Lower => dist.cdf(t),
    };
    let level = if tail == Tail::Two { 1.0 - alpha } else { 1.0 - 2.0 * alpha };
    let crit = dist.inverse_cdf(1.0 - (1.0 - level) / 2.0);
    Some(TTest {
        samples,
        estimate,
        se,
        hypothesized,
        t,
        df,
        p,
        lcl: estimate - crit * se,
        ucl: estimate + crit * se,
        level,
        tail,
        alpha,
    })
}

/// One-sample t-test of the mean of `data` against `mean0`. Needs at least two numbers.
pub fn ttest_one(data: &[f64], mean0: f64, tail: Tail, alpha: f64) -> Option<TTest> {
    let s = SampleStats::of(data);
    if s.n < 2 {
        return None;
    }
    finish(vec![s], s.mean, s.sem, s.n as f64 - 1.0, mean0, tail, alpha)
}

/// Two-sample t-test of `mean(a) - mean(b)` against `mean_diff`. With `equal_var` the pooled variance is
/// used (degrees of freedom `n1 + n2 - 2`); without it Welch's test with the Satterthwaite degrees of
/// freedom. The confidence interval uses the same standard error and degrees of freedom as the test.
pub fn ttest_two(a: &[f64], b: &[f64], mean_diff: f64, tail: Tail, alpha: f64, equal_var: bool) -> Option<TTest> {
    let (s1, s2) = (SampleStats::of(a), SampleStats::of(b));
    if s1.n < 2 || s2.n < 2 {
        return None;
    }
    let (n1, n2) = (s1.n as f64, s2.n as f64);
    let (v1, v2) = (s1.sd * s1.sd, s2.sd * s2.sd);
    let (se, df) = if equal_var {
        let pooled = ((n1 - 1.0) * v1 + (n2 - 1.0) * v2) / (n1 + n2 - 2.0);
        ((pooled * (1.0 / n1 + 1.0 / n2)).sqrt(), n1 + n2 - 2.0)
    } else {
        let (q1, q2) = (v1 / n1, v2 / n2);
        ((q1 + q2).sqrt(), (q1 + q2).powi(2) / (q1 * q1 / (n1 - 1.0) + q2 * q2 / (n2 - 1.0)))
    };
    finish(vec![s1, s2], s1.mean - s2.mean, se, df, mean_diff, tail, alpha)
}

/// Paired t-test on the differences `a[i] - b[i]` of the rows where both values exist, against
/// `mean_diff`. Needs at least two such rows.
pub fn ttest_paired(a: &[f64], b: &[f64], mean_diff: f64, tail: Tail, alpha: f64) -> Option<TTest> {
    let pairs: Vec<(f64, f64)> =
        a.iter().zip(b).filter(|(x, y)| !x.is_nan() && !y.is_nan()).map(|(x, y)| (*x, *y)).collect();
    if pairs.len() < 2 {
        return None;
    }
    let first: Vec<f64> = pairs.iter().map(|p| p.0).collect();
    let second: Vec<f64> = pairs.iter().map(|p| p.1).collect();
    let diff: Vec<f64> = pairs.iter().map(|p| p.0 - p.1).collect();
    let d = SampleStats::of(&diff);
    finish(
        vec![SampleStats::of(&first), SampleStats::of(&second)],
        d.mean,
        d.sem,
        d.n as f64 - 1.0,
        mean_diff,
        tail,
        alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_sided_p_values_add_up_to_one() {
        let x = [1.0, 2.0, 2.5, 4.0, 4.4];
        let up = ttest_one(&x, 2.0, Tail::Upper, 0.05).unwrap();
        let low = ttest_one(&x, 2.0, Tail::Lower, 0.05).unwrap();
        let two = ttest_one(&x, 2.0, Tail::Two, 0.05).unwrap();
        assert!((up.p + low.p - 1.0).abs() < 1e-12);
        assert!((two.p - 2.0 * up.p.min(low.p)).abs() < 1e-12);
    }

    #[test]
    fn paired_equals_one_sample_on_differences() {
        let a = [3.0, 4.0, 5.5, 6.0, f64::NAN];
        let b = [2.0, 4.5, 5.0, 4.0, 9.0];
        let d: Vec<f64> = vec![1.0, -0.5, 0.5, 2.0];
        let p = ttest_paired(&a, &b, 0.0, Tail::Two, 0.05).unwrap();
        let o = ttest_one(&d, 0.0, Tail::Two, 0.05).unwrap();
        assert!((p.t - o.t).abs() < 1e-12 && (p.p - o.p).abs() < 1e-12 && p.samples[0].n == 4);
    }

    #[test]
    fn welch_equals_pooled_for_equal_sizes_and_spreads() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [2.0, 3.0, 4.0, 5.0];
        let p = ttest_two(&a, &b, 0.0, Tail::Two, 0.05, true).unwrap();
        let w = ttest_two(&a, &b, 0.0, Tail::Two, 0.05, false).unwrap();
        assert!((p.t - w.t).abs() < 1e-12 && (p.df - w.df).abs() < 1e-12);
    }

    #[test]
    fn zero_spread_with_matching_mean_gives_nothing_and_does_not_panic() {
        assert!(ttest_one(&[3.0; 6], 3.0, Tail::Two, 0.05).is_none());
        assert!(ttest_two(&[3.0; 6], &[3.0; 6], 0.0, Tail::Two, 0.05, true).is_none());
        let r = ttest_one(&[3.0; 6], 2.0, Tail::Two, 0.05).unwrap();
        assert_eq!(r.p, 0.0);
    }

    #[test]
    fn too_few_numbers_give_nothing() {
        assert!(ttest_one(&[1.0], 0.0, Tail::Two, 0.05).is_none());
        assert!(ttest_two(&[1.0, 2.0], &[3.0], 0.0, Tail::Two, 0.05, true).is_none());
        assert!(ttest_one(&[1.0, 2.0], 0.0, Tail::Two, 1.5).is_none());
    }
}
