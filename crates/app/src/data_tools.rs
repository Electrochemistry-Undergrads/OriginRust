//! The graph's Data menu tools that act on the points of a plot: Mask Data Points (drag a
//! rectangle, click a masked point to unmask it), Move Data Points (drag a point) and Remove Bad
//! Data Points (click a point). They pick points with the Data Reader's code ([`reader`]), and
//! they only push an [`Action`]: the worksheet changes are `Document` edits of sheaf-core.

use crate::actions::{Action, DataEdit, DataTool};
use crate::reader::{self, Pick, Point};
use eframe::egui::{self, Color32, CursorIcon, Painter, Pos2, Rect, Response, Stroke};
use sheaf_core::{DataCell, Project, WindowId};
use sheaf_plot::LayerGeometry;

/// A drag in progress.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Drag {
    /// Mask: the corner where the rectangle began (screen).
    Rect(Pos2),
    /// Move: the point being dragged and where the pointer grabbed it, relative to the point's
    /// place (screen). The point follows the pointer by the distance dragged, not by its position.
    Point(Point, egui::Vec2),
}

/// The status-bar hint of a tool.
pub fn hint(tool: DataTool) -> &'static str {
    match tool {
        DataTool::Mask => {
            "Mask Data Points: drag a rectangle over the points; click a masked point to unmask it; Esc ends"
        }
        DataTool::Move => "Move Data Points: drag a point to its new place; Esc ends",
        DataTool::RemoveBad => "Remove Bad Data Points: click a point to make its Y value missing; Esc ends",
    }
}

/// The worksheet cell of the Y value of a point.
fn y_cell(project: &Project, graph: WindowId, p: Point) -> Option<DataCell> {
    let plot = project.graph(graph)?.layers.get(p.layer)?.plots.get(p.plot)?;
    Some(DataCell::new(plot.y, p.row))
}

/// The worksheet cell of the X value of a point; `None` when the plot is drawn against the row number
/// (no X column, or one without numbers).
fn x_cell(project: &Project, graph: WindowId, p: Point) -> Option<DataCell> {
    let plot = project.graph(graph)?.layers.get(p.layer)?.plots.get(p.plot)?;
    let x = plot.x.filter(|x| project.numeric(*x).is_some())?;
    Some(DataCell::new(x, p.row))
}

/// Whether the point is masked as the plot is drawn (Disable Masking switches the masks off).
fn is_masked(project: &Project, graph: WindowId, p: Point) -> bool {
    y_cell(project, graph, p)
        .and_then(|c| project.column(c.column))
        .and_then(|c| c.active_mask())
        .is_some_and(|m| m.contains(&p.row))
}

/// The cells that dropping `p` at screen position `to` writes, in the original values of the plot:
/// the pointer is taken back through the axes of the layer and the plot's offset and multiplier.
/// `None` when the position is not a finite number (outside a log axis).
fn moved(
    project: &Project,
    graph: WindowId,
    geometry: &[LayerGeometry],
    to_page: &dyn Fn(Pos2) -> (f64, f64),
    p: Point,
    to: Pos2,
) -> Option<DataEdit> {
    let y = y_cell(project, graph, p)?;
    let x = x_cell(project, graph, p);
    let (px, py) = to_page(to);
    // A waterfall member is drawn moved from the layer's frame; take the pointer back by its shift.
    let (sx, sy) = reader::shift(project, graph, geometry, p);
    let (dx, dy) = geometry.get(p.layer)?.page_to_data(px - sx, py - sy);
    let place = reader::placement(project, graph, p.layer, p.plot);
    let (xv, yv) = (place.source_x(dx), place.source_y(dy));
    if !yv.is_finite() || (x.is_some() && !xv.is_finite()) {
        return None;
    }
    Some(DataEdit::Move { y: (y, yv), x: x.map(|c| (c, xv)) })
}

fn ring(painter: &Painter, at: Pos2, color: Color32) {
    painter.circle_stroke(at, 6.0, Stroke::new(1.5_f32, color));
}

const BLUE: Color32 = Color32::from_rgb(0, 90, 200);

/// What the tools need of the graph window as drawn in this frame.
#[derive(Clone, Copy)]
pub struct View<'a> {
    pub project: &'a Project,
    pub graph: WindowId,
    pub geometry: &'a [LayerGeometry],
    pub to_screen: &'a dyn Fn((f64, f64)) -> Pos2,
    pub to_page: &'a dyn Fn(Pos2) -> (f64, f64),
}

/// Runs a tool for one frame: the cursor, the hover ring, the rectangle or the point being dragged,
/// and the action when the button is let go. `drag` is the tool's state between frames.
pub fn show(
    ui: &egui::Ui,
    painter: &Painter,
    response: &Response,
    view: &View<'_>,
    tool: DataTool,
    drag: &mut Option<Drag>,
    actions: &mut Vec<Action>,
) {
    let View { project, graph, geometry, to_screen, to_page } = *view;
    // Esc ends the mode (the caller does that); egui also ends a drag when it is pressed, which must
    // not write anything.
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        *drag = None;
        return;
    }
    ui.ctx().set_cursor_icon(if tool == DataTool::Move { CursorIcon::Move } else { CursorIcon::Crosshair });
    let near = |at: Pos2| match tool {
        // A click unmasks a masked point, so the nearest masked one is the target.
        DataTool::Mask => {
            reader::nearest_where(project, graph, geometry, to_screen, Pick::Edit, at, |p| is_masked(project, graph, p))
        }
        _ => reader::nearest_of(project, graph, geometry, to_screen, Pick::Edit, at),
    };
    let screen_of = |p: Point| reader::place(project, graph, geometry, to_screen, p).map(|(at, _, _)| at);

    if response.drag_started()
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
    {
        *drag = match tool {
            DataTool::Mask => Some(Drag::Rect(origin)),
            DataTool::Move => {
                near(origin).map(|p| Drag::Point(p, screen_of(p).map_or(egui::Vec2::ZERO, |at| origin - at)))
            }
            DataTool::RemoveBad => None,
        };
    }
    let pointer = response.interact_pointer_pos();
    match (*drag, pointer) {
        (Some(Drag::Rect(from)), Some(to)) => {
            let area = Rect::from_two_pos(from, to);
            painter.rect_filled(area, 0.0, Color32::from_rgba_unmultiplied(0, 90, 200, 40));
            painter.rect_stroke(area, 0.0, Stroke::new(1.0_f32, BLUE), egui::StrokeKind::Inside);
            let picked = reader::inside(project, graph, geometry, to_screen, Pick::Edit, area);
            for (at, _) in &picked {
                ring(painter, *at, BLUE);
            }
            if response.drag_stopped() {
                *drag = None;
                let cells: Vec<DataCell> = picked.iter().filter_map(|(_, p)| y_cell(project, graph, *p)).collect();
                if !cells.is_empty() {
                    actions.push(Action::DataEdit(DataEdit::Mask { cells, masked: true }));
                }
            }
        }
        (Some(Drag::Point(p, grab)), Some(pointer)) => {
            // The point is dropped where the pointer is, less the offset at which it was grabbed.
            let to = pointer - grab;
            let from = screen_of(p);
            if let Some(from) = from {
                painter.line_segment([from, to], Stroke::new(1.0_f32, BLUE));
                ring(painter, from, Color32::from_gray(140));
            }
            ring(painter, to, BLUE);
            if response.drag_stopped() {
                *drag = None;
                // A drag back to where it began (within half a pixel) writes nothing.
                if from.is_none_or(|from| from.distance(to) >= 0.5) {
                    actions.extend(moved(project, graph, geometry, to_page, p, to).map(Action::DataEdit));
                }
            }
        }
        _ => {
            if response.drag_stopped() {
                *drag = None;
            }
            if let Some(at) = response.hover_pos().and_then(near).and_then(screen_of) {
                ring(painter, at, BLUE);
            }
        }
    }

    if response.clicked()
        && let Some(at) = pointer
        && let Some(p) = near(at)
        && let Some(cell) = y_cell(project, graph, p)
    {
        match tool {
            DataTool::Mask => actions.push(Action::DataEdit(DataEdit::Mask { cells: vec![cell], masked: false })),
            DataTool::RemoveBad => actions.push(Action::DataEdit(DataEdit::RemoveBad { cells: vec![cell] })),
            DataTool::Move => {}
        }
    }
}
