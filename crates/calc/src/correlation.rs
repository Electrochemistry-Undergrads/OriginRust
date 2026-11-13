//! Correlation coefficients: Pearson, Spearman and Kendall, with p-values and the matrix of a column set.
//!
//! The rules follow what Origin reports for `corrcoef` (see `docs/spec/statistics.md`): Pearson's r,
//! Spearman's rho (Pearson's r of the mean ranks) and Kendall's tau-b (ties counted in the denominator);
//! the p-value of the first two is the two-sided t test with `n - 2` degrees of freedom, the one of Kendall's
//! tau the two-sided normal test of the tie-corrected score. Missing values are left out pair by pair
//! (default) or row by row over all columns (listwise).

use crate::nonparam::midranks;
use statrs::distribution::{ContinuousCDF, Normal, StudentsT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrKind {
    Pearson,
    Spearman,
    Kendall,
}

impl CorrKind {
    pub const ALL: [CorrKind; 3] = [CorrKind::Pearson, CorrKind::Spearman, CorrKind::Kendall];

    pub fn label(self) -> &'static str {
        match self {
            CorrKind::Pearson => "Pearson",
            CorrKind::Spearman => "Spearman",
            CorrKind::Kendall => "Kendall",
        }
    }
}

/// A coefficient with the number of pairs it rests on and its two-sided p-value. `r` is NaN when a column is
/// constant or there are fewer than two pairs; `p` is NaN as well where the test is undefined (a coefficient
/// of exactly +1 or -1, which includes every case of two pairs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corr {
    pub n: usize,
    pub r: f64,
    pub p: f64,
}

fn pearson_r(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        let (da, db) = (a - mx, b - my);
        sxy += da * db;
        sxx += da * da;
        syy += db * db;
    }
    if sxx <= 0.0 || syy <= 0.0 {
        return f64::NAN;
    }
    (sxy / (sxx * syy).sqrt()).clamp(-1.0, 1.0)
}

/// Strictly descending pairs (i < j with v[i] > v[j]) of `v`, counted by a merge sort; `v` is sorted by the call.
fn inversions(v: &mut Vec<f64>) -> f64 {
    let n = v.len();
    let mut buf = vec![0.0; n];
    let mut count = 0.0;
    let mut width = 1;
    while width < n {
        let mut lo = 0;
        while lo < n {
            let (mid, hi) = ((lo + width).min(n), (lo + 2 * width).min(n));
            let (mut i, mut j, mut k) = (lo, mid, lo);
            while i < mid && j < hi {
                if v[i] <= v[j] {
                    buf[k] = v[i];
                    i += 1;
                } else {
                    buf[k] = v[j];
                    count += (mid - i) as f64;
                    j += 1;
                }
                k += 1;
            }
            while i < mid {
                buf[k] = v[i];
                i += 1;
                k += 1;
            }
            while j < hi {
                buf[k] = v[j];
                j += 1;
                k += 1;
            }
            lo += 2 * width;
        }
        std::mem::swap(v, &mut buf);
        width *= 2;
    }
    count
}

/// Kendall's score `S` (concordant minus discordant pairs) and the tie groups of both samples, in
/// O(n log n): after sorting by x then y, the discordant pairs are the strict inversions of y.
fn kendall_score(x: &[f64], y: &[f64]) -> (f64, Vec<usize>, Vec<usize>) {
    let n = x.len();
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&a, &b| x[a].total_cmp(&x[b]).then(y[a].total_cmp(&y[b])));
    let (tx, ty) = (midranks(x).1, midranks(y).1);
    let pairs = |g: &[usize]| g.iter().map(|&t| (t * (t - 1)) as f64 / 2.0).sum::<f64>();
    // Pairs tied in both samples: runs of equal (x, y) in the sorted order.
    let mut both = 0.0;
    let mut run = 1usize;
    for w in 1..=n {
        if w < n && x[idx[w]] == x[idx[w - 1]] && y[idx[w]] == y[idx[w - 1]] {
            run += 1;
        } else {
            both += (run * (run.max(1) - 1)) as f64 / 2.0;
            run = 1;
        }
    }
    let n0 = (n * n.saturating_sub(1)) as f64 / 2.0;
    let comparable = n0 - pairs(&tx) - pairs(&ty) + both;
    let mut ys: Vec<f64> = idx.iter().map(|&i| y[i]).collect();
    let discordant = inversions(&mut ys);
    (comparable - 2.0 * discordant, tx, ty)
}

/// The correlation of two columns; pairs with a NaN are left out.
pub fn correlation(x: &[f64], y: &[f64], kind: CorrKind) -> Corr {
    let (xs, ys): (Vec<f64>, Vec<f64>) =
        x.iter().zip(y).filter(|(a, b)| !a.is_nan() && !b.is_nan()).map(|(a, b)| (*a, *b)).unzip();
    correlation_of_pairs(&xs, &ys, kind)
}

fn correlation_of_pairs(x: &[f64], y: &[f64], kind: CorrKind) -> Corr {
    let n = x.len();
    let undefined = Corr { n, r: f64::NAN, p: f64::NAN };
    if n < 2 {
        return undefined;
    }
    let nf = n as f64;
    let (r, p) = match kind {
        CorrKind::Pearson | CorrKind::Spearman => {
            let r = if kind == CorrKind::Pearson { pearson_r(x, y) } else { pearson_r(&midranks(x).0, &midranks(y).0) };
            let p = if r.is_nan() || n < 3 || r.abs() > 1.0 - 1e-14 {
                f64::NAN
            } else {
                let t = r * ((nf - 2.0) / (1.0 - r * r)).sqrt();
                StudentsT::new(0.0, 1.0, nf - 2.0).map_or(f64::NAN, |d| 2.0 * d.cdf(-t.abs()))
            };
            (r, p)
        }
        CorrKind::Kendall => {
            let (s, tx, ty) = kendall_score(x, y);
            let pairs = nf * (nf - 1.0) / 2.0;
            let tied = |g: &[usize]| g.iter().map(|&t| (t * (t - 1)) as f64 / 2.0).sum::<f64>();
            let denominator = ((pairs - tied(&tx)) * (pairs - tied(&ty))).sqrt();
            if denominator <= 0.0 {
                return undefined;
            }
            let r = (s / denominator).clamp(-1.0, 1.0);
            let sum = |g: &[usize], f: &dyn Fn(f64) -> f64| g.iter().map(|&t| f(t as f64)).sum::<f64>();
            let v0 = nf * (nf - 1.0) * (2.0 * nf + 5.0);
            let vt = sum(&tx, &|t| t * (t - 1.0) * (2.0 * t + 5.0));
            let vu = sum(&ty, &|t| t * (t - 1.0) * (2.0 * t + 5.0));
            let v1 = sum(&tx, &|t| t * (t - 1.0)) * sum(&ty, &|t| t * (t - 1.0));
            let v2 = sum(&tx, &|t| t * (t - 1.0) * (t - 2.0)) * sum(&ty, &|t| t * (t - 1.0) * (t - 2.0));
            let mut var = (v0 - vt - vu) / 18.0 + v1 / (2.0 * nf * (nf - 1.0));
            if n > 2 {
                var += v2 / (9.0 * nf * (nf - 1.0) * (nf - 2.0));
            }
            let p = if r.abs() > 1.0 - 1e-14 || var <= 0.0 {
                f64::NAN
            } else {
                Normal::new(0.0, 1.0).map_or(f64::NAN, |d| 2.0 * d.cdf(-(s / var.sqrt()).abs()))
            };
            (r, p)
        }
    };
    Corr { n, r, p }
}

/// The correlations of every pair of columns. `get(i, j)` is the result for columns `i` and `j`; the diagonal
/// has `r = 1` and no p-value (NaN), or `r = NaN` for a column without two different numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrMatrix {
    pub kind: CorrKind,
    pub size: usize,
    pub cells: Vec<Corr>,
}

impl CorrMatrix {
    pub fn get(&self, i: usize, j: usize) -> Corr {
        self.cells[i * self.size + j]
    }
}

/// Correlation matrix of the columns. With `listwise` only the rows where every column has a number are used;
/// otherwise each pair of columns uses the rows where both have one.
pub fn correlation_matrix(columns: &[&[f64]], kind: CorrKind, listwise: bool) -> CorrMatrix {
    let k = columns.len();
    let rows = columns.iter().map(|c| c.len()).max().unwrap_or(0);
    let value = |c: usize, i: usize| columns[c].get(i).copied().unwrap_or(f64::NAN);
    let complete: Vec<usize> = (0..rows).filter(|&i| (0..k).all(|c| !value(c, i).is_nan())).collect();
    let mut cells = Vec::with_capacity(k * k);
    for i in 0..k {
        for j in 0..k {
            let cell = if i == j {
                let own: Vec<f64> = if listwise {
                    complete.iter().map(|&row| value(i, row)).collect()
                } else {
                    (0..rows).map(|row| value(i, row)).filter(|v| !v.is_nan()).collect()
                };
                let spread = own.iter().any(|v| *v != own[0]);
                Corr { n: own.len(), r: if own.len() >= 2 && spread { 1.0 } else { f64::NAN }, p: f64::NAN }
            } else if listwise {
                let xs: Vec<f64> = complete.iter().map(|&row| value(i, row)).collect();
                let ys: Vec<f64> = complete.iter().map(|&row| value(j, row)).collect();
                correlation_of_pairs(&xs, &ys, kind)
            } else {
                let xs: Vec<f64> = (0..rows).map(|row| value(i, row)).collect();
                let ys: Vec<f64> = (0..rows).map(|row| value(j, row)).collect();
                correlation(&xs, &ys, kind)
            };
            cells.push(cell);
        }
    }
    CorrMatrix { kind, size: k, cells }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_and_reversed_relations_have_no_p_value() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let up = [2.0, 4.0, 6.0, 8.0, 10.0];
        let down = [5.0, 4.0, 3.0, 2.0, 1.0];
        for kind in CorrKind::ALL {
            let a = correlation(&x, &up, kind);
            assert!((a.r - 1.0).abs() < 1e-12 && a.p.is_nan(), "{kind:?}");
            let b = correlation(&x, &down, kind);
            assert!((b.r + 1.0).abs() < 1e-12 && b.p.is_nan(), "{kind:?}");
        }
    }

    #[test]
    fn constant_columns_and_short_columns_give_nan_without_panicking() {
        for kind in CorrKind::ALL {
            assert!(correlation(&[1.0, 2.0, 3.0], &[4.0; 3], kind).r.is_nan());
            assert!(correlation(&[1.0], &[2.0], kind).r.is_nan());
            assert!(correlation(&[], &[], kind).r.is_nan());
            assert!(correlation(&[f64::NAN, 1.0], &[2.0, f64::NAN], kind).r.is_nan());
            // Two pairs: r is +-1 and there is no test.
            let two = correlation(&[1.0, 2.0], &[2.0, 5.0], kind);
            assert!(two.r == 1.0 && two.p.is_nan() && two.n == 2);
        }
    }

    #[test]
    fn matrix_diagonal_and_missing_values() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [2.0, 1.0, 4.0, f64::NAN, 6.0, 5.0];
        let c = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0];
        let pair = correlation_matrix(&[&a, &b, &c], CorrKind::Pearson, false);
        assert_eq!(pair.get(0, 0).r, 1.0);
        assert_eq!(pair.get(0, 1).n, 5);
        assert_eq!(pair.get(0, 2).n, 6);
        let list = correlation_matrix(&[&a, &b, &c], CorrKind::Pearson, true);
        assert_eq!(list.get(0, 2).n, 5);
        assert_eq!(pair.get(1, 0), correlation(&b, &a, CorrKind::Pearson));
        assert!((pair.get(0, 1).r - pair.get(1, 0).r).abs() < 1e-15);
    }

    #[test]
    fn kendall_fast_score_matches_the_pair_loop_with_ties() {
        let mut seed = 12345u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 33) % 9) as f64
        };
        for n in [2usize, 3, 10, 57] {
            let x: Vec<f64> = (0..n).map(|_| next()).collect();
            let y: Vec<f64> = (0..n).map(|_| next()).collect();
            let mut s = 0.0;
            for i in 0..n {
                for j in i + 1..n {
                    let (dx, dy) = (x[i] - x[j], y[i] - y[j]);
                    if dx != 0.0 && dy != 0.0 {
                        s += if (dx > 0.0) == (dy > 0.0) { 1.0 } else { -1.0 };
                    }
                }
            }
            assert_eq!(kendall_score(&x, &y).0, s, "n = {n}");
        }
    }

    #[test]
    fn kendall_of_many_rows_is_fast() {
        let n = 200_000usize;
        let x: Vec<f64> = (0..n).map(|i| ((i * 7919) % 10007) as f64).collect();
        let y: Vec<f64> = (0..n).map(|i| ((i * 104729) % 10009) as f64).collect();
        let c = correlation(&x, &y, CorrKind::Kendall);
        assert!(c.r.is_finite());
    }

    #[test]
    fn kendall_and_spearman_of_a_small_case() {
        let x = [1.0, 2.0, 4.0];
        let y = [2.0, 1.0, 5.0];
        let k = correlation(&x, &y, CorrKind::Kendall);
        assert!((k.r - 1.0 / 3.0).abs() < 1e-12);
        let s = correlation(&x, &y, CorrKind::Spearman);
        assert!((s.r - 0.5).abs() < 1e-12);
    }
}
