//! Excel files on the command line: `sheaf import`, `sheaf info`, `sheaf export` and `sheaf export-data`.

use sheaf_core::Project;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-excel-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A workbook of the Excel fixtures of `sheaf-io`.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../io/tests/fixtures/excel").join(name)
}

fn sheaf(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// Imports `file` with extra flags; the project is empty when the command failed.
fn import(dir: &Path, file: &Path, extra: &[&str]) -> (bool, String, Project) {
    let out = dir.join("out.sheaf");
    let _ = std::fs::remove_file(&out);
    let mut args = vec!["import", s(file), "--into", s(&out)];
    args.extend_from_slice(extra);
    let o = sheaf(&args);
    let project = if o.status.success() { sheaf_io::project_file::load(&out).unwrap() } else { Project::new() };
    (o.status.success(), text(&o.stderr), project)
}

fn sheet_names(p: &Project) -> Vec<String> {
    p.workbooks().next().unwrap().sheets.iter().map(|s| s.name.clone()).collect()
}

#[test]
fn an_excel_file_becomes_one_workbook_with_a_sheet_per_excel_sheet() {
    let dir = scratch("sheets");
    let (ok, err, p) = import(&dir, &fixture("legacy.xls"), &[]);
    assert!(ok, "{err}");
    assert_eq!(p.workbooks().count(), 1);
    assert_eq!(sheet_names(&p), ["Old", "Second"]);
    assert_eq!(p.workbooks().next().unwrap().long_name, "legacy.xls");
    let (ok, err, p) = import(&dir, &fixture("basic.xlsx"), &[]);
    assert!(ok, "{err}");
    assert_eq!(p.workbooks().next().unwrap().sheets[0].columns[0].long_name, "Potential");
}

#[test]
fn sheets_are_chosen_by_name_or_position_and_hidden_ones_read_unless_skipped() {
    let dir = scratch("choose");
    let (ok, err, p) = import(&dir, &fixture("legacy.xls"), &["--sheets", "Second,1"]);
    assert!(ok, "{err}");
    assert_eq!(sheet_names(&p), ["Second", "Old"]);

    // Hidden sheets are read, as Origin's impExcel reads them.
    let (ok, err, p) = import(&dir, &fixture("hidden.xlsx"), &[]);
    assert!(ok, "{err}");
    assert_eq!(sheet_names(&p), ["Visible", "Hidden", "Secret"]);
    let (ok, err, p) = import(&dir, &fixture("hidden.xlsx"), &["--hidden-sheets"]);
    assert!(ok, "{err}");
    assert_eq!(sheet_names(&p), ["Visible", "Hidden", "Secret"]);
    let (ok, err, p) = import(&dir, &fixture("hidden.xlsx"), &["--skip-hidden-sheets"]);
    assert!(ok, "{err}");
    assert_eq!(sheet_names(&p), ["Visible"]);
    assert!(err.contains("Hidden"), "the skipped sheet is reported: {err}");
    let (ok, err, _) = import(&dir, &fixture("hidden.xlsx"), &["--hidden-sheets", "--skip-hidden-sheets"]);
    assert!(!ok, "the two switches contradict each other: {err}");
    let (ok, err, p) = import(&dir, &fixture("hidden.xlsx"), &["--sheets", "hidden"]);
    assert!(ok, "{err}");
    assert_eq!(sheet_names(&p), ["Hidden"]);

    let (ok, err, _) = import(&dir, &fixture("legacy.xls"), &["--sheets", "Nope"]);
    assert!(!ok && err.contains("no sheet 'Nope'") && err.contains("Old, Second"), "{err}");
}

#[test]
fn the_header_and_row_flags_of_text_files_apply_to_excel_files() {
    let dir = scratch("headers");
    let (ok, err, p) = import(&dir, &fixture("basic.xlsx"), &["--no-rename-columns", "--sheets", "Data"]);
    assert!(ok, "{err}");
    let sheet = &p.workbooks().next().unwrap().sheets[0];
    assert_eq!(sheet.columns[0].long_name, "");
    assert_eq!(sheet.row_count(), 5, "the header rows are still skipped");

    let (ok, err, p) =
        import(&dir, &fixture("basic.xlsx"), &["--sheets", "Data", "--first-row", "2", "--last-row", "4"]);
    assert!(ok, "{err}");
    let sheet = &p.workbooks().next().unwrap().sheets[0];
    assert_eq!(sheet.row_count(), 3);
    assert_eq!(sheet.columns[0].numeric().unwrap(), [-0.1, 0.0, 0.1]);
    assert_eq!(sheet.columns[0].long_name, "Potential");
}

#[test]
fn excel_files_in_a_folder_and_in_the_other_layouts_are_imported() {
    let dir = scratch("folder");
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::copy(fixture("legacy.xls"), data.join("a.xls")).unwrap();
    std::fs::write(data.join("b.csv"), "t,v\n1,2\n2,3\n3,4\n4,5\n").unwrap();
    std::fs::write(data.join("~$lock.xlsx"), "not a workbook").unwrap();
    let (ok, err, p) = import(&dir, &data, &[]);
    assert!(ok, "{err}");
    assert_eq!(p.workbooks().count(), 2, "the Excel file and the text file, not the lock file");
    let (ok, err, p) = import(&dir, &data, &["--mode", "sheets", "--book-name", "all"]);
    assert!(ok, "{err}");
    assert_eq!(p.workbooks().count(), 1);
    assert!(p.workbooks().next().unwrap().sheets.len() >= 2);
}

#[test]
fn info_reads_an_excel_file() {
    let o = sheaf(&["info", s(&fixture("legacy.xls"))]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    let out = text(&o.stdout);
    assert!(out.contains("format: excel") && out.contains("sheet Old") && out.contains("sheet Second"), "{out}");
}

#[test]
fn a_sheet_exports_to_xlsx_and_reads_back() {
    let dir = scratch("roundtrip");
    let csv = dir.join("run.csv");
    std::fs::write(&csv, "Time,Current\ns,A\n0,0.5\n1,1.25\n2,1e-9\n3,4\n").unwrap();
    let proj = dir.join("run.sheaf");
    assert!(
        sheaf(&[
            "import",
            s(&csv),
            "--into",
            s(&proj),
            "--header-lines",
            "2",
            "--long-name-line",
            "1",
            "--units-line",
            "2"
        ])
        .status
        .success()
    );
    let xlsx = dir.join("run.xlsx");
    let o = sheaf(&["export-data", s(&proj), "-o", s(&xlsx)]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(text(&o.stdout).contains("wrote"));
    let (ok, err, p) = import(&dir, &xlsx, &[]);
    assert!(ok, "{err}");
    let sheet = &p.workbooks().next().unwrap().sheets[0];
    assert_eq!((sheet.columns[0].long_name.as_str(), sheet.columns[0].units.as_str()), ("Time", "s"));
    assert_eq!(sheet.columns[1].numeric().unwrap(), [0.5, 1.25, 1e-9, 4.0]);

    // Without the label rows the file starts with the data.
    let bare = dir.join("bare.xlsx");
    let o = sheaf(&["export-data", s(&proj), "-o", s(&bare), "--no-long-names", "--no-units"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    let (ok, err, p) = import(&dir, &bare, &[]);
    assert!(ok, "{err}");
    assert_eq!(p.workbooks().next().unwrap().sheets[0].columns[1].long_name, "");
}

#[test]
fn export_writes_every_sheet_with_all_sheets_and_takes_a_data_file() {
    let dir = scratch("export");
    let xlsx = dir.join("legacy-copy.xlsx");
    let o = sheaf(&["export", s(&fixture("legacy.xls")), "-o", s(&xlsx), "--all-sheets"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    let (ok, err, p) = import(&dir, &xlsx, &[]);
    assert!(ok, "{err}");
    assert_eq!(sheet_names(&p), ["Old", "Second"]);

    let one = dir.join("one.xlsx");
    let o = sheaf(&["export", s(&fixture("legacy.xls")), "-o", s(&one), "--sheet", "Second"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    let (ok, err, p) = import(&dir, &one, &[]);
    assert!(ok, "{err}");
    assert_eq!(sheet_names(&p), ["Second"]);

    let o = sheaf(&["export", s(&fixture("legacy.xls")), "-o", s(&one), "--all-sheets", "--sheet", "Second"]);
    assert!(!o.status.success(), "--all-sheets and --sheet exclude each other");
}

#[test]
fn an_xlsx_export_never_replaces_a_file_that_is_not_a_workbook() {
    let dir = scratch("exists");
    let target = dir.join("notes.xlsx");
    std::fs::write(&target, "my notes").unwrap();
    let o = sheaf(&["export", s(&fixture("legacy.xls")), "-o", s(&target)]);
    assert!(!o.status.success());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "my notes");
    // An xlsx that is there already is replaced.
    let again = dir.join("again.xlsx");
    for _ in 0..2 {
        let o = sheaf(&["export", s(&fixture("legacy.xls")), "-o", s(&again)]);
        assert!(o.status.success(), "{}", text(&o.stderr));
    }
}
