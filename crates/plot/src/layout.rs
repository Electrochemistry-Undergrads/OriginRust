//! Graph to scene (spec section 3): frames, axes, ticks, labels, titles,
//! legend and free text, in drawing order.

use crate::data::{self, PlotData};
use crate::fonts::TextMeasurer;
use crate::plots::{draw_plot, line_stroke, symbol_item, symbol_path};
use crate::scene::{Anchor, Cap, Item, Join, LayerScene, Scene, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use crate::ticks::ResolvedAxis;
use crate::{LayerGeometry, PlotError, Rect};
use sheaf_core::graph::{Axis, Layer, LegendPosition, LineStyle, Page, TickDirection};
use sheaf_core::{Project, WindowId};

/// Builds the scene of a graph.
pub(crate) fn build_scene(project: &Project, graph: WindowId, m: &dyn TextMeasurer) -> Result<Scene, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    let mut layers = Vec::new();
    for layer in &g.layers {
        let pds = data::collect(project, layer);
        let geom = layer_geometry(layer, &g.page, &pds, m);
        layers.push(layer_scene(project, layer, &geom, &pds, m));
    }
    Ok(Scene { width: g.page.width_pt, height: g.page.height_pt, background: g.page.background, layers })
}

/// Smallest distance in points kept between any label or title and the page edge.
const PAGE_MARGIN: f64 = 3.0;

/// The geometry of a layer: the nominal frame from the model, moved inwards
/// just enough that the Y title and the bottom X title stay inside the page
/// (spec section 3, "Keeping labels on the page"). When everything already
/// fits, the frame is the nominal one.
pub(crate) fn layer_geometry(layer: &Layer, page: &Page, pds: &[PlotData], m: &dyn TextMeasurer) -> LayerGeometry {
    let mut geom = data::resolve_layer(layer, page, pds);
    let mut scratch = Vec::new();
    let x_bottom = draw_x_labels(&mut scratch, layer, &geom, m);
    let y_left = draw_y_labels(&mut scratch, layer, &geom, m);
    let (xa, ya) = (&layer.x_axis, &layer.y_axis);
    let left_edge = if ya.show_title {
        let size = ya.title_font.size_pt;
        y_left - 0.6 * size - (ASCENT + DESCENT) * size
    } else {
        y_left
    };
    let bottom_edge = if xa.show_title {
        let size = xa.title_font.size_pt;
        x_bottom + 0.6 * size + (ASCENT + DESCENT) * size
    } else {
        x_bottom
    };
    let f = &mut geom.frame;
    // Never take more than half of the frame, whatever the labels need.
    let shift_x = (PAGE_MARGIN - left_edge).clamp(0.0, 0.5 * f.w);
    f.x += shift_x;
    f.w -= shift_x;
    let shift_y = (bottom_edge - (page.height_pt - PAGE_MARGIN)).clamp(0.0, 0.5 * f.h);
    f.h -= shift_y;
    geom
}

fn layer_scene(
    project: &Project,
    layer: &Layer,
    geom: &LayerGeometry,
    pds: &[PlotData],
    m: &dyn TextMeasurer,
) -> LayerScene {
    let mut scene = LayerScene { clip: geom.frame, grid: Vec::new(), plots: Vec::new(), overlay: Vec::new() };
    draw_grid(&mut scene.grid, layer, geom);
    for pd in pds {
        draw_plot(&mut scene.plots, pd, geom);
    }
    let out = &mut scene.overlay;
    draw_axis_lines(out, layer, geom.frame);
    draw_ticks(out, layer, geom);
    let x_labels_bottom = draw_x_labels(out, layer, geom, m);
    let y_labels_left = draw_y_labels(out, layer, geom, m);
    draw_titles(out, project, layer, geom, pds, m, x_labels_bottom, y_labels_left);
    draw_legend(out, project, layer, geom, pds, m);
    for label in &layer.labels {
        let f = geom.frame;
        let size = label.font.size_pt;
        let anchor = (f.x + label.x * f.w, f.y + label.y * f.h);
        // The anchor is the vertical middle of the text's first character.
        let (sin, cos) = label.rotation_deg.to_radians().sin_cos();
        let dy = (ASCENT - DESCENT) / 2.0 * size;
        let pos = (anchor.0 + dy * sin, anchor.1 + dy * cos);
        text::place(out, m, &label.text, &label.font, size, pos, Anchor::Start, label.rotation_deg);
    }
    scene
}

// ----- grid, axis lines, ticks ---------------------------------------------------------------

fn grid_stroke(style: &LineStyle) -> Stroke {
    let mut s = line_stroke(style);
    s.cap = Cap::Butt;
    s
}

fn draw_grid(out: &mut Vec<Item>, layer: &Layer, geom: &LayerGeometry) {
    let f = geom.frame;
    let mut lines = |style: &Option<LineStyle>, ticks: &[f64], axis: &ResolvedAxis, vertical: bool| {
        let Some(style) = style else { return };
        let mut d = String::new();
        for &v in ticks {
            let u = axis.to_unit(v);
            if !u.is_finite() {
                continue;
            }
            if vertical {
                let x = f.x + u * f.w;
                move_to(&mut d, x, f.y);
                line_to(&mut d, x, f.y + f.h);
            } else {
                let y = f.y + (1.0 - u) * f.h;
                move_to(&mut d, f.x, y);
                line_to(&mut d, f.x + f.w, y);
            }
        }
        if !d.is_empty() {
            out.push(Item::Path { d, fill: None, stroke: Some(grid_stroke(style)) });
        }
    };
    // Minor first, then major, so that major lines are on top.
    lines(&layer.x_axis.minor_grid, &geom.x.minor, &geom.x, true);
    lines(&layer.y_axis.minor_grid, &geom.y.minor, &geom.y, false);
    lines(&layer.x_axis.major_grid, &geom.x.major, &geom.x, true);
    lines(&layer.y_axis.major_grid, &geom.y.major, &geom.y, false);
}

fn draw_axis_lines(out: &mut Vec<Item>, layer: &Layer, f: Rect) {
    let (xa, ya) = (&layer.x_axis, &layer.y_axis);
    let (l, r, t, b) = (f.x, f.x + f.w, f.y, f.y + f.h);
    let same = xa.line_width_pt == ya.line_width_pt && xa.color == ya.color;
    let square = |a: &Axis| Stroke {
        color: a.color,
        width: a.line_width_pt,
        dash: Vec::new(),
        cap: Cap::Square,
        join: Join::Miter,
    };
    let line = |out: &mut Vec<Item>, x0, y0, x1, y1, a: &Axis| {
        let mut d = String::new();
        move_to(&mut d, x0, y0);
        line_to(&mut d, x1, y1);
        out.push(Item::Path { d, fill: None, stroke: Some(square(a)) });
    };
    if xa.show_opposite && ya.show_opposite && same {
        let mut d = String::new();
        move_to(&mut d, l, t);
        line_to(&mut d, r, t);
        line_to(&mut d, r, b);
        line_to(&mut d, l, b);
        d.push('Z');
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(xa.color, xa.line_width_pt)) });
        return;
    }
    line(out, l, b, r, b, xa);
    line(out, l, t, l, b, ya);
    if xa.show_opposite {
        line(out, l, t, r, t, xa);
    }
    if ya.show_opposite {
        line(out, r, t, r, b, ya);
    }
}

/// The part of a tick on one side of the axis line: `(from, to)` offsets in
/// the direction pointing into the frame (negative is outwards).
fn tick_span(dir: TickDirection, len: f64) -> Option<(f64, f64)> {
    match dir {
        TickDirection::In => Some((0.0, len)),
        TickDirection::Out => Some((0.0, -len)),
        TickDirection::Both => Some((-len, len)),
        TickDirection::None => None,
    }
}

/// How far ticks reach out of the frame on the axis' outer side.
fn outward_extent(axis: &Axis) -> f64 {
    let ext = |dir: TickDirection, len: f64| match dir {
        TickDirection::Out | TickDirection::Both => len.max(0.0),
        _ => 0.0,
    };
    let mut e = ext(axis.major_tick, axis.major_tick_len_pt);
    if axis.minor_ticks > 0 {
        e = e.max(ext(axis.minor_tick, axis.minor_tick_len_pt));
    }
    e
}

fn draw_ticks(out: &mut Vec<Item>, layer: &Layer, geom: &LayerGeometry) {
    let f = geom.frame;
    let stroke = |a: &Axis| Stroke::solid(a.color, a.line_width_pt);
    // Horizontal axis: ticks run vertically. `inward` is +1 when "into the frame" is +y.
    let mut side = |axis: &Axis, resolved: &ResolvedAxis, horizontal: bool, line_pos: f64, inward: f64| {
        let mut d = String::new();
        let sets = [
            (&resolved.major, axis.major_tick, axis.major_tick_len_pt),
            (&resolved.minor, axis.minor_tick, if axis.minor_ticks > 0 { axis.minor_tick_len_pt } else { 0.0 }),
        ];
        for (values, dir, len) in sets {
            let Some((a, b)) = tick_span(dir, len) else { continue };
            for &v in values {
                let u = resolved.to_unit(v);
                if !u.is_finite() {
                    continue;
                }
                if horizontal {
                    let x = f.x + u * f.w;
                    move_to(&mut d, x, line_pos + inward * a);
                    line_to(&mut d, x, line_pos + inward * b);
                } else {
                    let y = f.y + (1.0 - u) * f.h;
                    move_to(&mut d, line_pos + inward * a, y);
                    line_to(&mut d, line_pos + inward * b, y);
                }
            }
        }
        if !d.is_empty() {
            out.push(Item::Path { d, fill: None, stroke: Some(stroke(axis)) });
        }
    };
    let (xa, ya) = (&layer.x_axis, &layer.y_axis);
    side(xa, &geom.x, true, f.y + f.h, -1.0);
    if xa.opposite_ticks {
        side(xa, &geom.x, true, f.y, 1.0);
    }
    side(ya, &geom.y, false, f.x, 1.0);
    if ya.opposite_ticks {
        side(ya, &geom.y, false, f.x + f.w, -1.0);
    }
}

// ----- tick labels and titles ----------------------------------------------------------------

fn label_gap(axis: &Axis) -> f64 {
    0.35 * axis.tick_font.size_pt
}

/// Draws the X tick labels; returns the y of the bottom of the label block
/// (or of the tick ends when there are no labels).
fn draw_x_labels(out: &mut Vec<Item>, layer: &Layer, geom: &LayerGeometry, m: &dyn TextMeasurer) -> f64 {
    let (axis, f) = (&layer.x_axis, geom.frame);
    let tick_end = f.y + f.h + outward_extent(axis);
    if !axis.show_tick_labels {
        return tick_end;
    }
    let size = axis.tick_font.size_pt;
    let top = tick_end + label_gap(axis);
    let baseline = top + ASCENT * size;
    for (v, label) in geom.x.major.iter().zip(&geom.x.labels) {
        let u = geom.x.to_unit(*v);
        if u.is_finite() {
            text::place(out, m, label, &axis.tick_font, size, (f.x + u * f.w, baseline), Anchor::Middle, 0.0);
        }
    }
    top + (ASCENT + DESCENT) * size
}

/// Draws the Y tick labels; returns the x of the left edge of the widest
/// label (or of the tick ends when there are no labels).
fn draw_y_labels(out: &mut Vec<Item>, layer: &Layer, geom: &LayerGeometry, m: &dyn TextMeasurer) -> f64 {
    let (axis, f) = (&layer.y_axis, geom.frame);
    let tick_end = f.x - outward_extent(axis);
    if !axis.show_tick_labels {
        return tick_end;
    }
    let size = axis.tick_font.size_pt;
    let right = tick_end - label_gap(axis);
    let mut left = right;
    for (v, label) in geom.y.major.iter().zip(&geom.y.labels) {
        let u = geom.y.to_unit(*v);
        if !u.is_finite() {
            continue;
        }
        let y = f.y + (1.0 - u) * f.h;
        left = left.min(right - text::width(m, label, &axis.tick_font, size));
        text::place(
            out,
            m,
            label,
            &axis.tick_font,
            size,
            (right, y + (ASCENT - DESCENT) / 2.0 * size),
            Anchor::End,
            0.0,
        );
    }
    left
}

fn first_title(project: &Project, pds: &[PlotData], y_axis: bool) -> String {
    let Some(pd) = pds.first() else { return String::new() };
    if y_axis {
        return project.column(pd.plot.y).map(|c| c.axis_title()).unwrap_or_default();
    }
    match pd.plot.x.filter(|_| pd.x.is_some()).and_then(|id| project.column(id)) {
        Some(c) => c.axis_title(),
        None => "Row".to_string(),
    }
}

fn draw_titles(
    out: &mut Vec<Item>,
    project: &Project,
    layer: &Layer,
    geom: &LayerGeometry,
    pds: &[PlotData],
    m: &dyn TextMeasurer,
    x_labels_bottom: f64,
    y_labels_left: f64,
) {
    let f = geom.frame;
    let (xa, ya) = (&layer.x_axis, &layer.y_axis);
    if xa.show_title {
        let title = xa.title.clone().unwrap_or_else(|| first_title(project, pds, false));
        let size = xa.title_font.size_pt;
        let baseline = x_labels_bottom + 0.6 * size + ASCENT * size;
        text::place(out, m, &title, &xa.title_font, size, (f.x + f.w / 2.0, baseline), Anchor::Middle, 0.0);
    }
    if ya.show_title {
        let title = ya.title.clone().unwrap_or_else(|| first_title(project, pds, true));
        let size = ya.title_font.size_pt;
        // The text's bottom edge (baseline plus descent) faces the axis.
        let x = y_labels_left - 0.6 * size - DESCENT * size;
        text::place(out, m, &title, &ya.title_font, size, (x, f.y + f.h / 2.0), Anchor::Middle, 90.0);
    }
}

// ----- legend --------------------------------------------------------------------------------

fn draw_legend(
    out: &mut Vec<Item>,
    project: &Project,
    layer: &Layer,
    geom: &LayerGeometry,
    pds: &[PlotData],
    m: &dyn TextMeasurer,
) {
    let lg = &layer.legend;
    if !lg.visible || pds.is_empty() {
        return;
    }
    let fs = lg.font.size_pt;
    let f = geom.frame;
    let labels: Vec<String> = pds.iter().map(|pd| project.plot_label(pd.plot)).collect();
    let sample = 2.2 * fs;
    let gap = 0.5 * fs;
    let pad = 0.4 * fs;
    let row_h = 1.3 * fs;
    let text_w = labels.iter().map(|l| text::width(m, l, &lg.font, fs)).fold(0.0, f64::max);
    let w = 2.0 * pad + sample + gap + text_w;
    let h = 2.0 * pad + row_h * pds.len() as f64;

    // Margin from the nearest frame edges; ticks pointing in push the legend further.
    let tick_in =
        |dir: TickDirection, len: f64| if matches!(dir, TickDirection::In | TickDirection::Both) { len } else { 0.0 };
    let (xa, ya) = (&layer.x_axis, &layer.y_axis);
    let m_left = 0.8 * fs + tick_in(ya.major_tick, ya.major_tick_len_pt);
    let m_bottom = 0.8 * fs + tick_in(xa.major_tick, xa.major_tick_len_pt);
    let m_right = 0.8 * fs + if ya.opposite_ticks { tick_in(ya.major_tick, ya.major_tick_len_pt) } else { 0.0 };
    let m_top = 0.8 * fs + if xa.opposite_ticks { tick_in(xa.major_tick, xa.major_tick_len_pt) } else { 0.0 };
    let (x0, y0) = match lg.position {
        LegendPosition::TopLeft => (f.x + m_left, f.y + m_top),
        LegendPosition::TopRight => (f.x + f.w - m_right - w, f.y + m_top),
        LegendPosition::BottomLeft => (f.x + m_left, f.y + f.h - m_bottom - h),
        LegendPosition::BottomRight => (f.x + f.w - m_right - w, f.y + f.h - m_bottom - h),
        LegendPosition::Manual { x, y } => (f.x + x * f.w - w / 2.0, f.y + y * f.h - h / 2.0),
    };
    if lg.background.is_some() || lg.frame {
        let stroke = lg.frame.then(|| Stroke::solid(lg.font.color, 0.75));
        out.push(Item::Rect { rect: Rect { x: x0, y: y0, w, h }, fill: lg.background, stroke });
    }
    for (i, (pd, label)) in pds.iter().zip(&labels).enumerate() {
        let cy = y0 + pad + row_h * (i as f64 + 0.5);
        let sx0 = x0 + pad;
        let sx1 = sx0 + sample;
        let plot = pd.plot;
        if plot.kind.has_line() && plot.line.width_pt > 0.0 {
            let mut d = String::new();
            move_to(&mut d, sx0, cy);
            line_to(&mut d, sx1, cy);
            out.push(Item::Path { d, fill: None, stroke: Some(line_stroke(&plot.line)) });
        }
        if plot.kind.has_symbols() {
            let mut d = String::new();
            symbol_path(&mut d, plot.symbol.shape, (sx0 + sx1) / 2.0, cy, plot.symbol.size_pt);
            out.push(symbol_item(d, &plot.symbol));
        }
        let baseline = cy + (ASCENT - DESCENT) / 2.0 * fs;
        text::place(out, m, label, &lg.font, fs, (sx1 + gap, baseline), Anchor::Start, 0.0);
    }
}
