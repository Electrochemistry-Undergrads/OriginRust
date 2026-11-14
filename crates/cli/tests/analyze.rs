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
    let middle: f64 = text.lines().nth(4).unwrap().split(',').nth(1).unwrap().parse().unwrap();
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
    let rows: Vec<String> = std::fs::read_to_string(&export).unwrap().lines().skip(3).map(str::to_string).collect();
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
    let body: Vec<&str> = text.lines().skip(3).collect();
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

#[test]
fn info_shows_a_date_time_column_and_the_option_keeps_text() {
    let dir = scratch("dates");
    let path = dir.join("icp.csv");
    std::fs::write(&path, "Time,Cu\n2026-11-11 13:45:10,1.25\n2026-11-11 13:47:42,12.5\n2026-11-11 13:50:15,24.9\n")
        .unwrap();
    let (ok, stdout, stderr) = sheaf(&["info", path.to_str().unwrap()]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains(" date ") && stdout.contains("<2026-11-11 13:45:10 to 2026-11-11 13:50:15>"), "{stdout}");
    let (ok, stdout, _) = sheaf(&["info", path.to_str().unwrap(), "--dates-as-text"]);
    assert!(ok);
    assert!(stdout.contains(" text") && !stdout.contains(" date "), "{stdout}");
}

/// Two Gauss peaks (centres 6 and 13) on a sloped baseline, one X and one Y column.
fn peak_file(dir: &Path) -> PathBuf {
    let mut text = String::from("t,i\ns,A\n");
    for k in 0..400 {
        let x = k as f64 * 0.05;
        let g = |c: f64, s: f64, h: f64| h * (-(x - c) * (x - c) / (2.0 * s * s)).exp();
        text.push_str(&format!("{x},{}\n", 0.1 + 0.01 * x + g(6.0, 0.4, 1.0) + g(13.0, 0.5, 0.6)));
    }
    let path = dir.join("peaks.csv");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn peaks_chain_integrates_and_fits_every_peak() {
    let dir = scratch("peakchain");
    let data = peak_file(&dir);
    let export = dir.join("result.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "peaks",
        data.to_str().unwrap(),
        "--baseline",
        "minima",
        "--baseline-points",
        "4",
        "--min-height",
        "0.3",
        "--integrate",
        "--fit",
        "gauss",
        "--export",
        export.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Area (C)") && stdout.contains("Fit center"), "{stdout}");
    assert!(stdout.contains("Peak 1 of") && stdout.contains("Peak 2 of"), "{stdout}");
    assert!(stdout.contains("Gauss fit of 2 peaks") && stdout.contains("PeakAnalysis1"), "{stdout}");
    let exported = std::fs::read_to_string(&export).unwrap();
    assert!(exported.lines().next().unwrap().contains("Area"), "{exported}");

    // Without any chain option the plain peak finder runs, as before.
    let (ok, stdout, _) = sheaf(&["analyze", "peaks", data.to_str().unwrap()]);
    assert!(ok && stdout.contains("Peaks1") && !stdout.contains("Area"), "{stdout}");

    let (ok, _, stderr) = sheaf(&["analyze", "peaks", data.to_str().unwrap(), "--baseline", "anchors"]);
    assert!(!ok && stderr.contains("--anchors"), "{stderr}");
}

/// Two groups of five measurements (untreated and treated), one X-less sheet.
fn groups_file(dir: &Path) -> PathBuf {
    let path = dir.join("groups.csv");
    std::fs::write(
        &path,
        "untreated,treated,third\n10.1,12.0,9.0\n9.8,12.4,9.5\n10.4,11.7,9.2\n10.0,12.2,8.8\n9.7,12.1,9.1\n",
    )
    .unwrap();
    path
}

#[test]
fn ttest_anova_normality_and_histogram_run_from_the_command_line() {
    let dir = scratch("stats");
    let data = groups_file(&dir);
    let file = data.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "ttest", file, "--y", "untreated,treated", "--unequal", "--tail", "lower"]);
    assert!(ok, "{err}");
    assert!(out.contains("untreated vs treated") && out.contains("Welch") && out.contains("rejected"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "ttest", file, "--y", "untreated", "--test", "one-sample", "--mean", "10"]);
    assert!(ok, "{err}");
    assert!(out.contains("one-sample t-test"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "ttest", file, "--y", "untreated", "--test", "paired"]);
    assert!(!ok && err.contains("exactly two columns"), "{err}");

    let project = dir.join("stats.sheaf");
    let (ok, out, err) = sheaf(&[
        "analyze",
        "anova",
        file,
        "--y",
        "untreated,treated,third",
        "--comparison",
        "tukey",
        "--save",
        project.to_str().unwrap(),
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("Model") && out.contains("Tukey means comparison") && out.contains("ANOVA1"), "{out}");

    let (ok, out, err) = sheaf(&["analyze", "normality", file, "--y", "untreated,treated"]);
    assert!(ok, "{err}");
    assert!(out.contains("Shapiro-Wilk W"), "{out}");

    let (ok, out, err) = sheaf(&[
        "analyze",
        "histogram",
        file,
        "--y",
        "untreated",
        "--bin-size",
        "0.5",
        "--min",
        "9.5",
        "--max",
        "10.5",
        "--save",
        project.to_str().unwrap(),
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("Histogram1") && out.contains("graph Graph1"), "{out}");
    let (ok, info, err) = sheaf(&["info", project.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(info.contains("Graph1"), "{info}");
}

#[test]
fn stats_can_add_the_other_quantities_and_the_confidence_limits() {
    let dir = scratch("stats-all");
    let data = groups_file(&dir);
    let file = data.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "stats", file, "--y", "untreated"]);
    assert!(ok, "{err}");
    assert!(out.contains("Median") && !out.contains("Variance") && !out.contains("CL of Mean"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "stats", file, "--y", "untreated", "--all", "--level", "0.9"]);
    assert!(ok, "{err}");
    for word in ["IQR", "Variance", "Skewness", "Kurtosis", "CV", "Lower 90% CL of Mean", "Upper 90% CL of Mean"] {
        assert!(out.contains(word), "{word} missing in {out}");
    }
    let (ok, out, err) = sheaf(&["analyze", "stats", file, "--y", "untreated", "--add", "variance,limits"]);
    assert!(ok, "{err}");
    assert!(out.contains("Variance") && out.contains("Lower 95% CL of Mean") && !out.contains("IQR"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "stats", file, "--y", "untreated", "--add", "bogus"]);
    assert!(!ok && err.contains("unknown quantity"), "{err}");
}

#[test]
fn outlier_row_and_more_rank_tests_run_from_the_command_line() {
    let dir = scratch("breadth");
    let path = dir.join("breadth.csv");
    // The three Origin reference samples with nine numbers each.
    std::fs::write(
        &path,
        "a,b,c\n1.1,2.2,3.3\n2.3,3.1,1.1\n2.3,4.0,5.0\n4.0,4.0,2.2\n5.5,8.8,8.0\n6.1,10.5,7.5\n7.2,11.0,1.0\n7.2,12.5,9.1\n9.9,12.9,4.4\n",
    )
    .unwrap();
    let file = path.to_str().unwrap();

    // Origin sign2: eight differing pairs, all with the first smaller, exact two-sided p = 0.0078125.
    let (ok, out, err) = sheaf(&["analyze", "sign", file, "--y", "a,b"]);
    assert!(ok, "{err}");
    assert!(out.contains("a vs b") && out.contains("0.0078125") && out.contains("SignTest1"), "{out}");
    // Two-sample KS of nine numbers against nine: D = 4/9, exact p (checked against Origin on other samples
    // in the calc crate, not on these).
    let (ok, out, err) = sheaf(&["analyze", "ks2", file, "--y", "a,b"]);
    assert!(ok, "{err}");
    assert!(out.contains("0.444444") && out.contains("0.942809") && out.contains("exact p"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "ks2", file, "--y", "a,b", "--tail", "upper"]);
    assert!(ok, "{err}");
    assert!(out.contains("upper-tailed"), "{out}");
    // Origin mediantest: chi-square 1.18681, 2 df, p 0.552442.
    let (ok, out, err) = sheaf(&["analyze", "mediantest", file, "--y", "a,b,c"]);
    assert!(ok, "{err}");
    assert!(out.contains("chi-square(2) = 1.187") && out.contains("p = 0.5524"), "{out}");
    // Origin friedman: chi-square 7.38889, 2 df, p 0.0248613.
    let (ok, out, err) = sheaf(&["analyze", "friedman", file, "--y", "a,b,c"]);
    assert!(ok, "{err}");
    assert!(out.contains("chi-square(2) = 7.389") && out.contains("p = 0.02486"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "sign", file, "--y", "a,b,c"]);
    assert!(!ok && err.contains("exactly two columns"), "{err}");

    // Origin grubbs on a: G = 1.68403, critical value 2.21499, no outlier. Origin qtest: Q = 0.30682, 0.493.
    let (ok, out, err) = sheaf(&["analyze", "grubbs", file, "--y", "a"]);
    assert!(ok, "{err}");
    assert!(out.contains("1.684") && out.contains("no significant outlier"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "dixon", file, "--y", "a"]);
    assert!(ok, "{err}");
    assert!(out.contains("0.3068") && out.contains("0.493") && out.contains("no significant outlier"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "dixon", file, "--y", "a", "--alpha", "0.2"]);
    assert!(!ok && err.contains("0.1, 0.05 and 0.01"), "{err}");

    // Statistics on rows: columns behind the last chosen one, saved in a project.
    let project = dir.join("rows.sheaf");
    let (ok, out, err) =
        sheaf(&["analyze", "rowstats", file, "--y", "a,b,c", "--add", "n,sum", "--save", project.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(out.contains("Mean") && out.contains("Number of Points"), "{out}");
    let (ok, info, err) = sheaf(&["info", project.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(info.contains("Standard Deviation") && info.contains("Sum"), "{info}");
    let (ok, _, err) = sheaf(&["analyze", "rowstats", file, "--y", "a,b", "--add", "bogus"]);
    assert!(!ok && err.contains("unknown quantity"), "{err}");
}

/// The unbalanced two-way design of the Origin reference runs (cells of 3, 2, 4, 1, 3, 2 numbers), with the
/// level of factor A as text.
fn design_file(dir: &Path) -> PathBuf {
    let a = [1, 2, 2, 2, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1];
    let b = [1, 3, 2, 1, 3, 3, 2, 1, 3, 2, 2, 3, 3, 2, 1];
    let y = [
        14.214, 14.2532, 16.6664, 16.5478, 12.8569, 12.067, 14.8418, 14.8812, 13.4589, 15.412, 17.199, 14.2347,
        11.9925, 13.4436, 14.5733,
    ];
    let mut text = String::from("dose,batch,response\n");
    for i in 0..15 {
        text.push_str(&format!("{},{},{}\n", if a[i] == 1 { "low" } else { "high" }, b[i], y[i]));
    }
    let path = dir.join("design.csv");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn anova2_runs_on_indexed_and_raw_data_and_reports_origins_sums_of_squares() {
    let dir = scratch("anova2");
    let data = design_file(&dir);
    let file = data.to_str().unwrap();
    let export = dir.join("anova2.csv");
    let (ok, out, err) = sheaf(&[
        "analyze",
        "anova2",
        file,
        "--y",
        "dose,batch,response",
        "--comparison",
        "bonferroni",
        "--export",
        export.to_str().unwrap(),
    ]);
    assert!(ok, "{err}");
    // Origin's type III sums of squares for this unbalanced design: 9.7878 for A, 13.6434 for B.
    assert!(
        out.contains("dose*batch") && out.contains("Model") && out.contains("9.7878") && out.contains("13.6434"),
        "{out}"
    );
    assert!(
        out.contains("Bonferroni means comparison") && out.contains("type III") && out.contains("ANOVA2Way1"),
        "{out}"
    );
    let exported = std::fs::read_to_string(&export).unwrap();
    assert!(exported.contains("Sum of Squares") && exported.contains("dose: low"), "{exported}");

    let (ok, out, err) =
        sheaf(&["analyze", "anova2", file, "--y", "dose,batch,response", "--no-interaction", "--comparison", "none"]);
    assert!(ok, "{err}");
    assert!(!out.contains("dose*batch") && out.contains("9.74949"), "{out}");

    // Raw layout: one column per cell of a balanced 2 x 2 design.
    let raw = dir.join("raw.csv");
    std::fs::write(&raw, "xu,xv,yu,yv\n1,4,3,8\n2,5,5,9\n").unwrap();
    let (ok, out, err) = sheaf(&[
        "analyze",
        "anova2",
        raw.to_str().unwrap(),
        "--raw",
        "--y",
        "xu,xv,yu,yv",
        "--levels-a",
        "x,x,y,y",
        "--levels-b",
        "u,v,u,v",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("Factor A*Factor B"), "{out}");

    // Wrong number of columns, and levels that do not match the columns.
    let (ok, _, err) = sheaf(&["analyze", "anova2", file, "--y", "dose,response"]);
    assert!(!ok && err.contains("FACTOR_A,FACTOR_B,DATA"), "{err}");
    let (ok, _, err) = sheaf(&[
        "analyze",
        "anova2",
        raw.to_str().unwrap(),
        "--raw",
        "--y",
        "xu,xv",
        "--levels-a",
        "x",
        "--levels-b",
        "u,v",
    ]);
    assert!(!ok && err.contains("one level of factor A"), "{err}");
}

#[test]
fn rank_tests_and_correlation_run_from_the_command_line() {
    let dir = scratch("ranks");
    let path = dir.join("ranks.csv");
    // Columns of 9, 7 and 11 numbers with ties (the Origin reference samples); the shorter ones end early.
    std::fs::write(
        &path,
        "a,b,c\n1.1,2.2,3.3\n2.3,3.1,1.1\n2.3,4.0,5.0\n4.0,4.0,2.2\n5.5,8.8,8.0\n6.1,10.5,7.5\n7.2,11.0,1.0\n7.2,,9.1\n9.9,,4.4\n,,6.0\n,,12.5\n",
    )
    .unwrap();
    let file = path.to_str().unwrap();

    let (ok, out, err) = sheaf(&["analyze", "mannwhitney", file, "--y", "a,b", "--exact"]);
    assert!(ok, "{err}");
    // U = 26, exact p = 0.586189, asymptotic p = 0.594998 (Origin: 0.58618881 and 0.59499846).
    assert!(
        out.contains("a vs b") && out.contains("26") && out.contains("0.586189") && out.contains("0.594998"),
        "{out}"
    );
    assert!(out.contains("MannWhitney1"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "mannwhitney", file, "--y", "a,b,c"]);
    assert!(!ok && err.contains("exactly two columns"), "{err}");

    let (ok, out, err) = sheaf(&["analyze", "signrank", file, "--y", "a", "--median", "3"]);
    assert!(ok, "{err}");
    // T+ = 38, exact p = 0.0664062, asymptotic p = 0.0750477.
    assert!(out.contains("38") && out.contains("0.0664062") && out.contains("0.0750477"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "signrank", file, "--y", "a,b", "--test", "paired", "--tail", "lower"]);
    assert!(ok, "{err}");
    assert!(out.contains("Wilcoxon signed rank test") && out.contains("a vs b"), "{out}");

    let (ok, out, err) = sheaf(&["analyze", "kruskal", file, "--y", "a,b,c"]);
    assert!(ok, "{err}");
    // chi-square = 0.2929 with 2 degrees of freedom, p = 0.8638 (Origin: 0.29286571 and 0.86378374).
    assert!(
        out.contains("chi-square(2) = 0.2929") && out.contains("p = 0.8638") && out.contains("KruskalWallis1"),
        "{out}"
    );

    let (ok, out, err) = sheaf(&["analyze", "corr", file, "--y", "a,b,c", "--spearman", "--kendall"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("a vs b")
            && out.contains("Pearson r")
            && out.contains("Kendall tau")
            && out.contains("Correlation1"),
        "{out}"
    );
    let (ok, out, err) = sheaf(&["analyze", "corr", file, "--y", "a,b", "--listwise", "--no-pearson", "--spearman"]);
    assert!(ok, "{err}");
    assert!(out.contains("listwise") && !out.contains("Pearson"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "corr", file, "--y", "a"]);
    assert!(!ok && err.contains("at least two columns"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "corr", file, "--y", "a,b", "--no-pearson"]);
    assert!(!ok && err.contains("at least one of Pearson"), "{err}");
}

/// Distribution fit, probability plots and power calculations (spec `statistics.md` section 12). The numbers of
/// the calc crate are checked against scipy in `crates/calc/tests/scipy_stats_dist.rs`; here the wiring.
#[test]
fn distribution_fit_probability_plots_and_power_run_from_the_command_line() {
    let dir = scratch("distfit");
    let path = dir.join("sample.csv");
    std::fs::write(&path, "x\n1.2\n2.3\n2.9\n3.1\n4.4\n5.0\n6.8\n7.7\n9.1\n12.5\n").unwrap();
    let file = path.to_str().unwrap();

    let (ok, out, err) = sheaf(&["analyze", "distfit", file, "--y", "x"]);
    assert!(ok, "{err}");
    for name in ["Normal", "Lognormal", "Exponential", "Gamma", "Weibull", "Log-likelihood", "AIC", "BIC"] {
        assert!(out.contains(name), "{name}: {out}");
    }
    assert!(out.contains("DistFit1") && out.contains("lowest AIC"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "distfit", file, "--y", "x", "--family", "gamma,weibull"]);
    assert!(ok, "{err}");
    assert!(out.contains("Gamma") && out.contains("Weibull") && !out.contains("Lognormal"), "{out}");

    let project = dir.join("pp.sheaf");
    let (ok, out, err) = sheaf(&["analyze", "probplot", file, "--y", "x", "--save", project.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(out.contains("Probability plot") && out.contains("ProbPlot1"), "{out}");
    let (ok, info, err) = sheaf(&["info", project.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(info.contains("Data (ordered)") && info.contains("Percent"), "{info}");
    let (ok, out, err) = sheaf(&["analyze", "qqplot", file, "--y", "x", "--family", "weibull", "--no-graph"]);
    assert!(ok, "{err}");
    assert!(out.contains("Q-Q plot") && out.contains("Slope"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "qqplot", file, "--y", "x", "--family", "bogus"]);
    assert!(!ok && err.contains("bogus"), "{err}");

    // Two-sample t-test, d = 0.5, power 0.8: 64 per group (a well known table value).
    let (ok, out, err) =
        sheaf(&["analyze", "power", file, "--test", "two-sample", "--effect", "0.5", "--power", "0.8"]);
    assert!(ok, "{err}");
    assert!(out.contains("takes 64 and 64 observations"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "power", file, "--test", "two-sample", "--effect", "0.5", "--n", "64"]);
    assert!(ok, "{err}");
    assert!(out.contains("the power is 0.80"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "power", file, "--test", "anova", "--means", "4,5,6", "--sd", "2", "--n", "10,20"]);
    assert!(ok, "{err}");
    assert!(out.contains("One-way ANOVA of 3 groups"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "power", file, "--test", "one-sample", "--difference=-5", "--sd", "10", "--tail", "upper"]);
    assert!(ok, "{err}");
    assert!(out.contains("upper-tailed"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "power", file, "--test", "anova", "--effect", "0.25"]);
    assert!(!ok && err.contains("at least two groups"), "{err}");
}
