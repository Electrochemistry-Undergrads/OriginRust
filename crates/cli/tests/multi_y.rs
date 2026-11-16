//! `sheaf plot -t 3ys-y-yy` and the other three/four Y axes templates (T61, graphing spec 21.5).

use sheaf_core::Project;
use sheaf_core::graph::{AxisSide, Graph};
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-multi-y-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An X column and five Y columns Ya to Ye.
fn data_file(dir: &Path) -> String {
    let mut text = String::from("X,Ya,Yb,Yc,Yd,Ye\ns,V,C,V,C,V\n");
    for i in 0..=10 {
        let x = f64::from(i);
        text.push_str(&format!("{x},{},{},{},{},{}\n", x, x * x, 100.0 - x, x * 3.0, x / 7.0));
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

fn graph(p: &Project) -> &Graph {
    p.graphs().next().expect("one graph")
}

fn names(p: &Project, layer: usize) -> Vec<String> {
    graph(p).layers[layer].plots.iter().map(|q| p.column(q.y).unwrap().long_name.clone()).collect()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn each_template_makes_its_layers_and_deals_the_columns_out_in_turn() {
    for (name, layers, offsets) in [
        ("3ys-y-yy", 3, vec![0.0, 0.0, 10.0]),
        ("3ys-y-y-y", 3, vec![0.0, -50.0, 0.0]),
        ("4ys-y-yyy", 4, vec![0.0, 0.0, 12.0, 23.0]),
        ("4ys-yy-yy", 4, vec![0.0, 11.0, 0.0, 11.0]),
    ] {
        let dir = scratch(name);
        let data = data_file(&dir);
        let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
        let (ok, _, e) = sheaf(&["plot", &data, "-t", name, "-o", s(&svg), "--save", s(&saved)]);
        assert!(ok, "{name}: {e}");
        let p = sheaf_io::project_file::load(&saved).expect("the saved project loads");
        let g = graph(&p);
        assert_eq!(g.layers.len(), layers, "{name}");
        // Five columns round and round: layer 0 gets Ya and Yd (3 layers) or Ya and Ye (4 layers).
        assert_eq!(names(&p, 0)[0], "Ya");
        assert_eq!(names(&p, 1)[0], "Yb");
        let got: Vec<f64> = g.layers.iter().map(|l| l.y_axis.offset_pct).collect();
        assert_eq!(got, offsets, "{name}: axis offsets survive the .sheaf file");
        assert!(g.layers[1..].iter().all(|l| l.link.is_some()));
        assert_eq!(g.layers[0].y_axis.side, AxisSide::Normal);
        let svg_text = std::fs::read_to_string(&svg).unwrap();
        assert!(svg_text.starts_with("<svg") || svg_text.contains("<svg"), "{name}");
    }
}

#[test]
fn a_right_axis_choice_and_no_column_are_refused() {
    let dir = scratch("refuse");
    let data = data_file(&dir);
    let svg = dir.join("o.svg");
    let (ok, _, e) = sheaf(&["plot", &data, "-t", "3ys-y-yy", "-y", "Ya", "--y2", "Yb", "-o", s(&svg)]);
    assert!(!ok);
    assert!(e.contains("3ys-y-yy"), "{e}");
    let (ok, _, e) = sheaf(&["plot", &data, "-t", "4ys-yy-yy", "--panels", "stack", "-o", s(&svg)]);
    assert!(!ok);
    assert!(e.contains("cannot be used with --panels"), "{e}");
}

#[test]
fn fewer_columns_than_layers_still_make_every_layer() {
    let dir = scratch("few");
    let data = data_file(&dir);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, _, e) = sheaf(&["plot", &data, "-t", "4ys-y-yyy", "-y", "Ya", "-o", s(&svg), "--save", s(&saved)]);
    assert!(ok, "{e}");
    let p = sheaf_io::project_file::load(&saved).unwrap();
    assert_eq!(graph(&p).layers.len(), 4);
    assert_eq!(names(&p, 0), ["Ya"]);
    assert!(graph(&p).layers[3].plots.is_empty());
}
