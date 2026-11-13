//! `sheaf plot -t ternary` (graphing spec 14.20): the first three columns of a file are X, Y and Z.

use sheaf_core::graph::Coords;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-ternary-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn data(dir: &std::path::Path) -> String {
    let text = "Sand,Silt,Clay\n0.8,0.1,0.1\n0.6,0.3,0.1\n0.4,0.4,0.2\n0.2,0.3,0.5\n0.1,0.6,0.3\n0.54,0.27,0.09\n";
    let path = dir.join("soil.csv");
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn ternary_makes_a_triangle_with_three_titled_axes() {
    let dir = scratch("plot");
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .args(["plot", &data(&dir), "-t", "ternary", "-o", svg.to_str().unwrap(), "--save", saved.to_str().unwrap()])
        .output()
        .expect("the program starts");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let svg = std::fs::read_to_string(&svg).unwrap();
    for label in ["Sand", "Silt", "Clay", ">0.0<", ">0.2<", ">1.0<"] {
        assert!(svg.contains(label), "{label} missing");
    }
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert!(matches!(layer.coords, Coords::Ternary(_)));
    assert_eq!(layer.plots.len(), 1, "one plot, whose third column is Clay");
    let clay = layer.plots[0].size_from.and_then(|c| project.column(c)).map(|c| c.display_name().to_string());
    assert_eq!(clay.as_deref(), Some("Clay"));
}
