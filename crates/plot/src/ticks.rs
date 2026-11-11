//! Axis resolution: turns the axis settings and the data range into limits,
//! tick positions and tick label text (spec section 2).

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

/// Rounds to 12 significant digits to remove floating-point noise.
fn clean(v: f64) -> f64 {
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
    // Both limits fixed with the lower one above the upper one: a reversed axis.
    // Resolve it with the limits swapped and mark it.
    if let (Some(a), Some(b)) = (axis.min, axis.max) {
        let valid = |v: f64| v.is_finite() && (axis.scale == ScaleKind::Linear || v > 0.0);
        if valid(a) && valid(b) && a > b {
            let mut swapped = axis.clone();
            swapped.min = Some(b);
            swapped.max = Some(a);
            let mut r = resolve_axis(&swapped, data_min_max);
            r.reversed = true;
            return r;
        }
    }
    match axis.scale {
        ScaleKind::Linear => resolve_linear(axis, data_min_max),
        ScaleKind::Log10 => resolve_log(axis, data_min_max),
    }
}

fn resolve_linear(axis: &Axis, data: Option<(f64, f64)>) -> ResolvedAxis {
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
    let fixed_min = axis.min.filter(|v| v.is_finite());
    let fixed_max = axis.max.filter(|v| v.is_finite());

    let (mut min, mut max, mut step) = match axis.auto_range {
        AutoRange::Nice => {
            let step = fixed_step.unwrap_or_else(|| nice_step((hi - lo) / 5.0));
            (clean((lo / step + 1e-9).floor() * step), clean((hi / step - 1e-9).ceil() * step), step)
        }
        AutoRange::Tight => (lo, hi, fixed_step.unwrap_or_else(|| nice_step((hi - lo) / 5.0))),
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
        step = fixed_step.unwrap_or_else(|| nice_step((max - min) / 5.0));
    }

    let (major, step) = linear_major(min, max, step);
    let minor = linear_minor(min, max, step, axis.minor_ticks);
    let labels = linear_labels(axis, &major, step);
    ResolvedAxis { min, max, step, scale: ScaleKind::Linear, major, minor, labels, reversed: false }
}

fn linear_major(min: f64, max: f64, mut step: f64) -> (Vec<f64>, f64) {
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
            let t = clean(k * step);
            v.push(if t.abs() <= eps { 0.0 } else { t });
            k += 1.0;
        }
        return (v, step);
    }
}

fn linear_minor(min: f64, max: f64, step: f64, per_gap: u32) -> Vec<f64> {
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
    let (lo, hi) = match data {
        Some((a, b)) if b.is_finite() && b > 0.0 => {
            let a = if a.is_finite() && a > 0.0 { a } else { b };
            (a.min(b), b)
        }
        _ => (1.0, 10.0),
    };
    let mut e_min = (lo.log10() + 1e-9).floor() as i32;
    let mut e_max = (hi.log10() - 1e-9).ceil() as i32;
    if e_max <= e_min {
        e_max = e_min + 1;
    }
    let mut min = pow10(e_min);
    let mut max = pow10(e_max);
    if let Some(a) = axis.min.filter(|v| v.is_finite() && *v > 0.0) {
        min = a;
    }
    if let Some(b) = axis.max.filter(|v| v.is_finite() && *v > 0.0) {
        max = b;
    }
    if max.is_nan() || min.is_nan() || max <= min {
        max = min * 10.0;
    }
    e_min = (min.log10() - 1e-9).ceil() as i32;
    e_max = (max.log10() + 1e-9).floor() as i32;
    let decades = (max.log10() - min.log10()).round().max(1.0) as i32;
    let k = ((decades + 7) / 8).max(1); // at most about 8 labelled decades
    let mut major = Vec::new();
    let mut labels = Vec::new();
    for e in e_min..=e_max {
        if e.rem_euclid(k) == 0 {
            major.push(pow10(e));
            labels.push(format!("10\\+({})", signed(e)));
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

fn signed(e: i32) -> String {
    if e < 0 { format!("{MINUS}{}", -e) } else { e.to_string() }
}

fn linear_labels(axis: &Axis, major: &[f64], step: f64) -> Vec<String> {
    let div = if axis.tick_label_divisor.is_finite() && axis.tick_label_divisor != 0.0 {
        axis.tick_label_divisor
    } else {
        1.0
    };
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
    }
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
fn decimal(v: f64, d: usize) -> String {
    let s = format!("{:.*}", d, v.abs());
    let is_zero = s.bytes().all(|b| b == b'0' || b == b'.');
    if v < 0.0 && !is_zero { format!("{MINUS}{s}") } else { s }
}

/// `m×10^e` in markup form. `decimals` is the mantissa precision; `None`
/// trims trailing zeros.
fn scientific(v: f64, decimals: Option<usize>) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::Color;
    use sheaf_core::graph::{Font, TickDirection};

    fn axis() -> Axis {
        Axis {
            scale: ScaleKind::Linear,
            min: None,
            max: None,
            auto_range: AutoRange::Nice,
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
        assert_eq!((r.min, r.max, r.step), (0.0, 10.0, 2.0));
        assert_eq!(r.major, vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        // Values already on a tick stay there.
        let r = resolve_axis(&axis(), Some((0.0, 10.0)));
        assert_eq!((r.min, r.max), (0.0, 10.0));
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
        assert_eq!((r.min, r.max), (1.0, 10000.0));
        assert_eq!(r.major, vec![1.0, 10.0, 100.0, 1000.0, 10000.0]);
        assert_eq!(r.labels[2], "10\\+(2)");
        assert_eq!(r.minor.len(), 4 * 8);
        assert!(close(r.to_unit(100.0), 0.5));
        assert!(close(r.from_unit(0.25), 10.0));
        assert!(r.to_unit(-1.0).is_nan());
        let r = resolve_axis(&a, Some((0.001, 1.0)));
        assert_eq!(r.labels[0], "10\\+(\u{2212}3)");
        // Exact powers of ten stay where they are.
        let r = resolve_axis(&a, Some((10.0, 1000.0)));
        assert_eq!((r.min, r.max), (10.0, 1000.0));
        // No usable data.
        let r = resolve_axis(&a, None);
        assert_eq!((r.min, r.max), (1.0, 10.0));
    }

    #[test]
    fn log_axis_thins_labels_over_many_decades() {
        let mut a = axis();
        a.scale = ScaleKind::Log10;
        let r = resolve_axis(&a, Some((1.0, 1e16)));
        assert!(r.major.len() <= 9, "{} labels", r.major.len());
        assert_eq!(r.step, 100.0);
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
