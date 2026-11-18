//! Starts the real binary with pipes and runs a session: initialize, list the tools, then import,
//! plot, analyze, fit, export and save on a synthetic file. The numbers are compared with
//! `sheaf-ops` called directly, which is what the command line calls too.

use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let p = std::env::temp_dir().join(format!("sheaf-mcp-it-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Dir(p)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Sends the lines, closes stdin and returns the replies by id plus how many lines came back.
fn session(root: &Path, lines: &[Value]) -> (Vec<Value>, std::process::Output) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sheaf-mcp"))
        .arg("--root")
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        for l in lines {
            writeln!(stdin, "{l}").unwrap();
        }
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    let replies = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap_or_else(|e| panic!("stdout must be protocol only: {l:?} ({e})"))
        })
        .collect();
    (replies, out)
}

fn call(id: u64, name: &str, args: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": args}})
}

fn data(rows: usize) -> String {
    let mut s = String::from("x,y\n");
    for i in 0..rows {
        let x = i as f64 * 0.1;
        s.push_str(&format!("{x},{}\n", 2.0 * x + 1.0 + 0.01 * (7.0 * i as f64).sin()));
    }
    s
}

fn reply_for(replies: &[Value], id: u64) -> &Value {
    replies.iter().find(|r| r["id"] == id).unwrap_or_else(|| panic!("no reply for id {id}"))
}

fn text_of(r: &Value) -> Value {
    serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn a_full_session_over_pipes() {
    let dir = Dir::new("session");
    std::fs::write(dir.0.join("scan.csv"), data(60)).unwrap();
    let requests = vec![
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        call(3, "import_files", json!({"files": ["scan.csv"]})),
        call(4, "list_contents", json!({})),
        call(5, "plot", json!({"template": "line", "style": {"x_title": "t / s", "y_title": "I / A"}})),
        call(6, "export_graph", json!({"path": "fig.png", "dpi": 100, "return_image": true})),
        call(7, "analyze", json!({"tool": "statistics", "columns": ["B"]})),
        call(8, "fit", json!({"model": "linear", "y": ["B"]})),
        call(9, "export_data", json!({"path": "book1.csv"})),
        call(10, "save_project", json!({"path": "out.sheaf"})),
        call(11, "read_columns", json!({"columns": ["A", "B"], "from": 1, "to": 3})),
        call(12, "import_files", json!({"files": ["../elsewhere.csv"]})),
        json!({"jsonrpc": "2.0", "id": 13, "method": "resources/read", "params": {"uri": "sheaf://graph/Graph1.png"}}),
        json!({"jsonrpc": "2.0", "id": 14, "method": "nope"}),
    ];
    let (replies, out) = session(&dir.0, &requests);
    assert!(out.status.success(), "exit code {:?}", out.status.code());
    // Notifications get no reply: 14 requests have ids, 1 notification does not.
    assert_eq!(replies.len(), 14);

    let init = reply_for(&replies, 1);
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "sheaf-mcp");

    let names: Vec<&str> = reply_for(&replies, 2)["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for want in [
        "new_project",
        "open_project",
        "save_project",
        "import_files",
        "list_contents",
        "read_columns",
        "set_column_values",
        "plot",
        "export_graph",
        "export_data",
        "list_analysis_tools",
        "analyze",
        "sort_sheet",
        "filter_sheet",
        "list_fit_models",
        "fit",
        "list_simulate_functions",
        "simulate_curve",
        "simulate_surface",
    ] {
        assert!(names.contains(&want), "missing tool {want}");
    }

    let imported = text_of(reply_for(&replies, 3));
    assert_eq!(imported["imported"][0]["book"], "Book1");
    assert_eq!(imported["errors"], json!([]));
    assert_eq!(reply_for(&replies, 3)["result"]["structuredContent"], imported);

    let contents = text_of(reply_for(&replies, 4));
    let cols = &contents["workbooks"][0]["sheets"][0]["columns"];
    assert_eq!(cols.as_array().unwrap().len(), 2);
    assert_eq!(cols[0]["designation"], "X");

    let plot = text_of(reply_for(&replies, 5));
    assert_eq!(plot["graph"], "Graph1");
    assert_eq!(plot["created"], true);

    // The figure: a file on disk and a PNG image block of the same picture.
    let fig = reply_for(&replies, 6);
    assert_eq!(fig["result"]["isError"], false, "{fig}");
    let image = &fig["result"]["content"][1];
    assert_eq!(image["type"], "image");
    assert_eq!(image["mimeType"], "image/png");
    assert!(image["data"].as_str().unwrap().starts_with("iVBORw0KGgo"));
    let file = std::fs::read(dir.0.join("fig.png")).unwrap();
    assert_eq!(&file[..4], b"\x89PNG");
    assert_eq!(text_of(fig)["bytes"], file.len());

    // The same numbers as the library gives for the same file and options.
    let mut project = sheaf_core::Project::new();
    let request = sheaf_ops::import::ImportRequest {
        files: vec![dir.0.join("scan.csv")],
        ..sheaf_ops::import::ImportRequest::default()
    };
    sheaf_ops::import::import_files(&mut project, &request).unwrap();
    let spec = sheaf_ops::fit::FitSpec { y: vec!["B".into()], ..sheaf_ops::fit::FitSpec::default() };
    let prepared = sheaf_ops::fit::prepare(&spec).unwrap();
    let direct = sheaf_ops::fit::run(&mut project, &spec, &prepared).unwrap();
    let sheaf_ops::fit::FitRun::Fits { outcomes, .. } = direct.run else { panic!() };
    let want = &outcomes[0].report;

    let fit = text_of(reply_for(&replies, 8));
    let got = &fit["outcomes"][0];
    assert_eq!(got["model"], json!(want.model));
    for (g, w) in got["parameters"].as_array().unwrap().iter().zip(&want.params) {
        assert_eq!(g["name"], json!(w.name));
        // The text form of a number may differ from the f64 in the last digit.
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1e-300);
        assert!(close(g["value"].as_f64().unwrap(), w.value), "{g} vs {}", w.value);
        assert!(close(g["stderr"].as_f64().unwrap(), w.std_error), "{g} vs {}", w.std_error);
    }
    let slope = got["parameters"][1]["value"].as_f64().unwrap();
    assert!((slope - 2.0).abs() < 0.01, "slope {slope}");
    assert!(got["statistics"]["r2"].as_f64().unwrap() > 0.999);

    let stats = text_of(reply_for(&replies, 7));
    let mean_col = stats["table"]["headers"].as_array().unwrap().iter().position(|h| h == "Mean").unwrap();
    let ys: Vec<f64> = (0..60).map(|i| 2.0 * (i as f64 * 0.1) + 1.0 + 0.01 * (7.0 * i as f64).sin()).collect();
    let mean = ys.iter().sum::<f64>() / ys.len() as f64;
    assert!((stats["table"]["rows"][0]["values"][mean_col].as_f64().unwrap() - mean).abs() < 1e-9);

    let exported = std::fs::read_to_string(dir.0.join("book1.csv")).unwrap();
    assert!(exported.lines().count() >= 61, "{exported}");
    assert_eq!(text_of(reply_for(&replies, 9))["rows"], 60);

    let saved = text_of(reply_for(&replies, 10));
    assert!(dir.0.join("out.sheaf").exists());
    assert!(saved["path"].as_str().unwrap().ends_with("out.sheaf"));

    let read = text_of(reply_for(&replies, 11));
    assert_eq!(read["rows"].as_array().unwrap().len(), 3);
    assert_eq!(read["rows"][1][0], 0.1);
    assert_eq!(read["truncated"], false);

    let outside = reply_for(&replies, 12);
    assert_eq!(outside["result"]["isError"], true);
    assert!(
        outside["result"]["content"][0]["text"].as_str().unwrap().starts_with("path is outside the allowed folders")
    );

    let resource = reply_for(&replies, 13);
    assert_eq!(resource["result"]["contents"][0]["mimeType"], "image/png");
    assert!(resource["result"]["contents"][0]["blob"].as_str().unwrap().starts_with("iVBORw0KGgo"));

    assert_eq!(reply_for(&replies, 14)["error"]["code"], -32601);
}

#[test]
fn a_saved_project_opens_again() {
    let dir = Dir::new("reopen");
    std::fs::write(dir.0.join("scan.csv"), data(20)).unwrap();
    let (first, _) = session(
        &dir.0,
        &[call(1, "import_files", json!({"files": ["scan.csv"]})), call(2, "save_project", json!({"path": "p.sheaf"}))],
    );
    assert_eq!(reply_for(&first, 2)["result"]["isError"], false);
    let (second, _) = session(&dir.0, &[call(1, "open_project", json!({"path": "p.sheaf"}))]);
    let opened = text_of(reply_for(&second, 1));
    assert_eq!(opened["foreign"], false);
    assert_eq!(opened["summary"]["workbooks"][0]["sheets"][0]["rows"], 20);
}

#[test]
fn malformed_input_gets_an_error_and_the_server_goes_on() {
    let dir = Dir::new("bad");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sheaf-mcp"))
        .arg("--root")
        .arg(&dir.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"{not json\n\n").unwrap();
        stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"ping\"}\n").unwrap();
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    let lines: Vec<Value> =
        String::from_utf8_lossy(&out.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["error"]["code"], -32700);
    assert_eq!(lines[1]["result"], json!({}));
}

#[test]
fn version_and_bad_roots() {
    let v = Command::new(env!("CARGO_BIN_EXE_sheaf-mcp")).arg("--version").output().unwrap();
    assert!(String::from_utf8_lossy(&v.stdout).starts_with("sheaf-mcp "));
    let bad = Command::new(env!("CARGO_BIN_EXE_sheaf-mcp")).args(["--root", "no/such/folder"]).output().unwrap();
    assert!(!bad.status.success());
    assert!(bad.stdout.is_empty());
}
