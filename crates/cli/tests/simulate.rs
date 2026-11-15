//! Runs `sheaf simulate` and checks what it prints and writes. The numbers are those of the Simulate Curve
//! tool of `sheaf-core` (through `sheaf_ops::simulate`) and, for Gauss, those of its formula.

use sheaf_ops::simulate::{SimulateSpec, run_standalone};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-simulate-{tag}-{}", std::process::id()));
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

/// The two numeric columns of a written or printed CSV; the header and comment rows are skipped.
fn xy(csv: &str) -> (Vec<f64>, Vec<f64>) {
    let rows: Vec<Vec<f64>> = csv
        .lines()
        .filter_map(|l| l.split(',').map(|c| c.trim().parse::<f64>().ok()).collect::<Option<Vec<f64>>>())
        .filter(|r| r.len() == 2)
        .collect();
    (rows.iter().map(|r| r[0]).collect(), rows.iter().map(|r| r[1]).collect())
}

fn gauss(x: f64, y0: f64, xc: f64, w: f64, a: f64) -> f64 {
    y0 + a / (w * (std::f64::consts::PI / 2.0).sqrt()) * (-2.0 * ((x - xc) / w).powi(2)).exp()
}

#[test]
fn a_gauss_curve_is_written_as_csv_with_the_typed_parameters() {
    let dir = scratch("gauss");
    let out = dir.join("curve.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "simulate",
        "--function",
        "gauss",
        "--param",
        "y0=1",
        "--param",
        "xc=5,w=2",
        "--param",
        "A=4",
        "--from",
        "0",
        "--to",
        "10",
        "--points",
        "21",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("Parameters: y0=1, xc=5, w=2, A=4"), "{stdout}");
    assert!(stdout.contains("Simulated Gauss: 21 points, results in sheet Simulate1"), "{stdout}");
    let (x, y) = xy(&std::fs::read_to_string(&out).unwrap());
    assert_eq!(x.len(), 21);
    assert_eq!((x[0], x[10], x[20]), (0.0, 5.0, 10.0));
    for (xi, yi) in x.iter().zip(&y) {
        let want = gauss(*xi, 1.0, 5.0, 2.0, 4.0);
        assert!((yi - want).abs() <= 1e-12 * want.abs().max(1.0), "x={xi}: {yi} against {want}");
    }
    // The same numbers as the shared operation, which the MCP server calls too.
    let mut spec = SimulateSpec::new("Gauss");
    spec.params = ["y0=1", "xc=5", "w=2", "A=4"].map(String::from).to_vec();
    spec.from = Some(0.0);
    spec.to = Some(10.0);
    spec.points = Some(21);
    let (project, run) = run_standalone(&spec).unwrap();
    let ws = &project.workbook(run.book).unwrap().sheets[run.result_sheet];
    let expected: Vec<f64> = ws.columns[1].numeric().unwrap().to_vec();
    for (a, b) in y.iter().zip(&expected) {
        assert!((a - b).abs() <= 1e-12 * b.abs().max(1.0), "{a} against {b}");
    }
}

#[test]
fn log_spacing_and_the_defaults_of_the_window() {
    let dir = scratch("log");
    let out = dir.join("decay.csv");
    let (ok, _, stderr) = sheaf(&[
        "simulate",
        "-f",
        "ExpDec1",
        "--log",
        "--from",
        "1",
        "--to",
        "100",
        "--points",
        "3",
        "-p",
        "A1=2",
        "-p",
        "t1=10",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{stderr}");
    let (x, y) = xy(&std::fs::read_to_string(&out).unwrap());
    assert_eq!(x, [1.0, 10.0, 100.0]);
    for (xi, yi) in x.iter().zip(&y) {
        let want = 1.0 + 2.0 * (-xi / 10.0).exp();
        assert!((yi - want).abs() <= 1e-12, "x={xi}: {yi} against {want}");
    }
    // Nothing typed: Origin's window for Gauss, 200 points from -1 to 5, printed as CSV on standard output.
    let (ok, stdout, stderr) = sheaf(&["simulate"]);
    assert!(ok, "{stderr}");
    let (x, _) = xy(&stdout);
    assert_eq!((x.len(), x[0], x[199]), (200, -1.0, 5.0));
    assert!(stderr.contains("Simulated Gauss: 200 points"), "{stderr}");
    assert!(!stdout.contains("Parameters"), "the notes stay out of the curve: {stdout}");
}

#[test]
fn the_x_of_a_column_is_used_and_the_sheet_goes_into_the_saved_project() {
    let dir = scratch("xof");
    let data = dir.join("data.csv");
    std::fs::write(&data, "T,Signal\n0.5,1\n1.5,2\n2.5,3\n").unwrap();
    let project = dir.join("out.sheaf");
    let out = dir.join("curve.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "simulate",
        data.to_str().unwrap(),
        "--function",
        "Gauss",
        "--param",
        "xc=1.5",
        "--x-of",
        "T",
        "--save",
        project.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("results in sheet Simulate1"), "{stdout}");
    let (x, y) = xy(&std::fs::read_to_string(&out).unwrap());
    assert_eq!(x, [0.5, 1.5, 2.5]);
    assert_eq!(y[0], y[2]);
    assert!(y[1] > y[0]);
    let (ok, info, stderr) = sheaf(&["info", project.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!(info.contains("Simulate1"), "{info}");
}

#[test]
fn plot_writes_a_figure_and_the_project_keeps_the_graph() {
    let dir = scratch("plot");
    let figure = dir.join("curve.svg");
    let project = dir.join("curve.sheaf");
    let (ok, stdout, stderr) =
        sheaf(&["simulate", "-f", "Lorentz", "--plot", figure.to_str().unwrap(), "--save", project.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("graph Graph1"), "{stdout}");
    assert!(std::fs::read_to_string(&figure).unwrap().contains("<svg"));
    let (_, info, _) = sheaf(&["info", project.to_str().unwrap()]);
    assert!(info.contains("Graph1") && info.contains("Simulate1"), "{info}");
}

#[test]
fn the_list_names_the_functions_with_their_starting_values() {
    let (ok, stdout, stderr) = sheaf(&["simulate", "--list"]);
    assert!(ok, "{stderr}");
    let line = stdout.lines().find(|l| l.split_whitespace().next() == Some("Gauss")).expect("Gauss is listed");
    assert!(line.contains("y0=") && line.contains("xc=") && line.contains("w=") && line.contains("A="), "{line}");
}

#[test]
fn bad_requests_fail_with_a_message_and_write_nothing() {
    let dir = scratch("bad");
    let out = dir.join("never.csv");
    let o = out.to_str().unwrap();
    let data = dir.join("d.csv");
    std::fs::write(&data, "T,Y\n1,2\n2,3\n").unwrap();
    let d = data.to_str().unwrap();
    for (args, want) in [
        (vec!["simulate", "-f", "NoSuchFunction", "-o", o], "no built-in fit function named 'NoSuchFunction'"),
        (vec!["simulate", "-p", "zz=1", "-o", o], "no parameter 'zz'"),
        (vec!["simulate", "-p", "xc", "-o", o], "must be written NAME=VALUE"),
        (vec!["simulate", "--points", "1", "-o", o], "too few"),
        (vec!["simulate", "--from", "5", "--to", "5", "-o", o], "two different values"),
        (vec!["simulate", "--log", "--from", "-1", "--to", "5", "-o", o], "positive"),
        (vec!["simulate", "--x-of", "T", "-o", o], "need an input"),
        (vec!["simulate", d, "--x-of", "T", "--from", "1", "-o", o], "cannot be used with"),
        (vec!["simulate", d, "--x-of", "Nope", "-o", o], "column 'Nope' was not found"),
        (vec!["simulate", d, "--sheet", "Sheet1", "-o", o], "needs no sheet"),
    ] {
        let (ok, _, stderr) = sheaf(&args);
        assert!(!ok, "{args:?} should fail");
        assert!(stderr.contains(want), "{args:?}: {stderr}");
    }
    assert!(!Path::new(o).exists());
}
