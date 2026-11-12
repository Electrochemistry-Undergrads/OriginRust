//! `sheaf import` with folders, patterns and the layouts of a multi-file import.

use sheaf_core::project::Window;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-batch-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// An ICP-OES-like export: sample names, element columns with wavelengths in the names, a units
/// line, a date-time column, and entries below the detection limit.
fn icp(dir: &Path, name: &str, base: f64) {
    let text = format!(
        "Sample Name\tFe 238.204\tCu 324.754\tMeasured\nUnits\tmg/L\tmg/L\t\n\
         Blank\t<LOD\t0.02\t2026-05-04 09:15:22\nStd\t{base}\t{}\t2026-05-04 09:17:40\nSample\t{},234\tn.d.\t2026-05-04 09:19:58\n\
         Sample B\t1,234.5\t7\t2026-05-04 09:21:00\n",
        base + 1.0,
        base * 3.0
    );
    std::fs::write(dir.join(name), text).unwrap();
}

fn load(path: &Path) -> sheaf_core::Project {
    sheaf_io::project_file::load(path).unwrap()
}

#[test]
fn a_pattern_into_one_book_with_a_sheet_per_file() {
    let dir = scratch("sheets");
    for (i, n) in ["icp_1.txt", "icp_2.txt", "icp_10.txt"].iter().enumerate() {
        icp(&dir, n, 1.0 + i as f64);
    }
    std::fs::write(dir.join("readme.txt"), "not matched").unwrap();
    let out = dir.join("series.sheaf");
    let pattern = dir.join("icp_*.txt");
    let (ok, stdout, stderr) =
        sheaf(&["import", pattern.to_str().unwrap(), "--into", out.to_str().unwrap(), "--mode", "sheets"]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("matches 3 files"), "{stdout}");
    // The thousands separator in 1,234.5 was found, and the markers are reported.
    assert!(stderr.contains("thousands"), "{stderr}");
    assert!(stderr.contains("no-value marker"), "{stderr}");
    let p = load(&out);
    let books: Vec<_> = p.workbooks().collect();
    assert_eq!(books.len(), 1);
    let names: Vec<&str> = books[0].sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["icp_1", "icp_2", "icp_10"]);
    let fe = &books[0].sheets[0].columns[1];
    assert_eq!(fe.units, "mg/L");
    assert!(fe.numeric().unwrap()[0].is_nan());
    assert_eq!(fe.numeric().unwrap().last().copied(), Some(1234.5));
}

#[test]
fn a_folder_into_columns_keeps_the_file_names() {
    let dir = scratch("columns");
    icp(&dir, "a.txt", 1.0);
    icp(&dir, "b.txt", 2.0);
    let out = dir.join("both.sheaf");
    let (ok, _, stderr) = sheaf(&[
        "import",
        dir.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--mode",
        "columns",
        "--book-name",
        "ICP series",
    ]);
    assert!(ok, "{stderr}");
    let p = load(&out);
    let b = p.workbooks().next().unwrap();
    assert_eq!(b.long_name, "ICP series");
    assert_eq!(b.sheets.len(), 1);
    let s = &b.sheets[0];
    assert_eq!(s.columns.len(), 8);
    assert_eq!(s.columns[0].comments, "a");
    assert_eq!(s.columns[7].comments, "b");
    assert_eq!(s.columns[7].params.get("SourceFile").map(String::as_str), Some("b"));
}

#[test]
fn rows_books_and_mistakes() {
    let dir = scratch("rows");
    icp(&dir, "a.txt", 1.0);
    icp(&dir, "b.txt", 2.0);
    let rows = dir.join("rows.sheaf");
    let pattern = dir.join("*.txt");
    let (ok, _, e) = sheaf(&["import", pattern.to_str().unwrap(), "-o", rows.to_str().unwrap(), "--mode", "rows"]);
    assert!(ok, "{e}");
    let p = load(&rows);
    assert_eq!(p.workbooks().next().unwrap().sheets[0].row_count(), 8);

    let books = dir.join("books.sheaf");
    let (ok, _, e) = sheaf(&["import", pattern.to_str().unwrap(), "-o", books.to_str().unwrap()]);
    assert!(ok, "{e}");
    assert_eq!(load(&books).workbooks().count(), 2);

    // A pattern that matches nothing is an error, and nothing is written.
    let none = dir.join("none.sheaf");
    let missing = dir.join("zzz_*.txt");
    let (ok, _, e) = sheaf(&["import", missing.to_str().unwrap(), "-o", none.to_str().unwrap()]);
    assert!(!ok);
    assert!(e.contains("no file matches"), "{e}");
    assert!(!none.exists());

    // One bad file among good ones: the others are saved and the exit code says something failed.
    std::fs::write(dir.join("c.txt"), "").unwrap();
    let part = dir.join("part.sheaf");
    let (ok, _, e) = sheaf(&["import", pattern.to_str().unwrap(), "-o", part.to_str().unwrap(), "--mode", "sheets"]);
    assert!(!ok);
    assert!(e.contains("c.txt") && e.contains("could not be imported"), "{e}");
    let p = load(&part);
    let first = p.windows.iter().find_map(|w| if let Window::Workbook(b) = w { Some(b) } else { None }).unwrap();
    assert_eq!(first.sheets.len(), 2);
}
