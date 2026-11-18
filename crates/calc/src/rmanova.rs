//! Repeated measures ANOVA, one-way and two-way with every factor repeated (Origin 2016's Statistics > ANOVA >
//! One-Way and Two-Way Repeated Measures ANOVA; see `docs/spec/statistics.md` section 26).
//!
//! Each subject gives one number per cell of the design. Every within effect is tested on the subject's
//! scores of its orthonormal contrasts: with `d` degrees of freedom the effect's sum of squares is `n` times
//! the squared length of the mean score vector, its own error sum of squares is `(n - 1) tr(S)` for the
//! covariance matrix `S` of the scores (`d (n - 1)` degrees of freedom), and the same matrix gives Mauchly's
//! test of sphericity, the Greenhouse-Geisser, Huynh-Feldt and lower-bound epsilons and the multivariate tests.
//! The between-subjects part is the intercept and the subjects.

use crate::anova::{Comparison, MeansComparison, compare_means};
use nalgebra::DMatrix;
use statrs::distribution::{ChiSquared, ContinuousCDF, FisherSnedecor, StudentsT};

/// Why a repeated measures ANOVA could not be computed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RmError {
    /// Fewer than two subjects.
    TooFewSubjects,
    /// A factor with fewer than two levels, or no factor.
    TooFewLevels,
    /// A subject's row does not have one number per cell, or a number is not finite.
    BadData,
}

impl std::fmt::Display for RmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RmError::TooFewSubjects => write!(f, "a repeated measures ANOVA needs at least two subjects"),
            RmError::TooFewLevels => write!(f, "every factor needs at least two levels"),
            RmError::BadData => write!(f, "every subject needs one finite number in every cell of the design"),
        }
    }
}

impl std::error::Error for RmError {}

/// A within-subjects effect: factor A, factor B, or their interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RmTerm {
    A,
    B,
    Ab,
}

/// How the degrees of freedom are corrected for the lack of sphericity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Correction {
    /// Sphericity assumed: the plain degrees of freedom.
    Assumed,
    GreenhouseGeisser,
    HuynhFeldt,
    LowerBound,
}

impl Correction {
    pub const ALL: [Correction; 4] =
        [Correction::Assumed, Correction::GreenhouseGeisser, Correction::HuynhFeldt, Correction::LowerBound];

    pub fn label(self) -> &'static str {
        match self {
            Correction::Assumed => "Sphericity Assumed",
            Correction::GreenhouseGeisser => "Greenhouse-Geisser",
            Correction::HuynhFeldt => "Huynh-Feldt",
            Correction::LowerBound => "Lower-bound",
        }
    }
}

/// One row pair (effect and its error) of the within-subjects table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmRow {
    pub correction: Correction,
    pub df: f64,
    pub mean_sq: f64,
    pub df_error: f64,
    pub mean_sq_error: f64,
    pub f: f64,
    pub p: f64,
}

/// Mauchly's test of sphericity and the epsilons of one effect (only for an effect with at least two degrees
/// of freedom and more subjects than degrees of freedom).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphericity {
    pub w: f64,
    pub chi_sq: f64,
    pub df: f64,
    pub p: f64,
    pub greenhouse_geisser: f64,
    /// Capped at 1.
    pub huynh_feldt: f64,
    pub lower_bound: f64,
}

/// The multivariate tests of an effect, from the eigenvalue of the one-sample Hotelling problem. All four share
/// one F with `df` and `df_error` degrees of freedom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Multivariate {
    pub pillai: f64,
    pub wilks: f64,
    pub hotelling: f64,
    pub roy: f64,
    pub f: f64,
    pub df: f64,
    pub df_error: f64,
    pub p: f64,
}

/// The result for one within effect.
#[derive(Debug, Clone, PartialEq)]
pub struct RmEffect {
    pub term: RmTerm,
    pub ss: f64,
    pub ss_error: f64,
    /// The four rows: sphericity assumed, Greenhouse-Geisser, Huynh-Feldt, lower bound.
    pub rows: [RmRow; 4],
    pub sphericity: Option<Sphericity>,
    pub multivariate: Option<Multivariate>,
}

/// A row of the between-subjects table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmBetween {
    pub ss: f64,
    pub df: f64,
    pub mean_sq: f64,
    pub f: f64,
    pub p: f64,
}

/// Mean, standard error (standard deviation of the subjects' values over the square root of their number) and
/// confidence limits of one level or cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmLevel {
    pub mean: f64,
    pub se: f64,
    pub lcl: f64,
    pub ucl: f64,
}

/// The whole analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct RmAnova {
    /// Number of subjects.
    pub n: usize,
    /// Levels of factor A and (two-way) factor B.
    pub levels: Vec<usize>,
    pub effects: Vec<RmEffect>,
    pub intercept: RmBetween,
    pub subjects: RmBetween,
    data: Vec<Vec<f64>>,
}

fn f_upper(f: f64, d1: f64, d2: f64) -> f64 {
    if f.is_nan() || d1 <= 0.0 || d2 <= 0.0 {
        return f64::NAN;
    }
    if f.is_infinite() {
        return 0.0;
    }
    FisherSnedecor::new(d1, d2).map_or(f64::NAN, |d| 1.0 - d.cdf(f))
}

/// The orthonormal (Helmert) contrasts of `k` levels, `k - 1` rows.
fn helmert(k: usize) -> DMatrix<f64> {
    DMatrix::from_fn(k - 1, k, |r, c| {
        let i = (r + 1) as f64;
        let scale = 1.0 / (i * (i + 1.0)).sqrt();
        if c <= r {
            scale
        } else if c == r + 1 {
            -i * scale
        } else {
            0.0
        }
    })
}

fn kron(a: &DMatrix<f64>, b: &DMatrix<f64>) -> DMatrix<f64> {
    DMatrix::from_fn(a.nrows() * b.nrows(), a.ncols() * b.ncols(), |r, c| {
        a[(r / b.nrows(), c / b.ncols())] * b[(r % b.nrows(), c % b.ncols())]
    })
}

fn ones_row(k: usize) -> DMatrix<f64> {
    DMatrix::from_element(1, k, 1.0 / (k as f64).sqrt())
}

/// The contrast matrix of an effect over the row-major cells of the design.
fn contrasts(term: RmTerm, levels: &[usize]) -> DMatrix<f64> {
    let a = levels[0];
    match (term, levels.get(1)) {
        (_, None) => helmert(a),
        (RmTerm::A, Some(&b)) => kron(&helmert(a), &ones_row(b)),
        (RmTerm::B, Some(&b)) => kron(&ones_row(a), &helmert(b)),
        (RmTerm::Ab, Some(&b)) => kron(&helmert(a), &helmert(b)),
    }
}

/// Analyses `data` (one row per subject, one number per cell; for two factors the cells are A-major: all levels
/// of B for the first level of A, then the next). `levels` holds the number of levels of A, or of A and B;
/// `interaction` adds the interaction of a two-way design.
pub fn rm_anova(data: &[Vec<f64>], levels: &[usize], interaction: bool) -> Result<RmAnova, RmError> {
    if levels.is_empty() || levels.len() > 2 || levels.iter().any(|&l| l < 2) {
        return Err(RmError::TooFewLevels);
    }
    let cells: usize = levels.iter().product();
    let n = data.len();
    if n < 2 {
        return Err(RmError::TooFewSubjects);
    }
    if data.iter().any(|r| r.len() != cells || r.iter().any(|v| !v.is_finite())) {
        return Err(RmError::BadData);
    }
    let y = DMatrix::from_fn(n, cells, |r, c| data[r][c]);
    let nf = n as f64;
    let terms: Vec<RmTerm> = match levels.len() {
        1 => vec![RmTerm::A],
        _ if interaction => vec![RmTerm::A, RmTerm::B, RmTerm::Ab],
        _ => vec![RmTerm::A, RmTerm::B],
    };
    let effects = terms.into_iter().map(|t| effect(t, &y, levels)).collect();

    let grand = y.iter().sum::<f64>() / (nf * cells as f64);
    let ss_subjects: f64 = (0..n).map(|r| cells as f64 * (y.row(r).sum() / cells as f64 - grand).powi(2)).sum();
    let df_subjects = nf - 1.0;
    let ms_subjects = ss_subjects / df_subjects;
    let ss_int = nf * cells as f64 * grand * grand;
    let f_int = ss_int / ms_subjects;
    Ok(RmAnova {
        n,
        levels: levels.to_vec(),
        effects,
        intercept: RmBetween { ss: ss_int, df: 1.0, mean_sq: ss_int, f: f_int, p: f_upper(f_int, 1.0, df_subjects) },
        subjects: RmBetween { ss: ss_subjects, df: df_subjects, mean_sq: ms_subjects, f: f64::NAN, p: f64::NAN },
        data: data.to_vec(),
    })
}

fn effect(term: RmTerm, y: &DMatrix<f64>, levels: &[usize]) -> RmEffect {
    let n = y.nrows();
    let nf = n as f64;
    let w = contrasts(term, levels);
    let d = w.nrows();
    let df = d as f64;
    let scores = y * w.transpose();
    let mean: Vec<f64> = (0..d).map(|j| scores.column(j).sum() / nf).collect();
    let ss: f64 = nf * mean.iter().map(|m| m * m).sum::<f64>();
    let s = DMatrix::from_fn(d, d, |i, j| {
        (0..n).map(|r| (scores[(r, i)] - mean[i]) * (scores[(r, j)] - mean[j])).sum::<f64>() / (nf - 1.0)
    });
    let ss_error = (nf - 1.0) * s.trace();
    let df_error = (nf - 1.0) * df;
    let ms = ss / df;
    let ms_error = ss_error / df_error;
    let f = if ms_error == 0.0 { if ms == 0.0 { f64::NAN } else { f64::INFINITY } } else { ms / ms_error };

    let sphericity = sphericity(&s, n);
    let eps = |c: Correction| match c {
        Correction::Assumed => 1.0,
        Correction::GreenhouseGeisser => sphericity.map_or(1.0, |s| s.greenhouse_geisser),
        Correction::HuynhFeldt => sphericity.map_or(1.0, |s| s.huynh_feldt),
        Correction::LowerBound => 1.0 / df,
    };
    let rows = Correction::ALL.map(|c| {
        let e = eps(c);
        let (d1, d2) = (df * e, df_error * e);
        RmRow {
            correction: c,
            df: d1,
            mean_sq: ss / d1,
            df_error: d2,
            mean_sq_error: ss_error / d2,
            f,
            p: f_upper(f, d1, d2),
        }
    });
    RmEffect { term, ss, ss_error, rows, sphericity, multivariate: multivariate(&s, &mean, n) }
}

/// Mauchly's test and the epsilons from the covariance matrix `s` of the orthonormal contrast scores.
fn sphericity(s: &DMatrix<f64>, n: usize) -> Option<Sphericity> {
    let d = s.nrows();
    if d < 2 || n <= d {
        return None;
    }
    let (df, nf) = (d as f64, n as f64);
    let tr = s.trace();
    let tr2 = (s * s).trace();
    if tr.is_nan() || tr2.is_nan() || tr <= 0.0 || tr2 <= 0.0 {
        return None;
    }
    let w = s.determinant() / (tr / df).powi(d as i32);
    if w.is_nan() || w <= 0.0 {
        return None;
    }
    let w = w.min(1.0);
    let chi_sq = -(nf - 1.0 - (2.0 * df * df + df + 2.0) / (6.0 * df)) * w.ln();
    let chi_df = df * (df + 1.0) / 2.0 - 1.0;
    let p = ChiSquared::new(chi_df).map_or(f64::NAN, |c| 1.0 - c.cdf(chi_sq.max(0.0)));
    let gg = (tr * tr / (df * tr2)).clamp(1.0 / df, 1.0);
    let hf_den = df * (nf - 1.0 - df * gg);
    let hf = if hf_den > 0.0 { ((nf * df * gg - 2.0) / hf_den).min(1.0) } else { 1.0 };
    Some(Sphericity { w, chi_sq, df: chi_df, p, greenhouse_geisser: gg, huynh_feldt: hf, lower_bound: 1.0 / df })
}

/// Pillai's trace, Wilks' lambda, Hotelling's trace and Roy's root of the effect: the one-sample Hotelling
/// problem of the scores (a single non-zero eigenvalue `T^2 / (n - 1)`).
fn multivariate(s: &DMatrix<f64>, mean: &[f64], n: usize) -> Option<Multivariate> {
    let d = s.nrows();
    if n <= d {
        return None;
    }
    let inv = s.clone().try_inverse()?;
    let m = DMatrix::from_column_slice(d, 1, mean);
    let t2 = n as f64 * (m.transpose() * inv * &m)[(0, 0)];
    let lambda = t2 / (n as f64 - 1.0);
    let (df, df_error) = (d as f64, (n - d) as f64);
    let f = df_error / df * lambda;
    Some(Multivariate {
        pillai: lambda / (1.0 + lambda),
        wilks: 1.0 / (1.0 + lambda),
        hotelling: lambda,
        roy: lambda,
        f,
        df,
        df_error,
        p: f_upper(f, df, df_error),
    })
}

impl RmAnova {
    /// The effect of `term`, when it is in the model.
    pub fn effect(&self, term: RmTerm) -> Option<&RmEffect> {
        self.effects.iter().find(|e| e.term == term)
    }

    /// Number of levels (A, B) or cells (A*B) of an effect.
    pub fn level_count(&self, term: RmTerm) -> usize {
        match term {
            RmTerm::A => self.levels[0],
            RmTerm::B => self.levels.get(1).copied().unwrap_or(0),
            RmTerm::Ab => self.levels.iter().product(),
        }
    }

    /// Per-subject values of level `i` of an effect: the cell, or the mean over the other factor.
    fn subject_values(&self, term: RmTerm, i: usize) -> Vec<f64> {
        let (a, b) = (self.levels[0], self.levels.get(1).copied().unwrap_or(1));
        self.data
            .iter()
            .map(|row| match term {
                RmTerm::Ab => row[i],
                RmTerm::A if self.levels.len() == 1 => row[i],
                RmTerm::A => row[i * b..(i + 1) * b].iter().sum::<f64>() / b as f64,
                RmTerm::B => (0..a).map(|k| row[k * b + i]).sum::<f64>() / a as f64,
            })
            .collect()
    }

    /// Mean, standard error and the `1 - alpha` confidence limits (t with `n - 1` degrees of freedom) of every
    /// level of an effect (every cell for the interaction).
    pub fn level_stats(&self, term: RmTerm, alpha: f64) -> Vec<RmLevel> {
        let n = self.n as f64;
        let t = StudentsT::new(0.0, 1.0, n - 1.0).map_or(f64::NAN, |d| d.inverse_cdf(1.0 - alpha / 2.0));
        (0..self.level_count(term))
            .map(|i| {
                let v = self.subject_values(term, i);
                let mean = v.iter().sum::<f64>() / n;
                let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
                let se = sd / n.sqrt();
                RmLevel { mean, se, lcl: mean - t * se, ucl: mean + t * se }
            })
            .collect()
    }

    /// Pairwise comparisons of the level means of a one-way design: the error mean square of the table
    /// (sphericity assumed) with its degrees of freedom, `n` subjects per level. Pairs come in Origin's order
    /// `(1,2), (1,3), ..., (2,3), ...` and `diff` is `mean(i) - mean(j)` (Origin's sign; the statistic is
    /// unsigned); the Holm methods list the pairs by increasing p, as the table of Origin does.
    pub fn compare(&self, method: MeansComparison, alpha: f64) -> Vec<Comparison> {
        if self.levels.len() != 1 || method == MeansComparison::Dunnett {
            return Vec::new();
        }
        let Some(e) = self.effect(RmTerm::A) else { return Vec::new() };
        let row = e.rows[0];
        let levels: Vec<(usize, f64)> = self.level_stats(RmTerm::A, alpha).iter().map(|l| (self.n, l.mean)).collect();
        let mut list = compare_means(&levels, row.mean_sq_error, row.df_error, method, alpha);
        for c in &mut list {
            c.diff = -c.diff;
            std::mem::swap(&mut c.lcl, &mut c.ucl);
            c.lcl = -c.lcl;
            c.ucl = -c.ucl;
            // Origin prints |t| for every method but Tukey (q) and Scheffe's p-value comes from F.
            c.stat = if method == MeansComparison::Scheffe { c.diff.abs() / c.se } else { c.stat.abs() };
        }
        if !method.is_holm() {
            list.sort_by_key(|c| (c.i, c.j));
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_way() -> Vec<Vec<f64>> {
        vec![vec![1.0, 2.0, 4.0], vec![2.0, 2.5, 5.5], vec![1.5, 3.5, 4.0], vec![3.0, 3.0, 6.0], vec![2.5, 4.5, 5.0]]
    }

    #[test]
    fn the_one_way_table_splits_the_variance_into_levels_subjects_and_error() {
        let d = one_way();
        let r = rm_anova(&d, &[3], false).unwrap();
        let e = r.effect(RmTerm::A).unwrap();
        // Total SS = levels + subjects + error.
        let all: Vec<f64> = d.iter().flatten().copied().collect();
        let gm = all.iter().sum::<f64>() / all.len() as f64;
        let total: f64 = all.iter().map(|v| (v - gm).powi(2)).sum();
        assert!((total - (e.ss + e.ss_error + r.subjects.ss)).abs() < 1e-9);
        assert_eq!((e.rows[0].df, e.rows[0].df_error), (2.0, 8.0));
        assert_eq!(e.rows[3].df, 1.0);
        let s = e.sphericity.unwrap();
        assert!(s.greenhouse_geisser <= 1.0 && s.greenhouse_geisser >= 0.5 && s.huynh_feldt <= 1.0);
        let m = e.multivariate.unwrap();
        assert!((m.pillai + m.wilks - 1.0).abs() < 1e-12);
        assert_eq!((m.df, m.df_error), (2.0, 3.0));
    }

    #[test]
    fn two_levels_have_no_sphericity_and_equal_rows() {
        let d: Vec<Vec<f64>> = one_way().into_iter().map(|r| r[..2].to_vec()).collect();
        let e = rm_anova(&d, &[2], false).unwrap();
        let e = e.effect(RmTerm::A).unwrap();
        assert!(e.sphericity.is_none());
        assert!(e.rows.iter().all(|r| r.p == e.rows[0].p && r.df == 1.0));
        // The F test is the paired t test squared.
        let diff: Vec<f64> = d.iter().map(|r| r[0] - r[1]).collect();
        let m = diff.iter().sum::<f64>() / 5.0;
        let sd = (diff.iter().map(|x| (x - m).powi(2)).sum::<f64>() / 4.0).sqrt();
        let t = m / (sd / 5f64.sqrt());
        assert!((e.rows[0].f - t * t).abs() < 1e-9);
    }

    #[test]
    fn refuses_bad_designs() {
        assert_eq!(rm_anova(&one_way(), &[], false).unwrap_err(), RmError::TooFewLevels);
        assert_eq!(rm_anova(&one_way(), &[1], false).unwrap_err(), RmError::TooFewLevels);
        assert_eq!(rm_anova(&one_way()[..1], &[3], false).unwrap_err(), RmError::TooFewSubjects);
        assert_eq!(rm_anova(&one_way(), &[4], false).unwrap_err(), RmError::BadData);
        let mut bad = one_way();
        bad[0][0] = f64::NAN;
        assert_eq!(rm_anova(&bad, &[3], false).unwrap_err(), RmError::BadData);
    }

    #[test]
    fn two_way_has_an_error_term_for_each_effect() {
        let data: Vec<Vec<f64>> = (0..6)
            .map(|s| (0..6).map(|c| (s as f64 * 0.7).sin() + c as f64 * 0.3 + ((s * c) % 4) as f64 * 0.2).collect())
            .collect();
        let r = rm_anova(&data, &[2, 3], true).unwrap();
        assert_eq!(r.effects.len(), 3);
        let dfs: Vec<(f64, f64)> = r.effects.iter().map(|e| (e.rows[0].df, e.rows[0].df_error)).collect();
        assert_eq!(dfs, vec![(1.0, 5.0), (2.0, 10.0), (2.0, 10.0)]);
        let r = rm_anova(&data, &[2, 3], false).unwrap();
        assert_eq!(r.effects.len(), 2);
        assert_eq!(r.level_stats(RmTerm::Ab, 0.05).len(), 6);
        assert_eq!(r.level_stats(RmTerm::B, 0.05).len(), 3);
    }
}
