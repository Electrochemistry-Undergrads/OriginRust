//! `sheaf plot -t line --group COLUMN` (T224, Origin's plotgroup): one curve per value of the group column.

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-plot-by-group-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn a_line_graph_has_one_curve_per_group_value() {
    let dir = scratch("line");
    let csv = dir.join("d.csv");
    let mut text = String::from("t,Current,Cell\n");
    for (k, cell) in ["A", "B", "C"].iter().enumerate() {
        for i in 0..5 {
            text.push_str(&format!("{i},{},{cell}\n", f64::from(i) * (k as f64 + 1.0)));
        }
    }
    std::fs::write(&csv, text).unwrap();
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let args = [
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "line",
        "-y",
        "Current",
        "--group",
        "Cell",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ];
    let (ok, e) = sheaf(&args);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 3, "one curve per cell");
    let book = project.workbooks().next().unwrap();
    let grouped = book.sheets.iter().find(|s| s.name.starts_with("Grouped")).expect("the Grouped sheet");
    let names: Vec<&str> = grouped
        .columns
        .iter()
        .filter(|c| c.designation == sheaf_core::Designation::Y)
        .map(|c| c.long_name.as_str())
        .collect();
    assert_eq!(names, ["A", "B", "C"]);
    let text = std::fs::read_to_string(&svg).unwrap();
    for t in [">A<", ">B<", ">C<"] {
        assert!(text.contains(t), "legend {t}");
    }
    let (ok, e) = sheaf(&["plot", csv.to_str().unwrap(), "-t", "line", "--group", "Nope", "-o", svg.to_str().unwrap()]);
    assert!(!ok && e.contains("Nope"), "{e}");
}

#[test]
fn grouped_marginal_box_charts_have_a_box_per_group_in_each_margin() {
    // T373 (Origin 2021b grouped marginal plots).
    let dir = scratch("marginal");
    let csv = dir.join("d.csv");
    let mut text = String::from("x,y,g\n");
    for (k, g) in ["A", "B"].iter().enumerate() {
        for i in 0..8 {
            text.push_str(&format!("{},{},{g}\n", f64::from(i) + k as f64 * 3.0, f64::from(i * i % 5) + k as f64));
        }
    }
    std::fs::write(&csv, text).unwrap();
    let (png, saved) = (dir.join("o.png"), dir.join("p.sheaf"));
    for template in ["marginal-box-charts", "marginal-histograms"] {
        let args = [
            "plot",
            csv.to_str().unwrap(),
            "-t",
            template,
            "-y",
            "y",
            "--group",
            "g",
            "-o",
            png.to_str().unwrap(),
            "--save",
            saved.to_str().unwrap(),
        ];
        let (ok, e) = sheaf(&args);
        assert!(ok, "{template}: {e}");
        let project = sheaf_io::project_file::load(&saved).unwrap();
        let graph = project.graphs().next().unwrap();
        assert_eq!(graph.layers.iter().map(|l| l.plots.len()).collect::<Vec<_>>(), [2, 2, 2], "{template}");
        assert_eq!(graph.layers[0].y_axis.title.as_deref(), Some("y"), "named after the Y column, not a group");
        assert!(std::fs::metadata(&png).unwrap().len() > 1000);
    }
}
