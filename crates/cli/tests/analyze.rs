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
fn column_math_goes_row_by_row() {
    let dir = scratch("column-math");
    let data = data_file(&dir);
    let out = dir.join("colmath.csv");
    // ramp minus flat (2), row by row; Add is the default operator as in Origin.
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "column-math",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--op",
        "subtract",
        "--ref",
        "flat",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Column math"), "{stdout}");
    let rows: Vec<String> = std::fs::read_to_string(&out).unwrap().lines().skip(3).map(str::to_string).collect();
    assert_eq!(rows[0], "0,2,0,-2");
    assert_eq!(rows[10], "10,2,10,8");
    let (ok, _, stderr) =
        sheaf(&["analyze", "column-math", data.to_str().unwrap(), "--y", "ramp", "--value", "1", "--common"]);
    assert!(!ok && stderr.contains("row by row"), "{stderr}");
}

#[test]
fn convolution_and_signal_correlation_write_a_new_sheet() {
    let dir = scratch("signal");
    let data = data_file(&dir);
    let out = dir.join("conv.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "convolution",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--response",
        "flat",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Convolution"), "{stdout}");
    let rows: Vec<String> = std::fs::read_to_string(&out).unwrap().lines().skip(3).map(str::to_string).collect();
    assert_eq!(rows.len(), 21, "11 + 11 - 1");
    // 0..10 convolved with eleven 2s: 0, 2, 6, ...
    assert_eq!(rows[..3], ["0,0", "1,2", "2,6"]);
    let out = dir.join("corr.csv");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "signal-correlation",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--with",
        "ramp",
        "--circular",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    // Names and comments (no units row: lags and correlations have none), then the values.
    let rows: Vec<String> = std::fs::read_to_string(&out).unwrap().lines().skip(2).map(str::to_string).collect();
    assert_eq!(rows.len(), 11, "circular: lags 0 to 10");
    // Origin's circular order starts from the far end: lag 0 holds sum a[j - 1] a[j] = 330.
    assert_eq!(rows[0], "0,330");
}

#[test]
fn fft_writes_a_spectrum() {
    let dir = scratch("fft");
    let data = data_file(&dir);
    let out = dir.join("fft.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "fft",
        data.to_str().unwrap(),
        "--y",
        "flat",
        "--window",
        "hanning",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("FFT"), "{stdout}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("Frequency,Real,Imaginary,Magnitude,Amplitude,Phase,Power,dB"), "{text}");
    // A constant 2 with the amplitude-corrected window: amplitude 2 at frequency 0.
    let first = text.lines().find(|l| l.starts_with("0,")).unwrap();
    assert_eq!(first.split(',').nth(4), Some("2"), "{text}");
}

#[test]
fn ifft_of_two_columns() {
    let dir = scratch("ifft");
    let data = data_file(&dir);
    let out = dir.join("ifft.csv");
    // ramp as the real part and flat (2) as the imaginary part, in FFT order.
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "ifft",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--imaginary",
        "flat",
        "--no-shift",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("IFFT"), "{stdout}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("Time,Real,Imaginary,Amplitude,Phase"), "{text}");
    // Row 0 is the mean of each part: 5 + 2i.
    let row0 = text.lines().find(|l| l.starts_with("0,")).unwrap();
    assert!(row0.starts_with("0,5,2,"), "{row0}");
}

#[test]
fn fft_filter_adds_a_column() {
    let dir = scratch("fft-filter");
    let data = data_file(&dir);
    let out = dir.join("filtered.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "fft-filter",
        data.to_str().unwrap(),
        "--y",
        "flat",
        "--type",
        "high-pass",
        "--cutoff",
        "0.1",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("FFT filter"), "{stdout}");
    // The high pass keeps the offset of a constant: still 2 beside the input.
    let text = std::fs::read_to_string(&out).unwrap();
    let row = text.lines().find(|l| l.starts_with("0,")).unwrap();
    assert!(row.starts_with("0,2,2,"), "{text}");
}

#[test]
fn hilbert_adds_a_column() {
    let dir = scratch("hilbert");
    let data = data_file(&dir);
    let (ok, stdout, stderr) = sheaf(&["analyze", "hilbert", data.to_str().unwrap(), "--y", "ramp"]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Hilbert transform"), "{stdout}");
}

#[test]
fn level_crossing_lists_the_crossings() {
    let dir = scratch("level-crossing");
    let data = data_file(&dir);
    let out = dir.join("lx.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "level-crossing",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--level",
        "2.5",
        "--row",
        "right",
        "--details",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Level crossing") && stdout.contains("C reaches Y = 2.5 once"), "{stdout}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.lines().next().unwrap().contains("CrossingX1"), "{text}");
    // t, flat, ramp, then X, direction, the right-hand row (X 2.5 lies between rows 3 and 4) and Y.
    assert!(text.lines().any(|l| l == "0,2,0,2.5,1,4,2.5"), "{text}");
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

    let (ok, out, err) =
        sheaf(&["analyze", "anova", file, "--y", "untreated,treated,third", "--comparison", "holm-bonferroni"]);
    assert!(ok, "{err}");
    assert!(out.contains("Bonferroni-Holm means comparison") && out.contains("sorted by p-value"), "{out}");

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

/// Distribution fit and probability plots (spec `statistics.md` section 16). The numbers of the calc crate are
/// checked against scipy in `crates/calc/tests/scipy_stats_dist.rs`; here the wiring.
#[test]
fn distribution_fit_and_probability_plots_run_from_the_command_line() {
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
}

fn dunnett_groups_file(dir: &Path) -> PathBuf {
    let g = [
        [24.0, 26.5, 23.1, 25.9, 27.2, 25.0],
        [28.1, 30.2, 27.7, 29.9, 31.0, 28.4],
        [24.5, 25.0, 26.1, 23.8, 25.5, 27.0],
        [33.0, 31.5, 34.2, 32.2, 30.9, 33.3],
    ];
    let mut text = String::from("ctrl,drugA,drugB,drugC\n");
    for i in 0..6 {
        text.push_str(&format!("{},{},{},{}\n", g[0][i], g[1][i], g[2][i], g[3][i]));
    }
    let path = dir.join("groups.csv");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn anova_with_dunnett_comparison() {
    let dir = scratch("holm-dunnett");
    let data = dunnett_groups_file(&dir);
    let file = data.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "anova", file, "--y", "A,B,C,D", "--comparison", "dunnett"]);
    assert!(ok, "{err}");
    assert!(out.contains("Dunnett means comparison") && out.contains("against the control ctrl"), "{out}");
    assert!(out.contains("drugC - ctrl") && !out.contains("drugB - drugA"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "anova", file, "--y", "A,B,C,D", "--comparison", "dunnett", "--control", "4"]);
    assert!(ok, "{err}");
    assert!(out.contains("ctrl - drugC") && out.contains("against the control drugC"), "{out}");
    let (ok, _, err) =
        sheaf(&["analyze", "anova", file, "--y", "A,B,C,D", "--comparison", "dunnett", "--control", "9"]);
    assert!(!ok && err.contains("control"), "{err}");
    let (ok, _, err) =
        sheaf(&["analyze", "anova", file, "--y", "A,B,C,D", "--comparison", "dunnett", "--control", "0"]);
    assert!(!ok && err.contains("from 1"), "{err}");
}

#[test]
fn crosstab_counts_percents_and_tests() {
    let dir = scratch("crosstab");
    let path = dir.join("survey.csv");
    std::fs::write(&path, "treated,better\ntreated,better\ntreated,same\ncontrol,same\ncontrol,same\ncontrol,better\ncontrol,same\ntreated,better\n").unwrap();
    let export = dir.join("crosstab.csv");
    let (ok, out, err) =
        sheaf(&["analyze", "crosstab", path.to_str().unwrap(), "--y", "A,B", "--export", export.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(out.contains("Pearson chi-square(1) = 2") && out.contains("Fisher's exact test"), "{out}");
    assert!(out.contains("Crosstab1"), "{out}");
    let exported = std::fs::read_to_string(&export).unwrap();
    assert!(
        exported.contains("control") && exported.contains("Row %") && exported.contains("Odds ratio"),
        "{exported}"
    );
    let (ok, _, err) = sheaf(&["analyze", "crosstab", path.to_str().unwrap(), "--y", "A"]);
    assert!(!ok && err.contains("exactly two columns"), "{err}");
    // There is no X column to limit: refused before anything runs.
    let (ok, _, err) = sheaf(&["analyze", "crosstab", path.to_str().unwrap(), "--y", "A,B", "--x-range", "1:3"]);
    assert!(!ok && err.contains("no X column to limit") && err.contains("--rows"), "{err}");
    let (ok, out, err) = sheaf(&["analyze", "crosstab", path.to_str().unwrap(), "--y", "A,B", "--rows", "1:6"]);
    assert!(ok && out.contains("Crosstab1"), "{err}");
}

#[test]
fn anova2_refuses_dunnett_and_an_x_range() {
    let dir = scratch("anova2-dunnett");
    let data = design_file(&dir);
    let file = data.to_str().unwrap();
    let (ok, _, err) = sheaf(&["analyze", "anova2", file, "--y", "dose,batch,response", "--comparison", "dunnett"]);
    assert!(!ok && err.contains("one-way ANOVA only"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "anova2", file, "--y", "dose,batch,response", "--x-range", "1:3"]);
    assert!(!ok && err.contains("no X column to limit"), "{err}");
}

#[test]
fn power_needs_no_file_and_prints_origins_numbers() {
    // Two samples, means 5 and 6, sd 1.5, n = 10 per group: Origin gives 0.2921101.
    let (ok, out, err) = sheaf(&[
        "analyze",
        "power",
        "--test",
        "two-sample",
        "--mean1",
        "5",
        "--mean2",
        "6",
        "--sd",
        "1.5",
        "--sizes",
        "10,30",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("0.29211") && out.contains("0.71873"), "{out}");
    let (ok, out, err) = sheaf(&[
        "analyze",
        "power",
        "--test",
        "two-sample",
        "--mean1",
        "5",
        "--mean2",
        "6",
        "--sd",
        "1.5",
        "--powers",
        "0.8,0.9",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("37") && out.contains("49"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "power", "--test", "anova"]);
    assert!(!ok && err.contains("either --sizes"), "{err}");
}

#[test]
fn bin2d_counts_points_in_a_grid() {
    let dir = scratch("bin2d");
    let file = dir.join("points.csv");
    let x = [0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 1.2, 1.7, 2.2, 8.1, 8.8, 10.0, 0.0];
    let y = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 2.2, 2.7, 9.1, 0.4, 3.3, 5.0, 0.0];
    let text: String =
        std::iter::once("x,y\n".to_string()).chain(x.iter().zip(y).map(|(a, b)| format!("{a},{b}\n"))).collect();
    std::fs::write(&file, text).unwrap();
    let file = file.to_str().unwrap();
    let grid = ["--x-min", "0", "--x-max", "10", "--x-inc", "2", "--y-min", "0", "--y-max", "10", "--y-inc", "5"];
    let args: Vec<&str> = ["analyze", "bin2d", file, "--y", "y"].into_iter().chain(grid).collect();
    let (ok, out, err) = sheaf(&args);
    assert!(ok, "{err}");
    assert!(out.contains("Binning2D1") && out.contains("17 points") && out.contains("5 X bins by 2 Y bins"), "{out}");
    let args: Vec<&str> = ["analyze", "bin2d", file, "--y", "x,y", "--stat", "mean"].into_iter().chain(grid).collect();
    let (ok, out, err) = sheaf(&args);
    assert!(ok, "{err}");
    assert!(out.contains("(mean)"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "bin2d", file, "--y", "y", "--stat", "mode"]);
    assert!(!ok && err.contains("mode"), "{err}");
}

#[test]
fn discfreq_counts_the_distinct_values() {
    let dir = scratch("discfreq");
    let file = dir.join("cats.csv");
    std::fs::write(&file, "kind,n\nb,3\na,1\nB,3\nc,2\na,3\n").unwrap();
    let file = file.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "discfreq", file, "--y", "kind,n", "--relative", "--percent"]);
    assert!(ok, "{err}");
    assert!(out.contains("DiscretFreq1") && out.contains("5 cells, 3 distinct values"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "discfreq", file, "--y", "kind", "--no-count"]);
    assert!(!ok && err.contains("at least one"), "{err}");
}
