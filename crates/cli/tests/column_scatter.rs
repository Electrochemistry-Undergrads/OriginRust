//! `sheaf plot -t column-scatter` (T79).

use sheaf_core::graph::PlotKind;
use std::process::Command;

#[test]
fn the_column_scatter_template_makes_column_scatter_plots() {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-column-scatter-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("d.csv");
    std::fs::write(&csv, "x,a,b\n1,2,3\n2,4,1\n3,3,4\n4,5,2\n").unwrap();
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .args(["plot", csv.to_str().unwrap(), "-t", "column-scatter", "-o", svg.to_str().unwrap(), "--save"])
        .arg(&saved)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(std::fs::read_to_string(&svg).unwrap().contains("<svg"));
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.plots.len(), 2);
    assert!(layer.plots.iter().all(|p| p.kind == PlotKind::ColumnScatter));
}
