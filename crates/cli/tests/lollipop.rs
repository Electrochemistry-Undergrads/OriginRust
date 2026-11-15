//! `sheaf plot -t lollipop` (T33): two or more Y columns, X and Y exchanged, drop lines to the next plot.

use sheaf_core::graph::DropTarget;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-lollipop-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn the_lollipop_template_draws_drop_lines_between_the_y_columns() {
    let dir = scratch("draw");
    let csv = dir.join("d.csv");
    std::fs::write(&csv, "x,a,b,c\n1,2,3,5\n2,4,1,2\n3,3,4,1\n4,5,2,3\n").unwrap();
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "lollipop",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.contains("<svg"));
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 3);
    assert!(
        layer
            .plots
            .iter()
            .all(|p| p.decor.drop.vertical.on && p.decor.drop.vertical.target == DropTarget::NextDataPlot)
    );
    // The PNG export works as well.
    let png = dir.join("o.png");
    let (ok, e) = sheaf(&["plot", csv.to_str().unwrap(), "-t", "lollipop", "-o", png.to_str().unwrap()]);
    assert!(ok, "{e}");
    assert!(std::fs::metadata(&png).unwrap().len() > 1000);
}

#[test]
fn the_lollipop_template_refuses_a_single_y_column() {
    let dir = scratch("one");
    let csv = dir.join("d.csv");
    std::fs::write(&csv, "x,a,b\n1,2,3\n2,4,1\n").unwrap();
    let svg = dir.join("o.svg");
    let (ok, e) = sheaf(&["plot", csv.to_str().unwrap(), "-t", "lollipop", "--y", "a", "-o", svg.to_str().unwrap()]);
    assert!(!ok);
    assert!(e.contains("two or more Y columns"), "{e}");
}
