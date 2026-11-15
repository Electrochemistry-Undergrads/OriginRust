//! `sheaf-mcp --attach`: speaks MCP on stdio and forwards the calls to the desktop app over the link
//! (docs/spec/mcp.md sections 11.5 and 11.6).
//!
//! One thread reads stdin and one reads the link; both feed one event queue that the main loop works
//! off. So `ping` and `notifications/cancelled` are answered while a call waits for the app.

use crate::exec::NOT_ATTACHED;
use crate::link::client::{self, ClientOptions};
use crate::link::frame::{self, Line, REPLY_CAP, REQUEST_CAP};
use crate::link::proto::{self, NoticeKind, Request};
use crate::link::sanitize;
use crate::link::transport::Stream;
use crate::roots::{Roots, plain};
use crate::server::{
    INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, NEWEST_PROTOCOL, PARSE_ERROR, RpcError, initialize_result,
    negotiate, parse_message, prompts_get, reply_value,
};
use crate::{prompts, resources, schemas, tools};
use serde_json::{Value, json};
use sheaf_io::recovery::run_dir_candidates;
use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

/// How the proxy runs. [`ProxyConfig::new`] gives the values of the spec.
#[derive(Clone, Debug)]
pub struct ProxyConfig {
    /// The folders the assistant may use: absolute and canonical, as they are sent to the app.
    pub roots: Vec<String>,
    /// The run folders to look in, best first.
    pub candidates: Vec<PathBuf>,
    /// How long a call may wait for its reply.
    pub wait: Duration,
    /// How long the app has to say whether the call that timed out had started.
    pub cancel_grace: Duration,
    /// How long to wait for `mcp.json` after the app took the lock.
    pub info_wait: Duration,
    /// How long the app has to answer the hello.
    pub hello_timeout: Duration,
}

impl ProxyConfig {
    /// The roots are made absolute and canonical here (the code of the headless server's `Roots`), and the
    /// folders to look in are those of this computer.
    pub fn new(roots: &Roots) -> ProxyConfig {
        ProxyConfig {
            roots: roots.dirs().iter().map(|d| plain(d.clone()).to_string_lossy().into_owned()).collect(),
            candidates: run_dir_candidates(),
            wait: Duration::from_secs(50),
            cancel_grace: Duration::from_secs(2),
            info_wait: Duration::from_secs(2),
            hello_timeout: Duration::from_secs(10),
        }
    }
}

/// How many ids of cancelled requests the proxy keeps.
const CANCELLED_KEPT: usize = 1024;

enum Event {
    Stdin(Line),
    StdinEnd,
    Link { epoch: u64, line: String },
    LinkEnd { epoch: u64 },
}

/// How a call to the app ended.
enum Outcome {
    /// The `result` of the reply.
    Result(Value),
    /// An error reply of the app.
    Error(i64, String),
    /// The call did not get a reply; this is what to tell the assistant.
    Failed(String),
    /// The client cancelled the request: it gets no reply.
    Abandoned,
    /// Stdin ended.
    Eof,
}

struct Active {
    epoch: u64,
    writer: Stream,
}

struct Proxy<'a> {
    cfg: ProxyConfig,
    rx: Receiver<Event>,
    tx: Sender<Event>,
    out: &'a mut dyn Write,
    link: Option<Active>,
    epoch: u64,
    next_id: u64,
    protocol: String,
    client: String,
    /// A notice the app sent that no call has been told yet.
    notice: Option<NoticeKind>,
    /// A connection whose end the assistant has been told of by a reply already.
    told: Option<u64>,
    /// Lines to handle once the call that waits is done.
    backlog: VecDeque<String>,
    /// Stdin has ended. Every line read before it is still answered, a call that waits included.
    eof: bool,
    /// The ids of requests that the client cancelled before they were sent to the app, newest last.
    /// A request with such an id is dropped instead of sent.
    cancelled: VecDeque<String>,
    /// Nothing more can be written to stdout.
    ended: bool,
}

/// Runs the proxy until `input` ends, writing the MCP replies to `output`. Returns the exit code.
pub fn run(cfg: ProxyConfig, mut input: impl BufRead + Send + 'static, output: &mut dyn Write) -> i32 {
    let (tx, rx) = channel();
    let stdin_tx = tx.clone();
    let spawned = std::thread::Builder::new().name("sheaf-mcp-stdin".into()).spawn(move || {
        loop {
            match frame::read_line(&mut input, REQUEST_CAP, &mut || true) {
                Ok(Line::Eof) | Err(_) => {
                    let _ = stdin_tx.send(Event::StdinEnd);
                    return;
                }
                Ok(line) => {
                    if stdin_tx.send(Event::Stdin(line)).is_err() {
                        return;
                    }
                }
            }
        }
    });
    if spawned.is_err() {
        eprintln!("sheaf-mcp: could not start the reader thread");
        return 1;
    }
    let mut proxy = Proxy {
        cfg,
        rx,
        tx,
        out: output,
        link: None,
        epoch: 0,
        next_id: 1,
        protocol: NEWEST_PROTOCOL.to_string(),
        client: "sheaf-mcp".to_string(),
        notice: None,
        told: None,
        backlog: VecDeque::new(),
        eof: false,
        cancelled: VecDeque::new(),
        ended: false,
    };
    proxy.main_loop();
    proxy.drop_link();
    0
}

fn tool_error(text: &str) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": true})
}

/// Whether handling the line may need the app, so that it has to wait its turn behind a call.
fn needs_app(line: &str) -> bool {
    match serde_json::from_str::<Value>(line) {
        Ok(Value::Array(_)) => true,
        Ok(v) => {
            matches!(v.get("method").and_then(Value::as_str), Some("tools/call" | "resources/list" | "resources/read"))
        }
        Err(_) => false,
    }
}

/// Whether `line` is the notification that cancels the request `id`.
fn cancels(line: &str, id: &Value) -> bool {
    serde_json::from_str::<Value>(line).is_ok_and(|v| {
        v.get("method").and_then(Value::as_str) == Some("notifications/cancelled")
            && v.get("params").and_then(|p| p.get("requestId")) == Some(id)
    })
}

/// Writes the request, counting what went out. `Err` holds how many bytes were written before it failed.
fn write_counted(w: &mut Stream, line: &str) -> Result<(), usize> {
    let mut data = line.as_bytes().to_vec();
    data.push(b'\n');
    let mut sent = 0;
    while sent < data.len() {
        match w.write(&data[sent..]) {
            Ok(0) => return Err(sent),
            Ok(n) => sent += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err(sent),
        }
    }
    w.flush().map_err(|_| sent)
}

impl Proxy<'_> {
    fn main_loop(&mut self) {
        while !self.ended {
            let line = match self.backlog.pop_front() {
                Some(l) => l,
                None if self.eof => return,
                None => match self.rx.recv() {
                    Ok(Event::Stdin(Line::Text(l))) => l,
                    Ok(Event::Stdin(Line::TooLong)) => {
                        self.too_long();
                        continue;
                    }
                    Ok(Event::Stdin(Line::Eof)) | Ok(Event::StdinEnd) | Err(_) => return,
                    Ok(other) => {
                        self.idle_event(other);
                        continue;
                    }
                },
            };
            self.handle_line(&line);
        }
    }

    fn write_out(&mut self, line: &str) {
        if frame::write_line(&mut *self.out, line).is_err() {
            self.ended = true;
        }
    }

    fn too_long(&mut self) {
        let reply = json!({"jsonrpc": "2.0", "id": null,
            "error": {"code": INVALID_REQUEST, "message": "invalid request: the line is longer than 8 MB"}});
        self.write_out(&reply.to_string());
    }

    fn handle_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        let reply = match serde_json::from_str::<Value>(line) {
            Err(e) => Some(
                json!({"jsonrpc": "2.0", "id": null, "error": {"code": PARSE_ERROR, "message": format!("parse error: {e}")}}),
            ),
            Ok(Value::Array(items)) => {
                if items.is_empty() {
                    Some(json!({"jsonrpc": "2.0", "id": null,
                        "error": {"code": INVALID_REQUEST, "message": "invalid request: an empty batch"}}))
                } else {
                    let replies: Vec<Value> = items.into_iter().filter_map(|m| self.handle_message(m)).collect();
                    if replies.is_empty() { None } else { Some(Value::Array(replies)) }
                }
            }
            Ok(message) => self.handle_message(message),
        };
        if let Some(r) = reply {
            self.write_out(&r.to_string());
        }
    }

    fn handle_message(&mut self, message: Value) -> Option<Value> {
        let request = match parse_message(message) {
            Ok(r) => r,
            Err(reply) => return reply,
        };
        let id = request.id.clone().unwrap_or(Value::Null);
        let params = &request.params;
        let result: Option<Result<Value, RpcError>> = match request.method.as_str() {
            "initialize" => Some(self.initialize(params)),
            "ping" => Some(Ok(json!({}))),
            "tools/list" => Some(Ok(json!({"tools": tools::list_attached()}))),
            "tools/call" => self.tools_call(&id, params),
            "resources/list" => self.resource_call(&id, &|rid| Request::ResourcesList { id: rid }),
            "resources/read" => match params.get("uri").and_then(Value::as_str) {
                None => Some(Err(RpcError::new(INVALID_PARAMS, "invalid params: missing \"uri\""))),
                Some(uri) => {
                    let uri = uri.to_string();
                    self.resource_call(&id, &|rid| Request::ResourceRead { id: rid, uri: uri.clone() })
                }
            },
            "resources/templates/list" => Some(Ok(json!({"resourceTemplates": resources::templates()}))),
            "prompts/list" => Some(Ok(prompts::list())),
            "prompts/get" => Some(prompts_get(params)),
            "notifications/cancelled" => {
                self.note_cancelled(params.get("requestId"));
                Some(Ok(json!({})))
            }
            m if m.starts_with("notifications/") => Some(Ok(json!({}))),
            m => Some(Err(RpcError::new(METHOD_NOT_FOUND, format!("method not found: {m}")))),
        };
        let id = request.id?;
        Some(reply_value(id, result?))
    }

    /// Remembers that the client cancelled the request `id`, so that it is not sent if it has not been yet
    /// (it waits in the backlog, or it has not arrived).
    fn note_cancelled(&mut self, id: Option<&Value>) {
        let Some(id) = id.filter(|i| i.is_string() || i.is_number()) else { return };
        let key = id.to_string();
        if !self.cancelled.contains(&key) {
            if self.cancelled.len() >= CANCELLED_KEPT {
                self.cancelled.pop_front();
            }
            self.cancelled.push_back(key);
        }
    }

    /// Whether the request `id` was cancelled before it went out. The note is used up.
    fn take_cancelled(&mut self, id: &Value) -> bool {
        let key = id.to_string();
        match self.cancelled.iter().position(|k| *k == key) {
            Some(at) => {
                self.cancelled.remove(at);
                true
            }
            None => false,
        }
    }

    fn initialize(&mut self, params: &Value) -> Result<Value, RpcError> {
        self.protocol = negotiate(params)?;
        if let Some(info) = params.get("clientInfo") {
            let name = info.get("name").and_then(Value::as_str).unwrap_or_default();
            let version = info.get("version").and_then(Value::as_str).unwrap_or_default();
            let label = sanitize(&format!("{name} {version}"), 80);
            if !label.is_empty() {
                self.client = label;
            }
        }
        Ok(initialize_result(&self.protocol, self.instructions()))
    }

    fn instructions(&self) -> String {
        let roots = if self.cfg.roots.is_empty() {
            "No folder is allowed (sheaf-mcp was started without --root), so tools that read or write files refuse."
                .to_string()
        } else {
            format!(
                "Files can be read and written only under: {}. Relative paths start at the first folder.",
                self.cfg.roots.join(", ")
            )
        };
        format!(
            "You are attached to Sheaf, which a person has open on their computer, with their project in it. \
             Workbooks, sheets, columns and graphs are named by their names (Book1, Sheet1, B, Graph1), never by \
             numbers; rows count from 1. Read sheaf://app, when listed, for the active window and selection. \
             Every change you make appears in the person's undo list as \"AI: ...\". \
             You cannot open, create or save projects: the person does that. Name the sheet in your calls, \
             because an omitted sheet means the sheet the person has selected, and that can change between \
             calls. A call may wait while the person has a dialog open or is typing in a cell. Files you write \
             are never replaced, so pick a new file name. {roots}"
        )
    }

    fn tools_call(&mut self, id: &Value, params: &Value) -> Option<Result<Value, RpcError>> {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return Some(Err(RpcError::new(INVALID_PARAMS, "invalid params: missing the tool \"name\"")));
        };
        let args = match params.get("arguments") {
            None | Some(Value::Null) => json!({}),
            Some(a) if a.is_object() => a.clone(),
            Some(_) => {
                return Some(Err(RpcError::new(INVALID_PARAMS, "invalid params: \"arguments\" must be an object")));
            }
        };
        let Some(def) = schemas::defs().into_iter().find(|d| d.name == name) else {
            return Some(Err(RpcError::new(INVALID_PARAMS, format!("unknown tool: {name}"))));
        };
        if def.headless_only {
            return Some(Ok(tool_error(NOT_ATTACHED)));
        }
        let name = name.to_string();
        let structured = self.protocol == NEWEST_PROTOCOL;
        let make = |rid| Request::Call { id: rid, name: name.clone(), args: args.clone(), structured };
        match self.exchange(id, &make) {
            Outcome::Result(v) => Some(Ok(v)),
            Outcome::Error(code, message) if code == proto::CODE_PARAMS => Some(Err(RpcError::new(code, message))),
            Outcome::Error(code, _) if code == proto::CODE_CANCELLED => Some(Ok(tool_error("cancelled"))),
            Outcome::Error(_, message) | Outcome::Failed(message) => Some(Ok(tool_error(&message))),
            Outcome::Abandoned | Outcome::Eof => None,
        }
    }

    fn resource_call(&mut self, id: &Value, make: &dyn Fn(u64) -> Request) -> Option<Result<Value, RpcError>> {
        match self.exchange(id, make) {
            Outcome::Result(v) => Some(Ok(v)),
            Outcome::Error(code, message) => Some(Err(RpcError::new(code, message))),
            Outcome::Failed(message) => Some(Err(RpcError::new(proto::CODE_OTHER, message))),
            Outcome::Abandoned | Outcome::Eof => None,
        }
    }

    // ----- the link --------------------------------------------------------------

    fn connect_link(&mut self) -> Result<(), String> {
        let mut options = ClientOptions::new(self.client.clone(), self.cfg.roots.clone());
        options.info_wait = self.cfg.info_wait;
        options.hello_timeout = self.cfg.hello_timeout;
        let mut link = client::connect(&self.cfg.candidates, &options)?;
        self.epoch += 1;
        let epoch = self.epoch;
        let tx = self.tx.clone();
        std::thread::Builder::new()
            .name("sheaf-mcp-link".into())
            .spawn(move || {
                loop {
                    match frame::read_line(&mut link.reader, REPLY_CAP, &mut || true) {
                        Ok(Line::Text(line)) => {
                            if tx.send(Event::Link { epoch, line }).is_err() {
                                return;
                            }
                        }
                        _ => {
                            let _ = tx.send(Event::LinkEnd { epoch });
                            return;
                        }
                    }
                }
            })
            .map_err(|e| format!("could not start the link thread: {e}"))?;
        self.link = Some(Active { epoch, writer: link.writer });
        Ok(())
    }

    /// Closes the connection. Whatever still arrives on it is ignored.
    fn drop_link(&mut self) {
        if let Some(link) = self.link.take() {
            link.writer.shutdown();
        }
    }

    fn current(&self) -> Option<u64> {
        self.link.as_ref().map(|l| l.epoch)
    }

    /// An event that no call waits for.
    fn idle_event(&mut self, event: Event) {
        match event {
            Event::Stdin(line) => self.side_line(line),
            Event::StdinEnd => self.eof = true,
            Event::Link { epoch, line } if Some(epoch) == self.current() => self.read_link_line(epoch, &line),
            Event::LinkEnd { epoch } if Some(epoch) == self.current() => self.drop_link(),
            Event::Link { .. } | Event::LinkEnd { .. } => {}
        }
    }

    /// A line of the app that answers no call: a notice is kept for the next call.
    fn read_link_line(&mut self, epoch: u64, line: &str) {
        if let Some(kind) = notice_of(line)
            && self.told != Some(epoch)
        {
            self.notice = Some(kind);
        }
    }

    /// A line of stdin while a call waits or between two calls: answered at once unless it needs the app.
    fn side_line(&mut self, line: Line) {
        match line {
            Line::Text(l) if needs_app(&l) => self.backlog.push_back(l),
            Line::Text(l) => self.handle_line(&l),
            Line::TooLong => self.too_long(),
            Line::Eof => self.eof = true,
        }
    }

    fn drain(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            self.idle_event(event);
        }
    }

    fn send_cancel(&mut self, target: u64) -> Option<u64> {
        let id = self.next_id;
        self.next_id += 1;
        let link = self.link.as_mut()?;
        write_counted(&mut link.writer, &Request::Cancel { id, target }.to_line()).ok().map(|()| id)
    }

    fn give_up(&mut self, text: &str) -> Outcome {
        self.drop_link();
        Outcome::Failed(text.to_string())
    }

    /// Sends a request and waits for its reply (section 11.6). A request is written once; it is written
    /// again only when the connection was found dead before a single byte went out.
    fn exchange(&mut self, mcp_id: &Value, make: &dyn Fn(u64) -> Request) -> Outcome {
        self.drain();
        if self.ended {
            return Outcome::Eof;
        }
        if self.take_cancelled(mcp_id) {
            return Outcome::Abandoned;
        }
        if let Some(kind) = self.notice.take() {
            return Outcome::Failed(kind.for_assistant().to_string());
        }
        let mut again = false;
        let id = loop {
            if self.link.is_none()
                && let Err(why) = self.connect_link()
            {
                return Outcome::Failed(why);
            }
            let id = self.next_id;
            self.next_id += 1;
            let line = make(id).to_line();
            let Some(link) = self.link.as_mut() else { return Outcome::Failed(proto::LOST.to_string()) };
            match write_counted(&mut link.writer, &line) {
                Ok(()) => break id,
                Err(0) if !again => {
                    again = true;
                    self.drop_link();
                }
                Err(_) => return self.give_up(proto::LOST),
            }
        };
        self.wait(mcp_id, id)
    }

    fn wait(&mut self, mcp_id: &Value, id: u64) -> Outcome {
        let deadline = Instant::now() + self.cfg.wait;
        // The cancel we sent because time ran out, and when its answer is due.
        let mut timeout_cancel: Option<(u64, Instant)> = None;
        // The cancel we sent because the client asked.
        let mut user_cancel: Option<u64> = None;
        let Some(epoch) = self.current() else { return Outcome::Failed(proto::LOST.to_string()) };
        loop {
            let until = timeout_cancel.map_or(deadline, |(_, due)| due);
            match self.rx.recv_timeout(until.saturating_duration_since(Instant::now())) {
                Err(RecvTimeoutError::Timeout) => {
                    if timeout_cancel.is_some() {
                        return self.give_up(proto::STILL_RUNNING);
                    }
                    match self.send_cancel(id) {
                        Some(cid) => timeout_cancel = Some((cid, Instant::now() + self.cfg.cancel_grace)),
                        None => return self.give_up(proto::STILL_RUNNING),
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.ended = true;
                    self.send_cancel(id);
                    self.drop_link();
                    return Outcome::Eof;
                }
                // The end of input does not cancel the call: it was complete when it was read, so it is
                // answered, like on every other path (a script may close stdin right after its last line).
                Ok(Event::StdinEnd) => self.eof = true,
                Ok(Event::Stdin(Line::Text(l))) if cancels(&l, mcp_id) => {
                    if user_cancel.is_none() {
                        user_cancel = self.send_cancel(id);
                    }
                }
                Ok(Event::LinkEnd { epoch: g }) if g == epoch => {
                    self.drop_link();
                    return match self.notice.take() {
                        Some(kind) => Outcome::Failed(kind.for_assistant().to_string()),
                        None => Outcome::Failed(proto::LOST.to_string()),
                    };
                }
                Ok(Event::Link { epoch: g, line }) if g == epoch => {
                    let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
                    if let Some(kind) = notice_of(&line) {
                        if self.told != Some(epoch) {
                            self.notice = Some(kind);
                        }
                        continue;
                    }
                    let Some(rid) = v.get("id").and_then(Value::as_u64) else { continue };
                    if rid == id {
                        if let Some(result) = v.get("result") {
                            return Outcome::Result(result.clone());
                        }
                        let code = v["error"]["code"].as_i64().unwrap_or(proto::CODE_OTHER);
                        let message = v["error"]["message"].as_str().unwrap_or("the call failed").to_string();
                        if code == proto::CODE_CANCELLED && user_cancel.is_some() {
                            return Outcome::Abandoned;
                        }
                        if code == proto::CODE_CANCELLED && timeout_cancel.is_some() {
                            return self.give_up(proto::OWNER_BUSY);
                        }
                        if message == proto::REPLACED || message == proto::CLOSED {
                            // The notice that follows says the same: the assistant hears it once.
                            self.told = Some(epoch);
                            self.notice = None;
                            let kind =
                                if message == proto::REPLACED { NoticeKind::Replaced } else { NoticeKind::Closed };
                            return Outcome::Failed(kind.for_assistant().to_string());
                        }
                        return Outcome::Error(code, message);
                    }
                    let cancelled = v["result"]["cancelled"].as_bool();
                    if Some(rid) == user_cancel && cancelled == Some(true) {
                        return Outcome::Abandoned;
                    }
                    if let Some((cid, _)) = timeout_cancel
                        && rid == cid
                    {
                        return self.give_up(if cancelled == Some(true) {
                            proto::OWNER_BUSY
                        } else {
                            proto::STILL_RUNNING
                        });
                    }
                }
                Ok(other) => self.idle_event(other),
            }
        }
    }
}

fn notice_of(line: &str) -> Option<NoticeKind> {
    let v: Value = serde_json::from_str(line).ok()?;
    NoticeKind::parse(v.get("notice")?.get("kind")?.as_str()?)
}
