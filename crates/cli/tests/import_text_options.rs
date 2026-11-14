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
