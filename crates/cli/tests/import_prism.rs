//! `sheaf import` of a GraphPad Prism XML file (T447): each table a sheet, replicates as columns, excluded values
//! masked.

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-prism-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sheaf(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sheaf")).args(args).output().expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const PZFX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<GraphPadPrismFile PrismXMLVersion="5.00">
<Table ID="Table0" XFormat="numbers" YFormat="replicates" Replicates="2" TableType="XY">
<Title>Growth</Title>
<XColumn Subcolumns="1"><Title>Day</Title><Subcolumn><d>0</d><d>1</d><d>2</d></Subcolumn></XColumn>
<YColumn Subcolumns="2"><Title>WT</Title><Subcolumn><d>1</d><d Excluded="1">9</d><d>3</d></Subcolumn><Subcolumn><d>1.1</d><d>2.1</d><d>3.1</d></Subcolumn></YColumn>
</Table>
<Table ID="Table1" YFormat="SEN" TableType="OneWay"><Title>Means</Title>
<YColumn Subcolumns="3"><Title>KO</Title><Subcolumn><d>5</d></Subcolumn><Subcolumn><d>0.5</d></Subcolumn><Subcolumn><d>4</d></Subcolumn></YColumn>
</Table>
</GraphPadPrismFile>"#;

#[test]
fn a_prism_file_imports_as_a_book_with_a_sheet_per_table() {
    let dir = scratch("import");
    let path = dir.join("assay.pzfx");
    std::fs::write(&path, PZFX).unwrap();
    let out_path = dir.join("assay.sheaf");
    let (ok, _, err) = sheaf(&["import", path.to_str().unwrap(), "-o", out_path.to_str().unwrap()]);
    assert!(ok, "{err}");
    let p = sheaf_io::project_file::load(&out_path).unwrap();
    let b = p.workbooks().next().unwrap();
    let names: Vec<&str> = b.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Growth", "Means"]);
    let wt = &b.sheets[0].columns[1];
    assert_eq!((wt.long_name.as_str(), wt.comments.as_str()), ("WT", "Replicate 1"));
    assert!(wt.mask.contains(&1), "the excluded value is masked");
    let roles: Vec<&str> = b.sheets[1].columns.iter().map(|c| c.comments.as_str()).collect();
    assert_eq!(roles, ["Mean", "SEM", "N"]);
}
