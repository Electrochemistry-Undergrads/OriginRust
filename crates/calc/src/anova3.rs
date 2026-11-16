//! Three-way analysis of variance (Origin's Statistics > ANOVA > Three-Way ANOVA, `ANOVAThreeWay`; see
//! `docs/spec/statistics.md` section 25).
//!
//! As in [`crate::anova2`], the sum of squares of a term is the rise of the residual sum of squares when the
//! term's columns are left out of the full model, each factor coded by sum-to-zero columns and interactions by
//! their products: the "type III" sum of squares, which for a balanced design is the textbook table and for
//! unequal cell sizes is what Origin reports (measured on a balanced and an unbalanced design). The rows are
//! A, B, C, A*B, A*C, B*C, A*B*C (those that are switched on), Model, Error and Total.
//!
//! Also here: the descriptive statistics of every level and cell, Levene's test (a one-way ANOVA of the
//! absolute deviations from the cell means), the pairwise means comparisons of A, B, C and A*B built on
//! [`crate::anova::compare_means`], and the power of each F test through the noncentral F distribution.

use crate::anova::{Comparison, MeansComparison, compare_means};
use crate::anova2::{TwoWayRow, effect_columns, f_test, fit};
use crate::hypothesis::SampleStats;
use crate::power::pnf;
use statrs::distribution::{ContinuousCDF, FisherSnedecor};

/// The most cells whose pairwise comparison is listed (as in the two-way ANOVA).
pub const MAX_CELL_COMPARISON_CELLS: usize = 64;

/// The terms of the model. The main effects are always in; each interaction can be switched off (the check
/// boxes of Origin's Model tab; a model without them was not measured).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreeWayTerms {
    pub ab: bool,
    pub ac: bool,
    pub bc: bool,
    pub abc: bool,
}

impl Default for ThreeWayTerms {
    fn default() -> Self {
        ThreeWayTerms { ab: true, ac: true, bc: true, abc: true }
    }
}

/// An effect of the design, in Origin's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    A,
    B,
    C,
    Ab,
    Ac,
    Bc,
    Abc,
}

impl Effect {
    pub const ALL: [Effect; 7] = [Effect::A, Effect::B, Effect::C, Effect::Ab, Effect::Ac, Effect::Bc, Effect::Abc];

    /// Number of factors in the term.
    pub fn order(self) -> usize {
        match self {
            Effect::A | Effect::B | Effect::C => 1,
            Effect::Ab | Effect::Ac | Effect::Bc => 2,
            Effect::Abc => 3,
        }
    }

    /// Names of the factors in the term, as letters.
    pub fn letters(self) -> &'static str {
        match self {
            Effect::A => "A",
            Effect::B => "B",
            Effect::C => "C",
            Effect::Ab => "A*B",
            Effect::Ac => "A*C",
            Effect::Bc => "B*C",
            Effect::Abc => "A*B*C",
        }
    }
}

/// Why a three-way ANOVA could not be computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreeWayError {
    /// Fewer numbers than the model has parameters plus one.
    TooFewData(usize),
    /// A factor has fewer than two levels.
    TooFewLevels,
    /// The four inputs do not have the same length.
    LengthMismatch,
    /// A factor level has no data.
    EmptyLevel,
    /// Some combination of levels the model needs has no data (the model cannot be fitted).
    RankDeficient,
}

impl std::fmt::Display for ThreeWayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreeWayError::TooFewData(n) => {
                write!(f, "a three-way ANOVA needs more numbers than model terms, there are {n}")
            }
            ThreeWayError::TooFewLevels => write!(f, "each factor needs at least two levels"),
            ThreeWayError::LengthMismatch => write!(f, "the factor columns and the data column differ in length"),
            ThreeWayError::EmptyLevel => write!(f, "a factor level has no data"),
            ThreeWayError::RankDeficient => write!(
                f,
                "some combinations of the factors have no data, so the model cannot be fitted (Origin does not support \
                 empty cells either); switch interactions off or fill the cells"
            ),
        }
    }
}

/// Levene's test: a one-way ANOVA of the absolute deviations from the cell means over the cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Levene {
    pub model: TwoWayRow,
    pub error: TwoWayRow,
}

/// The power of the F test of one effect for the actual sample size and for hypothetical ones.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectPower {
    pub effect: Effect,
    pub alpha: f64,
    /// `(sample size, power)`: first the actual one, then the hypothetical sizes in the order given.
    pub powers: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThreeWayAnova {
    pub terms: ThreeWayTerms,
    /// Number of levels of A, B and C.
    pub levels: [usize; 3],
    /// Descriptive statistics per effect: the levels of A, B, C, then the cells of A*B (A changes slowest),
    /// A*C, B*C and A*B*C. Interactions that are switched off are still described.
    pub stats: Vec<(Effect, Vec<SampleStats>)>,
    pub overall: SampleStats,
    /// The rows of the table for the effects that are in the model.
    pub rows: Vec<(Effect, TwoWayRow)>,
    pub model: TwoWayRow,
    pub error: TwoWayRow,
    pub total: TwoWayRow,
    /// Parameters of the full model (including the constant).
    pub rank: usize,
    /// Cell means and sizes of the finest cells (A, B and C), A slowest, C fastest.
    pub cell_stats: Vec<SampleStats>,
    cell_of_row: Vec<usize>,
    y: Vec<f64>,
}

fn products(p: &[Vec<f64>], q: &[Vec<f64>]) -> Vec<Vec<f64>> {
    p.iter().flat_map(|a| q.iter().map(move |b| a.iter().zip(b).map(|(x, y)| x * y).collect())).collect()
}

/// Three-way ANOVA of `y` by the level numbers `a`, `b` and `c` (counted from 0; the number of levels is the
/// largest number plus one). Entries where `y` is NaN are ignored.
pub fn anova_three_way(
    a: &[usize],
    b: &[usize],
    c: &[usize],
    y: &[f64],
    terms: ThreeWayTerms,
) -> Result<ThreeWayAnova, ThreeWayError> {
    if a.len() != y.len() || b.len() != y.len() || c.len() != y.len() {
        return Err(ThreeWayError::LengthMismatch);
    }
    let keep: Vec<usize> = (0..y.len()).filter(|&i| !y[i].is_nan()).collect();
    let pick = |v: &[usize]| -> Vec<usize> { keep.iter().map(|&i| v[i]).collect() };
    let (a, b, c) = (pick(a), pick(b), pick(c));
    let y: Vec<f64> = keep.iter().map(|&i| y[i]).collect();
    let n = y.len();
    let count = |v: &[usize]| v.iter().max().map_or(0, |m| m + 1);
    let levels = [count(&a), count(&b), count(&c)];
    if levels.iter().any(|&l| l < 2) {
        return Err(ThreeWayError::TooFewLevels);
    }
    let [la, lb, lc] = levels;
    let group = |sel: &dyn Fn(usize) -> usize, groups: usize| -> Vec<SampleStats> {
        (0..groups)
            .map(|g| SampleStats::of(&(0..n).filter(|&i| sel(i) == g).map(|i| y[i]).collect::<Vec<f64>>()))
            .collect()
    };
    let mut stats = Vec::new();
    for effect in Effect::ALL {
        let (sel, groups): (Box<dyn Fn(usize) -> usize>, usize) = match effect {
            Effect::A => (Box::new(|i| a[i]), la),
            Effect::B => (Box::new(|i| b[i]), lb),
            Effect::C => (Box::new(|i| c[i]), lc),
            Effect::Ab => (Box::new(|i| a[i] * lb + b[i]), la * lb),
            Effect::Ac => (Box::new(|i| a[i] * lc + c[i]), la * lc),
            Effect::Bc => (Box::new(|i| b[i] * lc + c[i]), lb * lc),
            Effect::Abc => (Box::new(|i| (a[i] * lb + b[i]) * lc + c[i]), la * lb * lc),
        };
        stats.push((effect, group(&*sel, groups)));
    }
    if stats[..3].iter().any(|(_, s)| s.iter().any(|g| g.n == 0)) {
        return Err(ThreeWayError::EmptyLevel);
    }
    let included = |e: Effect| match e {
        Effect::Ab => terms.ab,
        Effect::Ac => terms.ac,
        Effect::Bc => terms.bc,
        Effect::Abc => terms.abc,
        _ => true,
    };
    let (ea, eb, ec) = (effect_columns(&a, la), effect_columns(&b, lb), effect_columns(&c, lc));
    let eab = products(&ea, &eb);
    let eac = products(&ea, &ec);
    let ebc = products(&eb, &ec);
    let eabc = products(&eab, &ec);
    let columns_of = |e: Effect| -> &Vec<Vec<f64>> {
        match e {
            Effect::A => &ea,
            Effect::B => &eb,
            Effect::C => &ec,
            Effect::Ab => &eab,
            Effect::Ac => &eac,
            Effect::Bc => &ebc,
            Effect::Abc => &eabc,
        }
    };
    let model_without = |skip: Option<Effect>| -> Vec<Vec<f64>> {
        let mut cols = vec![vec![1.0; n]];
        for e in Effect::ALL {
            if included(e) && Some(e) != skip {
                cols.extend(columns_of(e).iter().cloned());
            }
        }
        cols
    };
    let nominal_rank: usize =
        1 + Effect::ALL.iter().filter(|&&e| included(e)).map(|&e| columns_of(e).len()).sum::<usize>();
    if n <= nominal_rank {
        return Err(ThreeWayError::TooFewData(n));
    }
    let (sse_full, rank) = fit(&model_without(None), &y);
    if rank < nominal_rank {
        return Err(ThreeWayError::RankDeficient);
    }
    let overall = SampleStats::of(&y);
    let sst: f64 = y.iter().map(|v| (v - overall.mean).powi(2)).sum();
    let noise = 1e-20 * y.iter().map(|v| v * v).sum::<f64>();
    let sse = if sse_full <= noise { 0.0 } else { sse_full };
    let df_error = (n - rank) as f64;
    let mse = if df_error > 0.0 { sse / df_error } else { f64::NAN };
    let row = |ss: f64, df: f64| {
        let ss = if ss <= noise { 0.0 } else { ss };
        let (f, p) = f_test(ss, df, mse, df_error);
        TwoWayRow { df, sum_sq: ss, mean_sq: if df > 0.0 { ss / df } else { f64::NAN }, f, p }
    };
    let mut rows = Vec::new();
    for e in Effect::ALL.into_iter().filter(|&e| included(e)) {
        let (sse_without, rank_without) = fit(&model_without(Some(e)), &y);
        rows.push((e, row(sse_without - sse_full, (rank - rank_without) as f64)));
    }
    let model = row(sst - sse, (rank - 1) as f64);
    let error = TwoWayRow { df: df_error, sum_sq: sse, mean_sq: mse, f: f64::NAN, p: f64::NAN };
    let total = TwoWayRow { df: n as f64 - 1.0, sum_sq: sst, mean_sq: f64::NAN, f: f64::NAN, p: f64::NAN };
    let cell_of_row: Vec<usize> = (0..n).map(|i| (a[i] * lb + b[i]) * lc + c[i]).collect();
    let cell_stats = stats[6].1.clone();
    Ok(ThreeWayAnova { terms, levels, stats, overall, rows, model, error, total, rank, cell_stats, cell_of_row, y })
}

impl ThreeWayAnova {
    /// The descriptive statistics of an effect's levels or cells.
    pub fn stats_of(&self, effect: Effect) -> &[SampleStats] {
        &self.stats.iter().find(|(e, _)| *e == effect).expect("every effect is described").1
    }

    /// The table row of an effect, if it is in the model.
    pub fn row_of(&self, effect: Effect) -> Option<&TwoWayRow> {
        self.rows.iter().find(|(e, _)| *e == effect).map(|(_, r)| r)
    }

    /// Pairwise comparison of the levels of A, B or C, or of the A*B cells, with half the error mean square of
    /// the table (see the note in the body). The pairs come in Origin's order, `(1,2), (1,3), ..., (2,3), ...` (the first level changes
    /// slowest); the Holm methods list them by increasing p instead. Other interactions are not compared (Origin
    /// lists A*B only); the A*B comparison is left out beyond [`MAX_CELL_COMPARISON_CELLS`] cells.
    pub fn compare(&self, effect: Effect, method: MeansComparison, alpha: f64) -> Vec<Comparison> {
        if !matches!(effect, Effect::A | Effect::B | Effect::C | Effect::Ab) || self.error.df <= 0.0 {
            return Vec::new();
        }
        let stats = self.stats_of(effect);
        if stats.len() > MAX_CELL_COMPARISON_CELLS {
            return Vec::new();
        }
        let levels: Vec<(usize, f64)> = stats.iter().map(|s| (s.n, s.mean)).collect();
        // Origin's three-way comparisons use the standard error `sqrt(MSE / 2 (1/ni + 1/nj))` for every method
        // (half the variance of the two-way and one-way ANOVA; measured: the interaction pair SE 0.34186 and
        // factor A SE 0.19737 of the balanced run, with t, q, p and the limits all following from it).
        let mut list = compare_means(&levels, self.error.mean_sq / 2.0, self.error.df, method, alpha);
        if !method.is_holm() {
            list.sort_by_key(|c| (c.i, c.j));
        }
        list
    }

    /// Levene's test for equal variances: the one-way ANOVA of `|y - cell mean|` over the finest cells.
    pub fn levene(&self) -> Option<Levene> {
        let cells = self.cell_stats.len();
        let dev: Vec<f64> =
            self.y.iter().zip(&self.cell_of_row).map(|(v, &c)| (v - self.cell_stats[c].mean).abs()).collect();
        let mut sums = vec![(0usize, 0.0f64); cells];
        for (d, &c) in dev.iter().zip(&self.cell_of_row) {
            sums[c].0 += 1;
            sums[c].1 += d;
        }
        let used: Vec<(usize, f64)> = sums.into_iter().filter(|s| s.0 > 0).collect();
        let n = dev.len();
        let groups = used.len();
        if groups < 2 || n <= groups {
            return None;
        }
        let grand = dev.iter().sum::<f64>() / n as f64;
        let ss_total: f64 = dev.iter().map(|d| (d - grand).powi(2)).sum();
        let ss_model: f64 = used.iter().map(|(k, s)| *k as f64 * (s / *k as f64 - grand).powi(2)).sum();
        let ss_error = (ss_total - ss_model).max(0.0);
        let ss_error = if ss_error <= 1e-20 * dev.iter().map(|v| v * v).sum::<f64>() { 0.0 } else { ss_error };
        let (df_model, df_error) = ((groups - 1) as f64, (n - groups) as f64);
        let mse = ss_error / df_error;
        let (f, p) = f_test(ss_model, df_model, mse, df_error);
        Some(Levene {
            model: TwoWayRow { df: df_model, sum_sq: ss_model, mean_sq: ss_model / df_model, f, p },
            error: TwoWayRow { df: df_error, sum_sq: ss_error, mean_sq: mse, f: f64::NAN, p: f64::NAN },
        })
    }

    /// Power of the F test of each effect in the model, at the level `alpha`, for the actual sample size and
    /// for `hypothetical` sample sizes.
    ///
    /// The rule reproduces the numbers of Origin's balanced run (all seven effects, four sample sizes): the
    /// noncentrality is `df1 * F * N / n` with `n` the number of data, `df2 = N - rank` (rank: parameters of the
    /// full model), and the "actual" size of an effect is `n` times the number of factors in it (24 for A, B and
    /// C, 48 for the two-factor effects, 72 for A*B*C; one design measured, so Sheaf's reading of that rule).
    pub fn power(&self, alpha: f64, hypothetical: &[f64]) -> Vec<EffectPower> {
        let n = self.y.len() as f64;
        let mut out = Vec::new();
        for (effect, row) in &self.rows {
            if !(row.f.is_finite() && row.df > 0.0) {
                continue;
            }
            let at = |size: f64| -> f64 {
                let df2 = size - self.rank as f64;
                if df2 <= 0.0 {
                    return f64::NAN;
                }
                let Ok(dist) = FisherSnedecor::new(row.df, df2) else { return f64::NAN };
                let critical = dist.inverse_cdf(1.0 - alpha);
                (1.0 - pnf(critical, row.df, df2, row.df * row.f * size / n)).clamp(0.0, 1.0)
            };
            let actual = n * effect.order() as f64;
            let mut powers = vec![(actual, at(actual))];
            powers.extend(hypothetical.iter().map(|&s| (s, at(s))));
            out.push(EffectPower { effect: *effect, alpha, powers });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2 x 2 x 2 design with two numbers per cell: the mean of a cell is `10 + 3 sa + 1 sb + 2 sa sb` with the
    /// signs `s = +-1` of the levels, the two numbers differ from it by +-1. By hand: with sum-to-zero coding a
    /// coefficient `beta` of a term gives the sum of squares `N beta^2` (N = 16), the error sum of squares is
    /// 16 * 1 = 16 on 8 degrees of freedom.
    fn design() -> (Vec<usize>, Vec<usize>, Vec<usize>, Vec<f64>) {
        let (mut a, mut b, mut c, mut y) = (vec![], vec![], vec![], vec![]);
        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    for rep in 0..2 {
                        let (sa, sb) = (if i == 0 { 1.0 } else { -1.0 }, if j == 0 { 1.0 } else { -1.0 });
                        a.push(i);
                        b.push(j);
                        c.push(k);
                        y.push(10.0 + 3.0 * sa + sb + 2.0 * sa * sb + if rep == 0 { 1.0 } else { -1.0 });
                    }
                }
            }
        }
        (a, b, c, y)
    }

    #[test]
    fn balanced_two_by_two_by_two_matches_the_hand_computed_table() {
        let (a, b, c, y) = design();
        let r = anova_three_way(&a, &b, &c, &y, ThreeWayTerms::default()).unwrap();
        let ss = |e| r.row_of(e).unwrap().sum_sq;
        assert!((ss(Effect::A) - 16.0 * 9.0).abs() < 1e-9);
        assert!((ss(Effect::B) - 16.0).abs() < 1e-9);
        assert!((ss(Effect::Ab) - 16.0 * 4.0).abs() < 1e-9);
        for e in [Effect::C, Effect::Ac, Effect::Bc, Effect::Abc] {
            assert!(ss(e).abs() < 1e-9, "{e:?}");
        }
        assert!((r.error.sum_sq - 16.0).abs() < 1e-9);
        assert_eq!((r.error.df, r.model.df, r.total.df), (8.0, 7.0, 15.0));
        assert!((r.model.sum_sq + r.error.sum_sq - r.total.sum_sq).abs() < 1e-9);
        let a_row = r.row_of(Effect::A).unwrap();
        assert!((a_row.f - 144.0 / (16.0 / 8.0)).abs() < 1e-9);
        assert!(r.total.mean_sq.is_nan() && r.error.f.is_nan());
    }

    #[test]
    fn unbalanced_data_gets_type_three_sums_of_squares() {
        // One more number in a cell: the sums of squares of the terms are no longer additive.
        let (mut a, mut b, mut c, mut y) = design();
        a.push(0);
        b.push(0);
        c.push(0);
        y.push(17.0);
        let r = anova_three_way(&a, &b, &c, &y, ThreeWayTerms::default()).unwrap();
        assert_eq!(r.total.df, 16.0);
        let sum: f64 = r.rows.iter().map(|(_, row)| row.sum_sq).sum();
        // Type III sums of squares do not add up to the model sum of squares when the data are unbalanced.
        assert!((sum - r.model.sum_sq).abs() > 1e-6);
    }

    #[test]
    fn missing_data_and_bad_designs_are_reported() {
        let (a, b, c, mut y) = design();
        y[0] = f64::NAN;
        let r = anova_three_way(&a, &b, &c, &y, ThreeWayTerms::default()).unwrap();
        assert_eq!(r.total.df, 14.0);
        assert_eq!(anova_three_way(&a[..3], &b, &c, &y, ThreeWayTerms::default()), Err(ThreeWayError::LengthMismatch));
        let ones = vec![0; 16];
        assert_eq!(anova_three_way(&ones, &b, &c, &y, ThreeWayTerms::default()), Err(ThreeWayError::TooFewLevels));
        // One number per cell is too few for the full model (8 parameters, 8 numbers).
        let (a1, b1, c1): (Vec<usize>, Vec<usize>, Vec<usize>) = (
            a.iter().step_by(2).copied().collect(),
            b.iter().step_by(2).copied().collect(),
            c.iter().step_by(2).copied().collect(),
        );
        let y1: Vec<f64> = design().3.iter().step_by(2).copied().collect();
        assert!(matches!(
            anova_three_way(&a1, &b1, &c1, &y1, ThreeWayTerms::default()),
            Err(ThreeWayError::TooFewData(_))
        ));
        // Without the three-way interaction the same data have an error term.
        let t = ThreeWayTerms { abc: false, ..Default::default() };
        assert_eq!(anova_three_way(&a1, &b1, &c1, &y1, t).unwrap().error.df, 1.0);
        // An empty cell makes the model rank deficient.
        let (a2, b2, c2, y2): (Vec<usize>, Vec<usize>, Vec<usize>, Vec<f64>) = (0..16)
            .filter(|&i| !(a[i] == 1 && b[i] == 1 && c[i] == 1))
            .map(|i| (a[i], b[i], c[i], if y[i].is_nan() { 1.0 } else { y[i] }))
            .fold((vec![], vec![], vec![], vec![]), |mut acc, t| {
                acc.0.push(t.0);
                acc.1.push(t.1);
                acc.2.push(t.2);
                acc.3.push(t.3);
                acc
            });
        assert_eq!(anova_three_way(&a2, &b2, &c2, &y2, ThreeWayTerms::default()), Err(ThreeWayError::RankDeficient));
    }

    #[test]
    fn comparisons_come_in_row_major_pair_order_and_levene_uses_the_cell_deviations() {
        let (a, b, c, y) = design();
        let r = anova_three_way(&a, &b, &c, &y, ThreeWayTerms::default()).unwrap();
        let pairs: Vec<(usize, usize)> =
            r.compare(Effect::Ab, MeansComparison::Tukey, 0.05).iter().map(|c| (c.i, c.j)).collect();
        assert_eq!(pairs, [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
        assert!(r.compare(Effect::Ac, MeansComparison::Tukey, 0.05).is_empty());
        // Factor A: |difference| of the two marginal means is 6, SE = sqrt(MSE / 2 (1/8 + 1/8)) with MSE 2 (Origin's rule).
        let ca = &r.compare(Effect::A, MeansComparison::FisherLsd, 0.05)[0];
        assert!((ca.diff.abs() - 6.0).abs() < 1e-9 && (ca.se - 0.5).abs() < 1e-9);
        // Every cell holds 11/9-style pairs 10+-1 around its mean: the deviations are all 1, so Levene's F is 0.
        let l = r.levene().unwrap();
        assert_eq!((l.model.df, l.error.df), (7.0, 8.0));
        assert!(l.model.sum_sq.abs() < 1e-12 && l.error.sum_sq.abs() < 1e-12);
    }

    #[test]
    fn power_grows_with_the_sample_size_and_is_the_noncentral_f_tail() {
        let (a, b, c, y) = design();
        let r = anova_three_way(&a, &b, &c, &y, ThreeWayTerms::default()).unwrap();
        let p = r.power(0.05, &[50.0, 100.0, 200.0]);
        let a_power = &p.iter().find(|e| e.effect == Effect::A).unwrap().powers;
        assert_eq!(a_power[0].0, 16.0);
        assert!(a_power.windows(2).all(|w| w[1].1 >= w[0].1 - 1e-12));
        assert!(a_power[1].1 > 0.99);
    }
}
