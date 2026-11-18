//! The graph's Data menu tools that act on the points of a plot: Mask Data Points (drag a
//! rectangle or, with Space, a freehand path; click an unmasked point to mask it; Tab limits it to the
//! active plot; U switches to unmasking), Move Data Points (drag a point) and Remove Bad Data Points
//! (click or Left/Right pick a point, Enter removes it), as measured in Origin 2016 (graphing spec 20.4). They pick points with the Data Reader's code ([`reader`]), and
//! they only push an [`Action`]: the worksheet changes are `Document` edits of sheaf-core.

use crate::actions::{Action, DataEdit, DataTool};
use crate::reader::{self, Pick, Point};
use eframe::egui::{self, Color32, CursorIcon, Painter, Pos2, Rect, Response, Stroke};
use sheaf_core::{DataCell, Project, WindowId};
use sheaf_plot::LayerGeometry;

/// A drag in progress.
#[derive(Clone, Debug, PartialEq)]
pub enum Drag {
    /// Mask: the corner where the rectangle began (screen).
    Rect(Pos2),
    /// Mask with the freehand shape: the path so far (screen), closed by the last point.
    Path(Vec<Pos2>),
    /// Move: the point being dragged and where the pointer grabbed it, relative to the point's
    /// place (screen). The point follows the pointer by the distance dragged, not by its position.
    Point(Point, egui::Vec2),
}

/// What a Data menu tool keeps between frames.
#[derive(Clone, Debug, Default)]
pub struct State {
    /// The drag in progress.
    pub drag: Option<Drag>,
    /// Mask: Tab limits the tool to the active plot (Origin's default is all plots of the layer).
    pub active_plot_only: bool,
    /// Mask: U switches the tool to unmasking (Origin has this only on the Tools toolbar's mask
    /// flyout; Sheaf has no such button).
    pub unmask: bool,
    /// Mask: Space switches the rectangle to a freehand closed path.
    pub freehand: bool,
    /// The active plot (layer, plot): the first one until a click picks a point of another.
    pub active: Option<(usize, usize)>,
    /// Remove Bad: the point a click or the Left/Right keys picked; Enter removes it.
    pub picked: Option<Point>,
}

impl State {
    fn active_plot(&self) -> (usize, usize) {
        self.active.unwrap_or((0, 0))
    }

    /// The mask tool's words for the on-screen label.
    pub fn mask_label(&self) -> String {
        format!(
            "{} {} on {}",
            if self.unmask { "Unmask" } else { "Mask" },
            if self.freehand { "freehand" } else { "rectangle" },
            if self.active_plot_only { "the active plot" } else { "all plots" }
        )
    }
}

/// Whether `at` is inside the closed path (even-odd rule).
fn in_path(path: &[Pos2], at: Pos2) -> bool {
    let mut inside = false;
    let mut j = path.len().wrapping_sub(1);
    for i in 0..path.len() {
        let (a, b) = (path[i], path[j]);
        if (a.y > at.y) != (b.y > at.y) && at.x < (b.x - a.x) * (at.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// The status-bar hint of a tool.
pub fn hint(tool: DataTool) -> &'static str {
    match tool {
        DataTool::Mask => {
            "Mask Data Points: drag a rectangle over the points or click one; Tab: active plot, Space: freehand, U: unmask; Esc or right-click ends"
        }
        DataTool::Move => "Move Data Points: drag a point to its new place; Esc or right-click ends",
        DataTool::MarkRange => {
            "Mark Data Range: click a point to move the nearer marker there; Data > Set Display Range plots the rows between the markers; Esc or right-click ends"
        }
        DataTool::RemoveBad => {
            "Remove Bad Data Points: click a point (or Left/Right), Enter makes its Y value missing; Esc or right-click ends"
        }
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
    // Cumulative stacking draws each row above the plots before it; take that share back too.
    let lift = reader::cumulative_add(project, graph, p.layer, p.plot).and_then(|a| a.get(p.row).copied());
    let (xv, yv) = (place.source_x(dx), place.source_y(dy) - lift.unwrap_or(0.0));
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

/// Runs a tool for one frame: the cursor, the rectangle, path or point being dragged, the picked
/// point, and the action when the button is let go (or Enter is pressed). `state` is the tool's
/// state between frames.
pub fn show(
    ui: &egui::Ui,
    painter: &Painter,
    response: &Response,
    view: &View<'_>,
    tool: DataTool,
    state: &mut State,
    actions: &mut Vec<Action>,
) {
    let View { project, graph, geometry, to_screen, to_page } = *view;
    // Esc ends the mode (the caller does that); egui also ends a drag when it is pressed, which must
    // not write anything.
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.drag = None;
        return;
    }
    ui.ctx().set_cursor_icon(if tool == DataTool::Move { CursorIcon::Move } else { CursorIcon::Crosshair });
    if tool == DataTool::Mask {
        let pressed = |key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
        if pressed(egui::Key::Tab) {
            state.active_plot_only = !state.active_plot_only;
        }
        if pressed(egui::Key::Space) {
            state.freehand = !state.freehand;
            state.drag = None;
        }
        if pressed(egui::Key::U) {
            state.unmask = !state.unmask;
        }
        painter.text(
            response.rect.left_top() + egui::vec2(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            state.mask_label(),
            egui::FontId::proportional(12.0),
            BLUE,
        );
    }
    let active = state.active_plot();
    let only_active = tool == DataTool::Mask && state.active_plot_only;
    let allowed = |p: Point| !only_active || (p.layer, p.plot) == active;
    let unmask = state.unmask;
    let near = |at: Pos2| match tool {
        // Mask masks an unmasked point (a masked one stays); Unmask unmasks a masked one.
        DataTool::Mask => reader::nearest_where(project, graph, geometry, to_screen, Pick::Edit, at, true, |p| {
            allowed(p) && is_masked(project, graph, p) == unmask
        }),
        _ => reader::nearest_of(project, graph, geometry, to_screen, Pick::Edit, at),
    };
    let screen_of = |p: Point| reader::place(project, graph, geometry, to_screen, p).map(|(at, _, _)| at);

    if response.drag_started()
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
    {
        state.drag = match tool {
            DataTool::Mask if state.freehand => Some(Drag::Path(vec![origin])),
            DataTool::Mask => Some(Drag::Rect(origin)),
            DataTool::Move => {
                near(origin).map(|p| Drag::Point(p, screen_of(p).map_or(egui::Vec2::ZERO, |at| origin - at)))
            }
            DataTool::RemoveBad | DataTool::MarkRange => None,
        };
    }
    let pointer = response.interact_pointer_pos();
    if let (Some(Drag::Path(path)), Some(to)) = (&mut state.drag, pointer)
        && path.last() != Some(&to)
    {
        path.push(to);
    }
    let finish_mask = |picked: &[(Pos2, Point)], actions: &mut Vec<Action>| {
        let cells: Vec<DataCell> =
            picked.iter().filter(|(_, p)| allowed(*p)).filter_map(|(_, p)| y_cell(project, graph, *p)).collect();
        if !cells.is_empty() {
            actions.push(Action::DataEdit(DataEdit::Mask { cells, masked: !unmask }));
        }
    };
    match (state.drag.clone(), pointer) {
        (Some(Drag::Rect(from)), Some(to)) => {
            let area = Rect::from_two_pos(from, to);
            painter.rect_filled(area, 0.0, Color32::from_rgba_unmultiplied(0, 90, 200, 40));
            painter.rect_stroke(area, 0.0, Stroke::new(1.0_f32, BLUE), egui::StrokeKind::Inside);
            let picked = reader::inside(project, graph, geometry, to_screen, Pick::Edit, area);
            for (at, p) in &picked {
                if allowed(*p) {
                    ring(painter, *at, BLUE);
                }
            }
            if response.drag_stopped() {
                state.drag = None;
                finish_mask(&picked, actions);
            }
        }
        (Some(Drag::Path(path)), Some(_)) => {
            painter.add(egui::Shape::closed_line(path.clone(), Stroke::new(1.0_f32, BLUE)));
            let bounds = Rect::from_points(&path);
            let picked: Vec<(Pos2, Point)> = reader::inside(project, graph, geometry, to_screen, Pick::Edit, bounds)
                .into_iter()
                .filter(|(at, _)| in_path(&path, *at))
                .collect();
            for (at, p) in &picked {
                if allowed(*p) {
                    ring(painter, *at, BLUE);
                }
            }
            if response.drag_stopped() {
                state.drag = None;
                finish_mask(&picked, actions);
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
                state.drag = None;
                // A drag back to where it began (within half a pixel) writes nothing.
                if from.is_none_or(|from| from.distance(to) >= 0.5) {
                    actions.extend(moved(project, graph, geometry, to_page, p, to).map(Action::DataEdit));
                }
            }
        }
        _ => {
            if response.drag_stopped() {
                state.drag = None;
            }
        }
    }

    if response.clicked()
        && let Some(at) = pointer
        && let Some(p) = near(at)
    {
        state.active = Some((p.layer, p.plot));
        match tool {
            DataTool::Mask => {
                if let Some(cell) = y_cell(project, graph, p) {
                    actions.push(Action::DataEdit(DataEdit::Mask { cells: vec![cell], masked: !unmask }));
                }
            }
            // A click only picks the point (as the Data Reader does); Enter removes it.
            DataTool::RemoveBad => state.picked = Some(p),
            DataTool::Move => {}
            // The nearer marker of the clicked point's plot moves to its row.
            DataTool::MarkRange => {
                let markers = project
                    .graph(graph)
                    .and_then(|g| g.layers.get(p.layer))
                    .and_then(|l| l.plots.get(p.plot))
                    .and_then(|pl| pl.decor.markers);
                let (a, b) = match markers {
                    Some((lo, hi)) if p.row.abs_diff(lo) <= p.row.abs_diff(hi) => (p.row, hi),
                    Some((lo, _)) => (lo, p.row),
                    None => (p.row, p.row),
                };
                let op = crate::actions::MarkerOp::Move { plot: p.plot, a, b };
                actions
                    .push(Action::GraphEdit { graph, op: crate::actions::GraphOp::DataMarkers { layer: p.layer, op } });
            }
        }
    }

    if tool == DataTool::RemoveBad
        && let Some(p) = state.picked
    {
        let step =
            ui.input(|i| i.key_pressed(egui::Key::ArrowRight) as isize - i.key_pressed(egui::Key::ArrowLeft) as isize);
        let p = if step == 0 { p } else { reader::step(project, graph, p, step) };
        state.picked = Some(p);
        if let Some(at) = screen_of(p) {
            painter.circle_stroke(at, 7.0, Stroke::new(2.0_f32, BLUE));
        }
        if ui.input(|i| i.key_pressed(egui::Key::Enter))
            && let Some(cell) = y_cell(project, graph, p)
        {
            actions.push(Action::DataEdit(DataEdit::RemoveBad { cells: vec![cell] }));
            state.picked = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_is_inside_a_closed_path_by_the_even_odd_rule() {
        let tri = [Pos2::new(0.0, 0.0), Pos2::new(10.0, 0.0), Pos2::new(0.0, 10.0)];
        assert!(in_path(&tri, Pos2::new(2.0, 2.0)));
        assert!(!in_path(&tri, Pos2::new(8.0, 8.0)));
        assert!(!in_path(&tri, Pos2::new(-1.0, 2.0)));
        assert!(!in_path(&[], Pos2::new(0.0, 0.0)));
    }
}
