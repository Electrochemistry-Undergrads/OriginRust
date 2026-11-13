//! Checks of the horizontal bar templates from `sheaf plot`: `-t stacked-bar`, `stacked-bar-100`
//! and `floating-bar`.

use sheaf_core::graph::PlotKind;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-bars-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_bar_templates_make_horizontal_plots_of_their_kind() {
    let dir = scratch("kinds");
    let data = dir.join("d.csv");
    std::fs::write(&data, "X,A,B,C\n1,2,1,3\n2,3,2,1\n3,4,1,2\n4,5,2,1\n").unwrap();
    for (name, kind, plots) in [
        ("stacked-bar", PlotKind::StackedBar, 3),
        ("stacked-bar-100", PlotKind::Stacked100Bar, 3),
        // Columns go in pairs for a floating bar: the first of a pair is the lower end.
        ("floating-bar", PlotKind::FloatingBar, 1),
    ] {
        let (svg, saved) = (dir.join(format!("{name}.svg")), dir.join(format!("{name}.sheaf")));
        let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
            .args(["plot", data.to_str().unwrap(), "-t", name, "-o", svg.to_str().unwrap()])
            .args(["--save", saved.to_str().unwrap()])
            .output()
            .expect("the program starts");
        assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
        let p = sheaf_io::project_file::load(&saved).expect("the saved project loads");
        let l = &p.graphs().next().expect("one graph").layers[0];
        assert!(l.plots.iter().all(|q| q.kind == kind), "{name}");
        assert_eq!(l.plots.len(), plots, "{name}");
        assert!(std::fs::read_to_string(&svg).unwrap().contains("<svg"));
    }
}
