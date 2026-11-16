//! File > New > Graph, the Standard toolbar's New Graph, Save Project without Data... and
//! Duplicate Project without Data (T77).

use super::*;

fn settle(h: &mut Harness<'static, SheafApp>) {
    h.run_steps(30);
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-nodata-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn start_with(paths: Vec<PathBuf>, recent_path: PathBuf) -> Harness<'static, SheafApp> {
    let startup = Startup { paths, recent_path: Some(recent_path), ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    h
}

fn csv_in(dir: &Path) -> PathBuf {
    let path = dir.join("run.csv");
    std::fs::write(&path, "t,up\n1,2.5\n2,4.5\n3,6.5\n").unwrap();
    path
}

fn graphs(h: &Harness<'static, SheafApp>) -> Vec<String> {
    project(h).windows.iter().filter(|w| matches!(w, Window::Graph(_))).map(|w| w.name().to_string()).collect()
}

#[test]
fn the_menu_item_and_the_toolbar_button_make_an_empty_graph_in_one_undo_step() {
    let dir = scratch("new");
    let mut h = start_with(vec![], dir.join("recent.json"));
    menu_path(&mut h, &["File", "New", "Graph"]);
    settle(&mut h);
    assert_eq!(graphs(&h), ["Graph1"]);
    let id = project(&h).windows[0].id();
    assert!(project(&h).graph(id).unwrap().layers.iter().all(|l| l.plots.is_empty()), "no plots");
    assert_eq!(h.state().workspace.active(), Some(id), "the new graph is in front");
    assert_eq!(h.state().doc.undo_label(), Some("New graph"));

    assert!(!h.get_by_label("Standard toolbar: New Graph").accesskit_node().is_disabled());
    h.get_by_label("Standard toolbar: New Graph").click();
    settle(&mut h);
    assert_eq!(graphs(&h), ["Graph1", "Graph2"]);
    h.state_mut().doc.undo();
    assert_eq!(graphs(&h), ["Graph1"]);
    h.state_mut().doc.undo();
    assert!(graphs(&h).is_empty());
}

#[test]
fn the_two_project_items_are_enabled() {
    let dir = scratch("menu");
    let mut h = start_with(vec![], dir.join("recent.json"));
    menu_entry(&h, "File").click();
    settle(&mut h);
    for label in ["Save Project without Data...", "Duplicate Project without Data"] {
        assert!(entry_enabled(&h, label), "{label}");
    }
}

#[test]
fn saving_without_data_writes_a_copy_and_leaves_the_open_project_alone() {
    let dir = scratch("save");
    let mut h = start_with(vec![csv_in(&dir)], dir.join("recent.json"));
    let before = project(&h).clone();
    let dirty = h.state().doc.dirty;
    for name in ["a.sheaf", "b.opj"] {
        let file = dir.join(name);
        assert!(h.state_mut().save_project_without_data_to(file.clone()));
        let back = sheaf_io::open_project(&file).unwrap().project;
        let book = back.workbooks().next().unwrap();
        assert!(book.sheets[0].columns.len() >= 2, "{name}");
        assert!(book.sheets[0].columns.iter().all(|c| c.is_empty()), "{name}");
    }
    assert!(h.state().path.is_none(), "the project did not become the file");
    assert_eq!(h.state().doc.dirty, dirty);
    assert_eq!(*project(&h), before, "the open project keeps its data");
}

#[test]
fn duplicating_without_data_opens_an_unsaved_copy_with_the_windows_and_no_values() {
    let dir = scratch("dup");
    let mut h = start_with(vec![csv_in(&dir)], dir.join("recent.json"));
    let names: Vec<String> = project(&h).windows.iter().map(|w| w.name().to_string()).collect();
    let ctx = h.ctx.clone();
    h.state_mut().do_pending(Pending::DuplicateNoData, &ctx);
    settle(&mut h);
    let after: Vec<String> = project(&h).windows.iter().map(|w| w.name().to_string()).collect();
    assert_eq!(after, names);
    let book = project(&h).workbooks().next().unwrap().clone();
    assert!(book.sheets[0].columns.iter().all(|c| c.is_empty()));
    assert!(h.state().path.is_none());
    assert!(h.state().doc.dirty, "it is unsaved");
}

#[test]
fn duplicating_a_changed_project_asks_about_the_changes_first() {
    let dir = scratch("ask");
    let mut h = start_with(vec![csv_in(&dir)], dir.join("recent.json"));
    assert!(h.state().doc.dirty, "an import is a change");
    menu_path(&mut h, &["File", "Duplicate Project without Data"]);
    settle(&mut h);
    assert_eq!(h.state().pending, Some(Pending::DuplicateNoData));
    assert!(!project(&h).workbooks().next().unwrap().sheets[0].columns[0].is_empty(), "nothing happened yet");
}
