use super::*;
use sheaf_core::shape::{Shape, ShapeKind};

fn setup(tag: &str, kind: ShapeKind) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(&format!("shape-resize-{tag}"))]);
    let id = graph(&h).id;
    let mut shape = Shape::new(kind, (0.7, 0.7), (0.3, 0.3));
    shape.head_at_start = true;
    shape.width_pt = 2.0;
    h.state_mut().doc.apply("Shape", |p| p.graph_mut(id).unwrap().layers[0].shapes.push(shape));
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Shape(0)));
    settle(&mut h);
    h
}

#[test]
fn endpoints_and_corners_resize_across_the_fixed_point_without_losing_style() {
    for (tag, kind, from, to, expected) in [
        ("line", ShapeKind::Line, (0.7, 0.7), (0.1, 0.15), (0.1, 0.15, 0.3, 0.3)),
        ("arrow", ShapeKind::Arrow, (0.3, 0.3), (0.9, 0.8), (0.7, 0.7, 0.9, 0.8)),
        ("rect", ShapeKind::Rectangle, (0.7, 0.3), (0.1, 0.9), (0.1, 0.7, 0.3, 0.9)),
        ("ellipse", ShapeKind::Ellipse, (0.3, 0.7), (0.85, 0.1), (0.7, 0.1, 0.85, 0.3)),
    ] {
        let mut h = setup(tag, kind);
        let original = shapes(&h)[0].clone();
        drag_frame(&mut h, from, to);
        let s = &shapes(&h)[0];
        for (actual, expected) in [(s.x1, expected.0), (s.y1, expected.1), (s.x2, expected.2), (s.y2, expected.3)] {
            assert!((actual - expected).abs() < 0.015, "{tag}: {s:?}");
        }
        assert_eq!(s.color, original.color);
        assert_eq!(s.width_pt, original.width_pt);
        assert_eq!(s.head_at_start, original.head_at_start);
        assert_eq!(s.fill, original.fill);
        assert_eq!(h.state().doc.undo_label(), Some("Resize drawn object"));
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(shapes(&h)[0], original);
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
        settle(&mut h);
        assert_ne!(shapes(&h)[0], original);
    }
}

#[test]
fn drawing_tool_takes_precedence_over_selected_handle() {
    let mut h = setup("tool", ShapeKind::Rectangle);
    let original = shapes(&h)[0].clone();
    let id = graph(&h).id;
    h.state_mut().graphs.get_mut(&id).unwrap().shape_tool = Some(ShapeKind::Line);
    settle(&mut h);
    drag_frame(&mut h, (0.7, 0.7), (0.9, 0.8));
    assert_eq!(shapes(&h)[0], original);
    assert_eq!(shapes(&h).len(), 2);
}

#[test]
fn escape_cancels_preview_and_stale_resize_cannot_mutate_another_shape() {
    let mut h = setup("cancel", ShapeKind::Arrow);
    let original = shapes(&h);
    let start = frame_point(&h, 0.7, 0.7);
    let end = frame_point(&h, 0.9, 0.9);
    h.event(egui::Event::PointerMoved(start));
    h.step();
    h.event(egui::Event::PointerButton {
        pos: start,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.step();
    h.event(egui::Event::PointerMoved(end));
    h.step();
    assert_eq!(shapes(&h), original, "preview must not mutate the document");
    h.key_press(egui::Key::Escape);
    h.event(egui::Event::PointerButton {
        pos: end,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    assert_eq!(shapes(&h), original);
    assert_eq!(h.state().doc.undo_label(), Some("Shape"));
}

#[test]
fn stale_resize_cannot_overwrite_a_changed_shape() {
    let mut h = setup("stale", ShapeKind::Rectangle);
    let id = graph(&h).id;
    let before = shapes(&h)[0].clone();
    let mut after = before.clone();
    after.x1 = 0.1;
    h.state_mut().doc.apply("Change style", |p| p.graph_mut(id).unwrap().layers[0].shapes[0].width_pt = 5.0);
    let changed = shapes(&h);
    let revision = h.state().doc.revision();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::ResizeShape { graph: id, layer: 0, shape: 0, before, after }, &ctx);
    assert_eq!(shapes(&h), changed);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn escape_on_first_drag_frame_does_not_begin_a_move_or_resize() {
    let mut h = setup("escape-first", ShapeKind::Rectangle);
    let original = shapes(&h);
    let start = frame_point(&h, 0.7, 0.7);
    let end = frame_point(&h, 0.9, 0.9);
    h.event(egui::Event::PointerMoved(start));
    h.step();
    h.event(egui::Event::PointerButton {
        pos: start,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.step();
    h.event(egui::Event::PointerMoved(end));
    h.event(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.step();
    h.event(egui::Event::PointerButton {
        pos: end,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    assert_eq!(shapes(&h), original);
    assert_eq!(h.state().doc.undo_label(), Some("Shape"));
}

#[test]
fn handle_can_move_outside_the_layer_frame_and_return_to_undo_checkpoint() {
    let mut h = setup("outside", ShapeKind::Arrow);
    let before = shapes(&h)[0].clone();
    h.state_mut().doc.mark_saved();
    drag_frame(&mut h, (0.3, 0.3), (1.05, 0.8));
    let resized = shapes(&h)[0].clone();
    assert!((resized.x2 - 1.05).abs() < 0.02, "{resized:?}");
    assert!(h.state().doc.dirty);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(shapes(&h)[0], before);
    assert!(!h.state().doc.dirty);
}

#[test]
fn click_on_ellipse_corner_handle_preserves_selection_without_edit() {
    let mut h = setup("handle-click", ShapeKind::Ellipse);
    let before = shapes(&h);
    let revision = h.state().doc.revision();
    let corner = frame_point(&h, 0.7, 0.7);
    click_at(&mut h, corner);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(0))));
    assert_eq!(shapes(&h), before);
    assert_eq!(h.state().doc.revision(), revision);
    double_click_at(&mut h, corner);
    assert!(h.state().shape_dialog.is_some(), "handle double click opens shape properties");
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(0))));
}

#[test]
fn reorder_during_resize_cancels_the_stale_interaction() {
    let mut h = setup("reorder-drag", ShapeKind::Rectangle);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Second shape", |p| {
        p.graph_mut(id).unwrap().layers[0].shapes.push(Shape::new(ShapeKind::Line, (0.1, 0.1), (0.2, 0.2)));
    });
    settle(&mut h);
    let start = frame_point(&h, 0.7, 0.7);
    let end = frame_point(&h, 0.9, 0.9);
    h.event(egui::Event::PointerMoved(start));
    h.step();
    h.event(egui::Event::PointerButton {
        pos: start,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.step();
    h.event(egui::Event::PointerMoved(end));
    h.step();
    h.state_mut().edit_graph(id, GraphOp::OrderShape { layer: 0, shape: 0, front: true });
    let ordered = shapes(&h);
    let revision = h.state().doc.revision();
    h.event(egui::Event::PointerButton {
        pos: end,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    assert_eq!(shapes(&h), ordered);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn dragging_a_handle_onto_its_opposite_rejects_a_collapsed_shape() {
    for (tag, kind) in [
        ("collapse-line", ShapeKind::Line),
        ("collapse-arrow", ShapeKind::Arrow),
        ("collapse-box", ShapeKind::Rectangle),
        ("collapse-ellipse", ShapeKind::Ellipse),
    ] {
        let mut h = setup(tag, kind);
        let before = shapes(&h);
        let revision = h.state().doc.revision();
        drag_frame(&mut h, (0.7, 0.7), (0.3, 0.3));
        assert_eq!(shapes(&h), before, "{kind:?}");
        assert_eq!(h.state().doc.revision(), revision, "{kind:?}");
    }
}

#[test]
fn a_handle_of_the_reference_resizes_it_alone_within_a_multi_selection() {
    let mut h = setup("multi", ShapeKind::Rectangle);
    let id = graph(&h).id;
    let other = Shape::new(ShapeKind::Rectangle, (0.05, 0.05), (0.15, 0.15));
    h.state_mut().doc.apply("Other", |p| p.graph_mut(id).unwrap().layers[0].shapes.push(other.clone()));
    {
        let state = h.state_mut().graphs.get_mut(&id).unwrap();
        state.also = vec![(0, ObjectKind::Shape(1))];
        state.selected = Some((0, ObjectKind::Shape(0)));
    }
    settle(&mut h);
    drag_frame(&mut h, (0.7, 0.7), (0.8, 0.85));
    assert_eq!(h.state().doc.undo_label(), Some("Resize drawn object"), "a resize, not a move of both");
    assert_eq!(shapes(&h)[1], other);
    let s = &shapes(&h)[0];
    assert!((s.x1 - 0.8).abs() < 0.015 && (s.y1 - 0.85).abs() < 0.015, "{s:?}");
}
