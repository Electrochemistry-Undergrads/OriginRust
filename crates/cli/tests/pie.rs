//! `sheaf plot -t pie-3d` and `-t pie-bw` (graphing spec 14.21).

use sheaf_core::graph::PlotKind;
use std::process::Command;

fn run(template: &str) -> (String, sheaf_core::Project) {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-pie-{template}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("pie.csv");
    std::fs::write(&csv, "name,v\nA,3\nB,5\nC,2\nD,7\n").unwrap();
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .args([
            "plot",
            csv.to_str().unwrap(),
            "-t",
            template,
            "-o",
            svg.to_str().unwrap(),
            "--save",
            saved.to_str().unwrap(),
        ])
        .output()
        .expect("the program starts");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    (std::fs::read_to_string(&svg).unwrap(), sheaf_io::project_file::load(&saved).unwrap())
}

#[test]
fn pie_3d_and_pie_bw_make_their_plot_kinds_and_pie_stays() {
    for (name, kind) in [("pie-3d", PlotKind::Pie3D), ("pie-bw", PlotKind::PieBw), ("pie", PlotKind::Pie)] {
        let (svg, project) = run(name);
        let layer = &project.graphs().next().unwrap().layers[0];
        assert_eq!(layer.plots[0].kind, kind, "{name}");
        assert!(svg.contains("17.65%"), "{name}");
    }
}
