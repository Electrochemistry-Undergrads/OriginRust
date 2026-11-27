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

#[test]
fn treemaps_and_sunbursts_of_text_columns_count_the_rows() {
    // T392: nested frequency charts.
    let dir = std::env::temp_dir().join(format!("sheaf-cli-nested-freq-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("rows.csv");
    std::fs::write(&csv, "Class,Sex\nFirst,F\nThird,M\nFirst,M\nFirst,F\n").unwrap();
    let (png, saved) = (dir.join("f.png"), dir.join("f.sheaf"));
    let (c, p, s) = (csv.to_str().unwrap(), png.to_str().unwrap(), saved.to_str().unwrap());
    for template in ["sunburst", "treemap"] {
        let args = ["plot", c, "-t", template, "--header-lines", "1", "--long-name-line", "1", "-o", p, "--save", s];
        let (ok, e) = sheaf(&args);
        assert!(ok, "{template}: {e}");
        let project = sheaf_io::project_file::load(&saved).unwrap();
        let plot = &project.graphs().next().unwrap().layers[0].plots[0];
        assert!(project.numeric(plot.y).is_none(), "{template}: counted");
    }
}

#[test]
fn a_chord_diagram_takes_its_ends_and_an_optional_weight() {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-chord-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("trade.csv");
    std::fs::write(&csv, "From,To,Trade\nAnn,Bob,3\nBob,Ann,1\nCy,Ann,2\n").unwrap();
    let (png, saved) = (dir.join("c.png"), dir.join("c.sheaf"));
    let (c, p, s) = (csv.to_str().unwrap(), png.to_str().unwrap(), saved.to_str().unwrap());
    let (ok, e) = sheaf(&["plot", c, "-t", "chord", "-y", "Trade", "-o", p, "--save", s]);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let plot = &project.graphs().next().unwrap().layers[0].plots[0];
    assert_eq!(plot.kind, sheaf_core::graph::PlotKind::Chord);
    assert_eq!(project.column(plot.y).unwrap().long_name, "Trade");
    assert_eq!(project.column(plot.sankey.source.unwrap()).unwrap().long_name, "From");
    let (ok, e) = sheaf(&["plot", c, "-t", "chord", "--group", "From", "-o", p]);
    assert!(!ok && e.contains("a chord diagram needs a source and a target"), "{e}");
    // T506: a flow matrix, sources down the text column and one column per target.
    let matrix = dir.join("matrix.csv");
    std::fs::write(&matrix, "From,Ann,Bob\nAnn,0,3\nBob,1,0\nCy,2,0\n").unwrap();
    let m = matrix.to_str().unwrap();
    let (ok, e) = sheaf(&["plot", m, "-t", "chord", "--matrix", "-o", p, "--save", s]);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let plot = &project.graphs().next().unwrap().layers[0].plots[0];
    assert_eq!(plot.kind, sheaf_core::graph::PlotKind::Chord);
    assert_eq!(project.numeric(plot.y).unwrap(), &[1.0, 2.0, 3.0], "Bob, Cy to Ann; Ann to Bob");
    let (ok, e) = sheaf(&["plot", m, "-t", "network", "--matrix", "-o", p]);
    assert!(!ok && e.contains("--matrix is for chord diagrams"), "{e}");
}

#[test]
fn an_upset_plot_takes_the_flag_columns_or_the_named_ones() {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-upset-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("sets.csv");
    std::fs::write(&csv, "Id,A,B,C\n10,1,0,1\n11,1,1,0\n12,0,1,1\n13,1,0,0\n").unwrap();
    let (png, saved) = (dir.join("u.png"), dir.join("u.sheaf"));
    let (c, p, s) = (csv.to_str().unwrap(), png.to_str().unwrap(), saved.to_str().unwrap());
    // Without -y: every column of only 0 and 1 (not Id).
    let (ok, e) = sheaf(&["plot", c, "-t", "upset", "-o", p, "--save", s]);
    assert!(ok, "{e}");
    let project = sheaf_io::project_file::load(&saved).unwrap();
    let plot = &project.graphs().next().unwrap().layers[0].plots[0];
    assert_eq!(plot.kind, sheaf_core::graph::PlotKind::UpSet);
    let names: Vec<&str> =
        plot.sankey.stages.iter().map(|id| project.column(*id).unwrap().long_name.as_str()).collect();
    assert_eq!(names, ["A", "B", "C"]);
    let (ok, e) = sheaf(&["plot", c, "-t", "upset", "-y", "A", "-o", p]);
    assert!(!ok && e.contains("two or more set columns"), "{e}");
}
