//! Runs `sheaf simulate-surface` and checks what it prints and writes. The numbers are those of the Simulate
//! Surface tool of `sheaf-core` (through `sheaf_ops::simulate_surface`) and, for Plane and Gauss2D, those of their
//! formulas and of Origin's window (fitting.md 16).

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-simsurface-{tag}-{}", std::process::id()));
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

/// The numeric rows (three columns) of a written or printed CSV; header and comment rows are skipped.
fn xyz(csv: &str) -> Vec<[f64; 3]> {
    csv.lines()
        .filter_map(|l| l.split(',').map(|c| c.trim().parse::<f64>().ok()).collect::<Option<Vec<f64>>>())
        .filter(|r| r.len() == 3)
        .map(|r| [r[0], r[1], r[2]])
        .collect()
}

#[test]
fn a_plane_grid_is_written_as_csv_with_the_typed_parameters() {
    let dir = scratch("plane");
    let out = dir.join("plane.csv");
    let (ok, stdout, stderr) = sheaf(&[
        "simulate-surface",
        "--function",
        "Plane",
        "--param",
        "a=3,b=4",
        "--x-min",
        "0",
        "--x-max",
        "2",
        "--x-points",
        "3",
        "--y-min",
        "0",
        "--y-max",
        "1",
        "--y-points",
        "2",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("Simulated Plane: 6 points"), "{stdout}");
    let rows = xyz(&std::fs::read_to_string(&out).unwrap());
    assert_eq!(rows.len(), 6);
    // X changes fastest; z = 3x + 4y.
    assert_eq!(rows[0], [0.0, 0.0, 0.0]);
    assert_eq!(rows[2], [2.0, 0.0, 6.0]);
    assert_eq!(rows[5], [2.0, 1.0, 10.0]);
}

#[test]
fn the_default_is_origins_gauss2d_window_with_noise_that_a_seed_repeats() {
    let (ok, stdout, stderr) = sheaf(&["simulate-surface", "--noise", "0"]);
    assert!(ok, "stderr: {stderr}");
    let rows = xyz(&stdout);
    assert_eq!(rows.len(), 400, "20 by 20 points");
    assert_eq!(rows[0][..2], [-1.0, -1.0]);
    assert!((rows[0][2] - 1.4945338524781e-05).abs() < 1e-17, "{}", rows[0][2]);
    let a = sheaf(&["simulate-surface", "--seed", "4"]).1;
    let b = sheaf(&["simulate-surface", "--seed", "4"]).1;
    let c = sheaf(&["simulate-surface", "--seed", "5"]).1;
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(xyz(&a), rows, "the default noise is 10 %");
}

#[test]
fn list_and_errors_say_what_is_wrong() {
    let (ok, stdout, _) = sheaf(&["simulate-surface", "--list"]);
    assert!(ok);
    assert!(stdout.lines().count() >= 22 && stdout.contains("Gauss2D") && stdout.contains("Rational2D"), "{stdout}");
    let (ok, _, stderr) = sheaf(&["simulate-surface", "--function", "Nope"]);
    assert!(!ok && stderr.contains("no surface function"), "{stderr}");
    let (ok, _, stderr) = sheaf(&["simulate-surface", "--x-points", "1"]);
    assert!(!ok && stderr.contains("too few"), "{stderr}");
    let (ok, _, stderr) = sheaf(&["simulate-surface", "--param", "q=1"]);
    assert!(!ok && stderr.contains("no parameter"), "{stderr}");
}
