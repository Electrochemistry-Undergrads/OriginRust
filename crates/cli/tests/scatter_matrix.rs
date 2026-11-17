//! `sheaf plot -t scatter-matrix` (T114): every pair of the Y columns in an n x n grid of layers.

use sheaf_core::graph::PlotKind;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-scatmat-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn csv(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("data.csv");
    std::fs::write(&path, "x,a,b,c\n1,2,3,5\n2,4,1,4\n3,5,7,2\n4,7,2,8\n5,9,6,1\n").unwrap();
    path
}

fn plot(tag: &str, extra: &[&str]) -> sheaf_core::project::Project {
    let dir = scratch(tag);
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let mut args = vec!["plot", csv.to_str().unwrap(), "-t", "scatter-matrix"];
    args.extend_from_slice(extra);
    args.extend_from_slice(&["-o", svg.to_str().unwrap(), "--save", saved.to_str().unwrap()]);
    let (ok, e) = sheaf(&args);
    assert!(ok, "{e}");
    sheaf_io::project_file::load(&saved).unwrap()
}

#[test]
fn the_default_selection_is_every_y_column() {
    let p = plot("default", &[]);
    // x is the X column, so a, b and c give 3 x 3 layers.
    assert_eq!(p.graphs().last().unwrap().layers.len(), 9);
}

#[test]
fn an_x_column_can_be_named_with_y() {
    let p = plot("withx", &["-y", "x,a,b,c"]);
    assert_eq!(p.graphs().last().unwrap().layers.len(), 16);
}

#[test]
fn a_lower_triangle_with_histograms_on_the_diagonal() {
    let p = plot("lower", &["--matrix-display", "lower", "--matrix-diagonal", "histogram", "--matrix-gap", "2"]);
    let g = p.graphs().last().unwrap();
    assert_eq!(g.layers.len(), 6);
    let histograms = g.layers.iter().filter(|l| l.plots.iter().any(|q| q.kind == PlotKind::Histogram)).count();
    assert_eq!(histograms, 3);
}

#[test]
fn png_and_svg_output_succeed() {
    let dir = scratch("export");
    let csv = csv(&dir);
    for name in ["o.png", "o.svg"] {
        let out = dir.join(name);
        let (ok, e) = sheaf(&[
            "plot",
            csv.to_str().unwrap(),
            "-t",
            "scatter-matrix",
            "--matrix-no-names",
            "-o",
            out.to_str().unwrap(),
        ]);
        assert!(ok, "{e}");
        assert!(std::fs::metadata(&out).unwrap().len() > 0);
    }
}

#[test]
fn bad_selections_and_options_are_refused() {
    let dir = scratch("errors");
    let csv = csv(&dir);
    let c = csv.to_str().unwrap();
    let svg = dir.join("o.svg");
    let o = svg.to_str().unwrap();
    let refused = |extra: &[&str], needle: &str| {
        let mut args = vec!["plot", c];
        args.extend_from_slice(extra);
        args.extend_from_slice(&["-o", o]);
        let (ok, e) = sheaf(&args);
        assert!(!ok && e.contains(needle), "{extra:?}: {e}");
    };
    refused(&["-t", "scatter-matrix", "-y", "a"], "2 to 8");
    refused(&["-t", "scatter-matrix", "--y2", "c"], "cannot have a --y2 axis");
    refused(&["-t", "scatter-matrix", "--matrix-gap", "20"], "the gap is 0 to 10 percent of the page");
    refused(&["-t", "scatter-matrix", "--matrix-display", "x"], "choose square, upper or lower");
    refused(&["-t", "scatter-matrix", "--matrix-diagonal", "x"], "choose none or histogram");
    refused(&["--matrix-diagonal", "histogram"], "need -t scatter-matrix");
}
