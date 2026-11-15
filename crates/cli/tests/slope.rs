//! Checks of `sheaf plot --slope ROW_A,ROW_B`: slope annotations between two data points.

use sheaf_core::Project;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-slope-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn slope_adds_annotations_to_the_figure_and_the_project() {
    let dir = scratch("ok");
    let data = dir.join("d.csv");
    std::fs::write(&data, "X,A\n0,0\n1,2\n2,4\n3,6\n").unwrap();
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let args = [
        "plot",
        data.to_str().unwrap(),
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
        "--slope",
        "1,4",
        "--slope",
        "2,3",
    ];
    let (ok, err) = sheaf(&args);
    assert!(ok, "{err}");
    let p: Project = sheaf_io::project_file::load(&saved).unwrap();
    let slopes = &p.graphs().next().unwrap().layers[0].slopes;
    assert_eq!(slopes.len(), 2);
    assert_eq!((slopes[0].row_a, slopes[0].row_b), (0, 3), "rows are 1-based on the command line");
    assert_eq!((slopes[1].row_a, slopes[1].row_b), (1, 2));
    assert_eq!(slopes[0].plot, p.graphs().next().unwrap().layers[0].plots[0].y);
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.matches("slope = 2").count() == 2, "{text}");
}

#[test]
fn slope_rejects_a_bad_value_and_writes_nothing() {
    let dir = scratch("bad");
    let data = dir.join("d.csv");
    std::fs::write(&data, "X,A\n0,0\n1,1\n2,2\n").unwrap();
    let svg = dir.join("o.svg");
    for bad in ["1", "0,2", "a,b", "2,2", "1,2,3", "1,9"] {
        let (ok, err) = sheaf(&["plot", data.to_str().unwrap(), "-o", svg.to_str().unwrap(), "--slope", bad]);
        assert!(!ok && err.contains("--slope"), "{bad}: {err}");
        assert!(!svg.exists());
    }
}
