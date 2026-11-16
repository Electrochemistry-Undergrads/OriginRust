//! Graph > Add Straight Line... (T67).

use super::*;
use sheaf_core::straight_line::StraightLine;

fn lines(h: &Harness<'static, SheafApp>) -> Vec<StraightLine> {
    graph(h).layers[0].straight_lines.clone()
}

/// The first layer's X and Y range.
fn ranges(h: &Harness<'static, SheafApp>) -> ((f64, f64), (f64, f64)) {
    let g = &sheaf_plot::layout_geometry(project(h), graph(h).id).unwrap()[0];
    ((g.x.min, g.x.max), (g.y.min, g.y.max))
}

/// The screen position of the data point `(x, y)` of the first layer.
fn data_point(h: &Harness<'static, SheafApp>, x: f64, y: f64) -> egui::Pos2 {
    let g = &sheaf_plot::layout_geometry(project(h), graph(h).id).unwrap()[0];
    let (px, py) = g.data_to_page(x, y);
    graph_ui(h).page_point(px, py).unwrap()
}

fn open_dialog(h: &mut Harness<'static, SheafApp>) {
    menu(h, "Graph", "Add Straight Line...");
    assert!(h.state().straight_line_dialog.is_some(), "the Add Straight Line window opened");
}

#[test]
fn the_menu_opens_the_window_and_ok_adds_a_vertical_line_in_one_undo_step() {
    let mut h = start(vec![data_file("sline-add")]);
    assert!(lines(&h).is_empty());
    let ((x0, x1), (y0, y1)) = ranges(&h);
    let at = (x0 + x1) / 2.0;
    open_dialog(&mut h);
    type_into_text_window(&mut h, &format!("{at}"), true);
    click(&mut h, "OK");
    assert!(h.state().straight_line_dialog.is_none());
    let l = lines(&h);
    assert_eq!(l.len(), 1);
    assert!(l[0].vertical);
    assert!((l[0].value - at).abs() < 1e-9);
    assert_eq!(l[0].name, "vline");
    assert_eq!(l[0].span, Some((y0, y1)), "the ends are the Y range of the moment");
    assert!(l[0].show_label && l[0].selectable);
    assert_eq!(h.state().doc.undo_label(), Some("Add straight line"));
    menu(&mut h, "Edit", "Undo Add straight line");
    assert!(lines(&h).is_empty());
    menu(&mut h, "Edit", "Redo Add straight line");
    assert_eq!(lines(&h).len(), 1);
}

#[test]
fn a_horizontal_line_takes_the_x_range_and_the_next_name() {
    let mut h = start(vec![data_file("sline-h")]);
    let ((x0, x1), (y0, y1)) = ranges(&h);
    open_dialog(&mut h);
    click(&mut h, "Horizontal");
    type_into_text_window(&mut h, &format!("{}", (y0 + y1) / 2.0), true);
    click(&mut h, "OK");
    let l = lines(&h);
    assert_eq!(l.len(), 1);
    assert!(!l[0].vertical);
    assert_eq!(l[0].name, "hline");
    assert_eq!(l[0].span, Some((x0, x1)));
    // A second vertical line is named vline, a third vline1.
    for want in ["vline", "vline1"] {
        open_dialog(&mut h);
        click(&mut h, "OK");
        assert_eq!(lines(&h).last().unwrap().name, want);
    }
    assert_eq!(lines(&h).len(), 3);
}

#[test]
fn cancel_and_a_value_that_is_not_a_number_add_nothing() {
    let mut h = start(vec![data_file("sline-cancel")]);
    let before = h.state().doc.undo_label().map(str::to_string);
    open_dialog(&mut h);
    click(&mut h, "Cancel");
    assert!(h.state().straight_line_dialog.is_none());
    assert!(lines(&h).is_empty());
    open_dialog(&mut h);
    type_into_text_window(&mut h, "abc", true);
    click(&mut h, "OK");
    assert!(h.state().straight_line_dialog.is_some(), "the window stays with its message");
    assert!(lines(&h).is_empty());
    click(&mut h, "Cancel");
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before);
}

#[test]
fn a_click_selects_a_line_and_delete_removes_it_in_one_undo_step() {
    let mut h = start(vec![data_file("sline-del")]);
    let ((x0, x1), (y0, y1)) = ranges(&h);
    let at = (x0 + x1) / 2.0;
    open_dialog(&mut h);
    type_into_text_window(&mut h, &format!("{at}"), true);
    click(&mut h, "OK");
    // the label above the frame (Top): nothing else is there; on the frame plots and axes come first
    let on_line = data_point(&h, at, y1 + 0.05 * (y1 - y0));
    click_at(&mut h, on_line);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::StraightLine(0))));
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert!(lines(&h).is_empty());
    assert_eq!(h.state().doc.undo_label(), Some("Delete straight line"));
    menu(&mut h, "Edit", "Undo Delete straight line");
    assert_eq!(lines(&h).len(), 1);
}

#[test]
fn a_line_that_is_not_selectable_takes_no_click() {
    let mut h = start(vec![data_file("sline-nosel")]);
    let ((x0, x1), (y0, y1)) = ranges(&h);
    let at = (x0 + x1) / 2.0;
    let g = graph(&h).id;
    h.state_mut().doc.apply("Add straight line", |p| {
        let l = StraightLine { selectable: false, ..StraightLine::new(true, at) };
        p.graph_mut(g).unwrap().layers[0].straight_lines.push(l);
    });
    settle(&mut h);
    // the label above the frame (Top): nothing else is there; on the frame plots and axes come first
    let on_line = data_point(&h, at, y1 + 0.05 * (y1 - y0));
    click_at(&mut h, on_line);
    assert_ne!(graph_ui(&h).selected, Some((0, ObjectKind::StraightLine(0))));
}

#[test]
fn double_click_edits_the_line_in_one_undo_step() {
    let mut h = start(vec![data_file("sline-edit")]);
    let ((x0, x1), (y0, y1)) = ranges(&h);
    let at = (x0 + x1) / 2.0;
    open_dialog(&mut h);
    type_into_text_window(&mut h, &format!("{at}"), true);
    click(&mut h, "OK");
    // the label above the frame (Top): nothing else is there; on the frame plots and axes come first
    let on_line = data_point(&h, at, y1 + 0.05 * (y1 - y0));
    double_click_at(&mut h, on_line);
    assert!(h.state().straight_line_dialog.is_some(), "double click opens the window");
    let new = x0 + (x1 - x0) * 0.25;
    type_into_text_window(&mut h, &format!("{new}"), true);
    click(&mut h, "OK");
    assert!(h.state().straight_line_dialog.is_none());
    let l = lines(&h);
    assert_eq!(l.len(), 1, "OK replaces the line");
    assert!((l[0].value - new).abs() < 1e-9);
    assert_eq!(l[0].name, "vline");
    assert_eq!(h.state().doc.undo_label(), Some("Edit straight line"));
    menu(&mut h, "Edit", "Undo Edit straight line");
    assert!((lines(&h)[0].value - at).abs() < 1e-9);
}
