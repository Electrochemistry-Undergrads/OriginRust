//! Checks of multi-panel graphs from `sheaf plot`: `--panels` with `--panel-y`, `--per-panel`,
//! `--panel-gap`, `--panel-margins`, `--panel-order` and `--panel-link` (graphing spec 15.6).

use sheaf_core::Project;
use sheaf_core::graph::{Graph, LayerLink, ScaleLink};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-panels-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A CSV file with the X column E and the Y columns B, C and D (current in mA).
fn three_ys(dir: &Path, name: &str, scale: f64) -> String {
    let mut text = String::from("E,B,C,D\nV,mA,mA,mA\n");
    for i in 0..=20 {
        let x = 0.2 + 0.05 * f64::from(i);
        text.push_str(&format!("{x},{},{},{}\n", scale * (x - 0.7), scale * (0.7 - x) * 2.0, scale * x * x));
    }
    let path = dir.join(name);
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

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn graph(p: &Project) -> &Graph {
    p.graphs().next().expect("one graph")
}

/// The short names of the Y columns plotted in a layer, in plot order.
fn layer_columns(p: &Project, layer: usize) -> Vec<String> {
    graph(p).layers[layer].plots.iter().map(|q| p.column(q.y).unwrap().short_name.clone()).collect()
}

/// Runs `sheaf plot` on the inputs with the extra arguments, saves the project and returns it with
/// the SVG text.
fn plot_and_load(dir: &Path, inputs: &[&str], extra: &[&str]) -> (Project, String) {
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let mut args = vec!["plot"];
    args.extend_from_slice(inputs);
    args.extend_from_slice(&["-o", s(&svg), "--save", s(&saved)]);
    args.extend_from_slice(extra);
    let (ok, _, e) = sheaf(&args);
    assert!(ok, "{extra:?}: {e}");
    (sheaf_io::project_file::load(&saved).expect("the saved project loads"), std::fs::read_to_string(&svg).unwrap())
}

/// Runs `sheaf plot` with the extra arguments and returns its error message; the run must fail.
fn plot_error(dir: &Path, data: &str, extra: &[&str]) -> String {
    let out = dir.join("bad.svg");
    let mut args = vec!["plot", data, "-o", s(&out)];
    args.extend_from_slice(extra);
    let (ok, _, stderr) = sheaf(&args);
    assert!(!ok, "{extra:?} should fail");
    assert!(!out.exists(), "{extra:?} wrote a figure");
    stderr
}

const X_LINK: Option<LayerLink> = Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None });

#[test]
fn a_stack_puts_each_y_column_in_its_own_panel_with_the_first_at_the_bottom() {
    let dir = scratch("stack");
    let data = three_ys(&dir, "data.csv", 1.0);
    let (p, svg) = plot_and_load(&dir, &[&data], &["--panels", "stack"]);
    let g = graph(&p);
    assert_eq!(g.layers.len(), 3);
    for (layer, name) in ["B", "C", "D"].iter().enumerate() {
        assert_eq!(layer_columns(&p, layer), [*name]);
    }
    assert_eq!(g.layers[0].link, None);
    assert_eq!((g.layers[1].link, g.layers[2].link), (X_LINK, X_LINK));
    assert!(g.page_frame(0).top > g.page_frame(1).top, "the first panel is at the bottom");
    for layer in 0..3 {
        assert!(svg.contains(&format!("<g id=\"layer{layer}\">")));
    }
    // Only the bottom panel shows the X title.
    assert_eq!(svg.matches(">E (V)</text>").count(), 1);
    assert!(svg.contains("width=\"345.6pt\""), "the line template's page turned upright");

    let (p, svg) = plot_and_load(&dir, &[&data], &["--panels", "stack", "-t", "cv", "-y", "B,C"]);
    assert_eq!(graph(&p).layers.len(), 2);
    assert!(svg.contains("width=\"259.2pt\" height=\"360pt\""), "{}", &svg[..200]);
}

#[test]
fn several_files_give_one_panel_each_named_after_the_file() {
    let dir = scratch("files");
    let a = three_ys(&dir, "before.csv", 1.0);
    let b = three_ys(&dir, "after.csv", 3.0);
    let (p, _) = plot_and_load(&dir, &[&a, &b], &["--panels", "stack", "-y", "B"]);
    let g = graph(&p);
    assert_eq!(g.layers.len(), 2);
    let label = |layer: usize| g.layers[layer].plots[0].label.clone();
    assert_eq!((label(0), label(1)), (Some("before".to_string()), Some("after".to_string())));
    // --panel-y takes the named columns of every file: B of both files, then D of both.
    let (p, _) = plot_and_load(&dir, &[&a, &b], &["--panels", "2x1", "--panel-y", "B", "--panel-y", "D"]);
    let g = graph(&p);
    assert_eq!(g.layers.len(), 2);
    let labels: Vec<Option<String>> = g.layers[1].plots.iter().map(|q| q.label.clone()).collect();
    assert_eq!(labels, [Some("before".to_string()), Some("after".to_string())]);
    assert_eq!(layer_columns(&p, 1), ["D", "D"]);
}

#[test]
fn panel_y_and_per_panel_choose_the_curves_of_each_panel() {
    let dir = scratch("choose");
    let data = three_ys(&dir, "data.csv", 1.0);
    let (p, _) = plot_and_load(&dir, &[&data], &["--panels", "stack", "--panel-y", "B,C", "--panel-y", "D"]);
    assert_eq!(
        (layer_columns(&p, 0), layer_columns(&p, 1)),
        (vec!["B".to_string(), "C".to_string()], vec!["D".to_string()])
    );
    let (p, _) = plot_and_load(&dir, &[&data], &["--panels", "stack", "--per-panel", "1,2"]);
    assert_eq!(
        (layer_columns(&p, 0), layer_columns(&p, 1)),
        (vec!["B".to_string()], vec!["C".to_string(), "D".to_string()])
    );
    // Colors restart in every panel, as Origin's layers do.
    let g = graph(&p);
    assert_eq!(g.layers[0].plots[0].line.color, g.layers[1].plots[0].line.color);
}

#[test]
fn grids_and_the_layout_options() {
    let dir = scratch("grid");
    let data = three_ys(&dir, "data.csv", 1.0);
    let (p, svg) = plot_and_load(&dir, &[&data], &["--panels", "2x2"]);
    let g = graph(&p);
    assert_eq!(g.layers.len(), 3, "three curves fill three of the four cells");
    assert!(g.layers.iter().all(|l| l.link.is_none()));
    assert!(svg.contains("width=\"921.6pt\" height=\"691.2pt\""), "{}", &svg[..200]);
    // Top left, top right, bottom left.
    let f: Vec<_> = (0..3).map(|i| g.page_frame(i)).collect();
    assert!(f[0].top == f[1].top && f[1].left > f[0].left && f[2].top > f[0].top && f[2].left == f[0].left);
    // Gaps, margins, order and links.
    let extra = [
        "--panels",
        "1x3",
        "--panel-gap",
        "0",
        "--panel-margins",
        "10,5,10,20",
        "--panel-link",
        "y",
        "--panel-order",
        "top-down",
    ];
    let (p, _) = plot_and_load(&dir, &[&data], &extra);
    let g = graph(&p);
    let link = Some(LayerLink { parent: 0, x: ScaleLink::None, y: ScaleLink::Straight });
    assert_eq!((g.layers[1].link, g.layers[2].link), (link, link));
    let f: Vec<_> = (0..3).map(|i| g.page_frame(i)).collect();
    assert!((f[0].left - 0.10).abs() < 1e-9 && (f[0].width - 0.85 / 3.0).abs() < 1e-9, "{f:?}");
    assert!((f[1].left - (f[0].left + f[0].width)).abs() < 1e-9, "no gap: {f:?}");
    assert!((f[0].top - 0.10).abs() < 1e-9 && (f[0].height - 0.70).abs() < 1e-9, "{f:?}");
    assert!(!g.layers[1].y_axis.show_tick_labels && g.layers[0].x_axis.hide_end_label);
    // A stack without links: nothing is shared or hidden.
    let (p, _) =
        plot_and_load(&dir, &[&data], &["--panels", "stack", "--panel-link", "none", "--panel-order", "top-down"]);
    let g = graph(&p);
    assert!(g.layers.iter().all(|l| l.link.is_none() && l.x_axis.show_tick_labels));
    assert!(g.layers[0].frame.top < g.layers[1].frame.top, "the first panel is at the top");
}

#[test]
fn style_options_act_on_every_panel() {
    let dir = scratch("style");
    let data = three_ys(&dir, "data.csv", 1.0);
    let extra =
        ["--panels", "stack", "-y", "B,C", "--colors", "red,blue", "--y-min=-1", "--y-max", "1", "--y-title", "j"];
    let (p, svg) = plot_and_load(&dir, &[&data], &extra);
    let g = graph(&p);
    let colors: Vec<String> = g.layers.iter().map(|l| l.plots[0].line.color.hex_rgb()).collect();
    assert_eq!(colors, ["#FF0000", "#0000FF"]);
    assert!(g.layers.iter().all(|l| l.y_axis.min == Some(-1.0) && l.y_axis.max == Some(1.0)));
    // The same title on panels that touch is drawn once, centred on the stack.
    assert_eq!(svg.matches(">j</text>").count(), 1, "{svg}");
    // On a grid every panel gets it and draws it.
    let (_, svg) = plot_and_load(&dir, &[&data], &["--panels", "1x3", "--y-title", "j"]);
    assert_eq!(svg.matches(">j</text>").count(), 3);
}

#[test]
fn bad_panel_options_are_refused_with_clear_messages() {
    let dir = scratch("errors");
    let data = three_ys(&dir, "data.csv", 1.0);
    let e = plot_error(&dir, &data, &["--panels", "abc"]);
    assert!(e.contains("--panels must be 'stack' or ROWSxCOLUMNS such as 2x2 (got 'abc')"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "0x2"]);
    assert!(e.contains("rows x columns must be from 1 to 64 panels (got 0 x 2)"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "9x9"]);
    assert!(e.contains("(got 9 x 9)"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--per-panel", "2,2"]);
    assert!(e.contains("--per-panel counts add up to 4 but there are 3 curves"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--per-panel", "3,0"]);
    assert!(e.contains("--per-panel counts must be 1 or more"), "{e}");
    let e = plot_error(&dir, &data, &["--panel-gap", "5"]);
    assert!(e.contains("--panels"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--graph", "Graph1"]);
    assert!(e.contains("cannot be used with"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--y2", "D"]);
    assert!(e.contains("cannot be used with"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "-t", "double-y"]);
    assert!(e.contains("the double-y template cannot be used with --panels"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--panel-gap", "60"]);
    assert!(e.contains("--panel-gap must be from 0 to 50 percent of the page (got 60)"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--panel-gap=-1"]);
    assert!(e.contains("(got -1)"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--panel-margins", "10,10"]);
    assert!(e.contains("--panel-margins takes four numbers, L,R,T,B (got 2 numbers)"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--panel-margins", "50,50,10,10"]);
    assert!(e.contains("the margins and gaps leave no room for the panels"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--offsets", "0,1"]);
    assert!(e.contains("--offsets and --waterfall work on a graph of one panel; this graph has 3 panels"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--panel-y", "B", "-y", "C"]);
    assert!(e.contains("cannot be used with"), "{e}");
    let e = plot_error(&dir, &data, &["--panels", "stack", "--panel-y", "B,Nope"]);
    assert!(e.contains("column 'Nope' was not found"), "{e}");
}
