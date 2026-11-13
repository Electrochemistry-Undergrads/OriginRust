//! The limits and tick step of an axis that follows its data, as Origin picks them.
//!
//! All rules here were measured in Origin (graphing spec section 4). The plot crate (drawing) and the
//! `.opj` writer (the range Origin shows on open) both call these functions, so the two agree.
//!
//! Vocabulary: the data run from `lo` to `hi`, the span is `hi - lo`, the margin is the Rescale Margin
//! in percent (Origin's default is 8), and the minor spacing `h` is the distance between a major tick
//! and the next minor tick.

/// Origin's default Rescale Margin, in percent of the data span.
pub const DEFAULT_RESCALE_MARGIN_PCT: f64 = 8.0;

/// The span is scaled by this before the step table is read: the table was measured with the default
/// 8 % margin on both sides (1 + 2 * 0.08). It does not follow the margin; Origin's step for data 0..10
/// stays 5 at margins of 2, 4, 8, 12, 16 and 20 %.
const STEP_SPAN_FACTOR: f64 = 1.16;

fn pow10(e: i32) -> f64 {
    format!("1e{e}").parse().unwrap_or_else(|_| 10f64.powi(e))
}

/// Rounds to 12 significant digits to remove floating-point noise.
fn tidy(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    format!("{v:.11e}").parse().unwrap_or(v)
}

/// `(b, x)` with `b` a power of ten and `x = e / b` in `[1.05, 10.5)`.
fn normalise(e: f64) -> (f64, f64) {
    let mut k = (e / 1.05).log10().floor() as i32;
    let mut b = pow10(k);
    // Guard against log10 rounding at the edges of the interval.
    while e / b >= 10.5 {
        k += 1;
        b = pow10(k);
    }
    while e / b < 1.05 {
        k -= 1;
        b = pow10(k);
    }
    (b, e / b)
}

/// The major tick step of a linear axis for data that span `span`.
///
/// It is 1, 2 or 5 times a power of ten, read from the span scaled by 1.16: below 2.25 (in units of the
/// decade, which starts at 1.05) the step is half the decade, up to 4.5 it is the decade, up to 10.5 it is
/// twice the decade. Origin also takes a step of 2 for scaled spans from 12 to 40 (data spans 10.35 to 34.48)
/// in that one decade only; it is measured, not understood (graphing spec section 4).
pub fn linear_step(span: f64) -> f64 {
    step_for_scaled(span * STEP_SPAN_FACTOR)
}

/// [`linear_step`] for a span already scaled for the margins that are applied.
fn step_for_scaled(e: f64) -> f64 {
    if !(e.is_finite() && e > 0.0) {
        return 1.0;
    }
    if (12.0..40.0).contains(&e) {
        return 2.0;
    }
    let (b, x) = normalise(e);
    tidy(if x < 2.25 {
        0.5 * b
    } else if x < 4.5 {
        b
    } else {
        2.0 * b
    })
}

/// `floor(q)` that ignores float noise just below a whole number.
fn floor_tol(q: f64) -> f64 {
    (q + 1e-9).floor()
}

/// `ceil(q)` that ignores float noise just above a whole number.
fn ceil_tol(q: f64) -> f64 {
    (q - 1e-9).ceil()
}

/// Limits and major step of a linear axis for data `lo..hi`: `(from, to, step)`.
///
/// `from` and `to` lie on multiples of half the step, outside the data by at least `margin_pct` percent
/// of the span; with a margin of 0 they are the data limits. `lo..hi` must be finite with `hi > lo`.
pub fn linear_range(lo: f64, hi: f64, margin_pct: f64) -> (f64, f64, f64) {
    linear_range_edges(lo, hi, margin_pct, true, true)
}

/// [`linear_range`] for an axis with a bound end: the base of column plots, which Origin does not
/// pad (data 1..4 as columns give 0..4.5 with step 1). `free_low` and `free_high` say which ends get a
/// margin; a bound end is the data limit itself, and the step table is read with a span scaled for
/// one margin (1.08) instead of two. `lo..hi` includes the base.
pub fn linear_range_edges(lo: f64, hi: f64, margin_pct: f64, free_low: bool, free_high: bool) -> (f64, f64, f64) {
    let span = hi - lo;
    let sides = f64::from(u8::from(free_low) + u8::from(free_high));
    let step = step_for_scaled(span * (1.0 + sides * 0.08));
    let m = margin_pct.max(0.0) / 100.0 * span;
    let h = step / 2.0;
    let from = if free_low && m > 0.0 { tidy(floor_tol((lo - m) / h) * h) } else { lo };
    let to = if free_high && m > 0.0 { tidy(ceil_tol((hi + m) / h) * h) } else { hi };
    (from, to, step)
}

/// How the ticks of an automatic log axis are laid out.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LogTicks {
    /// Major ticks at powers of ten, minor ticks at 2 to 9 times a power of ten.
    Decades,
    /// Less than about one decade of data: major ticks at multiples of this step, as on a linear axis.
    Linear(f64),
}

/// Limits and tick layout of a log axis for data `lo..hi` (both positive).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LogRange {
    pub from: f64,
    pub to: f64,
    pub ticks: LogTicks,
}

/// Data spans of up to this many decades (in log10) get linear ticks (measured: 0.903 does, 0.929 does not).
const LOG_LINEAR_TICKS_BELOW: f64 = 0.92;
/// More than this many decades of data are rounded out to whole decades; up to it, to the 1..9 ticks.
const LOG_WHOLE_DECADES_ABOVE: f64 = 5.0;

/// The tick step Origin uses on a log axis with less than a decade of data, for the padded range `e`
/// (`hi' - lo'`, see [`log_range`]). Up to 1.45 it follows the linear table; to 20.4 it is 1; beyond
/// that it is half, one or two times the decade of `e` (changing at 4.6 and 9.3).
pub fn log_linear_step(e: f64) -> f64 {
    if !(e.is_finite() && e > 0.0) {
        return 1.0;
    }
    if e < 1.45 {
        let (b, x) = normalise(e);
        return tidy(if x < 2.25 {
            0.5 * b
        } else if x < 4.5 {
            b
        } else {
            2.0 * b
        });
    }
    if e < 20.4 {
        return 1.0;
    }
    let b = pow10(e.log10().floor() as i32);
    let x = e / b;
    tidy(if x < 4.6 {
        0.5 * b
    } else if x < 9.3 {
        b
    } else {
        2.0 * b
    })
}

/// The next whole multiple `k * 10^d` (k = 1..9) at or below `v`.
fn tick_floor(v: f64) -> f64 {
    let d = v.log10().floor() as i32;
    let mant = v / pow10(d);
    let k = floor_tol(mant).clamp(1.0, 9.0);
    tidy(k * pow10(d))
}

/// The next `k * 10^d` (k = 1..9, or the next power of ten) at or above `v`.
fn tick_ceil(v: f64) -> f64 {
    let d = v.log10().floor() as i32;
    let mant = v / pow10(d);
    let k = ceil_tol(mant).max(1.0);
    tidy(k * pow10(d))
}

/// Limits and ticks of a log axis for data `lo..hi` with `0 < lo < hi`.
///
/// The margin is a fraction of the span in log10 units on each side: `lo' = lo * 10^(-m s)`,
/// `hi' = hi * 10^(m s)` with `s = log10(hi / lo)`. Then, by `s`:
/// - more than 5 decades: whole decades outside `lo'..hi'`;
/// - from 0.92 to 5: the next tick of the form `k * 10^d` outside them;
/// - below 0.92: multiples of one ninth of the linear step ([`log_linear_step`]) outside them, with linear ticks.
pub fn log_range(lo: f64, hi: f64, margin_pct: f64) -> LogRange {
    let s = (hi / lo).log10();
    if !(s.is_finite() && s > 0.0) {
        let from = pow10(lo.log10().floor() as i32);
        return LogRange { from, to: from * 10.0, ticks: LogTicks::Decades };
    }
    let m = margin_pct.max(0.0) / 100.0 * s;
    let (lo2, hi2) = (lo * 10f64.powf(-m), hi * 10f64.powf(m));
    if m <= 0.0 {
        let ticks =
            if s < LOG_LINEAR_TICKS_BELOW { LogTicks::Linear(log_linear_step(hi2 - lo2)) } else { LogTicks::Decades };
        return LogRange { from: lo, to: hi, ticks };
    }
    if s < LOG_LINEAR_TICKS_BELOW {
        let inc = log_linear_step(hi2 - lo2);
        let g = inc / 9.0;
        let from = tidy(floor_tol(lo2 / g) * g);
        let to = tidy(ceil_tol(hi2 / g) * g);
        return LogRange { from, to, ticks: LogTicks::Linear(inc) };
    }
    if s > LOG_WHOLE_DECADES_ABOVE {
        let from = pow10((lo2.log10() + 1e-9).floor() as i32);
        let to = pow10((hi2.log10() - 1e-9).ceil() as i32);
        return LogRange { from, to, ticks: LogTicks::Decades };
    }
    LogRange { from: tick_floor(lo2), to: tick_ceil(hi2), ticks: LogTicks::Decades }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_follow_the_scaled_span() {
        assert_eq!(linear_step(10.0), 5.0);
        assert_eq!(linear_step(0.9), 0.2);
        assert_eq!(linear_step(0.91), 0.5);
        assert_eq!(linear_step(3.85), 1.0);
        assert_eq!(linear_step(3.9), 2.0);
        assert_eq!(linear_step(20.0), 2.0);
        assert_eq!(linear_step(65.0), 20.0);
        assert_eq!(linear_step(100.0), 50.0);
        assert_eq!(linear_step(25000.0), 10000.0);
        assert_eq!(linear_step(0.0), 1.0);
    }

    #[test]
    fn linear_limits_are_on_half_steps() {
        assert_eq!(linear_range(-0.2, 1.0, 8.0), (-0.5, 1.25, 0.5));
        assert_eq!(linear_range(0.0, 10.0, 8.0), (-2.5, 12.5, 5.0));
        assert_eq!(linear_range(12.0, 77.0, 8.0), (0.0, 90.0, 20.0));
        assert_eq!(linear_range(0.0, 10.0, 0.0), (0.0, 10.0, 5.0));
    }

    #[test]
    fn a_bound_end_gets_no_margin() {
        // Columns of 1..4 (base 0): Origin shows 0..4.5, step 1; of 1..4.5: 0..5, step 2.
        assert_eq!(linear_range_edges(0.0, 4.0, 8.0, false, true), (0.0, 4.5, 1.0));
        assert_eq!(linear_range_edges(0.0, 4.5, 8.0, false, true), (0.0, 5.0, 2.0));
        assert_eq!(linear_range_edges(0.0, 10.0, 8.0, false, true), (0.0, 12.5, 5.0));
        assert_eq!(linear_range_edges(-4.0, 0.0, 8.0, true, false), (-4.5, 0.0, 1.0));
    }

    #[test]
    fn log_limits_follow_origin() {
        let r = log_range(1.0, 1000.0, 8.0);
        assert_eq!((r.from, r.to, r.ticks), (0.5, 2000.0, LogTicks::Decades));
        let r = log_range(2.06, 8.0, 8.0);
        assert!((r.from - 16.0 / 9.0).abs() < 1e-10 && (r.to - 9.0).abs() < 1e-10);
        assert_eq!(r.ticks, LogTicks::Linear(1.0));
        let r = log_range(1.0, 1e6, 8.0);
        assert_eq!((r.from, r.to), (0.1, 1e7));
    }
}
