//! Ternary layers (graphing spec section 14.20): an equilateral triangle in which the three
//! fractions of a row give one point.
//!
//! The plots of a ternary layer are ordinary scatter (or line) plots. `data::collect` replaces the
//! columns of each by the normalised X and Y fractions of the rows (Z is `1 - x - y`), and
//! [`LayerGeometry`] places a point through [`TernaryMap`]. This module draws the grid, the three
//! axes with their ticks, labels and titles.
//!
//! Corners: X = 1 is the bottom right corner, Y = 1 the top corner and Z = 1 the bottom left
//! corner. The X axis runs along the bottom side from left to right, the Y axis along the right
//! side from bottom to top and the Z axis along the left side from top to bottom.

use crate::data::{PlotData, row_range};
use crate::fonts::TextMeasurer;
use crate::layout::AxisPaint;
use crate::plots::line_stroke;
use crate::scene::{Anchor, Cap, Item, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use crate::ticks::{ResolvedAxis, resolve_axis_edges};
use crate::{LayerGeometry, Rect};
use sheaf_core::Project;
use sheaf_core::graph::{Axis, AxisSide, Layer, LineStyle, ScaleKind, TernaryCoords, TernaryScale, TickDirection};
use std::borrow::Cow;

/// Where the triangle of a ternary layer is.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TernaryMap {
    /// Bottom left corner (Z = 1), page points.
    pub left: (f64, f64),
    /// Bottom right corner (X = 1).
    pub right: (f64, f64),
    /// Top corner (Y = 1).
    pub top: (f64, f64),
    /// True when the ticks count 0 to 100 instead of 0 to 1.
    pub percent: bool,
}

impl TernaryMap {
    /// The largest triangle that fits in `frame`, centred in it.
    pub fn new(frame: Rect, percent: bool) -> TernaryMap {
        let height_ratio = 3f64.sqrt() / 2.0;
        let side = frame.w.min(frame.h / height_ratio).max(0.0);
        let h = side * height_ratio;
        let (cx, cy) = (frame.x + frame.w / 2.0, frame.y + frame.h / 2.0);
        TernaryMap {
            left: (cx - side / 2.0, cy + h / 2.0),
            right: (cx + side / 2.0, cy + h / 2.0),
            top: (cx, cy - h / 2.0),
            percent,
        }
    }

    /// Length of a side.
    pub fn side(&self) -> f64 {
        self.right.0 - self.left.0
    }

    /// The page point of the fractions `x` and `y` (and `z = 1 - x - y`).
    pub fn point(&self, x: f64, y: f64) -> (f64, f64) {
        let z = 1.0 - x - y;
        (x * self.right.0 + y * self.top.0 + z * self.left.0, x * self.right.1 + y * self.top.1 + z * self.left.1)
    }

    /// The fractions `(x, y)` at a page point; the inverse of [`TernaryMap::point`].
    pub fn fractions(&self, px: f64, py: f64) -> (f64, f64) {
        let h = self.left.1 - self.top.1;
        let side = self.side();
        if h <= 0.0 || side <= 0.0 {
            return (f64::NAN, f64::NAN);
        }
        let y = (self.left.1 - py) / h;
        (((px - self.left.0) - y * side / 2.0) / side, y)
    }
}

/// The three parts of a row as fractions of their sum: `(x, y)`; `z` is the rest. A row with a
/// missing or negative part, or a zero sum, has none.
pub fn normalise(x: f64, y: f64, z: f64) -> Option<(f64, f64)> {
    let sum = x + y + z;
    (x.is_finite() && y.is_finite() && z.is_finite() && x >= 0.0 && y >= 0.0 && z >= 0.0 && sum > 0.0)
        .then(|| (x / sum, y / sum))
}

/// Rewrites the columns of a plot of a ternary layer: `x` and `y` become the normalised fractions
/// of each row (rows without a place are NaN, which no plot draws), `z` is the third column.
pub(crate) fn columns(pd: &mut PlotData, z: Option<&[f64]>) {
    let n = pd.y.len();
    let (mut xs, mut ys) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    if let (Some(x), Some(z)) = (pd.x.as_deref(), z) {
        for r in 0..n.min(x.len()).min(z.len()) {
            if let Some((a, b)) = normalise(x[r], pd.y[r], z[r]) {
                (xs[r], ys[r]) = (a, b);
            }
        }
    }
    pd.x = Some(Cow::Owned(xs));
    pd.y = Cow::Owned(ys);
    pd.x_format = None;
    pd.size_z = None;
    pd.color_z = None;
    pd.err = None;
    pd.xerr = None;
}

/// Whether the ticks of a layer count in percent: by the layer's setting, or (automatic) when a
/// row of the raw data sums to about 100 (99 to 101).
pub(crate) fn is_percent(project: &Project, layer: &Layer, settings: &TernaryCoords) -> bool {
    match settings.scale {
        TernaryScale::Fraction => false,
        TernaryScale::Percent => true,
        TernaryScale::Auto => layer.plots.iter().filter(|p| p.visible).any(|p| {
            let (Some(x), Some(y)) = (p.x.and_then(|c| project.numeric(c)), project.numeric(p.y)) else {
                return false;
            };
            let Some(z) = p.size_from.and_then(|c| project.numeric(c)) else { return false };
            let (a, b) = row_range(p, y.len());
            let filters = project.plot_row_filters(p);
            (a..b).filter(|r| filters.iter().all(|f| f.passes(*r))).any(|r| {
                let sum = x.get(r).copied().unwrap_or(f64::NAN) + y[r] + z.get(r).copied().unwrap_or(f64::NAN);
                normalise(x.get(r).copied().unwrap_or(f64::NAN), y[r], z.get(r).copied().unwrap_or(f64::NAN)).is_some()
                    && (99.0..=101.0).contains(&sum)
            })
        }),
    }
}

/// The scale shared by the three axes: 0 to 1 in steps of 0.25, or 0 to 100 in steps of 25
/// (Origin, measured: labels 0.00 to 1.00, or 0 to 100), with the tick settings of `axis`.
pub(crate) fn scale_axis(axis: &Axis, percent: bool) -> ResolvedAxis {
    let top = if percent { 100.0 } else { 1.0 };
    let fixed = Axis {
        scale: ScaleKind::Linear,
        min: Some(0.0),
        max: Some(top),
        major_step: Some(top / 4.0),
        minor_ticks: axis.minor_ticks,
        first_tick: None,
        axis_breaks: Vec::new(),
        additional_line: None,
        special_ticks: Vec::new(),
        reverse: false,
        ..axis.clone()
    };
    resolve_axis_edges(&fixed, None, (false, false))
}

/// The three titles: bottom (X), right (Y), left (Z).
fn titles(project: &Project, layer: &Layer, plots: &[PlotData]) -> [String; 3] {
    let name = |id: Option<sheaf_core::ColumnId>| {
        id.and_then(|c| project.column(c)).map(|c| c.axis_title()).unwrap_or_default()
    };
    let first = plots.first().map(|pd| pd.plot);
    [
        layer.x_axis.title.clone().unwrap_or_else(|| name(first.and_then(|p| p.x))),
        layer.y_axis.title.clone().unwrap_or_else(|| name(first.map(|p| p.y))),
        name(first.and_then(|p| p.size_from)),
    ]
}

/// A grid line between two fraction pairs.
fn grid_path(d: &mut String, map: &TernaryMap, a: (f64, f64), b: (f64, f64)) {
    let (p, q) = (map.point(a.0, a.1), map.point(b.0, b.1));
    move_to(d, p.0, p.1);
    line_to(d, q.0, q.1);
}

/// The grid: for each tick strictly between the ends, one line parallel to the side that the
/// axis' own value is 0 on. X lines and Z lines take the style of the layer's X axis, Y lines that
/// of its Y axis.
pub(crate) fn draw_grid(out: &mut Vec<Item>, layer: &Layer, geom: &LayerGeometry) {
    let Some(map) = geom.ternary else { return };
    let top = if map.percent { 100.0 } else { 1.0 };
    let inner = |values: &[f64]| -> Vec<f64> {
        values.iter().map(|v| v / top).filter(|v| *v > 1e-9 && *v < 1.0 - 1e-9).collect()
    };
    let (ticks, minor) = (inner(&geom.x.major), inner(&geom.x.minor));
    let stroke = |style: &LineStyle| {
        let mut s = line_stroke(style);
        s.cap = Cap::Butt;
        s
    };
    let xz = |t: &[f64]| -> Vec<((f64, f64), (f64, f64))> {
        t.iter().flat_map(|&v| [((v, 0.0), (v, 1.0 - v)), ((1.0 - v, 0.0), (0.0, 1.0 - v))]).collect()
    };
    let y = |t: &[f64]| -> Vec<((f64, f64), (f64, f64))> { t.iter().map(|&v| ((0.0, v), (1.0 - v, v))).collect() };
    // Minor lines first (Origin draws them dashed in light grey), the major lines over them.
    let families: [(&Option<LineStyle>, Vec<((f64, f64), (f64, f64))>); 4] = [
        (&layer.x_axis.minor_grid, xz(&minor)),
        (&layer.y_axis.minor_grid, y(&minor)),
        (&layer.x_axis.major_grid, xz(&ticks)),
        (&layer.y_axis.major_grid, y(&ticks)),
    ];
    for (style, lines) in families {
        let Some(style) = style else { continue };
        let mut d = String::new();
        for (a, b) in lines {
            grid_path(&mut d, &map, a, b);
        }
        if !d.is_empty() {
            out.push(Item::Path { d, fill: None, stroke: Some(stroke(style)) });
        }
    }
}

/// One of the three sides of the triangle.
struct Side {
    /// The page point of the side where the axis value is `v` (0 to 1).
    at: Box<dyn Fn(f64) -> (f64, f64)>,
    /// Unit vector pointing away from the triangle along a grid line (the direction of the ticks).
    tick: (f64, f64),
    /// Unit vector of the outward normal of the side.
    normal: (f64, f64),
    /// Rotation of the title, degrees counter-clockwise.
    title_rotation: f64,
    /// Which axis' settings the side uses: 0 X, 1 Y.
    axis: usize,
}

fn sides(map: &TernaryMap) -> [Side; 3] {
    let m = *map;
    let k = 3f64.sqrt() / 2.0;
    [
        // Bottom, X from left to right; ticks leave along the grid line X = const, down and left.
        Side {
            at: Box::new(move |v| m.point(v, 0.0)),
            tick: (-0.5, k),
            normal: (0.0, 1.0),
            title_rotation: 0.0,
            axis: 0,
        },
        // Right, Y from bottom to top; the grid lines Y = const are horizontal.
        Side {
            at: Box::new(move |v| m.point(1.0 - v, v)),
            tick: (1.0, 0.0),
            normal: (k, -0.5),
            title_rotation: -60.0,
            axis: 1,
        },
        // Left, Z from top to bottom; the grid lines Z = const run parallel to the right side.
        Side {
            at: Box::new(move |v| m.point(0.0, 1.0 - v)),
            tick: (-0.5, -k),
            normal: (-k, -0.5),
            title_rotation: 60.0,
            axis: 0,
        },
    ]
}

/// The rectangle `(left, top, right, bottom)` of a `w` by `h` box centred on `c` and turned by
/// `deg` degrees counter-clockwise.
fn turned_box(c: (f64, f64), w: f64, h: f64, deg: f64) -> (f64, f64, f64, f64) {
    let (s, co) = deg.to_radians().sin_cos();
    let (hx, hy) = ((w * co).abs() / 2.0 + (h * s).abs() / 2.0, (w * s).abs() / 2.0 + (h * co).abs() / 2.0);
    (c.0 - hx, c.1 - hy, c.0 + hx, c.1 + hy)
}

/// Draws the triangle, the ticks, the tick labels and the titles of a ternary layer. Returns the
/// smallest rectangle `(left, top, right, bottom)` that holds all of it.
pub(crate) fn draw_axes(
    out: &mut Vec<Item>,
    project: &Project,
    layer: &Layer,
    plots: &[PlotData],
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) -> (f64, f64, f64, f64) {
    let Some(map) = geom.ternary else { return (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY) };
    let mut edges = (map.left.0.min(map.top.0), map.top.1, map.right.0.max(map.top.0), map.left.1.max(map.right.1));
    let axes = [&layer.x_axis, &layer.y_axis];
    let paints = [AxisPaint::new(axes[0], None), AxisPaint::new(axes[1], None)];
    if axes[0].side != AxisSide::Hidden {
        let mut d = String::new();
        move_to(&mut d, map.left.0, map.left.1);
        line_to(&mut d, map.right.0, map.right.1);
        line_to(&mut d, map.top.0, map.top.1);
        d.push('Z');
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(paints[0].line, axes[0].line_width_pt)) });
    }
    let top = if map.percent { 100.0 } else { 1.0 };
    let names = titles(project, layer, plots);
    for (side, name) in sides(&map).iter().zip(&names) {
        let (axis, paint) = (axes[side.axis], &paints[side.axis]);
        if axis.side == AxisSide::Hidden {
            continue;
        }
        let tick_len = axis.major_tick_len_pt;
        // Ticks along the grid line, across the side.
        if let Some((a, b)) = match axis.major_tick {
            TickDirection::In => Some((0.0, -tick_len)),
            TickDirection::Out => Some((0.0, tick_len)),
            TickDirection::Both => Some((-tick_len, tick_len)),
            TickDirection::None => None,
        } {
            let mut d = String::new();
            for &v in &geom.x.major {
                let p = (side.at)(v / top);
                move_to(&mut d, p.0 + side.tick.0 * a, p.1 + side.tick.1 * a);
                line_to(&mut d, p.0 + side.tick.0 * b, p.1 + side.tick.1 * b);
            }
            if !d.is_empty() {
                let stroke = Stroke::solid(
                    axis.tick_color.unwrap_or(paint.line),
                    axis.tick_width_pt.unwrap_or(axis.line_width_pt),
                );
                out.push(Item::Path { d, fill: None, stroke: Some(stroke) });
            }
        }
        // Minor ticks the same way, shorter (Origin shows one between the major ticks).
        let minor_len = axis.minor_tick_len_pt;
        if let Some((a, b)) = match axis.minor_tick {
            TickDirection::In => Some((0.0, -minor_len)),
            TickDirection::Out => Some((0.0, minor_len)),
            TickDirection::Both => Some((-minor_len, minor_len)),
            TickDirection::None => None,
        } {
            let mut d = String::new();
            for &v in &geom.x.minor {
                let p = (side.at)(v / top);
                move_to(&mut d, p.0 + side.tick.0 * a, p.1 + side.tick.1 * a);
                line_to(&mut d, p.0 + side.tick.0 * b, p.1 + side.tick.1 * b);
            }
            if !d.is_empty() {
                let stroke = Stroke::solid(
                    axis.tick_color.unwrap_or(paint.line),
                    axis.tick_width_pt.unwrap_or(axis.line_width_pt),
                );
                out.push(Item::Path { d, fill: None, stroke: Some(stroke) });
            }
        }
        let out_len = match axis.major_tick {
            TickDirection::Out | TickDirection::Both => tick_len.max(0.0),
            _ => 0.0,
        };
        // The distance, along the normal, of the labels' outer edge from the side.
        let mut reach = out_len * (side.tick.0 * side.normal.0 + side.tick.1 * side.normal.1);
        let size = axis.tick_font.size_pt;
        if axis.show_tick_labels {
            let gap = 0.35 * size;
            for (v, label) in geom.x.labelled() {
                let p = (side.at)(v / top);
                let end = (p.0 + side.tick.0 * out_len, p.1 + side.tick.1 * out_len);
                let (w, h) = (text::width(m, label, &axis.tick_font, size), (ASCENT + DESCENT) * size);
                // Pushed off the tick end along the normal until the nearest point of the box is a gap away.
                let push = gap + side.normal.0.abs() * w / 2.0 + side.normal.1.abs() * h / 2.0;
                let centre = (end.0 + side.normal.0 * push, end.1 + side.normal.1 * push);
                let baseline = centre.1 + (ASCENT - DESCENT) / 2.0 * size;
                text::place(out, m, label, &paint.labels, size, (centre.0, baseline), Anchor::Middle, 0.0);
                let b = turned_box(centre, w, h, 0.0);
                edges = (edges.0.min(b.0), edges.1.min(b.1), edges.2.max(b.2), edges.3.max(b.3));
                let far = (centre.0 - p.0) * side.normal.0
                    + (centre.1 - p.1) * side.normal.1
                    + side.normal.0.abs() * w / 2.0
                    + side.normal.1.abs() * h / 2.0;
                reach = reach.max(far);
            }
        }
        if axis.show_title && !name.is_empty() {
            let tsize = axis.title_font.size_pt;
            let mid = (side.at)(0.5);
            let width = text::width(m, name, &axis.title_font, tsize);
            let thick = (ASCENT + DESCENT) * tsize;
            let dist = reach + 0.6 * tsize;
            // The baseline of a title below the triangle is its far edge; the sides' titles are
            // turned so that their tops face away from the triangle.
            let (baseline_off, body) = if side.title_rotation == 0.0 {
                (dist + ASCENT * tsize, dist + thick / 2.0)
            } else {
                (dist + DESCENT * tsize, dist + thick / 2.0)
            };
            let anchor = (mid.0 + side.normal.0 * baseline_off, mid.1 + side.normal.1 * baseline_off);
            text::place(out, m, name, &paint.title, tsize, anchor, Anchor::Middle, side.title_rotation);
            let c = (mid.0 + side.normal.0 * body, mid.1 + side.normal.1 * body);
            let b = turned_box(c, width, thick, side.title_rotation);
            edges = (edges.0.min(b.0), edges.1.min(b.1), edges.2.max(b.2), edges.3.max(b.3));
        }
    }
    edges
}
