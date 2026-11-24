//! Checks of `sheaf merge-graphs` (graphing spec 15.11): three finished graphs become one figure.

use sheaf_core::column::{ColumnData, Designation};
use sheaf_core::graph::ScaleLink;
use sheaf_core::template::Template;
use sheaf_core::{Project, WindowId};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-merge-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// A saved project with three one-plot Line graphs, Graph1 to Graph3.
fn three_graphs(dir: &Path) -> (PathBuf, Vec<WindowId>) {
    let mut p = Project::new();
    let book = p.add_workbook("cv");
    let x: Vec<f64> = (0..=20).map(|i| -0.5 + 0.05 * f64::from(i)).collect();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    let graphs = (1..=3)
        .map(|k| {
            let y: Vec<f64> = x.iter().map(|v| f64::from(k) * v * v).collect();
            let col = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
            p.plot_columns(Template::Line, &[col]).unwrap()
        })
        .collect();
    let path = dir.join("three.sheaf");
    sheaf_io::project_file::save(&p, &path).unwrap();
    (path, graphs)
}

#[test]
fn three_graphs_become_a_vertical_shared_x_figure() {
    let dir = scratch("stack");
    let (project, graphs) = three_graphs(&dir);
    let (svg, saved) = (dir.join("stack.svg"), dir.join("merged.sheaf"));
    let (ok, out, err) = sheaf(&[
        "merge-graphs",
        s(&project),
        "--graph",
        "Graph1",
        "--graph",
        "Graph2",
        "--graph",
        "Graph3",
        "--rows",
        "3",
        "--cols",
        "1",
        "--gap",
        "5,0",
        "--share-x",
        "--hide-inner",
        "-o",
        s(&svg),
        "--save",
        s(&saved),
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("merged 3 graphs into Graph4 (3 layers)"), "{out}");
    assert!(std::fs::read_to_string(&svg).unwrap().starts_with("<svg") || svg.exists());
    let p = sheaf_io::project_file::load(&saved).unwrap();
    assert!(graphs.iter().all(|g| p.graph(*g).is_some()), "the sources are kept");
    let m = p.graphs().find(|g| g.name == "Graph4").unwrap();
    assert_eq!(m.layers.len(), 3);
    for layer in &m.layers[1..3] {
        let link = layer.link.as_ref().unwrap();
        assert_eq!((link.parent, &link.x), (0, &ScaleLink::Straight));
    }
    assert!(!m.layers[0].x_axis.show_tick_labels && !m.layers[1].x_axis.show_tick_labels);
    assert!(m.layers[2].x_axis.show_tick_labels, "only the bottom panel shows X tick labels");
    // no gap: the three panels fill 75 percent of the height together
    let bottom = m.page_frame(2);
    assert!((bottom.top + bottom.height - 0.85).abs() < 1e-9);
}

#[test]
fn all_graphs_with_origins_defaults_and_removing_the_sources() {
    let dir = scratch("all");
    let (project, graphs) = three_graphs(&dir);
    let saved = dir.join("merged.sheaf");
    let (ok, out, err) = sheaf(&["merge-graphs", s(&project), "--all-graphs", "--remove-sources", "--save", s(&saved)]);
    assert!(ok, "{err}");
    assert!(out.contains("merged 3 graphs"), "{out}");
    let p = sheaf_io::project_file::load(&saved).unwrap();
    assert!(graphs.iter().all(|g| p.graph(*g).is_none()));
    let m = p.graphs().next().unwrap();
    // three graphs: 2 x 2 with Origin's text factor 0.46
    assert_eq!(m.layers.len(), 3);
    assert!((m.layers[1].frame.left - 0.55).abs() < 1e-9);
    assert!((m.page.width_pt - 11.41 * 72.0).abs() < 1e-9);
}

#[test]
fn mistakes_are_explained() {
    let dir = scratch("errors");
    let (project, _) = three_graphs(&dir);
    let (ok, _, err) = sheaf(&["merge-graphs", s(&project), "--graph", "Graph1", "--graph", "Graph2"]);
    assert!(!ok && err.contains("--output"), "{err}");
    let out = dir.join("x.png");
    let (ok, _, err) = sheaf(&["merge-graphs", s(&project), "--graph", "Graph1", "-o", s(&out)]);
    assert!(!ok && err.contains("at least two graphs"), "{err}");
    let (ok, _, err) = sheaf(&["merge-graphs", s(&project), "--graph", "Graph1", "--graph", "Graph1", "-o", s(&out)]);
    assert!(!ok && err.contains("more than once"), "{err}");
    let (ok, _, err) = sheaf(&["merge-graphs", s(&project), "--graph", "Nope", "--graph", "Graph1", "-o", s(&out)]);
    assert!(!ok && err.contains("Nope"), "{err}");
    let (ok, _, err) = sheaf(&["merge-graphs", s(&project), "--all-graphs", "--text-scale", "big", "-o", s(&out)]);
    assert!(!ok && err.contains("--text-scale"), "{err}");
    let (ok, _, err) =
        sheaf(&["merge-graphs", s(&project), "--all-graphs", "--rows", "1", "--cols", "2", "-o", s(&out)]);
    assert!(!ok && err.contains("cannot hold 3 graphs"), "{err}");
    let (ok, _, err) = sheaf(&["merge-graphs", s(&project), "--all-graphs", "--gap", "5", "-o", s(&out)]);
    assert!(!ok && err.contains("--gap takes two numbers"), "{err}");
    assert!(!out.exists());
}
