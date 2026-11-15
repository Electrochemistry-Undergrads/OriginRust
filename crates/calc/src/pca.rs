//! Principal component analysis of the columns of a data table (Statistics > Multivariate Analysis >
//! Principal Component Analysis).
//!
//! What Origin's `pca` X-Function does, measured on synthetic data (`docs/spec/statistics.md`, section 17):
//!
//! - The analysis is of the correlation matrix (the default) or of the covariance matrix (divisor n - 1).
//! - Missing values are excluded listwise (a row with a missing number in any column is left out) or
//!   pairwise (each matrix entry uses the rows where both columns have a number, with the means of those
//!   rows). Descriptive statistics (N, missing, mean, SD) are per column over the rows that are used:
//!   all rows with a number for pairwise, the complete rows for listwise.
//! - Eigenvalues are in descending order; percent of variance is `100 * eigenvalue / sum`.
//! - The sign of an eigenvector is arbitrary; Origin makes the **sum of the components of each eigenvector
//!   positive** (measured in 8 data sets with 4 to 5 variables), so does this code.
//! - Scores are `(x - mean)` (covariance) or `(x - mean) / SD` (correlation) times the eigenvectors. With
//!   "standardize scores" each score column is divided by the square root of its eigenvalue (unit
//!   variance). Rows that are not used have NaN scores.
//! - For a covariance analysis Origin also tests, for each k, that the last `p - k` eigenvalues are equal
//!   (Bartlett's test: chi-square `(n - 1 - (2p + 5)/6) (q ln(mean) - sum ln(l))`, `q` eigenvalues left,
//!   `(q + 2)(q - 1)/2` degrees of freedom; the last row is 0, 0, 0). `n` is the largest column count.

use nalgebra::{DMatrix, SymmetricEigen};
use statrs::distribution::{ChiSquared, ContinuousCDF};

/// How a principal component analysis is done.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcaOptions {
    /// Analyze the correlation matrix (true) or the covariance matrix.
    pub correlation: bool,
    /// Exclude missing values pairwise (true) instead of listwise.
    pub pairwise: bool,
    /// Number of components whose loadings and scores are kept (at most the number of columns).
    pub components: usize,
    /// Divide each score column by the square root of its eigenvalue.
    pub standardize_scores: bool,
}

impl Default for PcaOptions {
    fn default() -> Self {
        PcaOptions { correlation: true, pairwise: false, components: 2, standardize_scores: false }
    }
}

/// Why an analysis could not be done.
#[derive(Debug, Clone, PartialEq)]
pub enum PcaError {
    /// Fewer than two columns.
    TooFewColumns,
    /// Fewer than two usable rows (or a pair of columns with fewer than two common rows).
    TooFewRows,
    /// A column (0-based) has no spread, so its correlation is not defined.
    NoSpread(usize),
    /// A number is infinite.
    NotFinite,
}

impl std::fmt::Display for PcaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PcaError::TooFewColumns => write!(f, "a principal component analysis needs at least two columns"),
            PcaError::TooFewRows => write!(f, "too few rows with numbers (at least two are needed)"),
            PcaError::NoSpread(c) => write!(f, "column {} has no spread, so its correlation is not defined", c + 1),
            PcaError::NotFinite => write!(f, "a value is infinite"),
        }
    }
}

impl std::error::Error for PcaError {}

/// Test that the eigenvalues from one on down are equal (covariance analysis only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EigenTest {
    pub chi_square: f64,
    pub df: f64,
    pub p: f64,
}

/// The result of [`pca`]; `p` is the number of columns, `n` the number of rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Pca {
    /// Per column: numbers used, missing numbers in the column, mean and standard deviation.
    pub count: Vec<usize>,
    pub missing: Vec<usize>,
    pub mean: Vec<f64>,
    pub sd: Vec<f64>,
    /// The correlation matrix (also for a covariance analysis), `p x p`.
    pub correlation: Vec<Vec<f64>>,
    /// The matrix that was decomposed (correlation or covariance), `p x p`.
    pub matrix: Vec<Vec<f64>>,
    /// Eigenvalues, largest first, and the percent of the total and the running percent.
    pub eigenvalues: Vec<f64>,
    pub percent: Vec<f64>,
    pub cumulative: Vec<f64>,
    /// Eigenvectors of the first `components` components: `loadings[c][v]` is the weight of column `v` in
    /// component `c`.
    pub loadings: Vec<Vec<f64>>,
    /// `scores[c][row]` for the kept components; NaN for rows that were not used.
    pub scores: Vec<Vec<f64>>,
    /// Per eigenvalue, the test that it and all smaller ones are equal; present for covariance analyses.
    pub tests: Option<Vec<EigenTest>>,
}

fn mean_sd(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    let ss = v.iter().map(|x| (x - m) * (x - m)).sum::<f64>();
    (m, if v.len() > 1 { (ss / (n - 1.0)).sqrt() } else { f64::NAN })
}

/// Covariance and correlation of two equally long slices (divisor n - 1). NaN correlation without spread.
fn cov_corr(a: &[f64], b: &[f64]) -> (f64, f64) {
    let (ma, sa) = mean_sd(a);
    let (mb, sb) = mean_sd(b);
    let n = a.len() as f64;
    let c = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum::<f64>() / (n - 1.0);
    (c, c / (sa * sb))
}

/// Principal component analysis of `columns` (each a column of the table, NaN for a missing value; shorter
/// columns count as missing below their end).
pub fn pca(columns: &[Vec<f64>], options: &PcaOptions) -> Result<Pca, PcaError> {
    let p = columns.len();
    if p < 2 {
        return Err(PcaError::TooFewColumns);
    }
    let rows = columns.iter().map(Vec::len).max().unwrap_or(0);
    let at = |c: usize, r: usize| columns[c].get(r).copied().unwrap_or(f64::NAN);
    if columns.iter().flatten().any(|v| v.is_infinite()) {
        return Err(PcaError::NotFinite);
    }
    let complete: Vec<usize> = (0..rows).filter(|&r| (0..p).all(|c| at(c, r).is_finite())).collect();
    let missing: Vec<usize> = (0..p).map(|c| (0..rows).filter(|&r| !at(c, r).is_finite()).count()).collect();
    // The rows each column's descriptive statistics are taken over.
    let used: Vec<Vec<f64>> = (0..p)
        .map(|c| {
            let rs: Vec<usize> =
                if options.pairwise { (0..rows).filter(|&r| at(c, r).is_finite()).collect() } else { complete.clone() };
            rs.into_iter().map(|r| at(c, r)).collect()
        })
        .collect();
    if used.iter().any(|u| u.len() < 2) {
        return Err(PcaError::TooFewRows);
    }
    let count: Vec<usize> = used.iter().map(Vec::len).collect();
    let (mean, sd): (Vec<f64>, Vec<f64>) = used.iter().map(|u| mean_sd(u)).unzip();

    let mut cov = vec![vec![0.0; p]; p];
    let mut cor = vec![vec![0.0; p]; p];
    for i in 0..p {
        for j in 0..=i {
            let (a, b): (Vec<f64>, Vec<f64>) = if options.pairwise {
                (0..rows)
                    .filter(|&r| at(i, r).is_finite() && at(j, r).is_finite())
                    .map(|r| (at(i, r), at(j, r)))
                    .unzip()
            } else {
                complete.iter().map(|&r| (at(i, r), at(j, r))).unzip()
            };
            if a.len() < 2 {
                return Err(PcaError::TooFewRows);
            }
            let (c, r) = cov_corr(&a, &b);
            if i == j {
                cov[i][i] = c;
                cor[i][i] = 1.0;
                if c <= 0.0 || !c.is_finite() {
                    return Err(PcaError::NoSpread(i));
                }
            } else {
                if !r.is_finite() {
                    return Err(PcaError::NoSpread(if sd[i] > 0.0 { j } else { i }));
                }
                cov[i][j] = c;
                cov[j][i] = c;
                cor[i][j] = r;
                cor[j][i] = r;
            }
        }
    }
    let matrix = if options.correlation { cor.clone() } else { cov };
    let eig = SymmetricEigen::new(DMatrix::from_fn(p, p, |i, j| matrix[i][j]));
    let mut order: Vec<usize> = (0..p).collect();
    order.sort_by(|&a, &b| eig.eigenvalues[b].total_cmp(&eig.eigenvalues[a]));
    let eigenvalues: Vec<f64> = order.iter().map(|&k| eig.eigenvalues[k]).collect();
    let total: f64 = eigenvalues.iter().sum();
    let percent: Vec<f64> = eigenvalues.iter().map(|l| 100.0 * l / total).collect();
    let cumulative: Vec<f64> = percent
        .iter()
        .scan(0.0, |acc, x| {
            *acc += x;
            Some(*acc)
        })
        .collect();
    let keep = options.components.clamp(1, p);
    let loadings: Vec<Vec<f64>> = order[..keep]
        .iter()
        .map(|&k| {
            let mut v: Vec<f64> = (0..p).map(|i| eig.eigenvectors[(i, k)]).collect();
            if v.iter().sum::<f64>() < 0.0 {
                v.iter_mut().for_each(|x| *x = -*x);
            }
            v
        })
        .collect();
    let scores: Vec<Vec<f64>> = loadings
        .iter()
        .enumerate()
        .map(|(k, v)| {
            let scale = if options.standardize_scores { eigenvalues[k].sqrt() } else { 1.0 };
            let mut s = vec![f64::NAN; rows];
            for &r in &complete {
                let z: f64 = (0..p)
                    .map(|c| {
                        let d = at(c, r) - mean[c];
                        (if options.correlation { d / sd[c] } else { d }) * v[c]
                    })
                    .sum();
                s[r] = z / scale;
            }
            s
        })
        .collect();
    let tests = (!options.correlation).then(|| {
        let n = *count.iter().max().expect("p >= 2") as f64;
        (0..p)
            .map(|k| {
                let rest = &eigenvalues[k..];
                let q = rest.len() as f64;
                if rest.len() < 2 {
                    return EigenTest { chi_square: 0.0, df: 0.0, p: 0.0 };
                }
                let logs: f64 = rest.iter().map(|l| l.ln()).sum();
                let chi = (n - 1.0 - (2.0 * p as f64 + 5.0) / 6.0) * (q * (rest.iter().sum::<f64>() / q).ln() - logs);
                let df = (q + 2.0) * (q - 1.0) / 2.0;
                let pv = if chi.is_finite() {
                    ChiSquared::new(df).map_or(f64::NAN, |d| d.sf(chi.max(0.0)))
                } else {
                    f64::NAN
                };
                EigenTest { chi_square: chi, df, p: pv }
            })
            .collect()
    });
    Ok(Pca {
        count,
        missing,
        mean,
        sd,
        correlation: cor,
        matrix,
        eigenvalues,
        percent,
        cumulative,
        loadings,
        scores,
        tests,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} vs {b}");
    }

    /// x = (1,2,3,4), y = (2,4,5,9): means 2.5 and 5, variances 5/3 and 26/3, covariance 11/3, so the
    /// correlation is r = (11/3) / sqrt(5/3 * 26/3). The correlation matrix [[1, r], [r, 1]] has the
    /// eigenvalues 1 + r and 1 - r and the first eigenvector (1, 1) / sqrt 2.
    #[test]
    fn two_columns_correlation_by_hand() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![2.0, 4.0, 5.0, 9.0];
        let r = (11.0 / 3.0) / ((5.0f64 / 3.0) * (26.0 / 3.0)).sqrt();
        let a = pca(&[x.clone(), y.clone()], &PcaOptions { components: 2, ..Default::default() }).unwrap();
        near(a.correlation[0][1], r, 1e-12);
        near(a.eigenvalues[0], 1.0 + r, 1e-12);
        near(a.eigenvalues[1], 1.0 - r, 1e-12);
        near(a.percent[0], 100.0 * (1.0 + r) / 2.0, 1e-10);
        near(a.cumulative[1], 100.0, 1e-10);
        let s = 0.5f64.sqrt();
        near(a.loadings[0][0], s, 1e-12);
        near(a.loadings[0][1], s, 1e-12);
        near(a.mean[1], 5.0, 1e-12);
        near(a.sd[0], (5.0f64 / 3.0).sqrt(), 1e-12);
        // The first score of row 1: ((1 - 2.5) / sd_x + (2 - 5) / sd_y) / sqrt 2.
        let want = ((1.0 - 2.5) / a.sd[0] + (2.0 - 5.0) / a.sd[1]) * s;
        near(a.scores[0][0], want, 1e-12);
        assert!(a.tests.is_none());
        // The scores have the variance of their eigenvalue; standardized, variance 1.
        let var = |v: &[f64]| {
            let m = v.iter().sum::<f64>() / 4.0;
            v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / 3.0
        };
        near(var(&a.scores[0]), a.eigenvalues[0], 1e-12);
        let st = pca(&[x, y], &PcaOptions { standardize_scores: true, ..Default::default() }).unwrap();
        near(var(&st.scores[1]), 1.0, 1e-12);
    }

    /// Orthogonal columns x = (-2,-2,2,2) and y = (-1,1,-1,1): covariance 0, variances 16/3 and 4/3, so those
    /// are the eigenvalues and the first component is column x alone.
    #[test]
    fn covariance_of_orthogonal_columns() {
        let x = vec![-2.0, -2.0, 2.0, 2.0];
        let y = vec![-1.0, 1.0, -1.0, 1.0];
        let a = pca(&[x, y], &PcaOptions { correlation: false, components: 2, ..Default::default() }).unwrap();
        near(a.eigenvalues[0], 16.0 / 3.0, 1e-12);
        near(a.eigenvalues[1], 4.0 / 3.0, 1e-12);
        near(a.loadings[0][0], 1.0, 1e-12);
        near(a.scores[0][3], 2.0, 1e-12);
        // Bartlett: n = 4, p = 2, factor 4 - 1 - 9/6 = 1.5; q = 2: 2 ln(mean) - sum ln = 2 ln(10/3) - ln(64/9).
        let t = a.tests.as_ref().unwrap();
        let want = 1.5 * (2.0 * (10.0f64 / 3.0).ln() - (64.0f64 / 9.0).ln());
        near(t[0].chi_square, want, 1e-12);
        near(t[0].df, 2.0, 0.0);
        near(t[1].df, 0.0, 0.0);
    }

    #[test]
    fn missing_values_listwise_and_pairwise() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = vec![2.0, 1.0, f64::NAN, 5.0, 4.0, 7.0];
        let z = vec![1.0, 3.0, 2.0, 5.0, f64::NAN, 4.0];
        let cols = [x, y, z];
        let l = pca(&cols, &PcaOptions::default()).unwrap();
        assert_eq!(l.count, vec![4, 4, 4]);
        assert_eq!(l.missing, vec![0, 1, 1]);
        assert!(l.scores[0][2].is_nan() && l.scores[0][4].is_nan() && l.scores[0][0].is_finite());
        let w = pca(&cols, &PcaOptions { pairwise: true, ..Default::default() }).unwrap();
        assert_eq!(w.count, vec![6, 5, 5]);
        // Pairwise: the correlation of x and y uses the 5 rows where both exist.
        let (xs, ys): (Vec<f64>, Vec<f64>) =
            [(1.0, 2.0), (2.0, 1.0), (4.0, 5.0), (5.0, 4.0), (6.0, 7.0)].into_iter().unzip();
        near(w.correlation[0][1], cov_corr(&xs, &ys).1, 1e-12);
        // The trace of a correlation matrix is the number of columns.
        near(w.eigenvalues.iter().sum::<f64>(), 3.0, 1e-12);
    }

    #[test]
    fn errors() {
        assert_eq!(pca(&[vec![1.0, 2.0]], &PcaOptions::default()), Err(PcaError::TooFewColumns));
        let e = pca(&[vec![1.0, 2.0, 3.0], vec![5.0, 5.0, 5.0]], &PcaOptions::default());
        assert_eq!(e, Err(PcaError::NoSpread(1)));
        assert_eq!(pca(&[vec![1.0], vec![2.0]], &PcaOptions::default()), Err(PcaError::TooFewRows));
    }
}
