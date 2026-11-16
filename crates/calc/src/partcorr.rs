//! Partial correlation coefficients (Origin's Statistics > Descriptive Statistics > Partial Correlation
//! Coefficient, `pcorrcoef`; see `docs/spec/statistics.md` section 24).
//!
//! For two variables x and y and controlling variables z1..zk the partial correlation is read from the inverse
//! `P` of the correlation matrix `R` of (x, y, z1..zk): `r = -P[x,y] / sqrt(P[x,x] P[y,y])`. Its p-value is the
//! t test of `t = r sqrt(df / (1 - r^2))` with `df = n - 2 - k` degrees of freedom (halved for the one-tailed
//! test). With pairwise deletion every entry of `R` is the Pearson correlation of its own pair of columns on the
//! rows where both have a number, and `n` of a pair is the number of rows where x and y both have a number (the
//! gaps of the controlling variables do not enter `n`); with listwise deletion only the rows complete in every
//! column are used.

use crate::correlation::{CorrKind, correlation};
use nalgebra::DMatrix;
use statrs::distribution::{ContinuousCDF, StudentsT};

/// Why a partial correlation could not be computed (Origin returns no report in these cases and says nothing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartialCorrError {
    /// At least two variables are needed.
    TooFewVariables,
    /// At least one controlling variable is needed.
    NoControls,
    /// A controlling variable (0-based index into the controls) has no spread.
    ConstantControl(usize),
    /// Two controlling variables hold the same numbers.
    DuplicateControl(usize, usize),
    /// A variable holds the same numbers as a controlling variable.
    VariableIsControl(usize, usize),
    /// The two variables of the pair (indices into the variables) are perfectly correlated.
    PerfectPair(usize, usize),
    /// Fewer rows than the controls and the pair need (`n - 2 - k < 0`), or a correlation matrix that cannot be inverted.
    TooFewRows { pair: (usize, usize), n: usize },
}

impl std::fmt::Display for PartialCorrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartialCorrError::TooFewVariables => write!(f, "the partial correlation needs at least two variables"),
            PartialCorrError::NoControls => write!(f, "choose at least one controlling variable"),
            PartialCorrError::ConstantControl(i) => {
                write!(f, "controlling variable {} is constant (or has fewer than two numbers)", i + 1)
            }
            PartialCorrError::DuplicateControl(i, j) => {
                write!(f, "controlling variables {} and {} hold the same numbers", i + 1, j + 1)
            }
            PartialCorrError::VariableIsControl(v, c) => {
                write!(f, "variable {} holds the same numbers as controlling variable {}", v + 1, c + 1)
            }
            PartialCorrError::PerfectPair(i, j) => {
                write!(
                    f,
                    "variables {} and {} are perfectly correlated, so no partial correlation exists",
                    i + 1,
                    j + 1
                )
            }
            PartialCorrError::TooFewRows { pair: (i, j), n } => write!(
                f,
                "variables {} and {} have {n} rows in common, too few for this number of controlling variables",
                i + 1,
                j + 1
            ),
        }
    }
}

/// The partial correlations of every pair of variables.
#[derive(Debug, Clone, PartialEq)]
pub struct PartialCorr {
    /// Number of variables.
    pub size: usize,
    /// Number of controlling variables.
    pub controls: usize,
    /// Row-major `size x size` matrix of coefficients; the diagonal is 1.
    pub r: Vec<f64>,
    /// p-values (NaN on the diagonal and where the degrees of freedom are 0).
    pub p: Vec<f64>,
    /// Degrees of freedom `n - 2 - k` (0 on the diagonal).
    pub df: Vec<f64>,
    /// Number of rows each pair rests on (0 on the diagonal).
    pub n: Vec<usize>,
}

impl PartialCorr {
    pub fn r(&self, i: usize, j: usize) -> f64 {
        self.r[i * self.size + j]
    }
    pub fn p(&self, i: usize, j: usize) -> f64 {
        self.p[i * self.size + j]
    }
    pub fn df(&self, i: usize, j: usize) -> f64 {
        self.df[i * self.size + j]
    }
}

fn value(col: &[f64], i: usize) -> f64 {
    col.get(i).copied().unwrap_or(f64::NAN)
}

fn same_numbers(a: &[f64], b: &[f64]) -> bool {
    let rows = a.len().max(b.len());
    (0..rows).all(|i| {
        let (x, y) = (value(a, i), value(b, i));
        x == y || (x.is_nan() && y.is_nan())
    })
}

fn has_spread(col: &[f64]) -> bool {
    let mut it = col.iter().filter(|v| !v.is_nan());
    it.next().is_some_and(|first| it.any(|v| v != first))
}

/// Partial correlation of every pair of `vars` controlling for `controls`. Missing values are NaN or absent
/// (a shorter column). `one_tailed` halves the p-values.
pub fn partial_correlation(
    vars: &[&[f64]],
    controls: &[&[f64]],
    listwise: bool,
    one_tailed: bool,
) -> Result<PartialCorr, PartialCorrError> {
    if vars.len() < 2 {
        return Err(PartialCorrError::TooFewVariables);
    }
    if controls.is_empty() {
        return Err(PartialCorrError::NoControls);
    }
    for (i, c) in controls.iter().enumerate() {
        if !has_spread(c) {
            return Err(PartialCorrError::ConstantControl(i));
        }
        for (j, d) in controls.iter().enumerate().skip(i + 1) {
            if same_numbers(c, d) {
                return Err(PartialCorrError::DuplicateControl(i, j));
            }
        }
    }
    for (v, x) in vars.iter().enumerate() {
        for (c, z) in controls.iter().enumerate() {
            if same_numbers(x, z) {
                return Err(PartialCorrError::VariableIsControl(v, c));
            }
        }
    }
    let m = vars.len();
    let k = controls.len();
    let all: Vec<&[f64]> = vars.iter().chain(controls).copied().collect();
    let rows = all.iter().map(|c| c.len()).max().unwrap_or(0);
    let complete: Vec<usize> = (0..rows).filter(|&i| all.iter().all(|c| !value(c, i).is_nan())).collect();
    let listwise_cols: Vec<Vec<f64>> = if listwise {
        all.iter().map(|c| complete.iter().map(|&i| value(c, i)).collect()).collect()
    } else {
        Vec::new()
    };
    // Pearson correlation of columns a and b of `all`.
    let pearson = |a: usize, b: usize| -> f64 {
        if a == b {
            return 1.0;
        }
        if listwise {
            correlation(&listwise_cols[a], &listwise_cols[b], CorrKind::Pearson).r
        } else {
            let (xa, xb): (Vec<f64>, Vec<f64>) = (0..rows).map(|i| (value(all[a], i), value(all[b], i))).unzip();
            correlation(&xa, &xb, CorrKind::Pearson).r
        }
    };
    let mut out = PartialCorr {
        size: m,
        controls: k,
        r: vec![0.0; m * m],
        p: vec![f64::NAN; m * m],
        df: vec![0.0; m * m],
        n: vec![0; m * m],
    };
    for i in 0..m {
        out.r[i * m + i] = 1.0;
    }
    for i in 0..m {
        for j in i + 1..m {
            let n = if listwise {
                complete.len()
            } else {
                (0..rows).filter(|&row| !value(vars[i], row).is_nan() && !value(vars[j], row).is_nan()).count()
            };
            let df = n as f64 - 2.0 - k as f64;
            let too_few = PartialCorrError::TooFewRows { pair: (i, j), n };
            // The sub-matrix of (x, y, controls).
            let idx: Vec<usize> = [i, j].into_iter().chain(m..m + k).collect();
            let size = idx.len();
            let mut r = DMatrix::<f64>::zeros(size, size);
            for (a, &ia) in idx.iter().enumerate() {
                for (b, &ib) in idx.iter().enumerate().skip(a) {
                    let v = pearson(ia, ib);
                    r[(a, b)] = v;
                    r[(b, a)] = v;
                }
            }
            if r[(0, 1)].is_nan() {
                return Err(too_few);
            }
            if r[(0, 1)].abs() > 1.0 - 1e-12 {
                return Err(PartialCorrError::PerfectPair(i, j));
            }
            // Origin refuses n = 3 with one control (df 0); with two controls and df 0 it prints -1 or 1.
            if df < 0.0 || (df == 0.0 && k == 1) || r.iter().any(|v| v.is_nan()) {
                return Err(too_few);
            }
            let Some(inv) = r.try_inverse() else {
                return Err(too_few);
            };
            let mut partial = (-inv[(0, 1)] / (inv[(0, 0)] * inv[(1, 1)]).sqrt()).clamp(-1.0, 1.0);
            if df == 0.0 && !partial.is_nan() {
                // The residuals of two rows' worth of freedom are perfectly (anti)correlated.
                partial = partial.signum();
            }
            if partial.is_nan() {
                return Err(too_few);
            }
            let p = if df >= 1.0 && partial.abs() < 1.0 {
                let t = partial * (df / (1.0 - partial * partial)).sqrt();
                let tail = StudentsT::new(0.0, 1.0, df).map_or(f64::NAN, |d| d.cdf(-t.abs()));
                if one_tailed { tail } else { 2.0 * tail }
            } else {
                f64::NAN
            };
            for (a, b) in [(i, j), (j, i)] {
                out.r[a * m + b] = partial;
                out.p[a * m + b] = p;
                out.df[a * m + b] = df;
                out.n[a * m + b] = n;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const X: [f64; 6] = [1.0, 2.0, 3.0, 4.0, 5.0, 7.0];
    const Y: [f64; 6] = [2.0, 1.0, 4.0, 3.0, 7.0, 6.0];
    const Z: [f64; 6] = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0];

    fn pearson(a: &[f64], b: &[f64]) -> f64 {
        correlation(a, b, CorrKind::Pearson).r
    }

    #[test]
    fn one_control_matches_the_textbook_formula() {
        // r_xy.z = (r_xy - r_xz r_yz) / sqrt((1 - r_xz^2)(1 - r_yz^2))
        let (rxy, rxz, ryz) = (pearson(&X, &Y), pearson(&X, &Z), pearson(&Y, &Z));
        let expected = (rxy - rxz * ryz) / ((1.0 - rxz * rxz) * (1.0 - ryz * ryz)).sqrt();
        let r = partial_correlation(&[&X, &Y], &[&Z], false, false).unwrap();
        assert!((r.r(0, 1) - expected).abs() < 1e-12);
        assert_eq!(r.r(0, 1), r.r(1, 0));
        assert_eq!((r.r(0, 0), r.df(0, 0)), (1.0, 0.0));
        assert!(r.p(0, 0).is_nan());
        // df = 6 - 2 - 1 = 3 and the two-tailed t test.
        assert_eq!(r.df(0, 1), 3.0);
        let t = expected * (3.0 / (1.0 - expected * expected)).sqrt();
        let p = 2.0 * StudentsT::new(0.0, 1.0, 3.0).unwrap().cdf(-t.abs());
        assert!((r.p(0, 1) - p).abs() < 1e-12);
        let one = partial_correlation(&[&X, &Y], &[&Z], false, true).unwrap();
        assert!((one.p(0, 1) - p / 2.0).abs() < 1e-12);
    }

    #[test]
    fn missing_values_pairwise_use_the_pair_rows_only_listwise_the_complete_rows() {
        let mut z = Z.to_vec();
        z[2] = f64::NAN;
        let mut y = Y.to_vec();
        y[5] = f64::NAN;
        let pair = partial_correlation(&[&X, &y], &[&z], false, false).unwrap();
        // The pair (x, y) has 5 rows; the gap of the control does not enter n.
        assert_eq!(pair.n[1], 5);
        assert_eq!(pair.df(0, 1), 2.0);
        let list = partial_correlation(&[&X, &y], &[&z], true, false).unwrap();
        assert_eq!(list.n[1], 4);
        assert_eq!(list.df(0, 1), 1.0);
        // Listwise equals the textbook formula on the four complete rows.
        let rows = [0, 1, 3, 4];
        let pick = |c: &[f64]| -> Vec<f64> { rows.iter().map(|&i| c[i]).collect() };
        let (xs, ys, zs) = (pick(&X), pick(&y), pick(&z));
        let (rxy, rxz, ryz) = (pearson(&xs, &ys), pearson(&xs, &zs), pearson(&ys, &zs));
        let expected = (rxy - rxz * ryz) / ((1.0 - rxz * rxz) * (1.0 - ryz * ryz)).sqrt();
        assert!((list.r(0, 1) - expected).abs() < 1e-12);
    }

    #[test]
    fn the_refusals_are_named() {
        assert_eq!(partial_correlation(&[&X], &[&Z], false, false), Err(PartialCorrError::TooFewVariables));
        assert_eq!(partial_correlation(&[&X, &Y], &[], false, false), Err(PartialCorrError::NoControls));
        assert_eq!(
            partial_correlation(&[&X, &Y], &[&[3.0; 6]], false, false),
            Err(PartialCorrError::ConstantControl(0))
        );
        assert_eq!(
            partial_correlation(&[&X, &Y], &[&Z, &Z], false, false),
            Err(PartialCorrError::DuplicateControl(0, 1))
        );
        assert_eq!(partial_correlation(&[&X, &Y], &[&X], false, false), Err(PartialCorrError::VariableIsControl(0, 0)));
        let y2: Vec<f64> = X.iter().map(|v| 2.0 * v + 1.0).collect();
        assert_eq!(partial_correlation(&[&X, &y2], &[&Z], false, false), Err(PartialCorrError::PerfectPair(0, 1)));
        // Three rows and one control leave a singular matrix.
        assert!(partial_correlation(&[&X[..3], &Y[..3]], &[&Z[..3]], false, false).is_err());
    }
}
