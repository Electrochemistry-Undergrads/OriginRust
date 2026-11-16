//! `sheaf plot -t NAME` with a user graph template (T56): a name that is no built-in template plots
//! with the template of that name in `--templates-dir`.

use sheaf_core::graph::Dash;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project, UserTemplate};
use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-utpl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .args(args)
        .env_remove("SHEAF_TEMPLATES_DIR")
        .output()
        .expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Saves a one-plot template with a 3.5 pt dash-dot line into `dir`.
fn save_template(dir: &std::path::Path, name: &str) {
    let mut p = Project::new();
    let book = p.add_workbook("src");
    p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let g = p.plot_columns(Template::Line, &[y]).unwrap();
    let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
    plot.line.width_pt = 3.5;
    plot.line.dash = Dash::DashDot;
    let t = UserTemplate::from_graph(name, p.graph(g).unwrap()).unwrap();
    sheaf_io::user_templates::save(dir, &t).unwrap();
}

#[test]
fn a_user_template_name_plots_in_the_saved_look() {
    let dir = scratch("ok");
    let templates = dir.join("templates");
    save_template(&templates, "My Look");
    let data = dir.join("d.csv");
    std::fs::write(&data, "X,A,B\n0,0,1\n1,2,3\n2,4,5\n").unwrap();
    let (svg, saved) = (dir.join("o.svg"), dir.join("p.sheaf"));
    let (ok, err) = sheaf(&[
        "plot",
        data.to_str().unwrap(),
        "-t",
        "my look",
        "--templates-dir",
        templates.to_str().unwrap(),
        "-o",
        svg.to_str().unwrap(),
        "--save",
        saved.to_str().unwrap(),
    ]);
    assert!(ok, "{err}");
    let p: Project = sheaf_io::project_file::load(&saved).unwrap();
    let plots = &p.graphs().next().unwrap().layers[0].plots;
    assert_eq!(plots.len(), 2, "both Y columns are plotted");
    for plot in plots {
        assert_eq!((plot.line.width_pt, plot.line.dash), (3.5, Dash::DashDot), "the saved look, repeated");
    }
    assert!(svg.is_file());
}

#[test]
fn an_unknown_name_lists_the_built_in_templates_and_mentions_user_templates() {
    let dir = scratch("unknown");
    let data = dir.join("d.csv");
    std::fs::write(&data, "X,A\n0,0\n1,2\n").unwrap();
    let (ok, err) = sheaf(&[
        "plot",
        data.to_str().unwrap(),
        "-t",
        "nothing-like-this",
        "--templates-dir",
        dir.join("empty").to_str().unwrap(),
        "-o",
        dir.join("o.svg").to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(err.contains("unknown template") && err.contains("Save Template As"), "{err}");
}
