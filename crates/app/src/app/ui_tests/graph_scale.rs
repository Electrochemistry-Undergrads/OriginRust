//! View > Zoom All and the Tools-bar Scale In / Scale Out (T69; graphing spec section 23).
use super::*;
use crate::actions::GraphTool;
use sheaf_core::graph::ScaleKind;
use sheaf_core::layers::NewLayerKind;

fn id(h: &Harness<'static, SheafApp>) -> WindowId {
    project(h).graphs().next().unwrap().id
}

type Ends = (Option<f64>, Option<f64>);

fn axes(h: &Harness<'static, SheafApp>, layer: usize) -> [Ends; 2] {
    let g = project(h).graphs().next().unwrap();
    let l = &g.layers[layer];
    [(l.x_axis.min, l.x_axis.max), (l.y_axis.min, l.y_axis.max)]
}

/// Fixes layer 0 to x 0..10 and y 0..100.
fn fix_axes(h: &mut Harness<'static, SheafApp>) {
    let g = id(h);
    h.state_mut().doc.apply("Fix axes", |p| {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        (l.x_axis.min, l.x_axis.max) = (Some(0.0), Some(10.0));
        (l.y_axis.min, l.y_axis.max) = (Some(0.0), Some(100.0));
    });
    settle(h);
}

/// A screen point at fractions of layer `layer`'s frame (0,0 top left).
fn in_frame(h: &Harness<'static, SheafApp>, layer: usize, fx: f64, fy: f64) -> egui::Pos2 {
    let f = sheaf_plot::layout_geometry(project(h), id(h)).unwrap()[layer].frame;
    graph_ui(h).page_point(f.x + f.w * fx, f.y + f.h * fy).expect("drawn")
}

fn scale_in_tool(h: &mut Harness<'static, SheafApp>) {
    h.get_by_label("Tools toolbar: Scale In").click();
    settle(h);
}

fn scale_out_tool(h: &mut Harness<'static, SheafApp>) {
    h.get_by_label("Tools toolbar: Scale Out").click();
    settle(h);
}

fn close(a: Option<f64>, b: f64) -> bool {
    a.is_some_and(|a| (a - b).abs() < 0.05)
}

fn zoom_in_drag(h: &mut Harness<'static, SheafApp>, a: (f64, f64), b: (f64, f64)) {
    scale_in_tool(h);
    let (pa, pb) = (in_frame(h, 0, a.0, a.1), in_frame(h, 0, b.0, b.1));
    drag(h, pa, pb);
}

fn scale_out_click(h: &mut Harness<'static, SheafApp>) {
    scale_out_tool(h);
    let p = in_frame(h, 0, 0.5, 0.5);
    click_at(h, p);
}

#[test]
fn scale_in_sets_the_active_layers_axes_to_the_dragged_rectangle_and_ends() {
    let mut h = start(vec![data_file("scale-in")]);
    fix_axes(&mut h);
    scale_in_tool(&mut h);
    assert_eq!(graph_ui(&h).tool(), GraphTool::ScaleIn);
    let (pa, pb) = (in_frame(&h, 0, 0.2, 0.25), in_frame(&h, 0, 0.7, 0.5));
    drag(&mut h, pa, pb);
    let [x, y] = axes(&h, 0);
    assert!(close(x.0, 2.0) && close(x.1, 7.0), "{x:?}");
    assert!(close(y.0, 50.0) && close(y.1, 75.0), "{y:?}");
    assert_eq!(graph_ui(&h).tool(), GraphTool::Pointer, "the tool ends after one drag");
    assert_eq!(h.state().doc.undo_label(), Some("Zoom In/Out"));
    assert!(graph_ui(&h).selected.is_none(), "the drag only rescales");
}

#[test]
fn a_click_or_a_tiny_drag_with_scale_in_changes_nothing() {
    let mut h = start(vec![data_file("scale-click")]);
    fix_axes(&mut h);
    let before = axes(&h, 0);
    scale_in_tool(&mut h);
    let p = in_frame(&h, 0, 0.5, 0.5);
    click_at(&mut h, p);
    drag(&mut h, p, p + egui::vec2(3.0, 2.0));
    assert_eq!(axes(&h, 0), before);
    assert_eq!(h.state().doc.undo_label(), Some("Fix axes"));
    key(&mut h, egui::Key::Escape);
    assert_eq!(graph_ui(&h).tool(), GraphTool::Pointer, "Esc ends the tool");
}

#[test]
fn scale_in_steps_nest_and_scale_out_goes_back_one_step_at_a_time() {
    let mut h = start(vec![data_file("scale-nest")]);
    fix_axes(&mut h);
    let original = axes(&h, 0);
    let mut steps = vec![original];
    for _ in 0..3 {
        zoom_in_drag(&mut h, (0.25, 0.25), (0.75, 0.75));
        let now = axes(&h, 0);
        assert_ne!(now, *steps.last().unwrap());
        steps.push(now);
    }
    // each rectangle is read against the current view: half the span each time
    let [x, _] = steps[3];
    assert!(close(x.1.zip(x.0).map(|(a, b)| a - b), 1.25), "{x:?}");
    for want in steps.iter().rev().skip(1) {
        scale_out_click(&mut h);
        assert_eq!(axes(&h, 0), *want);
        assert_eq!(graph_ui(&h).tool(), GraphTool::Pointer, "the tool ends after one use");
    }
    // nothing left to go back to: no change and no undo step
    let undo = h.state().doc.undo_label().map(str::to_string);
    scale_out_click(&mut h);
    assert_eq!(axes(&h, 0), original);
    assert_eq!(h.state().doc.undo_label().map(str::to_string), undo);
}

#[test]
fn scale_out_takes_a_drag_of_any_rectangle_too_and_restores_automatic_ends() {
    let mut h = start(vec![data_file("scale-out-drag")]);
    let auto = axes(&h, 0);
    assert_eq!(auto, [(None, None), (None, None)]);
    zoom_in_drag(&mut h, (0.25, 0.25), (0.75, 0.75));
    assert_ne!(axes(&h, 0), auto);
    scale_out_tool(&mut h);
    let (pa, pb) = (in_frame(&h, 0, 0.1, 0.1), in_frame(&h, 0, 0.3, 0.2));
    drag(&mut h, pa, pb);
    assert_eq!(axes(&h, 0), auto);
}

#[test]
fn each_scale_step_is_one_undo_step() {
    let mut h = start(vec![data_file("scale-undo")]);
    fix_axes(&mut h);
    zoom_in_drag(&mut h, (0.0, 0.0), (0.5, 0.5));
    let first = axes(&h, 0);
    zoom_in_drag(&mut h, (0.0, 0.0), (0.5, 0.5));
    assert_ne!(axes(&h, 0), first);
    menu(&mut h, "Edit", "Undo Zoom In/Out");
    assert_eq!(axes(&h, 0), first);
    menu(&mut h, "Edit", "Undo Zoom In/Out");
    assert_eq!(axes(&h, 0)[0], (Some(0.0), Some(10.0)));
    // Scale Out after the undo has no stale step to take
    scale_out_click(&mut h);
    assert_eq!(axes(&h, 0)[0], (Some(0.0), Some(10.0)));
}

#[test]
fn a_log_axis_takes_the_rectangle_in_log10_and_stays_logarithmic() {
    let mut h = start(vec![data_file("scale-log")]);
    let g = id(&h);
    h.state_mut().doc.apply("Log", |p| {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.x_axis.scale = ScaleKind::Log10;
        (l.x_axis.min, l.x_axis.max) = (Some(1.0), Some(10000.0));
        (l.y_axis.min, l.y_axis.max) = (Some(0.0), Some(100.0));
    });
    settle(&mut h);
    zoom_in_drag(&mut h, (0.25, 0.25), (0.75, 0.75));
    let [x, _] = axes(&h, 0);
    // a quarter and three quarters of four decades
    assert!(x.0.is_some_and(|v| (v - 10.0).abs() < 0.1) && x.1.is_some_and(|v| (v - 1000.0).abs() < 10.0), "{x:?}");
    let g = project(&h).graphs().next().unwrap();
    assert_eq!(g.layers[0].x_axis.scale, ScaleKind::Log10);
}

#[test]
fn only_the_active_layer_changes_and_a_linked_axis_follows_its_parent() {
    let mut h = start(vec![data_file("scale-linked")]);
    fix_axes(&mut h);
    let g = id(&h);
    let ctx = h.ctx.clone();
    h.state_mut()
        .apply(Action::GraphEdit { graph: g, op: GraphOp::AddLayer { layer: 0, kind: NewLayerKind::RightY } }, &ctx);
    settle(&mut h);
    h.state_mut().doc.apply("Fix right y", |p| {
        let l = &mut p.graph_mut(g).unwrap().layers[1];
        (l.y_axis.min, l.y_axis.max) = (Some(9.0), Some(20.0));
    });
    settle(&mut h);
    // the first layer active: its x and y change; the right layer's own y does not
    h.state_mut().apply(Action::ActivateLayer { graph: g, layer: 0 }, &ctx);
    zoom_in_drag(&mut h, (0.2, 0.25), (0.7, 0.5));
    assert!(close(axes(&h, 0)[0].0, 2.0), "{:?}", axes(&h, 0));
    assert_eq!(axes(&h, 1)[1], (Some(9.0), Some(20.0)), "the linked layer's own y stays");
    let geometry = sheaf_plot::layout_geometry(project(&h), g).unwrap();
    assert!(close(Some(geometry[1].x.min), 2.0) && close(Some(geometry[1].x.max), 7.0), "x follows");
    // the right layer active: the rectangle goes through its own y axis; layer 0's y stays
    let y0 = axes(&h, 0)[1];
    h.state_mut().apply(Action::ActivateLayer { graph: g, layer: 1 }, &ctx);
    zoom_in_drag(&mut h, (0.0, 0.0), (0.5, 0.5));
    assert_ne!(axes(&h, 1)[1], (Some(9.0), Some(20.0)), "its own y changed");
    assert_eq!(axes(&h, 0)[1], y0, "layer 0's y did not follow");
    assert!(close(axes(&h, 0)[0].0, 2.0) && close(axes(&h, 0)[0].1, 4.5), "layer 0's x followed: {:?}", axes(&h, 0));
    // Scale Out restores the right layer's axes
    scale_out_click(&mut h);
    assert_eq!(axes(&h, 1)[1], (Some(9.0), Some(20.0)));
}

#[test]
fn another_tool_replaces_the_scale_tools() {
    let mut h = start(vec![data_file("scale-other")]);
    scale_in_tool(&mut h);
    assert_eq!(graph_ui(&h).tool(), GraphTool::ScaleIn);
    scale_out_tool(&mut h);
    assert_eq!(graph_ui(&h).tool(), GraphTool::ScaleOut);
    h.get_by_label("Tools toolbar: Data Reader").click();
    settle(&mut h);
    assert_eq!(graph_ui(&h).tool(), GraphTool::Reader);
}

fn far_label(h: &mut Harness<'static, SheafApp>) {
    let g = id(h);
    h.state_mut().doc.apply("Far label", |p| {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        let font = l.new_label_font();
        l.labels.push(sheaf_core::graph::TextLabel::new("far away", 4.0, 0.5, font));
    });
    settle(h);
}

#[test]
fn zoom_all_fits_an_object_outside_the_page_is_repeatable_and_whole_page_returns() {
    let mut h = start(vec![data_file("zoom-all")]);
    menu_path(&mut h, &["View", "Zoom All"]);
    let page_only = graph_ui(&h).zoom.factor;
    assert!(page_only < 1.0 && page_only > 0.5, "{page_only}");
    far_label(&mut h);
    menu_path(&mut h, &["View", "Zoom All"]);
    let all = graph_ui(&h).zoom;
    assert!(all.factor < page_only, "{} {page_only}", all.factor);
    menu_path(&mut h, &["View", "Zoom All"]);
    assert_eq!(graph_ui(&h).zoom, all, "a second Zoom All changes nothing");
    menu_path(&mut h, &["View", "Whole Page"]);
    assert_eq!(graph_ui(&h).zoom.factor, 1.0);
    // screen only: no undo step, the project is untouched
    assert_eq!(h.state().doc.undo_label(), Some("Far label"));
}
