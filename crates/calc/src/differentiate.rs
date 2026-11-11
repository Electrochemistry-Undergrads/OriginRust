//! Differentiation (Origin `differentiate`).
//!
//! First derivative: the average of the slopes of the two segments next to a point, the slope of
//! the single segment at both end points; this holds for uneven X as well (no weighting by
//! spacing). Higher orders apply the first-order rule repeatedly. Optionally the derivative is
//! taken from a local polynomial fit (Savitzky-Golay derivative).

use crate::util::{polyfit_centered, valid_indices};

/// Options of [`differentiate`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffOptions {
    /// Derivative order, 1 to 9.
    pub order: usize,
    /// Use a local polynomial fit instead of differences (Origin `smooth = 1`).
    pub smooth: bool,
    /// Polynomial order of the local fit (`poly`, default 2).
    pub poly: usize,
    /// Points of the window (`npts`, default 20); the window has `2 * (npts / 2) + 1` points.
    pub npts: usize,
}

impl Default for DiffOptions {
    fn default() -> Self {
        DiffOptions { order: 1, smooth: false, poly: 2, npts: 20 }
    }
}

fn first_derivative(x: &[f64], y: &[f64]) -> Vec<f64> {
    let n = x.len();
    let mut d = vec![f64::NAN; n];
    if n < 2 {
        return d;
    }
    let slope: Vec<f64> = (0..n - 1).map(|i| (y[i + 1] - y[i]) / (x[i + 1] - x[i])).collect();
    d[0] = slope[0];
    d[n - 1] = slope[n - 2];
    for i in 1..n - 1 {
        d[i] = (slope[i - 1] + slope[i]) / 2.0;
    }
    d
}

fn factorial(k: usize) -> f64 {
    (1..=k).map(|v| v as f64).product()
}

fn sg_derivative(x: &[f64], y: &[f64], order: usize, poly: usize, npts: usize) -> Vec<f64> {
    let n = x.len();
    let mut out = vec![f64::NAN; n];
    let half = npts / 2;
    let w = (2 * half + 1).min(n);
    if w <= poly || order > poly {
        return out;
    }
    for i in 0..n {
        let lo = i.saturating_sub(half).min(n - w);
        let xs = &x[lo..lo + w];
        let ys = &y[lo..lo + w];
        if let Some((c, scale)) = polyfit_centered(xs, ys, x[i], poly) {
            let mut v = c[order] * factorial(order) / scale.powi(order as i32);
            // Observed quirk: with order 2 the points of the outer half windows come out doubled.
            if order == 2 && (i < half || i + half >= n) && w == 2 * half + 1 {
                v *= 2.0;
            }
            out[i] = v;
        }
    }
    out
}

/// Derivative of `y` with respect to `x`. The result has the length of the input; rows where x or
/// y is missing stay NaN and are left out of the calculation (their neighbours are joined).
pub fn differentiate(x: &[f64], y: &[f64], opts: &DiffOptions) -> Vec<f64> {
    let n = x.len().min(y.len());
    let idx = valid_indices(n, |i| x[i].is_finite() && y[i].is_finite());
    let xs: Vec<f64> = idx.iter().map(|&i| x[i]).collect();
    let mut cur: Vec<f64> = idx.iter().map(|&i| y[i]).collect();
    if opts.smooth {
        cur = sg_derivative(&xs, &cur, opts.order.max(1), opts.poly, opts.npts);
    } else {
        for _ in 0..opts.order.max(1) {
            cur = first_derivative(&xs, &cur);
        }
    }
    let mut out = vec![f64::NAN; n];
    for (&i, v) in idx.iter().zip(cur) {
        out[i] = v;
    }
    out
}
