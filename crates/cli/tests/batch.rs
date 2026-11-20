//! Checks of `sheaf batch` (T235): File > Batch Processing... without a window.

use sheaf_core::analysis::{Boundary, OutputTarget, Request, RowRange, SmoothMethod, SmoothOptions, Tool};
use sheaf_core::column::{ColumnData, Designation, RecalcMode};
use sheaf_core::{BookTemplate, Project};
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

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// An analysis template "Assay" (sheet Data: X, Y and an Auto smoothing of Y) and two data files.
fn setup(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let mut p = Project::new();
    let b = p.add_workbook("Assay");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 0.0, 1.0])).unwrap();
    let o = SmoothOptions {
        method: SmoothMethod::AdjacentAveraging { weighted: false },
        npts: 3,
        boundary: Boundary::None,
    };
    let request = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) };
    p.analyze_linked(&request, &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
    p.workbook_mut(b).unwrap().sheets[0].name = "Data".into();
    let t = BookTemplate::analysis_from_workbook("Assay", &p, b).unwrap();
    let saved = sheaf_io::user_templates::save_book(dir, &t).unwrap();
    let run1 = dir.join("run1.csv");
    let run2 = dir.join("run2.csv");
    std::fs::write(&run1, "t,v\n1,0\n2,3\n3,0\n4,3\n5,0\n").unwrap();
    std::fs::write(&run2, "t,v\n1,1\n2,1\n3,1\n").unwrap();
    (saved.path, run1, run2)
}

#[test]
fn batch_makes_one_book_per_file_and_saves_the_project() {
    let dir = scratch("books");
    let (template, run1, run2) = setup(&dir);
    let missing = dir.join("missing.csv");
    let out = dir.join("runs.sheaf");
    let (ok, stdout, stderr) = sheaf(&["batch", s(&run1), s(&run2), s(&missing), "-t", s(&template), "-o", s(&out)]);
    assert!(ok, "{stdout}{stderr}");
    assert!(stderr.contains("skipped") && stderr.contains("missing.csv"), "{stderr}");
    let p = sheaf_io::project_file::load(&out).unwrap();
    let books: Vec<_> = p.workbooks().collect();
    assert_eq!(books.len(), 2);
    assert_eq!(books[0].long_name, "run1.csv");
    let cols = &books[0].sheets[0].columns;
    assert_eq!(cols[1].numeric().unwrap(), [0.0, 3.0, 0.0, 3.0, 0.0]);
    assert_eq!(cols[2].len(), 5, "the smoothing ran on the file's data");
    assert_eq!(p.analysis_links.len(), 2, "each book keeps its recalculating run");
}

#[test]
fn batch_writes_a_summary_and_can_delete_the_books() {
    let dir = scratch("summary");
    let (template, run1, run2) = setup(&dir);
    let out = dir.join("summary.sheaf");
    let csv = dir.join("summary.csv");
    let args = [
        "batch",
        s(&run1),
        s(&run2),
        "-t",
        s(&template),
        "--summary",
        "data",
        "--identifier",
        "File",
        "--delete-books",
        "--summary-csv",
        s(&csv),
        "-o",
        s(&out),
    ];
    let (ok, stdout, stderr) = sheaf(&args);
    assert!(ok, "{stdout}{stderr}");
    assert!(stdout.contains("8 row(s)") && stdout.contains("the books were deleted"), "{stdout}");
    let p = sheaf_io::project_file::load(&out).unwrap();
    let books: Vec<_> = p.workbooks().collect();
    assert_eq!(books.len(), 1, "only the summary is left");
    assert_eq!(books[0].long_name, "Batch Summary");
    assert!(p.analysis_links.is_empty(), "the deleted books' runs are gone");
    let text = std::fs::read_to_string(&csv).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("File"), "{text}");
    assert!(lines.iter().filter(|l| l.contains("run1.csv")).count() == 5, "{text}");
    assert!(lines.iter().filter(|l| l.contains("run2.csv")).count() == 3, "{text}");

    // A sheet the template does not have is refused before anything is written.
    let other = dir.join("other.sheaf");
    let (ok, _, stderr) = sheaf(&["batch", s(&run1), "-t", s(&template), "--summary", "Stats", "-o", s(&other)]);
    assert!(!ok && stderr.contains("no sheet 'Stats'") && stderr.contains("Data"), "{stderr}");
    assert!(!other.exists());
    // Nowhere to put the result.
    let (ok, _, stderr) = sheaf(&["batch", s(&run1), "-t", s(&template)]);
    assert!(!ok && stderr.contains("--output"), "{stderr}");
}
