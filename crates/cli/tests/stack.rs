//! Checks of stacked and waterfall figures from `sheaf plot` and `sheaf export`: `-t stacked`,
//! `-t waterfall` and the options `--stack`, `--offsets`, `--waterfall`, `--no-y-tick-labels`
//! and `--no-y-ticks`.

use sheaf_core::Project;
use sheaf_core::graph::{Graph, Layer, Stack, TickDirection};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-stack-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A CSV file with an X column and the Y columns A, B and C (Origin's first check curves x/2,
/// 1000 + 5x and -5 + x/2 over X = 0..20).
fn data_file(dir: &Path) -> String {
    let mut text = String::from("X,A,B,C\n");
    for i in 0..=20 {
        let x = f64::from(i);
        text.push_str(&format!("{x},{},{},{}\n", x / 2.0, 1000.0 + 5.0 * x, -5.0 + x / 2.0));
    }
    let path = dir.join("three.csv");
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

fn layer(p: &Project) -> &Layer {
    &graph(p).layers[0]
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// Runs `sheaf plot` on the data with the extra arguments, saves the project and loads it.
fn plot_and_load(dir: &Path, data: &str, extra: &[&str]) -> Project {
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let mut args = vec!["plot", data, "-o", s(&svg), "--save", s(&saved)];
    args.extend_from_slice(extra);
    let (ok, _, e) = sheaf(&args);
    assert!(ok, "{extra:?}: {e}");
    load(&saved)
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

#[test]
fn stack_sets_the_mode_of_the_first_layer() {
    let dir = scratch("mode");
    let data = data_file(&dir);
    let p = plot_and_load(&dir, &data, &["--stack", "auto"]);
    assert_eq!(layer(&p).stack, Stack::Auto { gap_percent: 10.0 });
    let p = plot_and_load(&dir, &data, &["--stack", "auto:25"]);
    assert_eq!(layer(&p).stack, Stack::Auto { gap_percent: 25.0 });
    let p = plot_and_load(&dir, &data, &["--stack=step:-500"]);
    assert_eq!(layer(&p).stack, Stack::Constant { step: -500.0 });
    let p = plot_and_load(&dir, &data, &["-t", "stacked", "--stack", "none"]);
    assert_eq!(layer(&p).stack, Stack::None);
    let text = std::fs::read_to_string(dir.join("o.svg")).unwrap();
    assert!(text.starts_with("<svg"));

    for bad in ["auto:abc", "auto:-5", "step:", "sideways"] {
        let stderr = plot_error(&dir, &data, &["--stack", bad]);
        assert!(stderr.contains("--stack"), "{bad}: {stderr}");
    }
}

#[test]
fn offsets_set_individual_offsets_in_plot_order() {
    let dir = scratch("offsets");
    let data = data_file(&dir);
    let p = plot_and_load(&dir, &data, &["--offsets=0,100"]);
    let l = layer(&p);
    assert_eq!(l.stack, Stack::Individual);
    let offset = |k: usize| l.plot_offset(l.plots[k].y).map(|o| (o.x, o.y));
    assert_eq!((offset(0), offset(1), offset(2)), (Some((0.0, 0.0)), Some((0.0, 100.0)), None));
    // Negative values in the = form.
    let p = plot_and_load(&dir, &data, &["--offsets=-1000,0,25"]);
    let l = layer(&p);
    assert_eq!(l.plot_offset(l.plots[0].y).map(|o| o.y), Some(-1000.0));

    let stderr = plot_error(&dir, &data, &["--offsets=0,1,2,3"]);
    assert!(stderr.contains("--offsets") && stderr.contains("3 curve"), "{stderr}");
    let stderr = plot_error(&dir, &data, &["--stack", "auto", "--waterfall", "5,10"]);
    assert!(stderr.contains("cannot be used with"), "{stderr}");
    let stderr = plot_error(&dir, &data, &["--stack", "auto", "--offsets", "1,2"]);
    assert!(stderr.contains("cannot be used with"), "{stderr}");
}

#[test]
fn waterfall_needs_two_percentages_and_a_graph_without_a_right_axis() {
    let dir = scratch("waterfall");
    let data = data_file(&dir);
    let p = plot_and_load(&dir, &data, &["--waterfall", "5,10"]);
    assert_eq!(layer(&p).stack, Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 });
    let p = plot_and_load(&dir, &data, &["--waterfall=-5,10"]);
    assert_eq!(layer(&p).stack, Stack::Waterfall { dx_percent: -5.0, dy_percent: 10.0 });
    let p = plot_and_load(&dir, &data, &["-t", "waterfall"]);
    assert_eq!(layer(&p).stack, Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 });

    for bad in [&["--waterfall", "5"][..], &["--waterfall", "5,200"], &["--waterfall", "5,10", "--y2", "C"]] {
        let stderr = plot_error(&dir, &data, bad);
        assert!(stderr.contains("--waterfall"), "{bad:?}: {stderr}");
    }
    let stderr = plot_error(&dir, &data, &["-t", "waterfall", "--y2", "C"]);
    assert!(stderr.contains("a waterfall cannot have a right Y axis"), "{stderr}");
}

#[test]
fn the_stacked_template_and_the_y_axis_options() {
    let dir = scratch("template");
    let data = data_file(&dir);
    let p = plot_and_load(&dir, &data, &["-t", "stacked"]);
    let l = layer(&p);
    assert_eq!(l.stack, Stack::Auto { gap_percent: 10.0 });
    assert!(!l.y_axis.show_tick_labels && l.y_axis.major_tick == TickDirection::None);
    assert_eq!(l.plots.len(), 3);
    // The curves span 0..140, so a Y axis would be labelled 0, 50, 100, 150; the X axis has
    // 0, 5, ..., 20. No 50 or 100 is drawn.
    let text = std::fs::read_to_string(dir.join("o.svg")).unwrap();
    assert!(text.contains(">20</text>"), "X tick labels are drawn");
    assert!(!text.contains(">50</text>") && !text.contains(">100</text>"), "Y tick labels are hidden");

    let p = plot_and_load(&dir, &data, &["--no-y-tick-labels", "--no-y-ticks"]);
    let y = &layer(&p).y_axis;
    assert!(!y.show_tick_labels);
    assert_eq!((y.major_tick, y.minor_tick), (TickDirection::None, TickDirection::None));
    assert_eq!(layer(&p).stack, Stack::None, "the Y axis options do not stack");
}

#[test]
fn export_applies_the_stack_options_to_a_saved_project() {
    let dir = scratch("export");
    let data = data_file(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, _, e) = sheaf(&["plot", &data, "-o", s(&svg), "--save", s(&saved)]);
    assert!(ok, "{e}");
    let plain = std::fs::read_to_string(&svg).unwrap();
    let out = dir.join("e.svg");
    let args = ["export", s(&saved), "--graph", "Graph1", "--stack", "step:500", "--no-y-tick-labels", "-o", s(&out)];
    let (ok, _, e) = sheaf(&args);
    assert!(ok, "{e}");
    let stacked = std::fs::read_to_string(&out).unwrap();
    assert_ne!(plain, stacked);
    // Without options the saved graph draws as before.
    let (ok, _, e) = sheaf(&["export", s(&saved), "-o", s(&out)]);
    assert!(ok, "{e}");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), plain);
}
