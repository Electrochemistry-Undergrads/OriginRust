//! Integration (Origin `integ1`).
//!
//! Trapezoid rule over the valid points of a row range, optional end-point baseline, absolute
//! area, cumulative curve, and the peak figures Origin reports with the area (position and height
//! of the largest magnitude, width at half height).

/// Which area is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AreaType {
    /// Signed area (`type = math`).
    #[default]
    Mathematical,
    /// Area of the absolute value of the curve (`type = abs`).
    Absolute,
}

/// Options of [`integrate`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IntegrateOptions {
    pub area_type: AreaType,
    /// Subtract the straight line through the first and last valid point first (`baseline = 1`).
    pub end_point_baseline: bool,
    /// Rows to integrate, 1-based inclusive. `None` means all rows.
    pub rows: Option<(usize, usize)>,
}

/// Result of [`integrate`].
#[derive(Debug, Clone, PartialEq)]
pub struct IntegrateResult {
    /// Integral over the range (Origin `area`).
    pub area: f64,
    /// X of the first and last valid point of the range (`x1`, `x2`).
    pub x1: f64,
    pub x2: f64,
    /// Row of the first point of the range and `i1 + valid points - 1` (`i1`, `i2`).
    pub i1: usize,
    pub i2: usize,
    /// Signed value of the point with the largest magnitude, after the baseline (`y0`).
    pub peak_height: f64,
    /// X of that point (`x0`).
    pub peak_x: f64,
    /// Width at half height (`dx`); negative for descending X.
    pub width: f64,
    /// Cumulative integral, one value per row of the range; NaN for rows that are not valid.
    pub cumulative: Vec<f64>,
}

fn segment(h: f64, ya: f64, yb: f64, abs: bool) -> f64 {
    if !abs {
        return h * (ya + yb) / 2.0;
    }
    let h = h.abs();
    // Compare signs directly: the product ya * yb underflows to zero below about 1e-154.
    if ya == 0.0 || yb == 0.0 || (ya > 0.0) == (yb > 0.0) {
        return (h * (ya + yb) / 2.0).abs();
    }
    // The segment crosses zero: the two triangles have area h (ya^2 + yb^2) / (2 (|ya| + |yb|)).
    // Written with the fractions a and b so that squaring tiny values cannot underflow.
    let s = ya.abs() + yb.abs();
    let (a, b) = (ya.abs() / s, yb.abs() / s);
    h * s * (a * a + b * b) / 2.0
}

/// Integrates y over x. Returns `None` when fewer than two valid points are in the range.
///
/// Points where x or y is NaN are skipped, not interpolated: the neighbours are joined directly.
pub fn integrate(x: &[f64], y: &[f64], opts: &IntegrateOptions) -> Option<IntegrateResult> {
    let n = x.len().min(y.len());
    let (lo, hi) = match opts.rows {
        Some((a, b)) => (a.max(1), b.min(n)),
        None => (1, n),
    };
    if lo > hi {
        return None;
    }
    let valid: Vec<usize> = (lo - 1..hi).filter(|&i| x[i].is_finite() && y[i].is_finite()).collect();
    if valid.len() < 2 {
        return None;
    }
    let xs: Vec<f64> = valid.iter().map(|&i| x[i]).collect();
    let mut ys: Vec<f64> = valid.iter().map(|&i| y[i]).collect();
    let m = xs.len();
    if opts.end_point_baseline {
        let (xa, ya, xb, yb) = (xs[0], ys[0], xs[m - 1], ys[m - 1]);
        for (xv, yv) in xs.iter().zip(ys.iter_mut()) {
            let line = if xb == xa { ya } else { ya + (yb - ya) * (xv - xa) / (xb - xa) };
            *yv -= line;
        }
    }
    let abs = opts.area_type == AreaType::Absolute;
    let mut cum_valid = Vec::with_capacity(m);
    let mut acc = 0.0;
    cum_valid.push(0.0);
    let mut math_abs_x = 0.0; // signed area with |dx|, decides the direction of the peak
    for j in 1..m {
        acc += segment(xs[j] - xs[j - 1], ys[j - 1], ys[j], abs);
        cum_valid.push(acc);
        math_abs_x += (xs[j] - xs[j - 1]).abs() * (ys[j - 1] + ys[j]) / 2.0;
    }
    let mut cumulative = vec![f64::NAN; hi - lo + 1];
    for (&row, &c) in valid.iter().zip(&cum_valid) {
        cumulative[row - (lo - 1)] = c;
    }
    // peak: largest magnitude, first one wins a tie
    let mut k = 0;
    for j in 1..m {
        if ys[j].abs() > ys[k].abs() {
            k = j;
        }
    }
    let width = half_height_width(&xs, &ys, k, math_abs_x >= 0.0);
    Some(IntegrateResult {
        area: acc,
        x1: xs[0],
        x2: xs[m - 1],
        i1: lo,
        i2: lo + m - 1,
        peak_height: ys[k],
        peak_x: xs[k],
        width,
        cumulative,
    })
}

/// Width at half height as Origin reports it (`integ1.dx`), see the spec for the derivation.
fn half_height_width(xs: &[f64], ys: &[f64], k: usize, positive_area: bool) -> f64 {
    let m = xs.len();
    let y0 = ys[k];
    let lev = y0 / 2.0;
    let inside = |v: f64| if positive_area { v >= lev } else { v <= lev };
    // left edge: first point from the left that is inside
    let xl = match (0..m).find(|&j| inside(ys[j])) {
        None => xs[k],
        Some(0) => xs[0],
        Some(j) => xs[j - 1] + (lev - ys[j - 1]) * (xs[j] - xs[j - 1]) / (ys[j] - ys[j - 1]),
    };
    // right edge: first point from the right that is inside
    let xr = match (0..m).rev().find(|&j| inside(ys[j])) {
        None => xs[k],
        Some(j) if j == m - 1 => {
            if k == m - 1 {
                // Observed quirk: with the peak on the last point Origin reports the position of
                // the left crossing itself as the width, i.e. the right edge is taken as 2 * xl.
                2.0 * xl
            } else if ys[m - 1] == y0 {
                xs[m - 1]
            } else {
                xs[k] + (lev - y0) * (xs[m - 1] - xs[k]) / (ys[m - 1] - y0)
            }
        }
        Some(j) => xs[j] + (lev - ys[j]) * (xs[j + 1] - xs[j]) / (ys[j + 1] - ys[j]),
    };
    xr - xl
}

/// Charge from current versus time: the signed trapezoid integral of the current.
///
/// With current in A and time in s the result is in coulomb. Missing points are skipped.
/// Returns `None` for fewer than two valid points.
pub fn charge(time: &[f64], current: &[f64], rows: Option<(usize, usize)>) -> Option<f64> {
    integrate(time, current, &IntegrateOptions { rows, ..IntegrateOptions::default() }).map(|r| r.area)
}
