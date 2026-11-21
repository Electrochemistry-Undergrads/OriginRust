//! Convolution, deconvolution and correlation of two signals and the Hilbert transform, as Origin's Analysis >
//! Signal Processing > Convolution (`conv`), Correlation (`corr1`) and Hilbert Transform
//! (`hilbert`), and Deconvolution (`deconv`). Convolution and correlation sums are taken directly (Origin uses FFTs, so its
//! numbers carry rounding noise of about 1e-15). See `docs/spec/analysis-tools.md` 13.

/// Convolution of `signal` with `response`.
///
/// Linear: the full convolution, `n + m - 1` values. Circular: both are padded with zeros to the
/// longer length `N` and the result has `N` values, `c[k] = sum_j s[j] r[(k - j) mod N]`. With
/// `normalize` the response is first divided by its Euclidean norm. Empty inputs give an empty
/// result.
pub fn convolve(signal: &[f64], response: &[f64], circular: bool, normalize: bool) -> Vec<f64> {
    let (n, m) = (signal.len(), response.len());
    if n == 0 || m == 0 {
        return Vec::new();
    }
    let scale = if normalize { norm(response) } else { 1.0 };
    let r: Vec<f64> = response.iter().map(|v| v / scale).collect();
    if circular {
        let len = n.max(m);
        (0..len).map(|k| (0..n).map(|j| signal[j] * r.get((k + len - j) % len).copied().unwrap_or(0.0)).sum()).collect()
    } else {
        let mut out = vec![0.0; n + m - 1];
        for (i, s) in signal.iter().enumerate() {
            for (j, v) in r.iter().enumerate() {
                out[i + j] += s * v;
            }
        }
        out
    }
}

/// Deconvolution of `signal` by `response`, the inverse of [`convolve`] (Origin's Signal
/// Processing > Deconvolution, `deconv`, documented, not measured; analysis-tools.md 20.1).
///
/// Done with the FFT over the signal's `N` points: the response is padded with zeros to `N`, the
/// signal's spectrum is divided by the response's, and the result has `N` values. So the circular
/// convolution with the same response is undone exactly, and so is the linear one: a linear
/// convolution of `s` (`n` values) gives back `s` followed by `m - 1` zeros. With `normalize` the
/// response is first divided by its Euclidean norm, as in [`convolve`]. Refused when the response
/// is longer than the signal or its spectrum has a zero (nothing can be divided by it).
pub fn deconvolve(signal: &[f64], response: &[f64], normalize: bool, circular: bool) -> Result<Vec<f64>, String> {
    use rustfft::{FftPlanner, num_complex::Complex};
    let (n, m) = (signal.len(), response.len());
    if n == 0 || m == 0 {
        return Ok(Vec::new());
    }
    if m > n {
        return Err(format!("the response ({m} points) is longer than the signal ({n} points)"));
    }
    let scale = if normalize { norm(response) } else { 1.0 };
    // Circular: the FFT over the signal's N points, N values. Linear (Origin 2016's default, T292,
    // measured): both padded with zeros to 2N - 1 points, and the first N - m + 1 values kept.
    let (len, keep) = if circular { (n, n) } else { (2 * n - 1, n - m + 1) };
    let mut planner = FftPlanner::<f64>::new();
    let forward = planner.plan_fft_forward(len);
    let mut c: Vec<Complex<f64>> = (0..len).map(|i| Complex::new(signal.get(i).copied().unwrap_or(0.0), 0.0)).collect();
    let mut r: Vec<Complex<f64>> =
        (0..len).map(|i| Complex::new(response.get(i).map_or(0.0, |v| v / scale), 0.0)).collect();
    forward.process(&mut c);
    forward.process(&mut r);
    let largest = r.iter().map(|z| z.norm()).fold(0.0, f64::max);
    if !largest.is_finite() || largest <= 0.0 || r.iter().any(|z| z.norm() <= largest * 1e-12) {
        return Err("the response's spectrum has a zero, so the signal cannot be divided by it".into());
    }
    for (a, b) in c.iter_mut().zip(&r) {
        *a /= *b;
    }
    planner.plan_fft_inverse(len).process(&mut c);
    Ok(c.iter().take(keep).map(|z| z.re / len as f64).collect())
}

/// Correlation of `a` with `b`, with the lag of each value.
///
/// Both are padded with zeros to the longer length `N`. Linear: lags `-(N-1)` to `N-1`, value
/// `sum_j a[j + k] b[j]` (so `k = 0` is the plain dot product). Circular: lags 0 to `N-1` in
/// Origin's order, value `k` is `sum_j a[(j - k - 1) mod N] b[j]` (measured: Origin lists the
/// circular lags from the far end). With `normalize` the values are divided by
/// `sqrt(sum a^2 * sum b^2)`.
pub fn correlate(a: &[f64], b: &[f64], circular: bool, normalize: bool) -> (Vec<f64>, Vec<f64>) {
    let len = a.len().max(b.len());
    if a.is_empty() || b.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let at = |i: isize| if i >= 0 { a.get(i as usize).copied().unwrap_or(0.0) } else { 0.0 };
    let scale = if normalize { norm(a) * norm(b) } else { 1.0 };
    let n = len as isize;
    if circular {
        let values = (0..n)
            .map(|k| b.iter().enumerate().map(|(j, v)| at((j as isize - k - 1).rem_euclid(n)) * v).sum::<f64>() / scale)
            .collect();
        ((0..len).map(|k| k as f64).collect(), values)
    } else {
        let lags: Vec<isize> = (-(n - 1)..n).collect();
        let values =
            lags.iter().map(|&k| b.iter().enumerate().map(|(j, v)| at(j as isize + k) * v).sum::<f64>() / scale);
        (lags.iter().map(|&k| k as f64).collect(), values.collect())
    }
}

/// The discrete Hilbert transform of `y`, as Origin's Signal Processing > Hilbert Transform
/// (`hilbert`, measured): the imaginary part of the analytic signal made by the FFT (the positive
/// frequencies doubled, the negative ones removed, the zero and, for an even length, the middle
/// frequency kept).
pub fn hilbert(y: &[f64]) -> Vec<f64> {
    use rustfft::{FftPlanner, num_complex::Complex};
    let n = y.len();
    if n == 0 {
        return Vec::new();
    }
    let mut buf: Vec<Complex<f64>> = y.iter().map(|&v| Complex::new(v, 0.0)).collect();
    let mut planner = FftPlanner::<f64>::new();
    planner.plan_fft_forward(n).process(&mut buf);
    for (k, c) in buf.iter_mut().enumerate() {
        let w = if k == 0 || (n.is_multiple_of(2) && k == n / 2) {
            1.0
        } else if k < n.div_ceil(2) {
            2.0
        } else {
            0.0
        };
        *c *= w;
    }
    planner.plan_fft_inverse(n).process(&mut buf);
    buf.iter().map(|c| c.im / n as f64).collect()
}

fn norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

#[cfg(test)]
#[allow(clippy::excessive_precision)] // Origin's printed digits, kept as printed
mod tests {
    use super::*;

    fn close(got: &[f64], want: &[f64]) {
        assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() < 1e-12, "{got:?} vs {want:?}");
        }
    }

    #[test]
    fn deconvolution_undoes_convolution() {
        let (s, r) = ([1.0, 2.0, -1.0, 3.0, 0.5], [2.0, -1.0, 0.5]);
        // Linear: numpy np.real(ifft(fft(np.convolve(s, r)) / fft(r, 7))) gives s and two zeros.
        let d = deconvolve(&convolve(&s, &r, false, false), &r, false, true).unwrap();
        assert_eq!(d.len(), 7);
        for (g, w) in d.iter().zip([1.0, 2.0, -1.0, 3.0, 0.5, 0.0, 0.0]) {
            assert!((g - w).abs() < 1e-12, "{d:?}");
        }
        for normalize in [false, true] {
            let d = deconvolve(&convolve(&s, &r, true, normalize), &r, normalize, true).unwrap();
            for (g, w) in d.iter().zip(s) {
                assert!((g - w).abs() < 1e-12, "{d:?}");
            }
        }
        assert!(deconvolve(&r, &s, false, true).is_err(), "a longer response");
        assert!(deconvolve(&s, &[1.0, -1.0], false, true).is_err(), "a zero at frequency 0");
        assert_eq!(deconvolve(&[], &r, false, true).unwrap(), Vec::<f64>::new());
    }

    /// Origin 2016's `deconv` (T292, measured 2026-11-21).
    #[test]
    fn deconvolution_matches_origin_2016() {
        let close = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
        // Linear (default): N - m + 1 values; an exact linear convolution comes back.
        let lin = deconvolve(&[1.0, 3.0, 5.0, 3.0, 0.0, 0.0], &[1.0, 1.0], false, false).unwrap();
        assert!(close(&lin, &[1.0, 2.0, 3.0, 0.0, 0.0]), "{lin:?}");
        // Normalize (Origin's default) scales by the response's norm.
        let n = deconvolve(&[1.0, 3.0, 5.0, 3.0, 0.0, 0.0], &[1.0, 1.0], true, false).unwrap();
        assert!((n[0] - 2f64.sqrt()).abs() < 1e-12);
        // A signal that is no exact convolution.
        let g = deconvolve(&[2.0, 5.0, 8.0, 11.0, 14.0, 17.0], &[1.0, 1.0], false, false).unwrap();
        assert!(close(&g, &[6.5, -1.5, 9.5, 1.5, 12.5]), "{g:?}");
        let e = deconvolve(&[1.0, 4.0, 2.0, 8.0, 5.0, 7.0, 3.0], &[2.0, 1.0, 0.5], false, false).unwrap();
        let origin = [0.48968613150007, 1.7543946243054, 3.8115497227443e-4, 3.5612107664375, 0.71929932803818];
        assert!(close(&e, &origin), "{e:?}");
        // Circular: the FFT over N points.
        let c = deconvolve(&[1.0, 4.0, 2.0, 8.0, 5.0, 7.0, 3.0], &[2.0, 1.0, 0.5], false, true).unwrap();
        let origin = [
            -0.15575607097438,
            2.0316114576394,
            0.023133288923878,
            3.4805304911282,
            0.75395143220493,
            2.2528916611155,
            0.18506631139103,
        ];
        assert!(close(&c, &origin), "{c:?}");
    }

    #[test]
    fn convolution_matches_origin_conv() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 0.5, 0.25];
        close(&convolve(&a, &b, false, false), &[1.0, 2.5, 4.25, 6.0, 2.75, 1.0]);
        close(
            &convolve(&a, &b, false, true),
            &[
                0.87287156094396945,
                2.1821789023599241,
                3.7097041340118708,
                5.2372293656638167,
                2.4003967925959167,
                0.87287156094396989,
            ],
        );
        close(&convolve(&a, &b, true, false), &[3.75, 3.5, 4.25, 6.0]);
        // A response longer than the signal.
        let s = [2.0, -1.0, 0.5];
        let r = [1.0, 2.0, 3.0, 4.0, 5.0];
        close(&convolve(&s, &r, false, false), &[2.0, 3.0, 4.5, 6.0, 7.5, -3.0, 2.5]);
        close(&convolve(&s, &r, true, false), &[-1.0, 5.5, 4.5, 6.0, 7.5]);
    }

    #[test]
    fn hilbert_matches_origin() {
        close(
            &hilbert(&[1.0, 3.0, -2.0, 4.0, 0.0, 5.0, 2.0, -1.0]),
            &[
                -2.3106601717798214,
                2.0177669529663689,
                -1.2248737341529163,
                -1.3106601717798214,
                -0.18933982822017859,
                -1.5177669529663689,
                3.7248737341529163,
                0.81066017177982141,
            ],
        );
        close(
            &hilbert(&[1.0, 3.0, -2.0, 4.0, 0.0, 5.0, 2.0]),
            &[
                -1.8240214214054773,
                2.9109728183618637,
                -1.2321059856186376,
                -1.2933405193024705,
                0.0075618404315286169,
                -2.3538711542579649,
                3.7848044217911578,
            ],
        );
    }

    #[test]
    fn correlation_matches_origin_corr1() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 0.5, 0.25];
        let (lags, v) = correlate(&a, &b, false, false);
        close(&lags, &[-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0]);
        close(&v, &[0.0, 0.25, 1.0, 2.75, 4.5, 5.0, 4.0]);
        let (_, v) = correlate(&a, &b, false, true);
        close(
            &v,
            &[
                0.0,
                0.039840953644479829,
                0.15936381457791915,
                0.43825049008927763,
                0.717137165600636,
                0.79681907288959564,
                0.63745525831167649,
            ],
        );
        let (lags, v) = correlate(&a, &b, true, false);
        close(&lags, &[0.0, 1.0, 2.0, 3.0]);
        close(&v, &[5.0, 5.25, 4.5, 2.75]);
        let (_, v) = correlate(&a, &a, false, false);
        close(&v, &[4.0, 11.0, 20.0, 30.0, 20.0, 11.0, 4.0]);
        // Five and three points, both orders.
        let c = [1.0, 2.0, 3.0, 4.0, 5.0];
        let d = [2.0, -1.0, 0.5];
        close(&correlate(&c, &d, true, false).1, &[10.0, 3.5, 4.5, 3.0, 1.5]);
        close(&correlate(&d, &c, true, false).1, &[3.0, 4.5, 3.5, 10.0, 1.5]);
        close(&correlate(&d, &c, false, false).1, &[10.0, 3.0, 4.5, 3.0, 1.5, 0.0, 0.5, 0.0, 0.0]);
    }
}
