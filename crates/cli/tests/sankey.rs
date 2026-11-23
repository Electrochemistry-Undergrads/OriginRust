//! `sheaf plot -t sankey` (T379, Origin 2020's Sankey diagram).

use std::process::Command;

fn sheaf(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn a_sankey_diagram_takes_the_text_columns_or_the_named_ones() {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-sankey-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("flows.csv");
    std::fs::write(
        &csv,
        "From,To,Via,Energy\nCoal,Power,x,25\nGas,Power,y,20\nPower,Homes,z,30\nPower,Industry,w,15\n",
    )
    .unwrap();
    let (png, saved) = (dir.join("s.png"), dir.join("s.sheaf"));
    let (c, p, s) = (csv.to_str().unwrap(), png.to_str().unwrap(), saved.to_str().unwrap());
    let (ok, e) = sheaf(&["plot", c, "-t", "sankey", "-o", p, "--save", s]);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let plot = &project.graphs().next().unwrap().layers[0].plots[0];
    let name = |id: Option<sheaf_core::ColumnId>| project.column(id.unwrap()).unwrap().long_name.clone();
    assert_eq!((name(plot.sankey.source), name(plot.sankey.target)), ("From".to_string(), "To".to_string()));
    assert!(std::fs::metadata(&png).unwrap().len() > 1000);
    let (ok, e) = sheaf(&["plot", c, "-t", "sankey", "--group", "From,Via", "-o", p, "--save", s]);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let plot = &project.graphs().next().unwrap().layers[0].plots[0];
    assert_eq!(project.column(plot.sankey.target.unwrap()).unwrap().long_name, "Via");
    let (ok, e) = sheaf(&["plot", c, "-t", "sankey", "--group", "From", "-o", p]);
    assert!(!ok && e.contains("source and a target"), "{e}");
}

#[test]
fn an_alluvial_diagram_counts_rows_or_weights_them_by_y() {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-alluvial-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("rows.csv");
    std::fs::write(&csv, "Class,Sex,Survived,Weight\nFirst,F,Yes,2\nThird,M,No,1\nFirst,M,No,3\n").unwrap();
    let (png, saved) = (dir.join("a.png"), dir.join("a.sheaf"));
    let (c, p, s) = (csv.to_str().unwrap(), png.to_str().unwrap(), saved.to_str().unwrap());
    let head = ["--header-lines", "1", "--long-name-line", "1"];
    let mut args = vec!["plot", c, "-t", "alluvial", "-o", p, "--save", s];
    args.extend(head);
    let (ok, e) = sheaf(&args);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let plot = &project.graphs().next().unwrap().layers[0].plots[0];
    assert_eq!(plot.sankey.stages.len(), 3, "the three text columns");
    assert!(project.numeric(plot.y).is_none(), "counted: no weight column");
    let mut args = vec!["plot", c, "-t", "alluvial", "--group", "Class,Survived", "-y", "Weight", "-o", p, "--save", s];
    args.extend(head);
    let (ok, e) = sheaf(&args);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let plot = &project.graphs().next().unwrap().layers[0].plots[0];
    assert_eq!(plot.sankey.stages.len(), 2);
    assert_eq!(project.column(plot.y).unwrap().long_name, "Weight");
    let (ok, e) = sheaf(&["plot", c, "-t", "alluvial", "--group", "Class", "-o", p]);
    assert!(!ok && e.contains("two or more stage columns"), "{e}");
}

#[test]
fn a_network_plot_takes_its_ends_and_an_optional_weight() {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-network-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("edges.csv");
    std::fs::write(&csv, "From,To,Papers\nAnn,Bob,3\nBob,Cy,1\nCy,Ann,2\n").unwrap();
    let (png, saved) = (dir.join("n.png"), dir.join("n.sheaf"));
    let (c, p, s) = (csv.to_str().unwrap(), png.to_str().unwrap(), saved.to_str().unwrap());
    let (ok, e) = sheaf(&["plot", c, "-t", "network", "-y", "Papers", "-o", p, "--save", s]);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let plot = &project.graphs().next().unwrap().layers[0].plots[0];
    assert_eq!(plot.kind, sheaf_core::graph::PlotKind::Network);
    assert_eq!(project.column(plot.y).unwrap().long_name, "Papers");
    assert_eq!(project.column(plot.sankey.target.unwrap()).unwrap().long_name, "To");
    let (ok, e) = sheaf(&["plot", c, "-t", "network", "--group", "From", "-o", p]);
    assert!(!ok && e.contains("source and a target"), "{e}");
}
