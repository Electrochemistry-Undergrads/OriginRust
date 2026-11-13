//! `sheaf analyze average`, `intersect` and `polygon-area` on small files.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-curves-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn file(dir: &Path, text: &str) -> PathBuf {
    let path = dir.join("data.csv");
    std::fs::write(&path, text).unwrap();
    path
}

fn sheaf(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn average_writes_the_mean_sheet() {
    let dir = scratch("average");
    let data = file(&dir, "x,a,b\nV,A,A\n0,0,2\n1,1,3\n2,2,4\n");
    let export = dir.join("mean.csv");
    let (ok, stdout, stderr) =
        sheaf(&["analyze", "average", data.to_str().unwrap(), "--export", export.to_str().unwrap()]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Average1"), "{stdout}");
    let rows: Vec<String> = std::fs::read_to_string(&export).unwrap().lines().map(str::to_string).collect();
    assert!(rows.iter().any(|r| r.starts_with("1,2,")), "{rows:?}");
    // The mean of 0..2 and 2..4 is 1..3, the SD is sqrt(2) everywhere, n = 2.
    assert!(rows.iter().any(|r| r.starts_with("2,3,1.41421356")), "{rows:?}");
}

#[test]
fn intersect_needs_two_curves_and_lists_the_point() {
    let dir = scratch("intersect");
    let data = file(&dir, "x,up,down,third\n,,,\n0,0,4,1\n4,4,0,1\n");
    let (ok, _, stderr) = sheaf(&["analyze", "intersect", data.to_str().unwrap()]);
    assert!(!ok && stderr.contains("exactly two"), "{stderr}");
    let export = dir.join("pts.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "intersect",
        data.to_str().unwrap(),
        "-y",
        "up,down",
        "--export",
        export.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("1 point"), "{stdout}");
    assert!(std::fs::read_to_string(&export).unwrap().lines().any(|r| r.starts_with("2,2")));
}

#[test]
fn polygon_area_prints_the_table() {
    let dir = scratch("polygon");
    let data = file(&dir, "x,y\nV,A\n0,0\n2,0\n2,3\n0,3\n");
    let (ok, stdout, stderr) = sheaf(&["analyze", "polygon-area", data.to_str().unwrap()]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Area") && stdout.contains("PolygonArea1"), "{stdout}");
    let line = stdout.lines().find(|l| l.starts_with("B:")).expect("a row for column B");
    let cells: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(cells[2], "6", "{line}");
}
