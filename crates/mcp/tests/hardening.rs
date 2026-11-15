//! Defects found in a second implementation of this server, checked against this one: a column
//! name that is another column's long name, links inside the root that point outside it, and names
//! that two workbooks or columns share.

use serde_json::{Value, json};
use sheaf_mcp::Server;
use sheaf_mcp::roots::Roots;
use std::path::PathBuf;

struct Dirs {
    base: PathBuf,
    root: PathBuf,
    #[cfg(unix)]
    outside: PathBuf,
}

impl Dirs {
    fn new(tag: &str) -> Dirs {
        let base = std::env::temp_dir().join(format!("sheaf-mcp-hard-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let base = {
            std::fs::create_dir_all(&base).unwrap();
            base.canonicalize().unwrap()
        };
        let root = base.join("root");
        std::fs::create_dir_all(&root).unwrap();
        #[cfg(unix)]
        let outside = {
            let path = base.join("outside");
            std::fs::create_dir_all(&path).unwrap();
            path
        };
        Dirs {
            base,
            root,
            #[cfg(unix)]
            outside,
        }
    }

    fn server(&self) -> Server {
        Server::new(Roots::new(std::slice::from_ref(&self.root)).unwrap())
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

fn import(s: &mut Server, d: &Dirs, name: &str, csv: &str) -> Value {
    std::fs::write(d.root.join(name), csv).unwrap();
    ok(&call(s, "import_files", json!({"files": [name]})))
}

const CSV: &str = "Time,Current\ns,A\n0,1\n1,2\n2,3\n3,4\n";

#[test]
fn creating_a_column_named_like_the_long_name_of_another_does_not_overwrite_it() {
    let d = Dirs::new("create");
    let mut s = d.server();
    import(&mut s, &d, "a.csv", "A,B\n0,1\n1,2\n2,3\n");
    // Give the X column a long name, then "create" a column with that name.
    ok(&call(
        &mut s,
        "set_column_values",
        json!({"column": "C", "formula": "col(A)", "create": {"long_name": "Time"}}),
    ));
    let before = ok(&call(&mut s, "read_columns", json!({})));
    let r = call(&mut s, "set_column_values", json!({"column": "Time", "formula": "99", "create": {}}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(text(&r).contains("long name of column C"), "{}", text(&r));
    let after = ok(&call(&mut s, "read_columns", json!({})));
    assert_eq!(before["rows"], after["rows"], "no column changed");
    assert_eq!(before["columns"], after["columns"], "no column was added");
}

#[test]
fn a_notification_with_a_bad_envelope_gets_no_reply() {
    let d = Dirs::new("notify");
    let mut s = d.server();
    for line in [
        r#"{"jsonrpc":"1.0","method":"ping"}"#,
        r#"{"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","method":"tools/call","params":5}"#,
    ] {
        assert!(s.handle_line(line).is_none(), "{line}");
    }
    // With an id the same mistakes are answered.
    let r = s.handle_line(r#"{"jsonrpc":"1.0","id":4,"method":"ping"}"#).unwrap();
    assert!(r.contains("-32600"), "{r}");
}

#[cfg(unix)]
mod links {
    use super::*;
    use std::os::unix::fs::symlink;

    /// An opened `.opj` is an Origin file, so saving over it makes a `.bak` copy first.
    fn opened_opj(s: &mut Server, d: &Dirs) -> PathBuf {
        import(s, d, "a.csv", CSV);
        ok(&call(s, "save_project", json!({"path": "p.opj"})));
        let opened = ok(&call(s, "open_project", json!({"path": "p.opj"})));
        assert_eq!(opened["foreign"], true, "{opened}");
        d.root.join("p.opj")
    }

    #[test]
    fn save_never_writes_through_a_link_beside_the_target() {
        for (suffix, points_outside) in [(".bak", true), (".sheaf", true), (".bak", false)] {
            let d = Dirs::new("save");
            let mut s = d.server();
            let target = opened_opj(&mut s, &d);
            let link = PathBuf::from(format!("{}{suffix}", target.display()));
            let behind = if points_outside { d.outside.join("victim") } else { d.root.join("inside-victim") };
            symlink(&behind, &link).unwrap();
            let r = call(&mut s, "save_project", json!({}));
            assert_eq!(r["isError"], true, "{suffix} {points_outside}: {r}");
            assert!(!behind.exists(), "{suffix}: something was written behind the link");
            // An existing file behind the link keeps its bytes.
            let _ = std::fs::remove_file(&link);
            let file = if points_outside { d.outside.join("keep") } else { d.root.join("inside-keep") };
            std::fs::write(&file, b"keep").unwrap();
            symlink(&file, &link).unwrap();
            let r = call(&mut s, "save_project", json!({}));
            if points_outside {
                assert_eq!(r["isError"], true, "{suffix}: {r}");
            }
            assert_eq!(std::fs::read(&file).unwrap(), b"keep", "{suffix}: the file behind the link changed");
        }
    }

    #[test]
    fn an_export_through_a_link_that_points_outside_is_refused() {
        let d = Dirs::new("export");
        let mut s = d.server();
        import(&mut s, &d, "a.csv", CSV);
        let keep = d.outside.join("keep.csv");
        std::fs::write(&keep, b"keep").unwrap();
        symlink(&keep, d.root.join("link.csv")).unwrap();
        symlink(d.outside.join("new.csv"), d.root.join("dangling.csv")).unwrap();
        symlink(&d.outside, d.root.join("dir")).unwrap();
        for path in ["link.csv", "dangling.csv", "dir/x.csv"] {
            let r = call(&mut s, "export_data", json!({"path": path}));
            assert_eq!(r["isError"], true, "{path}: {r}");
        }
        assert_eq!(std::fs::read(&keep).unwrap(), b"keep");
        assert!(!d.outside.join("new.csv").exists() && !d.outside.join("x.csv").exists());
        // A graph goes through the same check.
        ok(&call(&mut s, "plot", json!({})));
        symlink(d.outside.join("g.png"), d.root.join("g.png")).unwrap();
        let r = call(&mut s, "export_graph", json!({"path": "g.png"}));
        assert_eq!(r["isError"], true, "{r}");
        assert!(!d.outside.join("g.png").exists());
        // The same files without the links are written.
        let r = call(&mut s, "export_data", json!({"path": "fine.csv"}));
        assert_eq!(r["isError"], false, "{r}");
    }
}

#[test]
fn a_long_name_that_two_workbooks_share_is_refused_not_resolved_to_the_first() {
    let d = Dirs::new("books");
    let mut s = d.server();
    for sub in ["x", "y"] {
        std::fs::create_dir_all(d.root.join(sub)).unwrap();
        std::fs::write(d.root.join(sub).join("data.csv"), CSV).unwrap();
        ok(&call(&mut s, "import_files", json!({"files": [format!("{sub}/data.csv")]})));
    }
    let c = ok(&call(&mut s, "list_contents", json!({})));
    let books = c["workbooks"].as_array().unwrap();
    assert_eq!(books.len(), 2, "{c}");
    assert_eq!(books[0]["long_name"], books[1]["long_name"], "{c}");
    let long = books[0]["long_name"].as_str().unwrap().to_string();
    assert!(!long.is_empty());
    for (tool, args) in [
        ("list_contents", json!({"book": long})),
        ("read_columns", json!({"book": long})),
        ("analyze", json!({"tool": "statistics", "book": long})),
        ("analyze", json!({"tool": "ttest", "book": long})),
        ("fit", json!({"model": "linear", "book": long})),
        ("plot", json!({"book": long})),
    ] {
        let r = call(&mut s, tool, args.clone());
        assert_eq!(r["isError"], true, "{tool}: {r}");
        assert!(text(&r).contains("long name of several workbooks"), "{tool}: {}", text(&r));
    }
    // The short name works.
    let short = books[1]["name"].as_str().unwrap();
    ok(&call(&mut s, "read_columns", json!({"book": short})));
}

#[test]
fn a_long_name_that_two_columns_share_is_refused_not_resolved_to_the_first() {
    let d = Dirs::new("columns");
    let mut s = d.server();
    std::fs::write(d.root.join("c.csv"), "Run,Same,Same\ns,V,V\n0,1,2\n1,2,3\n2,3,5\n3,4,8\n").unwrap();
    ok(&call(
        &mut s,
        "import_files",
        json!({"files": ["c.csv"], "ascii": {"header_lines": 2, "long_name_line": 1, "units_line": 2}}),
    ));
    let c = ok(&call(&mut s, "list_contents", json!({})));
    let cols = c["workbooks"][0]["sheets"][0]["columns"].as_array().unwrap();
    assert_eq!(cols[1]["long_name"], "Same", "{c}");
    assert_eq!(cols[2]["long_name"], "Same", "{c}");
    for (tool, args) in [
        ("read_columns", json!({"columns": ["Same"]})),
        ("analyze", json!({"tool": "statistics", "columns": ["Same"]})),
        ("analyze", json!({"tool": "smooth", "columns": ["Same"]})),
        ("analyze", json!({"tool": "ttest", "columns": ["Same", "B"]})),
        ("set_column_values", json!({"column": "Same", "formula": "1"})),
        ("fit", json!({"model": "linear", "y": ["Same"]})),
        ("plot", json!({"y": ["Same"]})),
    ] {
        let r = call(&mut s, tool, args.clone());
        assert_eq!(r["isError"], true, "{tool}: {r}");
        assert!(text(&r).contains("long name of several columns"), "{tool}: {}", text(&r));
    }
    // A short name works.
    ok(&call(&mut s, "read_columns", json!({"columns": ["B"]})));
}

#[test]
fn plot_and_fit_options_are_parsed_with_structured_fields_and_confined() {
    let d = Dirs::new("plot-fit-options");
    let mut s = d.server();
    import(&mut s, &d, "a.csv", "X,Y,Z\n0,1,2\n1,3,4\n2,5,6\n3,7,8\n4,9,10\n");
    ok(&call(
        &mut s,
        "plot",
        json!({"panels": {"shape": "stack", "order": "bottom_up"}, "options": {"line_width": 2.0}}),
    ));
    ok(&call(&mut s, "plot", json!({"options": {"panels": "stack", "panel_order": "bottom-up", "y": ["B", "C"]}})));
    let r = call(&mut s, "plot", json!({"style": {"line_width": 1.0}, "options": {"line-width": 2.0}}));
    assert_eq!(r["isError"], true, "{r}");
    let r = call(&mut s, "plot", json!({"options": {"output": "outside.png"}}));
    assert_eq!(r["isError"], true, "{r}");
    let r = call(&mut s, "fit", json!({"formula": "a*x+b", "options": {"params": ["a", "b"], "y": ["B"]}}));
    assert_eq!(r["isError"], false, "{r}");
    ok(&call(&mut s, "fit", json!({"scale_errors": null, "options": {"model": "linear"}})));
    let r = call(&mut s, "fit", json!({"options": {"fdf": "../outside.fdf"}}));
    assert_eq!(r["isError"], true, "{r}");
    for value in ["10000001", "18446744073709551615", "999999999999999999999999999999999999999999999999999"] {
        let result = call(&mut s, "fit", json!({"options": {"max_iterations": value}}));
        assert_eq!(result["isError"], true, "{value}: {result}");
        let result = call(&mut s, "plot", json!({"options": {"panels": "stack", "per_panel": [value, "2"]}}));
        assert_eq!(result["isError"], true, "{value}: {result}");
    }
    for key in ["list", "save", "export", "plot", "input"] {
        let r = call(&mut s, "fit", json!({"options": {key: "outside"}}));
        assert_eq!(r["isError"], true, "{key}: {r}");
    }
}

#[test]
fn fit_options_match_legacy_fields() {
    let d = Dirs::new("fit-parity");
    let mut a = d.server();
    import(&mut a, &d, "a.csv", "X,Y\n0,1\n1,3\n2,5\n3,7\n4,9\n");
    let mut b = d.server();
    import(&mut b, &d, "a.csv", "X,Y\n0,1\n1,3\n2,5\n3,7\n4,9\n");
    let legacy = ok(&call(&mut a, "fit", json!({"model": "linear", "y": ["B"], "rows": "2:5", "confidence": 0.9})));
    let bridged =
        ok(&call(&mut b, "fit", json!({"options": {"model": "linear", "y": ["B"], "rows": "2:5", "confidence": 0.9}})));
    assert_eq!(legacy["outcomes"][0]["parameters"], bridged["outcomes"][0]["parameters"]);
    assert_eq!(legacy["outcomes"][0]["statistics"], bridged["outcomes"][0]["statistics"]);
}

#[test]
fn mixed_fit_selection_is_checked_in_the_selected_book() {
    let d = Dirs::new("fit-selected-book");
    let mut s = d.server();
    std::fs::write(d.root.join("first.csv"), "X,Special,Special\n0,1,2\n1,3,4\n2,5,6\n3,7,8\n").unwrap();
    std::fs::write(d.root.join("second.csv"), "X,Special\n0,1\n1,3\n2,5\n3,7\n").unwrap();
    for file in ["first.csv", "second.csv"] {
        ok(&call(&mut s, "import_files", json!({"files": [file], "ascii": {"header_lines": 1, "long_name_line": 1}})));
    }
    let contents = ok(&call(&mut s, "list_contents", json!({})));
    let book = contents["workbooks"][1]["name"].as_str().unwrap();
    let result = call(&mut s, "fit", json!({"y": ["Special"], "options": {"book": book}}));
    assert_eq!(result["isError"], false, "{result}");
}

#[test]
fn tool_schemas_show_session_options_but_not_cli_file_operations() {
    let d = Dirs::new("plot-fit-schemas");
    let mut s = d.server();
    let line = json!({"jsonrpc":"2.0", "id": 1, "method":"tools/list"});
    let reply: Value = serde_json::from_str(&s.handle_line(&line.to_string()).unwrap()).unwrap();
    let tools = reply["result"]["tools"].as_array().unwrap();
    for (name, present, absent) in [("plot", "panel_order", "output"), ("fit", "fdf", "save")] {
        let tool = tools.iter().find(|t| t["name"] == name).unwrap();
        let options = &tool["inputSchema"]["properties"]["options"];
        assert!(options["properties"].get(present).is_some(), "{tool}");
        assert!(options["properties"].get(absent).is_none(), "{tool}");
        assert_eq!(options["additionalProperties"], false);
    }
}

#[test]
fn cli_plot_and_fit_specs_match_protocol_results() {
    use clap::Parser;
    use sheaf_ops::args::{Cli, Command};
    let d = Dirs::new("cli-parity");
    let csv = "X,Y,Z\n0,1,2\n1,3,4\n2,5,6\n3,7,8\n4,9,10\n";
    let mut server = d.server();
    import(&mut server, &d, "a.csv", csv);
    ok(&call(
        &mut server,
        "plot",
        json!({"options": {"template": "line", "y": ["B"], "x_title": "Potential", "line_width": 2.0}}),
    ));
    ok(&call(&mut server, "save_project", json!({"path": "plot.sheaf"})));
    let saved = sheaf_ops::session::load_project(&d.root.join("plot.sheaf")).unwrap().project;
    let mut direct = sheaf_core::Project::new();
    let request = sheaf_ops::import::ImportRequest { files: vec![d.root.join("a.csv")], ..Default::default() };
    sheaf_ops::import::import_files(&mut direct, &request).unwrap();
    let Command::Plot(cli) = Cli::try_parse_from([
        "sheaf",
        "plot",
        "a.csv",
        "-o",
        "a.png",
        "--template",
        "line",
        "--y",
        "B",
        "--x-title",
        "Potential",
        "--line-width",
        "2",
    ])
    .unwrap()
    .command
    else {
        panic!()
    };
    let spec = sheaf_ops::cli_plot::plot_spec(&cli);
    sheaf_ops::plot::build_graph(&mut direct, &spec, sheaf_core::template::Template::Line).unwrap();
    assert_eq!(
        serde_json::to_value(saved.graphs().next().unwrap()).unwrap(),
        serde_json::to_value(direct.graphs().next().unwrap()).unwrap()
    );
    let response = ok(&call(
        &mut server,
        "fit",
        json!({"options": {"model": "linear", "y": ["B"], "rows": "2:5", "confidence": 0.9}}),
    ));
    let Command::Fit(cli) = Cli::try_parse_from([
        "sheaf",
        "fit",
        "a.csv",
        "--model",
        "linear",
        "--y",
        "B",
        "--rows",
        "2:5",
        "--confidence",
        "0.9",
    ])
    .unwrap()
    .command
    else {
        panic!()
    };
    let spec = sheaf_ops::cli_fit::fit_spec(&cli);
    let prepared = sheaf_ops::fit::prepare(&spec).unwrap();
    let result = sheaf_ops::fit::run(&mut direct, &spec, &prepared).unwrap();
    let sheaf_ops::fit::FitRun::Fits { outcomes, .. } = result.run else { panic!() };
    for (actual, expected) in
        response["outcomes"][0]["parameters"].as_array().unwrap().iter().zip(&outcomes[0].report.params)
    {
        assert!((actual["value"].as_f64().unwrap() - expected.value).abs() < 1e-12);
    }
}
