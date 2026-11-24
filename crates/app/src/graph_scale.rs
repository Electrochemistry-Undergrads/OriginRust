//! The Tools-bar Scale In / Scale Out tools and View > Zoom All (T69; behaviour measured in Origin
//! 2016, graphing spec section 23; clean-room).
//!
//! - Scale In: a one-shot rectangle tool. The active layer's x and y ends become the rectangle's
//!   edges, through the axis's own scale (log axes in log10). A linked axis changes in the layer it
//!   follows, so the layers linked to it follow. One undo step "Zoom In/Out".
//! - Scale Out: a one-shot tool whose click or drag takes one step off the per-window stack of the
//!   previous scales of the active layer.
//! - Zoom All: a screen-only zoom that fits the page and all objects into the window.

use crate::actions::Action;
use crate::graph_zoom::{self, Zoom};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use sheaf_core::graph::{Axis, Graph, ScaleLink};
use sheaf_core::{Document, WindowId};
use sheaf_plot::LayerGeometry;
use sheaf_plot::scene::Scene;

/// The undo label of both tools (Origin's Edit menu reads "Undo Zoom In/Out").
pub const UNDO_LABEL: &str = "Zoom In/Out";
/// A drag shorter than this on either side, in UI points, is a click: it changes nothing.
const MIN_DRAG: f32 = 4.0;
/// The grey kept round the objects by Zoom All, in UI points.
const ZOOM_ALL_MARGIN: f32 = 10.0;

/// Which of the two magnifier tools is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleTool {
    In,
    Out,
}

/// The fixed ends of one axis: `None` is automatic.
pub type Ends = (Option<f64>, Option<f64>);

/// One axis written by a Scale In: the layer that owns it (the parent of a linked axis), the values
/// before, and the ends written.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Change {
    layer: usize,
    y: bool,
    before: Ends,
    after: (f64, f64),
}

/// One Scale In of the layer `active`.
#[derive(Clone, Debug, PartialEq)]
struct Step {
    active: usize,
    changes: Vec<Change>,
}

/// The state of the two tools in one graph window: the tool, the drag, and the Scale Out stack.
#[derive(Default)]
pub struct ScaleState {
    pub tool: Option<ScaleTool>,
    start: Option<Pos2>,
    stack: Vec<Step>,
}

/// The new x ends and y ends (data units, left to right and bottom to top) for a rectangle on the
/// page given by two opposite corners (page points), read through the layer's axes. `None` where the
/// layer cannot be rescaled this way: a degenerate frame, a polar or ternary layer, an axis with
/// breaks (unmeasured in Origin), or a result that is not finite or has equal ends.
pub fn rect_to_ends(g: &LayerGeometry, a: (f64, f64), b: (f64, f64)) -> Option<((f64, f64), (f64, f64))> {
    let f = g.frame;
    if g.polar.is_some()
        || g.ternary.is_some()
        || f.w <= 0.0
        || f.h <= 0.0
        || !g.x.gaps.is_empty()
        || !g.y.gaps.is_empty()
    {
        return None;
    }
    let (left, right) = (a.0.min(b.0), a.0.max(b.0));
    let (top, bottom) = (a.1.min(b.1), a.1.max(b.1));
    let ux = |px: f64| (px - f.x) / f.w;
    let uy = |py: f64| 1.0 - (py - f.y) / f.h;
    let x = (g.x.from_unit(ux(left)), g.x.from_unit(ux(right)));
    let y = (g.y.from_unit(uy(bottom)), g.y.from_unit(uy(top)));
    let ok = |(lo, hi): (f64, f64)| lo.is_finite() && hi.is_finite() && lo != hi;
    (ok(x) && ok(y)).then_some((x, y))
}

/// The axis that holds the limits shown by axis `y` of layer `layer`: the layer's own, or the
/// parent's where the layer follows it with a straight link.
fn owner(graph: &Graph, layer: usize, y: bool) -> usize {
    match graph.valid_link(layer) {
        Some(k) if *(if y { &k.y } else { &k.x }) == ScaleLink::Straight => k.parent,
        _ => layer,
    }
}

fn axis_mut(graph: &mut Graph, layer: usize, y: bool) -> Option<&mut Axis> {
    let l = graph.layers.get_mut(layer)?;
    Some(if y { &mut l.y_axis } else { &mut l.x_axis })
}

fn ends(axis: &Axis) -> Ends {
    (axis.min, axis.max)
}

/// Scale In: the active layer's x and y ends. Returns whether anything changed. One undo step.
pub fn scale_in(
    doc: &mut Document,
    state: &mut ScaleState,
    graph: WindowId,
    layer: usize,
    x: (f64, f64),
    y: (f64, f64),
) -> bool {
    let mut step = Step { active: layer, changes: Vec::new() };
    if doc.project().graph(graph).is_none_or(|g| layer >= g.layers.len()) {
        return false;
    }
    doc.apply(UNDO_LABEL, |p| {
        let Some(g) = p.graph_mut(graph) else { return };
        for (is_y, (lo, hi)) in [(false, x), (true, y)] {
            let owner = owner(g, layer, is_y);
            if let Some(axis) = axis_mut(g, owner, is_y) {
                // the Reverse Order check box and a min above max cancel each other: keep it ascending
                let (lo, hi) = if axis.reverse { (lo.min(hi), lo.max(hi)) } else { (lo, hi) };
                step.changes.push(Change { layer: owner, y: is_y, before: ends(axis), after: (lo, hi) });
                axis.min = Some(lo);
                axis.max = Some(hi);
            }
        }
    });
    let changed = !step.changes.is_empty();
    if changed {
        state.stack.push(step);
    }
    changed
}

/// Scale Out: restores the scale from before the last Scale In of layer `layer` that is still
/// current (a step whose axes were changed since, for example by Undo, is dropped). Returns whether
/// a step was taken back. One undo step.
pub fn scale_out(doc: &mut Document, state: &mut ScaleState, graph: WindowId, layer: usize) -> bool {
    let current = |g: &Graph, c: &Change| {
        g.layers.get(c.layer).map(|l| if c.y { &l.y_axis } else { &l.x_axis }).map(ends)
            == Some((Some(c.after.0), Some(c.after.1)))
    };
    let Some(g) = doc.project().graph(graph) else { return false };
    let mut found = None;
    while let Some(i) = state.stack.iter().rposition(|s| s.active == layer) {
        let step = state.stack.remove(i);
        if step.changes.iter().all(|c| current(g, c)) {
            found = Some(step);
            break;
        }
    }
    let Some(step) = found else { return false };
    doc.apply(UNDO_LABEL, |p| {
        if let Some(g) = p.graph_mut(graph) {
            for c in &step.changes {
                if let Some(axis) = axis_mut(g, c.layer, c.y) {
                    (axis.min, axis.max) = c.before;
                }
            }
        }
    });
    true
}

/// The zoom and scroll for View > Zoom All: the page and every object (`bounds`, page points, also
/// outside the page) fit into `window` with a small margin. It does not depend on the current zoom,
/// so doing it twice changes nothing. The page stays where the T65 layout puts a page smaller than
/// the window (centred); what lies outside the page is not drawn (unbuilt, see graphing 23.4).
pub fn zoom_all(window: Rect, page: Vec2, bounds: &[Rect]) -> Zoom {
    let mut all = Rect::from_min_size(Pos2::ZERO, page);
    for b in bounds.iter().filter(|b| b.is_finite()) {
        all = all.union(*b);
    }
    let avail = graph_zoom::content(window).size() - Vec2::splat(2.0 * ZOOM_ALL_MARGIN);
    let fit = graph_zoom::fit_scale(window, page);
    let scale = (avail.x / all.width()).min(avail.y / all.height());
    let factor = (scale / fit).clamp(graph_zoom::MIN_ZOOM, 1.0);
    Zoom { factor, scroll: Vec2::ZERO }
}

/// The bounds of all objects of a scene, page points.
pub fn scene_bounds(scene: &Scene) -> Vec<Rect> {
    scene
        .layers
        .iter()
        .flat_map(|l| &l.objects)
        .map(|o| {
            Rect::from_min_size(
                Pos2::new(o.bounds.x as f32, o.bounds.y as f32),
                egui::vec2(o.bounds.w as f32, o.bounds.h as f32),
            )
        })
        .collect()
}

/// The tools' input for one frame, over the page: Esc ends the tool; Scale In draws the dotted
/// rectangle of a drag and asks for the new ends on release; Scale Out takes a click or a drag.
/// `to_page` converts a screen position to page points.
pub fn show(
    ui: &egui::Ui,
    painter: &egui::Painter,
    response: &egui::Response,
    geometry: &[LayerGeometry],
    to_page: &dyn Fn(Pos2) -> (f64, f64),
    graph: WindowId,
    active: usize,
    state: &mut ScaleState,
    actions: &mut Vec<Action>,
) {
    let Some(tool) = state.tool else {
        state.start = None;
        return;
    };
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.tool = None;
        state.start = None;
        return;
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(match tool {
            ScaleTool::In => egui::CursorIcon::ZoomIn,
            ScaleTool::Out => egui::CursorIcon::ZoomOut,
        });
    }
    if response.drag_started() {
        state.start = ui.input(|i| i.pointer.press_origin());
    }
    let now = response.interact_pointer_pos();
    if let (Some(a), Some(b)) = (state.start, now)
        && tool == ScaleTool::In
        && response.dragged()
    {
        let dotted = Stroke::new(1.0_f32, Color32::from_gray(40));
        for shape in egui::Shape::dashed_line(&rect_outline(Rect::from_two_pos(a, b)), dotted, 3.0, 3.0) {
            painter.add(shape);
        }
    }
    let ctrl = ui.input(|i| i.modifiers.command);
    match tool {
        ScaleTool::In if response.drag_stopped() => {
            let rect = state.start.zip(now).map(|(a, b)| Rect::from_two_pos(a, b));
            state.start = None;
            let Some(rect) = rect.filter(|r| r.width() >= MIN_DRAG && r.height() >= MIN_DRAG) else {
                // a tiny drag ends the tool without a change, as a click does in Origin
                state.tool = None;
                return;
            };
            if ctrl {
                actions.push(Action::Hint("Scale In: the zoom graph of Ctrl+drag is not built".to_string()));
                return;
            }
            let ends = geometry.get(active).and_then(|g| rect_to_ends(g, to_page(rect.min), to_page(rect.max)));
            match ends {
                Some((x, y)) => {
                    state.tool = None;
                    actions.push(Action::ScaleIn { graph, layer: active, x, y });
                }
                None => actions
                    .push(Action::Hint("Scale In: this layer's axes cannot be rescaled by a rectangle".to_string())),
            }
        }
        ScaleTool::In if response.clicked() => {
            // a plain click changes nothing and ends the tool (graphing 23.3)
            state.start = None;
            state.tool = None;
        }
        ScaleTool::Out if response.clicked() || response.drag_stopped() => {
            state.start = None;
            state.tool = None;
            actions.push(Action::ScaleOut { graph, layer: active });
        }
        _ => {}
    }
}

fn rect_outline(r: Rect) -> Vec<Pos2> {
    vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::graph::ScaleKind;
    use sheaf_plot::ticks::ResolvedAxis;

    fn axis(min: f64, max: f64, scale: ScaleKind) -> ResolvedAxis {
        ResolvedAxis {
            min,
            max,
            step: 1.0,
            scale,
            formula: None,
            discrete: None,
            major: Vec::new(),
            minor: Vec::new(),
            labels: Vec::new(),
            reversed: false,
            gaps: Vec::new(),
            minor_labels: Vec::new(),
        }
    }

    fn geometry(x: ResolvedAxis, y: ResolvedAxis) -> LayerGeometry {
        let frame = sheaf_plot::Rect { x: 100.0, y: 50.0, w: 200.0, h: 100.0 };
        LayerGeometry {
            frame,
            clip: frame,
            x,
            y,
            polar: None,
            ternary: None,
            connect_across_breaks: false,
            connect_across_missing: false,
        }
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn linear_rectangle_gives_its_edges_left_to_right_and_bottom_to_top() {
        let g = geometry(axis(0.0, 10.0, ScaleKind::Linear), axis(0.0, 100.0, ScaleKind::Linear));
        // x 20 % to 70 % of the frame, y: the frame's top quarter to its middle
        let ((x0, x1), (y0, y1)) = rect_to_ends(&g, (140.0, 75.0), (240.0, 100.0)).unwrap();
        assert!(near(x0, 2.0) && near(x1, 7.0), "{x0} {x1}");
        assert!(near(y0, 50.0) && near(y1, 75.0), "{y0} {y1}");
        // the opposite corners in any order give the same ends
        assert_eq!(rect_to_ends(&g, (240.0, 100.0), (140.0, 75.0)), Some(((x0, x1), (y0, y1))));
    }

    #[test]
    fn a_log_axis_maps_the_rectangle_linearly_in_log10() {
        let g = geometry(axis(1.0, 10000.0, ScaleKind::Log10), axis(0.01, 100.0, ScaleKind::Log10));
        let ((x0, x1), (y0, y1)) = rect_to_ends(&g, (150.0, 50.0), (250.0, 150.0)).unwrap();
        assert!(near(x0, 10f64.powf(1.0)) && near(x1, 10f64.powf(3.0)), "{x0} {x1}");
        assert!(near(y0, 0.01) && near(y1, 100.0), "{y0} {y1}");
    }

    #[test]
    fn outside_the_frame_extends_the_range() {
        let g = geometry(axis(0.0, 10.0, ScaleKind::Linear), axis(0.0, 100.0, ScaleKind::Linear));
        let ((x0, _), _) = rect_to_ends(&g, (50.0, 60.0), (200.0, 140.0)).unwrap();
        assert!(near(x0, -2.5), "{x0}");
    }

    #[test]
    fn a_reversed_axis_stays_reversed() {
        let mut x = axis(0.0, 10.0, ScaleKind::Linear);
        x.reversed = true;
        let g = geometry(x, axis(0.0, 100.0, ScaleKind::Linear));
        let ((x0, x1), _) = rect_to_ends(&g, (100.0, 60.0), (200.0, 140.0)).unwrap();
        assert!(near(x0, 10.0) && near(x1, 5.0), "{x0} {x1}");
    }

    #[test]
    fn scale_in_on_a_reverse_checkbox_axis_keeps_it_reversed() {
        let mut doc = Document::default();
        let id = doc.apply("g", |p| p.add_graph(sheaf_core::template::Template::Line));
        doc.apply("rev", |p| {
            let a = &mut p.graph_mut(id).unwrap().layers[0].x_axis;
            (a.reverse, a.min, a.max) = (true, Some(0.0), Some(10.0));
        });
        let mut state = ScaleState::default();
        assert!(scale_in(&mut doc, &mut state, id, 0, (10.0, 5.0), (0.0, 1.0)));
        let a = &doc.project().graph(id).unwrap().layers[0].x_axis;
        let r = sheaf_plot::ticks::resolve_axis(a, None);
        assert!(r.reversed, "the check box still reverses the axis");
        assert!(near(r.min, 5.0) && near(r.max, 10.0), "{} {}", r.min, r.max);
    }

    #[test]
    fn a_degenerate_rectangle_gives_nothing() {
        let g = geometry(axis(0.0, 10.0, ScaleKind::Linear), axis(0.0, 100.0, ScaleKind::Linear));
        assert_eq!(rect_to_ends(&g, (150.0, 60.0), (150.0, 140.0)), None);
    }

    #[test]
    fn zoom_all_fits_outside_objects_and_is_idempotent() {
        let window = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let page = egui::vec2(400.0, 300.0);
        let alone = zoom_all(window, page, &[]);
        assert!(alone.factor < 1.0 && alone.factor > 0.9, "{}", alone.factor);
        let far = Rect::from_min_size(Pos2::new(1000.0, 100.0), egui::vec2(100.0, 20.0));
        let z = zoom_all(window, page, &[far]);
        assert!(z.factor < alone.factor, "{} {}", z.factor, alone.factor);
        assert_eq!(z, zoom_all(window, page, &[far]));
        // the whole box fits the window
        let fit = graph_zoom::fit_scale(window, page) * z.factor;
        assert!(1100.0 * fit <= graph_zoom::content(window).width());
    }
}
