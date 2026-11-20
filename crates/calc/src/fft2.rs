//! Two-dimensional discrete Fourier transform of a matrix (T196, Origin's `fft2` and `ifft2`).
//!
//! Conventions are numpy's `fft2` and `ifft2`: the forward transform is not normalized, and the
//! inverse divides by `rows * cols`, so that `ifft2(fft2(a)) == a`. The output is not shifted: the
//! zero frequency is the first cell. The data are row-major, `rows * cols` values.

use rustfft::{FftPlanner, num_complex::Complex};

/// The 2D transform of the complex data `(re, im)` (`im` empty means zero), forward or `inverse`.
/// Returns `(re, im)` of the result. Panics if the lengths do not match `rows * cols`.
pub fn fft2(re: &[f64], im: &[f64], rows: usize, cols: usize, inverse: bool) -> (Vec<f64>, Vec<f64>) {
    let n = rows * cols;
    assert_eq!(re.len(), n, "re must hold rows * cols values");
    assert!(im.is_empty() || im.len() == n, "im must be empty or hold rows * cols values");
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    let mut data: Vec<Complex<f64>> =
        (0..n).map(|k| Complex::new(re[k], if im.is_empty() { 0.0 } else { im[k] })).collect();
    let mut planner = FftPlanner::<f64>::new();
    let row_fft = if inverse { planner.plan_fft_inverse(cols) } else { planner.plan_fft_forward(cols) };
    for row in data.chunks_mut(cols) {
        row_fft.process(row);
    }
    let col_fft = if inverse { planner.plan_fft_inverse(rows) } else { planner.plan_fft_forward(rows) };
    let mut column = vec![Complex::new(0.0, 0.0); rows];
    for c in 0..cols {
        for r in 0..rows {
            column[r] = data[r * cols + c];
        }
        col_fft.process(&mut column);
        for r in 0..rows {
            data[r * cols + c] = column[r];
        }
    }
    let scale = if inverse { 1.0 / n as f64 } else { 1.0 };
    (data.iter().map(|z| z.re * scale).collect(), data.iter().map(|z| z.im * scale).collect())
}

#[cfg(test)]
mod tests {
    use super::fft2;

    #[test]
    fn matches_numpy_fft2_and_inverts() {
        // numpy.fft.fft2([[1, 2, 3], [4, 5, 6]]) = [[21, -3+1.732i, -3-1.732i], [-9, 0, 0]]
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let (re, im) = fft2(&a, &[], 2, 3, false);
        let want_re = [21.0, -3.0, -3.0, -9.0, 0.0, 0.0];
        let want_im = [0.0, 3f64.sqrt(), -(3f64.sqrt()), 0.0, 0.0, 0.0];
        for k in 0..6 {
            assert!((re[k] - want_re[k]).abs() < 1e-12 && (im[k] - want_im[k]).abs() < 1e-12, "{k}: {re:?} {im:?}");
        }
        let (back_re, back_im) = fft2(&re, &im, 2, 3, true);
        for k in 0..6 {
            assert!((back_re[k] - a[k]).abs() < 1e-12 && back_im[k].abs() < 1e-12, "{back_re:?}");
        }
        assert_eq!(fft2(&[], &[], 0, 0, false), (Vec::new(), Vec::new()));
    }
}
