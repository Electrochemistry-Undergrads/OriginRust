//! The inverse of a matrix as Origin 2016's `minverse` makes it (Analysis > Mathematics > Inverse,
//! measured by T290; probes in `re-workspace/scratch/t290`).
//!
//! - A square matrix of full rank gets its inverse (Gauss-Jordan elimination with partial pivoting).
//! - Any other matrix (singular, or not square) gets its Moore-Penrose pseudoinverse, `cols x rows`,
//!   from the singular value decomposition. Singular values at or below `max(rows, cols) * eps * s_max`
//!   count as zero (measured: a 3 x 3 drops 5e-16 and keeps 7e-16, a 4 x 4 drops 7e-16 and keeps 9.5e-16).
//!   Origin then warns that the matrix is not full rank.
//! - A missing cell takes part as Origin's internal missing-value number, -1.23456789e-300, so in
//!   effect as 0 (Origin's result shows that number's trace, 3.086e-301, where Sheaf's elimination order
//!   gives 0).
//!
//! Origin's full-rank test is not documented. Sheaf's rule reproduces all 25 measured cases: the rows,
//! then the columns, are scaled so that their largest entry is 1. A row or column whose largest entry
//! squares to 0 (below about 1.5e-162) fails. Then LU elimination with partial pivoting fails at a pivot
//! no larger than `n * eps`. So `diag(1e-17, 1)` is inverted to `diag(1e17, 1)`, but `[[1, 2], [2, 4 +
//! 1e-15]]` and `diag(1e-200, 1)` take the pseudoinverse.

use nalgebra::DMatrix;

/// Origin's internal missing value, which `minverse` takes as a number.
pub const ORIGIN_MISSING: f64 = -1.23456789e-300;

/// The result of [`minverse`].
#[derive(Clone, Debug, PartialEq)]
pub struct Inverse {
    /// Row by row, `rows x cols` of the result (the input's `cols x rows`).
    pub values: Vec<f64>,
    pub rows: usize,
    pub cols: usize,
    /// True when the matrix was not square or not of full rank, so this is the pseudoinverse.
    pub pseudo: bool,
}

/// The inverse (or pseudoinverse) of the `rows x cols` matrix `a`, row by row, as Origin's `minverse`;
/// `None` for an empty matrix or a non-finite result.
pub fn minverse(a: &[f64], rows: usize, cols: usize) -> Option<Inverse> {
    if rows == 0 || cols == 0 || a.len() != rows * cols {
        return None;
    }
    let a: Vec<f64> = a.iter().map(|v| if v.is_nan() { ORIGIN_MISSING } else { *v }).collect();
    if a.iter().any(|v| v.is_infinite()) {
        return None;
    }
    let (values, pseudo) = if rows == cols && full_rank(&a, rows) {
        (gauss_jordan(&a, rows)?, false)
    } else {
        (pseudoinverse(&a, rows, cols)?, true)
    };
    values.iter().all(|v| v.is_finite()).then_some(Inverse { values, rows: cols, cols: rows, pseudo })
}

/// Sheaf's copy of Origin's full-rank test for a square matrix (see the module notes).
fn full_rank(a: &[f64], n: usize) -> bool {
    let mut m = a.to_vec();
    for r in 0..n {
        let big = m[r * n..(r + 1) * n].iter().fold(0.0f64, |b, v| b.max(v.abs()));
        if big * big == 0.0 {
            return false;
        }
        m[r * n..(r + 1) * n].iter_mut().for_each(|v| *v /= big);
    }
    for c in 0..n {
        let big = (0..n).fold(0.0f64, |b, r| b.max(m[r * n + c].abs()));
        if big * big == 0.0 {
            return false;
        }
        (0..n).for_each(|r| m[r * n + c] /= big);
    }
    let tol = n as f64 * f64::EPSILON;
    for j in 0..n {
        let p = (j..n).max_by(|&x, &y| m[x * n + j].abs().total_cmp(&m[y * n + j].abs())).unwrap_or(j);
        if m[p * n + j].abs() <= tol {
            return false;
        }
        if p != j {
            for c in 0..n {
                m.swap(p * n + c, j * n + c);
            }
        }
        for r in j + 1..n {
            let f = m[r * n + j] / m[j * n + j];
            if f != 0.0 {
                for c in j..n {
                    m[r * n + c] -= f * m[j * n + c];
                }
            }
        }
    }
    true
}

/// The inverse of a square matrix by Gauss-Jordan elimination with partial pivoting; `None` at a zero pivot.
fn gauss_jordan(a: &[f64], n: usize) -> Option<Vec<f64>> {
    // Rows of [A | I], reduced to [I | A^-1].
    let mut m: Vec<Vec<f64>> = (0..n)
        .map(|r| {
            let mut row = a[r * n..(r + 1) * n].to_vec();
            row.extend((0..n).map(|c| if c == r { 1.0 } else { 0.0 }));
            row
        })
        .collect();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs())).unwrap_or(col);
        if m[pivot][col] == 0.0 {
            return None;
        }
        m.swap(col, pivot);
        let p = m[col][col];
        for v in &mut m[col] {
            *v /= p;
        }
        let pivot_row = m[col].clone();
        for (r, row) in m.iter_mut().enumerate() {
            let f = row[col];
            if r != col && f != 0.0 {
                for (v, p) in row.iter_mut().zip(&pivot_row) {
                    *v -= f * p;
                }
            }
        }
    }
    Some(m.iter().flat_map(|row| row[n..].iter().copied()).collect())
}

/// The Moore-Penrose pseudoinverse (`cols x rows`, row by row) with Origin's cut-off for small singular values.
fn pseudoinverse(a: &[f64], rows: usize, cols: usize) -> Option<Vec<f64>> {
    let m = DMatrix::from_row_slice(rows, cols, a);
    let svd = m.svd(true, true);
    let (u, v_t) = (svd.u?, svd.v_t?);
    let s_max = svd.singular_values.iter().fold(0.0f64, |b, s| b.max(*s));
    let tol = rows.max(cols) as f64 * f64::EPSILON * s_max;
    let mut out = vec![0.0; cols * rows];
    for (k, &s) in svd.singular_values.iter().enumerate() {
        if s <= tol {
            continue;
        }
        for i in 0..cols {
            let vi = v_t[(k, i)] / s;
            for j in 0..rows {
                out[i * rows + j] += vi * u[(j, k)];
            }
        }
    }
    Some(out)
}

#[cfg(test)]
// Origin's numbers as it printed them.
#[allow(clippy::excessive_precision)]
mod tests {
    use super::*;

    fn close(got: &[f64], want: &[f64], tol: f64) {
        assert_eq!(got.len(), want.len());
        for (k, (g, w)) in got.iter().zip(want).enumerate() {
            assert!((g - w).abs() <= tol * w.abs().max(1.0), "{k}: {g} against {w}");
        }
    }

    #[test]
    fn a_full_rank_square_matrix_gets_its_inverse() {
        // Measured in Origin 2016.
        let r = minverse(&[4.0, 7.0, 2.0, 3.0, 6.0, 1.0, 2.0, 5.0, 3.0], 3, 3).unwrap();
        assert!(!r.pseudo);
        let origin = [
            1.4444444444444442,
            -1.2222222222222221,
            -0.55555555555555547,
            -0.77777777777777768,
            0.88888888888888884,
            0.22222222222222221,
            0.33333333333333331,
            -0.66666666666666663,
            0.33333333333333331,
        ];
        close(&r.values, &origin, 1e-14);
        // Badly scaled but of full rank: inverted, not cut.
        let r = minverse(&[1e-17, 0.0, 0.0, 1.0], 2, 2).unwrap();
        assert!(!r.pseudo);
        close(&r.values, &[1e17, 0.0, 0.0, 1.0], 1e-15);
        let r = minverse(&[1.0, 1.0, 1.0, 1.0 + 1e-10], 2, 2).unwrap();
        assert!(!r.pseudo, "a pivot of 1e-10 is full rank");
        close(&r.values, &[9999999173.59636, -9999999172.59636, -9999999172.59636, 9999999172.59636], 1e-9);
    }

    #[test]
    fn origins_full_rank_test_on_every_measured_case() {
        let not_full: [(&[f64], usize); 10] = [
            (&[1.0, 2.0, 2.0, 4.0], 2),
            (&[0.0], 1),
            (&[1e-300, 0.0, 0.0, 1e-300], 2),
            (&[1.0, 2.0, 2.0, 4.0 + 1e-15], 2),
            (&[1e-200, 0.0, 0.0, 1.0], 2),
            (&[2.0, 1.0, 4.0, 2.0000000000000004], 2),
            (&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3),
            (&[1e-320, 0.0, 0.0, 1.0], 2),
            (&[1e-162, 0.0, 0.0, 1.0], 2),
            (&[1.0, 0.0, 0.0, 0.0, 1e-14, 0.0, 0.0, 0.0, 0.0], 3),
        ];
        for (a, n) in not_full {
            assert!(!full_rank(a, n), "{a:?} is not full rank in Origin");
        }
        let full: [(&[f64], usize); 9] = [
            (&[1e-14, 0.0, 0.0, 1.0], 2),
            (&[1e-16, 0.0, 0.0, 1.0], 2),
            (&[1e-100, 0.0, 0.0, 1.0], 2),
            (&[1e-150, 0.0, 0.0, 1e-150], 2),
            (&[1e-160, 0.0, 0.0, 1e-160], 2),
            (&[1e-160, 0.0, 0.0, 1.0], 2),
            (&[1.0, ORIGIN_MISSING, 3.0, 4.0], 2),
            (&[1.0, 1.0, 1.0, 1.0 + 1e-6], 2),
            (&[1.0, 1.0, 1.0, 1.0 + 1e-8], 2),
        ];
        for (a, n) in full {
            assert!(full_rank(a, n), "{a:?} is full rank in Origin");
        }
    }

    #[test]
    fn singular_and_non_square_matrices_get_the_pseudoinverse() {
        // All measured in Origin 2016.
        let r = minverse(&[1.0, 2.0, 2.0, 4.0], 2, 2).unwrap();
        assert!(r.pseudo);
        close(&r.values, &[0.04, 0.08, 0.08, 0.16], 1e-13);
        let r = minverse(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
        assert!(r.pseudo);
        assert_eq!((r.rows, r.cols), (3, 2));
        let origin = [
            -0.94444444444444509,
            0.44444444444444486,
            -0.1111111111111119,
            0.11111111111111149,
            0.722222222222223,
            -0.22222222222222263,
        ];
        close(&r.values, &origin, 1e-12);
        let r = minverse(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2).unwrap();
        assert!(r.pseudo, "a tall matrix of full column rank is still not square");
        assert_eq!((r.rows, r.cols), (2, 3));
        let origin = [
            -1.3333333333333335,
            -0.33333333333333282,
            0.66666666666666641,
            1.0833333333333333,
            0.33333333333333287,
            -0.41666666666666641,
        ];
        close(&r.values, &origin, 1e-12);
        let k = minverse(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3).unwrap();
        let origin = [
            -0.6388888888888884,
            -0.16666666666666607,
            0.30555555555555508,
            -0.055555555555555594,
            0.0,
            0.055555555555555566,
            0.52777777777777724,
            0.16666666666666616,
            -0.19444444444444403,
        ];
        close(&k.values, &origin, 1e-12);
        assert_eq!(minverse(&[0.0], 1, 1).unwrap().values, vec![0.0]);
        // The cut-off is max(rows, cols) * eps * s_max.
        let diag = |n: usize, x: f64| {
            let mut a = vec![0.0; n * n];
            a[0] = 1.0;
            a[n + 1] = x;
            minverse(&a, n, n).unwrap().values[n + 1]
        };
        assert_eq!(diag(3, 5e-16), 0.0);
        assert!((diag(3, 7e-16) - 1428571428571428.5).abs() < 1.0);
        assert_eq!(diag(4, 7e-16), 0.0);
        assert!((diag(4, 9.5e-16) - 1052631578947368.4).abs() < 1.0);
        let r = minverse(&[1e-200, 0.0, 0.0, 1.0], 2, 2).unwrap();
        assert!(r.pseudo);
        assert_eq!(r.values, vec![0.0, 0.0, 0.0, 1.0]);
        let r = minverse(&[1e-300, 0.0, 0.0, 1e-300], 2, 2).unwrap();
        assert!(r.pseudo);
        close(&r.values, &[1e300, 0.0, 0.0, 1e300], 1e-13);
    }

    #[test]
    fn a_missing_cell_counts_as_origins_missing_number() {
        let r = minverse(&[1.0, f64::NAN, 3.0, 4.0], 2, 2).unwrap();
        assert!(!r.pseudo);
        close(&r.values, &[1.0, 3.086419725e-301, -0.75, 0.25], 1e-9);
        // Origin's cell holds the trace of the number (3.086e-301); Sheaf's elimination order gives 0.
        assert!(r.values[1].abs() < 1e-300, "{}", r.values[1]);
        assert_eq!(minverse(&[], 0, 0), None);
        assert_eq!(minverse(&[f64::INFINITY], 1, 1), None);
    }
}
