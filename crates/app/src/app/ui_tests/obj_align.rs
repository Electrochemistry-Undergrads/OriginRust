//! T159: Shift+click selects several drawn objects and text labels; the Object Edit toolbar's
//! Align and Make Same Width/Height buttons line them up with the last-selected one.
use super::*;
use sheaf_core::graph::TextLabel;
use sheaf_core::shape::{Shape, ShapeKind};

const LEFT: &str = "Object Edit toolbar: Align Left";
const TOP: &str = "Object Edit toolbar: Align Top";
const CENTER_V: &str = "Object Edit toolbar: Align Vertical Center";
const WIDTH: &str = "Object Edit toolbar: Make Same Width";
const HEIGHT: &str = "Object Edit toolbar: Make Same Height";

fn setup(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(&format!("obj-align-{tag}"))]);
    let id = graph(&h).id;
    let mut a = Shape::new(ShapeKind::Rectangle, (0.1, 0.1), (0.3, 0.25));
    a.fill = Some(sheaf_core::Color::rgba(220, 20, 20, 255));
    let mut b = Shape::new(ShapeKind::Rectangle, (0.5, 0.4), (0.8, 0.6));
    b.fill = Some(sheaf_core::Color::rgba(20, 20, 220, 255));
    h.state_mut().doc.apply("Shapes", |p| {
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.shapes = vec![a, b];
        let font = l.new_label_font();
        l.labels.push(TextLabel::new("Note", 0.15, 0.8, font));
    });
    settle(&mut h);
    h
}

fn enabled(h: &Harness<'static, SheafApp>, label: &str) -> bool {
    !h.get_by_label(label).accesskit_node().is_disabled()
}

fn shift_click_at(h: &mut Harness<'static, SheafApp>, at: egui::Pos2) {
    pause(h);
    h.event(egui::Event::PointerMoved(at));
    h.step();
    for pressed in [true, false] {
        h.input_mut().modifiers = egui::Modifiers::SHIFT;
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::SHIFT,
        });
        h.step();
    }
    h.input_mut().modifiers = egui::Modifiers::NONE;
    settle(h);
}

fn press(h: &mut Harness<'static, SheafApp>, label: &str) {
    assert!(enabled(h, label), "{label} is disabled");
    h.get_by_label(label).click();
    settle(h);
}

/// The drawn bounds of the selected objects, the reference last.
fn bounds(h: &Harness<'static, SheafApp>) -> Vec<(f64, f64, f64, f64)> {
    let id = graph(h).id;
    let (reference, others) = graph_ui(h).arrange_members(project(h), id).expect("two objects selected");
    others.iter().chain([&reference]).map(|m| m.bounds).collect()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-2
}

#[test]
fn shift_click_builds_a_selection_and_align_and_size_follow_the_last_selected_shape() {
    let mut h = setup("shapes");
    let (a, b) = (ObjectKind::Shape(0), ObjectKind::Shape(1));
    // One object: the buttons are state-disabled.
    let at = frame_point(&h, 0.2, 0.17);
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, a)));
    assert!(!enabled(&h, LEFT) && !enabled(&h, WIDTH));
    // Shift+click adds the second one, which becomes the reference.
    let at = frame_point(&h, 0.65, 0.5);
    shift_click_at(&mut h, at);
    assert_eq!(graph_ui(&h).multi_selection(), vec![(0, a), (0, b)]);
    let before = shapes(&h);
    press(&mut h, LEFT);
    let after = shapes(&h);
    assert!(close(after[0].x1, 0.5) && close(after[0].x2, 0.7), "{:?}", after[0]);
    assert_eq!((after[0].y1, after[0].y2), (before[0].y1, before[0].y2));
    assert_eq!(after[1], before[1], "the reference stays put");
    assert_eq!(h.state().doc.undo_label(), Some("Align objects"));
    let drawn = bounds(&h);
    assert!(close(drawn[0].0, drawn[1].0), "{drawn:?}");
    // The selection survives the change: the next buttons act on the same objects.
    press(&mut h, WIDTH);
    assert!(close(shapes(&h)[0].x2 - shapes(&h)[0].x1, 0.3), "{:?}", shapes(&h)[0]);
    assert_eq!(h.state().doc.undo_label(), Some("Make same width"));
    press(&mut h, HEIGHT);
    assert!(close(shapes(&h)[0].y2 - shapes(&h)[0].y1, 0.2), "{:?}", shapes(&h)[0]);
    assert!(close(shapes(&h)[0].y1, before[0].y1), "the top edge stays");
    assert_eq!(h.state().doc.undo_label(), Some("Make same height"));
    // Already the same: no change and no history.
    let revision = h.state().doc.revision();
    press(&mut h, WIDTH);
    assert_eq!(h.state().doc.revision(), revision);
    // One undo step per button.
    ctrl(&mut h, egui::Key::Z);
    ctrl(&mut h, egui::Key::Z);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(shapes(&h), before);
    // Undo changed the shapes, so their index-based selection is gone (as for one shape).
    assert!(graph_ui(&h).multi_selection().len() < 2);
    let at = frame_point(&h, 0.2, 0.17);
    click_at(&mut h, at);
    let at = frame_point(&h, 0.65, 0.5);
    shift_click_at(&mut h, at);
    assert_eq!(graph_ui(&h).multi_selection(), vec![(0, a), (0, b)]);
    // Shift+click on a selected object takes it out; a plain click selects one object again.
    shift_click_at(&mut h, at);
    assert_eq!(graph_ui(&h).multi_selection(), vec![(0, a)]);
    assert!(!enabled(&h, LEFT));
    shift_click_at(&mut h, at);
    assert_eq!(graph_ui(&h).multi_selection().len(), 2);
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).multi_selection(), vec![(0, b)]);
}

#[test]
fn a_text_label_can_be_the_reference_or_be_moved() {
    let mut h = setup("label");
    let id = graph(&h).id;
    let label = ObjectKind::Label(0);
    // The label last: the shape moves up to its top.
    {
        let state = h.state_mut().graphs.get_mut(&id).unwrap();
        state.also = vec![(0, ObjectKind::Shape(1))];
        state.selected = Some((0, label));
    }
    settle(&mut h);
    let labels_before = graph(&h).layers[0].labels.clone();
    press(&mut h, TOP);
    let drawn = bounds(&h);
    assert!(close(drawn[0].1, drawn[1].1), "{drawn:?}");
    assert_eq!(graph(&h).layers[0].labels, labels_before);
    // A text label keeps its size: Make Same Width with the shape as reference changes nothing.
    {
        let state = h.state_mut().graphs.get_mut(&id).unwrap();
        state.also = vec![(0, label)];
        state.selected = Some((0, ObjectKind::Shape(1)));
    }
    settle(&mut h);
    let revision = h.state().doc.revision();
    press(&mut h, WIDTH);
    assert_eq!(h.state().doc.revision(), revision);
    // ...but it moves: its centre goes onto the shape's vertical axis.
    press(&mut h, CENTER_V);
    let drawn = bounds(&h);
    assert!(close(drawn[0].0 + drawn[0].2 / 2.0, drawn[1].0 + drawn[1].2 / 2.0), "{drawn:?}");
    assert_ne!(graph(&h).layers[0].labels[0].x, labels_before[0].x);
}

#[test]
fn deleting_a_shape_ends_a_selection_that_names_shapes_by_index() {
    let mut h = setup("delete");
    let id = graph(&h).id;
    {
        let state = h.state_mut().graphs.get_mut(&id).unwrap();
        state.also = vec![(0, ObjectKind::Shape(0))];
        state.selected = Some((0, ObjectKind::Label(0)));
    }
    settle(&mut h);
    assert!(enabled(&h, LEFT));
    h.state_mut().edit_graph(id, GraphOp::DeleteShape { layer: 0, shape: 1 });
    settle(&mut h);
    assert_eq!(graph_ui(&h).multi_selection(), vec![(0, ObjectKind::Label(0))]);
    assert!(!enabled(&h, LEFT));
}
