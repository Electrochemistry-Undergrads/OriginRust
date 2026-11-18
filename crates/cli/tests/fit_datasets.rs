//! Runs `sheaf fit-implicit` and `sheaf compare-datasets` on small CSV files and checks what they print and write
//! (fitting.md 17 and 18). The circle is Origin's measured data (tools/origin-ref/t91/circle-data.json).

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-fitdatasets-{tag}-{}", std::process::id()));
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

const X: [f64; 24] = [
    6.8771, 6.7559, 5.9526, 5.2031, 3.9579, 2.8619, 1.6235, 0.2221, -0.7142, -1.9336, -2.5504, -2.9623, -2.9532,
    -2.5886, -2.2021, -1.2785, -0.1731, 1.1322, 2.7632, 3.8372, 4.8968, 5.8634, 6.5663, 6.9798,
];
const Y: [f64; 24] = [
    3.5294, 4.7287, 5.9146, 6.9587, 7.4546, 7.9726, 8.0536, 7.6544, 7.0358, 6.0738, 5.0668, 3.8005, 2.5562, 1.247,
    0.1363, -0.8845, -1.5579, -1.9176, -1.9789, -1.7822, -1.0059, -0.1353, 0.8643, 2.1296,
];

fn circle_csv(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("circle.csv");
    let mut text = String::from("x,y\n");
    for (x, y) in X.iter().zip(&Y) {
        text.push_str(&format!("{x},{y}\n"));
    }
    std::fs::write(&path, text).unwrap();
    path
}

/// The number after `name` in the first printed line that starts with it.
fn value(stdout: &str, name: &str) -> f64 {
    let line =
        stdout.lines().find(|l| l.starts_with(name)).unwrap_or_else(|| panic!("no line for {name} in\n{stdout}"));
    line[name.len()..].split_whitespace().next().unwrap().parse().unwrap()
}

#[test]
fn the_circle_of_origins_data_is_printed_and_the_report_sheet_written() {
    let dir = scratch("circle");
    let input = circle_csv(&dir);
    let out = dir.join("report.csv");
    let (ok, stdout, stderr) =
        sheaf(&["fit-implicit", input.to_str().unwrap(), "--x", "A", "--y", "B", "--output", out.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!((value(&stdout, "xc") - 2.0013316).abs() < 1e-5, "{stdout}");
    assert!((value(&stdout, "r") - 5.0035929).abs() < 1e-5, "{stdout}");
    assert_eq!(value(&stdout, "Degrees of freedom"), 21.0);
    let written = std::fs::read_to_string(&out).unwrap();
    assert!(written.contains("Standard Error") && written.contains("yc"), "{written}");
}

#[test]
fn an_unknown_function_or_missing_column_is_an_error() {
    let dir = scratch("errors");
    let input = circle_csv(&dir);
    let i = input.to_str().unwrap();
    let (ok, _, stderr) = sheaf(&["fit-implicit", i, "--x", "A", "--y", "B", "--function", "PlaneMod"]);
    assert!(!ok && stderr.contains("no implicit function"), "{stderr}");
    let (ok, _, stderr) = sheaf(&["fit-implicit", i, "--x", "A", "--y", "Nope"]);
    assert!(!ok && stderr.contains("Nope"), "{stderr}");
}

#[test]
fn two_datasets_are_compared_and_the_boxes_leave_blocks_out() {
    let dir = scratch("compare");
    let path = dir.join("two.csv");
    let mut text = String::from("x,a,x2,b\n");
    for i in 0..10 {
        let x = f64::from(i);
        text.push_str(&format!(
            "{x},{},{x},{}\n",
            1.0 + 2.0 * x + 0.02 * (3.0 * x).sin(),
            4.0 + 2.0 * x + 0.02 * (5.0 * x).sin()
        ));
    }
    std::fs::write(&path, text).unwrap();
    let p = path.to_str().unwrap();
    let (ok, stdout, stderr) = sheaf(&["compare-datasets", p, "--y1", "B", "--y2", "D"]);
    assert!(ok, "{stderr}");
    assert!(
        stdout.contains("Data1") && stdout.lines().any(|l| l.starts_with("Intercept")) && stdout.contains("Prob>F"),
        "{stdout}"
    );
    assert!(stdout.contains("are different"), "{stdout}");
    let (ok, stdout, stderr) = sheaf(&[
        "compare-datasets",
        p,
        "--y1",
        "B",
        "--y2",
        "D",
        "--no-parameters",
        "--no-statistics",
        "--name1",
        "Old",
        "--name2",
        "New",
    ]);
    assert!(ok, "{stderr}");
    assert!(
        stdout.contains("Old")
            && !stdout.lines().any(|l| l.starts_with("Intercept") || l.starts_with("Number of points")),
        "{stdout}"
    );
}
