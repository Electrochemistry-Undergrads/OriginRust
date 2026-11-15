//! Recent projects and recent imports (T38): the File menus, the clicks and the lists kept between
//! runs. Every test uses a recent file of its own in a temporary folder; none touches the user's
//! settings folder.

use super::*;
use sheaf_io::recent::RecentList;

/// A fresh, empty folder for one test.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-recent-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Resolve the macOS temporary-folder link and use the list's plain Windows path spelling.
    sheaf_io::recent::normalize(&std::fs::canonicalize(&dir).unwrap())
}

/// Starts the application with this recent file and these start-up paths.
fn start_with(paths: Vec<PathBuf>, recent_path: Option<PathBuf>) -> Harness<'static, SheafApp> {
    let startup = Startup { paths, recent_path, ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    h
}

/// A project file with one workbook of this name.
fn project_file_in(dir: &Path, file: &str, book: &str) -> PathBuf {
    let mut p = Project::new();
    p.add_workbook(book);
    let path = dir.join(file);
    sheaf_io::save::save_project(&p, &path).unwrap();
    path
}

/// A small CSV in `dir`.
fn csv_in(dir: &Path, file: &str) -> PathBuf {
    let path = dir.join(file);
    std::fs::write(&path, "t,a\n1,2.5\n2,4.5\n3,6.5\n").unwrap();
    path
}

fn listed(h: &Harness<'static, SheafApp>) -> (Vec<PathBuf>, Vec<PathBuf>) {
    (h.state().recent.projects().to_vec(), h.state().recent.imports().to_vec())
}

fn file_lists(file: &Path) -> RecentList {
    RecentList::load(file)
}

fn open_recent_projects(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["File", "Recent Projects"]);
}

fn open_recent_imports(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["File", "Import", "Recent Imports"]);
}

#[test]
fn a_project_opened_at_start_is_first_in_recent_projects_and_in_the_file() {
    let dir = scratch("open");
    let (old, new) = (project_file_in(&dir, "old.sheaf", "Old"), project_file_in(&dir, "new.sheaf", "New"));
    let file = dir.join("recent.json");
    let mut h = start_with(vec![old.clone()], Some(file.clone()));
    h.state_mut().open_paths(vec![new.clone()], false);
    settle(&mut h);
    assert_eq!(listed(&h).0, [new.clone(), old.clone()], "newest first");
    assert_eq!(file_lists(&file).projects, [new, old], "the file has them too");
    open_recent_projects(&mut h);
    let first = recent_entry(&h, "1", "new").rect().min.y;
    let second = recent_entry(&h, "2", "old").rect().min.y;
    assert!(first < second, "numbered and newest first");
}

#[test]
fn saving_and_saving_as_put_the_project_at_the_top() {
    let dir = scratch("save");
    let file = dir.join("recent.json");
    let mut h = start_with(vec![], Some(file.clone()));
    h.state_mut().doc.apply("New workbook", |p| {
        p.add_workbook("Book");
    });
    let (a, b) = (dir.join("a.sheaf"), dir.join("b.sheaf"));
    assert!(h.state_mut().save_to(a.clone()));
    assert!(h.state_mut().save_to(b.clone()));
    assert!(h.state_mut().save_to(a.clone()));
    assert_eq!(listed(&h).0, [a.clone(), b.clone()], "saving again moves to the top, never repeats");
    assert_eq!(file_lists(&file).projects, [a, b]);
}

#[test]
fn a_project_that_fails_to_open_or_save_is_not_listed() {
    let dir = scratch("fail");
    let file = dir.join("recent.json");
    let broken = dir.join("broken.sheaf");
    std::fs::write(&broken, "this is not a project").unwrap();
    let mut h = start_with(vec![], Some(file.clone()));
    h.state_mut().open_paths(vec![broken], false);
    settle(&mut h);
    assert!(h.state().message.is_some(), "the owner is told");
    assert!(listed(&h).0.is_empty());
    // A folder that does not exist: the save fails.
    assert!(!h.state_mut().save_to(dir.join("no-such-folder").join("x.sheaf")));
    assert!(listed(&h).0.is_empty());
    assert!(!file.exists() || file_lists(&file).projects.is_empty());
}

#[test]
fn clicking_a_recent_project_opens_it() {
    let dir = scratch("click");
    let (a, b) = (project_file_in(&dir, "a.sheaf", "AlphaBook"), project_file_in(&dir, "b.sheaf", "BetaBook"));
    let mut h = start_with(vec![a.clone()], Some(dir.join("recent.json")));
    h.state_mut().open_paths(vec![b], false);
    settle(&mut h);
    assert!(project(&h).workbooks().any(|w| w.long_name == "BetaBook"));
    open_recent_projects(&mut h);
    recent_entry(&h, "2", "a").click();
    settle(&mut h);
    assert!(project(&h).workbooks().any(|w| w.long_name == "AlphaBook"), "a.sheaf is open again");
    assert_eq!(h.state().path.as_deref(), Some(a.as_path()));
    assert_eq!(listed(&h).0[0], a, "and at the top");
}

#[test]
fn clicking_a_recent_project_with_unsaved_changes_asks_first() {
    let dir = scratch("ask");
    let a = project_file_in(&dir, "a.sheaf", "AlphaBook");
    let mut h = start_with(vec![], Some(dir.join("recent.json")));
    h.state_mut().recent.push_project(&a);
    h.state_mut().doc.apply("New workbook", |p| {
        p.add_workbook("Mine");
    });
    settle(&mut h);
    open_recent_projects(&mut h);
    recent_entry(&h, "1", "a").click();
    settle(&mut h);
    assert!(has_label(&h, "The project has unsaved changes."), "the question comes first");
    assert!(project(&h).workbooks().any(|w| w.long_name == "Mine"), "nothing was replaced yet");
    // Cancel keeps everything.
    click(&mut h, "Cancel");
    assert!(project(&h).workbooks().any(|w| w.long_name == "Mine"));
    // Click again and answer "Don't save": the project opens.
    open_recent_projects(&mut h);
    recent_entry(&h, "1", "a").click();
    settle(&mut h);
    click(&mut h, "Don't save");
    assert!(project(&h).workbooks().any(|w| w.long_name == "AlphaBook"));
    assert!(!project(&h).workbooks().any(|w| w.long_name == "Mine"));
}

#[test]
fn a_missing_recent_project_is_reported_and_removed() {
    let dir = scratch("missing");
    let (a, b) = (project_file_in(&dir, "a.sheaf", "A"), project_file_in(&dir, "b.sheaf", "B"));
    let file = dir.join("recent.json");
    let mut h = start_with(vec![a.clone()], Some(file.clone()));
    h.state_mut().open_paths(vec![b.clone()], false);
    settle(&mut h);
    std::fs::remove_file(&a).unwrap();
    open_recent_projects(&mut h);
    recent_entry(&h, "2", "a").click();
    settle(&mut h);
    let (title, text) = h.state().message.clone().expect("the owner is told");
    assert_eq!(title, "Could not open the project");
    assert!(text.contains("a.sheaf") && text.contains("not there"), "{text}");
    assert_eq!(listed(&h).0, std::slice::from_ref(&b), "the entry is gone from the list");
    assert_eq!(file_lists(&file).projects, [b], "and from the file");
    assert!(h.state().logs.messages.iter().any(|m| m.text.contains("Could not open the project")));
}

#[test]
fn a_recent_entry_that_is_a_folder_is_reported_and_never_followed() {
    let dir = scratch("folder");
    let file = dir.join("recent.json");
    let folder = dir.join("looks-like.sheaf");
    std::fs::create_dir(&folder).unwrap();
    let mut h = start_with(vec![], Some(file));
    h.state_mut().recent.push_project(&folder);
    settle(&mut h);
    open_recent_projects(&mut h);
    recent_entry(&h, "1", "looks-like").click();
    settle(&mut h);
    let (_, text) = h.state().message.clone().expect("the owner is told");
    assert!(text.contains("not a file"), "{text}");
    assert!(listed(&h).0.is_empty());
    assert!(h.state().path.is_none());
}

#[test]
fn an_imported_csv_is_first_in_recent_imports_and_clicking_it_imports_again() {
    let dir = scratch("import");
    let csv = csv_in(&dir, "run1.csv");
    let file = dir.join("recent.json");
    let mut h = start_with(vec![], Some(file.clone()));
    h.state_mut().open_paths(vec![csv.clone()], false);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), 1);
    assert_eq!(listed(&h).1, std::slice::from_ref(&csv));
    assert_eq!(file_lists(&file).imports, [csv]);
    assert!(listed(&h).0.is_empty(), "an import is not a project");
    open_recent_imports(&mut h);
    recent_entry(&h, "1", "run1").click();
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), 2, "imported again with the default settings");
    assert_eq!(project(&h).graphs().count(), 2, "and plotted, as a dropped file is");
    assert!(h.state().import.is_none(), "no import window: the defaults are used");
}

#[test]
fn the_import_window_and_a_batch_list_the_files_last_one_first() {
    let dir = scratch("batch");
    let (a, b, c) = (csv_in(&dir, "a.csv"), csv_in(&dir, "b.csv"), csv_in(&dir, "c.csv"));
    let mut h = start_with(vec![], Some(dir.join("recent.json")));
    h.state_mut().open_paths(vec![a.clone(), b.clone(), c.clone()], false);
    settle(&mut h);
    assert_eq!(listed(&h).1, [c, b, a], "pushed in import order, so the last one is first");
}

#[test]
fn a_file_that_cannot_be_imported_is_not_listed() {
    let dir = scratch("badimport");
    let good = csv_in(&dir, "good.csv");
    let missing = dir.join("missing.csv");
    let mut h = start_with(vec![], Some(dir.join("recent.json")));
    h.state_mut().open_paths(vec![good.clone(), missing], false);
    settle(&mut h);
    assert_eq!(listed(&h).1, [good]);
}

#[test]
fn a_missing_recent_import_is_reported_and_removed() {
    let dir = scratch("missing-import");
    let (a, b) = (csv_in(&dir, "a.csv"), csv_in(&dir, "b.csv"));
    let mut h = start_with(vec![], Some(dir.join("recent.json")));
    h.state_mut().open_paths(vec![a.clone(), b.clone()], false);
    settle(&mut h);
    std::fs::remove_file(&a).unwrap();
    let books = project(&h).workbooks().count();
    open_recent_imports(&mut h);
    recent_entry(&h, "2", "a").click();
    settle(&mut h);
    let (title, _) = h.state().message.clone().expect("the owner is told");
    assert_eq!(title, "Could not import the file");
    assert_eq!(project(&h).workbooks().count(), books, "nothing was imported");
    assert_eq!(listed(&h).1, [b]);
}

#[test]
fn clear_empties_one_list_and_leaves_the_other() {
    let dir = scratch("clear");
    let (p, csv) = (project_file_in(&dir, "a.sheaf", "A"), csv_in(&dir, "x.csv"));
    let file = dir.join("recent.json");
    let mut h = start_with(vec![p.clone(), csv.clone()], Some(file.clone()));
    h.state_mut().open_paths(vec![csv.clone()], false);
    settle(&mut h);
    assert_eq!((listed(&h).0.len(), listed(&h).1.len()), (1, 1));
    open_recent_projects(&mut h);
    menu_entry(&h, "Clear Recent Projects").click();
    settle(&mut h);
    assert_eq!(listed(&h), (vec![], vec![csv.clone()]));
    assert_eq!(file_lists(&file), RecentList { projects: vec![], imports: vec![csv] });
    open_recent_imports(&mut h);
    menu_entry(&h, "Clear Recent Imports").click();
    settle(&mut h);
    assert_eq!(listed(&h), (vec![], vec![]));
    assert_eq!(file_lists(&file), RecentList::default());
}

#[test]
fn an_empty_list_shows_a_greyed_none_and_a_greyed_clear() {
    let dir = scratch("empty");
    let mut h = start_with(vec![], Some(dir.join("recent.json")));
    open_recent_projects(&mut h);
    assert!(!entry_enabled(&h, "(none)"));
    assert!(!entry_enabled(&h, "Clear Recent Projects"));
    close_menus(&mut h);
    open_recent_imports(&mut h);
    assert!(!entry_enabled(&h, "(none)"));
    assert!(!entry_enabled(&h, "Clear Recent Imports"));
}

#[test]
fn recent_books_graphs_and_exports_stay_grey() {
    let dir = scratch("grey");
    let mut h = start_with(vec![csv_in(&dir, "g.csv")], Some(dir.join("recent.json")));
    // The graph is active: its File menu lists Recent Graphs and Recent Exports on the top level.
    menu_entry(&h, "File").click();
    settle(&mut h);
    for label in ["Recent Books", "Recent Graphs", "Recent Exports"] {
        assert!(!entry_enabled(&h, label), "{label} stays grey");
    }
    assert!(entry_enabled(&h, "Recent Projects"));
    close_menus(&mut h);
    // The workbook's File menu has Recent Exports inside Export.
    click(&mut h, "Title Book1 - g");
    menu_path(&mut h, &["File", "Export"]);
    assert!(!entry_enabled(&h, "Recent Exports"));
    assert!(!entry_enabled(&h, "Recent Books"));
}

#[test]
fn the_file_is_read_when_the_file_menu_opens_not_every_frame() {
    let dir = scratch("reload");
    let file = dir.join("recent.json");
    let (a, b, c) = (project_file_in(&dir, "a.sheaf", "A"), project_file_in(&dir, "b.sheaf", "B"), dir.join("c.sheaf"));
    let mut h = start_with(vec![a.clone()], Some(file.clone()));
    // Another instance adds an entry: this one does not look until the menu opens.
    sheaf_io::recent::record_project(&file, &b).unwrap();
    for _ in 0..20 {
        h.step();
    }
    assert_eq!(listed(&h).0, std::slice::from_ref(&a), "not read while no menu is open");
    menu_entry(&h, "File").click();
    settle(&mut h);
    assert_eq!(listed(&h).0, [b.clone(), a.clone()], "read when File opened");
    // While the menu stays open the file is not read again.
    sheaf_io::recent::record_project(&file, &c).unwrap();
    for _ in 0..20 {
        h.step();
    }
    assert_eq!(listed(&h).0, [b.clone(), a.clone()], "once per opening");
    close_menus(&mut h);
    menu_entry(&h, "File").click();
    settle(&mut h);
    assert_eq!(listed(&h).0, [c, b, a], "read again at the next opening");
}

#[test]
fn a_change_here_keeps_the_entries_another_instance_added() {
    let dir = scratch("merge");
    let file = dir.join("recent.json");
    let (a, b) = (project_file_in(&dir, "a.sheaf", "A"), project_file_in(&dir, "b.sheaf", "B"));
    let mut h = start_with(vec![], Some(file.clone()));
    sheaf_io::recent::record_project(&file, &a).unwrap();
    h.state_mut().open_paths(vec![b.clone()], false);
    settle(&mut h);
    assert_eq!(listed(&h).0, [b.clone(), a.clone()], "this instance did not wipe the other's entry");
    assert_eq!(file_lists(&file).projects, [b, a]);
}

#[test]
fn the_lists_survive_a_restart() {
    let dir = scratch("restart");
    let file = dir.join("recent.json");
    let (p, csv) = (project_file_in(&dir, "a.sheaf", "A"), csv_in(&dir, "x.csv"));
    {
        let mut h = start_with(vec![p.clone()], Some(file.clone()));
        h.state_mut().open_paths(vec![csv.clone()], false);
        settle(&mut h);
    }
    let h = start_with(vec![], Some(file));
    assert_eq!(listed(&h), (vec![p], vec![csv]));
}

#[test]
fn without_a_recent_file_nothing_is_kept_and_the_menus_are_empty() {
    let dir = scratch("none");
    let mut h = start_with(vec![csv_in(&dir, "x.csv")], None);
    assert_eq!(listed(&h), (vec![], vec![]));
    open_recent_imports(&mut h);
    assert!(!entry_enabled(&h, "(none)"));
}

#[test]
fn entries_are_numbered_to_f_and_a_sixteenth_drops_the_first_as_in_origin() {
    let dir = scratch("ten");
    let mut h = start_with(vec![], Some(dir.join("recent.json")));
    let files: Vec<PathBuf> = (1..=16).map(|n| csv_in(&dir, &format!("f{n:02}.csv"))).collect();
    for f in &files {
        h.state_mut().open_paths(vec![f.clone()], false);
    }
    settle(&mut h);
    assert_eq!(listed(&h).1.len(), 15, "fifteen entries at most, as Origin's Recent Projects");
    assert!(!listed(&h).1.contains(&files[0]), "the oldest dropped out");
    open_recent_imports(&mut h);
    let _ = recent_entry(&h, "A", "f07");
    let _ = recent_entry(&h, "F", "f02");
}

#[test]
fn an_opj_save_the_writer_refused_lists_the_companion_not_the_missing_opj() {
    let dir = scratch("refused");
    let file = dir.join("recent.json");
    let mut h = start_with(vec![], Some(file.clone()));
    h.state_mut().doc.apply("New workbook", |p| {
        let id = p.add_workbook("Book");
        // A sheet name outside Windows-1252: the .opj writer refuses it.
        p.workbook_mut(id).unwrap().sheets[0].name = "\u{65e5}\u{672c}".to_string();
    });
    let opj = dir.join("Data.opj");
    assert!(h.state_mut().save_to(opj.clone()));
    assert!(!opj.exists(), "the writer refused the name Origin cannot store, so the .opj was never written");
    let companion = dir.join("Data.opj.sheaf");
    assert!(companion.exists());
    assert_eq!(listed(&h).0, std::slice::from_ref(&companion));
    h.state_mut().message = None;
    h.state_mut().follow_recent(companion.clone(), RecentKind::Projects);
    settle(&mut h);
    assert!(h.state().message.as_ref().is_none_or(|m| !m.1.contains("not there")));
    assert_eq!(listed(&h).0, [companion]);
}

#[cfg(unix)]
#[test]
fn an_import_whose_name_is_not_valid_text_is_listed_under_its_real_name() {
    use std::os::unix::ffi::OsStringExt;
    let dir = scratch("rawname");
    let file = dir.join("recent.json");
    let raw = dir.join(std::ffi::OsString::from_vec(b"caf\xe9.csv".to_vec()));
    if std::fs::write(&raw, "t,a\n1,2.5\n2,4.5\n").is_err() {
        return; // The file system does not take such a name (macOS APFS refuses it).
    }
    let mut h = start_with(vec![], Some(file));
    h.state_mut().open_paths(vec![raw.clone()], false);
    settle(&mut h);
    assert_eq!(listed(&h).1, [raw]);
}

/// The entry numbered `n` whose text (the path without its extension, perhaps shortened in the
/// middle) ends with `stem`.
fn recent_entry<'a>(h: &'a Harness<'static, SheafApp>, n: &str, stem: &'a str) -> egui_kittest::Node<'a> {
    h.query_all_by_label_contains(stem)
        .find(|node| {
            let label = node.accesskit_node().label().unwrap_or_default().to_string();
            label.starts_with(&format!("{n} ")) && label.ends_with(stem)
        })
        .unwrap_or_else(|| panic!("no recent entry {n} ... {stem}"))
}
