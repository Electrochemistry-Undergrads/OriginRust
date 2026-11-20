//! `batch_process` (T238): the shared batch run of File > Batch Processing... and `sheaf batch` on the
//! session project; paths are checked against the roots.

use serde_json::{Value, json};
use sheaf_core::analysis::{Boundary, OutputTarget, Request, RowRange, SmoothMethod, SmoothOptions, Tool};
use sheaf_core::column::{ColumnData, Designation, RecalcMode};
use sheaf_core::{BookTemplate, Project};
use sheaf_mcp::Server;
use sheaf_mcp::roots::Roots;
use std::path::PathBuf;

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let p = std::env::temp_dir().join(format!("sheaf-mcp-batch-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Dir(p.canonicalize().unwrap())
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    let line = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}});
    let out = s.handle_line(&line.to_string()).expect("a reply");
    let v: Value = serde_json::from_str(&out).unwrap();
    v["result"].clone()
}

fn text(r: &Value) -> String {
    r["content"][0]["text"].as_str().unwrap_or("").to_string()
}

fn payload(r: &Value) -> Value {
    assert_eq!(r["isError"], false, "{r}");
    serde_json::from_str(&text(r)).unwrap()
}

/// An analysis template (sheet Data: X, Y and an Auto smoothing of Y) and two data files.
fn setup(dir: &Dir) -> String {
    let mut p = Project::new();
    let b = p.add_workbook("Assay");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 0.0, 1.0])).unwrap();
    let o = SmoothOptions {
        method: SmoothMethod::AdjacentAveraging { weighted: false },
        npts: 3,
        boundary: Boundary::None,
    };
    let request = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) };
    p.analyze_linked(&request, &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
    p.workbook_mut(b).unwrap().sheets[0].name = "Data".into();
    let t = BookTemplate::analysis_from_workbook("Assay", &p, b).unwrap();
    let saved = sheaf_io::user_templates::save_book(&dir.0, &t).unwrap();
    std::fs::write(dir.0.join("run1.csv"), "t,v\n1,0\n2,3\n3,0\n4,3\n5,0\n").unwrap();
    std::fs::write(dir.0.join("run2.csv"), "t,v\n1,1\n2,1\n3,1\n").unwrap();
    saved.path.display().to_string()
}

#[test]
fn batch_process_makes_the_books_and_the_summary() {
    let dir = Dir::new("run");
    let template = setup(&dir);
    let mut s = Server::new(Roots::new(std::slice::from_ref(&dir.0)).unwrap());
    let files = [dir.0.join("run*.csv").display().to_string()];
    let r = payload(&call(&mut s, "batch_process", json!({"files": files, "template": template})));
    assert_eq!(r["template"], "Assay");
    assert_eq!(r["books"].as_array().unwrap().len(), 2, "{r}");
    assert_eq!(r["books"][0]["file"], "run1.csv");
    assert_eq!(r["books"][0]["filled"], 2);
    assert_eq!(r["summary"], Value::Null);
    let book = r["books"][0]["book"].as_str().unwrap().to_string();
    let cols = payload(&call(&mut s, "read_columns", json!({"book": book})));
    assert!(cols.to_string().contains("3.0") || cols.to_string().contains("3,"), "{cols}");

    let r = payload(&call(
        &mut s,
        "batch_process",
        json!({"files": files, "template": template, "summary": "data", "identifier": "File", "delete_books": true}),
    ));
    assert_eq!(r["deleted"], true);
    assert_eq!(r["books"][0]["book"], Value::Null, "the books are gone");
    assert_eq!(r["summary"]["rows"], 8, "{r}");
    let contents = payload(&call(&mut s, "list_contents", json!({})));
    assert!(contents.to_string().contains("Batch Summary"), "{contents}");
}

#[test]
fn bad_requests_change_nothing() {
    let dir = Dir::new("bad");
    let template = setup(&dir);
    let other = Dir::new("outside");
    let mut s = Server::new(Roots::new(std::slice::from_ref(&dir.0)).unwrap());
    let files = [dir.0.join("run1.csv").display().to_string()];
    let refused = |r: &Value| r["isError"] == true;
    let r = call(&mut s, "batch_process", json!({"files": files, "template": template, "summary": "Stats"}));
    assert!(refused(&r) && text(&r).contains("no sheet 'Stats'"), "{r}");
    let r = call(&mut s, "batch_process", json!({"files": files, "template": template, "delete_books": true}));
    assert!(refused(&r) && text(&r).contains("needs summary"), "{r}");
    let outside = other.0.join("x.sheafbooktemplate").display().to_string();
    let r = call(&mut s, "batch_process", json!({"files": files, "template": outside}));
    assert!(refused(&r), "a template outside the roots is refused: {r}");
    let r = call(
        &mut s,
        "batch_process",
        json!({"files": [dir.0.join("none.csv").display().to_string()], "template": template}),
    );
    assert!(refused(&r), "{r}");
    let contents = payload(&call(&mut s, "list_contents", json!({})));
    assert!(!contents.to_string().contains("Book"), "nothing was added: {contents}");
}
