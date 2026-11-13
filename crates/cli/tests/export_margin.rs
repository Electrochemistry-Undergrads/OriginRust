//! `--margin`, `--border-width` and a `%` size of `sheaf plot` (Origin's export margin and size options).

use std::path::PathBuf;
use std::process::Command;

fn data(dir: &std::path::Path) -> PathBuf {
    let mut text = String::from("t,ramp\ns,A\n");
    for i in 0..=10 {
        text.push_str(&format!("{i},{i}\n"));
    }
    let path = dir.join("data.csv");
    std::fs::write(&path, text).unwrap();
    path
}

fn run(data: &std::path::Path, out: &std::path::Path, extra: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sheaf")).arg("plot").arg(data).arg("-o").arg(out).args(extra).output().unwrap()
}

/// The size in points of the root element of an SVG.
fn svg_size(path: &std::path::Path) -> (f64, f64) {
    let svg = std::fs::read_to_string(path).unwrap();
    let attr = |name: &str| -> f64 {
        svg.split(&format!("{name}=\"")).nth(1).unwrap().split("pt\"").next().unwrap().parse().unwrap()
    };
    (attr("width"), attr("height"))
}

#[test]
fn margin_modes_change_the_exported_area() {
    let dir: PathBuf = std::env::temp_dir().join(format!("sheaf-cli-margin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let data = data(&dir);
    let size_of = |name: &str, extra: &[&str]| {
        let out = dir.join(name);
        let r = run(&data, &out, extra);
        assert!(r.status.success(), "{name}: {}", String::from_utf8_lossy(&r.stderr));
        svg_size(&out)
    };
    let page = size_of("page.svg", &["--margin", "page"]);
    let tight = size_of("tight.svg", &["--margin", "tight"]);
    let border = size_of("border.svg", &["--margin", "border"]);
    let auto = size_of("auto.svg", &[]);
    let wide = size_of("wide.svg", &["--margin", "border", "--border-width", "20"]);
    let in_page = size_of("tip.svg", &["--margin", "tight-in-page"]);
    assert!(tight.0 < page.0 && tight.1 < page.1, "{tight:?} vs {page:?}");
    assert!((border.0 - tight.0 - 10.0).abs() < 0.01, "the default border is 5 pt a side: {border:?} vs {tight:?}");
    assert_eq!(auto, border, "the default is Origin's auto, which is Border");
    assert!((wide.0 - tight.0 - 40.0).abs() < 0.01, "{wide:?}");
    assert!(in_page.0 <= page.0 && (in_page.0 - tight.0).abs() < 0.01, "nothing hangs over the page here");

    // A size in percent is of the exported area; in inches it is that area's width.
    let half = size_of("half.svg", &["--margin", "tight", "--image-width", "50%"]);
    assert!((half.0 - tight.0 / 2.0).abs() < 0.01 && (half.1 - tight.1 / 2.0).abs() < 0.01, "{half:?}");
    let inches = size_of("in.svg", &["--margin", "tight", "--image-width", "2in"]);
    assert!((inches.0 - 144.0).abs() < 0.01 && (inches.1 - 144.0 * tight.1 / tight.0).abs() < 0.01, "{inches:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_margin_and_size_options_are_refused() {
    let dir: PathBuf = std::env::temp_dir().join(format!("sheaf-cli-margin-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let data = data(&dir);
    let out = dir.join("x.svg");
    for extra in [
        &["--margin", "wide"][..],
        &["--margin", "border", "--border-width", "-3"],
        &["--image-width", "0%"],
        &["--image-width", "50%", "--image-height", "2in"],
    ] {
        let r = run(&data, &out, extra);
        assert!(!r.status.success(), "{extra:?} should fail");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
