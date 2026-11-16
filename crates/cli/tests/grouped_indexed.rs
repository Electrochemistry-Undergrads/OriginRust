//! `sheaf plot -t grouped-columns` and `-t grouped-box` (T75): one numeric column split by one or two group columns.

use sheaf_core::graph::{IndexedError, PlotKind};
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-grouped-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Electrolyte, Scan and Current: three replicates per cell.
fn csv(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("d.csv");
    let mut text = String::from("Electrolyte,Scan,Current\n");
    for (i, e) in ["KCl", "NaCl", "LiCl"].iter().enumerate() {
        for scan in [1, 2] {
            for r in 0..3 {
                text.push_str(&format!("{e},{scan},{}\n", 1.0 + i as f64 + scan as f64 * 0.5 + r as f64 * 0.1));
            }
        }
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn grouped_columns_split_by_two_groups_with_error_bars() {
    let dir = scratch("columns");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "grouped-columns",
        "-y",
        "Current",
        "--group",
        "Electrolyte,Scan",
        "--group-error",
        "sd",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&svg).unwrap();
    for t in [">KCl<", ">NaCl<", ">LiCl<", ">Electrolyte<"] {
        assert!(text.contains(t), "{t}");
    }
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 2, "one plot per level of the second group column");
    assert!(layer.plots.iter().all(|p| p.kind == PlotKind::Column));
    let labels: Vec<_> = layer.plots.iter().map(|p| p.label.clone().unwrap()).collect();
    assert_eq!(labels, ["1", "2"]);
    assert!(layer.plots.iter().all(|p| p.indexed.as_ref().unwrap().error == IndexedError::Sd));
}

#[test]
fn grouped_box_with_one_group_and_the_refusals() {
    let dir = scratch("box");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "grouped-box",
        "-y",
        "Current",
        "--group",
        "Electrolyte",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 1);
    assert_eq!(layer.plots[0].kind, PlotKind::Box);
    assert!(layer.plots[0].indexed.is_some());
    let o = svg.to_str().unwrap();
    for (args, want) in [
        (vec!["-t", "grouped-box", "-y", "Current"], "need --group"),
        (vec!["-t", "grouped-box", "-y", "Current", "--group", "Electrolyte,Scan,Current"], "cannot also be a group"),
        (vec!["-t", "grouped-box", "-y", "Current", "--group", "Electrolyte", "--y2", "Scan"], "right Y axis"),
        (vec!["-t", "grouped-box", "-y", "Current", "--group", "Electrolyte", "--panels", "stack"], "panels"),
        (
            vec!["-t", "grouped-box", "-y", "Current", "--group", "Electrolyte", "--group-error", "sd"],
            "needs -t grouped-columns",
        ),
        (
            vec!["-t", "grouped-columns", "-y", "Current", "--group", "Electrolyte", "--group-error", "x"],
            "none, sd or se",
        ),
        (
            vec!["-t", "grouped-columns", "-y", "Current", "--group", "Electrolyte,Scan,Current"],
            "cannot also be a group",
        ),
    ] {
        let mut a = vec!["plot", csv.to_str().unwrap()];
        a.extend(args.iter());
        a.extend(["-o", o]);
        let (ok, e) = sheaf(&a);
        assert!(!ok && e.contains(want), "{args:?}: {e}");
    }
}
