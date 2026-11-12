//! Runs the `sheaf` program itself on small files and checks what it prints and writes.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A fresh folder in the system temp directory.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// t = 0..10 in steps of 1 (s); "flat" is 2 everywhere, "ramp" equals t.
fn data_file(dir: &Path) -> PathBuf {
    let mut text = String::from("t,flat,ramp\ns,A,A\n");
    for i in 0..=10 {
        text.push_str(&format!("{i},2,{i}\n"));
    }
    let path = dir.join("data.csv");
    std::fs::write(&path, text).unwrap();
    path
}

/// Runs `sheaf` and returns (success, stdout, stderr).
fn sheaf(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn integrate_prints_the_areas_and_exports_the_result_sheet() {
    let dir = scratch("integrate");
    let data = data_file(&dir);
    let export = dir.join("areas.csv");
    let (ok, stdout, stderr) =
        sheaf(&["analyze", "integrate", data.to_str().unwrap(), "--export", export.to_str().unwrap()]);
    assert!(ok, "stderr: {stderr}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines[0].starts_with("Data") && lines[0].contains("Area (C)"), "header: {}", lines[0]);
    let cells = |line: &str| line.split_whitespace().map(str::to_string).collect::<Vec<_>>();
    assert_eq!(cells(lines[1])[..5], ["B:", "flat", "20", "0", "10"]);
    assert_eq!(cells(lines[2])[..3], ["C:", "ramp", "50"]);
    assert!(lines[3].contains("2 columns") && lines[3].contains("Integrate1"));
    let exported = std::fs::read_to_string(&export).unwrap();
    let rows: Vec<&str> = exported.lines().collect();
    assert!(rows[0].starts_with("Data,Area,X start,X end"), "{}", rows[0]);
    assert!(rows.iter().any(|r| r.starts_with("B: flat,20,0,10,")), "{exported}");
}

#[test]
fn a_row_range_with_baseline_and_a_saved_project() {
    let dir = scratch("rows");
    let data = data_file(&dir);
    let project = dir.join("out.sheaf");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "integrate",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--rows",
        "3:5",
        "--save",
        project.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    // Rows 3 to 5 are t = 2, 3, 4; the area under y = t is 6.
    assert!(stdout.lines().nth(1).unwrap().split_whitespace().collect::<Vec<_>>()[2..5] == ["6", "2", "4"]);
    // The saved project has the running integral next to its source, and the result sheet.
    let (ok, info, stderr) = sheaf(&["info", project.to_str().unwrap()]);
    assert!(ok, "stderr: {stderr}");
    assert!(info.contains("Integral of ramp"), "{info}");
    assert!(info.contains("Integrate1"), "{info}");
    // Analysing the saved project again works on a named sheet of it.
    let (ok, stdout, stderr) =
        sheaf(&["analyze", "stats", project.to_str().unwrap(), "--sheet", "Integrate1", "--y", "Area"]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Mean"), "{stdout}");
}

#[test]
fn the_fft_filter_and_the_b_spline_can_be_chosen() {
    let dir = scratch("fft-bspline");
    let data = data_file(&dir);
    let out = dir.join("out.csv");
    // A constant column goes through the FFT filter unchanged.
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "smooth",
        data.to_str().unwrap(),
        "--y",
        "flat",
        "--method",
        "fft",
        "--points",
        "3",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.lines().any(|l| l == "5,2,2,5"), "{text}");

    // The interpolating B-spline reproduces a straight line.
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "interpolate",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--method",
        "bspline",
        "--points",
        "5",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.lines().any(|l| l == "2.5,2.5"), "{text}");

    // A smoothing factor above zero gives a smoothing spline; a negative one is refused.
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "interpolate",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--method",
        "bspline",
        "--smoothing",
        "0.5",
        "--points",
        "5",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let text = std::fs::read_to_string(&out).unwrap();
    let middle: f64 = text.lines().nth(3).unwrap().split(',').nth(1).unwrap().parse().unwrap();
    assert!((middle - 2.5).abs() < 1e-9, "{text}");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "interpolate",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--method",
        "bspline",
        "--smoothing=-0.5",
    ]);
    assert!(!ok);
    assert!(stderr.contains("smoothing factor"), "{stderr}");
}

#[test]
fn smooth_sort_and_filter_write_what_they_say() {
    let dir = scratch("misc");
    let data = data_file(&dir);
    let out = dir.join("out.csv");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "smooth",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--method",
        "average",
        "--points",
        "3",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.lines().next().unwrap().ends_with("ramp,Smoothed ramp"), "{text}");
    // A moving average of a straight line is the line itself.
    assert!(text.lines().any(|l| l == "5,2,5,5"), "{text}");

    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "sort",
        data.to_str().unwrap(),
        "--by",
        "ramp",
        "--descending",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("sorted by column ramp"));
    assert_eq!(std::fs::read_to_string(&out).unwrap().lines().nth(2), Some("10,2,10"));

    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "filter",
        data.to_str().unwrap(),
        "--where",
        "col(ramp) > 7 && col(t) < 10",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("kept 2 of 11 rows"), "{stdout}");
    let kept: Vec<String> = std::fs::read_to_string(&out).unwrap().lines().skip(2).map(str::to_string).collect();
    assert_eq!(kept, ["8,2,8", "9,2,9"]);
}

#[test]
fn mistakes_give_a_message_and_a_failure_code() {
    let dir = scratch("errors");
    let data = data_file(&dir);
    let d = data.to_str().unwrap();
    for (args, expect) in [
        (vec!["analyze", "integrate", d, "--rows", "9-12"], "not a valid row range"),
        (vec!["analyze", "integrate", d, "--rows", "40:50"], "not a valid range"),
        (vec!["analyze", "integrate", d, "--y", "nope"], "column 'nope' was not found"),
        (vec!["analyze", "differentiate", d, "--order", "0"], "between 1 and 9"),
        (vec!["analyze", "normalize", d, "--method", "divide"], "--value"),
        (vec!["analyze", "baseline", d, "--line", "1,2,3"], "four numbers"),
        (vec!["analyze", "filter", d, "--where", "col(zzz) > 1"], "zzz"),
        (vec!["analyze", "interpolate", d, "--x-min", "1"], "both --x-min and --x-max"),
    ] {
        let (ok, _, stderr) = sheaf(&args);
        assert!(!ok, "{args:?} should fail");
        assert!(stderr.contains(expect), "{args:?}: expected '{expect}' in: {stderr}");
    }
}

#[test]
fn integrate_ranges_prints_one_row_per_interval_and_column() {
    let dir = scratch("ranges");
    let data = data_file(&dir);
    let export = dir.join("ranges.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "integrate",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--ranges",
        "0:4,4:10",
        "--row-ranges",
        "1:3",
        "--export",
        export.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines[0].contains("Absolute area") && lines[0].contains("Peak X"), "{}", lines[0]);
    assert!(lines[1].contains("range 1") && lines[2].contains("range 2") && lines[3].contains("range 3"), "{stdout}");
    // y = t: the areas under 0..4, 4..10 and rows 1..3 (t = 0..2) are 8, 42 and 2. In the
    // exported sheet the label comes first, then First row, Last row, X start, X end, Area (counted here from the right, after a units row).
    let exported = std::fs::read_to_string(&export).unwrap();
    let areas: Vec<&str> = exported.lines().skip(2).map(|l| l.rsplit(',').nth(4).unwrap()).collect();
    assert_eq!(areas, ["8", "42", "2"], "{exported}");
}

#[test]
fn subtract_ref_and_math_use_a_reference_from_another_file() {
    let dir = scratch("math");
    let data = data_file(&dir);
    // reference: x = 2, 4, 6 and y = x / 2
    let blank = dir.join("blank.csv");
    std::fs::write(&blank, "t,ref\ns,A\n2,1\n4,2\n6,3\n").unwrap();
    let export = dir.join("sub.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "subtract-ref",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--ref",
        "ref",
        "--ref-file",
        blank.to_str().unwrap(),
        "--export",
        export.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Subtract reference"), "{stdout}");
    let rows: Vec<String> = std::fs::read_to_string(&export).unwrap().lines().skip(2).map(str::to_string).collect();
    // t, flat, ramp, then the new column behind its source: ramp minus the reference (x / 2)
    assert_eq!(rows[0], "0,2,0,0");
    assert_eq!(rows[10], "10,2,10,5");
    // Common range only: a new sheet with t = 2..6.
    let out = dir.join("common.csv");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "math",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--op",
        "divide",
        "--ref",
        "ref",
        "--ref-file",
        blank.to_str().unwrap(),
        "--common",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let text = std::fs::read_to_string(&out).unwrap();
    let body: Vec<&str> = text.lines().skip(2).collect();
    assert_eq!(body.len(), 5, "{text}");
    assert!(body.iter().all(|l| l.ends_with(",2")), "{text}");
    // A constant, with a negative value.
    let (ok, stdout, stderr) = sheaf(&["analyze", "math", data.to_str().unwrap(), "--y", "ramp", "--value", "-3"]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Curve math"), "{stdout}");
}

#[test]
fn curve_math_and_range_mistakes_are_explained() {
    let dir = scratch("math-errors");
    let data = data_file(&dir);
    let d = data.to_str().unwrap();
    for (args, expect) in [
        (vec!["analyze", "math", d], "--value NUMBER or --ref COLUMN"),
        (vec!["analyze", "subtract-ref", d, "--value", "1"], "--ref COLUMN"),
        (vec!["analyze", "math", d, "--op", "divide", "--value", "0"], "divide by 0"),
        (vec!["analyze", "math", d, "--ref", "nope"], "'nope' was not found"),
        (vec!["analyze", "integrate", d, "--ranges", "1-4"], "not a valid X range"),
        (vec!["analyze", "integrate", d, "--ranges", "1:4", "--absolute"], "do not apply"),
    ] {
        let (ok, _, stderr) = sheaf(&args);
        assert!(!ok, "{args:?} should fail");
        assert!(stderr.contains(expect), "{args:?}: expected '{expect}' in: {stderr}");
    }
}
