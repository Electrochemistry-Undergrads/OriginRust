use super::*;

/// The Gauss function of the library written out (Origin's form).
fn gauss(x: f64, y0: f64, xc: f64, w: f64, a: f64) -> f64 {
    y0 + a / (w * (std::f64::consts::PI / 2.0).sqrt()) * (-2.0 * ((x - xc) / w).powi(2)).exp()
}

fn open_window(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["Analysis", "Fitting", "Simulate Curve..."]);
    assert!(h.state().simulate.is_some());
}

/// Types a number into the field that is named by its label.
fn type_number(h: &mut Harness<'static, SheafApp>, name: &str, value: &str) {
    let at = h.get_by_label(name).rect().center();
    double_click_at(h, at);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(value.into()));
    h.key_press(egui::Key::Enter);
    settle(h);
}

fn type_gauss(h: &mut Harness<'static, SheafApp>) {
    for (name, value) in
        [("y0", "1"), ("xc", "5"), ("w", "2"), ("A", "4"), ("From", "0"), ("To", "10"), ("Points", "5")]
    {
        type_number(h, name, value);
    }
}

#[test]
fn simulate_curve_and_simulate_surface_are_in_the_fitting_menu() {
    let mut h = start(vec![data_file("simulate-menu")]);
    menu_path(&mut h, &["Analysis", "Fitting"]);
    assert!(entry_enabled(&h, "Simulate Curve..."));
    assert!(entry_enabled(&h, "Simulate Surface..."));
    click(&mut h, "Simulate Curve...");
    assert!(h.state().simulate.is_some());
    assert!(h.state().simulate.as_ref().unwrap().preview_for_test().is_some());
    click(&mut h, "Cancel");
    assert!(h.state().simulate.is_none());
}

#[test]
fn a_gauss_with_typed_parameters_goes_to_a_new_sheet_in_one_undo_step() {
    let mut h = start(vec![data_file("simulate-gauss")]);
    let before = project(&h).clone();
    open_window(&mut h);
    // Origin's window writes to a new workbook and draws a graph; here: a new sheet, no graph.
    h.get_by_label("Output").click();
    settle(&mut h);
    let into = h
        .query_all(By::new())
        .find(|n| n.accesskit_node().label().is_some_and(|l| l.starts_with("New sheet in")))
        .expect("the Output list offers the open workbook")
        .rect()
        .center();
    click_at(&mut h, into);
    settle(&mut h);
    click(&mut h, "Plot the curve");
    type_gauss(&mut h);
    assert_eq!(h.state().simulate.as_ref().unwrap().values_for_test(), [1.0, 5.0, 2.0, 4.0]);
    let revision = h.state().doc.revision();
    click(&mut h, "OK");
    assert!(h.state().simulate.is_none());
    assert_ne!(h.state().doc.revision(), revision);
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheets.len(), 2);
    let made = &book.sheets[1];
    assert_eq!(made.name, "Simulate1");
    assert_eq!(book.active_sheet, 1);
    assert_eq!(made.columns.iter().map(|c| c.designation).collect::<Vec<_>>(), [Designation::X, Designation::Y]);
    let xs = made.columns[0].numeric().unwrap();
    let ys = made.columns[1].numeric().unwrap();
    assert_eq!(xs, [0.0, 2.5, 5.0, 7.5, 10.0]);
    for (x, y) in xs.iter().zip(ys) {
        assert!((y - gauss(*x, 1.0, 5.0, 2.0, 4.0)).abs() < 1e-12, "x = {x}: {y}");
    }
    // Origin's names (fitting.md 13.1).
    assert_eq!((made.columns[1].long_name.as_str(), made.columns[1].comments.as_str()), ("Simulated Y1", "Gauss"));
    assert!(h.state().status.starts_with("Simulate curve"), "{}", h.state().status);
    assert_eq!(project(&h).graphs().count(), before.graphs().count(), "no graph without Plot");
    menu(&mut h, "Edit", "Undo Simulate curve");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Edit", "Redo Simulate curve");
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 2);
}

#[test]
fn another_function_starts_on_its_initial_values_and_the_preview_follows_the_window() {
    let mut h = start(vec![data_file("simulate-function")]);
    open_window(&mut h);
    let d = h.state().simulate.as_ref().unwrap();
    assert_eq!(d.preview_for_test(), Some(Ok(200)), "Origin's 200 points");
    h.state_mut().simulate.as_mut().unwrap().set_for_test("ExpDec1", 1.0, 10.0, 7, false);
    settle(&mut h);
    let d = h.state().simulate.as_ref().unwrap();
    assert_eq!(d.values_for_test().len(), 3, "y0, A and t1");
    assert_eq!(d.preview_for_test(), Some(Ok(7)));
}

#[test]
fn the_output_can_be_a_new_workbook_that_holds_only_the_curve() {
    let mut h = start(vec![data_file("simulate-newbook")]);
    open_window(&mut h);
    h.get_by_label("Output").click();
    settle(&mut h);
    click(&mut h, "New workbook");
    click(&mut h, "OK");
    let p = project(&h);
    assert_eq!(p.workbooks().count(), 2);
    let made = p.workbooks().last().unwrap();
    assert_eq!(made.sheets.len(), 1, "the empty first sheet is not left behind");
    assert_eq!(made.sheets[0].name, "Simulate1");
    assert_eq!(made.sheets[0].columns.len(), 2);
    assert_eq!(made.sheets[0].columns[1].numeric().unwrap().len(), 200);
    // As in Origin, the curve is also drawn, and its graph comes to the front.
    let graph = p.graphs().last().expect("Plot the curve is on by default");
    assert_eq!(h.state().active, Some(graph.id));
    menu(&mut h, "Edit", "Undo Simulate curve");
    assert_eq!(project(&h).workbooks().count(), 1);
}

#[test]
fn the_x_of_a_column_is_used_row_by_row() {
    let mut h = start(vec![data_file("simulate-column")]);
    let x = sheet(&h).columns[0].id;
    open_window(&mut h);
    h.state_mut().simulate.as_mut().unwrap().use_column_for_test(x);
    settle(&mut h);
    click(&mut h, "OK");
    let book = project(&h).workbooks().next().unwrap();
    let made = &book.sheets[1];
    assert_eq!(made.columns[0].numeric().unwrap(), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(made.columns[1].numeric().unwrap().len(), 4);
    assert!(made.columns[0].comments.starts_with("X of column"), "{}", made.columns[0].comments);
}

#[test]
fn plot_makes_one_graph_with_a_log_axis_for_log_spacing() {
    let mut h = start(vec![data_file("simulate-plot")]);
    let graphs = project(&h).graphs().count();
    open_window(&mut h);
    type_number(&mut h, "From", "0.1");
    type_number(&mut h, "To", "100");
    click(&mut h, "Logarithmic spacing");
    // "Plot the curve" is on by default, as Origin draws the curve.
    click(&mut h, "OK");
    assert_eq!(project(&h).graphs().count(), graphs + 1);
    let graph = project(&h).graphs().last().unwrap();
    assert_eq!(h.state().active, Some(graph.id));
    assert_eq!(graph.layers[0].plots.len(), 1);
    assert_eq!(graph.layers[0].plots[0].kind, PlotKind::Line);
    assert_eq!(graph.layers[0].x_axis.scale, ScaleKind::Log10);
    menu(&mut h, "Edit", "Undo Simulate curve");
    assert_eq!(project(&h).graphs().count(), graphs, "the sheet and the graph are one undo step");
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);
}

#[test]
fn a_range_that_cannot_be_used_is_refused_with_a_message_and_changes_nothing() {
    let mut h = start(vec![data_file("simulate-invalid")]);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open_window(&mut h);
    type_number(&mut h, "From", "1");
    type_number(&mut h, "To", "1");
    click(&mut h, "OK");
    assert!(h.state().simulate.is_some(), "the window stays open");
    let preview = h.state().simulate.as_ref().unwrap().preview_for_test().unwrap();
    assert!(preview.unwrap_err().contains("two different values"));
    let problem = h.state().simulate.as_ref().unwrap().problem_for_test();
    assert_eq!(problem, None, "OK is off, so the preview is the only message");
    type_number(&mut h, "To", "10");
    assert!(h.state().simulate.as_ref().unwrap().preview_for_test().unwrap().is_ok());
    type_number(&mut h, "Points", "1");
    click(&mut h, "OK");
    let preview = h.state().simulate.as_ref().unwrap().preview_for_test().unwrap();
    assert!(preview.unwrap_err().contains("too few"));
    type_number(&mut h, "Points", "20");
    type_number(&mut h, "From", "-1");
    click(&mut h, "Logarithmic spacing");
    click(&mut h, "OK");
    let preview = h.state().simulate.as_ref().unwrap().preview_for_test().unwrap();
    assert!(preview.unwrap_err().contains("positive"));
    assert_eq!(h.state().simulate.as_ref().unwrap().problem_for_test(), None);
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
}

/// Analysis > Fitting > Simulate Surface...: Origin's window opens on Gauss2D with its own values (fitting.md 16.1);
/// OK writes the grid to a new workbook as one undo step.
#[test]
fn simulate_surface_opens_on_gauss2d_and_writes_a_new_workbook_in_one_undo_step() {
    let mut h = start(vec![data_file("simsurface")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Analysis", "Fitting", "Simulate Surface..."]);
    let d = h.state().simulate_surface.as_ref().expect("the window is open");
    assert_eq!(d.function_for_test(), "Gauss2D");
    assert_eq!(d.values_for_test(), [0.0, 1.0, 0.0, 0.3, 0.0, 0.3], "z0, A, xc, w1, yc, w2");
    h.state_mut().simulate_surface.as_mut().unwrap().set_grid_for_test((-1.0, 1.0, 3), (-1.0, 1.0, 2), 0.0);
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().simulate_surface.is_none());
    let p = project(&h);
    assert_eq!(p.workbooks().count(), 2);
    let made = p.workbooks().last().unwrap();
    assert_eq!(made.sheets.len(), 1, "the empty first sheet is not left behind");
    let sheet = &made.sheets[0];
    assert_eq!(sheet.name, "SimulateSurface1");
    let names: Vec<&str> = sheet.columns.iter().map(|c| c.long_name.as_str()).collect();
    assert_eq!(names, ["Simulated X1", "Simulated Y1", "Simulated Z1"]);
    assert_eq!(
        sheet.columns.iter().map(|c| c.designation).collect::<Vec<_>>(),
        [Designation::X, Designation::Y, Designation::Z]
    );
    assert_eq!(sheet.columns[0].numeric().unwrap(), [-1.0, 0.0, 1.0, -1.0, 0.0, 1.0]);
    assert_eq!(sheet.columns[1].numeric().unwrap(), [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);
    let z = sheet.columns[2].numeric().unwrap();
    // Gauss2D at (-1, -1) and (0, -1): the numbers Origin gives at noise 0 (fitting.md 16.2).
    assert!((z[0] - 1.4945338524781e-05).abs() < 1e-17, "{}", z[0]);
    assert!((z[1] - 0.0038659201394728).abs() < 1e-15, "{}", z[1]);
    assert!(h.state().status.starts_with("Simulate surface"), "{}", h.state().status);
    menu(&mut h, "Edit", "Undo Simulate surface");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Edit", "Redo Simulate surface");
    assert_eq!(project(&h).workbooks().count(), 2);
}

#[test]
fn choosing_another_surface_function_brings_its_own_values_and_a_bad_grid_is_refused() {
    let mut h = start(vec![data_file("simsurface-other")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Analysis", "Fitting", "Simulate Surface..."]);
    h.state_mut().simulate_surface.as_mut().unwrap().choose_function_named("Rational2D");
    settle(&mut h);
    let d = h.state().simulate_surface.as_ref().unwrap();
    assert_eq!((d.function_for_test().as_str(), d.values_for_test().len()), ("Rational2D", 10));
    assert_eq!(d.values_for_test()[0], 100.0);
    h.state_mut().simulate_surface.as_mut().unwrap().set_grid_for_test((1.0, 1.0, 20), (-1.0, 1.0, 20), 0.0);
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().simulate_surface.is_some(), "OK is off while the grid is wrong");
    assert_eq!(project(&h), &before);
    click(&mut h, "Cancel");
    assert!(h.state().simulate_surface.is_none());
    assert_eq!(project(&h), &before);
}
