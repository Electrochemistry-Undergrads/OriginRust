//! Axis resolution: turns the axis settings and the data range into limits,
//! tick positions and tick label text (spec section 2).

use sheaf_core::autorange::{self, LogTicks};
use sheaf_core::datetime::{self, DatePattern, TimePattern};
use sheaf_core::graph::{AutoRange, Axis, AxisBreak, ScaleKind, TickFormat};
use std::borrow::Cow;

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
    /// The breaks of the axis that lie inside its limits, from the low end up (see [`AxisBreak`]).
    pub gaps: Vec<ResolvedBreak>,
    /// Labels of the minor ticks, one per entry of `minor`; empty unless the axis writes them.
    pub minor_labels: Vec<String>,
}

/// A break of an axis with its place on the axis, positions as fractions of the axis length from the
/// low end (before any reversal).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ResolvedBreak {
    /// First and last value that is cut out.
    pub from: f64,
    pub to: f64,
    /// Unit positions where the part before the break ends and the part after it starts.
    pub u_from: f64,
    pub u_to: f64,
    /// The unit positions the same values would have without the break.
    plain_from: f64,
    plain_to: f64,
}

impl ResolvedBreak {
    /// The gap on the page as `(low, high)` fractions of the length, in the direction the axis is drawn.
    pub fn span(&self, reversed: bool) -> (f64, f64) {
        if reversed { (1.0 - self.u_to, 1.0 - self.u_from) } else { (self.u_from, self.u_to) }
    }
}

impl ResolvedAxis {
    /// Axis limits in the direction they are drawn.
    pub fn endpoints(&self) -> (f64, f64) {
        if self.reversed { (self.max, self.min) } else { (self.min, self.max) }
    }

    /// The ticks that carry a label and the label: the major ticks, then the minor ones when the axis
    /// writes their labels.
    pub fn labelled(&self) -> impl Iterator<Item = (f64, &String)> {
        self.major.iter().copied().zip(&self.labels).chain(self.minor.iter().copied().zip(&self.minor_labels))
    }

    /// Position of `v` along the axis: 0 at `min`, 1 at `max`. Linear or
    /// logarithmic according to the scale. Values that cannot be placed
    /// (non-positive on a log axis) give NaN.
    pub fn to_unit(&self, v: f64) -> f64 {
        let u = self.to_unit_ascending(v);
        if self.reversed { 1.0 - u } else { u }
    }

    fn to_unit_ascending(&self, v: f64) -> f64 {
        let u = self.plain_unit(v);
        if self.gaps.is_empty() || !u.is_finite() {
            return u;
        }
        // Each part between two breaks is drawn linearly (in the axis' own scale) in its share of the length.
        let segs = self.segments();
        for (plain, unit) in &segs {
            if u >= plain.0 - 1e-12 && u <= plain.1 + 1e-12 {
                return lerp(u, *plain, *unit);
            }
        }
        // Below the low end or above the high end: extend the first or last part; inside a break: NaN.
        let (first, last) = (segs[0], segs[segs.len() - 1]);
        if u < first.0.0 {
            lerp(u, first.0, first.1)
        } else if u > last.0.1 {
            lerp(u, last.0, last.1)
        } else {
            f64::NAN
        }
    }

    /// The parts of the axis between its breaks: `(plain, unit)` ranges, where `plain` is the position the
    /// values would have without breaks and `unit` the position they are drawn at.
    fn segments(&self) -> Vec<((f64, f64), (f64, f64))> {
        let mut out = Vec::with_capacity(self.gaps.len() + 1);
        let (mut plain, mut unit) = (0.0, 0.0);
        for g in &self.gaps {
            out.push(((plain, g.plain_from), (unit, g.u_from)));
            (plain, unit) = (g.plain_to, g.u_to);
        }
        out.push(((plain, 1.0), (unit, 1.0)));
        out
    }

    /// The length of the axis in its own scale: `max - min` on a linear axis, the number of decades on a
    /// log10 axis, the transformed span on the others. What Origin's X:Y ratio compares (graphing 18.7a).
    pub fn scale_span(&self) -> f64 {
        match self.scale {
            ScaleKind::Linear => (self.max - self.min).abs(),
            ScaleKind::Log10 => (self.max.log10() - self.min.log10()).abs(),
            other => (other.forward(self.max) - other.forward(self.min)).abs(),
        }
    }

    /// Position along the axis without the break.
    fn plain_unit(&self, v: f64) -> f64 {
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
        let u = if self.gaps.is_empty() {
            u
        } else {
            // A position inside a gap reads as the value where that break starts.
            let segs = self.segments();
            let last = segs[segs.len() - 1];
            match segs.iter().position(|(_, unit)| u <= unit.1 + 1e-12) {
                Some(0) => lerp(u, segs[0].1, segs[0].0),
                Some(i) if u >= segs[i].1.0 - 1e-12 => lerp(u, segs[i].1, segs[i].0),
                Some(i) => segs[i - 1].0.1,
                None => lerp(u, last.1, last.0),
            }
        };
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

/// The value of 1, 2, 5 or 10 times a power of ten nearest to `raw` on a logarithmic scale.
pub fn nearest_nice_step(raw: f64) -> f64 {
    if !(raw.is_finite() && raw > 0.0) {
        return 1.0;
    }
    let p = pow10(raw.log10().floor() as i32);
    [1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|m| m * p)
        .min_by(|a, b| (a / raw).ln().abs().total_cmp(&(b / raw).ln().abs()))
        .unwrap_or(1.0)
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
            swapped.reverse = false;
            let mut r = resolve_axis_edges(&swapped, data_min_max, free);
            r.reversed = !axis.reverse;
            return r;
        }
    }
    let mut r = match axis.scale {
        ScaleKind::Linear => resolve_linear(axis, data_min_max, free),
        ScaleKind::Log10 => resolve_log(axis, data_min_max),
        _ => crate::ticks_scales::resolve_other(axis, data_min_max),
    };
    r.reversed ^= axis.reverse;
    apply_break(axis, &mut r);
    add_special_ticks(axis, &mut r);
    apply_label_formula(axis, &mut r, axis.tick_label_formula.trim(), false);
    if !axis.tick_prefix.is_empty() || !axis.tick_suffix.is_empty() {
        for label in r.labels.iter_mut().filter(|l| !l.is_empty()) {
            *label = format!("{}{label}{}", axis.tick_prefix, axis.tick_suffix);
        }
    }
    r
}

/// Adds the special ticks of `axis` that lie inside the limits (and outside a break), with their labels.
fn add_special_ticks(axis: &Axis, r: &mut ResolvedAxis) {
    let eps = 1e-9 * (r.max - r.min).abs();
    for t in &axis.special_ticks {
        let inside = t.value.is_finite() && t.value >= r.min - eps && t.value <= r.max + eps;
        let cut = r.gaps.iter().any(|g| t.value > g.from + eps && t.value < g.to - eps);
        if !inside || cut || r.major.iter().any(|v| (v - t.value).abs() <= eps) {
            continue;
        }
        let label =
            if t.label.is_empty() { linear_labels(axis, &[t.value], r.step.abs())[0].clone() } else { t.label.clone() };
        // A tick that was a minor tick becomes a major one.
        if let Some(i) = r.minor.iter().position(|v| (v - t.value).abs() <= eps) {
            r.minor.remove(i);
        }
        r.major.push(t.value);
        r.labels.push(label);
    }
}

/// Apply a facing formula to the shared positions. An empty override shows the numeric tick values.
pub(crate) fn opposite_labels<'a>(axis: &Axis, resolved: &'a ResolvedAxis) -> Cow<'a, ResolvedAxis> {
    let Some(formula) = axis.opposite_style.tick_label_formula.as_deref() else {
        return Cow::Borrowed(resolved);
    };
    let formula = formula.trim();
    let mut facing = resolved.clone();
    apply_label_formula(axis, &mut facing, if formula.is_empty() { "x" } else { formula }, true);
    Cow::Owned(facing)
}

/// Writes labels from an expression of the tick value. Special ticks with their own text keep it.
/// When relabelling one edge, decorate only replacement labels because inherited labels already have affixes.
fn apply_label_formula(axis: &Axis, r: &mut ResolvedAxis, formula: &str, with_affixes: bool) {
    let eps = 1e-9 * (r.max - r.min).abs();
    let own: Vec<bool> = r
        .major
        .iter()
        .map(|v| axis.special_ticks.iter().any(|t| !t.label.is_empty() && (t.value - v).abs() <= eps))
        .collect();
    let spacing = |values: &[f64]| {
        let mut v = values.to_vec();
        v.sort_by(f64::total_cmp);
        v.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 0.0).fold(f64::INFINITY, f64::min)
    };
    let write = |values: &[f64]| -> Vec<String> {
        if formula.is_empty() {
            return Vec::new();
        }
        let mapped: Vec<f64> =
            values.iter().map(|v| sheaf_core::scales::eval_tick_formula(formula, *v).unwrap_or(f64::NAN)).collect();
        let step = spacing(&mapped);
        let step = if step.is_finite() { step } else { 1.0 };
        let good: Vec<f64> = mapped.iter().map(|v| if v.is_finite() { *v } else { 0.0 }).collect();
        linear_labels(axis, &good, step)
            .into_iter()
            .zip(&mapped)
            .map(|(l, m)| if m.is_finite() { l } else { String::new() })
            .collect()
    };
    for (i, l) in write(&r.major).into_iter().enumerate() {
        if !own[i] {
            r.labels[i] =
                if with_affixes && !l.is_empty() && (!axis.tick_prefix.is_empty() || !axis.tick_suffix.is_empty()) {
                    format!("{}{l}{}", axis.tick_prefix, axis.tick_suffix)
                } else {
                    l
                };
        }
    }
    if axis.minor_tick_labels && !r.minor.is_empty() {
        r.minor_labels = if formula.is_empty() {
            let gap = r.step.abs() / f64::from(axis.minor_ticks.max(1) + 1);
            linear_labels(axis, &r.minor, if gap.is_finite() && gap > 0.0 { gap } else { 1.0 })
        } else {
            write(&r.minor)
        };
    }
}

/// Cuts the breaks of `axis` out of the resolved axis: the ticks inside them go, the ticks after a break
/// can follow their own step up to the next break, and positions are squeezed to leave the gaps.
/// Breaks that do not lie inside the limits, are empty, or overlap the one before are ignored, as is a
/// break whose gap would not fit after the gap before it. With Auto Position the breaks left share the
/// axis evenly (graphing spec 3.6: break k of n at k / (n + 1), as Origin 2016 placed one to seven).
fn apply_break(axis: &Axis, r: &mut ResolvedAxis) {
    let mut wanted: Vec<AxisBreak> = axis
        .axis_breaks
        .iter()
        .filter(|b| b.from.is_finite() && b.to.is_finite() && b.from < b.to && r.min < b.from && b.to < r.max)
        .copied()
        .collect();
    wanted.sort_by(|a, b| a.from.total_cmp(&b.from));
    let mut breaks: Vec<AxisBreak> = Vec::with_capacity(wanted.len());
    for b in wanted {
        if breaks.last().is_none_or(|p| b.from > p.to) {
            breaks.push(b);
        }
    }
    if breaks.is_empty() {
        return;
    }
    let plain = ResolvedAxis { gaps: Vec::new(), ..r.clone() };
    let n = breaks.len();
    let mut gaps: Vec<ResolvedBreak> = Vec::with_capacity(n);
    let mut used: Vec<AxisBreak> = Vec::with_capacity(n);
    for (k, b) in breaks.iter().enumerate() {
        let (plain_from, plain_to) = (plain.plain_unit(b.from), plain.plain_unit(b.to));
        if !(plain_from.is_finite() && plain_to.is_finite() && plain_from > 0.0 && plain_to < 1.0) {
            continue;
        }
        let pos = if b.auto_position { (k + 1) as f64 / (n + 1) as f64 } else { b.position_pct / 100.0 };
        let pos = pos.clamp(0.02, 0.98);
        let gap = (b.gap_pct / 100.0).clamp(0.0, 0.3);
        let (u_from, u_to) = (pos - gap / 2.0, pos + gap / 2.0);
        let floor = gaps.last().map_or(0.0, |g| g.u_to);
        if u_from <= floor || u_to >= 1.0 {
            continue;
        }
        gaps.push(ResolvedBreak { from: b.from, to: b.to, u_from, u_to, plain_from, plain_to });
        used.push(*b);
    }
    if gaps.is_empty() {
        return;
    }
    let eps = 1e-9 * (r.max - r.min);
    let inside = |v: f64| gaps.iter().any(|g| v > g.from + eps && v < g.to - eps);
    let mut ticks: Vec<(f64, String)> =
        r.major.iter().copied().zip(r.labels.iter().cloned()).filter(|(v, _)| !inside(*v)).collect();
    if r.scale == ScaleKind::Linear {
        for (k, b) in used.iter().enumerate() {
            let Some(step) = b.step_after.filter(|s| s.is_finite() && *s > 0.0) else { continue };
            let end = used.get(k + 1).map_or(r.max, |next| next.from);
            ticks.retain(|(v, _)| *v < b.to - eps || *v > end + eps);
            let after = linear_major(b.to, end, step, false).0;
            let labels = linear_labels(axis, &after, step);
            ticks.extend(after.into_iter().zip(labels));
        }
        ticks.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    r.minor.retain(|v| !inside(*v));
    (r.major, r.labels) = ticks.into_iter().unzip();
    r.gaps = gaps;
}

/// `u` mapped linearly from the range `from` to the range `to`.
fn lerp(u: f64, from: (f64, f64), to: (f64, f64)) -> f64 {
    let w = from.1 - from.0;
    if w.abs() < 1e-300 { to.0 } else { to.0 + (u - from.0) * (to.1 - to.0) / w }
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

    let (mut major, step) = linear_major(min, max, step, date);
    let mut minor = linear_minor(min, max, step, axis.minor_ticks);
    if let Some(anchor) = axis.first_tick.filter(|a| a.is_finite() && !date) {
        // Ticks at the anchor and whole steps from it: the multiples of the step, shifted.
        let shifted = |v: &mut Vec<f64>| v.iter_mut().for_each(|x| *x = clean(*x + anchor));
        (major, _) = linear_major(min - anchor, max - anchor, step, false);
        shifted(&mut major);
        minor = linear_minor(min - anchor, max - anchor, step, axis.minor_ticks);
        shifted(&mut minor);
    }
    let labels = linear_labels(axis, &major, step);
    ResolvedAxis {
        min,
        max,
        step,
        scale: ScaleKind::Linear,
        major,
        minor,
        labels,
        reversed: false,
        gaps: Vec::new(),
        minor_labels: Vec::new(),
    }
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
            gaps: Vec::new(),
            minor_labels: Vec::new(),
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
    ResolvedAxis {
        min,
        max,
        step: pow10(k),
        scale: ScaleKind::Log10,
        major,
        minor,
        labels,
        reversed: false,
        gaps: Vec::new(),
        minor_labels: Vec::new(),
    }
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
            opposite_offset_pct: None,
            opposite_style: Default::default(),
            major_grid: None,
            minor_grid: None,
            side: Default::default(),
            offset_pct: 0.0,
            color_follows_plot: false,
            title_color_follows_plot: false,
            title_on_top: false,
            tick_label_source: Default::default(),
            tick_label_angle_deg: 0.0,
            hide_end_label: false,
            reverse: false,
            first_tick: None,
            axis_breaks: Vec::new(),
            additional_line: None,
            at_position: None,
            arrow_begin: false,
            arrow_end: false,
            title_rotation_deg: None,
            title_offset_pt: (0.0, 0.0),
            special_ticks: Vec::new(),
            tick_label_formula: String::new(),
            minor_tick_labels: false,
            tick_color: None,
            tick_width_pt: None,
            tick_prefix: String::new(),
            tick_suffix: String::new(),
            rug: Default::default(),
            rug_opposite: Default::default(),
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
    fn first_tick_prefix_suffix_and_reverse() {
        let mut a = axis();
        a.min = Some(0.0);
        a.max = Some(10.0);
        a.major_step = Some(4.0);
        a.first_tick = Some(1.0);
        let r = resolve_axis(&a, Some((0.0, 10.0)));
        assert_eq!(r.major, vec![1.0, 5.0, 9.0]);
        a.first_tick = None;
        a.tick_suffix = " V".into();
        a.tick_prefix = "~".into();
        let r = resolve_axis(&a, Some((0.0, 10.0)));
        assert_eq!(r.labels, vec!["~0 V", "~4 V", "~8 V"]);
        assert!(!r.reversed);
        a.reverse = true;
        let r = resolve_axis(&a, Some((0.0, 10.0)));
        assert!(r.reversed && (r.to_unit(0.0) - 1.0).abs() < 1e-12);
        a.min = Some(10.0);
        a.max = Some(0.0);
        assert!(
            !resolve_axis(&a, Some((0.0, 10.0))).reversed,
            "a limit pair that is already reversed and the check box cancel"
        );
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

#[cfg(test)]
mod feature_tests {
    use super::*;
    use sheaf_core::Color;
    use sheaf_core::graph::{AxisBreak, SpecialTick};

    fn fixed(min: f64, max: f64, step: f64) -> Axis {
        Axis { min: Some(min), max: Some(max), major_step: Some(step), ..tests_axis() }
    }

    #[test]
    fn a_break_cuts_out_a_region_and_squeezes_the_rest() {
        let mut a = fixed(0.0, 100.0, 10.0);
        a.axis_breaks = vec![AxisBreak { step_after: Some(20.0), gap_pct: 2.0, ..AxisBreak::new(30.0, 70.0) }];
        let r = resolve_axis(&a, Some((0.0, 100.0)));
        assert_eq!(r.major, vec![0.0, 10.0, 20.0, 30.0, 80.0, 100.0]);
        // Ticks inside the break are gone; after it they follow the step after the break.
        assert!(r.major.iter().all(|v| *v <= 30.0 || *v >= 70.0));
        assert!(r.major.contains(&80.0) && !r.major.contains(&90.0));
        assert_eq!(r.labels.len(), r.major.len());
        // The gap sits at the middle of the axis (50 %, 2 % wide here); the ends keep their place.
        assert!((r.to_unit(0.0)).abs() < 1e-12 && (r.to_unit(100.0) - 1.0).abs() < 1e-12);
        assert!((r.to_unit(30.0) - 0.49).abs() < 1e-12 && (r.to_unit(70.0) - 0.51).abs() < 1e-12);
        assert!(r.to_unit(50.0).is_nan(), "values inside the break cannot be placed");
        assert!((r.from_unit(r.to_unit(85.0)) - 85.0).abs() < 1e-9);
        // A break outside the limits is ignored.
        a.axis_breaks = vec![AxisBreak::new(200.0, 300.0)];
        assert!(resolve_axis(&a, Some((0.0, 100.0))).gaps.is_empty());
    }

    /// Unit positions of the break ends read off Origin 2016's PNG exports (1600 px wide, axis -25 to 125,
    /// Auto Position, automatic Break Half Length; graphing spec 3.6) and the gap of Sheaf's default 1.75 %.
    #[test]
    fn several_breaks_share_the_axis_as_in_origin() {
        let tol = 0.0015; // about 1.3 px of the 879 px Y axis
        let at = |breaks: &[(f64, f64)]| {
            let mut a = fixed(-25.0, 125.0, 5.0);
            a.axis_breaks = breaks.iter().map(|&(f, t)| AxisBreak::new(f, t)).collect();
            resolve_axis(&a, Some((1.0, 100.0)))
        };
        // One break, Y axis: 12.5 at 0.4912, 87.5 at 0.5088.
        let r = at(&[(12.5, 87.5)]);
        assert!((r.to_unit(12.5) - 0.4912).abs() < tol && (r.to_unit(87.5) - 0.5088).abs() < tol);
        // Two breaks, Y axis (the second the middle half of what the first left).
        let r = at(&[(12.5, 87.5), (96.875, 115.625)]);
        for (v, u) in [(12.5, 0.3242), (87.5, 0.3424), (96.875, 0.6581), (115.625, 0.6758)] {
            assert!((r.to_unit(v) - u).abs() < tol, "{v}: {} against Origin's {u}", r.to_unit(v));
        }
        // Two breaks of the X axis made at once ([5, 35] and [65, 95]), 1090 px long.
        let r = at(&[(5.0, 35.0), (65.0, 95.0)]);
        for (v, u) in [(5.0, 0.3248), (35.0, 0.3413), (65.0, 0.6583), (95.0, 0.6752)] {
            assert!((r.to_unit(v) - u).abs() < tol, "{v}: {} against Origin's {u}", r.to_unit(v));
        }
        // Three breaks at 25, 50 and 75 %: each part keeps its own scale, the order of the list does not matter.
        let r = at(&[(117.96875, 122.65625), (12.5, 87.5), (96.875, 115.625)]);
        assert_eq!(r.gaps.len(), 3);
        let mids: Vec<f64> = r.gaps.iter().map(|g| (g.u_from + g.u_to) / 2.0).collect();
        assert!(mids.iter().zip([0.25, 0.5, 0.75]).all(|(m, e)| (m - e).abs() < 1e-12), "{mids:?}");
        assert!(r.to_unit(100.0).is_nan() && r.to_unit(120.0).is_nan());
        for v in [-25.0, 0.0, 12.5, 87.5, 90.0, 96.875, 115.625, 117.0, 122.65625, 125.0] {
            assert!((r.from_unit(r.to_unit(v)) - v).abs() < 1e-9, "{v}");
        }
        assert!(r.major.iter().all(|v| !(12.5 < *v && *v < 87.5) && !(96.875 < *v && *v < 115.625)));
        // A position inside a gap reads as the value where that break starts.
        assert!((r.from_unit(0.5) - 96.875).abs() < 1e-9);
        // Overlapping breaks keep the first; a manual position that leaves no room for the gap drops it.
        let mut a = fixed(0.0, 100.0, 10.0);
        a.axis_breaks = vec![AxisBreak::new(20.0, 40.0), AxisBreak::new(30.0, 50.0)];
        assert_eq!(resolve_axis(&a, Some((0.0, 100.0))).gaps.len(), 1);
        let manual = |pos: f64, f, t| AxisBreak { auto_position: false, position_pct: pos, ..AxisBreak::new(f, t) };
        a.axis_breaks = vec![manual(60.0, 20.0, 30.0), manual(40.0, 60.0, 70.0)];
        let r = resolve_axis(&a, Some((0.0, 100.0)));
        assert_eq!(r.gaps.len(), 1, "the second break would sit below the first");
        a.axis_breaks = vec![manual(30.0, 20.0, 30.0), manual(70.0, 60.0, 70.0)];
        let r = resolve_axis(&a, Some((0.0, 100.0)));
        assert!((r.gaps[0].u_from + r.gaps[0].u_to - 0.6).abs() < 1e-12);
        assert!((r.gaps[1].u_from + r.gaps[1].u_to - 1.4).abs() < 1e-12);
    }

    #[test]
    fn the_step_after_a_break_ends_at_the_next_break() {
        let mut a = fixed(0.0, 100.0, 10.0);
        a.axis_breaks =
            vec![AxisBreak { step_after: Some(5.0), ..AxisBreak::new(15.0, 35.0) }, AxisBreak::new(55.0, 75.0)];
        let r = resolve_axis(&a, Some((0.0, 100.0)));
        assert_eq!(r.major, vec![0.0, 10.0, 35.0, 40.0, 45.0, 50.0, 55.0, 80.0, 90.0, 100.0]);
        assert_eq!(r.labels.len(), r.major.len());
        assert_eq!(r.labels[2], "35");
    }

    #[test]
    fn special_ticks_the_label_formula_and_minor_labels() {
        let mut a = fixed(0.0, 10.0, 5.0);
        a.special_ticks = vec![
            SpecialTick { value: 2.5, label: "low".into() },
            SpecialTick { value: 7.0, label: String::new() },
            SpecialTick { value: 50.0, label: "outside".into() },
        ];
        let r = resolve_axis(&a, Some((0.0, 10.0)));
        assert_eq!(r.major, vec![0.0, 5.0, 10.0, 2.5, 7.0]);
        assert_eq!(r.labels, ["0", "5", "10", "low", "7"]);
        a.special_ticks.clear();
        a.tick_label_formula = "x*100".into();
        let r = resolve_axis(&a, Some((0.0, 10.0)));
        assert_eq!(r.labels, ["0", "500", "1000"]);
        a.tick_label_formula.clear();
        a.minor_tick_labels = true;
        let r = resolve_axis(&a, Some((0.0, 10.0)));
        assert_eq!(r.minor, vec![2.5, 7.5]);
        assert_eq!(r.minor_labels, ["2.5", "7.5"]);
        assert_eq!(r.labelled().count(), 5);
        // A formula that does not parse leaves the labels empty rather than wrong.
        a.tick_label_formula = "x*".into();
        let r = resolve_axis(&a, Some((0.0, 10.0)));
        assert!(r.labels.iter().all(String::is_empty));
        let _ = Color::BLACK;
    }
}
