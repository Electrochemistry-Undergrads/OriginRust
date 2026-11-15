//! Save Window As and window files (T54, `docs/spec/window-files.md`): the File menu, opening
//! `.sheafbook` and `.sheafgraph` files as an append, and the Recent Books and Recent Graphs lists.
//! Every test uses a recent file of its own in a temporary folder.

use super::*;
use sheaf_io::recent::RecentList;

/// More frames than the usual settle: a window that was just added needs them before the menu bar
/// of its kind is drawn.
fn settle(h: &mut Harness<'static, SheafApp>) {
    h.run_steps(30);
}

/// A fresh, empty folder for one test.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-savewin-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    sheaf_io::recent::normalize(&std::fs::canonicalize(&dir).unwrap())
}

fn start_with(paths: Vec<PathBuf>, recent_path: Option<PathBuf>) -> Harness<'static, SheafApp> {
    let startup = Startup { paths, recent_path, ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    h
}

/// A CSV of three columns in `dir`; importing it makes `Book1` and `Graph1`.
fn csv_in(dir: &Path, file: &str) -> PathBuf {
    let path = dir.join(file);
    std::fs::write(&path, "t,up,down\n1,2.5,30.5\n2,4.5,20.5\n3,6.5,10.5\n").unwrap();
    path
}

fn names(h: &Harness<'static, SheafApp>) -> Vec<String> {
    project(h).windows.iter().map(|w| w.name().to_string()).collect()
}

fn id_of(h: &Harness<'static, SheafApp>, name: &str) -> WindowId {
    project(h).windows.iter().find(|w| w.name() == name).unwrap_or_else(|| panic!("no window {name}")).id()
}

/// The X and Y numbers of every plot of a graph, by plot.
fn plotted(h: &Harness<'static, SheafApp>, graph: &str) -> Vec<(Vec<f64>, Vec<f64>)> {
    let p = project(h);
    let g = p.graph(id_of(h, graph)).unwrap();
    let numbers = |id| p.column(id).unwrap().numeric().unwrap().to_vec();
    g.layers.iter().flat_map(|l| &l.plots).map(|pl| (numbers(pl.x.unwrap()), numbers(pl.y))).collect()
}

/// The numbers of one column of a workbook.
fn book_column(h: &Harness<'static, SheafApp>, book: &str, short: &str) -> Vec<f64> {
    project(h).workbook(id_of(h, book)).unwrap().sheet().column_by_name(short).unwrap().numeric().unwrap().to_vec()
}

/// Saves the window of this name as a window file (what File > Save Window As... does after the
/// chooser).
fn save_as(h: &mut Harness<'static, SheafApp>, window: &str, path: PathBuf) -> bool {
    let id = id_of(h, window);
    h.state_mut().save_window_to(id, path)
}

/// A new, empty project (the answer to File > New > Project), the recent file kept.
fn new_project(h: &mut Harness<'static, SheafApp>) {
    let ctx = h.ctx.clone();
    h.state_mut().do_pending(Pending::New, &ctx);
    settle(h);
}

fn open_recent(h: &mut Harness<'static, SheafApp>, list: &str) {
    menu_path(h, &["File", list]);
}

fn entry<'a>(h: &'a Harness<'static, SheafApp>, n: &str, stem: &'a str) -> egui_kittest::Node<'a> {
    h.query_all_by_label_contains(stem)
        .find(|node| {
            let label = node.accesskit_node().label().unwrap_or_default().to_string();
            label.starts_with(&format!("{n} ")) && label.ends_with(stem)
        })
        .unwrap_or_else(|| panic!("no recent entry {n} ... {stem}"))
}

/// Imports a CSV (Book1 and Graph1), saves the graph and the book as window files and starts over
/// with an empty project. Returns the harness, the graph file and the book file.
fn saved_graph_and_book(tag: &str) -> (Harness<'static, SheafApp>, PathBuf, PathBuf, PathBuf) {
    let dir = scratch(tag);
    let mut h = start_with(vec![csv_in(&dir, "run.csv")], Some(dir.join("recent.json")));
    let (graph, book) = (dir.join("fig.sheafgraph"), dir.join("data.sheafbook"));
    assert!(save_as(&mut h, "Graph1", graph.clone()));
    assert!(save_as(&mut h, "Book1", book.clone()));
    new_project(&mut h);
    (h, dir, graph, book)
}

#[test]
fn save_window_as_is_enabled_in_a_workbook_and_a_graph_and_save_template_as_stays_grey() {
    let dir = scratch("menu");
    let mut h = start_with(vec![csv_in(&dir, "g.csv")], Some(dir.join("recent.json")));
    // The graph is active.
    menu_entry(&h, "File").click();
    settle(&mut h);
    assert!(entry_enabled(&h, "Save Window As..."), "enabled in a graph");
    assert!(!entry_enabled(&h, "Save Template As..."), "Save Template As stays grey");
    close_menus(&mut h);
    click(&mut h, "Title Book1 - g");
    menu_entry(&h, "File").click();
    settle(&mut h);
    assert!(entry_enabled(&h, "Save Window As..."), "enabled in a workbook");
    assert!(!entry_enabled(&h, "Save Template As..."));
    assert!(!entry_enabled(&h, "Save Workbook As Analysis Template..."));
}

#[test]
fn a_saved_graph_opens_in_a_new_project_with_its_book_and_plots_the_same_data() {
    let dir = scratch("graph");
    let mut h = start_with(vec![csv_in(&dir, "run.csv")], Some(dir.join("recent.json")));
    let before = plotted(&h, "Graph1");
    assert!(!before.is_empty());
    let file = dir.join("fig.sheafgraph");
    assert!(save_as(&mut h, "Graph1", file.clone()));
    assert!(file.exists());

    new_project(&mut h);
    assert!(project(&h).windows.is_empty());
    h.state_mut().open_paths(vec![file.clone()], false);
    settle(&mut h);
    assert_eq!(names(&h), ["Book1", "Graph1"], "the book comes with the graph");
    assert_eq!(plotted(&h, "Graph1"), before, "the graph plots the same data");
    assert_eq!(book_column(&h, "Book1", "B"), [2.5, 4.5, 6.5]);
    assert_eq!(h.state().workspace.active(), Some(id_of(&h, "Graph1")), "the saved window is in front");
    assert!(h.state().path.is_none(), "the project did not become the file");
}

#[test]
fn a_saved_workbook_opens_with_its_data() {
    let (mut h, _dir, _graph, book) = saved_graph_and_book("book");
    h.state_mut().open_paths(vec![book], false);
    settle(&mut h);
    assert_eq!(names(&h), ["Book1"]);
    assert_eq!(book_column(&h, "Book1", "C"), [30.5, 20.5, 10.5]);
    assert_eq!(h.state().workspace.active(), Some(id_of(&h, "Book1")));
}

#[test]
fn a_name_clash_renames_the_opened_windows_and_says_so() {
    let (mut h, _dir, graph, _book) = saved_graph_and_book("clash");
    h.state_mut().open_paths(vec![graph.clone()], false);
    settle(&mut h);
    h.state_mut().message = None;
    // The same file again: both names are taken.
    h.state_mut().open_paths(vec![graph], false);
    settle(&mut h);
    assert_eq!(names(&h), ["Book1", "Graph1", "Book2", "Graph2"]);
    let (title, text) = h.state().message.clone().expect("the owner is told about the renames");
    assert!(title.contains("window file"), "{title}");
    assert!(text.contains("Book1 is renamed Book2") && text.contains("Graph1 is renamed Graph2"), "{text}");
    let (first, second) = (plotted(&h, "Graph1"), plotted(&h, "Graph2"));
    assert_eq!(first, second, "each copy plots its own copy of the same data");
    let g1 = project(&h).graph(id_of(&h, "Graph1")).unwrap().layers[0].plots[0].y;
    let g2 = project(&h).graph(id_of(&h, "Graph2")).unwrap().layers[0].plots[0].y;
    assert_ne!(g1, g2, "not the same column");
}

#[test]
fn opening_a_window_file_is_one_undo_step_that_leaves_the_project_as_it_was() {
    let (mut h, _dir, graph, _book) = saved_graph_and_book("undo");
    h.state_mut().open_paths(vec![graph], false);
    settle(&mut h);
    assert_eq!(project(&h).windows.len(), 2);
    assert_eq!(h.state().doc.undo_label(), Some("Open window file"));
    menu(&mut h, "Edit", "Undo Open window file");
    assert!(project(&h).windows.is_empty(), "both windows leave in one step");
    // With no window the File menu carries Undo and Redo.
    menu(&mut h, "File", "Redo Open window file");
    assert_eq!(names(&h), ["Book1", "Graph1"]);
}

#[test]
fn a_window_file_never_replaces_the_open_project() {
    let (mut h, _dir, _graph, book) = saved_graph_and_book("keep");
    h.state_mut().doc.apply("New workbook", |p| {
        p.add_workbook("Mine");
    });
    h.state_mut().open_paths(vec![book], false);
    settle(&mut h);
    assert!(project(&h).workbooks().any(|b| b.long_name == "Mine"), "the project's own window is still there");
    assert_eq!(names(&h).len(), 2);
    assert!(h.state().pending.is_none(), "no question about unsaved changes");
    assert!(h.state().doc.dirty, "opening a window file leaves the project changed");
    assert!(h.state().path.is_none(), "and it did not become the file");
}

#[test]
fn a_dropped_window_file_and_one_on_the_command_line_are_opened_as_an_append() {
    let (h, dir, graph, book) = saved_graph_and_book("route");
    drop(h);
    // The command line: the book, then the graph; their names clash and the second is renamed.
    let mut h = start_with(vec![book.clone(), graph.clone()], Some(dir.join("recent2.json")));
    assert_eq!(names(&h), ["Book1", "Book2", "Graph1"], "both added, nothing replaced");
    // A drop asks nothing either.
    h.state_mut().open_paths(vec![book], true);
    settle(&mut h);
    assert_eq!(names(&h).len(), 4);
    assert!(h.state().import.is_none(), "not taken for a data file");
}

#[test]
fn a_project_and_a_window_file_given_together_open_the_project_and_add_the_window() {
    let dir = scratch("together");
    let mut h = start_with(vec![csv_in(&dir, "run.csv")], Some(dir.join("recent.json")));
    let book = dir.join("only.sheafbook");
    assert!(save_as(&mut h, "Book1", book.clone()));
    h.state_mut().doc.apply("New workbook", |p| {
        p.add_workbook("Other");
    });
    let project_path = dir.join("p.sheaf");
    assert!(h.state_mut().save_to(project_path.clone()));
    new_project(&mut h);
    h.state_mut().open_paths(vec![book, project_path], false);
    settle(&mut h);
    let have = names(&h);
    assert!(project(&h).workbooks().any(|b| b.long_name == "Other"), "the project opened: {have:?}");
    assert_eq!(project(&h).workbooks().count(), 3, "and the window was added: {have:?}");
}

#[test]
fn saving_a_window_lists_it_in_recent_books_or_recent_graphs_and_the_file() {
    let dir = scratch("lists");
    let file = dir.join("recent.json");
    let mut h = start_with(vec![csv_in(&dir, "run.csv")], Some(file.clone()));
    let (graph, book) = (dir.join("fig.sheafgraph"), dir.join("data.sheafbook"));
    assert!(save_as(&mut h, "Graph1", graph.clone()));
    assert!(save_as(&mut h, "Book1", book.clone()));
    assert_eq!(h.state().recent.graphs(), std::slice::from_ref(&graph));
    assert_eq!(h.state().recent.books(), std::slice::from_ref(&book));
    assert!(h.state().recent.projects().is_empty(), "never in the projects");
    let stored = RecentList::load(&file);
    assert_eq!((stored.graphs, stored.books), (vec![graph], vec![book]));
}

#[test]
fn clicking_a_recent_graph_opens_it_and_a_recent_book_opens_it() {
    let (mut h, _dir, graph, book) = saved_graph_and_book("click");
    // The saves put them in the lists; opening a file moves it to the top again.
    h.state_mut().open_paths(vec![book.clone()], false);
    settle(&mut h);
    open_recent(&mut h, "Recent Books");
    entry(&h, "1", "data").click();
    settle(&mut h);
    assert_eq!(names(&h), ["Book1", "Book2"], "the click adds the book once more");
    // The active window is a workbook, so Recent Graphs is not in its menu; open the graph by code
    // path the click uses.
    h.state_mut().follow_recent(graph.clone(), RecentKind::Graphs);
    settle(&mut h);
    assert!(names(&h).contains(&"Graph1".to_string()));
    // The renames of the second copy were reported in a window; closing it as the owner would.
    assert!(h.state().message.is_some());
    h.state_mut().message = None;
    close_menus(&mut h);
    open_recent(&mut h, "Recent Graphs");
    assert_eq!(h.state().recent.graphs(), std::slice::from_ref(&graph));
    entry(&h, "1", "fig").click();
    settle(&mut h);
    assert_eq!(names(&h), ["Book1", "Book2", "Book3", "Graph1", "Book4", "Graph2"], "the click opens the graph file");
    assert_eq!(h.state().workspace.active(), Some(id_of(&h, "Graph2")));
}

#[test]
fn a_recent_entry_whose_file_is_gone_or_of_the_other_kind_is_reported_and_removed() {
    let (mut h, dir, graph, book) = saved_graph_and_book("gone");
    std::fs::remove_file(&book).unwrap();
    h.state_mut().follow_recent(book.clone(), RecentKind::Books);
    settle(&mut h);
    let (title, text) = h.state().message.clone().expect("told");
    assert_eq!(title, "Could not open the workbook file");
    assert!(text.contains("not there"), "{text}");
    assert!(!h.state().recent.books().contains(&book), "removed");
    assert!(project(&h).windows.is_empty());
    // A graph file in the books list is the wrong kind.
    h.state_mut().message = None;
    h.state_mut().recent.push_window(WindowFileKind::Book, &graph);
    h.state_mut().follow_recent(graph.clone(), RecentKind::Books);
    settle(&mut h);
    let (_, text) = h.state().message.clone().expect("told");
    assert!(text.contains("graph file, not a workbook file"), "{text}");
    assert!(h.state().recent.books().is_empty());
    assert_eq!(RecentList::load(&dir.join("recent.json")).books, Vec::<PathBuf>::new());
}

#[test]
fn clear_recent_books_and_graphs_empty_one_list_each() {
    let (mut h, dir, graph, _book) = saved_graph_and_book("clear");
    h.state_mut().open_paths(vec![graph], false);
    settle(&mut h);
    open_recent(&mut h, "Recent Graphs");
    menu_entry(&h, "Clear Recent Graphs").click();
    settle(&mut h);
    assert!(h.state().recent.graphs().is_empty());
    assert_eq!(h.state().recent.books().len(), 1, "the other list is left");
    close_menus(&mut h);
    open_recent(&mut h, "Recent Books");
    menu_entry(&h, "Clear Recent Books").click();
    settle(&mut h);
    assert!(h.state().recent.books().is_empty());
    let stored = RecentList::load(&dir.join("recent.json"));
    assert!(stored.books.is_empty() && stored.graphs.is_empty());
    close_menus(&mut h);
    open_recent(&mut h, "Recent Books");
    assert!(!entry_enabled(&h, "(none)"));
    assert!(!entry_enabled(&h, "Clear Recent Books"));
}

#[test]
fn a_damaged_window_file_adds_nothing_and_is_not_listed() {
    let dir = scratch("damaged");
    let mut h = start_with(vec![], Some(dir.join("recent.json")));
    let bad = dir.join("bad.sheafgraph");
    std::fs::write(&bad, "this is not an archive").unwrap();
    h.state_mut().open_paths(vec![bad], false);
    settle(&mut h);
    let (title, _) = h.state().message.clone().expect("told");
    assert_eq!(title, "Could not open the window file");
    assert!(project(&h).windows.is_empty());
    assert!(h.state().recent.graphs().is_empty());
}

#[test]
fn a_name_without_the_extension_gets_it_and_a_project_file_is_never_written_over() {
    let dir = scratch("ext");
    let mut h = start_with(vec![csv_in(&dir, "run.csv")], Some(dir.join("recent.json")));
    let project_file = dir.join("keep.sheaf");
    std::fs::write(&project_file, "a project that must stay").unwrap();
    assert!(save_as(&mut h, "Book1", dir.join("plain")));
    assert!(dir.join("plain.sheafbook").exists());
    assert!(save_as(&mut h, "Graph1", project_file.clone()));
    assert_eq!(std::fs::read_to_string(&project_file).unwrap(), "a project that must stay");
    assert!(dir.join("keep.sheaf.sheafgraph").exists());
    // The extension of the other kind is not taken for the right one.
    assert!(save_as(&mut h, "Graph1", dir.join("g.sheafbook")));
    assert!(dir.join("g.sheafbook.sheafgraph").exists());
    // Upper case is the same extension.
    assert!(save_as(&mut h, "Book1", dir.join("UP.SHEAFBOOK")));
    assert!(dir.join("UP.SHEAFBOOK").exists());
}

#[test]
fn a_formula_that_reads_a_book_outside_the_window_is_dropped_and_the_owner_is_told() {
    let dir = scratch("notes");
    let mut h = start_with(vec![csv_in(&dir, "run.csv")], Some(dir.join("recent.json")));
    h.state_mut().doc.apply("New workbook", |p| {
        p.add_workbook("Other");
    });
    let first = project(&h).workbooks().find(|b| b.name == "Book1").unwrap().sheet().columns[1].id;
    h.state_mut().doc.apply("Formula", |p| {
        let col = p.column_mut(first).unwrap();
        col.formula = Some("[Other]Sheet1!col(A)*2".to_string());
    });
    let file = dir.join("book1.sheafbook");
    assert!(save_as(&mut h, "Book1", file.clone()));
    let (title, text) = h.state().message.clone().expect("a note");
    assert_eq!(title, "Notes about the saved window");
    assert!(text.contains("Other"), "{text}");
    // Opened elsewhere the values are there and the formula is not.
    new_project(&mut h);
    h.state_mut().open_paths(vec![file], false);
    settle(&mut h);
    let col = project(&h).workbooks().next().unwrap().sheet().columns[1].clone();
    assert!(col.formula.is_none(), "no dangling formula");
    assert_eq!(col.numeric().unwrap().len(), 3, "the values stay");
}

/// A project file of a book named `book`, with a clean start: an empty project is open afterwards.
fn project_with(h: &mut Harness<'static, SheafApp>, dir: &Path, file: &str, book: &str) -> PathBuf {
    let mut p = Project::new();
    p.add_workbook(book);
    let path = dir.join(file);
    sheaf_io::save::save_project(&p, &path).unwrap();
    new_project(h);
    path
}

/// The long names of the workbooks, in order.
fn books(h: &Harness<'static, SheafApp>) -> Vec<String> {
    project(h).workbooks().map(|b| b.long_name.clone()).collect()
}

/// Makes the open project dirty with a workbook of its own.
fn make_dirty(h: &mut Harness<'static, SheafApp>) {
    h.state_mut().doc.apply("New workbook", |p| {
        p.add_workbook("Mine");
    });
    settle(h);
}

#[test]
fn a_window_file_picked_in_open_asks_nothing_and_a_project_asks_only_when_changed() {
    let (mut h, dir, graph, _book) = saved_graph_and_book("picked");
    let other = project_with(&mut h, &dir, "p.sheaf", "Other");
    make_dirty(&mut h);
    h.state_mut().open_picked(graph);
    settle(&mut h);
    assert!(h.state().pending.is_none(), "a window file asks nothing: it does not replace the project");
    assert!(!has_label(&h, "The project has unsaved changes."));
    assert_eq!(names(&h).len(), 3, "its book and graph were added");
    assert!(books(&h).contains(&"Mine".to_string()), "and the project's own book is still there");
    // A project replaces this one, so it asks first.
    h.state_mut().open_picked(other);
    settle(&mut h);
    assert!(matches!(h.state().pending, Some(Pending::OpenFile)));
    assert!(has_label(&h, "The project has unsaved changes."));
    click(&mut h, "Don't save");
    assert_eq!(books(&h), ["Other"]);
    assert_eq!(names(&h).len(), 1);
}

#[test]
fn a_project_picked_in_open_with_no_changes_opens_at_once() {
    let (mut h, dir, _graph, _book) = saved_graph_and_book("picked-clean");
    let other = project_with(&mut h, &dir, "p.sheaf", "Other");
    h.state_mut().open_picked(other);
    settle(&mut h);
    assert!(h.state().pending.is_none());
    assert_eq!(books(&h), ["Other"]);
}

#[test]
fn a_project_and_a_window_file_dropped_with_unsaved_changes_ask_then_open_both() {
    let (mut h, dir, _graph, book) = saved_graph_and_book("together-dirty");
    let other = project_with(&mut h, &dir, "p.sheaf", "Other");
    make_dirty(&mut h);
    h.state_mut().open_paths(vec![book.clone(), other.clone()], false);
    settle(&mut h);
    assert!(matches!(h.state().pending, Some(Pending::OpenFile)), "the question comes first");
    assert_eq!(books(&h), ["Mine"], "nothing was replaced or added yet");
    click(&mut h, "Don't save");
    assert_eq!(books(&h).len(), 2, "the project opened and the window file joined it");
    assert!(books(&h).contains(&"Other".to_string()));
    assert!(h.state().pending_windows.is_empty());

    // Cancel: nothing opens and the window file does not wait for a later question.
    make_dirty(&mut h);
    h.state_mut().open_paths(vec![book, other], false);
    settle(&mut h);
    click(&mut h, "Cancel");
    assert_eq!(books(&h).len(), 3, "nothing opened and nothing was added");
    assert!(books(&h).contains(&"Mine".to_string()));
    assert!(h.state().pending_windows.is_empty(), "the window file is not kept for later");
}

#[test]
fn a_window_file_is_not_added_when_the_project_given_with_it_cannot_be_opened() {
    let (mut h, dir, _graph, book) = saved_graph_and_book("bad-project");
    make_dirty(&mut h);
    let bad = dir.join("bad.sheaf");
    std::fs::write(&bad, b"this is not a project").unwrap();
    // Dirty: after the question.
    h.state_mut().open_paths(vec![book.clone(), bad.clone()], false);
    settle(&mut h);
    click(&mut h, "Don't save");
    assert_eq!(books(&h), ["Mine"], "the old project stays as it was");
    let (title, _) = h.state().message.clone().expect("the owner is told");
    assert_eq!(title, "Could not open the project");
    // Clean: at once.
    h.state_mut().message = None;
    new_project(&mut h);
    h.state_mut().open_paths(vec![book, bad], false);
    settle(&mut h);
    assert!(project(&h).windows.is_empty(), "the window file is not added to the old project");
    assert!(!h.state().doc.dirty);
    assert_eq!(h.state().message.clone().map(|m| m.0).as_deref(), Some("Could not open the project"));
}
