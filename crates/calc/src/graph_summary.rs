//! The numbers behind Graphical Summary (T40): the descriptive statistics of one sample with confidence
//! intervals of the mean, the median and the standard deviation, and the Anderson-Darling normality test.
//!
//! Nothing is defined again here: the statistics come from [`stats`] (the Descriptive Statistics tool), the
//! quartiles use its Origin quartile rule, the mean interval is [`mean_confidence_limits`] and the normality
//! test is [`anderson_darling`]. Only the two intervals below are new. The method is written down in
//! `docs/spec/statistics.md`, section 22.

use crate::normality::{TestResult, anderson_darling};
use crate::stats::{Stats, mean_confidence_limits, stats};
use statrs::distribution::{Binomial, ChiSquared, ContinuousCDF, DiscreteCDF};

/// Everything the Graphical Summary reports for one sample.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleSummary {
    /// The descriptive statistics (N, missing, mean, SD, SE, variance, skewness, kurtosis, extremes, quartiles).
    pub stats: Stats,
    /// Confidence level used for the intervals, for example 0.95.
    pub level: f64,
    /// Student t interval of the mean (NaN, NaN for fewer than two numbers).
    pub mean_ci: (f64, f64),
    /// Distribution-free interval of the median, see [`median_confidence_limits`].
    pub median_ci: (f64, f64),
    /// The coverage the median interval really has (at least `level` unless the sample is too small to reach it).
    pub median_coverage: f64,
    /// Chi-square interval of the standard deviation.
    pub sd_ci: (f64, f64),
    /// Anderson-Darling A squared and p-value; `None` for fewer than 8 numbers or no spread.
    pub anderson_darling: Option<TestResult>,
}

/// The interval of the population median from order statistics: `[x(k), x(n+1-k)]` (ascending sample, 1-based
/// ranks) with `k` the largest rank for which `P(Binomial(n, 1/2) <= k-1) <= (1 - level)/2`. The ends are
/// sample values (no interpolation) and the coverage is exact binomial: `1 - 2 P(Binomial(n, 1/2) <= k-1)`,
/// which is at least `level` (conservative). When even `k = 1` cannot reach `level` the interval is the whole
/// range `[min, max]` and the (lower) coverage is returned. Returns `((lo, hi), coverage)`; NaN for fewer than
/// two numbers or a level outside (0, 1). `sorted` must be ascending without NaN.
pub fn median_confidence_limits(sorted: &[f64], level: f64) -> ((f64, f64), f64) {
    let n = sorted.len();
    if n < 2 || !(level > 0.0 && level < 1.0) {
        return ((f64::NAN, f64::NAN), f64::NAN);
    }
    let Ok(b) = Binomial::new(0.5, n as u64) else { return ((f64::NAN, f64::NAN), f64::NAN) };
    let alpha_half = (1.0 - level) / 2.0;
    let mut k = 1;
    for cand in 1..=(n / 2) {
        if b.cdf((cand - 1) as u64) <= alpha_half + 1e-12 {
            k = cand;
        } else {
            break;
        }
    }
    let coverage = 1.0 - 2.0 * b.cdf((k - 1) as u64);
    ((sorted[k - 1], sorted[n - k]), coverage)
}

/// The interval of the standard deviation from the chi-square distribution:
/// `s sqrt((n-1)/chi2_{1-a/2, n-1})` to `s sqrt((n-1)/chi2_{a/2, n-1})`, `a = 1 - level`.
/// NaN for fewer than two numbers or a level outside (0, 1).
pub fn sd_confidence_limits(s: &Stats, level: f64) -> (f64, f64) {
    if s.n < 2 || !(level > 0.0 && level < 1.0) || !s.sd.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    let df = (s.n - 1) as f64;
    let Ok(chi) = ChiSquared::new(df) else { return (f64::NAN, f64::NAN) };
    let a = 1.0 - level;
    (s.sd * (df / chi.inverse_cdf(1.0 - a / 2.0)).sqrt(), s.sd * (df / chi.inverse_cdf(a / 2.0)).sqrt())
}

/// Summary of one sample (NaN entries count as missing) at confidence `level` (0.95 for 95 %).
pub fn graph_summary(data: &[f64], level: f64) -> SampleSummary {
    let st = stats(data);
    let mut v: Vec<f64> = data.iter().copied().filter(|a| !a.is_nan()).collect();
    v.sort_by(|a, b| a.total_cmp(b));
    let (median_ci, median_coverage) = median_confidence_limits(&v, level);
    SampleSummary {
        level,
        mean_ci: mean_confidence_limits(&st, level),
        median_ci,
        median_coverage,
        sd_ci: sd_confidence_limits(&st, level),
        anderson_darling: anderson_darling(data),
        stats: st,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: [f64; 12] = [2.1, 3.4, 3.9, 4.4, 4.8, 5.0, 5.3, 6.1, 6.6, 7.2, 8.0, 11.5];

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol * b.abs().max(1.0), "{a} vs {b}");
    }

    // Reference values: numpy/scipy 1.18 on DATA (see docs/spec/statistics.md section 22.3).
    const REF_MEAN: f64 = 5.691666666666667;
    const REF_SD: f64 = 2.4640905876697032;
    const REF_SKEW: f64 = 1.058210251752913;
    const REF_KURT: f64 = 1.866917485643393;
    const REF_MEAN_LO: f64 = 4.1260582004245965;
    const REF_MEAN_HI: f64 = 7.257275132908738;
    const REF_SD_LO: f64 = 1.7455498235905689;
    const REF_SD_HI: f64 = 4.183725242485017;
    const REF_MED_LO: f64 = 3.9;
    const REF_MED_HI: f64 = 7.2;
    const REF_MED_COV: f64 = 0.96142578125;
    const REF_A2: f64 = 0.30557037512815555;

    #[test]
    fn matches_scipy() {
        let s = graph_summary(&DATA, 0.95);
        assert_eq!(s.stats.n, 12);
        close(s.stats.mean, REF_MEAN, 1e-12);
        close(s.stats.sd, REF_SD, 1e-12);
        close(s.stats.skewness, REF_SKEW, 1e-10);
        close(s.stats.kurtosis, REF_KURT, 1e-10);
        close(s.mean_ci.0, REF_MEAN_LO, 1e-9);
        close(s.mean_ci.1, REF_MEAN_HI, 1e-9);
        close(s.sd_ci.0, REF_SD_LO, 1e-9);
        close(s.sd_ci.1, REF_SD_HI, 1e-9);
        close(s.median_ci.0, REF_MED_LO, 1e-12);
        close(s.median_ci.1, REF_MED_HI, 1e-12);
        close(s.median_coverage, REF_MED_COV, 1e-10);
        let ad = s.anderson_darling.unwrap();
        close(ad.statistic, REF_A2, 1e-9);
        assert!(ad.p > 0.0 && ad.p < 1.0);
    }

    #[test]
    fn statistics_are_the_descriptive_statistics_tool_numbers() {
        let s = graph_summary(&DATA, 0.9);
        assert_eq!(s.stats, stats(&DATA));
        assert_eq!(s.anderson_darling, anderson_darling(&DATA));
        assert_eq!((s.stats.min, s.stats.max), (2.1, 11.5));
    }

    #[test]
    fn median_interval_is_exact_binomial_and_conservative() {
        // n = 12, 95 %: P(Bin <= 2) = 0.01929 <= 0.025, P(Bin <= 3) = 0.0730 > 0.025, so k = 3.
        let (ci, cov) = median_confidence_limits(&DATA, 0.95);
        assert_eq!(ci, (DATA[2], DATA[9]));
        close(cov, 1.0 - 2.0 * 0.019287109375, 1e-12);
        assert!(cov >= 0.95);
        // Too few points for the level: the whole range with the lower coverage.
        let (ci, cov) = median_confidence_limits(&[1.0, 2.0, 3.0, 4.0], 0.95);
        assert_eq!(ci, (1.0, 4.0));
        close(cov, 1.0 - 2.0 / 16.0, 1e-12);
    }

    #[test]
    fn small_samples_give_nan_where_undefined() {
        let s = graph_summary(&[1.0], 0.95);
        assert!(s.mean_ci.0.is_nan() && s.sd_ci.0.is_nan() && s.median_ci.0.is_nan());
        assert!(s.anderson_darling.is_none());
        let e = graph_summary(&[], 0.95);
        assert_eq!(e.stats.n, 0);
        let bad = graph_summary(&DATA, 1.5);
        assert!(bad.mean_ci.0.is_nan() && bad.sd_ci.1.is_nan());
    }

    #[test]
    fn missing_values_are_counted_and_ignored() {
        let mut d = DATA.to_vec();
        d.insert(3, f64::NAN);
        let s = graph_summary(&d, 0.95);
        assert_eq!(s.stats.missing, 1);
        assert_eq!(s.median_ci, graph_summary(&DATA, 0.95).median_ci);
    }
}
