//! Tests that drive the application like a user: they click menu entries,
//! column headers and dialog buttons and type into fields, without a real
//! window. They guard the wiring between the interface and the project
//! (which click leads to which change), not the look.

use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT, Queryable};
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

/// Starts the application with the Project Explorer docked, so that its rows are on screen for
/// the tests that click them (a panel in the auto-hide strip is only on screen while the mouse is
/// on its tab; the tests of that behaviour use `start_auto_hide`).
fn start(paths: Vec<PathBuf>) -> Harness<'static, SheafApp> {
    let mut h = start_auto_hide(paths);
    h.state_mut().panels.set_pinned(PanelId::Explorer, true);
    settle(&mut h);
    h
}

fn start_auto_hide(paths: Vec<PathBuf>) -> Harness<'static, SheafApp> {
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

/// Shows and activates a window by a double click on its row in the project explorer (a single
/// click only selects the row, as in Origin).
fn show_window(h: &mut Harness<'static, SheafApp>, label: &str) {
    let at = h.get_by_label(label).rect().center();
    double_click_at(h, at);
    // So that the next click is not counted with these two.
    pause(h);
}

fn click(h: &mut Harness<'static, SheafApp>, label: &str) {
    h.get_by_label(label).click();
    settle(h);
}

/// Clicks the text drawn in the grid (a column head such as "C(Y1)" or a cell such as "4.5").
/// Plain labels expose their text as a value, not as a label; the click lands in the middle of
/// the text, where the grid has its click area for the header or cell.
fn click_text(h: &mut Harness<'static, SheafApp>, text: &str) {
    pause(h);
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
    not_toolbar(h, label).into_iter().next().unwrap_or_else(|| panic!("nothing is labelled {label:?}")).click();
    settle(h);
}

/// The widgets whose label contains the text, leaving out toolbar buttons, whose labels
/// ("Standard toolbar: Copy") carry the names of the menu entries they stand for.
fn not_toolbar<'a>(h: &'a Harness<'static, SheafApp>, text: &'a str) -> Vec<egui_kittest::Node<'a>> {
    h.query_all_by_label_contains(text)
        .filter(|n| !n.accesskit_node().label().unwrap_or_default().contains(" toolbar: "))
        .collect()
}

/// The menu entry with this text: the one whose label is exactly the text, otherwise the one
/// whose label contains it (entries with a shortcut carry it in the label, "Undo Import data
/// Ctrl+Z"). When a submenu and its first entry have the same name, the one further right is the
/// entry.
fn menu_entry<'a>(h: &'a Harness<'static, SheafApp>, text: &'a str) -> egui_kittest::Node<'a> {
    // The label without the arrow egui draws after a submenu and without a trailing shortcut.
    let name = |n: &egui_kittest::Node<'_>| {
        let label = n.accesskit_node().label().unwrap_or_default().to_string();
        let label = label.trim_end_matches(" \u{23f5}").to_string();
        match label.rsplit_once(' ') {
            Some((head, key)) if key.contains('+') => head.to_string(),
            _ => label,
        }
    };
    let pick =
        |nodes: Vec<egui_kittest::Node<'a>>| nodes.into_iter().max_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x));
    let all: Vec<egui_kittest::Node<'a>> = not_toolbar(h, text);
    let exact: Vec<egui_kittest::Node<'a>> = all
        .iter()
        .filter(|n| name(n) == text || n.accesskit_node().label().as_deref() == Some(text))
        .cloned()
        .collect();
    pick(exact).or_else(|| pick(all)).unwrap_or_else(|| panic!("no menu entry {text:?}"))
}

/// Opens the menus along the path and clicks the last entry:
/// `menu_path(h, &["Plot", "Symbol", "Scatter"])`.
fn menu_path(h: &mut Harness<'static, SheafApp>, path: &[&str]) {
    for text in path {
        menu_entry(h, text).click();
        settle(h);
    }
}

/// Opens a menu of the menu bar and clicks one of its entries.
fn menu(h: &mut Harness<'static, SheafApp>, menu: &str, entry: &str) {
    menu_path(h, &[menu, entry]);
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
    // ... and are open as child windows of the workspace, each with its own title bar.
    assert_eq!(h.state().workspace.shown().len(), 2);
    assert!(h.query_by_label("Title Book1 - sample").is_some());
    assert!(h.query_by_label("Title Graph1 - sample").is_some());
}

#[test]
fn menu_new_workbook_then_undo_and_redo() {
    let mut h = start(Vec::new());
    assert_eq!(project(&h).windows.len(), 0);
    menu_path(&mut h, &["File", "New", "Worksheet..."]);
    assert_eq!(project(&h).workbooks().count(), 1);
    assert_eq!(project(&h).workbooks().next().unwrap().sheet().columns.len(), 2);
    menu(&mut h, "Edit", "Undo New workbook");
    assert_eq!(project(&h).windows.len(), 0);
    // With no window the menu bar has no Edit menu (Origin's bar for this case), but the keys
    // still work: Ctrl+Shift+Z is Redo.
    assert!(h.query_by_label("Edit").is_none());
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), 1);
    menu(&mut h, "Edit", "Undo New workbook");
    assert_eq!(project(&h).windows.len(), 0);
}

#[test]
fn undo_and_redo_are_reachable_with_the_mouse_when_no_window_is_open() {
    let mut h = start(Vec::new());
    menu_path(&mut h, &["File", "New", "Worksheet..."]);
    menu(&mut h, "Edit", "Undo New workbook");
    assert_eq!(project(&h).windows.len(), 0);
    assert!(h.query_by_label("Edit").is_none());
    // Redo from the File menu of the short bar.
    menu(&mut h, "File", "Redo");
    assert_eq!(project(&h).workbooks().count(), 1);
    menu(&mut h, "Edit", "Undo New workbook");
    assert_eq!(project(&h).windows.len(), 0);
    // Undo from the File menu: redo, delete the window, then undo the delete with no window.
    menu(&mut h, "File", "Redo");
    h.state_mut().doc.apply("Delete window", |p| p.windows.clear());
    settle(&mut h);
    assert_eq!(project(&h).windows.len(), 0);
    menu(&mut h, "File", "Undo");
    assert_eq!(project(&h).workbooks().count(), 1);
}

#[test]
fn selecting_a_column_header_and_plotting_it_makes_a_graph_of_that_column() {
    let mut h = start(vec![data_file("plot")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "C(Y1)");
    let book = project(&h).workbooks().next().unwrap().id;
    let c = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().id;
    assert_eq!(h.state().books[&book].selected, vec![c]);
    menu_path(&mut h, &["Plot", "Symbol", "Scatter"]);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), 2);
    let plots = &graphs[1].layers[0].plots;
    assert_eq!(plots.len(), 1);
    assert_eq!(plots[0].y, c);
}

#[test]
fn double_click_edits_a_cell_and_enter_stores_the_value() {
    let mut h = start(vec![data_file("edit")]);
    show_window(&mut h, "Book1 - sample");
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
    show_window(&mut h, "Book1 - sample");
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
    show_window(&mut h, "Book1 - sample");
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
    show_window(&mut h, "Book1 - sample");
    let shown = project(&h).workbooks().next().unwrap().sheet().column_by_name("B").unwrap().shown_text(0);
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
    show_window(&mut h, "Book1 - sample");
    right_click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Set As", "Y Error"]);
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().column_by_name("C").unwrap().designation, Designation::YError);
}

#[test]
fn set_column_values_fills_the_column_from_a_formula() {
    let mut h = start(vec![data_file("formula")]);
    show_window(&mut h, "Book1 - sample");
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Set Column Values...");
    h.event(egui::Event::Text("col(B)*2 + i".to_string()));
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(column(&h, "C"), [6.0, 11.0, 16.0, 21.0]);
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().column_by_name("C").unwrap().formula.as_deref(), Some("col(B)*2 + i"));
    assert!(h.state().set_values.is_none(), "OK closes the window");

    // A formula with a mistake keeps the window open, shows the problem and changes nothing.
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Set Column Values...");
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
    show_window(&mut h, "Book1 - sample");
    // C = col(B)*2, set to recalculate automatically.
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Set Column Values...");
    h.event(egui::Event::Text("col(B)*2".to_string()));
    settle(&mut h);
    click(&mut h, "Auto");
    click(&mut h, "OK");
    assert_eq!(column(&h, "C"), [5.0, 9.0, 13.0, 17.0]);
    let mark = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().recalc;
    assert_eq!(mark, sheaf_core::RecalcMode::Auto);
    h.get_by_label("Lock of column C: auto, current");

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
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Set Column Values...");
    click(&mut h, "Manual");
    click(&mut h, "OK");
    edit_cell(&mut h, "B", 2, "1");
    assert_eq!(column(&h, "C")[2], 13.0, "a Manual column keeps its values");
    h.get_by_label("Lock of column C: manual, pending");
    // The cells of a calculated column cannot be typed over.
    edit_cell(&mut h, "C", 0, "99");
    assert_eq!(column(&h, "C")[0], 5.0);
    right_click_text(&mut h, "C(Y1)");
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
    show_window(&mut h, "Graph1 - sample");
    menu_path(&mut h, &["Format", "Axes", "Y Axis..."]);
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
fn ctrl_z_and_ctrl_shift_z_undo_and_redo() {
    let mut h = start(vec![data_file("keys")]);
    assert_eq!(h.state().doc.undo_label(), Some("Import data"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(project(&h).windows.len(), 0, "Ctrl+Z undid the import");
    assert_eq!(h.state().doc.redo_label(), Some("Import data"));
    // Ctrl+Y is Origin's key for Nonlinear Curve Fit, so redo is Ctrl+Shift+Z.
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), 1, "Ctrl+Shift+Z redid it");
}

#[test]
fn file_export_graphs_opens_the_export_window_and_close_closes_it() {
    let mut h = start(vec![data_file("export")]);
    show_window(&mut h, "Graph1 - sample");
    menu_path(&mut h, &["File", "Export", "Graphs..."]);
    assert!(h.state().export.is_some(), "the export window is open");
    assert!(h.query_by_label("Save as...").is_some());
    click(&mut h, "Close");
    assert!(h.state().export.is_none());
}

#[test]
fn integrate_ranges_runs_from_the_analysis_menu_with_two_ranges() {
    let mut h = start(vec![data_file("ranges")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Analysis", "Mathematics", "Integrate Ranges..."]);
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
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Analysis", "Data Manipulation", "Subtract Reference Data..."]);
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
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Analysis", "Mathematics", "Simple Curve Math..."]);
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
    show_window(&mut h, "Book1 - icp");
    // The long menu of a column head needs a tall window to show all its items.
    h.set_size(egui::vec2(1280.0, 1400.0));
    settle(&mut h);
    right_click_text(&mut h, "A(X1)");
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

#[test]
fn the_import_window_appends_columns_to_the_active_sheet_and_reads_thousands_separators() {
    let mut h = start(vec![data_file("modes")]);
    show_window(&mut h, "Book1 - sample");
    // A second file, tab separated, with thousands separators in the first column.
    let more = data_file("modes").with_file_name("more.txt");
    std::fs::write(&more, "w\tz\n1,000.5\t1\n2,000.5\t2\n3,000.5\t3\n4,000.5\t4\n").unwrap();
    h.input_mut().dropped_files.push(egui::DroppedFile { path: Some(more), ..Default::default() });
    settle(&mut h);
    assert!(h.state().import.is_some());
    // The window found the separator by itself and says so.
    assert!(h.query_by_label_contains("found: comma").is_some());
    // Put the data into the active sheet as new columns.
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("New book")).click();
    settle(&mut h);
    click(&mut h, "New columns");
    click(&mut h, "Import");
    assert!(h.state().import.is_none());
    assert_eq!(project(&h).workbooks().count(), 1, "no new book was made");
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().columns.len(), 5);
    assert_eq!(column(&h, "D"), [1000.5, 2000.5, 3000.5, 4000.5]);
    assert_eq!(column(&h, "E"), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5], "the first file's columns are untouched");
}

/// Analysis > Peaks and Baseline > Peak Analyzer... runs the chain on a curve with two peaks on a slope: the window
/// opens from the menu, OK adds the baseline column and the result sheet, one undo removes both.
#[test]
fn analyze_peaks_runs_from_the_analysis_menu_and_undoes_in_one_step() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-peakchain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("peaks.csv");
    let mut text = String::from("t,i\n");
    for k in 0..400 {
        let x = k as f64 * 0.05;
        let g = |c: f64, s: f64, a: f64| a * (-(x - c) * (x - c) / (2.0 * s * s)).exp();
        text.push_str(&format!("{x},{}\n", 0.1 + 0.01 * x + g(6.0, 0.4, 1.0) + g(13.0, 0.5, 0.6)));
    }
    std::fs::write(&path, text).unwrap();
    let mut h = start(vec![path]);
    show_window(&mut h, "Book1 - peaks");
    menu_path(&mut h, &["Analysis", "Peaks and Baseline", "Peak Analyzer..."]);
    assert!(h.state().analysis.is_some(), "the Peak Analyzer window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!(b.sheets.len(), 2);
    assert!(b.sheets[1].name.starts_with("PeakAnalysis"), "{}", b.sheets[1].name);
    assert_eq!(b.sheets[0].columns.len(), 3, "a baseline column beside the data");
    let center = b.sheets[1].column_by_name("Center").unwrap().numeric().unwrap().to_vec();
    assert_eq!(center.len(), 2);
    assert!((center[0] - 6.0).abs() < 1e-9 && (center[1] - 13.0).abs() < 1e-9, "{center:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Analyze peaks"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!((b.sheets.len(), b.sheets[0].columns.len()), (1, 2));
}

/// A folder for the project files of one save test.
fn save_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-save-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn ctrl_s(h: &mut Harness<'static, SheafApp>) {
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    settle(h);
}

/// Gives the second column of the first sheet a column formula, which the .opj writer cannot
/// store (only the values are written), so saving needs the companion.
fn add_formula(h: &mut Harness<'static, SheafApp>) {
    h.state_mut().doc.apply("Set formula", |p| {
        if let Some(Window::Workbook(b)) = p.windows.first_mut() {
            b.sheets[0].columns[1].formula = Some("col(A)*2".into());
        }
    });
    settle(h);
}

#[test]
fn ctrl_s_saves_an_opj_with_a_companion_for_what_it_cannot_hold_and_reopening_uses_it() {
    let dir = save_dir("formula");
    let opj = dir.join("lab.opj");
    let mut h = start(vec![data_file("save-formula")]);
    add_formula(&mut h);
    h.state_mut().path = Some(opj.clone());
    ctrl_s(&mut h);
    let companion = dir.join("lab.opj.sheaf");
    assert!(opj.exists(), "the .opj is written");
    assert!(companion.exists(), "the companion is written");
    assert!(!dir.join("lab.sheaf").exists(), "a user's own lab.sheaf name is not used");
    assert!(!h.state().doc.dirty);
    assert!(h.state().status.contains("lab.opj.sheaf"), "{}", h.state().status);
    let (title, text) = h.state().message.clone().expect("a message says what only the companion holds");
    assert!(title.contains("companion") && text.contains("formula"), "{title}: {text}");
    assert_eq!(h.state().path.as_deref(), Some(opj.as_path()));

    let h2 = start(vec![opj.clone()]);
    let book = project(&h2).workbooks().next().unwrap();
    assert_eq!(book.sheets[0].columns[1].formula.as_deref(), Some("col(A)*2"), "the formula is back");
    assert_eq!(project(&h2).graphs().count(), 1, "the graph is back");
    assert_eq!(h2.state().path.as_deref(), Some(opj.as_path()), "it saves to the .opj again");
    assert!(h2.state().status.contains("companion"), "{}", h2.state().status);
}

#[test]
fn a_graph_is_saved_inside_the_opj_without_a_companion() {
    let dir = save_dir("graph");
    let opj = dir.join("lab.opj");
    let mut h = start(vec![data_file("save-graph")]);
    assert_eq!(project(&h).graphs().count(), 1, "the import made a graph");
    h.state_mut().path = Some(opj.clone());
    ctrl_s(&mut h);
    assert!(opj.exists());
    assert!(!dir.join("lab.opj.sheaf").exists(), "graphs need no companion any more");
    let h2 = start(vec![opj.clone()]);
    assert_eq!(project(&h2).graphs().count(), 1, "the graph is read back from the .opj");
    assert_eq!(project(&h2).workbooks().count(), 1);
}

#[test]
fn a_save_that_loses_nothing_writes_no_companion_and_deletes_an_old_one() {
    let dir = save_dir("clean");
    let opj = dir.join("lab.opj");
    let mut h = start(vec![data_file("save-clean")]);
    add_formula(&mut h);
    h.state_mut().path = Some(opj.clone());
    ctrl_s(&mut h);
    assert!(dir.join("lab.opj.sheaf").exists());
    // Drop the formula: now the .opj holds everything.
    h.state_mut().message = None;
    h.state_mut().doc.apply("Clear formula", |p| {
        if let Some(Window::Workbook(b)) = p.windows.first_mut() {
            b.sheets[0].columns[1].formula = None;
        }
    });
    settle(&mut h);
    ctrl_s(&mut h);
    assert!(opj.exists());
    assert!(!dir.join("lab.opj.sheaf").exists(), "the companion Sheaf wrote is deleted");
    assert!(h.state().message.is_none(), "nothing to report");
    assert!(h.state().status.starts_with("Saved"), "{}", h.state().status);
}

#[test]
fn a_project_origin_refuses_is_saved_as_the_companion_only_with_an_explanation() {
    let dir = save_dir("refuse");
    let opj = dir.join("lab.opj");
    let mut h = start(vec![data_file("save-refuse")]);
    h.state_mut().doc.apply("Rename sheet", |p| {
        if let Some(Window::Workbook(b)) = p.windows.first_mut() {
            b.sheets[0].name = "a sheet name that is far too long for Origin to store".into();
        }
    });
    settle(&mut h);
    h.state_mut().path = Some(opj.clone());
    ctrl_s(&mut h);
    assert!(!opj.exists(), "no half-written .opj");
    assert!(dir.join("lab.opj.sheaf").exists());
    let (title, text) = h.state().message.clone().expect("the refusal is explained");
    assert!(title.contains("could not be written") && text.contains("too long"), "{title}: {text}");
    assert!(!h.state().doc.dirty, "the data is safe in the companion");
}

#[test]
fn a_project_opened_from_sheaf_keeps_saving_to_sheaf() {
    let dir = save_dir("keep-sheaf");
    let sheaf = dir.join("lab.sheaf");
    let mut h = start(vec![data_file("save-keep")]);
    h.state_mut().save_to(sheaf.clone());
    assert!(sheaf.exists());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "no .opj and no companion");
    let mut h2 = start(vec![sheaf.clone()]);
    assert_eq!(h2.state().path.as_deref(), Some(sheaf.as_path()));
    h2.state_mut().doc.apply("Touch", |_| {});
    ctrl_s(&mut h2);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "Ctrl+S wrote the same .sheaf");
}

#[test]
fn a_name_without_a_known_extension_is_saved_as_opj() {
    let dir = save_dir("ext");
    let mut h = start(vec![data_file("save-ext")]);
    assert!(h.state_mut().save_to(dir.join("lab")));
    assert!(dir.join("lab.opj").exists());
    assert!(!dir.join("lab").exists());
}

#[test]
fn saving_over_an_imported_origin_file_keeps_a_copy_of_the_original() {
    let dir = save_dir("keep-original");
    let opj = dir.join("lab.opj");
    let mut h = start(vec![data_file("save-orig")]);
    assert!(h.state_mut().save_to(opj.clone()));
    // Pretend Origin wrote this file: its bytes are what the copy must hold.
    let original = std::fs::read(&opj).unwrap();
    let mut h2 = start(vec![]);
    h2.state_mut().foreign_source = Some(opj.clone());
    h2.state_mut().doc.apply("Touch", |_| {});
    assert!(h2.state_mut().save_to(opj.clone()));
    assert_eq!(std::fs::read(dir.join("lab.opj.bak")).unwrap(), original);
    assert!(h2.state().status.contains("lab.opj.bak"));
}

/// A mouse click at a point of the screen.
fn click_at(h: &mut Harness<'static, SheafApp>, at: egui::Pos2) {
    pause(h);
    h.event(egui::Event::PointerMoved(at));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    settle(h);
}

fn window_ids(h: &Harness<'static, SheafApp>) -> (WindowId, WindowId) {
    let p = project(h);
    (p.workbooks().next().unwrap().id, p.graphs().next().unwrap().id)
}

#[test]
fn clicking_a_window_makes_it_the_active_one_and_brings_it_to_the_front() {
    let mut h = start(vec![data_file("activate")]);
    let (book, graph) = window_ids(&h);
    assert_eq!(h.state().active, Some(graph), "the newest window is active");
    click(&mut h, "Title Book1 - sample");
    assert_eq!(h.state().active, Some(book));
    assert_eq!(h.state().workspace.shown().last(), Some(&book), "and on top of the stack");
    assert_eq!(bar_names(&h).get(4).map(String::as_str), Some("Column"), "the workbook's menu bar");
    // A click into the body of the other window, in the part the first one does not cover.
    let title = h.get_by_label("Title Graph1 - sample").rect().center();
    click_at(&mut h, title + egui::vec2(0.0, 315.0));
    assert_eq!(h.state().active, Some(graph));
    assert_eq!(h.state().workspace.shown().last(), Some(&graph));
    assert_eq!(bar_names(&h).get(4).map(String::as_str), Some("Data"), "the graph's menu bar");
}

#[test]
fn maximise_fills_the_workspace_and_restore_gives_the_old_size_back() {
    let mut h = start(vec![data_file("maximise")]);
    let width = |h: &Harness<'static, SheafApp>| h.get_by_label("Title Graph1 - sample").rect().width();
    let (_, graph) = window_ids(&h);
    let normal = width(&h);
    assert!(normal < 700.0);
    click(&mut h, "Maximise Graph1 - sample");
    assert_eq!(h.state().workspace.mode(graph), Some(crate::workspace::Mode::Maximised));
    assert!(width(&h) > 850.0, "fills the area right of the docked project explorer: {}", width(&h));
    click(&mut h, "Restore Graph1 - sample");
    assert_eq!(h.state().workspace.mode(graph), Some(crate::workspace::Mode::Normal));
    assert!((width(&h) - normal).abs() < 2.0, "back to {normal}, is {}", width(&h));
}

#[test]
fn minimise_makes_a_bar_and_clicking_the_bar_restores() {
    let mut h = start(vec![data_file("minimise")]);
    let (book, graph) = window_ids(&h);
    click(&mut h, "Minimise Graph1 - sample");
    assert_eq!(h.state().workspace.mode(graph), Some(crate::workspace::Mode::Minimised));
    assert!(h.query_by_label("Title Graph1 - sample").is_none());
    assert_eq!(h.state().active, Some(book), "the next window becomes active");
    click(&mut h, "Restore Graph1 - sample");
    assert_eq!(h.state().workspace.mode(graph), Some(crate::workspace::Mode::Normal));
    assert_eq!(h.state().active, Some(graph));
}

#[test]
fn closing_a_window_hides_it_and_keeps_its_data_until_it_is_shown_again() {
    let mut h = start(vec![data_file("close")]);
    let (book, graph) = window_ids(&h);
    click(&mut h, "Close Graph1 - sample");
    assert!(h.query_by_label("Title Graph1 - sample").is_none(), "no longer in the workspace");
    assert_eq!(project(&h).graphs().count(), 1, "but still in the project");
    assert_eq!(project(&h).graphs().next().unwrap().layers[0].plots.len(), 2);
    assert_eq!(h.state().active, Some(book));
    // The project explorer still lists it, and clicking it shows it again.
    show_window(&mut h, "Graph1 - sample");
    assert_eq!(h.state().active, Some(graph));
    assert!(h.query_by_label("Title Graph1 - sample").is_some());
    // A closed window is not reopened by the next change of the project.
    click(&mut h, "Close Graph1 - sample");
    menu_path(&mut h, &["File", "New", "Worksheet..."]);
    assert!(h.query_by_label("Title Graph1 - sample").is_none());
}

#[test]
fn a_million_row_workbook_scrolls_quickly_inside_a_window() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-big-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("big.csv");
    let mut text = String::from(
        "x,y
",
    );
    for i in 0..1_000_000 {
        text.push_str(&format!(
            "{i},{}
",
            i % 977
        ));
    }
    std::fs::write(&path, text).unwrap();
    let mut h = start(vec![path]);
    assert_eq!(project(&h).workbooks().next().unwrap().sheet().row_count(), 1_000_000);
    // Scroll with the mouse wheel over the cells of the workbook, frame after frame.
    click(&mut h, "Title Book1 - big");
    let at = h.get_by_label("Title Book1 - big").rect().center() + egui::vec2(0.0, 120.0);
    h.event(egui::Event::PointerMoved(at));
    let frames = 30;
    let started = std::time::Instant::now();
    for _ in 0..frames {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -40.0),
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    // The view really moved: row numbers far below the first screen are drawn.
    let deepest = h
        .query_all(By::new().role(egui::accesskit::Role::Label))
        .filter_map(|n| n.value().and_then(|v| v.parse::<usize>().ok()))
        .max()
        .unwrap_or(0);
    assert!(deepest > 1000, "the grid did not scroll (deepest row shown: {deepest})");
    let per_frame = started.elapsed().as_secs_f64() * 1000.0 / frames as f64;
    println!("1,000,000 rows in a window: {per_frame:.1} ms per frame while scrolling");
    assert!(per_frame < 150.0, "scrolling is too slow: {per_frame:.1} ms per frame");
}

// ----- menu bars per window type, the Window menu, shortcuts -----------------------------

/// The names of the top-level menus on screen, left to right.
fn bar_names(h: &Harness<'static, SheafApp>) -> Vec<String> {
    let mut nodes: Vec<_> = h
        .query_all(By::new().role(egui::accesskit::Role::Button))
        .filter(|n| n.rect().min.y < 20.0 && n.rect().min.x < 800.0)
        .map(|n| (n.rect().min.x, n.accesskit_node().label().unwrap_or_default().to_string()))
        .collect();
    nodes.sort_by(|a, b| a.0.total_cmp(&b.0));
    nodes.into_iter().map(|(_, l)| l).collect()
}

const WORKBOOK_BAR: [&str; 13] = [
    "File",
    "Edit",
    "View",
    "Plot",
    "Column",
    "Worksheet",
    "Analysis",
    "Statistics",
    "Image",
    "Tools",
    "Format",
    "Window",
    "Help",
];
const GRAPH_BAR: [&str; 11] =
    ["File", "Edit", "View", "Graph", "Data", "Analysis", "Gadgets", "Tools", "Format", "Window", "Help"];
const NO_WINDOW_BAR: [&str; 5] = ["File", "View", "Tools", "Window", "Help"];

/// True when the entry is drawn and can be clicked.
fn entry_enabled(h: &Harness<'static, SheafApp>, text: &str) -> bool {
    !menu_entry(h, text).accesskit_node().is_disabled()
}

/// Closes open menus.
fn close_menus(h: &mut Harness<'static, SheafApp>) {
    for _ in 0..4 {
        h.key_press(egui::Key::Escape);
        settle(h);
    }
}

#[test]
fn activating_a_workbook_and_then_a_graph_swaps_the_menu_bar() {
    let mut h = start(vec![data_file("bars")]);
    // The graph was made last and is active.
    assert_eq!(bar_names(&h), GRAPH_BAR);
    click(&mut h, "Title Book1 - sample");
    assert_eq!(bar_names(&h), WORKBOOK_BAR);
    // The graph's title bar is partly under the workbook: use the Window menu.
    menu_path(&mut h, &["Window", "2 Graph1 - sample"]);
    assert_eq!(bar_names(&h), GRAPH_BAR);
    // Closing every window leaves the short bar.
    click(&mut h, "Close Graph1 - sample");
    click(&mut h, "Close Book1 - sample");
    assert_eq!(bar_names(&h), NO_WINDOW_BAR);
    // An open window brings its own bar back.
    show_window(&mut h, "Book1 - sample");
    assert_eq!(bar_names(&h), WORKBOOK_BAR);
}

/// Opens the menus along the path, checks that the last entry is there and can be used, and
/// closes the menus again.
fn assert_reachable(h: &mut Harness<'static, SheafApp>, path: &[&str]) {
    let (last, parents) = path.split_last().unwrap();
    for text in parents {
        menu_entry(h, text).click();
        settle(h);
    }
    assert!(entry_enabled(h, last), "{path:?} is greyed");
    close_menus(h);
}

#[test]
fn every_command_of_a_workbook_is_reachable_from_its_origin_menu() {
    let mut h = start(vec![data_file("reach-book")]);
    click(&mut h, "Title Book1 - sample");
    click_text(&mut h, "C(Y1)");
    let paths: &[&[&str]] = &[
        &["File", "New", "Project"],
        &["File", "New", "Worksheet..."],
        &["File", "Open..."],
        &["File", "Save Project"],
        &["File", "Save Project As..."],
        &["File", "Import", "Single ASCII..."],
        &["File", "Import", "Multiple ASCII..."],
        &["File", "Import", "Comma Delimited (CSV)..."],
        &["File", "Export", "Worksheet as ASCII..."],
        &["File", "Exit"],
        &["Edit", "Undo"],
        &["Edit", "Delete"],
        &["Plot", "Line", "Line"],
        &["Plot", "Symbol", "Scatter"],
        &["Plot", "Line + Symbol", "Line + Symbol"],
        &["Plot", "User Templates", "Cyclic voltammetry overlay"],
        &["Column", "Set As", "X"],
        &["Column", "Set As", "Y"],
        &["Column", "Set As", "Z"],
        &["Column", "Set As", "Label"],
        &["Column", "Set As", "Disregard"],
        &["Column", "Set As", "Y Error"],
        &["Column", "Set As", "X Error"],
        &["Column", "Set Column Values..."],
        &["Column", "Recalculate"],
        &["Column", "Add New Columns..."],
        &["Worksheet", "Sort Worksheet", "Ascending"],
        &["Worksheet", "Sort Worksheet", "Descending"],
        &["Worksheet", "Worksheet Query..."],
        &["Analysis", "Mathematics", "Simple Curve Math..."],
        &["Analysis", "Mathematics", "Differentiate..."],
        &["Analysis", "Mathematics", "Integrate..."],
        &["Analysis", "Mathematics", "Integrate Ranges..."],
        &["Analysis", "Mathematics", "Interpolate/Extrapolate..."],
        &["Analysis", "Mathematics", "Normalize Columns..."],
        &["Analysis", "Data Manipulation", "Subtract Reference Data..."],
        &["Analysis", "Fitting", "Linear Fit..."],
        &["Analysis", "Fitting", "Polynomial Fit..."],
        &["Analysis", "Fitting", "Nonlinear Curve Fit..."],
        &["Analysis", "Signal Processing", "Smooth..."],
        &["Analysis", "Peaks and Baseline", "Find Peaks..."],
        &["Analysis", "Peaks and Baseline", "Subtract Baseline..."],
        &["Statistics", "Descriptive Statistics", "Statistics on Columns..."],
        &["Format", "Column..."],
        &["Window", "Cascade"],
        &["Window", "Tile Horizontally"],
        &["Window", "Tile Vertically"],
        &["Window", "Properties..."],
        &["Help", "About Sheaf..."],
    ];
    for path in paths {
        assert_reachable(&mut h, path);
    }
}

#[test]
fn every_command_of_a_graph_is_reachable_from_its_origin_menu() {
    let mut h = start(vec![data_file("reach-graph")]);
    assert_eq!(bar_names(&h), GRAPH_BAR);
    let paths: &[&[&str]] = &[
        &["File", "Export", "Graphs..."],
        &["File", "Import", "Single ASCII..."],
        &["Edit", "Undo"],
        &["Graph", "Legend", "Legend Properties..."],
        &["Format", "Page Properties..."],
        &["Format", "Plot Properties..."],
        &["Format", "Axes", "X Axis..."],
        &["Format", "Axes", "Y Axis..."],
        &["Window", "Cascade"],
        &["Window", "Properties..."],
        &["Help", "About Sheaf..."],
    ];
    for path in paths {
        assert_reachable(&mut h, path);
    }
}

#[test]
fn menu_items_sheaf_lacks_are_greyed_with_origins_name_and_a_tooltip() {
    let mut h = start(vec![data_file("grey")]);
    click(&mut h, "Title Book1 - sample");
    menu_path(&mut h, &["Plot", "Column/Bar/Pie"]);
    assert!(!entry_enabled(&h, "Stacked Column"), "an item Sheaf lacks is greyed");
    menu_entry(&h, "Stacked Column").hover();
    h.run_steps(30);
    assert!(h.query_by_label_contains("Not available in Sheaf yet").is_some(), "the tooltip says why");
    close_menus(&mut h);
    // A greyed item with a shortcut still shows the key.
    click(&mut h, "File");
    assert!(!entry_enabled(&h, "Print... Ctrl+P"));
}

#[test]
fn a_command_that_needs_a_selection_is_greyed_until_a_column_is_selected() {
    let mut h = start(vec![data_file("needs")]);
    click(&mut h, "Title Book1 - sample");
    click(&mut h, "Column");
    assert!(!entry_enabled(&h, "Set Column Values... Ctrl+Q"), "no column is selected");
    close_menus(&mut h);
    click_text(&mut h, "C(Y1)");
    click(&mut h, "Column");
    assert!(entry_enabled(&h, "Set Column Values... Ctrl+Q"));
}

#[test]
fn window_menu_lists_the_windows_in_order_and_choosing_one_activates_it() {
    let mut h = start(vec![data_file("windows")]);
    let (book, graph) = window_ids(&h);
    assert_eq!(h.state().active, Some(graph));
    click(&mut h, "Window");
    // Numbered in project order; the active window (the graph) is the one the tick is drawn at.
    assert!(h.query_by_label("1 Book1 - sample").is_some());
    assert!(h.query_by_label("2 Graph1 - sample").is_some());
    assert_eq!(h.state().menu_context().active, Some(graph));
    h.get_by_label("1 Book1 - sample").click();
    settle(&mut h);
    assert_eq!(h.state().active, Some(book), "choosing a window in the list activates it");
    assert_eq!(bar_names(&h), WORKBOOK_BAR);
}

#[test]
fn cascade_and_tile_arrange_the_windows() {
    let mut h = start(vec![data_file("tile")]);
    let rect = |h: &Harness<'static, SheafApp>, title: &str| h.get_by_label(&format!("Title {title}")).rect();
    menu_path(&mut h, &["Window", "Tile Vertically"]);
    let (a, b) = (rect(&h, "Book1 - sample"), rect(&h, "Graph1 - sample"));
    assert!((a.min.y - b.min.y).abs() < 1.0, "side by side: same top");
    assert!(a.max.x <= b.min.x + 8.0 || b.max.x <= a.min.x + 8.0, "and not on each other: {a:?} {b:?}");
    menu_path(&mut h, &["Window", "Tile Horizontally"]);
    let (a, b) = (rect(&h, "Book1 - sample"), rect(&h, "Graph1 - sample"));
    assert!((a.min.x - b.min.x).abs() < 1.0, "one above the other: same left edge");
    assert!((a.min.y - b.min.y).abs() > 100.0);
    menu_path(&mut h, &["Window", "Cascade"]);
    let (a, b) = (rect(&h, "Book1 - sample"), rect(&h, "Graph1 - sample"));
    assert!(((b.min - a.min).x - 26.0).abs() < 1.0 && ((b.min - a.min).y - 26.0).abs() < 1.0, "{a:?} {b:?}");
}

#[test]
fn origins_keys_run_the_commands_they_belong_to() {
    let mut h = start(vec![data_file("keys2")]);
    click(&mut h, "Title Book1 - sample");
    click_text(&mut h, "C(Y1)");
    // Ctrl+Q is Set Column Values ...
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Q);
    settle(&mut h);
    assert!(h.state().set_values.is_some());
    h.get_all_by_label("Cancel").last().unwrap().click();
    settle(&mut h);
    // ... Ctrl+D adds a column ...
    let columns = |h: &Harness<'static, SheafApp>| project(h).workbooks().next().unwrap().sheet().columns.len();
    let before = columns(&h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::D);
    settle(&mut h);
    assert_eq!(columns(&h), before + 1);
    // ... and Ctrl+Y is Origin's key for Nonlinear Curve Fit, not redo.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    settle(&mut h);
    assert!(h.state().fit.is_some(), "the fit window opened");
    assert_eq!(columns(&h), before + 1, "and nothing was redone");
    // Ctrl+I is no longer Import (Origin has Zoom In there): it does nothing in a workbook.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::I);
    settle(&mut h);
    assert!(h.state().import.is_none());
}

#[test]
fn alt_and_the_underlined_letter_opens_a_top_level_menu() {
    let mut h = start(vec![data_file("alt")]);
    click(&mut h, "Title Book1 - sample");
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::F);
    settle(&mut h);
    assert!(h.query_by_label("Save Project Ctrl+S").is_some(), "Alt+F opens File");
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::C);
    settle(&mut h);
    assert!(h.query_by_label("Save Project Ctrl+S").is_none());
    assert!(!not_toolbar(&h, "Set Column Values").is_empty(), "Alt+C opens Column");
    // Alt+K is the letter of Worksheet.
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::K);
    settle(&mut h);
    assert!(!not_toolbar(&h, "Sort Worksheet").is_empty());
}

// ----- the workbook window as in Origin: selection, clipboard, label cells, right-click menus -----

/// The centre of the text drawn in the grid.
fn text_pos(h: &Harness<'static, SheafApp>, text: &str) -> egui::Pos2 {
    grid_text(h, text).rect().center()
}

/// Press at one point, drag to another and release.
fn drag(h: &mut Harness<'static, SheafApp>, from: egui::Pos2, to: egui::Pos2) {
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(egui::Event::PointerMoved(from));
    h.step();
    h.event(button(from, true));
    h.step();
    h.event(egui::Event::PointerMoved(from.lerp(to, 0.5)));
    h.step();
    h.event(egui::Event::PointerMoved(to));
    h.step();
    h.event(button(to, false));
    h.step();
    settle(h);
}

/// Lets more than a double-click interval of simulated time pass, so that the next click is not
/// counted as the second click of a double click (egui counts clicks by time only).
fn pause(h: &mut Harness<'static, SheafApp>) {
    h.run_steps(25);
}

/// A click on the text drawn in the grid with Shift, Ctrl or both held.
fn click_text_with(h: &mut Harness<'static, SheafApp>, text: &str, modifiers: egui::Modifiers) {
    pause(h);
    let at = text_pos(h, text);
    h.event(egui::Event::PointerMoved(at));
    h.step();
    for pressed in [true, false] {
        h.input_mut().modifiers = modifiers;
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers });
        h.step();
    }
    h.input_mut().modifiers = egui::Modifiers::NONE;
    settle(h);
}

/// Two quick clicks at one point (a double click).
fn double_click_at(h: &mut Harness<'static, SheafApp>, at: egui::Pos2) {
    // Long enough after the last click that this is not counted as part of it.
    h.run_steps(25);
    h.event(egui::Event::PointerMoved(at));
    for pressed in [true, false, true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    settle(h);
}

fn book_ui<'a>(h: &'a Harness<'static, SheafApp>) -> &'a BookUi {
    h.state().books.values().next().expect("a workbook window was drawn")
}

fn sheet<'a>(h: &'a Harness<'static, SheafApp>) -> &'a sheaf_core::Worksheet {
    project(h).workbooks().next().unwrap().sheet()
}

fn column_id(h: &Harness<'static, SheafApp>, short: &str) -> ColumnId {
    sheet(h).column_by_name(short).unwrap().id
}

/// Drags over the cells showing these two texts.
fn select_block(h: &mut Harness<'static, SheafApp>, from: &str, to: &str) {
    let (a, b) = (text_pos(h, from), text_pos(h, to));
    drag(h, a, b);
}

fn paste(h: &mut Harness<'static, SheafApp>, text: &str) {
    h.event(egui::Event::Paste(text.to_string()));
    settle(h);
}

fn ctrl(h: &mut Harness<'static, SheafApp>, key: egui::Key) {
    h.key_press_modifiers(egui::Modifiers::COMMAND, key);
    settle(h);
}

#[test]
fn clicking_headers_selects_columns_rows_and_everything_and_dragging_selects_a_block() {
    let mut h = start(vec![data_file("select")]);
    show_window(&mut h, "Book1 - sample");
    let (a, b, c) = (column_id(&h, "A"), column_id(&h, "B"), column_id(&h, "C"));

    // A click on a column name selects the column; Ctrl extends by one, Shift by a range.
    click_text(&mut h, "B(Y1)");
    assert_eq!(book_ui(&h).sel, Sel::Columns);
    assert_eq!(book_ui(&h).selected, vec![b]);
    click_text_with(&mut h, "C(Y1)", egui::Modifiers::COMMAND);
    settle(&mut h);
    assert_eq!(book_ui(&h).selected, vec![b, c]);
    click_text_with(&mut h, "C(Y1)", egui::Modifiers::COMMAND);
    settle(&mut h);
    assert_eq!(book_ui(&h).selected, vec![b], "Ctrl-click on a selected column takes it out again");
    click_text(&mut h, "C(Y1)");
    click_text_with(&mut h, "A(X1)", egui::Modifiers::SHIFT);
    settle(&mut h);
    assert_eq!(book_ui(&h).selected, vec![a, b, c], "Shift-click selects the range");

    // The same selection is what Plot acts on.
    click_text(&mut h, "B(Y1)");
    click_text_with(&mut h, "C(Y1)", egui::Modifiers::SHIFT);
    settle(&mut h);
    menu_path(&mut h, &["Plot", "Symbol", "Scatter"]);
    let plots = &project(&h).graphs().last().unwrap().layers[0].plots;
    assert_eq!(plots.len(), 2, "both selected columns are plotted");

    // A click on a row number selects the row, with the cells themselves not filled.
    let title_x = text_pos(&h, "Long Name").x;
    let row_y = text_pos(&h, "6.5").y;
    click_at(&mut h, egui::pos2(title_x, row_y));
    assert_eq!(book_ui(&h).sel, Sel::Rows { anchor: 2, end: 2 });
    assert!(book_ui(&h).selected.is_empty());
    let target = book_ui(&h).target(sheet(&h)).unwrap();
    assert_eq!((target.columns, target.rows), (vec![0, 1, 2], 2..3));

    // The corner above the row numbers selects everything.
    let corner_y = text_pos(&h, "A(X1)").y;
    click_at(&mut h, egui::pos2(title_x, corner_y));
    assert_eq!(book_ui(&h).sel, Sel::All);
    assert_eq!(book_ui(&h).selected, vec![a, b, c]);

    // Dragging over cells selects a block; the columns it touches are the selected columns.
    select_block(&mut h, "2.5", "20.5");
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 0), end: (2, 1) });
    assert_eq!(book_ui(&h).selected, vec![b, c]);
    let target = book_ui(&h).target(sheet(&h)).unwrap();
    assert_eq!((target.columns, target.rows, target.top_left), (vec![1, 2], 0..2, (1, 0)));
    // Showing another sheet or deleting the columns leaves nothing stale behind.
    menu(&mut h, "Column", "Add New Columns...");
    menu(&mut h, "Edit", "Undo Add column");
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 0), end: (2, 1) });
}

#[test]
fn copy_gives_tab_separated_text_and_paste_puts_it_back_in_one_undo_step() {
    let mut h = start(vec![data_file("copypaste")]);
    show_window(&mut h, "Book1 - sample");
    select_block(&mut h, "2.5", "20.5");
    ctrl(&mut h, egui::Key::C);
    assert_eq!(h.state().copied, "2.5\t30.5\n4.5\t20.5\n");
    assert_eq!(h.state().doc.undo_label(), Some("Import data"), "copying changes nothing");

    // Paste at the cell two rows down (Ctrl+V reaches the program as an event with the text).
    click_text(&mut h, "6.5");
    let copied = h.state().copied.clone();
    paste(&mut h, &copied);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 2.5, 4.5]);
    assert_eq!(column(&h, "C"), [30.5, 20.5, 30.5, 20.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Paste"));
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 2), end: (2, 3) }, "the pasted block is selected");
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5]);

    // The Edit menu does the same: Copy including label rows writes the label rows first.
    select_block(&mut h, "2.5", "20.5");
    menu_path(&mut h, &["Edit", "Copy", "Copy including label rows"]);
    assert!(h.state().copied.starts_with("up\tdown\n\t\n\t\n"), "{:?}", h.state().copied);
    assert!(h.state().copied.ends_with("2.5\t30.5\n4.5\t20.5\n"));
}

#[test]
fn plain_copy_and_copy_full_precision_both_write_every_digit() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-fullprec-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.csv");
    std::fs::write(&path, "t,v\n1,0.1234567891234\n2,5\n").unwrap();
    let mut h = start(vec![path]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "B(Y1)");
    ctrl(&mut h, egui::Key::C);
    assert_eq!(
        h.state().copied,
        "0.1234567891234
5
",
        "plain Copy keeps every digit"
    );
    menu_path(&mut h, &["Edit", "Copy", "Copy full precision"]);
    assert_eq!(h.state().copied, "0.1234567891234\n5\n");
}

#[test]
fn paste_that_is_larger_than_the_sheet_adds_rows_and_columns_with_numbers_and_text() {
    let mut h = start(vec![data_file("grow")]);
    show_window(&mut h, "Book1 - sample");
    // The last cell of column B: a 2 x 3 block runs past the last row and the last column.
    click_text(&mut h, "8.5");
    paste(&mut h, "5\t6\t7\n8\t9\t10\n");
    let s = sheet(&h);
    assert_eq!(s.columns.len(), 4, "one column was added");
    assert_eq!(s.row_count(), 5, "and one row");
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 5.0, 8.0]);
    assert_eq!(column(&h, "C")[3..], [6.0, 9.0]);
    let d = s.column_by_name("D").unwrap().numeric().unwrap();
    assert_eq!(d[3..], [7.0, 10.0]);
    assert!(h.state().status.contains("1 new column"), "{}", h.state().status);
    assert_eq!(h.state().doc.undo_label(), Some("Paste"));
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(sheet(&h).columns.len(), 3);
    assert_eq!(sheet(&h).row_count(), 4);

    // Words stay text, numbers stay numbers; a comma-separated block works as well.
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewWorkbook, &ctx);
    settle(&mut h);
    let book = project(&h).workbooks().last().unwrap().id;
    h.state_mut().focus(book);
    settle(&mut h);
    let sheet = project(&h).workbook(book).unwrap().sheet().clone();
    h.state_mut().books.entry(book).or_default().select_cells(&sheet, (0, 0), (0, 0));
    paste(&mut h, "alpha,1.5\nbeta,2\ngamma,x\n");
    let s = project(&h).workbook(book).unwrap().sheet();
    assert_eq!(s.columns.len(), 2);
    assert_eq!(s.columns[0].text().unwrap(), ["alpha", "beta", "gamma"]);
    assert_eq!(s.columns[1].text().unwrap(), ["1.5", "2", "x"], "a column with a word in it stays text");
}

#[test]
fn delete_clears_the_selected_cells_and_cut_also_copies_each_in_one_undo_step() {
    let mut h = start(vec![data_file("clear")]);
    show_window(&mut h, "Book1 - sample");
    select_block(&mut h, "2.5", "20.5");
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert!(column(&h, "B")[..2].iter().all(|x| x.is_nan()));
    assert_eq!(column(&h, "B")[2..], [6.5, 8.5]);
    assert!(column(&h, "C")[..2].iter().all(|x| x.is_nan()));
    assert_eq!(h.state().doc.undo_label(), Some("Clear cells"));
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5]);

    // A selected column is emptied completely; the Delete key does not delete the column.
    click_text(&mut h, "C(Y1)");
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert!(column(&h, "C").is_empty());
    assert_eq!(sheet(&h).columns.len(), 3);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5]);

    // Cut copies first, then clears, as one step.
    select_block(&mut h, "6.5", "10.5");
    ctrl(&mut h, egui::Key::X);
    assert_eq!(h.state().copied, "6.5\t10.5\n");
    assert_eq!(h.state().doc.undo_label(), Some("Cut cells"));
    assert_eq!(column(&h, "B")[2..].iter().filter(|x| x.is_nan()).count(), 1);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
}

#[test]
fn a_calculated_column_is_left_alone_by_clear_and_paste() {
    let mut h = start(vec![data_file("locked")]);
    show_window(&mut h, "Book1 - sample");
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Set Column Values...");
    h.event(egui::Event::Text("col(B)*2".to_string()));
    settle(&mut h);
    click(&mut h, "OK");
    click_text(&mut h, "C(Y1)");
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!(column(&h, "C"), [5.0, 9.0, 13.0, 17.0]);
    assert!(h.state().status.contains("formula"), "{}", h.state().status);
}

#[test]
fn double_clicking_a_label_cell_edits_it_in_place_as_one_undo_step() {
    let mut h = start(vec![data_file("labels")]);
    show_window(&mut h, "Book1 - sample");
    // The cell under the column name in the Long Name row, then Units and Comments.
    let x = text_pos(&h, "B(Y1)").x;
    for (title, text) in [("Long Name", "Signal"), ("Units", "mA"), ("Comments", "Scan 1")] {
        let at = egui::pos2(x, text_pos(&h, title).y);
        double_click_at(&mut h, at);
        assert!(book_ui(&h).editing_label.is_some(), "{title} is being edited");
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.event(egui::Event::Text(text.to_string()));
        settle(&mut h);
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert!(book_ui(&h).editing_label.is_none());
        assert_eq!(h.state().doc.undo_label(), Some("Edit label"));
    }
    let b = sheet(&h).column_by_name("B").unwrap();
    assert_eq!((b.long_name.as_str(), b.units.as_str(), b.comments.as_str()), ("Signal", "mA", "Scan 1"));
    // The label is drawn in the band.
    grid_text(&h, "Signal");
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(sheet(&h).column_by_name("B").unwrap().comments, "");
    assert_eq!(sheet(&h).column_by_name("B").unwrap().units, "mA", "one step undoes one label");
    ctrl(&mut h, egui::Key::Z);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(sheet(&h).column_by_name("B").unwrap().long_name, "up", "all three edits are undone");
    // Escape leaves the label as it was.
    let at = egui::pos2(x, text_pos(&h, "Units").y);
    double_click_at(&mut h, at);
    h.event(egui::Event::Text("zzz".to_string()));
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert_eq!(sheet(&h).column_by_name("B").unwrap().units, "");
}

#[test]
fn a_user_parameter_gets_a_label_row_with_its_name_and_the_head_shows_designation_and_group() {
    let mut h = start(vec![data_file("params")]);
    show_window(&mut h, "Book1 - sample");
    // Heads: "A(X1)" is the X column, the Y columns after it belong to the same data set.
    for head in ["A(X1)", "B(Y1)", "C(Y1)"] {
        grid_text(&h, head);
    }
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Set parameter", |p| {
        p.column_mut(b).unwrap().params.insert("Sample".to_string(), "S-7".to_string());
    });
    settle(&mut h);
    grid_text(&h, "Sample");
    grid_text(&h, "S-7");
    // The label rows are in Origin's order above the data.
    let ys: Vec<f32> =
        ["Long Name", "Units", "Comments", "F(x)=", "Sample", "2.5"].iter().map(|t| text_pos(&h, t).y).collect();
    assert!(ys.windows(2).all(|w| w[0] < w[1]), "{ys:?}");
    // A second X column starts the next data set.
    right_click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Set As", "X"]);
    grid_text(&h, "C(X2)");
}

#[test]
fn the_lock_mark_shows_the_recalculate_mode_and_the_state_of_a_formula_column() {
    let mut h = start(vec![data_file("lock")]);
    show_window(&mut h, "Book1 - sample");
    assert!(h.query_by_label_contains("Lock of column").is_none(), "no lock without a formula");
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Set Column Values...");
    h.event(egui::Event::Text("col(B)*2".to_string()));
    settle(&mut h);
    click(&mut h, "Auto");
    click(&mut h, "OK");
    h.get_by_label("Lock of column C: auto, current");
    // The formula text is in the F(x)= row.
    grid_text(&h, "col(B)*2");
}

/// The y positions of these entries of an open menu, which must grow in this order.
fn assert_in_order(h: &Harness<'static, SheafApp>, names: &[&str]) {
    let ys: Vec<f32> = names.iter().map(|n| menu_entry(h, n).rect().min.y).collect();
    assert!(ys.windows(2).all(|w| w[0] < w[1]), "{names:?} are not in this order on the menu: {ys:?}");
}

#[test]
fn the_column_header_menu_has_origins_items_in_origins_order_and_runs_them() {
    let mut h = start(vec![data_file("headmenu")]);
    h.set_size(egui::vec2(1280.0, 1400.0));
    show_window(&mut h, "Book1 - sample");
    right_click_text(&mut h, "C(Y1)");
    assert_eq!(book_ui(&h).selected, vec![column_id(&h, "C")], "right-clicking selects the column first");
    assert_in_order(
        &h,
        &[
            "Plot",
            "Cut",
            "Copy",
            "Copy Columns to...",
            "Paste",
            "Insert",
            "Delete",
            "Clear",
            "Remove Links",
            "Set As",
            "Set As Categorical",
            "Set Column Values...",
            "Set Multiple Columns Values...",
            "Fill Column with",
            "Set Sampling Interval...",
            "Show X Column...",
            "Filter",
            "Mask",
            "Mask Cells by Condition...",
            "Sort Column",
            "Sort Worksheet",
            "Sort Columns by Label...",
            "Reverse Order",
            "Normalize...",
            "Frequency Count...",
            "Statistics on Columns...",
            "Hide/Unhide Columns",
            "Move Columns",
            "Swap Columns...",
            "Column Width...",
            "Slide Show of Dependent Graphs",
            "Add or Update Sparklines...",
            "Format Cells...",
            "Properties...",
        ],
    );
    // What Sheaf has is enabled, the rest is greyed.
    for have in ["Plot", "Cut", "Copy", "Paste", "Delete", "Clear", "Set As", "Properties...", "Normalize..."] {
        assert!(entry_enabled(&h, have), "{have} should be enabled");
    }
    for lack in ["Insert", "Copy Columns to...", "Set As Categorical", "Frequency Count...", "Column Width..."] {
        assert!(!entry_enabled(&h, lack), "{lack} should be greyed");
    }
    // Clear empties the column and one undo brings it back.
    click_containing(&mut h, "Clear");
    assert!(column(&h, "C").is_empty());
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5]);
    // Add New Columns adds one.
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Add New Columns...");
    assert_eq!(sheet(&h).columns.len(), 4);
    // Normalize opens its window for the selected column; Statistics on Columns too.
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Normalize...");
    assert!(h.state().analysis.is_some());
    h.state_mut().analysis = None;
    settle(&mut h);
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Statistics on Columns...");
    assert!(h.state().analysis.is_some());
    h.state_mut().analysis = None;
    settle(&mut h);
    // Delete removes the selected column.
    right_click_text(&mut h, "B(Y1)");
    menu_entry(&h, "Delete").click();
    settle(&mut h);
    assert_eq!(sheet(&h).columns.len(), 3, "four columns, one deleted");
    assert!(sheet(&h).column_by_name("B").is_none());
}

#[test]
fn the_cell_and_row_menus_copy_and_clear_and_grey_what_sheaf_lacks() {
    let mut h = start(vec![data_file("cellmenu")]);
    h.set_size(egui::vec2(1280.0, 1400.0));
    show_window(&mut h, "Book1 - sample");
    right_click_text(&mut h, "6.5");
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 2), end: (1, 2) }, "right-clicking selects the cell");
    assert_in_order(
        &h,
        &["Cut", "Copy", "Paste", "Paste Transpose", "Insert", "Clear", "Insert Graph...", "Format Cells..."],
    );
    assert!(entry_enabled(&h, "Copy"));
    assert!(!entry_enabled(&h, "Paste Transpose"));
    assert!(!entry_enabled(&h, "Insert Graph..."));
    click_containing(&mut h, "Clear");
    assert!(column(&h, "B")[2].is_nan());
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(column(&h, "B")[2], 6.5);

    // The row number's menu is the short list.
    let title_x = text_pos(&h, "Long Name").x;
    let at = egui::pos2(title_x, text_pos(&h, "6.5").y);
    h.event(egui::Event::PointerMoved(at));
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.step();
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    assert_eq!(book_ui(&h).sel, Sel::Rows { anchor: 2, end: 2 });
    assert_in_order(&h, &["Cut", "Copy", "Paste", "Insert", "Delete", "Clear", "Insert Graph...", "Format Cells..."]);
    assert!(h.query_by_label("Paste Transpose").is_none(), "the row menu is the short form");
    click_containing(&mut h, "Copy");
    assert_eq!(h.state().copied, "3\t6.5\t10.5\n");
}

/// Right-clicks the tab of a sheet.
fn right_click_tab(h: &mut Harness<'static, SheafApp>, name: &str) {
    right_click(h, name);
}

#[test]
fn the_sheet_tab_menu_adds_duplicates_renames_and_deletes_sheets() {
    let mut h = start(vec![data_file("tabs")]);
    show_window(&mut h, "Book1 - sample");
    let first = sheet(&h).name.clone();
    right_click_tab(&mut h, &first);
    assert_in_order(
        &h,
        &[
            "Insert",
            "Delete",
            "Add",
            "Add Graph as Sheet...",
            "Add Notes as Sheet",
            "Duplicate Without Data",
            "Duplicate",
            "Sheet Name to Show",
            "Name and Comments...",
            "Navigate...",
        ],
    );
    assert!(!entry_enabled(&h, "Add Notes as Sheet"));
    menu_entry(&h, "Add").click();
    settle(&mut h);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 2);
    assert_eq!(project(&h).workbooks().next().unwrap().active_sheet, 1, "the new sheet is shown");
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);

    // Duplicate copies the data under new column ids.
    right_click_tab(&mut h, &first);
    menu_entry(&h, "Duplicate").click();
    settle(&mut h);
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheets.len(), 2);
    assert_eq!(book.sheets[1].columns.len(), 3);
    assert_ne!(book.sheets[1].columns[0].id, book.sheets[0].columns[0].id);
    assert_eq!(book.sheets[1].columns[1].numeric(), book.sheets[0].columns[1].numeric());
    let copy_name = book.sheets[1].name.clone();

    // Name and Comments asks for a name and renames the sheet.
    right_click_tab(&mut h, &copy_name);
    menu_entry(&h, "Name and Comments...").click();
    settle(&mut h);
    h.state_mut().sheet_name.as_mut().expect("the dialog is open").name = "Run 2".to_string();
    click(&mut h, "OK");
    assert_eq!(project(&h).workbooks().next().unwrap().sheets[1].name, "Run 2");
    assert_eq!(h.state().doc.undo_label(), Some("Rename sheet"));

    // Names are compared without regard to case: "RUN 2" would clash with "Run 2".
    right_click_tab(&mut h, &first);
    menu_entry(&h, "Name and Comments...").click();
    settle(&mut h);
    h.state_mut().sheet_name.as_mut().expect("the dialog is open").name = "RUN 2".to_string();
    click(&mut h, "OK");
    assert_eq!(project(&h).workbooks().next().unwrap().sheets[0].name, first, "the clash was refused");
    assert!(h.state().status.contains("no other sheet"), "{}", h.state().status);

    // Delete removes the sheet; the last sheet cannot be deleted.
    right_click_tab(&mut h, "Run 2");
    menu_entry(&h, "Delete").click();
    settle(&mut h);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);
    right_click_tab(&mut h, &first);
    menu_entry(&h, "Delete").click();
    settle(&mut h);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);
    assert!(h.state().status.contains("at least one sheet"));
}

#[test]
fn the_corner_menu_lists_the_grid_switches_greyed() {
    let mut h = start(vec![data_file("corner")]);
    show_window(&mut h, "Book1 - sample");
    let title_x = text_pos(&h, "Long Name").x;
    let at = egui::pos2(title_x, text_pos(&h, "A(X1)").y);
    h.event(egui::Event::PointerMoved(at));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    settle(&mut h);
    assert_in_order(
        &h,
        &[
            "Row Labels for Column Label Rows",
            "Column Header",
            "Row Header",
            "Column Grid",
            "Row Grid",
            "Page Break Preview Lines",
        ],
    );
    assert!(!entry_enabled(&h, "Column Grid"));
}

#[test]
fn the_grid_shows_five_decimals_and_blank_for_missing_values() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-five-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.csv");
    std::fs::write(&path, "t,v\n1,0.841470984807897\n2,\n3,-0.7568\n").unwrap();
    let mut h = start(vec![path]);
    show_window(&mut h, "Book1 - sample");
    grid_text(&h, "0.84147");
    grid_text(&h, "-0.7568");
    assert!(h.query_all(By::new().role(egui::accesskit::Role::Label).value("--")).next().is_none());
}

// ----- toolbars --------------------------------------------------------------------------------

macro_rules! bar_button {
    ($h:expr, $bar:expr, $tip:expr) => {
        $h.get_by_label(&format!("{} toolbar: {}", $bar, $tip))
    };
}

fn bar_button_shown(h: &Harness<'static, SheafApp>, bar: &str, tip: &str) -> bool {
    h.query_by_label(&format!("{bar} toolbar: {tip}")).is_some()
}

#[test]
fn clicking_line_in_the_2d_graphs_bar_with_a_column_selected_makes_a_line_graph() {
    let mut h = start(vec![data_file("tb-line")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "C(Y1)");
    let c = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().id;
    let before = project(&h).graphs().count();
    bar_button!(h, "2D Graphs", "Line").click();
    settle(&mut h);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 1);
    let plots = &graphs[before].layers[0].plots;
    assert_eq!(plots.len(), 1);
    assert_eq!(plots[0].y, c);
}

#[test]
fn a_bar_is_hidden_and_shown_again_from_view_toolbars() {
    let mut h = start(Vec::new());
    assert!(bar_button_shown(&h, "Tools", "Pointer"));
    menu(&mut h, "View", "Toolbars...");
    settle(&mut h);
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Tools").click();
    settle(&mut h);
    assert!(!bar_button_shown(&h, "Tools", "Pointer"), "the Tools bar is hidden");
    assert!(bar_button_shown(&h, "Standard", "Open"), "the other bars stay");
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Tools").click();
    settle(&mut h);
    assert!(bar_button_shown(&h, "Tools", "Pointer"), "and shown again");
}

#[test]
fn ctrl_t_opens_the_toolbars_window() {
    let mut h = start(Vec::new());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::T);
    settle(&mut h);
    assert!(h.state().toolbars.dialog);
}

#[test]
fn greyed_toolbar_buttons_do_nothing_and_enabled_ones_follow_the_selection() {
    let mut h = start(vec![data_file("tb-grey")]);
    show_window(&mut h, "Book1 - sample");
    let windows = project(&h).windows.len();
    // Column is a 2D type Sheaf lacks: greyed, a click changes nothing.
    assert!(bar_button!(h, "2D Graphs", "Column").accesskit_node().is_disabled());
    bar_button!(h, "2D Graphs", "Column").click();
    settle(&mut h);
    assert_eq!(project(&h).windows.len(), windows);
    assert!(h.state().doc.undo_label().is_some_and(|l| !l.contains("Column")));
    // Line needs a selected column.
    click_text(&mut h, "C(Y1)");
    assert!(!bar_button!(h, "2D Graphs", "Line").accesskit_node().is_disabled());
    // Set as X is enabled only with columns selected; Pointer is on.
    assert!(!bar_button!(h, "Standard", "Set as X").accesskit_node().is_disabled());
    assert!(!bar_button!(h, "Tools", "Pointer").accesskit_node().is_disabled());
    assert!(bar_button!(h, "Tools", "Zoom In").accesskit_node().is_disabled());
}

#[test]
fn toolbar_buttons_follow_the_window_type() {
    let mut h = start(vec![data_file("tb-kind")]);
    click(&mut h, "Title Graph1 - sample");
    // In a graph window the plot buttons have no worksheet to plot from.
    assert!(bar_button!(h, "2D Graphs", "Line").accesskit_node().is_disabled());
    assert!(bar_button!(h, "Standard", "Add New Columns").accesskit_node().is_disabled());
    assert!(!bar_button!(h, "Standard", "Open").accesskit_node().is_disabled());
}

// ----- the side panels (ADR 0007, step 7) -------------------------------------------------

fn move_pointer(h: &mut Harness<'static, SheafApp>, at: egui::Pos2) {
    h.event(egui::Event::PointerMoved(at));
    settle(h);
}

/// A point in the middle of the grey workspace, away from every panel and window.
fn empty_workspace_point() -> egui::Pos2 {
    egui::pos2(900.0, 650.0)
}

/// True when a plain label (the status bar boxes, the log lines) shows exactly this text.
fn label_text(h: &Harness<'static, SheafApp>, text: &str) -> bool {
    h.query(By::new().role(egui::accesskit::Role::Label).value(text)).is_some()
}

/// True when some plain label shows text that contains this.
fn label_containing(h: &Harness<'static, SheafApp>, text: &str) -> bool {
    h.query_all(By::new().role(egui::accesskit::Role::Label))
        .any(|n| n.accesskit_node().value().is_some_and(|v| v.contains(text)))
}

#[test]
fn the_explorer_slides_out_when_the_mouse_is_on_its_tab_and_goes_when_the_mouse_leaves() {
    let mut h = start_auto_hide(vec![data_file("hover")]);
    // At rest only the vertical tab names are on screen, not the rows.
    assert!(h.query_by_label("Project Explorer tab").is_some());
    assert!(h.query_by_label("Book1 - sample").is_none());
    assert_eq!(h.state().panels.flyout(), None);
    h.get_by_label("Project Explorer tab").hover();
    settle(&mut h);
    assert_eq!(h.state().panels.flyout(), Some(PanelId::Explorer));
    assert!(h.query_by_label("Book1 - sample").is_some(), "the window list is shown");
    assert!(h.query_by_label("Graph1 - sample").is_some());
    // Moving into the panel keeps it open; moving to the workspace closes it.
    let row = h.get_by_label("Book1 - sample").rect().center();
    move_pointer(&mut h, row);
    assert_eq!(h.state().panels.flyout(), Some(PanelId::Explorer));
    move_pointer(&mut h, empty_workspace_point());
    assert_eq!(h.state().panels.flyout(), None);
    assert!(h.query_by_label("Book1 - sample").is_none());
}

#[test]
fn a_click_on_the_tab_keeps_the_panel_open_until_a_click_elsewhere() {
    let mut h = start_auto_hide(vec![data_file("sticky")]);
    click(&mut h, "Project Explorer tab");
    move_pointer(&mut h, empty_workspace_point());
    assert_eq!(h.state().panels.flyout(), Some(PanelId::Explorer), "opened by a click, it stays");
    click_at(&mut h, empty_workspace_point());
    assert_eq!(h.state().panels.flyout(), None);
}

#[test]
fn the_pin_docks_the_explorer_and_the_same_button_lets_it_go_back_to_the_strip() {
    let mut h = start_auto_hide(vec![data_file("pin")]);
    h.get_by_label("Project Explorer tab").hover();
    settle(&mut h);
    click(&mut h, "Pin Project Explorer");
    assert!(h.state().panels.is_pinned(PanelId::Explorer));
    assert_eq!(h.state().panels.flyout(), None);
    assert!(h.query_by_label("Project Explorer tab").is_none(), "a docked panel has no tab");
    move_pointer(&mut h, empty_workspace_point());
    assert!(h.query_by_label("Book1 - sample").is_some(), "still on screen with the mouse elsewhere");
    click(&mut h, "Auto Hide Project Explorer");
    assert!(!h.state().panels.is_pinned(PanelId::Explorer));
    assert!(h.query_by_label("Project Explorer tab").is_some());
    click_at(&mut h, empty_workspace_point());
    assert!(h.query_by_label("Book1 - sample").is_none());
}

#[test]
fn alt_1_and_the_view_menu_hide_and_show_the_project_explorer() {
    let mut h = start_auto_hide(vec![data_file("alt1")]);
    assert!(h.state().panels.is_shown(PanelId::Explorer));
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::Num1);
    settle(&mut h);
    assert!(!h.state().panels.is_shown(PanelId::Explorer));
    assert!(h.query_by_label("Project Explorer tab").is_none(), "its tab is gone");
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::Num1);
    settle(&mut h);
    assert!(h.state().panels.is_shown(PanelId::Explorer));
    assert!(h.query_by_label("Book1 - sample").is_some(), "shown again, slid out");
    // The same through the menu.
    menu(&mut h, "View", "Project Explorer");
    assert!(!h.state().panels.is_shown(PanelId::Explorer));
    // The X button of the panel hides it too.
    menu(&mut h, "View", "Project Explorer");
    assert!(h.state().panels.is_shown(PanelId::Explorer));
    click(&mut h, "Close Project Explorer");
    assert!(!h.state().panels.is_shown(PanelId::Explorer));
}

#[test]
fn a_double_click_shows_and_activates_a_window_and_a_single_click_only_selects_the_row() {
    let mut h = start(vec![data_file("dbl")]);
    let (book, graph) = window_ids(&h);
    click(&mut h, "Close Graph1 - sample");
    assert_eq!(h.state().active, Some(book));
    pause(&mut h);
    click(&mut h, "Graph1 - sample");
    assert_eq!(h.state().panels.explorer.selected, Some(graph), "the row is selected");
    assert_eq!(h.state().active, Some(book), "but nothing is activated");
    assert!(h.query_by_label("Title Graph1 - sample").is_none());
    show_window(&mut h, "Graph1 - sample");
    assert_eq!(h.state().active, Some(graph));
    assert!(h.query_by_label("Title Graph1 - sample").is_some());
}

#[test]
fn the_explorer_lists_name_long_name_short_name_and_type_with_the_active_window_in_bold() {
    let mut h = start(vec![data_file("cols")]);
    let (book, graph) = window_ids(&h);
    for head in ["Name", "Long Name", "Short Name", "Type"] {
        assert!(h.query_by_label(&format!("Sort by {head}")).is_some(), "{head}");
    }
    assert_eq!(h.state().panels.explorer.bold, vec![graph], "the graph was made last and is active");
    show_window(&mut h, "Book1 - sample");
    assert_eq!(h.state().panels.explorer.bold, vec![book]);
    // The folder tree shows the root, named after the project (this one has no file yet).
    assert!(h.query_by_label("UNTITLED").is_some());
}

#[test]
fn the_window_menu_of_the_explorer_has_origins_commands_and_the_live_ones_work() {
    let mut h = start(vec![data_file("ctx")]);
    let (book, graph) = window_ids(&h);
    right_click(&mut h, "Book1 - sample");
    for name in ["Show", "Hide", "Rename...", "Delete", "Duplicate", "Properties..."] {
        assert!(h.query_by_label(name).is_some(), "{name}");
    }
    assert!(h.get_by_label("Duplicate").accesskit_node().is_disabled(), "Sheaf cannot duplicate a window yet");
    click(&mut h, "Hide");
    assert!(!h.state().workspace.is_shown(book));
    assert_eq!(project(&h).workbooks().count(), 1, "hiding keeps the data");
    right_click(&mut h, "Graph1 - sample");
    click(&mut h, "Properties...");
    assert!(h.state().rename.as_ref().is_some_and(|r| r.window == graph));
    click(&mut h, "Cancel");
    right_click(&mut h, "Book1 - sample");
    click(&mut h, "Show");
    assert!(h.state().workspace.is_shown(book));
    assert_eq!(h.state().active, Some(book));
}

#[test]
fn windows_move_into_folders_and_the_list_shows_the_selected_folder() {
    let mut h = start(vec![data_file("folders")]);
    let (book, graph) = window_ids(&h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewFolder(Vec::new()), &ctx);
    settle(&mut h);
    assert_eq!(project(&h).root.folders.len(), 1);
    assert_eq!(h.state().panels.explorer.folder, vec![0], "the new folder is selected");
    assert!(h.query_by_label("Folder1").is_some(), "and shown in the tree");
    h.state_mut().apply(Action::MoveWindow { window: book, folder: vec![0] }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).root.folders[0].windows, [book]);
    // Folder1 is selected: its list holds the book only.
    assert!(h.query_by_label("Book1 - sample").is_some());
    assert!(h.query_by_label("Graph1 - sample").is_none());
    click(&mut h, "UNTITLED");
    assert!(h.query_by_label("Book1 - sample").is_none(), "the root lists what no folder holds");
    assert!(h.query_by_label("Graph1 - sample").is_some());
    assert_eq!(h.state().panels.explorer.folder, Vec::<usize>::new());
    // One undo step moves it back, another removes the folder.
    h.state_mut().apply(Action::Undo, &ctx);
    assert!(project(&h).root.folders[0].windows.is_empty());
    h.state_mut().apply(Action::Undo, &ctx);
    assert!(project(&h).root.folders.is_empty());
    assert!(project(&h).window(graph).is_some());
}

#[test]
fn dragging_a_row_onto_a_folder_moves_the_window_into_it() {
    let mut h = start(vec![data_file("drag")]);
    let (book, _) = window_ids(&h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewFolder(Vec::new()), &ctx);
    click(&mut h, "UNTITLED");
    pause(&mut h);
    let from = h.get_by_label("Book1 - sample").rect().center();
    let to = h.get_by_label("Folder1").rect().center();
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(egui::Event::PointerMoved(from));
    h.step();
    h.event(button(from, true));
    h.step();
    for k in 1..=6 {
        h.event(egui::Event::PointerMoved(from + (to - from) * (k as f32 / 6.0)));
        h.step();
    }
    h.event(button(to, false));
    h.step();
    settle(&mut h);
    assert_eq!(project(&h).root.folders[0].windows, [book], "dropped on the folder");
    assert_eq!(h.state().doc.undo_label(), Some("Move window"));
}

/// How many pixels of the rendered graph are close to this colour.
fn pixels_like(h: &Harness<'static, SheafApp>, graph: WindowId, c: sheaf_core::Color) -> usize {
    let raster = sheaf_plot::render_raster(project(h), graph, 1.0).expect("the graph renders");
    let near = |a: u8, b: u8| a.abs_diff(b) < 24;
    raster
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| near(p[0], c.r) && near(p[1], c.g) && near(p[2], c.b) && p[3] > 200)
        .count()
}

#[test]
fn the_object_manager_lists_the_plots_and_a_tick_box_hides_one_from_the_graph() {
    let mut h = start_auto_hide(vec![data_file("om")]);
    let (_, graph) = window_ids(&h);
    h.get_by_label("Object Manager tab").hover();
    settle(&mut h);
    assert_eq!(h.state().panels.flyout(), Some(PanelId::Objects));
    assert!(h.query_by_label("Show layer: Layer1").is_some());
    let plots = project(&h).graph(graph).unwrap().layers[0].plots.clone();
    assert_eq!(plots.len(), 2);
    let second = plots[1].line.color;
    let before = pixels_like(&h, graph, second);
    assert!(before > 20, "the second plot is drawn: {before}");
    let tick = format!("Show plot: {}", project(&h).plot_label(&plots[1]));
    assert!(h.query_by_label(&tick).is_some());
    click(&mut h, &tick);
    assert!(!project(&h).graph(graph).unwrap().layers[0].plots[1].visible);
    assert!(project(&h).graph(graph).unwrap().layers[0].plots[0].visible, "the other plot stays");
    assert_eq!(pixels_like(&h, graph, second), 0, "it is gone from the rendered graph");
    assert_eq!(h.state().doc.undo_label(), Some("Hide plot"));
    // The tick comes back with Undo, and the layer's tick box acts on all of its plots.
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(pixels_like(&h, graph, second) > 20);
    click(&mut h, "Show layer: Layer1");
    assert!(project(&h).graph(graph).unwrap().layers[0].plots.iter().all(|p| !p.visible));
    click(&mut h, "Show layer: Layer1");
    assert!(project(&h).graph(graph).unwrap().layers[0].plots.iter().all(|p| p.visible));
}

/// Where the row of the docked Object Manager with this label is (the same name can also be in
/// a dialog, which lies further left).
fn in_right_panel(h: &Harness<'static, SheafApp>, label: &str) -> egui::Pos2 {
    h.query_all_by_label(label)
        .max_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x))
        .unwrap_or_else(|| panic!("no {label:?}"))
        .rect()
        .center()
}

#[test]
fn the_object_manager_selection_is_the_one_of_plot_details() {
    let mut h = start(vec![data_file("om-sel")]);
    let (_, graph) = window_ids(&h);
    h.state_mut().panels.set_pinned(PanelId::Objects, true);
    settle(&mut h);
    let plots = project(&h).graph(graph).unwrap().layers[0].plots.clone();
    let name = project(&h).plot_label(&plots[1]);
    // A double click on the second plot opens Plot Details with that plot selected.
    let at = in_right_panel(&h, &name);
    double_click_at(&mut h, at);
    let dialog = h.state().graph_props.as_ref().expect("Plot Details is open");
    assert_eq!(dialog.selected_plot(), 1);
    // Choosing the first one in the Object Manager moves the selection of the dialog.
    pause(&mut h);
    let first = project(&h).plot_label(&plots[0]);
    let at = in_right_panel(&h, &first);
    click_at(&mut h, at);
    assert_eq!(h.state().graph_props.as_ref().unwrap().selected_plot(), 0);
    assert_eq!(h.state().panels.objects.selected, Some((0, Some(0))));
}

#[test]
fn the_status_bar_shows_average_sum_and_count_of_the_selected_cells_and_the_fixed_boxes() {
    let mut h = start(vec![data_file("stats")]);
    show_window(&mut h, "Book1 - sample");
    assert!(label_text(&h, "AU : ON") && label_text(&h, "Radian"), "Origin always shows these two");
    let sheet_name = project(&h).workbooks().next().unwrap().sheet().name.clone();
    assert!(label_text(&h, &format!("[Book1]{sheet_name}!")), "the active sheet");
    assert!(!label_containing(&h, "Average="), "nothing selected, no statistics");
    // Column B holds 2.5, 4.5, 6.5, 8.5.
    click_text(&mut h, "B(Y1)");
    assert!(label_text(&h, "Average=5.5 Sum=22 Count=4"));
    // A block of two cells: 4.5 and 6.5.
    let b = column_id(&h, "B");
    let book = project(&h).workbooks().next().unwrap().id;
    let sheet = project(&h).workbook(book).unwrap().sheet().clone();
    let i = sheet.column_index(b).unwrap();
    h.state_mut().books.get_mut(&book).unwrap().select_cells(&sheet, (i, 1), (i, 2));
    settle(&mut h);
    assert!(label_text(&h, "Average=5.5 Sum=11 Count=2"));
    h.state_mut().books.get_mut(&book).unwrap().clear_selection();
    settle(&mut h);
    assert!(!label_containing(&h, "Average="));
}

#[test]
fn the_status_bar_shows_the_hint_of_the_toolbar_button_under_the_mouse() {
    let mut h = start(vec![data_file("hint")]);
    assert!(label_containing(&h, "Imported"));
    let button = h.get_by_label("Standard toolbar: Open").rect().center();
    move_pointer(&mut h, button);
    move_pointer(&mut h, button + egui::vec2(1.0, 0.0));
    assert!(label_text(&h, "Open"), "the hint");
    move_pointer(&mut h, empty_workspace_point());
    move_pointer(&mut h, empty_workspace_point() + egui::vec2(1.0, 0.0));
    assert!(!label_text(&h, "Open"));
    assert!(label_containing(&h, "Imported"), "the last message comes back");
}

#[test]
fn the_messages_log_keeps_what_the_status_bar_said_and_can_be_cleared() {
    let mut h = start(vec![data_file("log")]);
    assert!(h.state().logs.messages.iter().any(|m| m.text.contains("Imported 1 file")));
    // A warning is logged as a warning.
    h.state_mut().warn("Nothing to plot: test");
    assert_eq!(h.state().logs.messages.last().unwrap().level, crate::panels::logs::Level::Warning);
    // Showing the panel lists them, and Clear empties it.
    h.state_mut().panels.open(PanelId::Messages);
    settle(&mut h);
    assert!(label_containing(&h, "Imported 1 file(s)"));
    click(&mut h, "Clear Messages");
    assert!(h.state().logs.messages.is_empty());
    assert!(label_containing(&h, "No messages yet"));
}

#[test]
fn a_message_box_is_logged_once_and_a_tool_report_goes_to_the_results_log() {
    let mut h = start(vec![data_file("results")]);
    h.state_mut().message = Some(("Could not save the project".to_string(), "disk full".to_string()));
    settle(&mut h);
    settle(&mut h);
    let hits = h.state().logs.messages.iter().filter(|m| m.text.contains("disk full")).count();
    assert_eq!(hits, 1, "the box stays on screen for many frames but is logged once");
    h.state_mut().message = None;
    settle(&mut h);
    // Integrate writes a report: the Results Log gets it and its tab appears.
    assert!(!h.state().panels.is_shown(PanelId::Results));
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Analyze { book, columns: Vec::new(), kind: AnalysisKind::Integrate }, &ctx);
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(h.state().logs.results.len(), 1);
    assert!(h.state().logs.results[0].title.contains("Integrate"));
    assert!(h.state().panels.is_shown(PanelId::Results));
    assert!(h.query_by_label("Results Log tab").is_some());
    h.state_mut().panels.open(PanelId::Results);
    settle(&mut h);
    click(&mut h, "Clear Results");
    assert!(h.state().logs.results.is_empty());
}

/// Statistics > Hypothesis Testing > Two-Sample t-Test... opens the window with the two Y columns of the sheet; OK adds a result
/// sheet, and one undo removes it.
#[test]
fn the_statistics_menu_runs_a_t_test_and_undoes_in_one_step() {
    let mut h = start(vec![data_file("ttest")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Statistics", "Hypothesis Testing", "Two-Sample t-Test..."]);
    assert!(h.state().analysis.is_some(), "the t-test window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!(b.sheets.len(), 2);
    assert!(b.sheets[1].name.starts_with("TTest"), "{}", b.sheets[1].name);
    let ws = &b.sheets[1];
    let t_col = ws.columns.iter().find(|c| c.long_name == "t").expect("a t column").numeric().unwrap().to_vec();
    assert_eq!(t_col.len(), 1);
    // up = 2.5 .. 8.5 (mean 5.5), down = 30.5 .. 0.5 (mean 15.5), both with SD 2.582 and 12.91.
    assert!(t_col[0] < -1.0 && t_col[0].is_finite(), "{t_col:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Two-sample t-test"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);
}

/// Statistics > Histogram... adds the counts sheet and a graph with a column plot.
#[test]
fn the_statistics_menu_makes_a_histogram_graph() {
    let mut h = start(vec![data_file("histogram")]);
    show_window(&mut h, "Book1 - sample");
    let graphs_before = project(&h).graphs().count();
    menu_path(&mut h, &["Statistics", "Descriptive Statistics", "Frequency Counts..."]);
    click(&mut h, "OK");
    let p = project(&h);
    assert_eq!(p.graphs().count(), graphs_before + 1);
    let g = p.graphs().last().unwrap();
    assert_eq!(g.layers[0].plots[0].kind, sheaf_core::graph::PlotKind::Column);
    assert!(p.workbooks().next().unwrap().sheets[1].name.starts_with("Histogram"));
}
