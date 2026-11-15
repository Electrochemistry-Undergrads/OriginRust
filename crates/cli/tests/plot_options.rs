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

/// The X pixel of every vertex of the red curve in an SVG.
fn red_curve_xs(svg: &str) -> Vec<f64> {
    let tag = svg.split("<path ").find(|t| t.contains("stroke=\"#FF0000\"")).expect("a red curve");
    let d = tag.split("d=\"").nth(1).unwrap().split('"').next().unwrap();
    d.split(['M', 'L']).filter(|s| !s.is_empty()).map(|s| s.split(' ').next().unwrap().parse().unwrap()).collect()
}

#[test]
fn sort_x_connects_an_unsorted_curve_in_increasing_x_and_leaves_it_alone_without_the_flag() {
    let dir = scratch("sortx");
    let data = dir.join("unsorted.csv");
    std::fs::write(&data, "t,v\ns,A\n3,2\n1,6\n2,4\n5,8\n4,1\n").unwrap();
    let data = data.to_str().unwrap();
    let (off, on) = (dir.join("off.svg"), dir.join("on.svg"));
    let base = ["plot", data, "--template", "line", "--colors", "#ff0000", "--line-width", "3"];
    let (ok, _, e) = sheaf(&[&base[..], &["-o", off.to_str().unwrap()]].concat());
    assert!(ok, "{e}");
    let (ok, _, e) = sheaf(&[&base[..], &["--sort-x", "-o", on.to_str().unwrap()]].concat());
    assert!(ok, "{e}");
    let xs_off = red_curve_xs(&std::fs::read_to_string(&off).unwrap());
    let xs_on = red_curve_xs(&std::fs::read_to_string(&on).unwrap());
    assert_eq!(xs_off.len(), 5);
    assert!(xs_off.windows(2).any(|w| w[0] > w[1]), "worksheet order without the flag: {xs_off:?}");
    assert_eq!(xs_on.len(), 5);
    assert!(xs_on.windows(2).all(|w| w[0] < w[1]), "increasing X with --sort-x: {xs_on:?}");
}

#[test]
fn trendline_adds_curves_and_a_label_and_refuses_bad_kinds() {
    let dir = scratch("trend");
    let data = dir.join("noisy.csv");
    std::fs::write(&data, "t,v\ns,A\n1,2.1\n2,3.9\n3,6.2\n4,7.8\n5,10.1\n6,12.2\n").unwrap();
    let data = data.to_str().unwrap();
    let (plain, with) = (dir.join("plain.svg"), dir.join("with.svg"));
    let base = ["plot", data, "--template", "scatter", "--colors", "#ff0000"];
    let (ok, _, e) = sheaf(&[&base[..], &["-o", plain.to_str().unwrap()]].concat());
    assert!(ok, "{e}");
    let (ok, _, e) = sheaf(&[&base[..], &["--trendline", "linear", "--trendline", "ma:3", "-o", with.to_str().unwrap()]].concat());
    assert!(ok, "{e}");
    let (plain, with) = (std::fs::read_to_string(&plain).unwrap(), std::fs::read_to_string(&with).unwrap());
    // Curves in the plot colour: the two trendline paths are new; the label is written.
    let paths = |s: &str| s.matches("stroke=\"#FF0000\"").count();
    assert_eq!(paths(&with), paths(&plain) + 2);
    assert!(with.contains("y = 2.02") && with.contains("R"), "label text missing");
    assert!(!plain.contains("y = 2.02"));
    let (ok, _, e) = sheaf(&[&base[..], &["--trendline", "poly:11", "-o", dir.join("x.svg").to_str().unwrap()]].concat());
    assert!(!ok && e.contains("--trendline") && e.contains("2 to 10"), "{e}");
    let (ok, _, e) = sheaf(&[&base[..], &["--trendline", "cubic", "-o", dir.join("x.svg").to_str().unwrap()]].concat());
    assert!(!ok && e.contains("cubic"), "{e}");
    // Without the label.
    let bare = dir.join("bare.svg");
    let (ok, _, e) =
        sheaf(&[&base[..], &["--trendline", "linear", "--no-trendline-label", "-o", bare.to_str().unwrap()]].concat());
    assert!(ok, "{e}");
    assert!(!std::fs::read_to_string(&bare).unwrap().contains("y = 2.02"));
}
