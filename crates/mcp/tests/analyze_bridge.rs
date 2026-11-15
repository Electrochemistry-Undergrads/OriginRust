//! `analyze` reaches every `sheaf analyze` tool: the hand-mapped ones and, through the command
//! line's own clap definitions, all the others. The tests enumerate the subcommands from clap, call
//! each with empty options (nothing may panic) and compare the numbers of bridged tools with the
//! command line's code run in-process on the same synthetic file.

use clap::{CommandFactory, Parser};
use serde_json::{Value, json};
use sheaf_mcp::Server;
use sheaf_mcp::roots::Roots;
use sheaf_ops::analyze::{Outcome, Target, run_job};
use sheaf_ops::args::{Cli, Command};
use sheaf_ops::cli_analyze::job_from;
use sheaf_ops::{ReadSettings, read::read_data_file};
use std::path::PathBuf;

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let p = std::env::temp_dir().join(format!("sheaf-mcp-bridge-{tag}-{}", std::process::id()));
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

/// Four numeric columns, 120 rows: a slow sine with a fast ripple, a scaled copy with a
/// different phase, a pseudo-random column and a second pseudo-random column.
fn csv() -> String {
    let mut seed: u64 = 12345;
    let mut next = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (1u64 << 31) as f64
    };
    let mut s = String::from("T,A,B,C,D\n");
    for i in 0..120 {
        let t = i as f64 * 0.01;
        let a = (2.0 * std::f64::consts::PI * 3.0 * t).sin() + 0.1 * (2.0 * std::f64::consts::PI * 40.0 * t).sin();
        let b = 0.5 * a + 0.2 * (7.0 * t).cos() + 0.05 * next();
        s.push_str(&format!("{t},{a},{b},{},{}\n", next() * 4.0, next() * 4.0 + 1.0));
    }
    s
}

fn server(dir: &Dir) -> Server {
    let mut s = Server::new(Roots::new(std::slice::from_ref(&dir.0)).unwrap());
    let reply = call(&mut s, "import_files", json!({"files": ["data.csv"]}));
    assert_eq!(reply["isError"], false, "{reply}");
    s
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

fn analyze_subcommands() -> Vec<String> {
    let cmd = Cli::command();
    let analyze = cmd.find_subcommand("analyze").unwrap();
    analyze.get_subcommands().map(|c| c.get_name().to_string()).filter(|n| n != "help").collect()
}

#[test]
fn every_cli_analysis_tool_is_listed_and_reachable_and_none_panics_without_options() {
    let dir = Dir::new("all");
    std::fs::write(dir.0.join("data.csv"), csv()).unwrap();
    let mut s = server(&dir);
    let listed: Vec<String> = payload(&call(&mut s, "list_analysis_tools", json!({})))["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    let subs = analyze_subcommands();
    assert!(subs.len() >= 55, "{} subcommands", subs.len());
    for sub in &subs {
        let snake = sub.replace('-', "_");
        let alias = match snake.as_str() {
            "peaks" => "find_peaks",
            "stats" => "statistics",
            other => other,
        };
        assert!(listed.iter().any(|n| n == alias), "{sub} is not listed (as {alias})");
        for name in [snake.as_str(), sub.as_str()] {
            // A fresh project each time: results become Y columns the next tool would work on.
            let mut fresh = server(&dir);
            let r = call(&mut fresh, "analyze", json!({"tool": name, "options": {}}));
            let t = text(&r);
            assert!(!t.contains("unknown analysis tool"), "{name}: {t}");
            assert!(!t.contains("internal error"), "{name} panicked: {t}");
        }
    }
    // The listing has the hand-mapped tools, then the others with a JSON Schema.
    let all = payload(&call(&mut s, "list_analysis_tools", json!({})));
    let ttest = all["tools"].as_array().unwrap().iter().find(|t| t["name"] == "ttest").unwrap();
    assert_eq!(ttest["bridged"], true);
    assert!(ttest["options"].as_array().unwrap().iter().any(|o| o["name"] == "mean" && o["type"] == "number"));
    assert_eq!(ttest["input_schema"]["additionalProperties"], false);
    // An unknown tool names the tools.
    let t = text(&call(&mut s, "analyze", json!({"tool": "nope"})));
    assert!(t.contains("unknown analysis tool") && t.contains("mannwhitney") && t.contains("integrate"), "{t}");
}

/// The result of `sheaf analyze` run in-process: the table, the message and the result sheet's
/// numeric columns.
struct CliRun {
    table: Option<Vec<(String, Vec<f64>)>>,
    headers: Vec<String>,
    message: String,
    sheet: Vec<Vec<f64>>,
}

fn run_cli(dir: &Dir, argv: &[&str]) -> CliRun {
    let path = dir.0.join("data.csv");
    let mut full = vec!["sheaf", "analyze"];
    full.push(argv[0]);
    full.push("data.csv");
    full.extend_from_slice(&argv[1..]);
    let cli = Cli::try_parse_from(&full).unwrap_or_else(|e| panic!("{argv:?}: {e}"));
    let Command::Analyze(a) = cli.command else { panic!("analyze") };
    let (t, job) = job_from(a.tool).unwrap();
    let mut project = sheaf_core::Project::new();
    project.adopt_workbook(read_data_file(&path, &ReadSettings::default()).unwrap().workbook);
    let target = Target { y: t.y, book: t.book, sheet: t.sheet, rows: t.rows, x_range: t.x_range, ..Target::default() };
    let run = run_job(&mut project, &target, job).unwrap();
    let ws = &project.workbook(run.book).unwrap().sheets[run.result_sheet];
    let sheet = ws.columns.iter().map(|c| c.numeric().map(<[f64]>::to_vec).unwrap_or_default()).collect();
    let Outcome::Report(r) = run.outcome else { panic!("report") };
    CliRun {
        table: r.table.as_ref().map(|t| t.rows.clone()),
        headers: r.table.as_ref().map(|t| t.headers.clone()).unwrap_or_default(),
        message: r.message,
        sheet,
    }
}

fn number(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// Equal up to the last digit that a JSON text keeps (serde_json without `float_roundtrip` reads a
/// float with an error of a few units in the last place).
fn same(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= 1e-13 * a.abs().max(b.abs())
}

/// Calls `analyze` and compares the table and the result sheet with the command line's.
fn parity(dir: &Dir, tool: &str, columns: &[&str], options: Value, argv: &[&str]) {
    let mut s = server(dir);
    let r = call(&mut s, "analyze", json!({"tool": tool, "columns": columns, "options": options}));
    let out = payload(&r);
    let cli = run_cli(dir, argv);
    assert_eq!(out["message"], json!(cli.message), "{tool}: message");
    match (&cli.table, out["table"].as_object()) {
        (Some(rows), Some(t)) => {
            assert_eq!(t["headers"], json!(cli.headers), "{tool}: headers");
            let got = t["rows"].as_array().unwrap();
            assert_eq!(got.len(), rows.len(), "{tool}: table rows");
            for (g, (label, values)) in got.iter().zip(rows) {
                assert_eq!(g["label"], json!(label), "{tool}");
                let gv = g["values"].as_array().unwrap();
                assert_eq!(gv.len(), values.len(), "{tool}");
                for (x, y) in gv.iter().zip(values) {
                    assert!(same(number(x), *y), "{tool}: {label}: {x} vs {y}");
                }
            }
        }
        (None, None) => {}
        _ => panic!("{tool}: one has a table and the other has not: {out}"),
    }
    // The sheet that holds the results, read back through the server.
    let sheet = out["new_sheet"].as_str().or(out["sheet"].as_str()).unwrap().to_string();
    let read = payload(&call(&mut s, "read_columns", json!({"sheet": sheet, "limit": 10000})));
    let rows = read["rows"].as_array().unwrap();
    for (c, col) in cli.sheet.iter().enumerate() {
        // The server reads only a cell cap per page; these sheets are small.
        let n = col.len();
        assert!(rows.len() >= n, "{tool}: column {c} has {n} rows, the server gave {}", rows.len());
        for (r, y) in col.iter().enumerate() {
            let x = number(&rows[r][c]);
            assert!(same(x, *y), "{tool}: sheet column {c} row {r}: {x} vs {y}");
        }
    }
}

#[test]
fn bridged_tools_give_the_numbers_of_the_command_line() {
    let dir = Dir::new("parity");
    std::fs::write(dir.0.join("data.csv"), csv()).unwrap();
    parity(&dir, "ttest", &["B", "C"], json!({}), &["ttest", "--y", "B,C"]);
    parity(
        &dir,
        "ttest",
        &["C"],
        json!({"test": "one-sample", "mean": 2.0}),
        &["ttest", "--y", "C", "--test", "one-sample", "--mean", "2"],
    );
    parity(&dir, "anova", &["B", "C", "D"], json!({}), &["anova", "--y", "B,C,D"]);
    parity(
        &dir,
        "histogram",
        &["C"],
        json!({"bins": 8, "no_graph": true}),
        &["histogram", "--y", "C", "--bins", "8", "--no-graph"],
    );
    parity(&dir, "normality", &["B", "C"], json!({}), &["normality", "--y", "B,C"]);
    parity(&dir, "corr", &["A", "B", "C"], json!({"spearman": true}), &["corr", "--y", "A,B,C", "--spearman"]);
    parity(&dir, "mannwhitney", &["C", "D"], json!({}), &["mannwhitney", "--y", "C,D"]);
    parity(
        &dir,
        "fft_filter",
        &["A"],
        json!({"type": "low-pass", "cutoff": 10.0}),
        &["fft-filter", "--y", "A", "--type", "low-pass", "--cutoff", "10"],
    );
    parity(&dir, "polygon_area", &["A"], json!({}), &["polygon-area", "--y", "A"]);
    parity(&dir, "hilbert", &["A"], json!({}), &["hilbert", "--y", "A"]);
}

#[test]
fn selection_keys_map_to_the_command_line_options_and_bad_options_are_named() {
    let dir = Dir::new("sel");
    std::fs::write(dir.0.join("data.csv"), csv()).unwrap();
    let mut s = server(&dir);
    // rows and x_range as on the command line, a negative number as a value.
    let r = call(&mut s, "analyze", json!({"tool": "normality", "columns": ["C"], "rows": "5:100"}));
    assert_eq!(r["isError"], false, "{}", text(&r));
    let t = text(&call(&mut s, "analyze", json!({"tool": "normality", "options": {"nope": 1}})));
    assert!(t.contains("unknown option 'nope'") && t.contains("alpha"), "{t}");
    let t = text(&call(&mut s, "analyze", json!({"tool": "normality", "options": {"export": "x.csv"}})));
    assert!(t.contains("unknown option 'export'"), "{t}");
    let t = text(&call(&mut s, "analyze", json!({"tool": "ttest", "columns": ["Nope"]})));
    assert!(t.contains("Nope"), "{t}");
}

#[test]
fn power_puts_its_workbook_into_the_session_project() {
    let dir = Dir::new("power");
    std::fs::write(dir.0.join("data.csv"), csv()).unwrap();
    let mut s = server(&dir);
    // A workbook already there: the power table is not the first one.
    call(&mut s, "import_files", json!({"files": ["data.csv"]}));
    let before = payload(&call(&mut s, "list_contents", json!({})))["workbooks"].as_array().unwrap().len();
    let out = payload(&call(
        &mut s,
        "analyze",
        json!({"tool": "power", "options": {"test": "two-sample", "sizes": [10, 20, 40], "mean2": 1.0, "sd": 1.0}}),
    ));
    let book = out["book"].as_str().unwrap().to_string();
    assert!(out["message"].as_str().unwrap().ends_with(&format!("in workbook {book}")), "{out}");
    let after = payload(&call(&mut s, "list_contents", json!({})));
    assert_eq!(after["workbooks"].as_array().unwrap().len(), before + 1);
    assert!(after["workbooks"].as_array().unwrap().iter().any(|b| b["name"] == json!(book)));
    let read = payload(&call(&mut s, "read_columns", json!({"book": book})));
    assert_eq!(read["rows"].as_array().unwrap().len(), 3);
    // The numbers are the command line's.
    let spec = sheaf_ops::cli_analyze::power_spec(
        match Cli::try_parse_from([
            "sheaf",
            "analyze",
            "power",
            "--test",
            "two-sample",
            "--sizes",
            "10,20,40",
            "--mean2",
            "1",
            "--sd",
            "1",
        ])
        .unwrap()
        .command
        {
            Command::Analyze(a) => a.tool,
            _ => panic!(),
        },
    )
    .unwrap()
    .0;
    let (p, _) = sheaf_ops::analyze::run_power(&spec).unwrap();
    let want = p.workbooks().next().unwrap().sheets[0].columns[1].numeric().unwrap().to_vec();
    for (row, w) in read["rows"].as_array().unwrap().iter().zip(want) {
        assert!(same(number(&row[1]), w));
    }
}

/// The MCP `analyze` tool must give the same job as the command line for the same request.
#[test]
fn mcp_defaults_equal_the_command_line_defaults() {
    // (subcommand, extra arguments, MCP tool, MCP options)
    let cases: [(&str, &[&str], &str, serde_json::Value); 16] = [
        ("integrate", &[], "integrate", serde_json::json!({})),
        ("smooth", &[], "smooth", serde_json::json!({})),
        ("differentiate", &[], "differentiate", serde_json::json!({})),
        ("normalize", &[], "normalize", serde_json::json!({})),
        ("interpolate", &[], "interpolate", serde_json::json!({})),
        ("fft", &[], "fft", serde_json::json!({})),
        ("hilbert", &[], "hilbert", serde_json::json!({})),
        ("envelope", &[], "envelope", serde_json::json!({})),
        ("level-crossing", &["--level", "0.5"], "level_crossing", serde_json::json!({"level": 0.5})),
        ("median-filter", &[], "median_filter", serde_json::json!({})),
        ("decimate", &[], "decimate", serde_json::json!({})),
        ("decimate", &["--filter", "fir"], "decimate", serde_json::json!({"filter": "fir"})),
        ("translate", &["--by", "5"], "translate", serde_json::json!({"by": 5.0})),
        ("peaks", &[], "find_peaks", serde_json::json!({})),
        ("stats", &[], "statistics", serde_json::json!({})),
        ("intersect", &[], "intersect", serde_json::json!({})),
    ];
    for (sub, extra, tool, options) in cases {
        let mut argv = vec!["sheaf", "analyze", sub, "data.csv"];
        argv.extend_from_slice(extra);
        let cli = Cli::try_parse_from(&argv).unwrap_or_else(|e| panic!("{sub}: {e}"));
        let Command::Analyze(a) = cli.command else { panic!("{sub}: not analyze") };
        let (_, cli_job) = job_from(a.tool).unwrap();
        let mcp_job = sheaf_mcp::analysis_tools::build(tool, options.as_object().unwrap()).unwrap();
        let (sheaf_ops::analyze::Job::Tool(c), sheaf_ops::analyze::Job::Tool(m)) = (cli_job, mcp_job) else {
            panic!("{sub}: not a plain tool")
        };
        assert_eq!(c, m, "{sub} {extra:?}");
    }
}
