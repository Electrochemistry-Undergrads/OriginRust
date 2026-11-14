//! Cross tabulation: a table of counts of two categorical variables with its row, column and total
//! percentages, the chi-square tests of independence (Pearson and likelihood ratio) and, for a table of two
//! rows and two columns, Fisher's exact test.
//!
//! The formulas are the textbook ones; the reference numbers are scipy's (`chi2_contingency`, with
//! `lambda_="log-likelihood"` for the G-test, and `fisher_exact`), see `tools/scipy_stats_compare.py`.

use statrs::distribution::{ChiSquared, ContinuousCDF};
use statrs::function::gamma::ln_gamma;

/// A table of counts with the quantities derived from it. `counts[i][j]` is the count of row level `i` and
/// column level `j`.
#[derive(Debug, Clone, PartialEq)]
pub struct Crosstab {
    pub counts: Vec<Vec<usize>>,
    pub row_totals: Vec<usize>,
    pub col_totals: Vec<usize>,
    pub total: usize,
    /// Counts expected if the two variables were independent: `row total * column total / total`.
    pub expected: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrosstabError {
    /// Fewer than two rows or two columns.
    TooSmall,
    /// The rows of the table do not all have the same length.
    Ragged,
    /// A row or a column adds up to zero, so its expected counts are zero.
    EmptyMargin,
}

impl std::fmt::Display for CrosstabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrosstabError::TooSmall => write!(f, "a cross tabulation needs at least two levels in each variable"),
            CrosstabError::Ragged => write!(f, "the rows of the table have different lengths"),
            CrosstabError::EmptyMargin => write!(f, "a row or a column of the table has no counts"),
        }
    }
}

/// A chi-square test of independence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChiSquareTest {
    pub statistic: f64,
    pub df: f64,
    pub p: f64,
}

/// Fisher's exact test of a 2 x 2 table (the first row against the second, the first column against the
/// second).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fisher {
    /// Sample odds ratio `a d / (b c)`; infinite where `b c = 0`.
    pub odds_ratio: f64,
    /// Two-sided: the probability of all tables (with the same margins) that are no more likely than this one.
    pub p_two_sided: f64,
    /// The first row has a smaller count in the first column (`P(X <= a)`).
    pub p_less: f64,
    /// The first row has a larger count in the first column (`P(X >= a)`).
    pub p_greater: f64,
}

fn chi_square_sf(x: f64, df: f64) -> f64 {
    ChiSquared::new(df).map_or(f64::NAN, |d| d.sf(x.max(0.0)))
}

impl Crosstab {
    pub fn new(counts: Vec<Vec<usize>>) -> Result<Crosstab, CrosstabError> {
        let (nr, nc) = (counts.len(), counts.first().map_or(0, Vec::len));
        if nr < 2 || nc < 2 {
            return Err(CrosstabError::TooSmall);
        }
        if counts.iter().any(|r| r.len() != nc) {
            return Err(CrosstabError::Ragged);
        }
        let row_totals: Vec<usize> = counts.iter().map(|r| r.iter().sum()).collect();
        let col_totals: Vec<usize> = (0..nc).map(|j| counts.iter().map(|r| r[j]).sum()).collect();
        if row_totals.contains(&0) || col_totals.contains(&0) {
            return Err(CrosstabError::EmptyMargin);
        }
        let total: usize = row_totals.iter().sum();
        let expected = row_totals
            .iter()
            .map(|&r| col_totals.iter().map(|&c| r as f64 * c as f64 / total as f64).collect())
            .collect();
        Ok(Crosstab { counts, row_totals, col_totals, total, expected })
    }

    pub fn rows(&self) -> usize {
        self.counts.len()
    }

    pub fn cols(&self) -> usize {
        self.col_totals.len()
    }

    /// The count as a percentage of its row total.
    pub fn row_percent(&self, i: usize, j: usize) -> f64 {
        100.0 * self.counts[i][j] as f64 / self.row_totals[i] as f64
    }

    pub fn col_percent(&self, i: usize, j: usize) -> f64 {
        100.0 * self.counts[i][j] as f64 / self.col_totals[j] as f64
    }

    pub fn total_percent(&self, i: usize, j: usize) -> f64 {
        100.0 * self.counts[i][j] as f64 / self.total as f64
    }

    /// Cells whose expected count is below 5 (the rule of thumb for trusting the chi-square approximation).
    pub fn cells_expected_below_5(&self) -> usize {
        self.expected.iter().flatten().filter(|e| **e < 5.0).count()
    }

    fn df(&self) -> f64 {
        ((self.rows() - 1) * (self.cols() - 1)) as f64
    }

    /// Pearson's chi-square: the sum of `(observed - expected)^2 / expected`.
    pub fn pearson(&self) -> ChiSquareTest {
        let mut x = 0.0;
        for (orow, erow) in self.counts.iter().zip(&self.expected) {
            for (o, e) in orow.iter().zip(erow) {
                x += (*o as f64 - e).powi(2) / e;
            }
        }
        ChiSquareTest { statistic: x, df: self.df(), p: chi_square_sf(x, self.df()) }
    }

    /// The likelihood ratio chi-square (G-test): `2 sum observed ln(observed / expected)`, empty cells adding 0.
    pub fn likelihood_ratio(&self) -> ChiSquareTest {
        let mut g = 0.0;
        for (orow, erow) in self.counts.iter().zip(&self.expected) {
            for (o, e) in orow.iter().zip(erow) {
                if *o > 0 {
                    g += *o as f64 * (*o as f64 / e).ln();
                }
            }
        }
        let g = 2.0 * g;
        ChiSquareTest { statistic: g, df: self.df(), p: chi_square_sf(g, self.df()) }
    }

    /// Pearson's chi-square of a 2 x 2 table with Yates' continuity correction (each observed count is moved
    /// by 0.5, or by less where that would pass the expected count, towards it). `None` for larger tables.
    pub fn yates(&self) -> Option<ChiSquareTest> {
        if self.rows() != 2 || self.cols() != 2 {
            return None;
        }
        let mut x = 0.0;
        for (orow, erow) in self.counts.iter().zip(&self.expected) {
            for (o, e) in orow.iter().zip(erow) {
                let diff = e - *o as f64;
                let adjusted = *o as f64 + diff.signum() * diff.abs().min(0.5);
                x += (adjusted - e).powi(2) / e;
            }
        }
        Some(ChiSquareTest { statistic: x, df: 1.0, p: chi_square_sf(x, 1.0) })
    }

    /// Fisher's exact test; `None` unless the table is 2 x 2.
    pub fn fisher(&self) -> Option<Fisher> {
        if self.rows() != 2 || self.cols() != 2 {
            return None;
        }
        let (a, b, c, d) = (self.counts[0][0], self.counts[0][1], self.counts[1][0], self.counts[1][1]);
        let odds_ratio = if b > 0 && c > 0 { (a as f64 * d as f64) / (b as f64 * c as f64) } else { f64::INFINITY };
        // X = the count in the first cell, hypergeometric with these margins.
        let (n, row1, col1) = (self.total, a + b, a + c);
        let lo = col1.saturating_sub(n - row1);
        let hi = row1.min(col1);
        let ln_choose =
            |n: usize, k: usize| ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0);
        let ln_total = ln_choose(n, col1);
        let pmf: Vec<f64> =
            (lo..=hi).map(|k| (ln_choose(row1, k) + ln_choose(n - row1, col1 - k) - ln_total).exp()).collect();
        let at = |k: usize| pmf[k - lo];
        let p_less: f64 = (lo..=a).map(at).sum::<f64>().min(1.0);
        let p_greater: f64 = (a..=hi).map(at).sum::<f64>().min(1.0);
        let observed = at(a);
        let p_two_sided: f64 = pmf.iter().filter(|p| **p <= observed * (1.0 + 1e-9)).sum::<f64>().min(1.0);
        Some(Fisher { odds_ratio, p_two_sided, p_less, p_greater })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_by_two_textbook_numbers() {
        // The tea-tasting table: 3 of 4 right each way.
        let t = Crosstab::new(vec![vec![3, 1], vec![1, 3]]).unwrap();
        let f = t.fisher().unwrap();
        assert!((f.p_two_sided - 34.0 / 70.0).abs() < 1e-12);
        assert!((f.p_greater - 17.0 / 70.0).abs() < 1e-12);
        assert!((f.odds_ratio - 9.0).abs() < 1e-12);
        let p = t.pearson();
        assert!((p.statistic - 2.0).abs() < 1e-12 && p.df == 1.0);
        assert_eq!(t.cells_expected_below_5(), 4);
        assert!((t.row_percent(0, 0) - 75.0).abs() < 1e-12);
        assert!((t.total_percent(1, 1) - 37.5).abs() < 1e-12);
    }

    #[test]
    fn empty_cells_and_margins() {
        let t = Crosstab::new(vec![vec![5, 0], vec![0, 5]]).unwrap();
        // G = 2 * 10 ln 2.
        assert!((t.likelihood_ratio().statistic - 20.0 * std::f64::consts::LN_2).abs() < 1e-12);
        assert!(t.fisher().unwrap().odds_ratio.is_infinite());
        assert_eq!(Crosstab::new(vec![vec![1, 2]]), Err(CrosstabError::TooSmall));
        assert_eq!(Crosstab::new(vec![vec![1, 2], vec![3]]), Err(CrosstabError::Ragged));
        assert_eq!(Crosstab::new(vec![vec![1, 0], vec![3, 0]]), Err(CrosstabError::EmptyMargin));
        assert!(Crosstab::new(vec![vec![1, 2, 3], vec![3, 2, 1]]).unwrap().fisher().is_none());
    }
}
