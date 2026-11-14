//! Polar layers (graphing spec section 14.18): a circle with the angle running round it and the
//! radius from its centre to its edge.
//!
//! The data of a polar layer are read as `(angle in degrees, radius)` pairs (see `data::collect`),
//! so the plots themselves are the ordinary line and symbol plots; the only difference is how
//! [`LayerGeometry`] places a point (through [`PolarMap`]). This module draws what stands round
//! them: the grid, the circle with its angle labels and the radius axis.

use crate::fonts::TextMeasurer;
use crate::layout::AxisPaint;
use crate::plots::line_stroke;
use crate::scene::{Anchor, Cap, Item, Stroke, line_to, move_to, push_num};
use crate::text::{self, ASCENT, DESCENT};
use crate::ticks::{ResolvedAxis, resolve_axis_edges};
use crate::{LayerGeometry, Rect};
use sheaf_core::graph::{
    AngleUnit, Axis, AxisSide, Layer, LineStyle, PolarCoords, ScaleKind, TickDirection, TickLabelSource,
};

/// Where the circle of a polar layer is and how angles run on it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PolarMap {
    /// Centre of the circle on the page, in points.
    pub cx: f64,
    pub cy: f64,
    /// Radius of the circle, in points.
    pub radius: f64,
    /// Where angle 0 is, in degrees counter-clockwise from the right.
    pub zero_deg: f64,
    pub clockwise: bool,
    /// A radar chart: the first spoke points up, the spokes follow clockwise, and the rings and
    /// the outline are polygons (graphing spec section 14.19).
    pub radar: bool,
    /// The angle of a full turn in the unit of the angle column (360, or 2 pi for radians).
    pub circle: f64,
    /// The centre stands for radius 0 and the radius axis' lower limit is a hole round it (Origin's
    /// "Center at Zero", on by default for polar graphs, spec 14.18). A radar chart puts the lower
    /// limit at the centre.
    pub zero_centred: bool,
}

impl PolarMap {
    /// The circle of a polar layer drawn in `frame`: centred in it, half the smaller side wide.
    pub fn new(frame: Rect, p: &PolarCoords) -> PolarMap {
        PolarMap {
            cx: frame.x + frame.w / 2.0,
            cy: frame.y + frame.h / 2.0,
            radius: frame.w.min(frame.h) / 2.0,
            zero_deg: if p.radar {
                90.0
            } else if p.zero_deg.is_finite() {
                p.zero_deg
            } else {
                0.0
            },
            clockwise: p.clockwise || p.radar,
            radar: p.radar,
            circle: if p.radar { 360.0 } else { p.angle_unit.circle() },
            zero_centred: !p.radar,
        }
    }

    /// The direction, in radians counter-clockwise from the right, in which data angle `theta`
    /// (in the unit of the angle column) points on the page.
    pub fn direction(&self, theta: f64) -> f64 {
        let deg = theta * 360.0 / self.circle;
        let turn = if self.clockwise { -deg } else { deg };
        (self.zero_deg + turn).to_radians()
    }

    /// The page point at data angle `theta` and `u` times the radius of the circle.
    pub fn point(&self, theta: f64, u: f64) -> (f64, f64) {
        let (s, c) = self.direction(theta).sin_cos();
        (self.cx + u * self.radius * c, self.cy - u * self.radius * s)
    }

    /// The data angle in `[0, circle)` and the fraction of the radius at a page point.
    pub fn angle_and_unit(&self, px: f64, py: f64) -> (f64, f64) {
        let (dx, dy) = (px - self.cx, self.cy - py);
        let u = dx.hypot(dy) / self.radius;
        let a = dy.atan2(dx).to_degrees() - self.zero_deg;
        (if self.clockwise { -a } else { a }.rem_euclid(360.0) * self.circle / 360.0, u)
    }
}

/// The angle axis of a polar layer: one full turn (0 to 360 degrees in steps of 30, or 0 to 2 pi in
/// steps of a quarter of pi with labels such as `0.25` and the pi sign), whatever the data hold. The
/// end of the turn coincides with 0, so it carries no tick and no label of its own.
pub(crate) fn angle_axis(axis: &Axis, unit: AngleUnit) -> ResolvedAxis {
    if unit == AngleUnit::Radians {
        let quarter = std::f64::consts::FRAC_PI_4;
        let major: Vec<f64> = (0..8).map(|k| f64::from(k) * quarter).collect();
        let labels = (0..8).map(|k| format!("{:.2}\u{3c0}", f64::from(k) * 0.25)).collect();
        return ResolvedAxis {
            min: 0.0,
            max: std::f64::consts::TAU,
            step: quarter,
            scale: ScaleKind::Linear,
            major,
            minor: Vec::new(),
            labels,
            reversed: false,
            gap: None,
            minor_labels: Vec::new(),
        };
    }
    let fixed = Axis {
        scale: ScaleKind::Linear,
        min: Some(0.0),
        max: Some(360.0),
        major_step: Some(30.0),
        minor_ticks: 0,
        first_tick: None,
        axis_break: None,
        special_ticks: Vec::new(),
        tick_label_source: TickLabelSource::Numbers,
        tick_label_divisor: 1.0,
        tick_format: sheaf_core::graph::TickFormat::Auto,
        tick_prefix: String::new(),
        tick_suffix: String::new(),
        tick_label_formula: String::new(),
        reverse: false,
        ..axis.clone()
    };
    let mut r = resolve_axis_edges(&fixed, None, (false, false));
    let keep: Vec<bool> = r.major.iter().map(|v| *v < 360.0 - 1e-9).collect();
    let mut flags = keep.iter();
    r.major.retain(|_| *flags.next().unwrap_or(&true));
    let mut flags = keep.iter();
    r.labels.retain(|_| *flags.next().unwrap_or(&true));
    r
}

/// The angle axis of a radar layer: one major tick per spoke, at `360 i / n` degrees, labelled with
/// the spoke's name.
pub(crate) fn radar_axis(names: &[String]) -> ResolvedAxis {
    let n = names.len().max(1) as f64;
    ResolvedAxis {
        min: 0.0,
        max: 360.0,
        step: 360.0 / n,
        scale: ScaleKind::Linear,
        major: (0..names.len()).map(|i| 360.0 * i as f64 / n).collect(),
        minor: Vec::new(),
        labels: names.to_vec(),
        reversed: false,
        gap: None,
        minor_labels: Vec::new(),
    }
}

/// A closed polygon through the points at `u` times the radius on every spoke.
fn polygon(d: &mut String, map: &PolarMap, spokes: &[f64], u: f64) {
    for (i, &a) in spokes.iter().enumerate() {
        let p = map.point(a, u);
        if i == 0 {
            move_to(d, p.0, p.1);
        } else {
            line_to(d, p.0, p.1);
        }
    }
    if !spokes.is_empty() {
        d.push('Z');
    }
}

fn circle(d: &mut String, cx: f64, cy: f64, r: f64) {
    move_to(d, cx, cy - r);
    for (x, y) in [(cx, cy + r), (cx, cy - r)] {
        d.push('A');
        push_num(d, r);
        d.push(' ');
        push_num(d, r);
        d.push_str(" 0 1 1 ");
        push_num(d, x);
        d.push(' ');
        push_num(d, y);
    }
}

/// The grid: rings at the radius ticks, spokes at the angle ticks. The styles are those of the
/// layer's Y axis (rings) and X axis (spokes); minor rings first so that major ones lie on top.
pub(crate) fn draw_grid(out: &mut Vec<Item>, layer: &Layer, geom: &LayerGeometry) {
    let Some(map) = geom.polar else { return };
    let stroke = |style: &LineStyle| {
        let mut s = line_stroke(style);
        s.cap = Cap::Butt;
        s
    };
    let rings = |out: &mut Vec<Item>, style: &Option<LineStyle>, ticks: &[f64]| {
        let Some(style) = style else { return };
        let mut d = String::new();
        for &v in ticks {
            let u = geom.radius_unit(v);
            if u.is_finite() && u > 1e-9 && u < 1.0 - 1e-9 {
                if map.radar {
                    polygon(&mut d, &map, &geom.x.major, u);
                } else {
                    circle(&mut d, map.cx, map.cy, u * map.radius);
                }
            }
        }
        if !d.is_empty() {
            out.push(Item::Path { d, fill: None, stroke: Some(stroke(style)) });
        }
    };
    rings(out, &layer.y_axis.minor_grid, &geom.y.minor);
    rings(out, &layer.y_axis.major_grid, &geom.y.major);
    if let Some(style) = &layer.x_axis.major_grid {
        let mut d = String::new();
        // The spokes start at the hole round the centre, when the radius axis has one.
        let hole = if map.zero_centred { geom.radius_unit(geom.y.min).clamp(0.0, 1.0) } else { 0.0 };
        for &v in &geom.x.major {
            let (p, q) = (map.point(v, hole), map.point(v, 1.0));
            move_to(&mut d, p.0, p.1);
            line_to(&mut d, q.0, q.1);
        }
        if !d.is_empty() {
            out.push(Item::Path { d, fill: None, stroke: Some(stroke(style)) });
        }
    }
}

/// The part of a tick on one side of its axis: `(from, to)` offsets towards the inside of the
/// circle (negative is outwards).
fn tick_span(dir: TickDirection, len: f64) -> Option<(f64, f64)> {
    match dir {
        TickDirection::In => Some((0.0, len)),
        TickDirection::Out => Some((0.0, -len)),
        TickDirection::Both => Some((-len, len)),
        TickDirection::None => None,
    }
}

fn outward(dir: TickDirection, len: f64) -> f64 {
    match dir {
        TickDirection::Out | TickDirection::Both => len.max(0.0),
        _ => 0.0,
    }
}

/// Draws the outer circle, the angle ticks and the angle labels outside the circle. Returns the
/// smallest rectangle that holds the labels and the circle, `(left, top, right, bottom)`.
pub(crate) fn draw_angle_axis(
    out: &mut Vec<Item>,
    axis: &Axis,
    paint: &AxisPaint,
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) -> (f64, f64, f64, f64) {
    let Some(map) = geom.polar else { return (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY) };
    let mut edges = (map.cx - map.radius, map.cy - map.radius, map.cx + map.radius, map.cy + map.radius);
    if axis.side == AxisSide::Hidden {
        return edges;
    }
    let mut d = String::new();
    if map.radar {
        polygon(&mut d, &map, &geom.x.major, 1.0);
    } else {
        circle(&mut d, map.cx, map.cy, map.radius);
    }
    out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(paint.line, axis.line_width_pt)) });
    // Ticks across the circle.
    if let Some((a, b)) = tick_span(axis.major_tick, axis.major_tick_len_pt) {
        let mut d = String::new();
        for &v in &geom.x.major {
            let p = map.point(v, 1.0);
            let (s, c) = map.direction(v).sin_cos();
            // `a` and `b` count towards the centre: step back along the radius.
            move_to(&mut d, p.0 - a * c, p.1 + a * s);
            line_to(&mut d, p.0 - b * c, p.1 + b * s);
        }
        if !d.is_empty() {
            let stroke =
                Stroke::solid(axis.tick_color.unwrap_or(paint.line), axis.tick_width_pt.unwrap_or(axis.line_width_pt));
            out.push(Item::Path { d, fill: None, stroke: Some(stroke) });
        }
    }
    if !axis.show_tick_labels {
        return edges;
    }
    let size = axis.tick_font.size_pt;
    let gap = outward(axis.major_tick, axis.major_tick_len_pt) + 0.35 * size;
    for (v, label) in geom.x.labelled() {
        let (s, c) = map.direction(v).sin_cos();
        let p = (map.cx + (map.radius + gap) * c, map.cy - (map.radius + gap) * s);
        let w = text::width(m, label, &axis.tick_font, size);
        // The text box is pushed away from the circle by half its own size in the direction of
        // the label, so that the nearest point of the box (not its middle) is `gap` off the circle.
        let centre = (p.0 + c * w / 2.0, p.1 - s * (ASCENT + DESCENT) * size / 2.0);
        let baseline = centre.1 + (ASCENT - DESCENT) / 2.0 * size;
        text::place(out, m, label, &paint.labels, size, (centre.0, baseline), Anchor::Middle, 0.0);
        edges.0 = edges.0.min(centre.0 - w / 2.0);
        edges.2 = edges.2.max(centre.0 + w / 2.0);
        edges.1 = edges.1.min(centre.1 - (ASCENT + DESCENT) * size / 2.0);
        edges.3 = edges.3.max(centre.1 + (ASCENT + DESCENT) * size / 2.0);
    }
    edges
}

/// How far left of the circle's edge a polar layer's radius axis stands, in radii (measured, spec 14.18).
const RADIUS_AXIS_OFFSET: f64 = 0.2;

/// Draws the radius axis. A polar layer has it twice, left of the circle (0.2 radii off its edge): the
/// upper one runs from the circle's top down to the hole (the radius axis' lower limit) and the lower
/// one mirrors it, both labelled with the same numbers; with no hole they join at the centre, which
/// is labelled once. A radar chart has it on its first spoke only, from the centre up.
pub(crate) fn draw_radius_axis(
    out: &mut Vec<Item>,
    axis: &Axis,
    paint: &AxisPaint,
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) {
    let Some(map) = geom.polar else { return };
    if axis.side == AxisSide::Hidden {
        return;
    }
    let ax = if map.radar { map.cx } else { map.cx - (1.0 + RADIUS_AXIS_OFFSET) * map.radius };
    let hole = if map.radar { 0.0 } else { geom.radius_unit(geom.y.min).clamp(0.0, 1.0) };
    let mut d = String::new();
    if map.radar {
        move_to(&mut d, ax, map.cy - map.radius);
        line_to(&mut d, ax, map.cy);
    } else {
        move_to(&mut d, ax, map.cy - map.radius);
        line_to(&mut d, ax, map.cy - hole * map.radius);
        if hole < 1e-9 {
            line_to(&mut d, ax, map.cy + map.radius);
        } else {
            move_to(&mut d, ax, map.cy + hole * map.radius);
            line_to(&mut d, ax, map.cy + map.radius);
        }
    }
    out.push(Item::Path {
        d,
        fill: None,
        stroke: Some(Stroke { cap: Cap::Square, ..Stroke::solid(paint.line, axis.line_width_pt) }),
    });
    // The ticks point left (out) or right (in) of the axis line.
    let rows = |u: f64| match (u < 1e-9, map.radar) {
        (true, _) => vec![map.cy],
        (false, true) => vec![map.cy - u * map.radius],
        (false, false) => vec![map.cy - u * map.radius, map.cy + u * map.radius],
    };
    let mut d = String::new();
    for (values, dir, len) in [
        (&geom.y.major, axis.major_tick, axis.major_tick_len_pt),
        (&geom.y.minor, axis.minor_tick, if axis.minor_ticks > 0 { axis.minor_tick_len_pt } else { 0.0 }),
    ] {
        let Some((a, b)) = tick_span(dir, len) else { continue };
        for &v in values {
            let u = geom.radius_unit(v);
            if !u.is_finite() || !(-1e-9..=1.0 + 1e-9).contains(&u) {
                continue;
            }
            for y in rows(u) {
                move_to(&mut d, ax + a, y);
                line_to(&mut d, ax + b, y);
            }
        }
    }
    if !d.is_empty() {
        let stroke =
            Stroke::solid(axis.tick_color.unwrap_or(paint.line), axis.tick_width_pt.unwrap_or(axis.line_width_pt));
        out.push(Item::Path { d, fill: None, stroke: Some(stroke) });
    }
    if !axis.show_tick_labels {
        return;
    }
    let size = axis.tick_font.size_pt;
    let x = ax - outward(axis.major_tick, axis.major_tick_len_pt) - 0.35 * size;
    for (v, label) in geom.y.labelled() {
        let u = geom.radius_unit(v);
        if !u.is_finite() || !(-1e-9..=1.0 + 1e-9).contains(&u) {
            continue;
        }
        for y in rows(u) {
            text::place(out, m, label, &paint.labels, size, (x, y + (ASCENT - DESCENT) / 2.0 * size), Anchor::End, 0.0);
        }
    }
}
