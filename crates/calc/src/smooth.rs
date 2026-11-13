//! Smoothing (Origin `smooth`): adjacent averaging (plain or weighted), Savitzky-Golay, the
//! percentile filter (with Origin's boundary conditions), the FFT filter, Lowess, Loess and the
//! binomial filter.
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
    /// Local linear regression with tricube weights (Origin `lw`); needs the X values. The window
    /// is the `ceil(span * n)` nearest points in X, or `npts` points when `span` is not a
    /// proportion. No robustness iterations; `boundary` does not apply.
    Lowess { span: LocalSpan },
    /// Like [`SmoothMethod::Lowess`] with a local quadratic (Origin `le`).
    Loess { span: LocalSpan },
    /// The three-point filter `1 2 1 / 4` applied `order` times (Origin `bm`, its `order`);
    /// `npts` does not apply. Points near an end use a shorter window when `boundary` is
    /// [`Boundary::None`]; reflection mirrors around the end point (y[-1] = y[1]).
    Binomial { order: usize },
}

/// How Lowess and Loess choose the points of each local fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocalSpan {
    /// A fraction of the points (Origin `span`, 0 to 1, default 0.1).
    Proportion(f64),
    /// A fixed number of points, `SmoothOptions::npts`.
    Points,
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
            SmoothMethod::FftFilter
            | SmoothMethod::Lowess { .. }
            | SmoothMethod::Loess { .. }
            | SmoothMethod::Binomial { .. } => unreachable!("handled before the loop"),
        };
    }
    out
}

/// One pass of the binomial filter `1 2 1 / 4`.
fn binomial_pass(s: &[f64], b: Boundary) -> Vec<f64> {
    let n = s.len();
    if n < 2 {
        return s.to_vec();
    }
    let at = |idx: isize| -> f64 {
        if (0..n as isize).contains(&idx) {
            return s[idx as usize];
        }
        match b {
            // Mirror around the end point: y[-1] = y[1].
            Boundary::Reflect => {
                let period = 2 * (n as isize - 1);
                let r = idx.rem_euclid(period);
                s[if r < n as isize { r } else { period - r } as usize]
            }
            Boundary::Periodic => s[idx.rem_euclid(n as isize) as usize],
            Boundary::Repeat => s[idx.clamp(0, n as isize - 1) as usize],
            Boundary::Extrapolate => {
                if idx < 0 {
                    s[0] + idx as f64 * (s[1] - s[0])
                } else {
                    s[n - 1] + (idx - (n as isize - 1)) as f64 * (s[n - 1] - s[n - 2])
                }
            }
            Boundary::None => unreachable!(),
        }
    };
    (0..n)
        .map(|i| {
            if b == Boundary::None && (i == 0 || i == n - 1) {
                s[i]
            } else {
                (at(i as isize - 1) + 2.0 * s[i] + at(i as isize + 1)) / 4.0
            }
        })
        .collect()
}

/// Local regression at every point: the `q` points nearest in X, tricube weights on the distance
/// scaled by the largest of those distances, a polynomial of degree `deg` evaluated at the point.
fn local_regression(x: &[f64], y: &[f64], q: usize, deg: usize) -> Vec<f64> {
    let n = y.len();
    let q = q.clamp(1, n);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if q <= deg {
            out.push(y[i]);
            continue;
        }
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| (x[a] - x[i]).abs().total_cmp(&(x[b] - x[i]).abs()));
        order.truncate(q);
        let h = order.iter().map(|&j| (x[j] - x[i]).abs()).fold(0.0, f64::max);
        let (mut xs, mut ys, mut ws) = (Vec::new(), Vec::new(), Vec::new());
        for &j in &order {
            let u = if h > 0.0 { (x[j] - x[i]).abs() / h } else { 0.0 };
            let w = (1.0 - u.powi(3)).powi(3);
            if w > 0.0 {
                xs.push(x[j] - x[i]);
                ys.push(y[j]);
                ws.push(w);
            }
        }
        out.push(weighted_poly_at_zero(&xs, &ys, &ws, deg).unwrap_or(y[i]));
    }
    out
}

/// Value at 0 of the weighted least-squares polynomial of degree `deg` (normal equations).
fn weighted_poly_at_zero(xs: &[f64], ys: &[f64], ws: &[f64], deg: usize) -> Option<f64> {
    let m = deg + 1;
    if xs.len() < m {
        return None;
    }
    // Scale x so the normal equations stay well conditioned.
    let scale = xs.iter().fold(0.0_f64, |a, v| a.max(v.abs()));
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let mut a = vec![vec![0.0; m + 1]; m];
    for ((&x, &y), &w) in xs.iter().zip(ys).zip(ws) {
        let t = x / scale;
        for r in 0..m {
            for c in 0..m {
                a[r][c] += w * t.powi((r + c) as i32);
            }
            a[r][m] += w * y * t.powi(r as i32);
        }
    }
    for col in 0..m {
        let piv = (col..m).max_by(|&p, &q| a[p][col].abs().total_cmp(&a[q][col].abs()))?;
        if a[piv][col].abs() < 1e-14 * a[0][0].abs().max(1e-300) {
            return None;
        }
        a.swap(col, piv);
        for r in 0..m {
            if r != col {
                let f = a[r][col] / a[col][col];
                for c in col..=m {
                    a[r][c] -= f * a[col][c];
                }
            }
        }
    }
    Some(a[0][m] / a[0][0])
}

/// FFT filter of Origin (`smooth method:=fft`, default script options), for points with
/// ascending X. Rules (all observed, see `docs/spec/analysis-tools.md`):
///
/// 1. A straight baseline is subtracted: the line from the mean of the first `w` points (at the
///    first X) to the mean of the last `w` points (at the last X), `w = n / 100` limited to
///    10..=100 for `n > 10` and `1` otherwise (the upper limit 100 was found at n = 20000 and 50000;
///    n / 100 was found up to n = 5000).
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
    let w = if n > 10 { (n / 100).clamp(10, 100) } else { 1 };
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
    let local = matches!(opts.method, SmoothMethod::Lowess { .. } | SmoothMethod::Loess { .. });
    let idx =
        valid_indices(y.len(), |i| y[i].is_finite() && (!(fft || local) || x.get(i).is_some_and(|v| v.is_finite())));
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
    let sm = match opts.method {
        SmoothMethod::Lowess { span } | SmoothMethod::Loess { span } => {
            let deg = if matches!(opts.method, SmoothMethod::Lowess { .. }) { 1 } else { 2 };
            let q = match span {
                LocalSpan::Proportion(p) => (p.clamp(0.0, 1.0) * yv.len() as f64).ceil() as usize,
                LocalSpan::Points => opts.npts,
            };
            let xv: Vec<f64> = idx.iter().map(|&i| x[i]).collect();
            local_regression(&xv, &yv, q, deg)
        }
        SmoothMethod::Binomial { order } => {
            let mut s = yv.clone();
            for _ in 0..order {
                s = binomial_pass(&s, opts.boundary);
            }
            s
        }
        _ => smooth_valid(&yv, opts),
    };
    for (&i, v) in idx.iter().zip(sm) {
        out[i] = v;
    }
    out
}
