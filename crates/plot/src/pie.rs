//! Origin's two other pies (graphing spec 14.17, 14.19): the 3D colour pie, an ellipse seen from
//! above at a slant with a side band, and the flat black and white pie whose slices carry hatch
//! patterns. The flat colour pie stays in `plots.rs`.
//!
//! The scene has no clip item, so a hatch is computed against the slice outline: [`hatch`] cuts
//! parallel lines against any simple polygon and returns the pieces inside it.

use crate::LayerGeometry;
use crate::Rect;
use crate::data::{PieData, PlotData};
use crate::scene::{Anchor, Item, Stroke, TextItem, line_to, move_to, push_num};
use sheaf_core::Color;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// Height of the ellipse over its width in the 3D pie (measured: about one half).
pub(crate) const SLANT: f64 = 0.5;
/// Thickness of the 3D pie's side band over the ellipse's width.
const BAND: f64 = 0.1;
/// Share of each channel kept in the side band's darker shade.
const DARKER: f64 = 0.6;

/// The angles (from, to) in radians of every slice, clockwise from 12 o'clock.
fn spans(pie: &PieData) -> Vec<(f64, f64)> {
    let total: f64 = pie.values.iter().sum();
    let mut from = 0.0;
    pie.values
        .iter()
        .map(|v| {
            let to = from + v / total * TAU;
            let s = (from, to);
            from = to;
            s
        })
        .collect()
}

fn darker(c: Color) -> Color {
    let k = |v: u8| (f64::from(v) * DARKER).round() as u8;
    Color::rgb(k(c.r), k(c.g), k(c.b))
}

fn arc_to(d: &mut String, rx: f64, ry: f64, sweep: bool, x: f64, y: f64) {
    d.push('A');
    push_num(d, rx);
    d.push(' ');
    push_num(d, ry);
    d.push_str(if sweep { " 0 0 1 " } else { " 0 0 0 " });
    push_num(d, x);
    d.push(' ');
    push_num(d, y);
}

/// The outline of a slice of the ellipse (`rx`, `ry`) round (`cx`, `cy`), as path data.
fn slice_path(cx: f64, cy: f64, rx: f64, ry: f64, from: f64, to: f64) -> String {
    let point = |a: f64| (cx + rx * a.sin(), cy - ry * a.cos());
    let mut d = String::new();
    if to - from >= TAU - 1e-9 {
        move_to(&mut d, cx, cy - ry);
        arc_to(&mut d, rx, ry, true, cx, cy + ry);
        arc_to(&mut d, rx, ry, true, cx, cy - ry);
    } else {
        let (p, q) = (point(from), point(to));
        move_to(&mut d, cx, cy);
        line_to(&mut d, p.0, p.1);
        d.push('A');
        push_num(&mut d, rx);
        d.push(' ');
        push_num(&mut d, ry);
        d.push_str(if to - from > PI { " 0 1 1 " } else { " 0 0 1 " });
        push_num(&mut d, q.0);
        d.push(' ');
        push_num(&mut d, q.1);
    }
    d.push('Z');
    d
}

/// The 3D colour pie.
pub(crate) fn draw_pie_3d(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(pie) = &pd.pie else { return };
    let f = geom.frame;
    out.push(Item::Rect { rect: f, fill: None, stroke: Some(Stroke::solid(Color::BLACK, 1.0)) });
    if pie.values.iter().sum::<f64>() <= 0.0 {
        return;
    }
    let rx = (0.42 * f.h / (SLANT + BAND)).min(0.3 * f.w);
    let (ry, t) = (SLANT * rx, BAND * 2.0 * rx);
    let (cx, cy) = (f.x + 0.42 * f.w, f.y + 0.5 * f.h - 0.5 * t);
    let point = |a: f64| (cx + rx * a.sin(), cy - ry * a.cos());
    let outline = Stroke::solid(Color::BLACK, 0.5);
    let spans = spans(pie);
    // The visible part of the side band is the front half of the rim: angles 90 to 270 degrees.
    for (i, &(from, to)) in spans.iter().enumerate() {
        let (a, b) = (from.max(FRAC_PI_2), to.min(PI + FRAC_PI_2));
        if b <= a {
            continue;
        }
        let (p, q) = (point(a), point(b));
        let mut d = String::new();
        move_to(&mut d, p.0, p.1);
        arc_to(&mut d, rx, ry, true, q.0, q.1);
        line_to(&mut d, q.0, q.1 + t);
        arc_to(&mut d, rx, ry, false, p.0, p.1 + t);
        d.push('Z');
        out.push(Item::Path { d, fill: Some(darker(pie.colors[i])), stroke: Some(outline.clone()) });
    }
    for (i, &(from, to)) in spans.iter().enumerate() {
        let d = slice_path(cx, cy, rx, ry, from, to);
        out.push(Item::Path { d, fill: Some(pie.colors[i]), stroke: Some(outline.clone()) });
    }
    for &(from, to) in &spans {
        let mid = (from + to) / 2.0;
        // Labels of the front half sit below the band's lower edge.
        let drop = if mid.cos() < 0.0 { t } else { 0.0 };
        let (x, y) = (cx + 1.12 * rx * mid.sin(), cy - 1.12 * ry * mid.cos() + drop);
        label(out, pie, to - from, x, y, mid);
    }
    let colors = pie.colors.clone();
    legend(out, pie, f, &mut |out, rect, i| {
        out.push(Item::Rect { rect, fill: Some(colors[i]), stroke: Some(outline.clone()) });
    });
}

/// The 2D black and white pie: white slices with a hatch pattern each.
pub(crate) fn draw_pie_bw(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(pie) = &pd.pie else { return };
    let f = geom.frame;
    out.push(Item::Rect { rect: f, fill: None, stroke: Some(Stroke::solid(Color::BLACK, 1.0)) });
    if pie.values.iter().sum::<f64>() <= 0.0 {
        return;
    }
    let r = 0.35 * f.h.min(f.w);
    let (cx, cy) = (f.x + 0.42 * f.w, f.y + 0.5 * f.h);
    let point = |a: f64, k: f64| (cx + k * r * a.sin(), cy - k * r * a.cos());
    let outline = Stroke::solid(Color::BLACK, 0.5);
    let spans = spans(pie);
    for (i, &(from, to)) in spans.iter().enumerate() {
        // The slice as a polygon (the arc in steps of about 2 degrees) for the hatch.
        let steps = (((to - from) / 0.035).ceil() as usize).max(2);
        let mut poly: Vec<(f64, f64)> = Vec::new();
        if to - from < TAU - 1e-9 {
            poly.push((cx, cy));
        }
        poly.extend((0..=steps).map(|k| point(from + (to - from) * k as f64 / steps as f64, 1.0)));
        let d = slice_path(cx, cy, r, r, from, to);
        out.push(Item::Path { d: d.clone(), fill: Some(Color::WHITE), stroke: None });
        pattern(out, &poly, i);
        out.push(Item::Path { d, fill: None, stroke: Some(outline.clone()) });
        let mid = (from + to) / 2.0;
        let (x, y) = point(mid, 1.12);
        label(out, pie, to - from, x, y, mid);
    }
    legend(out, pie, f, &mut |out, rect, i| {
        out.push(Item::Rect { rect, fill: Some(Color::WHITE), stroke: None });
        let (x1, y1) = (rect.x + rect.w, rect.y + rect.h);
        pattern(out, &[(rect.x, rect.y), (x1, rect.y), (x1, y1), (rect.x, y1)], i);
        out.push(Item::Rect { rect, fill: None, stroke: Some(outline.clone()) });
    });
}

/// The share of a slice (`span` radians of the circle), outside it next to its middle (`mid` is
/// the angle of the middle).
fn label(out: &mut Vec<Item>, pie: &PieData, span: f64, x: f64, y: f64, mid: f64) {
    let size = pie.label_font.size_pt;
    let anchor = if mid.sin() > 0.3 {
        Anchor::Start
    } else if mid.sin() < -0.3 {
        Anchor::End
    } else {
        Anchor::Middle
    };
    out.push(Item::Text(TextItem {
        x,
        y: y + 0.35 * size,
        text: format!("{:.2}%", span / TAU * 100.0),
        family: pie.label_font.family.clone(),
        size,
        bold: pie.label_font.bold,
        italic: pie.label_font.italic,
        color: pie.label_font.color,
        anchor,
        rotate_ccw: 0.0,
    }));
}

/// The legend: a square and the name of each slice from the top right corner down; `square` draws
/// the square of slice `i` in the rectangle it is given.
fn legend(out: &mut Vec<Item>, pie: &PieData, f: Rect, square: &mut dyn FnMut(&mut Vec<Item>, Rect, usize)) {
    let fs = pie.legend_font.size_pt;
    let row = 1.3 * fs;
    let width = pie.names.iter().map(|n| n.chars().count()).max().unwrap_or(0) as f64 * 0.55 * fs + 2.4 * fs;
    let x0 = f.x + f.w - width - 0.5 * fs;
    for (i, name) in pie.names.iter().enumerate() {
        let y = f.y + 0.5 * fs + i as f64 * row;
        square(out, Rect { x: x0, y, w: fs, h: fs }, i);
        out.push(Item::Text(TextItem {
            x: x0 + 1.4 * fs,
            y: y + 0.8 * fs,
            text: name.clone(),
            family: pie.legend_font.family.clone(),
            size: fs,
            bold: pie.legend_font.bold,
            italic: pie.legend_font.italic,
            color: pie.legend_font.color,
            anchor: Anchor::Start,
            rotate_ccw: 0.0,
        }));
    }
}

/// The hatch patterns of the black and white pie, by slice: the angles of the line families
/// (degrees, counter-clockwise from the right) and their spacing in points. Slices past the
/// eighth reuse the patterns with a wider spacing.
const PATTERNS: [(&[f64], f64); 8] = [
    (&[45.0], 3.0),
    (&[135.0], 3.0),
    (&[0.0], 3.0),
    (&[90.0], 3.0),
    (&[0.0, 90.0], 4.0),
    (&[45.0, 135.0], 4.0),
    (&[45.0], 1.8),
    (&[135.0], 1.8),
];

/// Draws the hatch pattern of slice `i` inside `poly`.
pub(crate) fn pattern(out: &mut Vec<Item>, poly: &[(f64, f64)], i: usize) {
    let (angles, spacing) = PATTERNS[i % PATTERNS.len()];
    let spacing = spacing * (1 + i / PATTERNS.len()) as f64;
    let mut d = String::new();
    for &a in angles {
        for (p, q) in hatch(poly, a.to_radians(), spacing) {
            move_to(&mut d, p.0, p.1);
            line_to(&mut d, q.0, q.1);
        }
    }
    if !d.is_empty() {
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(Color::BLACK, 0.5)) });
    }
}

/// Parallel lines `spacing` apart at `angle` (radians, counter-clockwise as seen on the page, so
/// 45 degrees runs from the lower left to the upper right) cut to the inside of the simple
/// polygon `poly`. The lines sit at multiples of `spacing` from the page origin, so that
/// neighbouring shapes continue each other's pattern.
pub(crate) fn hatch(poly: &[(f64, f64)], angle: f64, spacing: f64) -> Vec<((f64, f64), (f64, f64))> {
    let u = (angle.cos(), -angle.sin());
    let n = (-u.1, u.0);
    let along = |p: (f64, f64)| p.0 * u.0 + p.1 * u.1;
    let across = |p: (f64, f64)| p.0 * n.0 + p.1 * n.1;
    if poly.len() < 3 || spacing <= 0.0 {
        return Vec::new();
    }
    let lo = poly.iter().map(|p| across(*p)).fold(f64::INFINITY, f64::min);
    let hi = poly.iter().map(|p| across(*p)).fold(f64::NEG_INFINITY, f64::max);
    let mut out = Vec::new();
    let mut k = (lo / spacing).ceil();
    while k * spacing <= hi {
        let c = k * spacing;
        k += 1.0;
        let mut hits: Vec<f64> = Vec::new();
        for j in 0..poly.len() {
            let (p, q) = (poly[j], poly[(j + 1) % poly.len()]);
            let (sp, sq) = (across(p) - c, across(q) - c);
            if (sp < 0.0) != (sq < 0.0) {
                let t = sp / (sp - sq);
                hits.push(along(p) + t * (along(q) - along(p)));
            }
        }
        hits.sort_by(|a, b| a.total_cmp(b));
        for pair in hits.chunks(2).filter(|p| p.len() == 2) {
            if pair[1] - pair[0] > 1e-6 {
                let at = |s: f64| (s * u.0 + c * n.0, s * u.1 + c * n.1);
                out.push((at(pair[0]), at(pair[1])));
            }
        }
    }
    out
}
