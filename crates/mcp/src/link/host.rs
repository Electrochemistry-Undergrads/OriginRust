//! The app's side of the link: the listener, a reader and a writer thread for each connection, and the
//! queue of jobs the UI thread works off (docs/spec/mcp.md sections 11.9 and 11.12).
//!
//! The threads here read and write sockets and queue jobs. They never touch the project: the app calls
//! [`LinkServer::poll_jobs`] on its UI thread, runs each job with [`Job::execute`] and the reply goes out
//! by itself.

use super::discovery::{self, Info, LockError};
use super::frame::{self, Line, REPLY_CAP, REQUEST_CAP};
use super::proto::{self, Hello, NoticeKind, Request};
use super::sanitize;
use super::transport::{Endpoint, Kind, Listener, Stream, constant_time_eq};
use crate::exec::{self, Policy, Session};
use crate::files::Protected;
use crate::roots::{Roots, plain};
use crate::{resources, schemas, tools};
use serde_json::{Value, json};
use sheaf_core::Document;
use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Longest client name shown, in characters.
pub const CLIENT_CHARS: usize = 40;
/// Longest root shown, in characters.
const ROOT_CHARS: usize = 200;
/// Longest error text shown, in characters.
pub const ERROR_CHARS: usize = 200;

const QUEUED: u8 = 0;
const RUNNING: u8 = 1;
const CANCELLED: u8 = 2;
const DONE: u8 = 3;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How the link is set up. [`LinkConfig::new`] gives the values of the spec.
#[derive(Clone)]
pub struct LinkConfig {
    /// The app's version, told to a client in the hello reply and written to `mcp.json`.
    pub app_version: String,
    pub kind: Kind,
    /// How long a new connection has to say hello.
    pub hello_timeout: Duration,
    pub max_connections: usize,
    /// How long [`LinkServer::stop`] may take in all.
    pub stop_wait: Duration,
    /// Called from the link's threads when there is something for the UI thread (a job, a connect or a
    /// disconnect): ask the window for a repaint. It must not call back into the [`LinkServer`].
    pub wake: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl LinkConfig {
    pub fn new(app_version: impl Into<String>) -> LinkConfig {
        LinkConfig {
            app_version: app_version.into(),
            kind: Kind::native(),
            hello_timeout: Duration::from_secs(10),
            max_connections: proto::MAX_CONNECTIONS,
            stop_wait: Duration::from_secs(2),
            wake: None,
        }
    }
}

/// Why the endpoint could not be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartError {
    /// Another Sheaf holds the lock: one endpoint per user.
    Busy(String),
    /// This folder cannot be used: it fails the checks, or the socket cannot be made.
    Unusable(String),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::Busy(m) | StartError::Unusable(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for StartError {}

/// What the owner can see of a connection. The text is sanitised.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnInfo {
    pub id: u64,
    pub client: String,
    pub roots: Vec<String>,
}

/// A connection that came or went, for the status bar and the Messages Log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Connected(ConnInfo),
    Disconnected(ConnInfo),
}

// ----- connections and jobs ---------------------------------------------------------

enum Out {
    Line(String),
    Close,
}

struct JobState {
    phase: AtomicU8,
    replied: AtomicBool,
}

struct Inflight {
    id: u64,
    state: Arc<JobState>,
}

struct Conn {
    info: ConnInfo,
    policy: Policy,
    session: Mutex<Session>,
    tx: Sender<Out>,
    closed: AtomicBool,
    inflight: Mutex<Option<Inflight>>,
}

impl Conn {
    fn send(&self, line: String) {
        let _ = self.tx.send(Out::Line(line));
    }

    fn send_result(&self, id: u64, result: &Value) {
        let line = proto::result_line(id, result);
        if line.len() > REPLY_CAP {
            self.send(proto::error_line(Some(id), proto::CODE_OTHER, "the result is too large to send"));
        } else {
            self.send(line);
        }
    }

    fn clear_inflight(&self, id: u64) {
        let mut slot = lock(&self.inflight);
        if slot.as_ref().is_some_and(|f| f.id == id) {
            *slot = None;
        }
    }

    /// Answers the call that waits in the queue, if one does, with an error: the call is cancelled
    /// first, so it never runs. A call that already runs is left alone.
    fn cancel_queued(&self, code: i64, message: &str) {
        let target = lock(&self.inflight).as_ref().map(|f| (f.id, f.state.clone()));
        if let Some((id, state)) = target
            && state.phase.compare_exchange(QUEUED, CANCELLED, Ordering::AcqRel, Ordering::Acquire).is_ok()
            && !state.replied.swap(true, Ordering::AcqRel)
        {
            self.clear_inflight(id);
            self.send(proto::error_line(Some(id), code, message));
        }
    }

    /// Ends the connection after what is queued for it has been written.
    fn close(&self) {
        let _ = self.tx.send(Out::Close);
    }
}

/// What a job asks for.
#[derive(Clone, Debug)]
pub enum Op {
    Call { name: String, args: Value, structured: bool },
    ResourcesList,
    ResourceRead { uri: String },
}

/// One request of an assistant, waiting for the UI thread.
pub struct Job {
    conn: Arc<Conn>,
    req_id: u64,
    op: Op,
    state: Arc<JobState>,
    created: Instant,
}

/// What a job did, for the status bar and the Messages Log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Done {
    /// The tool, or `resources` for a resource operation.
    pub tool: String,
    /// The undo step name, `AI: <tool> <target>`: sanitised and cut.
    pub label: String,
    /// The message of a failed call: sanitised and cut.
    pub failed: Option<String>,
}

impl Job {
    pub fn client(&self) -> &str {
        &self.conn.info.client
    }

    pub fn conn_id(&self) -> u64 {
        self.conn.info.id
    }

    pub fn op(&self) -> &Op {
        &self.op
    }

    /// The tool of a call.
    pub fn tool(&self) -> Option<&str> {
        match &self.op {
            Op::Call { name, .. } => Some(name),
            _ => None,
        }
    }

    /// True for a tool that changes the project: it waits while a dialog is open or a cell is edited.
    pub fn mutates(&self) -> bool {
        self.tool().is_some_and(|n| schemas::defs().iter().any(|d| d.name == n && d.mutates))
    }

    pub fn created(&self) -> Instant {
        self.created
    }

    /// True while the job has been neither started nor cancelled.
    pub fn is_queued(&self) -> bool {
        self.state.phase.load(Ordering::Acquire) == QUEUED
    }

    /// Answers a job that has not started with an error and makes sure it never runs. Returns false when
    /// it had started or was cancelled already.
    pub fn cancel(&self, message: &str) -> bool {
        if self.state.phase.compare_exchange(QUEUED, CANCELLED, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return false;
        }
        if !self.state.replied.swap(true, Ordering::AcqRel) {
            self.conn.clear_inflight(self.req_id);
            self.conn.send(proto::error_line(Some(self.req_id), proto::CODE_OTHER, message));
        }
        true
    }

    fn finish(&self, line: String) {
        if self.state.replied.swap(true, Ordering::AcqRel) {
            return;
        }
        self.conn.clear_inflight(self.req_id);
        self.conn.send(line);
    }

    /// Takes the job for running: queued to running. Returns false when it was cancelled in the meantime,
    /// in which case it must not run (the reply has been sent already).
    pub fn begin(&self) -> bool {
        self.state.phase.compare_exchange(QUEUED, RUNNING, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }

    /// [`begin`](Job::begin) and [`run`](Job::run) in one step. Call it on the thread that owns `doc`.
    ///
    /// `protected` is the set of names a new file may not take, computed now from the live document
    /// (`files::protected_set`). Returns `None` when the job was cancelled and so did not run.
    pub fn execute(&self, doc: &mut Document, protected: Protected) -> Option<Done> {
        self.begin().then(|| self.run(doc, protected))
    }

    /// Runs on the UI thread with a context reader for `sheaf://app`. The reader is called only
    /// for that resource, after the job begins, and receives the current document without edit access.
    pub fn execute_with_app_context(
        &self,
        doc: &mut Document,
        protected: Protected,
        app_context: &dyn Fn(&Document) -> Value,
    ) -> Option<Done> {
        self.begin().then(|| self.run_with_context(doc, protected, Some(app_context)))
    }

    /// Runs a job that [`begin`](Job::begin) accepted and sends the reply.
    pub fn run(&self, doc: &mut Document, protected: Protected) -> Done {
        self.run_with_context(doc, protected, None)
    }

    fn run_with_context(
        &self,
        doc: &mut Document,
        protected: Protected,
        app_context: Option<&dyn Fn(&Document) -> Value>,
    ) -> Done {
        let done = match &self.op {
            Op::Call { name, args, structured } => {
                let mut policy = self.conn.policy.clone();
                policy.protected = protected;
                let label = exec::undo_label(name, args, doc.project());
                let mut session = lock(&self.conn.session);
                let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    exec::run_on_document(doc, &mut session, &policy, name, args)
                }));
                drop(session);
                let (result, failed) = match run {
                    Ok(Ok(out)) => (out.into_result(*structured), None),
                    Ok(Err(e)) => {
                        let message = e.message.clone();
                        (e.into_result(*structured), Some(message))
                    }
                    Err(_) => {
                        let message = "internal error: the tool failed unexpectedly".to_string();
                        (tools::ToolError::new(message.clone()).into_result(*structured), Some(message))
                    }
                };
                self.finish(proto::result_line(self.req_id, &result));
                Done { tool: name.clone(), label, failed: failed.map(|m| sanitize(&m, ERROR_CHARS)) }
            }
            Op::ResourcesList => {
                let mut list = resources::list(doc.project());
                if app_context.is_some() {
                    list.insert(1, resources::app_descriptor());
                }
                let listed = json!({"resources": list});
                self.finish(proto::result_line(self.req_id, &listed));
                Done { tool: "resources".into(), label: "AI: resources".into(), failed: None }
            }
            Op::ResourceRead { uri } => {
                let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if uri == resources::APP_URI
                        && let Some(context) = app_context
                    {
                        Ok(resources::read_app(&context(doc)))
                    } else {
                        resources::read(doc.project(), uri)
                    }
                }));
                let (line, failed) = match read {
                    Ok(Ok(v)) => (proto::result_line(self.req_id, &v), None),
                    Ok(Err(e)) => (proto::error_line(Some(self.req_id), e.code, &e.message), Some(e.message)),
                    Err(_) => {
                        let m = "internal error while reading the resource";
                        (proto::error_line(Some(self.req_id), proto::CODE_OTHER, m), Some(m.to_string()))
                    }
                };
                self.finish(line);
                Done {
                    tool: "resources".into(),
                    label: "AI: resources".into(),
                    failed: failed.map(|m| sanitize(&m, ERROR_CHARS)),
                }
            }
        };
        self.state.phase.store(DONE, Ordering::Release);
        done
    }
}

/// Tidies a list of jobs the UI keeps: drops those that are no longer queued (a client cancelled or left)
/// and cancels those that have waited more than `max_age` at `now`, with the reply `the owner is busy or
/// has a dialog open in Sheaf; the call did not run; try again`. This is the backstop for clients other
/// than the proxy, whose own limit is shorter.
pub fn sweep(pending: &mut Vec<Job>, max_age: Duration, now: Instant) {
    pending.retain(|job| {
        if !job.is_queued() {
            return false;
        }
        if now.saturating_duration_since(job.created) >= max_age {
            job.cancel(proto::OWNER_BUSY);
            return false;
        }
        true
    });
}

// ----- the server -------------------------------------------------------------------

struct Slot {
    id: u64,
    stream: Stream,
    tx: Sender<Out>,
}

#[derive(Default)]
struct State {
    conns: Vec<Arc<Conn>>,
    slots: Vec<Slot>,
    queue: VecDeque<Job>,
    events: Vec<Event>,
    threads: Vec<JoinHandle<()>>,
}

struct Shared {
    cfg: LinkConfig,
    endpoint: Endpoint,
    closing: AtomicBool,
    active: AtomicUsize,
    next_id: AtomicU64,
    state: Mutex<State>,
}

impl Shared {
    fn wake(&self) {
        if let Some(w) = &self.cfg.wake {
            w();
        }
    }
}

/// The endpoint of the app. Dropping it stops it, as [`LinkServer::stop`] does.
pub struct LinkServer {
    shared: Arc<Shared>,
    dir: PathBuf,
    lock: Option<File>,
    accept: Option<JoinHandle<()>>,
}

impl LinkServer {
    /// Starts listening with the run folder `dir`: checks and, if need be, makes the folder, takes
    /// `mcp.lock`, removes an old socket and `mcp.json`, binds and writes `mcp.json`.
    pub fn start(dir: &Path, cfg: LinkConfig) -> Result<LinkServer, StartError> {
        discovery::check_dir(dir, true)
            .map_err(|why| StartError::Unusable(format!("the folder '{}' cannot be used: {why}", dir.display())))?;
        let lock = match discovery::acquire_lock(dir) {
            Ok(l) => l,
            Err(LockError::Busy) => {
                let who = discovery::read_pid(dir).map(|p| format!(" (process {p})")).unwrap_or_default();
                return Err(StartError::Busy(format!("another Sheaf{who} already has the AI link on")));
            }
            Err(LockError::Other(m)) => return Err(StartError::Unusable(m)),
        };
        // Whoever holds the lock is the only one that may have made these.
        discovery::remove_endpoint_files(dir);
        let (listener, endpoint) = Listener::bind(cfg.kind, dir).map_err(StartError::Unusable)?;
        let info = Info { pid: std::process::id(), app: cfg.app_version.clone(), endpoint: endpoint.clone() };
        if let Err(e) = discovery::write_info(dir, &info) {
            discovery::remove_endpoint_files(dir);
            return Err(StartError::Unusable(format!("could not write mcp.json in '{}': {e}", dir.display())));
        }
        let shared = Arc::new(Shared {
            cfg,
            endpoint,
            closing: AtomicBool::new(false),
            active: AtomicUsize::new(0),
            next_id: AtomicU64::new(1),
            state: Mutex::new(State::default()),
        });
        let accept = {
            let shared = shared.clone();
            std::thread::Builder::new().name("sheaf-link-accept".into()).spawn(move || accept_loop(&shared, &listener))
        };
        let accept = match accept {
            Ok(h) => h,
            Err(e) => {
                discovery::remove_endpoint_files(dir);
                return Err(StartError::Unusable(format!("could not start the link thread: {e}")));
            }
        };
        Ok(LinkServer { shared, dir: dir.to_path_buf(), lock: Some(lock), accept: Some(accept) })
    }

    /// Starts in the first of `candidates` that can be used. A folder that fails its checks is skipped; a
    /// folder whose lock another Sheaf holds ends the search, so there is never a second endpoint.
    pub fn start_first(candidates: &[PathBuf], cfg: &LinkConfig) -> Result<LinkServer, StartError> {
        let mut why: Vec<String> = Vec::new();
        for dir in candidates {
            match LinkServer::start(dir, cfg.clone()) {
                Ok(server) => return Ok(server),
                Err(StartError::Busy(m)) => return Err(StartError::Busy(m)),
                Err(StartError::Unusable(m)) => why.push(m),
            }
        }
        if why.is_empty() {
            why.push("there is no folder for the link on this computer".to_string());
        }
        Err(StartError::Unusable(why.join("; ")))
    }

    /// The run folder in use.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.shared.endpoint
    }

    /// Takes every job that is waiting, oldest first. The UI thread calls this once a frame.
    pub fn poll_jobs(&self) -> Vec<Job> {
        lock(&self.shared.state).queue.drain(..).collect()
    }

    /// The connections that came or went since the last call.
    pub fn poll_events(&self) -> Vec<Event> {
        std::mem::take(&mut lock(&self.shared.state).events)
    }

    /// The connections that are open and have said hello.
    pub fn connections(&self) -> Vec<ConnInfo> {
        lock(&self.shared.state).conns.iter().map(|c| c.info.clone()).collect()
    }

    /// Tells every client that something happened and closes the connections: a queued call is answered
    /// with the same text first. The link stays on and a client may connect again.
    pub fn drop_connections(&self, kind: NoticeKind) {
        let (conns, slots) = {
            let st = lock(&self.shared.state);
            (st.conns.clone(), st.slots.iter().map(|s| s.tx.clone()).collect::<Vec<_>>())
        };
        for c in &conns {
            c.cancel_queued(proto::CODE_OTHER, kind.message());
            c.send(proto::notice_line(kind));
            c.close();
        }
        // A connection that has not said hello is closed without a word.
        for tx in slots {
            let _ = tx.send(Out::Close);
        }
    }

    /// Turns the link off: the owner's untick, or the app leaving. Queued calls are answered with `the
    /// owner turned the link off` and never run, every client gets that notice and is disconnected,
    /// the threads are waited for at most `stop_wait` (a thread that does not end is left to end when its
    /// socket fails), `mcp.sock` and `mcp.json` are removed and the lock is dropped. Never hangs.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        let Some(accept) = self.accept.take() else { return };
        let shared = self.shared.clone();
        let deadline = Instant::now() + shared.cfg.stop_wait;
        shared.closing.store(true, Ordering::Release);
        // The accept thread sleeps in `accept`: a connection to ourselves wakes it up.
        let _ = Stream::connect(&shared.endpoint);
        self.drop_connections(NoticeKind::Closed);
        let mut handles = vec![accept];
        handles.append(&mut lock(&shared.state).threads);
        // Writers send what is queued and close; give them half the time before the sockets are cut.
        wait_for(&mut handles, Instant::now() + shared.cfg.stop_wait / 2);
        for slot in lock(&shared.state).slots.iter() {
            slot.stream.shutdown();
        }
        wait_for(&mut handles, deadline);
        drop(handles);
        {
            let mut st = lock(&shared.state);
            st.queue.clear();
            st.conns.clear();
        }
        discovery::remove_endpoint_files(&self.dir);
        self.lock = None;
    }
}

impl Drop for LinkServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Joins the threads that end before `deadline` and leaves the others in the list.
fn wait_for(handles: &mut Vec<JoinHandle<()>>, deadline: Instant) {
    loop {
        let (done, rest): (Vec<_>, Vec<_>) = std::mem::take(handles).into_iter().partition(JoinHandle::is_finished);
        for h in done {
            let _ = h.join();
        }
        *handles = rest;
        if handles.is_empty() || Instant::now() >= deadline {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

// ----- threads ----------------------------------------------------------------------

fn accept_loop(shared: &Arc<Shared>, listener: &Listener) {
    loop {
        let stream = match listener.accept() {
            Ok(s) => s,
            Err(_) => {
                if shared.closing.load(Ordering::Acquire) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
        };
        if shared.closing.load(Ordering::Acquire) {
            return;
        }
        let before = shared.active.fetch_add(1, Ordering::AcqRel);
        if before >= shared.cfg.max_connections {
            shared.active.fetch_sub(1, Ordering::AcqRel);
            refuse(stream, proto::BUSY_APP);
            continue;
        }
        if spawn_connection(shared, stream).is_none() {
            shared.active.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

/// Tells a client that will not be served why, with a short write that cannot hold the accept thread.
fn refuse(mut stream: Stream, message: &str) {
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let _ = frame::write_line(&mut stream, &proto::error_line(None, proto::CODE_OTHER, message));
    stream.shutdown();
}

fn spawn_connection(shared: &Arc<Shared>, stream: Stream) -> Option<()> {
    let id = shared.next_id.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = channel();
    let writer_stream = stream.try_clone().ok()?;
    let handle_stream = stream.try_clone().ok()?;
    let writer = std::thread::Builder::new()
        .name("sheaf-link-write".into())
        .spawn(move || write_loop(writer_stream, &rx))
        .ok()?;
    lock(&shared.state).slots.push(Slot { id, stream: handle_stream, tx: tx.clone() });
    let reader = {
        let shared = shared.clone();
        std::thread::Builder::new().name("sheaf-link-read".into()).spawn(move || serve(&shared, stream, id, tx))
    };
    let mut st = lock(&shared.state);
    match reader {
        Ok(reader) => {
            st.threads.retain(|h| !h.is_finished());
            st.threads.push(writer);
            st.threads.push(reader);
            Some(())
        }
        Err(_) => {
            st.slots.retain(|s| s.id != id);
            None
        }
    }
}

fn write_loop(mut stream: Stream, rx: &Receiver<Out>) {
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    while let Ok(out) = rx.recv() {
        match out {
            Out::Line(line) => {
                if frame::write_line(&mut stream, &line).is_err() {
                    break;
                }
            }
            Out::Close => break,
        }
    }
    stream.shutdown();
}

/// Cleans up after a connection when its reader thread ends, however it ends.
struct Leaving {
    shared: Arc<Shared>,
    id: u64,
    tx: Sender<Out>,
}

impl Drop for Leaving {
    fn drop(&mut self) {
        let conn = {
            let mut st = lock(&self.shared.state);
            st.slots.retain(|s| s.id != self.id);
            let at = st.conns.iter().position(|c| c.info.id == self.id);
            at.map(|i| st.conns.remove(i))
        };
        if let Some(conn) = conn {
            conn.closed.store(true, Ordering::Release);
            // A queued call of a client that is gone never runs; a running one finishes unheard.
            conn.cancel_queued(proto::CODE_OTHER, proto::LOST);
            lock(&self.shared.state).events.push(Event::Disconnected(conn.info.clone()));
        }
        let _ = self.tx.send(Out::Close);
        self.shared.active.fetch_sub(1, Ordering::AcqRel);
        self.shared.wake();
    }
}

fn serve(shared: &Arc<Shared>, stream: Stream, id: u64, tx: Sender<Out>) {
    let _leaving = Leaving { shared: shared.clone(), id, tx: tx.clone() };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let mut reader = BufReader::new(stream);
    let deadline = Instant::now() + shared.cfg.hello_timeout;
    // A connection that is not let in yet gets a small line and a clock that runs whatever it sends: a
    // peer that sends a byte now and then must not hold one of the few connections.
    let first = frame::read_line_by(&mut reader, frame::HELLO_CAP, Some(deadline), &mut || {
        Instant::now() < deadline && !shared.closing.load(Ordering::Acquire)
    });
    let line = match first {
        Ok(Line::Text(l)) => l,
        Ok(Line::TooLong) => {
            let _ = tx.send(Out::Line(proto::error_line(None, proto::CODE_PARAMS, "the first line is too long")));
            return;
        }
        // Silence, end of input or a failed read: nothing to answer.
        _ => return,
    };
    let fail = |message: &str| {
        let _ = tx.send(Out::Line(proto::error_line(None, proto::CODE_OTHER, message)));
    };
    let hello = match Hello::parse(&line) {
        Ok(h) => h,
        Err(None) => return,
        Err(Some(why)) => return fail(&why),
    };
    if let Some(expected) = shared.endpoint.token()
        && !constant_time_eq(hello.token.as_deref().unwrap_or_default().as_bytes(), expected.as_bytes())
    {
        return fail(proto::NOT_ALLOWED);
    }
    let paths: Vec<PathBuf> = hello.roots.iter().map(PathBuf::from).collect();
    let roots = match Roots::from_absolute(&paths) {
        Ok(r) => r,
        Err(why) => return fail(&sanitize(&why, ERROR_CHARS)),
    };
    let client = match sanitize(&hello.client, CLIENT_CHARS) {
        c if c.is_empty() => "unknown client".to_string(),
        c => c,
    };
    let shown: Vec<String> =
        roots.dirs().iter().map(|d| sanitize(&plain(d.clone()).display().to_string(), ROOT_CHARS)).collect();
    let conn = Arc::new(Conn {
        info: ConnInfo { id, client, roots: shown },
        policy: Policy::attached(roots),
        session: Mutex::new(Session::default()),
        tx: tx.clone(),
        closed: AtomicBool::new(false),
        inflight: Mutex::new(None),
    });
    {
        let mut st = lock(&shared.state);
        if shared.closing.load(Ordering::Acquire) {
            return;
        }
        st.conns.push(conn.clone());
        st.events.push(Event::Connected(conn.info.clone()));
    }
    conn.send(proto::hello_reply(&shared.cfg.app_version));
    shared.wake();
    loop {
        let line = frame::read_line(&mut reader, REQUEST_CAP, &mut || !shared.closing.load(Ordering::Acquire));
        match line {
            Ok(Line::Text(l)) if l.trim().is_empty() => {}
            Ok(Line::Text(l)) => handle_request(shared, &conn, &l),
            Ok(Line::TooLong) => {
                conn.send(proto::error_line(None, proto::CODE_PARAMS, "invalid request: the line is longer than 8 MB"));
                return;
            }
            Ok(Line::Eof) | Err(_) => return,
        }
    }
}

/// True for the tools that read nothing of the project: the reader thread answers them at once, so that a
/// dialog or a cell being edited never holds them back.
fn is_pure(name: &str) -> bool {
    matches!(name, "list_analysis_tools" | "list_fit_models")
}

fn handle_request(shared: &Arc<Shared>, conn: &Arc<Conn>, line: &str) {
    let request = match Request::parse(line) {
        Ok(r) => r,
        Err((id, why)) => return conn.send(proto::error_line(id, proto::CODE_PARAMS, &why)),
    };
    let id = request.id();
    let op = match request {
        Request::Cancel { id, target } => {
            let waiting = lock(&conn.inflight).as_ref().filter(|f| f.id == target).map(|f| f.state.clone());
            let cancelled = waiting.is_some_and(|state| {
                state.phase.compare_exchange(QUEUED, CANCELLED, Ordering::AcqRel, Ordering::Acquire).is_ok()
                    && !state.replied.swap(true, Ordering::AcqRel)
            });
            conn.send(proto::result_line(id, &json!({"cancelled": cancelled})));
            if cancelled {
                conn.clear_inflight(target);
                conn.send(proto::error_line(Some(target), proto::CODE_CANCELLED, "cancelled"));
            }
            return;
        }
        Request::Call { name, args, structured, .. } => {
            if !tools::exists(&name) {
                return conn.send(proto::error_line(Some(id), proto::CODE_PARAMS, &format!("unknown tool: {name}")));
            }
            if is_pure(&name) {
                if lock(&conn.inflight).is_some() {
                    return conn.send(proto::error_line(Some(id), proto::CODE_OTHER, proto::ONE_CALL));
                }
                return conn.send_result(id, &run_pure(&name, &args, structured));
            }
            Op::Call { name, args, structured }
        }
        Request::ResourcesList { .. } => Op::ResourcesList,
        Request::ResourceRead { uri, .. } => Op::ResourceRead { uri },
    };
    let state = Arc::new(JobState { phase: AtomicU8::new(QUEUED), replied: AtomicBool::new(false) });
    {
        let mut slot = lock(&conn.inflight);
        if slot.is_some() {
            drop(slot);
            return conn.send(proto::error_line(Some(id), proto::CODE_OTHER, proto::ONE_CALL));
        }
        *slot = Some(Inflight { id, state: state.clone() });
    }
    let job = Job { conn: conn.clone(), req_id: id, op, state, created: Instant::now() };
    {
        let mut st = lock(&shared.state);
        if shared.closing.load(Ordering::Acquire) || conn.closed.load(Ordering::Acquire) {
            drop(st);
            job.cancel(proto::CLOSED);
            return;
        }
        st.queue.push_back(job);
    }
    shared.wake();
}

/// A tool that reads no project, run on an empty one.
fn run_pure(name: &str, args: &Value, structured: bool) -> Value {
    let mut doc = Document::default();
    let mut session = Session::default();
    let policy = Policy::attached(Roots::default());
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        exec::run_on_document(&mut doc, &mut session, &policy, name, args)
    }));
    match run {
        Ok(Ok(out)) => out.into_result(structured),
        Ok(Err(e)) => e.into_result(structured),
        Err(_) => tools::ToolError::new("internal error: the tool failed unexpectedly").into_result(structured),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tools_answered_by_the_reader_thread_read_no_project() {
        for name in ["list_analysis_tools", "list_fit_models"] {
            let def = schemas::defs().into_iter().find(|d| d.name == name).unwrap();
            assert!(def.read_only && !def.mutates && is_pure(name), "{name}");
        }
        assert!(!is_pure("list_contents") && !is_pure("export_data"));
    }
}
