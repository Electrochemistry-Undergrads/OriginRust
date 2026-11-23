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
fn normalize_matches_a_reference_column_or_divides_by_a_reference_cell() {
    // T260: Origin's normalize methods ref and cell.
    let dir = scratch("normalize-ref");
    let data = data_file(&dir);
    let d = data.to_str().unwrap();
    let normalized = |extra: &[&str], tag: &str| -> Vec<f64> {
        let project = dir.join(format!("{tag}.sheaf"));
        let mut args = vec!["analyze", "normalize", d, "--y", "ramp"];
        args.extend_from_slice(extra);
        args.extend_from_slice(&["--save", project.to_str().unwrap()]);
        let (ok, _, stderr) = sheaf(&args);
        assert!(ok, "{args:?}: {stderr}");
        let p = sheaf_io::project_file::load(&project).unwrap();
        let c =
            p.workbooks().next().unwrap().sheets[0].columns.iter().find(|c| c.long_name == "Normalized ramp").unwrap();
        c.numeric().unwrap().to_vec()
    };
    // The ramp's maximum (10) is made flat's maximum (2): times 0.2.
    let v = normalized(&["--method", "ref", "--ref", "flat", "--stat", "max"], "ref");
    assert!(v.iter().enumerate().all(|(i, x)| (x - 0.2 * i as f64).abs() < 1e-12), "{v:?}");
    // Divided by flat's third cell (2).
    let v = normalized(&["--method", "cell", "--ref", "flat", "--row", "3"], "cell");
    assert_eq!(v[8], 4.0);
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
    // T302: the result is a column at the end of the input's sheet (row 2: t 1, flat 2, ramp 1, and the
    // interpolated value 2.5 at X 2.5 of the 5-point grid).
    assert!(text.lines().any(|l| l == "1,2,1,2.5"), "{text}");

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
    // Row 2 of the result column (the fourth column, after the names, units and comments lines).
    let middle: f64 = text.lines().nth(4).unwrap().split(',').nth(3).unwrap().parse().unwrap();
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
        (vec!["analyze", "normalize", d, "--method", "ref"], "--ref COLUMN"),
        (vec!["analyze", "normalize", d, "--method", "cell", "--ref", "flat", "--row", "0"], "counts from 1"),
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
fn envelope_writes_both_curves_to_a_new_sheet() {
    let dir = scratch("envelope");
    let y = [0.0, 2.0, 1.0, 3.0, 0.5, 4.0, 0.2, 2.0, 1.0, 5.0, 4.0, 3.0];
    let mut text = String::from("t,y\n");
    for (i, v) in y.iter().enumerate() {
        text.push_str(&format!("{i},{v}\n"));
    }
    let data = dir.join("peaks.csv");
    std::fs::write(&data, text).unwrap();
    let out = dir.join("env.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "envelope",
        data.to_str().unwrap(),
        "--y",
        "y",
        "--type",
        "both",
        "--npts",
        "1",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Envelope"), "{stdout}");
    let text = std::fs::read_to_string(&out).unwrap();
    let rows: Vec<&str> = text.lines().filter(|l| l.starts_with("0,") || l.starts_with("9,")).collect();
    // Origin: the upper curve starts at 1.6138392857142858 and passes through the peak 5 at row 9.
    assert!(rows[0].starts_with("0,1.61383928571429,0,1.5"), "{text}");
    assert!(rows[1].starts_with("9,5,9,"), "{text}");
    // Too few extremes: refused.
    let (ok, _, stderr) = sheaf(&["analyze", "envelope", data.to_str().unwrap(), "--y", "y", "--npts", "10"]);
    assert!(!ok && stderr.contains("extreme"), "{stderr}");
}

#[test]
fn coherence_writes_frequency_and_coherence() {
    let dir = scratch("coherence");
    let data = data_file(&dir);
    let out = dir.join("coherence.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "coherence",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--with",
        "ramp",
        "--fft-len",
        "8",
        "--win-len",
        "8",
        "--overlap",
        "4",
        "--window",
        "hanning",
        "--interval",
        "0.5",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Coherence"), "{stdout}");
    // A signal with itself has coherence 1 at every frequency: 0 to 1 in steps of 0.25.
    let text = std::fs::read_to_string(&out).unwrap();
    let rows: Vec<&str> =
        text.lines().filter(|l| l.contains(',') && l.starts_with(|c: char| c.is_ascii_digit())).collect();
    assert_eq!(rows.len(), 5, "{text}");
    assert_eq!(rows[0], "0,1", "{text}");
    assert_eq!(rows[4], "1,1", "{text}");
    // A window longer than the FFT is refused.
    let (ok, _, _) = sheaf(&[
        "analyze",
        "coherence",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--with",
        "flat",
        "--fft-len",
        "8",
        "--win-len",
        "16",
    ]);
    assert!(!ok);
}

#[test]
fn decimate_writes_every_nth_point_to_a_new_sheet() {
    let dir = scratch("decimate");
    let data = data_file(&dir);
    let out = dir.join("decimated.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "decimate",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--factor",
        "2",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Decimation"), "{stdout}");
    let rows: Vec<String> = std::fs::read_to_string(&out).unwrap().lines().skip(3).map(str::to_string).collect();
    assert_eq!(rows, ["0,0", "2,2", "4,4", "6,6", "8,8", "10,10"]);
    // Origin refuses a factor of 3 on 11 points.
    let (ok, _, stderr) = sheaf(&["analyze", "decimate", data.to_str().unwrap(), "--y", "ramp", "--factor", "3"]);
    assert!(!ok && stderr.contains("at least 30 points"), "{stderr}");
}

#[test]
fn trace_interp_resamples_a_loop_into_a_new_sheet() {
    let dir = scratch("trace-interp");
    // Origin's loop: equal Y neighbours merge, leaving the diamond (1,0) (3,1) (1,2) (-1,1)
    let data = dir.join("loop.csv");
    std::fs::write(
        &data,
        "v,i
V,A
0,0
2,0
3,1
2,2
0,2
-1,1
",
    )
    .unwrap();
    let out = dir.join("trace.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "trace-interp",
        data.to_str().unwrap(),
        "--y",
        "i",
        "--points",
        "10",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Trace interpolation"), "{stdout}");
    // T326: the X and Y go to the end of the input sheet (v, i, InterpolatedX1, InterpolatedY1).
    let rows: Vec<String> = std::fs::read_to_string(&out).unwrap().lines().skip(3).map(str::to_string).collect();
    assert_eq!(rows.len(), 10);
    let last_two =
        |r: &str| r.rsplitn(3, ',').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(",");
    assert_eq!(
        (last_two(&rows[0]), last_two(&rows[3]), last_two(&rows[9])),
        ("1,0".into(), "3,1".into(), "-1,1".into())
    );
    // One point is no trace.
    let (ok, _, stderr) = sheaf(&["analyze", "trace-interp", data.to_str().unwrap(), "--y", "i", "--points", "1"]);
    assert!(!ok && !stderr.is_empty());
}

#[test]
fn deconvolution_and_stft_write_a_new_sheet() {
    let dir = scratch("deconv-stft");
    let data = data_file(&dir);
    let d = data.to_str().unwrap();
    let out = dir.join("deconv.csv");
    let o = out.to_str().unwrap();
    let (ok, stdout, stderr) =
        sheaf(&["analyze", "deconvolution", d, "--y", "flat", "--response", "ramp", "--circular", "--export", o]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Deconvolution"), "{stdout}");
    let rows = std::fs::read_to_string(&out).unwrap().lines().skip(3).count();
    assert_eq!(rows, 11, "circular: as many values as the signal");
    // Linear (the default, as Origin): N - m + 1 values, one here.
    let (ok, _, stderr) = sheaf(&["analyze", "deconvolution", d, "--y", "flat", "--response", "ramp", "--export", o]);
    assert!(ok, "stderr: {stderr}");
    assert_eq!(std::fs::read_to_string(&out).unwrap().lines().skip(3).count(), 1);
    // Eleven 2s have zeros in their spectrum over 11 points: nothing can be divided by it.
    let (ok, _, stderr) = sheaf(&["analyze", "deconvolution", d, "--y", "ramp", "--response", "flat", "--circular"]);
    assert!(!ok && stderr.contains("zero"), "{stderr}");
    let out = dir.join("stft.csv");
    let o = out.to_str().unwrap();
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "stft",
        d,
        "--y",
        "ramp",
        "--window-len",
        "4",
        "--fft-len",
        "8",
        "--overlap",
        "2",
        "--export",
        o,
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("STFT"), "{stdout}");
    let text = std::fs::read_to_string(&out).unwrap();
    let rows: Vec<&str> = text.lines().skip(3).collect();
    assert_eq!(rows.len(), 5, "0 to the Nyquist frequency of 8 points");
    assert_eq!(rows[0].split(',').count(), 5, "frequency and four segments");
    let (ok, _, stderr) = sheaf(&["analyze", "stft", d, "--y", "ramp"]);
    assert!(!ok && stderr.contains("window length"), "11 points, a 64-point window: {stderr}");
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

    // Origin's Voigt as the peak shape.
    let (ok, stdout, stderr) =
        sheaf(&["analyze", "peaks", data.to_str().unwrap(), "--min-height", "0.3", "--fit", "voigt"]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Voigt fit of 2 peaks") && stdout.contains("Fit FWHM"), "{stdout}");

    // A function per peak: the first Lorentz, the rest the --fit function.
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "peaks",
        data.to_str().unwrap(),
        "--min-height",
        "0.3",
        "--fit",
        "gauss",
        "--fit-shapes",
        "lorentz",
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Lorentz+Gauss fit of 2 peaks"), "{stdout}");
    let (ok, _, stderr) =
        sheaf(&["analyze", "peaks", data.to_str().unwrap(), "--min-height", "0.3", "--fit-shapes", "voigt"]);
    assert!(!ok && stderr.contains("--fit"), "the per-peak list needs --fit: {stderr}");

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
fn variance_and_proportion_tests_run_from_the_command_line() {
    let dir = scratch("vartest");
    let data = groups_file(&dir);
    let file = data.to_str().unwrap();
    let (ok, out, err) =
        sheaf(&["analyze", "vartest", file, "--y", "untreated", "--variance", "0.1", "--levels", "95"]);
    assert!(ok, "{err}");
    assert!(out.contains("One-sample test for variance") && out.contains("NOT significantly different"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "vartest", file, "--y", "untreated,treated", "--tail", "upper"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("Two-sample test for variance") && out.contains("Alternative Hypothesis: Variance1/Variance2 > 1"),
        "{out}"
    );
    let (ok, _, err) = sheaf(&["analyze", "vartest", file, "--y", "untreated", "--ratio", "1"]);
    assert!(!ok && err.contains("exactly two columns"), "{err}");

    let (ok, out, err) = sheaf(&["analyze", "proptest", file, "--successes", "7", "--size", "10", "--exact"]);
    assert!(ok, "{err}");
    assert!(out.contains("One-sample proportion test") && out.contains("Null Hypothesis: Proportion = 0.5"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "proptest", file, "--successes", "15,9", "--size", "40,35", "--levels", "90,95"]);
    assert!(ok, "{err}");
    assert!(out.contains("Two-sample proportion test") && out.contains("Proportion1 - Proportion2"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "proptest", file, "--successes", "11", "--size", "10"]);
    assert!(!ok && err.contains("no more successes"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "proptest", file, "--successes", "1,2,3", "--size", "10,10,10"]);
    assert!(!ok && err.contains("one --successes"), "{err}");

    // T267: Poisson rate tests from counts.
    let (ok, out, err) =
        sheaf(&["analyze", "poissontest", file, "--total", "12", "--size", "5", "--exposure", "2", "--rate", "0.8"]);
    assert!(ok, "{err}");
    assert!(out.contains("One-sample Poisson rate test") && out.contains("Null Hypothesis: Rate = 0.8"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "poissontest", file, "--total", "20,8", "--size", "10,8", "--levels", "95"]);
    assert!(ok, "{err}");
    assert!(out.contains("Two-sample Poisson rate test") && out.contains("Rate1 - Rate2"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "poissontest", file, "--total", "3", "--size", "0"]);
    assert!(!ok && err.contains("sample sizes above 0"), "{err}");
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

    // T279: the stem-and-leaf display, its leaf unit in the notes; a bad unit is refused.
    let (ok, out, err) = sheaf(&["analyze", "stemleaf", file, "--y", "untreated", "--leaf-unit", "0.1", "--split"]);
    assert!(ok, "{err}");
    assert!(out.contains("leaf unit 0.1") && out.contains("StemLeaf"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "stemleaf", file, "--y", "untreated", "--leaf-unit=-1"]);
    assert!(!ok && err.contains("leaf unit"), "{err}");

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
    for (((a, b), c), d) in g[0].iter().zip(&g[1]).zip(&g[2]).zip(&g[3]) {
        text.push_str(&format!("{a},{b},{c},{d}\n"));
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

/// Principal component analysis (spec `statistics.md` section 17). The numbers are checked against Origin in
/// `crates/calc/tests/origin_pca.rs`; here the wiring.
#[test]
fn principal_component_analysis_runs_from_the_command_line() {
    let dir = scratch("pca");
    let path = dir.join("table.csv");
    std::fs::write(&path, "a,b,c\n1,2.3,3\n2,3.9,1\n3,6.4,4\n4,7.8,1\n5,10.4,5\n6,11.9,9\n7,14.2,2\n8,15.8,6\n")
        .unwrap();
    let file = path.to_str().unwrap();
    let project = dir.join("pca.sheaf");
    let (ok, out, err) = sheaf(&["analyze", "pca", file, "--y", "a,b,c", "--save", project.to_str().unwrap()]);
    assert!(ok, "{err}");
    for name in ["Eigenvalue", "Percent of variance", "PC1", "PC3", "PCA1", "Loadings of PC1"] {
        assert!(out.contains(name), "{name}: {out}");
    }
    assert!(!out.contains("Chi-square"), "{out}");
    let (ok, info, err) = sheaf(&["info", project.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(info.contains("Principal Component 1") && info.contains("Coefficients of PC2"), "{info}");
    // Origin's pca draws a scree plot, a loading plot and a biplot; a score plot is a switch.
    for graph in ["Scree Plot", "Loading Plot", "Biplot"] {
        assert!(info.contains(graph), "{graph}: {info}");
    }
    assert!(!info.contains("Scores Plot"), "{info}");
    let more = dir.join("pca-more.sheaf");
    let (ok, _, err) = sheaf(&[
        "analyze",
        "pca",
        file,
        "--y",
        "a,b,c",
        "--components",
        "3",
        "--score-plot",
        "--x-component",
        "2",
        "--y-component",
        "3",
        "--no-biplot",
        "--save",
        more.to_str().unwrap(),
    ]);
    assert!(ok, "{err}");
    let (ok, info, err) = sheaf(&["info", more.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(info.contains("Scores Plot") && info.contains("Loading Plot") && !info.contains("Biplot"), "{info}");
    let (ok, out, err) =
        sheaf(&["analyze", "pca", file, "--y", "a,b,c", "--covariance", "--components", "3", "--no-graph"]);
    assert!(ok, "{err}");
    assert!(out.contains("Chi-square") && out.contains("covariance"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "pca", file, "--y", "a"]);
    assert!(!ok && err.contains("at least two columns"), "{err}");
}

/// Quantile regression (spec `statistics.md` section 29, T342): independent columns, then the dependent one.
#[test]
fn quantile_regression_runs_from_the_command_line() {
    let dir = scratch("quantreg");
    let path = dir.join("qr.csv");
    let mut text = String::from("x,y\n");
    for i in 1..=30 {
        let x = f64::from(i);
        text.push_str(&format!("{x},{}\n", 2.0 + 0.5 * x + (7.3 * x).sin()));
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "quantreg", file, "--y", "x,y", "--quantiles", "0.25,0.5"]);
    assert!(ok, "{err}");
    assert!(out.contains("Pseudo R-square") && out.contains("Quantile 0.25") && out.contains("QuantReg1"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "quantreg", file, "--y", "x,y", "--no-intercept", "--no-graph"]);
    assert!(ok && !out.contains("Intercept"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "quantreg", file, "--y", "x,y", "--quantiles", "1.2"]);
    assert!(!ok && err.contains("between 0 and 1"), "{err}");
}

/// Time series pivot (spec `statistics.md` section 32, T358).
#[test]
fn time_series_pivot_runs_from_the_command_line() {
    let dir = scratch("timepivot");
    let path = dir.join("tp.csv");
    std::fs::write(&path, "t,v\n2026-01-05 09:00,1\n2026-01-05 17:00,3\n2026-01-06 08:00,10\n2026-02-01 12:00,7\n")
        .unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "timepivot", file, "--y", "v", "--period", "month", "--stat", "sum"]);
    assert!(ok, "{err}");
    assert!(out.contains("2026-01") && out.contains("2026-02") && out.contains("Sum of"), "{out}");
    assert!(out.contains("2 period(s)") && out.contains("TimePivot1"), "{out}");
    let (ok, out, err) = sheaf(&["analyze", "timepivot", file, "--y", "v"]);
    assert!(ok && out.contains("3 period(s)"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "timepivot", file, "--y", "v", "--period", "fortnight"]);
    assert!(!ok && err.contains("--period"), "{err}");
}

/// Manhattan plot (spec `statistics.md` section 33, T362).
#[test]
fn manhattan_plot_runs_from_the_command_line() {
    let dir = scratch("manhattan");
    let path = dir.join("gwas.csv");
    std::fs::write(&path, "chrom,pos,p\n1,10,0.5\n1,100,1e-9\n2,20,0.01\n2,50,2e-8\nX,5,0.3\n").unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "manhattan", file, "--y", "chrom,pos,p"]);
    assert!(ok, "{err}");
    assert!(out.contains("3 groups, 5 points; 2 at or above the line") && out.contains("Manhattan1"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "manhattan", file, "--y", "chrom,pos,p", "--threshold", "0.05", "--no-graph"]);
    assert!(ok && out.contains("3 at or above"), "{err} {out}");
    // Two columns are the position and the p-value with the group from the sheet's X: chrom is X itself here.
    let (ok, _, err) = sheaf(&["analyze", "manhattan", file, "--y", "chrom,pos"]);
    assert!(!ok && err.contains("choose the group column"), "{err}");
    let (ok, out, err) = sheaf(&["analyze", "manhattan", file, "--y", "pos,p"]);
    assert!(ok && out.contains("3 groups"), "the group from the X column: {err} {out}");
}

/// Distribution overview of right-censored data (spec `statistics.md` section 37, T366).
#[test]
fn distribution_overview_runs_from_the_command_line() {
    let dir = scratch("lifedist");
    let path = dir.join("life.csv");
    std::fs::write(&path, "id,t,c\n1,2,0\n2,4,0\n3,6,1\n4,8,0\n5,9,1\n").unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "lifedist", file, "--y", "t,c"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("Weibull fit to 5 cases (3 failures, 2 right-censored)") && out.contains("LifeDist1"),
        "{out}"
    );
    let (ok, out, err) =
        sheaf(&["analyze", "lifedist", file, "--y", "t,c", "--distribution", "exponential", "--no-graph"]);
    assert!(ok && out.contains("Mean (theta) 9.66"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "lifedist", file, "--y", "t,c", "--distribution", "gamma"]);
    assert!(!ok && err.contains("--distribution"), "{err}");
}

/// Stability study (spec `statistics.md` section 38, T368).
#[test]
fn stability_study_runs_from_the_command_line() {
    let dir = scratch("stability");
    let path = dir.join("stab.csv");
    let mut text = String::from("t,assay,batch\n");
    for (b, start) in [(1, 100.0), (2, 100.0)] {
        for (i, t) in [0.0, 3.0, 6.0, 9.0, 12.0].iter().enumerate() {
            let wobble = if i % 2 == 0 { 0.1 } else { -0.1 } * f64::from(b);
            text.push_str(&format!("{t},{},{b}\n", start - 0.5 * t + wobble));
        }
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "stability", file, "--y", "t,assay,batch", "--lsl", "90"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("2 batch(es), 10 points; model: one line for all batches") && out.contains("Stability1"),
        "{out}"
    );
    let (ok, out, err) =
        sheaf(&["analyze", "stability", file, "--y", "t,assay", "--lsl", "90", "--usl", "110", "--no-graph"]);
    assert!(ok && out.contains("1 batch(es)") && out.contains("two-sided"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "stability", file, "--y", "t,assay"]);
    assert!(!ok && err.contains("specification limit"), "{err}");
}

/// Process capability (spec `statistics.md` section 36, T365).
#[test]
fn process_capability_runs_from_the_command_line() {
    let dir = scratch("capability");
    let path = dir.join("cap.csv");
    std::fs::write(&path, "i,v\n1,10\n2,12\n3,11\n4,13\n5,12\n6,14\n").unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "capability", file, "--y", "v", "--lsl", "5", "--usl", "20"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("6 values, mean 12") && out.contains("moving range of 2") && out.contains("Capability1"),
        "{out}"
    );
    let (ok, out, err) =
        sheaf(&["analyze", "capability", file, "--y", "v", "--usl", "20", "--model", "weibull", "--no-graph"]);
    assert!(ok && out.contains("Weibull fit") && out.contains("percentile method"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "capability", file, "--y", "v"]);
    assert!(!ok && err.contains("specification limit"), "{err}");
}

/// Rare event charts (spec `statistics.md` section 35, T364).
#[test]
fn rare_event_charts_run_from_the_command_line() {
    let dir = scratch("rarechart");
    let path = dir.join("rare.csv");
    std::fs::write(&path, "n,g,t\n1,10,2\n2,25,4\n3,3,6\n4,40,8\n5,18,3\n6,300,5\n7,12,7\n").unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "rarechart", file, "--y", "g", "--limits", "sigma"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("G chart of 7 values") && out.contains("point(s) 6 beyond") && out.contains("RareChart1"),
        "{out}"
    );
    let (ok, out, err) = sheaf(&[
        "analyze",
        "rarechart",
        file,
        "--y",
        "t",
        "--chart",
        "t",
        "--distribution",
        "exponential",
        "--no-graph",
    ]);
    assert!(ok && out.contains("T chart of 7 values (exponential mean 5)"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "rarechart", file, "--y", "g", "--chart", "x"]);
    assert!(!ok && err.contains("--chart"), "{err}");
}

/// Random forest (spec `statistics.md` section 44, T377).
#[test]
fn random_forest_runs_from_the_command_line() {
    let dir = scratch("randomforest");
    let path = dir.join("rf.csv");
    let mut text = String::from("a,b,kind,size\n");
    for i in 0..40 {
        let kind = if i % 10 >= 5 { "high" } else { "low" };
        text.push_str(&format!("{},{},{kind},{}\n", i % 10, (i * 7) % 5, 2 * (i % 10)));
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "randomforest", file, "--y", "a,b,kind", "--trees", "20"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("Classification forest of 20 trees on 40 complete rows (2 classes)")
            && out.contains("RandomForest1"),
        "{out}"
    );
    let (ok, out, err) = sheaf(&["analyze", "randomforest", file, "--y", "a,b,size", "--trees", "10", "--no-graph"]);
    assert!(ok && out.contains("Regression forest"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "randomforest", file, "--y", "a,b,kind", "--task", "boosting"]);
    assert!(!ok && err.contains("--task"), "{err}");
}

/// ARIMA forecast (spec `statistics.md` section 43, T376).
#[test]
fn arima_forecast_runs_from_the_command_line() {
    let dir = scratch("arima");
    let path = dir.join("ts.csv");
    std::fs::write(&path, "t,v\n0,0\n1,1\n2,3\n3,4\n4,6\n5,7\n6,9\n").unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "arima", file, "--y", "v", "--p", "0", "--d", "1", "--horizon", "2"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("ARIMA(0,1,0) by conditional least squares: Constant (drift) 1.5") && out.contains("ARIMA1"),
        "{out}"
    );
    let (ok, out, err) =
        sheaf(&["analyze", "arima", file, "--y", "v", "--p", "1", "--d", "1", "--no-constant", "--no-graph"]);
    assert!(ok && out.contains("ARIMA(1,1,0)") && out.contains("AR 1"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "arima", file, "--y", "v", "--p", "0", "--no-constant"]);
    assert!(!ok && err.contains("nothing to fit"), "{err}");
}

/// Exponential smoothing forecast (spec `statistics.md` section 42, T374).
#[test]
fn exponential_smoothing_forecast_runs_from_the_command_line() {
    let dir = scratch("forecast");
    let path = dir.join("ts.csv");
    let mut text = String::from("t,v\n");
    for i in 0..24 {
        text.push_str(&format!("{i},{}\n", 10.0 + 0.2 * f64::from(i) + [2.0, -1.0, 0.5, -1.5][i as usize % 4]));
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) =
        sheaf(&["analyze", "forecast", file, "--y", "v", "--model", "additive", "--period", "4", "--horizon", "4"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("additive Holt-Winters smoothing, fitted alpha")
            && out.contains("4 forecast(s)")
            && out.contains("Forecast1"),
        "{out}"
    );
    let (ok, out, err) =
        sheaf(&["analyze", "forecast", file, "--y", "v", "--model", "simple", "--constants", "0.5", "--no-graph"]);
    assert!(ok && out.contains("simple smoothing, given alpha = 0.5"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "forecast", file, "--y", "v", "--constants", "0.5"]);
    assert!(!ok && err.contains("takes 2 constant"), "{err}");
}

/// Stationarity tests (spec `statistics.md` section 41, T372).
#[test]
fn stationarity_tests_run_from_the_command_line() {
    let dir = scratch("stationarity");
    let path = dir.join("ts.csv");
    let mut text = String::from("t,walk\n");
    let (mut st, mut y) = (7u64, 0.0);
    for i in 0..150 {
        st = st.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        y += (st >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
        text.push_str(&format!("{i},{y}\n"));
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "stationarity", file, "--y", "walk"]);
    assert!(ok, "{err}");
    assert!(out.contains("does not reject a unit root at 5%") && out.contains("Stationarity1"), "{out}");
    let (ok, out, err) = sheaf(&[
        "analyze",
        "stationarity",
        file,
        "--y",
        "walk",
        "--adf-terms",
        "trend",
        "--adf-lags",
        "2",
        "--kpss-null",
        "trend",
    ]);
    assert!(ok && out.contains("with 2 lag(s)") && out.contains("trend stationarity"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "stationarity", file, "--y", "walk", "--kpss-null", "drift"]);
    assert!(!ok && err.contains("--kpss-null"), "{err}");
}

/// Seasonal decomposition (spec `statistics.md` section 40, T370).
#[test]
fn seasonal_decomposition_runs_from_the_command_line() {
    let dir = scratch("decompose");
    let path = dir.join("ts.csv");
    let pattern = [1.0, -2.0, 0.5, 0.5];
    let mut text = String::from("t,v\n");
    for i in 0..16 {
        text.push_str(&format!("{i},{}\n", 3.0 + 0.5 * f64::from(i) + pattern[i as usize % 4]));
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "decompose", file, "--y", "v", "--period", "4"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("additive decomposition with period 4; seasonal indices from -2 to 1")
            && out.contains("Decompose1"),
        "{out}"
    );
    let (ok, out, err) =
        sheaf(&["analyze", "decompose", file, "--y", "v", "--period", "4", "--model", "multiplicative", "--no-graph"]);
    assert!(ok && out.contains("multiplicative decomposition"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "decompose", file, "--y", "v", "--period", "9"]);
    assert!(!ok && err.contains("two full periods"), "{err}");
}

/// Autocorrelation (spec `statistics.md` section 34, T363).
#[test]
fn autocorrelation_runs_from_the_command_line() {
    let dir = scratch("acf");
    let path = dir.join("ts.csv");
    let mut text = String::from("t,v\n");
    for i in 0..50 {
        text.push_str(&format!("{i},{}\n", if i % 2 == 0 { 1.0 } else { -1.0 } + 0.01 * f64::from(i)));
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "acf", file, "--y", "v"]);
    assert!(ok, "{err}");
    assert!(out.contains("50 values, lags 1 to 16") && out.contains("ACF1"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "acf", file, "--y", "v", "--lags", "4", "--confidence", "99", "--no-graph"]);
    assert!(ok && out.contains("lags 1 to 4") && out.contains("99%"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "acf", file, "--y", "v", "--lags", "60"]);
    assert!(!ok && err.contains("too few"), "{err}");
}

/// Binned scatter (spec `statistics.md` section 31, T356).
#[test]
fn binned_scatter_runs_from_the_command_line() {
    let dir = scratch("binscatter");
    let path = dir.join("bs.csv");
    let mut text = String::from("x,y\n");
    for i in 0..30 {
        text.push_str(&format!("{i},{}\n", i % 3));
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "binscatter", file, "--y", "y", "--bins", "5"]);
    assert!(ok, "{err}");
    assert!(out.contains("5 bins of 30 points") && out.contains("BinScatter1"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "binscatter", file, "--y", "y", "--bins", "3", "--equal-width", "--no-graph"]);
    assert!(ok && out.contains("equal width"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "binscatter", file, "--y", "y", "--bins", "0"]);
    assert!(!ok && err.contains("--bins"), "{err}");
}

/// Change point analysis (spec `statistics.md` section 30, T346): each --y column is a series.
#[test]
fn change_point_analysis_runs_from_the_command_line() {
    let dir = scratch("changepoint");
    let path = dir.join("cp.csv");
    let mut text = String::from("t,v\n");
    for i in 0..60 {
        let level = if i < 25 { 1.0 } else { 6.0 };
        text.push_str(&format!("{i},{}\n", level + 0.2 * (f64::from(i * i) * 0.77).sin()));
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "changepoint", file, "--y", "v"]);
    assert!(ok, "{err}");
    assert!(out.contains("1 change point(s)") && out.contains("= 25") && out.contains("ChangePoint1"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "changepoint", file, "--y", "v", "--change", "meanvar", "--penalty", "30", "--no-graph"]);
    assert!(ok && out.contains("penalty 30 per change"), "{err} {out}");
    let (ok, _, err) = sheaf(&["analyze", "changepoint", file, "--y", "v", "--penalty", "huge"]);
    assert!(!ok && err.contains("--penalty"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "changepoint", file, "--y", "v", "--change", "slope"]);
    assert!(!ok && err.contains("--change"), "{err}");
}

/// ROC curve (spec `statistics.md` section 19): the --y columns are the test variables, the state column last.
#[test]
fn roc_curve_runs_from_the_command_line() {
    let dir = scratch("roc");
    let path = dir.join("marker.csv");
    std::fs::write(&path, "score,state\n3,yes\n4,yes\n5,yes\n1,no\n2,no\n4,no\n").unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "roc", file, "--y", "score,state", "--positive", "yes"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("Area") && out.contains("0.8333") && out.contains("ROC1") && out.contains("as Origin's"),
        "{out}"
    );
    let (ok, out, err) = sheaf(&[
        "analyze",
        "roc",
        file,
        "--y",
        "score,state",
        "--positive",
        "yes",
        "--lower-is-positive",
        "--no-graph",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("lower test values") && out.contains("0.1667"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "roc", file, "--y", "score,state", "--positive", "maybe"]);
    assert!(!ok && err.contains("one state"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "roc", file, "--y", "score"]);
    assert!(!ok && err.contains("at least two columns"), "{err}");
}

/// Kaplan-Meier estimator (spec `statistics.md` section 20): time, censor and group columns.
#[test]
fn kaplan_meier_runs_from_the_command_line() {
    let dir = scratch("kaplan");
    let path = dir.join("surv.csv");
    // Origin's measured example, `tools/origin-ref/kaplan-suite1.lines.txt`: 14 cases, 4 censored (1), two groups.
    std::fs::write(
        &path,
        "time,censor,group\n3,0,1\n5,0,2\n5,1,1\n6,0,2\n8,0,1\n8,0,2\n9,1,2\n12,0,1\n12,0,2\n15,1,1\n17,0,2\n20,0,1\n22,1,2\n25,0,1\n",
    )
    .unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "kaplan-meier", file, "--y", "time,censor"]);
    assert!(ok, "{err}");
    assert!(out.contains("KaplanMeier1") && out.contains("14.10"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "kaplan-meier", file, "--y", "time,censor,group", "--log-rank", "--breslow", "--no-graph"]);
    assert!(ok, "{err}");
    assert!(out.contains("Log-rank test: chi-square 0.2542") && out.contains("Breslow test"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "kaplan-meier", file, "--y", "time"]);
    assert!(!ok && err.contains("two or three columns"), "{err}");
}

/// Graphical summary (spec `statistics.md` section 22): grouped input gives a table block and a graph per group.
#[test]
fn graphsummary_runs_from_the_command_line() {
    let dir = scratch("gsum");
    let path = dir.join("gs.csv");
    let mut text = String::from("value,other,grp\n");
    for i in 0..30 {
        let v = 10.0 + (f64::from(i) * 1.3).sin() * 3.0 + f64::from(i) * 0.1;
        text.push_str(&format!("{v:.4},{},{}\n", v * 2.0, if i % 3 == 0 { "ctl" } else { "trt" }));
    }
    std::fs::write(&path, text).unwrap();
    let file = path.to_str().unwrap();
    let tsv = dir.join("out.tsv");
    let (ok, out, err) = sheaf(&[
        "analyze",
        "graphsummary",
        file,
        "--y",
        "value",
        "--groups",
        "grp",
        "--level",
        "90",
        "--export",
        tsv.to_str().unwrap(),
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("GraphSummary1") && out.contains("graphs"), "{out}");
    let exported = std::fs::read_to_string(&tsv).unwrap();
    for needle in ["Description Statistics", "90% Confidence Intervals", "Normality Test", "ctl", "trt", "A-Squared"] {
        assert!(exported.contains(needle), "{needle} missing in {exported}");
    }
    let (ok, _, err) = sheaf(&["analyze", "graphsummary", file, "--y", "value", "--arrangement", "per-group"]);
    assert!(!ok && err.contains("not available yet"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "graphsummary", file, "--y", "value", "--arrangement", "nonsense"]);
    assert!(!ok && err.contains("--arrangement"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "graphsummary", file, "--y", "value", "--level", "120"]);
    assert!(!ok && err.contains("between 0 and 100"), "{err}");
}

/// K-means cluster analysis (spec `statistics.md` section 18): Origin's measured 12-case example in two variables.
#[test]
fn kmeans_runs_from_the_command_line() {
    let dir = scratch("kmeans");
    let path = dir.join("points.csv");
    std::fs::write(&path, "a,b\n1,1\n1.5,2\n3,4\n5,7\n3.5,5\n4.5,5\n3.5,4.5\n6,8\n7.5,7\n8,9\n7,6.5\n9,8.5\n").unwrap();
    let file = path.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "kmeans", file, "--y", "a,b", "--clusters", "2"]);
    assert!(ok, "{err}");
    // Origin: two clusters of six, sums of squares 23.0417 and 15.0417.
    assert!(out.contains("KMeans1") && out.contains("23.04") && out.contains("15.04"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "kmeans", file, "--y", "a,b", "--clusters", "3", "--no-graph", "--no-anova"]);
    assert!(ok, "{err}");
    assert!(out.contains("Cluster 3: 4 cases"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "kmeans", file, "--y", "a,b", "--clusters", "20"]);
    assert!(!ok && err.contains("cannot be made"), "{err}");
}

/// Four groups in columns, the third and fourth clearly apart from the first two.
fn four_groups_file(dir: &Path) -> PathBuf {
    let path = dir.join("groups.csv");
    std::fs::write(
        &path,
        "A,B,C,D\n9.1,10.0,13.2,17.9\n10.2,11.1,14.1,18.8\n9.7,10.4,13.8,19.4\n10.9,11.6,15.0,18.1\n9.4,10.3,14.4,17.2\n",
    )
    .unwrap();
    path
}

#[test]
fn anova_comparison_plot_writes_an_svg_with_brackets_and_labels() {
    let dir = scratch("comparison-plot");
    let data = four_groups_file(&dir);
    let out = dir.join("plot.svg");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "anova",
        data.to_str().unwrap(),
        "--y",
        "A,B,C,D",
        "--comparison-plot",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("multiple comparisons plot"), "{stdout}");
    let svg = std::fs::read_to_string(&out).unwrap();
    // Four category labels and asterisk labels of the significant pairs (Tukey: 5 of 6 are below 0.05).
    for name in ["A", "B", "C", "D"] {
        assert!(svg.contains(&format!(">{name}<")), "{name}");
    }
    assert!(svg.matches(">****<").count() >= 3, "{svg}");
    // P-value labels and all pairs, as a box plot.
    let all = dir.join("all.svg");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "anova",
        data.to_str().unwrap(),
        "--y",
        "A,B,C,D",
        "--comparison-plot",
        all.to_str().unwrap(),
        "--plot-type",
        "box",
        "--labels",
        "p-values",
        "--all-pairs",
    ]);
    assert!(ok, "stderr: {stderr}");
    let svg = std::fs::read_to_string(&all).unwrap();
    assert!(svg.matches(">p ").count() + svg.matches("p = ").count() + svg.matches("p &lt; ").count() >= 6, "{svg}");
}

#[test]
fn ttest_comparison_plot_and_the_refusals() {
    let dir = scratch("comparison-ttest");
    let data = four_groups_file(&dir);
    let out = dir.join("t.svg");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "ttest",
        data.to_str().unwrap(),
        "--y",
        "A,D",
        "--comparison-plot",
        out.to_str().unwrap(),
        "--error-bar",
        "ci95",
        "--plot-type",
        "scatter",
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(std::fs::read_to_string(&out).unwrap().contains(">****<"));
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "ttest",
        data.to_str().unwrap(),
        "--y",
        "A",
        "--test",
        "one-sample",
        "--comparison-plot",
        out.to_str().unwrap(),
    ]);
    assert!(!ok && stderr.contains("two-sample"), "{stderr}");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "anova",
        data.to_str().unwrap(),
        "--y",
        "A,B",
        "--comparison",
        "none",
        "--comparison-plot",
        out.to_str().unwrap(),
    ]);
    assert!(!ok && stderr.contains("means comparison"), "{stderr}");
}

#[test]
fn partial_corr_reproduces_an_origin_run_and_explains_its_refusals() {
    let dir = scratch("partial-corr");
    let file = dir.join("pc.csv");
    // The 20 rows of the Origin run with two controlling variables: r = 0.3195, p = 0.19622, df = 16.
    std::fs::write(
        &file,
        "a,b,c,d
0.632,4.903,-1.224,0.095
2.929,7.068,0.378,0.145
2.723,8.643,0.995,-0.345
1.709,5.173,-0.513,-0.062
0.907,4.698,-1.329,0.273
1.646,5.831,-0.066,0.692
3.531,6.826,0.478,1.097
2.656,7.268,1.098,0.21
1.02,5.578,-1.216,-1.79
0.775,3.524,-1.293,0.702
2.334,6.63,0.706,-1.42
2.373,6.334,0.474,-0.171
3.316,9.095,1.534,-1.474
2.562,8.275,0.982,0.002
2.13,7.155,-0.102,-0.779
1.767,6.4,-0.274,0.214
7.435,9.746,2.571,3.3
2.119,6.412,-0.289,-0.049
1.819,5.007,-0.785,0.842
-0.479,5.129,-1.138,-1.323
",
    )
    .unwrap();
    let file = file.to_str().unwrap();
    let (ok, out, err) =
        sheaf(&["analyze", "partial-corr", file, "--y", "a,b", "--control", "c,d", "--show-significance"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("0.319499") && out.contains("0.196219") && out.contains("Controlling variables: c, d"),
        "{out}"
    );
    assert!(out.contains("Two-tailed test of significance is used.") && out.contains("PartialCorr1"), "{out}");
    let (ok, out, err) =
        sheaf(&["analyze", "partial-corr", file, "--y", "a,b", "--control", "c", "--one-tailed", "--flag-significant"]);
    assert!(ok, "{err}");
    assert!(out.contains("One-tailed test of significance is used."), "{out}");

    // A control is required, a variable cannot control itself, an X range has nothing to limit.
    let (ok, _, err) = sheaf(&["analyze", "partial-corr", file, "--y", "a,b"]);
    assert!(!ok && err.contains("--control"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "partial-corr", file, "--y", "a,b", "--control", "b"]);
    assert!(!ok && err.contains("both a variable and a controlling variable"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "partial-corr", file, "--y", "a,b", "--control", "c", "--x-range", "1:3"]);
    assert!(!ok, "{err}");
}

#[test]
fn anova3_reports_origins_type_three_table_and_options() {
    let dir = scratch("anova3");
    let file = dir.join("a3.csv");
    // The balanced 2 x 3 x 2 design of the Origin run (24 numbers): A has F = 43.28302, p = 2.61651E-5.
    std::fs::write(
        &file,
        "a,b,c,y
lo,b1,c1,15.814
lo,b2,c2,15.4569
hi,b1,c2,17.6589
hi,b3,c1,16.2762
lo,b2,c1,14.8436
lo,b3,c2,15.9664
hi,b2,c2,20.2379
lo,b1,c1,16.4812
lo,b3,c1,14.5347
hi,b1,c2,17.82
hi,b3,c2,15.9952
lo,b2,c1,13.867
hi,b1,c1,17.812
hi,b2,c2,20.4605
lo,b1,c2,17.4733
lo,b3,c1,15.7478
hi,b2,c1,16.2572
hi,b3,c2,15.4057
lo,b2,c2,14.5925
hi,b1,c1,16.4532
hi,b3,c1,17.6181
lo,b1,c2,16.3418
lo,b3,c2,16.499
hi,b2,c1,17.66
",
    )
    .unwrap();
    let file = file.to_str().unwrap();
    let export = dir.join("a3-out.csv");
    let (ok, out, err) = sheaf(&[
        "analyze",
        "anova3",
        file,
        "--y",
        "a,b,c,y",
        "--levene",
        "--power",
        "--power-sizes",
        "50,100",
        "--comparison",
        "tukey,bonferroni",
        "--export",
        export.to_str().unwrap(),
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains("20.234") && out.contains("43.283") && out.contains("a*b*c") && out.contains("Model"),
        "{out}"
    );
    assert!(out.contains("type III") && out.contains("ANOVA3Way1"), "{out}");
    assert!(out.contains("Tukey means comparison") && out.contains("Bonferroni means comparison"), "{out}");
    assert!(out.contains("Levene's test"), "{out}");
    let exported = std::fs::read_to_string(&export).unwrap();
    assert!(exported.contains("Sum of Squares") && exported.contains("a: lo"), "{exported}");

    // Interactions can be left out; the F tests then use the pooled error.
    let (ok, out, err) =
        sheaf(&["analyze", "anova3", file, "--y", "a,b,c,y", "--no-abc", "--no-ac", "--comparison", "none"]);
    assert!(ok, "{err}");
    assert!(!out.contains("a*b*c") && !out.contains("a*c") && out.contains("a*b"), "{out}");

    // Three columns are too few, Dunnett is not offered, an X range has nothing to limit.
    let (ok, _, err) = sheaf(&["analyze", "anova3", file, "--y", "a,b,y"]);
    assert!(!ok && err.contains("FACTOR_A,FACTOR_B,FACTOR_C,DATA"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "anova3", file, "--y", "a,b,c,y", "--comparison", "dunnett"]);
    assert!(!ok && err.contains("Dunnett"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "anova3", file, "--y", "a,b,c,y", "--x-range", "1:3"]);
    assert!(!ok, "{err}");
}

/// The one-way repeated measures run of Origin (8 subjects, 4 levels) as indexed rows `level,y,subject`.
fn rm_one_way_csv() -> String {
    let table = [
        [22.5244, 23.5244, 24.5244, 25.5244],
        [18.138, 19.677, 22.5219, 24.6265],
        [17.0564, 21.0059, 19.8137, 22.3672],
        [23.5919, 22.9043, 26.4075, 24.4996],
        [19.4001, 19.1614, 18.8032, 21.3784],
        [20.3396, 22.2064, 23.9391, 25.4933],
        [21.2722, 21.8676, 23.4055, 25.5475],
        [15.6833, 19.3813, 19.8708, 19.163],
    ];
    let mut text = String::from("level,y,subject\n");
    for (s, row) in table.iter().enumerate() {
        for (l, v) in row.iter().enumerate() {
            text.push_str(&format!("L{},{v},S{}\n", l + 1, s + 1));
        }
    }
    text
}

#[test]
fn rmanova_reports_origins_one_way_tables_and_refuses_what_it_cannot_do() {
    let dir = scratch("rmanova1");
    let file = dir.join("rm1.csv");
    std::fs::write(&file, rm_one_way_csv()).unwrap();
    let file = file.to_str().unwrap();
    let export = dir.join("rm1-out.csv");
    let (ok, out, err) =
        sheaf(&["analyze", "rmanova", file, "--y", "level,y,subject", "--export", export.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(out.contains("One-way repeated measures ANOVA") && out.contains("ANOVAOneWayRM1"), "{out}");
    assert!(out.contains("15.672") && out.contains("Intercept") && out.contains("804.75"), "{out}");
    assert!(out.contains("Tukey means comparison"), "{out}");
    let exported = std::fs::read_to_string(&export).unwrap();
    for text in ["Greenhouse-Geisser", "Huynh-Feldt", "Lower-bound", "0.94848", "0.963657", "9.86391", "3.54149"] {
        assert!(exported.contains(text), "{text} is missing from {exported}");
    }
    let (ok, out, err) = sheaf(&["analyze", "rmanova", file, "--y", "level,y,subject", "--comparison", "none"]);
    assert!(ok && !out.contains("means comparison"), "{out}{err}");

    // Two columns are too few, Dunnett and the interaction of a one-way design are refused.
    let (ok, _, err) = sheaf(&["analyze", "rmanova", file, "--y", "level,y"]);
    assert!(!ok && err.contains("FACTOR,DATA,SUBJECT"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "rmanova", file, "--y", "level,y,subject", "--comparison", "dunnett"]);
    assert!(!ok && err.contains("Dunnett"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "rmanova", file, "--y", "level,y,subject", "--interaction"]);
    assert!(!ok && err.contains("two-way"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "rmanova", file, "--y", "level,y,subject", "--x-range", "1:3"]);
    assert!(!ok, "{err}");
}

#[test]
fn rmanova_two_way_has_an_error_term_per_effect() {
    let dir = scratch("rmanova2");
    let file = dir.join("rm2.csv");
    // The 6 subjects of the Origin run, A (2 levels) by B (3 levels), one number per cell.
    let cells = [
        [32.6833, 33.3234, 31.7134, 35.1833, 34.4234, 33.5134],
        [34.0195, 34.1319, 33.1258, 34.1635, 36.0622, 36.4435],
        [30.368, 30.1007, 29.7489, 32.7849, 30.9238, 31.9115],
        [30.0507, 29.5896, 29.8773, 30.6486, 32.8153, 31.4332],
        [33.6047, 33.1549, 34.0038, 35.4702, 34.2338, 36.4597],
        [32.5875, 32.353, 33.6288, 34.7314, 35.5193, 33.6986],
    ];
    let mut text = String::from("a,b,y,s\n");
    for (s, row) in cells.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            text.push_str(&format!("A{},B{},{v},S{}\n", c / 3 + 1, c % 3 + 1, s + 1));
        }
    }
    std::fs::write(&file, text).unwrap();
    let file = file.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "rmanova", file, "--y", "a,b,y,s", "--two-way", "--interaction"]);
    assert!(ok, "{err}");
    assert!(out.contains("Two-way repeated measures ANOVA") && out.contains("ANOVATwoWayRM1"), "{out}");
    assert!(out.contains("Error(a*b)") && out.contains("2181.6") && out.contains("0.9115"), "{out}");
    // Without the interaction (Origin's default) there is no A*B row.
    let (ok, out, err) = sheaf(&["analyze", "rmanova", file, "--y", "a,b,y,s", "--two-way"]);
    assert!(ok && !out.contains("Error(a*b)"), "{out}{err}");
    let (ok, _, err) = sheaf(&["analyze", "rmanova", file, "--y", "a,b,y,s", "--two-way", "--comparison", "tukey"]);
    assert!(!ok && err.contains("no means comparisons"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "rmanova", file, "--y", "a,y,s", "--two-way"]);
    assert!(!ok && err.contains("FACTOR_A,FACTOR_B,DATA,SUBJECT"), "{err}");
}

/// The rows of an exported sheet that are numbers (the header rows are not).
fn numeric_rows(text: &str) -> Vec<Vec<f64>> {
    text.lines().filter_map(|l| l.split(',').map(|v| v.parse::<f64>().ok()).collect::<Option<Vec<f64>>>()).collect()
}

/// 16 samples of a ramp (column "ramp", 0..15) and a short pair of Haar coefficient columns.
fn wavelet_file(dir: &Path) -> PathBuf {
    let s = std::f64::consts::SQRT_2;
    // x = 4 2 5 5 1 7 0 0: ca = (6 10 8 0) / sqrt2, cd = (2 0 -6 0) / sqrt2 (Haar, Origin's alignment).
    let (ca, cd) = ([6.0 / s, 10.0 / s, 8.0 / s, 0.0], [2.0 / s, 0.0, -6.0 / s, 0.0]);
    let mut text = String::from("t,ramp,ca,cd\n");
    for i in 0..16 {
        let (a, d) = if i < 4 { (ca[i].to_string(), cd[i].to_string()) } else { (String::new(), String::new()) };
        text.push_str(&format!("{i},{i},{a},{d}\n"));
    }
    let path = dir.join("wavelet.csv");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn wavelet_decompose_writes_the_coefficients_and_reconstruct_gives_them_back() {
    let dir = scratch("wavelet");
    let data = wavelet_file(&dir);
    let out = dir.join("dwt.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "wavelet-decompose",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--wavelet",
        "haar",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Wavelet decomposition"), "{stdout}");
    let text = std::fs::read_to_string(&out).unwrap();
    let rows: Vec<Vec<f64>> = numeric_rows(&text);
    assert_eq!(rows.len(), 8);
    // Ramp 0, 1, 2, ..: ca[k] = (x[2k] + x[2k+1]) / sqrt2, cd[k] = (x[2k] - x[2k+1]) / sqrt2 = -1 / sqrt2.
    assert!((rows[0][0] - 1.0 / 2f64.sqrt()).abs() < 1e-12 && (rows[0][1] + 1.0 / 2f64.sqrt()).abs() < 1e-12);
    assert!(text.lines().nth(1).unwrap().contains("Approx Coeff of Haar dwt on"), "{text}");

    let out = dir.join("idwt.csv");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "wavelet-reconstruct",
        data.to_str().unwrap(),
        "--y",
        "ca",
        "--with",
        "cd",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let got: Vec<f64> = numeric_rows(&std::fs::read_to_string(&out).unwrap()).into_iter().map(|r| r[0]).collect();
    let want = [4.0, 2.0, 5.0, 5.0, 1.0, 7.0, 0.0, 0.0];
    assert_eq!(got.len(), 8);
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < 1e-12, "{got:?}");
    }
}

#[test]
fn wavelet_denoise_and_smooth_run_and_refuse_what_origin_refuses() {
    let dir = scratch("wavelet-denoise");
    let data = wavelet_file(&dir);
    let out = dir.join("den.csv");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "wavelet-denoise",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--wavelet",
        "db2",
        "--level",
        "2",
        "--percent",
        "0",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    // A threshold of 0 % keeps every coefficient: the ramp comes back.
    let rows = numeric_rows(&std::fs::read_to_string(&out).unwrap());
    assert_eq!(rows.len(), 16);
    for (i, r) in rows.iter().enumerate() {
        assert!((r[1] - i as f64).abs() < 1e-9, "row {i}: {r:?}");
    }
    // Four levels do not fit 16 periodic points (limit 3), as in Origin.
    let (ok, _, stderr) = sheaf(&["analyze", "wavelet-denoise", data.to_str().unwrap(), "--y", "ramp", "--level", "4"]);
    assert!(!ok && stderr.contains("level 4"), "{stderr}");
    let (ok, _, stderr) =
        sheaf(&["analyze", "wavelet-smooth", data.to_str().unwrap(), "--y", "ramp", "--wavelet", "x"]);
    assert!(!ok && stderr.contains("not a wavelet"), "{stderr}");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "wavelet-smooth",
        data.to_str().unwrap(),
        "--y",
        "ramp",
        "--wavelet",
        "db2",
        "--cutoff",
        "50",
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Wavelet smoothing"), "{stdout}");
}

#[test]
fn cwt_writes_one_column_per_scale_and_refuses_what_origin_refuses() {
    let dir = scratch("cwt");
    // A unit impulse at row 8 of 16, and the scales 1, 2.9, 4.
    let mut text = String::from(
        "t,sig,scale
",
    );
    for i in 0..16 {
        let s = ["1", "2.9", "4"].get(i).copied().unwrap_or("");
        text.push_str(&format!(
            "{i},{},{s}
",
            if i == 8 { 1 } else { 0 }
        ));
    }
    let data = dir.join("cwt.csv");
    std::fs::write(&data, text).unwrap();
    let out = dir.join("cwt-out.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "cwt",
        data.to_str().unwrap(),
        "--y",
        "sig",
        "--with",
        "scale",
        "--wavelet",
        "mexhat",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Continuous wavelet") && stdout.contains("CWTResults1"), "{stdout}");
    let text = std::fs::read_to_string(&out).unwrap();
    let rows = numeric_rows(&text);
    assert_eq!(rows.len(), 16);
    assert_eq!(rows[0].len(), 3);
    // Scale 2.9 is cut to 2, so its column differs from scale 4 and both are centred half a sample after the impulse.
    assert!((rows[8][1] - rows[9][1]).abs() < 1e-12 && (rows[7][1] - rows[10][1]).abs() < 1e-12, "{rows:?}");
    assert!(text.contains("cwt of B, C"), "{text}");
    let (ok, _, stderr) =
        sheaf(&["analyze", "cwt", data.to_str().unwrap(), "--y", "sig", "--with", "scale", "--param", "3"]);
    assert!(!ok && stderr.contains("wave number"), "{stderr}");
}

/// Origin's Peak Analyzer goal Subtract Baseline: User Defined anchors by the 2nd derivative,
/// Subtracted_Data Y1 and Baseline_Data Y1 beside the curve.
#[test]
fn baseline_goal_subtract_writes_origins_columns() {
    let dir = scratch("baseline-goal");
    let mut text = String::from("x,y\n");
    for k in 0..401 {
        let x = k as f64 * 0.05;
        let g = |c: f64, w: f64| (-(x - c) * (x - c) / w).exp();
        text.push_str(&format!("{x},{}\n", 0.2 + 0.01 * x + g(5.0, 0.5) + 0.6 * g(12.0, 0.8)));
    }
    let data = dir.join("two.csv");
    std::fs::write(&data, text).unwrap();
    let out = dir.join("out.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "peaks",
        data.to_str().unwrap(),
        "--baseline-goal",
        "subtract",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("through 8 anchor points"), "{stdout}");
    let csv = std::fs::read_to_string(&out).unwrap();
    let header = csv.lines().next().unwrap();
    assert!(header.contains("Subtracted_Data Y1") && header.contains("Baseline_Data Y1"), "{header}");
    let (ok, _, stderr) =
        sheaf(&["analyze", "peaks", data.to_str().unwrap(), "--baseline-goal", "create", "--integrate"]);
    assert!(!ok && stderr.contains("only makes the baseline"), "{stderr}");
}

/// T296: the charge of each sweep of a cyclic voltammogram.
#[test]
fn cv_charge_splits_the_sweeps() {
    let dir = scratch("cv-charge");
    let data = dir.join("cv.csv");
    std::fs::write(
        &data,
        "E,I
0,2
0.25,2
0.5,2
0.75,2
1,2
0.75,-1
0.5,-1
0.25,-1
0,-1
",
    )
    .unwrap();
    let d = data.to_str().unwrap();
    let (ok, stdout, stderr) = sheaf(&["analyze", "cv-charge", d, "--y", "I", "--scan-rate", "0.1"]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("anodic") && stdout.contains("cathodic"), "{stdout}");
    assert!(stdout.contains("20"), "2 A over 1 V at 0.1 V/s: {stdout}");
    let (ok, _, stderr) = sheaf(&["analyze", "cv-charge", d, "--y", "I", "--scan-rate", "0"]);
    assert!(!ok && stderr.contains("scan rate"), "{stderr}");
}

/// `--baseline als` (T318): the Asymmetric Least Squares baseline stays on the sloping background.
#[test]
fn als_baseline_goal_follows_the_background() {
    let dir = scratch("baseline-als");
    let mut text = String::from("x,y\n");
    for k in 0..401 {
        let x = k as f64 * 0.05;
        text.push_str(&format!("{x},{}\n", 0.2 + 0.01 * x + (-(x - 10.0) * (x - 10.0) / 0.5).exp()));
    }
    let data = dir.join("one.csv");
    std::fs::write(&data, text).unwrap();
    let out = dir.join("out.csv");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "peaks",
        data.to_str().unwrap(),
        "--baseline-goal",
        "create",
        "--baseline",
        "als",
        "--als-smoothing",
        "6",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let csv = std::fs::read_to_string(&out).unwrap();
    let rows = numeric_rows(&csv);
    // Row 200 is x = 10, the top of the peak: the baseline stays near 0.3.
    assert!((rows[200][2] - 0.3).abs() < 0.02, "{:?}", rows[200]);
    let (ok, _, stderr) =
        sheaf(&["analyze", "peaks", data.to_str().unwrap(), "--baseline", "als", "--als-asymmetric", "2"]);
    assert!(!ok && stderr.contains("asymmetric factor"), "{stderr}");
}

/// `analyze fsd` (T341): Fourier self-deconvolution writes the narrowed curve beside the input.
#[test]
fn fsd_narrows_the_bands() {
    let dir = scratch("fsd");
    let mut text = String::from("x,y\n");
    for k in 0..512 {
        let x = k as f64 * 0.05;
        let l = |c: f64| 1.0 / ((x - c) * (x - c) + 1.0);
        text.push_str(&format!("{x},{}\n", l(12.3) + l(13.3)));
    }
    let data = dir.join("two.csv");
    std::fs::write(&data, text).unwrap();
    let out = dir.join("out.csv");
    let (ok, _, stderr) = sheaf(&[
        "analyze",
        "fsd",
        data.to_str().unwrap(),
        "--fwhm",
        "2",
        "--k",
        "3",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    let csv = std::fs::read_to_string(&out).unwrap();
    assert!(csv.lines().next().unwrap().contains("FSD of"), "{csv:.200}");
    let rows = numeric_rows(&csv);
    let v: Vec<f64> = rows.iter().map(|r| r[2]).collect();
    let tops = (1..v.len() - 1).filter(|&i| v[i] > v[i - 1] && v[i] >= v[i + 1] && v[i] > 0.5).count();
    assert_eq!(tops, 2, "two bands after FSD");
    let (ok, _, stderr) = sheaf(&["analyze", "fsd", data.to_str().unwrap(), "--fwhm", "2", "--k", "1"]);
    assert!(!ok && stderr.contains("enhancement factor"), "{stderr}");
}

/// `analyze peaks --fsd-fwhm` (T344): the search on the FSD-narrowed curve finds two hidden bands.
#[test]
fn the_fsd_peak_search_finds_hidden_bands() {
    let dir = scratch("fsd-peaks");
    let mut text = String::from("x,y\n");
    for k in 0..512 {
        let x = k as f64 * 0.05;
        let l = |c: f64| 1.0 / ((x - c) * (x - c) + 1.0);
        text.push_str(&format!("{x},{}\n", 0.1 + l(12.3) + l(13.3)));
    }
    let data = dir.join("two.csv");
    std::fs::write(&data, text).unwrap();
    let (ok, stdout, stderr) =
        sheaf(&["analyze", "peaks", data.to_str().unwrap(), "--integrate", "--baseline", "minimum"]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Peak 1") && !stdout.contains("Peak 2"), "{stdout}");
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "peaks",
        data.to_str().unwrap(),
        "--integrate",
        "--baseline",
        "minimum",
        "--fsd-fwhm",
        "2",
        "--fsd-k",
        "3",
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Peak 2"), "{stdout}");
}

/// `analyze peaks --regions` (T352): piecewise peak fitting, one analysis per region.
#[test]
fn piecewise_regions_are_analysed_on_their_own() {
    let dir = scratch("piecewise");
    let mut text = String::from("x,y\n");
    for k in 0..401 {
        let x = k as f64 * 0.05;
        let g = |c: f64| (-(x - c) * (x - c) / 0.5).exp();
        let y = if x < 10.0 { 0.2 + g(5.0) } else { 1.0 + 0.5 * g(15.0) };
        text.push_str(&format!("{x},{y}\n"));
    }
    let data = dir.join("steps.csv");
    std::fs::write(&data, text).unwrap();
    let (ok, stdout, stderr) = sheaf(&[
        "analyze",
        "peaks",
        data.to_str().unwrap(),
        "--integrate",
        "--baseline",
        "minimum",
        "--regions",
        "0:9.95,10:20",
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("region 1") && stdout.contains("region 2"), "{stdout}");
    let (ok, _, stderr) = sheaf(&["analyze", "peaks", data.to_str().unwrap(), "--regions", "0-9"]);
    assert!(!ok && stderr.contains("region"), "{stderr}");
}

#[test]
fn stats_percentiles_add_one_column_each() {
    let dir = scratch("stats-percentiles");
    let data = groups_file(&dir);
    let file = data.to_str().unwrap();
    let (ok, out, err) = sheaf(&["analyze", "stats", file, "--y", "untreated", "--percentiles", "5,97.5"]);
    assert!(ok, "{err}");
    assert!(out.contains("P5") && out.contains("P97.5"), "{out}");
    let (ok, _, err) = sheaf(&["analyze", "stats", file, "--y", "untreated", "--percentiles", "101"]);
    assert!(!ok && err.contains("between 0 and 100"), "{err}");
}

#[test]
fn acceptance_sampling_designs_and_evaluates_plans_without_a_data_file() {
    // scipy (tools/acceptance-ref.py): AQL 1 %, LTPD 5 %, risks 5 % and 10 % give n 132, c 3.
    let (ok, out, err) = sheaf(&["analyze", "acceptance", "--aql", "0.01", "--ltpd", "0.05"]);
    assert!(ok, "{err}");
    assert!(out.contains("Sample size n") && out.contains("132"), "{out}");
    let dir = scratch("acceptance");
    let csv = dir.join("oc.csv");
    let (ok, out, err) =
        sheaf(&["analyze", "acceptance", "--n", "50", "--c", "1", "--lot", "1000", "--export", csv.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert!(out.contains("AOQL") && out.contains("0.0158626"), "{out}");
    assert!(std::fs::read_to_string(&csv).unwrap().lines().count() > 100);
    let (ok, _, err) = sheaf(&["analyze", "acceptance", "--aql", "0.1", "--ltpd", "0.05"]);
    assert!(!ok && err.contains("below LTPD"), "{err}");
}
