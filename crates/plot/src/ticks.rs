//! Axis resolution: turns the axis settings and the data range into limits,
//! tick positions and tick label text (spec section 2).

use sheaf_core::autorange::{self, LogTicks};
use sheaf_core::datetime::{self, DatePattern, TimePattern};
use sheaf_core::graph::{AutoRange, Axis, ScaleKind, TickFormat};

/// The multiplication sign used in scientific tick labels.
const TIMES: char = '\u{d7}';
/// The real minus sign (U+2212) used for negative numbers.
const MINUS: char = '\u{2212}';

/// An axis with everything decided: limits, ticks and label text.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedAxis {
    /// Lower limit (data units).
    pub min: f64,
    /// Upper limit (data units).
    pub max: f64,
    /// Distance between major ticks (linear), or `10^k` for a log axis that labels every k-th decade.
    pub step: f64,
    /// Linear or log10.
    pub scale: ScaleKind,
    /// Major tick values inside the limits.
    pub major: Vec<f64>,
    /// Minor tick values inside the limits (never equal to a major tick).
    pub minor: Vec<f64>,
    /// One label per major tick, in the text markup of the `markup` module
    /// (superscripts are written `\+(..)`).
    pub labels: Vec<String>,
    /// True when the user fixed the lower limit above the upper one: `min` and
    /// `max` are still the smaller and larger value, but the axis runs from
    /// `max` to `min` (values decrease along the axis).
    pub reversed: bool,
}

impl ResolvedAxis {
    /// Position of `v` along the axis: 0 at `min`, 1 at `max`. Linear or
    /// logarithmic according to the scale. Values that cannot be placed
    /// (non-positive on a log axis) give NaN.
    pub fn to_unit(&self, v: f64) -> f64 {
        let u = self.to_unit_ascending(v);
        if self.reversed { 1.0 - u } else { u }
    }

    fn to_unit_ascending(&self, v: f64) -> f64 {
        match self.scale {
            ScaleKind::Linear => (v - self.min) / (self.max - self.min),
            ScaleKind::Log10 => {
                if v > 0.0 {
                    (v.log10() - self.min.log10()) / (self.max.log10() - self.min.log10())
                } else {
                    f64::NAN
                }
            }
            other => {
                let (a, b) = (other.forward(self.min), other.forward(self.max));
                (other.forward(v) - a) / (b - a)
            }
        }
    }

    /// Inverse of [`ResolvedAxis::to_unit`].
    pub fn from_unit(&self, u: f64) -> f64 {
        let u = if self.reversed { 1.0 - u } else { u };
        match self.scale {
            ScaleKind::Linear => self.min + u * (self.max - self.min),
            ScaleKind::Log10 => {
                let (a, b) = (self.min.log10(), self.max.log10());
                10f64.powf(a + u * (b - a))
            }
            other => {
                let (a, b) = (other.forward(self.min), other.forward(self.max));
                other.inverse(a + u * (b - a))
            }
        }
    }
}

fn pow10(e: i32) -> f64 {
    format!("1e{e}").parse().unwrap_or_else(|_| 10f64.powi(e))
}

/// Rounds `raw` up to the next value of `mult[i] * 10^k`.
fn round_up_to(raw: f64, mult: &[f64]) -> f64 {
    if !(raw.is_finite() && raw > 0.0) {
        return 1.0;
    }
    let mut e = raw.log10().floor() as i32;
    // Guard against log10 rounding: make sure 10^e <= raw < 10^(e+1).
    if pow10(e + 1) <= raw * (1.0 + 1e-12) {
        e += 1;
    }
    let p = pow10(e);
    for m in mult {
        if m * p >= raw * (1.0 - 1e-9) {
            return m * p;
        }
    }
    10.0 * p
}

/// Rounds `raw` up to 1, 2, 5 or 10 times a power of ten.
pub fn nice_step(raw: f64) -> f64 {
    round_up_to(raw, &[1.0, 2.0, 5.0, 10.0])
}

/// Rounds `raw` up to 1, 1.5, 2, 2.5, 3, 4, 5 or 10 times a power of ten.
pub fn step15(raw: f64) -> f64 {
    round_up_to(raw, &[1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 10.0])
}

/// Limit and step of a zero-centred axis for data reaching `max_abs`:
/// returns `(limit, step)` so that the axis runs `-limit..+limit`.
pub fn symmetric_limit(max_abs: f64) -> (f64, f64) {
    if !(max_abs.is_finite() && max_abs > 0.0) {
        return (1.0, 0.5);
    }
    let target = max_abs * 1.05;
    let step = step15(target / 3.0);
    let n = (target / step - 1e-9).ceil().max(1.0);
    (clean(n * step), step)
}

/// [`clean`], except on a date axis, where values are about 2.4e6 and 12 digits is too coarse.
fn clean_d(v: f64, date: bool) -> f64 {
    if date { v } else { clean(v) }
}

/// Rounds to 12 significant digits to remove floating-point noise.
pub(crate) fn clean(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    format!("{v:.11e}").parse().unwrap_or(v)
}

/// Resolves limits, ticks and labels for `axis`.
///
/// `data_min_max` is the range of the finite data on this axis (positive
/// values only for a log axis); `None` means there is no data.
pub fn resolve_axis(axis: &Axis, data_min_max: Option<(f64, f64)>) -> ResolvedAxis {
    resolve_axis_edges(axis, data_min_max, (true, true))
}

/// [`resolve_axis`] for data whose lower and/or upper end is the base of column plots, which an
/// automatic linear range does not pad (`free` is `(low, high)`: whether the end gets the margin).
pub fn resolve_axis_edges(axis: &Axis, data_min_max: Option<(f64, f64)>, free: (bool, bool)) -> ResolvedAxis {
    // Two finite limits so far apart that their span overflows cannot be drawn: ignore them.
    if let (Some(a), Some(b)) = (axis.min, axis.max)
        && a.is_finite()
        && b.is_finite()
        && !(b - a).is_finite()
    {
        let mut open = axis.clone();
        open.min = None;
        open.max = None;
        return resolve_axis_edges(&open, data_min_max, free);
    }
    // Both limits fixed with the lower one above the upper one: a reversed axis.
    // Resolve it with the limits swapped and mark it.
    if let (Some(a), Some(b)) = (axis.min, axis.max) {
        let valid = |v: f64| axis.scale.accepts(v);
        if valid(a) && valid(b) && a > b {
            let mut swapped = axis.clone();
            swapped.min = Some(b);
            swapped.max = Some(a);
            let mut r = resolve_axis_edges(&swapped, data_min_max, free);
            r.reversed = true;
            return r;
        }
    }
    match axis.scale {
        ScaleKind::Linear => resolve_linear(axis, data_min_max, free),
        ScaleKind::Log10 => resolve_log(axis, data_min_max),
        _ => crate::ticks_scales::resolve_other(axis, data_min_max),
    }
}

/// The most ticks a category axis gets with one tick per row; beyond that its step follows the
/// usual rule (graphing spec 14.3).
pub const CATEGORY_TICKS_MAX: f64 = 200.0;

/// Resolves a category axis (graphing spec 14.3): rows stand at 1, 2, 3, ..., the automatic limits
/// are half a step beyond the first and last drawn row and are not rounded, and the major step is 1
/// while the axis spans at most [`CATEGORY_TICKS_MAX`]. `rows` is the smallest and largest position
/// of a drawn row (`None` without rows, read as row 1).
pub(crate) fn resolve_category(axis: &Axis, rows: Option<(f64, f64)>) -> ResolvedAxis {
    let (lo, hi) = rows.unwrap_or((1.0, 1.0));
    let (auto_min, auto_max) = (lo - 0.5, hi + 0.5);
    let fixed = |v: Option<f64>| v.filter(|v| v.is_finite());
    let span = (fixed(axis.max).unwrap_or(auto_max) - fixed(axis.min).unwrap_or(auto_min)).abs();
    let step = axis
        .major_step
        .filter(|s| s.is_finite() && *s > 0.0)
        .unwrap_or_else(|| if span <= CATEGORY_TICKS_MAX { 1.0 } else { nice_step(span / 5.0) });
    let drawn = Axis { auto_range: AutoRange::Tight, major_step: Some(step), ..axis.clone() };
    resolve_axis(&drawn, Some((auto_min, auto_max)))
}

/// Replaces the tick labels with the cells of a label column (graphing spec 14.3): the major tick
/// at a whole number k shows row k (counted from 1); every other tick gets an empty label, which
/// draws nothing.
pub(crate) fn label_from_cells(resolved: &mut ResolvedAxis, cells: &[String]) {
    resolved.labels = resolved
        .major
        .iter()
        .map(|&v| {
            let k = v.round();
            let whole = (v - k).abs() <= 1e-9 * v.abs().max(1.0);
            if whole && k >= 1.0 && k <= cells.len() as f64 { cells[k as usize - 1].clone() } else { String::new() }
        })
        .collect();
}

fn resolve_linear(axis: &Axis, data: Option<(f64, f64)>, free: (bool, bool)) -> ResolvedAxis {
    let (mut lo, mut hi) = match data {
        Some((a, b)) if a.is_finite() && b.is_finite() => (a.min(b), a.max(b)),
        _ => (0.0, 1.0),
    };
    if lo == hi {
        if lo == 0.0 {
            lo = -1.0;
            hi = 1.0;
        } else {
            let (a, b) = (lo - 0.5 * lo.abs(), hi + 0.5 * hi.abs());
            lo = a;
            hi = b;
        }
    }
    let fixed_step = axis.major_step.filter(|s| s.is_finite() && *s > 0.0);
    let date = matches!(axis.tick_format, TickFormat::DateTime(_));
    // The major step of an axis that picks it: Origin's table for numbers (graphing spec section 4), a round
    // time for dates.
    let tidy_step = |span: f64| if date { date_step(span / 5.0) } else { nice_step(span / 5.0) };
    let auto_step = tidy_step;
    let fixed_min = axis.min.filter(|v| v.is_finite());
    let fixed_max = axis.max.filter(|v| v.is_finite());

    let (mut min, mut max, mut step) = match axis.auto_range {
        AutoRange::Nice if !date && fixed_step.is_none() && data.is_some() => {
            autorange::linear_range_edges(lo, hi, axis.rescale_margin_pct, free.0, free.1)
        }
        AutoRange::Nice => {
            let step = fixed_step.unwrap_or_else(|| auto_step(hi - lo));
            // Date numbers are large (about 2.4e6): they are not rounded to 12 digits (a second is 1e-5 days).
            (clean_d((lo / step + 1e-9).floor() * step, date), clean_d((hi / step - 1e-9).ceil() * step, date), step)
        }
        AutoRange::Tight => (lo, hi, fixed_step.unwrap_or_else(|| auto_step(hi - lo))),
        AutoRange::SymmetricZero => {
            let m = lo.abs().max(hi.abs());
            match fixed_step {
                Some(s) => {
                    let n = (m * 1.05 / s).ceil().max(1.0);
                    (-clean(n * s), clean(n * s), s)
                }
                None => {
                    let (l, s) = symmetric_limit(m);
                    (-l, l, s)
                }
            }
        }
    };

    let overridden = fixed_min.is_some() || fixed_max.is_some();
    if axis.auto_range == AutoRange::SymmetricZero && overridden {
        // With only one side fixed, the other side is its negative.
        match (fixed_min, fixed_max) {
            (Some(a), None) => {
                min = a;
                max = -a;
            }
            (None, Some(b)) => {
                max = b;
                min = -b;
            }
            (Some(a), Some(b)) => {
                min = a;
                max = b;
            }
            (None, None) => {}
        }
    } else {
        if let Some(a) = fixed_min {
            min = a;
        }
        if let Some(b) = fixed_max {
            max = b;
        }
    }
    if max.is_nan() || min.is_nan() || max <= min {
        max = min + 1.0;
    }
    if overridden {
        step = fixed_step.unwrap_or_else(|| auto_step(max - min));
    }

    let (major, step) = linear_major(min, max, step, date);
    let minor = linear_minor(min, max, step, axis.minor_ticks);
    let labels = linear_labels(axis, &major, step);
    ResolvedAxis { min, max, step, scale: ScaleKind::Linear, major, minor, labels, reversed: false }
}

pub(crate) fn linear_major(min: f64, max: f64, mut step: f64, date: bool) -> (Vec<f64>, f64) {
    let eps = 1e-9 * (max - min);
    loop {
        let k0 = ((min - eps) / step).ceil();
        let k1 = ((max + eps) / step).floor();
        if k1 - k0 + 1.0 > 200.0 {
            step *= 2.0;
            continue;
        }
        let mut v = Vec::new();
        let mut k = k0;
        while k <= k1 {
            let t = clean_d(k * step, date);
            v.push(if t.abs() <= eps { 0.0 } else { t });
            k += 1.0;
        }
        return (v, step);
    }
}

pub(crate) fn linear_minor(min: f64, max: f64, step: f64, per_gap: u32) -> Vec<f64> {
    if per_gap == 0 {
        return Vec::new();
    }
    let n = i64::from(per_gap) + 1;
    let sub = step / n as f64;
    let eps = 1e-9 * (max - min);
    let j0 = ((min - eps) / sub).ceil() as i64;
    let j1 = ((max + eps) / sub).floor() as i64;
    if j1 - j0 > 5000 {
        return Vec::new();
    }
    (j0..=j1).filter(|j| j.rem_euclid(n) != 0).map(|j| clean(j as f64 * sub)).collect()
}

fn resolve_log(axis: &Axis, data: Option<(f64, f64)>) -> ResolvedAxis {
    let have_data = matches!(data, Some((_, b)) if b.is_finite() && b > 0.0);
    let (lo, hi) = match data {
        Some((a, b)) if b.is_finite() && b > 0.0 => {
            let a = if a.is_finite() && a > 0.0 { a } else { b };
            (a.min(b), b)
        }
        _ => (1.0, 10.0),
    };
    let fixed_min = axis.min.filter(|v| v.is_finite() && *v > 0.0);
    let fixed_max = axis.max.filter(|v| v.is_finite() && *v > 0.0);
    // Origin's automatic range (graphing spec section 4) for "round out"; the other modes keep whole decades.
    let auto = (axis.auto_range == AutoRange::Nice && have_data)
        .then(|| autorange::log_range(lo, hi, axis.rescale_margin_pct));
    let (mut min, mut max) = match auto {
        Some(r) => (r.from, r.to),
        None => {
            let mut e_min = (lo.log10() + 1e-9).floor() as i32;
            let mut e_max = (hi.log10() - 1e-9).ceil() as i32;
            if e_max <= e_min {
                e_max = e_min + 1;
            }
            e_min = e_min.min(e_max - 1);
            (pow10(e_min), pow10(e_max))
        }
    };
    if let Some(a) = fixed_min {
        min = a;
    }
    if let Some(b) = fixed_max {
        max = b;
    }
    if max.is_nan() || min.is_nan() || max <= min {
        max = min * 10.0;
    }
    // Less than a decade: linear ticks (graphing spec 3.3 "log tick rule"; section 4 for the step).
    let linear_inc = match auto {
        Some(r) if fixed_min.is_none() && fixed_max.is_none() => match r.ticks {
            LogTicks::Linear(inc) => Some(inc),
            LogTicks::Decades => None,
        },
        _ if (max / min).log10() <= 1.0 => Some(autorange::log_linear_step(max - min)),
        _ => None,
    };
    if let Some(inc) = linear_inc {
        let (major, step) = linear_major(min, max, inc, false);
        let labels = linear_labels(axis, &major, step);
        return ResolvedAxis {
            min,
            max,
            step,
            scale: ScaleKind::Log10,
            major,
            minor: Vec::new(),
            labels,
            reversed: false,
        };
    }
    let e_min = (min.log10() - 1e-9).ceil() as i32;
    let e_max = (max.log10() + 1e-9).floor() as i32;
    let decades = (max.log10() - min.log10()).round().max(1.0) as i32;
    let k = ((decades + 7) / 8).max(1); // at most about 8 labelled decades
    let mut major = Vec::new();
    let mut labels = Vec::new();
    for e in e_min..=e_max {
        if e.rem_euclid(k) == 0 {
            major.push(pow10(e));
            labels.push(log_label(e, axis.tick_format));
        }
    }
    let mut minor = Vec::new();
    if axis.minor_ticks > 0 {
        for d in (min.log10().floor() as i32)..=(max.log10().ceil() as i32) {
            for m in 2..=9 {
                let v = f64::from(m) * pow10(d);
                if v >= min * (1.0 - 1e-9) && v <= max * (1.0 + 1e-9) {
                    minor.push(v);
                }
            }
        }
    }
    ResolvedAxis { min, max, step: pow10(k), scale: ScaleKind::Log10, major, minor, labels, reversed: false }
}

/// The label of the major tick at `10^e` on a log axis. With the automatic format it is written as
/// Origin writes it: as a plain number from 0.001 to 1000000, and as `1E7`, `1E−4` beyond
/// (graphing.md section 3.3). The scientific format gives `10` with a superscript exponent.
fn log_label(e: i32, format: TickFormat) -> String {
    match format {
        TickFormat::Scientific(_) => format!("10\\+({})", signed(e)),
        _ if (-3..=6).contains(&e) => {
            if e >= 0 {
                format!("1{}", "0".repeat(e as usize))
            } else {
                format!("0.{}1", "0".repeat((-e - 1) as usize))
            }
        }
        _ => format!("1E{}", signed(e)),
    }
}

fn signed(e: i32) -> String {
    if e < 0 { format!("{MINUS}{}", -e) } else { e.to_string() }
}

pub(crate) fn linear_labels(axis: &Axis, major: &[f64], step: f64) -> Vec<String> {
    let div = if axis.tick_label_divisor.is_finite() && axis.tick_label_divisor != 0.0 {
        axis.tick_label_divisor
    } else {
        1.0
    };
    if let TickFormat::DateTime(pat) = axis.tick_format {
        return date_labels(major, step, pat);
    }
    let vals: Vec<f64> = major.iter().map(|v| clean(v / div)).collect();
    match axis.tick_format {
        TickFormat::Auto => {
            let sd = (step / div).abs();
            let max_abs = vals.iter().fold(0.0f64, |m, v| m.max(v.abs()));
            if max_abs >= 1e6 || sd < 1e-4 {
                vals.iter().map(|v| scientific(*v, None)).collect()
            } else {
                let d = auto_decimals(sd);
                vals.iter().map(|v| decimal(*v, d)).collect()
            }
        }
        TickFormat::Decimal(d) => vals.iter().map(|v| decimal(*v, usize::from(d.min(15)))).collect(),
        TickFormat::Scientific(d) => vals.iter().map(|v| scientific(*v, Some(usize::from(d.min(15))))).collect(),
        TickFormat::DateTime(_) => unreachable!("handled above"),
    }
}

/// Tick steps for a date axis, in seconds: 1, 2, 5, 10, 15 and 30 seconds, the same in minutes,
/// 1, 2, 3, 6 and 12 hours, 1, 2 and 5 days and a week. Longer spans use the usual 1-2-5 steps in days.
const DATE_STEPS_S: [f64; 21] = [
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0, 10_800.0, 21_600.0,
    43_200.0, 86_400.0, 172_800.0, 432_000.0, 604_800.0,
];

fn date_step(raw_days: f64) -> f64 {
    if !(raw_days.is_finite() && raw_days > 0.0) {
        return 1.0;
    }
    let raw_s = raw_days * 86_400.0;
    match DATE_STEPS_S.iter().find(|s| **s >= raw_s) {
        Some(s) => s / 86_400.0,
        None => nice_step(raw_days),
    }
}

/// Tick labels of a date axis. Steps of a day or more show the date; shorter steps show the time of
/// day, with the date in front when the ticks cover more than one calendar day.
fn date_labels(major: &[f64], step: f64, pat: DatePattern) -> Vec<String> {
    // Tick values carry a few milliseconds of rounding noise; the smallest step is one second.
    let major: Vec<f64> = major.iter().map(|v| (v * 86_400.0).round() / 86_400.0).collect();
    let major = major.as_slice();
    if step >= 1.0 - 1e-9 {
        return major.iter().map(|v| datetime::format_date(*v, pat)).collect();
    }
    let time = if step < 1.0 / 1440.0 - 1e-12 { TimePattern::Hms } else { TimePattern::Hm };
    let multi_day = match (major.first(), major.last()) {
        (Some(a), Some(b)) => a.floor() != b.floor(),
        _ => false,
    };
    major
        .iter()
        .map(|v| if multi_day { datetime::format_date_time(*v, pat, time) } else { datetime::format_time(*v, time) })
        .collect()
}

/// Smallest number of decimals (0..=10) that represents `step` exactly enough.
fn auto_decimals(step: f64) -> usize {
    for d in 0..=10usize {
        let f = 10f64.powi(d as i32);
        let r = (step * f).round() / f;
        if (r - step).abs() <= 1e-9 * step.abs() {
            return d;
        }
    }
    10
}

/// Fixed-decimal text with a real minus sign; negative zero prints as `0`.
pub(crate) fn decimal(v: f64, d: usize) -> String {
    let s = format!("{:.*}", d, v.abs());
    let is_zero = s.bytes().all(|b| b == b'0' || b == b'.');
    if v < 0.0 && !is_zero { format!("{MINUS}{s}") } else { s }
}

/// `m×10^e` in markup form. `decimals` is the mantissa precision; `None`
/// trims trailing zeros.
pub(crate) fn scientific(v: f64, decimals: Option<usize>) -> String {
    if v == 0.0 {
        return decimal(0.0, decimals.unwrap_or(0));
    }
    let s = match decimals {
        Some(d) => format!("{:.*e}", d, v.abs()),
        None => format!("{:.9e}", v.abs()),
    };
    let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
    let mant =
        if decimals.is_none() && mant.contains('.') { mant.trim_end_matches('0').trim_end_matches('.') } else { mant };
    let exp: i32 = exp.parse().unwrap_or(0);
    let sign = if v < 0.0 { MINUS.to_string() } else { String::new() };
    format!("{sign}{mant}{TIMES}10\\+({})", signed(exp))
}

/// A default axis for tests in other modules.
#[cfg(test)]
pub(crate) fn tests_axis() -> Axis {
    tests::axis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::Color;
    use sheaf_core::graph::{Font, TickDirection};

    pub(super) fn axis() -> Axis {
        Axis {
            scale: ScaleKind::Linear,
            min: None,
            max: None,
            auto_range: AutoRange::Nice,
            rescale_margin_pct: 8.0,
            major_step: None,
            minor_ticks: 1,
            title: None,
            show_title: true,
            title_font: Font::new("Arial", 12.0),
            show_tick_labels: true,
            tick_font: Font::new("Arial", 10.0),
            tick_format: TickFormat::Auto,
            tick_label_divisor: 1.0,
            major_tick: TickDirection::Out,
            minor_tick: TickDirection::Out,
            major_tick_len_pt: 5.0,
            minor_tick_len_pt: 2.5,
            line_width_pt: 1.0,
            color: Color::BLACK,
            show_opposite: false,
            opposite_ticks: false,
            major_grid: None,
            minor_grid: None,
            side: Default::default(),
            color_follows_plot: false,
            title_color_follows_plot: false,
            tick_label_source: Default::default(),
            tick_label_angle_deg: 0.0,
            hide_end_label: false,
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn symmetric_check_values() {
        for (m, limit, step) in [
            (33.55, 45.0, 15.0),
            (10.5, 12.0, 4.0),
            (7.632, 9.0, 3.0),
            (7.0, 7.5, 2.5),
            (0.9, 1.2, 0.4),
            (5.041, 6.0, 2.0),
        ] {
            let (l, s) = symmetric_limit(m);
            assert!(close(l, limit) && close(s, step), "m={m}: got {l}, {s}");
        }
    }

    #[test]
    fn step_tables() {
        for (raw, want) in [
            (0.9, 1.0),
            (1.0, 1.0),
            (1.1, 2.0),
            (2.0, 2.0),
            (2.1, 5.0),
            (5.0, 5.0),
            (5.1, 10.0),
            (0.03, 0.05),
            (1000.0, 1000.0),
            (12.0, 20.0),
        ] {
            assert!(close(nice_step(raw), want), "nice_step({raw}) = {}", nice_step(raw));
        }
        for (raw, want) in
            [(1.2, 1.5), (1.6, 2.0), (2.2, 2.5), (2.6, 3.0), (3.5, 4.0), (4.5, 5.0), (6.0, 10.0), (0.11, 0.15)]
        {
            assert!(close(step15(raw), want), "step15({raw}) = {}", step15(raw));
        }
        assert_eq!(nice_step(0.0), 1.0);
        assert_eq!(nice_step(f64::NAN), 1.0);
    }

    #[test]
    fn nice_limits_round_outwards() {
        let r = resolve_axis(&axis(), Some((0.13, 9.2)));
        // Origin's own limits for this data (measured): 8 % margin, rounded out to half the step.
        assert_eq!((r.min, r.max, r.step), (-2.5, 10.0, 5.0));
        assert_eq!(r.major, vec![0.0, 5.0, 10.0]);
        assert_eq!(r.minor, vec![-2.5, 2.5, 7.5]);
        // The margin pushes data on a tick limit off it.
        let r = resolve_axis(&axis(), Some((0.0, 10.0)));
        assert_eq!((r.min, r.max), (-2.5, 12.5));
        // A margin of 0 gives the data limits.
        let mut a = axis();
        a.rescale_margin_pct = 0.0;
        let r = resolve_axis(&a, Some((0.0, 10.0)));
        assert_eq!((r.min, r.max, r.step), (0.0, 10.0, 5.0));
    }

    #[test]
    fn tight_and_fixed_limits() {
        let mut a = axis();
        a.auto_range = AutoRange::Tight;
        let r = resolve_axis(&a, Some((0.13, 0.97)));
        assert_eq!((r.min, r.max), (0.13, 0.97));
        a.min = Some(0.0);
        a.max = Some(1.0);
        let r = resolve_axis(&a, Some((0.13, 0.97)));
        assert_eq!((r.min, r.max, r.step), (0.0, 1.0, 0.2));
        assert_eq!(r.labels, vec!["0.0", "0.2", "0.4", "0.6", "0.8", "1.0"]);
        a.major_step = Some(0.25);
        let r = resolve_axis(&a, None);
        assert_eq!(r.major, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
    }

    #[test]
    fn symmetric_axis_and_one_sided_fix() {
        let mut a = axis();
        a.auto_range = AutoRange::SymmetricZero;
        let r = resolve_axis(&a, Some((-30.0, 33.55)));
        assert_eq!((r.min, r.max, r.step), (-45.0, 45.0, 15.0));
        assert_eq!(r.major.len(), 7);
        a.max = Some(20.0);
        let r = resolve_axis(&a, Some((-30.0, 33.55)));
        assert_eq!((r.min, r.max), (-20.0, 20.0));
        a.max = None;
        a.major_step = Some(10.0);
        let r = resolve_axis(&a, Some((-30.0, 33.55)));
        assert_eq!((r.min, r.max), (-40.0, 40.0));
    }

    #[test]
    fn limits_with_an_overflowing_span_are_ignored() {
        let mut a = axis();
        a.min = Some(1e308);
        a.max = Some(-1e308);
        let r = resolve_axis(&a, Some((0.0, 10.0)));
        assert!(!r.reversed);
        assert_eq!((r.min, r.max), (-2.5, 12.5));
        assert!(!r.major.is_empty() && r.labels.len() == r.major.len());
    }

    #[test]
    fn reversed_symmetric_and_log_axes() {
        // Symmetric about zero with both limits fixed behaves like the forward axis, flipped.
        let mut a = axis();
        a.auto_range = AutoRange::SymmetricZero;
        a.min = Some(0.6);
        a.max = Some(-0.2);
        let r = resolve_axis(&a, Some((-0.2, 0.6)));
        let mut f = a.clone();
        f.min = Some(-0.2);
        f.max = Some(0.6);
        let fwd = resolve_axis(&f, Some((-0.2, 0.6)));
        assert!(r.reversed && !fwd.reversed);
        assert_eq!((r.min, r.max, &r.major), (fwd.min, fwd.max, &fwd.major));
        assert!(r.to_unit(r.max).abs() < 1e-12 && (r.to_unit(r.min) - 1.0).abs() < 1e-12);

        // Log axis: 1000 to 1 runs from the right decade to the left one.
        let mut l = axis();
        l.scale = ScaleKind::Log10;
        l.min = Some(1000.0);
        l.max = Some(1.0);
        let r = resolve_axis(&l, Some((1.0, 1000.0)));
        assert!(r.reversed);
        assert_eq!((r.min, r.max), (1.0, 1000.0));
        assert_eq!(r.major, vec![1.0, 10.0, 100.0, 1000.0]);
        assert!(r.to_unit(1000.0).abs() < 1e-12);
        assert!((r.to_unit(1.0) - 1.0).abs() < 1e-12);
        assert!((r.to_unit(10.0) - 2.0 / 3.0).abs() < 1e-12);
        assert!((r.from_unit(r.to_unit(31.6)) - 31.6).abs() < 1e-9);
    }

    #[test]
    fn degenerate_data_ranges() {
        let r = resolve_axis(&axis(), None);
        assert_eq!((r.min, r.max), (0.0, 1.0));
        let r = resolve_axis(&axis(), Some((0.0, 0.0)));
        assert!(r.min <= -1.0 && r.max >= 1.0);
        let r = resolve_axis(&axis(), Some((4.0, 4.0)));
        assert!(r.min < 4.0 && r.max > 4.0);
    }

    #[test]
    fn tick_values_are_cleaned() {
        let mut a = axis();
        a.min = Some(0.0);
        a.max = Some(0.5);
        a.major_step = Some(0.1);
        let r = resolve_axis(&a, None);
        assert_eq!(r.major, vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5]);
        assert_eq!(r.labels[3], "0.3");
    }

    #[test]
    fn auto_decimals_follow_the_step() {
        let mut a = axis();
        a.min = Some(0.0);
        a.max = Some(1.0);
        assert_eq!(resolve_axis(&a, None).labels[1], "0.2");
        a.max = Some(100.0);
        assert_eq!(resolve_axis(&a, None).labels[1], "20");
        a.max = Some(0.05);
        assert_eq!(resolve_axis(&a, None).labels[1], "0.01");
        a.min = Some(-1.0);
        a.max = Some(1.0);
        let r = resolve_axis(&a, None);
        assert_eq!(r.labels[0], "\u{2212}1.0");
        assert_eq!(r.labels[r.labels.len() / 2], "0.0");
    }

    #[test]
    fn divisor_and_explicit_formats() {
        let mut a = axis();
        a.min = Some(0.0);
        a.max = Some(5000.0);
        a.tick_label_divisor = 1000.0;
        assert_eq!(resolve_axis(&a, None).labels, vec!["0", "1", "2", "3", "4", "5"]);
        a.tick_label_divisor = 1.0;
        a.tick_format = TickFormat::Decimal(2);
        assert_eq!(resolve_axis(&a, None).labels[1], "1000.00");
        a.tick_format = TickFormat::Scientific(1);
        let l = resolve_axis(&a, None).labels;
        assert_eq!(l[0], "0.0");
        assert_eq!(l[1], "1.0\u{d7}10\\+(3)");
    }

    #[test]
    fn date_axis_uses_round_times_and_date_labels() {
        use sheaf_core::datetime::{date_value, time_value};
        let mut a = axis();
        a.tick_format = TickFormat::DateTime(DatePattern::Iso);
        let d0 = date_value(2026, 11, 1).unwrap();
        // Thirty days: weekly steps, labelled as dates.
        let r = resolve_axis(&a, Some((d0, d0 + 30.0)));
        assert!(r.labels.len() >= 4 && r.labels.len() <= 12, "{:?}", r.labels);
        assert!(r.labels.iter().all(|l| l.len() == 10 && l.starts_with("2026-")), "{:?}", r.labels);
        assert!((r.step - 7.0).abs() < 1e-9, "{}", r.step);
        // Six hours on one day: two-hour ticks labelled with the time only.
        let t0 = d0 + time_value(9, 0, 0.0).unwrap();
        let r = resolve_axis(&a, Some((t0, t0 + 6.0 / 24.0)));
        assert!(r.labels.iter().all(|l| l.len() == 5 && l.as_bytes()[2] == b':'), "{:?}", r.labels);
        assert!((r.step - 2.0 / 24.0).abs() < 1e-9, "{}", r.step);
        assert_eq!(r.labels[0], "08:00");
        // Across midnight the date is part of the label.
        let t1 = d0 + time_value(20, 0, 0.0).unwrap();
        let r = resolve_axis(&a, Some((t1, t1 + 8.0 / 24.0)));
        assert!(r.labels.iter().any(|l| l.starts_with("2026-11-02 ")), "{:?}", r.labels);
        // Two minutes: second steps show seconds.
        let r = resolve_axis(&a, Some((t0, t0 + 120.0 / 86_400.0)));
        assert!(r.labels[0].len() == 8, "{:?}", r.labels);
        // A plain axis is not affected.
        assert_eq!(resolve_axis(&axis(), Some((0.0, 1.0))).labels, vec!["0.0", "0.5", "1.0"]);
    }

    #[test]
    fn scientific_labels_when_large_or_tiny() {
        let mut a = axis();
        a.min = Some(0.0);
        a.max = Some(5e6);
        let r = resolve_axis(&a, None);
        assert_eq!(r.labels[0], "0");
        assert_eq!(r.labels[1], "1\u{d7}10\\+(6)");
        a.min = Some(-3e-5);
        a.max = Some(3e-5);
        let r = resolve_axis(&a, None);
        assert!(r.labels.contains(&"\u{2212}2\u{d7}10\\+(\u{2212}5)".to_string()), "{:?}", r.labels);
        assert_eq!(scientific(1.25e-7, None), "1.25\u{d7}10\\+(\u{2212}7)");
        assert_eq!(decimal(-0.0, 1), "0.0");
        assert_eq!(decimal(-0.04, 1), "0.0");
    }

    #[test]
    fn log_axis_decades() {
        let mut a = axis();
        a.scale = ScaleKind::Log10;
        let r = resolve_axis(&a, Some((3.0, 7000.0)));
        // Origin pads 8 % of the log span and rounds to the next 1..9 tick: 3..7000 gives 1..20000.
        assert_eq!((r.min, r.max), (1.0, 20000.0));
        assert_eq!(r.major, vec![1.0, 10.0, 100.0, 1000.0, 10000.0]);
        assert_eq!(r.labels, vec!["1", "10", "100", "1000", "10000"], "plain numbers, as Origin writes them");
        assert_eq!(r.minor.len(), 4 * 8 + 1, "2..9 in four decades and 20000");
        assert!(close(r.to_unit(100.0), 2.0 / 20000f64.log10()));
        assert!(close(r.from_unit(0.5), 10f64.powf(20000f64.log10() / 2.0)));
        assert!(r.to_unit(-1.0).is_nan());
        let r = resolve_axis(&a, Some((0.001, 1.0)));
        assert_eq!(r.labels[0], "0.001");
        // Beyond 0.001 and 1000000 Origin writes 1E-4 and 1E7.
        assert_eq!(log_label(-4, TickFormat::Auto), "1E\u{2212}4");
        assert_eq!(log_label(7, TickFormat::Auto), "1E7");
        assert_eq!(log_label(6, TickFormat::Auto), "1000000");
        assert_eq!(log_label(0, TickFormat::Decimal(2)), "1");
        // The scientific format keeps powers of ten.
        a.tick_format = TickFormat::Scientific(0);
        let r = resolve_axis(&a, Some((0.001, 1.0)));
        assert_eq!(r.labels[0], "10\\+(\u{2212}3)");
        a.tick_format = TickFormat::Auto;
        // Data on powers of ten are padded too (measured in Origin: 10..1000 gives 6..2000).
        let r = resolve_axis(&a, Some((10.0, 1000.0)));
        assert_eq!((r.min, r.max), (6.0, 2000.0));
        // With the margin at 0 they stay where they are.
        a.rescale_margin_pct = 0.0;
        let r = resolve_axis(&a, Some((10.0, 1000.0)));
        assert_eq!((r.min, r.max), (10.0, 1000.0));
        a.rescale_margin_pct = 8.0;
        // No usable data.
        let r = resolve_axis(&a, None);
        assert_eq!((r.min, r.max), (1.0, 10.0));
    }

    #[test]
    fn log_axis_under_a_decade_gets_linear_ticks_as_origin() {
        let mut a = axis();
        a.scale = ScaleKind::Log10;
        // Measured: data 2.06..8 give 1.7778..9 and ticks at 2, 3, ... 9 (step 1); 20..60 give ticks every 10.
        let r = resolve_axis(&a, Some((2.06, 8.0)));
        assert!((r.min - 16.0 / 9.0).abs() < 1e-9 && close(r.max, 9.0), "{} {}", r.min, r.max);
        assert_eq!(r.major, vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        assert_eq!(r.labels[0], "2");
        assert!(r.minor.is_empty());
        let r = resolve_axis(&a, Some((20.0, 60.0)));
        assert_eq!(r.major, vec![20.0, 30.0, 40.0, 50.0, 60.0]);
        // A decade or more: ticks at the decades.
        let r = resolve_axis(&a, Some((1.5, 150.0)));
        assert_eq!((r.min, r.max), (1.0, 300.0));
        assert_eq!(r.major, vec![1.0, 10.0, 100.0]);
        // Fixed limits within one decade also get linear ticks.
        a.min = Some(1.0);
        a.max = Some(5.0);
        let r = resolve_axis(&a, Some((2.0, 3.0)));
        assert!(r.major.len() >= 4 && r.major.iter().all(|v| (1.0..=5.0).contains(v)), "{:?}", r.major);
    }

    #[test]
    fn log_axis_thins_labels_over_many_decades() {
        let mut a = axis();
        a.scale = ScaleKind::Log10;
        let r = resolve_axis(&a, Some((1.0, 1e16)));
        assert!(r.major.len() <= 9, "{} labels", r.major.len());
        assert_eq!(r.step, 1000.0);
    }

    #[test]
    fn minor_ticks_between_majors() {
        let mut a = axis();
        a.minor_ticks = 4;
        a.min = Some(0.0);
        a.max = Some(10.0);
        let r = resolve_axis(&a, None);
        assert_eq!(r.step, 2.0);
        assert_eq!(r.minor.len(), 5 * 4);
        assert!(close(r.minor[0], 0.4));
        a.minor_ticks = 0;
        assert!(resolve_axis(&a, None).minor.is_empty());
        // Continued before the first major tick while inside the limits.
        a.minor_ticks = 1;
        a.min = Some(1.0);
        a.max = Some(9.0);
        let r = resolve_axis(&a, None);
        assert_eq!(r.major, vec![2.0, 4.0, 6.0, 8.0]);
        assert_eq!(r.minor, vec![1.0, 3.0, 5.0, 7.0, 9.0]);
    }

    #[test]
    fn never_more_than_200_ticks() {
        let mut a = axis();
        a.min = Some(0.0);
        a.max = Some(1000.0);
        a.major_step = Some(1.0);
        let r = resolve_axis(&a, None);
        assert!(r.major.len() <= 200);
        assert!(r.step >= 8.0);
    }

    #[test]
    fn unit_mapping_round_trips() {
        let r = resolve_axis(&axis(), Some((2.0, 8.0)));
        for v in [2.0, 3.5, 8.0] {
            assert!(close(r.from_unit(r.to_unit(v)), v));
        }
    }
}
