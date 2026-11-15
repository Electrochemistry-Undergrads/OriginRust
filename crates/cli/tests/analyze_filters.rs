//! `sheaf analyze iir-filter` and `fir-filter` (T55) on a synthetic signal: 10 Hz and 200 Hz sines at
//! Fs = 1000 Hz. The checks are physical (the stopband is attenuated, the passband kept); the numbers of
//! the designs against `scipy.signal` are in `crates/calc/tests/scipy_filters.rs`.

use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const N: usize = 1000;

fn signal_file(dir: &Path) -> PathBuf {
    let mut text = String::from("t,y\n");
    for i in 0..N {
        let t = i as f64 / 1000.0;
        let y = (2.0 * PI * 10.0 * t).sin() + (2.0 * PI * 200.0 * t).sin();
        text.push_str(&format!("{t},{y}\n"));
    }
    let path = dir.join("signal.csv");
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

/// Runs `sheaf analyze <args>` on the signal and returns the filtered column.
fn filtered(tag: &str, args: &[&str]) -> Vec<f64> {
    let dir = scratch(tag);
    let data = signal_file(&dir);
    let out = dir.join("out.csv");
    let mut all = vec!["analyze"];
    all.extend_from_slice(args);
    all.extend_from_slice(&[data.to_str().unwrap(), "--y", "B", "--export", out.to_str().unwrap()]);
    let (ok, stdout, stderr) = sheaf(&all);
    assert!(ok, "stderr: {stderr}");
    assert!(stdout.contains("filter"), "{stdout}");
    let text = std::fs::read_to_string(&out).unwrap();
    // names, then the comments (the filter), then the rows
    text.lines().skip(2).map(|l| l.rsplit(',').next().unwrap().parse().unwrap()).collect()
}

/// Amplitude of the sine of `freq` Hz in the middle of the record (a single-frequency DFT over whole periods).
fn amplitude(y: &[f64], freq: f64) -> f64 {
    let (lo, hi) = (200, 800);
    let (mut re, mut im) = (0.0, 0.0);
    for (i, v) in y.iter().enumerate().take(hi).skip(lo) {
        let a = 2.0 * PI * freq * i as f64 / 1000.0;
        re += v * a.cos();
        im += v * a.sin();
    }
    2.0 * re.hypot(im) / (hi - lo) as f64
}

#[test]
fn iir_low_pass_keeps_10_hz_and_removes_200_hz() {
    let y = filtered("iir-low", &["iir-filter", "--order", "4", "--cutoff", "50", "--forward-backward"]);
    assert_eq!(y.len(), N);
    assert!((amplitude(&y, 10.0) - 1.0).abs() < 0.02, "{}", amplitude(&y, 10.0));
    assert!(amplitude(&y, 200.0) < 0.005, "{}", amplitude(&y, 200.0));
}

#[test]
fn iir_high_pass_and_the_short_alias() {
    let y = filtered(
        "iir-high",
        &["iir", "--type", "high-pass", "--method", "chebyshev1", "--order", "6", "--cutoff", "80"],
    );
    assert!(amplitude(&y, 10.0) < 0.02, "{}", amplitude(&y, 10.0));
    assert!((amplitude(&y, 200.0) - 1.0).abs() < 0.15, "{}", amplitude(&y, 200.0));
}

#[test]
fn iir_minimum_order_band_stop_removes_the_200_hz_band() {
    let y = filtered(
        "iir-stop",
        &[
            "iir-filter",
            "--type",
            "band-stop",
            "--method",
            "elliptic",
            "--minimum",
            "--pass",
            "100",
            "--pass2",
            "300",
            "--stop",
            "150",
            "--stop2",
            "250",
            "--ripple",
            "0.1",
            "--attenuation",
            "50",
            "--forward-backward",
        ],
    );
    assert!((amplitude(&y, 10.0) - 1.0).abs() < 0.05, "{}", amplitude(&y, 10.0));
    assert!(amplitude(&y, 200.0) < 0.01, "{}", amplitude(&y, 200.0));
}

#[test]
fn fir_window_and_least_squares_remove_200_hz_without_a_delay() {
    let w = filtered("fir-window", &["fir-filter", "--order", "100", "--cutoff", "50"]);
    assert!((amplitude(&w, 10.0) - 1.0).abs() < 0.02, "{}", amplitude(&w, 10.0));
    assert!(amplitude(&w, 200.0) < 0.01, "{}", amplitude(&w, 200.0));
    // aligned with the input: the 10 Hz sine stays in phase (the sine has no cosine part at row 250: t = 0.25 s)
    let t = 0.4;
    let i = (t * 1000.0) as usize;
    assert!((w[i] - (2.0 * PI * 10.0 * t).sin()).abs() < 0.03, "{} at row {i}", w[i]);
    let ls = filtered(
        "fir-ls",
        &["fir", "--method", "least-squares", "--order", "80", "--pass", "50", "--stop", "100", "--forward-backward"],
    );
    assert!((amplitude(&ls, 10.0) - 1.0).abs() < 0.03, "{}", amplitude(&ls, 10.0));
    assert!(amplitude(&ls, 200.0) < 0.02, "{}", amplitude(&ls, 200.0));
}

#[test]
fn fir_minimum_order_with_a_kaiser_window() {
    let y = filtered(
        "fir-min",
        &[
            "fir-filter",
            "--minimum",
            "--window",
            "kaiser",
            "--pass",
            "40",
            "--stop",
            "100",
            "--ripple",
            "0.1",
            "--attenuation",
            "60",
        ],
    );
    assert!((amplitude(&y, 10.0) - 1.0).abs() < 0.01, "{}", amplitude(&y, 10.0));
    assert!(amplitude(&y, 200.0) < 0.002, "{}", amplitude(&y, 200.0));
}

#[test]
fn unusable_filters_are_refused_with_a_reason() {
    let dir = scratch("filters-bad");
    let data = signal_file(&dir);
    let d = data.to_str().unwrap();
    let (ok, _, err) = sheaf(&["analyze", "iir-filter", d, "--y", "B", "--cutoff", "600"]);
    assert!(!ok && err.contains("Nyquist"), "{err}");
    let (ok, _, err) =
        sheaf(&["analyze", "iir-filter", d, "--y", "B", "--type", "band-pass", "--cutoff", "300", "--cutoff2", "100"]);
    assert!(!ok && err.contains("first cutoff"), "{err}");
    let (ok, _, err) =
        sheaf(&["analyze", "fir-filter", d, "--y", "B", "--type", "high-pass", "--order", "21", "--cutoff", "100"]);
    assert!(!ok && err.contains("even order"), "{err}");
    let (ok, _, err) = sheaf(&["analyze", "fir-filter", d, "--y", "B", "--order", "2000", "--cutoff", "100"]);
    assert!(!ok && err.contains("order"), "{err}");
}
