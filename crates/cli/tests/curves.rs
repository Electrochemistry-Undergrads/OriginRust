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
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "average",
        data.to_str().unwrap(),
        "--x",
        "same",
        "--sd",
        "--count",
        "--export",
        export.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("AverageResult1"), "{stdout}");
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
    let (ok, stdout, stderr) =
        sheaf(&["analyze", "intersect", data.to_str().unwrap(), "-y", "up,down", "--export", export.to_str().unwrap()]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("1 point"), "{stdout}");
    // T322: the columns go to the end of the input sheet: row 1 is x, up, down, third, then 2, 2.
    let text = std::fs::read_to_string(&export).unwrap();
    assert!(text.lines().any(|r| r.ends_with(",2,2")), "{text}");
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

#[test]
fn xyz_area_prints_the_surface_area_of_the_x_y_z_columns() {
    let dir = scratch("xyzarea");
    let data = file(
        &dir,
        "x,y,z
0,0,0
1,0,2
0,1,3
1,1,5
",
    );
    let (ok, stdout, stderr) = sheaf(&["analyze", "xyz-area", data.to_str().unwrap()]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Area") && stdout.contains("XYZArea1"), "{stdout}");
    assert!(stdout.contains("3.74165738677394"), "sqrt(14): {stdout}");
    // Two points are not a surface.
    let few = file(
        &dir,
        "x,y,z
0,0,0
1,1,1
",
    );
    let (ok, _, stderr) = sheaf(&["analyze", "xyz-area", few.to_str().unwrap()]);
    assert!(!ok && stderr.contains("at least three"), "{stderr}");
}

#[test]
fn average_on_a_cycle_is_refused_unless_the_curves_share_their_x() {
    let dir = scratch("cycle");
    // Two scans with their own, turning X: refused with a hint to choose the rows of one sweep.
    let data = file(&dir, "x1,a,x2,b\n,,,\n0,0,0,0\n1,1,0.5,1\n0,2,1,2\n");
    let (ok, _, stderr) = sheaf(&["analyze", "average", data.to_str().unwrap(), "-y", "a,b"]);
    assert!(!ok && stderr.contains("turn back"), "{stderr}");
    // The same scans on a shared X are averaged row by row, turning included.
    let shared = file(&dir, "x,a,b\n,,\n0,0,2\n1,1,3\n0,2,4\n");
    let (ok, stdout, stderr) = sheaf(&["analyze", "average", shared.to_str().unwrap(), "--x", "same"]);
    assert!(ok && stdout.contains("3 X values"), "{stdout} {stderr}");
}

#[test]
fn average_takes_evenly_spaced_points_and_refuses_a_zero_reference() {
    let dir = scratch("even");
    let data = file(&dir, "x,a,b\n,,\n0,0,2\n1,1,3\n2,2,4\n");
    let export = dir.join("even.csv");
    let (ok, _, stderr) =
        sheaf(&["analyze", "average", data.to_str().unwrap(), "--points", "5", "--export", export.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert_eq!(std::fs::read_to_string(&export).unwrap().lines().count(), 5 + 2, "5 rows under two header rows");
    let (ok, _, stderr) = sheaf(&["analyze", "average", data.to_str().unwrap(), "--reference-curve", "0"]);
    assert!(!ok && stderr.contains("counts from 1"), "{stderr}");
}
