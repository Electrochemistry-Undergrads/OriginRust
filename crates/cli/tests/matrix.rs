//! Checks of `sheaf matrix`: new, set, transform, convert, to-sheet, stats and plot.

use sheaf_core::Project;
use sheaf_core::graph::FieldSource;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-matrix-{tag}-{}", std::process::id()));
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

fn ok(args: &[&str]) -> String {
    let (good, out, err) = sheaf(args);
    assert!(good, "{args:?}: {err}");
    out
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn load(path: &Path) -> Project {
    sheaf_io::project_file::load(path).expect("the saved project loads")
}

/// The first sheet of the first matrix of a saved project.
fn sheet0(path: &Path) -> sheaf_core::MatrixSheet {
    load(path).matrix_books().next().expect("a matrix").sheets[0].clone()
}

fn new_matrix(dir: &Path) -> PathBuf {
    let path = dir.join("m.sheaf");
    ok(&[
        "matrix",
        "new",
        "-o",
        s(&path),
        "--cols",
        "4",
        "--rows",
        "3",
        "--x-range=10:40",
        "--y-range=1:3",
        "--formula",
        "i*10+j",
        "--long-name",
        "demo",
    ]);
    path
}

#[test]
fn new_makes_a_matrix_from_a_formula() {
    let dir = scratch("new");
    let p = load(&new_matrix(&dir));
    let m = p.matrix_books().next().unwrap();
    assert_eq!(m.long_name, "demo");
    let sheet = &m.sheets[0];
    assert_eq!((sheet.rows, sheet.cols), (3, 4));
    assert_eq!(sheet.value(2, 3), 34.0);
    assert_eq!((sheet.x_at(1), sheet.y_at(2)), (20.0, 3.0));
}

#[test]
fn set_changes_values_and_size_and_a_bad_formula_is_an_error() {
    let dir = scratch("set");
    let path = new_matrix(&dir);
    ok(&[
        "matrix",
        "set",
        s(&path),
        "--formula",
        "x+y",
        "--cols",
        "2",
        "--rows",
        "2",
        "--x-range=0:1",
        "--y-range=0:2",
    ]);
    let sheet = &sheet0(&path);
    assert_eq!((sheet.rows, sheet.cols), (2, 2));
    assert_eq!(sheet.value(1, 1), 3.0);
    let before = std::fs::read(&path).unwrap();
    let (good, _, err) = sheaf(&["matrix", "set", s(&path), "--formula", "i +"]);
    assert!(!good && err.contains("error"), "{err}");
    assert_eq!(std::fs::read(&path).unwrap(), before, "the project is untouched");
}

#[test]
fn transform_operations() {
    let dir = scratch("transform");
    let path = new_matrix(&dir);
    let out = dir.join("t.sheaf");
    ok(&["matrix", "transform", s(&path), "transpose", "-o", s(&out)]);
    let sheet = &sheet0(&out);
    assert_eq!((sheet.rows, sheet.cols), (4, 3));
    assert_eq!(sheet.value(3, 2), 34.0);
    ok(&["matrix", "transform", s(&path), "expand", "-o", s(&out)]);
    assert_eq!(sheet0(&out).cols, 8);
    ok(&["matrix", "transform", s(&path), "flip-horizontal", "--coordinates", "-o", s(&out)]);
    let sheet = &sheet0(&out);
    assert_eq!((sheet.value(0, 0), sheet.x_at(0)), (14.0, 40.0));
    ok(&["matrix", "transform", s(&path), "rotate-cw", "-o", s(&out)]);
    assert_eq!(sheet0(&out).value(0, 0), 31.0);
    let (good, _, err) = sheaf(&["matrix", "transform", s(&path), "shrink", "--cols", "0", "-o", s(&out)]);
    assert!(!good && err.contains("at least 1"), "{err}");
}

#[test]
fn stats_prints_the_numbers() {
    let dir = scratch("stats");
    let path = new_matrix(&dir);
    let out = ok(&["matrix", "stats", s(&path)]);
    assert!(out.contains("n        12") && out.contains("sum      270") && out.contains("min      11"), "{out}");
}

#[test]
fn convert_a_worksheet_and_xyz_columns_and_back() {
    let dir = scratch("convert");
    let csv = dir.join("table.csv");
    std::fs::write(&csv, "A,B,C\n1,5,9\n2,6,10\n3,7,11\n4,8,12\n").unwrap();
    let out = dir.join("c.sheaf");
    ok(&["matrix", "convert", s(&csv), "-o", s(&out)]);
    let p = load(&out);
    let sheet = &p.matrix_books().next().unwrap().sheets[0];
    assert_eq!((sheet.rows, sheet.cols), (4, 3));
    assert_eq!((sheet.value(0, 2), sheet.x_at(2), sheet.y_at(3)), (9.0, 3.0, 4.0));

    let xyz = dir.join("xyz.csv");
    let mut text = String::from("x,y,z\n");
    for x in [1, 2, 3] {
        for y in [10, 20, 30, 40] {
            text.push_str(&format!("{x},{y},{}\n", x * 100 + y));
        }
    }
    std::fs::write(&xyz, text).unwrap();
    let out2 = dir.join("g.sheaf");
    ok(&["matrix", "convert", s(&xyz), "-o", s(&out2), "--xyz"]);
    let sheet = &sheet0(&out2);
    assert_eq!((sheet.rows, sheet.cols), (4, 3));
    assert_eq!((sheet.value(1, 2), sheet.y_at(3)), (320.0, 40.0));

    // Back to a worksheet, written as text too.
    let txt = dir.join("back.txt");
    ok(&["matrix", "to-sheet", s(&out2), "--method", "xyz", "--export", s(&txt), "-o", s(&dir.join("b.sheaf"))]);
    let text = std::fs::read_to_string(&txt).unwrap();
    assert_eq!(text.lines().count(), 1 + 12, "{text}");
    assert!(text.lines().nth(1).unwrap().contains("1\t10\t110"), "{text}");

    // Scattered points are refused by the regular gridding.
    let scatter = dir.join("scatter.csv");
    std::fs::write(&scatter, "x,y,z\n0.113,0.207,1\n0.911,0.1,2\n0.5,0.549,3\n0.2,0.9,4\n0.83,0.8,5\n0.4,0.151,6\n")
        .unwrap();
    let (good, _, err) = sheaf(&["matrix", "convert", s(&scatter), "-o", s(&dir.join("x.sheaf")), "--xyz"]);
    assert!(!good && err.contains("regular grid"), "{err}");
}

#[test]
fn plot_a_matrix_in_each_template_and_save_the_graph() {
    let dir = scratch("plot");
    let path = new_matrix(&dir);
    for t in ["contour", "contour-lines", "heatmap"] {
        let svg = dir.join(format!("{t}.svg"));
        let saved = dir.join(format!("{t}.sheaf"));
        ok(&["matrix", "plot", s(&path), "-o", s(&svg), "-t", t, "--save", s(&saved)]);
        let text = std::fs::read_to_string(&svg).unwrap();
        assert!(text.contains("<svg") && text.contains("<path") || text.contains("<rect"), "{t}");
        let p = load(&saved);
        let g = p.graphs().next().expect("the graph is saved");
        let f = g.layers[0].field.as_ref().expect("with its field");
        assert!(matches!(f.source, FieldSource::Matrix { .. }));
    }
    let png = dir.join("c.png");
    ok(&["matrix", "plot", s(&path), "-o", s(&png), "--dpi", "50", "--major", "4", "--z-min=0", "--z-max=50"]);
    assert_eq!(&std::fs::read(&png).unwrap()[1..4], b"PNG");
    let (good, _, err) = sheaf(&["matrix", "plot", s(&path), "-o", s(&png), "-t", "nonsense"]);
    assert!(!good && err.contains("unknown template"), "{err}");
}

#[test]
fn plot_xyz_columns_with_and_without_a_grid() {
    let dir = scratch("plotxyz");
    let csv = dir.join("pts.csv");
    let mut text = String::from("x,y,z\n");
    for i in 0..40 {
        let (x, y) = (
            f64::from(i * 7 % 40) / 10.0 + f64::from(i % 3) * 0.031,
            f64::from(i * 13 % 40) / 10.0 + f64::from(i % 5) * 0.017,
        );
        text.push_str(&format!("{x},{y},{}\n", x * x - y));
    }
    std::fs::write(&csv, text).unwrap();
    let svg = dir.join("o.svg");
    ok(&["matrix", "plot", s(&csv), "-o", s(&svg)]);
    assert!(std::fs::read_to_string(&svg).unwrap().contains("<path"));
    let (good, _, err) = sheaf(&["matrix", "plot", s(&csv), "-o", s(&svg), "-t", "heatmap"]);
    assert!(!good && err.contains("regular grid"), "a heat map needs a grid: {err}");
}

#[test]
fn a_matrix_survives_saving_as_opj_in_the_companion_file() {
    let dir = scratch("opj");
    let opj = dir.join("m.opj");
    let (good, out, err) = sheaf(&["matrix", "new", "-o", s(&opj), "--cols", "3", "--rows", "2", "--formula", "i+j"]);
    assert!(good, "{err}");
    let companion = dir.join("m.opj.sheaf");
    assert!(companion.exists(), "the companion keeps the matrix: {out}{err}");
    assert_eq!(sheet0(&companion).value(1, 2), 5.0);
}
