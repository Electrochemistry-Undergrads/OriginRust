//! Saving from the command line defaults to .opj and keeps a companion for what the .opj cannot hold.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-save-{tag}-{}", std::process::id()));
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
fn import_without_extension_writes_an_opj_that_holds_the_graph() {
    let dir = scratch("import");
    let data = data_file(&dir);
    let out = dir.join("lab");
    let (ok, stdout, stderr) = sheaf(&["import", data.to_str().unwrap(), "--plot", "-o", out.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("lab.opj"), "{stdout}");
    assert!(dir.join("lab.opj").exists());
    assert!(!dir.join("lab").exists());
    assert!(dir.join("lab.opj.sheaf").exists(), "the companion keeps the automatic legend anchor");
    let opened = sheaf_io::open_project(&dir.join("lab.opj")).unwrap();
    assert!(opened.companion_used);
    assert_eq!(
        opened.project.graphs().next().unwrap().layers[0].legend.position,
        sheaf_core::graph::LegendPosition::Automatic
    );
    let bytes = std::fs::read(dir.join("lab.opj")).unwrap();
    let raw = sheaf_io::opj::read_opj_from(std::io::Cursor::new(bytes), &dir.join("lab.opj")).unwrap();
    assert_eq!(raw.graphs.len(), 1, "the graph is also written into the .opj itself");
}

/// A .sheaf project of the data file whose second column has a long name with a character outside
/// Windows-1252, which the .opj writer cannot store as it is.
fn project_with_formula(dir: &Path) -> PathBuf {
    let data = data_file(dir);
    let sheaf_path = dir.join("formula.sheaf");
    assert!(sheaf(&["import", data.to_str().unwrap(), "-o", sheaf_path.to_str().unwrap()]).0);
    let mut project = sheaf_io::project_file::load(&sheaf_path).unwrap();
    if let Some(sheaf_core::Window::Workbook(b)) = project.windows.first_mut() {
        b.sheets[0].columns[1].long_name = "Current \u{3a9}".into();
    }
    sheaf_io::project_file::save(&project, &sheaf_path).unwrap();
    sheaf_path
}

#[test]
fn an_opj_that_cannot_hold_everything_gets_a_companion_that_opening_uses() {
    let dir = scratch("companion");
    let source = project_with_formula(&dir);
    let out = dir.join("lab.opj");
    let (ok, _, stderr) = sheaf(&["export", source.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!(out.exists());
    let companion = dir.join("lab.opj.sheaf");
    assert!(sheaf_io::project_file::is_companion(&companion));
    assert!(stderr.contains("1252"), "{stderr}");
    let opened = sheaf_io::open_project(&out).unwrap();
    assert!(opened.companion_used);
    let book = opened.project.workbooks().next().unwrap();
    assert_eq!(book.sheets[0].columns[1].long_name, "Current \u{3a9}");
}

#[test]
fn a_save_that_loses_nothing_removes_an_old_companion() {
    let dir = scratch("remove");
    let source = project_with_formula(&dir);
    let out = dir.join("lab.opj");
    let first = sheaf(&["export", source.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert!(first.0, "{}", first.2);
    assert!(dir.join("lab.opj.sheaf").exists());
    let data = dir.join("data.csv");
    let (ok, stdout, stderr) = sheaf(&["import", data.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!(!dir.join("lab.opj.sheaf").exists());
    assert!(stdout.contains("removed"), "{stdout}");
}

#[test]
fn an_explicit_sheaf_path_still_writes_a_sheaf_project_alone() {
    let dir = scratch("sheaf");
    let data = data_file(&dir);
    let out = dir.join("lab.sheaf");
    let (ok, _, stderr) = sheaf(&["import", data.to_str().unwrap(), "--plot", "-o", out.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert_eq!(sheaf_io::project_file::load(&out).unwrap().graphs().count(), 1);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2, "the data file and the project only");
}

#[test]
fn a_project_the_opj_writer_refuses_gets_the_companion_only_and_an_error() {
    let dir = scratch("refuse");
    let data = data_file(&dir);
    let sheaf_path = dir.join("long.sheaf");
    assert!(sheaf(&["import", data.to_str().unwrap(), "-o", sheaf_path.to_str().unwrap()]).0);
    let mut project = sheaf_io::project_file::load(&sheaf_path).unwrap();
    if let Some(sheaf_core::Window::Workbook(b)) = project.windows.first_mut() {
        b.sheets[0].name = "a sheet name that is far too long for Origin to store".into();
    }
    sheaf_io::project_file::save(&project, &sheaf_path).unwrap();
    let out = dir.join("long.opj");
    let (ok, stdout, stderr) = sheaf(&["export", sheaf_path.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert!(!ok, "a refused .opj is an error");
    assert!(stderr.contains("was not written") && stderr.contains("too long"), "{stderr}");
    assert!(stdout.contains("long.opj.sheaf"), "{stdout}");
    assert!(!out.exists());
    assert!(sheaf_io::project_file::is_companion(&dir.join("long.opj.sheaf")));
}
