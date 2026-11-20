//! `sheaf import` with a multi-character delimiter, fixed-width columns and the Data Import Options.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-text-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn import(dir: &Path, file: &str, text: &str, extra: &[&str]) -> (bool, String, sheaf_core::Project) {
    std::fs::write(dir.join(file), text).unwrap();
    let out = dir.join("out.sheaf");
    let mut args: Vec<String> = vec![
        "import".into(),
        dir.join(file).to_string_lossy().into_owned(),
        "--into".into(),
        out.to_string_lossy().into_owned(),
    ];
    args.extend(extra.iter().map(|s| s.to_string()));
    let o = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(&args).output().expect("the program starts");
    let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
    let project =
        if o.status.success() { sheaf_io::project_file::load(&out).unwrap() } else { sheaf_core::Project::new() };
    (o.status.success(), stderr, project)
}

#[test]
fn a_string_delimiter_and_fixed_widths_are_read() {
    let dir = scratch("delims");
    let (ok, err, p) = import(&dir, "pipes.txt", "t::i\n1::2\n2::3\n3::4\n4::5\n", &["--delimiter", "::"]);
    assert!(ok, "{err}");
    let sheet = &p.workbooks().next().unwrap().sheets[0];
    assert_eq!(sheet.columns.len(), 2);
    assert_eq!(sheet.columns[1].long_name, "i");

    let (ok, err, p) = import(&dir, "fixed.txt", "  t  i\n  1  2\n  2  3\n  3  4\n  4  5\n", &["--fixed-width", "3"]);
    assert!(ok, "{err}");
    let sheet = &p.workbooks().next().unwrap().sheets[0];
    assert_eq!(sheet.columns.len(), 2);
    assert_eq!(sheet.columns[1].numeric().unwrap()[..], [2.0, 3.0, 4.0, 5.0]);

    let (ok, err, _) = import(&dir, "bad.txt", "1\n", &["--fixed-width", "0"]);
    assert!(!ok && err.contains("above 0"), "{err}");
}

#[test]
fn delimiter_character_sets_and_combined_runs_are_imported() {
    let dir = scratch("delimiter-sets");
    let text = "t|;i\n1|;2\n2|;3\n3|;4\n4|;5\n";
    let (ok, err, p) = import(&dir, "sets.txt", text, &["--delimiter-chars", "|;", "--combine-delimiters"]);
    assert!(ok, "{err}");
    let sheet = &p.workbooks().next().unwrap().sheets[0];
    assert_eq!(sheet.columns.len(), 2);
    assert_eq!(sheet.columns[1].numeric().unwrap(), [2.0, 3.0, 4.0, 5.0]);
    let (ok, err, p) = import(&dir, "sets.txt", text, &["--delimiter-chars", "|;"]);
    assert!(ok, "{err}");
    assert_eq!(p.workbooks().next().unwrap().sheets[0].columns.len(), 3, "empty middle cells remain by default");
    for flags in [
        vec!["--delimiter-chars", ""],
        vec!["--delimiter-chars", "|;", "--delimiter", "comma"],
        vec!["--delimiter-chars", "|;", "--fixed-width", "3"],
        vec!["--combine-delimiters"],
    ] {
        let (ok, _, _) = import(&dir, "sets.txt", text, &flags);
        assert!(!ok, "invalid flags: {flags:?}");
    }
}

#[test]
fn partial_row_and_column_ranges_keep_the_selected_headers() {
    let dir = scratch("partial");
    let text = "t,a,b,c\n1,11,21,31\n2,12,22,32\n3,13,23,33\n4,14,24,34\n5,15,25,35\n";
    let flags = [
        "--first-row",
        "2",
        "--last-row",
        "5",
        "--read-rows",
        "1",
        "--skip-rows",
        "1",
        "--first-column",
        "2",
        "--skip-columns",
        "1",
    ];
    let (ok, err, p) = import(&dir, "partial.csv", text, &flags);
    assert!(ok, "{err}");
    let sheet = &p.workbooks().next().unwrap().sheets[0];
    assert_eq!(sheet.columns.len(), 2);
    assert_eq!(sheet.columns[0].long_name, "a");
    assert_eq!(sheet.columns[1].long_name, "c");
    assert_eq!(sheet.columns[0].numeric().unwrap(), [12.0, 14.0]);
    assert_eq!(sheet.columns[1].numeric().unwrap(), [32.0, 34.0]);
    for flags in
        [vec!["--first-row", "0"], vec!["--read-columns", "0"], vec!["--first-column", "4", "--last-column", "2"]]
    {
        let (ok, err, _) = import(&dir, "partial.csv", text, &flags);
        assert!(!ok && err.contains("selection"), "{flags:?}: {err}");
    }
}

#[test]
fn partial_except_first_applies_to_later_files_in_a_batch() {
    let dir = scratch("except-first");
    let text = "t,a,b\n1,11,21\n2,12,22\n3,13,23\n4,14,24\n";
    let first = dir.join("first.csv");
    let second = dir.join("second.csv");
    std::fs::write(&first, text).unwrap();
    std::fs::write(&second, text).unwrap();
    let output = dir.join("output.sheaf");
    let result = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .arg("import")
        .arg(&first)
        .arg(&second)
        .arg("--into")
        .arg(&output)
        .args([
            "--first-row",
            "2",
            "--skip-rows",
            "1",
            "--first-column",
            "2",
            "--last-column",
            "2",
            "--partial-except-first",
        ])
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let p = sheaf_io::project_file::load(&output).unwrap();
    let books: Vec<_> = p.workbooks().collect();
    assert_eq!(books[0].sheet().columns.len(), 3);
    assert_eq!(books[0].sheet().row_count(), 4);
    assert_eq!(books[1].sheet().columns.len(), 1);
    assert_eq!(books[1].sheet().columns[0].numeric().unwrap(), [12.0, 14.0]);
}

#[test]
fn the_data_import_options_change_names_and_labels() {
    let dir = scratch("names");
    let text = "Time,Current\ns,A\n1,2\n2,3\n3,4\n4,5\n";
    let (ok, err, p) = import(&dir, "run7.csv", text, &["--short-name-line", "1"]);
    assert!(ok, "{err}");
    let book = p.workbooks().next().unwrap();
    assert_eq!(book.sheets[0].columns[1].short_name, "Current");
    assert_eq!(book.sheets[0].name, "run7");

    let (ok, err, p) =
        import(&dir, "run7.csv", text, &["--no-rename-columns", "--no-rename-sheet", "--no-rename-book"]);
    assert!(ok, "{err}");
    let book = p.workbooks().next().unwrap();
    assert_eq!(book.sheets[0].name, "Sheet1");
    assert_eq!(book.long_name, "");
    assert_eq!(book.sheets[0].columns[1].units, "");
}

#[test]
fn sparklines_yes_marks_the_columns_and_shows_the_row() {
    let dir = scratch("sparklines");
    let text = "t,i\n1,2\n2,3\n3,4\n";
    let (ok, err, p) = import(&dir, "s.csv", text, &["--sparklines", "yes"]);
    assert!(ok, "{err}");
    let sheet = &p.workbooks().next().unwrap().sheets[0];
    assert!(sheet.view.sparklines);
    assert!(sheet.columns.iter().all(|c| c.sparkline));
    let (ok, err, p) = import(&dir, "s.csv", text, &[]);
    assert!(ok, "{err}");
    assert!(!p.workbooks().next().unwrap().sheets[0].view.sparklines, "no sparklines by default");
}
