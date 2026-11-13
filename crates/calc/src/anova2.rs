//! Two-way analysis of variance, with or without interaction, for balanced and unbalanced designs.
//!
//! What Origin reports for "Two-Way ANOVA" (see `docs/spec/statistics.md`): the sum of squares of a
//! term is the increase of the residual sum of squares when the term is left out of the full model, each
//! factor coded by sum-to-zero (effect) columns. For a balanced design that is the textbook table; for
//! unequal cell sizes it is the "type III" sum of squares (every term adjusted for all the others), and
//! without an interaction it equals the type II sum of squares. The error term is the residual of the full
//! model. Means comparisons use the marginal means of each factor level (and of each cell for the
//! interaction), the error mean square of the table and its degrees of freedom.

use crate::anova::{Comparison, MeansComparison, compare_means};
use crate::hypothesis::SampleStats;
use statrs::distribution::{ContinuousCDF, FisherSnedecor};

/// One row of the two-way table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoWayRow {
    pub df: f64,
    pub sum_sq: f64,
    pub mean_sq: f64,
    /// NaN where the row has no F test (the error and total rows, or no error degrees of freedom).
    pub f: f64,
    pub p: f64,
}

/// Why a two-way ANOVA could not be computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoWayError {
    /// Origin needs at least four numbers.
    TooFewData(usize),
    /// A factor has fewer than two levels.
    TooFewLevels,
    /// The three inputs do not have the same length.
    LengthMismatch,
    /// A factor level has no data (it was numbered but never used).
    EmptyLevel,
    /// With an interaction, every combination of the two factors needs data.
    EmptyCell { a: usize, b: usize },
    /// Without an interaction the levels of the factors must be connected through the data.
    Disconnected,
}

impl std::fmt::Display for TwoWayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwoWayError::TooFewData(n) => write!(f, "a two-way ANOVA needs at least 4 numbers, there are {n}"),
            TwoWayError::TooFewLevels => write!(f, "each factor needs at least two levels"),
            TwoWayError::LengthMismatch => write!(f, "the factor columns and the data column differ in length"),
            TwoWayError::EmptyLevel => write!(f, "a factor level has no data"),
            TwoWayError::EmptyCell { a, b } => write!(
                f,
                "the combination of level {} of factor A and level {} of factor B has no data; with an interaction \
                 every combination needs data (Origin does not support empty cells either), so switch the \
                 interaction off or fill the cell",
                a + 1,
                b + 1
            ),
            TwoWayError::Disconnected => {
                write!(f, "the data do not connect the levels of the two factors, so the effects cannot be separated")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TwoWayAnova {
    pub interaction: bool,
    /// Descriptive statistics of each level of factor A and of factor B, of each cell (factor A changes
    /// slowest: A1B1, A1B2, ...) and of all the data.
    pub levels_a: Vec<SampleStats>,
    pub levels_b: Vec<SampleStats>,
    pub cells: Vec<SampleStats>,
    pub overall: SampleStats,
    pub factor_a: TwoWayRow,
    pub factor_b: TwoWayRow,
    pub interaction_row: Option<TwoWayRow>,
    pub model: TwoWayRow,
    pub error: TwoWayRow,
    pub total: TwoWayRow,
    pub r_squared: f64,
    pub root_mse: f64,
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Residual sum of squares and rank of the least-squares fit of `y` on the columns `cols`
/// (Gram-Schmidt with a second pass; columns that depend on earlier ones add nothing to the rank).
fn fit(cols: &[Vec<f64>], y: &[f64]) -> (f64, usize) {
    let mut basis: Vec<Vec<f64>> = Vec::new();
    for c in cols {
        let mut v = c.clone();
        let norm0 = dot(&v, &v).sqrt();
        if norm0 == 0.0 {
            continue;
        }
        for _ in 0..2 {
            for q in &basis {
                let d = dot(q, &v);
                v.iter_mut().zip(q).for_each(|(x, qx)| *x -= d * qx);
            }
        }
        let norm = dot(&v, &v).sqrt();
        if norm > 1e-9 * norm0 {
            v.iter_mut().for_each(|x| *x /= norm);
            basis.push(v);
        }
    }
    let mut r = y.to_vec();
    for q in &basis {
        let d = dot(q, &r);
        r.iter_mut().zip(q).for_each(|(x, qx)| *x -= d * qx);
    }
    (dot(&r, &r), basis.len())
}

/// Sum-to-zero columns of a factor with `levels` levels: level `l` of the first `levels - 1` gets 1, the last
/// level gets -1, the others 0.
fn effect_columns(factor: &[usize], levels: usize) -> Vec<Vec<f64>> {
    (0..levels - 1)
        .map(|l| {
            factor
                .iter()
                .map(|&x| {
                    if x == l {
                        1.0
                    } else if x == levels - 1 {
                        -1.0
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect()
}

fn f_test(ss: f64, df: f64, mse: f64, dfe: f64) -> (f64, f64) {
    if df <= 0.0 || dfe <= 0.0 || mse.is_nan() {
        return (f64::NAN, f64::NAN);
    }
    let ms = ss / df;
    if mse == 0.0 {
        return if ms > 0.0 { (f64::INFINITY, 0.0) } else { (f64::NAN, f64::NAN) };
    }
    let f = ms / mse;
    let p = FisherSnedecor::new(df, dfe).map(|d| 1.0 - d.cdf(f)).unwrap_or(f64::NAN);
    (f, p)
}

/// Two-way ANOVA of `y` by the level numbers `a` and `b` (counted from 0; the number of levels is the
/// largest number plus one). Entries where `y` is NaN are ignored.
pub fn anova_two_way(a: &[usize], b: &[usize], y: &[f64], interaction: bool) -> Result<TwoWayAnova, TwoWayError> {
    if a.len() != y.len() || b.len() != y.len() {
        return Err(TwoWayError::LengthMismatch);
    }
    let keep: Vec<usize> = (0..y.len()).filter(|&i| !y[i].is_nan()).collect();
    let (a, b, y): (Vec<usize>, Vec<usize>, Vec<f64>) = (
        keep.iter().map(|&i| a[i]).collect(),
        keep.iter().map(|&i| b[i]).collect(),
        keep.iter().map(|&i| y[i]).collect(),
    );
    let n = y.len();
    if n < 4 {
        return Err(TwoWayError::TooFewData(n));
    }
    let (na, nb) = (a.iter().max().map_or(0, |m| m + 1), b.iter().max().map_or(0, |m| m + 1));
    if na < 2 || nb < 2 {
        return Err(TwoWayError::TooFewLevels);
    }
    let group = |sel: &dyn Fn(usize) -> usize, count: usize| -> Vec<SampleStats> {
        (0..count)
            .map(|l| SampleStats::of(&(0..n).filter(|&i| sel(i) == l).map(|i| y[i]).collect::<Vec<f64>>()))
            .collect()
    };
    let levels_a = group(&|i| a[i], na);
    let levels_b = group(&|i| b[i], nb);
    let cells = group(&|i| a[i] * nb + b[i], na * nb);
    if levels_a.iter().chain(&levels_b).any(|s| s.n == 0) {
        return Err(TwoWayError::EmptyLevel);
    }
    if interaction && let Some(c) = cells.iter().position(|s| s.n == 0) {
        return Err(TwoWayError::EmptyCell { a: c / nb, b: c % nb });
    }

    let ca = effect_columns(&a, na);
    let cb = effect_columns(&b, nb);
    let cab: Vec<Vec<f64>> =
        ca.iter().flat_map(|p| cb.iter().map(move |q| p.iter().zip(q).map(|(x, y)| x * y).collect())).collect();
    let ones = vec![vec![1.0; n]];
    let with = |parts: &[&[Vec<f64>]]| -> Vec<Vec<f64>> {
        let mut cols = ones.clone();
        for p in parts {
            cols.extend(p.iter().cloned());
        }
        cols
    };
    let empty: &[Vec<f64>] = &[];
    let full_cols = if interaction { with(&[&ca, &cb, &cab]) } else { with(&[&ca, &cb]) };
    let (sse, rank) = fit(&full_cols, &y);
    let expected_rank = if interaction { na * nb } else { na + nb - 1 };
    if rank < expected_rank {
        return Err(TwoWayError::Disconnected);
    }
    let (sse_no_a, rank_no_a) = fit(&if interaction { with(&[empty, &cb, &cab]) } else { with(&[&cb]) }, &y);
    let (sse_no_b, rank_no_b) = fit(&if interaction { with(&[&ca, empty, &cab]) } else { with(&[&ca]) }, &y);

    let overall = SampleStats::of(&y);
    let sst: f64 = y.iter().map(|v| (v - overall.mean).powi(2)).sum();
    let sse = if sse <= 1e-20 * y.iter().map(|v| v * v).sum::<f64>() { 0.0 } else { sse };
    let (df_error, df_model) = ((n - rank) as f64, (rank - 1) as f64);
    let mse = if df_error > 0.0 { sse / df_error } else { f64::NAN };
    // Sums of squares below rounding noise (constant data) count as exactly zero.
    let noise = 1e-20 * y.iter().map(|v| v * v).sum::<f64>();
    let row = |ss: f64, df: f64| {
        let ss = if ss <= noise { 0.0 } else { ss };
        let (f, p) = f_test(ss, df, mse, df_error);
        TwoWayRow { df, sum_sq: ss, mean_sq: if df > 0.0 { ss / df } else { f64::NAN }, f, p }
    };
    let factor_a = row(sse_no_a - sse, (rank - rank_no_a) as f64);
    let factor_b = row(sse_no_b - sse, (rank - rank_no_b) as f64);
    let interaction_row = interaction.then(|| {
        let (sse_no_ab, rank_no_ab) = fit(&with(&[&ca, &cb]), &y);
        row(sse_no_ab - sse, (rank - rank_no_ab) as f64)
    });
    let model = row(sst - sse, df_model);
    let error = TwoWayRow { df: df_error, sum_sq: sse.max(0.0), mean_sq: mse, f: f64::NAN, p: f64::NAN };
    let total =
        TwoWayRow { df: n as f64 - 1.0, sum_sq: sst, mean_sq: sst / (n as f64 - 1.0), f: f64::NAN, p: f64::NAN };
    Ok(TwoWayAnova {
        interaction,
        levels_a,
        levels_b,
        cells,
        overall,
        factor_a,
        factor_b,
        interaction_row,
        model,
        error,
        total,
        r_squared: if sst > 0.0 { (sst - sse) / sst } else { f64::NAN },
        root_mse: mse.sqrt(),
    })
}

impl TwoWayAnova {
    fn compare(&self, stats: &[SampleStats], method: MeansComparison, alpha: f64) -> Vec<Comparison> {
        let levels: Vec<(usize, f64)> = stats.iter().map(|s| (s.n, s.mean)).collect();
        compare_means(&levels, self.error.mean_sq, self.error.df, method, alpha)
    }

    /// Pairwise comparison of the levels of factor A.
    pub fn compare_a(&self, method: MeansComparison, alpha: f64) -> Vec<Comparison> {
        self.compare(&self.levels_a, method, alpha)
    }

    /// Pairwise comparison of the levels of factor B.
    pub fn compare_b(&self, method: MeansComparison, alpha: f64) -> Vec<Comparison> {
        self.compare(&self.levels_b, method, alpha)
    }

    /// Pairwise comparison of the cells (every combination of a level of A and a level of B), as Origin does
    /// when the interaction is on.
    ///
    /// With more than `MAX_CELL_COMPARISON_CELLS` cells the list is left out: the pairs grow with the square
    /// of the cells and the studentized range probabilities of Tukey's method would take minutes.
    pub fn compare_cells(&self, method: MeansComparison, alpha: f64) -> Vec<Comparison> {
        if self.cells.len() > MAX_CELL_COMPARISON_CELLS {
            return Vec::new();
        }
        self.compare(&self.cells, method, alpha)
    }
}

/// The most cells whose pairwise comparison is listed.
pub const MAX_CELL_COMPARISON_CELLS: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_design_with_many_cells_skips_the_cell_comparison_instead_of_stalling() {
        let (mut a, mut b, mut y) = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..9 {
            for j in 0..9 {
                for r in 0..3 {
                    a.push(i);
                    b.push(j);
                    y.push((i * 3 + j * 5 + r * 7 % 4) as f64 + 0.1 * r as f64);
                }
            }
        }
        let t = std::time::Instant::now();
        let r = anova_two_way(&a, &b, &y, true).unwrap();
        assert!(r.compare_cells(MeansComparison::Tukey, 0.05).is_empty());
        assert_eq!(r.compare_a(MeansComparison::Tukey, 0.05).len(), 36);
        assert!(t.elapsed().as_secs_f64() < 10.0);
    }

    #[test]
    fn a_balanced_design_splits_the_total_sum_of_squares_exactly() {
        // 2 x 2 design, two numbers per cell.
        let a = [0, 0, 0, 0, 1, 1, 1, 1];
        let b = [0, 0, 1, 1, 0, 0, 1, 1];
        let y = [1.0, 2.0, 4.0, 5.0, 3.0, 5.0, 8.0, 9.0];
        let r = anova_two_way(&a, &b, &y, true).unwrap();
        let ab = r.interaction_row.unwrap();
        let parts = r.factor_a.sum_sq + r.factor_b.sum_sq + ab.sum_sq + r.error.sum_sq;
        assert!((parts - r.total.sum_sq).abs() < 1e-9);
        assert_eq!((r.factor_a.df, r.factor_b.df, ab.df, r.error.df, r.total.df), (1.0, 1.0, 1.0, 4.0, 7.0));
        assert!((r.model.sum_sq - (r.total.sum_sq - r.error.sum_sq)).abs() < 1e-9);
    }

    #[test]
    fn errors_are_named() {
        let a = [0, 0, 1, 1];
        let b = [0, 1, 0, 1];
        assert_eq!(anova_two_way(&a[..3], &b, &[1.0; 4], true), Err(TwoWayError::LengthMismatch));
        assert_eq!(anova_two_way(&a[..3], &b[..3], &[1.0, 2.0, 3.0], true), Err(TwoWayError::TooFewData(3)));
        assert_eq!(anova_two_way(&[0; 4], &b, &[1.0, 2.0, 3.0, 4.0], true), Err(TwoWayError::TooFewLevels));
        // Level 1 of A only meets level 0 of B: the interaction has an empty cell, the additive model does not.
        let a = [0, 0, 0, 1, 1];
        let b = [0, 1, 1, 0, 0];
        let y = [1.0, 2.0, 3.0, 4.0, 6.0];
        assert_eq!(anova_two_way(&a, &b, &y, true), Err(TwoWayError::EmptyCell { a: 1, b: 1 }));
        assert!(anova_two_way(&a, &b, &y, false).is_ok());
        // Two separate blocks: the effects of A and B cannot be told apart.
        let a = [0, 0, 1, 1];
        let b = [0, 0, 1, 1];
        assert_eq!(anova_two_way(&a, &b, &[1.0, 2.0, 3.0, 5.0], false), Err(TwoWayError::Disconnected));
    }

    #[test]
    fn constant_data_and_one_number_per_cell_do_not_panic() {
        let a = [0, 0, 1, 1, 0, 1];
        let b = [0, 1, 0, 1, 1, 0];
        let r = anova_two_way(&a, &b, &[5.0; 6], true).unwrap();
        assert!(r.factor_a.f.is_nan() && r.factor_a.sum_sq == 0.0);
        assert!(r.compare_a(MeansComparison::Tukey, 0.05).iter().all(|c| !c.significant));
        // One number per cell leaves no error degrees of freedom: no F tests, no comparisons.
        let a = [0, 0, 0, 1, 1, 1];
        let b = [0, 1, 2, 0, 1, 2];
        let r = anova_two_way(&a, &b, &[1.0, 3.0, 2.0, 5.0, 4.0, 8.0], true).unwrap();
        assert_eq!(r.error.df, 0.0);
        assert!(r.factor_a.p.is_nan() && r.error.mean_sq.is_nan());
        assert!(r.compare_b(MeansComparison::Bonferroni, 0.05).is_empty());
    }

    #[test]
    fn nan_data_is_ignored() {
        let a = [0, 0, 0, 0, 1, 1, 1, 1, 1];
        let b = [0, 0, 1, 1, 0, 0, 1, 1, 1];
        let y = [1.0, 2.0, 4.0, 5.0, 3.0, 5.0, 8.0, 9.0, f64::NAN];
        let r = anova_two_way(&a, &b, &y, true).unwrap();
        assert_eq!(r.total.df, 7.0);
    }
}
