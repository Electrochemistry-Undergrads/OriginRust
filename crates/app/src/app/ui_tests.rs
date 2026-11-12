//! Tests that drive the application like a user: they click menu entries,
//! column headers and dialog buttons and type into fields, without a real
//! window. They guard the wiring between the interface and the project
//! (which click leads to which change), not the look.

use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
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
    // Short frames: a double click needs both clicks within 0.3 s of simulated time, and every
    // pointer event of a click is a frame of its own.
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
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

/// Clicks the text drawn in the grid (a column head such as "C(Y)" or a cell such as "4.5").
/// Plain labels expose their text as a value, not as a label; the click lands in the middle of
/// the text, where the grid has its click area for the header or cell.
fn click_text(h: &mut Harness<'static, SheafApp>, text: &str) {
    grid_text(h, text).click();
    settle(h);
}

fn right_click_text(h: &mut Harness<'static, SheafApp>, text: &str) {
    grid_text(h, text).click_secondary();
    settle(h);
}

/// The label node that shows this text (the text run inside it has the same value, so the
/// role is part of the query).
fn grid_text<'a>(h: &'a Harness<'static, SheafApp>, text: &'a str) -> egui_kittest::Node<'a> {
    h.get(By::new().role(egui::accesskit::Role::Label).value(text))
}

/// Clicks the one widget whose label contains the text (menu entries carry
/// their keyboard shortcut in the label, for example "Undo Import data Ctrl+Z").
fn click_containing(h: &mut Harness<'static, SheafApp>, label: &str) {
    h.get_by_label_contains(label).click();
    settle(h);
}

/// Opens a menu of the menu bar and clicks one of its entries.
fn menu(h: &mut Harness<'static, SheafApp>, menu: &str, entry: &str) {
    click(h, menu);
    click_containing(h, entry);
}

fn right_click(h: &mut Harness<'static, SheafApp>, label: &str) {
    h.get_by_label(label).click_secondary();
    settle(h);
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
fn a_data_file_given_at_start_is_imported_and_plotted() {
    let h = start(vec![data_file("start")]);
    let p = project(&h);
    assert_eq!(p.workbooks().count(), 1);
    let graph = p.graphs().next().expect("a graph of the file");
    assert_eq!(graph.layers[0].plots.len(), 2, "both Y columns are plotted");
    assert!(h.state().doc.dirty);
    // Both windows are listed in the project explorer ...
    assert_eq!(h.query_all_by_label("Book1 - sample").count(), 1);
    assert_eq!(h.query_all_by_label("Graph1 - sample").count(), 1);
    // ... and have a tab (the tab titles are drawn by the docking library and are not exposed).
    assert_eq!(h.state().dock.iter_all_tabs().count(), 2);
}

#[test]
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
fn selecting_a_column_header_and_plotting_it_makes_a_graph_of_that_column() {
    let mut h = start(vec![data_file("plot")]);
    click(&mut h, "Book1 - sample");
    click_text(&mut h, "C(Y)");
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
fn double_click_edits_a_cell_and_enter_stores_the_value() {
    let mut h = start(vec![data_file("edit")]);
    click(&mut h, "Book1 - sample");
    let cell = grid_text(&h, "4.5");
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
fn enter_then_clicking_elsewhere_leaves_the_next_cell_alone() {
    let mut h = start(vec![data_file("enter")]);
    click(&mut h, "Book1 - sample");
    let cell = grid_text(&h, "4.5");
    cell.click();
    cell.click();
    settle(&mut h);
    // Enter without typing: nothing changes, and the editor moves to the next row.
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let editing = h.state().books.values().next().unwrap().editing.clone();
    assert_eq!(editing.map(|e| (e.1, e.2)), Some((2, "6.5".to_string())), "the next cell is shown with its value");
    // Click somewhere else to leave the editor.
    click(&mut h, "Book1 - sample");
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Import data"), "no edit step was added");
}

#[test]
fn editing_a_cell_starts_from_the_full_precision_value_and_unchanged_text_stores_nothing() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-precise-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.csv");
    std::fs::write(
        &path,
        "t,v
1,0.1234567891234
2,5
",
    )
    .unwrap();
    let mut h = start(vec![path]);
    click(&mut h, "Book1 - sample");
    let shown = project(&h).workbooks().next().unwrap().sheet().column_by_name("B").unwrap().cell_text(0);
    assert_ne!(shown, "0.1234567891234", "the grid shows a rounded value");
    let cell = grid_text(&h, &shown);
    cell.click();
    cell.click();
    settle(&mut h);
    let editing = h.state().books.values().next().unwrap().editing.clone().unwrap();
    assert_eq!(editing.2, "0.1234567891234", "the editor holds every digit");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    let cell = grid_text(&h, &shown);
    cell.click();
    cell.click();
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(column(&h, "B")[0], 0.1234567891234);
    assert_ne!(h.state().doc.undo_label(), Some("Edit cell"));
}

#[test]
fn the_column_menu_changes_the_designation() {
    let mut h = start(vec![data_file("designation")]);
    click(&mut h, "Book1 - sample");
    right_click_text(&mut h, "C(Y)");
    click_containing(&mut h, "Set as");
    click(&mut h, "Y error");
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().column_by_name("C").unwrap().designation, Designation::YError);
}

#[test]
fn set_column_values_fills_the_column_from_a_formula() {
    let mut h = start(vec![data_file("formula")]);
    click(&mut h, "Book1 - sample");
    right_click_text(&mut h, "C(Y)");
    click(&mut h, "Set Column Values...");
    h.event(egui::Event::Text("col(B)*2 + i".to_string()));
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(column(&h, "C"), [6.0, 11.0, 16.0, 21.0]);
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().column_by_name("C").unwrap().formula.as_deref(), Some("col(B)*2 + i"));
    assert!(h.state().set_values.is_none(), "OK closes the window");

    // A formula with a mistake keeps the window open, shows the problem and changes nothing.
    right_click_text(&mut h, "C(Y)");
    click(&mut h, "Set Column Values...");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("col(Nope)".to_string()));
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().set_values.is_some());
    assert_eq!(column(&h, "C"), [6.0, 11.0, 16.0, 21.0]);
}

#[test]
fn an_auto_column_follows_an_edited_source_cell_and_a_manual_one_waits_to_be_recalculated() {
    let mut h = start(vec![data_file("recalc")]);
    click(&mut h, "Book1 - sample");
    // C = col(B)*2, set to recalculate automatically.
    right_click_text(&mut h, "C(Y)");
    click(&mut h, "Set Column Values...");
    h.event(egui::Event::Text("col(B)*2".to_string()));
    settle(&mut h);
    click(&mut h, "Auto");
    click(&mut h, "OK");
    assert_eq!(column(&h, "C"), [5.0, 9.0, 13.0, 17.0]);
    let mark = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().recalc;
    assert_eq!(mark, sheaf_core::RecalcMode::Auto);
    grid_text(&h, "auto");

    // Edit a source cell: the Auto column changes in the same undo step. (The double click
    // that opens the cell editor is covered by the tests above; here the finished edit is
    // sent as the action the editor produces.)
    edit_cell(&mut h, "B", 1, "10");
    assert_eq!(column(&h, "B"), [2.5, 10.0, 6.5, 8.5]);
    assert_eq!(column(&h, "C"), [5.0, 20.0, 13.0, 17.0]);
    menu(&mut h, "Edit", "Undo Edit cell");
    assert_eq!(column(&h, "B")[1], 4.5);
    assert_eq!(column(&h, "C")[1], 9.0);

    // Make it Manual: the next source edit only marks the column, Recalculate updates it.
    right_click_text(&mut h, "C(Y)");
    click(&mut h, "Set Column Values...");
    click(&mut h, "Manual");
    click(&mut h, "OK");
    edit_cell(&mut h, "B", 2, "1");
    assert_eq!(column(&h, "C")[2], 13.0, "a Manual column keeps its values");
    grid_text(&h, "pending");
    // The cells of a calculated column cannot be typed over.
    edit_cell(&mut h, "C", 0, "99");
    assert_eq!(column(&h, "C")[0], 5.0);
    right_click_text(&mut h, "C(Y)");
    click(&mut h, "Recalculate");
    assert_eq!(column(&h, "C")[2], 2.0);
    assert_eq!(h.state().doc.undo_label(), Some("Recalculate"));
}

/// Stores text in a cell the way the cell editor does when it is left.
fn edit_cell(h: &mut Harness<'static, SheafApp>, short: &str, row: usize, text: &str) {
    let book = project(h).workbooks().next().unwrap();
    let column = book.sheet().column_by_name(short).unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SetCell { column, row, text: text.to_string() }, &ctx);
    settle(h);
}

#[test]
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
fn deleting_a_workbook_removes_its_plots_and_can_be_undone() {
    let mut h = start(vec![data_file("delete")]);
    // The context menu of the workbook in the project explorer.
    right_click(&mut h, "Book1 - sample");
    click(&mut h, "Delete");
    assert_eq!(project(&h).workbooks().count(), 0);
    assert_eq!(project(&h).graphs().next().unwrap().layers[0].plots.len(), 0);
    menu(&mut h, "Edit", "Undo Delete window");
    assert_eq!(project(&h).workbooks().count(), 1);
    assert_eq!(project(&h).graphs().next().unwrap().layers[0].plots.len(), 2);
}

#[test]
fn an_analysis_tool_runs_from_its_window_and_shows_the_result_sheet() {
    let mut h = start(vec![data_file("analysis")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Analyze { book, columns: Vec::new(), kind: AnalysisKind::Integrate }, &ctx);
    settle(&mut h);
    assert!(h.state().analysis.is_some(), "the Integrate window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!(b.sheets.len(), 2);
    assert_eq!(b.sheets[1].name, "Integrate1");
    assert_eq!(b.active_sheet, 1, "the result sheet is shown");
    // Area under "up" (2.5, 4.5, 6.5, 8.5 over t = 1..4) and under "down" (30.5 .. 0.5).
    assert_eq!(b.sheets[1].columns[1].numeric().unwrap(), [16.5, 46.5]);
    assert_eq!(b.sheets[0].columns.len(), 5, "two running-integral columns were added");
    assert_eq!(h.state().doc.undo_label(), Some("Integrate"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!((b.sheets.len(), b.sheets[0].columns.len()), (1, 3), "one undo removes everything the tool added");
}

/// The FFT filter is chosen in the Smooth window's method list; OK adds a smoothed column that
/// equals what sheaf-calc computes for the same options (the window's own default of 5 points).
#[test]
fn the_fft_filter_is_chosen_in_the_smooth_window_and_runs() {
    let mut h = start(vec![data_file("fft")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Analyze { book, columns: Vec::new(), kind: AnalysisKind::Smooth }, &ctx);
    settle(&mut h);
    // A combo box shows its current choice as its value, not as a label.
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Savitzky-Golay")).click();
    settle(&mut h);
    click(&mut h, "FFT filter");
    // The FFT note appears only once the FFT filter is chosen (the query panics if it is missing).
    grid_text(&h, "The FFT filter keeps frequencies below 1 / (2 dx points); larger values smooth more.");
    click(&mut h, "OK");
    let b = project(&h).workbooks().next().unwrap();
    let smoothed = b.sheet().columns.iter().find(|c| c.comments.contains("FFT filter")).expect("an FFT column");
    let t = column(&h, "A");
    let up = column(&h, "B");
    let options =
        sheaf_calc::SmoothOptions { method: sheaf_calc::SmoothMethod::FftFilter, npts: 5, ..Default::default() };
    let want = sheaf_calc::smooth_xy(&t, &up, &options);
    let got = smoothed.numeric().unwrap();
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        assert!((g - w).abs() < 1e-12, "{got:?} vs {want:?}");
    }
}

#[test]
fn a_linear_fit_runs_from_its_window_adds_the_curve_to_the_graph_and_undoes_in_one_step() {
    let mut h = start(vec![data_file("fit")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let plots_before = project(&h).graphs().next().unwrap().layers[0].plots.len();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::Linear }, &ctx);
    settle(&mut h);
    assert!(h.state().fit.is_some(), "the Fit window is open");
    click(&mut h, "OK");
    assert!(h.state().fit.is_none(), "OK closes the window");
    let b = project(&h).workbooks().next().unwrap();
    let names: Vec<&str> = b.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["sample", "FitLinear1", "FitLinear1Stats", "FitCurve1"]);
    assert_eq!(b.active_sheet, 1, "the parameter sheet is shown");
    // "up" is exactly 0.5 + 2t.
    let values = b.sheets[1].column_by_name("Value").unwrap().numeric().unwrap();
    assert!((values[0] - 0.5).abs() < 1e-12 && (values[1] - 2.0).abs() < 1e-12, "{values:?}");
    assert_eq!(project(&h).graphs().next().unwrap().layers[0].plots.len(), plots_before + 1);
    assert_eq!(h.state().doc.undo_label(), Some("Fit"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!((b.sheets.len(), b.sheets[0].columns.len()), (1, 3), "one undo removes everything the fit added");
    assert_eq!(project(&h).graphs().next().unwrap().layers[0].plots.len(), plots_before);
}

#[test]
fn the_fit_window_offers_bands_and_iteration_controls() {
    let mut h = start(vec![data_file("fitbands")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::Linear }, &ctx);
    settle(&mut h);
    // The controls are there and have Origin's defaults.
    h.get_by_label("Maximum iterations");
    h.get_by_label("Band confidence level");
    click(&mut h, "Confidence and prediction");
    click(&mut h, "OK");
    assert!(h.state().fit.is_none(), "OK closes the window");
    let b = project(&h).workbooks().next().unwrap();
    let curve = b.sheets.iter().find(|s| s.name == "FitCurve1").expect("the curve sheet");
    let names: Vec<&str> = curve.columns.iter().map(|c| c.long_name.as_str()).collect();
    assert!(names.iter().any(|n| n.starts_with("LCL")), "{names:?}");
    assert!(names.iter().any(|n| n.starts_with("UPL")), "{names:?}");
    assert_eq!(curve.columns.len(), 2 + 4, "the curve and four band columns");
}

#[test]
fn sorting_and_filtering_a_sheet() {
    let mut h = start(vec![data_file("sort")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let down = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SortSheet { column: down, descending: false }, &ctx);
    settle(&mut h);
    assert_eq!(column(&h, "C"), [0.5, 10.5, 20.5, 30.5]);
    assert_eq!(column(&h, "A"), [4.0, 3.0, 2.0, 1.0], "rows stay together");
    assert_eq!(h.state().doc.undo_label(), Some("Sort sheet"));

    h.state_mut().apply(Action::FilterRows(book), &ctx);
    settle(&mut h);
    assert!(h.state().filter.is_some());
    h.event(egui::Event::Text("col(A) >= 3".to_string()));
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().filter.is_none());
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!(b.sheets[1].name, "Filter1");
    assert_eq!(b.sheets[1].columns[0].numeric().unwrap(), [4.0, 3.0]);
    assert_eq!(b.active_sheet, 1);
}

#[test]
fn ctrl_z_and_ctrl_y_undo_and_redo() {
    let mut h = start(vec![data_file("keys")]);
    assert_eq!(h.state().doc.undo_label(), Some("Import data"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(project(&h).windows.len(), 0, "Ctrl+Z undid the import");
    assert_eq!(h.state().doc.redo_label(), Some("Import data"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), 1, "Ctrl+Y redid it");
}

#[test]
fn the_graph_menu_opens_the_export_window_and_close_closes_it() {
    let mut h = start(vec![data_file("export")]);
    click(&mut h, "Graph1 - sample");
    click(&mut h, "Graph");
    // The graph toolbar has an "Export..." button too; the menu entry is drawn on top of it.
    h.get_all_by_label("Export...").last().expect("the menu entry").click();
    settle(&mut h);
    assert!(h.state().export.is_some(), "the export window is open");
    assert!(h.query_by_label("Save as...").is_some());
    click(&mut h, "Close");
    assert!(h.state().export.is_none());
}

#[test]
fn integrate_ranges_runs_from_the_analysis_menu_with_two_ranges() {
    let mut h = start(vec![data_file("ranges")]);
    click(&mut h, "Book1 - sample");
    menu(&mut h, "Analysis", "Integrate Ranges");
    assert!(h.state().analysis.is_some(), "the window is open");
    click(&mut h, "Add range");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    let b = project(&h).workbooks().next().unwrap();
    let results = b.sheets.last().unwrap();
    assert_eq!(results.name, "Integrate1");
    // two Y columns times two ranges; each range is the whole X range 1..4
    assert_eq!(results.columns[0].text().unwrap().len(), 4);
    let area = results.columns.iter().find(|c| c.long_name == "Area").unwrap().numeric().unwrap();
    assert_eq!(area, [16.5, 16.5, 46.5, 46.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Integrate ranges"));
}

#[test]
fn subtract_reference_data_runs_from_the_analysis_menu_in_one_undo_step() {
    let mut h = start(vec![data_file("subref-a"), data_file("subref-b")]);
    assert_eq!(project(&h).workbooks().count(), 2);
    click(&mut h, "Book1 - sample");
    menu(&mut h, "Analysis", "Subtract Reference Data");
    assert!(h.state().analysis.is_some(), "the window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    // The reference is the "up" column of the other workbook (the first column not used as input).
    let added: Vec<&sheaf_core::Column> = project(&h)
        .workbooks()
        .flat_map(|b| b.sheet().columns.iter())
        .filter(|c| c.comments.contains("minus reference"))
        .collect();
    assert_eq!(added.len(), 2, "one column for each of the two Y columns");
    assert_eq!(added[0].numeric().unwrap(), [0.0; 4], "up minus up");
    assert_eq!(added[1].numeric().unwrap(), [28.0, 16.0, 4.0, -8.0], "down minus up");
    assert_eq!(h.state().doc.undo_label(), Some("Subtract reference"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(project(&h).workbooks().all(|b| b.sheet().columns.len() == 3), "one undo removes both columns");
}

#[test]
fn simple_math_with_a_constant_adds_a_column_for_each_y_column() {
    let mut h = start(vec![data_file("math")]);
    click(&mut h, "Book1 - sample");
    menu(&mut h, "Analysis", "Simple Math");
    assert!(h.state().analysis.is_some(), "the window is open");
    // With no other column to use as reference the operand is the constant, 1 for a start.
    click(&mut h, "OK");
    let b = project(&h).workbooks().next().unwrap();
    let plus: Vec<&sheaf_core::Column> = b.sheet().columns.iter().filter(|c| c.comments.contains("plus 1")).collect();
    assert_eq!(plus.len(), 2);
    assert_eq!(plus[0].numeric().unwrap(), [3.5, 5.5, 7.5, 9.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Curve math"));
}

fn date_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("icp.csv");
    std::fs::write(&path, "Time,Cu\n2026-11-11 13:45:10,1.5\n2026-11-11 13:50:15,2.5\n2026-11-11 13:55:20,3.5\n")
        .unwrap();
    path
}

#[test]
fn the_import_window_imports_dates_as_numbers_or_keeps_them_as_text() {
    use sheaf_core::format::NumberFormat;
    // Default: a date column with a date-time format, shown as text in the cells.
    let mut h = start(Vec::new());
    h.input_mut().dropped_files.push(egui::DroppedFile { path: Some(date_file("dates")), ..Default::default() });
    settle(&mut h);
    click(&mut h, "Import");
    let book = project(&h).workbooks().next().unwrap();
    let col = book.sheet().column_by_name("A").unwrap();
    assert!(matches!(col.format, NumberFormat::DateTime(..)));
    assert_eq!(col.cell_text(0), "2026-11-11 13:45:10");
    // Keep as text: the same file arrives as text cells.
    let mut h = start(Vec::new());
    h.input_mut().dropped_files.push(egui::DroppedFile { path: Some(date_file("dates-text")), ..Default::default() });
    settle(&mut h);
    click(&mut h, "Keep as text");
    click(&mut h, "Import");
    let book = project(&h).workbooks().next().unwrap();
    let col = book.sheet().column_by_name("A").unwrap();
    assert!(col.numeric().is_none());
    assert_eq!(col.cell_text(0), "2026-11-11 13:45:10");
}

#[test]
fn column_properties_switch_a_date_time_column_to_a_date_only_format() {
    use sheaf_core::datetime::DatePattern;
    use sheaf_core::format::NumberFormat;
    let mut h = start(Vec::new());
    h.input_mut().dropped_files.push(egui::DroppedFile { path: Some(date_file("dates-props")), ..Default::default() });
    settle(&mut h);
    click(&mut h, "Import");
    click(&mut h, "Book1 - icp");
    right_click_text(&mut h, "A(X)");
    click(&mut h, "Properties...");
    // The format box starts on "Date and time" for the imported column.
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Date and time")).click();
    settle(&mut h);
    click(&mut h, "Date");
    click(&mut h, "OK");
    let book = project(&h).workbooks().next().unwrap();
    let col = book.sheet().column_by_name("A").unwrap();
    assert!(matches!(col.format, NumberFormat::Date(_)), "{:?}", col.format);
    assert_eq!(col.cell_text(0), NumberFormat::Date(DatePattern::Iso).format(col.numeric().unwrap()[0]));
    assert_eq!(col.cell_text(0), "2026-11-11");
}
