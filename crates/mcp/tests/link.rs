//! The link between `sheaf-mcp --attach` and the app (docs/spec/mcp.md section 11), tested with the real
//! proxy code over a real socket (a Unix socket, and loopback TCP with its token) and a LinkServer in this
//! process whose jobs a test thread runs the way the app's UI thread will.

use serde_json::{Value, json};
use sheaf_core::Document;
use sheaf_mcp::files::Protected;
use sheaf_mcp::link::client::{self, ClientOptions};
use sheaf_mcp::link::discovery;
use sheaf_mcp::link::frame::{self, Line};
use sheaf_mcp::link::host::{Event, Job, LinkConfig, LinkServer, StartError, sweep};
use sheaf_mcp::link::proto::{self, NoticeKind, Request};
use sheaf_mcp::link::transport::{Endpoint, Kind, Listener, Stream};
use sheaf_mcp::proxy::{self, ProxyConfig};
use sheaf_mcp::roots::Roots;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

// ----- helpers ----------------------------------------------------------------------

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let base = std::env::temp_dir().canonicalize().unwrap().join(format!("sl-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("root")).unwrap();
        Scratch(base)
    }

    /// The run folder; the app makes it.
    fn run(&self) -> PathBuf {
        self.0.join("run")
    }

    fn root(&self) -> PathBuf {
        self.0.join("root")
    }

    fn roots(&self) -> Vec<String> {
        ProxyConfig::new(&Roots::new(&[self.root()]).unwrap()).roots
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn kinds() -> Vec<Kind> {
    let mut v = vec![Kind::native()];
    if Kind::native() != Kind::Tcp {
        v.push(Kind::Tcp);
    }
    v
}

fn eventually(what: &str, mut cond: impl FnMut() -> bool) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {what}");
}

/// The thread that stands in for the app's UI thread.
struct Executor {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    /// Milliseconds a job stays running before it does its work; 0 for none.
    slow: Arc<AtomicU64>,
    log: Arc<Mutex<Vec<String>>>,
    doc: Arc<Mutex<Document>>,
    /// While paused and holding ([`Executor::hold`]), the jobs taken from the link but not run, so a test
    /// can wait until a call is really queued instead of sleeping (T240).
    holding: Arc<AtomicBool>,
    held: Arc<Mutex<Vec<Job>>>,
    handle: Option<JoinHandle<()>>,
}

impl Executor {
    fn start(server: Arc<LinkServer>, paused: bool) -> Executor {
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(paused));
        let slow = Arc::new(AtomicU64::new(0));
        let log = Arc::new(Mutex::new(Vec::new()));
        let doc = Arc::new(Mutex::new(Document::default()));
        let holding = Arc::new(AtomicBool::new(false));
        let held: Arc<Mutex<Vec<Job>>> = Arc::new(Mutex::new(Vec::new()));
        let handle = {
            let (stop, paused, slow, log, doc) = (stop.clone(), paused.clone(), slow.clone(), log.clone(), doc.clone());
            let (holding, held) = (holding.clone(), held.clone());
            std::thread::spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    if paused.load(Ordering::Acquire) {
                        if holding.load(Ordering::Acquire) {
                            held.lock().unwrap().extend(server.poll_jobs());
                        }
                    } else {
                        // Held jobs first, in order; one that was cancelled meanwhile does not start.
                        let mut jobs: Vec<Job> = held.lock().unwrap().drain(..).collect();
                        jobs.extend(server.poll_jobs());
                        for job in jobs {
                            let mut doc = doc.lock().unwrap();
                            let delay = slow.load(Ordering::Acquire);
                            let done = if delay > 0 {
                                job.begin().then(|| {
                                    std::thread::sleep(Duration::from_millis(delay));
                                    job.run(&mut doc, Protected::default())
                                })
                            } else {
                                job.execute(&mut doc, Protected::default())
                            };
                            if let Some(done) = done {
                                log.lock().unwrap().push(done.tool);
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(3));
                }
            })
        };
        Executor { stop, paused, slow, log, doc, holding, held, handle: Some(handle) }
    }

    fn pause(&self, on: bool) {
        self.paused.store(on, Ordering::Release);
    }

    /// Pauses and takes the jobs off the link without running them (see `held`).
    fn hold(&self) {
        self.holding.store(true, Ordering::Release);
        self.pause(true);
    }

    /// How many jobs are held and still waiting to start.
    fn held_queued(&self) -> usize {
        self.held.lock().unwrap().iter().filter(|j| j.is_queued()).count()
    }

    fn ran(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }

    /// The log once it holds at least `n` jobs: a job is logged after its reply has gone out, so a test
    /// that has its reply waits for the entry (T240).
    fn ran_at_least(&self, n: usize) -> Vec<String> {
        eventually("the job log", || self.ran().len() >= n);
        self.ran()
    }

    fn end(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            h.join().unwrap();
        }
    }
}

struct App {
    scratch: Scratch,
    server: Arc<LinkServer>,
    exec: Executor,
}

fn app_with(tag: &str, kind: Kind, paused: bool, tweak: impl FnOnce(&mut LinkConfig)) -> App {
    let scratch = Scratch::new(tag);
    let mut cfg = LinkConfig::new("9.9.9");
    cfg.kind = kind;
    tweak(&mut cfg);
    let server = Arc::new(LinkServer::start(&scratch.run(), cfg).unwrap());
    let exec = Executor::start(server.clone(), paused);
    App { scratch, server, exec }
}

fn app(tag: &str, kind: Kind) -> App {
    app_with(tag, kind, false, |_| {})
}

impl App {
    fn proxy_config(&self) -> ProxyConfig {
        let mut cfg = ProxyConfig::new(&Roots::new(&[self.scratch.root()]).unwrap());
        cfg.candidates = vec![self.scratch.run()];
        cfg
    }

    fn assistant(&self) -> Assistant {
        Assistant::start(self.proxy_config())
    }

    /// Stops the executor and then the link; how long the stop took.
    fn stop(self) -> Duration {
        self.stop_keep().0
    }

    /// Like [`App::stop`], and the scratch folder is kept.
    fn stop_keep(mut self) -> (Duration, Scratch) {
        self.exec.end();
        let server = Arc::try_unwrap(self.server).ok().expect("only the executor held the server");
        let start = Instant::now();
        server.stop();
        (start.elapsed(), self.scratch)
    }
}

struct ChannelReader {
    rx: Receiver<Vec<u8>>,
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

struct Shared(Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// An MCP client talking to the real proxy through pipes made of channels.
struct Assistant {
    tx: Option<Sender<Vec<u8>>>,
    out: Arc<Mutex<Vec<u8>>>,
    handle: Option<JoinHandle<i32>>,
}

impl Assistant {
    fn start(cfg: ProxyConfig) -> Assistant {
        let (tx, rx) = channel();
        let out = Arc::new(Mutex::new(Vec::new()));
        let writer = out.clone();
        let handle = std::thread::spawn(move || {
            let mut w = Shared(writer);
            proxy::run(cfg, BufReader::new(ChannelReader { rx, buf: Vec::new(), pos: 0 }), &mut w)
        });
        Assistant { tx: Some(tx), out, handle: Some(handle) }
    }

    fn send(&self, v: Value) {
        self.tx.as_ref().unwrap().send(format!("{v}\n").into_bytes()).unwrap();
    }

    fn lines(&self) -> Vec<Value> {
        let text = String::from_utf8_lossy(&self.out.lock().unwrap()).into_owned();
        text.lines().map(|l| serde_json::from_str(l).unwrap()).collect()
    }

    fn find(&self, id: u64) -> Option<Value> {
        self.lines().into_iter().find(|l| l["id"] == id)
    }

    fn reply(&self, id: u64) -> Value {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(15) {
            if let Some(v) = self.find(id) {
                return v;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("no reply to {id}; got {:?}", self.lines());
    }

    fn silent_for(&self, id: u64, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
        assert!(self.find(id).is_none(), "{id} was answered: {:?}", self.find(id));
    }

    fn init(&self) {
        self.send(json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test-client", "version": "1.0"}}}));
        assert_eq!(self.reply(1)["result"]["serverInfo"]["name"], "sheaf-mcp");
    }

    fn request(&self, id: u64, name: &str, args: Value) {
        self.send(
            json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": args}}),
        );
    }

    /// The `result` of a tool call.
    fn call(&self, id: u64, name: &str, args: Value) -> Value {
        self.request(id, name, args);
        let r = self.reply(id);
        assert!(r.get("error").is_none(), "{r}");
        r["result"].clone()
    }

    fn finish(mut self) -> i32 {
        self.tx = None;
        self.handle.take().unwrap().join().unwrap()
    }
}

fn text(result: &Value) -> String {
    result["content"][0]["text"].as_str().unwrap_or_default().to_string()
}

/// A raw client that has said hello.
fn raw(dir: &Path, roots: Vec<String>) -> Result<client::Link, String> {
    client::connect(&[dir.to_path_buf()], &ClientOptions::new("raw", roots))
}

fn read_text(link: &mut client::Link) -> Option<String> {
    match frame::read_line(&mut link.reader, frame::REPLY_CAP, &mut || true) {
        Ok(Line::Text(t)) => Some(t),
        _ => None,
    }
}

// ----- the handshake ----------------------------------------------------------------

#[test]
fn a_call_goes_to_the_app_and_back_on_every_transport() {
    for kind in kinds() {
        let a = app("call", kind);
        let who = a.assistant();
        who.init();
        who.send(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
        let tools = who.reply(2)["result"]["tools"].clone();
        let names: Vec<&str> = tools.as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names.len(), 17, "{names:?}");
        for hidden in ["new_project", "open_project", "save_project"] {
            assert!(!names.contains(&hidden), "{hidden} is hidden");
        }
        let r = who.call(3, "list_contents", json!({}));
        assert_eq!(r["isError"], false, "{r}");
        assert!(r["structuredContent"].is_object(), "{r}");
        // The app sees who is connected, and what they may touch.
        let conns = a.server.connections();
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].client, "test-client 1.0");
        assert_eq!(conns[0].roots, a.scratch.roots());
        assert_eq!(who.finish(), 0);
        eventually("the disconnect", || a.server.connections().is_empty());
        let events = a.server.poll_events();
        assert!(matches!(events.as_slice(), [Event::Connected(c), Event::Disconnected(d)] if c == d), "{events:?}");
        a.stop();
    }
}

#[test]
fn a_change_made_through_the_link_is_one_named_undo_step() {
    let a = app("undo", Kind::native());
    std::fs::write(a.scratch.root().join("a.csv"), "x,y\n1,2\n2,4\n3,6\n").unwrap();
    let who = a.assistant();
    who.init();
    let r = who.call(2, "import_files", json!({"files": ["a.csv"]}));
    assert_eq!(r["isError"], false, "{r}");
    {
        let mut doc = a.exec.doc.lock().unwrap();
        assert_eq!(doc.project().workbooks().count(), 1);
        assert_eq!(doc.undo_label(), Some("AI: import_files a.csv"));
        doc.undo();
        assert_eq!(doc.project().workbooks().count(), 0);
    }
    // A failing call leaves no step.
    let r = who.call(3, "set_column_values", json!({"column": "Z", "formula": "1"}));
    assert_eq!(r["isError"], true);
    assert_eq!(a.exec.doc.lock().unwrap().undo_label(), None);
    // The tools that open or save are refused by the proxy and the app is not asked.
    let r = who.call(4, "save_project", json!({"path": "x.opj"}));
    assert_eq!((r["isError"].clone(), text(&r)), (json!(true), proto_not_attached()));
    who.finish();
    a.stop();
}

fn proto_not_attached() -> String {
    sheaf_mcp::exec::NOT_ATTACHED.to_string()
}

#[test]
fn the_app_refuses_a_relative_root_a_missing_root_and_a_bad_first_line() {
    for kind in kinds() {
        let a = app("hello", kind);
        let dir = a.scratch.run();
        assert!(raw(&dir, vec!["relative/dir".into()]).err().unwrap().contains("not an absolute path"));
        let missing = a.scratch.0.join("nowhere").to_string_lossy().into_owned();
        assert!(raw(&dir, vec![missing]).is_err());
        // A file is no folder either.
        std::fs::write(a.scratch.0.join("f"), b"x").unwrap();
        assert!(raw(&dir, vec![a.scratch.0.join("f").to_string_lossy().into_owned()]).is_err());
        // Text that is not a hello gets no answer, and the connection is closed.
        let info = discovery::probe(&dir, Duration::from_secs(1)).unwrap();
        let mut s = Stream::connect(&info.endpoint).unwrap();
        s.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        let mut got = Vec::new();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let _ = s.read_to_end(&mut got);
        assert!(got.is_empty(), "{got:?}");
        assert!(a.server.connections().is_empty());
        a.stop();
    }
}

#[test]
fn the_proxy_sends_absolute_roots_only() {
    let scratch = Scratch::new("abs");
    let here = std::env::current_dir().unwrap();
    let roots = Roots::new(&[PathBuf::from("."), PathBuf::from("src")]).unwrap();
    let cfg = ProxyConfig::new(&roots);
    assert_eq!(cfg.roots.len(), 2);
    let expected_root = sheaf_mcp::roots::plain(here.canonicalize().unwrap());
    for r in &cfg.roots {
        assert!(Path::new(r).is_absolute() && Path::new(r).starts_with(&expected_root), "{r}");
    }
    drop(scratch);
}

#[test]
fn the_client_name_and_roots_are_cleaned_before_the_app_keeps_them() {
    let a = app("clean", Kind::native());
    let roots = a.scratch.roots();
    let link = client::connect(
        &[a.scratch.run()],
        &ClientOptions::new("ev\u{202e}il\u{0007} \u{200b}client with a very long name that goes on and on", roots),
    )
    .unwrap();
    let shown = &a.server.connections()[0];
    assert!(shown.client.starts_with("evil client"), "{}", shown.client);
    assert!(shown.client.chars().count() <= 40 && !shown.client.contains('\u{202e}'), "{}", shown.client);
    drop(link);
    a.stop();
}

#[test]
fn a_tcp_hello_needs_the_token_and_a_unix_one_needs_none() {
    let a = app("token", Kind::Tcp);
    let info = discovery::probe(&a.scratch.run(), Duration::from_secs(1)).unwrap();
    let Endpoint::Tcp { token, .. } = &info.endpoint else { panic!("{info:?}") };
    assert_eq!(token.len(), 64);
    for hello in [
        json!({"hello": {"v": 1, "client": "x", "roots": []}}),
        json!({"hello": {"v": 1, "client": "x", "roots": [], "token": "0".repeat(64)}}),
        json!({"hello": {"v": 1, "client": "x", "roots": [], "token": "short"}}),
    ] {
        let mut s = Stream::connect(&info.endpoint).unwrap();
        frame::write_line(&mut s, &hello.to_string()).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(s);
        let Ok(Line::Text(reply)) = frame::read_line(&mut reader, 1000, &mut || false) else { panic!("no reply") };
        assert_eq!(proto::parse_hello_reply(&reply), Err(proto::HelloFail::Refused("not allowed".into())), "{hello}");
        assert!(matches!(frame::read_line(&mut reader, 1000, &mut || false), Ok(Line::Eof) | Err(_)));
    }
    assert!(a.server.connections().is_empty());
    // The right token is welcome.
    assert!(raw(&a.scratch.run(), vec![]).is_ok());
    a.stop();
}

#[test]
fn a_connection_that_never_says_hello_is_closed_and_the_ninth_is_refused() {
    let a = app_with("nohello", Kind::native(), false, |c| {
        c.hello_timeout = Duration::from_millis(300);
        c.max_connections = 2;
    });
    let info = discovery::probe(&a.scratch.run(), Duration::from_secs(1)).unwrap();
    let mut silent = Stream::connect(&info.endpoint).unwrap();
    silent.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let start = Instant::now();
    let mut buf = [0u8; 16];
    assert!(matches!(silent.read(&mut buf), Ok(0) | Err(_)));
    assert!(start.elapsed() < Duration::from_secs(3), "closed after the hello deadline");

    let (l1, l2) = (raw(&a.scratch.run(), vec![]).unwrap(), raw(&a.scratch.run(), vec![]).unwrap());
    assert_eq!(raw(&a.scratch.run(), vec![]).err().as_deref(), Some(proto::BUSY_APP));
    drop((l1, l2));
    eventually("room for another", || raw(&a.scratch.run(), vec![]).is_ok());
    a.stop();
}

#[test]
fn a_full_tcp_host_delivers_busy_repeatedly_and_a_silent_rejected_peer_cannot_hold_accept() {
    let a = app_with("busy-tcp", Kind::Tcp, false, |config| config.max_connections = 1);
    let held = raw(&a.scratch.run(), vec![]).unwrap();
    for attempt in 0..32 {
        assert_eq!(raw(&a.scratch.run(), vec![]).err().as_deref(), Some(proto::BUSY_APP), "attempt {attempt}");
        assert_eq!(a.server.connections().len(), 1, "a busy reply must not admit another client");
    }
    let info = discovery::probe(&a.scratch.run(), Duration::from_secs(1)).unwrap();
    let silent = Stream::connect(&info.endpoint).unwrap();
    let start = Instant::now();
    assert_eq!(raw(&a.scratch.run(), vec![]).err().as_deref(), Some(proto::BUSY_APP));
    assert!(start.elapsed() < Duration::from_secs(3), "a rejected silent peer must not block later refusals");
    drop((silent, held));
    eventually("room after busy replies", || raw(&a.scratch.run(), vec![]).is_ok());
    a.stop();
}

#[test]
fn a_peer_that_sends_a_byte_now_and_then_is_cut_off_at_the_hello_deadline() {
    let a = app_with("trickle", Kind::native(), false, |c| {
        c.hello_timeout = Duration::from_millis(600);
        c.max_connections = 2;
    });
    let info = discovery::probe(&a.scratch.run(), Duration::from_secs(1)).unwrap();
    let mut slow = Stream::connect(&info.endpoint).unwrap();
    slow.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
    let start = Instant::now();
    // One byte every 100 ms is never a read timeout on the app's side (250 ms), yet it is no hello.
    let mut closed = false;
    while start.elapsed() < Duration::from_secs(5) && !closed {
        closed = slow.write_all(b" ").is_err() || {
            let mut buf = [0u8; 8];
            matches!(slow.read(&mut buf), Ok(0))
        };
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(closed && start.elapsed() < Duration::from_secs(3), "closed after {:?}", start.elapsed());
    // The slot is free again, and a first line over the small cap is refused with a reason.
    let mut big = Stream::connect(&info.endpoint).unwrap();
    big.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let _ = big.write_all(format!("{}\n", "x".repeat(frame::HELLO_CAP + 10)).as_bytes());
    let mut reader = BufReader::new(big);
    let Ok(Line::Text(reply)) = frame::read_line(&mut reader, 1000, &mut || false) else { panic!("no reply") };
    assert!(reply.contains("too long"), "{reply}");
    assert!(raw(&a.scratch.run(), vec![]).is_ok());
    a.stop();
}

#[test]
fn a_hello_with_many_roots_is_refused_and_repeats_count_once() {
    let a = app("roots", Kind::native());
    let root = a.scratch.roots();
    let many: Vec<String> = (0..=proto::MAX_ROOTS).map(|i| format!("{}/x{i}", root[0])).collect();
    let e = raw(&a.scratch.run(), many).err().unwrap();
    assert!(e.contains("too many roots"), "{e}");
    let repeated = vec![root[0].clone(); 300];
    let link = raw(&a.scratch.run(), repeated).unwrap();
    assert_eq!(a.server.connections()[0].roots.len(), 1);
    drop(link);
    a.stop();
}

// ----- requests ---------------------------------------------------------------------

#[test]
fn a_second_call_in_flight_and_an_oversize_line_are_errors() {
    let a = app_with("one", Kind::native(), true, |_| {});
    let mut link = raw(&a.scratch.run(), vec![]).unwrap();
    let call = |id| Request::Call { id, name: "list_contents".into(), args: json!({}), structured: false };
    frame::write_line(&mut link.writer, &call(1).to_line()).unwrap();
    frame::write_line(&mut link.writer, &call(2).to_line()).unwrap();
    let reply: Value = serde_json::from_str(&read_text(&mut link).unwrap()).unwrap();
    assert_eq!(reply["id"], 2);
    assert_eq!(reply["error"]["message"], proto::ONE_CALL);
    // Garbage and an unknown tool are answered and the connection stays.
    frame::write_line(&mut link.writer, "nonsense").unwrap();
    assert!(read_text(&mut link).unwrap().contains("parse error"));
    let unknown = Request::Call { id: 9, name: "rm_rf".into(), args: json!({}), structured: false };
    frame::write_line(&mut link.writer, &unknown.to_line()).unwrap();
    let reply: Value = serde_json::from_str(&read_text(&mut link).unwrap()).unwrap();
    assert_eq!((reply["id"].clone(), reply["error"]["code"].clone()), (json!(9), json!(proto::CODE_PARAMS)));
    // A line over 8 MiB is answered with an error and the connection is closed.
    let big = format!("{}\n", "x".repeat(8 * 1024 * 1024 + 10));
    link.writer.write_all(big.as_bytes()).unwrap();
    assert!(read_text(&mut link).unwrap().contains("longer than 8 MB"));
    // The call that still waited is answered as the connection goes, and then it is closed.
    let lost = read_text(&mut link).unwrap();
    assert!(lost.contains(proto::LOST) && lost.contains("\"id\":1"), "{lost}");
    assert!(read_text(&mut link).is_none());
    a.stop();
}

#[test]
fn a_tool_that_reads_no_project_is_answered_while_the_owner_is_busy() {
    let a = app_with("pure", Kind::native(), true, |_| {});
    let who = a.assistant();
    who.init();
    let r = who.call(2, "list_fit_models", json!({}));
    assert_eq!(r["isError"], false, "{r}");
    let r = who.call(3, "list_analysis_tools", json!({}));
    assert_eq!(r["isError"], false, "{r}");
    // A read of the project waits for the owner's thread.
    who.request(4, "list_contents", json!({}));
    who.silent_for(4, 200);
    a.exec.pause(false);
    assert_eq!(who.reply(4)["result"]["isError"], false);
    who.finish();
    a.stop();
}

#[test]
fn ping_is_answered_while_a_call_waits() {
    let a = app_with("ping", Kind::native(), true, |_| {});
    let who = a.assistant();
    who.init();
    who.request(2, "list_contents", json!({}));
    who.send(json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}));
    assert_eq!(who.reply(3)["result"], json!({}));
    assert!(who.find(2).is_none(), "the call still waits");
    // A request that needs the app waits its turn and keeps its place.
    who.send(json!({"jsonrpc": "2.0", "id": 4, "method": "resources/list"}));
    who.silent_for(4, 100);
    a.exec.pause(false);
    assert_eq!(who.reply(2)["result"]["isError"], false);
    assert!(who.reply(4)["result"]["resources"].is_array());
    who.finish();
    a.stop();
}

#[test]
fn a_cancelled_call_that_was_still_queued_never_runs() {
    for kind in kinds() {
        let a = app_with("cancel", kind, true, |_| {});
        let who = a.assistant();
        who.init();
        who.request(7, "list_contents", json!({}));
        std::thread::sleep(Duration::from_millis(100));
        who.send(json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 7}}));
        // The app confirms and the proxy sends nothing for request 7.
        who.send(json!({"jsonrpc": "2.0", "id": 8, "method": "ping"}));
        who.reply(8);
        who.silent_for(7, 300);
        a.exec.pause(false);
        // Another call works, which also shows the connection is free again.
        let r = who.call(9, "list_contents", json!({}));
        assert_eq!(r["isError"], false);
        assert_eq!(a.exec.ran_at_least(1), ["list_contents"], "only the second call ran");
        who.finish();
        a.stop();
    }
}

#[test]
fn a_call_that_waits_too_long_is_cancelled_and_says_that_it_did_not_run() {
    let a = app_with("busy", Kind::native(), true, |_| {});
    let mut cfg = a.proxy_config();
    cfg.wait = Duration::from_millis(300);
    let who = Assistant::start(cfg);
    who.init();
    let r = who.call(2, "list_contents", json!({}));
    assert_eq!((r["isError"].clone(), text(&r)), (json!(true), proto::OWNER_BUSY.to_string()));
    a.exec.pause(false);
    // The call was cancelled in the app, so it never runs; the next one uses a new connection.
    let r = who.call(3, "list_contents", json!({}));
    assert_eq!(r["isError"], false);
    assert_eq!(a.exec.ran_at_least(1), ["list_contents"]);
    who.finish();
    a.stop();
}

#[test]
fn a_call_that_runs_too_long_says_so_and_is_not_repeated() {
    let a = app("slow", Kind::native());
    a.exec.slow.store(1500, Ordering::Release);
    let mut cfg = a.proxy_config();
    cfg.wait = Duration::from_millis(300);
    cfg.cancel_grace = Duration::from_secs(2);
    let who = Assistant::start(cfg);
    who.init();
    let r = who.call(2, "list_contents", json!({}));
    assert_eq!((r["isError"].clone(), text(&r)), (json!(true), proto::STILL_RUNNING.to_string()));
    // The job finishes into a connection that is gone; it ran once.
    eventually("the slow job", || a.exec.ran().len() == 1);
    a.exec.slow.store(0, Ordering::Release);
    assert_eq!(who.call(3, "list_contents", json!({}))["isError"], false);
    assert_eq!(a.exec.ran_at_least(2).len(), 2);
    who.finish();
    a.stop();
}

#[test]
fn a_call_that_waits_is_answered_after_the_end_of_input() {
    let a = app_with("eof", Kind::native(), true, |_| {});
    let mut who = a.assistant();
    who.init();
    who.request(2, "list_contents", json!({}));
    std::thread::sleep(Duration::from_millis(100));
    // The script closes stdin right after its last line: the call still runs and is answered.
    who.tx = None;
    std::thread::sleep(Duration::from_millis(100));
    a.exec.pause(false);
    assert_eq!(who.reply(2)["result"]["isError"], false);
    assert_eq!(who.finish(), 0);
    assert_eq!(a.exec.ran_at_least(1), ["list_contents"]);
    a.stop();
}

#[test]
fn calls_read_before_the_end_of_input_are_all_answered_in_order() {
    let a = app("eof-two", Kind::native());
    let mut who = a.assistant();
    who.init();
    who.request(2, "list_contents", json!({}));
    who.request(3, "list_contents", json!({}));
    who.tx = None;
    assert_eq!(who.reply(2)["result"]["isError"], false);
    assert_eq!(who.reply(3)["result"]["isError"], false);
    assert_eq!(who.finish(), 0);
    a.stop();
}

#[test]
fn a_cancelled_call_that_waited_behind_another_never_runs() {
    let a = app_with("cancel-backlog", Kind::native(), true, |_| {});
    a.exec.hold();
    let who = a.assistant();
    who.init();
    // A is with the app (queued); B waits in the proxy behind it.
    who.request(2, "list_contents", json!({}));
    who.request(3, "list_contents", json!({}));
    eventually("call 2 to wait in Sheaf", || a.exec.held_queued() == 1);
    std::thread::sleep(Duration::from_millis(50));
    for id in [3, 2] {
        who.send(json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": id}}));
    }
    who.send(json!({"jsonrpc": "2.0", "id": 4, "method": "ping"}));
    who.reply(4);
    a.exec.pause(false);
    // Another call goes through, which shows that the queue behind the cancelled ones is free.
    assert_eq!(who.call(5, "list_contents", json!({}))["isError"], false);
    assert_eq!(a.exec.ran_at_least(1), ["list_contents"], "only the last call ran");
    who.silent_for(3, 100);
    who.silent_for(2, 0);
    who.finish();
    a.stop();
}

#[test]
fn a_call_cancelled_before_it_arrives_is_never_sent() {
    let a = app("cancel-early", Kind::native());
    let who = a.assistant();
    who.init();
    who.send(json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 6}}));
    who.request(6, "list_contents", json!({}));
    assert_eq!(who.call(7, "list_contents", json!({}))["isError"], false);
    who.silent_for(6, 100);
    assert_eq!(a.exec.ran_at_least(1), ["list_contents"]);
    who.finish();
    a.stop();
}

#[test]
fn a_job_that_waited_55_seconds_is_cancelled_by_the_app_and_never_runs() {
    let a = app_with("sweep", Kind::native(), true, |_| {});
    let mut link = raw(&a.scratch.run(), vec![]).unwrap();
    let call = Request::Call { id: 1, name: "list_contents".into(), args: json!({}), structured: false };
    frame::write_line(&mut link.writer, &call.to_line()).unwrap();
    let mut pending = Vec::new();
    eventually("the job", || {
        pending.extend(a.server.poll_jobs());
        !pending.is_empty()
    });
    let made = pending[0].created();
    // Not yet: it stays queued.
    sweep(&mut pending, Duration::from_secs(55), made + Duration::from_secs(54));
    assert_eq!(pending.len(), 1);
    let job = pending.remove(0);
    pending.push(job);
    sweep(&mut pending, Duration::from_secs(55), made + Duration::from_secs(56));
    assert!(pending.is_empty());
    let reply: Value = serde_json::from_str(&read_text(&mut link).unwrap()).unwrap();
    assert_eq!((reply["id"].clone(), reply["error"]["message"].clone()), (json!(1), json!(proto::OWNER_BUSY)));
    // The connection is free for the next call.
    frame::write_line(&mut link.writer, &Request::ResourcesList { id: 2 }.to_line()).unwrap();
    eventually("the second job", || {
        pending.extend(a.server.poll_jobs());
        !pending.is_empty()
    });
    assert!(pending[0].execute(&mut Document::default(), Protected::default()).is_some());
    a.stop();
}

#[cfg(unix)]
#[test]
fn a_socket_path_that_is_too_long_is_refused_with_the_way_out() {
    let scratch = Scratch::new("long");
    let dir = scratch.0.join("a".repeat(90)).join("run");
    let err = LinkServer::start(&dir, LinkConfig::new("1")).err().unwrap();
    assert!(
        matches!(&err, StartError::Unusable(m) if m.contains("SHEAF_MCP_RUN_DIR") && m.contains("too long")),
        "{err}"
    );
    // The lock was given back.
    assert!(discovery::acquire_lock(&dir).is_ok());
}

// ----- lifecycle --------------------------------------------------------------------

#[test]
fn turning_the_link_off_with_clients_connected_is_prompt_and_leaves_no_files() {
    for kind in kinds() {
        let a = app_with("off", kind, true, |c| c.stop_wait = Duration::from_millis(1000));
        let dir = a.scratch.run();
        let who = a.assistant();
        who.init();
        // A call is queued, a second client is connected and never reads, a third has not said hello.
        who.request(2, "list_contents", json!({}));
        let idle = raw(&dir, vec![]).unwrap();
        let info = discovery::probe(&dir, Duration::from_secs(1)).unwrap();
        let _half = Stream::connect(&info.endpoint).unwrap();
        eventually("both clients", || a.server.connections().len() == 2);
        assert!(dir.join(discovery::INFO_NAME).exists());
        let (took, _scratch) = a.stop_keep();
        assert!(took < Duration::from_millis(1500), "{took:?}");
        for name in [discovery::INFO_NAME, "mcp.sock"] {
            assert!(!dir.join(name).exists(), "{name} was removed");
        }
        // The queued call was answered, and the proxy tells the assistant once.
        let r = who.reply(2)["result"].clone();
        assert_eq!((r["isError"].clone(), text(&r)), (json!(true), proto::CLOSED_FOR_ASSISTANT.to_string()));
        let r = who.call(3, "list_contents", json!({}));
        assert!(text(&r).contains("no Sheaf is listening"), "{r}");
        assert!(text(&r).contains(dir.to_str().unwrap()), "names the folder: {r}");
        // The lock is free again.
        assert!(discovery::acquire_lock(&dir).is_ok());
        drop(idle);
        who.finish();
    }
}

#[test]
fn the_link_can_be_turned_off_and_on_again() {
    for kind in kinds() {
        let scratch = Scratch::new("again");
        let mut cfg = LinkConfig::new("1");
        cfg.kind = kind;
        let first = LinkServer::start(&scratch.run(), cfg.clone()).unwrap();
        let first_endpoint = first.endpoint().clone();
        first.stop();
        let second = Arc::new(LinkServer::start(&scratch.run(), cfg).unwrap());
        if kind == Kind::Tcp {
            assert_ne!(second.endpoint(), &first_endpoint, "a new port and token");
        }
        let exec = Executor::start(second.clone(), false);
        let mut pc = ProxyConfig::new(&Roots::default());
        pc.candidates = vec![scratch.run()];
        let who = Assistant::start(pc);
        who.init();
        assert_eq!(who.call(2, "list_contents", json!({}))["isError"], false);
        who.finish();
        drop(exec);
    }
}

#[test]
fn a_second_app_refuses_to_listen_and_the_first_keeps_the_endpoint() {
    let a = app("second", Kind::native());
    let dir = a.scratch.run();
    let err = LinkServer::start(&dir, LinkConfig::new("2")).err().unwrap();
    assert!(matches!(err, StartError::Busy(_)), "{err}");
    assert!(err.to_string().contains(&std::process::id().to_string()), "names the process: {err}");
    // Looking through several folders ends at the busy one: no second endpoint appears elsewhere.
    let other = a.scratch.0.join("other");
    let err = LinkServer::start_first(&[dir.clone(), other.clone()], &LinkConfig::new("2")).err().unwrap();
    assert!(matches!(err, StartError::Busy(_)));
    assert!(!other.exists());
    let who = a.assistant();
    who.init();
    assert_eq!(who.call(2, "list_contents", json!({}))["isError"], false);
    who.finish();
    a.stop();
}

#[test]
fn the_app_listens_in_the_first_folder_that_can_be_used() {
    let scratch = Scratch::new("first");
    let file = scratch.0.join("file");
    std::fs::write(&file, b"x").unwrap();
    let ok = scratch.run();
    let server = LinkServer::start_first(&[file.clone(), ok.clone()], &LinkConfig::new("1")).unwrap();
    assert_eq!(server.dir(), ok);
    let err = LinkServer::start_first(std::slice::from_ref(&file), &LinkConfig::new("1")).err();
    assert!(matches!(err, Some(StartError::Unusable(m)) if m.contains("not a folder")));
    let err = LinkServer::start_first(&[], &LinkConfig::new("1")).err();
    assert!(matches!(err, Some(StartError::Unusable(_))));
}

#[test]
fn an_old_endpoint_whose_lock_is_dead_is_ignored_and_replaced() {
    for kind in kinds() {
        let scratch = Scratch::new("stale");
        let dir = scratch.run();
        // What a crashed app leaves: a lock file nobody holds, mcp.json, and a socket file.
        discovery::check_dir(&dir, true).unwrap();
        drop(discovery::acquire_lock(&dir).unwrap());
        let old = discovery::Info {
            pid: 1,
            app: "old".into(),
            endpoint: match kind {
                Kind::Tcp => Endpoint::Tcp { addr: "127.0.0.1:9".parse().unwrap(), token: "t".repeat(64) },
                #[cfg(unix)]
                Kind::Unix => Endpoint::Unix(dir.join("mcp.sock")),
            },
        };
        discovery::write_info(&dir, &old).unwrap();
        std::fs::write(dir.join("mcp.sock"), b"stale").unwrap();
        let err = client::connect(std::slice::from_ref(&dir), &ClientOptions::new("c", vec![])).err().unwrap();
        assert!(err.contains("no lock") && err.contains(dir.to_str().unwrap()), "{err}");
        // The app that starts next takes over.
        let mut cfg = LinkConfig::new("new");
        cfg.kind = kind;
        let server = LinkServer::start(&dir, cfg).unwrap();
        let link = client::connect(std::slice::from_ref(&dir), &ClientOptions::new("c", vec![])).unwrap();
        assert_eq!(link.app, "new");
        drop((link, server));
    }
}

#[test]
fn the_client_tries_every_folder_and_names_those_where_nothing_listens() {
    let a = app("many", Kind::native());
    let nowhere = a.scratch.0.join("nowhere");
    let empty = a.scratch.0.join("empty");
    discovery::check_dir(&empty, true).unwrap();
    let opts = ClientOptions::new("c", vec![]);
    // The second folder has the app: the first is only mentioned when nothing is found at all.
    assert!(client::connect(&[nowhere.clone(), a.scratch.run()], &opts).is_ok());
    let err = client::connect(&[nowhere.clone(), empty.clone()], &opts).err().unwrap();
    assert!(err.contains(&format!("{}: does not exist", nowhere.display())), "{err}");
    assert!(err.contains(&format!("{}: no lock", empty.display())), "{err}");
    assert!(err.contains("Tools > AI Assistants (MCP)"), "{err}");
    a.stop();
}

#[cfg(unix)]
#[test]
fn the_socket_and_its_folder_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let a = app("mode", Kind::Unix);
    let mode = |p: PathBuf| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(a.scratch.run()), 0o700);
    assert_eq!(mode(a.scratch.run().join("mcp.sock")), 0o600);
    assert_eq!(mode(a.scratch.run().join("mcp.json")), 0o600);
    a.stop();
}

#[test]
fn the_owner_replacing_the_project_tells_the_assistant_once() {
    let a = app_with("replaced", Kind::native(), true, |_| {});
    a.exec.hold();
    let who = a.assistant();
    who.init();
    // A call that waits is answered with the news, and the notice that follows is not told again. The
    // call must be queued in Sheaf before the project is replaced (T240: a fixed sleep was too short
    // under load, so the replace closed a connection still saying hello, or came before the call).
    who.request(2, "list_contents", json!({}));
    eventually("call 2 to wait in Sheaf", || a.exec.held_queued() == 1);
    a.server.drop_connections(NoticeKind::Replaced);
    let r = who.reply(2)["result"].clone();
    assert_eq!((r["isError"].clone(), text(&r)), (json!(true), proto::REPLACED_FOR_ASSISTANT.to_string()));
    a.exec.pause(false);
    assert_eq!(who.call(3, "list_contents", json!({}))["isError"], false);
    // The executor logs a job after its reply has gone out.
    assert_eq!(a.exec.ran_at_least(1), ["list_contents"], "only the call after the news ran");
    // With nothing waiting, the next call hears the news and is not sent; the one after works.
    a.server.drop_connections(NoticeKind::Replaced);
    eventually("the connection to close", || a.server.connections().is_empty());
    std::thread::sleep(Duration::from_millis(100));
    let r = who.call(4, "list_contents", json!({}));
    assert_eq!(text(&r), proto::REPLACED_FOR_ASSISTANT);
    assert_eq!(a.exec.ran().len(), 1, "that call was not sent");
    assert_eq!(who.call(5, "list_contents", json!({}))["isError"], false);
    who.finish();
    a.stop();
}

// ----- a fake app -------------------------------------------------------------------

/// An endpoint that says hello and then does what the test says with each call.
fn fake_app(dir: &Path, on_call: impl Fn(&mut Stream) + Send + 'static) -> (std::fs::File, Arc<AtomicUsize>) {
    discovery::check_dir(dir, true).unwrap();
    let lock = discovery::acquire_lock(dir).unwrap();
    let (listener, endpoint) = Listener::bind(Kind::native(), dir).unwrap();
    discovery::write_info(dir, &discovery::Info { pid: 1, app: "fake".into(), endpoint }).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    std::thread::spawn(move || {
        while let Ok(mut stream) = listener.accept() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let Ok(Line::Text(_hello)) = frame::read_line(&mut reader, 1 << 20, &mut || true) else { continue };
            frame::write_line(&mut stream, &proto::hello_reply("fake")).unwrap();
            while let Ok(Line::Text(l)) = frame::read_line(&mut reader, 1 << 20, &mut || true) {
                if matches!(Request::parse(&l), Ok(Request::Call { .. })) {
                    counted.fetch_add(1, Ordering::SeqCst);
                    on_call(&mut stream);
                }
            }
        }
    });
    (lock, calls)
}

#[test]
fn a_reply_that_is_lost_is_not_retried() {
    let scratch = Scratch::new("lost");
    let (_lock, calls) = fake_app(&scratch.run(), |s| s.shutdown());
    let mut cfg = ProxyConfig::new(&Roots::default());
    cfg.candidates = vec![scratch.run()];
    let who = Assistant::start(cfg);
    who.init();
    let r = who.call(2, "list_contents", json!({}));
    assert_eq!((r["isError"].clone(), text(&r)), (json!(true), proto::LOST.to_string()));
    assert_eq!(calls.load(Ordering::SeqCst), 1, "the request was written once");
    who.finish();
}

#[test]
fn a_reply_that_comes_late_is_ignored() {
    let scratch = Scratch::new("laterep");
    // The fake app answers the call `1` after the proxy gave up on it, then answers nothing else.
    let (_lock, _calls) = fake_app(&scratch.run(), |s| {
        std::thread::sleep(Duration::from_millis(600));
        let _ = frame::write_line(s, &proto::result_line(1, &json!({"content": [], "isError": false})));
    });
    let mut cfg = ProxyConfig::new(&Roots::default());
    cfg.candidates = vec![scratch.run()];
    cfg.wait = Duration::from_millis(200);
    cfg.cancel_grace = Duration::from_millis(100);
    let who = Assistant::start(cfg);
    who.init();
    let r = who.call(2, "list_contents", json!({}));
    assert_eq!(text(&r), proto::STILL_RUNNING);
    std::thread::sleep(Duration::from_millis(700));
    who.send(json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}));
    who.reply(3);
    assert_eq!(who.lines().iter().filter(|l| l["id"] == 2).count(), 1, "one answer for call 2");
    who.finish();
}

// ----- the real binary --------------------------------------------------------------

#[test]
fn the_binary_attaches_to_the_app_and_says_where_it_looked_when_there_is_none() {
    use std::process::{Command, Stdio};
    let a = app("bin", Kind::native());
    let run = |scratch: &Scratch, run_dir: &Path| -> Vec<Value> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sheaf-mcp"))
            .args(["--attach", "--root"])
            .arg(scratch.root())
            .env("SHEAF_MCP_RUN_DIR", run_dir)
            .env("HOME", &scratch.0)
            .env("LOCALAPPDATA", &scratch.0)
            .env("XDG_DATA_HOME", &scratch.0)
            .env_remove("XDG_RUNTIME_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let stdin = child.stdin.as_mut().unwrap();
            let lines = [
                json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}),
                json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
                json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "list_contents", "arguments": {}}}),
            ];
            for l in &lines {
                writeln!(stdin, "{l}").unwrap();
            }
        }
        drop(child.stdin.take());
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect()
    };
    let replies = run(&a.scratch, &a.scratch.run());
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[1]["result"]["tools"].as_array().unwrap().len(), 17);
    assert_eq!(replies[2]["result"]["isError"], false, "{}", replies[2]);
    assert!(replies[0]["result"]["instructions"].as_str().unwrap().contains("attached to Sheaf"));
    a.stop();

    let none = Scratch::new("none");
    let replies = run(&none, &none.run());
    assert_eq!(replies[1]["result"]["tools"].as_array().unwrap().len(), 17, "tools/list needs no app");
    let message = text(&replies[2]["result"]);
    assert_eq!(replies[2]["result"]["isError"], true);
    assert!(message.contains("no Sheaf is listening") && message.contains(none.run().to_str().unwrap()), "{message}");
}
