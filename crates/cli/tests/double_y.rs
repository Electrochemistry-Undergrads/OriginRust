//! Checks of double-Y figures from `sheaf plot` and `sheaf export`: `-t double-y`, `--y2` and the
//! `--y2-*` and `--axis-colors` options.

use sheaf_core::Project;
use sheaf_core::graph::{AutoRange, Graph, ScaleLink, TickDirection};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-double-y-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A CSV file with an X column and the Y columns Bv, Cq, Dv and Eq (long names; V and C units).
fn data_file(dir: &Path) -> String {
    let mut text = String::from("X,Bv,Cq,Dv,Eq\ns,V,C,V,C\n");
    for i in 0..=10 {
        let x = f64::from(i);
        text.push_str(&format!("{x},{},{},{},{}\n", x * 0.1, (x - 5.0) / 100.0, 1.0 - x * 0.1, x * x / 1000.0));
    }
    let path = dir.join("data.csv");
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

fn sheaf(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn load(path: &Path) -> Project {
    sheaf_io::project_file::load(path).expect("the saved project loads")
}

fn graph(p: &Project) -> &Graph {
    p.graphs().next().expect("one graph")
}

/// The long names of the Y columns plotted in a layer, in plot order.
fn layer_columns(p: &Project, layer: usize) -> Vec<String> {
    graph(p).layers[layer].plots.iter().map(|q| p.column(q.y).unwrap().long_name.clone()).collect()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn the_double_y_template_alternates_the_columns_between_the_axes() {
    let dir = scratch("alternate");
    let data = data_file(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, _, e) = sheaf(&["plot", &data, "-t", "double-y", "-o", s(&svg), "--save", s(&saved)]);
    assert!(ok, "{e}");
    let p = load(&saved);
    assert_eq!(graph(&p).layers.len(), 2);
    assert_eq!(layer_columns(&p, 0), ["Bv", "Dv"]);
    assert_eq!(layer_columns(&p, 1), ["Cq", "Eq"]);
    assert_eq!(graph(&p).link_parent(1), Some(0));

    let (ok, _, stderr) = sheaf(&["plot", &data, "-t", "double-y", "-y", "Bv", "-o", s(&svg)]);
    assert!(!ok);
    assert!(stderr.contains("a double-Y graph needs at least two Y columns"), "{stderr}");
}

#[test]
fn y2_puts_columns_on_a_right_axis_with_its_own_limits_and_title() {
    let dir = scratch("y2");
    let data = data_file(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let args = [
        "plot",
        &data,
        "-y",
        "Bv",
        "--y2",
        "Cq",
        "--y2-title",
        "Charge",
        "--y2-min=-1",
        "--y2-max=1",
        "-o",
        s(&svg),
        "--save",
        s(&saved),
    ];
    let (ok, _, e) = sheaf(&args);
    assert!(ok, "{e}");
    let p = load(&saved);
    let g = graph(&p);
    assert_eq!(g.layers.len(), 2);
    let link = g.layers[1].link.as_ref().expect("linked");
    assert_eq!((link.parent, &link.x, &link.y), (0, &ScaleLink::Straight, &ScaleLink::None));
    let y2 = &g.layers[1].y_axis;
    assert_eq!((y2.min, y2.max, y2.title.as_deref()), (Some(-1.0), Some(1.0), Some("Charge")));
    assert_eq!(layer_columns(&p, 0), ["Bv"]);
    assert_eq!(layer_columns(&p, 1), ["Cq"]);
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.contains(">Charge</text>"));

    // Without -y the left axis takes the other columns (here all of one unit: Bv and Dv).
    let (ok, _, e) = sheaf(&["plot", &data, "--y2", "Cq,Eq", "-o", s(&svg), "--save", s(&saved)]);
    assert!(ok, "{e}");
    let p = load(&saved);
    assert_eq!(layer_columns(&p, 0), ["Bv", "Dv"]);
    assert_eq!(layer_columns(&p, 1), ["Cq", "Eq"]);

    let (ok, _, stderr) = sheaf(&["plot", &data, "-y", "Bv", "--y2", "Bv", "-o", s(&svg)]);
    assert!(!ok);
    assert!(stderr.contains("a column cannot be on both axes"), "{stderr}");
}

#[test]
fn y2_options_need_a_right_axis() {
    let dir = scratch("no-right");
    let data = data_file(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p1.sheaf"));
    let (ok, _, e) = sheaf(&["plot", &data, "-y", "Bv", "-o", s(&svg), "--save", s(&saved)]);
    assert!(ok, "{e}");
    let out = dir.join("e.svg");
    let (ok, _, stderr) = sheaf(&["export", s(&saved), "--graph", "Graph1", "--y2-min=0", "-o", s(&out)]);
    assert!(!ok);
    assert!(stderr.contains("the graph has no right Y axis"), "{stderr}");
    assert!(!out.exists());
}

#[test]
fn a_right_axis_on_the_cv_template_copies_its_style() {
    let dir = scratch("cv");
    let data = data_file(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, _, e) = sheaf(&["plot", &data, "-t", "cv", "-y", "Bv", "--y2", "Cq", "-o", s(&svg), "--save", s(&saved)]);
    assert!(ok, "{e}");
    let p = load(&saved);
    let g = graph(&p);
    let (left, right) = (&g.layers[0], &g.layers[1]);
    assert_eq!(right.y_axis.major_tick, TickDirection::In);
    assert_eq!(right.y_axis.auto_range, AutoRange::SymmetricZero);
    assert_eq!(right.default_line.width_pt, left.default_line.width_pt);
    assert_eq!(right.plots[0].line.width_pt, 1.75);
    assert!(!left.y_axis.show_opposite, "the right edge belongs to the right axis");
    assert!(left.x_axis.show_opposite, "the top line still closes the box");
    assert!(!left.y_axis.color_follows_plot && !right.y_axis.color_follows_plot, "black axes");
}

#[test]
fn colors_and_labels_count_the_curves_in_legend_order() {
    let dir = scratch("legend");
    let data = data_file(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    // Bv left, Cq right, Dv left: the legend lists Bv, Dv, Cq.
    let args = [
        "plot",
        &data,
        "-t",
        "double-y",
        "-y",
        "Bv,Cq,Dv",
        "--labels",
        "L1,L2,L3",
        "--colors",
        "red,green,blue",
        "--axis-colors",
        "fixed",
        "-o",
        s(&svg),
        "--save",
        s(&saved),
    ];
    let (ok, _, e) = sheaf(&args);
    assert!(ok, "{e}");
    let p = load(&saved);
    let g = graph(&p);
    let labels = |layer: usize| -> Vec<String> {
        g.layers[layer].plots.iter().map(|q| q.label.clone().unwrap_or_default()).collect()
    };
    assert_eq!(layer_columns(&p, 0), ["Bv", "Dv"]);
    assert_eq!(labels(0), ["L1", "L2"]);
    assert_eq!(labels(1), ["L3"]);
    let hex: Vec<String> = g.layers.iter().flat_map(|l| l.plots.iter().map(|q| q.line.color.hex_rgb())).collect();
    assert_eq!(hex, ["#FF0000", "#008000", "#0000FF"]);
    assert!(g.layers.iter().all(|l| !l.y_axis.color_follows_plot), "--axis-colors fixed");
    let text = std::fs::read_to_string(&svg).unwrap();
    let at = |t: &str| text.find(&format!(">{t}</text>")).unwrap_or_else(|| panic!("{t} missing"));
    assert!(at("L1") < at("L2") && at("L2") < at("L3"));
    // One legend: each label once.
    assert_eq!(text.matches(">L3</text>").count(), 1);

    // --no-legend hides the legend of both layers.
    let (ok, _, e) = sheaf(&["export", s(&saved), "--no-legend", "-o", s(&svg)]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(!text.contains(">L1</text>") && !text.contains(">L3</text>"));
}
