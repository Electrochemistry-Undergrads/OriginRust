use super::*;
use crate::actions::{DataTool, GraphTool};
use sheaf_core::RecalcMode;

/// The screen position of a row of a plot as the graph draws it (with the plot's offsets and multipliers).
fn point_pos(h: &Harness<'static, SheafApp>, plot: usize, row: usize) -> egui::Pos2 {
    let (_, g) = window_ids(h);
    let geometry = sheaf_plot::layout_geometry(project(h), g).unwrap();
    let place = crate::reader::placement(project(h), g, 0, plot);
    let (x, y) = crate::gadget::plot_xy(project(h), g, 0, plot).unwrap();
    let (px, py) = geometry[0].data_to_page(place.x(x[row]), place.y(y[row]));
    graph_ui(h).page_point(px, py).expect("the graph was drawn")
}

/// A screen position inside the frame of the first layer that is far from every drawn point.
fn empty_spot(h: &Harness<'static, SheafApp>) -> egui::Pos2 {
    let (_, g) = window_ids(h);
    let f = sheaf_plot::layout_geometry(project(h), g).unwrap()[0].frame;
    let points: Vec<egui::Pos2> =
        (0..2).flat_map(|p| (0..4).map(move |r| (p, r))).map(|(p, r)| point_pos(h, p, r)).collect();
    let mut best = (0.0_f32, egui::Pos2::ZERO);
    for i in 1..20 {
        for j in 1..20 {
            let at = graph_ui(h).page_point(f.x + f.w * f64::from(i) / 20.0, f.y + f.h * f64::from(j) / 20.0).unwrap();
            let near = points.iter().map(|p| p.distance(at)).fold(f32::MAX, f32::min);
            if near > best.0 {
                best = (near, at);
            }
        }
    }
    assert!(best.0 > 45.0, "a spot far from the points: {best:?}");
    best.1
}

/// The screen position of a display coordinate of the first layer.
fn data_pos(h: &Harness<'static, SheafApp>, x: f64, y: f64) -> egui::Pos2 {
    let (_, g) = window_ids(h);
    let geometry = sheaf_plot::layout_geometry(project(h), g).unwrap();
    let (px, py) = geometry[0].data_to_page(x, y);
    graph_ui(h).page_point(px, py).expect("the graph was drawn")
}

fn mask_of(h: &Harness<'static, SheafApp>, plot: usize) -> Vec<usize> {
    let y = graph(h).layers[0].plots[plot].y;
    project(h).column(y).unwrap().mask.iter().copied().collect()
}

fn data_menu(h: &mut Harness<'static, SheafApp>, item: &str) {
    menu(h, "Data", item);
}

/// Scatter plots, so that every plot kind in these tests is one the tools accept.
fn scatter(h: &mut Harness<'static, SheafApp>) {
    edit_graph(h, |g| g.layers[0].plots.iter_mut().for_each(|p| p.kind = PlotKind::Scatter));
}

#[test]
fn the_three_data_menu_items_are_enabled_start_a_mode_and_name_it_in_the_status_bar() {
    let mut h = start(vec![data_file("gdt-menu")]);
    for (item, tool, words) in [
        ("Mask Data Points", DataTool::Mask, "drag a rectangle"),
        ("Move Data Points", DataTool::Move, "drag a point"),
        ("Remove Bad Data Points", DataTool::RemoveBad, "click a point"),
    ] {
        data_menu(&mut h, item);
        assert_eq!(graph_ui(&h).tool(), GraphTool::Data(tool), "{item}");
        assert!(h.state().status.contains(words) && h.state().status.contains("Esc"), "{}", h.state().status);
    }
    assert!(graph_ui(&h).data_tool.is_some() && !graph_ui(&h).reader, "one mode at a time");
    h.get_by_label("Tools toolbar: Pointer").click();
    settle(&mut h);
    assert_eq!(graph_ui(&h).tool(), GraphTool::Pointer, "the Pointer button ends the mode");
    assert!(graph_ui(&h).data_tool.is_none());
    data_menu(&mut h, "Mask Data Points");
    h.get_by_label("Tools toolbar: Data Reader").click();
    settle(&mut h);
    assert_eq!(graph_ui(&h).tool(), GraphTool::Reader, "another tool replaces the mode");
}

#[test]
fn esc_leaves_a_data_menu_mode_and_the_click_then_selects_as_usual() {
    let mut h = start(vec![data_file("gdt-esc")]);
    scatter(&mut h);
    data_menu(&mut h, "Remove Bad Data Points");
    let before = h.state().doc.undo_label().map(str::to_string);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(graph_ui(&h).data_tool.is_none());
    let arg0 = point_pos(&h, 0, 1);
    click_at(&mut h, arg0);
    assert!(column(&h, "B")[1].is_finite(), "the mode was left, so the click removed nothing");
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before);
}

#[test]
fn dragging_a_rectangle_masks_exactly_the_y_cells_of_the_points_inside_in_one_undo_step() {
    let mut h = start(vec![data_file("gdt-mask")]);
    scatter(&mut h);
    data_menu(&mut h, "Mask Data Points");
    let (a, b) = (point_pos(&h, 0, 1), point_pos(&h, 0, 2));
    let pad = egui::vec2(8.0, 8.0);
    drag(&mut h, a.min(b) - pad, a.max(b) + pad);
    assert_eq!(mask_of(&h, 0), vec![1, 2], "the two points inside, in the plot's Y column");
    assert!(mask_of(&h, 1).is_empty(), "another plot's column is not touched");
    let book = project(&h).workbooks().next().unwrap();
    assert!(book.sheet().column_by_name("A").unwrap().mask.is_empty(), "the X cells are not masked");
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5], "values stay");
    assert_eq!(h.state().doc.undo_label(), Some("Mask 2 points of B(Y)"));
    assert!(graph_ui(&h).data_tool.is_some(), "the tool stays on");
    menu(&mut h, "Edit", "Undo Mask 2 points of B(Y)");
    assert!(mask_of(&h, 0).is_empty());
    menu(&mut h, "Edit", "Redo Mask 2 points of B(Y)");
    assert_eq!(mask_of(&h, 0), vec![1, 2]);
}

#[test]
fn a_rectangle_over_no_point_does_nothing_and_a_click_on_a_masked_point_unmasks_it() {
    let mut h = start(vec![data_file("gdt-unmask")]);
    scatter(&mut h);
    data_menu(&mut h, "Mask Data Points");
    let p = point_pos(&h, 0, 1);
    let empty = h.state().doc.undo_label().map(str::to_string);
    drag(&mut h, p + egui::vec2(60.0, 60.0), p + egui::vec2(80.0, 80.0));
    assert!(mask_of(&h, 0).is_empty());
    assert_eq!(h.state().doc.undo_label().map(str::to_string), empty, "no undo step");
    click_at(&mut h, p);
    assert!(mask_of(&h, 0).is_empty(), "a click on a point that is not masked does nothing");
    assert_eq!(h.state().doc.undo_label().map(str::to_string), empty);
    drag(&mut h, p - egui::vec2(8.0, 8.0), p + egui::vec2(8.0, 8.0));
    assert_eq!(mask_of(&h, 0), vec![1]);
    click_at(&mut h, p);
    assert!(mask_of(&h, 0).is_empty(), "the masked point is unmasked");
    assert_eq!(h.state().doc.undo_label(), Some("Unmask 1 point of B(Y)"));
    menu(&mut h, "Edit", "Undo Unmask 1 point of B(Y)");
    assert_eq!(mask_of(&h, 0), vec![1], "undo masks it again");
}

#[test]
fn masked_points_that_are_hidden_and_other_plot_kinds_cannot_be_picked() {
    let mut h = start(vec![data_file("gdt-hidden")]);
    scatter(&mut h);
    data_menu(&mut h, "Mask Data Points");
    let p = point_pos(&h, 0, 1);
    drag(&mut h, p - egui::vec2(8.0, 8.0), p + egui::vec2(8.0, 8.0));
    assert_eq!(mask_of(&h, 0), vec![1]);
    h.state_mut().doc.apply("Hide masked points", |p| p.hide_masked = true);
    settle(&mut h);
    click_at(&mut h, p);
    assert_eq!(mask_of(&h, 0), vec![1], "a point that is not drawn cannot be unmasked from the graph");
    h.state_mut().doc.apply("Show masked points", |p| p.hide_masked = false);
    edit_graph(&mut h, |g| g.layers[0].plots.iter_mut().for_each(|p| p.kind = PlotKind::Column));
    let q = point_pos(&h, 0, 2);
    drag(&mut h, q - egui::vec2(8.0, 8.0), q + egui::vec2(8.0, 8.0));
    assert_eq!(mask_of(&h, 0), vec![1], "a column plot has no single point to mask");
}

#[test]
fn remove_bad_data_points_makes_the_clicked_y_cell_missing_and_keeps_the_row() {
    let mut h = start(vec![data_file("gdt-remove")]);
    scatter(&mut h);
    data_menu(&mut h, "Remove Bad Data Points");
    let arg0 = point_pos(&h, 0, 2) + egui::vec2(3.0, -3.0);
    click_at(&mut h, arg0);
    let b = column(&h, "B");
    assert_eq!(b.len(), 4, "the row stays");
    assert!(b[2].is_nan() && b[1] == 4.5 && b[3] == 8.5);
    assert_eq!(column(&h, "A"), [1.0, 2.0, 3.0, 4.0], "the X cell stays");
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5], "another plot's cell stays");
    assert_eq!(h.state().doc.undo_label(), Some("Remove bad point B(Y) row 3"));
    // The next click, on a point of the other plot, uses the same mode.
    let arg0 = point_pos(&h, 1, 0);
    click_at(&mut h, arg0);
    assert!(column(&h, "C")[0].is_nan());
    assert_eq!(h.state().doc.undo_label(), Some("Remove bad point C(Y) row 1"));
    menu(&mut h, "Edit", "Undo Remove bad point C(Y) row 1");
    menu(&mut h, "Edit", "Undo Remove bad point B(Y) row 3");
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5]);
}

#[test]
fn a_click_far_from_every_point_removes_nothing() {
    let mut h = start(vec![data_file("gdt-far")]);
    scatter(&mut h);
    data_menu(&mut h, "Remove Bad Data Points");
    let before = h.state().doc.undo_label().map(str::to_string);
    let spot = empty_spot(&h);
    click_at(&mut h, spot);
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
}

#[test]
fn move_data_points_drags_a_point_to_the_data_coordinates_under_the_pointer() {
    let mut h = start(vec![data_file("gdt-move")]);
    scatter(&mut h);
    data_menu(&mut h, "Move Data Points");
    let (tx, ty) = (2.4, 6.0);
    let arg0 = point_pos(&h, 0, 1);
    let arg1 = data_pos(&h, tx, ty);
    drag(&mut h, arg0, arg1);
    let (a, b) = (column(&h, "A"), column(&h, "B"));
    // One screen pixel is well under 0.5 % of the axis.
    assert!((a[1] - tx).abs() < 0.03 && (b[1] - ty).abs() < 0.1, "moved to {} {}", a[1], b[1]);
    assert_eq!((a[0], a[2], a[3]), (1.0, 3.0, 4.0), "other X cells stay");
    assert_eq!((b[0], b[2], b[3]), (2.5, 6.5, 8.5), "other Y cells stay");
    assert_eq!(h.state().doc.undo_label(), Some("Move point B(Y) row 2"));
    menu(&mut h, "Edit", "Undo Move point B(Y) row 2");
    assert_eq!((column(&h, "A")[1], column(&h, "B")[1]), (2.0, 4.5), "X and Y come back in one step");
    menu(&mut h, "Edit", "Redo Move point B(Y) row 2");
    assert_eq!(column(&h, "A")[1], a[1]);
}

#[test]
fn a_click_or_a_drag_that_starts_away_from_a_point_moves_nothing() {
    let mut h = start(vec![data_file("gdt-move-none")]);
    scatter(&mut h);
    data_menu(&mut h, "Move Data Points");
    let before = h.state().doc.undo_label().map(str::to_string);
    let arg0 = point_pos(&h, 0, 1);
    click_at(&mut h, arg0);
    let away = empty_spot(&h);
    drag(&mut h, away, away + egui::vec2(20.0, -20.0));
    assert_eq!(column(&h, "A"), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before);
}

#[test]
fn moving_a_point_on_a_log_axis_writes_the_value_under_the_pointer() {
    let csv = csv_file("gdt-log", "decades", "x,y\n1,5\n10,50\n100,500\n1000,5000\n");
    let mut h = start(vec![csv]);
    scatter(&mut h);
    edit_graph(&mut h, |g| {
        g.layers[0].x_axis.scale = ScaleKind::Log10;
        g.layers[0].y_axis.scale = ScaleKind::Log10;
    });
    data_menu(&mut h, "Move Data Points");
    let arg0 = point_pos(&h, 0, 1);
    let arg1 = data_pos(&h, 300.0, 20.0);
    drag(&mut h, arg0, arg1);
    let (x, y) = (column(&h, "A")[1], column(&h, "B")[1]);
    assert!((x / 300.0 - 1.0).abs() < 0.02, "X {x}");
    assert!((y / 20.0 - 1.0).abs() < 0.02, "Y {y}");
    assert_eq!(column(&h, "A")[2], 100.0, "other rows stay");
    assert_eq!(h.state().doc.undo_label(), Some("Move point B(Y) row 2"));
}

#[test]
fn moving_a_scaled_and_offset_plot_writes_the_original_values_not_the_displayed_ones() {
    let mut h = start(vec![data_file("gdt-offset")]);
    scatter(&mut h);
    edit_graph(&mut h, |g| {
        let layer = &mut g.layers[0];
        layer.stack = Stack::Individual;
        layer.plots[1].visible = false;
        layer.x_axis.min = Some(0.0);
        layer.x_axis.max = Some(20.0);
        layer.y_axis.min = Some(0.0);
        layer.y_axis.max = Some(60.0);
        let y = layer.plots[0].y;
        let o = layer.plot_offset_mut(y);
        (o.x, o.y, o.x_multiplier, o.y_multiplier) = (5.0, 7.0, 2.0, 3.0);
    });
    // Row 1 is drawn at X = 2 * 2 + 5 = 9, Y = 4.5 * 3 + 7 = 20.5.
    let from = point_pos(&h, 0, 1);
    assert!(from.distance(data_pos(&h, 9.0, 20.5)) < 0.01);
    data_menu(&mut h, "Move Data Points");
    let arg1 = data_pos(&h, 11.0, 31.0);
    drag(&mut h, from, arg1);
    let (x, y) = (column(&h, "A")[1], column(&h, "B")[1]);
    assert!((x - 3.0).abs() < 0.03, "X {x}: (11 - 5) / 2");
    assert!((y - 8.0).abs() < 0.1, "Y {y}: (31 - 7) / 3");
}

#[test]
fn rows_of_a_plot_without_an_x_column_move_in_y_only() {
    let mut h = start(vec![data_file("gdt-no-x")]);
    scatter(&mut h);
    edit_graph(&mut h, |g| g.layers[0].plots[0].x = None);
    data_menu(&mut h, "Move Data Points");
    let arg0 = point_pos(&h, 0, 1);
    let arg1 = point_pos(&h, 0, 1) + egui::vec2(0.0, -30.0);
    drag(&mut h, arg0, arg1);
    assert_eq!(column(&h, "A"), [1.0, 2.0, 3.0, 4.0], "no X cell to change");
    assert!(column(&h, "B")[1] > 4.5, "up on the screen is a larger Y");
    assert_eq!(h.state().doc.undo_label(), Some("Move point B(Y) row 2"));
}

#[test]
fn a_column_that_a_formula_keeps_up_to_date_refuses_remove_and_move_but_can_be_masked() {
    let mut h = start(vec![data_file("gdt-locked")]);
    scatter(&mut h);
    let c = graph(&h).layers[0].plots[1].y;
    h.state_mut().doc.edit("Formula", |p| p.set_column_values_with(c, "col(B)*2", None, RecalcMode::Auto)).unwrap();
    settle(&mut h);
    assert_eq!(column(&h, "C"), [5.0, 9.0, 13.0, 17.0]);
    data_menu(&mut h, "Remove Bad Data Points");
    let before = h.state().doc.undo_label().map(str::to_string);
    let arg0 = point_pos(&h, 1, 2);
    click_at(&mut h, arg0);
    assert_eq!(column(&h, "C"), [5.0, 9.0, 13.0, 17.0]);
    assert!(h.state().status.contains("calculated from a formula"), "{}", h.state().status);
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before, "no undo step");
    data_menu(&mut h, "Move Data Points");
    let arg0 = point_pos(&h, 1, 2);
    let arg1 = point_pos(&h, 1, 2) + egui::vec2(0.0, -30.0);
    drag(&mut h, arg0, arg1);
    assert_eq!(column(&h, "C"), [5.0, 9.0, 13.0, 17.0]);
    assert_eq!(column(&h, "A"), [1.0, 2.0, 3.0, 4.0], "all or nothing: the X cell did not move either");
    data_menu(&mut h, "Mask Data Points");
    let p = point_pos(&h, 1, 2);
    drag(&mut h, p - egui::vec2(8.0, 8.0), p + egui::vec2(8.0, 8.0));
    assert_eq!(mask_of(&h, 1), vec![2], "a mask is not a typed value");
}

#[test]
fn a_masked_point_changes_an_auto_analysis_result_and_removing_a_point_recalculates_a_formula() {
    let mut h = start(vec![data_file("gdt-follow")]);
    scatter(&mut h);
    let (book, _) = window_ids(&h);
    let new = |p: &mut sheaf_core::Project| {
        p.add_column(book, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(Vec::new()))
    };
    let d = h.state_mut().doc.edit("New column", new).unwrap();
    h.state_mut().doc.edit("Formula", |p| p.set_column_values_with(d, "col(B)+1", None, RecalcMode::Auto)).unwrap();
    settle(&mut h);
    assert_eq!(project(&h).numeric(d).unwrap(), [3.5, 5.5, 7.5, 9.5]);
    data_menu(&mut h, "Remove Bad Data Points");
    let arg0 = point_pos(&h, 0, 3);
    click_at(&mut h, arg0);
    let after = project(&h).numeric(d).unwrap().to_vec();
    assert_eq!(&after[..3], [3.5, 5.5, 7.5]);
    assert!(after[3].is_nan(), "the formula column follows");
    menu(&mut h, "Edit", "Undo Remove bad point B(Y) row 4");
    assert_eq!(project(&h).numeric(d).unwrap(), [3.5, 5.5, 7.5, 9.5], "source and result come back together");
}

#[test]
fn the_tools_do_nothing_on_a_graph_without_a_plot() {
    let mut h = start(vec![data_file("gdt-empty")]);
    edit_graph(&mut h, |g| g.layers[0].plots.clear());
    let before = h.state().doc.undo_label().map(str::to_string);
    let middle = data_pos(&h, 2.5, 5.0);
    for item in ["Mask Data Points", "Move Data Points", "Remove Bad Data Points"] {
        data_menu(&mut h, item);
        click_at(&mut h, middle);
        drag(&mut h, middle, middle + egui::vec2(60.0, 40.0));
        assert_eq!(h.state().doc.undo_label().map(str::to_string), before, "{item}");
    }
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
}

#[test]
fn a_drag_in_progress_shows_the_rectangle_and_leaves_the_data_alone_until_it_is_let_go() {
    let mut h = start(vec![data_file("gdt-overlay")]);
    scatter(&mut h);
    data_menu(&mut h, "Mask Data Points");
    let (a, b) = (point_pos(&h, 0, 1), point_pos(&h, 0, 2));
    let (from, to) = (a.min(b) - egui::vec2(8.0, 8.0), a.max(b) + egui::vec2(8.0, 8.0));
    h.event(egui::Event::PointerMoved(from));
    h.step();
    h.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.step();
    h.event(egui::Event::PointerMoved(from.lerp(to, 0.5)));
    h.step();
    h.event(egui::Event::PointerMoved(to));
    settle(&mut h);
    assert!(graph_ui(&h).data_dragging(), "the rectangle is being dragged");
    assert!(mask_of(&h, 0).is_empty(), "nothing is written before the button is let go");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!graph_ui(&h).data_dragging() && graph_ui(&h).data_tool.is_none(), "Esc drops the drag and the mode");
    assert!(mask_of(&h, 0).is_empty());
}

/// Where a point is drawn in a waterfall of 5 % and 10 %: member `plot` is moved right and up by that
/// many shifts of the front frame.
fn waterfall_pos(h: &Harness<'static, SheafApp>, plot: usize, x: f64, y: f64) -> egui::Pos2 {
    let (_, g) = window_ids(h);
    let f = sheaf_plot::layout_geometry(project(h), g).unwrap()[0].frame;
    let (px, py) = sheaf_plot::layout_geometry(project(h), g).unwrap()[0].data_to_page(x, y);
    let k = plot as f64;
    graph_ui(h).page_point(px + k * 0.05 * f.w, py - k * 0.10 * f.h).expect("the graph was drawn")
}

/// Two plots as a waterfall with fixed axes (X 0..5, Y 0..40).
fn waterfall(h: &mut Harness<'static, SheafApp>) {
    scatter(h);
    edit_graph(h, |g| {
        let layer = &mut g.layers[0];
        layer.stack = Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 };
        (layer.x_axis.min, layer.x_axis.max) = (Some(0.0), Some(5.0));
        (layer.y_axis.min, layer.y_axis.max) = (Some(0.0), Some(40.0));
    });
}

#[test]
fn a_waterfall_curve_is_picked_where_it_is_drawn_not_where_the_front_curve_would_be() {
    let mut h = start(vec![data_file("gdt-waterfall")]);
    waterfall(&mut h);
    // Mask: a rectangle around the drawn points 1 and 2 of the second curve masks only that column.
    data_menu(&mut h, "Mask Data Points");
    let (a, b) = (waterfall_pos(&h, 1, 2.0, 20.5), waterfall_pos(&h, 1, 3.0, 10.5));
    let pad = egui::vec2(6.0, 6.0);
    drag(&mut h, a.min(b) - pad, a.max(b) + pad);
    assert_eq!(mask_of(&h, 1), vec![1, 2]);
    assert!(mask_of(&h, 0).is_empty(), "the front curve is not touched");
    // Remove Bad: a click on a drawn point of the second curve removes that curve's value.
    data_menu(&mut h, "Remove Bad Data Points");
    let at = waterfall_pos(&h, 1, 1.0, 30.5);
    click_at(&mut h, at);
    assert!(column(&h, "C")[0].is_nan());
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    // Move: the point lands under the pointer in the waterfall's own frame.
    data_menu(&mut h, "Move Data Points");
    let (from, to) = (waterfall_pos(&h, 1, 4.0, 0.5), waterfall_pos(&h, 1, 4.5, 12.0));
    drag(&mut h, from, to);
    let (x, y) = (column(&h, "A")[3], column(&h, "C")[3]);
    assert!((x - 4.5).abs() < 0.05 && (y - 12.0).abs() < 0.3, "moved to {x} {y}");
}

#[test]
fn rows_that_skip_points_leaves_without_a_symbol_cannot_be_picked() {
    let mut h = start(vec![data_file("gdt-skip")]);
    scatter(&mut h);
    edit_graph(&mut h, |g| g.layers[0].plots[0].symbol.every = 2);
    // Symbols are on rows 1 and 3 (counted from 1) of the first plot; rows 2 and 4 have none.
    data_menu(&mut h, "Remove Bad Data Points");
    let at = point_pos(&h, 0, 1);
    click_at(&mut h, at);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5], "a row without a symbol is not removed");
    data_menu(&mut h, "Mask Data Points");
    let (a, b) = (point_pos(&h, 0, 0), point_pos(&h, 0, 3));
    let pad = egui::vec2(8.0, 8.0);
    drag(&mut h, a.min(b) - pad, a.max(b) + pad);
    assert_eq!(mask_of(&h, 0), vec![0, 2], "only the rows with a symbol are masked");
}

#[test]
fn a_layer_that_shows_no_data_has_no_points_to_pick() {
    let mut h = start(vec![data_file("gdt-no-data")]);
    scatter(&mut h);
    edit_graph(&mut h, |g| g.layers[0].display.show_data = false);
    data_menu(&mut h, "Remove Bad Data Points");
    let at = point_pos(&h, 0, 1);
    click_at(&mut h, at);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    data_menu(&mut h, "Mask Data Points");
    let a = point_pos(&h, 0, 1);
    drag(&mut h, a - egui::vec2(8.0, 8.0), a + egui::vec2(8.0, 8.0));
    assert!(mask_of(&h, 0).is_empty());
}

#[test]
fn points_beyond_the_axis_range_are_clipped_and_cannot_be_picked() {
    let mut h = start(vec![data_file("gdt-clipped")]);
    scatter(&mut h);
    // Row 4 (X = 4) lies just right of the frame, within the reach of a click in the margin.
    edit_graph(&mut h, |g| (g.layers[0].x_axis.min, g.layers[0].x_axis.max) = (Some(0.0), Some(3.9)));
    let out = point_pos(&h, 0, 3);
    let (_, g) = window_ids(&h);
    let f = sheaf_plot::layout_geometry(project(&h), g).unwrap()[0].frame;
    let inside = graph_ui(&h).page_point(f.x + f.w, f.y + f.h / 2.0).unwrap();
    assert!(out.x > inside.x, "the point is right of the frame");
    data_menu(&mut h, "Remove Bad Data Points");
    click_at(&mut h, out);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5], "the clipped point stays");
    data_menu(&mut h, "Mask Data Points");
    drag(&mut h, out - egui::vec2(8.0, 8.0), out + egui::vec2(8.0, 8.0));
    assert!(mask_of(&h, 0).is_empty());
    // A point inside the range is still picked.
    data_menu(&mut h, "Remove Bad Data Points");
    let at = point_pos(&h, 0, 2);
    click_at(&mut h, at);
    assert!(column(&h, "B")[2].is_nan());
    // With Clip Data to Frame off the point is drawn, so it is picked.
    edit_graph(&mut h, |g| g.layers[0].display.clip_data = false);
    click_at(&mut h, out);
    assert!(column(&h, "B")[3].is_nan());
}

#[test]
fn a_move_drag_that_returns_to_where_it_began_writes_nothing_and_a_drag_moves_by_its_length() {
    let mut h = start(vec![data_file("gdt-move-back")]);
    scatter(&mut h);
    data_menu(&mut h, "Move Data Points");
    let before = h.state().doc.undo_label().map(str::to_string);
    // Grab the point 20 points off its centre, still within reach, and drag away and back.
    let press = point_pos(&h, 0, 1) + egui::vec2(20.0, 0.0);
    let button = |pressed| egui::Event::PointerButton {
        pos: press,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(egui::Event::PointerMoved(press));
    h.step();
    h.event(button(true));
    h.step();
    h.event(egui::Event::PointerMoved(press + egui::vec2(0.0, 40.0)));
    h.step();
    h.event(egui::Event::PointerMoved(press));
    h.step();
    h.event(button(false));
    h.step();
    settle(&mut h);
    assert_eq!(column(&h, "A"), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before, "no undo step");
    // The same grab dragged 40 points up moves the point by those 40 points, not to the pointer.
    let old = point_pos(&h, 0, 1);
    drag(&mut h, press, press + egui::vec2(0.0, -40.0));
    let new = point_pos(&h, 0, 1);
    assert!((new.x - old.x).abs() < 0.5 && (new.y - (old.y - 40.0)).abs() < 0.5, "moved from {old:?} to {new:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Move point B(Y) row 2"));
}
