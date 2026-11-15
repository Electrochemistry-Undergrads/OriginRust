//! `simulate_curve` and `list_simulate_functions`: the numbers are those of the shared operation behind
//! `sheaf simulate`, a request that fails changes nothing, and the new sheet is one undo step of the session.

use serde_json::{Value, json};
use sheaf_mcp::Server;
use sheaf_mcp::roots::Roots;
use sheaf_ops::simulate::{SimulateSpec, run_standalone};
use std::path::PathBuf;

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let p = std::env::temp_dir().join(format!("sheaf-mcp-simulate-{tag}-{}", std::process::id()));
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

fn server(dir: &Dir) -> Server {
    Server::new(Roots::new(std::slice::from_ref(&dir.0)).unwrap())
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

fn numbers(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|n| n.as_f64().unwrap()).collect()
}

/// Equal up to the last digits: the JSON reader of the tests does not give every double back exactly.
fn close(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() <= 1e-14 * y.abs().max(1.0), "{x} against {y}");
    }
}

fn gauss(x: f64, y0: f64, xc: f64, w: f64, a: f64) -> f64 {
    y0 + a / (w * (std::f64::consts::PI / 2.0).sqrt()) * (-2.0 * ((x - xc) / w).powi(2)).exp()
}

#[test]
fn a_simulated_gauss_has_the_numbers_of_the_shared_operation_and_of_its_formula() {
    let dir = Dir::new("gauss");
    let mut s = server(&dir);
    let r = payload(&call(
        &mut s,
        "simulate_curve",
        json!({"function": "gauss", "params": {"y0": 1, "xc": 5, "w": 2, "A": 4}, "from": 0, "to": 10, "points": 21}),
    ));
    assert_eq!(r["sheet"], "Simulate1");
    assert_eq!(r["function"], "Gauss");
    assert_eq!(r["points"], 21);
    assert_eq!(r["graph"], Value::Null);
    assert_eq!(r["x_column"], "Book1:Simulate1:A");
    assert_eq!(r["y_column"], "Book1:Simulate1:B");
    let (x, y) = (numbers(&r["x"]), numbers(&r["y"]));
    assert_eq!((x[0], x[10], x[20]), (0.0, 5.0, 10.0));
    for (xi, yi) in x.iter().zip(&y) {
        let want = gauss(*xi, 1.0, 5.0, 2.0, 4.0);
        assert!((yi - want).abs() <= 1e-12 * want.abs().max(1.0), "x={xi}: {yi} against {want}");
    }
    let mut spec = SimulateSpec::new("Gauss");
    spec.params = ["y0=1", "xc=5", "w=2", "A=4"].map(String::from).to_vec();
    spec.from = Some(0.0);
    spec.to = Some(10.0);
    spec.points = Some(21);
    let (project, run) = run_standalone(&spec).unwrap();
    let ws = &project.workbook(run.book).unwrap().sheets[run.result_sheet];
    close(&y, ws.columns[1].numeric().unwrap());
    // The sheet is in the session: the numbers read back.
    let cols = payload(&call(&mut s, "read_columns", json!({"sheet": "Simulate1", "columns": ["B"]})));
    let read: Vec<f64> = cols["rows"].as_array().unwrap().iter().map(|r| r[0].as_f64().unwrap()).collect();
    close(&read, &y);
}

#[test]
fn the_defaults_log_spacing_and_the_x_of_a_column() {
    let dir = Dir::new("x");
    std::fs::write(dir.0.join("data.csv"), "T,Signal\n0.5,1\n1.5,2\n2.5,3\n").unwrap();
    let mut s = server(&dir);
    let r = payload(&call(&mut s, "simulate_curve", json!({})));
    let x = numbers(&r["x"]);
    // Origin's window for Gauss: 200 points from -1 to 5 (fitting.md 13.1).
    assert_eq!((x.len(), x[0], x[199]), (200, -1.0, 5.0));

    let r = payload(&call(
        &mut s,
        "simulate_curve",
        json!({"function": "ExpDec1", "log": true, "from": 1, "to": 100, "points": 3}),
    ));
    close(&numbers(&r["x"]), &[1.0, 10.0, 100.0]);
    assert_eq!(r["sheet"], "Simulate2");

    assert_eq!(call(&mut s, "import_files", json!({"files": ["data.csv"]}))["isError"], false);
    let r = payload(&call(&mut s, "simulate_curve", json!({"book": "Book2", "x_of": "T", "params": {"xc": 1.5}})));
    assert_eq!(numbers(&r["x"]), [0.5, 1.5, 2.5]);
    let y = numbers(&r["y"]);
    assert_eq!(y[0], y[2]);
    assert!(y[1] > y[0]);
    assert_eq!(r["book"], "Book2");

    for extra in [json!({"from": 0}), json!({"to": 5}), json!({"points": 50}), json!({"log": true})] {
        let mut args = json!({"book": "Book2", "x_of": "T"});
        args.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let r = call(&mut s, "simulate_curve", args.clone());
        assert_eq!(r["isError"], true, "{args}");
        assert!(text(&r).contains("cannot be combined"), "{args}: {}", text(&r));
    }
}

#[test]
fn plot_and_image_make_a_graph_and_a_picture() {
    let dir = Dir::new("plot");
    let mut s = server(&dir);
    let r = payload(&call(&mut s, "simulate_curve", json!({"function": "Lorentz", "plot": true})));
    assert_eq!(r["graph"], "Graph1");
    assert!(r["message"].as_str().unwrap().contains("graph Graph1"), "{r}");
    let r =
        payload(&call(&mut s, "simulate_curve", json!({"function": "Lorentz", "image": "curve.png", "values": false})));
    assert_eq!(r["graph"], "Graph2");
    assert!(r.get("x").is_none());
    assert!(std::fs::metadata(dir.0.join("curve.png")).unwrap().len() > 100);
    // A path outside the allowed folders is refused before anything is added.
    let refused = call(&mut s, "simulate_curve", json!({"image": "../outside.png"}));
    assert_eq!(refused["isError"], true, "{refused}");
    let contents = payload(&call(&mut s, "list_contents", json!({})));
    assert_eq!(contents["workbooks"].as_array().unwrap().len(), 1, "{contents}");
}

#[test]
fn a_bad_request_is_an_error_and_changes_nothing() {
    let dir = Dir::new("bad");
    let mut s = server(&dir);
    for (args, want) in [
        (json!({"function": "NoSuchFunction"}), "no built-in fit function"),
        (json!({"params": {"zz": 1}}), "no parameter 'zz'"),
        (json!({"points": 1}), "too few"),
        (json!({"from": 3, "to": 3}), "two different values"),
        (json!({"log": true, "from": -1, "to": 4}), "positive"),
        (json!({"x_of": "A"}), "needs a project"),
        (json!({"sheet": "Sheet1"}), "needs no sheet"),
        (json!({"oops": 1}), "oops"),
    ] {
        let r = call(&mut s, "simulate_curve", args.clone());
        assert_eq!(r["isError"], true, "{args}");
        assert!(text(&r).contains(want), "{args}: {}", text(&r));
    }
    let contents = payload(&call(&mut s, "list_contents", json!({})));
    assert_eq!(contents["workbooks"], json!([]), "{contents}");
}

#[test]
fn the_list_has_the_functions_with_their_starting_values() {
    let dir = Dir::new("list");
    let mut s = server(&dir);
    let all = payload(&call(&mut s, "list_simulate_functions", json!({})));
    let functions = all["functions"].as_array().unwrap();
    assert!(functions.len() > 100);
    let g = functions.iter().find(|f| f["name"] == "Gauss").unwrap();
    let names: Vec<&str> = g["parameters"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["y0", "xc", "w", "A"]);
    assert!(g["parameters"][0]["start"].is_number());
    let peaks = payload(&call(&mut s, "list_simulate_functions", json!({"category": "peak functions"})));
    assert!(peaks["functions"].as_array().unwrap().iter().all(|f| f["category"] == "Peak Functions"));
    assert!(peaks["functions"].as_array().unwrap().len() < functions.len());
}
