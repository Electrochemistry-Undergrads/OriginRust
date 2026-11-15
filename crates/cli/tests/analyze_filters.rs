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
    // aligned with the input: the 10 Hz sine stays in phase. At row 425 (t = 0.425 s) the input is
    // sin(8.5 pi) = 1; the 50 samples of delay of an order 100 filter read sin(7.5 pi) = -1 there.
    let t = 0.425;
    let i = (t * 1000.0) as usize;
    assert!((w[i] - 1.0).abs() < 0.03, "{} at row {i}", w[i]);
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

#[test]
fn the_window_parameter_reaches_the_filter() {
    let base = ["fir-filter", "--order", "60", "--cutoff", "50", "--window", "kaiser"];
    let with = |param: &str| {
        let mut a = base.to_vec();
        a.extend_from_slice(&["--window-param", param]);
        filtered(&format!("fir-kaiser-{param}"), &a)
    };
    let (flat, steep) = (with("0"), with("12"));
    // beta 0 is a rectangular window: its stopband leaks far more than beta 12
    assert!(
        amplitude(&flat, 200.0) > 5.0 * amplitude(&steep, 200.0),
        "{} {}",
        amplitude(&flat, 200.0),
        amplitude(&steep, 200.0)
    );
    let rect = filtered("fir-rect", &["fir-filter", "--order", "60", "--cutoff", "50", "--window", "rectangle"]);
    for (a, b) in flat.iter().zip(&rect) {
        assert!((a - b).abs() < 1e-12);
    }
}

#[test]
fn a_window_parameter_that_overflows_is_refused_even_without_scaling() {
    let dir = scratch("filters-overflow");
    let data = signal_file(&dir);
    let d = data.to_str().unwrap();
    for extra in [&[][..], &["--no-scale"][..]] {
        let mut a = vec!["analyze", "fir-filter", d, "--y", "B", "--cutoff", "100", "--window", "kaiser"];
        a.extend_from_slice(&["--window-param", "800"]);
        a.extend_from_slice(extra);
        let (ok, _, err) = sheaf(&a);
        assert!(!ok && err.contains("not finite"), "{extra:?}: {err}");
    }
}

#[test]
fn a_high_order_band_pass_is_not_all_zero() {
    // order 100 at Fs = 2000 Hz: the gain used to underflow to 0 and the column came out all zero
    let dir = scratch("filters-high-order");
    let mut text = String::from("t,y\n");
    for i in 0..400 {
        let t = i as f64 / 2000.0;
        text.push_str(&format!("{t},{}\n", (2.0 * PI * 450.0 * t).sin()));
    }
    let data = dir.join("tone.csv");
    std::fs::write(&data, text).unwrap();
    let out = dir.join("out.csv");
    let (ok, _, err) = sheaf(&[
        "analyze",
        "iir-filter",
        data.to_str().unwrap(),
        "--y",
        "B",
        "--type",
        "band-pass",
        "--order",
        "100",
        "--cutoff",
        "300",
        "--cutoff2",
        "600",
        "--export",
        out.to_str().unwrap(),
    ]);
    assert!(ok, "{err}");
    let y: Vec<f64> = std::fs::read_to_string(&out)
        .unwrap()
        .lines()
        .skip(2)
        .map(|l| l.rsplit(',').next().unwrap().parse().unwrap())
        .collect();
    let peak = y.iter().skip(200).fold(0.0f64, |a, v| a.max(v.abs()));
    assert!((peak - 1.0).abs() < 0.05, "peak {peak}");
}
