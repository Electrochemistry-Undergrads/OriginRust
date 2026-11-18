//! Rise time and fall time of a step in a range of rows (Gadgets > Rise Time, T138). Built from Origin's
//! documented behaviour, not measured: the amplitude runs from the smallest to the largest Y of the range,
//! the low and high levels are 10 % and 90 % of it (by default), and the time is the X distance between the
//! first crossing of the first level and the next crossing of the other one, crossings found by straight
//! interpolation between neighbouring points. A range whose last value is below its first is a fall.

/// The result over a range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiseTime {
    /// True for a rise (the range ends higher than it starts), false for a fall.
    pub rising: bool,
    /// The smallest and largest Y of the range.
    pub low: f64,
    pub high: f64,
    /// The two levels (low + 10 % and low + 90 % of the amplitude by default).
    pub level_lo: f64,
    pub level_hi: f64,
    /// Where the curve crosses the first level and then the other one.
    pub x_start: f64,
    pub x_end: f64,
    /// `x_end - x_start`.
    pub time: f64,
}

/// The X where the curve, from row `from` on, first crosses `level` in the direction `up`.
fn crossing(x: &[f64], y: &[f64], from: usize, to: usize, level: f64, up: bool) -> Option<(usize, f64)> {
    for i in from..to {
        let (y0, y1) = (y[i], y[i + 1]);
        if !(y0.is_finite() && y1.is_finite() && x[i].is_finite() && x[i + 1].is_finite()) {
            continue;
        }
        let hit = if up { y0 < level && y1 >= level } else { y0 > level && y1 <= level };
        if hit {
            let t = (level - y0) / (y1 - y0);
            return Some((i, x[i] + t * (x[i + 1] - x[i])));
        }
    }
    None
}

/// The rise (or fall) time over rows `first..=last` (0-based) with the levels at `lo_pct` and `hi_pct`
/// percent of the amplitude; `None` with fewer than two points, no amplitude or no crossing.
pub fn rise_time(x: &[f64], y: &[f64], (first, last): (usize, usize), lo_pct: f64, hi_pct: f64) -> Option<RiseTime> {
    let last = last.min(x.len().min(y.len()).checked_sub(1)?);
    if first >= last {
        return None;
    }
    let finite: Vec<f64> = y[first..=last].iter().copied().filter(|v| v.is_finite()).collect();
    let (low, high) = finite.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
    if high <= low {
        return None;
    }
    let rising = finite.last()? >= finite.first()?;
    let level_lo = low + lo_pct / 100.0 * (high - low);
    let level_hi = low + hi_pct / 100.0 * (high - low);
    let (start_level, end_level) = if rising { (level_lo, level_hi) } else { (level_hi, level_lo) };
    let (i, x_start) = crossing(x, y, first, last, start_level, rising)?;
    let (_, x_end) = crossing(x, y, i, last, end_level, rising)?;
    Some(RiseTime { rising, low, high, level_lo, level_hi, x_start, x_end, time: x_end - x_start })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ramp_rises_between_its_10_and_90_percent_points() {
        // 0 until x = 2, a ramp to 10 at x = 4, then flat.
        let x: Vec<f64> = (0..=60).map(|i| f64::from(i) * 0.1).collect();
        let y: Vec<f64> = x.iter().map(|&t| ((t - 2.0) * 5.0).clamp(0.0, 10.0)).collect();
        let r = rise_time(&x, &y, (0, 60), 10.0, 90.0).unwrap();
        assert!(r.rising);
        assert_eq!((r.low, r.high), (0.0, 10.0));
        assert!((r.x_start - 2.2).abs() < 1e-9 && (r.x_end - 3.8).abs() < 1e-9, "{r:?}");
        assert!((r.time - 1.6).abs() < 1e-9);
        // The mirror image falls in the same time.
        let down: Vec<f64> = y.iter().map(|v| 10.0 - v).collect();
        let f = rise_time(&x, &down, (0, 60), 10.0, 90.0).unwrap();
        assert!(!f.rising && (f.time - 1.6).abs() < 1e-9, "{f:?}");
        assert!(rise_time(&x, &vec![1.0; 61], (0, 60), 10.0, 90.0).is_none(), "no amplitude");
        assert!(rise_time(&x, &y, (5, 5), 10.0, 90.0).is_none());
    }
}
