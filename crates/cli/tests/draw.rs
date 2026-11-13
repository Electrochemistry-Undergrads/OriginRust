//! Checks of `sheaf plot --draw "KIND@X1,Y1,X2,Y2"`: drawn objects placed by fractions of the frame.

use sheaf_core::Project;
use sheaf_core::shape::ShapeKind;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-draw-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn draw_adds_objects_to_the_figure_and_the_project() {
    let dir = scratch("ok");
    let data = dir.join("d.csv");
    std::fs::write(&data, "X,A\n0,0\n1,1\n2,4\n3,9\n").unwrap();
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let args = [
        "plot",
        data.to_str().unwrap(),
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
        "--draw",
        "arrow@0.2,0.2,0.5,0.4",
        "--draw",
        "ellipse@0.5,0.5,0.8,0.9",
    ];
    let (ok, err) = sheaf(&args);
    assert!(ok, "{err}");
    let p: Project = sheaf_io::project_file::load(&saved).unwrap();
    let shapes = &p.graphs().next().unwrap().layers[0].shapes;
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].kind, ShapeKind::Arrow);
    assert_eq!((shapes[1].x2, shapes[1].y2), (0.8, 0.9));
}

#[test]
fn draw_rejects_a_bad_value_and_writes_nothing() {
    let dir = scratch("bad");
    let data = dir.join("d.csv");
    std::fs::write(&data, "X,A\n0,0\n1,1\n").unwrap();
    let svg = dir.join("o.svg");
    for bad in ["arrow", "blob@0,0,1,1", "line@0,0,1"] {
        let (ok, err) = sheaf(&["plot", data.to_str().unwrap(), "-o", svg.to_str().unwrap(), "--draw", bad]);
        assert!(!ok && err.contains("--draw"), "{bad}: {err}");
        assert!(!svg.exists());
    }
}
