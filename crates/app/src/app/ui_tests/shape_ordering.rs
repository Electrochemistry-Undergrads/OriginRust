//! Shape ordering is scoped to one layer and selection indices remain safe across history.
use super::*;
use sheaf_core::shape::{Shape, ShapeKind};

fn setup(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(&format!("shape-order-{tag}"))]);
    let id = graph(&h).id;
    let mut rect = Shape::new(ShapeKind::Rectangle, (0.2, 0.2), (0.7, 0.7));
    rect.fill = Some(sheaf_core::Color::rgba(220, 20, 20, 255));
    let mut ellipse = Shape::new(ShapeKind::Ellipse, (0.35, 0.35), (0.85, 0.85));
    ellipse.fill = Some(sheaf_core::Color::rgba(20, 20, 220, 255));
    h.state_mut().doc.apply("Shapes", |p| p.graph_mut(id).unwrap().layers[0].shapes = vec![rect, ellipse]);
    settle(&mut h);
    h
}
fn context(h: &mut Harness<'static, SheafApp>, point: (f64, f64), action: &str) {
    let at = frame_point(h, point.0, point.1);
    right_click_at(h, at);
    assert!(entry_enabled(h, action));
    menu_entry(h, action).click();
    settle(h);
}

#[test]
fn context_reorders_overlapping_filled_shapes_and_picking_follows_paint_order() {
    let mut h = setup("context");
    let original = shapes(&h);
    context(&mut h, (0.5, 0.5), "Push to the back");
    assert_eq!(shapes(&h), vec![original[1].clone(), original[0].clone()]);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(0))));
    assert_eq!(h.state().doc.undo_label(), Some("Push drawn object to back"));
    // The next hit in the overlap is the rectangle, now drawn last.
    let overlap = frame_point(&h, 0.5, 0.5);
    right_click_at(&mut h, overlap);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(1))));
    close_menus(&mut h);
    // The part of the ellipse outside the rectangle remains hittable: move it to the front.
    context(&mut h, (0.77, 0.6), "Bring to the front");
    assert_eq!(shapes(&h), original);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(1))));
    assert_eq!(h.state().doc.undo_label(), Some("Bring drawn object to front"));
    let overlap = frame_point(&h, 0.5, 0.5);
    right_click_at(&mut h, overlap);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(1))));
    close_menus(&mut h);
    // Already at the front and stale indices create no history or project changes.
    let revision = h.state().doc.revision();
    context(&mut h, (0.5, 0.5), "Bring to the front");
    assert_eq!(h.state().doc.revision(), revision);
    let id = graph(&h).id;
    h.state_mut().edit_graph(id, GraphOp::OrderShape { layer: 0, shape: 99, front: false });
    h.state_mut().edit_graph(id, GraphOp::OrderShape { layer: 99, shape: 0, front: true });
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(shapes(&h), original);
}

#[test]
fn undo_redo_clear_stale_shape_selection_before_delete_or_properties() {
    let mut h = setup("history");
    let original = shapes(&h);
    context(&mut h, (0.5, 0.5), "Push to the back");
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(0))));
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(shapes(&h), original);
    assert_eq!(graph_ui(&h).selected, None, "undo must not select a different shape");
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!(shapes(&h), original, "Delete after undo cannot delete the wrong shape");
    let overlap = frame_point(&h, 0.5, 0.5);
    click_at(&mut h, overlap);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(1))));
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut h);
    assert_eq!(graph_ui(&h).selected, None, "redo also clears the stale index");
    assert_eq!(shapes(&h), vec![original[1].clone(), original[0].clone()]);
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!(shapes(&h).len(), 2);
    // Opening Properties by a fresh hit edits the visible object rather than the former selection.
    let overlap = frame_point(&h, 0.5, 0.5);
    double_click_at(&mut h, overlap);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(1))));
    assert!(h.state().shape_dialog.is_some());
}

#[test]
fn ordering_stays_in_its_layer_and_history_keeps_unrelated_selection() {
    let mut h = setup("layers");
    let id = graph(&h).id;
    h.state_mut().doc.apply("Other layer", |p| {
        let g = p.graph_mut(id).unwrap();
        let mut layer = g.layers[0].clone();
        layer.shapes.reverse();
        g.layers.push(layer);
    });
    let untouched = graph(&h).layers[1].shapes.clone();
    let original = graph(&h).layers[0].shapes.clone();
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Legend));
    h.state_mut().edit_graph(id, GraphOp::OrderShape { layer: 0, shape: 0, front: true });
    assert_eq!(graph(&h).layers[1].shapes, untouched);
    assert_eq!(graph(&h).layers[0].shapes, vec![original[1].clone(), original[0].clone()]);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(graph(&h).layers[0].shapes, original);
    assert_eq!(h.state().graphs[&id].selected, Some((0, ObjectKind::Legend)));
    // An object already at the back cannot produce an undo step.
    let revision = h.state().doc.revision();
    h.state_mut().edit_graph(id, GraphOp::OrderShape { layer: 0, shape: 0, front: false });
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn ordering_closes_properties_and_undo_invalidates_an_open_shape_menu() {
    let mut h = setup("dialog");
    let id = graph(&h).id;
    let overlap = frame_point(&h, 0.5, 0.5);
    double_click_at(&mut h, overlap);
    assert!(h.state().shape_dialog.is_some());
    h.state_mut().edit_graph(id, GraphOp::OrderShape { layer: 0, shape: 1, front: false });
    assert!(h.state().shape_dialog.is_none(), "reordering invalidates the open properties dialog");
    settle(&mut h);
    let overlap = frame_point(&h, 0.5, 0.5);
    right_click_at(&mut h, overlap);
    assert!(h.query_by_label("Properties...").is_some());
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(graph_ui(&h).selected, None);
    assert!(h.query_by_label("Properties...").is_none(), "the context menu cannot use the old index");
}

#[test]
fn undo_ai_changes_also_clears_shape_selection_when_the_stack_changes() {
    let mut h = setup("ai-history");
    let id = graph(&h).id;
    let original = shapes(&h);
    h.state_mut().doc.apply("AI: shape change", |p| p.graph_mut(id).unwrap().layers[0].shapes.reverse());
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Shape(0)));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::UndoAiChanges, &ctx);
    assert_eq!(shapes(&h), original);
    assert_eq!(graph_ui(&h).selected, None, "Undo AI uses the ordinary shape selection cleanup");
    assert!(h.state().status.contains("Undone 1 AI change"));
}
