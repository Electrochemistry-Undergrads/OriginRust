//! `sheaf plot -t violin` (T37): one kernel density silhouette per Y column.

use sheaf_core::graph::PlotKind;
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-violin-{tag}-{}", std::process::id()));
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
    let mut text = String::from("x,alpha,beta,gamma\n");
    for i in 0..24 {
        let (a, b, c) =
            (5.0 + (i % 7) as f64 * 0.5, 2.0 + (i * i % 11) as f64 * 0.4, 8.0 + (i % 5) as f64 + (i / 12) as f64 * 6.0);
        text.push_str(&format!("{},{a},{b},{c}\n", i + 1));
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn every_violin_template_draws_one_silhouette_per_y_column() {
    for (template, kind) in [
        ("violin", PlotKind::Violin),
        ("violin-box", PlotKind::ViolinBox),
        ("violin-point", PlotKind::ViolinPoint),
        ("violin-quartile", PlotKind::ViolinQuartile),
        ("violin-stick", PlotKind::ViolinStick),
        ("half-violin", PlotKind::HalfViolin),
    ] {
        let dir = scratch(template);
        let csv = csv(&dir);
        let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
        let (ok, e) = sheaf(&[
            "plot",
            csv.to_str().unwrap(),
            "-t",
            template,
            "-o",
            svg.to_str().unwrap(),
            "--save",
            saved.to_str().unwrap(),
        ]);
        assert!(ok, "{template}: {e}");
        let text = std::fs::read_to_string(&svg).unwrap();
        assert!(text.contains("<svg") && text.contains("alpha") && text.contains("gamma"), "{template}");
        let project = sheaf_io::project_file::load(&saved).unwrap();
        let layer = &project.graphs().next().unwrap().layers[0];
        assert_eq!(layer.plots.len(), 3, "{template}");
        assert!(layer.plots.iter().all(|p| p.kind == kind), "{template}");
        assert_eq!(layer.default_kind, kind);
    }
}

#[test]
fn the_png_export_works_and_a_right_y_axis_is_refused() {
    let dir = scratch("png");
    let csv = csv(&dir);
    let png = dir.join("o.png");
    let (ok, e) = sheaf(&["plot", csv.to_str().unwrap(), "-t", "violin", "-o", png.to_str().unwrap()]);
    assert!(ok, "{e}");
    assert!(std::fs::metadata(&png).unwrap().len() > 1000);
    let svg = dir.join("o.svg");
    let (ok, e) = sheaf(&[
        "plot",
        csv.to_str().unwrap(),
        "-t",
        "violin",
        "--y",
        "alpha",
        "--y2",
        "beta",
        "-o",
        svg.to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(e.contains("violin plot cannot have a right Y axis"), "{e}");
}
