//! `sheaf plot -t interval-plot` and `-t pareto` (T83).

use sheaf_core::graph::{IntervalError, PlotKind, StatPlot};
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-stat-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn csv(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("d.csv");
    std::fs::write(&path, "Kind,A,B\nx,1,2\ny,2,4\nx,3,6\nx,4,8\n").unwrap();
    path
}

#[test]
fn an_interval_plot_takes_the_error_option() {
    let dir = scratch("interval");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "interval-plot",
        "-y",
        "B,C",
        "--interval-error",
        "sd",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let p = sheaf_io::project_file::load(&saved).unwrap();
    let plots = &p.graphs().last().unwrap().layers[0].plots;
    assert_eq!(plots.len(), 2);
    assert!(plots.iter().all(|q| q.kind == PlotKind::Scatter));
    assert_eq!(plots[0].stat, Some(StatPlot::Interval { error: IntervalError::Sd, level: 95.0 }));
}

#[test]
fn a_pareto_chart_counts_a_text_column_and_bad_combinations_are_refused() {
    let dir = scratch("pareto");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let c = csv.to_str().unwrap();
    let (ok, e) =
        sheaf(&["plot", c, "-t", "pareto", "-y", "A", "-o", svg.to_str().unwrap(), "--save", saved.to_str().unwrap()]);
    assert!(ok, "{e}");
    let p = sheaf_io::project_file::load(&saved).unwrap();
    assert_eq!(p.graphs().last().unwrap().layers.len(), 2);

    let o = svg.to_str().unwrap();
    let (ok, e) = sheaf(&["plot", c, "-t", "pareto", "-y", "A", "--y2", "B", "-o", o]);
    assert!(!ok && e.contains("--y2"), "{e}");
    let (ok, e) = sheaf(&["plot", c, "-t", "pareto", "-y", "B,C", "-o", o]);
    assert!(!ok && e.contains("exactly one column"), "{e}");
    let (ok, e) = sheaf(&["plot", c, "-t", "scatter", "-y", "B", "--interval-error", "sd", "-o", o]);
    assert!(!ok && e.contains("-t interval-plot"), "{e}");
    let (ok, e) = sheaf(&["plot", c, "-t", "interval-plot", "-y", "B", "--interval-level", "120", "-o", o]);
    assert!(!ok && e.contains("percent"), "{e}");
}
