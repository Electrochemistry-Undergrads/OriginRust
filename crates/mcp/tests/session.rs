//! A voltammetry-style session over pipes: a synthetic peak file is imported, paged through,
//! plotted, integrated, fitted with a Gauss peak, exported as PNG and saved as `.opj`. The fit is
//! compared with `sheaf-ops` (the path the command line also calls) and with the known peak, the
//! integral with a trapezoid sum. Only synthetic data is used.

use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let p = std::env::temp_dir().join(format!("sheaf-mcp-sess-{tag}-{}", std::process::id()));
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

fn run(root: &Path, lines: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sheaf-mcp"))
        .arg("--root")
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
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
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn call(id: u64, name: &str, args: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": args}})
}

fn reply(replies: &[Value], id: u64) -> &Value {
    replies.iter().find(|r| r["id"] == id).unwrap_or_else(|| panic!("no reply for id {id}"))
}

fn payload(r: &Value) -> Value {
    assert_eq!(r["result"]["isError"], false, "{r}");
    serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

const N: usize = 201;
const AREA: f64 = 0.35;
const FWHM: f64 = 0.12;

/// Potential from 0 to 1 V, one Gaussian peak at 0.5 V on a small offset.
fn peak(e: f64) -> f64 {
    let sigma = FWHM / 2.354_820_045;
    0.02 + AREA / (sigma * (2.0 * std::f64::consts::PI).sqrt()) * (-(e - 0.5).powi(2) / (2.0 * sigma * sigma)).exp()
}

fn csv() -> String {
    let mut s = String::from("E / V,I / uA\n");
    for i in 0..N {
        let e = i as f64 / (N - 1) as f64;
        s.push_str(&format!("{e},{}\n", peak(e)));
    }
    s
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1e-12)
}

fn numbers(v: &Value, out: &mut Vec<f64>) {
    match v {
        Value::Number(n) => out.push(n.as_f64().unwrap()),
        Value::Array(a) => a.iter().for_each(|x| numbers(x, out)),
        Value::Object(o) => o.values().for_each(|x| numbers(x, out)),
        _ => {}
    }
}

#[test]
fn import_to_save_session() {
    let dir = Dir::new("main");
    std::fs::write(dir.0.join("cv.csv"), csv()).unwrap();

    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}});
    let r = run(
        &dir.0,
        &[
            init,
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
            call(3, "import_files", json!({"files": ["cv.csv"]})),
            call(4, "list_contents", json!({})),
            call(5, "read_columns", json!({"limit": 200})),
            call(6, "read_columns", json!({"from": 101, "to": 201})),
            call(7, "plot", json!({"template": "line"})),
            call(8, "analyze", json!({"tool": "integrate", "columns": ["B"]})),
            call(9, "fit", json!({"model": "Gauss", "y": ["B"]})),
            call(10, "export_graph", json!({"path": "fig.png", "dpi": 80, "return_image": true})),
            call(11, "save_project", json!({"path": "session.opj"})),
            call(12, "import_files", json!({"files": ["../outside.csv"]})),
            call(13, "open_project", json!({"path": "session.opj"})),
        ],
    );

    assert_eq!(reply(&r, 1)["result"]["protocolVersion"], "2025-06-18");
    assert!(reply(&r, 2)["result"]["tools"].as_array().unwrap().len() >= 16);
    assert_eq!(payload(reply(&r, 3))["errors"], json!([]));

    let contents = payload(reply(&r, 4));
    assert_eq!(contents["workbooks"][0]["sheets"][0]["columns"].as_array().unwrap().len(), 2);

    // Paged read: 200 cells are 100 rows, so the first page stops and says where to go on.
    let first = payload(reply(&r, 5));
    assert_eq!(first["total_rows"], N);
    assert_eq!(first["truncated"], true);
    assert_eq!(first["rows"].as_array().unwrap().len(), 100);
    let next = &first["next"];
    assert_eq!(next["from"], 101);
    let second = payload(reply(&r, 6));
    assert_eq!(second["rows"].as_array().unwrap().len(), 101);
    assert_eq!(second["truncated"], false);
    assert!(close(second["rows"][100][0].as_f64().unwrap(), 1.0));
    assert!(close(second["rows"][0][1].as_f64().unwrap(), peak(100.0 / 200.0)));

    assert_eq!(payload(reply(&r, 7))["graph"], "Graph1");

    // The integral is in the reply as a number close to the trapezoid sum.
    let ys: Vec<f64> = (0..N).map(|i| peak(i as f64 / (N - 1) as f64)).collect();
    let trapezoid: f64 = ys.windows(2).map(|w| 0.5 * (w[0] + w[1]) / (N - 1) as f64).sum();
    let mut found = Vec::new();
    numbers(&payload(reply(&r, 8)), &mut found);
    assert!(found.iter().any(|v| (v - trapezoid).abs() < 1e-9), "{trapezoid} not in {found:?}");

    // The same fit through sheaf-ops, which the command line also calls.
    let mut project = sheaf_core::Project::new();
    let request = sheaf_ops::import::ImportRequest {
        files: vec![dir.0.join("cv.csv")],
        ..sheaf_ops::import::ImportRequest::default()
    };
    sheaf_ops::import::import_files(&mut project, &request).unwrap();
    let spec =
        sheaf_ops::fit::FitSpec { model: "Gauss".into(), y: vec!["B".into()], ..sheaf_ops::fit::FitSpec::default() };
    let prepared = sheaf_ops::fit::prepare(&spec).unwrap();
    let direct = sheaf_ops::fit::run(&mut project, &spec, &prepared).unwrap();
    let sheaf_ops::fit::FitRun::Fits { outcomes, .. } = direct.run else { panic!() };
    let want = &outcomes[0].report;
    let fit = payload(reply(&r, 9));
    let params = fit["outcomes"][0]["parameters"].as_array().unwrap();
    assert_eq!(params.len(), want.params.len());
    for (g, w) in params.iter().zip(&want.params) {
        assert_eq!(g["name"], json!(w.name));
        assert!(close(g["value"].as_f64().unwrap(), w.value), "{g} vs {}", w.value);
    }
    let value_of = |name: &str| params.iter().find(|p| p["name"] == name).unwrap()["value"].as_f64().unwrap();
    assert!((value_of("xc") - 0.5).abs() < 1e-3);
    assert!((value_of("y0") - 0.02).abs() < 1e-3);
    assert!((value_of("A") - AREA).abs() < 5e-3);

    // The figure: an image block that decodes to a PNG.
    let fig = reply(&r, 10);
    assert_eq!(fig["result"]["isError"], false, "{fig}");
    let image = &fig["result"]["content"][1];
    assert_eq!(image["mimeType"], "image/png");
    let bytes = decode_base64(image["data"].as_str().unwrap());
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert!(dir.0.join("fig.png").exists());

    // Saved as .opj and opened again.
    assert!(payload(reply(&r, 11))["path"].as_str().unwrap().ends_with("session.opj"));
    assert!(dir.0.join("session.opj").metadata().unwrap().len() > 0);

    // A path outside the root is refused.
    let outside = reply(&r, 12);
    assert_eq!(outside["result"]["isError"], true);
    assert!(outside["result"]["content"][0]["text"].as_str().unwrap().contains("outside the allowed folders"));

    let opened = payload(reply(&r, 13));
    assert_eq!(opened["summary"]["workbooks"][0]["sheets"][0]["rows"], N);
}

#[test]
fn files_outside_the_root_are_refused_everywhere() {
    let dir = Dir::new("roots");
    let other = Dir::new("roots-other");
    std::fs::write(other.0.join("secret.csv"), csv()).unwrap();
    let abs = other.0.join("secret.csv").display().to_string();
    let r = run(
        &dir.0,
        &[
            call(1, "import_files", json!({"files": [abs]})),
            call(2, "open_project", json!({"path": other.0.join("p.opj").display().to_string()})),
            call(3, "save_project", json!({"path": other.0.join("p.sheaf").display().to_string()})),
            call(4, "export_data", json!({"path": "../x.csv"})),
        ],
    );
    for id in 1..=4 {
        assert_eq!(reply(&r, id)["result"]["isError"], true, "{}", reply(&r, id));
    }
    assert!(!other.0.join("p.sheaf").exists());
}

fn decode_base64(s: &str) -> Vec<u8> {
    let val = |c: u8| -> u32 {
        (match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("bad base64"),
        }) as u32
    };
    let data: Vec<u8> = s.bytes().filter(|c| *c != b'=').collect();
    let mut out = Vec::new();
    for chunk in data.chunks(4) {
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            n |= val(*c) << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    out
}
