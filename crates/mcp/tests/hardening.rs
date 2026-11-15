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
