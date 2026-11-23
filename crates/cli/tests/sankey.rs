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
