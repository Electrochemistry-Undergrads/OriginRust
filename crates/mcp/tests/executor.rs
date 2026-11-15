//! The shared executor (docs/spec/mcp.md 11.10): the headless server runs on a `Document` and the
//! app runs the same code with an undo history. These tests drive both the way the app will.

use serde_json::{Value, json};
use sheaf_core::column::{ColumnData, Designation, RecalcMode};
use sheaf_core::{Document, Project};
use sheaf_mcp::Server;
use sheaf_mcp::exec::{Policy, Session, run_on_document};
use sheaf_mcp::files::{OpenFiles, protected_set};
use sheaf_mcp::roots::Roots;
use std::path::PathBuf;

struct Dirs {
    base: PathBuf,
    root: PathBuf,
}

impl Dirs {
    fn new(tag: &str) -> Dirs {
        let base = std::env::temp_dir().join(format!("sheaf-mcp-exec-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let base = base.canonicalize().unwrap();
        let root = base.join("root");
        std::fs::create_dir_all(&root).unwrap();
        Dirs { base, root }
    }

    fn roots(&self) -> Roots {
        Roots::new(std::slice::from_ref(&self.root)).unwrap()
    }

    fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(&self.root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }
}

impl Drop for Dirs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

/// The reply of `tools/call` through the headless server.
fn headless(s: &mut Server, name: &str, args: Value) -> Value {
    let line = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}});
    let out = s.handle_line(&line.to_string()).expect("a reply");
    serde_json::from_str::<Value>(&out).unwrap()["result"].clone()
}

/// A caller of the executor, as the app is.
struct Caller {
    doc: Document,
    session: Session,
    policy: Policy,
}

impl Caller {
    fn new(doc: Document, policy: Policy) -> Caller {
        Caller { doc, session: Session::default(), policy }
    }

    fn with_history(d: &Dirs) -> Caller {
        Caller::new(Document::default(), Policy::headless(d.roots()))
    }

    fn attached(d: &Dirs) -> Caller {
        Caller::new(Document::default(), Policy::attached(d.roots()))
    }

    fn call(&mut self, name: &str, args: Value) -> Value {
        match run_on_document(&mut self.doc, &mut self.session, &self.policy, name, &args) {
            Ok(out) => out.into_result(true),
            Err(e) => e.into_result(true),
        }
    }
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

fn write_csv(d: &Dirs, name: &str) {
    // A line with a little deterministic scatter, so a fit is not a perfect one.
    let rows: String =
        (0..20).map(|i| format!("{i},{}\n", 2.0 * f64::from(i) + 1.0 + f64::from(i * 7 % 5) * 0.1)).collect();
    std::fs::write(d.root.join(name), format!("x,y\n{rows}")).unwrap();
}

fn rows(project: &Project, column: &str) -> Vec<f64> {
    let ws = &project.workbooks().next().unwrap().sheets[0];
    ws.columns.iter().find(|c| c.short_name == column).unwrap().numeric().unwrap().to_vec()
}

// ----- the headless server on a Document ---------------------------------------------------

/// A project with B = col(A)*2 as an automatic formula, and values in C that no formula gives.
fn project_with_formula_column() -> Project {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
    let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
    p.column_mut(a).unwrap().short_name = "A".into();
    p.column_mut(b).unwrap().short_name = "B".into();
    p.set_column_values_with(b, "col(A)*2", None, RecalcMode::Auto).unwrap();
    p
}

#[test]
fn a_formula_column_follows_set_column_values_in_the_headless_server() {
    let d = Dirs::new("follow");
    let file = d.root.join("f.sheaf");
    sheaf_io::save::save_project(&project_with_formula_column(), &file).unwrap();
    let mut s = Server::new(d.roots());
    ok(&headless(&mut s, "open_project", json!({"path": "f.sheaf"})));
    let before = ok(&headless(&mut s, "read_columns", json!({"columns": ["B"]})));
    assert_eq!(before["rows"], json!([[2.0], [4.0], [6.0]]));
    ok(&headless(&mut s, "set_column_values", json!({"column": "A", "formula": "col(A)*10"})));
    let after = ok(&headless(&mut s, "read_columns", json!({"columns": ["B"]})));
    assert_eq!(after["rows"], json!([[20.0], [40.0], [60.0]]), "B follows its input");
}

#[test]
fn opening_a_project_with_a_formula_column_leaves_its_data_unchanged() {
    let d = Dirs::new("open");
    // The stored values of B are not what the formula gives: a recalculation would show.
    let mut p = project_with_formula_column();
    let b = p.workbooks().next().unwrap().sheets[0].columns[1].id;
    *p.column_mut(b).unwrap().numeric_mut().unwrap() = vec![100.0, 200.0, 300.0];
    sheaf_io::save::save_project(&p, &d.root.join("stale.sheaf")).unwrap();

    let mut s = Server::new(d.roots());
    ok(&headless(&mut s, "open_project", json!({"path": "stale.sheaf"})));
    assert_eq!(rows(s.project(), "B"), [100.0, 200.0, 300.0]);
    // The same through the executor with a history: replace, not an edit.
    let mut c = Caller::with_history(&d);
    ok(&c.call("open_project", json!({"path": "stale.sheaf"})));
    assert_eq!(rows(c.doc.project(), "B"), [100.0, 200.0, 300.0]);
    assert!(c.doc.undo_label().is_none(), "opening is no undo step");
    assert_eq!(c.doc.generation(), 1);
    assert!(!c.doc.dirty);
}

#[test]
fn new_project_replaces_the_document_and_clears_the_history() {
    let d = Dirs::new("new");
    write_csv(&d, "a.csv");
    let mut c = Caller::with_history(&d);
    ok(&c.call("import_files", json!({"files": ["a.csv"]})));
    assert!(c.doc.undo_label().is_some());
    ok(&c.call("new_project", json!({})));
    assert!(c.doc.undo_label().is_none());
    assert_eq!(c.doc.project().workbooks().count(), 0);
    assert_eq!(c.doc.generation(), 1);
    assert!(c.session.suggested.is_empty());
}

#[test]
fn a_failed_open_keeps_the_project() {
    let d = Dirs::new("failopen");
    write_csv(&d, "a.csv");
    let mut c = Caller::with_history(&d);
    ok(&c.call("import_files", json!({"files": ["a.csv"]})));
    let before = c.doc.project().clone();
    refused(&c.call("open_project", json!({"path": "missing.sheaf"})));
    refused(&c.call("open_project", json!({"path": "a.csv"})));
    assert_eq!(c.doc.project(), &before);
    assert_eq!(c.doc.generation(), 0);
}

#[test]
fn a_fit_whose_plot_cannot_be_written_leaves_no_fit_behind() {
    let d = Dirs::new("fit");
    write_csv(&d, "a.csv");
    let mut s = Server::new(d.roots());
    ok(&headless(&mut s, "import_files", json!({"files": ["a.csv"]})));
    let before = ok(&headless(&mut s, "list_contents", json!({})));
    let e = refused(&headless(&mut s, "fit", json!({"model": "linear", "y": ["B"], "plot": "no_such_folder/fit.png"})));
    assert!(e.contains("could not write"), "{e}");
    assert_eq!(ok(&headless(&mut s, "list_contents", json!({}))), before, "no fit sheets, columns or graph");
    // Without a plot the same fit works and adds its results.
    ok(&headless(&mut s, "fit", json!({"model": "linear", "y": ["B"]})));
    assert_ne!(ok(&headless(&mut s, "list_contents", json!({}))), before);
}

#[test]
fn results_name_the_book_and_the_sheet() {
    let d = Dirs::new("names");
    write_csv(&d, "a.csv");
    let mut s = Server::new(d.roots());
    ok(&headless(&mut s, "import_files", json!({"files": ["a.csv"]})));
    let set = ok(&headless(&mut s, "set_column_values", json!({"column": "C", "formula": "col(B)+1", "create": {}})));
    assert_eq!((set["book"].as_str(), set["sheet"].as_str()), (Some("Book1"), Some("a")));
    let plot = ok(&headless(&mut s, "plot", json!({"y": ["B"]})));
    assert_eq!((plot["book"].as_str(), plot["sheet"].as_str()), (Some("Book1"), Some("a")));
    assert_eq!(plot["graph"], "Graph1");
    let restyle = ok(&headless(&mut s, "plot", json!({"graph": "Graph1", "style": {"line_width": 2.0}})));
    assert!(restyle["book"].is_null() && restyle["sheet"].is_null(), "{restyle}");
    let sort = ok(&headless(&mut s, "sort_sheet", json!({"key": "B", "descending": true})));
    assert_eq!((sort["book"].as_str(), sort["sheet"].as_str()), (Some("Book1"), Some("a")));
    let filter = ok(&headless(&mut s, "filter_sheet", json!({"condition": "col(B)>10"})));
    assert_eq!(filter["book"], "Book1");
    assert_eq!(filter["source_sheet"], "a");
    assert_ne!(filter["sheet"], "a", "the rows went to a new sheet, as before");
    let export = ok(&headless(&mut s, "export_data", json!({"path": "out.csv", "sheet": "a"})));
    assert_eq!((export["book"].as_str(), export["sheet"].as_str()), (Some("Book1"), Some("a")));
    let xlsx = ok(&headless(&mut s, "export_data", json!({"path": "out.xlsx"})));
    assert_eq!((xlsx["book"].as_str(), xlsx["sheet"].as_str()), (Some("Book1"), Some("a")));
}

// ----- with a history ----------------------------------------------------------------------

#[test]
fn each_mutating_call_is_one_named_undo_step_and_undo_restores_the_project() {
    let d = Dirs::new("steps");
    write_csv(&d, "a.csv");
    let mut c = Caller::with_history(&d);
    let empty = c.doc.project().clone();
    ok(&c.call("import_files", json!({"files": ["a.csv"]})));
    assert_eq!(c.doc.undo_label(), Some("AI: import_files a.csv"));
    let imported = c.doc.project().clone();
    ok(&c.call("plot", json!({"y": ["B"]})));
    assert_eq!(c.doc.undo_label(), Some("AI: plot Book1 B(Y)"));
    ok(&c.call("set_column_values", json!({"column": "C", "formula": "col(B)*2", "create": {}})));
    assert_eq!(c.doc.undo_label(), Some("AI: set_column_values Book1 C = col(B)*2"));
    assert!(c.doc.dirty);
    c.doc.undo();
    c.doc.undo();
    assert_eq!(c.doc.project(), &imported);
    c.doc.undo();
    assert_eq!(c.doc.project(), &empty);
}

#[test]
fn failing_panicking_and_empty_calls_leave_no_step_and_reads_leave_none() {
    let d = Dirs::new("nosteps");
    write_csv(&d, "a.csv");
    let mut c = Caller::with_history(&d);
    ok(&c.call("import_files", json!({"files": ["a.csv"]})));
    let revision = c.doc.revision();
    let before = c.doc.project().clone();
    // A failure.
    refused(&c.call("set_column_values", json!({"column": "B", "formula": "col(Nope)+"})));
    refused(&c.call("plot", json!({"y": ["Nope"]})));
    // A change that changes nothing: the data is sorted already.
    ok(&c.call("sort_sheet", json!({"key": "A"})));
    // Reads.
    ok(&c.call("list_contents", json!({})));
    ok(&c.call("read_columns", json!({})));
    ok(&c.call("export_data", json!({"path": "out.csv"})));
    assert_eq!(c.doc.project(), &before);
    assert_eq!(c.doc.revision(), revision);
    assert_eq!(c.doc.undo_label(), Some("AI: import_files a.csv"));
}

#[test]
fn the_call_cap_refuses_further_changes_but_not_reads() {
    let d = Dirs::new("cap");
    write_csv(&d, "a.csv");
    let mut c = Caller::with_history(&d);
    c.policy.change_cap = Some(3);
    ok(&c.call("import_files", json!({"files": ["a.csv"]})));
    refused(&c.call("plot", json!({"y": ["Nope"]})));
    ok(&c.call("sort_sheet", json!({"key": "B", "descending": true})));
    let e = refused(&c.call("sort_sheet", json!({"key": "B"})));
    assert!(e.starts_with("this connection has made 3 changes"), "{e}");
    ok(&c.call("list_contents", json!({})));
    assert_eq!(c.session.mutating_calls(), 3);
}

#[test]
fn attached_the_tools_that_open_replace_or_save_are_refused_and_hidden() {
    let d = Dirs::new("hidden");
    write_csv(&d, "a.csv");
    let mut c = Caller::attached(&d);
    ok(&c.call("import_files", json!({"files": ["a.csv"]})));
    let before = c.doc.project().clone();
    for (tool, args) in [
        ("new_project", json!({})),
        ("open_project", json!({"path": "a.sheaf"})),
        ("save_project", json!({"path": "a.opj"})),
    ] {
        let e = refused(&c.call(tool, args));
        assert_eq!(e, "not available while attached: the person using Sheaf opens and saves projects", "{tool}");
    }
    assert_eq!(c.doc.project(), &before);
    assert_eq!(c.doc.generation(), 0);
    assert_eq!(d.names(), ["a.csv"]);
    let all: Vec<String> = sheaf_mcp::tools::list().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    let shown: Vec<String> =
        sheaf_mcp::tools::list_attached().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    assert_eq!(all.len(), 16);
    assert_eq!(shown.len(), 13);
    for hidden in ["new_project", "open_project", "save_project"] {
        assert!(all.iter().any(|n| n == hidden) && !shown.iter().any(|n| n == hidden), "{hidden}");
    }
}

// ----- files written while attached --------------------------------------------------------

fn graph_caller(d: &Dirs) -> Caller {
    write_csv(d, "a.csv");
    let mut c = Caller::attached(d);
    ok(&c.call("import_files", json!({"files": ["a.csv"]})));
    ok(&c.call("plot", json!({"y": ["B"]})));
    c
}

#[test]
fn attached_exports_never_replace_a_file() {
    let d = Dirs::new("noreplace");
    let mut c = graph_caller(&d);
    for (tool, args) in [
        ("export_data", json!({"path": "out.csv"})),
        ("export_data", json!({"path": "out.xlsx"})),
        ("export_graph", json!({"graph": "Graph1", "path": "fig.png"})),
        ("export_graph", json!({"graph": "Graph1", "path": "fig.pdf"})),
    ] {
        let path = args["path"].as_str().unwrap().to_string();
        ok(&c.call(tool, args.clone()));
        let written = std::fs::read(d.root.join(&path)).unwrap();
        assert!(!written.is_empty(), "{path}");
        let e = refused(&c.call(tool, args));
        assert!(e.starts_with("will not overwrite: '"), "{path}: {e}");
        assert!(e.contains("choose a new file name"), "{e}");
        assert_eq!(std::fs::read(d.root.join(&path)).unwrap(), written, "{path} is as it was");
    }
    assert_eq!(d.names(), ["a.csv", "fig.pdf", "fig.png", "out.csv", "out.xlsx"], "no temporary file is left");
}

#[test]
fn an_attached_multi_page_pdf_never_replaces_a_file_either() {
    let d = Dirs::new("multipdf");
    let mut c = graph_caller(&d);
    ok(&c.call("plot", json!({"y": ["B"], "template": "scatter"})));
    ok(&c.call("export_graph", json!({"all_graphs": true, "path": "all.pdf"})));
    let first = std::fs::read(d.root.join("all.pdf")).unwrap();
    let e = refused(&c.call("export_graph", json!({"all_graphs": true, "path": "all.pdf"})));
    assert!(e.contains("will not overwrite"), "{e}");
    assert_eq!(std::fs::read(d.root.join("all.pdf")).unwrap(), first);
    assert!(!d.names().iter().any(|n| n.contains("tmp")), "{:?}", d.names());
}

#[test]
fn headless_exports_still_replace_a_file() {
    let d = Dirs::new("replace");
    write_csv(&d, "a.csv");
    let mut s = Server::new(d.roots());
    ok(&headless(&mut s, "import_files", json!({"files": ["a.csv"]})));
    std::fs::write(d.root.join("out.csv"), "old").unwrap();
    ok(&headless(&mut s, "export_data", json!({"path": "out.csv"})));
    assert_ne!(std::fs::read_to_string(d.root.join("out.csv")).unwrap(), "old");
    // The attach rules do not apply to a caller with a history either, when it is not attached.
    let mut c = Caller::with_history(&d);
    ok(&c.call("import_files", json!({"files": ["a.csv"]})));
    ok(&c.call("export_data", json!({"path": "out.csv"})));
}

#[test]
fn an_attached_fit_plot_is_checked_before_the_fit_and_written_once() {
    let d = Dirs::new("fitplot");
    write_csv(&d, "a.csv");
    let mut c = Caller::attached(&d);
    ok(&c.call("import_files", json!({"files": ["a.csv"]})));
    std::fs::write(d.root.join("fit.png"), "mine").unwrap();
    let before = c.doc.project().clone();
    let e = refused(&c.call("fit", json!({"model": "linear", "y": ["B"], "plot": "fit.png"})));
    assert!(e.contains("will not overwrite"), "{e}");
    assert_eq!(c.doc.project(), &before, "no fit was made");
    assert_eq!(std::fs::read_to_string(d.root.join("fit.png")).unwrap(), "mine");
    let done = ok(&c.call("fit", json!({"model": "linear", "y": ["B"], "plot": "fit2.png"})));
    assert!(done["image"].as_str().unwrap().ends_with("fit2.png"), "{done}");
    assert!(std::fs::metadata(d.root.join("fit2.png")).unwrap().len() > 100);
    assert_eq!(c.doc.undo_label(), Some("AI: fit linear Book1 B(Y)"));
}

#[test]
fn attached_exports_refuse_the_names_of_the_open_project() {
    let d = Dirs::new("protected");
    let mut c = graph_caller(&d);
    let open = OpenFiles {
        project: Some(d.root.join("work.opj")),
        origin: Some(d.root.join("original.opju")),
        recovered_from: None,
    };
    c.policy.protected = protected_set(&open, &[d.root.join("recovery")]);
    std::fs::create_dir(d.root.join("recovery")).unwrap();
    for name in [
        "work.opj",
        "work.opj.sheaf",
        "work.opj.bak",
        "original.opju.bak",
        "original.opju.sheaf",
        "planted.bak",
        "other.opj",
        "recovery/autosave.txt",
    ] {
        let e = refused(&c.call("export_data", json!({"path": name})));
        assert!(e.contains("belongs to the project open in Sheaf") || e.contains("kept for"), "{name}: {e}");
    }
    assert!(!d.root.join("planted.bak").exists());
    assert!(!d.root.join("work.opj.sheaf").exists());
    // A name that is not the project's is fine.
    ok(&c.call("export_data", json!({"path": "work.csv"})));
}

#[cfg(unix)]
#[test]
fn an_attached_export_never_follows_a_link() {
    let d = Dirs::new("links");
    let mut c = graph_caller(&d);
    let outside = d.base.join("outside.txt");
    std::fs::write(&outside, "keep").unwrap();
    // A link inside the root that points outside it is refused by the roots; one that points at a file
    // inside the root is refused as an existing name.
    std::os::unix::fs::symlink(d.root.join("a.csv"), d.root.join("alias.csv")).unwrap();
    std::os::unix::fs::symlink(d.root.join("not_yet.csv"), d.root.join("dangling.csv")).unwrap();
    for name in ["alias.csv", "dangling.csv"] {
        refused(&c.call("export_data", json!({"path": name})));
    }
    assert!(!d.root.join("not_yet.csv").exists());
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "keep");
    assert_eq!(std::fs::read_to_string(d.root.join("a.csv")).unwrap().lines().next(), Some("x,y"));
}

// ----- one result in both modes ------------------------------------------------------------

/// The same script, through the headless server, through the executor on a document with a history, and
/// attached (new files only, written through a temporary name), gives the same replies and the same
/// project. The attached run may differ in nothing the caller can read here.
#[test]
fn the_headless_server_and_the_executor_give_the_same_results() {
    let d = Dirs::new("parity");
    write_csv(&d, "a.csv");
    let script: Vec<(&str, Value)> = vec![
        ("import_files", json!({"files": ["a.csv"]})),
        ("list_contents", json!({})),
        ("set_column_values", json!({"column": "C", "formula": "col(B)*2", "create": {}})),
        ("read_columns", json!({"columns": ["A", "C"], "from": 3, "limit": 12})),
        ("plot", json!({"y": ["B"], "template": "scatter"})),
        ("analyze", json!({"tool": "statistics", "columns": ["B"]})),
        ("analyze", json!({"tool": "integrate", "columns": ["B"]})),
        ("sort_sheet", json!({"key": "B", "descending": true})),
        ("filter_sheet", json!({"condition": "col(B)>11"})),
        ("fit", json!({"model": "linear", "y": ["B"], "plot": "fit.png"})),
        ("export_data", json!({"path": "out.csv"})),
        ("export_data", json!({"path": "out", "format": "xlsx"})),
        ("export_data", json!({"path": "out2.xlsx", "all_sheets": true})),
        ("export_graph", json!({"graph": "Graph1", "path": "fig.png", "return_image": false})),
        ("list_contents", json!({})),
        ("read_columns", json!({"columns": ["B"], "from": 2, "limit": 5})),
    ];
    let clean = |d: &Dirs| {
        for f in ["fit.png", "out.csv", "out.xlsx", "out2.xlsx", "fig.png"] {
            let _ = std::fs::remove_file(d.root.join(f));
        }
    };
    let mut s = Server::new(d.roots());
    let first: Vec<Value> = script.iter().map(|(t, a)| headless(&mut s, t, a.clone())).collect();
    clean(&d);
    let mut c = Caller::with_history(&d);
    // The server's reply went through text; so does the caller's (the parser rounds the last digit of
    // some numbers, which must not count as a difference).
    let second: Vec<Value> =
        script.iter().map(|(t, a)| serde_json::from_str(&c.call(t, a.clone()).to_string()).unwrap()).collect();
    for (i, ((tool, _), (a, b))) in script.iter().zip(first.iter().zip(&second)).enumerate() {
        assert_eq!(a["isError"], false, "step {i} {tool}: {a}");
        assert_eq!(a, b, "step {i} {tool}");
    }
    assert_eq!(s.project(), c.doc.project());
    assert!(d.root.join("out.xlsx").exists(), "an extension-less xlsx export is written as .xlsx");

    clean(&d);
    let mut a = Caller::attached(&d);
    let third: Vec<Value> =
        script.iter().map(|(t, a2)| serde_json::from_str(&a.call(t, a2.clone()).to_string()).unwrap()).collect();
    for (i, ((tool, _), (x, y))) in script.iter().zip(first.iter().zip(&third)).enumerate() {
        assert_eq!(x, y, "attached, step {i} {tool}");
    }
    assert_eq!(s.project(), a.doc.project());
    assert_eq!(d.names().iter().filter(|n| n.contains("tmp")).count(), 0, "{:?}", d.names());
}

#[test]
fn an_extension_less_xlsx_export_reports_the_file_that_was_written() {
    let d = Dirs::new("xlsxname");
    write_csv(&d, "a.csv");
    for attached in [false, true] {
        let name = if attached { "att" } else { "head" };
        let mut c = if attached { Caller::attached(&d) } else { Caller::with_history(&d) };
        ok(&c.call("import_files", json!({"files": ["a.csv"]})));
        let out = ok(&c.call("export_data", json!({"path": name, "format": "xlsx"})));
        let reported = PathBuf::from(out["path"].as_str().unwrap());
        assert_eq!(reported, d.root.join(format!("{name}.xlsx")), "attached: {attached}");
        assert!(reported.exists());
        // The name the writer would have made is checked like any other: it exists now.
        if attached {
            let e = refused(&c.call("export_data", json!({"path": name, "format": "xlsx"})));
            assert!(e.contains("will not overwrite"), "{e}");
        }
    }
    assert!(!d.root.join("head").exists() && !d.root.join("att").exists());
}
