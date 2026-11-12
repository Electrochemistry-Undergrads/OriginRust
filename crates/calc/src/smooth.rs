//! Smoothing (Origin `smooth`): adjacent averaging (plain or weighted), Savitzky-Golay, the
//! percentile filter (with Origin's boundary conditions) and the FFT filter.
//!
//! Window: `npts` points means `2 * (npts / 2) + 1` points centred on the point, so an even
//! `npts` behaves like the next odd number. Missing values are removed before smoothing and put
//! back (as NaN) afterwards.

use crate::util::{polyfit_centered, valid_indices};

/// What happens where the window sticks out of the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Boundary {
    /// The window shrinks symmetrically (adjacent averaging, percentile); Savitzky-Golay fits the
    /// first or last full window and evaluates it at the edge point.
    #[default]
    None,
    /// Mirror including the end point: y[-1] = y[0], y[-2] = y[1], ...
    Reflect,
    /// The end value is repeated.
    Repeat,
    /// The data wrap around.
    Periodic,
    /// Straight-line extension through the two end points.
    Extrapolate,
}

/// Smoothing method.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SmoothMethod {
    /// Moving average; `weighted` uses parabolic weights `1 - (j / (h + 1))^2`.
    AdjacentAveraging { weighted: bool },
    /// Local polynomial fit of the given order, evaluated at the window centre.
    SavitzkyGolay { poly_order: usize },
    /// Percentile of the window, taken as a data value (no interpolation).
    Percentile { percent: f64 },
    /// Low-pass FFT filter with a parabolic window; `npts` sets the cutoff frequency
    /// `1 / (2 dx npts)` instead of a window length. Needs the X values (see [`smooth_xy`]);
    /// `boundary` does not apply. Origin's script-only options (padding, old filter, baseline
    /// choice) are not offered; the defaults are used.
    FftFilter,
}

/// Options of [`smooth`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmoothOptions {
    pub method: SmoothMethod,
    /// Points of the window (default 5).
    pub npts: usize,
    pub boundary: Boundary,
}

impl Default for SmoothOptions {
    /// Origin's defaults: Savitzky-Golay, 5 points, polynomial order 2, no boundary extension.
    fn default() -> Self {
        SmoothOptions { method: SmoothMethod::SavitzkyGolay { poly_order: 2 }, npts: 5, boundary: Boundary::None }
    }
}

fn map_index(idx: isize, n: usize, b: Boundary) -> Option<usize> {
    let ni = n as isize;
    if (0..ni).contains(&idx) {
        return Some(idx as usize);
    }
    match b {
        Boundary::Reflect => {
            let r = idx.rem_euclid(2 * ni);
            Some(if r < ni { r } else { 2 * ni - 1 - r } as usize)
        }
        Boundary::Periodic => Some(idx.rem_euclid(ni) as usize),
        Boundary::Repeat => Some(idx.clamp(0, ni - 1) as usize),
        _ => None,
    }
}

/// Value of the padded series at (possibly out of range) index `idx`.
fn padded(y: &[f64], idx: isize, b: Boundary) -> f64 {
    let n = y.len();
    match b {
        Boundary::Extrapolate if n >= 2 => {
            if idx < 0 {
                y[0] + idx as f64 * (y[1] - y[0])
            } else if idx >= n as isize {
                y[n - 1] + (idx - (n as isize - 1)) as f64 * (y[n - 1] - y[n - 2])
            } else {
                y[idx as usize]
            }
        }
        _ => y[map_index(idx, n, b).unwrap_or_else(|| idx.clamp(0, n as isize - 1) as usize)],
    }
}

fn smooth_valid(y: &[f64], opts: &SmoothOptions) -> Vec<f64> {
    let n = y.len();
    let h = opts.npts / 2;
    if n == 0 || h == 0 {
        return y.to_vec();
    }
    let mut out = vec![0.0; n];
    for i in 0..n {
        out[i] = match opts.method {
            SmoothMethod::AdjacentAveraging { weighted } => {
                let k = if opts.boundary == Boundary::None { h.min(i).min(n - 1 - i) } else { h };
                let (mut s, mut sw) = (0.0, 0.0);
                for j in -(k as isize)..=(k as isize) {
                    let w = if weighted {
                        let r = j as f64 / (h as f64 + 1.0);
                        1.0 - r * r
                    } else {
                        1.0
                    };
                    s += w * padded(y, i as isize + j, opts.boundary);
                    sw += w;
                }
                s / sw
            }
            SmoothMethod::Percentile { percent } => {
                let k = if opts.boundary == Boundary::None { h.min(i).min(n - 1 - i) } else { h };
                let mut w: Vec<f64> =
                    (-(k as isize)..=(k as isize)).map(|j| padded(y, i as isize + j, opts.boundary)).collect();
                w.sort_by(|a, b| a.total_cmp(b));
                let m = w.len();
                let pos = ((m as f64 * percent / 100.0).ceil() as isize - 1).clamp(0, m as isize - 1);
                w[pos as usize]
            }
            SmoothMethod::SavitzkyGolay { poly_order } => {
                let wlen = 2 * h + 1;
                if opts.boundary == Boundary::None {
                    let w = wlen.min(n);
                    if w <= poly_order {
                        y[i]
                    } else {
                        let lo = i.saturating_sub(h).min(n - w);
                        let xs: Vec<f64> = (lo..lo + w).map(|v| v as f64).collect();
                        polyfit_centered(&xs, &y[lo..lo + w], i as f64, poly_order).map_or(y[i], |(c, _)| c[0])
                    }
                } else if wlen <= poly_order {
                    y[i]
                } else {
                    let xs: Vec<f64> = (-(h as isize)..=(h as isize)).map(|v| v as f64).collect();
                    let ys: Vec<f64> =
                        (-(h as isize)..=(h as isize)).map(|j| padded(y, i as isize + j, opts.boundary)).collect();
                    polyfit_centered(&xs, &ys, 0.0, poly_order).map_or(y[i], |(c, _)| c[0])
                }
            }
            SmoothMethod::FftFilter => unreachable!("handled before the loop"),
        };
    }
    out
}

/// FFT filter of Origin 2016 (`smooth method:=fft`, default script options), for points with
/// ascending X. Rules (all observed, see `docs/spec/analysis-tools.md`):
///
/// 1. A straight baseline is subtracted: the line from the mean of the first `w` points (at the
///    first X) to the mean of the last `w` points (at the last X), `w = max(10, n / 100)` for
///    `n > 10` and `1` otherwise.
/// 2. The remainder is transformed with a plain length-`n` FFT (no padding). With `R` the X range
///    the bin spacing is `df = 1 / R`, and the cutoff is `fc = 1 / (dx * 2 * npts)`, `dx = R / (n-1)`.
/// 3. Cosine part `k` is kept when `(k + 1) df <= fc`, sine part `k` when `k df <= fc` (the sine
///    part of the first bin is always kept); a kept part is multiplied by `1 - (k df / fc)^2`
///    (by 1 for `k = 1`). The mean is kept. The Nyquist bin of an even `n` is dropped.
/// 4. After the inverse transform the baseline is added back.
fn fft_filter(x: &[f64], y: &[f64], npts: usize) -> Vec<f64> {
    use rustfft::{FftPlanner, num_complex::Complex};
    let n = y.len();
    if n < 2 {
        return y.to_vec();
    }
    let range = x[n - 1] - x[0];
    if range.is_nan() || range <= 0.0 || range.is_infinite() {
        return y.to_vec();
    }
    let dx = range / (n - 1) as f64;
    let df = 1.0 / range;
    let fc = 1.0 / dx / (2.0 * npts.max(1) as f64);
    let w = if n > 10 { (n / 100).max(10) } else { 1 };
    let a = y[..w].iter().sum::<f64>() / w as f64;
    let b = y[n - w..].iter().sum::<f64>() / w as f64;
    let base: Vec<f64> = x.iter().map(|&xi| a + (b - a) * (xi - x[0]) / range).collect();
    let mut buf: Vec<Complex<f64>> = y.iter().zip(&base).map(|(yi, bi)| Complex::new(yi - bi, 0.0)).collect();
    let mut planner = FftPlanner::<f64>::new();
    planner.plan_fft_forward(n).process(&mut buf);
    let mut spec = vec![Complex::new(0.0, 0.0); n];
    spec[0] = buf[0];
    for k in 1..=(n - 1) / 2 {
        let weight = if k == 1 {
            1.0
        } else {
            let r = k as f64 * df / fc;
            1.0 - r * r
        };
        let keep_cos = (k + 1) as f64 * df <= fc;
        let keep_sin = k == 1 || k as f64 * df <= fc;
        let re = if keep_cos { buf[k].re * weight } else { 0.0 };
        let im = if keep_sin { buf[k].im * weight } else { 0.0 };
        spec[k] = Complex::new(re, im);
        spec[n - k] = Complex::new(re, -im);
    }
    planner.plan_fft_inverse(n).process(&mut spec);
    spec.iter().zip(&base).map(|(s, bi)| s.re / n as f64 + bi).collect()
}

/// Smooths `y` taking the row number as X (it matters only for the FFT filter, whose cutoff
/// depends on the X spacing). The result has the length of the input; missing values stay NaN.
pub fn smooth(y: &[f64], opts: &SmoothOptions) -> Vec<f64> {
    let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
    smooth_xy(&x, y, opts)
}

/// Smooths `y` against the X values `x` (same length). Rows where X or Y is missing are removed
/// before smoothing and come back as NaN. The FFT filter sorts the points by X first (Origin does
/// the same) and returns the values in the original row order; the other methods work in row order.
pub fn smooth_xy(x: &[f64], y: &[f64], opts: &SmoothOptions) -> Vec<f64> {
    let fft = opts.method == SmoothMethod::FftFilter;
    let idx = valid_indices(y.len(), |i| y[i].is_finite() && (!fft || x.get(i).is_some_and(|v| v.is_finite())));
    let mut out = vec![f64::NAN; y.len()];
    if fft {
        let mut order = idx;
        order.sort_by(|&a, &b| x[a].total_cmp(&x[b]));
        let xv: Vec<f64> = order.iter().map(|&i| x[i]).collect();
        let yv: Vec<f64> = order.iter().map(|&i| y[i]).collect();
        for (&i, v) in order.iter().zip(fft_filter(&xv, &yv, opts.npts)) {
            out[i] = v;
        }
        return out;
    }
    let yv: Vec<f64> = idx.iter().map(|&i| y[i]).collect();
    let sm = smooth_valid(&yv, opts);
    for (&i, v) in idx.iter().zip(sm) {
        out[i] = v;
    }
    out
}
