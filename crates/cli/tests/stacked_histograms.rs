//! `sheaf plot -t stacked-histograms` (T62): one histogram panel per Y column, shared bins and X.

use sheaf_core::graph::PlotKind;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-stackhist-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn csv(dir: &Path, columns: usize) -> PathBuf {
    let path = dir.join("d.csv");
    let mut text = String::from("x,alpha,beta,gamma\n");
    for i in 0..30 {
        let row: Vec<String> =
            (0..columns).map(|c| format!("{}", 5.0 + c as f64 * 6.0 + ((i * (c + 3)) % 11) as f64 * 0.9)).collect();
        text.push_str(&format!("{},{}\n", i + 1, row.join(",")));
    }
    let header = ["x", "alpha", "beta", "gamma"][..columns + 1].join(",");
    let body: Vec<&str> = text.lines().skip(1).collect();
    std::fs::write(&path, format!("{header}\n{}\n", body.join("\n"))).unwrap();
    path
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn three_columns_become_three_stacked_histogram_panels_with_shared_bins() {
    let dir = scratch("three");
    let data = csv(&dir, 3);
    let (png, saved) = (dir.join("o.png"), dir.join("p.sheaf"));
    let (ok, e) = sheaf(&["plot", s(&data), "-t", "stacked-histograms", "-o", s(&png), "--save", s(&saved)]);
    assert!(ok, "{e}");
    assert!(std::fs::metadata(&png).unwrap().len() > 1000);
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let g = project.graphs().next().unwrap();
    assert_eq!(g.layers.len(), 3);
    let hists: Vec<_> = g.layers.iter().map(|l| l.plots[0].hist).collect();
    assert!(g.layers.iter().all(|l| l.plots[0].kind == PlotKind::Histogram));
    assert!(hists.iter().all(|h| *h == hists[0] && h.bin_size.is_some()));
}

#[test]
fn stacked_histograms_refuse_panels_and_text_only_data() {
    let dir = scratch("bad");
    let data = csv(&dir, 3);
    let out = dir.join("bad.svg");
    let (ok, e) = sheaf(&["plot", s(&data), "-t", "stacked-histograms", "--panels", "stack", "-o", s(&out)]);
    assert!(!ok && e.contains("--panels"), "{e}");
    let text = dir.join("t.csv");
    std::fs::write(&text, "a,b\nx,y\nz,w\n").unwrap();
    let (ok, _) = sheaf(&["plot", s(&text), "-t", "stacked-histograms", "-o", s(&out)]);
    assert!(!ok);
    assert!(!out.exists());
}
