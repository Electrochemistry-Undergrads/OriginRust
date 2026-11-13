//! Runs `sheaf fit` on small files and checks what it prints and writes.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-fit-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// "noisy" in row i: a line with a fixed wiggle; "err" in row i: its error bar.
fn noisy(i: u32) -> f64 {
    1.5 + 0.5 * f64::from(i) + 0.3 * (7.0 * f64::from(i)).sin()
}
fn err(i: u32) -> f64 {
    0.1 + 0.2 * f64::from(i % 3)
}

/// x = 0..20; "line" is exactly 1.5 + 0.5x; "decay" is 3 + 7 exp(-x/4); "noisy" and its errors "err".
fn data_file(dir: &Path) -> PathBuf {
    let mut text = String::from("x,line,decay,noisy,err\nV,mA,mA,mA,mA\n");
    for i in 0..=20 {
        let x = f64::from(i);
        text.push_str(&format!("{x},{},{},{},{}\n", 1.5 + 0.5 * x, 3.0 + 7.0 * (-x / 4.0).exp(), noisy(i), err(i)));
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

/// The cells of the printed row that starts with `name`.
fn row<'a>(stdout: &'a str, name: &str) -> Vec<&'a str> {
    stdout
        .lines()
        .find(|l| l.split_whitespace().next() == Some(name))
        .unwrap_or_else(|| panic!("no row {name} in\n{stdout}"))
        .split_whitespace()
        .collect()
}

#[test]
fn a_linear_fit_prints_parameters_statistics_and_anova() {
    let dir = scratch("linear");
    let data = data_file(&dir);
    let export = dir.join("params.csv");
    let figure = dir.join("fit.svg");
    let project = dir.join("fit.sheaf");
    let (ok, stdout, stderr) = sheaf(&[
        "fit",
        data.to_str().unwrap(),
        "--y",
        "line",
        "--export",
        export.to_str().unwrap(),
        "--plot",
        figure.to_str().unwrap(),
        "--save",
        project.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert_eq!(row(&stdout, "Intercept")[1], "1.5");
    assert_eq!(row(&stdout, "Slope")[1], "0.5");
    assert!(stdout.contains("Number of points"), "{stdout}");
    assert!(stdout.contains("Corrected Total"), "{stdout}");
    assert!(stdout.contains("FitLinear1"), "{stdout}");
    let params = std::fs::read_to_string(&export).unwrap();
    assert!(params.lines().any(|l| l.starts_with("Intercept,1.5")), "{params}");
    let svg = std::fs::read_to_string(&figure).unwrap();
    assert!(svg.contains("Linear fit"), "the legend names the fitted curve");
    assert!(project.exists());
}

#[test]
fn nonlinear_fits_take_starting_values_bounds_and_fixed_parameters() {
    let dir = scratch("nonlinear");
    let data = data_file(&dir);
    let d = data.to_str().unwrap();
    let (ok, stdout, stderr) = sheaf(&["fit", d, "--y", "decay", "--model", "ExpDec1"]);
    assert!(ok, "stderr: {stderr}");
    let t1: f64 = row(&stdout, "t1")[1].parse().unwrap();
    assert!((t1 - 4.0).abs() < 1e-5, "{stdout}");

    let (ok, stdout, stderr) = sheaf(&[
        "fit",
        d,
        "--y",
        "decay",
        "--formula",
        "y0 + A*exp(-x/t)",
        "--params",
        "y0,A,t",
        "--init",
        "A=5,t=2",
        "--fix",
        "y0=3",
        "--bounds",
        "t=0:",
    ]);
    assert!(ok, "stderr: {stderr}");
    assert_eq!(row(&stdout, "y0")[1], "3");
    assert!(row(&stdout, "y0").contains(&"fixed"));
    let t: f64 = row(&stdout, "t")[1].parse().unwrap();
    assert!((t - 4.0).abs() < 1e-5, "{stdout}");
}

#[test]
fn instrumental_weighting_uses_the_error_column_named_with_yerr() {
    let dir = scratch("weights");
    let data = data_file(&dir);
    let (ok, stdout, stderr) =
        sheaf(&["fit", data.to_str().unwrap(), "--y", "noisy", "--weight", "instrumental", "--yerr", "err"]);
    assert!(ok, "stderr: {stderr}");
    // Weighted least squares by hand, with w = 1/err^2.
    let (mut sw, mut sx, mut sy, mut sxx, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for i in 0..=20 {
        let (x, y, w) = (f64::from(i), noisy(i), 1.0 / (err(i) * err(i)));
        sw += w;
        sx += w * x;
        sy += w * y;
        sxx += w * x * x;
        sxy += w * x * y;
    }
    let slope = (sw * sxy - sx * sy) / (sw * sxx - sx * sx);
    let intercept = (sy - slope * sx) / sw;
    let printed = |name: &str| row(&stdout, name)[1].parse::<f64>().unwrap();
    assert!((printed("Slope") - slope).abs() < 1e-5 * slope.abs(), "{stdout}\nwant slope {slope}");
    assert!((printed("Intercept") - intercept).abs() < 1e-5 * intercept.abs().max(1.0), "{stdout}");
    // The unweighted fit gives a different slope, so the weights did something.
    let (_, plain, _) = sheaf(&["fit", data.to_str().unwrap(), "--y", "noisy"]);
    assert_ne!(row(&plain, "Slope")[1], row(&stdout, "Slope")[1]);

    // --yerr without instrumental weighting, and a count mismatch, are refused.
    let (ok, _, stderr) = sheaf(&["fit", data.to_str().unwrap(), "--y", "noisy", "--yerr", "err"]);
    assert!(!ok && stderr.contains("--weight instrumental"), "{stderr}");
    let (ok, _, stderr) =
        sheaf(&["fit", data.to_str().unwrap(), "--y", "noisy", "--weight", "instrumental", "--yerr", "err,line,decay"]);
    assert!(!ok && stderr.contains("one per --y column"), "{stderr}");
}

#[test]
fn the_function_list_and_clear_errors() {
    let (ok, stdout, _) = sheaf(&["fit", "--list"]);
    assert!(ok);
    assert!(stdout.lines().any(|l| l.trim_start().starts_with("Gauss ")), "{stdout}");

    let dir = scratch("errors");
    let data = data_file(&dir);
    let d = data.to_str().unwrap();
    for (args, expect) in [
        (vec!["fit", d, "--model", "NoSuchThing"], "--list"),
        (vec!["fit", d, "--model", "poly"], "poly:2"),
        (vec!["fit", d, "--y", "line", "--init", "Slope"], "NAME=VALUE"),
        (vec!["fit", d, "--y", "line", "--fix", "Nope=1"], "no parameter 'Nope'"),
        (vec!["fit", d, "--y", "line", "--weight", "instrumental"], "Y-error column"),
        (vec!["fit", d, "--confidence", "1.5"], "--confidence"),
    ] {
        let (ok, _, stderr) = sheaf(&args);
        assert!(!ok, "{args:?} should fail");
        assert!(stderr.contains(expect), "{args:?}: {stderr}");
    }
    let (ok, _, stderr) = sheaf(&["fit"]);
    assert!(!ok && stderr.contains("INPUT"), "{stderr}");
}

#[test]
fn a_second_fit_without_y_leaves_the_result_columns_of_the_first_alone() {
    let dir = scratch("refit");
    let data = dir.join("one.csv");
    let mut text = String::from(
        "x,y
",
    );
    for i in 0..=20 {
        let x = f64::from(i);
        text.push_str(&format!(
            "{x},{}
",
            3.0 + 7.0 * (-x / 4.0).exp()
        ));
    }
    std::fs::write(&data, text).unwrap();
    let project = dir.join("it.sheaf");
    let (ok, _, stderr) = sheaf(&["fit", data.to_str().unwrap(), "-m", "ExpDec1", "--save", project.to_str().unwrap()]);
    assert!(ok, "stderr: {stderr}");
    let (ok, stdout, stderr) = sheaf(&["fit", project.to_str().unwrap(), "-m", "linear"]);
    assert!(ok, "stderr: {stderr}");
    assert_eq!(
        stdout.matches("Number of points").count(),
        1,
        "one report set for one column:
{stdout}"
    );
    assert!(!stdout.contains("Fitted y"), "{stdout}");
}

#[test]
fn params_without_formula_is_refused() {
    let dir = scratch("params");
    let data = data_file(&dir);
    let (ok, _, stderr) = sheaf(&["fit", data.to_str().unwrap(), "-y", "line", "-m", "linear", "--params", "a,b"]);
    assert!(!ok);
    assert!(stderr.contains("--formula"), "{stderr}");
    let (ok, _, stderr) = sheaf(&["fit", data.to_str().unwrap(), "-y", "line", "--params", "a,b"]);
    assert!(!ok);
    assert!(stderr.contains("--formula"), "{stderr}");
}

#[test]
fn bands_are_listed_drawn_and_checked() {
    let dir = scratch("bands");
    let data = data_file(&dir);
    let d = data.to_str().unwrap();
    let figure = dir.join("bands.svg");
    let (ok, stdout, stderr) =
        sheaf(&["fit", d, "--y", "noisy", "--bands", "both", "--level", "90", "--plot", figure.to_str().unwrap()]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Bands at 90% confidence level"), "{stdout}");
    assert!(stdout.contains("upper prediction limit") && stdout.contains("lower confidence limit"), "{stdout}");
    let svg = std::fs::read_to_string(&figure).unwrap();
    assert!(svg.contains("upper confidence limit"), "the bands are in the legend");

    let (ok, stdout, _) = sheaf(&["fit", d, "--y", "noisy", "--bands", "prediction"]);
    assert!(ok);
    assert!(stdout.contains("95%") && stdout.contains("prediction limit") && !stdout.contains("confidence limit"));
    let (ok, stdout, _) = sheaf(&["fit", d, "--y", "noisy"]);
    assert!(ok && !stdout.contains("Bands at"));

    for level in ["0", "100", "-5"] {
        let (ok, _, stderr) = sheaf(&["fit", d, "--bands", "both", &format!("--level={level}")]);
        assert!(!ok && stderr.contains("--level"), "{level}: {stderr}");
    }
}

#[test]
fn iteration_controls_are_passed_on_and_checked() {
    let dir = scratch("iterations");
    let data = data_file(&dir);
    let d = data.to_str().unwrap();
    let (ok, stdout, stderr) =
        sheaf(&["fit", d, "--y", "decay", "-m", "ExpDec1", "--max-iterations", "300", "--tolerance", "1e-12"]);
    assert!(ok, "stderr: {stderr}");
    let t1: f64 = row(&stdout, "t1")[1].parse().unwrap();
    assert!((t1 - 4.0).abs() < 1e-5, "{stdout}");
    let (ok, _, stderr) = sheaf(&["fit", d, "--y", "decay", "-m", "ExpDec1", "--max-iterations", "0"]);
    assert!(!ok && stderr.contains("--max-iterations"), "{stderr}");
    let (ok, _, stderr) = sheaf(&["fit", d, "--y", "decay", "-m", "ExpDec1", "--tolerance", "0"]);
    assert!(!ok && stderr.contains("--tolerance"), "{stderr}");
}

#[test]
fn linear_fit_with_x_error_takes_the_x_error_column() {
    let dir = scratch("xerr");
    let mut text = String::from("x,y,sx\n");
    for i in 0..=10 {
        let x = f64::from(i);
        text.push_str(&format!("{x},{},0.1\n", 1.0 + 2.0 * x + 0.2 * (3.0 * x).sin()));
    }
    let path = dir.join("xe.csv");
    std::fs::write(&path, text).unwrap();
    let (ok, out, err) = sheaf(&["fit", path.to_str().unwrap(), "--model", "linearx", "--y", "B", "--xerr", "C"]);
    assert!(ok, "{err}");
    assert!(out.contains("Slope"), "{out}");
    let (ok, _, err) = sheaf(&["fit", path.to_str().unwrap(), "--model", "linearx", "--y", "B"]);
    assert!(!ok);
    assert!(err.contains("X-error"), "{err}");
}

#[test]
fn regress_compare_and_rank_print_their_tables() {
    let dir = scratch("more");
    let data = data_file(&dir);
    let p = data.to_str().unwrap();
    let (ok, out, err) = sheaf(&["fit", p, "--y", "noisy", "--regress", "x,decay"]);
    assert!(ok, "{err}");
    assert!(out.contains("Intercept") && out.contains("Prob>F"), "{out}");
    // An exact exponential can fit to a residual of exactly 0 (it does on macOS on arm64), which
    // leaves AICc undefined; a little fixed noise keeps the comparison meaningful on every machine.
    let mut text = String::from("x,decay\n");
    for i in 0..=20 {
        let x = f64::from(i);
        text.push_str(&format!("{x},{}\n", 3.0 + 7.0 * (-x / 4.0).exp() + 0.01 * (3.0 * x).sin()));
    }
    let noisy_decay = dir.join("decay.csv");
    std::fs::write(&noisy_decay, text).unwrap();
    let q = noisy_decay.to_str().unwrap();
    let (ok, out, err) = sheaf(&["fit", q, "--y", "decay", "--model", "ExpDec1", "--compare", "linear"]);
    assert!(ok, "{err}");
    assert!(out.contains("AICc:   ExpDec1 is preferred"), "{out}");
    let (ok, out, err) = sheaf(&["fit", p, "--y", "decay", "--rank", "ExpDec1,Gauss,ExpDec2"]);
    assert!(ok, "{err}");
    assert!(out.contains("Rank") && out.contains("ExpDec1"), "{out}");
}
