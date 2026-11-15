//! `sheaf plot -t parallel` and `-t parallel-index` (T49): one vertical axis per Y column, a line per row.

use sheaf_core::graph::PlotKind;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-parallel-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Three groups of ten rows, four numeric columns.
fn csv(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("d.csv");
    let mut text = String::from("species,p1,p2,p3,p4,code\n");
    for (g, name) in ["setosa", "versicolor", "virginica"].iter().enumerate() {
        for i in 0..10 {
            let k = (i * 7 % 10) as f64;
            text.push_str(&format!(
                "{name},{},{},{},{},{g}\n",
                1.0 + g as f64 + k / 10.0,
                5.0 - g as f64,
                k,
                g * 10 + i
            ));
        }
    }
    std::fs::write(&path, text).unwrap();
    path
}

/// The row lines of an SVG: paths with a stroke and no fill that run through several points.
fn line_paths(svg: &str) -> usize {
    svg.split("<path ")
        .filter(|p| {
            let d = p.split("d=\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("");
            d.starts_with('M')
                && d.matches('L').count() == 3
                && p.contains("fill=\"none\"")
                && p.split('>').next().is_some_and(|tag| tag.contains("stroke-linecap=\"round\""))
        })
        .count()
}

#[test]
fn a_parallel_plot_draws_a_line_per_row_with_a_colour_scale() {
    let dir = scratch("plot");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "parallel",
        "-y",
        "p1,p2,p3,p4",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert_eq!(line_paths(&text), 30);
    // The axes' titles and the colour scale's, which is the last column's.
    for t in ["p1", "p2", "p3", "p4"] {
        assert!(text.contains(&format!(">{t}<")), "{t}");
    }
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 4);
    assert!(layer.plots.iter().all(|p| p.kind == PlotKind::Parallel));
    assert_eq!(layer.plots[0].parallel.color, layer.plots[3].y.into());
    // --group none: every line in the plot colour, no colour scale; a numeric column named instead.
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "parallel",
        "-y",
        "p1,p2,p3,p4",
        "--group",
        "none",
        "-o",
        svg.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let plain = std::fs::read_to_string(&svg).unwrap();
    assert_eq!(line_paths(&plain), 30);
    assert!(plain.matches("<rect").count() < text.matches("<rect").count());
    let (ok, e) =
        sheaf(&["plot", csv.to_str().unwrap(), "-t", "parallel", "--group", "code", "-o", svg.to_str().unwrap()]);
    assert!(ok, "{e}");
}

#[test]
fn a_parallel_index_colours_by_group_and_refuses_what_it_cannot_draw() {
    let dir = scratch("index");
    let csv = csv(&dir);
    let svg = dir.join("o.svg");
    // The first text column groups the rows by default: three groups, three legend entries.
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "parallel-index",
        "-y",
        "p1,p2,p3,p4",
        "-o",
        svg.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert_eq!(line_paths(&text), 30);
    for g in ["setosa", "versicolor", "virginica"] {
        assert_eq!(text.matches(&format!(">{g}<")).count(), 1, "{g}");
    }
    // A numeric column can group the rows when it is named.
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "parallel-index",
        "-y",
        "p1,p2,p3",
        "--group",
        "code",
        "-o",
        svg.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    // Refusals.
    let (ok, e) =
        sheaf(&["plot", csv.to_str().unwrap(), "-t", "parallel-index", "-y", "p1,p2", "-o", svg.to_str().unwrap()]);
    assert!(!ok);
    assert!(e.contains("three or more numeric columns"), "{e}");
    let (ok, e) = sheaf(&["plot", csv.to_str().unwrap(), "-t", "parallel", "-y", "p1", "-o", svg.to_str().unwrap()]);
    assert!(!ok);
    assert!(e.contains("two or more numeric columns"), "{e}");
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "parallel",
        "--y",
        "p1,p2",
        "--y2",
        "c",
        "-o",
        svg.to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(e.contains("parallel plot cannot have a right Y axis"), "{e}");
    let (ok, e) =
        sheaf(&["plot", csv.to_str().unwrap(), "-t", "line", "--group", "species", "-o", svg.to_str().unwrap()]);
    assert!(!ok);
    assert!(e.contains("--group needs"), "{e}");
}

#[test]
fn a_parallel_plot_refuses_a_text_colour_column_and_points_to_parallel_index() {
    let dir = scratch("textcolor");
    let csv = csv(&dir);
    let svg = dir.join("o.svg");
    let (ok, e) =
        sheaf(&["plot", csv.to_str().unwrap(), "-t", "parallel", "--group", "species", "-o", svg.to_str().unwrap()]);
    assert!(!ok, "a text colour column is refused");
    assert!(e.contains("numeric") && e.contains("parallel-index"), "{e}");
}
