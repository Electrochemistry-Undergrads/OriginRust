//! The Data Reader tool of a graph window (Origin's Tools toolbar): while it is on, the pointer
//! snaps to the nearest data point of any shown plot, a click pins that point, the Left and Right
//! arrow keys step through the rows of the pinned plot, and a label shows the plot, the row and
//! the X and Y values. Esc, or the Pointer button, switches back to the pointer.

use crate::gadget::plot_xy;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use sheaf_core::graph::PlotKind;
use sheaf_core::stack::Place;
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

/// How far from a point the Data menu tools still pick it, in screen points: Origin 2016 picked up
/// to 8 to 9 pixels from the default symbol and 14 at symbol size 24 (spec 20.4, M7), about half
/// the symbol plus a margin and not below 9.
fn edit_reach(project: &Project, graph: WindowId, p: Point) -> f32 {
    let size = project
        .graph(graph)
        .and_then(|g| g.layers.get(p.layer))
        .and_then(|l| l.plots.get(p.plot))
        .map_or(0.0, |plot| plot.symbol.size_pt);
    (size as f32 / 2.0 + 2.0).max(9.0)
}

/// How far outside the frame, in page points, a point still counts as drawn (a symbol on the axis
/// limit sits on the frame edge).
const CLIP_TOLERANCE: f64 = 0.5;

/// Where a data point is on the screen, and its values.
pub fn place(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    p: Point,
) -> Option<(Pos2, f64, f64)> {
    let plot = project.graph(graph)?.layers.get(p.layer)?.plots.get(p.plot)?;
    let filters = project.plot_row_filters(plot);
    if !plot.visible || !filters.iter().all(|f| f.passes(p.row)) {
        return None;
    }
    let (x, y) = plot_xy(project, graph, p.layer, p.plot)?;
    let (start, end) = sheaf_core::stack::row_range(plot, x.len().min(y.len()));
    if !(start..end).contains(&p.row) {
        return None;
    }
    let (xv, yv) = (*x.get(p.row)?, *y.get(p.row)?);
    if !(xv.is_finite() && yv.is_finite()) {
        return None;
    }
    let geom = geometry.get(p.layer)?;
    let placement = placement(project, graph, p.layer, p.plot);
    let shift = sheaf_plot::waterfall_shift(project, graph, p.layer, &placement, geom);
    let at = to_screen(page_of(geom, &placement, shift, xv, yv));
    (at.x.is_finite() && at.y.is_finite()).then_some((at, xv, yv))
}

/// Where a value of a plot is on the page: the plot's place in the stack, the layer's axes and the
/// plot's waterfall shift.
fn page_of(geom: &LayerGeometry, place: &Place, shift: (f64, f64), xv: f64, yv: f64) -> (f64, f64) {
    let (px, py) = geom.data_to_page(place.x(xv), place.y(yv));
    (px + shift.0, py + shift.1)
}

/// How far the plot of a point is moved on the page from its layer's frame: the waterfall shift of
/// its member, `(0, 0)` outside a waterfall.
pub fn shift(project: &Project, graph: WindowId, geometry: &[LayerGeometry], p: Point) -> (f64, f64) {
    let Some(geom) = geometry.get(p.layer) else { return (0.0, 0.0) };
    sheaf_plot::waterfall_shift(project, graph, p.layer, &placement(project, graph, p.layer, p.plot), geom)
}

pub fn placement(project: &Project, graph: WindowId, layer: usize, plot: usize) -> Place {
    sheaf_plot::stack_places(project, graph, layer).ok().and_then(|o| o.get(plot).copied()).unwrap_or_default()
}

/// Which points of a graph a tool can pick.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// The Data Reader: every shown point.
    Reader,
    /// The Data menu tools (Mask, Move, Remove Bad): the points the graph draws, of line, scatter and
    /// line + symbol plots of rectangular layers. Not picked: layers with Show Elements Data off,
    /// rows that Skip Points leaves without a symbol, masked points that Hide Masked Points does not
    /// draw, and points the frame clips (outside the axis range).
    Edit,
}

/// Calls `f` with the screen position and the point of every point of the graph that `pick` allows.
fn each_point(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    pick: Pick,
    mut f: impl FnMut(Pos2, Point),
) {
    let Some(g) = project.graph(graph) else { return };
    for (li, layer) in g.layers.iter().enumerate() {
        let Some(geom) = geometry.get(li) else { continue };
        if pick == Pick::Edit && (geom.polar.is_some() || geom.ternary.is_some() || !layer.display.show_data) {
            continue;
        }
        // Data outside the clip rectangle is not drawn; with Clip Data to Frame off it is.
        let clip = geom.clip;
        let clipped = |(px, py): (f64, f64)| {
            pick == Pick::Edit
                && layer.display.clip_data
                && !(px >= clip.x - CLIP_TOLERANCE
                    && px <= clip.x + clip.w + CLIP_TOLERANCE
                    && py >= clip.y - CLIP_TOLERANCE
                    && py <= clip.y + clip.h + CLIP_TOLERANCE)
        };
        for (pi, plot) in layer.plots.iter().enumerate() {
            if !plot.visible {
                continue;
            }
            if pick == Pick::Edit && !matches!(plot.kind, PlotKind::Line | PlotKind::Scatter | PlotKind::LineSymbol) {
                continue;
            }
            let Some((x, y)) = plot_xy(project, graph, li, pi) else { continue };
            let filters = project.plot_row_filters(plot);
            let (start, end) = sheaf_core::stack::row_range(plot, x.len().min(y.len()));
            let placement = placement(project, graph, li, pi);
            let shift = sheaf_plot::waterfall_shift(project, graph, li, &placement, geom);
            let hidden = if pick == Pick::Edit && project.hide_masked {
                project.column(plot.y).and_then(|c| c.active_mask())
            } else {
                None
            };
            // Skip Points draws a symbol on every n-th shown row (missing values count), from the first.
            let every = if pick == Pick::Edit && plot.kind.supports_skip_points() {
                plot.symbol.every.max(1) as usize
            } else {
                1
            };
            let mut shown = 0_usize;
            for (row, (xv, yv)) in x.iter().zip(&y).enumerate().take(end).skip(start) {
                if !filters.iter().all(|f| f.passes(row)) {
                    continue;
                }
                shown += 1;
                if !(shown - 1).is_multiple_of(every)
                    || !(xv.is_finite() && yv.is_finite())
                    || hidden.is_some_and(|m| m.contains(&row))
                {
                    continue;
                }
                let page = page_of(geom, &placement, shift, *xv, *yv);
                let at = to_screen(page);
                if at.x.is_finite() && at.y.is_finite() && !clipped(page) {
                    f(at, Point { layer: li, plot: pi, row });
                }
            }
        }
    }
}

/// The data point of a shown plot nearest to `pointer`, if one is within reach.
pub fn nearest(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    pointer: Pos2,
) -> Option<Point> {
    nearest_of(project, graph, geometry, to_screen, Pick::Reader, pointer)
}

/// Like [`nearest`] for the points that `pick` allows.
pub fn nearest_of(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    pick: Pick,
    pointer: Pos2,
) -> Option<Point> {
    nearest_where(project, graph, geometry, to_screen, pick, pointer, false, |_| true)
}

/// The nearest point within reach among those that `pick` allows and `keep` accepts. With
/// `measured_reach` the reach is the one measured for Origin's Mask click ([`edit_reach`]); the
/// other callers keep [`SNAP`] (Move's grab and Remove Bad's click were not measured).
pub fn nearest_where(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    pick: Pick,
    pointer: Pos2,
    measured_reach: bool,
    keep: impl Fn(Point) -> bool,
) -> Option<Point> {
    let mut best: Option<(f32, Point)> = None;
    each_point(project, graph, geometry, to_screen, pick, |at, p| {
        let d = at.distance(pointer);
        let reach = if measured_reach { edit_reach(project, graph, p) } else { SNAP };
        if d <= reach && best.is_none_or(|(b, _)| d < b) && keep(p) {
            best = Some((d, p));
        }
    });
    best.map(|(_, p)| p)
}

/// Where a data point is on the screen (`None` when it is not shown or not a number).
pub fn screen_point(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    p: Point,
) -> Option<Pos2> {
    place(project, graph, geometry, to_screen, p).map(|(at, _, _)| at)
}

/// Like [`nearest`], limited to the plot `only` (layer and plot index) when given: the Slope tool's
/// second click.
pub fn nearest_in(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    pointer: Pos2,
    only: Option<(usize, usize)>,
) -> Option<Point> {
    nearest_where(project, graph, geometry, to_screen, Pick::Reader, pointer, false, |p| {
        only.is_none_or(|o| o == (p.layer, p.plot))
    })
}

/// The points that `pick` allows whose screen position is inside `area` (edges included), with
/// their positions.
pub fn inside(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    pick: Pick,
    area: Rect,
) -> Vec<(Pos2, Point)> {
    let mut found = Vec::new();
    each_point(project, graph, geometry, to_screen, pick, |at, p| {
        if area.contains(at) {
            found.push((at, p));
        }
    });
    found
}

/// The next row of the pinned plot in direction `step` (+1 or -1) that has a value.
pub fn step(project: &Project, graph: WindowId, p: Point, step: isize) -> Point {
    if step == 0 {
        return p;
    }
    let Some(plot) = project.graph(graph).and_then(|g| g.layers.get(p.layer)).and_then(|l| l.plots.get(p.plot)) else {
        return p;
    };
    let Some((x, y)) = plot_xy(project, graph, p.layer, p.plot) else { return p };
    let filters = project.plot_row_filters(plot);
    let (start, end) = sheaf_core::stack::row_range(plot, x.len().min(y.len()));
    let mut row = p.row as isize;
    loop {
        row += step;
        if row < start as isize || row as usize >= end {
            return p;
        }
        let r = row as usize;
        if filters.iter().all(|f| f.passes(r)) && x[r].is_finite() && y[r].is_finite() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation, template::Template};

    #[test]
    fn reader_follows_scaled_points_but_returns_original_values() {
        let mut p = Project::new();
        let b = p.add_workbook("Data");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 6.0])).unwrap();
        let g = p.add_graph(Template::Scatter);
        p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        layer.stack = sheaf_core::graph::Stack::Individual;
        layer.x_axis.min = Some(0.0);
        layer.x_axis.max = Some(20.0);
        layer.y_axis.min = Some(0.0);
        layer.y_axis.max = Some(30.0);
        let o = layer.plot_offset_mut(y);
        (o.x, o.y, o.x_multiplier, o.y_multiplier) = (5.0, 7.0, 2.0, 3.0);
        let geometry = sheaf_plot::layout_geometry(&p, g).unwrap();
        let to_screen = |(x, y)| egui::pos2(x as f32, y as f32);
        let point = Point { layer: 0, plot: 0, row: 1 };
        let at = to_screen(geometry[0].data_to_page(9.0, 19.0));
        assert_eq!(nearest(&p, g, &geometry, &to_screen, at), Some(point));
        let (marker, xv, yv) = place(&p, g, &geometry, &to_screen, point).unwrap();
        assert_eq!((marker, xv, yv), (at, 2.0, 4.0));
        let unscaled = to_screen(geometry[0].data_to_page(2.0, 4.0));
        assert!(nearest(&p, g, &geometry, &to_screen, unscaled).is_none());
        assert_eq!(step(&p, g, point, 1), Point { row: 2, ..point });
    }

    #[test]
    fn filtered_rows_cannot_be_picked_drawn_or_stepped_to_by_the_data_reader() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 100.0, 200.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 500.0, 0.0])).unwrap();
        let graph = p.add_graph(Template::Scatter);
        p.add_plot(graph, 0, y).unwrap();
        p.set_row_filter(book, 0, "col(A) != 100").unwrap();
        let geometry = sheaf_plot::layout_geometry(&p, graph).unwrap();
        let to_screen = |(x, y)| egui::pos2(x as f32, y as f32);
        let a = Point { layer: 0, plot: 0, row: 0 };
        let hidden = Point { row: 1, ..a };
        let last = Point { row: 2, ..a };
        assert!(place(&p, graph, &geometry, &to_screen, hidden).is_none());
        let at = to_screen(geometry[0].data_to_page(100.0, 500.0));
        assert!(nearest(&p, graph, &geometry, &to_screen, at).is_none());
        assert_eq!(step(&p, graph, a, 1), last);
        assert_eq!(step(&p, graph, last, -1), a);
        assert_eq!(step(&p, graph, hidden, 0), hidden);
        p.workbook_mut(book).unwrap().sheet_mut().filter.as_mut().unwrap().enabled = false;
        assert_eq!(step(&p, graph, a, 1), hidden);
        p.graph_mut(graph).unwrap().layers[0].plots[0].rows = Some((1, 3));
        assert!(place(&p, graph, &geometry, &to_screen, a).is_none());
        assert_eq!(step(&p, graph, hidden, -1), hidden, "the reader stays within the plot range");
    }
}
