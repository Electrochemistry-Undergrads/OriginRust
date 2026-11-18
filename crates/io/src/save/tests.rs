use super::*;
use crate::{open_project, project_file};
use sheaf_core::column::{Column, ColumnData, Designation};
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::project::Window;
use sheaf_core::template::Template;
use sheaf_core::worksheet::{Workbook, Worksheet};
use std::time::{Duration, SystemTime};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-io-save-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn workbook_project() -> Project {
    let mut sheet = Worksheet::new("Data");
    sheet.columns = vec![
        Column::new(ColumnId(0), "A", Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])),
        Column::new(ColumnId(0), "B", Designation::Y, ColumnData::numeric(vec![4.0, 5.0, 6.0])),
    ];
    let mut book = Workbook::new(WindowId(0), "Book1");
    book.sheets = vec![sheet];
    let mut p = Project::new();
    p.adopt_named_workbook(book);
    p
}

fn project_with_graph() -> Project {
    let mut p = workbook_project();
    p.add_graph(Template::Line);
    p
}

/// A project the .opj cannot hold completely: the long name of column B has a character outside
/// Windows-1252, which the writer replaces by a question mark.
fn lossy_project() -> Project {
    let mut p = workbook_project();
    if let Some(Window::Workbook(b)) = p.windows.first_mut() {
        b.sheets[0].columns[1].long_name = "\u{3a9} (ohm)".into();
    }
    p
}

fn long_name_of_b(p: &Project) -> Option<String> {
    p.workbooks().next().and_then(|b| b.sheets[0].columns.get(1)).map(|c| c.long_name.clone())
}

fn set_mtime(path: &Path, when: SystemTime) {
    let f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_modified(when).unwrap();
}

#[test]
fn a_name_without_extension_becomes_opj() {
    let dir = scratch("ext");
    let out = save_project(&workbook_project(), &dir.join("lab")).unwrap();
    assert_eq!(out.path, dir.join("lab.opj"));
    assert!(dir.join("lab.opj").exists());
    assert!(!dir.join("lab").exists());
}

#[test]
fn an_explicit_sheaf_path_stays_a_sheaf_project_without_companion() {
    let dir = scratch("sheaf");
    let out = save_project(&project_with_graph(), &dir.join("lab.sheaf")).unwrap();
    assert!(out.main_written && out.companion.is_none());
    assert_eq!(project_file::load(&dir.join("lab.sheaf")).unwrap().graphs().count(), 1);
    assert!(!project_file::is_companion(&dir.join("lab.sheaf")));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
}

#[test]
fn worksheet_comments_round_trip_in_native_and_opj_companion_saves() {
    let dir = scratch("sheet-comments");
    let mut p = workbook_project();
    let book = p.workbooks().next().unwrap().id;
    let text = "Thermal scan\nKeep baseline at 25 °C.\n";
    p.workbook_mut(book).unwrap().sheets[0].comments = text.into();

    let native = dir.join("lab.sheaf");
    let saved = save_project(&p, &native).unwrap();
    assert!(saved.main_written && saved.companion.is_none());
    let opened = open_project(&native).unwrap();
    assert_eq!(opened.project.workbook(book).unwrap().sheets[0].comments, text);

    let opj = dir.join("lab.opj");
    let saved = save_project(&p, &opj).unwrap();
    assert!(saved.main_written && saved.companion.is_some());
    assert!(saved.warnings.iter().any(|warning| warning.contains("1 sheet(s) have worksheet comments")));
    let opened = open_project(&opj).unwrap();
    assert!(opened.companion_used);
    assert_eq!(opened.project.workbook(book).unwrap().sheets[0].comments, text);
}

#[test]
fn formulas_and_folders_are_written_into_the_opj_and_need_no_companion() {
    use sheaf_core::column::RecalcMode;
    let dir = scratch("formula-folder");
    let opj = dir.join("lab.opj");
    let mut p = workbook_project();
    let b = p.workbooks().next().unwrap().sheets[0].columns[1].id;
    p.set_column_values_with(b, "col(A)+3", None, RecalcMode::Auto).unwrap();
    let book = p.windows[0].id();
    p.root.folders.push(sheaf_core::project::Folder { name: "Runs".into(), windows: vec![book], ..Default::default() });
    let out = save_project(&p, &opj).unwrap();
    assert!(out.main_written && out.companion.is_none(), "{:?}", out.warnings);
    assert!(!dir.join("lab.opj.sheaf").exists());
    let opened = open_project(&opj).unwrap();
    assert!(!opened.companion_used);
    let col = &opened.project.workbooks().next().unwrap().sheets[0].columns[1];
    assert_eq!((col.formula.as_deref(), col.recalc), (Some("col(A)+3"), RecalcMode::Auto));
    assert_eq!(opened.project.root.folders.len(), 1);
    assert_eq!(opened.project.root.folders[0].name, "Runs");
    assert_eq!(opened.project.root.folders[0].windows, [opened.project.windows[0].id()]);
}

#[test]
fn a_graph_is_written_into_the_opj_and_needs_no_companion() {
    let dir = scratch("graph");
    let opj = dir.join("lab.opj");
    let out = save_project(&project_with_graph(), &opj).unwrap();
    assert!(out.main_written && out.companion.is_none(), "{:?}", out.warnings);
    assert!(!dir.join("lab.opj.sheaf").exists());
    let opened = open_project(&opj).unwrap();
    assert!(!opened.companion_used);
    assert_eq!(opened.project.graphs().count(), 1, "the graph is read back from the .opj");
}

#[test]
fn a_project_the_opj_cannot_hold_gets_a_companion_that_opening_uses() {
    let dir = scratch("lossy");
    let opj = dir.join("lab.opj");
    let out = save_project(&lossy_project(), &opj).unwrap();
    assert!(out.main_written);
    let companion = dir.join("lab.opj.sheaf");
    assert_eq!(out.companion.as_deref(), Some(companion.as_path()));
    assert!(out.warnings.iter().any(|w| w.contains("Windows-1252")), "{:?}", out.warnings);
    assert!(out.status().contains("lab.opj.sheaf"));
    assert!(out.details().unwrap().contains("Windows-1252"));
    assert!(project_file::is_companion(&companion));
    // The user's own lab.sheaf is a different name and is not created.
    assert!(!dir.join("lab.sheaf").exists());
    // The opj alone has a question mark; opening it picks the companion, which has the character.
    let opened = open_project(&opj).unwrap();
    assert!(opened.companion_used && !opened.foreign);
    assert_eq!(long_name_of_b(&opened.project).as_deref(), Some("\u{3a9} (ohm)"));
    assert_eq!(opened.save_path.as_deref(), Some(opj.as_path()));
    assert!(opened.warnings[0].contains("lab.opj.sheaf"));
}

#[test]
fn a_workbook_only_project_has_no_companion_and_an_old_marked_companion_is_deleted() {
    let dir = scratch("clean");
    let opj = dir.join("lab.opj");
    save_project(&lossy_project(), &opj).unwrap();
    let companion = dir.join("lab.opj.sheaf");
    assert!(companion.exists());
    let out = save_project(&workbook_project(), &opj).unwrap();
    assert!(out.companion.is_none() && out.warnings.is_empty());
    assert_eq!(out.companion_removed.as_deref(), Some(companion.as_path()));
    assert!(!companion.exists());
    assert!(out.details().is_none());
}

#[test]
fn a_companion_that_sheaf_did_not_write_is_never_deleted() {
    let dir = scratch("foreign-companion");
    let opj = dir.join("lab.opj");
    let theirs = dir.join("lab.opj.sheaf");
    project_file::save(&workbook_project(), &theirs).unwrap();
    let out = save_project(&workbook_project(), &opj).unwrap();
    assert!(out.companion_removed.is_none());
    assert!(theirs.exists());
    // And it is not trusted when opening either.
    let opened = open_project(&opj).unwrap();
    assert!(opened.foreign && !opened.companion_used);
}

#[test]
fn a_refused_project_writes_the_companion_only_and_leaves_the_old_opj_alone() {
    let dir = scratch("refuse");
    let opj = dir.join("lab.opj");
    save_project(&workbook_project(), &opj).unwrap();
    let before = std::fs::read(&opj).unwrap();
    let mut p = workbook_project();
    if let Some(Window::Workbook(b)) = p.windows.first_mut() {
        b.sheets[0].name = "a sheet name that is far too long for Origin to store".into();
    }
    let out = save_project(&p, &opj).unwrap();
    assert!(!out.main_written);
    assert!(out.refusal.is_some());
    assert_eq!(std::fs::read(&opj).unwrap(), before, "the old .opj is untouched");
    let companion = dir.join("lab.opj.sheaf");
    assert!(project_file::is_companion(&companion));
    assert!(out.status().contains("Could not write"));
    assert!(out.details().unwrap().contains("not written"));
    assert_eq!(project_file::load(&companion).unwrap().workbooks().count(), 1);
    assert!(std::fs::read_dir(&dir).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".tmp")));
}

#[test]
fn a_refused_project_with_no_old_opj_writes_no_opj() {
    let dir = scratch("refuse-new");
    let mut p = workbook_project();
    if let Some(Window::Workbook(b)) = p.windows.first_mut() {
        b.sheets[0].name = "a sheet name that is far too long for Origin to store".into();
    }
    let out = save_project(&p, &dir.join("lab.opj")).unwrap();
    assert!(!out.main_written);
    assert!(!dir.join("lab.opj").exists());
    assert!(dir.join("lab.opj.sheaf").exists());
}

#[test]
fn a_newer_opj_is_opened_and_the_older_companion_is_not_used() {
    let dir = scratch("newer");
    let opj = dir.join("lab.opj");
    save_project(&lossy_project(), &opj).unwrap();
    // Origin saved the .opj later.
    set_mtime(&opj, SystemTime::now() + Duration::from_secs(3600));
    let opened = open_project(&opj).unwrap();
    assert!(opened.foreign && !opened.companion_used);
    assert_eq!(long_name_of_b(&opened.project).as_deref(), Some("? (ohm)"), "the .opj itself has the question mark");
    assert!(opened.warnings.iter().any(|w| w.contains("older") && w.contains("not used")), "{:?}", opened.warnings);
}

#[test]
fn opening_a_companion_by_name_saves_to_its_opj() {
    let dir = scratch("by-name");
    let opj = dir.join("lab.opj");
    save_project(&lossy_project(), &opj).unwrap();
    let opened = open_project(&dir.join("lab.opj.sheaf")).unwrap();
    assert_eq!(opened.save_path.as_deref(), Some(opj.as_path()));
    let plain = dir.join("plain.sheaf");
    project_file::save(&workbook_project(), &plain).unwrap();
    assert_eq!(open_project(&plain).unwrap().save_path.as_deref(), Some(plain.as_path()));
}

#[test]
fn companion_names() {
    assert_eq!(companion_path(Path::new("d/x.opj")), PathBuf::from("d/x.opj.sheaf"));
    assert_eq!(companion_target(Path::new("d/x.opj.sheaf")), Some(PathBuf::from("d/x.opj")));
    assert_eq!(companion_target(Path::new("d/x.sheaf")), None);
}

#[test]
fn a_companion_name_held_by_a_users_own_file_is_never_replaced() {
    let dir = scratch("clobber");
    let companion = dir.join("lab.opj.sheaf");
    // A plain .sheaf project (not marked as a companion) sits under the companion's name.
    project_file::save(&workbook_project(), &companion).unwrap();
    let before = std::fs::read(&companion).unwrap();
    let err = save_project(&lossy_project(), &dir.join("lab.opj")).unwrap_err();
    assert!(matches!(err, IoError::CompanionExists { .. }), "{err}");
    assert_eq!(std::fs::read(&companion).unwrap(), before);
    assert!(!dir.join("lab.opj").exists(), "nothing is half-written");
    // A workbook-only save needs no companion and goes through.
    save_project(&workbook_project(), &dir.join("lab.opj")).unwrap();
    assert_eq!(std::fs::read(&companion).unwrap(), before);
}

#[test]
fn the_original_file_is_copied_once_before_it_is_replaced() {
    let dir = scratch("backup");
    let f = dir.join("x.opj");
    std::fs::write(&f, b"original").unwrap();
    let b = backup_original(&f).unwrap().expect("a copy is made");
    assert_eq!(std::fs::read(&b).unwrap(), b"original");
    std::fs::write(&f, b"rewritten").unwrap();
    assert!(backup_original(&f).unwrap().is_none(), "the first copy is kept");
    assert_eq!(std::fs::read(&b).unwrap(), b"original");
    assert!(backup_original(&dir.join("missing.opj")).unwrap().is_none());
}

#[cfg(unix)]
#[test]
fn the_backup_is_never_written_through_a_symlink() {
    let dir = scratch("backup_link");
    let f = dir.join("x.opj");
    std::fs::write(&f, b"original").unwrap();
    let outside = dir.join("outside");
    std::os::unix::fs::symlink(&outside, dir.join("x.opj.bak")).unwrap();
    assert!(backup_original(&f).is_err());
    assert!(!outside.exists(), "nothing was created behind the link");
}

#[cfg(unix)]
#[test]
fn a_link_to_an_existing_file_counts_as_a_backup_already_there_and_its_target_is_not_touched() {
    let dir = scratch("backup_link_file");
    let f = dir.join("x.opj");
    std::fs::write(&f, b"original").unwrap();
    let elsewhere = dir.join("elsewhere");
    std::fs::write(&elsewhere, b"precious").unwrap();
    std::os::unix::fs::symlink(&elsewhere, dir.join("x.opj.bak")).unwrap();
    assert!(backup_original(&f).unwrap().is_none());
    assert_eq!(std::fs::read(&elsewhere).unwrap(), b"precious");
}

#[cfg(unix)]
#[test]
fn the_backup_has_the_permissions_of_the_original_and_an_existing_backup_is_never_overwritten() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("backup_mode");
    let f = dir.join("x.opj");
    std::fs::write(&f, b"original").unwrap();
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o640)).unwrap();
    let b = backup_original(&f).unwrap().expect("a copy is made");
    assert_eq!(std::fs::metadata(&b).unwrap().permissions().mode() & 0o777, 0o640);
    std::fs::write(&b, b"edited by hand").unwrap();
    assert!(backup_original(&f).unwrap().is_none());
    assert_eq!(std::fs::read(&b).unwrap(), b"edited by hand");
}

#[test]
fn a_polar_layer_is_kept_by_the_companion_and_the_opj_warns() {
    let dir = scratch("polar");
    let opj = dir.join("lab.opj");
    let mut p = workbook_project();
    let y = p.workbooks().next().unwrap().sheets[0].columns[1].id;
    let g = p.add_graph(Template::PolarRTheta);
    p.add_plot(g, 0, y).unwrap();
    let out = save_project(&p, &opj).unwrap();
    assert!(out.warnings.iter().any(|w| w.contains("polar")), "{:?}", out.warnings);
    assert!(out.companion.is_some(), "the layer the .opj cannot hold goes to the companion");
    let opened = open_project(&opj).unwrap();
    assert!(opened.companion_used);
    let layer = &opened.project.graphs().next().unwrap().layers[0];
    let polar = layer.coords.polar().expect("the polar layer comes back from the companion");
    assert_eq!(polar.radius, sheaf_core::graph::RadiusColumn::X);
}

#[test]
fn a_ternary_layer_is_kept_by_the_companion_and_the_opj_warns() {
    let dir = scratch("ternary");
    let opj = dir.join("soil.opj");
    let mut p = workbook_project();
    let book = p.workbooks().next().unwrap().id;
    let y = p.workbooks().next().unwrap().sheets[0].columns[1].id;
    p.add_column(book, 0, Designation::Z, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
    p.plot_columns(Template::Ternary, &[y]).unwrap();
    let out = save_project(&p, &opj).unwrap();
    assert!(out.warnings.iter().any(|w| w.contains("ternary")), "{:?}", out.warnings);
    assert!(!out.warnings.iter().any(|w| w.contains("bubble")), "{:?}", out.warnings);
    assert!(out.companion.is_some());
    let opened = open_project(&opj).unwrap();
    assert!(opened.companion_used);
    let layer = &opened.project.graphs().next().unwrap().layers[0];
    assert!(layer.coords.ternary().is_some(), "the ternary layer comes back from the companion");
    assert!(layer.plots[0].size_from.is_some(), "with its Z column");
}

#[test]
fn a_save_to_opju_is_refused_and_writes_nothing() {
    let dir = scratch("opju-refused");
    let path = dir.join("fig.opju");
    let err = save_project(&workbook_project(), &path).unwrap_err();
    assert!(matches!(err, IoError::OpjuWrite { .. }), "{err}");
    assert!(err.to_string().contains(".opj"), "the message points to .opj: {err}");
    assert!(!path.exists(), "no file is written");
    assert!(!dir.join("fig.opju.sheaf").exists(), "and no companion");
}

#[test]
fn page_setup_survives_a_sheaf_project_and_the_opj_companion() {
    use sheaf_core::page_setup::{Margins, Orientation, PageSetup, Paper};
    let dir = scratch("page-setup");
    let mut p = project_with_graph();
    p.page_setup = PageSetup {
        paper: Paper::Letter,
        orientation: Orientation::Landscape,
        margins: Margins { left: 10.0, top: 20.0, right: 30.0, bottom: 40.0 },
    };
    // A .sheaf project.
    let out = save_project(&p, &dir.join("a.sheaf")).unwrap();
    assert!(out.main_written);
    assert_eq!(project_file::load(&dir.join("a.sheaf")).unwrap().page_setup, p.page_setup);
    // An .opj cannot hold it: the companion does, and opening the .opj takes the companion.
    let out = save_project(&p, &dir.join("b.opj")).unwrap();
    assert!(out.companion.is_some(), "{:?}", out.warnings);
    assert!(out.warnings.iter().any(|w| w.contains("Page Setup")), "{:?}", out.warnings);
    let opened = open_project(&dir.join("b.opj")).unwrap();
    assert!(opened.companion_used);
    assert_eq!(opened.project.page_setup, p.page_setup);
    // The default setup alone does not force a companion.
    let out = save_project(&project_with_graph(), &dir.join("c.opj")).unwrap();
    assert!(out.companion.is_none(), "{:?}", out.warnings);
}

#[test]
fn automatic_legend_position_survives_native_and_companion_saves() {
    use sheaf_core::graph::LegendPosition;
    let dir = scratch("automatic-legend");
    let mut p = workbook_project();
    let column = p.workbooks().next().unwrap().sheet().columns[1].id;
    let graph = p.plot_columns(Template::Line, &[column]).unwrap();
    for position in [LegendPosition::Automatic, LegendPosition::TopRight, LegendPosition::Manual { x: 0.7, y: -0.1 }] {
        p.graph_mut(graph).unwrap().layers[0].legend.position = position;
        let native = dir.join("legend.sheaf");
        save_project(&p, &native).unwrap();
        assert_eq!(open_project(&native).unwrap().project, p);
    }
    p.graph_mut(graph).unwrap().layers[0].legend.position = LegendPosition::Automatic;
    let opj = dir.join("legend.opj");
    let out = save_project(&p, &opj).unwrap();
    assert!(out.warnings.iter().any(|w| w.contains("automatic legend anchor")), "{:?}", out.warnings);
    assert!(out.companion.is_some());
    let reopened = open_project(&opj).unwrap();
    assert!(reopened.companion_used);
    assert_eq!(reopened.project, p);
}

#[test]
fn a_project_without_data_reloads_with_its_columns_and_no_values_in_both_formats() {
    let mut p = project_with_graph();
    p.add_graph(Template::Line); // a graph without plots as well
    if let Some(Window::Workbook(b)) = p.windows.first_mut() {
        b.sheets[0].columns[1].long_name = "Signal".into();
        b.sheets[0].columns[1].units = "mV".into();
    }
    let stripped = p.without_data();
    let dir = scratch("nodata");
    for name in ["a.sheaf", "b.opj"] {
        let path = dir.join(name);
        save_project(&stripped, &path).unwrap();
        let back = open_project(&path).unwrap().project;
        let book = back.workbooks().next().unwrap();
        assert_eq!(book.sheets[0].columns.len(), 2, "{name}");
        assert!(book.sheets[0].columns.iter().all(|c| c.is_empty()), "{name}");
        assert_eq!(book.sheets[0].columns[1].long_name, "Signal", "{name}");
        assert_eq!(book.sheets[0].columns[1].units, "mV", "{name}");
        assert_eq!(back.windows.iter().filter(|w| matches!(w, Window::Graph(_))).count(), 2, "{name}");
    }
    // The project that was saved from is not touched.
    assert_eq!(p.workbooks().next().unwrap().sheets[0].columns[0].len(), 3);
}
