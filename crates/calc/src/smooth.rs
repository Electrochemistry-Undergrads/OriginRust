//! Smoothing (Origin `smooth`): adjacent averaging (plain or weighted), Savitzky-Golay and the
//! percentile filter, with Origin's boundary conditions. The FFT filter is not implemented.
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
        };
    }
    out
}

/// Smooths `y`. The result has the length of the input; missing values stay NaN.
pub fn smooth(y: &[f64], opts: &SmoothOptions) -> Vec<f64> {
    let idx = valid_indices(y.len(), |i| y[i].is_finite());
    let yv: Vec<f64> = idx.iter().map(|&i| y[i]).collect();
    let sm = smooth_valid(&yv, opts);
    let mut out = vec![f64::NAN; y.len()];
    for (&i, v) in idx.iter().zip(sm) {
        out[i] = v;
    }
    out
}
