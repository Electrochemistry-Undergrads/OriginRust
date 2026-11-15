//! `sheaf plot -t scatter-rug` (T41): a scatter with rugs on the bottom and left axes.

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-rug-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn the_scatter_rug_template_shows_rugs_on_the_bottom_and_left_axes_and_saves_them() {
    let dir = scratch("draw");
    let csv = dir.join("d.csv");
    std::fs::write(&csv, "x,a,b\n1,2,3\n2,4,1\n3,3,4\n4,5,2\n").unwrap();
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "scatter-rug",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    assert!(std::fs::read_to_string(&svg).unwrap().contains("<svg"));
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert!(layer.x_axis.rug.show && layer.y_axis.rug.show);
    assert!(!layer.x_axis.rug_opposite.show && !layer.y_axis.rug_opposite.show);
    let png = dir.join("o.png");
    let (ok, e) = sheaf(&["plot", csv.to_str().unwrap(), "-t", "scatter-rug", "-o", png.to_str().unwrap()]);
    assert!(ok, "{e}");
    assert!(std::fs::metadata(&png).unwrap().len() > 100);
}
