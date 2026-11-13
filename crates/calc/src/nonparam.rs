//! Rank-based tests: Mann-Whitney (two independent samples), Wilcoxon signed rank (one sample or pairs) and
//! Kruskal-Wallis (several samples).
//!
//! The rules follow what Origin reports for `mwtest`, `signrank1`, `signrank2` and `kwanova` (see
//! `docs/spec/statistics.md`): ties get the mean of the ranks they span, the asymptotic p-values use the
//! tie-corrected variance and a continuity correction of 0.5, and the exact p-values are the permutation
//! distribution of the rank sum with those mean ranks.

use crate::hypothesis::Tail;
use statrs::distribution::{ChiSquared, ContinuousCDF, Normal};

/// Ranks of `values` (ties share the mean rank) and the sizes of the groups of tied values
/// (only groups of two or more).
pub fn midranks(values: &[f64]) -> (Vec<f64>, Vec<usize>) {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut ranks = vec![0.0; values.len()];
    let mut ties = Vec::new();
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        while j + 1 < order.len() && values[order[j + 1]] == values[order[i]] {
            j += 1;
        }
        let mean_rank = (i + j) as f64 / 2.0 + 1.0;
        for &k in &order[i..=j] {
            ranks[k] = mean_rank;
        }
        if j > i {
            ties.push(j - i + 1);
        }
        i = j + 1;
    }
    (ranks, ties)
}

fn finite(data: &[f64]) -> Vec<f64> {
    data.iter().copied().filter(|x| !x.is_nan()).collect()
}

fn normal_cdf(z: f64) -> f64 {
    Normal::new(0.0, 1.0).expect("standard normal").cdf(z)
}

/// `sum(t^3 - t)` over the tie groups.
fn tie_term(ties: &[usize]) -> f64 {
    ties.iter().map(|&t| (t as f64).powi(3) - t as f64).sum()
}

/// The largest amount of work (table cells to fill) an exact p-value may take before the asymptotic one is
/// reported alone.
const EXACT_WORK_LIMIT: f64 = 4.0e8;

/// The Origin signed rank test reports exact p-values up to this many (non-zero) differences.
pub const SIGNED_RANK_EXACT_MAX_N: usize = 80;

/// Counts of the subsets of size `k` of the doubled ranks by their sum: `counts[s]` is the number of
/// subsets whose doubled ranks add up to `s`. `None` when the table would be too big.
fn subset_sum_counts(doubled: &[usize], k: usize) -> Option<Vec<f64>> {
    let total: usize = doubled.iter().sum();
    if (doubled.len() * (k + 1)) as f64 * (total + 1) as f64 > EXACT_WORK_LIMIT {
        return None;
    }
    let mut table = vec![vec![0.0f64; total + 1]; k + 1];
    table[0][0] = 1.0;
    let mut reach = 0usize;
    for &w in doubled {
        reach += w;
        for j in (1..=k).rev() {
            let (lower, upper) = table.split_at_mut(j);
            let (prev, cur) = (&lower[j - 1], &mut upper[0]);
            for s in (w..=reach).rev() {
                cur[s] += prev[s - w];
            }
        }
    }
    Some(table.swap_remove(k))
}

/// Lower and upper tail probabilities `P(S <= observed)` and `P(S >= observed)` of a count table.
fn tails_from_counts(counts: &[f64], observed: usize) -> (f64, f64) {
    let total: f64 = counts.iter().sum();
    let lower: f64 = counts[..=observed.min(counts.len() - 1)].iter().sum();
    let upper: f64 = counts[observed.min(counts.len())..].iter().sum();
    ((lower / total).min(1.0), (upper / total).min(1.0))
}

fn pick_tail(lower: f64, upper: f64, tail: Tail) -> f64 {
    match tail {
        Tail::Lower => lower,
        Tail::Upper => upper,
        Tail::Two => (2.0 * lower.min(upper)).min(1.0),
    }
}

fn as_doubled(ranks: &[f64]) -> Vec<usize> {
    ranks.iter().map(|r| (2.0 * r).round() as usize).collect()
}

// ----- Mann-Whitney -------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct MannWhitney {
    pub n1: usize,
    pub n2: usize,
    pub rank_sum1: f64,
    pub rank_sum2: f64,
    pub mean_rank1: f64,
    pub mean_rank2: f64,
    /// The U statistic of the first sample, `rank_sum1 - n1 (n1 + 1) / 2`; the second is `n1 n2 - u1`.
    pub u: f64,
    pub mean_u: f64,
    /// Standard deviation of U with the correction for ties.
    pub sd_u: f64,
    /// `(U - mean + 0.5) / sd` for U below the mean and `- 0.5` above it (the continuity correction goes
    /// towards the mean), as Origin prints it; 0 when all numbers are equal.
    pub z: f64,
    /// Asymptotic p-value of the chosen tail (normal approximation, continuity correction 0.5).
    pub p_asymptotic: f64,
    /// Exact p-value from the permutation distribution of the rank sum, when asked for and small enough.
    pub p_exact: Option<f64>,
    pub tail: Tail,
}

impl MannWhitney {
    /// The p-value to decide with: the exact one if there is one.
    pub fn p(&self) -> f64 {
        self.p_exact.unwrap_or(self.p_asymptotic)
    }
}

/// Mann-Whitney test of two independent samples; NaN entries are ignored. `None` when a sample is empty.
/// `tail` is the alternative about the first sample: `Upper` means it tends to be larger.
pub fn mann_whitney(x: &[f64], y: &[f64], tail: Tail, exact: bool) -> Option<MannWhitney> {
    let (x, y) = (finite(x), finite(y));
    let (n1, n2) = (x.len(), y.len());
    if n1 == 0 || n2 == 0 {
        return None;
    }
    let pooled: Vec<f64> = x.iter().chain(&y).copied().collect();
    let (ranks, ties) = midranks(&pooled);
    let nn = (n1 + n2) as f64;
    let rank_sum1: f64 = ranks[..n1].iter().sum();
    let rank_sum2 = nn * (nn + 1.0) / 2.0 - rank_sum1;
    let u = rank_sum1 - (n1 * (n1 + 1)) as f64 / 2.0;
    let mean_u = (n1 * n2) as f64 / 2.0;
    let var = (n1 * n2) as f64 / 12.0 * ((nn + 1.0) - tie_term(&ties) / (nn * (nn - 1.0)));
    let sd_u = if var > 0.0 { var.sqrt() } else { 0.0 };
    let (z, p_asymptotic) = if sd_u > 0.0 {
        let toward = if u < mean_u {
            0.5
        } else if u > mean_u {
            -0.5
        } else {
            0.0
        };
        let lower = normal_cdf((u - mean_u + 0.5) / sd_u);
        let z = (u - mean_u + toward) / sd_u;
        // With the exact option Origin prints the upper-tail asymptotic p of the plain Z, not the
        // continuity-corrected one of the run without it (measured; the other tails are unchanged).
        let upper = if exact { 1.0 - normal_cdf(z) } else { 1.0 - normal_cdf((u - mean_u - 0.5) / sd_u) };
        (z, pick_tail(lower, upper, tail))
    } else {
        (0.0, 1.0)
    };
    let p_exact = exact
        .then(|| {
            // The smaller sample is the subset whose rank sum is tabulated.
            let doubled = as_doubled(&ranks);
            let observed = (2.0 * rank_sum1).round() as usize;
            let counts = subset_sum_counts(&doubled, n1)?;
            let (lower, upper) = tails_from_counts(&counts, observed);
            Some(pick_tail(lower, upper, tail))
        })
        .flatten();
    Some(MannWhitney {
        n1,
        n2,
        rank_sum1,
        rank_sum2,
        mean_rank1: rank_sum1 / n1 as f64,
        mean_rank2: rank_sum2 / n2 as f64,
        u,
        mean_u,
        sd_u,
        z,
        p_asymptotic,
        p_exact,
        tail,
    })
}

// ----- Wilcoxon signed rank -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct SignedRank {
    /// Differences that are not zero; only they are ranked.
    pub n: usize,
    pub n_zero: usize,
    pub n_positive: usize,
    pub n_negative: usize,
    /// Sum of the ranks of the positive differences (the statistic Origin prints) and of the negative ones.
    pub t_plus: f64,
    pub t_minus: f64,
    pub mean_t: f64,
    pub sd_t: f64,
    /// `(T - mean + 0.5) / sd` for T below the mean and `- 0.5` above it; NaN when no difference is non-zero.
    pub z: f64,
    /// Asymptotic p-value: lower is `P(Z <= z)`, upper `P(Z >= z)`, two-sided twice the smaller tail of that.
    pub p_asymptotic: f64,
    /// Exact p-value (permutation distribution of the sign pattern with the same mean ranks); Origin
    /// reports it up to [`SIGNED_RANK_EXACT_MAX_N`] differences.
    pub p_exact: Option<f64>,
    pub tail: Tail,
}

impl SignedRank {
    /// The p-value to decide with: the exact one if there is one.
    pub fn p(&self) -> f64 {
        self.p_exact.unwrap_or(self.p_asymptotic)
    }
}

/// Signed rank test of the differences `d`: the null hypothesis is that their median is 0. `Upper` means the
/// differences tend to be positive. NaN entries are ignored.
pub fn signed_rank(d: &[f64], tail: Tail) -> SignedRank {
    let d: Vec<f64> = finite(d);
    let n_zero = d.iter().filter(|v| **v == 0.0).count();
    let nonzero: Vec<f64> = d.iter().copied().filter(|v| *v != 0.0).collect();
    let n = nonzero.len();
    let abs: Vec<f64> = nonzero.iter().map(|v| v.abs()).collect();
    let (ranks, ties) = midranks(&abs);
    let t_plus: f64 = nonzero.iter().zip(&ranks).filter(|(v, _)| **v > 0.0).map(|(_, r)| r).sum::<f64>() + 0.0;
    let total = (n * (n + 1)) as f64 / 2.0;
    let t_minus = total - t_plus;
    let nf = n as f64;
    let mean_t = nf * (nf + 1.0) / 4.0;
    let var = nf * (nf + 1.0) * (2.0 * nf + 1.0) / 24.0 - tie_term(&ties) / 48.0;
    let sd_t = if var > 0.0 { var.sqrt() } else { 0.0 };
    let n_positive = nonzero.iter().filter(|v| **v > 0.0).count();
    let (z, p_asymptotic) = if n == 0 {
        (f64::NAN, 1.0)
    } else if sd_t > 0.0 {
        let toward = if t_plus < mean_t {
            0.5
        } else if t_plus > mean_t {
            -0.5
        } else {
            0.0
        };
        let z = (t_plus - mean_t + toward) / sd_t;
        let lower = normal_cdf(z);
        (z, pick_tail(lower, 1.0 - lower, tail))
    } else {
        (0.0, 1.0)
    };
    let p_exact = if n == 0 {
        Some(1.0)
    } else if n <= SIGNED_RANK_EXACT_MAX_N {
        let doubled = as_doubled(&ranks);
        let total_doubled: usize = doubled.iter().sum();
        // Subset sums of any size: one table row, filled over all sizes.
        let mut counts = vec![0.0f64; total_doubled + 1];
        counts[0] = 1.0;
        let mut reach = 0;
        for &w in &doubled {
            reach += w;
            for s in (w..=reach).rev() {
                counts[s] += counts[s - w];
            }
        }
        let observed = (2.0 * t_plus).round() as usize;
        let (lower, upper) = tails_from_counts(&counts, observed);
        Some(pick_tail(lower, upper, tail))
    } else {
        None
    };
    SignedRank {
        n,
        n_zero,
        n_positive,
        n_negative: n - n_positive,
        t_plus,
        t_minus,
        mean_t,
        sd_t,
        z,
        p_asymptotic,
        p_exact,
        tail,
    }
}

/// One-sample signed rank test of `x` against a hypothesized median.
pub fn signed_rank_one(x: &[f64], median: f64, tail: Tail) -> SignedRank {
    let d: Vec<f64> = x.iter().map(|v| v - median).collect();
    signed_rank(&d, tail)
}

/// Paired signed rank test: the rows where both samples have a number are paired and `x - y` is tested.
pub fn signed_rank_paired(x: &[f64], y: &[f64], tail: Tail) -> SignedRank {
    let d: Vec<f64> = x.iter().zip(y).map(|(a, b)| a - b).collect();
    signed_rank(&d, tail)
}

// ----- Kruskal-Wallis -----------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RankGroup {
    pub n: usize,
    pub rank_sum: f64,
    pub mean_rank: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KruskalWallis {
    pub groups: Vec<RankGroup>,
    pub n: usize,
    /// The H statistic with the correction for ties; chi-squared distributed with `df` degrees of freedom.
    pub chi_square: f64,
    pub df: f64,
    pub p: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankTestError {
    TooFewGroups,
    EmptyGroup(usize),
}

impl std::fmt::Display for RankTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RankTestError::TooFewGroups => write!(f, "the Kruskal-Wallis test needs at least two groups"),
            RankTestError::EmptyGroup(i) => write!(f, "group {} has no numbers", i + 1),
        }
    }
}

/// Kruskal-Wallis test of the groups; NaN entries are ignored. If all numbers are equal the statistic is 0
/// and the p-value 1.
pub fn kruskal_wallis(groups: &[&[f64]]) -> Result<KruskalWallis, RankTestError> {
    if groups.len() < 2 {
        return Err(RankTestError::TooFewGroups);
    }
    let data: Vec<Vec<f64>> = groups.iter().map(|g| finite(g)).collect();
    if let Some(i) = data.iter().position(Vec::is_empty) {
        return Err(RankTestError::EmptyGroup(i));
    }
    let pooled: Vec<f64> = data.iter().flatten().copied().collect();
    let n = pooled.len();
    let (ranks, ties) = midranks(&pooled);
    let mut at = 0;
    let mut out = Vec::new();
    for g in &data {
        let sum: f64 = ranks[at..at + g.len()].iter().sum();
        at += g.len();
        out.push(RankGroup { n: g.len(), rank_sum: sum, mean_rank: sum / g.len() as f64 });
    }
    let nf = n as f64;
    let h = 12.0 / (nf * (nf + 1.0)) * out.iter().map(|g| g.rank_sum * g.rank_sum / g.n as f64).sum::<f64>()
        - 3.0 * (nf + 1.0);
    let correction = 1.0 - tie_term(&ties) / (nf * nf * nf - nf);
    let chi_square = if correction > 0.0 { (h / correction).max(0.0) } else { 0.0 };
    let df = (groups.len() - 1) as f64;
    let p = ChiSquared::new(df).map_or(f64::NAN, |d| 1.0 - d.cdf(chi_square));
    Ok(KruskalWallis { groups: out, n, chi_square, df, p })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midranks_share_the_mean_rank() {
        let (r, ties) = midranks(&[3.0, 1.0, 3.0, 2.0, 3.0]);
        assert_eq!(r, vec![4.0, 1.0, 4.0, 2.0, 4.0]);
        assert_eq!(ties, vec![3]);
    }

    #[test]
    fn mann_whitney_of_separated_samples_has_the_smallest_exact_p() {
        let m = mann_whitney(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0, 7.0], Tail::Two, true).unwrap();
        assert_eq!(m.u, 0.0);
        // 2 / C(7, 3)
        assert!((m.p_exact.unwrap() - 2.0 / 35.0).abs() < 1e-12);
        assert!((m.p() - 2.0 / 35.0).abs() < 1e-12);
    }

    #[test]
    fn all_equal_numbers_give_p_of_one_without_dividing_by_zero() {
        let m = mann_whitney(&[5.0; 4], &[5.0; 5], Tail::Two, true).unwrap();
        assert_eq!((m.z, m.p_asymptotic, m.p_exact), (0.0, 1.0, Some(1.0)));
        let s = signed_rank_one(&[3.0; 5], 3.0, Tail::Two);
        assert!(s.z.is_nan() && s.p() == 1.0 && s.n == 0 && s.n_zero == 5);
        let k = kruskal_wallis(&[&[5.0; 3], &[5.0; 4]]).unwrap();
        assert_eq!((k.chi_square, k.p), (0.0, 1.0));
    }

    #[test]
    fn empty_and_tiny_inputs_do_not_panic() {
        assert!(mann_whitney(&[], &[1.0], Tail::Two, false).is_none());
        assert!(mann_whitney(&[f64::NAN], &[1.0], Tail::Two, true).is_none());
        let m = mann_whitney(&[3.5], &[1.0, 2.0, 4.0], Tail::Lower, true).unwrap();
        assert!(m.p() > 0.0 && m.p() <= 1.0);
        let s = signed_rank_one(&[], 0.0, Tail::Two);
        assert_eq!(s.n, 0);
        assert_eq!(kruskal_wallis(&[&[1.0, 2.0]]), Err(RankTestError::TooFewGroups));
        assert_eq!(kruskal_wallis(&[&[1.0, 2.0], &[f64::NAN]]), Err(RankTestError::EmptyGroup(1)));
        // A group of one is fine.
        assert!(kruskal_wallis(&[&[1.0], &[2.0, 3.0]]).is_ok());
    }

    #[test]
    fn signed_rank_exact_p_of_all_positive_differences() {
        let s = signed_rank(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], Tail::Two);
        assert_eq!(s.t_plus, 36.0);
        assert!((s.p_exact.unwrap() - 2.0 / 256.0).abs() < 1e-12);
        let up = signed_rank(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], Tail::Upper);
        assert!((up.p_exact.unwrap() - 1.0 / 256.0).abs() < 1e-12);
    }

    #[test]
    fn nan_pairs_drop_out_of_paired_tests() {
        let s = signed_rank_paired(&[1.0, f64::NAN, 3.0, 4.0], &[0.0, 2.0, f64::NAN, 1.0], Tail::Two);
        assert_eq!(s.n, 2);
    }

    #[test]
    fn signed_rank_statistic_is_never_negative_zero() {
        let s = signed_rank(&[-1.0, -2.0, -3.0], Tail::Two);
        assert_eq!(format!("{}", s.t_plus), "0");
    }

    #[test]
    fn large_exact_tests_fall_back_to_the_asymptotic_p() {
        let x: Vec<f64> = (0..400).map(f64::from).collect();
        let y: Vec<f64> = (0..400).map(|i| f64::from(i) + 0.5).collect();
        let m = mann_whitney(&x, &y, Tail::Two, true).unwrap();
        assert!(m.p_exact.is_none());
        assert!(m.p() > 0.5);
    }
}
