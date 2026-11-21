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

/// One sweep of a cyclic voltammogram (T296): the points from one turn of the potential to the next.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CvSegment {
    /// Indices of the first and last point (the turning point is shared by the two sweeps it joins).
    pub first: usize,
    pub last: usize,
    /// The potential rises (anodic sweep) rather than falls (cathodic sweep).
    pub anodic: bool,
    pub e_start: f64,
    pub e_end: f64,
    /// The charge passed during the sweep, `integral of I dt = integral of I |dE| / v`: positive for an
    /// anodic (oxidation) current, negative for a cathodic one, in the current's unit times seconds when
    /// the scan rate is in potential units per second.
    pub charge: f64,
}

/// Splits a cyclic voltammogram (potential `e`, current `i`) into its sweeps at every turn of the
/// potential and gives each sweep's charge at `scan_rate` (potential units per second, above 0): the
/// trapezoid sum of `I |dE|` divided by the scan rate (T296). With `end_point_baseline` the straight
/// line through the sweep's first and last point (current against potential) is subtracted first.
/// Missing points are skipped; steps where the potential does not change stay in the sweep. `None`
/// for a bad scan rate or fewer than two usable points.
pub fn cv_charge(e: &[f64], i: &[f64], scan_rate: f64, end_point_baseline: bool) -> Option<Vec<CvSegment>> {
    if !(scan_rate.is_finite() && scan_rate > 0.0) {
        return None;
    }
    let pts: Vec<usize> = (0..e.len().min(i.len())).filter(|&k| e[k].is_finite() && i[k].is_finite()).collect();
    if pts.len() < 2 {
        return None;
    }
    // Runs of points between turns: (index into `pts` of the first and last point, rising).
    let mut runs: Vec<(usize, usize, bool)> = Vec::new();
    let (mut start, mut dir): (usize, Option<bool>) = (0, None);
    for k in 1..pts.len() {
        let d = e[pts[k]] - e[pts[k - 1]];
        if d == 0.0 {
            continue;
        }
        let up = d > 0.0;
        match dir {
            None => dir = Some(up),
            Some(was) if was != up => {
                runs.push((start, k - 1, was));
                start = k - 1;
                dir = Some(up);
            }
            _ => {}
        }
    }
    runs.push((start, pts.len() - 1, dir.unwrap_or(true)));
    let segments = runs
        .into_iter()
        .filter(|&(a, b, _)| b > a)
        .map(|(a, b, up)| {
            let (f, l) = (pts[a], pts[b]);
            let base = |k: usize| {
                if end_point_baseline && e[l] != e[f] {
                    i[f] + (i[l] - i[f]) * (e[k] - e[f]) / (e[l] - e[f])
                } else {
                    0.0
                }
            };
            let sum: f64 = (a..b)
                .map(|k| {
                    let (p, q) = (pts[k], pts[k + 1]);
                    0.5 * ((i[p] - base(p)) + (i[q] - base(q))) * (e[q] - e[p]).abs()
                })
                .sum();
            CvSegment { first: f, last: l, anodic: up, e_start: e[f], e_end: e[l], charge: sum / scan_rate }
        })
        .collect();
    Some(segments)
}

/// Charge from current versus time: the signed trapezoid integral of the current.
///
/// With current in A and time in s the result is in coulomb. Missing points are skipped.
/// Returns `None` for fewer than two valid points.
pub fn charge(time: &[f64], current: &[f64], rows: Option<(usize, usize)>) -> Option<f64> {
    integrate(time, current, &IntegrateOptions { rows, ..IntegrateOptions::default() }).map(|r| r.area)
}

#[cfg(test)]
mod cv_tests {
    use super::*;

    #[test]
    fn a_voltammogram_splits_at_its_turns_and_divides_by_the_scan_rate() {
        // 0 -> 1 -> 0 V in steps of 0.25 V at 0.1 V/s; 2 A going up, -1 A coming down.
        let e = [0.0, 0.25, 0.5, 0.75, 1.0, 0.75, 0.5, 0.25, 0.0];
        let i = [2.0, 2.0, 2.0, 2.0, 2.0, -1.0, -1.0, -1.0, -1.0];
        let s = cv_charge(&e, &i, 0.1, false).unwrap();
        assert_eq!(s.len(), 2);
        assert!(s[0].anodic && !s[1].anodic);
        assert_eq!((s[0].first, s[0].last, s[1].first, s[1].last), (0, 4, 4, 8), "the turn is shared");
        // Up: 2 A over 1 V at 0.1 V/s is 10 s, so 20 C. Down: the first step goes from 2 A to -1 A.
        assert!((s[0].charge - 20.0).abs() < 1e-12, "{}", s[0].charge);
        let down = (0.5 * (2.0 - 1.0) * 0.25 + 3.0 * -0.25) / 0.1;
        assert!((s[1].charge - down).abs() < 1e-12, "{} {down}", s[1].charge);
        // A sloping background through the sweep's ends is taken away.
        let ramp: Vec<f64> = e.iter().map(|v| 1.0 + v).collect();
        let b = cv_charge(&e[..5], &ramp[..5], 0.1, true).unwrap();
        assert!(b[0].charge.abs() < 1e-12, "a straight line has no charge above its ends");
        // Missing points are skipped; flat steps stay in the sweep.
        let e2 = [0.0, 0.5, f64::NAN, 0.5, 1.0, 0.5];
        let i2 = [1.0, 1.0, 5.0, 1.0, 1.0, 1.0];
        let s2 = cv_charge(&e2, &i2, 1.0, false).unwrap();
        assert_eq!(s2.len(), 2);
        assert!((s2[0].charge - 1.0).abs() < 1e-12);
        assert!(cv_charge(&e, &i, 0.0, false).is_none());
        assert!(cv_charge(&[1.0], &[1.0], 1.0, false).is_none());
    }
}
