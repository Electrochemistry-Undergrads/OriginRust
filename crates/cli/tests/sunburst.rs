//! `sheaf plot -t sunburst` (T51): rings by category columns, angles by the value column, coloured by value.

use sheaf_core::graph::{PlotKind, SunburstOrder};
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-sunburst-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Continent, country, city and population (an empty country on purpose).
pub fn csv(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("d.csv");
    let rows = [
        ("Asia", "China", "Beijing", "21"),
        ("Asia", "China", "Shanghai", "24"),
        ("Asia", "India", "Delhi", "19"),
        ("Asia", "India", "Mumbai", "12"),
        ("Asia", "Japan", "Tokyo", "14"),
        ("Europe", "France", "Paris", "11"),
        ("Europe", "France", "Lyon", "2"),
        ("Europe", "Germany", "Berlin", "4"),
        ("Europe", "", "Andorra", "1"),
        ("Africa", "Egypt", "Cairo", "10"),
        ("Africa", "Nigeria", "Lagos", "15"),
        ("Oceania", "Australia", "Sydney", "5"),
    ];
    let mut text = String::from("continent,country,city,population\n");
    for (a, b, c, d) in rows {
        text.push_str(&format!("{a},{b},{c},{d}\n"));
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_three_level_sunburst_draws_wedges_labels_and_a_colour_scale_and_saves_its_settings() {
    let dir = scratch("plot");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "sunburst",
        "-y",
        "population",
        "--group",
        "continent,country,city",
        "--sunburst-labels",
        "both",
        "--sunburst-hole",
        "25",
        "--width",
        "8",
        "--height",
        "6",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let text = std::fs::read_to_string(&svg).unwrap();
    for t in [">Asia<", ">China<", ">population<"] {
        assert!(text.contains(t), "{t}");
    }
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 1);
    assert_eq!(layer.plots[0].kind, PlotKind::Sunburst);
    let s = &layer.plots[0].sunburst;
    assert_eq!(s.categories.len(), 3);
    assert_eq!((s.hole, s.show_names, s.show_values, s.order), (0.25, true, true, SunburstOrder::Worksheet));
}

#[test]
fn text_columns_are_the_default_categories_and_options_and_refusals() {
    let dir = scratch("opts");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "sunburst",
        "--sunburst-start",
        "0",
        "--sunburst-gap",
        "2",
        "--sunburst-order",
        "size",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let s = &project.graphs().next().unwrap().layers[0].plots[0].sunburst;
    assert_eq!((s.categories.len(), s.start_angle_deg, s.ring_gap_pt, s.order), (3, 0.0, 2.0, SunburstOrder::BySize));
    let o = svg.to_str().unwrap();
    for (args, want) in [
        (vec!["-t", "sunburst", "--y2", "population"], "right Y axis"),
        (vec!["-t", "sunburst", "--panels", "stack"], "panels"),
        (vec!["-t", "sunburst", "--sunburst-hole", "95"], "between 0 and 90"),
        (vec!["-t", "sunburst", "--sunburst-labels", "x"], "names, values, both or none"),
        (vec!["-t", "scatter", "--sunburst-hole", "20"], "need -t sunburst"),
        (vec!["-t", "sunburst", "--group", "population"], "cannot also be a category"),
    ] {
        let mut a = vec!["plot", csv.to_str().unwrap()];
        a.extend(args.iter());
        a.extend(["-o", o]);
        let (ok, e) = sheaf(&a);
        assert!(!ok && e.contains(want), "{args:?}: {e}");
    }
}
