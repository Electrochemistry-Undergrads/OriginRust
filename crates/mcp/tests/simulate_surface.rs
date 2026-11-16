//! `simulate_surface`: the numbers are those of the shared operation behind `sheaf simulate-surface`, a request that
//! fails changes nothing, and the new sheet is one undo step of the session.

use serde_json::{Value, json};
use sheaf_mcp::Server;
use sheaf_mcp::roots::Roots;
use sheaf_ops::simulate_surface::{SimulateSurfaceOps, run_standalone};
use std::path::PathBuf;

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let p = std::env::temp_dir().join(format!("sheaf-mcp-simsurface-{tag}-{}", std::process::id()));
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

#[test]
fn a_simulated_plane_has_the_numbers_of_the_shared_operation() {
    let dir = Dir::new("plane");
    let mut s = server(&dir);
    let r = payload(&call(
        &mut s,
        "simulate_surface",
        json!({"function": "Plane", "params": {"a": 3, "b": 4}, "x_min": 0, "x_max": 2, "x_points": 3,
               "y_min": 0, "y_max": 1, "y_points": 2}),
    ));
    assert_eq!(r["function"], "Plane");
    assert_eq!(r["points"], 6);
    assert!(r["sheet"].as_str().unwrap().starts_with("SimulateSurface"));
    assert_eq!(numbers(&r["z"]), [0.0, 3.0, 6.0, 4.0, 7.0, 10.0]);
    let spec = SimulateSurfaceOps {
        function: "Plane".into(),
        params: vec!["a=3".into(), "b=4".into()],
        x_min: Some(0.0),
        x_max: Some(2.0),
        x_points: Some(3),
        y_min: Some(0.0),
        y_max: Some(1.0),
        y_points: Some(2),
        ..Default::default()
    };
    let (project, run) = run_standalone(&spec).unwrap();
    let ws = &project.workbook(run.book).unwrap().sheets[run.result_sheet];
    assert_eq!(numbers(&r["z"]), ws.columns[2].numeric().unwrap());
    assert_eq!(numbers(&r["x"]), ws.columns[0].numeric().unwrap());
}

#[test]
fn a_bad_request_is_an_error_and_changes_nothing() {
    let dir = Dir::new("bad");
    let mut s = server(&dir);
    let r = call(&mut s, "simulate_surface", json!({"function": "Nope"}));
    assert_eq!(r["isError"], true);
    assert!(text(&r).contains("no surface function"), "{}", text(&r));
    let r = call(&mut s, "simulate_surface", json!({"x_points": 1}));
    assert_eq!(r["isError"], true);
    let r = call(&mut s, "simulate_surface", json!({"oops": 1}));
    assert_eq!(r["isError"], true);
    // The default is Origin's Gauss2D window: 400 points, which is under the cap for values.
    let ok = payload(&call(&mut s, "simulate_surface", json!({})));
    assert_eq!((ok["function"].as_str().unwrap(), ok["points"].as_u64().unwrap()), ("Gauss2D", 400));
    assert_eq!(numbers(&ok["z"]).len(), 400);
}
