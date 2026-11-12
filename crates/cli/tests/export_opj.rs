//! `sheaf export ... -o file.opj` writes the project as an Origin project that `sheaf` reads back.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-opj-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn data_file(dir: &Path) -> PathBuf {
    let mut text = String::from("Time,Signal\ns,mV\n");
    for i in 0..=10 {
        text.push_str(&format!("{i},{}\n", f64::from(i) * 0.5));
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
fn a_data_file_becomes_an_origin_project() {
    let dir = scratch("data");
    let data = data_file(&dir);
    let out = dir.join("out.opj");
    let (ok, stdout, stderr) = sheaf(&["export", data.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("wrote"), "{stdout}");
    let imp = sheaf_io::opj::read_opj(&out).unwrap();
    assert_eq!(imp.version, "9.3");
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    let sheet = &imp.workbooks[0].sheets[0];
    assert_eq!(sheet.columns.len(), 2);
    assert_eq!(sheet.columns[1].long_name, "Signal");
    assert_eq!(sheet.columns[1].units, "mV");
    assert_eq!(sheet.columns[1].numeric().unwrap()[10], 5.0);
}

#[test]
fn a_sheaf_project_with_a_graph_is_written_with_its_graph() {
    let dir = scratch("project");
    let data = data_file(&dir);
    let project = dir.join("lab.sheaf");
    let (ok, _, stderr) = sheaf(&["import", data.to_str().unwrap(), "-o", project.to_str().unwrap(), "--plot"]);
    assert!(ok, "{stderr}");
    let out = dir.join("lab.opj");
    let (ok, _, stderr) = sheaf(&["export", project.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    let imp = sheaf_io::opj::read_opj(&out).unwrap();
    assert_eq!(imp.workbooks.len(), 1);
    assert_eq!(imp.graphs.len(), 1, "the graph is written: {stderr}");
    // the graph of the written file can be exported as a figure, and the file as a project again
    let png = dir.join("fig.png");
    let (ok, _, stderr) = sheaf(&["export", out.to_str().unwrap(), "-o", png.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!(png.metadata().unwrap().len() > 1000);
    let again = dir.join("again.opj");
    let (ok, _, stderr) = sheaf(&["export", out.to_str().unwrap(), "-o", again.to_str().unwrap()]);
    assert!(ok, "{stderr}");
}

#[test]
fn the_input_is_never_overwritten_and_figure_options_are_refused() {
    let dir = scratch("refuse");
    let data = data_file(&dir);
    let out = dir.join("a.opj");
    assert!(sheaf(&["export", data.to_str().unwrap(), "-o", out.to_str().unwrap()]).0);
    let before = std::fs::read(&out).unwrap();
    let (ok, _, stderr) = sheaf(&["export", out.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert!(!ok);
    assert!(stderr.contains("also the input"), "{stderr}");
    assert_eq!(std::fs::read(&out).unwrap(), before);
    let (ok, _, stderr) = sheaf(&["export", data.to_str().unwrap(), "--graph", "Graph1", "-o", out.to_str().unwrap()]);
    assert!(!ok);
    assert!(stderr.contains("--graph"), "{stderr}");
    // a data file cannot be exported as a figure
    let (ok, _, stderr) = sheaf(&["export", data.to_str().unwrap(), "-o", dir.join("f.png").to_str().unwrap()]);
    assert!(!ok);
    assert!(stderr.contains("not a project"), "{stderr}");
}
