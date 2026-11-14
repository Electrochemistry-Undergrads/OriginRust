//! Convolution and correlation of two signals, as Origin's Analysis > Signal Processing >
//! Convolution (`conv`) and Correlation (`corr1`). The sums are taken directly (Origin uses FFTs,
//! so its numbers carry rounding noise of about 1e-15). See `docs/spec/analysis-tools.md` 13.

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
