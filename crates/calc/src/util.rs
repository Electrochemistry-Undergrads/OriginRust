//! Small shared helpers.

/// Least-squares polynomial fit of degree `deg` to `(xs, ys)`, returning the coefficients
/// `c[0] + c[1] u + c[2] u^2 + ...` in the variable `u = (x - x0) / scale`, together with `scale`.
/// Returns `None` when the system is singular.
pub(crate) fn polyfit_centered(xs: &[f64], ys: &[f64], x0: f64, deg: usize) -> Option<(Vec<f64>, f64)> {
    let scale = xs.iter().map(|v| (v - x0).abs()).fold(0.0_f64, f64::max);
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let m = deg + 1;
    // normal equations on the scaled variable
    let mut a = vec![vec![0.0; m + 1]; m];
    for (xv, yv) in xs.iter().zip(ys) {
        let u = (xv - x0) / scale;
        let mut pw = vec![1.0; 2 * m];
        for k in 1..2 * m {
            pw[k] = pw[k - 1] * u;
        }
        for r in 0..m {
            for c in 0..m {
                a[r][c] += pw[r + c];
            }
            a[r][m] += pw[r] * yv;
        }
    }
    // Gaussian elimination with partial pivoting
    for col in 0..m {
        let mut piv = col;
        for r in col + 1..m {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        if a[piv][col].abs() < 1e-300 {
            return None;
        }
        a.swap(col, piv);
        for r in col + 1..m {
            let f = a[r][col] / a[col][col];
            let pivot_row = a[col].clone();
            for (dst, src) in a[r].iter_mut().zip(&pivot_row).skip(col) {
                *dst -= f * src;
            }
        }
    }
    let mut sol = vec![0.0; m];
    for r in (0..m).rev() {
        let mut s = a[r][m];
        for c in r + 1..m {
            s -= a[r][c] * sol[c];
        }
        sol[r] = s / a[r][r];
    }
    Some((sol, scale))
}

/// Compresses the entries where `keep` is true; returns the kept indices.
pub(crate) fn valid_indices(n: usize, keep: impl Fn(usize) -> bool) -> Vec<usize> {
    (0..n).filter(|&i| keep(i)).collect()
}
