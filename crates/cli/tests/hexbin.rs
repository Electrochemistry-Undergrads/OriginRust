//! `sheaf plot -t hexbin` (T45): X against the first Y column binned into hexagons coloured by count.

use sheaf_core::graph::{HexbinBy, PlotKind};
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-hexbin-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn csv(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("d.csv");
    let mut text = String::from("x,y,z\n");
    for i in 0..300 {
        let (x, y) = ((i * 37 % 101) as f64 / 10.0, (i * 53 % 97) as f64 / 7.0);
        text.push_str(&format!("{x},{y},{}\n", x + y));
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_hexbin_plot_draws_hexagons_and_a_colour_scale_and_refuses_a_right_axis_and_panels() {
    let dir = scratch("svg");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "hexbin",
        "--hex-bins",
        "12",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.contains("Count"));
    // Hexagons are closed six-corner paths: one M and five L.
    let hexagons = text
        .split("<path ")
        .filter(|p| {
            let d = p.split("d=\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("");
            d.matches('L').count() == 5 && d.matches('M').count() == 1 && d.ends_with('Z')
        })
        .count();
    assert!(hexagons > 20, "{hexagons}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 1, "one hexbin plot per layer");
    assert_eq!(layer.plots[0].kind, PlotKind::Hexbin);
    assert_eq!(layer.plots[0].hexbin.x.by, HexbinBy::Bins(12));
    let (ok, e) =
        sheaf(&["plot", csv.to_str().unwrap(), "-t", "hexbin", "--y", "y", "--y2", "z", "-o", svg.to_str().unwrap()]);
    assert!(!ok);
    assert!(e.contains("hexbin plot cannot have a right Y axis"), "{e}");
    let (ok, e) =
        sheaf(&["plot", csv.to_str().unwrap(), "-t", "hexbin", "--panels", "stack", "-o", svg.to_str().unwrap()]);
    assert!(!ok);
    assert!(e.contains("hexbin"), "{e}");
    let (ok, e) =
        sheaf(&["plot", csv.to_str().unwrap(), "-t", "hexbin", "--hex-bins", "1", "-o", svg.to_str().unwrap()]);
    assert!(!ok);
    assert!(e.contains("--hex-bins"), "{e}");
}
