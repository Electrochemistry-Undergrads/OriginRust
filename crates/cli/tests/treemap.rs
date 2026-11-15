//! `sheaf plot -t treemap` (T50): nested rectangles by category columns, areas by the value column.

use sheaf_core::graph::{PlotKind, TreemapLayout, TreemapOrder};
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-treemap-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Continents, countries and populations in millions (a duplicate and a missing value on purpose), and an id column.
fn csv(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("d.csv");
    let rows = [
        ("Asia", "China", "1412"),
        ("Asia", "India", "1408"),
        ("Asia", "Indonesia", "276"),
        ("Asia", "Pakistan", "231"),
        ("Asia", "Japan", "125"),
        ("Africa", "Nigeria", "218"),
        ("Africa", "Ethiopia", "123"),
        ("Africa", "Egypt", "109"),
        ("Europe", "Russia", "144"),
        ("Europe", "Germany", "83"),
        ("Europe", "France", "68"),
        ("Europe", "France", "1"),
        ("Europe", "Italy", ""),
        ("North America", "USA", "333"),
        ("North America", "Mexico", "128"),
        ("South America", "Brazil", "215"),
        ("South America", "Argentina", "46"),
        ("Oceania", "Australia", "26"),
    ];
    let mut text = String::from("continent,country,population,id\n");
    for (i, (a, b, c)) in rows.iter().enumerate() {
        text.push_str(&format!("{a},{b},{c},{i}\n"));
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_two_level_treemap_draws_rectangles_and_labels_and_saves_its_settings() {
    let dir = scratch("plot");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "treemap",
        "-y",
        "population",
        "--group",
        "continent,country",
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
    // Headers and the big leaves are labelled; the values with them.
    for t in [">Asia<", ">China<", ">India<", ">1412<", ">1408<"] {
        assert!(text.contains(t), "{t}");
    }
    // No axes: no tick label of an axis, no legend text for the series.
    assert!(!text.contains(">population<"));
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 1);
    assert_eq!(layer.plots[0].kind, PlotKind::Treemap);
    assert_eq!(layer.plots[0].treemap.categories.len(), 2);
    let t = &layer.plots[0].treemap;
    assert_eq!(
        (t.layout, t.order, t.depth, t.gap_pt),
        (TreemapLayout::Squarified, TreemapOrder::Categorical, None, 1.5)
    );
}

#[test]
fn text_columns_are_the_default_categories_and_the_last_numeric_column_the_value() {
    let dir = scratch("default");
    let csv = csv(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "treemap",
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let p = &project.graphs().next().unwrap().layers[0].plots[0];
    assert_eq!(p.treemap.categories.len(), 2);
    let name = project.column(p.y).unwrap().short_name.clone();
    assert_eq!(name, "D", "the last numeric Y column (id) is the value");
}

#[test]
fn every_layout_order_depth_and_gap_option_makes_a_treemap() {
    let dir = scratch("options");
    let csv = csv(&dir);
    for layout in ["squarified", "squarified-plus", "split", "pivot-size", "pivot-middle", "pivot-split-size"] {
        let svg = dir.join(format!("{layout}.svg"));
        let (ok, e) = sheaf(&[
            "plot",
            csv.to_str().unwrap(),
            "-t",
            "treemap",
            "-y",
            "population",
            "--treemap-layout",
            layout,
            "--treemap-order",
            "alphabetical",
            "--treemap-gap",
            "3",
            "-o",
            svg.to_str().unwrap(),
        ]);
        assert!(ok, "{layout}: {e}");
        let text = std::fs::read_to_string(&svg).unwrap();
        assert!(text.contains("<rect"), "{layout}");
    }
    // Depth 1 shows only the continents.
    let (one, two) = (dir.join("one.svg"), dir.join("two.svg"));
    for (out, depth) in [(&one, "1"), (&two, "2")] {
        let (ok, e) = sheaf(&[
            "plot",
            csv.to_str().unwrap(),
            "-t",
            "treemap",
            "-y",
            "population",
            "--treemap-depth",
            depth,
            "--width",
            "8",
            "--height",
            "6",
            "-o",
            out.to_str().unwrap(),
        ]);
        assert!(ok, "{e}");
    }
    let (one, two) = (std::fs::read_to_string(one).unwrap(), std::fs::read_to_string(two).unwrap());
    assert!(!one.contains(">China<") && two.contains(">China<"));
    assert!(one.contains(">Asia<"));
    // Order by a column.
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "treemap",
        "-y",
        "population",
        "--treemap-layout",
        "split",
        "--treemap-order",
        "column:id",
        "-o",
        dir.join("col.svg").to_str().unwrap(),
    ]);
    assert!(ok, "{e}");
}

#[test]
fn bad_treemap_requests_are_refused_with_a_message() {
    let dir = scratch("errors");
    let csv = csv(&dir);
    let svg = dir.join("o.svg");
    let run = |extra: &[&str]| {
        let mut args = vec!["plot", csv.to_str().unwrap(), "-o", svg.to_str().unwrap()];
        args.extend_from_slice(extra);
        sheaf(&args)
    };
    let (ok, e) = run(&["-t", "treemap", "--treemap-layout", "spiral"]);
    assert!(!ok && e.contains("not a layout"), "{e}");
    let (ok, e) = run(&["-t", "treemap", "--treemap-order", "random"]);
    assert!(!ok && e.contains("choose categorical"), "{e}");
    let (ok, e) = run(&["-t", "treemap", "--treemap-depth", "0"]);
    assert!(!ok && e.contains("1 or more"), "{e}");
    let (ok, e) = run(&["-t", "treemap", "--treemap-gap", "50"]);
    assert!(!ok && e.contains("between 0 and 20"), "{e}");
    let (ok, e) = run(&["-t", "line", "--treemap-gap", "2"]);
    assert!(!ok && e.contains("need -t treemap"), "{e}");
    let (ok, e) = run(&["-t", "treemap", "--y2", "id"]);
    assert!(!ok && e.contains("right Y axis"), "{e}");
    let (ok, e) = run(&["-t", "treemap", "--group", "nosuch"]);
    assert!(!ok && e.contains("not found"), "{e}");
    let (ok, e) = run(&["-t", "treemap", "--group", "population", "-y", "population"]);
    assert!(!ok && e.contains("cannot also be a category"), "{e}");
}
