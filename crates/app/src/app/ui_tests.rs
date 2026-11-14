//! Tests that drive the application like a user: they click menu entries,
//! column headers and dialog buttons and type into fields, without a real
//! window. They guard the wiring between the interface and the project
//! (which click leads to which change), not the look.

use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT, Queryable};
use sheaf_core::graph::{LegendPosition, PlotKind, ScaleKind};
use sheaf_plot::scene::ObjectKind;

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

/// Chooses a scale type in the axis dialog's list (the list shows the current type, Linear here).
fn pick_scale(h: &mut Harness<'static, SheafApp>, name: &str) {
    h.get_by_value("Linear").click();
    settle(h);
    click(h, name);
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
    // Enter without typing: nothing changes, and the cell below becomes the active cell.
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(book_ui(&h).editing.is_none());
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 2), end: (1, 2) });
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

/// Opens the Recalculate drop-down of the Set Values window (showing `from`) and picks `to`.
fn choose_recalc(h: &mut Harness<'static, SheafApp>, from: &str, to: &str) {
    h.get_by_value(from).click();
    settle(h);
    h.get_all_by_label(to).last().unwrap().click();
    settle(h);
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
    choose_recalc(&mut h, "Manual", "Auto");
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
    choose_recalc(&mut h, "Auto", "Manual");
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
    pick_scale(&mut h, "Log10");
    assert_eq!(scale(&h), ScaleKind::Log10, "the change applies while the window is open");
    click(&mut h, "Line and Ticks");
    click(&mut h, "Ticks");
    assert!(project(&h).graphs().next().unwrap().layers[0].y_axis.opposite_ticks);
    assert_eq!(h.state().doc.undo_label(), Some("Change graph settings"));
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(scale(&h), ScaleKind::Linear, "one undo reverts everything the window did");
    assert!(!project(&h).graphs().next().unwrap().layers[0].y_axis.opposite_ticks);
    assert_eq!(h.state().doc.undo_label(), Some("Import data"));
}

fn open_props(h: &mut Harness<'static, SheafApp>, layer: usize, tab: PropsTab, plot: Option<usize>) {
    let (_, graph) = window_ids(h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphProperties { graph, layer, tab, plot }, &ctx);
    settle(h);
}

#[test]
fn the_axis_dialog_switches_between_the_horizontal_and_the_vertical_axis() {
    let mut h = start(vec![data_file("axis-switch")]);
    open_props(&mut h, 0, PropsTab::XAxis, None);
    assert_eq!(h.state().graph_props.as_ref().unwrap().tab(), PropsTab::XAxis);
    for tab in ["Scale", "Tick Labels", "Title", "Grids", "Line and Ticks", "Special Ticks", "Breaks"] {
        assert!(h.query_by_label(tab).is_some(), "the tab {tab}");
    }
    click(&mut h, "Vertical");
    assert_eq!(h.state().graph_props.as_ref().unwrap().tab(), PropsTab::YAxis);
    pick_scale(&mut h, "Log10");
    assert_eq!(graph(&h).layers[0].y_axis.scale, ScaleKind::Log10);
    assert_eq!(graph(&h).layers[0].x_axis.scale, ScaleKind::Linear, "only the chosen axis changes");
    click(&mut h, "Horizontal");
    assert_eq!(h.state().graph_props.as_ref().unwrap().tab(), PropsTab::XAxis);
    pick_scale(&mut h, "Log10");
    assert_eq!(graph(&h).layers[0].x_axis.scale, ScaleKind::Log10);
    click(&mut h, "OK");
    assert!(h.state().graph_props.is_none());
    assert_eq!(h.state().doc.undo_label(), Some("Change graph settings"));
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(graph(&h).layers[0].y_axis.scale, ScaleKind::Linear, "both axes were one undo step");
    assert_eq!(graph(&h).layers[0].x_axis.scale, ScaleKind::Linear);
}

#[test]
fn a_change_on_the_scale_tab_fixes_the_limit() {
    let mut h = start(vec![data_file("axis-scale")]);
    open_props(&mut h, 0, PropsTab::YAxis, None);
    assert!(graph(&h).layers[0].y_axis.min.is_none());
    // The first "Auto" box is the one of From.
    h.query_all_by_label("Auto").next().expect("From has an Auto box").click();
    settle(&mut h);
    // The automatic lower limit (Origin's 8 % margin below the data) becomes the fixed one.
    assert_eq!(graph(&h).layers[0].y_axis.min, Some(-2.0));
    assert!(graph(&h).layers[0].y_axis.max.is_none());
    click(&mut h, "Tick Labels");
    click(&mut h, "Hide Last Label");
}

#[test]
fn cancel_takes_the_graph_back_and_leaves_no_undo_entry() {
    let mut h = start(vec![data_file("axis-cancel")]);
    let before = graph(&h).clone();
    open_props(&mut h, 0, PropsTab::YAxis, None);
    pick_scale(&mut h, "Log10");
    assert_eq!(graph(&h).layers[0].y_axis.scale, ScaleKind::Log10);
    click(&mut h, "Cancel");
    assert!(h.state().graph_props.is_none());
    assert_eq!(*graph(&h), before);
    assert_eq!(h.state().doc.undo_label(), Some("Import data"), "no undo entry is left");
}

#[test]
fn apply_moves_the_starting_point_of_cancel() {
    let mut h = start(vec![data_file("axis-apply")]);
    open_props(&mut h, 0, PropsTab::YAxis, None);
    pick_scale(&mut h, "Log10");
    click(&mut h, "Apply");
    assert!(h.state().graph_props.is_some(), "Apply keeps the window open");
    h.get_by_value("Log10").click();
    settle(&mut h);
    click(&mut h, "Linear");
    assert_eq!(graph(&h).layers[0].y_axis.scale, ScaleKind::Linear);
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[0].y_axis.scale, ScaleKind::Log10, "Cancel goes back to the Apply");
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(graph(&h).layers[0].y_axis.scale, ScaleKind::Linear);
}

#[test]
fn the_plot_details_tree_chooses_a_plot_of_the_second_layer() {
    let mut h = start(vec![data_file("details-tree")]);
    let (_, g) = window_ids(&h);
    h.state_mut().doc.apply("Double Y", |p| {
        p.add_right_y_layer(g, 0).unwrap();
        let graph = p.graph_mut(g).unwrap();
        let moved = graph.layers[0].plots.remove(1);
        graph.layers[1].plots.push(moved);
    });
    let name = project(&h).plot_label(&graph(&h).layers[1].plots[0]);
    open_props(&mut h, 0, PropsTab::Plots, None);
    assert_eq!(h.state().graph_props.as_ref().unwrap().layer, 0);
    for entry in ["Layer 1", "Layer 2"] {
        assert!(h.query_by_label(entry).is_some(), "{entry} is in the tree");
    }
    h.get(By::new().role(egui::accesskit::Role::Button).label(name.as_str())).click();
    settle(&mut h);
    let d = h.state().graph_props.as_ref().unwrap();
    assert_eq!((d.layer, d.selected_plot()), (1, 0), "the plot of layer 2 is chosen");
    assert_eq!(d.tab(), PropsTab::Plots);
    // Its Group tab removes it from layer 2 only; Cancel brings it back.
    click(&mut h, "Group");
    click(&mut h, "Remove plot");
    assert!(graph(&h).layers[1].plots.is_empty());
    assert_eq!(graph(&h).layers[0].plots.len(), 1);
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[1].plots.len(), 1, "Cancel restored the plot");
}

#[test]
fn plot_details_opened_for_the_page_and_the_legend_show_those_tabs() {
    let mut h = start(vec![data_file("details-page")]);
    open_props(&mut h, 0, PropsTab::Page, None);
    assert_eq!(h.state().graph_props.as_ref().unwrap().tab(), PropsTab::Page);
    click(&mut h, "Text");
    click(&mut h, "+10 %");
    open_props(&mut h, 0, PropsTab::Legend, None);
    assert_eq!(h.state().graph_props.as_ref().unwrap().tab(), PropsTab::Legend);
    let frame = graph(&h).layers[0].legend.frame;
    click(&mut h, "Draw a frame");
    assert_ne!(graph(&h).layers[0].legend.frame, frame);
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
    click(&mut h, "Import Mode");
    click(&mut h, "Make a graph of each file");
    click(&mut h, "OK");
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
    click(&mut h, "Done");
    assert!(h.state().fit.is_none(), "Done closes the window");
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
    // The controls are on their pages of the left list and have Origin's defaults.
    click(&mut h, "Advanced");
    h.get_by_label("Maximum iterations");
    h.get_by_label("Confidence level");
    click(&mut h, "Fitted Curves");
    h.get_by_label("Band confidence level");
    click(&mut h, "Confidence and prediction");
    click(&mut h, "Done");
    assert!(h.state().fit.is_none(), "Done closes the window");
    let b = project(&h).workbooks().next().unwrap();
    let curve = b.sheets.iter().find(|s| s.name == "FitCurve1").expect("the curve sheet");
    let names: Vec<&str> = curve.columns.iter().map(|c| c.long_name.as_str()).collect();
    assert!(names.iter().any(|n| n.starts_with("LCL")), "{names:?}");
    assert!(names.iter().any(|n| n.starts_with("UPL")), "{names:?}");
    assert_eq!(curve.columns.len(), 2 + 4, "the curve and four band columns");
}

/// A CSV with X, two decaying columns of one time constant and a weight column.
fn decays_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("decays.csv");
    let mut csv = String::from("t,a,b,w\n");
    for i in 0..30 {
        let x = i as f64 * 0.3;
        csv += &format!(
            "{x},{},{},{}\n",
            1.0 + 4.0 * (-x / 2.0).exp() + 0.01 * (7.0 * i as f64).sin(),
            0.5 + 2.0 * (-x / 2.0).exp() + 0.01 * (5.0 * i as f64).sin(),
            1.0 + 0.5 * (3.0 * i as f64).sin().abs()
        );
    }
    std::fs::write(&path, csv).unwrap();
    path
}

#[test]
fn the_fit_window_offers_direct_weighting_and_the_scale_errors_switch() {
    let mut h = start(vec![decays_file("fitdirect")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::Linear }, &ctx);
    settle(&mut h);
    use sheaf_core::fitting::FitWeighting;
    let request = |h: &Harness<'static, SheafApp>| h.state().fit.as_ref().unwrap().request_for_test().unwrap();
    assert_eq!((request(&h).weighting, request(&h).scale_errors), (FitWeighting::None, true));
    click(&mut h, "Data Selection");
    h.get_by_label("Direct");
    click(&mut h, "Direct");
    h.get_by_label("Weight column");
    let r = request(&h);
    assert_eq!(r.weighting, FitWeighting::Direct);
    assert!(r.y_error.is_some(), "a weight column is preselected");
    click(&mut h, "Advanced");
    click(&mut h, "Scale Error with sqrt(Reduced Chi-Sqr)");
    assert!(!request(&h).scale_errors, "the box was ticked and is now off");
    // The weight column w is chosen and the fit runs with the weights from it.
    let w = project(&h).workbooks().next().unwrap().sheets[0].column_by_name("w").unwrap().id;
    h.state_mut().fit.as_mut().unwrap().weight_column_for_test(w);
    assert_eq!(request(&h).y_error, Some(w));
    click(&mut h, "Done");
    assert!(h.state().fit.is_none(), "Done closes the window");
    let b = project(&h).workbooks().next().unwrap();
    assert!(b.sheets.iter().any(|s| s.name == "FitLinear1"), "the fit was written");
}

#[test]
fn the_fit_window_fits_several_columns_independently_or_as_a_global_fit() {
    let mut h = start(vec![decays_file("fitglobal")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::BuiltIn }, &ctx);
    settle(&mut h);
    h.state_mut().fit.as_mut().unwrap().choose_for_test("ExpDec1");
    let sheet = &project(&h).workbooks().next().unwrap().sheets[0];
    let b = sheet.column_by_name("b").unwrap().id;
    settle(&mut h);
    click(&mut h, "Data Selection");
    h.get_by_label("Also fit");
    h.state_mut().fit.as_mut().unwrap().also_fit_for_test(&[b], None);
    settle(&mut h);
    h.get_by_label("Multiple datasets");
    h.get_by_label("Global Fit");
    // Global fit: the Parameters tab gets a Shared column.
    h.state_mut().fit.as_mut().unwrap().also_fit_for_test(&[b], Some(&["t1"]));
    settle(&mut h);
    click(&mut h, "Parameters");
    h.get_by_label("Shared");
    click(&mut h, "Done");
    // Done with no iterations of its own writes the starting values; the table is the guess.
    assert!(h.state().fit.is_none());
    let names: Vec<String> = project(&h).workbooks().next().unwrap().sheets.iter().map(|s| s.name.clone()).collect();
    assert!(names.contains(&"FitNLGlobal1".to_string()), "{names:?}");
    assert!(names.contains(&"FitNL1".to_string()) && names.contains(&"FitNL2".to_string()), "{names:?}");
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1, "one undo removes all of it");

    // Independent fits: one set of result sheets per column, no combined table.
    h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::BuiltIn }, &ctx);
    settle(&mut h);
    h.state_mut().fit.as_mut().unwrap().choose_for_test("ExpDec1");
    h.state_mut().fit.as_mut().unwrap().also_fit_for_test(&[b], None);
    click(&mut h, "Fit");
    let names: Vec<String> = project(&h).workbooks().next().unwrap().sheets.iter().map(|s| s.name.clone()).collect();
    assert!(!names.iter().any(|n| n.starts_with("FitNLGlobal")), "{names:?}");
    assert!(names.contains(&"FitNL2".to_string()), "{names:?}");
}

fn sheet_names(h: &Harness<'static, SheafApp>) -> Vec<String> {
    project(h).workbooks().next().unwrap().sheets.iter().map(|s| s.name.clone()).collect()
}

#[test]
fn linear_fit_with_x_error_asks_for_an_x_error_column_then_fits() {
    let mut h = start(vec![decays_file("fitxerr")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::LinearXError }, &ctx);
    settle(&mut h);
    h.get_by_label("NLFit (Straight line with X error)");
    click(&mut h, "Done");
    assert!(h.state().fit.is_some(), "without an X-error column the window stays open");
    assert!(!h.query_all_by_label_contains("X-error").collect::<Vec<_>>().is_empty());
    // An X-error column right after the Y column is what the fit needs.
    h.state_mut().doc.apply("Add X error", |p| {
        let id = p.new_column_id();
        let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
        let at = ws.column_by_name("B").map(|c| ws.column_index(c.id).unwrap()).unwrap() + 1;
        let name = ws.next_short_name();
        ws.columns
            .insert(at, sheaf_core::Column::new(id, name, Designation::XError, ColumnData::numeric(vec![0.05; 30])));
    });
    click(&mut h, "Done");
    assert!(h.state().fit.is_none(), "{:?}", h.state().fit.as_ref().map(|d| d.problem_for_test()));
    assert!(sheet_names(&h).contains(&"FitLinear1".to_string()), "{:?}", sheet_names(&h));
}

#[test]
fn multiple_regression_compare_models_and_rank_models_are_in_the_fitting_menu() {
    let mut h = start(vec![decays_file("fitmore")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    let sheet = &project(&h).workbooks().next().unwrap().sheets[0];
    let (t, b, w) = (
        sheet.column_by_name("t").unwrap().id,
        sheet.column_by_name("b").unwrap().id,
        sheet.column_by_name("w").unwrap().id,
    );
    // Multiple Linear Regression: pick the variables, OK writes FitMR1 and its statistics.
    h.state_mut().apply(Action::FitMore { book, column: None, kind: FitMoreKind::Multiple }, &ctx);
    settle(&mut h);
    h.get_by_label("Multiple Linear Regression");
    h.get_by_label("Independent variables (X)");
    h.state_mut().fit_more.as_mut().unwrap().choose_x_for_test(&[t, b, w]);
    click(&mut h, "OK");
    assert!(h.state().fit_more.is_none());
    let names = sheet_names(&h);
    assert!(names.contains(&"FitMR1".to_string()) && names.contains(&"FitMR1Stats".to_string()), "{names:?}");
    // Compare Models.
    h.state_mut().apply(Action::FitMore { book, column: None, kind: FitMoreKind::Compare }, &ctx);
    settle(&mut h);
    h.get_by_label("Model 1");
    h.state_mut().fit_more.as_mut().unwrap().choose_models_for_test("ExpDec1", "Linear");
    click(&mut h, "OK");
    assert!(h.state().fit_more.is_none());
    assert!(sheet_names(&h).contains(&"FitCompare1".to_string()), "{:?}", sheet_names(&h));
    // Rank Models; with fewer than two functions the window stays open and says why.
    h.state_mut().apply(Action::FitMore { book, column: None, kind: FitMoreKind::Rank }, &ctx);
    settle(&mut h);
    h.state_mut().fit_more.as_mut().unwrap().choose_functions_for_test(&["ExpDec1"]);
    click(&mut h, "OK");
    assert!(h.state().fit_more.is_some());
    assert!(h.state().fit_more.as_ref().unwrap().problem_for_test().unwrap().contains("at least two"));
    h.state_mut().fit_more.as_mut().unwrap().choose_functions_for_test(&["ExpDec1", "ExpDec2", "Gauss"]);
    click(&mut h, "OK");
    assert!(h.state().fit_more.is_none());
    assert!(sheet_names(&h).contains(&"FitRank1".to_string()), "{:?}", sheet_names(&h));
    // Each step is one undo.
    let n = sheet_names(&h).len();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(sheet_names(&h).len(), n - 1);
}

#[test]
fn the_quick_fits_open_nlfit_with_a_preset_function_and_a_locked_category() {
    let mut h = start(vec![decays_file("fitquick")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    for (kind, title, default, offered) in [
        (
            FitKind::Exponential,
            "Fit Exponential (ExpDec1)",
            "ExpDec1",
            vec!["ExpDec1", "ExpDec2", "ExpDec3", "ExpGrow1"],
        ),
        (
            FitKind::Sigmoidal,
            "Fit Sigmoidal (Boltzmann)",
            "Boltzmann",
            vec!["Boltzmann", "Logistic", "Hill", "DoseResp"],
        ),
        (FitKind::SinglePeak, "Single Peak Fit (Gauss)", "Gauss", vec!["Gauss", "Lorentz"]),
    ] {
        h.state_mut().apply(Action::Fit { book, column: None, kind }, &ctx);
        settle(&mut h);
        h.get_by_label(title);
        let (function, list, quick) = h.state().fit.as_ref().unwrap().function_for_test();
        assert!(quick);
        assert_eq!(function, default);
        let mut list = list;
        list.sort();
        let mut want: Vec<String> = offered.iter().map(|s| s.to_string()).collect();
        want.sort();
        assert_eq!(list, want, "{title}");
        click(&mut h, "Cancel");
        assert!(h.state().fit.is_none());
    }
    // The menu items are no longer greyed.
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::Sigmoidal }, &ctx);
    settle(&mut h);
    click(&mut h, "Done");
    let names: Vec<String> = project(&h).workbooks().next().unwrap().sheets.iter().map(|s| s.name.clone()).collect();
    assert!(names.contains(&"FitNL1".to_string()), "{names:?}");
}

#[test]
fn the_fit_window_has_origins_tabs_pages_and_lower_panel() {
    let mut h = start(vec![data_file("fitpages")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::BuiltIn }, &ctx);
    settle(&mut h);
    // The title names the function; the tabs run along the top, the pages stand at the left of
    // Settings, and the lower panel has its own tabs.
    h.get_by_label("NLFit (Gauss)");
    for tab in ["Settings", "Code", "Parameters", "Bounds"] {
        h.get_by_label(tab);
    }
    for page in ["Function Selection", "Data Selection", "Fitted Curves", "Find X/Y", "Advanced", "Output"] {
        h.get_by_label(page);
    }
    for tab in ["Fit Curve", "Residual", "Formula", "Messages"] {
        h.get_by_label(tab);
    }
    // The Function Selection page holds the Category drop-down, which offers the closed-form fits too.
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Peak Functions")).click();
    settle(&mut h);
    click(&mut h, "Polynomial Fit");
    h.get_by_label("NLFit (Polynomial)");
    h.get_by_label("Order");
    // The Parameters tab lists the parameters of the polynomial; Bounds is greyed for it.
    click(&mut h, "Parameters");
    h.get_by_label("Intercept");
    click(&mut h, "Bounds");
    assert!(h.query_by_label_contains("no bounds").is_some());
    // A nonlinear function brings its parameters to the Bounds tab.
    click(&mut h, "Settings");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Polynomial Fit")).click();
    settle(&mut h);
    click(&mut h, "Peak Functions");
    click(&mut h, "Bounds");
    h.get_by_label("Lower Bounds");
    // Data Selection holds the weights, Fitted Curves the curve options; Code shows the formula.
    click(&mut h, "Settings");
    click(&mut h, "Data Selection");
    h.get_by_label("Weights");
    click(&mut h, "Fitted Curves");
    h.get_by_label("Number of Points");
    h.get_by_label("Use Input Data Range");
    h.get_by_label("Update Legend on Source Graph");
    click(&mut h, "Find X/Y");
    click(&mut h, "Output");
    click(&mut h, "Code");
    assert!(h.query_all_by_label_contains("y = ").count() >= 1);
    // (The data has four points: too few for a peak function, so the rest uses the polynomial.)
    click(&mut h, "Settings");
    click(&mut h, "Function Selection");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Peak Functions")).click();
    settle(&mut h);
    click(&mut h, "Polynomial Fit");
    // The lower panel draws the data and the curve; the Formula tab shows the formula; the arrow folds it.
    let (data, curve, error) = h.state().fit.as_ref().unwrap().preview_for_test().expect("a preview");
    assert!(data >= 4 && curve > 100, "{data} {curve} {error:?}");
    click(&mut h, "Residual");
    assert!(h.query_by_label_contains("Residuals at the values").is_some());
    click(&mut h, "Formula");
    assert!(h.query_by_label_contains("Parameters: ").is_some());
    click(&mut h, "Hide lower panel");
    assert!(h.query_by_label("Fit Curve").is_none(), "the lower panel is folded away");
    click(&mut h, "Show lower panel");
    h.get_by_label("Fit Curve");
    // Fit until converged tries the fit and reports; nothing is written until Fit or Done.
    let before = project(&h).workbooks().next().unwrap().sheets.len();
    click(&mut h, "Fit until converged");
    assert!(h.state().fit.is_some(), "the trial leaves the window open");
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), before, "the trial wrote nothing");
    assert!(!h.state().fit.as_ref().unwrap().log_for_test().is_empty(), "the Messages tab has the report");
    click(&mut h, "Cancel");
    assert!(h.state().fit.is_none());
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), before, "Cancel writes nothing");
}

#[test]
fn fit_finishes_the_iterations_and_writes_done_writes_the_table_as_it_is() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-fitbuttons-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("decay.csv");
    let mut csv = String::from(
        "t,y
",
    );
    for i in 0..20 {
        let x = i as f64 * 0.5;
        csv += &format!(
            "{x},{}
",
            1.0 + 4.0 * (-x / 3.0).exp() + 0.02 * (37.0 * (i + 1) as f64).sin()
        );
    }
    std::fs::write(&path, csv).unwrap();
    let open = |h: &mut Harness<'static, SheafApp>| {
        let book = project(h).workbooks().next().unwrap().id;
        let ctx = h.ctx.clone();
        h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::BuiltIn }, &ctx);
        settle(h);
        h.state_mut().fit.as_mut().unwrap().choose_for_test("ExpDec1");
        settle(h);
    };
    let sheet_count = |h: &Harness<'static, SheafApp>| project(h).workbooks().next().unwrap().sheets.len();

    // Fit after one iteration finishes the rest, writes the results and closes, in one undo step.
    let mut h = start(vec![path.clone()]);
    open(&mut h);
    let before = sheet_count(&h);
    click(&mut h, "1 Iteration");
    click(&mut h, "Fit");
    assert!(h.state().fit.is_none(), "Fit closes the window");
    assert_eq!(sheet_count(&h), before + 3, "parameters, statistics and curve sheets");
    let b = project(&h).workbooks().next().unwrap();
    let params = b.sheets.iter().find(|s| s.column_by_name("Value").is_some()).unwrap();
    let t = params.column_by_name("Value").unwrap().numeric().unwrap()[2];
    assert!((t - 3.0).abs() < 0.3, "t1 = {t}: the fit went on to convergence");
    assert_eq!(h.state().doc.undo_label(), Some("Fit"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(sheet_count(&h), before);

    // Done after one iteration writes the values of the table without iterating further.
    let mut h = start(vec![path]);
    open(&mut h);
    click(&mut h, "1 Iteration");
    let one = h.state().fit.as_ref().unwrap().rows_for_test();
    click(&mut h, "Done");
    assert!(h.state().fit.is_none(), "Done closes the window");
    let b = project(&h).workbooks().next().unwrap();
    let params = b.sheets.iter().find(|s| s.column_by_name("Value").is_some()).unwrap();
    let values = params.column_by_name("Value").unwrap().numeric().unwrap().to_vec();
    for (v, r) in values.iter().zip(&one) {
        assert!((v - r.2).abs() <= 1e-9 * r.2.abs().max(1.0), "{v} vs {}", r.2);
    }
}

#[test]
fn the_fit_window_steps_one_iteration_runs_to_convergence_and_reverts() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-fitsteps-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("decay.csv");
    let mut csv = String::from(
        "t,y
",
    );
    for i in 0..20 {
        let x = i as f64 * 0.5;
        csv += &format!(
            "{x},{}
",
            1.0 + 4.0 * (-x / 3.0).exp() + 0.02 * (37.0 * (i + 1) as f64).sin()
        );
    }
    std::fs::write(&path, csv).unwrap();
    let mut h = start(vec![path]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Fit { book, column: None, kind: FitKind::BuiltIn }, &ctx);
    settle(&mut h);
    h.state_mut().fit.as_mut().unwrap().choose_for_test("ExpDec1");
    settle(&mut h);
    let rows = |h: &Harness<'static, SheafApp>| h.state().fit.as_ref().unwrap().rows_for_test();
    let start_rows = rows(&h);
    assert_eq!(start_rows.len(), 3);
    assert!(start_rows.iter().all(|r| !r.1), "no starting value typed yet");
    let before = project(&h).workbooks().next().unwrap().sheets.len();

    // One iteration moves the parameters by one step and says so.
    click(&mut h, "1 Iteration");
    let one = rows(&h);
    assert!(one.iter().all(|r| r.1), "the table now holds the values of the iteration");
    assert_ne!(one.iter().map(|r| r.2).collect::<Vec<_>>(), start_rows.iter().map(|r| r.2).collect::<Vec<_>>());
    let msg = h.state().fit.as_ref().unwrap().message_for_test();
    assert!(msg.contains("Iteration 1") || msg.contains("converged"), "{msg}");

    // Fit until converged runs from there until it converges, and keeps the window.
    click(&mut h, "Fit until converged");
    let done = rows(&h);
    let msg = h.state().fit.as_ref().unwrap().message_for_test();
    assert!(msg.contains("converged") && !msg.contains("did not"), "{msg}");
    let t = done.iter().find(|r| r.0 == "t1").unwrap().2;
    assert!((t - 3.0).abs() < 0.3, "t1 = {t}");

    // Revert goes back one press at a time.
    click(&mut h, "Revert");
    assert_eq!(rows(&h), one);
    click(&mut h, "Revert");
    assert_eq!(rows(&h), start_rows);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), before, "nothing was written");
    click(&mut h, "Cancel");
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
    menu_path(&mut h, &["File", "Export Graphs..."]);
    assert!(h.state().export.is_some(), "the export window is open");
    assert!(h.query_by_label("Apply").is_some());
    click(&mut h, "Cancel");
    assert!(h.state().export.is_none());
}

#[test]
fn export_graphs_dialog_has_origins_groups_and_buttons() {
    let mut h = start(vec![data_file("export-layout")]);
    show_window(&mut h, "Graph1 - sample");
    ctrl(&mut h, egui::Key::G);
    assert!(h.state().export.is_some());
    for label in [
        "Export Graphs",
        "Image Type",
        "Select Graph(s)",
        "File Name(s)",
        "Enable Substitution",
        "Path",
        "Overwrite Existing",
        "Graph Theme",
        "Export Settings",
        "Image Size",
        "Image Settings",
        "Auto Preview",
        "Preview",
        "OK",
        "Cancel",
        "Apply",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    click(&mut h, "Cancel");
    assert!(h.state().export.is_none());
}

#[test]
fn export_graphs_file_name_substitution_and_apply_enabling() {
    let mut h = start(vec![data_file("export-apply")]);
    show_window(&mut h, "Graph1 - sample");
    menu_path(&mut h, &["File", "Export Graphs..."]);
    let graph_id = graph(&h).id;
    // The default name is the long name of the graph.
    let name = h.state().export.as_ref().unwrap().name_for(&h.state().doc, graph_id);
    assert_eq!(name, "sample");
    assert!(h.query_by_label(&format!("File will be saved as {name}.png")).is_some());
    assert!(!h.get_by_label("Apply").accesskit_node().is_disabled());
    h.state_mut().export.as_mut().unwrap().set_file_name("fig_<short name>");
    settle(&mut h);
    assert!(h.query_by_label("File will be saved as fig_Graph1.png").is_some());
    h.state_mut().export.as_mut().unwrap().set_file_name("");
    settle(&mut h);
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled(), "no file name, no Apply");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
}

#[test]
fn export_graphs_apply_writes_the_file_and_asks_before_replacing_it() {
    let mut h = start(vec![data_file("export-write")]);
    show_window(&mut h, "Graph1 - sample");
    menu_path(&mut h, &["File", "Export Graphs..."]);
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-export-out-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    h.state_mut().export.as_mut().unwrap().set_path(&dir);
    settle(&mut h);
    click(&mut h, "Apply");
    let file = dir.join("sample.png");
    let bytes = std::fs::read(&file).expect("Apply wrote the picture");
    assert_eq!(&bytes[1..4], b"PNG");
    assert!(h.state().export.is_some(), "Apply keeps the window open");
    std::fs::write(&file, b"old").unwrap();
    click(&mut h, "Apply");
    assert_eq!(std::fs::read(&file).unwrap(), b"old", "the existing file waits for an answer");
    assert!(h.query_by_label("Replace").is_some());
    click(&mut h, "Replace");
    assert_eq!(&std::fs::read(&file).unwrap()[1..4], b"PNG");
    // OK writes and closes.
    std::fs::remove_file(&file).unwrap();
    click(&mut h, "OK");
    assert!(file.exists());
    assert!(h.state().export.is_none(), "OK closes the window");
    let _ = std::fs::remove_dir_all(&dir);
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
    click(&mut h, "OK");
    let book = project(&h).workbooks().next().unwrap();
    let col = book.sheet().column_by_name("A").unwrap();
    assert!(matches!(col.format, NumberFormat::DateTime(..)));
    assert_eq!(col.cell_text(0), "2026-11-11 13:45:10");
    // Keep as text: the same file arrives as text cells.
    let mut h = start(Vec::new());
    h.input_mut().dropped_files.push(egui::DroppedFile { path: Some(date_file("dates-text")), ..Default::default() });
    settle(&mut h);
    click(&mut h, "Miscellaneous");
    click(&mut h, "Keep as text");
    click(&mut h, "OK");
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
    click(&mut h, "OK");
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
    // The delimiter (Import Options) and the thousands separator (Miscellaneous) are found by
    // the window itself, which says so.
    assert!(h.query_by_label_contains("found: tab").is_some());
    click(&mut h, "Miscellaneous");
    assert!(h.query_by_label_contains("found: comma").is_some());
    // Put the data into the active sheet as new columns (the Import Mode node of the tree).
    click(&mut h, "Import Mode");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("New Books")).click();
    settle(&mut h);
    click(&mut h, "New Columns");
    click(&mut h, "OK");
    assert!(h.state().import.is_none());
    assert_eq!(project(&h).workbooks().count(), 1, "no new book was made");
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().columns.len(), 5);
    assert_eq!(column(&h, "D"), [1000.5, 2000.5, 3000.5, 4000.5]);
    assert_eq!(column(&h, "E"), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5], "the first file's columns are untouched");
}

#[test]
fn the_import_tree_nodes_reach_the_options_and_ok_applies_them() {
    let mut h = start(Vec::new());
    h.input_mut().dropped_files.push(egui::DroppedFile { path: Some(data_file("tree")), ..Default::default() });
    settle(&mut h);
    // The tree has Origin's groups; the first node holds the delimiter.
    for node in [
        "Import Options",
        "Import Mode",
        "Header Lines",
        "Column Designation",
        "Partial Import",
        "Miscellaneous",
        "File Info and Parameters",
    ] {
        h.get_by_label(node);
    }
    h.get_by_label("Delimiter");
    click(&mut h, "Header Lines");
    h.get_by_label("Lines before the data");
    h.get_by_label("Line with the units");
    click(&mut h, "Partial Import");
    h.get_by_label("Skip data rows");
    click(&mut h, "Miscellaneous");
    h.get_by_label("Thousands separator");
    click(&mut h, "File Info and Parameters");
    click(&mut h, "Column Designation");
    click(&mut h, "The first column is X");
    // Windows order: OK, then Cancel.
    let ok = h.get_by_label("OK").rect().center().x;
    let cancel = h.get_by_label("Cancel").rect().center().x;
    assert!(ok < cancel, "OK is left of Cancel");
    click(&mut h, "OK");
    assert!(h.state().import.is_none());
    let book = project(&h).workbooks().next().unwrap();
    assert_ne!(book.sheet().columns[0].designation, Designation::X, "the option reached the importer");
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
/// Gives the first layer a bar overlap, which the .opj writer cannot store (so the save needs the
/// companion).
fn add_lossy(h: &mut Harness<'static, SheafApp>) {
    set_overlap(h, 40.0);
}

fn set_overlap(h: &mut Harness<'static, SheafApp>, pct: f64) {
    h.state_mut().doc.apply("Change bar overlap", |p| {
        if let Some(g) = p.windows.iter_mut().find_map(|w| if let Window::Graph(g) = w { Some(g) } else { None }) {
            g.layers[0].bars.overlap_pct = pct;
        }
    });
    settle(h);
}

#[test]
fn ctrl_s_saves_an_opj_with_a_companion_for_what_it_cannot_hold_and_reopening_uses_it() {
    let dir = save_dir("formula");
    let opj = dir.join("lab.opj");
    let mut h = start(vec![data_file("save-formula")]);
    add_lossy(&mut h);
    h.state_mut().path = Some(opj.clone());
    ctrl_s(&mut h);
    let companion = dir.join("lab.opj.sheaf");
    assert!(opj.exists(), "the .opj is written");
    assert!(companion.exists(), "the companion is written");
    assert!(!dir.join("lab.sheaf").exists(), "a user's own lab.sheaf name is not used");
    assert!(!h.state().doc.dirty);
    assert!(h.state().status.contains("lab.opj.sheaf"), "{}", h.state().status);
    let (title, text) = h.state().message.clone().expect("a message says what only the companion holds");
    assert!(title.contains("companion") && text.contains("overlap"), "{title}: {text}");
    assert_eq!(h.state().path.as_deref(), Some(opj.as_path()));

    let h2 = start(vec![opj.clone()]);
    assert_eq!(graph(&h2).layers[0].bars.overlap_pct, 40.0, "the overlap is back from the companion");
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
    add_lossy(&mut h);
    h.state_mut().path = Some(opj.clone());
    ctrl_s(&mut h);
    assert!(dir.join("lab.opj.sheaf").exists());
    // No overlap: now the .opj holds everything.
    h.state_mut().message = None;
    set_overlap(&mut h, 0.0);
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
        &["Analysis", "Mathematics", "Average Multiple Curves..."],
        &["Analysis", "Mathematics", "Intersect..."],
        &["Analysis", "Mathematics", "Polygon Area..."],
        &["Analysis", "Data Manipulation", "Subtract Reference Data..."],
        &["Analysis", "Fitting", "Linear Fit..."],
        &["Analysis", "Fitting", "Polynomial Fit..."],
        &["Analysis", "Fitting", "Fit Linear with X Error..."],
        &["Analysis", "Fitting", "Multiple Linear Regression..."],
        &["Analysis", "Fitting", "Compare Models..."],
        &["Analysis", "Fitting", "Rank Models..."],
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
        &["File", "Export Graphs..."],
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
    assert!(entry_enabled(&h, "Stacked Column"), "stacked columns exist now");
    for live in ["Stacked Bar", "100% Stacked Bar", "Floating Bar"] {
        assert!(entry_enabled(&h, live), "{live} exists now");
    }
    assert!(entry_enabled(&h, "3D Color Pie") && entry_enabled(&h, "2D B&W Pie"), "the pies exist now");
    assert!(h.query_by_label("2D Color Pie").is_none(), "Origin has no 2D Color Pie");
    assert!(!entry_enabled(&h, "Column + Label"), "an item Sheaf lacks is greyed");
    menu_entry(&h, "Column + Label").hover();
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
    // New columns are Text & Numeric (Origin's default), so the words stay as text cells next to numbers.
    let shown = |c: usize| (0..3).map(|r| s.columns[c].cell_text(r)).collect::<Vec<_>>();
    assert_eq!(shown(0), ["alpha", "beta", "gamma"]);
    assert_eq!(shown(1), ["1.5", "2", "x"], "the word stays in a column of numbers");
    assert_eq!(s.columns[1].numeric().unwrap()[..2], [1.5, 2.0]);
    assert!(s.columns[1].numeric().unwrap()[2].is_nan(), "numeric views see the text as missing");
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
    choose_recalc(&mut h, "Manual", "Auto");
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
    for have in [
        "Plot",
        "Cut",
        "Copy",
        "Paste",
        "Insert",
        "Delete",
        "Clear",
        "Set As Categorical",
        "Set As",
        "Properties...",
        "Normalize...",
    ] {
        assert!(entry_enabled(&h, have), "{have} should be enabled");
    }
    for lack in ["Copy Columns to...", "Frequency Count..."] {
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
    // Stacked Area is a 2D type Sheaf lacks: greyed, a click changes nothing.
    assert!(bar_button!(h, "2D Graphs", "Stacked Area").accesskit_node().is_disabled());
    bar_button!(h, "2D Graphs", "Stacked Area").click();
    settle(&mut h);
    assert_eq!(project(&h).windows.len(), windows);
    assert!(h.state().doc.undo_label().is_some_and(|l| !l.contains("Stacked Area")));
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
    // Origin's lines are 0.5 pt thin; thicken them so that the pixel count is steady.
    h.state_mut().doc.apply("Line width", |p| {
        for plot in &mut p.graph_mut(graph).unwrap().layers[0].plots {
            plot.line.width_pt = 3.0;
        }
    });
    settle(&mut h);
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
    assert!(label_text(&h, &format!("1: [Book1]{sheet_name}!")), "the active sheet");
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

// ----- graph window: picking objects with the mouse (gui-look.md section 4) -----------------------

fn graph_ui<'a>(h: &'a Harness<'static, SheafApp>) -> &'a crate::graph_view::GraphUi {
    h.state().graphs.values().next().expect("a graph window was drawn")
}

fn on(h: &Harness<'static, SheafApp>, layer: usize, kind: ObjectKind) -> egui::Pos2 {
    graph_ui(h).point_on(layer, kind).unwrap_or_else(|| panic!("{kind:?} is not drawn"))
}

fn right_click_at(h: &mut Harness<'static, SheafApp>, at: egui::Pos2) {
    pause(h);
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
    settle(h);
}

fn graph<'a>(h: &'a Harness<'static, SheafApp>) -> &'a sheaf_core::Graph {
    project(h).graphs().next().unwrap()
}

#[test]
fn double_clicks_on_the_graph_open_the_settings_of_what_was_clicked() {
    let mut h = start(vec![data_file("gdbl")]);
    for (kind, tab, plot) in [
        (ObjectKind::YAxis, PropsTab::YAxis, None),
        (ObjectKind::XAxis, PropsTab::XAxis, None),
        (ObjectKind::Legend, PropsTab::Legend, None),
        (ObjectKind::Plot(1), PropsTab::Plots, Some(1)),
    ] {
        h.state_mut().graph_props = None;
        let at = on(&h, 0, kind);
        double_click_at(&mut h, at);
        let d = h.state().graph_props.as_ref().unwrap_or_else(|| panic!("{kind:?} opened nothing"));
        assert_eq!((d.tab(), d.layer), (tab, 0), "{kind:?}");
        if let Some(p) = plot {
            assert_eq!(d.selected_plot(), p, "the clicked plot is the chosen one");
        }
    }
}

#[test]
fn a_click_selects_a_plot_and_its_menu_changes_the_plot_type_in_one_undo_step() {
    let mut h = start(vec![data_file("gplot")]);
    let at = on(&h, 0, ObjectKind::Plot(0));
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Plot(0))));
    right_click_at(&mut h, at);
    assert!(h.query_by_label("Hide Data Plot").is_some(), "the plot's menu");
    assert!(h.query_by_label("Go to Book1").is_some());
    menu_path(&mut h, &["Change Plot to", "Scatter"]);
    assert_eq!(graph(&h).layers[0].plots[0].kind, PlotKind::Scatter);
    assert_ne!(graph(&h).layers[0].plots[1].kind, PlotKind::Scatter, "only the clicked plot changes");
    assert_eq!(h.state().doc.undo_label(), Some("Change plot type"));
    menu(&mut h, "Edit", "Undo Change plot type");
    assert_ne!(graph(&h).layers[0].plots[0].kind, PlotKind::Scatter);
}

#[test]
fn the_plot_menu_moves_a_plot_to_the_right_y_axis_and_back_as_one_undo_step() {
    let mut h = start(vec![data_file("gright")]);
    let before = graph(&h).clone();
    assert_eq!(before.layers.len(), 1);
    let second = before.layers[0].plots[1].y;
    let at = on(&h, 0, ObjectKind::Plot(1));
    click_at(&mut h, at);
    right_click_at(&mut h, at);
    assert!(h.query_by_label("Move to Left Y Axis").is_none(), "it is on the left now");
    click(&mut h, "Move to Right Y Axis");
    let g = graph(&h);
    assert_eq!(g.layers.len(), 2, "a right Y layer was made");
    assert_eq!(g.right_y_member(0), Some(1));
    assert_eq!(g.layers[0].plots.len(), 1);
    assert_eq!(g.layers[1].plots.len(), 1);
    assert_eq!(g.layers[1].plots[0].y, second);
    assert_eq!(h.state().doc.undo_label(), Some("Move plot to right Y axis"));
    // Now its menu offers the way back; the empty right layer stays.
    let at = on(&h, 1, ObjectKind::Plot(0));
    right_click_at(&mut h, at);
    assert!(h.query_by_label("Move to Right Y Axis").is_none());
    click(&mut h, "Move to Left Y Axis");
    let g = graph(&h);
    assert_eq!(g.layers.len(), 2, "Origin keeps the empty layer");
    assert!(g.layers[1].plots.is_empty());
    assert_eq!(g.layers[0].plots.len(), 2);
    // Undo steps back one move at a time.
    menu(&mut h, "Edit", "Undo Move plot to left Y axis");
    assert_eq!(graph(&h).layers[1].plots.len(), 1);
    menu(&mut h, "Edit", "Undo Move plot to right Y axis");
    assert_eq!(*graph(&h), before);
}

#[test]
fn plot_details_chooses_the_y_axis_of_a_plot() {
    let mut h = start(vec![data_file("gdetail")]);
    let before = graph(&h).clone();
    let second = before.layers[0].plots[1].y;
    let at = on(&h, 0, ObjectKind::Plot(1));
    double_click_at(&mut h, at);
    assert!(h.state().graph_props.is_some(), "Plot Details is open");
    click(&mut h, "Group");
    click(&mut h, "Right");
    let g = graph(&h);
    assert_eq!(g.layers.len(), 2);
    assert_eq!(g.layers[1].plots[0].y, second);
    assert_eq!(g.right_y_member(0), Some(1));
    let d = h.state().graph_props.as_ref().unwrap();
    assert_eq!((d.layer, d.selected_plot()), (1, 0), "the dialog follows the plot");
    // The same choice made again changes nothing; Left brings it back.
    click(&mut h, "Left");
    let g = graph(&h);
    assert_eq!(g.layers[0].plots.len(), 2);
    assert!(g.layers[1].plots.is_empty());
    assert_eq!(g.layers[0].plots[1].y, second);
    // The dialog's edits are one undo step.
    assert_eq!(h.state().doc.undo_label(), Some("Change graph settings"));
    h.state_mut().doc.undo();
    assert_eq!(*graph(&h), before);
}

#[test]
fn the_axis_menu_hides_grids_zooms_and_rescales() {
    let mut h = start(vec![data_file("gaxis")]);
    let at = on(&h, 0, ObjectKind::YAxis);
    right_click_at(&mut h, at);
    click(&mut h, "Axis Zoom In");
    let y = &graph(&h).layers[0].y_axis;
    let (lo, hi) = (y.min.expect("zoom fixes the limits"), y.max.unwrap());
    assert!(hi - lo > 0.0);
    // The other axis is untouched.
    assert!(graph(&h).layers[0].x_axis.min.is_none());
    right_click_at(&mut h, at);
    click(&mut h, "Show Grids");
    assert!(graph(&h).layers[0].y_axis.major_grid.is_some());
    // Ctrl+R (Graph > Rescale to Show All) gives automatic limits back.
    menu(&mut h, "Graph", "Rescale to Show All");
    assert!(graph(&h).layers[0].y_axis.min.is_none());
    assert_eq!(h.state().doc.undo_label(), Some("Rescale to show all"));
}

#[test]
fn delete_removes_the_selected_legend() {
    let mut h = start(vec![data_file("glegend")]);
    let at = on(&h, 0, ObjectKind::Legend);
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Legend)));
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert!(!graph(&h).layers[0].legend.visible);
    assert_eq!(h.state().doc.undo_label(), Some("Delete legend"));
}

#[test]
fn the_legend_is_dragged_to_a_manual_position_in_one_undo_step() {
    let mut h = start(vec![data_file("dragleg")]);
    let before = graph(&h).layers[0].legend.position;
    assert!(!matches!(before, LegendPosition::Manual { .. }), "a corner to begin with");
    let at = on(&h, 0, ObjectKind::Legend);
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Legend)));
    drag(&mut h, at, at + egui::vec2(-80.0, 60.0));
    let LegendPosition::Manual { x, y } = graph(&h).layers[0].legend.position else {
        panic!("the legend is placed by hand now: {:?}", graph(&h).layers[0].legend.position);
    };
    // The centre was in the upper right of the frame and moved left and down.
    assert!(x < 0.95 && y > 0.05 && x > 0.3 && y < 0.6, "{x} {y}");
    assert_eq!(h.state().doc.undo_label(), Some("Move legend"));
    assert!(h.state().graph_props.is_none(), "a drag is not a double click");
    menu(&mut h, "Edit", "Undo Move legend");
    assert_eq!(graph(&h).layers[0].legend.position, before);
}

#[test]
fn reconstruct_legend_goes_back_to_the_automatic_text() {
    let mut h = start(vec![data_file("reconleg")]);
    h.state_mut().doc.apply("Legend text", |p| {
        let g = p.graphs().next().unwrap().id;
        p.graph_mut(g).unwrap().layers[0].legend.text = Some("\\l(1) first".into());
    });
    settle(&mut h);
    // From the legend's right-click menu ...
    let at = on(&h, 0, ObjectKind::Legend);
    right_click_at(&mut h, at);
    menu_path(&mut h, &["Legend", "Reconstruct Legend"]);
    assert_eq!(graph(&h).layers[0].legend.text, None);
    assert_eq!(h.state().doc.undo_label(), Some("Reconstruct legend"));
    // ... and from the Graph menu.
    h.state_mut().doc.apply("Legend text", |p| {
        let g = p.graphs().next().unwrap().id;
        p.graph_mut(g).unwrap().layers[0].legend.text = Some("custom".into());
    });
    settle(&mut h);
    menu(&mut h, "Graph", "Legend");
    menu_path(&mut h, &["Reconstruct Legend"]);
    assert_eq!(graph(&h).layers[0].legend.text, None);
}

#[test]
fn the_legend_tab_edits_text_and_columns_and_reconstructs() {
    let mut h = start(vec![data_file("legtab")]);
    h.state_mut().doc.apply("Legend", |p| {
        let g = p.graphs().next().unwrap().id;
        let legend = &mut p.graph_mut(g).unwrap().layers[0].legend;
        legend.text = Some("\\l(2) only the second".into());
        legend.columns = 2;
    });
    open_props(&mut h, 0, PropsTab::Legend, None);
    assert!(h.query_by_label("Columns").is_some() && h.query_by_label("Text").is_some());
    click(&mut h, "Reconstruct");
    assert_eq!(graph(&h).layers[0].legend.text, None);
    assert_eq!(graph(&h).layers[0].legend.columns, 2);
}

#[test]
fn the_layer_icon_has_origins_menu_and_a_double_click_opens_the_plot_list() {
    let mut h = start(vec![data_file("gicon")]);
    let at = graph_ui(&h).page_point(0.0, 0.0).unwrap() + egui::vec2(9.0, 9.0);
    right_click_at(&mut h, at);
    for entry in ["Hide Layer", "Layer Contents...", "Plot Setup...", "Axis..."] {
        assert!(h.query_by_label_contains(entry).is_some(), "{entry}");
    }
    close_menus(&mut h);
    double_click_at(&mut h, at);
    let d = h.state().graph_props.as_ref().expect("Plot Setup");
    assert_eq!(d.tab(), PropsTab::Plots);
}

#[test]
fn after_a_click_into_a_graph_the_next_click_reaches_the_window_title_under_it() {
    // The bug seen before: a layer stayed over the workspace after a click into the graph.
    let mut h = start(vec![data_file("gpopup")]);
    let (book, graph_id) = window_ids(&h);
    let inside = on(&h, 0, ObjectKind::Plot(0));
    click_at(&mut h, inside);
    right_click_at(&mut h, inside);
    close_menus(&mut h);
    assert_eq!(h.state().active, Some(graph_id));
    click(&mut h, "Title Book1 - sample");
    assert_eq!(h.state().active, Some(book), "the click on the title was not swallowed");
}

#[test]
fn a_saved_project_reopens_with_its_windows_where_they_were() {
    let dir = save_dir("places");
    let file = dir.join("arranged.sheaf");
    let mut h = start(vec![data_file("places")]);
    let (book, graph_id) = window_ids(&h);
    click(&mut h, "Minimise Book1 - sample");
    click(&mut h, "Maximise Graph1 - sample");
    let before = h.state().workspace.places();
    assert_eq!(before.len(), 2);
    h.state_mut().save_to(file.clone());
    assert!(!h.state().doc.dirty, "recording the places does not count as a change");

    let h2 = start(vec![file]);
    let after = h2.state().workspace.places();
    assert_eq!(after, before, "same places, states and stacking order");
    assert_eq!(h2.state().workspace.mode(book), Some(crate::workspace::Mode::Minimised));
    assert_eq!(h2.state().workspace.mode(graph_id), Some(crate::workspace::Mode::Maximised));
    assert_eq!(h2.state().active, Some(graph_id));
}

#[test]
fn deleting_a_column_drops_the_stack_settings_and_tick_labels_that_name_it() {
    let mut p = Project::new();
    let book = p.add_workbook("w");
    let names = p.add_column(book, 0, Designation::X, ColumnData::text(vec!["a".into(), "b".into()])).unwrap();
    let y1 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let y2 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![3.0, 4.0])).unwrap();
    let g = p.plot_columns(Template::Line, &[y1, y2]).unwrap();
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.x_axis.tick_label_source = TickLabelSource::Column { column: names };
        l.plot_offset_mut(y1).y = 2.0;
        l.plot_offset_mut(y2).follows = Some(y1);
    }
    delete_columns(&mut p, &[names, y1]);
    let l = &p.graph(g).unwrap().layers[0];
    assert_eq!(l.x_axis.tick_label_source, TickLabelSource::Auto);
    assert!(l.plot_offset(y1).is_none());
    assert_eq!(l.plot_offset(y2).and_then(|o| o.follows), None);
}

#[test]
fn the_integrate_gadget_shows_a_range_and_new_output_writes_the_area_to_a_sheet() {
    let mut h = start(vec![data_file("gadget")]);
    menu(&mut h, "Gadgets", "Integrate...");
    let g = graph_ui(&h).gadget.expect("the gadget is open");
    assert_eq!((g.layer, g.plot), (0, 0));
    assert!(g.x0 < g.x1, "over the middle of the X axis");
    let range = graph_ui(&h).gadget_rect.expect("and drawn");
    // Drag the right edge further right: the range grows.
    drag(&mut h, range.right_center(), range.right_center() + egui::vec2(40.0, 0.0));
    assert!(graph_ui(&h).gadget.unwrap().x1 > g.x1);
    let range = graph_ui(&h).gadget_rect.unwrap();
    right_click_at(&mut h, range.center());
    click(&mut h, "Baseline: Line Through End Points");
    assert!(graph_ui(&h).gadget.unwrap().end_point_baseline);
    let sheets = |h: &Harness<'static, SheafApp>| project(h).workbooks().next().unwrap().sheets.len();
    let before = sheets(&h);
    right_click_at(&mut h, range.center());
    click(&mut h, "New Output");
    assert_eq!(h.state().doc.undo_label(), Some("Integrate"));
    assert!(sheets(&h) > before, "the result sheet");
    // Esc closes the gadget.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(graph_ui(&h).gadget.is_none());
}

#[test]
fn copy_page_puts_the_graph_on_the_clipboard_at_40_percent_with_a_border() {
    let mut h = start(vec![data_file("copypage")]);
    menu(&mut h, "Edit", "Copy Page");
    // Origin: size factor 40 %, margin auto (border round everything drawn), 300 dpi.
    let layout = sheaf_plot::ExportLayout { margin: sheaf_plot::Margin::Auto, border_px: None };
    let area = sheaf_plot::export_area(h.state().doc.project(), graph(&h).id, layout, Some(300.0)).unwrap();
    let page = &graph(&h).page;
    assert!(area.w < page.width_pt, "a border box is smaller than the page");
    let expected = [(area.w / 72.0 * 300.0 * 0.4).round() as usize, (area.h / 72.0 * 300.0 * 0.4).round() as usize];
    let got = h.state().copied_page.expect("a picture was copied");
    assert!(got[0].abs_diff(expected[0]) <= 1 && got[1].abs_diff(expected[1]) <= 1, "{got:?} vs {expected:?}");
}

#[test]
fn the_data_reader_pins_a_point_and_steps_through_the_rows() {
    let mut h = start(vec![data_file("reader")]);
    let at = on(&h, 0, ObjectKind::Plot(0));
    bar_button!(h, "Tools", "Data Reader").click();
    settle(&mut h);
    assert!(graph_ui(&h).reader, "the tool is on");
    assert!(!bar_button!(h, "Tools", "Pointer").accesskit_node().is_disabled());
    click_at(&mut h, at + egui::vec2(4.0, 4.0));
    let p = graph_ui(&h).reader_point.expect("a click pins the nearest point");
    assert_eq!((p.layer, p.plot), (0, 0));
    h.event(egui::Event::PointerMoved(at));
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert_eq!(graph_ui(&h).reader_point.unwrap().row, p.row + 1, "Right steps to the next row");
    assert!(graph(&h).layers[0].plots.iter().all(|pl| pl.visible), "the reader changes nothing");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!graph_ui(&h).reader);
}

// ----- Statistics: two-way ANOVA, nonparametric tests, correlation ---------------------------------------

/// Writes `text` as `<name>.csv` in a fresh folder.
fn csv_file(tag: &str, name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.csv"));
    std::fs::write(&path, text).unwrap();
    path
}

/// The result sheet that the last tool added to the first workbook: its name and the numbers of a column.
fn result_column(h: &Harness<'static, SheafApp>, column: &str) -> (String, Vec<f64>) {
    let b = project(h).workbooks().next().unwrap();
    let ws = b.sheets.last().unwrap();
    let c = ws.columns.iter().find(|c| c.long_name == column).unwrap_or_else(|| panic!("no column {column}"));
    (ws.name.clone(), c.numeric().unwrap().to_vec())
}

/// Statistics > ANOVA > Two-Way ANOVA...: the factor columns can be text; the window starts with the first
/// three columns as factor A, factor B and data; OK adds the result sheet and one undo removes it.
#[test]
fn the_statistics_menu_runs_a_two_way_anova_with_text_factors() {
    let data = csv_file(
        "anova2",
        "design",
        "dose,batch,response\nlow,1,1\nlow,1,2\nlow,2,4\nlow,2,5\nhigh,1,3\nhigh,1,5\nhigh,2,8\nhigh,2,9\n",
    );
    let mut h = start(vec![data]);
    show_window(&mut h, "Book1 - design");
    menu_path(&mut h, &["Statistics", "ANOVA", "Two-Way ANOVA..."]);
    assert!(h.state().analysis.is_some(), "the two-way ANOVA window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    let (sheet, df) = result_column(&h, "DF");
    assert!(sheet.starts_with("ANOVA2Way"), "{sheet}");
    // dose, batch, dose*batch, model, error, total of the balanced 2 x 2 design with two numbers per cell.
    assert_eq!(df, vec![1.0, 1.0, 1.0, 3.0, 4.0, 7.0]);
    let (_, ss) = result_column(&h, "Sum of Squares");
    // Total 53.875; dose (means 3 and 6.25) 21.125 and batch (3 and 7.25... ) 28.125, each with one degree of freedom.
    assert!((ss[5] - 53.875).abs() < 1e-9 && (ss[0] - 21.125).abs() < 1e-9 && (ss[1] - 28.125).abs() < 1e-9, "{ss:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Two-way ANOVA"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);
}

/// Statistics > Descriptive Statistics > Cross Tabulation...: two text columns, counts and tests in a new sheet.
#[test]
fn the_descriptive_menu_runs_a_cross_tabulation_of_text_columns() {
    let data = csv_file(
        "crosstab",
        "survey",
        "treated,better\ntreated,better\ntreated,same\ncontrol,same\ncontrol,same\ncontrol,better\ncontrol,same\ntreated,better\n",
    );
    let mut h = start(vec![data]);
    show_window(&mut h, "Book1 - survey");
    menu_path(&mut h, &["Statistics", "Descriptive Statistics", "Cross Tabulation..."]);
    assert!(h.state().analysis.is_some(), "the cross tabulation window is open");
    for label in
        ["Row variable", "Column variable", "Row percent", "Column percent", "Total percent", "Expected counts"]
    {
        assert!(has_label(&h, label), "{label} is missing");
    }
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    let (sheet, counts) = result_column(&h, "better");
    assert!(sheet.starts_with("Crosstab"), "{sheet}");
    // Rows control and treated, then the total: 1 and 3 better, 4 in all.
    assert_eq!(counts, vec![1.0, 3.0, 4.0]);
    let (_, value) = result_column(&h, "Value");
    assert!((value[0] - 2.0).abs() < 1e-12, "Pearson chi-square {value:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Cross tabulation"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);
}

/// Statistics > Descriptive Statistics > 2D Frequency Counts/Binning...: the two Y columns are X and Y.
#[test]
fn the_descriptive_menu_runs_a_2d_frequency_count() {
    let mut h = start(vec![data_file("bin2d")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Statistics", "Descriptive Statistics", "2D Frequency Counts/Binning..."]);
    assert!(h.state().analysis.is_some(), "the 2D frequency count window is open");
    assert!(has_label(&h, "Set the X range") && has_label(&h, "Set the Y range"));
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    let (sheet, total) = result_column(&h, "Total");
    assert!(sheet.starts_with("Bin2D"), "{sheet}");
    // Four pairs in 10 x 10 bins over the data range.
    assert_eq!(total.iter().sum::<f64>(), 4.0);
    assert_eq!(total.len(), 10);
    assert_eq!(h.state().doc.undo_label(), Some("2D frequency count"));
}

/// Statistics > ANOVA > One-Way ANOVA...: Holm-Bonferroni and Dunnett are in the list of means comparisons; Dunnett
/// asks for the control group.
#[test]
fn the_one_way_anova_window_offers_holm_and_dunnett() {
    let mut h = start(vec![data_file("anova-dunnett")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Statistics", "ANOVA", "One-Way ANOVA..."]);
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Tukey")).click();
    settle(&mut h);
    assert!(has_label(&h, "Holm-Bonferroni") && has_label(&h, "Dunnett"));
    click(&mut h, "Dunnett");
    assert!(has_label(&h, "Control group"), "Dunnett asks for the control group");
    click(&mut h, "OK");
    let ws = project(&h).workbooks().next().unwrap().sheets.last().unwrap().clone();
    let labels =
        ws.columns.iter().find(|c| c.long_name == "Dunnett comparison").expect("the Dunnett table").text().unwrap();
    assert_eq!(labels, ["down - up"], "the first column is the control");
    assert_eq!(h.state().doc.undo_label(), Some("One-way ANOVA"));
}

/// Statistics > Nonparametric Tests > Mann-Whitney Test... with the two Y columns of the sheet.
#[test]
fn the_nonparametric_menu_runs_a_mann_whitney_test() {
    let mut h = start(vec![data_file("mannwhitney")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Statistics", "Nonparametric Tests", "Mann-Whitney Test..."]);
    assert!(h.state().analysis.is_some(), "the Mann-Whitney window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    let (sheet, u) = result_column(&h, "U");
    assert!(sheet.starts_with("MannWhitney"), "{sheet}");
    // up = 2.5 .. 8.5 against down = 30.5 .. 0.5: ranks 3 4 5 7 against 8 6 2 1... U of the first sample.
    assert_eq!(u.len(), 1);
    assert!(u[0].is_finite() && (0.0..=16.0).contains(&u[0]), "{u:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Mann-Whitney test"));
}

/// Statistics > Nonparametric Tests > One-Sample Wilcoxon Signed Rank Test...: one row per chosen column.
#[test]
fn the_nonparametric_menu_runs_a_one_sample_signed_rank_test() {
    let mut h = start(vec![data_file("signrank")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Statistics", "Nonparametric Tests", "One-Sample Wilcoxon Signed Rank Test..."]);
    assert!(h.state().analysis.is_some(), "the signed rank window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    let (sheet, n) = result_column(&h, "N");
    assert!(sheet.starts_with("SignedRank"), "{sheet}");
    assert_eq!(n, vec![4.0, 4.0], "one row per chosen column");
    assert_eq!(h.state().doc.undo_label(), Some("One-sample Wilcoxon signed rank test"));
}

/// Statistics > Nonparametric Tests > Paired Sample Wilcoxon Signed Rank Test...: the same window, starting with
/// the paired test, which gives a single row for the two columns.
#[test]
fn the_nonparametric_menu_runs_a_paired_signed_rank_test() {
    let mut h = start(vec![data_file("signrank-paired")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Statistics", "Nonparametric Tests", "Paired Sample Wilcoxon Signed Rank Test..."]);
    assert!(h.state().analysis.is_some(), "the paired window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    let (_, n) = result_column(&h, "N");
    assert_eq!(n, vec![4.0], "the paired test is one row");
    assert_eq!(h.state().doc.undo_label(), Some("Paired Wilcoxon signed rank test"));
}

/// Statistics > Nonparametric Tests > Kruskal-Wallis ANOVA...
#[test]
fn the_nonparametric_menu_runs_a_kruskal_wallis_test() {
    let mut h = start(vec![data_file("kruskal")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Statistics", "Nonparametric Tests", "Kruskal-Wallis ANOVA..."]);
    click(&mut h, "OK");
    let (sheet, value) = result_column(&h, "Value");
    assert!(sheet.starts_with("KruskalWallis"), "{sheet}");
    // chi-square, DF and p of the two groups.
    assert_eq!(value.len(), 3);
    assert_eq!(value[1], 1.0);
    assert!(value[2] > 0.0 && value[2] <= 1.0, "{value:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Kruskal-Wallis test"));
}

/// The other nonparametric tests of the Statistics menu run on the two columns of the sample file.
#[test]
fn the_nonparametric_menu_runs_sign_ks_median_and_friedman_tests() {
    // up = 2.5 4.5 6.5 8.5 and down = 30.5 20.5 10.5 0.5: the first is smaller in every pair.
    for (label, sheet_prefix, column, undo) in [
        ("Paired Sample Sign Test...", "SignTest", "Prob", "Sign test"),
        ("Two Sample Kolmogorov-Smirnov Test...", "KolmogorovSmirnov", "D", "Two-sample Kolmogorov-Smirnov test"),
        ("Mood's Median Test...", "MedianTest", "Value", "Median test"),
        ("Friedman ANOVA...", "Friedman", "Value", "Friedman ANOVA"),
    ] {
        let mut h = start(vec![data_file("np-more")]);
        show_window(&mut h, "Book1 - sample");
        menu_path(&mut h, &["Statistics", "Nonparametric Tests", label]);
        assert!(h.state().analysis.is_some(), "the window of {label} is open");
        click(&mut h, "OK");
        assert!(h.state().analysis.is_none(), "{label}");
        let (sheet, values) = result_column(&h, column);
        assert!(sheet.starts_with(sheet_prefix), "{label}: {sheet}");
        assert!(!values.is_empty(), "{label}");
        assert_eq!(h.state().doc.undo_label(), Some(undo));
    }
}

/// Statistics > Descriptive Statistics > Grubbs Test... and Dixon's Q-test...: one row per chosen column.
#[test]
fn the_descriptive_menu_runs_the_outlier_tests() {
    for (label, sheet_prefix, column, undo) in
        [("Grubbs Test...", "Grubbs", "G", "Grubbs test"), ("Dixon's Q-test...", "Dixon", "Q", "Dixon's Q test")]
    {
        let mut h = start(vec![data_file("outlier")]);
        show_window(&mut h, "Book1 - sample");
        menu_path(&mut h, &["Statistics", "Descriptive Statistics", label]);
        assert!(h.state().analysis.is_some(), "the window of {label} is open");
        click(&mut h, "OK");
        let (sheet, values) = result_column(&h, column);
        assert!(sheet.starts_with(sheet_prefix), "{label}: {sheet}");
        assert_eq!(values.len(), 2, "{label}: one row per chosen column");
        assert_eq!(h.state().doc.undo_label(), Some(undo));
    }
}

/// Statistics > Descriptive Statistics > Statistics on Rows...: mean and SD columns appear behind the last column.
#[test]
fn the_descriptive_menu_adds_row_statistics_columns() {
    let mut h = start(vec![data_file("rowstats")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Statistics", "Descriptive Statistics", "Statistics on Rows..."]);
    assert!(h.state().analysis.is_some(), "the row statistics window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    assert_eq!(h.state().doc.undo_label(), Some("Statistics on rows"));
    let sheet = &project(&h).workbooks().next().unwrap().sheets[0];
    let names: Vec<&str> = sheet.columns.iter().map(|c| c.long_name.as_str()).collect();
    assert_eq!(&names[names.len() - 2..], ["Mean", "Standard Deviation"], "{names:?}");
    let mean = sheet.columns[names.len() - 2].numeric().unwrap();
    assert_eq!(mean[0], (2.5 + 30.5) / 2.0);
}

/// Statistics > Descriptive Statistics > Correlation Coefficient...: up and down are perfectly anti-correlated.
#[test]
fn the_statistics_menu_computes_correlation_coefficients() {
    let mut h = start(vec![data_file("corr")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Statistics", "Descriptive Statistics", "Correlation Coefficient..."]);
    assert!(h.state().analysis.is_some(), "the correlation window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    let (sheet, up) = result_column(&h, "up");
    assert!(sheet.starts_with("Correlation"), "{sheet}");
    // The Pearson matrix: column "up" holds r(up, up) = 1 and r(down, up) = -1 (the data are straight lines).
    assert!((up[0] - 1.0).abs() < 1e-12 && (up[1] + 1.0).abs() < 1e-12, "{up:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Correlation coefficients"));
}

// ----- free text labels ---------------------------------------------------------------------

fn labels<'a>(h: &'a Harness<'static, SheafApp>) -> &'a [sheaf_core::graph::TextLabel] {
    &graph(h).layers[0].labels
}

/// Types into the text box of the Text window (it has the keyboard when the window opens).
fn type_into_text_window(h: &mut Harness<'static, SheafApp>, text: &str, replace: bool) {
    if replace {
        h.event(egui::Event::Key {
            key: egui::Key::A,
            physical_key: Some(egui::Key::A),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        });
        h.step();
    }
    h.event(egui::Event::Text(text.to_string()));
    settle(h);
}

#[test]
fn add_text_places_a_label_and_double_click_edits_it_each_change_one_undo_step() {
    let mut h = start(vec![data_file("addtext")]);
    assert!(labels(&h).is_empty());
    menu(&mut h, "Graph", "Add Text...");
    assert!(h.state().text_dialog.is_some(), "the Text window opened");
    type_into_text_window(&mut h, r"\b(Zn-LMO)", false);
    click(&mut h, "OK");
    assert!(h.state().text_dialog.is_none());
    assert_eq!(labels(&h).len(), 1);
    assert_eq!(labels(&h)[0].text, r"\b(Zn-LMO)");
    assert_eq!((labels(&h)[0].x, labels(&h)[0].y), (0.5, 0.5), "the middle of the layer");
    assert_eq!(h.state().doc.undo_label(), Some("Add text"));

    // A double click opens it; a changed text, size and rotation are one step.
    let at = on(&h, 0, ObjectKind::Label(0));
    double_click_at(&mut h, at);
    assert!(h.state().text_dialog.is_some(), "double click opens the Text window");
    type_into_text_window(&mut h, "Peak 311", true);
    click(&mut h, "OK");
    assert_eq!(labels(&h).len(), 1);
    assert_eq!(labels(&h)[0].text, "Peak 311");
    assert_eq!(h.state().doc.undo_label(), Some("Edit text"));
    menu(&mut h, "Edit", "Undo Edit text");
    assert_eq!(labels(&h)[0].text, r"\b(Zn-LMO)");
    menu(&mut h, "Edit", "Undo Add text");
    assert!(labels(&h).is_empty());
}

#[test]
fn an_empty_text_or_cancel_adds_no_label() {
    let mut h = start(vec![data_file("emptytext")]);
    menu(&mut h, "Graph", "Add Text...");
    click(&mut h, "OK");
    assert!(h.state().text_dialog.is_none());
    assert!(labels(&h).is_empty());
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "x", false);
    click(&mut h, "Cancel");
    assert!(labels(&h).is_empty());
    assert!(!h.state().doc.dirty || h.state().doc.undo_label() != Some("Add text"));
}

#[test]
fn the_text_tool_places_a_label_where_the_page_is_clicked() {
    let mut h = start(vec![data_file("texttool")]);
    bar_button!(h, "Tools", "Text").click();
    settle(&mut h);
    assert!(graph_ui(&h).text_tool, "the tool is on");
    let frame = sheaf_plot::layout_geometry(project(&h), graph(&h).id).unwrap()[0].frame;
    // A quarter of the way across and down the frame.
    let at = graph_ui(&h).page_point(frame.x + frame.w * 0.25, frame.y + frame.h * 0.75).unwrap();
    click_at(&mut h, at);
    assert!(h.state().text_dialog.is_some(), "the click opened the Text window");
    assert!(!graph_ui(&h).text_tool, "the tool ends after one label");
    type_into_text_window(&mut h, "After Delithiation", false);
    click(&mut h, "OK");
    let l = &labels(&h)[0];
    assert_eq!(l.text, "After Delithiation");
    assert!((l.x - 0.25).abs() < 0.01 && (l.y - 0.75).abs() < 0.01, "{} {}", l.x, l.y);
}

#[test]
fn a_label_is_dragged_to_a_new_place_in_one_undo_step() {
    let mut h = start(vec![data_file("dragtext")]);
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "Zn-LMO", false);
    click(&mut h, "OK");
    let before = (labels(&h)[0].x, labels(&h)[0].y);
    let at = on(&h, 0, ObjectKind::Label(0));
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Label(0))));
    drag(&mut h, at, at + egui::vec2(60.0, -40.0));
    let after = (labels(&h)[0].x, labels(&h)[0].y);
    assert!(after.0 > before.0 && after.1 < before.1, "{before:?} -> {after:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Move text"));
    assert!(h.state().text_dialog.is_none(), "a drag is not a double click");
    menu(&mut h, "Edit", "Undo Move text");
    assert_eq!((labels(&h)[0].x, labels(&h)[0].y), before);
}

/// With a graph active, the Analysis menu works on its data plot, as Origin's does on the active
/// data plot: the selected plot, or else the active layer's first plot. The results go to the
/// plot's workbook.
#[test]
fn the_analysis_menu_on_a_graph_works_on_its_data_plot() {
    let mut h = start(vec![data_file("graph-analysis")]);
    show_window(&mut h, "Graph1 - sample");
    let (book, gid, ys) = {
        let p = project(&h);
        let g = p.graphs().next().unwrap();
        (p.workbooks().next().unwrap().id, g.id, g.layers[0].plots.iter().map(|pl| pl.y).collect::<Vec<_>>())
    };
    assert_eq!(h.state().menu_context().curve, Some((book, ys[0])), "the first plot of the active layer");
    h.state_mut().graphs.get_mut(&gid).unwrap().selected = Some((0, sheaf_plot::scene::ObjectKind::Plot(1)));
    assert_eq!(h.state().menu_context().curve, Some((book, ys[1])), "a selected plot wins");
    menu_path(&mut h, &["Analysis", "Mathematics", "Integrate..."]);
    let dialog =
        h.state().analysis.as_ref().expect("the Integrate window is open, not a message to activate a workbook");
    assert_eq!(dialog.columns(), &[ys[1]]);
}

// ----- the dialogs in Origin's layout: Column Properties, the analysis frame, the Import Wizard ----

/// Types into the n-th text box of the window that is open (in the order they are drawn), replacing
/// what it holds.
fn type_into_box(h: &mut Harness<'static, SheafApp>, n: usize, text: &str) {
    h.query_all_by_role(egui::accesskit::Role::TextInput).nth(n).expect("the text box exists").focus();
    settle(h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(text.to_string()));
    settle(h);
}

/// Whether at least one widget carries this label (a label can appear in several places).
fn has_label(h: &Harness<'static, SheafApp>, label: &str) -> bool {
    h.query_all_by_label(label).next().is_some()
}

fn is_disabled(h: &Harness<'static, SheafApp>, label: &str) -> bool {
    h.get_by_label(label).accesskit_node().is_disabled()
}

/// The x position of a button, to check the order in which buttons are drawn.
fn button_x(h: &Harness<'static, SheafApp>, label: &str) -> f32 {
    h.get_by_label(label).rect().center().x
}

fn open_column_props(h: &mut Harness<'static, SheafApp>, short: &str) -> ColumnId {
    let id = column_id(h, short);
    let dialog = ColumnPropsDialog::new(project(h), id);
    h.state_mut().column_props = dialog;
    settle(h);
    id
}

/// Opens an analysis window on the named columns of the first workbook.
fn open_analysis_on(h: &mut Harness<'static, SheafApp>, kind: AnalysisKind, shorts: &[&str]) {
    let book = project(h).workbooks().next().unwrap().id;
    let columns = shorts.iter().map(|s| column_id(h, s)).collect();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Analyze { book, columns, kind }, &ctx);
    settle(h);
}

fn open_analysis(h: &mut Harness<'static, SheafApp>, kind: AnalysisKind) {
    let book = project(h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Analyze { book, columns: Vec::new(), kind }, &ctx);
    settle(h);
}

#[test]
fn column_properties_has_origins_fields_and_buttons_in_windows_order() {
    let mut h = start(vec![data_file("colprops-layout")]);
    open_column_props(&mut h, "B");
    for label in [
        "<< Prev",
        "Next >>",
        "Short Name",
        "Long Name",
        "Units",
        "Comments",
        "Options",
        "Plot Designation",
        "Format",
        "Display",
        "Column Width",
        "Apply to all columns to the right",
    ] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    assert!(button_x(&h, "OK") < button_x(&h, "Cancel") && button_x(&h, "Cancel") < button_x(&h, "Apply"));
    assert!(is_disabled(&h, "Apply"), "Apply is grey until something changes");
}

#[test]
fn column_properties_fields_change_the_model_and_apply_keeps_the_window_open() {
    let mut h = start(vec![data_file("colprops-apply")]);
    let b = open_column_props(&mut h, "B");
    // Long name and units.
    type_into_box(&mut h, 1, "Signal");
    type_into_box(&mut h, 2, "mA");
    assert!(!is_disabled(&h, "Apply"), "a change turns Apply on");
    assert_eq!(project(&h).column(b).unwrap().long_name, "up", "nothing reaches the project before Apply");
    click(&mut h, "Apply");
    assert!(h.state().column_props.is_some(), "Apply does not close the window");
    let c = project(&h).column(b).unwrap();
    assert_eq!((c.long_name.as_str(), c.units.as_str()), ("Signal", "mA"));
    assert!(is_disabled(&h, "Apply"), "Apply is grey again once it was applied");
    // The designation, through its drop-down, then OK.
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Y")).click();
    settle(&mut h);
    click(&mut h, "Y error");
    click(&mut h, "OK");
    assert!(h.state().column_props.is_none(), "OK closes the window");
    assert_eq!(project(&h).column(b).unwrap().designation, Designation::YError);
}

#[test]
fn column_properties_cancel_changes_nothing() {
    let mut h = start(vec![data_file("colprops-cancel")]);
    let b = open_column_props(&mut h, "B");
    let before = project(&h).column(b).unwrap().clone();
    let undo = h.state().doc.undo_label().map(str::to_string);
    type_into_box(&mut h, 2, "mA");
    type_into_box(&mut h, 1, "Signal");
    click(&mut h, "Cancel");
    assert!(h.state().column_props.is_none());
    let c = project(&h).column(b).unwrap();
    assert_eq!((c.long_name.as_str(), c.units.as_str(), c.designation), ("up", "", before.designation));
    assert_eq!(h.state().doc.undo_label().map(str::to_string), undo, "no undo step was made");
}

#[test]
fn column_properties_next_and_prev_move_along_the_sheet_and_keep_the_changes() {
    let mut h = start(vec![data_file("colprops-steps")]);
    let a = open_column_props(&mut h, "A");
    assert!(is_disabled(&h, "<< Prev"), "the first column has no previous one");
    type_into_box(&mut h, 1, "Time");
    click(&mut h, "Next >>");
    let b = column_id(&h, "B");
    assert_eq!(h.state().column_props.as_ref().unwrap().column(), b);
    assert_eq!(project(&h).column(a).unwrap().long_name, "Time", "Next applies the changes of the column left");
    click(&mut h, "<< Prev");
    assert_eq!(h.state().column_props.as_ref().unwrap().column(), a);
}

#[test]
fn column_properties_apply_to_all_columns_to_the_right_copies_designation_and_format() {
    use sheaf_core::format::NumberFormat;
    let mut h = start(vec![data_file("colprops-right")]);
    let b = open_column_props(&mut h, "B");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Automatic")).click();
    settle(&mut h);
    click(&mut h, "Scientific");
    click(&mut h, "Apply to all columns to the right");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Y")).click();
    settle(&mut h);
    click(&mut h, "Z");
    click(&mut h, "OK");
    let c = column_id(&h, "C");
    for id in [b, c] {
        let col = project(&h).column(id).unwrap();
        assert_eq!(col.designation, Designation::Z);
        assert!(matches!(col.format, NumberFormat::Scientific(_)), "{:?}", col.format);
    }
    let a = project(&h).column(column_id(&h, "A")).unwrap();
    assert_eq!(a.designation, Designation::X, "the column to the left is not touched");
}

#[test]
fn the_analysis_window_has_origins_frame_and_apply_runs_without_closing() {
    let mut h = start(vec![data_file("frame-apply")]);
    open_analysis(&mut h, AnalysisKind::Integrate);
    for label in ["Dialog Theme", "Description", "Recalculate", "Input", "Output", "Auto Preview", "Preview"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    h.get_by_label("Integrate: integ1");
    assert!(button_x(&h, "OK") < button_x(&h, "Cancel") && button_x(&h, "Cancel") < button_x(&h, "Apply"));
    click(&mut h, "Apply");
    assert!(h.state().analysis.is_some(), "Apply leaves the window open");
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!(b.sheets.len(), 2, "the tool ran");
    assert_eq!(b.sheets[1].columns[1].numeric().unwrap(), [16.5, 46.5]);
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
}

#[test]
fn smoothing_offers_lowess_loess_and_binomial_with_their_own_fields() {
    let mut h = start(vec![data_file("smooth-methods")]);
    open_analysis(&mut h, AnalysisKind::Smooth);
    assert!(has_label(&h, "Points of window"));
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Savitzky-Golay")).click();
    settle(&mut h);
    click(&mut h, "Lowess");
    assert!(has_label(&h, "Span") && !has_label(&h, "Points of window"), "Lowess is set by its span");
    assert!(is_disabled(&h, "At the ends"), "the ends do not apply to a local fit");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Lowess")).click();
    settle(&mut h);
    click(&mut h, "Binomial");
    assert!(has_label(&h, "Order") && !has_label(&h, "Span"));
    click(&mut h, "OK");
    let b = project(&h).workbooks().next().unwrap();
    let new = b.sheets[0].columns.iter().find(|c| c.long_name.starts_with("Smoothed")).expect("the smoothed column");
    assert!(new.comments.contains("binomial, order 1"), "{}", new.comments);
}

#[test]
fn find_peaks_has_smoothing_and_foot_options_and_writes_the_feet() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-pkfeet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("feet.csv");
    let mut text = String::from(
        "x,y
",
    );
    for k in 0..201 {
        let x = k as f64 * 0.1;
        text.push_str(&format!(
            "{x},{}
",
            (-(x - 4.0).powi(2)).exp() + 0.8 * (-(x - 9.0).powi(2)).exp() + 0.2
        ));
    }
    std::fs::write(&path, text).unwrap();
    let mut h = start(vec![path]);
    open_analysis(&mut h, AnalysisKind::Peaks);
    for label in ["Smoothing points before the search", "Foot height", "Limit the distance of the feet from the peak"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    click(&mut h, "OK");
    let b = project(&h).workbooks().next().unwrap();
    let peaks = &b.sheets[1];
    assert_eq!(peaks.column_by_name("Peak position (x)").unwrap().numeric().unwrap(), [4.0, 9.0]);
    // Without a limit the feet are the lowest points on each side: the start, the valley, the flat tail.
    let left = peaks.columns.iter().find(|c| c.long_name.starts_with("Left foot")).unwrap().numeric().unwrap();
    let right = peaks.columns.iter().find(|c| c.long_name.starts_with("Right foot")).unwrap().numeric().unwrap();
    assert!(left[0] == 0.0 && right[1] > 14.0, "{left:?} {right:?}");
    assert!((left[1] - right[0]).abs() < 1e-9, "the valley is the shared foot: {left:?} {right:?}");
}

/// The column a Smooth run added next to B.
fn smoothed_column(h: &Harness<'static, SheafApp>) -> ColumnId {
    let sheet = project(h).workbooks().next().unwrap().sheet();
    sheet.columns.iter().find(|c| c.long_name == "Smoothed up").expect("the smoothed column").id
}

#[test]
fn recalculate_offers_manual_and_auto_for_smoothing_only_and_a_manual_result_waits() {
    let mut h = start(vec![data_file("recalc-manual")]);
    open_analysis_on(&mut h, AnalysisKind::Smooth, &["B"]);
    // Manual is the default, as in Origin.
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Manual")).click();
    settle(&mut h);
    assert!(!is_disabled(&h, "Auto"), "Smooth can follow its input");
    click(&mut h, "Auto");
    click(&mut h, "Cancel");
    // A tool that cannot be linked shows None and greys the other two.
    open_analysis_on(&mut h, AnalysisKind::Normalize, &["B"]);
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("None")).click();
    settle(&mut h);
    assert!(is_disabled(&h, "Manual") && is_disabled(&h, "Auto"));
    click(&mut h, "Cancel");
    // Manual: the result is marked when the input changes and runs again on Recalculate.
    open_analysis_on(&mut h, AnalysisKind::Smooth, &["B"]);
    click(&mut h, "OK");
    let b = column_id(&h, "B");
    let out = smoothed_column(&h);
    let before = project(&h).numeric(out).unwrap().to_vec();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SetCell { column: b, row: 0, text: "100".into() }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).numeric(out).unwrap(), before.as_slice(), "a Manual result waits");
    assert_eq!(project(&h).column(out).unwrap().analysis.unwrap().state, sheaf_core::FormulaState::Pending);
    h.state_mut().apply(Action::Recalculate(vec![out]), &ctx);
    settle(&mut h);
    assert_ne!(project(&h).numeric(out).unwrap(), before.as_slice(), "Recalculate ran the tool again");
    assert_eq!(project(&h).column(out).unwrap().analysis.unwrap().state, sheaf_core::FormulaState::Current);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets[0].columns.len(), 4, "the column was reused");
}

#[test]
fn an_auto_result_follows_its_input_in_the_same_undo_step() {
    let mut h = start(vec![data_file("recalc-auto")]);
    open_analysis_on(&mut h, AnalysisKind::Smooth, &["B"]);
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Manual")).click();
    settle(&mut h);
    click(&mut h, "Auto");
    click(&mut h, "OK");
    let b = column_id(&h, "B");
    let out = smoothed_column(&h);
    let first = project(&h).numeric(out).unwrap().to_vec();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SetCell { column: b, row: 0, text: "100".into() }, &ctx);
    settle(&mut h);
    let up = project(&h).numeric(b).unwrap().to_vec();
    let x = project(&h).numeric(column_id(&h, "A")).unwrap().to_vec();
    let want = sheaf_calc::smooth_xy(&x, &up, &sheaf_calc::SmoothOptions::default());
    assert_eq!(project(&h).numeric(out).unwrap(), want.as_slice());
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).numeric(b).unwrap()[0], 2.5, "one Undo reverts the edit");
    assert_eq!(project(&h).numeric(out).unwrap(), first.as_slice(), "and the result with it");
}

/// Analysis > Mathematics > Average Multiple Curves...: the mean of up and down on their shared X, with the SD
/// as the error column and the count; one Undo removes the result.
#[test]
fn the_mathematics_menu_averages_curves() {
    let mut h = start(vec![data_file("avecurves")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Analysis", "Mathematics", "Average Multiple Curves..."]);
    assert!(h.state().analysis.is_some(), "the window is open");
    for label in ["All X values of the curves", "Evenly spaced", "Those of one curve", "Linear", "Akima spline"] {
        assert!(has_label(&h, label), "{label}");
    }
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    let (sheet, mean) = result_column(&h, "Mean");
    assert!(sheet.starts_with("Average"), "{sheet}");
    assert_eq!(mean, vec![16.5, 12.5, 8.5, 4.5], "(2.5 + 30.5) / 2 and so on");
    let (_, sd) = result_column(&h, "SD");
    // The sample SD of two values is their distance over the square root of two.
    for (s, gap) in sd.iter().zip([28.0, 16.0, 4.0, 8.0]) {
        assert!((s - gap / 2.0_f64.sqrt()).abs() < 1e-9, "{sd:?}");
    }
    let (_, n) = result_column(&h, "N");
    assert_eq!(n, vec![2.0; 4]);
    assert_eq!(h.state().doc.undo_label(), Some("Average multiple curves"));
}

/// The evenly spaced choice shows its point count and gives that many rows.
#[test]
fn average_curves_can_use_evenly_spaced_x() {
    let mut h = start(vec![data_file("avecurves-even")]);
    open_analysis_on(&mut h, AnalysisKind::AverageCurves, &["B", "C"]);
    click(&mut h, "Evenly spaced");
    assert!(has_label(&h, "Number of points"));
    click(&mut h, "OK");
    let (_, x) = result_column(&h, "t");
    assert_eq!(x.len(), 100, "the default is 100 points");
    assert_eq!((x[0], x[99]), (1.0, 4.0));
}

/// One column is not enough to average: the window stays open and says why.
#[test]
fn average_curves_with_one_column_is_refused_in_the_window() {
    let mut h = start(vec![data_file("avecurves-one")]);
    open_analysis_on(&mut h, AnalysisKind::AverageCurves, &["B"]);
    click(&mut h, "OK");
    assert!(h.state().analysis.is_some(), "the window stays open");
    assert!(h.state().doc.undo_label().is_none_or(|l| l != "Average multiple curves"));
}

/// Analysis > Mathematics > Intersect...: up and down cross between t = 3 and t = 4, at t = 10 / 3.
#[test]
fn the_mathematics_menu_finds_the_intersection_of_two_curves() {
    let mut h = start(vec![data_file("intersect")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Analysis", "Mathematics", "Intersect..."]);
    assert!(h.state().analysis.is_some(), "the window is open");
    assert!(has_label(&h, "Also list the points where the curves touch without crossing"));
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    let (sheet, x) = result_column(&h, "Intersection X (t)");
    assert!(sheet.starts_with("Intersect"), "{sheet}");
    assert_eq!(x.len(), 1);
    assert!((x[0] - 10.0 / 3.0).abs() < 1e-9, "{x:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Intersect"));
}

/// Analysis > Mathematics > Polygon Area...: up and down are two open polylines; each is closed by its last
/// point back to its first. up and down both run along a line, so their polygons have no area (the loop test below checks real numbers); the table has one row per column.
#[test]
fn the_mathematics_menu_gives_the_polygon_area_of_each_column() {
    let mut h = start(vec![data_file("polyarea")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Analysis", "Mathematics", "Polygon Area..."]);
    assert!(h.state().analysis.is_some(), "the window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    let (sheet, area) = result_column(&h, "Area");
    assert!(sheet.starts_with("PolygonArea"), "{sheet}");
    assert_eq!(area.len(), 2, "one row per column");
    assert!(area.iter().all(|a| a.abs() < 1e-9), "straight lines enclose nothing: {area:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Polygon area"));
}

/// A closed loop in a data file: the window passes the right X and Y, so the area is the 2 x 3 rectangle's.
#[test]
fn the_polygon_area_window_measures_a_closed_loop() {
    let loop_csv = csv_file("polyloop", "loop", "x,y\n0,0\n2,0\n2,3\n0,3\n");
    let mut h = start(vec![loop_csv]);
    show_window(&mut h, "Book1 - loop");
    menu_path(&mut h, &["Analysis", "Mathematics", "Polygon Area..."]);
    click(&mut h, "OK");
    let (_, area) = result_column(&h, "Area");
    assert_eq!(area, vec![6.0]);
    assert_eq!(result_column(&h, "Signed area").1, vec![6.0], "counterclockwise");
    assert_eq!(result_column(&h, "Centroid X").1, vec![1.0]);
    assert_eq!(result_column(&h, "Centroid Y").1, vec![1.5]);
    assert_eq!(result_column(&h, "Perimeter").1, vec![10.0]);
}

/// From a graph, Average Multiple Curves and Intersect start with the curves of the layer, so OK
/// runs without picking the columns again.
#[test]
fn from_a_graph_the_curve_tools_start_with_the_plots_of_the_layer() {
    let mut h = start(vec![data_file("graph-curves")]);
    assert!(h.state().menu_context().layer_curves.len() >= 2, "the graph plots up and down");
    menu_path(&mut h, &["Analysis", "Mathematics", "Intersect..."]);
    assert!(h.state().analysis.is_some());
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "it ran with the two plots");
    let (sheet, x) = result_column(&h, "Intersection X (t)");
    assert!(sheet.starts_with("Intersect"), "{sheet}");
    assert!((x[0] - 10.0 / 3.0).abs() < 1e-9, "{x:?}");
}

#[test]
fn from_a_graph_average_multiple_curves_starts_with_the_plots_of_the_layer() {
    let mut h = start(vec![data_file("graph-average")]);
    menu_path(&mut h, &["Analysis", "Mathematics", "Average Multiple Curves..."]);
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "it ran with the two plots");
    let (sheet, mean) = result_column(&h, "Mean");
    assert!(sheet.starts_with("Average"), "{sheet}");
    assert_eq!(mean, vec![16.5, 12.5, 8.5, 4.5]);
}

/// The new tools can follow their input: Intersect with Auto moves its point when a cell changes, in the same
/// undo step as the edit.
#[test]
fn an_auto_intersect_follows_an_edit_of_its_input() {
    let mut h = start(vec![data_file("intersect-auto")]);
    open_analysis_on(&mut h, AnalysisKind::Intersect, &["B", "C"]);
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Manual")).click();
    settle(&mut h);
    assert!(!is_disabled(&h, "Auto"), "Intersect can follow its input");
    click(&mut h, "Auto");
    let c = column_id(&h, "C");
    click(&mut h, "OK");
    let (_, first) = result_column(&h, "Intersection X (t)");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SetCell { column: c, row: 3, text: "20".into() }, &ctx);
    settle(&mut h);
    let (_, second) = result_column(&h, "Intersection X (t)");
    assert_ne!(first, second, "the crossing moved");
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    let (_, back) = result_column(&h, "Intersection X (t)");
    assert_eq!(first, back, "one Undo takes the edit and the result back");
}

#[test]
fn the_output_node_can_send_results_to_a_new_sheet_or_to_existing_columns() {
    let mut h = start(vec![data_file("output-choice")]);
    let b_id = column_id(&h, "B");
    open_analysis_on(&mut h, AnalysisKind::Smooth, &["B"]);
    assert!(has_label(&h, "Output to"));
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("New columns beside the input")).click();
    settle(&mut h);
    click(&mut h, "A new sheet");
    click(&mut h, "OK");
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!((b.sheets.len(), b.sheets[0].columns.len()), (2, 3), "the input sheet is untouched");
    assert_eq!(b.sheets[1].columns.len(), 2, "an X and a Y column");
    // Existing columns: the result lands in the chosen column (by default the one after the input).
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Analyze { book, columns: vec![b_id], kind: AnalysisKind::Smooth }, &ctx);
    settle(&mut h);
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("New columns beside the input")).click();
    settle(&mut h);
    click(&mut h, "Existing columns");
    assert!(has_label(&h, "Result for B"));
    click(&mut h, "OK");
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!(b.sheets[0].columns.len(), 3, "no column was added");
    assert!(b.sheets[0].columns[2].long_name.starts_with("Smoothed"), "{}", b.sheets[0].columns[2].long_name);
}

#[test]
fn auto_apply_runs_the_tool_again_after_each_change_and_replaces_its_result() {
    let mut h = start(vec![data_file("auto-apply")]);
    open_analysis_on(&mut h, AnalysisKind::Smooth, &["B"]);
    assert!(has_label(&h, "Auto Apply"));
    click(&mut h, "Auto Apply");
    settle(&mut h);
    let cols = |h: &Harness<'static, SheafApp>| project(h).workbooks().next().unwrap().sheets[0].columns.len();
    assert_eq!(cols(&h), 4, "ticking the box ran the tool once");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Savitzky-Golay")).click();
    settle(&mut h);
    click(&mut h, "Adjacent averaging");
    settle(&mut h);
    assert_eq!(cols(&h), 4, "the new run replaced the old result");
    let c = project(&h).workbooks().next().unwrap().sheets[0].columns[2].comments.clone();
    assert!(c.contains("adjacent averaging"), "{c}");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    assert_eq!(cols(&h), 4);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(cols(&h), 3, "one Undo removes everything the window did");
}

#[test]
fn the_analysis_window_cancel_changes_nothing() {
    let mut h = start(vec![data_file("frame-cancel")]);
    open_analysis(&mut h, AnalysisKind::Smooth);
    let undo = h.state().doc.undo_label().map(str::to_string);
    click(&mut h, "Cancel");
    assert!(h.state().analysis.is_none());
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!((b.sheets.len(), b.sheets[0].columns.len()), (1, 3));
    assert_eq!(h.state().doc.undo_label().map(str::to_string), undo);
}

#[test]
fn the_analysis_window_options_change_the_result() {
    let mut h = start(vec![data_file("frame-options")]);
    open_analysis(&mut h, AnalysisKind::Normalize);
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Divide by the maximum")).click();
    settle(&mut h);
    click(&mut h, "To the range 0 to 1");
    click(&mut h, "OK");
    let b = project(&h).workbooks().next().unwrap();
    let added: Vec<&sheaf_core::Column> =
        b.sheet().columns.iter().filter(|c| !["A", "B", "C"].contains(&c.short_name.as_str())).collect();
    assert_eq!(added.len(), 2, "one normalized column for each Y column");
    let v = added[0].numeric().unwrap();
    assert!(v.iter().copied().fold(f64::INFINITY, f64::min).abs() < 1e-12, "{v:?}");
    assert!((v.iter().copied().fold(f64::NEG_INFINITY, f64::max) - 1.0).abs() < 1e-12);
}

#[test]
fn a_preview_is_computed_on_request_and_writes_nothing() {
    let mut h = start(vec![data_file("frame-preview")]);
    open_analysis(&mut h, AnalysisKind::Smooth);
    assert!(h.state().analysis.as_ref().unwrap().preview_state().is_none(), "no preview before it is asked for");
    click(&mut h, "Preview");
    assert_eq!(h.state().analysis.as_ref().unwrap().preview_state(), Some(Ok(4)));
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!((b.sheets.len(), b.sheets[0].columns.len()), (1, 3), "a preview writes nothing");
    assert_eq!(h.state().doc.undo_label(), Some("Import data"));
    // The arrow hides and shows the pane.
    click(&mut h, "<<");
    click(&mut h, ">>");
    h.get_by_label("Auto Preview");
}

#[test]
fn auto_preview_follows_a_changed_option() {
    let mut h = start(vec![data_file("frame-auto")]);
    open_analysis(&mut h, AnalysisKind::Normalize);
    click(&mut h, "Auto Preview");
    assert_eq!(h.state().analysis.as_ref().unwrap().preview_state(), Some(Ok(4)), "ticking computes it");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Divide by the maximum")).click();
    settle(&mut h);
    click(&mut h, "To the range 0 to 1");
    assert_eq!(h.state().analysis.as_ref().unwrap().preview_state(), Some(Ok(4)));
    assert_eq!(project(&h).workbooks().next().unwrap().sheet().columns.len(), 3, "still nothing written");
}

#[test]
fn a_tool_without_a_preview_greys_the_preview_controls() {
    let mut h = start(vec![data_file("frame-nopreview")]);
    open_analysis(&mut h, AnalysisKind::Peaks);
    assert!(is_disabled(&h, "Preview"));
    assert!(is_disabled(&h, "Auto Preview"));
}

#[test]
fn the_range_picker_of_the_analysis_window_chooses_the_input_columns() {
    let mut h = start(vec![data_file("frame-picker")]);
    open_analysis(&mut h, AnalysisKind::Integrate);
    assert_eq!(h.state().analysis.as_ref().unwrap().columns().len(), 2);
    click(&mut h, "...");
    h.get_by_label("Select Input Columns");
    // Leave only the column "up" (B).
    click(&mut h, "C  down");
    // Two windows have an OK button now: the small picker was opened last, so it is drawn last.
    h.query_all_by_label("OK").last().expect("the picker's OK").click();
    settle(&mut h);
    assert_eq!(h.state().analysis.as_ref().unwrap().columns().len(), 1);
    click(&mut h, "OK");
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!(b.sheets[1].columns[1].numeric().unwrap(), [16.5], "only the chosen column was integrated");
}

#[test]
fn the_statistics_windows_share_the_frame() {
    let mut h = start(vec![data_file("frame-stats")]);
    open_analysis(&mut h, AnalysisKind::TTest);
    for label in ["Dialog Theme", "Recalculate", "Input", "Output", "Apply"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    assert!(is_disabled(&h, "Preview"), "a test has no preview");
    click(&mut h, "Cancel");
    assert!(h.state().analysis.is_none());
}

#[test]
fn the_file_menu_and_ctrl_3_open_the_import_wizard_on_its_source_page() {
    let mut h = start(Vec::new());
    menu_path(&mut h, &["File", "Import", "Import Wizard..."]);
    assert_eq!(h.state().import_wizard.as_ref().map(|w| w.page()), Some(WizardPage::Source));
    for label in ["Data Type", "Data Source", "Import Filter", "Target Window", "ASCII", "Cancel"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    // No file chosen yet: Back, Next and Finish are grey.
    for label in ["<< Back", "Next >>", "Finish"] {
        assert!(is_disabled(&h, label), "{label} is grey without a file");
    }
    assert!(is_disabled(&h, "Binary") && is_disabled(&h, "User Defined") && is_disabled(&h, "Clipboard"));
    click(&mut h, "Cancel");
    assert!(h.state().import_wizard.is_none());
    assert_eq!(project(&h).workbooks().count(), 0, "Cancel imports nothing");
    ctrl(&mut h, egui::Key::Num3);
    assert!(h.state().import_wizard.is_some(), "Ctrl+3 opens the wizard");
}

#[test]
fn typing_a_path_enables_the_wizard_buttons_and_the_pages_follow_next_and_back() {
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::ImportWizard, &ctx);
    settle(&mut h);
    let file = data_file("wizard-path");
    type_into_box(&mut h, 0, &file.display().to_string());
    assert!(!is_disabled(&h, "Next >>"), "a file that exists enables Next");
    assert!(!is_disabled(&h, "Finish"));
    for page in [
        WizardPage::HeaderLines,
        WizardPage::ColumnNames,
        WizardPage::FileNameOptions,
        WizardPage::DataColumns,
        WizardPage::SaveFilters,
    ] {
        click(&mut h, "Next >>");
        assert_eq!(h.state().import_wizard.as_ref().unwrap().page(), page);
    }
    assert!(is_disabled(&h, "Next >>"), "the last page has no next one");
    click(&mut h, "<< Back");
    assert_eq!(h.state().import_wizard.as_ref().unwrap().page(), WizardPage::DataColumns);
    // Cancel at the left; Back, Next and Finish at the right.
    assert!(button_x(&h, "Cancel") < button_x(&h, "<< Back"));
    assert!(button_x(&h, "<< Back") < button_x(&h, "Next >>") && button_x(&h, "Next >>") < button_x(&h, "Finish"));
}

#[test]
fn the_wizard_pages_drive_the_import_options_and_finish_imports() {
    let mut h = start(Vec::new());
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-wizard-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("scan.csv");
    std::fs::write(&file, "t,i\ns,mA\n1,2.5\n2,4.5\n3,6.5\n").unwrap();
    h.state_mut().import_wizard = Some(ImportWizard::new(vec![file]));
    settle(&mut h);
    // Header Lines and Column Names pages: the option boxes, and the lines of the file.
    click(&mut h, "Next >>");
    h.get_by_label("Lines before the data");
    click(&mut h, "Next >>");
    h.get_by_label("Line with the names");
    h.get_by_label("Line with the units");
    h.get_by_label("Line with comments");
    // File Name Options: the file name becomes the sheet name.
    click(&mut h, "Next >>");
    click(&mut h, "Use the file name as the name of the sheet");
    // Data Columns: the first column is not X.
    click(&mut h, "Next >>");
    assert!(has_label(&h, "Delimiter"));
    click(&mut h, "The first column is X");
    // Save Filters: shown grey.
    click(&mut h, "Next >>");
    assert!(is_disabled(&h, "Save the settings as an import filter"));
    click(&mut h, "Finish");
    assert!(h.state().import_wizard.is_none(), "Finish closes the wizard");
    let book = project(&h).workbooks().next().expect("the file was imported");
    assert_eq!(book.sheet().name, "scan", "the File Name Options page named the sheet");
    assert_ne!(book.sheet().columns[0].designation, Designation::X, "the Data Columns option reached the importer");
    assert_eq!(book.sheet().columns[1].units, "mA", "the units line was found");
}

#[test]
fn the_wizard_source_page_chooses_the_filter_and_the_import_mode() {
    let mut h = start(vec![data_file("wizard-mode")]);
    show_window(&mut h, "Book1 - sample");
    let more = data_file("wizard-mode").with_file_name("more.txt");
    std::fs::write(&more, "w\tz\n1\t1\n2\t2\n3\t3\n4\t4\n").unwrap();
    h.state_mut().import_wizard = Some(ImportWizard::new(vec![more]));
    settle(&mut h);
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("New Books")).click();
    settle(&mut h);
    click(&mut h, "New Columns");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("<None>")).click();
    settle(&mut h);
    click(&mut h, "Delimited Text");
    click(&mut h, "Make a graph of each file");
    click(&mut h, "Finish");
    assert_eq!(project(&h).workbooks().count(), 1, "the data went into the active sheet");
    assert_eq!(project(&h).workbooks().next().unwrap().sheet().columns.len(), 5);
    assert_eq!(project(&h).graphs().count(), 1, "no graph was made for the new columns");
}

#[test]
fn a_path_that_is_not_a_file_disables_next_and_finish_until_it_is_valid() {
    let mut h = start(Vec::new());
    let file = data_file("wizard-badpath");
    h.state_mut().import_wizard = Some(ImportWizard::new(vec![file.clone()]));
    settle(&mut h);
    assert!(!is_disabled(&h, "Finish"));
    type_into_box(&mut h, 0, "C:/no/such/file.csv");
    assert!(has_label(&h, "This path is not a file."));
    assert!(is_disabled(&h, "Next >>") && is_disabled(&h, "Finish"), "the old file must not be imported");
    type_into_box(&mut h, 0, &file.display().to_string());
    assert!(!is_disabled(&h, "Next >>") && !is_disabled(&h, "Finish"));
    assert!(!has_label(&h, "This path is not a file."));
}

#[test]
fn the_wizard_lists_the_files_previews_the_chosen_one_and_removes_one() {
    let mut h = start(Vec::new());
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-wizard-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (a, b) = (dir.join("a.csv"), dir.join("b.csv"));
    std::fs::write(&a, "p,q\n1,2\n3,4\n").unwrap();
    std::fs::write(&b, "r,s\n5,6\n7,8\n").unwrap();
    h.state_mut().import_wizard = Some(ImportWizard::new(vec![a, b]));
    settle(&mut h);
    assert!(has_label(&h, "a.csv") && has_label(&h, "b.csv") && has_label(&h, "Add files..."));
    click(&mut h, "b.csv");
    click(&mut h, "Next >>");
    assert!(h.query_by_label_contains("r,s").is_some(), "the lines of the chosen file are shown");
    assert!(h.query_by_label_contains("p,q").is_none());
    click(&mut h, "<< Back");
    right_click(&mut h, "b.csv");
    click(&mut h, "Remove from the list");
    assert!(!has_label(&h, "b.csv"));
    click(&mut h, "Finish");
    assert_eq!(project(&h).workbooks().count(), 1, "only the file left in the list was imported");
}

#[test]
fn ctrl_3_with_the_wizard_open_keeps_it_and_its_file() {
    let mut h = start(Vec::new());
    let file = data_file("wizard-ctrl3");
    h.state_mut().import_wizard = Some(ImportWizard::new(vec![file]));
    settle(&mut h);
    click(&mut h, "Next >>");
    ctrl(&mut h, egui::Key::Num3);
    assert_eq!(h.state().import_wizard.as_ref().map(|w| w.page()), Some(WizardPage::HeaderLines));
    assert!(!is_disabled(&h, "Finish"), "the file is still chosen");
}

// ----- worksheet keyboard navigation and drag selection -----------------------------------------------

fn key(h: &mut Harness<'static, SheafApp>, key: egui::Key) {
    h.key_press(key);
    settle(h);
}

fn key_with(h: &mut Harness<'static, SheafApp>, modifiers: egui::Modifiers, key: egui::Key) {
    h.key_press_modifiers(modifiers, key);
    settle(h);
}

fn cells(h: &Harness<'static, SheafApp>) -> Sel {
    book_ui(h).sel.clone()
}

fn at(col: usize, row: usize) -> Sel {
    Sel::Cells { anchor: (col, row), end: (col, row) }
}

#[test]
fn arrow_keys_move_the_active_cell_and_shift_extends_the_block() {
    let mut h = start(vec![data_file("arrows")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "4.5");
    assert_eq!(cells(&h), at(1, 1));
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(cells(&h), at(1, 2));
    key(&mut h, egui::Key::ArrowRight);
    assert_eq!(cells(&h), at(2, 2));
    key(&mut h, egui::Key::ArrowRight);
    assert_eq!(cells(&h), at(2, 2), "clamped at the last column");
    for _ in 0..4 {
        key(&mut h, egui::Key::ArrowUp);
    }
    assert_eq!(cells(&h), at(2, 0), "clamped at row 0");
    key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
    assert_eq!(cells(&h), Sel::Cells { anchor: (2, 0), end: (1, 1) });
    assert_eq!(book_ui(&h).selected.len(), 2);
    // Down goes on into the empty rows below the data.
    click_text(&mut h, "8.5");
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(cells(&h), at(1, 4));
}

#[test]
fn ctrl_arrows_home_and_end_jump_to_the_edges() {
    let mut h = start(vec![data_file("jumps")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "4.5");
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::ArrowDown);
    assert_eq!(cells(&h), at(1, 3), "the last data row");
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::ArrowUp);
    assert_eq!(cells(&h), at(1, 0));
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::ArrowRight);
    assert_eq!(cells(&h), at(2, 0));
    key(&mut h, egui::Key::Home);
    assert_eq!(cells(&h), at(0, 0));
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::End);
    assert_eq!(cells(&h), at(2, 3));
    key_with(&mut h, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Home);
    assert_eq!(cells(&h), Sel::Cells { anchor: (2, 3), end: (0, 0) });
}

#[test]
fn typing_starts_editing_and_enter_stores_the_value_and_moves_down() {
    let mut h = start(vec![data_file("typing")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "4.5");
    h.event(egui::Event::Text("9".to_string()));
    settle(&mut h);
    assert_eq!(book_ui(&h).editing.as_ref().map(|e| (e.1, e.2.clone())), Some((1, "9".to_string())));
    h.event(egui::Event::Text(".5".to_string()));
    settle(&mut h);
    key(&mut h, egui::Key::Enter);
    assert_eq!(column(&h, "B"), [2.5, 9.5, 6.5, 8.5]);
    assert!(book_ui(&h).editing.is_none());
    assert_eq!(cells(&h), at(1, 2));
    assert_eq!(h.state().doc.undo_label(), Some("Edit cell"));
    // Tab stores and moves right; Escape gives up.
    h.event(egui::Event::Text("1".to_string()));
    settle(&mut h);
    key(&mut h, egui::Key::Tab);
    assert_eq!(column(&h, "B"), [2.5, 9.5, 1.0, 8.5]);
    assert_eq!(cells(&h), at(2, 2));
    h.event(egui::Event::Text("5".to_string()));
    settle(&mut h);
    key(&mut h, egui::Key::Escape);
    assert!(book_ui(&h).editing.is_none());
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5]);
    // F2 edits keeping the content.
    key(&mut h, egui::Key::F2);
    assert_eq!(book_ui(&h).editing.as_ref().map(|e| e.2.clone()), Some("10.5".to_string()));
}

#[test]
fn tab_moves_right_shift_tab_left_and_enter_down() {
    let mut h = start(vec![data_file("tab")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "2.5");
    key(&mut h, egui::Key::Tab);
    assert_eq!(cells(&h), at(2, 0));
    key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::Tab);
    assert_eq!(cells(&h), at(1, 0));
    key(&mut h, egui::Key::Enter);
    assert_eq!(cells(&h), at(1, 1));
}

#[test]
fn dragging_over_column_heads_selects_the_columns_in_between() {
    let mut h = start(vec![data_file("headdrag")]);
    show_window(&mut h, "Book1 - sample");
    let (a, b, c) = (column_id(&h, "A"), column_id(&h, "B"), column_id(&h, "C"));
    let (pa, pc) = (text_pos(&h, "A(X1)"), text_pos(&h, "C(Y1)"));
    drag(&mut h, pa, pc);
    assert_eq!(cells(&h), Sel::Columns);
    assert_eq!(book_ui(&h).selected, vec![a, b, c]);
    // Right to left gives the same columns, in sheet order.
    click_text(&mut h, "A(X1)");
    let pb = text_pos(&h, "B(Y1)");
    drag(&mut h, pc, pb);
    assert_eq!(book_ui(&h).selected, vec![b, c]);
    // A plain click on a head still selects just that column.
    click_text(&mut h, "A(X1)");
    assert_eq!(book_ui(&h).selected, vec![a]);
}

#[test]
fn dragging_over_row_numbers_selects_the_rows_in_between() {
    let mut h = start(vec![data_file("rowdrag")]);
    show_window(&mut h, "Book1 - sample");
    let x = text_pos(&h, "Long Name").x;
    let (y1, y3) = (text_pos(&h, "2.5").y, text_pos(&h, "6.5").y);
    drag(&mut h, egui::pos2(x, y1), egui::pos2(x, y3));
    assert_eq!(cells(&h), Sel::Rows { anchor: 0, end: 2 });
}

// ----- menu items that were greyed although the code existed (brief T01) -----

/// Selects the columns B and C of the first workbook with the mouse and returns their ids.
fn select_b_and_c(h: &mut Harness<'static, SheafApp>) -> (ColumnId, ColumnId) {
    show_window(h, "Book1 - sample");
    click_text(h, "B(Y1)");
    click_text_with(h, "C(Y1)", egui::Modifiers::COMMAND);
    settle(h);
    let (b, c) = (column_id(h, "B"), column_id(h, "C"));
    assert_eq!(book_ui(h).selected, vec![b, c]);
    (b, c)
}

#[test]
fn multi_panel_items_make_one_panel_per_selected_column() {
    for item in ["Vertical 2 Panel", "Horizontal 2 Panel", "4 Panel", "9 Panel"] {
        let mut h = start(vec![data_file("panels")]);
        let (b, c) = select_b_and_c(&mut h);
        menu_path(&mut h, &["Plot", "Multi-Panel", item]);
        assert_eq!(project(&h).graphs().count(), 2, "{item} makes a graph");
        let g = project(&h).graphs().last().unwrap();
        // Two columns fill two of the cells; each panel holds one plot.
        assert_eq!(g.layers.len(), 2, "{item}");
        assert_eq!(g.layers[0].plots.iter().map(|p| p.y).collect::<Vec<_>>(), vec![b]);
        assert_eq!(g.layers[1].plots.iter().map(|p| p.y).collect::<Vec<_>>(), vec![c]);
    }
    // The layouts follow Origin: 2 x 1 stacks the panels, 1 x 2 puts them side by side.
    let mut h = start(vec![data_file("panels2")]);
    select_b_and_c(&mut h);
    menu_path(&mut h, &["Plot", "Multi-Panel", "Vertical 2 Panel"]);
    let f = |h: &Harness<'static, SheafApp>, i: usize| project(h).graphs().last().unwrap().layers[i].frame;
    assert!((f(&h, 0).left - f(&h, 1).left).abs() < 1e-9 && f(&h, 0).top != f(&h, 1).top, "stacked");
    select_b_and_c(&mut h);
    menu_path(&mut h, &["Plot", "Multi-Panel", "Horizontal 2 Panel"]);
    assert!((f(&h, 0).top - f(&h, 1).top).abs() < 1e-9 && f(&h, 0).left != f(&h, 1).left, "side by side");
}

#[test]
fn y_error_plots_a_scatter_with_the_error_bars_of_the_error_column() {
    let mut h = start(vec![data_file("yerr")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let err = h
        .state_mut()
        .doc
        .edit("Add error column", |p| {
            p.add_column(book, 0, Designation::YError, sheaf_core::ColumnData::numeric(vec![0.5; 4]))
        })
        .unwrap();
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Plot", "Symbol", "Y Error"]);
    assert_eq!(project(&h).graphs().count(), 2);
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers[0].plots.len(), 1);
    assert_eq!(g.layers[0].plots[0].y_error, Some(err), "the error column that follows the Y column");
    assert!(g.layers[0].plots[0].kind.has_symbols() && !g.layers[0].plots[0].kind.has_line());
}

#[test]
fn y_error_without_an_error_column_still_plots_but_says_so() {
    let mut h = start(vec![data_file("yerr2")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Plot", "Symbol", "Y Error"]);
    assert_eq!(project(&h).graphs().count(), 2);
    assert!(h.state().status.contains("without error bars"), "{}", h.state().status);
}

#[test]
fn add_error_bars_gives_the_plots_the_error_column_of_their_sheet() {
    let mut h = start(vec![data_file("adderr")]);
    let (book, graph) = window_ids(&h);
    let err = h
        .state_mut()
        .doc
        .edit("Add error column", |p| {
            p.add_column(book, 0, Designation::YError, sheaf_core::ColumnData::numeric(vec![0.5; 4]))
        })
        .unwrap();
    settle(&mut h);
    // Column C is the last Y column, so only its plot has an error column after it.
    assert!(project(&h).graph(graph).unwrap().layers[0].plots.iter().all(|p| p.y_error.is_none()));
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    let plots = &project(&h).graph(graph).unwrap().layers[0].plots;
    assert_eq!(plots[0].y_error, None, "B has no error column");
    assert_eq!(plots[1].y_error, Some(err));
    assert_eq!(h.state().doc.undo_label(), Some("Add error bars"));
    // Nothing left to add: no new undo step, and a message.
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    assert_eq!(h.state().doc.undo_label(), Some("Add error bars"));
    assert!(h.state().status.contains("No error bars added"));
}

#[test]
fn update_legend_keeps_custom_text_and_adds_the_plots_it_lacks() {
    let mut h = start(vec![data_file("updleg")]);
    h.state_mut().doc.apply("Legend text", |p| {
        let g = p.graphs().next().unwrap().id;
        p.graph_mut(g).unwrap().layers[0].legend.text = Some("\\l(1) first\n\\l(9) %(9)".into());
    });
    settle(&mut h);
    menu(&mut h, "Graph", "Legend");
    menu_path(&mut h, &["Update Legend"]);
    // Plot 1 keeps its text, plot 9 does not exist, plot 2 is added.
    assert_eq!(graph(&h).layers[0].legend.text.as_deref(), Some("\\l(1) first\n\\l(2) %(2)"));
    assert_eq!(h.state().doc.undo_label(), Some("Update legend"));
    // From the legend's right-click menu: text that equals the automatic one is not stored.
    h.state_mut().doc.apply("Legend text", |p| {
        let g = p.graphs().next().unwrap().id;
        p.graph_mut(g).unwrap().layers[0].legend.text = Some("\\l(1) %(1)".into());
    });
    settle(&mut h);
    let at = on(&h, 0, ObjectKind::Legend);
    right_click_at(&mut h, at);
    menu_path(&mut h, &["Legend", "Update Legend"]);
    assert_eq!(graph(&h).layers[0].legend.text, None, "the updated text is the automatic one");
}

#[test]
fn window_refresh_recalculates_the_waiting_columns_of_the_workbook() {
    let mut h = start(vec![data_file("refresh")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let (b, c) = {
        let s = project(&h).workbooks().next().unwrap().sheet();
        (s.column_by_name("B").unwrap().id, s.column_by_name("C").unwrap().id)
    };
    h.state_mut()
        .doc
        .edit("Formula", |p| p.set_column_values_with(c, "col(B) * 2", None, sheaf_core::RecalcMode::Manual))
        .unwrap();
    h.state_mut().doc.apply("Change B", |p| {
        p.column_mut(b).unwrap().set_cell_text(0, "10");
    });
    settle(&mut h);
    let state = |h: &Harness<'static, SheafApp>| project(h).column(c).unwrap().formula_state;
    assert_eq!(state(&h), sheaf_core::FormulaState::Pending, "Manual: waits for the user");
    show_window(&mut h, "Book1 - sample");
    assert_eq!(h.state().active, Some(book));
    // F5 is the key of Window > Refresh.
    h.key_press(egui::Key::F5);
    settle(&mut h);
    assert_eq!(state(&h), sheaf_core::FormulaState::Current);
    assert_eq!(project(&h).numeric(c).unwrap()[0], 20.0);
    assert_eq!(h.state().doc.undo_label(), Some("Refresh"));
    // From the menu, with nothing waiting: no new undo step.
    menu_path(&mut h, &["Window", "Refresh"]);
    assert_eq!(h.state().doc.undo_label(), Some("Refresh"));
    assert!(h.state().status.contains("Refreshed"));
}

#[test]
fn sort_range_moves_the_selected_columns_together_and_sort_columns_sorts_each_alone() {
    let mut h = start(vec![data_file("sortrange")]);
    let (b, c) = select_b_and_c(&mut h);
    // B rises with t and C falls: sorting the range by B descending reverses both together.
    menu_path(&mut h, &["Worksheet", "Sort Range", "Descending"]);
    assert_eq!(project(&h).numeric(b).unwrap(), [8.5, 6.5, 4.5, 2.5]);
    assert_eq!(project(&h).numeric(c).unwrap(), [0.5, 10.5, 20.5, 30.5]);
    assert_eq!(column(&h, "A"), [1.0, 2.0, 3.0, 4.0], "column A is not in the range");
    assert_eq!(h.state().doc.undo_label(), Some("Sort range"));
    // Each column on its own: B ascending again, C stays as it is.
    menu_path(&mut h, &["Worksheet", "Sort Columns", "Ascending"]);
    assert_eq!(project(&h).numeric(b).unwrap(), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(project(&h).numeric(c).unwrap(), [0.5, 10.5, 20.5, 30.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Sort columns"));
    menu_path(&mut h, &["Worksheet", "Sort Columns", "Descending"]);
    assert_eq!(project(&h).numeric(c).unwrap(), [30.5, 20.5, 10.5, 0.5]);
}

#[test]
fn set_values_apply_waits_for_a_change_and_recalculate_defaults_to_manual() {
    let mut h = start(vec![data_file("setvals")]);
    show_window(&mut h, "Book1 - sample");
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Set Column Values...");
    // Apply is grey until something changes; the drop-down opens on Manual.
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    assert!(h.state().set_values.is_some());
    h.get_by_value("Manual").click();
    settle(&mut h);
    h.get_all_by_label("Auto").last().unwrap().click();
    settle(&mut h);
    assert!(!h.get_by_label("Apply").accesskit_node().is_disabled());
    h.get_by_value("Auto").click();
    settle(&mut h);
    h.get_all_by_label("Manual").last().unwrap().click();
    settle(&mut h);
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());

    // Typing a formula enables Apply; Apply keeps the window open and then goes grey again.
    h.state_mut().set_values.as_mut().unwrap().set_formula("col(B)*2");
    settle(&mut h);
    assert!(!h.get_by_label("Apply").accesskit_node().is_disabled());
    click(&mut h, "Apply");
    assert_eq!(column(&h, "C"), [5.0, 9.0, 13.0, 17.0]);
    assert!(h.state().set_values.is_some());
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().column_by_name("C").unwrap().recalc, sheaf_core::RecalcMode::Manual);
}

#[test]
fn set_values_row_range_limits_the_rows_that_are_filled() {
    let mut h = start(vec![data_file("setrows")]);
    show_window(&mut h, "Book1 - sample");
    right_click_text(&mut h, "C(Y1)");
    click_containing(&mut h, "Set Column Values...");
    h.event(egui::Event::Text("100".to_string()));
    settle(&mut h);
    // Both ends are <auto> until typed into: rows 2 to 3 only.
    h.state_mut().set_values.as_mut().unwrap().set_rows("2", "3");
    settle(&mut h);
    click(&mut h, "OK");
    let c = column(&h, "C");
    assert_eq!(c.len(), 4);
    assert_ne!(c[0], 100.0);
    assert_eq!(&c[1..3], [100.0, 100.0]);
    assert!(c[3] != 100.0);
}

// ----- T02: menu labels, keys and context menus as in Origin -----

#[test]
fn ctrl_g_is_go_to_in_a_workbook_and_export_graphs_in_a_graph() {
    let mut h = start(vec![data_file("ctrl-g")]);
    show_window(&mut h, "Book1 - sample");
    ctrl(&mut h, egui::Key::G);
    assert!(h.state().go_to.is_some(), "Go To opened in the workbook");
    assert!(h.state().export.is_none());
    let book = project(&h).workbooks().next().unwrap().id;
    // The dialog's own parsing is tested in dialogs/go_to.rs; here the outcome moves the cursor.
    h.state_mut().go_to = None;
    let sheet = project(&h).workbook(book).unwrap().sheets[0].clone();
    h.state_mut().books.get_mut(&book).unwrap().go_to(&sheet, 1, 2);
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 2), end: (1, 2) });
    show_window(&mut h, "Graph1 - sample");
    ctrl(&mut h, egui::Key::G);
    assert!(h.state().export.is_some(), "Export Graphs opened in the graph");
    assert!(h.state().go_to.is_none());
}

#[test]
fn go_to_window_moves_the_cursor_to_the_typed_cell() {
    let mut h = start(vec![data_file("go-to")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Edit", "Go to..."]);
    assert!(h.state().go_to.is_some());
    h.event(egui::Event::Text("3".to_string()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().go_to.is_none(), "Enter closed the window");
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (0, 2), end: (0, 2) });
}

#[test]
fn ctrl_shift_c_copies_with_the_label_rows() {
    let mut h = start(vec![data_file("copy-labels")]);
    show_window(&mut h, "Book1 - sample");
    select_block(&mut h, "2.5", "20.5");
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::C);
    settle(&mut h);
    assert!(h.state().copied.starts_with("up\tdown\n"), "{:?}", h.state().copied);
}

#[test]
fn the_file_menu_has_recent_graphs_and_no_reimport_only_in_a_graph() {
    let mut h = start(vec![data_file("file-menus")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["File"]);
    assert!(h.query_by_label_contains("Recent Books").is_some());
    assert!(h.query_by_label_contains("Recent Graphs").is_none());
    assert!(h.query_by_label_contains("Open Sample Projects").is_some());
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    show_window(&mut h, "Graph1 - sample");
    menu_path(&mut h, &["File"]);
    assert!(h.query_by_label_contains("Recent Graphs").is_some());
    assert!(h.query_by_label_contains("Export Graphs...").is_some());
}

#[test]
fn the_import_items_differ_in_what_the_chooser_takes_and_csv_presets_the_comma() {
    use crate::actions::ImportPick;
    let mut h = start(vec![data_file("import-picks")]);
    let file = data_file("import-picks-file");
    let ctx = h.ctx.clone();
    h.state_mut().start_import(Some(vec![file.clone()]), ImportPick::Csv);
    assert_eq!(h.state().import.as_ref().unwrap().delimiter(), sheaf_io::ascii::Delimiter::Comma);
    h.state_mut().import = None;
    h.state_mut().start_import(Some(vec![file]), ImportPick::Multiple);
    assert_ne!(h.state().import.as_ref().unwrap().delimiter(), sheaf_io::ascii::Delimiter::Comma);
    let _ = ctx;
}

#[test]
fn the_explorer_keys_f2_delete_and_ctrl_a_work_on_the_highlighted_window() {
    let mut h = start(vec![data_file("explorer-keys")]);
    h.get_by_label("Book1 - sample").click();
    settle(&mut h);
    assert!(h.state().panels.explorer.focus, "a click in the explorer gives it the keys");
    h.key_press(egui::Key::F2);
    settle(&mut h);
    assert!(h.state().rename.is_some(), "F2 renames the window");
    h.state_mut().rename = None;
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    settle(&mut h);
    assert!(h.state().panels.explorer.all, "Ctrl+A highlights every window");
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!(project(&h).windows.len(), 0, "Delete removed them all");
    h.state_mut().doc.undo();
    h.state_mut().doc.undo();
    assert_eq!(project(&h).windows.len(), 2, "each deletion can be undone");
}

#[test]
fn a_folder_can_be_renamed_and_deleted_with_its_windows() {
    let mut h = start(vec![data_file("folders")]);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewFolder(Vec::new()), &ctx);
    h.state_mut().apply(Action::RenameFolder(vec![0]), &ctx);
    h.state_mut().rename.as_mut().unwrap().long_name = "Samples".to_string();
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(project(&h).root.folders[0].name, "Samples");
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().apply(Action::MoveWindow { window: book, folder: vec![0] }, &ctx);
    h.state_mut().apply(Action::DeleteFolder(vec![0]), &ctx);
    assert!(project(&h).root.folders.is_empty());
    assert!(project(&h).workbook(book).is_none(), "the window in the folder went with it");
    menu(&mut h, "Edit", "Undo Delete folder");
    assert_eq!(project(&h).root.folders.len(), 1);
}

#[test]
fn the_main_title_and_the_status_reference_follow_origins_pattern() {
    use std::path::Path;
    assert_eq!(main_title(None, false, None), "Sheaf - UNTITLED");
    assert_eq!(main_title(Some(Path::new("C:/a/p.opj")), true, None), "Sheaf - C:/a/p.opj *");
    assert_eq!(main_title(Some(Path::new("p.opj")), true, Some("Book1".into())), "Sheaf - p.opj * - [Book1 *]");
    let mut h = start(vec![data_file("status-ref")]);
    show_window(&mut h, "Book1 - sample");
    assert_eq!(h.state().active_reference(), "1: [Book1]sample!");
    select_block(&mut h, "2.5", "20.5");
    assert_eq!(h.state().active_reference(), "1: [Book1]sample!B:C[1:2]");
}

#[test]
fn the_layer_icon_menu_describes_plots_as_origin_does_and_has_the_hint_line() {
    let h = start(vec![data_file("layer-menu")]);
    let graph = project(&h).graphs().next().unwrap();
    let plot = graph.layers[0].plots[0].clone();
    let line = crate::graph_view::plot_source_line(project(&h), &plot);
    assert!(line.starts_with("g1 [Book1]sample! "), "{line}");
    assert!(line.contains("(X), ") && line.contains("(Y) [1*:4*]"), "{line}");
    let entries = menus::layer_icon_menu(0, 1, &[line], None);
    let labels: Vec<String> = entries
        .iter()
        .filter_map(|e| if let menus::Entry::Item(i) = e { Some(i.label.clone()) } else { None })
        .collect();
    assert!(labels.iter().any(|l| l == "Double click to open Plot Setup"));
}

#[test]
fn typing_with_a_block_selected_edits_its_active_cell_and_enter_and_tab_move_inside_the_block() {
    let mut h = start(vec![data_file("blocktype")]);
    show_window(&mut h, "Book1 - sample");
    grid_text(&h, "4.5").click();
    settle(&mut h);
    // The block of the columns "up" and "down" over the first two rows.
    let ws = sheet(&h).clone();
    h.state_mut().books.values_mut().next().unwrap().select_cells(&ws, (1, 0), (2, 1));
    settle(&mut h);
    h.event(egui::Event::Text("9".to_string()));
    settle(&mut h);
    let editing = book_ui(&h).editing.clone().expect("typing starts editing");
    assert_eq!((editing.0, editing.1, editing.2.as_str()), (ws.columns[1].id, 0, "9"));
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 0), end: (2, 1) }, "the block stays selected");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(column(&h, "B"), [9.0, 4.5, 6.5, 8.5]);
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 0), end: (2, 1) });
    assert_eq!(book_ui(&h).active_cell(), Some((1, 1)));
    h.event(egui::Event::Text("8".to_string()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(column(&h, "B"), [9.0, 8.0, 6.5, 8.5]);
    // Past the bottom of the block: the top of the next column of the block.
    assert_eq!(book_ui(&h).active_cell(), Some((2, 0)));
    h.event(egui::Event::Text("7".to_string()));
    settle(&mut h);
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert_eq!(column(&h, "C")[0], 7.0);
    // Tab moved right inside the block: past the last column, to the start of the next row.
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 0), end: (2, 1) });
    assert_eq!(book_ui(&h).active_cell(), Some((1, 1)));
}

#[test]
fn exchange_axes_swaps_the_columns_and_the_axis_settings_in_one_undo_step() {
    let mut h = start(vec![data_file("axis-exchange")]);
    let (_, g) = window_ids(&h);
    h.state_mut().doc.apply("Test", |p| {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.x_axis.scale = ScaleKind::Log10;
    });
    let before = graph(&h).layers[0].clone();
    let (x, y) = (before.plots[0].x.unwrap(), before.plots[0].y);
    menu(&mut h, "Graph", "Exchange X-Y Axes");
    let l = &graph(&h).layers[0];
    assert_eq!((l.plots[0].x, l.plots[0].y), (Some(y), x));
    assert_eq!(l.y_axis.scale, ScaleKind::Log10, "the axis settings went with their data");
    assert_eq!(l.x_axis.scale, ScaleKind::Linear);
    assert_eq!(h.state().doc.undo_label(), Some("Exchange X-Y axes"));
    menu(&mut h, "Edit", "Undo Exchange X-Y axes");
    assert_eq!(graph(&h).layers[0], before);
}

#[test]
fn the_axis_dialog_lists_the_edges_and_has_working_breaks_and_special_ticks_tabs() {
    let mut h = start(vec![data_file("axis-pages")]);
    open_props(&mut h, 0, PropsTab::XAxis, None);
    // Scale and Breaks name the axes Horizontal and Vertical; the other tabs name the four edges.
    assert!(h.query_by_label("Horizontal").is_some());
    click(&mut h, "Title");
    for edge in ["Bottom", "Top", "Left", "Right"] {
        assert!(h.query_by_label(edge).is_some(), "{edge}");
    }
    click(&mut h, "Breaks");
    assert!(h.query_by_label("Horizontal").is_some() && h.query_by_label("Bottom").is_none());
    h.get_by_label("Show Break").click();
    settle(&mut h);
    assert!(graph(&h).layers[0].x_axis.axis_break.is_some());
    click(&mut h, "Special Ticks");
    click(&mut h, "Add Tick");
    assert_eq!(graph(&h).layers[0].x_axis.special_ticks.len(), 1);
}

#[test]
fn insert_and_delete_rows_and_columns_from_the_selection_and_undo() {
    use crate::actions::CellOp;
    let mut h = start(vec![data_file("insdel")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    let sheet = project(&h).workbooks().next().unwrap().sheet().clone();

    // A block of cells in the second column: only that column moves.
    h.state_mut().books.entry(book).or_default().select_cells(&sheet, (1, 1), (1, 2));
    h.state_mut().apply(Action::Cells { book, op: CellOp::Insert }, &ctx);
    settle(&mut h);
    let up = column(&h, "B");
    assert_eq!(up.len(), 6);
    assert!(up[1].is_nan() && up[2].is_nan());
    assert_eq!(column(&h, "A").len(), 4);
    assert_eq!(h.state().doc.undo_label(), Some("Insert rows"));
    h.state_mut().apply(Action::Cells { book, op: CellOp::Delete }, &ctx);
    settle(&mut h);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    h.state_mut().apply(Action::Undo, &ctx);
    assert_eq!(column(&h, "B").len(), 6);

    // Whole columns: Insert adds a column before the selection, Delete removes it.
    h.state_mut().apply(Action::Undo, &ctx);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    h.state_mut().books.entry(book).or_default().clear_selection();
    h.state_mut().books.entry(book).or_default().sel = crate::grid::Sel::Columns;
    let b = project(&h).workbooks().next().unwrap().sheet().columns[1].id;
    h.state_mut().books.entry(book).or_default().selected = vec![b];
    h.state_mut().apply(Action::Cells { book, op: CellOp::Insert }, &ctx);
    settle(&mut h);
    let names: Vec<String> =
        project(&h).workbooks().next().unwrap().sheet().columns.iter().map(|c| c.short_name.clone()).collect();
    assert_eq!(names, ["A", "D", "B", "C"]);
    assert_eq!(h.state().doc.undo_label(), Some("Insert columns"));
    h.state_mut().apply(Action::Cells { book, op: CellOp::Delete }, &ctx);
    settle(&mut h);
    let names: Vec<String> =
        project(&h).workbooks().next().unwrap().sheet().columns.iter().map(|c| c.short_name.clone()).collect();
    assert_eq!(names, ["A", "B", "C"], "the inserted column was the selection");
}

#[test]
fn columns_move_hide_and_unhide_and_each_step_undoes() {
    use sheaf_core::sheetops::MoveTo;
    let mut h = start(vec![data_file("movehide")]);
    let ctx = h.ctx.clone();
    let ids: Vec<ColumnId> = project(&h).workbooks().next().unwrap().sheet().columns.iter().map(|c| c.id).collect();
    let names = |h: &Harness<'static, SheafApp>| -> String {
        project(h).workbooks().next().unwrap().sheet().columns.iter().map(|c| c.short_name.clone()).collect()
    };
    assert_eq!(names(&h), "ABC");
    h.state_mut().apply(Action::MoveColumns { columns: vec![ids[2]], to: MoveTo::First }, &ctx);
    assert_eq!(names(&h), "CAB");
    assert_eq!(h.state().doc.undo_label(), Some("Move columns"));
    h.state_mut().apply(Action::MoveColumns { columns: vec![ids[2]], to: MoveTo::Right }, &ctx);
    assert_eq!(names(&h), "ACB");
    h.state_mut().apply(Action::Undo, &ctx);
    h.state_mut().apply(Action::Undo, &ctx);
    assert_eq!(names(&h), "ABC");

    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().apply(Action::HideColumns(vec![ids[1]]), &ctx);
    settle(&mut h);
    assert!(project(&h).column(ids[1]).unwrap().hidden);
    assert!(h.query_by_label_contains("B(").is_none() && h.query_by_label_contains("C(").is_some(), "B has no head");
    h.state_mut().apply(Action::UnhideColumns(book), &ctx);
    settle(&mut h);
    assert!(!project(&h).column(ids[1]).unwrap().hidden);
    assert!(h.query_by_label_contains("B(").is_some());
    h.state_mut().apply(Action::Undo, &ctx);
    assert!(project(&h).column(ids[1]).unwrap().hidden, "undo brings the hiding back");
}

#[test]
fn reverse_order_and_the_custom_sort_windows_work_and_undo() {
    let mut h = start(vec![data_file("customsort")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    let c = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().id;
    h.state_mut().apply(Action::ReverseColumns(vec![c]), &ctx);
    assert_eq!(column(&h, "C"), [0.5, 10.5, 20.5, 30.5]);
    assert_eq!(column(&h, "A"), [1.0, 2.0, 3.0, 4.0], "the other columns stay");
    assert_eq!(h.state().doc.undo_label(), Some("Reverse order"));
    h.state_mut().apply(Action::Undo, &ctx);
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5]);

    // Sort Worksheet > Custom...: the window opens with the first column as the key; OK sorts.
    h.state_mut().apply(Action::SortCustom { book, columns: Vec::new() }, &ctx);
    settle(&mut h);
    assert!(h.state().sort.is_some());
    click(&mut h, "Descending");
    click(&mut h, "OK");
    assert!(h.state().sort.is_none());
    assert_eq!(column(&h, "A"), [4.0, 3.0, 2.0, 1.0]);
    assert_eq!(column(&h, "C"), [0.5, 10.5, 20.5, 30.5], "rows stay together");
    assert_eq!(h.state().doc.undo_label(), Some("Sort (nested)"));

    // Sort Columns by Label (the long names are t, alpha, down).
    let b = project(&h).workbooks().next().unwrap().sheet().column_by_name("B").unwrap().id;
    h.state_mut().doc.apply("Name", |p| p.column_mut(b).unwrap().long_name = "alpha".into());
    h.state_mut().apply(Action::SortByLabel(book), &ctx);
    settle(&mut h);
    click(&mut h, "OK");
    let names: String =
        project(&h).workbooks().next().unwrap().sheet().columns.iter().map(|c| c.short_name.clone()).collect();
    assert_eq!(names, "BCA", "long names alpha, down, t");
}

#[test]
fn masking_cells_marks_them_for_analysis_and_undoes() {
    use crate::actions::CellOp;
    use sheaf_core::sheetops::MaskOp;
    let mut h = start(vec![data_file("mask")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    let sheet = project(&h).workbooks().next().unwrap().sheet().clone();
    h.state_mut().books.entry(book).or_default().select_cells(&sheet, (2, 1), (2, 2));
    h.state_mut().apply(Action::Cells { book, op: CellOp::Mask(MaskOp::Apply) }, &ctx);
    settle(&mut h);
    let c = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().clone();
    assert_eq!(c.mask.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
    let seen = c.numeric_unmasked().unwrap();
    assert!(seen[1].is_nan() && seen[2].is_nan() && seen[0] == 30.5 && seen[3] == 0.5);
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5], "the values stay in the sheet");
    h.state_mut().apply(Action::Cells { book, op: CellOp::Mask(MaskOp::Swap) }, &ctx);
    assert!(project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().mask.is_empty());
    h.state_mut().apply(Action::Undo, &ctx);
    assert_eq!(project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().mask.len(), 2);
}

#[test]
fn the_worksheet_filter_hides_rows_without_deleting_them_and_undoes() {
    let mut h = start(vec![data_file("rowfilter")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::RowFilterDialog { book, column: None }, &ctx);
    settle(&mut h);
    assert!(h.state().row_filter.is_some());
    h.event(egui::Event::Text("col(A) >= 3".to_string()));
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().row_filter.is_none());
    let sheet = project(&h).workbooks().next().unwrap().sheet();
    assert_eq!(sheet.shown_rows(4), Some(vec![2, 3]));
    assert_eq!(column(&h, "A"), [1.0, 2.0, 3.0, 4.0], "nothing was deleted");
    settle(&mut h);
    assert_eq!(h.query_all_by_label("1").count(), 0, "row 1 is not shown");
    assert_eq!(h.query_all_by_label("3").count(), 2, "row 3 is: its number and its value");
    // Disable shows everything again; undo goes back step by step.
    h.state_mut().apply(Action::ToggleRowFilter(book), &ctx);
    assert!(project(&h).workbooks().next().unwrap().sheet().shown_rows(4).is_none());
    h.state_mut().apply(Action::Undo, &ctx);
    h.state_mut().apply(Action::Undo, &ctx);
    assert!(project(&h).workbooks().next().unwrap().sheet().filter.is_none());
}

#[test]
fn transpose_and_reduce_rows_write_a_new_sheet_and_undo() {
    use crate::dialogs::reshape::ReshapeKind;
    let mut h = start(vec![data_file("reshape")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Reshape { book, columns: Vec::new(), kind: ReshapeKind::Transpose }, &ctx);
    settle(&mut h);
    assert!(h.state().reshape.is_some());
    click(&mut h, "OK");
    assert!(h.state().reshape.is_none());
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!(b.sheets.len(), 2);
    assert_eq!(b.active_sheet, 1, "the new sheet is shown");
    // Three columns of four rows give four columns of three rows; the first is the first data row.
    assert_eq!(b.sheets[1].columns.len(), 4);
    assert_eq!(b.sheets[1].columns[0].numeric().unwrap(), [1.0, 2.5, 30.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Transpose"));
    h.state_mut().apply(Action::Undo, &ctx);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);

    h.state_mut().apply(Action::SelectSheet { book, sheet: 0 }, &ctx);
    h.state_mut().apply(Action::Reshape { book, columns: Vec::new(), kind: ReshapeKind::Reduce }, &ctx);
    settle(&mut h);
    click(&mut h, "OK");
    let b = project(&h).workbooks().next().unwrap();
    assert_eq!(b.sheets[1].columns[0].numeric().unwrap(), [1.5, 3.5], "two rows to one, averaged");
}

#[test]
fn set_as_categorical_toggles_and_group_and_subject_are_designations() {
    use sheaf_core::Designation;
    let mut h = start(vec![data_file("categorical")]);
    let ctx = h.ctx.clone();
    let b = project(&h).workbooks().next().unwrap().sheet().column_by_name("B").unwrap().id;
    h.state_mut().apply(Action::SetCategorical(vec![b]), &ctx);
    assert!(project(&h).column(b).unwrap().categorical);
    assert_eq!(h.state().doc.undo_label(), Some("Set as categorical"));
    h.state_mut().apply(Action::SetCategorical(vec![b]), &ctx);
    assert!(!project(&h).column(b).unwrap().categorical, "a second use takes the mark off");
    h.state_mut().apply(Action::SetDesignation { columns: vec![b], designation: Designation::Group }, &ctx);
    assert_eq!(project(&h).column(b).unwrap().designation, Designation::Group);
    assert_eq!(Designation::Group.tag(), "G");
    assert_eq!(Designation::Subject.tag(), "S");
}

// ----- spreadsheet keys (Excel's, unbound in Origin's worksheet) ---------------------------------

#[test]
fn ctrl_a_selects_the_sheet_and_ctrl_or_shift_space_the_columns_or_rows_of_the_block() {
    let mut h = start(vec![data_file("excel-select")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "4.5");
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::A);
    assert_eq!(cells(&h), Sel::All);
    click_text(&mut h, "4.5");
    key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::ArrowRight);
    key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    // Shift+Space also arrives as a typed space; it must not start editing.
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Space);
    h.event(egui::Event::Text(" ".to_string()));
    settle(&mut h);
    assert_eq!(cells(&h), Sel::Rows { anchor: 1, end: 2 });
    assert!(book_ui(&h).editing.is_none());
    click_text(&mut h, "4.5");
    key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::ArrowRight);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Space);
    assert_eq!(cells(&h), Sel::Columns);
    assert_eq!(book_ui(&h).selected, vec![column_id(&h, "B"), column_id(&h, "C")]);
}

#[test]
fn backspace_empties_the_cell_and_starts_editing_it() {
    let mut h = start(vec![data_file("excel-backspace")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "4.5");
    key(&mut h, egui::Key::Backspace);
    assert_eq!(book_ui(&h).editing.as_ref().map(|e| (e.1, e.2.clone())), Some((1, String::new())));
    key(&mut h, egui::Key::Enter);
    let b = column(&h, "B");
    assert!(b[1].is_nan(), "the cell was emptied: {b:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Edit cell"));
}

#[test]
fn ctrl_enter_stores_the_typed_value_in_every_cell_of_the_block_in_one_undo_step() {
    let mut h = start(vec![data_file("excel-fill")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "2.5");
    key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::ArrowRight);
    key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    h.event(egui::Event::Text("7".to_string()));
    settle(&mut h);
    assert!(book_ui(&h).editing.is_some());
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Enter);
    assert!(book_ui(&h).editing.is_none());
    assert_eq!(column(&h, "B"), [7.0, 7.0, 6.5, 8.5]);
    assert_eq!(column(&h, "C"), [7.0, 7.0, 10.5, 0.5]);
    assert_eq!(cells(&h), Sel::Cells { anchor: (1, 0), end: (2, 1) }, "the block stays selected");
    assert_eq!(h.state().doc.undo_label(), Some("Fill cells"));
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(column(&h, "C"), [30.5, 20.5, 10.5, 0.5]);
}

#[test]
fn ctrl_plus_inserts_and_ctrl_minus_deletes_at_the_selection() {
    let mut h = start(vec![data_file("excel-insert")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "4.5");
    key_with(&mut h, egui::Modifiers::SHIFT, egui::Key::Space);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Minus);
    assert_eq!(column(&h, "B"), [2.5, 6.5, 8.5], "row 2 was deleted");
    assert_eq!(column(&h, "A"), [1.0, 3.0, 4.0]);
    // Ctrl++ is Ctrl+Shift+= on most keyboards.
    key_with(&mut h, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Equals);
    assert_eq!(sheet(&h).column_by_name("B").unwrap().len(), 4, "a blank row was inserted");
    assert!(column(&h, "B")[1].is_nan());
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Plus);
    assert_eq!(sheet(&h).column_by_name("B").unwrap().len(), 5);
}

#[test]
fn ctrl_page_down_and_up_step_through_the_sheets() {
    let mut h = start(vec![data_file("excel-sheets")]);
    show_window(&mut h, "Book1 - sample");
    let (book, _) = window_ids(&h);
    h.state_mut()
        .doc
        .apply("Test", |p| p.workbook_mut(book).unwrap().sheets.push(sheaf_core::Worksheet::new("Sheet2")));
    settle(&mut h);
    click_text(&mut h, "4.5");
    let active = |h: &Harness<'static, SheafApp>| project(h).workbooks().next().unwrap().active_sheet;
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::PageDown);
    assert_eq!(active(&h), 1);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::PageDown);
    assert_eq!(active(&h), 1, "stays on the last sheet");
    // The second sheet is empty; the keys still leave it.
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::PageUp);
    assert_eq!(active(&h), 0);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::PageUp);
    assert_eq!(active(&h), 0, "stays on the first sheet");
}

#[test]
fn clear_worksheet_empties_every_column_in_one_undo_step() {
    let mut h = start(vec![data_file("clearsheet")]);
    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Worksheet", "Clear Worksheet..."]);
    assert!(column(&h, "A").is_empty() && column(&h, "C").is_empty());
    assert_eq!(project(&h).workbooks().next().unwrap().sheet().columns.len(), 3, "the columns stay");
    assert_eq!(h.state().doc.undo_label(), Some("Clear worksheet"));
    menu(&mut h, "Edit", "Undo Clear worksheet");
    assert_eq!(column(&h, "A"), [1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn plot_specialized_polar_makes_a_polar_graph_of_the_selected_columns() {
    use sheaf_core::graph::RadiusColumn;
    let mut h = start(vec![data_file("polar-menu")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "C(Y1)");
    let c = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().id;
    let before = project(&h).graphs().count();
    menu_path(&mut h, &["Plot", "Specialized", "Polar theta(X) r(Y)"]);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 1);
    let layer = &graphs[before].layers[0];
    assert_eq!(layer.coords.polar().map(|p| p.radius), Some(RadiusColumn::Y));
    assert_eq!(layer.plots.len(), 1);
    assert_eq!(layer.plots[0].y, c);

    // Back in the worksheet, the other item.
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Plot", "Specialized", "Polar r(X) theta(Y)"]);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 2);
    assert_eq!(graphs[before + 1].layers[0].coords.polar().map(|p| p.radius), Some(RadiusColumn::X));
}

#[test]
fn the_polar_button_of_the_2d_graphs_bar_makes_a_polar_graph() {
    let mut h = start(vec![data_file("polar-button")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "C(Y1)");
    let before = project(&h).graphs().count();
    bar_button!(h, "2D Graphs", "Polar theta(X) r(Y)").click();
    settle(&mut h);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 1);
    assert!(graphs[before].layers[0].coords.polar().is_some());
}

#[test]
fn the_import_window_reads_fixed_width_columns_and_apply_keeps_the_window_open() {
    use crate::actions::ImportPick;
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-fixed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("aligned.txt");
    let mut text = format!("{:>10}{:>10}\n", "t", "i");
    for k in 1..=5 {
        text.push_str(&format!("{:>10}{:>10}\n", k, k * 3));
    }
    std::fs::write(&file, text).unwrap();
    let mut h = start_auto_hide(vec![]);
    h.state_mut().start_import(Some(vec![file]), ImportPick::Multiple);
    settle(&mut h);
    // Delimiter: Fixed Width, with the default width of 10 characters.
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Detect")).click();
    settle(&mut h);
    click(&mut h, "Fixed Width");
    h.get_by_label("Widths");
    // Apply imports and leaves the window; OK imports and closes it. Buttons run OK, Cancel, Apply.
    assert!(button_x(&h, "OK") < button_x(&h, "Cancel") && button_x(&h, "Cancel") < button_x(&h, "Apply"));
    click(&mut h, "Apply");
    assert!(h.state().import.is_some(), "Apply keeps the window");
    assert_eq!(project(&h).workbooks().count(), 1);
    let book = project(&h).workbooks().next().unwrap();
    assert_eq!(book.sheet().columns.len(), 2, "the widths cut the lines into two columns");
    assert_eq!(column(&h, "B"), [3.0, 6.0, 9.0, 12.0, 15.0]);
    click(&mut h, "OK");
    assert!(h.state().import.is_none());
    assert_eq!(project(&h).workbooks().count(), 2, "OK imported the file once more");
}

#[test]
fn the_data_import_options_page_controls_the_names() {
    use crate::actions::ImportPick;
    let mut h = start_auto_hide(vec![]);
    h.state_mut().start_import(Some(vec![data_file("data-import-options")]), ImportPick::Multiple);
    settle(&mut h);
    click(&mut h, "Data Import Options");
    h.get_by_label("Line with the short names");
    h.get_by_label("Rename the columns from the header lines");
    h.get_by_label("Rename the book from the file name");
    // Apply the new range to plots belongs to Replace Existing Data and is grey otherwise.
    assert!(is_disabled(&h, "Apply the new range to plots"));
    click(&mut h, "Rename the sheet from the file name");
    click(&mut h, "Rename the columns from the header lines");
    click(&mut h, "OK");
    let book = project(&h).workbooks().next().expect("imported");
    assert_eq!(book.sheet().name, "Sheet1");
    assert_eq!(book.sheet().columns[1].long_name, "", "the header line was skipped, not used as a label");
    assert_eq!(column(&h, "B"), [2.5, 4.5, 6.5, 8.5]);
}

#[test]
fn column_properties_offers_month_day_of_week_and_text_and_converts_the_cells() {
    use sheaf_core::column::Storage;
    use sheaf_core::format::{NameStyle, NumberFormat};
    let mut h = start(vec![data_file("colprops-kinds")]);
    let b = open_column_props(&mut h, "B");
    let start_kind = project(&h).column(b).unwrap().storage();
    // The Format list names Origin's kinds.
    fn combo<'a>(h: &'a Harness<'static, SheafApp>, v: &'a str) -> egui_kittest::Node<'a> {
        h.get(By::new().role(egui::accesskit::Role::ComboBox).value(v))
    }
    let kind_name = if start_kind == Storage::Mixed { "Text & Numeric" } else { "Numeric" };
    combo(&h, kind_name).click();
    settle(&mut h);
    for item in ["Numeric", "Text", "Time", "Date", "Month", "Day of Week"] {
        assert!(has_label(&h, item), "{item} missing from the Format list");
    }
    click(&mut h, "Month");
    click(&mut h, "Apply");
    let c = project(&h).column(b).unwrap();
    assert_eq!(c.format, NumberFormat::Month(NameStyle::Short));
    let first = c.numeric().unwrap()[0];
    assert_eq!(c.cell_text(0), NumberFormat::Month(NameStyle::Short).format(first));
    // The display of a month is its name style.
    combo(&h, "Abbreviation").click();
    settle(&mut h);
    click(&mut h, "Full name");
    click(&mut h, "Apply");
    assert_eq!(project(&h).column(b).unwrap().format, NumberFormat::Month(NameStyle::Full));
    // Text keeps what the cells showed.
    combo(&h, "Month").click();
    settle(&mut h);
    click(&mut h, "Text");
    click(&mut h, "OK");
    let c = project(&h).column(b).unwrap();
    assert_eq!(c.storage(), Storage::Text);
    assert_eq!(c.format, NumberFormat::General);
    assert_eq!(c.text().unwrap()[0], NumberFormat::Month(NameStyle::Full).format(first));
}

#[test]
fn column_properties_refuses_a_short_name_longer_than_origins_limit() {
    let mut h = start(vec![data_file("colprops-longname")]);
    let b = open_column_props(&mut h, "B");
    type_into_box(&mut h, 0, "ABCDEFGHIJKLMNOPQRS");
    click(&mut h, "OK");
    assert!(h.state().column_props.is_some(), "the window stays open");
    assert!(has_label(&h, "The short name may have at most 18 characters."));
    assert_eq!(project(&h).column(b).unwrap().short_name, "B");
    type_into_box(&mut h, 0, "ABCDEFGHIJKLMNOPQR");
    click(&mut h, "OK");
    assert_eq!(project(&h).column(b).unwrap().short_name, "ABCDEFGHIJKLMNOPQR");
}

#[test]
fn plot_specialized_radar_and_its_toolbar_button_make_a_radar_graph() {
    let mut h = start(vec![data_file("radar-menu")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "C(Y1)");
    let before = project(&h).graphs().count();
    menu_path(&mut h, &["Plot", "Specialized", "Radar"]);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 1);
    let layer = &graphs[before].layers[0];
    assert!(layer.coords.polar().is_some_and(|p| p.radar));
    assert_eq!(layer.plots.len(), 1);
    assert_eq!(layer.plots[0].kind, sheaf_core::graph::PlotKind::LineSymbol);

    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "C(Y1)");
    bar_button!(h, "2D Graphs", "Radar").click();
    settle(&mut h);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 2);
    assert!(graphs[before + 1].layers[0].coords.polar().is_some_and(|p| p.radar));
}

#[test]
fn plot_specialized_ternary_and_its_button_make_a_ternary_graph_of_the_selected_columns() {
    let mut h = start(vec![data_file("ternary-menu")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "B(Y1)");
    let column = |h: &Harness<'static, SheafApp>, name: &str| {
        project(h).workbooks().next().unwrap().sheet().column_by_name(name).unwrap().id
    };
    let (b, c) = (column(&h, "B"), column(&h, "C"));
    let before = project(&h).graphs().count();
    menu_path(&mut h, &["Plot", "Specialized", "Ternary"]);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 1);
    let layer = &graphs[before].layers[0];
    assert!(layer.coords.ternary().is_some());
    assert_eq!(layer.plots.len(), 1);
    assert_eq!((layer.plots[0].y, layer.plots[0].size_from), (b, Some(c)), "Y, and the next column as Z");

    // The 2D Graphs button does the same.
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "B(Y1)");
    bar_button!(h, "2D Graphs", "Ternary").click();
    settle(&mut h);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 2);
    assert!(graphs[before + 1].layers[0].coords.ternary().is_some());
}

// ----- drawn objects: Line, Arrow, Rectangle, Circle tools ------------------------------------------

fn shapes(h: &Harness<'static, SheafApp>) -> Vec<sheaf_core::shape::Shape> {
    graph(h).layers[0].shapes.clone()
}

/// A page point at the given fractions of the first layer's frame, on screen.
fn frame_point(h: &Harness<'static, SheafApp>, fx: f64, fy: f64) -> egui::Pos2 {
    let f = sheaf_plot::layout_geometry(project(h), graph(h).id).unwrap()[0].frame;
    graph_ui(h).page_point(f.x + f.w * fx, f.y + f.h * fy).unwrap()
}

fn drag_frame(h: &mut Harness<'static, SheafApp>, a: (f64, f64), b: (f64, f64)) {
    let (from, to) = (frame_point(h, a.0, a.1), frame_point(h, b.0, b.1));
    drag(h, from, to);
}

#[test]
fn a_drag_with_the_rectangle_tool_draws_one_object_and_the_tool_ends() {
    let mut h = start(vec![data_file("drawrect")]);
    bar_button!(h, "Tools", "Rectangle").click();
    settle(&mut h);
    assert!(graph_ui(&h).shape_tool.is_some(), "the tool is on");
    drag_frame(&mut h, (0.2, 0.2), (0.5, 0.4));
    let s = shapes(&h);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].kind, sheaf_core::shape::ShapeKind::Rectangle);
    assert!((s[0].x1 - 0.2).abs() < 0.01 && (s[0].y2 - 0.4).abs() < 0.01, "{:?}", s[0]);
    assert!(graph_ui(&h).shape_tool.is_none(), "the tool returns to the Pointer");
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Shape(0))));
    assert_eq!(h.state().doc.undo_label(), Some("Add drawn object"));
    menu(&mut h, "Edit", "Undo Add drawn object");
    assert!(shapes(&h).is_empty());
}

#[test]
fn a_drawn_arrow_is_moved_and_deleted_with_undo() {
    let mut h = start(vec![data_file("movearrow")]);
    bar_button!(h, "Tools", "Arrow").click();
    settle(&mut h);
    drag_frame(&mut h, (0.2, 0.2), (0.6, 0.2));
    assert_eq!(shapes(&h)[0].kind, sheaf_core::shape::ShapeKind::Arrow);
    let before = shapes(&h)[0].clone();
    // Pick it in the middle of the line and drag it down.
    let at = frame_point(&h, 0.4, 0.2);
    pause(&mut h);
    drag(&mut h, at, at + egui::vec2(0.0, 50.0));
    let after = shapes(&h)[0].clone();
    assert!(after.y1 > before.y1 && (after.x1 - before.x1).abs() < 1e-9, "{before:?} -> {after:?}");
    assert!((after.y2 - after.y1 - (before.y2 - before.y1)).abs() < 1e-9, "the length is kept");
    assert_eq!(h.state().doc.undo_label(), Some("Move drawn object"));
    menu(&mut h, "Edit", "Undo Move drawn object");
    assert_eq!(shapes(&h)[0], before);
    // Delete removes the selected object; Undo brings it back.
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert!(shapes(&h).is_empty());
    assert_eq!(h.state().doc.undo_label(), Some("Delete drawn object"));
    menu(&mut h, "Edit", "Undo Delete drawn object");
    assert_eq!(shapes(&h).len(), 1);
}

#[test]
fn double_clicking_a_drawn_object_opens_its_properties() {
    let mut h = start(vec![data_file("shapeprops")]);
    bar_button!(h, "Tools", "Circle").click();
    settle(&mut h);
    drag_frame(&mut h, (0.3, 0.3), (0.7, 0.7));
    assert_eq!(shapes(&h)[0].kind, sheaf_core::shape::ShapeKind::Ellipse);
    // The top of the ellipse is on its outline.
    let at = frame_point(&h, 0.5, 0.3);
    double_click_at(&mut h, at);
    assert!(h.state().shape_dialog.is_some(), "the properties window opened");
    click(&mut h, "Filled");
    click(&mut h, "OK");
    assert!(shapes(&h)[0].fill.is_some());
    assert_eq!(h.state().doc.undo_label(), Some("Edit drawn object"));
}

/// An arrow (object 0) and a rectangle (object 1), drawn with the tools.
fn draw_arrow_and_rectangle(h: &mut Harness<'static, SheafApp>) {
    bar_button!(h, "Tools", "Arrow").click();
    settle(h);
    drag_frame(h, (0.2, 0.2), (0.6, 0.2));
    bar_button!(h, "Tools", "Rectangle").click();
    settle(h);
    drag_frame(h, (0.2, 0.5), (0.6, 0.7));
    assert_eq!(shapes(h).len(), 2);
}

#[test]
fn the_drawn_object_window_closes_when_delete_removes_its_object() {
    use sheaf_core::shape::ShapeKind;
    let mut h = start(vec![data_file("shapestaledel")]);
    draw_arrow_and_rectangle(&mut h);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Fill", |p| {
        p.graph_mut(id).unwrap().layers[0].shapes[1].fill = Some(sheaf_core::Color::rgba(1, 2, 3, 255));
    });
    let at = frame_point(&h, 0.4, 0.2);
    double_click_at(&mut h, at);
    assert!(h.state().shape_dialog.is_some(), "the arrow's window opened");
    // Delete removes the arrow; the rectangle now has its index, but the window must not edit it.
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!(shapes(&h).len(), 1);
    assert!(h.state().shape_dialog.is_none(), "the window of a deleted object closes");
    let rect = shapes(&h)[0].clone();
    assert_eq!(rect.kind, ShapeKind::Rectangle);
    assert!(rect.fill.is_some(), "the rectangle keeps its fill");
}

#[test]
fn the_drawn_object_window_closes_on_undo_and_redo_of_its_layer() {
    let mut h = start(vec![data_file("shapestaleundo")]);
    draw_arrow_and_rectangle(&mut h);
    // Double-click the rectangle's top edge, then undo its drawing.
    let at = frame_point(&h, 0.4, 0.5);
    double_click_at(&mut h, at);
    assert!(h.state().shape_dialog.is_some(), "the rectangle's window opened");
    menu(&mut h, "Edit", "Undo Add drawn object");
    assert_eq!(shapes(&h).len(), 1);
    assert!(h.state().shape_dialog.is_none(), "undo closed the window");
    // Redo while a window of the arrow is open closes it as well.
    let at = frame_point(&h, 0.4, 0.2);
    double_click_at(&mut h, at);
    assert!(h.state().shape_dialog.is_some(), "the arrow's window opened");
    menu(&mut h, "Edit", "Redo Add drawn object");
    assert_eq!(shapes(&h).len(), 2);
    assert!(h.state().shape_dialog.is_none(), "redo closed the window");
}

#[test]
fn the_drawn_object_window_closes_when_another_kind_takes_its_index() {
    let mut h = start(vec![data_file("shapestalekind")]);
    draw_arrow_and_rectangle(&mut h);
    let at = frame_point(&h, 0.4, 0.2);
    double_click_at(&mut h, at);
    assert!(h.state().shape_dialog.is_some());
    // A change that does not go through the graph or undo handlers (the object list shifts).
    let id = graph(&h).id;
    h.state_mut().doc.apply("Remove", |p| {
        p.graph_mut(id).unwrap().layers[0].shapes.remove(0);
    });
    settle(&mut h);
    assert!(h.state().shape_dialog.is_none(), "the arrow's window closed: a rectangle is at its index");
}

#[test]
fn a_drag_shorter_than_the_minimum_draws_nothing_and_keeps_the_tool_on() {
    let mut h = start(vec![data_file("drawtiny")]);
    bar_button!(h, "Tools", "Line").click();
    settle(&mut h);
    // A press held long enough to be a drag (egui decides by distance or time), moved about 2
    // page points (a page point is about half a screen pixel here): under the 3 point minimum.
    let from = frame_point(&h, 0.3, 0.3);
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
    h.run_steps(100);
    h.event(egui::Event::PointerMoved(from + egui::vec2(1.0, 0.0)));
    h.step();
    h.event(button(from + egui::vec2(1.0, 0.0), false));
    h.step();
    settle(&mut h);
    assert!(shapes(&h).is_empty(), "too short to draw");
    assert!(graph_ui(&h).shape_tool.is_some(), "the tool is still on");
    // Escape ends the tool.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(graph_ui(&h).shape_tool.is_none(), "Escape ends the tool");
    assert!(shapes(&h).is_empty());
}

/// A drag from `from` to `to` with Shift held (the whole gesture).
fn drag_shifted(h: &mut Harness<'static, SheafApp>, from: egui::Pos2, to: egui::Pos2) {
    let shift = egui::Modifiers::SHIFT;
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: shift,
    };
    for event in [
        egui::Event::PointerMoved(from),
        button(from, true),
        egui::Event::PointerMoved(from.lerp(to, 0.5)),
        egui::Event::PointerMoved(to),
        button(to, false),
    ] {
        h.event_modifiers(event, shift);
        h.step();
    }
    settle(h);
}

#[test]
fn shift_while_drawing_snaps_a_line_and_squares_a_rectangle() {
    use sheaf_core::shape::ShapeKind;
    let mut h = start(vec![data_file("drawshift")]);
    let f = sheaf_plot::layout_geometry(project(&h), graph(&h).id).unwrap()[0].frame;
    // A line that is about 10 degrees off horizontal becomes horizontal; the length is kept.
    bar_button!(h, "Tools", "Line").click();
    settle(&mut h);
    let (a, b) = (frame_point(&h, 0.2, 0.3), frame_point(&h, 0.6, 0.3) + egui::vec2(0.0, 12.0));
    drag_shifted(&mut h, a, b);
    let s = shapes(&h);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].kind, ShapeKind::Line);
    assert!((s[0].y2 - s[0].y1).abs() < 1e-9, "horizontal: {:?}", s[0]);
    assert!(s[0].x2 > s[0].x1 + 0.1);
    // A rectangle dragged wider than tall is a square in page points.
    bar_button!(h, "Tools", "Rectangle").click();
    settle(&mut h);
    let (a, b) = (frame_point(&h, 0.1, 0.5), frame_point(&h, 0.4, 0.6));
    drag_shifted(&mut h, a, b);
    let r = shapes(&h)[1].clone();
    assert_eq!(r.kind, ShapeKind::Rectangle);
    let (w, hgt) = ((r.x2 - r.x1).abs() * f.w, (r.y2 - r.y1).abs() * f.h);
    assert!((w - hgt).abs() < 1e-6, "square: {w} x {hgt}");
}

#[test]
fn data_manipulation_items_and_median_filter_open_and_run() {
    let mut h = start(vec![data_file("manip")]);
    show_window(&mut h, "Book1 - sample");
    // Both columns are straight lines, so the line through the first and last point removes them.
    menu_path(&mut h, &["Analysis", "Data Manipulation", "Subtract Straight Line..."]);
    assert!(h.state().analysis.is_some(), "the window is open");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    let added: Vec<Vec<f64>> = project(&h)
        .workbooks()
        .flat_map(|b| b.sheet().columns.iter())
        .filter(|c| c.long_name.ends_with("minus line"))
        .map(|c| c.numeric().unwrap().to_vec())
        .collect();
    assert_eq!(added.len(), 2);
    for v in &added {
        assert!(v.iter().all(|x| x.abs() < 1e-9), "{v:?}");
    }
    assert_eq!(h.state().doc.undo_label(), Some("Subtract straight line"));

    show_window(&mut h, "Book1 - sample");
    menu_path(&mut h, &["Analysis", "Mathematics", "Median Filter..."]);
    click(&mut h, "OK");
    assert_eq!(h.state().doc.undo_label(), Some("Median filter"));

    // Translate X writes a new sheet.
    show_window(&mut h, "Book1 - sample");
    let sheets_before = project(&h).workbooks().next().unwrap().sheets.len();
    menu_path(&mut h, &["Analysis", "Data Manipulation", "Curve Translate X..."]);
    click(&mut h, "OK");
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), sheets_before + 1);
    assert_eq!(h.state().doc.undo_label(), Some("Translate X"));
}

#[test]
fn reduce_x_and_translate_y_run_from_the_menu_and_the_menu_lists_translate_y_after_x() {
    let mut h = start(vec![data_file("manip2")]);
    show_window(&mut h, "Book1 - sample");
    // Two of the Data Manipulation items open their windows (they are no longer greyed) and Cancel changes nothing.
    let undo_before = h.state().doc.undo_label().map(str::to_string);
    for path in [
        ["Analysis", "Data Manipulation", "Reduce to Evenly Spaced X..."],
        ["Analysis", "Data Manipulation", "Curve Translate Y..."],
    ] {
        menu_path(&mut h, &path);
        assert!(h.state().analysis.is_some(), "{path:?} opens its window");
        click(&mut h, "Cancel");
        assert!(h.state().analysis.is_none());
        assert_eq!(h.state().doc.undo_label().map(str::to_string), undo_before, "Cancel changes nothing");
    }
    let analysis = menus::bar(WindowKind::Workbook).iter().find(|m| m.name() == "Analysis").expect("the Analysis menu");
    let manip = analysis
        .entries
        .iter()
        .find_map(|e| match e {
            menus::Entry::Submenu(s) if menus::plain(&s.label) == "Data Manipulation" => Some(s),
            _ => None,
        })
        .expect("the Data Manipulation submenu");
    let names: Vec<String> = manip
        .entries
        .iter()
        .filter_map(|e| if let menus::Entry::Item(i) = e { Some(menus::plain(&i.label)) } else { None })
        .collect();
    let x = names.iter().position(|n| n == "Curve Translate X...").unwrap_or_else(|| panic!("{names:?}"));
    assert_eq!(names[x + 1], "Curve Translate Y...");

    // Reduce to 4 evenly spaced X values (the default is 100 points: the run keeps the X range).
    open_analysis_on(&mut h, AnalysisKind::ReduceX, &["B"]);
    click(&mut h, "OK");
    assert_eq!(h.state().doc.undo_label(), Some("Reduce to evenly spaced X"));
    let sheets = &project(&h).workbooks().next().unwrap().sheets;
    let cols = &sheets.last().unwrap().columns;
    assert_eq!(cols.len(), 2);
    assert_eq!(cols[0].numeric().unwrap().len(), 100);
    assert_eq!(cols[0].numeric().unwrap()[0], 1.0);
    assert_eq!(cols[0].numeric().unwrap()[99], 4.0);

    // The reduced sheet is now the active one. Translate Y with the shift set to 1.5 writes the Y column
    // plus 1.5 beside it, and Undo removes it.
    let reduced = cols[1].numeric().unwrap().to_vec();
    open_analysis_on(&mut h, AnalysisKind::TranslateY, &["B"]);
    h.state_mut().analysis.as_mut().unwrap().set_translate_by(1.5);
    click(&mut h, "OK");
    assert_eq!(h.state().doc.undo_label(), Some("Translate Y"));
    let shifted = project(&h)
        .workbooks()
        .next()
        .unwrap()
        .sheet()
        .columns
        .iter()
        .find(|c| c.long_name == "up shifted")
        .map(|c| c.numeric().unwrap().to_vec())
        .expect("the shifted column");
    let expected: Vec<f64> = reduced.iter().map(|v| v + 1.5).collect();
    assert_eq!(shifted, expected);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(project(&h).workbooks().next().unwrap().sheet().columns.iter().all(|c| c.long_name != "up shifted"));
}

#[test]
fn the_preview_of_a_tool_that_writes_a_new_sheet_shows_the_new_curve() {
    let mut h = start(vec![data_file("manip-preview")]);
    // The input is t = 1..4, up = 2.5, 4.5, 6.5, 8.5. Translate X by 10 keeps Y and moves X.
    open_analysis_on(&mut h, AnalysisKind::TranslateX, &["B"]);
    h.state_mut().analysis.as_mut().unwrap().set_translate_by(10.0);
    click(&mut h, "Preview");
    let (rx, ry) = h.state().analysis.as_ref().unwrap().preview_result().expect("a preview");
    assert_eq!(rx, [11.0, 12.0, 13.0, 14.0]);
    assert_eq!(ry, [2.5, 4.5, 6.5, 8.5]);
    click(&mut h, "Cancel");
    // Reduce to Evenly Spaced X: 100 points from X = 1 to 4, on the straight line y = 2 x + 0.5.
    open_analysis_on(&mut h, AnalysisKind::ReduceX, &["B"]);
    click(&mut h, "Preview");
    let (rx, ry) = h.state().analysis.as_ref().unwrap().preview_result().expect("a preview");
    assert_eq!((rx.len(), rx[0], rx[99]), (100, 1.0, 4.0));
    assert!(rx.iter().zip(&ry).all(|(x, y)| (y - (2.0 * x + 0.5)).abs() < 1e-9), "{ry:?}");
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1, "a preview writes nothing");
}

#[test]
fn a_median_filter_made_with_recalculate_auto_follows_the_edited_input() {
    let mut h = start(vec![data_file("manip-auto")]);
    open_analysis_on(&mut h, AnalysisKind::MedianFilter, &["B"]);
    // Manual is the default; the median filter can follow its input, so Auto is offered.
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Manual")).click();
    settle(&mut h);
    assert!(!is_disabled(&h, "Auto"));
    click(&mut h, "Auto");
    click(&mut h, "OK");
    let b = column_id(&h, "B");
    let out = project(&h)
        .workbooks()
        .next()
        .unwrap()
        .sheet()
        .columns
        .iter()
        .find(|c| c.long_name == "Median filtered up")
        .expect("the filtered column")
        .id;
    // The window of 5 shrinks at the ends to as many points as fit on both sides, and a straight line
    // is its own median.
    assert_eq!(project(&h).numeric(out).unwrap(), [2.5, 4.5, 6.5, 8.5]);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SetCell { column: b, row: 0, text: "100".into() }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).numeric(out).unwrap(), [100.0, 6.5, 6.5, 8.5], "the same column was updated");
    assert_eq!(project(&h).column(out).unwrap().analysis.unwrap().state, sheaf_core::FormulaState::Current);
}
