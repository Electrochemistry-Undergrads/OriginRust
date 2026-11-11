//! Checks of the figure options of `sheaf plot`: bad numbers are refused with a message,
//! reversed axis limits are accepted.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-plot-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn data_file(dir: &Path) -> PathBuf {
    let mut text = String::from("t,ramp\ns,A\n");
    for i in 0..=10 {
        text.push_str(&format!("{i},{i}\n"));
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

#[test]
fn zero_or_negative_sizes_are_refused_with_a_clear_message() {
    let dir = scratch("sizes");
    let data = data_file(&dir);
    let (data, out) = (data.to_str().unwrap(), dir.join("o.png"));
    let out = out.to_str().unwrap();
    for (option, value) in [("--width", "0"), ("--height", "-1"), ("--dpi", "0"), ("--line-width", "NaN")] {
        let joined = format!("{option}={value}");
        let (ok, _, stderr) = sheaf(&["plot", data, &joined, "-o", out]);
        assert!(!ok, "{option} {value} should fail");
        assert!(stderr.contains(option) && stderr.contains("greater than 0"), "{option}: {stderr}");
    }
    assert!(!Path::new(out).exists());
}

#[test]
fn equal_limits_are_refused_and_reversed_limits_are_drawn() {
    let dir = scratch("limits");
    let data = data_file(&dir);
    let data = data.to_str().unwrap();
    let same = dir.join("same.png");
    let (ok, _, stderr) = sheaf(&["plot", data, "--x-min", "3", "--x-max", "3", "-o", same.to_str().unwrap()]);
    assert!(!ok);
    assert!(stderr.contains("--x-min and --x-max"), "{stderr}");

    let forward = dir.join("forward.svg");
    let reversed = dir.join("reversed.svg");
    let (ok1, _, e1) = sheaf(&["plot", data, "--x-min", "1", "--x-max", "5", "-o", forward.to_str().unwrap()]);
    let (ok2, _, e2) = sheaf(&["plot", data, "--x-min", "5", "--x-max", "1", "-o", reversed.to_str().unwrap()]);
    assert!(ok1 && ok2, "{e1} {e2}");
    let (f, r) = (std::fs::read_to_string(&forward).unwrap(), std::fs::read_to_string(&reversed).unwrap());
    assert_ne!(f, r, "reversing the limits must change the picture");
}

#[test]
fn tiny_huge_sizes_and_overflowing_limits_are_refused_with_the_option_name() {
    let dir = scratch("extremes");
    let data = data_file(&dir);
    let (data, out) = (data.to_str().unwrap(), dir.join("o.svg"));
    let out = out.to_str().unwrap();
    for (option, value) in [("--width", "1e-9"), ("--height", "1e9"), ("--width", "0.01")] {
        let joined = format!("{option}={value}");
        let (ok, _, stderr) = sheaf(&["plot", data, &joined, "-o", out]);
        assert!(!ok, "{option} {value} should fail");
        assert!(stderr.contains(option) && stderr.contains("inches"), "{option}: {stderr}");
    }
    let (ok, _, stderr) = sheaf(&["plot", data, "--x-min=1e308", "--x-max=-1e308", "-o", out]);
    assert!(!ok);
    assert!(stderr.contains("too far apart"), "{stderr}");
    assert!(!Path::new(out).exists());
    // A normal size is still accepted.
    let (ok, _, stderr) = sheaf(&["plot", data, "--width", "3", "--height", "2", "-o", out]);
    assert!(ok, "{stderr}");
}

#[test]
fn reversed_limits_put_the_tick_labels_in_decreasing_order_left_to_right() {
    let dir = scratch("order");
    let data = data_file(&dir);
    let svg = dir.join("r.svg");
    let (ok, _, e) = sheaf(&["plot", data.to_str().unwrap(), "--x-min=10", "--x-max=0", "-o", svg.to_str().unwrap()]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&svg).unwrap();
    let pos = |label: &str| -> f64 {
        let needle = format!(">{label}</text>");
        let end = text.find(&needle).unwrap_or_else(|| panic!("label {label} missing"));
        let start = text[..end].rfind("<text").unwrap();
        let tag = &text[start..end];
        let x = tag.split(" x=\"").nth(1).unwrap().split('"').next().unwrap();
        x.parse().unwrap()
    };
    // The X labels are written before the Y labels, so the first match is the X label.
    assert!(pos("10") < pos("6") && pos("6") < pos("0"), "labels are not in decreasing order");
}
