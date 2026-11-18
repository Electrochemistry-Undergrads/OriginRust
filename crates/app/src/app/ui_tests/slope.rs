use super::*;

fn slopes(h: &Harness<'static, SheafApp>) -> Vec<sheaf_core::graph::SlopeAnnotation> {
    graph(h).layers[0].slopes.clone()
}

/// The screen position of the data point `(x, y)` of the first layer.
fn data_point(h: &Harness<'static, SheafApp>, x: f64, y: f64) -> egui::Pos2 {
    let g = &sheaf_plot::layout_geometry(project(h), graph(h).id).unwrap()[0];
    let (px, py) = g.data_to_page(x, y);
    graph_ui(h).page_point(px, py).unwrap()
}

#[test]
fn two_clicks_with_the_slope_tool_add_one_annotation_and_delete_and_undo_work() {
    let mut h = start(vec![data_file("slope-tool")]);
    h.get_by_label("Tools toolbar: Slope").click();
    settle(&mut h);
    assert!(graph_ui(&h).slope_tool, "the tool is on");
    // The first click picks the point (t = 1), the second a point of the same plot (t = 4).
    let a = data_point(&h, 1.0, 2.5);
    click_at(&mut h, a + egui::vec2(3.0, 2.0));
    assert!(slopes(&h).is_empty(), "one point is not an annotation yet");
    assert_eq!(graph_ui(&h).slope_first.map(|p| p.row), Some(0));
    assert!(h.state().status.contains("second point"), "{}", h.state().status);
    let b = data_point(&h, 4.0, 8.5);
    click_at(&mut h, b);
    let s = slopes(&h);
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].row_a, s[0].row_b), (0, 3));
    assert_eq!(s[0].plot, graph(&h).layers[0].plots[0].y);
    assert!(s[0].show_legs && s[0].color.is_none());
    assert!(!graph_ui(&h).slope_tool, "the tool returns to the Pointer");
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Slope(0))));
    assert_eq!(h.state().doc.undo_label(), Some("Add slope annotation"));
    // Undo takes it away and Redo brings it back.
    menu(&mut h, "Edit", "Undo Add slope annotation");
    assert!(slopes(&h).is_empty());
    menu(&mut h, "Edit", "Redo Add slope annotation");
    assert_eq!(slopes(&h).len(), 1);
    // Select it with a click on its segment's middle, then Delete removes it; Undo restores it.
    let mid = data_point(&h, 2.5, 5.5);
    click_at(&mut h, mid);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Slope(0))));
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert!(slopes(&h).is_empty());
    assert_eq!(h.state().doc.undo_label(), Some("Delete slope annotation"));
    menu(&mut h, "Edit", "Undo Delete slope annotation");
    assert_eq!(slopes(&h).len(), 1);
}

#[test]
fn escape_cancels_the_slope_tool_and_a_click_far_from_data_picks_nothing() {
    let mut h = start(vec![data_file("slope-esc")]);
    h.get_by_label("Tools toolbar: Slope").click();
    settle(&mut h);
    let far = frame_point(&h, 0.5, 0.02);
    click_at(&mut h, far);
    assert!(graph_ui(&h).slope_first.is_none(), "no data point within reach");
    let second = data_point(&h, 2.0, 4.5);
    click_at(&mut h, second);
    assert!(graph_ui(&h).slope_first.is_some());
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!graph_ui(&h).slope_tool && graph_ui(&h).slope_first.is_none());
    assert!(slopes(&h).is_empty());
    assert_ne!(h.state().doc.undo_label(), Some("Add slope annotation"));
}

#[test]
fn double_clicking_an_annotation_opens_its_properties_and_the_change_is_one_undo_step() {
    let mut h = start(vec![data_file("slope-props")]);
    graph_mut_layer(&mut h);
    let at = data_point(&h, 2.5, 5.5);
    double_click_at(&mut h, at);
    assert!(h.state().slope_dialog.is_some(), "the properties window opened");
    click(&mut h, "Show Legs");
    click(&mut h, "OK");
    assert!(!slopes(&h)[0].show_legs);
    assert_eq!(h.state().doc.undo_label(), Some("Edit slope annotation"));
    menu(&mut h, "Edit", "Undo Edit slope annotation");
    assert!(slopes(&h)[0].show_legs);
}

/// Adds a slope between rows 1 and 4 to the first graph's plot.
fn graph_mut_layer(h: &mut Harness<'static, SheafApp>) {
    let y = graph(h).layers[0].plots[0].y;
    let g = graph(h).id;
    h.state_mut().doc.apply("Add slope annotation", |p| {
        p.graph_mut(g).unwrap().layers[0].slopes.push(sheaf_core::graph::SlopeAnnotation::new(y, 0, 3));
    });
    settle(h);
}

#[test]
fn the_slope_tool_ignores_unusable_plots_and_refuses_a_masked_second_point() {
    let mut h = start(vec![data_file("slope-refuse")]);
    let y = graph(&h).layers[0].plots[0].y;
    let g = graph(&h).id;
    // A plot without one point per row: the click is ignored and the status bar says why.
    h.state_mut().doc.apply("Histogram", |p| {
        p.graph_mut(g).unwrap().layers[0].plots[0].kind = sheaf_core::graph::PlotKind::Histogram;
    });
    settle(&mut h);
    h.get_by_label("Tools toolbar: Slope").click();
    settle(&mut h);
    let at = data_point(&h, 1.0, 2.5);
    click_at(&mut h, at);
    assert!(graph_ui(&h).slope_first.is_none(), "no first point on a histogram");
    assert!(h.state().status.contains("Slopes need a plot with one point per row"), "{}", h.state().status);
    assert!(slopes(&h).is_empty());
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // A masked second point is refused; the first point stays picked.
    h.state_mut().doc.apply("Scatter", |p| {
        p.graph_mut(g).unwrap().layers[0].plots[0].kind = sheaf_core::graph::PlotKind::Scatter;
        p.column_mut(y).unwrap().mask.insert(3);
    });
    settle(&mut h);
    h.get_by_label("Tools toolbar: Slope").click();
    settle(&mut h);
    let a = data_point(&h, 1.0, 2.5);
    click_at(&mut h, a);
    assert_eq!(graph_ui(&h).slope_first.map(|p| p.row), Some(0));
    let b = data_point(&h, 4.0, 8.5);
    click_at(&mut h, b);
    assert!(slopes(&h).is_empty(), "a masked point cannot end a slope");
    assert!(graph_ui(&h).slope_tool && graph_ui(&h).slope_first.is_some());
    assert!(h.state().status.contains("Row 4"), "{}", h.state().status);
}
