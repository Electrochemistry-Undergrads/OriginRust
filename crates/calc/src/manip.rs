//! Data manipulation tools (Analysis > Data Manipulation) and the median filter (Analysis >
//! Mathematics): subtract the straight line through two points of a curve, reduce a curve to evenly
//! spaced X values, translate a curve, and a running median.
//!
//! Origin's behaviour for these was taken from its public documentation and is not measured; the
//! rules and the list of what an Origin run has to confirm are in `docs/spec/analysis-tools.md`
//! section 12. Missing values are NaN.

use serde::{Deserialize, Serialize};

use crate::baseline::subtract_line;
use crate::interp::{Extrapolation, InterpMethod, InterpOptions, SplineBoundary, interp1};

fn linear(extrapolation: Extrapolation) -> InterpOptions {
    InterpOptions { method: InterpMethod::Linear, boundary: SplineBoundary::NotAKnot, extrapolation, smoothing: 0.0 }
}

/// The value of the curve `(x, y)` at `at`, by straight lines between its points (points with a
/// missing X or Y are dropped, unsorted data are sorted by X). `None` when `at` is outside the X
/// range of the curve, or when the curve has fewer than two valid points.
pub fn value_at(x: &[f64], y: &[f64], at: f64) -> Option<f64> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for (a, b) in x.iter().zip(y) {
        if a.is_finite() && b.is_finite() {
            lo = lo.min(*a);
            hi = hi.max(*a);
        }
    }
    if !at.is_finite() || at < lo || at > hi {
        return None;
    }
    let v = interp1(x, y, &[at], &linear(Extrapolation::Missing))[0];
    v.is_finite().then_some(v)
}

/// One of the two points of [`subtract_line_through`]: an X value, and a Y value when the point is
/// not on the curve. Without a Y the curve's own value at `x` is used (linear interpolation).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LinePoint {
    pub x: f64,
    pub y: Option<f64>,
}

/// Subtracts the straight line through two points from the curve `(x, y)`. A point without a Y is
/// taken from the curve at its X. `None` when the two X values are equal, when the curve has fewer
/// than two valid points, or when a point without a Y lies outside the X range of the curve.
pub fn subtract_line_through(x: &[f64], y: &[f64], p1: LinePoint, p2: LinePoint) -> Option<Vec<f64>> {
    if !(p1.x.is_finite() && p2.x.is_finite()) || p1.x == p2.x {
        return None;
    }
    if x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).count() < 2 {
        return None;
    }
    let resolve = |p: LinePoint| -> Option<(f64, f64)> {
        let v = match p.y {
            Some(v) if v.is_finite() => v,
            Some(_) => return None,
            None => value_at(x, y, p.x)?,
        };
        Some((p.x, v))
    };
    Some(subtract_line(x, y, resolve(p1)?, resolve(p2)?))
}

/// How [`reduce_evenly`] gets the Y value at each new X.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ReduceMethod {
    /// Straight lines between the points of the curve (the default). Grid points outside the
    /// X range of the data are missing.
    #[default]
    Interpolate,
    /// The mean of the points that fall in the bin around each new X (one grid step wide, centred
    /// on the new X, the lower edge belongs to the bin). Empty bins are missing.
    Average,
}

/// The new X values of [`reduce_evenly`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EvenGrid {
    /// Spacing: X values `lo, lo + step, ...` up to and including `hi` (when `hi - lo` is a whole
    /// number of steps, within rounding).
    Step(f64),
    /// This many points, the first at `lo` and the last at `hi` (at least 2).
    Count(usize),
}

/// Options of [`reduce_evenly`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReduceOptions {
    pub grid: EvenGrid,
    pub method: ReduceMethod,
    /// X range of the new points; `None` is the X range of the data.
    pub range: Option<(f64, f64)>,
}

impl Default for ReduceOptions {
    fn default() -> Self {
        ReduceOptions { grid: EvenGrid::Count(100), method: ReduceMethod::Interpolate, range: None }
    }
}

/// The evenly spaced X values of `grid` over `lo..=hi` and their spacing. `None` for a spacing that
/// is not a positive number, a count below 2, an empty range or more than 100 million points.
pub fn even_grid(grid: EvenGrid, lo: f64, hi: f64) -> Option<(Vec<f64>, f64)> {
    if !(lo.is_finite() && hi.is_finite()) || hi <= lo {
        return None;
    }
    let (n, step) = match grid {
        EvenGrid::Step(s) => {
            if !(s.is_finite() && s > 0.0) {
                return None;
            }
            // The tolerance keeps `hi` when the range is a whole number of steps (0.1 steps).
            let n = ((hi - lo) / s * (1.0 + 1e-12) + 1e-9).floor() + 1.0;
            if n > 1.0e8 {
                return None;
            }
            (n as usize, s)
        }
        EvenGrid::Count(n) => {
            if !(2..=100_000_000).contains(&n) {
                return None;
            }
            (n, (hi - lo) / (n - 1) as f64)
        }
    };
    // Rounding must not carry the last value past `hi` (the interpolation would then call it
    // outside the data), so values are capped at `hi` and a count grid ends exactly on it.
    let mut g: Vec<f64> = match grid {
        EvenGrid::Step(_) => (0..n).map(|i| (lo + step * i as f64).min(hi)).collect(),
        EvenGrid::Count(_) => (0..n).map(|i| (lo + (hi - lo) * i as f64 / (n - 1) as f64).min(hi)).collect(),
    };
    if let (EvenGrid::Count(_), Some(last)) = (grid, g.last_mut()) {
        *last = hi;
    }
    Some((g, step))
}

/// Reduces a curve to evenly spaced X values: returns the new X values and Y values.
///
/// Points with a missing X or Y are ignored. `None` when the curve has fewer than two valid
/// points, its X values are all equal, or the grid is not valid (see [`even_grid`]).
pub fn reduce_evenly(x: &[f64], y: &[f64], opts: &ReduceOptions) -> Option<(Vec<f64>, Vec<f64>)> {
    let pts: Vec<(f64, f64)> =
        x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(&a, &b)| (a, b)).collect();
    if pts.len() < 2 {
        return None;
    }
    let dlo = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let dhi = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let (lo, hi) = match opts.range {
        Some((a, b)) => (a.min(b), a.max(b)),
        None => (dlo, dhi),
    };
    let (gx, step) = even_grid(opts.grid, lo, hi)?;
    let gy = match opts.method {
        ReduceMethod::Interpolate => interp1(x, y, &gx, &linear(Extrapolation::Missing)),
        ReduceMethod::Average => {
            let mut sum = vec![0.0; gx.len()];
            let mut count = vec![0usize; gx.len()];
            for &(px, py) in &pts {
                // The bin of grid point k is [lo + (k - 1/2) step, lo + (k + 1/2) step).
                let k = ((px - lo) / step + 0.5).floor();
                if k >= 0.0 && (k as usize) < gx.len() {
                    sum[k as usize] += py;
                    count[k as usize] += 1;
                }
            }
            sum.iter().zip(&count).map(|(s, &c)| if c == 0 { f64::NAN } else { s / c as f64 }).collect()
        }
    };
    Some((gx, gy))
}

/// Adds `by` to every value (missing values stay missing).
pub fn translate(v: &[f64], by: f64) -> Vec<f64> {
    v.iter().map(|a| a + by).collect()
}

/// The shift that carries the curve's value at `at` to `to`: `to - y(at)`. `None` when `at` is
/// outside the X range of the curve or when `to` is not a number. (For a translation in X the
/// shift is simply `to - at`.)
pub fn shift_to_land(x: &[f64], y: &[f64], at: f64, to: f64) -> Option<f64> {
    if !to.is_finite() {
        return None;
    }
    Some(to - value_at(x, y, at)?)
}

/// What a median filter does at the ends of the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MedianEdge {
    /// The window shrinks symmetrically so that it stays centred (the first and last point keep
    /// their value). Same rule as Origin's percentile filter with no boundary extension, which was
    /// measured.
    #[default]
    Shrink,
    /// The window is full everywhere; points beyond the end repeat the end value.
    Repeat,
    /// The window is full everywhere; points beyond the end mirror the data, including the end
    /// point (`y[-1] = y[0]`).
    Reflect,
}

/// The median of a window: the middle value of the sorted values, the mean of the two middle ones
/// for an even count.
fn median_of(mut w: Vec<f64>) -> f64 {
    w.sort_by(f64::total_cmp);
    let m = w.len();
    if m % 2 == 1 { w[m / 2] } else { (w[m / 2 - 1] + w[m / 2]) / 2.0 }
}

/// A running median over a window of `npts` points (centred on each point).
///
/// Missing values are removed, the rest is filtered as one series and the missing rows are put back
/// as missing (as Origin's smoothing does). The window needs an odd number of points, at least 1;
/// `None` otherwise.
pub fn median_filter(y: &[f64], npts: usize, edge: MedianEdge) -> Option<Vec<f64>> {
    if npts == 0 || npts.is_multiple_of(2) {
        return None;
    }
    let valid: Vec<f64> = y.iter().copied().filter(|v| v.is_finite()).collect();
    let n = valid.len();
    let h = npts / 2;
    let filtered: Vec<f64> = (0..n)
        .map(|i| {
            let window: Vec<f64> = match edge {
                MedianEdge::Shrink => {
                    let k = h.min(i).min(n - 1 - i);
                    valid[i - k..=i + k].to_vec()
                }
                MedianEdge::Repeat => (-(h as isize)..=h as isize)
                    .map(|j| valid[(i as isize + j).clamp(0, n as isize - 1) as usize])
                    .collect(),
                MedianEdge::Reflect => (-(h as isize)..=h as isize)
                    .map(|j| {
                        let r = (i as isize + j).rem_euclid(2 * n as isize);
                        valid[if r < n as isize { r } else { 2 * n as isize - 1 - r } as usize]
                    })
                    .collect(),
            };
            median_of(window)
        })
        .collect();
    let mut next = filtered.into_iter();
    Some(y.iter().map(|v| if v.is_finite() { next.next().unwrap_or(f64::NAN) } else { f64::NAN }).collect())
}

/// What Reduce Duplicate X Data puts in place of the Y values of points that share an X (Origin's
/// `reducedup` method).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DupMethod {
    #[default]
    Mean,
    Median,
    Min,
    Max,
    Sum,
    /// Standard deviation with the n - 1 divisor.
    Sd,
    /// Standard error of the mean, SD / sqrt(n).
    Se,
    /// Origin's "RMS" gives the mean of the group (measured), not the root mean square.
    Rms,
}

/// The result of [`reduce_duplicates`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reduced {
    /// One X per group, sorted.
    pub x: Vec<f64>,
    /// The group's value by the method; a point alone in its group keeps its own Y.
    pub y: Vec<f64>,
    /// The X of every group of two or more points, and how many points it has.
    pub dup_x: Vec<f64>,
    pub dup_count: Vec<f64>,
}

/// Origin's default tolerance of Reduce Duplicate X Data, relative to the X range (measured: X
/// values 1e-8 apart in a range of 1 are one group, 9e-8 apart are not).
pub const DUP_TOL: f64 = 1e-8;

/// Reduce Duplicate X Data, as Origin's `reducedup` was measured (analysis-tools.md section 12):
/// pairs with a missing value are left out, the points are sorted by X, and neighbours whose X
/// values differ by no more than `tol` times the X range of the data form one group (a chain: each
/// point is compared with the one before it, not with the first of the group). A group's X
/// is its lower middle X (the median of an odd group, the lower of the two middle values of an even
/// one); its Y is the method's value of the group's Y values. `None` when no pair is left.
pub fn reduce_duplicates(x: &[f64], y: &[f64], method: DupMethod, tol: f64) -> Option<Reduced> {
    let mut pts: Vec<(f64, f64)> =
        x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(&a, &b)| (a, b)).collect();
    if pts.is_empty() {
        return None;
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let range = pts[pts.len() - 1].0 - pts[0].0;
    let limit = tol.max(0.0) * range;
    let mut out = Reduced::default();
    let mut start = 0;
    while start < pts.len() {
        let mut end = start + 1;
        while end < pts.len() && pts[end].0 - pts[end - 1].0 <= limit {
            end += 1;
        }
        let group = &pts[start..end];
        let n = group.len();
        out.x.push(group[(n - 1) / 2].0);
        let ys: Vec<f64> = group.iter().map(|p| p.1).collect();
        out.y.push(if n == 1 { ys[0] } else { group_value(&ys, method) });
        if n > 1 {
            out.dup_x.push(group[(n - 1) / 2].0);
            out.dup_count.push(n as f64);
        }
        start = end;
    }
    Some(out)
}

fn group_value(ys: &[f64], method: DupMethod) -> f64 {
    let n = ys.len() as f64;
    let mean = ys.iter().sum::<f64>() / n;
    let sd = || (ys.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    match method {
        DupMethod::Mean | DupMethod::Rms => mean,
        DupMethod::Median => median_of(ys.to_vec()),
        DupMethod::Min => ys.iter().copied().fold(f64::INFINITY, f64::min),
        DupMethod::Max => ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        DupMethod::Sum => ys.iter().sum(),
        DupMethod::Sd => sd(),
        DupMethod::Se => sd() / n.sqrt(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
        for (p, q) in a.iter().zip(b) {
            assert!((p.is_nan() && q.is_nan()) || (p - q).abs() < 1e-12, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn value_at_interpolates_and_refuses_outside() {
        let (x, y) = ([0.0, 2.0, 4.0], [0.0, 4.0, 0.0]);
        assert_eq!(value_at(&x, &y, 1.0), Some(2.0));
        assert_eq!(value_at(&x, &y, 3.0), Some(2.0));
        assert_eq!(value_at(&x, &y, 4.0), Some(0.0));
        assert_eq!(value_at(&x, &y, 4.5), None);
        assert_eq!(value_at(&x, &y, -0.1), None);
    }

    #[test]
    fn straight_line_through_two_points_of_the_curve() {
        // y = 2x + 1 plus a bump of 3 at x = 2; the line is chosen at x = 0 and x = 4 where the
        // curve is exactly the straight line 1 and 9, so the corrected curve is the bump alone.
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y = [1.0, 3.0, 8.0, 7.0, 9.0];
        let r = subtract_line_through(&x, &y, LinePoint { x: 0.0, y: None }, LinePoint { x: 4.0, y: None }).unwrap();
        close(&r, &[0.0, 0.0, 3.0, 0.0, 0.0]);
        let r = subtract_line_through(&x, &y, LinePoint { x: 0.0, y: Some(1.0) }, LinePoint { x: 4.0, y: Some(9.0) })
            .unwrap();
        close(&r, &[0.0, 0.0, 3.0, 0.0, 0.0]);
    }

    #[test]
    fn straight_line_point_between_data_uses_the_straight_segment() {
        // Points at X = 0.5 and 3.5 are between samples: the curve value there is interpolated
        // (2 and 8), the line is y = 2 + 2 (x - 0.5) = 2x + 1, which is the data itself.
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y = [1.0, 3.0, 5.0, 7.0, 9.0];
        let r = subtract_line_through(&x, &y, LinePoint { x: 0.5, y: None }, LinePoint { x: 3.5, y: None }).unwrap();
        close(&r, &[0.0; 5]);
    }

    #[test]
    fn straight_line_refusals() {
        let (x, y) = ([0.0, 1.0, 2.0], [0.0, 1.0, 2.0]);
        let p = |x| LinePoint { x, y: None };
        assert!(subtract_line_through(&x, &y, p(1.0), p(1.0)).is_none());
        assert!(subtract_line_through(&x, &y, p(0.0), p(3.0)).is_none());
        assert!(subtract_line_through(&x, &y, p(0.0), LinePoint { x: 3.0, y: Some(3.0) }).is_some());
        // Fewer than two valid points: refused even when both Y values are given.
        let both = (LinePoint { x: 0.0, y: Some(0.0) }, LinePoint { x: 1.0, y: Some(1.0) });
        assert!(subtract_line_through(&x, &y, both.0, both.1).is_some());
        assert!(subtract_line_through(&[0.0], &[1.0], both.0, both.1).is_none());
        assert!(subtract_line_through(&[0.0, 1.0], &[1.0, f64::NAN], both.0, both.1).is_none());
        assert!(subtract_line_through(&[], &[], both.0, both.1).is_none());
    }

    #[test]
    fn the_last_grid_point_never_passes_the_data_maximum() {
        // 0.1 * 3 and 10 / 147 * 147 are one ulp above the end in f64.
        let x: Vec<f64> = vec![0.0, 0.1, 0.2, 0.3];
        let y = vec![1.0, 2.0, 3.0, 4.0];
        let opts = ReduceOptions { grid: EvenGrid::Step(0.1), ..ReduceOptions::default() };
        let (gx, gy) = reduce_evenly(&x, &y, &opts).unwrap();
        assert_eq!(gx.len(), 4);
        assert_eq!(gx[3], 0.3);
        assert!(gy.iter().all(|v| v.is_finite()), "{gy:?}");
        close(&gy, &[1.0, 2.0, 3.0, 4.0]);
        let x = [0.0, 10.0];
        let y = [0.0, 20.0];
        let opts = ReduceOptions { grid: EvenGrid::Count(148), ..ReduceOptions::default() };
        let (gx, gy) = reduce_evenly(&x, &y, &opts).unwrap();
        assert_eq!((gx.len(), gx[147]), (148, 10.0));
        assert!(gy.iter().all(|v| v.is_finite()), "{gy:?}");
        assert_eq!(gy[147], 20.0);
        for n in [12, 23, 40, 45, 79, 84] {
            let (g, _) = even_grid(EvenGrid::Count(n), 0.0, 100.0).unwrap();
            assert_eq!(*g.last().unwrap(), 100.0);
            assert!(g.windows(2).all(|w| w[0] < w[1]));
        }
        for (hi, s) in [(0.7, 0.1), (0.6, 0.2), (0.3, 0.1)] {
            let (g, _) = even_grid(EvenGrid::Step(s), 0.0, hi).unwrap();
            assert_eq!(*g.last().unwrap(), hi);
        }
    }

    #[test]
    fn grid_by_step_and_count() {
        let (g, s) = even_grid(EvenGrid::Step(0.1), 0.0, 0.5).unwrap();
        assert_eq!(g.len(), 6);
        assert!((g[5] - 0.5).abs() < 1e-12 && (s - 0.1).abs() < 1e-15);
        let (g, _) = even_grid(EvenGrid::Step(0.4), 0.0, 1.0).unwrap();
        close(&g, &[0.0, 0.4, 0.8]);
        let (g, s) = even_grid(EvenGrid::Count(5), 1.0, 2.0).unwrap();
        close(&g, &[1.0, 1.25, 1.5, 1.75, 2.0]);
        assert_eq!(s, 0.25);
        assert!(even_grid(EvenGrid::Count(1), 1.0, 2.0).is_none());
        assert!(even_grid(EvenGrid::Step(0.0), 1.0, 2.0).is_none());
        assert!(even_grid(EvenGrid::Step(1.0), 2.0, 2.0).is_none());
    }

    #[test]
    fn reduce_by_interpolation() {
        // Uneven X on the line y = 10 x; the new grid 0, 0.5, ... 2 reads it exactly.
        let x = [0.0, 0.3, 1.1, 1.2, 2.0];
        let y: Vec<f64> = x.iter().map(|v| 10.0 * v).collect();
        let opts = ReduceOptions { grid: EvenGrid::Step(0.5), ..ReduceOptions::default() };
        let (gx, gy) = reduce_evenly(&x, &y, &opts).unwrap();
        close(&gx, &[0.0, 0.5, 1.0, 1.5, 2.0]);
        close(&gy, &[0.0, 5.0, 10.0, 15.0, 20.0]);
        // Descending X is sorted first; a range wider than the data gives missing values.
        let xr: Vec<f64> = x.iter().rev().copied().collect();
        let yr: Vec<f64> = y.iter().rev().copied().collect();
        let opts = ReduceOptions { grid: EvenGrid::Step(1.0), range: Some((0.0, 3.0)), ..ReduceOptions::default() };
        let (gx, gy) = reduce_evenly(&xr, &yr, &opts).unwrap();
        close(&gx, &[0.0, 1.0, 2.0, 3.0]);
        close(&gy, &[0.0, 10.0, 20.0, f64::NAN]);
    }

    #[test]
    fn reduce_by_averaging() {
        // Grid 0, 1, 2, 3 (bins [-0.5, 0.5), [0.5, 1.5), [1.5, 2.5), [2.5, 3.5)): bin 0 holds 0.0
        // and 0.4 (mean of 2 and 4), bin 1 holds 0.5 and 1.4 (mean of 6 and 10), bin 2 holds
        // nothing, bin 3 holds 3.0 (value 7).
        let x = [0.0, 0.4, 0.5, 1.4, 3.0];
        let y = [2.0, 4.0, 6.0, 10.0, 7.0];
        let opts = ReduceOptions { grid: EvenGrid::Step(1.0), method: ReduceMethod::Average, range: Some((0.0, 3.0)) };
        let (gx, gy) = reduce_evenly(&x, &y, &opts).unwrap();
        close(&gx, &[0.0, 1.0, 2.0, 3.0]);
        close(&gy, &[3.0, 8.0, f64::NAN, 7.0]);
    }

    #[test]
    fn reduce_refuses_bad_input() {
        let o = ReduceOptions::default();
        assert!(reduce_evenly(&[1.0], &[1.0], &o).is_none());
        assert!(reduce_evenly(&[1.0, 1.0], &[1.0, 2.0], &o).is_none());
        assert!(reduce_evenly(&[1.0, f64::NAN], &[1.0, 2.0], &o).is_none());
    }

    #[test]
    fn translation() {
        close(&translate(&[1.0, f64::NAN, 3.0], 0.5), &[1.5, f64::NAN, 3.5]);
        // The curve is 4 at x = 2; to bring it to 10 it moves up by 6.
        let (x, y) = ([0.0, 2.0, 4.0], [0.0, 4.0, 0.0]);
        assert_eq!(shift_to_land(&x, &y, 2.0, 10.0), Some(6.0));
        assert_eq!(shift_to_land(&x, &y, 1.0, 0.0), Some(-2.0));
        assert_eq!(shift_to_land(&x, &y, 9.0, 0.0), None);
    }

    #[test]
    fn median_shrinks_at_the_ends() {
        // Window 3 on 1 9 2 8 3 7: ends stay (window of 1), then median(1,9,2)=2, (9,2,8)=8,
        // (2,8,3)=3, (8,3,7)=7.
        let y = [1.0, 9.0, 2.0, 8.0, 3.0, 7.0];
        close(&median_filter(&y, 3, MedianEdge::Shrink).unwrap(), &[1.0, 2.0, 8.0, 3.0, 7.0, 7.0]);
        // Window 5: i=1 uses 3 points (1,9,2) -> 2; i=2 uses 5 (1,9,2,8,3) -> 3;
        // i=3 uses 5 (9,2,8,3,7) -> 7; i=4 uses 3 (8,3,7) -> 7.
        close(&median_filter(&y, 5, MedianEdge::Shrink).unwrap(), &[1.0, 2.0, 3.0, 7.0, 7.0, 7.0]);
    }

    #[test]
    fn median_with_padding_and_spikes() {
        let y = [1.0, 1.0, 50.0, 1.0, 1.0];
        close(&median_filter(&y, 3, MedianEdge::Repeat).unwrap(), &[1.0; 5]);
        // 5 1 2 3 4, window 3, end value repeated: (5,5,1)=5, (5,1,2)=2, (1,2,3)=2, (2,3,4)=3, (3,4,4)=4.
        let r = [5.0, 1.0, 2.0, 3.0, 4.0];
        close(&median_filter(&r, 3, MedianEdge::Repeat).unwrap(), &[5.0, 2.0, 2.0, 3.0, 4.0]);
        // Reflect, window 5: i=0 window (1,5,5,1,2) -> 2; i=4 window (2,3,4,4,3) -> 3.
        let m = median_filter(&r, 5, MedianEdge::Reflect).unwrap();
        close(&m[..1], &[2.0]);
        close(&m[4..], &[3.0]);
    }

    #[test]
    fn median_missing_values_and_window_rules() {
        // The missing row is removed, the rest filtered as one series (1 9 2 -> 1 2 2), put back.
        let y = [1.0, f64::NAN, 9.0, 2.0];
        close(&median_filter(&y, 3, MedianEdge::Shrink).unwrap(), &[1.0, f64::NAN, 2.0, 2.0]);
        assert!(median_filter(&y, 4, MedianEdge::Shrink).is_none());
        assert!(median_filter(&y, 0, MedianEdge::Shrink).is_none());
        close(&median_filter(&[3.0, 1.0, 2.0], 1, MedianEdge::Shrink).unwrap(), &[3.0, 1.0, 2.0]);
        close(&median_filter(&[3.0, 1.0, 2.0], 9, MedianEdge::Shrink).unwrap(), &[3.0, 2.0, 2.0]);
        assert!(median_filter(&[], 3, MedianEdge::Shrink).unwrap().is_empty());
    }

    #[test]
    fn median_of_an_even_count_is_the_mean_of_the_middle_two() {
        assert_eq!(median_of(vec![4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    /// Numbers measured with Origin's `reducedup` (`re-workspace/scratch/reducedup/reducedup*.out.txt`).
    #[test]
    #[allow(clippy::excessive_precision)] // the digits as Origin printed them
    fn reduce_duplicates_matches_origin() {
        let x = [3.0, 1.0, 2.0, 3.0, 2.0, 3.0, 5.0, 2.000000001, 7.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let run = |m| reduce_duplicates(&x, &y, m, 1e-8).unwrap();
        let r = run(DupMethod::Mean);
        assert_eq!(r.x, [1.0, 2.0, 3.0, 5.0, 7.0]);
        let close = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(p, q)| (p - q).abs() < 1e-12);
        assert!(close(&r.y, &[2.0, 5.333333333333333, 3.6666666666666665, 7.0, 9.0]), "{:?}", r.y);
        assert_eq!((r.dup_x.clone(), r.dup_count.clone()), (vec![2.0, 3.0], vec![3.0, 3.0]));
        for (m, two, three) in [
            (DupMethod::Median, 5.0, 4.0),
            (DupMethod::Min, 3.0, 1.0),
            (DupMethod::Max, 8.0, 6.0),
            (DupMethod::Sum, 16.0, 11.0),
            (DupMethod::Sd, 2.5166114784235836, 2.5166114784235831),
            (DupMethod::Se, 1.4529663145135581, 1.4529663145135578),
            (DupMethod::Rms, 5.333333333333333, 3.6666666666666665),
        ] {
            let r = run(m);
            assert!(close(&r.y, &[2.0, two, three, 7.0, 9.0]), "{m:?}: {:?}", r.y);
        }
        // The tolerance is relative to the X range, and a group's X is its lower middle X.
        let r = reduce_duplicates(&[100.0, 102.0, 1000.0], &[1.0, 2.0, 3.0], DupMethod::Mean, 0.01).unwrap();
        assert_eq!((r.x, r.y), (vec![100.0, 1000.0], vec![1.5, 3.0]));
        let r = reduce_duplicates(&[10.0, 10.2, 10.3, 50.0], &[1.0, 2.0, 3.0, 4.0], DupMethod::Mean, 0.01).unwrap();
        assert_eq!((r.x, r.y), (vec![10.2, 50.0], vec![2.0, 4.0]));
        let r = reduce_duplicates(&x, &y, DupMethod::Mean, 0.5).unwrap();
        assert_eq!((r.x, r.y), (vec![3.0], vec![5.0]));
        // reducedup3: the default tolerance, chained groups, pairs with a missing value left out.
        let r = reduce_duplicates(&[0.0, 1e-8, 1e-7, 1.0], &[1.0, 2.0, 3.0, 4.0], DupMethod::Mean, DUP_TOL).unwrap();
        assert_eq!((r.x, r.y), (vec![0.0, 1e-7, 1.0], vec![1.5, 3.0, 4.0]));
        let r = reduce_duplicates(&[0.0, 1e-10, 1e-9, 1.0], &[1.0, 2.0, 3.0, 4.0], DupMethod::Mean, DUP_TOL).unwrap();
        assert_eq!((r.x, r.y), (vec![1e-10, 1.0], vec![2.0, 4.0]));
        let r = reduce_duplicates(&[0.0, 0.3, 0.6, 0.9, 10.0], &[1.0, 2.0, 3.0, 4.0, 5.0], DupMethod::Mean, 0.035);
        assert_eq!(r.map(|r| (r.x, r.y)), Some((vec![0.3, 10.0], vec![2.5, 5.0])));
        let nan = f64::NAN;
        let r = reduce_duplicates(&[1.0, 2.0, nan, 3.0, 10.0], &[1.0, nan, 3.0, 4.0, 5.0], DupMethod::Mean, DUP_TOL);
        assert_eq!(r.map(|r| (r.x, r.y)), Some((vec![1.0, 3.0, 10.0], vec![1.0, 4.0, 5.0])));
        assert!(reduce_duplicates(&[nan], &[1.0], DupMethod::Mean, DUP_TOL).is_none());
    }
}
