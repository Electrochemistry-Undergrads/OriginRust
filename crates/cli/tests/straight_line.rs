//! Checks of `sheaf plot --vline VALUE` and `--hline VALUE`: straight lines at an axis value.

use sheaf_core::Project;
use sheaf_core::straight_line::StraightLineLabel;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-sline-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn vline_and_hline_add_lines_to_the_figure_and_the_project() {
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
        "--vline",
        "1.5",
        "--vline",
        "2,bottom",
        "--hline",
        "3",
    ];
    let (ok, err) = sheaf(&args);
    assert!(ok, "{err}");
    let p: Project = sheaf_io::project_file::load(&saved).unwrap();
    let lines = &p.graphs().next().unwrap().layers[0].straight_lines;
    assert_eq!(lines.len(), 3);
    assert!(lines[0].vertical && lines[0].value == 1.5 && lines[0].location == StraightLineLabel::Top);
    assert_eq!(lines[0].name, "vline");
    assert!(lines[1].vertical && lines[1].value == 2.0 && lines[1].location == StraightLineLabel::Bottom);
    assert_eq!(lines[1].name, "vline1");
    assert!(!lines[2].vertical && lines[2].value == 3.0);
    assert_eq!(lines[2].name, "hline");
    assert!(lines.iter().all(|l| l.span.is_none()), "the command line leaves the ends on the frame");
    let text = std::fs::read_to_string(&svg).unwrap();
    for label in [">1.5<", ">2<", ">3<"] {
        assert!(text.contains(label), "{label} is not in the figure: {text}");
    }
}

#[test]
fn vline_rejects_a_bad_value_and_writes_nothing() {
    let dir = scratch("bad");
    let data = dir.join("d.csv");
    std::fs::write(&data, "X,A\n0,0\n1,1\n2,2\n").unwrap();
    let svg = dir.join("o.svg");
    for (option, bad) in [("--vline", "abc"), ("--vline", "1,middle"), ("--hline", ""), ("--hline", "inf")] {
        let (ok, err) = sheaf(&["plot", data.to_str().unwrap(), "-o", svg.to_str().unwrap(), option, bad]);
        assert!(!ok && err.contains(option), "{option} {bad}: {err}");
        assert!(!svg.exists());
    }
}
