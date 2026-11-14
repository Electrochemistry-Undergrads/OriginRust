//! `sheaf analyze subtract-line | reduce-x | translate | median-filter` (T23).

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-manip-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// t = 0..=4 and y = 2t + 1 with a bump of 3 at t = 2; "spiky" is flat 5 with a spike of 100 at t = 2.
fn data_file(dir: &Path) -> PathBuf {
    let y = [1.0, 3.0, 8.0, 7.0, 9.0];
    let spiky = [5.0, 5.0, 100.0, 5.0, 5.0];
    let mut text = String::from("t,y,spiky\ns,A,A\n");
    for i in 0..5 {
        text.push_str(&format!("{i},{},{}\n", y[i], spiky[i]));
    }
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

fn run_export(dir: &Path, tag: &str, args: &[&str]) -> String {
    let data = data_file(dir);
    let out = dir.join(format!("{tag}.csv"));
    let mut full = vec!["analyze", args[0], data.to_str().unwrap(), "--y", args[1]];
    full.extend_from_slice(&args[2..]);
    full.extend_from_slice(&["--export", out.to_str().unwrap()]);
    let (ok, _, stderr) = sheaf(&full);
    assert!(ok, "stderr: {stderr}");
    std::fs::read_to_string(&out).unwrap()
}

#[test]
fn subtract_line_leaves_the_bump() {
    let dir = scratch("line");
    let text = run_export(&dir, "line", &["subtract-line", "y", "--x1", "0", "--x2", "4"]);
    assert!(text.lines().next().unwrap().contains("y minus line"), "{text}");
    assert!(text.lines().any(|l| l == "2,8,3,100"), "{text}");
    assert!(text.lines().any(|l| l == "4,9,0,5"), "{text}");
}

#[test]
fn subtract_line_outside_the_data_is_explained() {
    let dir = scratch("line-out");
    let data = data_file(&dir);
    let (ok, _, stderr) =
        sheaf(&["analyze", "subtract-line", data.to_str().unwrap(), "--y", "y", "--x1", "0", "--x2", "9"]);
    assert!(!ok);
    assert!(stderr.contains("inside the X range"), "{stderr}");
}

#[test]
fn reduce_x_by_intervals_and_every_n_points_as_origin() {
    let dir = scratch("reduce");
    // t = 0..4, y = 1 3 8 7 9. Intervals 2 wide: [0, 2) and [2, 4); the point at X End (4) is left out.
    let text = run_export(&dir, "reduce", &["reduce-x", "y", "--step", "2"]);
    assert!(text.lines().any(|l| l == "1,2"), "{text}");
    assert!(text.lines().any(|l| l == "3,7.5"), "{text}");
    assert!(!text.lines().any(|l| l.starts_with("5,")), "{text}");
    // The first point of every 2.
    let text = run_export(&dir, "reduce2", &["reduce-x", "y", "--every", "2", "--method", "first"]);
    for line in ["0,1", "2,8", "4,9"] {
        assert!(text.lines().any(|l| l == line), "{line} in {text}");
    }
}

#[test]
fn translate_in_x_and_in_y() {
    let dir = scratch("translate");
    let text = run_export(&dir, "tx", &["translate", "y", "--axis", "x", "--by", "-1"]);
    assert!(text.lines().any(|l| l == "-1,1"), "{text}");
    // The curve is 7 at t = 3; to bring it to 0 every value drops by 7.
    let text = run_export(&dir, "ty", &["translate", "y", "--axis", "y", "--at", "3", "--to", "0"]);
    assert!(text.lines().any(|l| l == "2,8,1,100"), "{text}");
}

#[test]
fn median_filter_removes_the_spike() {
    let dir = scratch("median");
    let text = run_export(&dir, "median", &["median-filter", "spiky", "--points", "3"]);
    assert!(text.lines().any(|l| l == "2,8,100,5"), "{text}");
    let data = data_file(&dir);
    let (ok, _, stderr) = sheaf(&["analyze", "median-filter", data.to_str().unwrap(), "--y", "spiky", "--points", "4"]);
    assert!(!ok);
    assert!(stderr.contains("odd"), "{stderr}");
}
