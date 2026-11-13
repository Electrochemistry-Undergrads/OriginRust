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
/// taken from the curve at its X. `None` when the two X values are equal, or when a point without a
/// Y lies outside the X range of the curve.
pub fn subtract_line_through(x: &[f64], y: &[f64], p1: LinePoint, p2: LinePoint) -> Option<Vec<f64>> {
    if !(p1.x.is_finite() && p2.x.is_finite()) || p1.x == p2.x {
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
    Some(((0..n).map(|i| lo + step * i as f64).collect(), step))
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
/// outside the X range of the curve. (For a translation in X the shift is simply `to - at`.)
pub fn shift_to_land(x: &[f64], y: &[f64], at: f64, to: f64) -> Option<f64> {
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
}
