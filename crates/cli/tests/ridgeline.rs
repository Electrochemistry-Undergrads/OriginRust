//! `sheaf plot -t ridgeline` (T44): one density ridge per Y column.

use sheaf_core::graph::PlotKind;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-ridgeline-{tag}-{}", std::process::id()));
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
    let mut text = String::from("x,alpha,beta,gamma\n");
    for i in 0..24 {
        let (a, b, c) =
            (5.0 + (i % 7) as f64 * 0.5, 2.0 + (i * i % 11) as f64 * 0.4, 8.0 + (i % 5) as f64 + (i / 12) as f64 * 6.0);
        text.push_str(&format!("{},{a},{b},{c}\n", i + 1));
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_ridgeline_chart_draws_one_ridge_per_y_column_and_refuses_a_right_axis() {
    let dir = scratch("svg");
    let csv = csv(&dir);
    let (svg, saved, png) = (dir.join("o.svg"), dir.join("p.sheaf"), dir.join("o.png"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "ridgeline",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.contains("alpha") && text.contains("beta") && text.contains("gamma"));
    // A fill (202 points: both baseline corners and 200 curve points) and a curve line per column.
    let curves = text
        .split("<path ")
        .filter(|p| {
            let d = p.split("d=\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("");
            d.matches('L').count() + 1 == 200 && !d.ends_with('Z')
        })
        .count();
    assert_eq!(curves, 3);
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 3);
    assert!(layer.plots.iter().all(|p| p.kind == PlotKind::Ridgeline));
    let (ok, e) = sheaf(&["plot", csv.to_str().unwrap(), "-t", "ridgeline", "-o", png.to_str().unwrap()]);
    assert!(ok, "{e}");
    assert!(std::fs::metadata(&png).unwrap().len() > 1000);
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "ridgeline",
        "--y",
        "alpha",
        "--y2",
        "beta",
        "-o",
        svg.to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(e.contains("ridgeline chart cannot have a right Y axis"), "{e}");
}
