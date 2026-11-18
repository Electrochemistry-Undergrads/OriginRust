//! MCP's analysis destination is the same core output path as a direct request.
use serde_json::{Value, json};
use sheaf_core::analysis::{self, OutputTarget, Request, RowRange, Tool};
use sheaf_core::{ColumnData, Designation, Project};
use sheaf_mcp::{Server, roots::Roots};
use std::path::PathBuf;

struct Dir(PathBuf);
impl Dir {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("sheaf-mcp-output-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        Self(p.canonicalize().unwrap())
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn call(server: &mut Server, name: &str, arguments: Value) -> Value {
    let request = json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":name,"arguments":arguments}});
    let response: Value = serde_json::from_str(&server.handle_line(&request.to_string()).unwrap()).unwrap();
    response["result"].clone()
}
fn payload(result: &Value) -> Value {
    assert_eq!(result["isError"], false, "{result}");
    serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap()
}
fn fixture(dir: &Dir) -> Project {
    let mut p = Project::new();
    let b = p.add_workbook("Synthetic");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric((0..12).map(|i| i as f64 * 0.5).collect())).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric((0..12).map(|i| (i * i) as f64).collect())).unwrap();
    p.column_mut(x).unwrap().long_name = "Potential".into();
    p.column_mut(y).unwrap().long_name = "Current".into();
    p.column_mut(y).unwrap().mask.insert(4);
    sheaf_io::project_file::save(&p, &dir.0.join("input.sheaf")).unwrap();
    p
}
fn server(dir: &Dir) -> Server {
    let mut s = Server::new(Roots::new(std::slice::from_ref(&dir.0)).unwrap());
    payload(&call(&mut s, "open_project", json!({"path": "input.sheaf"})));
    s
}
fn saved(s: &mut Server, d: &Dir) -> Project {
    payload(&call(s, "save_project", json!({"path": "result.sheaf"})));
    sheaf_io::project_file::load(&d.0.join("result.sheaf")).unwrap()
}

#[test]
fn output_new_sheet_matches_core_for_hand_mapped_and_bridged_tools() {
    let cases = [
        ("smooth", Tool::Smooth(analysis::SmoothOptions::default())),
        ("differentiate", Tool::Differentiate(analysis::DiffOptions::default())),
        ("median_filter", Tool::MedianFilter(analysis::MedianFilterSpec::default())),
        ("fft_filter", Tool::FftFilter(analysis::FftFilterSpec::default())),
        ("iir_filter", Tool::IirFilter(analysis::IirSpec::default())),
    ];
    for (name, tool) in cases {
        let dir = Dir::new(name);
        let mut direct = fixture(&dir);
        let book = direct.workbooks().next().unwrap().id;
        if matches!(name, "fft_filter" | "iir_filter") {
            // FFT rejects missing signal values; keep a masked source X column to check preservation.
            let ids: Vec<_> = direct.workbook(book).unwrap().sheets[0].columns.iter().map(|c| c.id).collect();
            direct.column_mut(ids[1]).unwrap().mask.clear();
            direct.column_mut(ids[0]).unwrap().mask.insert(4);
            sheaf_io::project_file::save(&direct, &dir.0.join("input.sheaf")).unwrap();
        }
        let mut s = server(&dir);
        let source = direct.workbook(book).unwrap().sheets[0].clone();
        let y = source.columns[1].id;
        let report = direct
            .analyze_to(&Request { columns: vec![y], rows: RowRange::All, tool }, &OutputTarget::NewSheet)
            .unwrap();
        let result = payload(&call(&mut s, "analyze", json!({"tool":name, "columns":["B"], "output":"new_sheet"})));
        let actual = saved(&mut s, &dir);
        let wb = actual.workbook(book).unwrap();
        assert_eq!(wb.sheets[0], source, "{name}: source including mask and X data");
        let index = report.new_sheet.expect("result sheet");
        assert_eq!(wb.sheets[index], direct.workbook(book).unwrap().sheets[index], "{name}: direct core result");
        assert_eq!(result["new_sheet"], wb.sheets[index].name, "{result}");
        let expected: Vec<String> = report
            .new_columns
            .iter()
            .map(|id| {
                let (b, s, c) = direct.locate(*id).unwrap();
                format!("{}:{}:{}", b.name, s.name, c.short_name)
            })
            .collect();
        assert_eq!(result["new_columns"], json!(expected), "{result}");
        assert_eq!(wb.sheets[index].columns[0].data, source.columns[0].data, "copied X values");
    }
}

#[test]
fn errors_and_unsupported_sheet_jobs_leave_the_project_untouched() {
    let d = Dir::new("rollback");
    let original = fixture(&d);
    let mut s = server(&d);
    for args in [
        json!({"tool":"smooth", "columns":["B"], "output":"new_sheet", "options":{"points":0}}),
        json!({"tool":"sort", "output":"new_sheet", "options":{"by":"B"}}),
        json!({"tool":"filter", "output":"new_sheet", "options":{"where":"col(B)>3", "in_place":true}}),
        json!({"tool":"smooth", "output":"unknown"}),
    ] {
        let r = call(&mut s, "analyze", args.clone());
        assert_eq!(r["isError"], true, "{args}: {r}");
        if args["tool"] == "sort" || args["tool"] == "filter" {
            assert!(r["content"][0]["text"].as_str().unwrap().contains("require next_to_input"), "{args}: {r}");
        }
    }
    let after = saved(&mut s, &d);
    let before_book = original.workbooks().next().unwrap();
    let after_book = after.workbooks().next().unwrap();
    assert_eq!(before_book.sheets, after_book.sheets);
}

#[test]
fn next_to_input_remains_the_default_and_explicit_choice() {
    for (tag, choice) in [("default", Value::Null), ("explicit", json!("next_to_input"))] {
        let d = Dir::new(tag);
        let mut direct = fixture(&d);
        let mut s = server(&d);
        let book = direct.workbooks().next().unwrap().id;
        let y = direct.workbook(book).unwrap().sheets[0].columns[1].id;
        let report = direct
            .analyze(&Request { columns: vec![y], rows: RowRange::All, tool: Tool::Differentiate(Default::default()) })
            .unwrap();
        let output =
            payload(&call(&mut s, "analyze", json!({"tool":"differentiate", "columns":["B"], "output":choice})));
        assert!(output["new_sheet"].is_null());
        let actual = saved(&mut s, &d);
        assert_eq!(actual.workbook(book).unwrap().sheets, direct.workbook(book).unwrap().sheets);
        assert_eq!(output["new_columns"].as_array().unwrap().len(), report.new_columns.len());
    }
}

#[test]
fn intrinsic_table_and_power_destinations_remain_available() {
    let d = Dir::new("tables");
    fixture(&d);
    let mut s = server(&d);
    let table = payload(&call(&mut s, "analyze", json!({"tool":"statistics", "columns":["B"], "output":"new_sheet"})));
    assert!(table["new_sheet"].is_string(), "{table}");
    assert!(table["table"].is_object(), "{table}");
    let power = payload(&call(
        &mut s,
        "analyze",
        json!({"tool":"power", "output":"new_sheet", "options":{"test":"two-sample", "sizes":[10,20], "mean2":1.0, "sd":1.0}}),
    ));
    assert!(power["book"].is_string(), "{power}");
    let read = payload(&call(&mut s, "read_columns", json!({"book":power["book"]})));
    assert_eq!(read["rows"].as_array().unwrap().len(), 2);
}

#[test]
fn tool_schema_advertises_both_output_choices() {
    let d = Dir::new("schema");
    fixture(&d);
    let mut s = server(&d);
    let req = json!({"jsonrpc":"2.0", "id":1, "method":"tools/list"});
    let reply: Value = serde_json::from_str(&s.handle_line(&req.to_string()).unwrap()).unwrap();
    let tools = reply["result"]["tools"].as_array().unwrap();
    let schema = &tools.iter().find(|t| t["name"] == "analyze").unwrap()["inputSchema"]["properties"]["output"];
    assert_eq!(schema["enum"], json!(["next_to_input", "new_sheet"]));
}
