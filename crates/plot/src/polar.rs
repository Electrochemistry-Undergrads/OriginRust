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
use sheaf_core::graph::{Axis, AxisSide, Layer, LineStyle, PolarCoords, ScaleKind, TickDirection, TickLabelSource};

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
}

impl PolarMap {
    /// The circle of a polar layer drawn in `frame`: centred in it, half the smaller side wide.
    pub fn new(frame: Rect, p: &PolarCoords) -> PolarMap {
        PolarMap {
            cx: frame.x + frame.w / 2.0,
            cy: frame.y + frame.h / 2.0,
            radius: frame.w.min(frame.h) / 2.0,
            zero_deg: if p.zero_deg.is_finite() { p.zero_deg } else { 0.0 },
            clockwise: p.clockwise,
        }
    }

    /// The direction, in radians counter-clockwise from the right, in which data angle `theta`
    /// (degrees) points on the page.
    pub fn direction(&self, theta_deg: f64) -> f64 {
        let turn = if self.clockwise { -theta_deg } else { theta_deg };
        (self.zero_deg + turn).to_radians()
    }

    /// The page point at data angle `theta_deg` and `u` times the radius of the circle.
    pub fn point(&self, theta_deg: f64, u: f64) -> (f64, f64) {
        let (s, c) = self.direction(theta_deg).sin_cos();
        (self.cx + u * self.radius * c, self.cy - u * self.radius * s)
    }

    /// The data angle in `[0, 360)` and the fraction of the radius at a page point.
    pub fn angle_and_unit(&self, px: f64, py: f64) -> (f64, f64) {
        let (dx, dy) = (px - self.cx, self.cy - py);
        let u = dx.hypot(dy) / self.radius;
        let a = dy.atan2(dx).to_degrees() - self.zero_deg;
        (if self.clockwise { -a } else { a }.rem_euclid(360.0), u)
    }
}

/// The angle axis of a polar layer: 0 to 360 degrees in steps of 30, whatever the data hold. The
/// end at 360 coincides with 0, so it carries no tick and no label of its own.
pub(crate) fn angle_axis(axis: &Axis) -> ResolvedAxis {
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
            let u = geom.y.to_unit(v);
            if u.is_finite() && u > 1e-9 && u < 1.0 - 1e-9 {
                circle(&mut d, map.cx, map.cy, u * map.radius);
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
        for &v in &geom.x.major {
            let p = map.point(v, 1.0);
            move_to(&mut d, map.cx, map.cy);
            line_to(&mut d, p.0, p.1);
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
    circle(&mut d, map.cx, map.cy, map.radius);
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

/// Draws the radius axis along the vertical diameter: the line, and at every radius tick a tick
/// and a label on the upper half and the same on the lower half, mirrored (measured, spec 14.17).
/// The centre carries one label only.
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
    let mut d = String::new();
    move_to(&mut d, map.cx, map.cy - map.radius);
    line_to(&mut d, map.cx, map.cy + map.radius);
    out.push(Item::Path {
        d,
        fill: None,
        stroke: Some(Stroke { cap: Cap::Square, ..Stroke::solid(paint.line, axis.line_width_pt) }),
    });
    // The ticks point left (out) or right (in) of the axis line.
    let rows = |u: f64| if u < 1e-9 { vec![map.cy] } else { vec![map.cy - u * map.radius, map.cy + u * map.radius] };
    let mut d = String::new();
    for (values, dir, len) in [
        (&geom.y.major, axis.major_tick, axis.major_tick_len_pt),
        (&geom.y.minor, axis.minor_tick, if axis.minor_ticks > 0 { axis.minor_tick_len_pt } else { 0.0 }),
    ] {
        let Some((a, b)) = tick_span(dir, len) else { continue };
        for &v in values {
            let u = geom.y.to_unit(v);
            if !u.is_finite() || !(-1e-9..=1.0 + 1e-9).contains(&u) {
                continue;
            }
            for y in rows(u) {
                move_to(&mut d, map.cx + a, y);
                line_to(&mut d, map.cx + b, y);
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
    let x = map.cx - outward(axis.major_tick, axis.major_tick_len_pt) - 0.35 * size;
    for (v, label) in geom.y.labelled() {
        let u = geom.y.to_unit(v);
        if !u.is_finite() || !(-1e-9..=1.0 + 1e-9).contains(&u) {
            continue;
        }
        for y in rows(u) {
            text::place(out, m, label, &paint.labels, size, (x, y + (ASCENT - DESCENT) / 2.0 * size), Anchor::End, 0.0);
        }
    }
}
