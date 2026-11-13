//! Curve mathematics: the mean of several curves, the intersections of two curves and the area of a
//! closed curve.
//!
//! None of the rules here were measured in Origin (see `docs/spec/analysis-curve-math.md`, section
//! "Unmeasured"); the tests use values worked out by hand from the formulas.
//!
//! All functions take parallel `x` and `y` slices. A point with a missing (non-finite) X or Y is
//! dropped before anything else.

use crate::interp::{Extrapolation, InterpMethod, InterpOptions, interp1};
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------------------------
// Average of several curves
// ---------------------------------------------------------------------------------------------

/// Where [`average_curves`] evaluates the curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommonX {
    /// Every X value of every curve inside the range, sorted and without repeats. When all curves
    /// have exactly the same X values (the usual case of scans of one voltammogram that share an X
    /// column) the X values are used as they are, row by row, in their original order and without
    /// interpolation; that case also accepts X values that are not monotonic (a cyclic sweep).
    Points,
    /// This many evenly spaced X values from the start to the end of the range.
    Evenly(usize),
    /// The X values of one curve (0-based index in the list of curves) that lie inside the range,
    /// in increasing order.
    Reference(usize),
}

/// Which X range [`average_curves`] covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum XRange {
    /// Only X values covered by every curve (the largest start to the smallest end).
    #[default]
    Intersection,
    /// From the smallest start to the largest end. A curve contributes only where it has data (it
    /// is never extrapolated), so the mean there uses fewer curves; the count column says how many.
    Union,
}

/// The interpolation between the points of a curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AverageMethod {
    #[default]
    Linear,
    CubicSpline,
    Akima,
}

/// Options of [`average_curves`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AverageOptions {
    pub x: CommonX,
    pub range: XRange,
    pub method: AverageMethod,
}

impl Default for AverageOptions {
    fn default() -> Self {
        AverageOptions { x: CommonX::Points, range: XRange::Intersection, method: AverageMethod::Linear }
    }
}

/// The mean curve.
#[derive(Debug, Clone, PartialEq)]
pub struct AverageResult {
    pub x: Vec<f64>,
    /// Mean of the curves that have a value at the X (NaN where none has).
    pub mean: Vec<f64>,
    /// Sample standard deviation (divisor n - 1); NaN where fewer than two curves contribute.
    pub sd: Vec<f64>,
    /// Number of curves that contribute at the X.
    pub n: Vec<usize>,
}

/// Why a curve calculation was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum CurveError {
    /// An average needs at least two curves.
    TooFewCurves,
    /// Curve (0-based) has fewer than two points with different X values.
    CurveTooShort(usize),
    /// The X values of a curve (0-based) go up and then down (or the reverse); `row` is the
    /// 1-based position of the first point that turns back, among the points that have numbers.
    NotMonotonic { curve: usize, row: usize },
    /// The curves share no X value.
    NoCommonRange,
    /// The reference curve (0-based) does not exist.
    NoReference(usize),
    /// Fewer than two points were asked for.
    TooFewPoints,
}

impl fmt::Display for CurveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CurveError::TooFewCurves => write!(f, "averaging needs at least two curves"),
            CurveError::CurveTooShort(c) => {
                write!(f, "curve {} has fewer than two points with different X values", c + 1)
            }
            CurveError::NotMonotonic { curve, row } => write!(
                f,
                "the X values of curve {} turn back at its point {row}; curves with different X values must \
                 increase (or decrease) all the way, so choose the rows of one sweep",
                curve + 1
            ),
            CurveError::NoCommonRange => write!(f, "the curves have no X range in common"),
            CurveError::NoReference(c) => write!(f, "there is no curve {} to take the X values from", c + 1),
            CurveError::TooFewPoints => write!(f, "at least two result points are needed"),
        }
    }
}

impl std::error::Error for CurveError {}

fn finite_points(x: &[f64], y: &[f64]) -> Vec<(f64, f64)> {
    x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(&a, &b)| (a, b)).collect()
}

/// Makes the points of one curve increasing in X. Equal neighbouring X values become their mean.
/// Decreasing curves are reversed. A curve that turns back is an error.
fn monotonic_increasing(points: &[(f64, f64)], curve: usize) -> Result<Vec<(f64, f64)>, CurveError> {
    let mut direction = 0.0_f64;
    for (i, w) in points.windows(2).enumerate() {
        let d = w[1].0 - w[0].0;
        if d != 0.0 {
            if direction == 0.0 {
                direction = d.signum();
            } else if d.signum() != direction {
                return Err(CurveError::NotMonotonic { curve, row: i + 2 });
            }
        }
    }
    let mut ordered: Vec<(f64, f64)> =
        if direction < 0.0 { points.iter().rev().copied().collect() } else { points.to_vec() };
    // Merge repeats of an X value.
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(ordered.len());
    let mut count = 0.0;
    for p in ordered.drain(..) {
        match out.last_mut() {
            Some(last) if last.0 == p.0 => {
                count += 1.0;
                last.1 += (p.1 - last.1) / count;
            }
            _ => {
                out.push(p);
                count = 1.0;
            }
        }
    }
    if out.len() < 2 {
        return Err(CurveError::CurveTooShort(curve));
    }
    Ok(out)
}

/// The value of an increasing polyline at `x`, which must lie inside its X range. Exact at the data
/// points.
fn linear_at(pts: &[(f64, f64)], x: f64) -> f64 {
    let i = pts.partition_point(|p| p.0 < x);
    if i == 0 {
        return pts[0].1;
    }
    if i >= pts.len() {
        return pts[pts.len() - 1].1;
    }
    let (x1, y1) = pts[i];
    if x1 == x {
        return y1;
    }
    let (x0, y0) = pts[i - 1];
    y0 + (x - x0) / (x1 - x0) * (y1 - y0)
}

/// The mean, standard deviation and count of curves on a common set of X values.
///
/// `curves` are `(x, y)` slice pairs, one per curve. Each curve is interpolated at the common X
/// values (never extrapolated; see [`XRange`]). Unless all curves have identical X values with
/// [`CommonX::Points`], every curve must be monotonic in X: increasing or decreasing all the way
/// (decreasing curves are reversed; repeated X values are merged into their mean). A curve whose X
/// turns back, such as both sweeps of a cyclic voltammogram in one column, is refused with
/// [`CurveError::NotMonotonic`]; give it one sweep at a time.
pub fn average_curves(curves: &[(&[f64], &[f64])], options: &AverageOptions) -> Result<AverageResult, CurveError> {
    if curves.len() < 2 {
        return Err(CurveError::TooFewCurves);
    }
    let raw: Vec<Vec<(f64, f64)>> = curves.iter().map(|(x, y)| finite_points(x, y)).collect();

    // The curves share one X column: average row by row.
    if options.x == CommonX::Points {
        if let Some(r) = shared_x_average(curves, &raw) {
            return Ok(r);
        }
    }

    let ordered: Vec<Vec<(f64, f64)>> =
        raw.iter().enumerate().map(|(i, c)| monotonic_increasing(c, i)).collect::<Result<_, _>>()?;
    let starts = ordered.iter().map(|c| c[0].0);
    let ends = ordered.iter().map(|c| c[c.len() - 1].0);
    let (lo, hi) = match options.range {
        XRange::Intersection => (starts.fold(f64::NEG_INFINITY, f64::max), ends.fold(f64::INFINITY, f64::min)),
        XRange::Union => (starts.fold(f64::INFINITY, f64::min), ends.fold(f64::NEG_INFINITY, f64::max)),
    };
    if lo > hi {
        return Err(CurveError::NoCommonRange);
    }

    let grid: Vec<f64> = match options.x {
        CommonX::Points => {
            let mut all: Vec<f64> = ordered.iter().flatten().map(|p| p.0).filter(|&x| x >= lo && x <= hi).collect();
            all.sort_by(f64::total_cmp);
            all.dedup();
            all
        }
        CommonX::Evenly(n) => {
            if n < 2 && lo != hi {
                return Err(CurveError::TooFewPoints);
            }
            if lo == hi {
                vec![lo]
            } else {
                let step = (hi - lo) / (n - 1) as f64;
                (0..n).map(|k| if k == n - 1 { hi } else { lo + step * k as f64 }).collect()
            }
        }
        CommonX::Reference(r) => {
            let c = ordered.get(r).ok_or(CurveError::NoReference(r))?;
            c.iter().map(|p| p.0).filter(|&x| x >= lo && x <= hi).collect()
        }
    };
    if grid.is_empty() {
        return Err(CurveError::NoCommonRange);
    }

    let method = match options.method {
        AverageMethod::Linear => InterpMethod::Linear,
        AverageMethod::CubicSpline => InterpMethod::CubicSpline,
        AverageMethod::Akima => InterpMethod::Akima,
    };
    let span = (hi - lo).abs();
    let columns: Vec<Vec<f64>> = ordered
        .iter()
        .map(|curve| {
            let (c_lo, c_hi) = (curve[0].0, curve[curve.len() - 1].0);
            let tol = 1e-12 * span.max(c_lo.abs()).max(c_hi.abs());
            let inside: Vec<usize> =
                (0..grid.len()).filter(|&k| grid[k] >= c_lo - tol && grid[k] <= c_hi + tol).collect();
            let mut out = vec![f64::NAN; grid.len()];
            if method == InterpMethod::Linear {
                for &k in &inside {
                    out[k] = linear_at(curve, grid[k].clamp(c_lo, c_hi));
                }
            } else {
                let at: Vec<f64> = inside.iter().map(|&k| grid[k].clamp(c_lo, c_hi)).collect();
                let xs: Vec<f64> = curve.iter().map(|p| p.0).collect();
                let ys: Vec<f64> = curve.iter().map(|p| p.1).collect();
                let opts = InterpOptions { method, extrapolation: Extrapolation::Missing, ..InterpOptions::default() };
                for (&k, v) in inside.iter().zip(interp1(&xs, &ys, &at, &opts)) {
                    out[k] = v;
                }
            }
            out
        })
        .collect();

    let (mut mean, mut sd, mut n) = (Vec::new(), Vec::new(), Vec::new());
    for k in 0..grid.len() {
        let (m, s, c) = mean_sd(columns.iter().map(|col| col[k]));
        mean.push(m);
        sd.push(s);
        n.push(c);
    }
    Ok(AverageResult { x: grid, mean, sd, n })
}

/// Row-by-row mean of curves that share their X values, decided from the raw X slices so a missing
/// Y or a shorter column does not hide the shared X. Row `k` uses the finite X values of the curves
/// that have a row `k`; they must all be equal. A curve contributes at a row only when its Y is
/// finite, and rows with no X or no contribution are dropped. `None` when the X values differ, a
/// curve has fewer than two points with numbers, or fewer than two rows remain.
fn shared_x_average(curves: &[(&[f64], &[f64])], raw: &[Vec<(f64, f64)>]) -> Option<AverageResult> {
    if raw.iter().any(|c| c.len() < 2) {
        return None;
    }
    // Row count of each curve: its X and Y both exist.
    let lens: Vec<usize> = curves.iter().map(|(x, y)| x.len().min(y.len())).collect();
    let rows = lens.iter().copied().max()?;
    let (mut xs, mut mean, mut sd, mut n) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for k in 0..rows {
        let mut x_here = f64::NAN;
        for (c, &len) in curves.iter().zip(&lens) {
            if k < len && c.0[k].is_finite() {
                if x_here.is_nan() {
                    x_here = c.0[k];
                } else if c.0[k] != x_here {
                    return None;
                }
            }
        }
        if x_here.is_nan() {
            continue;
        }
        let (m, s, count) = mean_sd(curves.iter().zip(&lens).filter(|&(_, &len)| k < len).map(|(c, _)| {
            if c.0[k].is_finite() { c.1[k] } else { f64::NAN }
        }));
        if count > 0 {
            xs.push(x_here);
            mean.push(m);
            sd.push(s);
            n.push(count);
        }
    }
    (xs.len() >= 2).then_some(AverageResult { x: xs, mean, sd, n })
}

/// Mean, sample standard deviation and count of the finite values.
fn mean_sd(values: impl Iterator<Item = f64> + Clone) -> (f64, f64, usize) {
    let v: Vec<f64> = values.filter(|x| x.is_finite()).collect();
    let n = v.len();
    if n == 0 {
        return (f64::NAN, f64::NAN, 0);
    }
    let mean = v.iter().sum::<f64>() / n as f64;
    let sd = if n < 2 { f64::NAN } else { (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt() };
    (mean, sd, n)
}

// ---------------------------------------------------------------------------------------------
// Intersections of two curves
// ---------------------------------------------------------------------------------------------

/// How two curves meet at an intersection point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeetKind {
    /// The curves pass through each other: near the point, the first curve is on one side of the
    /// second before it and on the other side after it.
    Crossing,
    /// The curves meet and stay on the same side: a tangent point, a vertex of one curve on the
    /// other, an end point on the other curve, or the ends of a stretch where they coincide.
    Touching,
}

/// One point where two curves meet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intersection {
    pub x: f64,
    pub y: f64,
    pub kind: MeetKind,
    /// Position along the first curve: the 0-based index of the point before it plus the fraction of
    /// the way to the next (among the points that have numbers).
    pub position_a: f64,
    /// The same along the second curve.
    pub position_b: f64,
}

#[derive(Clone, Copy)]
struct Hit {
    x: f64,
    y: f64,
    pa: f64,
    pb: f64,
    overlap: bool,
}

fn distinct_points(x: &[f64], y: &[f64]) -> Vec<(f64, f64)> {
    let mut pts = finite_points(x, y);
    pts.dedup();
    pts
}

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 - b.0, a.1 - b.1)
}

/// The directions from the point at `pos` along the polyline to its neighbours: (towards the
/// previous point, towards the next point). `None` at the ends of the curve.
fn neighbours(pts: &[(f64, f64)], pos: f64, at: (f64, f64)) -> (Option<(f64, f64)>, Option<(f64, f64)>) {
    let eps = 1e-9;
    let i = pos.floor() as usize;
    let frac = pos - i as f64;
    let vertex = if frac < eps {
        Some(i)
    } else if frac > 1.0 - eps {
        Some(i + 1)
    } else {
        None
    };
    match vertex {
        Some(v) => {
            let prev = (v > 0).then(|| sub(pts[v - 1], pts[v]));
            let next = (v + 1 < pts.len()).then(|| sub(pts[v + 1], pts[v]));
            (prev, next)
        }
        None => (Some(sub(pts[i], at)), Some(sub(pts[i + 1], at))),
    }
}

fn angle(v: (f64, f64)) -> f64 {
    v.1.atan2(v.0)
}

/// True when the curve A (rays `a`) passes from one side of the curve B (rays `b`) to the other.
fn passes_through(a: (Option<(f64, f64)>, Option<(f64, f64)>), b: (Option<(f64, f64)>, Option<(f64, f64)>)) -> bool {
    let (Some(a1), Some(a2), Some(b1), Some(b2)) = (a.0, a.1, b.0, b.1) else {
        return false;
    };
    let tau = std::f64::consts::TAU;
    let rel = |v: (f64, f64)| (angle(v) - angle(b1)).rem_euclid(tau);
    let wedge = rel(b2);
    let eps = 1e-9;
    // B folds back on itself: no sides.
    if wedge < eps || wedge > tau - eps {
        return false;
    }
    let (p, q) = (rel(a1), rel(a2));
    let on_ray = |r: f64| r < eps || r > tau - eps || (r - wedge).abs() < eps;
    if on_ray(p) || on_ray(q) {
        return false;
    }
    (p < wedge) != (q < wedge)
}

/// The points where curve A meets curve B, both taken as polylines through their points in order
/// (the curves need not be functions of X, so cyclic voltammograms work). They are sorted by
/// position along A.
///
/// With `include_touching` false only [`MeetKind::Crossing`] points are returned. Stretches where
/// the curves coincide are reported by their two ends as [`MeetKind::Touching`], whatever happens
/// after them. A point on both curves that is a vertex of either one is found once.
///
/// The test is pairwise over all segments (cost proportional to the product of the two lengths,
/// with a quick rejection by bounding box).
pub fn intersections(ax: &[f64], ay: &[f64], bx: &[f64], by: &[f64], include_touching: bool) -> Vec<Intersection> {
    let a = distinct_points(ax, ay);
    let b = distinct_points(bx, by);
    if a.len() < 2 || b.len() < 2 {
        return Vec::new();
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for p in a.iter().chain(&b) {
        min_x = min_x.min(p.0);
        max_x = max_x.max(p.0);
        min_y = min_y.min(p.1);
        max_y = max_y.max(p.1);
    }
    let scale = (max_x - min_x).hypot(max_y - min_y).max(f64::MIN_POSITIVE);
    let dist_tol = 1e-12 * scale;
    let e = 1e-10;

    let mut hits: Vec<Hit> = Vec::new();
    // The two ends of every shared stretch of a pair of segments, to find the points inside a stretch.
    let mut stretches: Vec<(Hit, Hit)> = Vec::new();
    for i in 0..a.len() - 1 {
        let (p, p2) = (a[i], a[i + 1]);
        let r = sub(p2, p);
        let (a_lo_x, a_hi_x) = (p.0.min(p2.0) - dist_tol, p.0.max(p2.0) + dist_tol);
        let (a_lo_y, a_hi_y) = (p.1.min(p2.1) - dist_tol, p.1.max(p2.1) + dist_tol);
        for j in 0..b.len() - 1 {
            let (q, q2) = (b[j], b[j + 1]);
            if q.0.max(q2.0) < a_lo_x || q.0.min(q2.0) > a_hi_x || q.1.max(q2.1) < a_lo_y || q.1.min(q2.1) > a_hi_y {
                continue;
            }
            let s = sub(q2, q);
            let denom = cross(r, s);
            let qp = sub(q, p);
            let (rl, sl) = (r.0.hypot(r.1), s.0.hypot(s.1));
            if denom.abs() > 1e-12 * rl * sl {
                let t = cross(qp, s) / denom;
                let u = cross(qp, r) / denom;
                if (-e..=1.0 + e).contains(&t) && (-e..=1.0 + e).contains(&u) {
                    let t = t.clamp(0.0, 1.0);
                    let u = u.clamp(0.0, 1.0);
                    hits.push(Hit {
                        x: p.0 + t * r.0,
                        y: p.1 + t * r.1,
                        pa: i as f64 + t,
                        pb: j as f64 + u,
                        overlap: false,
                    });
                }
            } else if cross(qp, r).abs() / rl <= dist_tol {
                // Parallel and on one line: the shared stretch, if any.
                let rr = r.0 * r.0 + r.1 * r.1;
                let t0 = (qp.0 * r.0 + qp.1 * r.1) / rr;
                let q2p = sub(q2, p);
                let t1 = (q2p.0 * r.0 + q2p.1 * r.1) / rr;
                let (lo, hi) = (t0.min(t1).max(0.0), t0.max(t1).min(1.0));
                if lo <= hi + e {
                    let ss = s.0 * s.0 + s.1 * s.1;
                    let ends: Vec<Hit> = [lo, hi.max(lo)]
                        .into_iter()
                        .map(|t| {
                            let (x, y) = (p.0 + t * r.0, p.1 + t * r.1);
                            let u = (((x - q.0) * s.0 + (y - q.1) * s.1) / ss).clamp(0.0, 1.0);
                            Hit { x, y, pa: i as f64 + t, pb: j as f64 + u, overlap: true }
                        })
                        .collect();
                    stretches.push((ends[0], ends[1]));
                    hits.extend(ends);
                }
            }
        }
    }

    // One point for the hits that are the same place on both curves (a vertex is hit by the two
    // segments next to it).
    hits.sort_by(|h, k| h.pa.total_cmp(&k.pa).then(h.pb.total_cmp(&k.pb)));
    let tol = 1e-9 * scale;
    let mut merged: Vec<Hit> = Vec::new();
    for h in hits {
        match merged
            .iter_mut()
            .rev()
            .take_while(|m| h.pa - m.pa < 1e-6)
            .find(|m| (m.x - h.x).hypot(m.y - h.y) <= tol && (m.pb - h.pb).abs() < 1e-6)
        {
            Some(m) => m.overlap |= h.overlap,
            None => merged.push(h),
        }
    }

    // A point inside a shared stretch (one shared piece arrives at it and the next carries on the
    // same way along both curves) is not an end of the stretch.
    let side = |from: &Hit, h: &Hit| {
        let d = from.pb - h.pb;
        if d.abs() < 1e-9 { 0 } else if d > 0.0 { 1 } else { -1 }
    };
    let inside_stretch = |h: &Hit| {
        let near = |u: &Hit| (u.pa - h.pa).abs() < 1e-6 && (u.pb - h.pb).abs() < 1e-6;
        let arriving = stretches.iter().filter(|(lo, hi)| near(hi) && hi.pa - lo.pa > 1e-9).map(|(lo, _)| side(lo, h));
        let leaving = stretches.iter().filter(|(lo, hi)| near(lo) && hi.pa - lo.pa > 1e-9).map(|(_, hi)| side(hi, h));
        let leaving: Vec<i32> = leaving.collect();
        arriving.into_iter().any(|s1| s1 != 0 && leaving.contains(&-s1))
    };
    merged.retain(|h| !(h.overlap && inside_stretch(h)));

    merged
        .into_iter()
        .filter_map(|h| {
            let kind = if h.overlap {
                MeetKind::Touching
            } else {
                let ra = neighbours(&a, h.pa, (h.x, h.y));
                let rb = neighbours(&b, h.pb, (h.x, h.y));
                if passes_through(ra, rb) { MeetKind::Crossing } else { MeetKind::Touching }
            };
            (include_touching || kind == MeetKind::Crossing).then_some(Intersection {
                x: h.x,
                y: h.y,
                kind,
                position_a: h.pa,
                position_b: h.pb,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Area of a closed curve
// ---------------------------------------------------------------------------------------------

/// The numbers of [`polygon_area`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolygonArea {
    /// Number of corners used (after dropping missing values and a last point equal to the first).
    pub points: usize,
    /// The shoelace sum: positive when the curve runs counterclockwise (in a graph with Y up),
    /// negative when it runs clockwise. For a polygon that crosses itself, the parts that run the
    /// other way count negatively.
    pub signed_area: f64,
    /// `|signed_area|`.
    pub area: f64,
    /// Centre of area of the polygon; `None` when the signed area is zero (also for a figure eight
    /// whose lobes cancel).
    pub centroid: Option<(f64, f64)>,
    /// Length of the closed boundary.
    pub perimeter: f64,
    /// Two edges that are not neighbours share a point, or neighbouring edges fold back along each other.
    pub self_intersecting: bool,
}

/// The area enclosed by a curve, closed by a straight line from its last point back to its first
/// (Gauss's shoelace formula). Returns `None` when fewer than three different points remain.
///
/// `signed_area = 1/2 * sum(x_i*y_{i+1} - x_{i+1}*y_i)` over the closed point list, and the
/// centroid is `sum((x_i + x_{i+1}) * c_i) / (6 * signed_area)` (and the same for y) with
/// `c_i = x_i*y_{i+1} - x_{i+1}*y_i`. The sums are taken relative to the first point to keep
/// precision when the curve is far from the origin.
pub fn polygon_area(x: &[f64], y: &[f64]) -> Option<PolygonArea> {
    let mut pts = distinct_points(x, y);
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    if pts.len() < 3 {
        return None;
    }
    let origin = pts[0];
    let q: Vec<(f64, f64)> = pts.iter().map(|&p| sub(p, origin)).collect();
    let n = q.len();
    let (mut sum, mut cx, mut cy, mut perimeter) = (0.0, 0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, r) = (q[i], q[(i + 1) % n]);
        let c = cross(p, r);
        sum += c;
        cx += (p.0 + r.0) * c;
        cy += (p.1 + r.1) * c;
        perimeter += (r.0 - p.0).hypot(r.1 - p.1);
    }
    let signed_area = sum / 2.0;
    let extent = q.iter().fold(0.0_f64, |m, p| m.max(p.0.abs()).max(p.1.abs()));
    let centroid = (signed_area.abs() > 1e-14 * extent * extent && signed_area != 0.0)
        .then(|| (origin.0 + cx / (6.0 * signed_area), origin.1 + cy / (6.0 * signed_area)));
    Some(PolygonArea {
        points: n,
        signed_area,
        area: signed_area.abs(),
        centroid,
        perimeter,
        self_intersecting: is_self_intersecting(&q),
    })
}

/// Whether two closed segments share a point.
fn segments_meet(p: (f64, f64), p2: (f64, f64), q: (f64, f64), q2: (f64, f64)) -> bool {
    let orient = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| cross(sub(b, a), sub(c, a));
    let on_segment = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
        c.0 >= a.0.min(b.0) && c.0 <= a.0.max(b.0) && c.1 >= a.1.min(b.1) && c.1 <= a.1.max(b.1)
    };
    let (d1, d2) = (orient(p, p2, q), orient(p, p2, q2));
    let (d3, d4) = (orient(q, q2, p), orient(q, q2, p2));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
        return true;
    }
    (d1 == 0.0 && on_segment(p, p2, q))
        || (d2 == 0.0 && on_segment(p, p2, q2))
        || (d3 == 0.0 && on_segment(q, q2, p))
        || (d4 == 0.0 && on_segment(q, q2, p2))
}

fn is_self_intersecting(q: &[(f64, f64)]) -> bool {
    let n = q.len();
    let seg = |i: usize| (q[i], q[(i + 1) % n]);
    // Neighbouring edges meet at their shared corner; they are a problem only if they fold back.
    for i in 0..n {
        let (a, b) = seg(i);
        let (_, c) = seg((i + 1) % n);
        let (u, v) = (sub(b, a), sub(c, b));
        if cross(u, v) == 0.0 && u.0 * v.0 + u.1 * v.1 < 0.0 {
            return true;
        }
    }
    // Sweep over the edges in order of their smallest X.
    let mut order: Vec<usize> = (0..n).collect();
    let min_x = |i: usize| seg(i).0.0.min(seg(i).1.0);
    let max_x = |i: usize| seg(i).0.0.max(seg(i).1.0);
    order.sort_by(|&a, &b| min_x(a).total_cmp(&min_x(b)));
    for (k, &i) in order.iter().enumerate() {
        let hi = max_x(i);
        for &j in &order[k + 1..] {
            if min_x(j) > hi {
                break;
            }
            let neighbours = (i + 1) % n == j || (j + 1) % n == i;
            if neighbours {
                continue;
            }
            let ((p, p2), (r, r2)) = (seg(i), seg(j));
            if segments_meet(p, p2, r, r2) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-12 * (1.0 + a.abs().max(b.abs()))
    }

    // ---- polygon area ----

    #[test]
    fn square_area_centroid_perimeter() {
        // Counterclockwise unit square scaled to 2 x 2, corner at (1, 1).
        let x = [1.0, 3.0, 3.0, 1.0];
        let y = [1.0, 1.0, 3.0, 3.0];
        let p = polygon_area(&x, &y).unwrap();
        assert_eq!(p.points, 4);
        assert!(close(p.signed_area, 4.0));
        assert!(close(p.area, 4.0));
        let (cx, cy) = p.centroid.unwrap();
        assert!(close(cx, 2.0) && close(cy, 2.0));
        assert!(close(p.perimeter, 8.0));
        assert!(!p.self_intersecting);
    }

    #[test]
    fn clockwise_is_negative_and_closing_point_is_ignored() {
        let x = [0.0, 0.0, 2.0, 2.0, 0.0];
        let y = [0.0, 2.0, 2.0, 0.0, 0.0];
        let p = polygon_area(&x, &y).unwrap();
        assert_eq!(p.points, 4);
        assert!(close(p.signed_area, -4.0));
        assert!(close(p.area, 4.0));
    }

    #[test]
    fn triangle_area_and_centroid() {
        // Vertices (0,0), (4,0), (0,3): area 6, centroid (4/3, 1), perimeter 12.
        let p = polygon_area(&[0.0, 4.0, 0.0], &[0.0, 0.0, 3.0]).unwrap();
        assert!(close(p.signed_area, 6.0));
        let (cx, cy) = p.centroid.unwrap();
        assert!(close(cx, 4.0 / 3.0) && close(cy, 1.0));
        assert!(close(p.perimeter, 12.0));
    }

    #[test]
    fn circle_approximation() {
        // A regular n-gon of radius r has area n/2 r^2 sin(2 pi / n); centred on (5, -2).
        let (n, r) = (360, 2.5);
        let x: Vec<f64> = (0..n).map(|k| 5.0 + r * (std::f64::consts::TAU * k as f64 / n as f64).cos()).collect();
        let y: Vec<f64> = (0..n).map(|k| -2.0 + r * (std::f64::consts::TAU * k as f64 / n as f64).sin()).collect();
        let p = polygon_area(&x, &y).unwrap();
        let exact = n as f64 / 2.0 * r * r * (std::f64::consts::TAU / n as f64).sin();
        assert!((p.area - exact).abs() < 1e-10, "{} {exact}", p.area);
        assert!((p.area - std::f64::consts::PI * r * r).abs() < 1e-3 * p.area);
        let (cx, cy) = p.centroid.unwrap();
        assert!((cx - 5.0).abs() < 1e-12 && (cy + 2.0).abs() < 1e-12);
        assert!(!p.self_intersecting);
    }

    #[test]
    fn figure_eight_cancels() {
        // Bow tie (0,0), (2,2), (2,0), (0,2): two triangles of area 1 run in opposite directions.
        let p = polygon_area(&[0.0, 2.0, 2.0, 0.0], &[0.0, 2.0, 0.0, 2.0]).unwrap();
        assert!(p.self_intersecting);
        assert!(close(p.signed_area, 0.0));
        assert!(p.centroid.is_none());
    }

    #[test]
    fn unequal_figure_eight() {
        // (0,0), (2,2), (2,0), (0,4): lobes 8/3 (counterclockwise) and 2/3 (clockwise).
        let p = polygon_area(&[0.0, 2.0, 2.0, 0.0], &[0.0, 2.0, 0.0, 4.0]).unwrap();
        assert!(p.self_intersecting);
        assert!(close(p.signed_area, 2.0));
        assert!(close(p.area, 2.0));
    }

    #[test]
    fn degenerate_inputs() {
        assert!(polygon_area(&[0.0, 1.0], &[0.0, 1.0]).is_none());
        assert!(polygon_area(&[0.0, 0.0, 0.0], &[1.0, 1.0, 1.0]).is_none());
        // Three points on a line: area zero, no centroid, folds back.
        let p = polygon_area(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0]).unwrap();
        assert!(close(p.area, 0.0) && p.centroid.is_none());
        // Missing values are dropped.
        let p = polygon_area(&[0.0, f64::NAN, 4.0, 0.0], &[0.0, 5.0, 0.0, 3.0]).unwrap();
        assert!(close(p.area, 6.0));
    }

    #[test]
    fn touching_corner_counts_as_self_intersecting() {
        // A square with a "pinched" vertex touching the opposite edge: (0,0),(4,0),(4,4),(2,0),(0,4)
        // has corner (2,0) on the edge (0,0)-(4,0).
        let p = polygon_area(&[0.0, 4.0, 4.0, 2.0, 0.0], &[0.0, 0.0, 4.0, 0.0, 4.0]).unwrap();
        assert!(p.self_intersecting);
    }

    // ---- intersections ----

    #[test]
    fn two_lines_cross_once() {
        // y = x and y = 4 - x meet at (2, 2).
        let r = intersections(&[0.0, 4.0], &[0.0, 4.0], &[0.0, 4.0], &[4.0, 0.0], false);
        assert_eq!(r.len(), 1);
        assert!(close(r[0].x, 2.0) && close(r[0].y, 2.0));
        assert_eq!(r[0].kind, MeetKind::Crossing);
        assert!(close(r[0].position_a, 0.5) && close(r[0].position_b, 0.5));
    }

    #[test]
    fn sine_and_line_cross_in_order() {
        // y = 1 on the polyline through (0,0), (1,2), (3,-2) crosses y = 1 at x = 0.5 and at
        // x = 1 + 2 * (1/4) = 1.5.
        let r = intersections(&[0.0, 1.0, 3.0], &[0.0, 2.0, -2.0], &[-1.0, 4.0], &[1.0, 1.0], false);
        assert_eq!(r.len(), 2);
        assert!(close(r[0].x, 0.5) && close(r[1].x, 1.5));
        assert!(r.iter().all(|h| close(h.y, 1.0)));
    }

    #[test]
    fn vertex_on_other_curve_counts_once() {
        // The V (0,1), (1,0), (2,1) and the line y = 0 meet at the vertex (1,0) and the V stays
        // above: touching. Hit by two segments, reported once.
        let v = ([0.0, 1.0, 2.0], [1.0, 0.0, 1.0]);
        let line = ([-1.0, 3.0], [0.0, 0.0]);
        assert!(intersections(&v.0, &v.1, &line.0, &line.1, false).is_empty());
        let r = intersections(&v.0, &v.1, &line.0, &line.1, true);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].kind, MeetKind::Touching);
        assert!(close(r[0].x, 1.0) && close(r[0].y, 0.0));
    }

    #[test]
    fn crossing_through_a_vertex_is_a_crossing() {
        // The zigzag (0,-1), (1,0), (2,1) is straight through (1,0) once the middle point is a
        // vertex, and the line y = 0 crosses there.
        let r = intersections(&[0.0, 1.0, 2.0], &[-1.0, 0.0, 1.0], &[-1.0, 3.0], &[0.0, 0.0], false);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].kind, MeetKind::Crossing);
        assert!(close(r[0].x, 1.0));
        // A bent path through the line at its vertex: (0,-1), (1,0), (2,-1) only touches.
        let r = intersections(&[0.0, 1.0, 2.0], &[-1.0, 0.0, -1.0], &[-1.0, 3.0], &[0.0, 0.0], false);
        assert!(r.is_empty());
    }

    #[test]
    fn tangent_parabola_touches() {
        // y = x^2 sampled at integers touches y = 0 at its vertex and is otherwise above it.
        let x: Vec<f64> = (-3..=3).map(f64::from).collect();
        let y: Vec<f64> = x.iter().map(|v| v * v).collect();
        assert!(intersections(&x, &y, &[-3.0, 3.0], &[0.0, 0.0], false).is_empty());
        let r = intersections(&x, &y, &[-3.0, 3.0], &[0.0, 0.0], true);
        assert_eq!(r.len(), 1);
        assert!(close(r[0].x, 0.0));
    }

    #[test]
    fn end_point_on_other_curve_is_touching() {
        // The segment from (0,0) to (1,1) ends on the line y = 1... at (1,1), which is its end.
        let r = intersections(&[0.0, 1.0], &[0.0, 1.0], &[-1.0, 3.0], &[1.0, 1.0], true);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].kind, MeetKind::Touching);
        assert!(intersections(&[0.0, 1.0], &[0.0, 1.0], &[-1.0, 3.0], &[1.0, 1.0], false).is_empty());
    }

    #[test]
    fn parallel_and_disjoint_have_none() {
        assert!(intersections(&[0.0, 4.0], &[0.0, 0.0], &[0.0, 4.0], &[1.0, 1.0], true).is_empty());
        // Collinear but not overlapping.
        assert!(intersections(&[0.0, 1.0], &[0.0, 0.0], &[2.0, 3.0], &[0.0, 0.0], true).is_empty());
        // Segments that would meet if extended.
        assert!(intersections(&[0.0, 1.0], &[0.0, 1.0], &[3.0, 5.0], &[0.0, -2.0], true).is_empty());
    }

    #[test]
    fn overlapping_stretch_reports_its_ends_as_touching() {
        // Both curves run along y = 0 on 1..=2 (A from 0 to 3, B from 1 to 2).
        let a = ([0.0, 3.0], [0.0, 0.0]);
        let b = ([1.0, 2.0], [0.0, 0.0]);
        assert!(intersections(&a.0, &a.1, &b.0, &b.1, false).is_empty());
        let r = intersections(&a.0, &a.1, &b.0, &b.1, true);
        assert_eq!(r.len(), 2);
        assert!(close(r[0].x, 1.0) && close(r[1].x, 2.0));
        assert!(r.iter().all(|h| h.kind == MeetKind::Touching));
    }

    #[test]
    fn loop_curves_cross_twice() {
        // Two unit-square-ish loops offset by half: the closed squares [0,2]^2 and [1,3]x[1,3]
        // outlines cross at (2,1) and (1,2).
        let a = ([0.0, 2.0, 2.0, 0.0, 0.0], [0.0, 0.0, 2.0, 2.0, 0.0]);
        let b = ([1.0, 3.0, 3.0, 1.0, 1.0], [1.0, 1.0, 3.0, 3.0, 1.0]);
        let r = intersections(&a.0, &a.1, &b.0, &b.1, false);
        assert_eq!(r.len(), 2);
        let mut at: Vec<(f64, f64)> = r.iter().map(|h| (h.x, h.y)).collect();
        at.sort_by(|p, q| p.0.total_cmp(&q.0));
        assert!(close(at[0].0, 1.0) && close(at[0].1, 2.0));
        assert!(close(at[1].0, 2.0) && close(at[1].1, 1.0));
    }

    // ---- average ----

    #[test]
    fn identical_x_averages_row_by_row_even_when_x_turns() {
        let x = [0.0, 1.0, 2.0, 1.0, 0.0];
        let a = [1.0, 2.0, 3.0, 2.0, 1.0];
        let b = [3.0, 4.0, 7.0, 4.0, 3.0];
        let r = average_curves(&[(&x, &a), (&x, &b)], &AverageOptions::default()).unwrap();
        assert_eq!(r.x, x);
        assert_eq!(r.mean, [2.0, 3.0, 5.0, 3.0, 2.0]);
        assert!(close(r.sd[2], (8.0_f64).sqrt())); // values 3 and 7: SD = 4/sqrt(2)
        assert_eq!(r.n, [2; 5]);
    }

    #[test]
    fn different_x_on_the_union_of_points() {
        // y = 2x on x = 0, 2, 4 and y = 2x + 2 on x = 1, 3, 5. Intersection range is 1..=4.
        let (xa, ya) = ([0.0, 2.0, 4.0], [0.0, 4.0, 8.0]);
        let (xb, yb) = ([1.0, 3.0, 5.0], [4.0, 8.0, 12.0]);
        let r = average_curves(&[(&xa, &ya), (&xb, &yb)], &AverageOptions::default()).unwrap();
        assert_eq!(r.x, [1.0, 2.0, 3.0, 4.0]);
        // Means of 2x and 2x + 2 are 2x + 1.
        for (x, m) in r.x.iter().zip(&r.mean) {
            assert!(close(*m, 2.0 * x + 1.0), "{x} {m}");
        }
        assert!(r.sd.iter().all(|s| close(*s, 2.0_f64.sqrt()))); // values differ by 2: SD = 2/sqrt(2)
        assert_eq!(r.n, [2; 4]);
    }

    #[test]
    fn union_range_uses_fewer_curves_at_the_ends() {
        let (xa, ya) = ([0.0, 2.0, 4.0], [0.0, 4.0, 8.0]);
        let (xb, yb) = ([1.0, 3.0, 5.0], [4.0, 8.0, 12.0]);
        let o = AverageOptions { range: XRange::Union, ..AverageOptions::default() };
        let r = average_curves(&[(&xa, &ya), (&xb, &yb)], &o).unwrap();
        assert_eq!(r.x, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(r.n, [1, 2, 2, 2, 2, 1]);
        assert!(close(r.mean[0], 0.0) && close(r.mean[5], 12.0));
        assert!(r.sd[0].is_nan() && r.sd[5].is_nan());
    }

    #[test]
    fn evenly_spaced_and_reference_grids() {
        let (xa, ya) = ([0.0, 4.0], [0.0, 8.0]);
        let (xb, yb) = ([0.0, 2.0, 4.0], [2.0, 2.0, 2.0]);
        let o = AverageOptions { x: CommonX::Evenly(5), ..AverageOptions::default() };
        let r = average_curves(&[(&xa, &ya), (&xb, &yb)], &o).unwrap();
        assert_eq!(r.x, [0.0, 1.0, 2.0, 3.0, 4.0]);
        assert_eq!(r.mean, [1.0, 2.0, 3.0, 4.0, 5.0]);
        let o = AverageOptions { x: CommonX::Reference(1), ..AverageOptions::default() };
        let r = average_curves(&[(&xa, &ya), (&xb, &yb)], &o).unwrap();
        assert_eq!(r.x, [0.0, 2.0, 4.0]);
        assert_eq!(r.mean, [1.0, 3.0, 5.0]);
        let o = AverageOptions { x: CommonX::Reference(2), ..AverageOptions::default() };
        assert_eq!(average_curves(&[(&xa, &ya), (&xb, &yb)], &o).unwrap_err(), CurveError::NoReference(2));
    }

    #[test]
    fn descending_x_is_reversed() {
        // A reverse sweep: X from 1 down to 0 on one curve, up on the other.
        let (xa, ya) = ([1.0, 0.5, 0.0], [10.0, 5.0, 0.0]);
        let (xb, yb) = ([0.0, 0.25, 0.5, 1.0], [2.0, 2.0, 2.0, 2.0]);
        let r = average_curves(&[(&xa, &ya), (&xb, &yb)], &AverageOptions::default()).unwrap();
        assert_eq!(r.x, [0.0, 0.25, 0.5, 1.0]);
        assert_eq!(r.mean, [1.0, 2.25, 3.5, 6.0]);
    }

    #[test]
    fn turning_curve_is_refused_unless_x_is_shared() {
        let (xa, ya) = ([0.0, 1.0, 0.0], [0.0, 1.0, 2.0]);
        let (xb, yb) = ([0.0, 0.5, 1.0], [0.0, 1.0, 2.0]);
        let e = average_curves(&[(&xa, &ya), (&xb, &yb)], &AverageOptions::default()).unwrap_err();
        assert_eq!(e, CurveError::NotMonotonic { curve: 0, row: 3 });
        assert!(e.to_string().contains("curve 1"));
    }

    #[test]
    fn repeated_x_merges_and_errors() {
        // Two Y values at x = 1 become their mean.
        let (xa, ya) = ([0.0, 1.0, 1.0, 2.0], [0.0, 1.0, 3.0, 4.0]);
        let (xb, yb) = ([0.0, 2.0], [0.0, 0.0]);
        let o = AverageOptions { x: CommonX::Evenly(3), ..AverageOptions::default() };
        let r = average_curves(&[(&xa, &ya), (&xb, &yb)], &o).unwrap();
        assert_eq!(r.mean, [0.0, 1.0, 2.0]);
        assert_eq!(average_curves(&[(&xa, &ya)], &o).unwrap_err(), CurveError::TooFewCurves);
        let (xc, yc) = ([5.0, 6.0], [1.0, 1.0]);
        assert_eq!(average_curves(&[(&xa, &ya), (&xc, &yc)], &o).unwrap_err(), CurveError::NoCommonRange);
        let single = ([1.0, 1.0], [1.0, 2.0]);
        assert_eq!(
            average_curves(&[(&xa, &ya), (&single.0, &single.1)], &o).unwrap_err(),
            CurveError::CurveTooShort(1)
        );
    }

    #[test]
    fn spline_method_reproduces_a_cubic_exactly_enough() {
        // A cubic spline (not-a-knot) is exact for a cubic: y = x^3 on two different grids.
        let xa: Vec<f64> = (0..=8).map(|k| f64::from(k) * 0.5).collect();
        let xb: Vec<f64> = (0..=6).map(|k| f64::from(k) * 0.7).collect();
        let ya: Vec<f64> = xa.iter().map(|x| x * x * x).collect();
        let yb: Vec<f64> = xb.iter().map(|x| x * x * x).collect();
        let o =
            AverageOptions { x: CommonX::Evenly(5), method: AverageMethod::CubicSpline, ..AverageOptions::default() };
        let r = average_curves(&[(&xa, &ya), (&xb, &yb)], &o).unwrap();
        for (x, m) in r.x.iter().zip(&r.mean) {
            assert!((m - x * x * x).abs() < 1e-9, "{x} {m}");
        }
    }

    #[test]
    fn akima_method_is_exact_on_a_straight_line() {
        // Akima reproduces a line exactly: two lines on different grids, mean on a third grid.
        let xa: Vec<f64> = (0..=8).map(|k| f64::from(k) * 0.5).collect();
        let xb: Vec<f64> = (0..=6).map(|k| f64::from(k) * 0.7).collect();
        let ya: Vec<f64> = xa.iter().map(|x| 2.0 * x + 1.0).collect();
        let yb: Vec<f64> = xb.iter().map(|x| 2.0 * x + 3.0).collect();
        let o = AverageOptions { x: CommonX::Evenly(5), method: AverageMethod::Akima, ..AverageOptions::default() };
        let r = average_curves(&[(&xa, &ya), (&xb, &yb)], &o).unwrap();
        assert_eq!(r.n, [2; 5]);
        for (x, m) in r.x.iter().zip(&r.mean) {
            assert!((m - (2.0 * x + 2.0)).abs() < 1e-9, "{x} {m}");
        }
        // It differs from the cubic spline on data that bends (so the method is really used).
        let yc: Vec<f64> = xa.iter().map(|x| if *x < 2.0 { 0.0 } else { x - 2.0 }).collect();
        let yd: Vec<f64> = xb.iter().map(|x| if *x < 2.0 { 0.0 } else { x - 2.0 }).collect();
        let s = AverageOptions { method: AverageMethod::CubicSpline, ..o };
        let akima = average_curves(&[(&xa, &yc), (&xb, &yd)], &o).unwrap();
        let spline = average_curves(&[(&xa, &yc), (&xb, &yd)], &s).unwrap();
        assert!(akima.mean.iter().zip(&spline.mean).any(|(p, q)| (p - q).abs() > 1e-6));
        // Akima keeps the flat part flat.
        assert!(close(akima.mean[0], 0.0) && close(akima.mean[1], 0.0));
    }

    #[test]
    fn spline_on_the_union_range_counts_the_ends() {
        // Cubics on [0, 4] and [1, 5]; a cubic spline (and Akima on a line) is exact inside each
        // curve, and the ends have one curve only.
        let xa: Vec<f64> = (0..=8).map(|k| f64::from(k) * 0.5).collect();
        let xb: Vec<f64> = (2..=10).map(|k| f64::from(k) * 0.5).collect();
        let ya: Vec<f64> = xa.iter().map(|x| x * x * x).collect();
        let yb: Vec<f64> = xb.iter().map(|x| x * x * x + 10.0).collect();
        let o = AverageOptions {
            x: CommonX::Evenly(6),
            range: XRange::Union,
            method: AverageMethod::CubicSpline,
        };
        let r = average_curves(&[(&xa, &ya), (&xb, &yb)], &o).unwrap();
        assert_eq!(r.x, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(r.n, [1, 2, 2, 2, 2, 1]);
        assert!(close(r.mean[0], 0.0) && (r.mean[5] - 135.0).abs() < 1e-9);
        assert!(r.sd[0].is_nan() && r.sd[5].is_nan());
        for k in 1..=4 {
            let x = r.x[k];
            assert!((r.mean[k] - (x * x * x + 5.0)).abs() < 1e-9, "{x} {}", r.mean[k]);
            assert!((r.sd[k] - 10.0 / 2.0_f64.sqrt()).abs() < 1e-9);
        }
        // Same counts for Akima.
        let o = AverageOptions { method: AverageMethod::Akima, ..o };
        assert_eq!(average_curves(&[(&xa, &ya), (&xb, &yb)], &o).unwrap().n, [1, 2, 2, 2, 2, 1]);
    }

    #[test]
    fn shared_turning_x_survives_a_missing_y_or_a_short_column() {
        // A cyclic sweep: X 0, 1, 2, 1, 0 shared by both curves.
        let x = [0.0, 1.0, 2.0, 1.0, 0.0];
        let a = [1.0, 2.0, 3.0, 2.0, 1.0];
        let b = [3.0, 4.0, 7.0, 4.0, f64::NAN];
        let r = average_curves(&[(&x, &a), (&x, &b)], &AverageOptions::default()).unwrap();
        assert_eq!(r.x, x);
        assert_eq!(r.n, [2, 2, 2, 2, 1]);
        assert_eq!(r.mean, [2.0, 3.0, 5.0, 3.0, 1.0]);
        assert!(r.sd[4].is_nan());
        // The second curve is one row shorter (its X column is cut to its own length).
        let r = average_curves(&[(&x, &a), (&x[..4], &b[..4])], &AverageOptions::default()).unwrap();
        assert_eq!(r.x, x);
        assert_eq!(r.n, [2, 2, 2, 2, 1]);
        // A missing Y in the middle of the first curve.
        let a2 = [1.0, f64::NAN, 3.0, 2.0, 1.0];
        let r = average_curves(&[(&x, &a2), (&x, &b)], &AverageOptions::default()).unwrap();
        assert_eq!(r.n, [2, 1, 2, 2, 1]);
        assert_eq!(r.mean[1], 4.0);
        // Different X values on a turning curve are still refused.
        let x2 = [0.0, 1.0, 2.0, 1.5, 0.0];
        assert!(matches!(
            average_curves(&[(&x, &a), (&x2, &b)], &AverageOptions::default()),
            Err(CurveError::NotMonotonic { .. })
        ));
    }

    #[test]
    fn a_stretch_over_several_segments_reports_only_its_two_ends() {
        let a = ([0.0, 1.0, 2.0, 3.0], [0.0; 4]);
        let b = ([0.5, 2.5], [0.0; 2]);
        let r = intersections(&a.0, &a.1, &b.0, &b.1, true);
        let xs: Vec<f64> = r.iter().map(|h| h.x).collect();
        assert_eq!(xs.len(), 2, "{xs:?}");
        assert!(close(xs[0], 0.5) && close(xs[1], 2.5));
        // Both curves with several shared vertices, the same way and reversed.
        let c = ([0.0, 1.0, 2.0, 3.0], [0.0; 4]);
        let r = intersections(&a.0, &a.1, &c.0, &c.1, true);
        assert_eq!(r.len(), 2);
        let rev = ([3.0, 2.0, 1.0, 0.0], [0.0; 4]);
        let r = intersections(&a.0, &a.1, &rev.0, &rev.1, true);
        assert_eq!(r.len(), 2);
        // A curve that turns back on the line has its turning point as an end of the stretch.
        let out_back = ([0.0, 2.0, 0.0], [0.0; 3]);
        let line = ([0.0, 1.0, 3.0], [0.0; 3]);
        let r = intersections(&out_back.0, &out_back.1, &line.0, &line.1, true);
        assert!(r.iter().any(|h| close(h.x, 2.0)), "{r:?}");
    }
}
