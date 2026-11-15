//! Protocol regressions using generated files only. Every session starts the real stdio binary.

use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const LINE_CAP: usize = 8 * 1024 * 1024;
static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "sheaf-mcp-regression-{tag}-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn raw_session(root: &Path, input: Vec<u8>) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sheaf-mcp"))
        .args(["--root"])
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    // Drain replies while writing: large requests must not deadlock against full pipes.
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout contains only JSON protocol replies"))
        .collect()
}

fn session(root: &Path, requests: &[Value]) -> Vec<Value> {
    let mut input = Vec::new();
    for request in requests {
        writeln!(input, "{request}").unwrap();
    }
    raw_session(root, input)
}

fn call(id: u64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": arguments}})
}

fn reply(replies: &[Value], id: u64) -> &Value {
    replies.iter().find(|r| r["id"] == id).unwrap_or_else(|| panic!("no reply to {id}: {replies:?}"))
}

fn data(replies: &[Value], id: u64) -> Value {
    let r = reply(replies, id);
    assert_eq!(r["result"]["isError"], false, "{r}");
    let text: Value = serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(r["result"]["structuredContent"], text);
    text
}

fn padded_ping(bytes: usize) -> Vec<u8> {
    let mut input = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#.to_vec();
    input.resize(bytes, b' ');
    input
}

#[test]
fn the_line_cap_accepts_exactly_eight_megabytes_with_or_without_a_newline() {
    let dir = Dir::new("cap-exact");
    for ending in [b"".as_slice(), b"\n"] {
        let mut input = padded_ping(LINE_CAP);
        input.extend_from_slice(ending);
        let replies = raw_session(&dir.0, input);
        assert_eq!(replies, [json!({"jsonrpc": "2.0", "id": 1, "result": {}})]);
    }
}

#[test]
fn an_oversized_line_is_refused_at_eof_too() {
    let dir = Dir::new("cap-eof");
    let replies = raw_session(&dir.0, padded_ping(LINE_CAP + 1));
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["id"], Value::Null);
    assert_eq!(replies[0]["error"]["code"], -32600, "{:?}", replies[0]);
    assert!(replies[0]["error"]["message"].as_str().unwrap().contains("8 MB"));
}

#[test]
fn an_oversized_line_is_drained_before_the_next_request() {
    let dir = Dir::new("cap-recovery");
    for size in [LINE_CAP + 1, 2 * LINE_CAP + 91] {
        let mut input = padded_ping(size);
        input.extend_from_slice(b"\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\r\n");
        let replies = raw_session(&dir.0, input);
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0]["error"]["code"], -32600);
        assert_eq!(replies[0]["id"], Value::Null);
        assert_eq!(reply(&replies, 2)["result"], json!({}));
    }
}

/// A tiny binary file built from the documented CHI layout, without instrument data.
fn chi_binary(code: &str, name: &str, channels: &[Vec<f32>], fields: &[(usize, f32)]) -> Vec<u8> {
    let mut bytes = vec![0x80, 0xf2, 0x1d, 0x00];
    for text in [code, name, "Synthetic MCP test", "", "", "", "", "", "", "A3145"] {
        bytes.extend_from_slice(&(text.len() as u32).to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
    }
    let body = bytes.len();
    bytes.resize(body + 0x650, 0);
    for (offset, value) in [
        (0x21c, channels[0].len() as u32),
        (0x224, channels[0].len() as u32),
        (0x248, 627),
        (0x2ec, channels.len() as u32),
    ] {
        bytes[body + offset..body + offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    for &(offset, value) in fields.iter().chain([(0x630, 1.0)].iter()) {
        bytes[body + offset..body + offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    for row in 0..channels[0].len() {
        for channel in channels {
            assert_eq!(channel.len(), channels[0].len());
            bytes.extend_from_slice(&channel[row].to_le_bytes());
        }
    }
    bytes
}

#[test]
fn binary_cv_import_splits_scans_preserves_values_and_round_trips() {
    let dir = Dir::new("binary-cv");
    let currents: Vec<f32> = (0..16).map(|i| (i + 1) as f32 * 0.001).collect();
    let bytes = chi_binary(
        "CV",
        "Cyclic Voltammetry",
        &[currents],
        &[(0x3f8, 1.0), (0x400, 1.0), (0x404, 0.0), (0x408, 0.5), (0x414, 4.0), (0x418, 0.25)],
    );
    // Content detection must work even without the .bin extension.
    std::fs::write(dir.0.join("cv.dat"), &bytes).unwrap();
    let replies = session(
        &dir.0,
        &[
            call(1, "import_files", json!({"files": ["cv.dat"]})),
            call(2, "list_contents", json!({})),
            call(3, "read_columns", json!({"columns": ["A", "B", "C", "D", "E", "F"], "to": 2})),
            call(4, "save_project", json!({"path": "cv.sheaf"})),
        ],
    );
    assert_eq!(data(&replies, 1)["errors"], json!([]));
    let contents = data(&replies, 2);
    let sheet = &contents["workbooks"][0]["sheets"][0];
    assert_eq!(sheet["name"], "Scans");
    assert_eq!(sheet["rows"], 8);
    let columns = sheet["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 6);
    assert_eq!(columns[0]["units"], "V");
    assert_eq!(columns[0]["designation"], "X");
    assert_eq!(columns[1]["units"], "A");
    assert_eq!(columns[2]["units"], "mA");
    assert_eq!(columns[2]["formula"], "col(B)*1000");
    assert_eq!(columns[3]["comments"], "Scan 2");
    let rows = data(&replies, 3)["rows"].clone();
    for (row, potential, first, second) in [(0, 1.0, 0.001, 0.009), (1, 0.75, 0.002, 0.010)] {
        assert_eq!(rows[row][0], potential);
        assert_eq!(rows[row][3], potential);
        for (col, value) in [(1, first), (2, first * 1000.0), (4, second), (5, second * 1000.0)] {
            assert!((rows[row][col].as_f64().unwrap() - value).abs() < 1e-5);
        }
    }
    data(&replies, 4);
    let reopened = session(
        &dir.0,
        &[
            call(1, "open_project", json!({"path": "cv.sheaf"})),
            call(2, "read_columns", json!({"columns": ["A", "B", "C", "D", "E", "F"], "to": 2})),
        ],
    );
    data(&reopened, 1);
    assert_eq!(data(&reopened, 2)["rows"], rows);
    assert_eq!(std::fs::read(dir.0.join("cv.dat")).unwrap(), bytes);
}

#[test]
fn binary_two_channel_amperometry_keeps_channel_order_and_integrates_charge() {
    let dir = Dir::new("binary-it");
    let bytes = chi_binary(
        "i-t",
        "Amperometric i-t Curve",
        &[vec![0.001, 0.002, 0.003, 0.004], vec![0.01, 0.02, 0.03, 0.04]],
        &[(0x418, 0.25), (0x430, 1.0)],
    );
    std::fs::write(dir.0.join("it.bin"), bytes).unwrap();
    let replies = session(
        &dir.0,
        &[
            call(1, "import_files", json!({"files": ["it.bin"]})),
            call(2, "read_columns", json!({"to": 4})),
            call(3, "list_contents", json!({})),
            call(4, "new_project", json!({})),
            call(
                5,
                "import_files",
                json!({"files": ["it.bin"], "chi": {"invert_current": true, "add_milliamp": false, "add_charge": false}}),
            ),
            call(6, "read_columns", json!({"to": 4})),
        ],
    );
    data(&replies, 1);
    let rows = data(&replies, 2)["rows"].clone();
    assert_eq!(rows[0], json!([0.0, 0.001, 1.0, 0.01, 10.0, 0.0, 0.0]));
    let expected = [0.75, 0.004, 4.0, 0.04, 40.0, 0.001875, 0.01875];
    for (cell, want) in rows[3].as_array().unwrap().iter().zip(expected) {
        assert!((cell.as_f64().unwrap() - want).abs() < 1e-10, "{cell} vs {want}");
    }
    assert_eq!(data(&replies, 3)["workbooks"][0]["sheets"][0]["columns"][6]["units"], "C");
    data(&replies, 4);
    data(&replies, 5);
    assert_eq!(data(&replies, 6)["rows"][3], json!([0.75, -0.004, -0.04]));
}

fn write_project(path: &Path) -> Vec<u8> {
    let mut project = sheaf_core::Project::new();
    let book = project.add_workbook("");
    for (designation, values) in
        [(sheaf_core::Designation::X, vec![0.0, 1.0, 2.0, 3.0]), (sheaf_core::Designation::Y, vec![1.0, 2.0, 3.0, 4.0])]
    {
        project.add_column(book, 0, designation, sheaf_core::ColumnData::numeric(values)).unwrap();
    }
    sheaf_io::save::save_project(&project, path).unwrap();
    std::fs::read(path).unwrap()
}

#[test]
fn binary_chronopotentiometry_preserves_time_and_potential() {
    let dir = Dir::new("binary-cp");
    let bytes = chi_binary(
        "CP",
        "Chronopotentiometry",
        &[vec![1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75, 1.0]],
        &[(0x400, 1.0), (0x404, 0.0), (0x414, 2.0), (0x418, 0.5), (0x47c, 0.001), (0x4ac, 0.001)],
    );
    std::fs::write(dir.0.join("cp.bin"), bytes).unwrap();
    let replies = session(
        &dir.0,
        &[
            call(1, "import_files", json!({"files": ["cp.bin"], "chi": {"split_segments": false}})),
            call(2, "read_columns", json!({"columns": ["A", "B"]})),
            call(3, "list_contents", json!({})),
        ],
    );
    data(&replies, 1);
    assert_eq!(
        data(&replies, 2)["rows"],
        json!([
            [0.0, 1.0],
            [0.5, 0.75],
            [1.0, 0.5],
            [1.5, 0.25],
            [2.0, 0.0],
            [2.5, 0.25],
            [3.0, 0.5],
            [3.5, 0.75],
            [4.0, 1.0]
        ])
    );
    let contents = data(&replies, 3);
    let sheets = contents["workbooks"][0]["sheets"].as_array().unwrap();
    assert_eq!(sheets.len(), 1, "only Data, with Segments disabled");
    assert_eq!(sheets[0]["columns"][0]["units"], "s");
    assert_eq!(sheets[0]["columns"][1]["units"], "V");
}

#[test]
fn saving_over_an_opened_or_imported_opj_keeps_the_first_original_backup() {
    for tool in ["open_project", "import_files"] {
        let dir = Dir::new(tool);
        let original = write_project(&dir.0.join("original.opj"));
        let arguments =
            if tool == "open_project" { json!({"path": "original.opj"}) } else { json!({"files": ["original.opj"]}) };
        let replies = session(
            &dir.0,
            &[
                call(1, tool, arguments),
                call(2, "set_column_values", json!({"column": "B", "formula": "i*7"})),
                call(3, "save_project", json!({"path": "original.opj"})),
                call(4, "set_column_values", json!({"column": "B", "formula": "i*9"})),
                call(5, "save_project", json!({"path": "original.opj"})),
            ],
        );
        data(&replies, 1);
        if tool == "open_project" {
            assert_eq!(data(&replies, 1)["foreign"], true);
        }
        data(&replies, 2);
        assert!(data(&replies, 3)["backup"].as_str().unwrap().ends_with("original.opj.bak"));
        data(&replies, 4);
        assert_eq!(data(&replies, 5)["backup"], Value::Null);
        assert_eq!(std::fs::read(dir.0.join("original.opj.bak")).unwrap(), original);
        assert_ne!(std::fs::read(dir.0.join("original.opj")).unwrap(), original);
        // Repeat in a new process: an existing backup is still the first original.
        let reopened = session(
            &dir.0,
            &[
                call(1, "open_project", json!({"path": "original.opj"})),
                call(2, "save_project", json!({"path": "original.opj"})),
            ],
        );
        data(&reopened, 1);
        assert_eq!(data(&reopened, 2)["backup"], Value::Null);
        assert_eq!(std::fs::read(dir.0.join("original.opj.bak")).unwrap(), original);
    }
}

#[cfg(windows)]
struct Junction(PathBuf);

#[cfg(windows)]
impl Junction {
    fn new(link: &Path, target: &Path) -> Self {
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference = 'Stop'; New-Item -ItemType Junction -Path $env:SHEAF_TEST_JUNCTION -Target $env:SHEAF_TEST_TARGET | Out-Null",
            ])
            .env("SHEAF_TEST_JUNCTION", link)
            .env("SHEAF_TEST_TARGET", target)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(std::fs::canonicalize(link).unwrap(), std::fs::canonicalize(target).unwrap());
        Self(link.to_path_buf())
    }
}

#[cfg(windows)]
impl Drop for Junction {
    fn drop(&mut self) {
        // Remove the junction itself before the owned temporary directory is cleaned up.
        std::fs::remove_dir(&self.0).unwrap();
    }
}

#[cfg(windows)]
#[test]
fn windows_junctions_cannot_read_or_write_outside_the_root() {
    let dir = Dir::new("junctions");
    let root = dir.0.join("allowed");
    let outside = dir.0.join("outside");
    let inside = root.join("inside");
    for folder in [&root, &outside, &inside] {
        std::fs::create_dir(folder).unwrap();
    }
    let csv = b"x,y\n0,1\n1,2\n";
    std::fs::write(inside.join("data.csv"), csv).unwrap();
    std::fs::write(outside.join("data.csv"), csv).unwrap();
    let original = write_project(&outside.join("project.opj"));
    let _escape = Junction::new(&root.join("escape"), &outside);
    let _safe = Junction::new(&root.join("safe"), &inside);
    let replies = session(
        &root,
        &[
            call(1, "import_files", json!({"files": ["safe/data.csv"]})),
            call(2, "plot", json!({"template": "line"})),
            call(3, "import_files", json!({"files": ["escape/data.csv"]})),
            call(4, "open_project", json!({"path": "escape/project.opj"})),
            call(5, "save_project", json!({"path": "escape/project.opj"})),
            call(6, "save_project", json!({"path": "escape/new/sub/project"})),
            call(7, "export_data", json!({"path": "escape/data.csv"})),
            call(8, "export_graph", json!({"path": "escape/figure.svg"})),
            call(9, "read_columns", json!({"columns": ["A", "B"]})),
            call(10, "save_project", json!({"path": "safe/saved.sheaf"})),
        ],
    );
    data(&replies, 1);
    data(&replies, 2);
    for id in 3..=8 {
        let r = reply(&replies, id);
        assert_eq!(r["result"]["isError"], true, "{r}");
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("outside the allowed folders"), "{r}");
    }
    assert_eq!(data(&replies, 9)["rows"], json!([[0.0, 1.0], [1.0, 2.0]]));
    data(&replies, 10);
    assert!(inside.join("saved.sheaf").is_file());
    assert_eq!(std::fs::read(outside.join("data.csv")).unwrap(), csv);
    assert_eq!(std::fs::read(outside.join("project.opj")).unwrap(), original);
    assert!(!outside.join("project.opj.bak").exists());
    assert!(!outside.join("project.opj.sheaf").exists());
    assert!(!outside.join("new").exists());
    assert!(!outside.join("figure.svg").exists());
}
