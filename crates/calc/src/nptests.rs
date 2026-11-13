//! More rank and count based tests: the sign test of paired samples, the two-sample Kolmogorov-Smirnov test,
//! Mood's median test and Friedman's ANOVA.
//!
//! The rules follow what Origin reports for `sign2`, `kstest2`, `mediantest` and `friedman` (see
//! `docs/spec/statistics.md`, section 10), including the places where Origin is not the textbook
//! (one-sided sign test p-values, the size limits of the exact p-values).

use crate::hypothesis::Tail;
use crate::nonparam::midranks;
use crate::stats::quantile_origin;
use statrs::distribution::{Binomial, ChiSquared, ContinuousCDF, DiscreteCDF, Normal};

fn finite(data: &[f64]) -> Vec<f64> {
    data.iter().copied().filter(|x| !x.is_nan()).collect()
}

fn normal_cdf(z: f64) -> f64 {
    Normal::new(0.0, 1.0).expect("standard normal").cdf(z)
}

fn chi_square_sf(x: f64, df: f64) -> f64 {
    ChiSquared::new(df).map_or(f64::NAN, |d| 1.0 - d.cdf(x))
}

// ----- sign test --------------------------------------------------------------------------------------------

/// The sign test uses the exact binomial p-value up to this many pairs that differ, and the normal
/// approximation with continuity correction above (Origin switches between 25 and 26).
pub const SIGN_EXACT_MAX_N: usize = 25;

/// Result of [`sign_test`].
#[derive(Debug, Clone, PartialEq)]
pub struct SignTest {
    /// Pairs where both samples have a number.
    pub pairs: usize,
    /// Pairs where the first number is smaller than the second.
    pub first_smaller: usize,
    /// Pairs where the second number is smaller than the first.
    pub second_smaller: usize,
    /// Pairs with equal numbers (left out of the test).
    pub ties: usize,
    /// Number of pairs that differ (`first_smaller + second_smaller`).
    pub n: usize,
    /// The statistic Origin prints: `first_smaller`.
    pub statistic: f64,
    /// The normal score, only when the normal approximation was used.
    pub z: Option<f64>,
    pub p: f64,
}

/// Paired sign test of `a` against `b` (rows where either has no number are dropped). `Upper` is Origin's
/// "F(x) > G(y)". Needs at least two pairs, otherwise `None` (Origin prints nothing).
pub fn sign_test(a: &[f64], b: &[f64], tail: Tail) -> Option<SignTest> {
    let (mut less, mut greater, mut ties) = (0usize, 0usize, 0usize);
    for (x, y) in a.iter().zip(b) {
        if x.is_nan() || y.is_nan() {
            continue;
        }
        match x.partial_cmp(y) {
            Some(std::cmp::Ordering::Less) => less += 1,
            Some(std::cmp::Ordering::Greater) => greater += 1,
            _ => ties += 1,
        }
    }
    let pairs = less + greater + ties;
    if pairs < 2 {
        return None;
    }
    let n = less + greater;
    let mut z = None;
    let p = if n == 0 {
        1.0
    } else if n <= SIGN_EXACT_MAX_N {
        let m = less.min(greater);
        let pm = Binomial::new(0.5, n as u64).expect("binomial").cdf(m as u64);
        match (tail, less.cmp(&greater)) {
            (Tail::Two, _) => (2.0 * pm).min(1.0),
            (_, std::cmp::Ordering::Equal) => 0.5,
            (Tail::Lower, std::cmp::Ordering::Less) | (Tail::Upper, std::cmp::Ordering::Greater) => pm,
            _ => 1.0 - pm,
        }
    } else {
        let nf = n as f64;
        let zz = ((less as f64 - nf / 2.0).abs() - 0.5) / (nf.sqrt() / 2.0);
        z = Some(zz);
        match tail {
            Tail::Two => (2.0 * (1.0 - normal_cdf(zz.abs()))).min(1.0),
            Tail::Upper => 1.0 - normal_cdf(zz),
            Tail::Lower => normal_cdf(zz),
        }
    };
    Some(SignTest { pairs, first_smaller: less, second_smaller: greater, ties, n, statistic: less as f64, z, p })
}

// ----- two-sample Kolmogorov-Smirnov ------------------------------------------------------------------------

/// The exact p-value of the two-sample test is used up to this many pairs of numbers (`n1 * n2`); above it
/// Origin uses an asymptotic formula.
pub const KS2_EXACT_MAX_PAIRS: usize = 10_000;

/// Result of [`ks_two_sample`].
#[derive(Debug, Clone, PartialEq)]
pub struct KsTwoSample {
    pub n1: usize,
    pub n2: usize,
    /// The distance of the tail asked for: the largest `F1 - F2` (upper), `F2 - F1` (lower) or the largest
    /// absolute difference (two-sided) of the empirical distribution functions.
    pub d: f64,
    /// `sqrt(n1 n2 / (n1 + n2)) d`.
    pub z: f64,
    pub p: f64,
    /// True when the p-value is the exact one.
    pub exact: bool,
}

/// `P(D >= d)` of the two-sample statistic for continuous data, `dnum = d n1 n2` (an integer). Counts the
/// lattice paths that stay strictly inside the band.
fn ks_exact_p(n1: usize, n2: usize, dnum: i64) -> f64 {
    let mut prob = vec![vec![0.0_f64; n2 + 1]; n1 + 1];
    prob[0][0] = 1.0;
    for i in 0..=n1 {
        for j in 0..=n2 {
            if i == 0 && j == 0 {
                continue;
            }
            if (i as i64 * n2 as i64 - j as i64 * n1 as i64).abs() >= dnum {
                prob[i][j] = 0.0;
                continue;
            }
            let mut s = 0.0;
            if i > 0 {
                s += i as f64 * prob[i - 1][j];
            }
            if j > 0 {
                s += j as f64 * prob[i][j - 1];
            }
            prob[i][j] = s / (i + j) as f64;
        }
    }
    (1.0 - prob[n1][n2]).clamp(0.0, 1.0)
}

/// Kolmogorov's limiting upper tail probability `2 sum (-1)^(j-1) exp(-2 j^2 z^2)`.
fn kolmogorov_q(z: f64) -> f64 {
    if z <= 0.0 {
        return 1.0;
    }
    let mut s = 0.0;
    for j in 1..=200 {
        let jf = f64::from(j);
        let term = 2.0 * (-2.0 * jf * jf * z * z).exp();
        s += if j % 2 == 1 { term } else { -term };
        if term < 1e-18 {
            break;
        }
    }
    s.clamp(0.0, 1.0)
}

/// Two-sample Kolmogorov-Smirnov test of `x` against `y` (NaN ignored); `Upper` is "F(x) > G(y)". The
/// p-value is exact when `n1 * n2 <= KS2_EXACT_MAX_PAIRS` (ties are not taken into account, as in Origin),
/// otherwise the asymptotic `Q(sqrt(n1 n2 / (n1 + n2)) D + 0.5 / sqrt(max(n1, n2)))`. A one-sided p-value is
/// half of the two-sided p-value of the one-sided distance. Needs two numbers in each sample.
pub fn ks_two_sample(x: &[f64], y: &[f64], tail: Tail) -> Option<KsTwoSample> {
    let (x, y) = (finite(x), finite(y));
    let (n1, n2) = (x.len(), y.len());
    if n1 < 2 || n2 < 2 {
        return None;
    }
    let mut pooled: Vec<(f64, bool)> = x.iter().map(|&v| (v, true)).chain(y.iter().map(|&v| (v, false))).collect();
    pooled.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut i, mut j) = (0i64, 0i64);
    let (mut plus, mut minus) = (0i64, 0i64);
    let mut k = 0;
    while k < pooled.len() {
        let v = pooled[k].0;
        while k < pooled.len() && pooled[k].0 == v {
            if pooled[k].1 {
                i += 1;
            } else {
                j += 1;
            }
            k += 1;
        }
        let diff = i * n2 as i64 - j * n1 as i64;
        plus = plus.max(diff);
        minus = minus.max(-diff);
    }
    let dnum = match tail {
        Tail::Two => plus.max(minus),
        Tail::Upper => plus,
        Tail::Lower => minus,
    };
    let scale = (n1 * n2) as f64;
    let d = dnum as f64 / scale;
    let ne = scale / (n1 + n2) as f64;
    let z = ne.sqrt() * d;
    let exact = n1 * n2 <= KS2_EXACT_MAX_PAIRS;
    let two_sided = if exact { ks_exact_p(n1, n2, dnum) } else { kolmogorov_q(z + 0.5 / (n1.max(n2) as f64).sqrt()) };
    let p = if tail == Tail::Two { two_sided } else { two_sided / 2.0 };
    Some(KsTwoSample { n1, n2, d, z, p, exact })
}

// ----- median test ------------------------------------------------------------------------------------------

/// One group of [`MedianTest`].
#[derive(Debug, Clone, PartialEq)]
pub struct MedianGroup {
    pub n: usize,
    /// Numbers at or below the overall median.
    pub at_or_below: usize,
    /// Numbers above the overall median.
    pub above: usize,
}

/// Result of [`median_test`].
#[derive(Debug, Clone, PartialEq)]
pub struct MedianTest {
    /// The median of all the numbers together.
    pub median: f64,
    pub groups: Vec<MedianGroup>,
    pub chi_square: f64,
    pub df: f64,
    pub p: f64,
}

/// Mood's median test of the groups (NaN ignored): the chi-square statistic of the table of counts at or
/// below and above the overall median, without continuity correction; `k - 1` degrees of freedom. Cells
/// with an expected count of zero are left out (all numbers equal gives chi-square 0 and p = 1).
/// `None` when there are fewer than two groups or a group is empty.
pub fn median_test(groups: &[&[f64]]) -> Option<MedianTest> {
    let data: Vec<Vec<f64>> = groups.iter().map(|g| finite(g)).collect();
    if data.len() < 2 || data.iter().any(Vec::is_empty) {
        return None;
    }
    let mut pooled: Vec<f64> = data.iter().flatten().copied().collect();
    pooled.sort_by(|a, b| a.total_cmp(b));
    let median = quantile_origin(&pooled, 0.5);
    let out: Vec<MedianGroup> = data
        .iter()
        .map(|g| {
            let low = g.iter().filter(|&&v| v <= median).count();
            MedianGroup { n: g.len(), at_or_below: low, above: g.len() - low }
        })
        .collect();
    let total = pooled.len() as f64;
    let col_low: f64 = out.iter().map(|g| g.at_or_below as f64).sum();
    let col_high = total - col_low;
    let mut chi = 0.0;
    for g in &out {
        for (observed, col) in [(g.at_or_below as f64, col_low), (g.above as f64, col_high)] {
            let expected = g.n as f64 * col / total;
            if expected > 0.0 {
                chi += (observed - expected).powi(2) / expected;
            }
        }
    }
    let df = (out.len() - 1) as f64;
    Some(MedianTest { median, groups: out, chi_square: chi, df, p: chi_square_sf(chi, df) })
}

// ----- Friedman ---------------------------------------------------------------------------------------------

/// One treatment (column) of [`Friedman`].
#[derive(Debug, Clone, PartialEq)]
pub struct FriedmanGroup {
    pub rank_sum: f64,
    pub mean_rank: f64,
}

/// Result of [`friedman`].
#[derive(Debug, Clone, PartialEq)]
pub struct Friedman {
    /// Number of blocks (rows).
    pub n: usize,
    pub groups: Vec<FriedmanGroup>,
    pub chi_square: f64,
    pub df: f64,
    pub p: f64,
}

/// Why [`friedman`] has no result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FriedmanError {
    TooFewColumns,
    /// The columns differ in length or have a missing number (1-based block); Origin gives no result either.
    Incomplete(usize),
    TooFewBlocks,
}

impl std::fmt::Display for FriedmanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FriedmanError::TooFewColumns => write!(f, "Friedman's test needs at least two columns"),
            FriedmanError::Incomplete(r) => {
                write!(f, "Friedman's test needs a number in every column of every row; row {r} has none")
            }
            FriedmanError::TooFewBlocks => write!(f, "Friedman's test needs at least two rows"),
        }
    }
}

/// Friedman's ANOVA: each row is a block, each column a treatment. The numbers of a row are ranked (ties
/// share the mean rank); `chi2 = 12 / (n k (k + 1)) sum R_j^2 - 3 n (k + 1)` with `k - 1` degrees of
/// freedom. Origin applies no correction for ties (measured: the textbook tie-corrected statistic differs
/// from its numbers). If all numbers in every row are equal chi-square is 0 and p is 1.
pub fn friedman(columns: &[&[f64]]) -> Result<Friedman, FriedmanError> {
    let k = columns.len();
    if k < 2 {
        return Err(FriedmanError::TooFewColumns);
    }
    let n = columns.iter().map(|c| c.len()).max().unwrap_or(0);
    for r in 0..n {
        if columns.iter().any(|c| c.get(r).is_none_or(|v| v.is_nan())) {
            return Err(FriedmanError::Incomplete(r + 1));
        }
    }
    if n < 2 {
        return Err(FriedmanError::TooFewBlocks);
    }
    let mut sums = vec![0.0; k];
    for r in 0..n {
        let row: Vec<f64> = columns.iter().map(|c| c[r]).collect();
        let (ranks, _) = midranks(&row);
        for (s, rk) in sums.iter_mut().zip(&ranks) {
            *s += rk;
        }
    }
    let (nf, kf) = (n as f64, k as f64);
    let h = 12.0 / (nf * kf * (kf + 1.0)) * sums.iter().map(|s| s * s).sum::<f64>() - 3.0 * nf * (kf + 1.0);
    let chi = h.max(0.0);
    let df = kf - 1.0;
    let groups = sums.iter().map(|&s| FriedmanGroup { rank_sum: s, mean_rank: s / nf }).collect();
    Ok(Friedman { n, groups, chi_square: chi, df, p: chi_square_sf(chi, df) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use statrs::distribution::Discrete;

    #[test]
    fn sign_test_counts_and_exact_p() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let b = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 9.0];
        let s = sign_test(&a, &b, Tail::Two).unwrap();
        assert_eq!((s.first_smaller, s.second_smaller, s.ties, s.n), (8, 0, 1, 8));
        assert!((s.p - 2.0 / 256.0).abs() < 1e-15);
        assert!(s.z.is_none());
        assert!(sign_test(&a[..1], &b[..1], Tail::Two).is_none());
    }

    #[test]
    fn ks_two_sample_of_separated_samples() {
        let r = ks_two_sample(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0, 7.0], Tail::Two).unwrap();
        assert_eq!(r.d, 1.0);
        assert!((r.p - 2.0 / 35.0).abs() < 1e-12);
        let lower = ks_two_sample(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0, 7.0], Tail::Upper).unwrap();
        assert_eq!(lower.d, 1.0);
        assert!(ks_two_sample(&[1.0], &[2.0, 3.0], Tail::Two).is_none());
    }

    #[test]
    fn median_test_of_two_halves() {
        let r = median_test(&[&[1.0, 2.0], &[3.0, 4.0]]).unwrap();
        assert_eq!(r.median, 2.5);
        assert!((r.chi_square - 4.0).abs() < 1e-12);
        assert_eq!(r.df, 1.0);
    }

    #[test]
    fn friedman_of_a_perfectly_ordered_design() {
        let r = friedman(&[&[1.0, 2.0, 3.0, 4.0], &[2.0, 3.0, 4.0, 5.0], &[3.0, 4.0, 5.0, 6.0]]).unwrap();
        assert_eq!(r.chi_square, 8.0);
        assert_eq!(r.groups[2].rank_sum, 12.0);
        assert_eq!(friedman(&[&[1.0, f64::NAN], &[2.0, 3.0]]), Err(FriedmanError::Incomplete(2)));
        // Origin's tie handling: all numbers equal gives chi-square 0.
        let flat = friedman(&[&[5.0; 4], &[5.0; 4], &[5.0; 4]]).unwrap();
        assert_eq!((flat.chi_square, flat.p), (0.0, 1.0));
    }

    #[test]
    fn exact_binomial_matches_pmf_sum() {
        let b = Binomial::new(0.5, 10).unwrap();
        let direct: f64 = (0..=3).map(|k| b.pmf(k)).sum();
        assert!((b.cdf(3) - direct).abs() < 1e-15);
    }
}
