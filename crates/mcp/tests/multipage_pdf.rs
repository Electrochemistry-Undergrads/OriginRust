//! `export_graph` with several graphs writes one multi-page PDF (T32, mcp.md 5.2), driven over
//! the protocol as an assistant would.

use serde_json::{Value, json};
use sheaf_mcp::Server;
use sheaf_mcp::roots::Roots;
use std::path::PathBuf;

struct Dirs {
    base: PathBuf,
    root: PathBuf,
}

impl Dirs {
    fn new(tag: &str) -> Dirs {
        let base = std::env::temp_dir().join(format!("sheaf-mcp-multipdf-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let base = base.canonicalize().unwrap();
        let root = base.join("root");
        std::fs::create_dir_all(&root).unwrap();
        Dirs { base, root }
    }
}

impl Drop for Dirs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    let line = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}});
    let out = s.handle_line(&line.to_string()).expect("a reply");
    serde_json::from_str::<Value>(&out).unwrap()["result"].clone()
}

fn text(r: &Value) -> String {
    r["content"][0]["text"].as_str().unwrap_or("").to_string()
}

fn ok(r: &Value) -> Value {
    assert_eq!(r["isError"], false, "{r}");
    serde_json::from_str(&text(r)).unwrap()
}

fn refused(r: &Value) -> String {
    assert_eq!(r["isError"], true, "{r}");
    text(r)
}

/// A server on `d.root` with two graphs of different page sizes: Graph1 (default page) and
/// Graph2 (7 x 5 in).
fn two_graphs(d: &Dirs) -> Server {
    let mut s = Server::new(Roots::new(std::slice::from_ref(&d.root)).unwrap());
    std::fs::write(d.root.join("cv.csv"), "E,I\nV,A\n0,1\n1,4\n2,9\n3,16\n").unwrap();
    ok(&call(&mut s, "import_files", json!({"files": ["cv.csv"]})));
    assert_eq!(ok(&call(&mut s, "plot", json!({"template": "line"})))["graph"], "Graph1");
    let second = ok(&call(&mut s, "plot", json!({"template": "scatter", "size": {"width_in": 7, "height_in": 5}})));
    assert_eq!(second["graph"], "Graph2");
    s
}

/// The PDF's text (its dictionaries are not compressed).
fn pdf_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn page_count(pdf: &[u8]) -> usize {
    let t = pdf_text(pdf);
    let i = t.find("/Count ").expect("page tree count");
    t[i + 7..].split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

/// The pages' media boxes in file order (page order for this writer).
fn media_boxes(pdf: &[u8]) -> Vec<Vec<f64>> {
    let t = pdf_text(pdf);
    t.match_indices("/MediaBox [")
        .map(|(i, m)| {
            let rest = &t[i + m.len()..];
            rest[..rest.find(']').unwrap()].split_whitespace().map(|x| x.parse().unwrap()).collect()
        })
        .collect()
}

fn same_box(a: &[f64], b: &[f64]) -> bool {
    a.len() == 4 && b.len() == 4 && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
}

#[test]
fn graphs_in_the_order_given_become_the_pages_of_one_pdf() {
    let d = Dirs::new("order");
    let mut s = two_graphs(&d);
    let out = ok(&call(&mut s, "export_graph", json!({"graphs": ["Graph2", "Graph1"], "path": "x.pdf"})));
    assert_eq!(out["graphs"], json!(["Graph2", "Graph1"]));
    assert_eq!(out["pages"], 2);
    assert_eq!(out["graph"], "Graph2");
    assert_eq!(out["format"], "pdf");
    let pdf = std::fs::read(d.root.join("x.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert_eq!(out["bytes"], pdf.len());
    assert_eq!(page_count(&pdf), 2);
    let boxes = media_boxes(&pdf);
    assert_eq!(boxes.len(), 2, "{boxes:?}");
    // Each page is the one-graph PDF export of its graph, in the order asked for.
    for (page, name) in boxes.iter().zip(["Graph2", "Graph1"]) {
        let file = format!("{name}.pdf");
        ok(&call(&mut s, "export_graph", json!({"graph": name, "path": file})));
        let one = media_boxes(&std::fs::read(d.root.join(&file)).unwrap());
        assert_eq!(one.len(), 1);
        assert!(same_box(page, &one[0]), "{name}: page {page:?} vs one-graph export {:?}", one[0]);
    }
    assert!(!same_box(&boxes[0], &boxes[1]), "the two graphs have different pages: {boxes:?}");
}

#[test]
fn all_graphs_exports_every_graph_in_project_order() {
    let d = Dirs::new("all");
    let mut s = two_graphs(&d);
    let out = ok(&call(&mut s, "export_graph", json!({"all_graphs": true, "path": "all.pdf", "format": "pdf"})));
    assert_eq!(out["graphs"], json!(["Graph1", "Graph2"]));
    assert_eq!(out["pages"], 2);
    let pdf = std::fs::read(d.root.join("all.pdf")).unwrap();
    assert_eq!(page_count(&pdf), 2);
    assert_eq!(media_boxes(&pdf).len(), 2);
}

#[test]
fn one_graph_in_the_list_is_the_one_graph_export() {
    let d = Dirs::new("one");
    let mut s = two_graphs(&d);
    let listed = ok(&call(&mut s, "export_graph", json!({"graphs": ["Graph1"], "path": "listed.png", "dpi": 72})));
    let named = ok(&call(&mut s, "export_graph", json!({"graph": "Graph1", "path": "named.png", "dpi": 72})));
    assert_eq!(listed["graph"], "Graph1");
    assert!(listed.get("pages").is_none(), "{listed}");
    assert_eq!(listed["width_px"], named["width_px"]);
    assert_eq!(std::fs::read(d.root.join("listed.png")).unwrap(), std::fs::read(d.root.join("named.png")).unwrap());
}

#[test]
fn several_graphs_are_refused_outside_one_pdf_file() {
    let d = Dirs::new("refuse");
    let mut s = two_graphs(&d);
    let both = ["Graph1", "Graph2"];
    let msg = refused(&call(&mut s, "export_graph", json!({"graphs": both, "path": "x.png"})));
    assert!(msg.contains("PDF"), "{msg}");
    let msg = refused(&call(&mut s, "export_graph", json!({"graphs": both, "path": "x.pdf", "format": "svg"})));
    assert!(msg.contains("PDF"), "{msg}");
    let msg = refused(&call(&mut s, "export_graph", json!({"graphs": both})));
    assert!(msg.contains("path"), "{msg}");
    let msg =
        refused(&call(&mut s, "export_graph", json!({"all_graphs": true, "path": "x.pdf", "return_image": true})));
    assert!(msg.contains("return_image"), "{msg}");
    let msg = refused(&call(&mut s, "export_graph", json!({"graph": "Graph1", "graphs": both, "path": "x.pdf"})));
    assert!(msg.contains("graph"), "{msg}");
    refused(&call(&mut s, "export_graph", json!({"graph": "Graph1", "all_graphs": true, "path": "x.pdf"})));
    refused(&call(&mut s, "export_graph", json!({"graphs": both, "all_graphs": true, "path": "x.pdf"})));
    let msg = refused(&call(&mut s, "export_graph", json!({"graphs": ["Graph1", "Graph9"], "path": "x.pdf"})));
    assert!(msg.contains("Graph9"), "{msg}");
    refused(&call(&mut s, "export_graph", json!({"graphs": [], "path": "x.pdf"})));
    assert!(!d.root.join("x.pdf").exists() && !d.root.join("x.png").exists(), "nothing was written");
}

#[test]
fn a_multipage_path_outside_the_root_is_refused() {
    let d = Dirs::new("root");
    let mut s = two_graphs(&d);
    let outside = d.base.join("escape.pdf");
    for path in [outside.to_string_lossy().into_owned(), "../escape.pdf".to_string()] {
        refused(&call(&mut s, "export_graph", json!({"all_graphs": true, "path": path})));
    }
    assert!(!outside.exists());
}

#[test]
fn all_graphs_without_a_graph_is_refused() {
    let d = Dirs::new("none");
    let mut s = Server::new(Roots::new(std::slice::from_ref(&d.root)).unwrap());
    let msg = refused(&call(&mut s, "export_graph", json!({"all_graphs": true, "path": "x.pdf"})));
    assert!(msg.contains("no graph"), "{msg}");
}
