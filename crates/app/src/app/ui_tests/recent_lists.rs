//! The Most Recently Used menus and Recent Exports (T78, `docs/spec/recent-lists.md`). Every test
//! keeps its lists in a recent file of its own in a temporary folder.

use super::*;
use crate::menus::{Cmd, Entry, WindowKind};
use crate::mru::MruMenu;
use sheaf_io::recent::{ExportKind, RecentList};

fn start_with(paths: Vec<PathBuf>, recent_path: PathBuf, themes_dir: Option<PathBuf>) -> Harness<'static, SheafApp> {
    let startup = Startup { paths, recent_path: Some(recent_path), themes_dir, ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    h.state_mut().panels.set_pinned(PanelId::Explorer, true);
    settle(&mut h);
    h
}

/// The recent file of a test, next to a data file of its own.
fn files(tag: &str) -> (PathBuf, PathBuf) {
    let data = data_file(tag);
    let recent = data.parent().unwrap().join("recent.json");
    (data, recent)
}

fn listed(file: &Path, menu: &str) -> Vec<String> {
    RecentList::load(file).command_keys(menu).to_vec()
}

fn plot_scatter_of_c(h: &mut Harness<'static, SheafApp>) {
    show_window(h, "Book1 - sample.csv");
    click_text(h, "C(Y1)");
    menu_path(h, &["Plot", "Symbol", "Scatter"]);
}

/// The first command of a top-level menu in the bar of this kind (a command the menu offers).
fn first_cmd(kind: WindowKind, menu: MruMenu) -> Cmd {
    fn first(entries: &[Entry]) -> Option<Cmd> {
        entries.iter().find_map(|e| match e {
            Entry::Item(i) => i.cmd.clone(),
            Entry::Submenu(s) => first(&s.entries),
            _ => None,
        })
    }
    let m = crate::menus::bar(kind).iter().find(|m| m.name() == menu.name()).unwrap();
    first(&m.entries).unwrap()
}

#[test]
fn a_plot_made_from_the_menu_is_first_in_most_recently_used_and_the_entry_makes_it_again() {
    let (data, file) = files("mru-plot");
    let mut h = start_with(vec![data], file.clone(), None);
    plot_scatter_of_c(&mut h);
    assert_eq!(project(&h).graphs().count(), 2);
    // The new graph is active and graphs have no Plot menu: back to the workbook.
    show_window(&mut h, "Book1 - sample.csv");
    let keys = listed(&file, "plot");
    assert_eq!(keys.len(), 1, "{keys:?}");
    assert!(keys[0].starts_with("Plot > ") && keys[0].ends_with("Scatter"), "{keys:?}");
    menu_path(&mut h, &["Plot", "Most Recently Used"]);
    assert!(entry_enabled(&h, "1 Scatter"));
    menu_entry(&h, "1 Scatter").click();
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), 3, "choosing the entry plots again");
    assert_eq!(listed(&file, "plot"), keys, "used again: still one entry");
}

#[test]
fn the_list_is_kept_for_the_next_run_and_the_other_menus_stay_empty() {
    let (data, file) = files("mru-restart");
    let mut h = start_with(vec![data.clone()], file.clone(), None);
    plot_scatter_of_c(&mut h);
    drop(h);
    let mut h = start_with(vec![data], file.clone(), None);
    show_window(&mut h, "Book1 - sample.csv");
    menu_path(&mut h, &["Plot", "Most Recently Used"]);
    assert!(h.query_by_label("1 Scatter").is_some(), "back after a restart");
    close_menus(&mut h);
    menu_path(&mut h, &["Statistics", "Most Recently Used Items"]);
    assert!(!entry_enabled(&h, "(none)"), "Statistics has nothing yet");
}

#[test]
fn an_empty_list_shows_a_greyed_placeholder_in_each_menu() {
    let (data, file) = files("mru-empty");
    let mut h = start_with(vec![data], file, None);
    show_window(&mut h, "Book1 - sample.csv");
    for (menu, item) in [
        ("Plot", "Most Recently Used"),
        ("Worksheet", "Most Recently Used Items"),
        ("Analysis", "Most Recently Used Items"),
        ("Statistics", "Most Recently Used Items"),
    ] {
        menu_path(&mut h, &[menu, item]);
        assert!(!entry_enabled(&h, "(none)"), "{menu}");
        close_menus(&mut h);
    }
}

#[test]
fn commands_of_every_menu_are_recorded_and_an_entry_that_cannot_run_is_greyed() {
    let (data, file) = files("mru-menus");
    let mut h = start_with(vec![data], file.clone(), None);
    show_window(&mut h, "Book1 - sample.csv");
    for m in [MruMenu::Worksheet, MruMenu::Analysis, MruMenu::Statistics] {
        let cmd = first_cmd(WindowKind::Workbook, m);
        let ctx = h.ctx.clone();
        h.state_mut().apply(crate::actions::Action::UsedCommand { menu: m, cmd }, &ctx);
        assert_eq!(listed(&file, &m.list_name()).len(), 1, "{m:?}");
    }
    menu_path(&mut h, &["Analysis", "Most Recently Used Items"]);
    let key = listed(&file, "analysis")[0].clone();
    let label = format!("1 {}", key.rsplit(" > ").next().unwrap());
    // Enabled exactly when the command itself can run on the current selection.
    let cmd = first_cmd(WindowKind::Workbook, MruMenu::Analysis);
    let can = crate::menus::resolve(&cmd, &h.state().menu_context()).is_ok();
    assert_eq!(entry_enabled(&h, &label), can);
}

#[test]
fn a_command_the_menus_no_longer_have_is_not_listed() {
    let (data, file) = files("mru-stale");
    std::fs::write(&file, r#"{"commands": {"plot": ["Plot > A Plot That Was Removed"]}}"#).unwrap();
    let mut h = start_with(vec![data], file, None);
    show_window(&mut h, "Book1 - sample.csv");
    menu_path(&mut h, &["Plot", "Most Recently Used"]);
    assert!(!entry_enabled(&h, "(none)"));
    assert!(h.query_by_label_contains("A Plot That Was Removed").is_none());
}

fn export_ready(tag: &str) -> (Harness<'static, SheafApp>, PathBuf, PathBuf) {
    let (data, file) = files(tag);
    let dir = data.parent().unwrap().to_path_buf();
    let mut h = start_with(vec![data], file.clone(), Some(dir.join("themes")));
    show_window(&mut h, "Graph1 - sample");
    (h, file, dir)
}

#[test]
fn a_successful_graph_export_is_listed_and_the_entry_opens_the_export_again() {
    let (mut h, file, dir) = export_ready("recent-export");
    menu_path(&mut h, &["File", "Export Graphs..."]);
    assert!(h.state().export.is_some());
    h.state_mut().export.as_mut().unwrap().set_file_name("figure");
    h.state_mut().export.as_mut().unwrap().set_path(&dir.join("out"));
    settle(&mut h);
    click(&mut h, "Apply");
    assert!(dir.join("out/figure.png").is_file());
    let stored = RecentList::load(&file).exports;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].kind, ExportKind::Graph);
    assert!(stored[0].path.ends_with("figure.png"));
    click(&mut h, "Cancel");
    assert!(h.state().export.is_none());
    menu_path(&mut h, &["File", "Recent Exports"]);
    assert!(h.query_all_by_label_contains("figure.png").next().is_some(), "the file name keeps its extension");
    h.query_all_by_label_contains("figure.png").next().unwrap().click();
    settle(&mut h);
    assert!(h.state().export.is_some(), "the export dialog is open again");
}

#[test]
fn a_failed_or_cancelled_export_is_not_listed_and_a_missing_window_is_reported() {
    let (mut h, file, dir) = export_ready("recent-export-fail");
    menu_path(&mut h, &["File", "Export Graphs..."]);
    click(&mut h, "Cancel");
    assert!(RecentList::load(&file).exports.is_empty());
    // An entry whose window is gone: reported, nothing opens.
    let entry = sheaf_io::recent::ExportEntry {
        path: dir.join("out/gone.png"),
        kind: ExportKind::Graph,
        window: "Graph99 - nothing".into(),
    };
    h.state_mut().recent.push_export(&entry);
    settle(&mut h);
    menu_path(&mut h, &["File", "Recent Exports"]);
    h.query_all_by_label_contains("gone.png").next().unwrap().click();
    settle(&mut h);
    assert!(h.state().export.is_none());
    assert!(h.state().message.as_ref().is_some_and(|(t, _)| t == "Recent Exports"));
}

#[test]
fn recent_exports_is_empty_and_greyed_at_first_and_clear_empties_it() {
    let (mut h, file, dir) = export_ready("recent-export-clear");
    menu_path(&mut h, &["File", "Recent Exports"]);
    assert!(!entry_enabled(&h, "(none)"));
    assert!(!entry_enabled(&h, "Clear Recent Exports"));
    close_menus(&mut h);
    // A workbook has it under File > Export.
    show_window(&mut h, "Book1 - sample.csv");
    menu_path(&mut h, &["File", "Export", "Recent Exports"]);
    assert!(!entry_enabled(&h, "(none)"));
    close_menus(&mut h);
    show_window(&mut h, "Graph1 - sample");
    let entry =
        sheaf_io::recent::ExportEntry { path: dir.join("a.png"), kind: ExportKind::Graph, window: "Graph1".into() };
    h.state_mut().recent.push_export(&entry);
    settle(&mut h);
    menu_path(&mut h, &["File", "Recent Exports"]);
    assert!(entry_enabled(&h, "Clear Recent Exports"));
    menu_entry(&h, "Clear Recent Exports").click();
    settle(&mut h);
    assert!(RecentList::load(&file).exports.is_empty());
}
