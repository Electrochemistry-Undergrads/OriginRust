//! Checks of column charts and text tick labels from `sheaf plot` and `sheaf export`: `-t column`
//! and the options `--x-labels`, `--x-label-angle`, `--bar-gap` and `--bar-overlap`.

use sheaf_core::Project;
use sheaf_core::graph::{BarSpacing, Layer, PlotKind, TickLabelSource};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-column-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An ICP-OES-like file: sample names in the first column and three element columns.
fn icp_file(dir: &Path) -> String {
    let text = "Sample,Li,K,Mn\nS1,1.2,0.5,0.2\nS2,2.5,0.9,0.3\nS3,0.8,1.4,0.25\nS4,3.1,0.7,0.4\nS5,1.9,1.1,0.15\n";
    let path = dir.join("icp.csv");
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

fn layer(p: &Project) -> &Layer {
    &p.graphs().next().expect("one graph").layers[0]
}

/// Runs `sheaf plot` on the data with the extra arguments, saves the project and returns it with
/// the SVG text.
fn plot_and_load(dir: &Path, data: &str, extra: &[&str]) -> (Project, String) {
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let mut args = vec!["plot", data, "-o", s(&svg), "--save", s(&saved)];
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

/// The contents of the SVG's text elements (the writer puts each on one line).
fn label_texts(svg: &str) -> Vec<String> {
    svg.lines()
        .filter(|l| l.starts_with("<text ") && l.ends_with("</text>"))
        .filter_map(|l| l.trim_end_matches("</text>").rsplit_once('>').map(|(_, t)| t.to_string()))
        .collect()
}

/// The number of rectangles in the plot area of the first layer.
fn bars_in_plot_area(svg: &str) -> usize {
    let start = svg.find("<g class=\"plots\"").expect("a plot group");
    let end = start + svg[start..].find("</g>").expect("the end of the plot group");
    svg[start..end].matches("<rect ").count()
}

#[test]
fn the_column_template_draws_one_group_of_bars_per_sample_with_the_names_under_it() {
    let dir = scratch("template");
    let data = icp_file(&dir);
    let (p, svg) = plot_and_load(&dir, &data, &["-t", "column"]);
    let l = layer(&p);
    assert_eq!(l.plots.len(), 3);
    assert!(l.plots.iter().all(|q| q.kind == PlotKind::Column));
    let texts = label_texts(&svg);
    for name in ["S1", "S2", "S3", "S4", "S5", "Sample", "Li", "K", "Mn"] {
        assert!(texts.contains(&name.to_string()), "{name} in {texts:?}");
    }
    assert!(!texts.contains(&"Row".to_string()));
    // 15 bars in the plot area; besides them the page background, the clip rectangle and three
    // small bars in the legend.
    assert_eq!(bars_in_plot_area(&svg), 15);
    assert_eq!(svg.matches("<rect ").count(), 1 + 1 + 15 + 3);
    // Nothing of the new settings is written while they keep their defaults.
    assert_eq!(l.bars, BarSpacing::default());
    assert_eq!(l.x_axis.tick_label_source, TickLabelSource::Auto);
    // Other output formats work too.
    let png = dir.join("o.png");
    let (ok, _, e) = sheaf(&["plot", &data, "-t", "column", "-o", s(&png), "--dpi", "50"]);
    assert!(ok && png.exists(), "{e}");
}

#[test]
fn label_angle_bar_gap_and_overlap_are_set_and_checked() {
    let dir = scratch("options");
    let data = icp_file(&dir);
    let (p, svg) = plot_and_load(
        &dir,
        &data,
        &["-t", "column", "--x-label-angle", "45", "--bar-gap", "0", "--bar-overlap=-20", "-y", "Li,Mn"],
    );
    let l = layer(&p);
    assert_eq!(l.x_axis.tick_label_angle_deg, 45.0);
    assert_eq!(l.bars, BarSpacing { gap_pct: 0.0, overlap_pct: -20.0 });
    assert_eq!(l.plots.len(), 2);
    assert_eq!(svg.matches("transform=\"rotate(-45 ").count(), 5, "the five sample names are turned");

    for (args, option) in [
        (&["--x-label-angle", "100"][..], "--x-label-angle must be between -90 and 90"),
        (&["--x-label-angle=-91"], "--x-label-angle must be between -90 and 90"),
        (&["--bar-gap", "120"], "--bar-gap must be between 0 and 100"),
        (&["--bar-overlap", "101"], "--bar-overlap must be between -100 and 100"),
    ] {
        let stderr = plot_error(&dir, &data, args);
        assert!(stderr.contains(option), "{args:?}: {stderr}");
    }
}

#[test]
fn x_labels_choose_numbers_or_a_column() {
    let dir = scratch("labels");
    let data = icp_file(&dir);
    let (p, svg) = plot_and_load(&dir, &data, &["-t", "column", "--x-labels", "numbers"]);
    assert_eq!(layer(&p).x_axis.tick_label_source, TickLabelSource::Numbers);
    let texts = label_texts(&svg);
    assert!(texts.contains(&"3".to_string()) && !texts.contains(&"S3".to_string()), "{texts:?}");

    // A column of the sheet by its long name: here the Mn values label the ticks.
    let (p, svg) = plot_and_load(&dir, &data, &["--x-labels", "Mn"]);
    let mn = p.workbooks().next().unwrap().sheets[0].columns[3].id;
    assert_eq!(layer(&p).x_axis.tick_label_source, TickLabelSource::Column { column: mn });
    assert!(label_texts(&svg).contains(&"0.25".to_string()));
    let (p, _) = plot_and_load(&dir, &data, &["--x-labels", "auto"]);
    assert_eq!(layer(&p).x_axis.tick_label_source, TickLabelSource::Auto);

    let stderr = plot_error(&dir, &data, &["--x-labels", "Nope"]);
    assert!(stderr.contains("no column named 'Nope'") && stderr.contains("columns: A, B, C, D"), "{stderr}");
}

#[test]
fn names_in_a_label_column_of_a_project_can_label_the_ticks() {
    let dir = scratch("project");
    // The names are not the first column, so they are imported as a label column.
    let path = dir.join("named.csv");
    std::fs::write(&path, "Time,Li,Sample\n1,1.2,S1\n2,2.5,S2\n3,0.8,S3\n").unwrap();
    let saved = dir.join("p.sheaf");
    let (ok, _, e) = sheaf(&["import", s(&path), "-o", s(&saved), "--plot"]);
    assert!(ok, "{e}");
    let out = dir.join("e.svg");
    let (ok, _, e) = sheaf(&["export", s(&saved), "--x-labels", "Sample", "-o", s(&out)]);
    assert!(ok, "{e}");
    let texts = label_texts(&std::fs::read_to_string(&out).unwrap());
    for name in ["S1", "S2", "S3"] {
        assert!(texts.contains(&name.to_string()), "{name} in {texts:?}");
    }
    // Without the option the saved graph draws as before, with numbers.
    let (ok, _, e) = sheaf(&["export", s(&saved), "-o", s(&out)]);
    assert!(ok, "{e}");
    assert!(!label_texts(&std::fs::read_to_string(&out).unwrap()).contains(&"S1".to_string()));
}

#[test]
fn an_unknown_template_lists_the_column_template() {
    let dir = scratch("unknown");
    let data = icp_file(&dir);
    let stderr = plot_error(&dir, &data, &["-t", "nope"]);
    for name in ["line", "scatter", "line-symbol", "column", "cv", "double-y", "stacked", "waterfall"] {
        assert!(stderr.contains(name), "{name}: {stderr}");
    }
}
