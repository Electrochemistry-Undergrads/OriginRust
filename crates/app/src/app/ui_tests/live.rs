//! The AI link (T25 phase 2): Tools > AI Assistants (MCP) and what the UI thread does with the calls
//! of `sheaf-mcp --attach`. The fake assistant is the real proxy code running in this process, talking
//! to the app over a real socket. Every test listens in its own folder under the temporary folder; none
//! touches the user's real run folders (`Startup::default()` has no folder for the link).

use super::*;
use serde_json::{Value, json};
use sheaf_mcp::link::client::{self, ClientOptions};
use sheaf_mcp::link::frame::{self, Line};
use sheaf_mcp::link::proto::{CLOSED_FOR_ASSISTANT, Request};
use sheaf_mcp::proxy::{self, ProxyConfig};
use sheaf_mcp::roots::Roots;
use std::io::{BufReader, Read, Write};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The folders of one test: the run folder of the link and the folder an assistant may use.
struct Dirs {
    base: PathBuf,
}

impl Dirs {
    fn new(tag: &str) -> Dirs {
        // The Unix socket path has a length limit, so the names stay short.
        let base = std::env::temp_dir().canonicalize().unwrap().join(format!("sl-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("root")).unwrap();
        std::fs::write(base.join("root/sample.csv"), "t,up,down\n1,2.5,30.5\n2,4.5,20.5\n3,6.5,10.5\n4,8.5,0.5\n")
            .unwrap();
        Dirs { base }
    }

    fn run(&self) -> PathBuf {
        self.base.join("run")
    }

    fn root(&self) -> PathBuf {
        self.base.join("root")
    }

    fn csv(&self) -> String {
        self.root().join("sample.csv").to_string_lossy().into_owned()
    }
}

impl Drop for Dirs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

/// The application with the link folder of `dirs`; `on` is the `--mcp` flag.
fn start_link(dirs: &Dirs, on: bool) -> Harness<'static, SheafApp> {
    let startup = Startup { mcp: on, mcp_dirs: Some(vec![dirs.run()]), ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    h
}

fn tick_link(h: &mut Harness<'static, SheafApp>) {
    menu(h, "Tools", "AI Assistants (MCP)");
}

/// Runs frames until `cond` holds.
fn until(h: &mut Harness<'static, SheafApp>, what: &str, mut cond: impl FnMut(&Harness<'static, SheafApp>) -> bool) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(15) {
        if cond(h) {
            return;
        }
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {what}");
}

struct ChannelReader {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    buf: Vec<u8>,
    pos: usize,
}

impl Read for ChannelReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.buf.len() {
            match self.rx.recv() {
                Ok(b) => {
                    self.buf = b;
                    self.pos = 0;
                }
                Err(_) => return Ok(0),
            }
        }
        let n = out.len().min(self.buf.len() - self.pos);
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// An MCP client talking to the real `sheaf-mcp --attach` code through pipes made of channels.
struct Assistant {
    tx: Option<Sender<Vec<u8>>>,
    out: Arc<Mutex<Vec<u8>>>,
    handle: Option<JoinHandle<i32>>,
    next: u64,
}

impl Assistant {
    fn start(dirs: &Dirs) -> Assistant {
        let mut cfg = ProxyConfig::new(&Roots::new(&[dirs.root()]).unwrap());
        cfg.candidates = vec![dirs.run()];
        let (tx, rx) = channel();
        let out = Arc::new(Mutex::new(Vec::new()));
        let sink = out.clone();
        let handle = std::thread::spawn(move || {
            let mut w = Sink(sink);
            proxy::run(cfg, BufReader::new(ChannelReader { rx, buf: Vec::new(), pos: 0 }), &mut w)
        });
        Assistant { tx: Some(tx), out, handle: Some(handle), next: 1 }
    }

    fn send(&self, v: Value) {
        self.tx.as_ref().unwrap().send(format!("{v}\n").into_bytes()).unwrap();
    }

    fn find(&self, id: u64) -> Option<Value> {
        let text = String::from_utf8_lossy(&self.out.lock().unwrap()).into_owned();
        text.lines().map(|l| serde_json::from_str::<Value>(l).unwrap()).find(|l| l["id"] == id)
    }

    /// Sends a tool call; the id to wait for.
    fn request(&mut self, name: &str, args: Value) -> u64 {
        self.next += 1;
        let id = self.next;
        self.send(
            json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": args}}),
        );
        id
    }

    /// The reply to `id` (the whole JSON-RPC message) once the app has answered, running its frames.
    fn wait(&self, h: &mut Harness<'static, SheafApp>, id: u64) -> Value {
        let mut reply = None;
        until(h, "the reply of the app", |_| {
            reply = self.find(id);
            reply.is_some()
        });
        reply.unwrap()
    }

    /// A call and its `result` (a tool error is a result with `isError`).
    fn call(&mut self, h: &mut Harness<'static, SheafApp>, name: &str, args: Value) -> Value {
        let id = self.request(name, args);
        let r = self.wait(h, id);
        assert!(r.get("error").is_none(), "{r}");
        r["result"].clone()
    }

    fn end(mut self) {
        self.tx = None;
        self.handle.take().unwrap().join().unwrap();
    }
}

fn text(result: &Value) -> String {
    result["content"][0]["text"].as_str().unwrap_or_default().to_string()
}

fn log_has(h: &Harness<'static, SheafApp>, needle: &str) -> bool {
    h.state().logs.messages.iter().any(|m| m.text.contains(needle))
}

fn graph_count(h: &Harness<'static, SheafApp>) -> usize {
    project(h).windows.iter().filter(|w| matches!(w, Window::Graph(_))).count()
}

#[test]
fn the_menu_tick_turns_the_endpoint_on_and_off_and_it_starts_off() {
    let dirs = Dirs::new("tick");
    let mut h = start_link(&dirs, false);
    assert!(!h.state().live.active(), "a run without --mcp starts with the link off");
    assert!(!dirs.run().join("mcp.json").exists());
    tick_link(&mut h);
    assert!(h.state().live.active());
    assert!(dirs.run().join("mcp.json").exists(), "the endpoint is described for the proxy");
    assert!(log_has(&h, "AI link on"));
    tick_link(&mut h);
    assert!(!h.state().live.active());
    assert!(!dirs.run().join("mcp.json").exists());
    assert!(log_has(&h, "AI link off"));
    // Off, then on again.
    tick_link(&mut h);
    assert!(h.state().live.active());
    assert!(dirs.run().join("mcp.json").exists());
}

#[test]
fn the_flag_turns_it_on_at_start_and_a_second_app_gets_a_warning_instead() {
    let dirs = Dirs::new("flag");
    let first = start_link(&dirs, true);
    assert!(first.state().live.active());
    let second = start_link(&dirs, true);
    assert!(!second.state().live.active(), "one endpoint per user");
    assert!(log_has(&second, "The AI link could not start"), "{:?}", second.state().logs.messages);
    assert!(second.state().message.is_none(), "a failed start is a warning, not a message box");
}

#[test]
fn without_a_folder_the_link_cannot_start_and_says_so() {
    let mut h = start(Vec::new());
    tick_link(&mut h);
    assert!(!h.state().live.active());
    assert!(log_has(&h, "The AI link could not start"));
}

#[test]
fn an_assistant_imports_and_plots_and_each_change_is_one_undo_step() {
    let dirs = Dirs::new("plot");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    let r = a.call(&mut h, "import_files", json!({"files": [dirs.csv()]}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(project(&h).windows.len(), 1, "the book window is there");
    assert!(h.state().workspace.contains(project(&h).windows[0].id()), "and it is open as usual");
    assert_eq!(h.state().doc.undo_label(), Some("AI: import_files sample.csv"));
    assert!(h.state().doc.dirty);
    assert_eq!(h.state().live.clients().len(), 1);
    assert!(log_has(&h, "AI assistant connected"));

    let r = a.call(&mut h, "plot", json!({"y": ["B"]}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(graph_count(&h), 1);
    let graph = project(&h).windows.iter().find(|w| matches!(w, Window::Graph(_))).unwrap().id();
    assert!(h.state().workspace.contains(graph), "the graph window opens like one the owner made");
    let label = h.state().doc.undo_label().unwrap().to_string();
    assert!(label.starts_with("AI: plot "), "{label}");
    assert!(log_has(&h, &label), "the Messages Log names the undo step");

    // The chip counts the assistant, and its tooltip names it and its folders.
    assert!(h.query_by_label_contains("AI: connected (1)").is_some(), "the status bar shows the chip");

    menu(&mut h, "Edit", "Undo AI: plot");
    assert_eq!(graph_count(&h), 0, "one Undo removes the plot");
    assert_eq!(h.state().doc.undo_label(), Some("AI: import_files sample.csv"));
    assert_eq!(project(&h).windows.len(), 1);
    a.end();
}

#[test]
fn a_failing_call_is_a_warning_and_a_log_line_not_a_message_box() {
    let dirs = Dirs::new("fail");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    let r = a.call(&mut h, "import_files", json!({"files": ["/definitely/not/there.csv"]}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(h.state().message.is_none(), "no modal for a failed call");
    assert!(log_has(&h, "AI: import_files failed"), "{:?}", h.state().logs.messages);
    assert!(h.state().status.starts_with("AI: import_files failed"), "{}", h.state().status);
    assert!(!h.state().doc.dirty, "a failure leaves the project alone");
    assert!(h.state().doc.undo_label().is_none(), "and no undo step");
    a.end();
}

#[test]
fn a_change_waits_for_a_dialog_while_a_read_is_answered_at_once() {
    let dirs = Dirs::new("modal");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut writer = Assistant::start(&dirs);
    let mut reader = Assistant::start(&dirs);
    // Both are connected before the dialog opens.
    let r = reader.call(&mut h, "list_contents", json!({}));
    assert_eq!(r["isError"], false, "{r}");
    let r = writer.call(&mut h, "list_contents", json!({}));
    assert_eq!(r["isError"], false, "{r}");

    h.state_mut().about = true;
    settle(&mut h);
    let import = writer.request("import_files", json!({"files": [dirs.csv()]}));
    until(&mut h, "the call to be queued", |h| h.state().live.waiting() == 1);
    for _ in 0..10 {
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(writer.find(import).is_none(), "the change does not run under a dialog");
    assert!(project(&h).windows.is_empty());

    // A read is not stuck behind it.
    let r = reader.call(&mut h, "list_contents", json!({}));
    assert_eq!(r["isError"], false, "{r}");
    assert!(writer.find(import).is_none());
    assert_eq!(h.state().live.waiting(), 1);

    h.state_mut().about = false;
    let r = writer.wait(&mut h, import);
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(project(&h).windows.len(), 1, "it ran after the dialog closed");
    assert_eq!(h.state().live.waiting(), 0);
    writer.end();
    reader.end();
}

#[test]
fn a_change_waits_while_a_cell_is_being_edited() {
    let dirs = Dirs::new("edit");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    a.call(&mut h, "import_files", json!({"files": [dirs.csv()]}));
    let book = project(&h).windows[0].id();
    let column = project(&h).workbook(book).unwrap().sheet().columns[0].id;
    h.state_mut().books.entry(book).or_default().editing = Some((column, 0, "1".to_string()));
    let id = a.request("sort_sheet", json!({"key": "A", "descending": true}));
    until(&mut h, "the call to be queued", |h| h.state().live.waiting() == 1);
    for _ in 0..10 {
        h.step();
    }
    assert!(a.find(id).is_none(), "the edit goes first");
    h.state_mut().books.entry(book).or_default().editing = None;
    let r = a.wait(&mut h, id);
    assert!(r.get("error").is_none(), "{r}");
    assert_eq!(h.state().live.waiting(), 0);
    a.end();
}

#[test]
fn only_one_change_runs_per_frame_and_the_replies_stay_in_order() {
    let dirs = Dirs::new("one");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    let mut b = Assistant::start(&dirs);
    a.call(&mut h, "list_contents", json!({}));
    b.call(&mut h, "list_contents", json!({}));
    h.state_mut().about = true;
    settle(&mut h);
    let x = a.request("import_files", json!({"files": [dirs.csv()]}));
    let y = b.request("import_files", json!({"files": [dirs.csv()]}));
    until(&mut h, "both calls to be queued", |h| h.state().live.waiting() == 2);
    h.state_mut().about = false;
    h.step();
    assert!(h.state().live.waiting() <= 1, "at least one ran");
    assert!(h.state().live.waiting() >= 1 || project(&h).windows.len() == 1);
    a.wait(&mut h, x);
    b.wait(&mut h, y);
    assert_eq!(project(&h).windows.len(), 2, "both ran");
    // Two separate undo steps.
    assert_eq!(h.state().doc.undo_label(), Some("AI: import_files sample.csv"));
    h.state_mut().doc.undo();
    assert_eq!(h.state().doc.undo_label(), Some("AI: import_files sample.csv"));
    a.end();
    b.end();
}

#[test]
fn the_owner_starting_a_new_project_drops_the_connections() {
    let dirs = Dirs::new("new");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    a.call(&mut h, "list_contents", json!({}));
    assert_eq!(h.state().live.clients().len(), 1);
    menu_path(&mut h, &["File", "New", "Project"]);
    until(&mut h, "the connection to be dropped", |h| h.state().live.clients().is_empty());
    assert!(log_has(&h, "The project was replaced"), "{:?}", h.state().logs.messages);
    assert!(h.state().live.active(), "the link itself stays on");
    // The assistant hears it once, on its next call, and can go on after looking again.
    let r = a.call(&mut h, "list_contents", json!({}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(text(&r).contains("replaced"), "{}", text(&r));
    let r = a.call(&mut h, "list_contents", json!({}));
    assert_eq!(r["isError"], false, "{r}");
    a.end();
}

#[test]
fn unticking_with_a_client_connected_stops_promptly_and_removes_the_files() {
    let dirs = Dirs::new("off");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    a.call(&mut h, "list_contents", json!({}));
    assert!(dirs.run().join("mcp.json").exists());
    let before = Instant::now();
    tick_link(&mut h);
    assert!(before.elapsed() < Duration::from_secs(5), "the owner is not kept waiting: {:?}", before.elapsed());
    assert!(!h.state().live.active());
    assert!(h.state().live.clients().is_empty());
    assert!(!dirs.run().join("mcp.json").exists());
    assert!(!dirs.run().join("mcp.sock").exists());
    assert!(h.query_by_label_contains("AI: connected").is_none(), "the chip goes with the link");
    // The assistant is told, not left hanging.
    let r = a.call(&mut h, "list_contents", json!({}));
    assert_eq!(r["isError"], true, "{r}");
    // The lock is free again: ticking on works.
    tick_link(&mut h);
    assert!(h.state().live.active());
    a.end();
}

#[test]
fn a_waiting_call_is_answered_when_the_owner_turns_the_link_off() {
    let dirs = Dirs::new("offq");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    a.call(&mut h, "list_contents", json!({}));
    h.state_mut().about = true;
    settle(&mut h);
    let id = a.request("import_files", json!({"files": [dirs.csv()]}));
    until(&mut h, "the call to be queued", |h| h.state().live.waiting() == 1);
    h.state_mut().about = false;
    // Off before the next frame can run it.
    h.state_mut().live.stop();
    let mut reply = None;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) && reply.is_none() {
        reply = a.find(id);
        std::thread::sleep(Duration::from_millis(5));
    }
    let reply = reply.expect("the waiting call got an answer");
    assert!(project(&h).windows.is_empty(), "it never ran");
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    assert_eq!(text(&reply["result"]), CLOSED_FOR_ASSISTANT, "{reply}");
    a.end();
}

#[test]
fn the_attached_app_refuses_to_save_a_project_for_the_assistant() {
    let dirs = Dirs::new("save");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let target = dirs.root().join("out.sheaf");
    let options = ClientOptions::new("raw", vec![dirs.root().to_string_lossy().into_owned()]);
    let mut link = client::connect(&[dirs.run()], &options).unwrap();
    let call = Request::Call { id: 1, name: "save_project".into(), args: json!({"path": target}), structured: false };
    link.writer.write_all(format!("{}\n", call.to_line()).as_bytes()).unwrap();
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let line = match frame::read_line(&mut link.reader, frame::REPLY_CAP, &mut || true) {
            Ok(Line::Text(t)) => Some(t),
            _ => None,
        };
        let _ = tx.send(line);
    });
    let start = Instant::now();
    let line = loop {
        h.step();
        if let Ok(line) = rx.try_recv() {
            break line.expect("a reply");
        }
        assert!(start.elapsed() < Duration::from_secs(15), "no reply");
        std::thread::sleep(Duration::from_millis(5));
    };
    let reply: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    assert!(!target.exists(), "nothing was written");
    assert!(h.state().path.is_none(), "and the project has no new file name");
}

#[test]
fn an_assistant_cannot_replace_a_file_next_to_the_project() {
    let dirs = Dirs::new("keep");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    a.call(&mut h, "import_files", json!({"files": [dirs.csv()]}));
    let existing = dirs.root().join("keep.csv");
    std::fs::write(&existing, "mine").unwrap();
    let r = a.call(&mut h, "export_data", json!({"path": existing.to_string_lossy()}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(text(&r).contains("overwrite"), "{}", text(&r));
    assert_eq!(std::fs::read_to_string(&existing).unwrap(), "mine");
    a.end();
}

#[test]
fn the_chip_goes_when_the_assistant_ends() {
    let dirs = Dirs::new("chip");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    a.call(&mut h, "list_contents", json!({}));
    until(&mut h, "the chip", |h| h.query_by_label_contains("AI: connected (1)").is_some());
    a.end();
    until(&mut h, "the assistant to be gone", |h| {
        h.state().live.clients().is_empty() && h.query_by_label_contains("AI: connected").is_none()
    });
    assert!(log_has(&h, "AI assistant disconnected"), "{:?}", h.state().logs.messages);
    assert!(h.state().live.active(), "the link stays on");
}

#[test]
fn the_tooltip_and_the_log_show_a_client_name_without_bidi_or_control_characters() {
    let dirs = Dirs::new("bidi");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let roots = vec![dirs.root().to_string_lossy().into_owned()];
    let link = client::connect(&[dirs.run()], &ClientOptions::new("ev\u{202e}il\u{7}-client\u{200b}", roots)).unwrap();
    until(&mut h, "the client to show", |h| !h.state().live.clients().is_empty());
    let tip = crate::live::tooltip(h.state().live.clients());
    assert!(tip.contains("evil-client"), "{tip}");
    assert!(!tip.contains('\u{202e}') && !tip.contains('\u{7}') && !tip.contains('\u{200b}'), "{tip:?}");
    let displayed_root = sheaf_mcp::roots::plain(dirs.root());
    assert!(tip.contains(displayed_root.to_string_lossy().as_ref()), "the tooltip names the folders: {tip}");
    assert!(log_has(&h, "evil-client"), "{:?}", h.state().logs.messages);
    assert!(
        !h.state().logs.messages.iter().any(|m| m.text.contains('\u{202e}')),
        "no bidi character reached the Messages Log"
    );
    drop(link);
}

#[test]
fn a_change_waits_under_the_fit_more_dialog_too() {
    let dirs = Dirs::new("more");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    a.call(&mut h, "import_files", json!({"files": [dirs.csv()]}));
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::FitMore { book, column: None, kind: FitMoreKind::Multiple }, &ctx);
    settle(&mut h);
    assert!(h.state().fit_more.is_some() && h.state().dialog_open(), "the dialog counts as open");
    let id = a.request("sort_sheet", json!({"key": "A", "descending": true}));
    until(&mut h, "the call to be queued", |h| h.state().live.waiting() == 1);
    for _ in 0..10 {
        h.step();
    }
    assert!(a.find(id).is_none(), "the change does not run under the dialog");
    h.state_mut().fit_more = None;
    let r = a.wait(&mut h, id);
    assert!(r.get("error").is_none(), "{r}");
    assert_eq!(h.state().live.waiting(), 0);
    a.end();
}
