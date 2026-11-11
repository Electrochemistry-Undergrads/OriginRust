//! Tests that drive the application like a user: they click menu entries,
//! column headers and dialog buttons and type into fields, without a real
//! window. They guard the wiring between the interface and the project
//! (which click leads to which change), not the look.

use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sheaf_core::graph::ScaleKind;

/// Writes a small data file into a fresh temporary folder and returns its path.
fn data_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.csv");
    std::fs::write(&path, "t,up,down\n1,2.5,30.5\n2,4.5,20.5\n3,6.5,10.5\n4,8.5,0.5\n").unwrap();
    path
}

fn start(paths: Vec<PathBuf>) -> Harness<'static, SheafApp> {
    let startup = Startup { paths, screenshot: None, show: Vec::new() };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    h
}

/// Runs a few frames so that clicks are handled and the result is drawn.
fn settle(h: &mut Harness<'static, SheafApp>) {
    h.run_steps(6);
}

fn click(h: &mut Harness<'static, SheafApp>, label: &str) {
    h.get_by_label(label).click();
    settle(h);
}

/// Opens a menu of the menu bar and clicks one of its entries.
fn menu(h: &mut Harness<'static, SheafApp>, menu: &str, entry: &str) {
    click(h, menu);
    click(h, entry);
}

fn project<'a>(h: &'a Harness<'static, SheafApp>) -> &'a Project {
    h.state().doc.project()
}

/// The numbers of the column with this short name in the first workbook.
fn column(h: &Harness<'static, SheafApp>, short: &str) -> Vec<f64> {
    let book = project(h).workbooks().next().unwrap();
    book.sheet().column_by_name(short).unwrap().numeric().unwrap().to_vec()
}

#[test]
#[ignore = "draft: not yet adjusted to the labels of the real widgets"]
fn a_data_file_given_at_start_is_imported_and_plotted() {
    let h = start(vec![data_file("start")]);
    let p = project(&h);
    assert_eq!(p.workbooks().count(), 1);
    let graph = p.graphs().next().expect("a graph of the file");
    assert_eq!(graph.layers[0].plots.len(), 2, "both Y columns are plotted");
    assert!(h.state().doc.dirty);
    // Both windows are listed in the project explorer and have a tab.
    assert!(h.query_all_by_label_contains("Book1").count() >= 2);
    assert!(h.query_all_by_label_contains("Graph1").count() >= 2);
}

#[test]
#[ignore = "draft: not yet adjusted to the labels of the real widgets"]
fn menu_new_workbook_then_undo_and_redo() {
    let mut h = start(Vec::new());
    assert_eq!(project(&h).windows.len(), 0);
    menu(&mut h, "Worksheet", "New Workbook");
    assert_eq!(project(&h).workbooks().count(), 1);
    assert_eq!(project(&h).workbooks().next().unwrap().sheet().columns.len(), 2);
    menu(&mut h, "Edit", "Undo New workbook");
    assert_eq!(project(&h).windows.len(), 0);
    menu(&mut h, "Edit", "Redo New workbook");
    assert_eq!(project(&h).workbooks().count(), 1);
}

#[test]
#[ignore = "draft: not yet adjusted to the labels of the real widgets"]
fn selecting_a_column_header_and_plotting_it_makes_a_graph_of_that_column() {
    let mut h = start(vec![data_file("plot")]);
    click(&mut h, "Book1 - sample");
    click(&mut h, "C(Y)");
    let book = project(&h).workbooks().next().unwrap().id;
    let c = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().id;
    assert_eq!(h.state().books[&book].selected, vec![c]);
    menu(&mut h, "Plot", "Scatter");
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), 2);
    let plots = &graphs[1].layers[0].plots;
    assert_eq!(plots.len(), 1);
    assert_eq!(plots[0].y, c);
}

#[test]
#[ignore = "draft: not yet adjusted to the labels of the real widgets"]
fn double_click_edits_a_cell_and_enter_stores_the_value() {
    let mut h = start(vec![data_file("edit")]);
    click(&mut h, "Book1 - sample");
    let cell = h.get_by_label("4.5");
    cell.click();
    cell.click();
    settle(&mut h);
    assert!(h.state().books.values().next().unwrap().editing.is_some(), "the cell is being edited");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("7.25".to_string()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(column(&h, "B"), [2.5, 7.25, 6.5, 8.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Edit cell"));
}

#[test]
#[ignore = "draft: not yet adjusted to the labels of the real widgets"]
fn the_column_menu_changes_the_designation() {
    let mut h = start(vec![data_file("designation")]);
    click(&mut h, "Book1 - sample");
    h.get_by_label("C(Y)").click_secondary();
    settle(&mut h);
    click(&mut h, "Set as");
    click(&mut h, "Y error");
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().column_by_name("C").unwrap().designation, Designation::YError);
}

#[test]
#[ignore = "draft: not yet adjusted to the labels of the real widgets"]
fn set_column_values_fills_the_column_from_a_formula() {
    let mut h = start(vec![data_file("formula")]);
    click(&mut h, "Book1 - sample");
    h.get_by_label("C(Y)").click_secondary();
    settle(&mut h);
    click(&mut h, "Set Column Values...");
    h.event(egui::Event::Text("col(B)*2 + i".to_string()));
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(column(&h, "C"), [6.0, 11.0, 16.0, 21.0]);
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().column_by_name("C").unwrap().formula.as_deref(), Some("col(B)*2 + i"));
    assert!(h.state().set_values.is_none(), "OK closes the window");

    // A formula with a mistake keeps the window open, shows the problem and changes nothing.
    h.get_by_label("C(Y)").click_secondary();
    settle(&mut h);
    click(&mut h, "Set Column Values...");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("col(Nope)".to_string()));
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().set_values.is_some());
    assert_eq!(column(&h, "C"), [6.0, 11.0, 16.0, 21.0]);
}

#[test]
#[ignore = "draft: not yet adjusted to the labels of the real widgets"]
fn graph_settings_change_the_axis_live_and_undo_as_one_step() {
    let mut h = start(vec![data_file("axis")]);
    click(&mut h, "Graph1 - sample");
    menu(&mut h, "Graph", "Y Axis...");
    assert!(h.state().graph_props.is_some());
    let scale = |h: &Harness<'static, SheafApp>| project(h).graphs().next().unwrap().layers[0].y_axis.scale;
    assert_eq!(scale(&h), ScaleKind::Linear);
    click(&mut h, "Log10");
    assert_eq!(scale(&h), ScaleKind::Log10, "the change applies while the window is open");
    click(&mut h, "Legend");
    click(&mut h, "Draw a frame");
    assert_eq!(h.state().doc.undo_label(), Some("Change graph settings"));
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(scale(&h), ScaleKind::Linear, "one undo reverts everything the window did");
    assert_eq!(h.state().doc.undo_label(), Some("Import data"));
}

#[test]
fn a_dropped_data_file_goes_through_the_import_window() {
    let mut h = start(Vec::new());
    let file = data_file("drop");
    h.input_mut().dropped_files.push(egui::DroppedFile { path: Some(file), ..Default::default() });
    settle(&mut h);
    assert!(h.state().import.is_some(), "the import window is open");
    assert_eq!(project(&h).windows.len(), 0, "nothing is imported before the user confirms");
    // The preview shows the detected columns.
    assert!(h.query_by_label("B(Y)").is_some());
    click(&mut h, "Make a graph of each file");
    click(&mut h, "Import");
    assert!(h.state().import.is_none());
    assert_eq!(project(&h).workbooks().count(), 1);
    assert_eq!(project(&h).graphs().count(), 0, "the graph option was switched off");
}

#[test]
#[ignore = "draft: not yet adjusted to the labels of the real widgets"]
fn deleting_a_workbook_removes_its_plots_and_can_be_undone() {
    let mut h = start(vec![data_file("delete")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::DeleteWindow(book), &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), 0);
    assert_eq!(project(&h).graphs().next().unwrap().layers[0].plots.len(), 0);
    menu(&mut h, "Edit", "Undo Delete window");
    assert_eq!(project(&h).workbooks().count(), 1);
    assert_eq!(project(&h).graphs().next().unwrap().layers[0].plots.len(), 2);
}
