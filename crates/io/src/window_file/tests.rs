use super::*;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation, RecalcMode};
use std::io::Write;

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sheaf-io-winfile-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A workbook with numbers, text and mixed cells, units and a long name.
fn rich_book(p: &mut Project, long: &str) -> (WindowId, ColumnId, ColumnId, ColumnId) {
    let b = p.add_workbook(long);
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, f64::NAN, 3.5])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, f64::INFINITY, 9.0, -0.0])).unwrap();
    let t = p
        .add_column(
            b,
            0,
            Designation::Label,
            ColumnData::text(vec!["a".into(), "".into(), "q\"\n".into(), "\u{e9}".into()]),
        )
        .unwrap();
    p.column_mut(y).unwrap().units = "mA".into();
    p.column_mut(y).unwrap().long_name = "current".into();
    (b, x, y, t)
}

fn column_values(p: &Project, id: ColumnId) -> Vec<f64> {
    p.numeric(id).unwrap().to_vec()
}

#[test]
fn a_workbook_file_round_trips_with_all_its_data() {
    let dir = Scratch::new("book");
    let mut p = Project::new();
    let (b, x, y, t) = rich_book(&mut p, "demo \u{3b1}");
    rich_book(&mut p, "other");
    let path = dir.file("demo.sheafbook");
    let saved = save_window(&p, b, &path).unwrap();
    assert_eq!((saved.kind, saved.windows), (WindowFileKind::Book, 1));
    assert!(saved.notes.is_empty());

    let back = load_window(&path).unwrap();
    assert_eq!((back.kind, back.primary, back.project.windows.len()), (WindowFileKind::Book, 0, 1));
    let original = p.workbook(b).unwrap();
    assert_eq!(back.project.workbook(b).unwrap(), original, "the workbook is the same, ids and data included");
    assert_eq!(column_values(&back.project, y).len(), 4);
    assert!(column_values(&back.project, x)[2].is_nan());
    assert_eq!(back.project.column(t).unwrap().data, p.column(t).unwrap().data);
}

#[test]
fn a_graph_file_holds_its_two_source_books_and_opens_with_its_data() {
    let dir = Scratch::new("graph");
    let mut p = Project::new();
    let (_b1, _, y1, _) = rich_book(&mut p, "first");
    let (_b2, _, y2, _) = rich_book(&mut p, "second");
    let (_b3, _, _, _) = rich_book(&mut p, "not read by the graph");
    let g = p.plot_columns(Template::Line, &[y1, y2]).unwrap();
    let path = dir.file("two.sheafgraph");
    let saved = save_window(&p, g, &path).unwrap();
    assert_eq!((saved.kind, saved.windows), (WindowFileKind::Graph, 3));

    let back = load_window(&path).unwrap();
    assert_eq!(back.kind, WindowFileKind::Graph);
    assert!(matches!(back.project.windows[back.primary], Window::Graph(_)));
    assert_eq!(back.project.workbooks().count(), 2);

    // Into another project that already has windows of the same names: renamed, still the same plot.
    let mut other = Project::new();
    other.add_workbook("mine");
    other.add_workbook("also mine");
    other.add_graph(Template::Line);
    let (ids, dropped) = other.append_windows(back.project);
    assert_eq!((ids.len(), dropped), (3, 0));
    let names: std::collections::HashSet<String> = other.windows.iter().map(|w| w.name().to_lowercase()).collect();
    assert_eq!(names.len(), other.windows.len(), "names stay unique");
    let new_graph = other.graph(ids[back.primary]).unwrap();
    let plotted: Vec<Vec<f64>> = new_graph.layers[0].plots.iter().map(|plot| column_values(&other, plot.y)).collect();
    assert_eq!(plotted.len(), 2);
    assert_eq!(plotted[0], column_values(&p, y1));
    assert_eq!(plotted[1], column_values(&p, y2));
}

#[test]
fn a_formula_that_reads_a_book_outside_the_window_is_dropped_with_a_note() {
    let dir = Scratch::new("links");
    let mut p = Project::new();
    let (a, _, _, _) = rich_book(&mut p, "");
    let (_b, _, _, _) = rich_book(&mut p, "");
    let c = p.add_column(a, 0, Designation::Y, ColumnData::numeric(vec![0.0; 4])).unwrap();
    p.set_column_values_with(c, "[Book2]Sheet1!col(B) + 1", None, RecalcMode::Auto).unwrap();
    let before = p.column(c).unwrap().data.clone();
    let path = dir.file("a.sheafbook");
    let saved = save_window(&p, a, &path).unwrap();
    assert_eq!(saved.notes.len(), 1, "{:?}", saved.notes);
    assert!(saved.notes[0].contains("Book2"));
    let back = load_window(&path).unwrap();
    let col = back.project.column(c).unwrap();
    assert_eq!((col.formula.clone(), col.recalc), (None, RecalcMode::None));
    assert_eq!(col.data, before, "the values stay");
}

#[test]
fn a_file_of_garbage_or_a_cut_file_is_an_error_not_a_panic() {
    let dir = Scratch::new("corrupt");
    let junk = dir.file("junk.sheafbook");
    std::fs::write(&junk, b"this is not a zip archive at all").unwrap();
    assert!(matches!(load_window(&junk), Err(IoError::Zip { .. })));
    assert!(!is_window_file(&dir.file("missing.txt")));
    assert_eq!(window_file_kind(&junk), None);

    let mut p = Project::new();
    let (b, _, _, _) = rich_book(&mut p, "");
    let good = dir.file("good.sheafbook");
    save_window(&p, b, &good).unwrap();
    let bytes = std::fs::read(&good).unwrap();
    let cut = dir.file("cut.sheafbook");
    std::fs::write(&cut, &bytes[..bytes.len() / 2]).unwrap();
    assert!(load_window(&cut).is_err());
    assert!(matches!(load_window(&dir.file("absent.sheafbook")), Err(IoError::Io { .. })));
}

/// An archive with the given manifest and project descriptions.
fn archive(path: &Path, manifest: &str, project: Option<&str>) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(manifest.as_bytes()).unwrap();
    if let Some(project) = project {
        zip.start_file("project.json", options).unwrap();
        zip.write_all(project.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn a_marker_that_does_not_fit_the_content_is_reported() {
    let dir = Scratch::new("marker");
    let empty = serde_json::to_string(&Project::new()).unwrap();
    let manifest = |marker: &str| format!(r#"{{"format":"sheaf-project","version":1,"window":"{marker}"}}"#);

    let unknown = dir.file("unknown.sheafbook");
    archive(&unknown, &manifest("chart"), Some(&empty));
    let e = load_window(&unknown).unwrap_err();
    assert!(matches!(e, IoError::Corrupt { .. }) && e.to_string().contains("chart"), "{e}");

    let nothing = dir.file("nothing.sheafgraph");
    archive(&nothing, &manifest("graph"), Some(&empty));
    let e = load_window(&nothing).unwrap_err();
    assert!(matches!(e, IoError::Corrupt { .. }), "{e}");

    let no_project = dir.file("noproject.sheafbook");
    archive(&no_project, &manifest("workbook"), None);
    assert!(matches!(load_window(&no_project), Err(IoError::Corrupt { .. })));

    let newer = dir.file("newer.sheafbook");
    archive(&newer, r#"{"format":"sheaf-project","version":99,"window":"workbook"}"#, Some(&empty));
    assert!(matches!(load_window(&newer), Err(IoError::UnsupportedVersion { .. })));

    let foreign = dir.file("foreign.sheafbook");
    archive(&foreign, r#"{"format":"something-else","version":1,"window":"workbook"}"#, Some(&empty));
    assert!(load_window(&foreign).is_err());
}

#[test]
fn a_missing_column_data_member_is_an_error_naming_the_column() {
    let dir = Scratch::new("nodata");
    let mut p = Project::new();
    let (b, _, _, _) = rich_book(&mut p, "");
    let extract = p.extract_window(b).unwrap();
    let json = serde_json::to_string(&extract.project).unwrap();
    let path = dir.file("nodata.sheafbook");
    archive(&path, r#"{"format":"sheaf-project","version":1,"window":"workbook"}"#, Some(&json));
    let error = load_window(&path).expect_err("a column without its data member is an error");
    assert!(matches!(error, IoError::MissingColumnData { .. }), "{error:?}");
    let shown = error.to_string();
    assert!(shown.contains("nodata.sheafbook"), "the error names the file: {shown}");
    let IoError::MissingColumnData { column, .. } = &error else { unreachable!() };
    assert!(!column.is_empty() && shown.contains(column.as_str()), "the error names the column: {shown}");
}

#[test]
fn a_window_file_is_never_taken_for_a_project_and_a_project_never_for_a_window_file() {
    let dir = Scratch::new("never");
    let mut p = Project::new();
    let (b, _, _, _) = rich_book(&mut p, "");
    let window = dir.file("w.sheafbook");
    save_window(&p, b, &window).unwrap();
    let project = dir.file("p.sheaf");
    project_file::save(&p, &project).unwrap();

    assert!(matches!(project_file::load(&window), Err(IoError::IsAWindowFile { .. })));
    assert!(matches!(crate::open_project(&window), Err(IoError::IsAWindowFile { .. })));
    assert!(matches!(load_window(&project), Err(IoError::NotAWindowFile { .. })));
    assert!(!crate::is_project_file(&window), "by extension");
    assert!(is_window_file(&window) && !is_window_file(&project));
    assert_eq!(window_file_kind(&window), Some(WindowFileKind::Book));
    assert_eq!(window_file_kind(&project), None);

    // A window file renamed to .sheaf is still a window file.
    let renamed = dir.file("renamed.sheaf");
    std::fs::copy(&window, &renamed).unwrap();
    assert!(is_window_file(&renamed) && !crate::is_project_file(&renamed));
    assert!(crate::open_project(&renamed).is_err());
    assert_eq!(load_window(&renamed).unwrap().kind, WindowFileKind::Book);

    // A project renamed to .sheafbook is not accepted as a window file.
    let disguised = dir.file("disguised.sheafbook");
    std::fs::copy(&project, &disguised).unwrap();
    assert!(matches!(load_window(&disguised), Err(IoError::NotAWindowFile { .. })));

    // The manifest of a project does not carry the window key; a companion neither.
    let manifest = |path: &Path| {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut text = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("manifest.json").unwrap(), &mut text).unwrap();
        text
    };
    assert!(!manifest(&project).contains("window"));
    assert!(manifest(&window).contains(r#""window": "workbook""#));
    assert!(!project_file::is_companion(&window));
}

#[test]
fn the_extension_names_the_kind_in_any_letter_case() {
    assert_eq!(WindowFileKind::from_extension(Path::new("a.SheafBook")), Some(WindowFileKind::Book));
    assert_eq!(WindowFileKind::from_extension(Path::new("a.sheafgraph")), Some(WindowFileKind::Graph));
    assert_eq!(WindowFileKind::from_extension(Path::new("a.sheaf")), None);
    assert_eq!(WindowFileKind::from_extension(Path::new("a")), None);
    assert!(is_window_file(Path::new("nowhere/a.SHEAFGRAPH")));
    assert!(!is_window_file(Path::new("nowhere/a.sheaf")), "an unreadable .sheaf is no window file");
    assert!(!is_window_file(Path::new("nowhere/a.opj")));
    for kind in [WindowFileKind::Book, WindowFileKind::Graph] {
        assert_eq!(WindowFileKind::from_marker(kind.marker()), Some(kind));
    }
}

#[test]
fn a_name_without_the_extension_of_its_kind_gets_it_added() {
    let book = default_extension(Path::new("/x/demo"), WindowFileKind::Book);
    assert_eq!(book, Path::new("/x/demo.sheafbook"));
    assert_eq!(default_extension(Path::new("/x/demo.v2"), WindowFileKind::Graph), Path::new("/x/demo.v2.sheafgraph"));
    assert_eq!(default_extension(Path::new("/x/g"), WindowFileKind::Graph), Path::new("/x/g.sheafgraph"));
    assert_eq!(
        default_extension(Path::new("/x/keep.sheaf"), WindowFileKind::Graph),
        Path::new("/x/keep.sheaf.sheafgraph"),
        "a project name is never reused"
    );
    assert_eq!(
        default_extension(Path::new("/x/g.sheafbook"), WindowFileKind::Graph),
        Path::new("/x/g.sheafbook.sheafgraph"),
        "the extension of the other kind is not reused"
    );
    assert_eq!(default_extension(Path::new("/x/ok.SheafGraph"), WindowFileKind::Graph), Path::new("/x/ok.SheafGraph"));
}

#[test]
fn saving_replaces_an_older_file_and_leaves_no_temporary_file() {
    let dir = Scratch::new("atomic");
    let mut p = Project::new();
    let (b, _, y, _) = rich_book(&mut p, "");
    let path = dir.file("w.sheafbook");
    std::fs::write(&path, b"old").unwrap();
    save_window(&p, b, &path).unwrap();
    p.column_mut(y).unwrap().data = ColumnData::numeric(vec![42.0]);
    save_window(&p, b, &path).unwrap();
    assert_eq!(column_values(&load_window(&path).unwrap().project, y), [42.0]);
    let names: Vec<String> =
        std::fs::read_dir(&dir.0).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["w.sheafbook"]);
}

#[test]
fn a_failed_save_leaves_the_old_file_and_no_temporary_file() {
    let dir = Scratch::new("failed");
    let mut p = Project::new();
    let (b, _, _, _) = rich_book(&mut p, "");
    let path = dir.file("w.sheafbook");
    save_window(&p, b, &path).unwrap();
    let before = std::fs::read(&path).unwrap();
    let nowhere = dir.file("no-folder/w.sheafbook");
    assert!(save_window(&p, b, &nowhere).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(matches!(save_window(&p, WindowId(999), &path), Err(IoError::NoSuchWindow { id: 999 })));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn opening_the_same_window_file_twice_adds_two_windows_with_different_names() {
    let dir = Scratch::new("twice");
    let mut p = Project::new();
    let (b, _, y, _) = rich_book(&mut p, "");
    let path = dir.file("w.sheafbook");
    save_window(&p, b, &path).unwrap();
    let mut target = Project::new();
    for _ in 0..2 {
        let file = load_window(&path).unwrap();
        target.append_windows(file.project);
    }
    assert_eq!(target.windows.len(), 2);
    assert_ne!(target.windows[0].name(), target.windows[1].name());
    let ids: Vec<ColumnId> = target.workbooks().map(|w| w.sheets[0].columns[1].id).collect();
    assert_ne!(ids[0], ids[1], "the columns have their own ids");
    assert_eq!(column_values(&target, ids[1]).len(), column_values(&p, y).len());
}
