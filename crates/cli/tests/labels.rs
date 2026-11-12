//! Checks of `sheaf plot --label "TEXT@X,Y"`: free text labels placed by fractions of the frame.

use sheaf_core::Project;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-label-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn data_file(dir: &Path) -> String {
    let mut text = String::from("X,A\n");
    for i in 0..=10 {
        text.push_str(&format!("{i},{}\n", i * i));
    }
    let path = dir.join("d.csv");
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn label_puts_the_text_in_the_figure_and_the_project() {
    let dir = scratch("ok");
    let data = data_file(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let args = [
        "plot",
        data.as_str(),
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
        "--label",
        "Zn-LMO@0.85,0.1",
        "--label",
        r"\b(After Delithiation)@0.2,0.5",
    ];
    let (ok, err) = sheaf(&args);
    assert!(ok, "{err}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.contains("Zn-LMO"), "the plain label is drawn");
    assert!(text.contains("After Delithiation"), "the escape is resolved and the text drawn");
    assert!(!text.contains(r"\b("), "the escape itself is not drawn");
    let p: Project = sheaf_io::project_file::load(&saved).unwrap();
    let labels = &p.graphs().next().unwrap().layers[0].labels;
    assert_eq!(labels.len(), 2);
    assert_eq!((labels[0].text.as_str(), labels[0].x, labels[0].y), ("Zn-LMO", 0.85, 0.1));
    assert_eq!(labels[1].text, r"\b(After Delithiation)");
}

#[test]
fn a_malformed_label_is_refused_with_a_message() {
    let dir = scratch("bad");
    let data = data_file(&dir);
    let out = dir.join("bad.svg");
    for (value, needle) in
        [("no position", "no @"), ("text@1", "not two numbers"), ("text@a,b", "not two numbers"), ("@0.5,0.5", "empty")]
    {
        let (ok, err) = sheaf(&["plot", data.as_str(), "-o", out.to_str().unwrap(), "--label", value]);
        assert!(!ok, "{value}");
        assert!(err.contains(needle) && err.contains("TEXT@X,Y"), "{value}: {err}");
        assert!(!out.exists());
    }
}
