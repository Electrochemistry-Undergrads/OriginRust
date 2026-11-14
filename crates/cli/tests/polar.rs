//! `sheaf plot -t polar` and `-t polar-r-theta` (graphing spec 14.18).

use sheaf_core::graph::{Coords, RadiusColumn};
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-polar-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn data(dir: &std::path::Path) -> String {
    let mut text = String::from("theta,r\n");
    for i in 0..=24 {
        let t = f64::from(i) * 15.0;
        text += &format!("{t},{:.4}\n", 1.5 + 0.5 * (2.0 * t.to_radians()).cos());
    }
    let path = dir.join("polar.csv");
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

fn run(template: &str, tag: &str) -> (String, sheaf_core::Project) {
    let dir = scratch(tag);
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .args(["plot", &data(&dir), "-t", template, "-o", svg.to_str().unwrap(), "--save", saved.to_str().unwrap()])
        .output()
        .expect("the program starts");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    (std::fs::read_to_string(&svg).unwrap(), sheaf_io::project_file::load(&saved).unwrap())
}

#[test]
fn polar_makes_a_circle_with_the_angles_round_it() {
    let (svg, project) = run("polar", "theta");
    let layer = &project.graphs().next().unwrap().layers[0];
    assert_eq!(layer.coords.polar().map(|c| c.radius), Some(RadiusColumn::Y));
    // Angle labels every 30 degrees, a circle drawn with arcs.
    for label in [">0<", ">30<", ">150<", ">330<"] {
        assert!(svg.contains(label), "{label} missing");
    }
    assert!(svg.contains("0 1 1"), "the circle is drawn with arcs");
}

#[test]
fn polar_r_theta_takes_the_radius_from_x() {
    let (_, project) = run("polar-r-theta", "r-theta");
    let layer = &project.graphs().next().unwrap().layers[0];
    assert!(matches!(layer.coords, Coords::Polar(c) if c.radius == RadiusColumn::X));
}

#[test]
fn radar_names_the_spokes_by_the_first_column_and_closes_each_polygon() {
    let dir = scratch("radar");
    let csv = dir.join("radar.csv");
    std::fs::write(&csv, "Skill,A,B\nSpeed,4,2\nPower,3,4.5\nRange,5,3\nComfort,2,4\nSafety,4.5,2.5\nPrice,3.5,5\n")
        .unwrap();
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .args([
            "plot",
            csv.to_str().unwrap(),
            "-t",
            "radar",
            "-o",
            svg.to_str().unwrap(),
            "--save",
            saved.to_str().unwrap(),
        ])
        .output()
        .expect("the program starts");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let svg = std::fs::read_to_string(&svg).unwrap();
    for name in ["Speed", "Power", "Range", "Comfort", "Safety", "Price"] {
        assert!(svg.contains(&format!(">{name}<")), "{name} missing");
    }
    assert!(svg.contains("Z"), "the grid polygons are closed paths");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let layer = &project.graphs().next().unwrap().layers[0];
    assert!(layer.coords.polar().is_some_and(|p| p.radar));
    assert_eq!(layer.plots.len(), 2);
}
