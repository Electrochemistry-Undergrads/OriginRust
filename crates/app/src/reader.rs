//! The Data Reader tool of a graph window (Origin's Tools toolbar): while it is on, the pointer
//! snaps to the nearest data point of any shown plot, a click pins that point, the Left and Right
//! arrow keys step through the rows of the pinned plot, and a label shows the plot, the row and
//! the X and Y values. Esc, or the Pointer button, switches back to the pointer.

use crate::gadget::plot_xy;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use sheaf_core::{Project, WindowId};
use sheaf_plot::LayerGeometry;

/// A data point of a graph: layer, plot and row (counted from 0).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub layer: usize,
    pub plot: usize,
    pub row: usize,
}

/// How far from the pointer, in screen points, a data point is still picked.
const SNAP: f32 = 30.0;

/// Where a data point is on the screen, and its values.
fn place(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    p: Point,
) -> Option<(Pos2, f64, f64)> {
    let (x, y) = plot_xy(project, graph, p.layer, p.plot)?;
    let (xv, yv) = (*x.get(p.row)?, *y.get(p.row)?);
    if !(xv.is_finite() && yv.is_finite()) {
        return None;
    }
    let (ox, oy) = offset(project, graph, p.layer, p.plot);
    let at = to_screen(geometry.get(p.layer)?.data_to_page(xv + ox, yv + oy));
    (at.x.is_finite() && at.y.is_finite()).then_some((at, xv, yv))
}

fn offset(project: &Project, graph: WindowId, layer: usize, plot: usize) -> (f64, f64) {
    sheaf_plot::stack_offsets(project, graph, layer).ok().and_then(|o| o.get(plot).copied()).unwrap_or((0.0, 0.0))
}

/// The data point of a shown plot nearest to `pointer`, if one is within reach.
pub fn nearest(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    pointer: Pos2,
) -> Option<Point> {
    let g = project.graph(graph)?;
    let mut best: Option<(f32, Point)> = None;
    for (li, layer) in g.layers.iter().enumerate() {
        let Some(geom) = geometry.get(li) else { continue };
        for (pi, plot) in layer.plots.iter().enumerate() {
            if !plot.visible {
                continue;
            }
            let Some((x, y)) = plot_xy(project, graph, li, pi) else { continue };
            let (ox, oy) = offset(project, graph, li, pi);
            for (row, (xv, yv)) in x.iter().zip(&y).enumerate() {
                if !(xv.is_finite() && yv.is_finite()) {
                    continue;
                }
                let at = to_screen(geom.data_to_page(xv + ox, yv + oy));
                let d = at.distance(pointer);
                if d <= SNAP && best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, Point { layer: li, plot: pi, row }));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// The next row of the pinned plot in direction `step` (+1 or -1) that has a value.
pub fn step(project: &Project, graph: WindowId, p: Point, step: isize) -> Point {
    let Some((x, y)) = plot_xy(project, graph, p.layer, p.plot) else { return p };
    let mut row = p.row as isize;
    loop {
        row += step;
        if row < 0 || row as usize >= x.len().min(y.len()) {
            return p;
        }
        let r = row as usize;
        if x[r].is_finite() && y[r].is_finite() {
            return Point { row: r, ..p };
        }
    }
}

/// Draws the cross hair over the point's layer frame, the marker and the label of a data point.
pub fn draw(
    painter: &egui::Painter,
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    p: Point,
    pinned: bool,
) {
    let Some((at, xv, yv)) = place(project, graph, geometry, to_screen, p) else { return };
    let Some(geom) = geometry.get(p.layer) else { return };
    let f = geom.frame;
    let frame = Rect::from_min_max(to_screen((f.x, f.y)), to_screen((f.x + f.w, f.y + f.h)));
    let hair = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 90, 200, if pinned { 200 } else { 110 }));
    painter.line_segment([egui::pos2(frame.left(), at.y), egui::pos2(frame.right(), at.y)], hair);
    painter.line_segment([egui::pos2(at.x, frame.top()), egui::pos2(at.x, frame.bottom())], hair);
    let marker = Rect::from_center_size(at, egui::vec2(7.0, 7.0));
    painter.rect_stroke(marker, 0.0, Stroke::new(1.5_f32, Color32::from_rgb(0, 90, 200)), egui::StrokeKind::Middle);

    let name = project
        .graph(graph)
        .and_then(|g| g.layers.get(p.layer))
        .and_then(|l| l.plots.get(p.plot))
        .map(|pl| project.plot_label(pl))
        .unwrap_or_default();
    let fmt = |v: f64| sheaf_core::format::format_general(v, 6);
    let text = format!("{name} [{}]\nX = {}\nY = {}", p.row + 1, fmt(xv), fmt(yv));
    let font = egui::FontId::proportional(12.0);
    let galley = painter.layout_no_wrap(text, font, Color32::BLACK);
    // Right of and above the point, kept inside the frame.
    let size = galley.size() + egui::vec2(8.0, 6.0);
    let mut min = at + egui::vec2(10.0, -10.0 - size.y);
    min.x = min.x.min(frame.right() - size.x).max(frame.left());
    min.y = min.y.max(frame.top()).min(frame.bottom() - size.y);
    let label = Rect::from_min_size(min, size);
    painter.rect_filled(label, 0.0, Color32::from_rgba_unmultiplied(255, 255, 225, 235));
    painter.rect_stroke(label, 0.0, Stroke::new(1.0_f32, Color32::from_gray(120)), egui::StrokeKind::Inside);
    painter.galley(label.min + egui::vec2(4.0, 3.0), galley, Color32::BLACK);
}
