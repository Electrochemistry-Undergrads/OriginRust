//! The AI link of the app (task T25, phase 2; docs/spec/mcp.md section 11): the endpoint that
//! `sheaf-mcp --attach` talks to, and the per-frame work of the UI thread on the jobs it queues.
//!
//! The link's threads read sockets and queue jobs; only [`Live::step`] touches the project, on the
//! UI thread, once a frame, before the new windows are opened.

mod context;

pub use context::Context;

use eframe::egui;
use sheaf_core::Document;
use sheaf_mcp::files::{OpenFiles, protected_set};
use sheaf_mcp::link::host::{ConnInfo, Event, Job, LinkConfig, LinkServer, StartError, sweep};
use sheaf_mcp::link::proto::NoticeKind;
use sheaf_mcp::link::sanitize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A queued call that has waited this long is answered with an error (the proxy's own limit, 50
/// seconds, comes first).
pub const MAX_WAIT: Duration = Duration::from_secs(55);

/// How often the window repaints while an assistant is connected, so a lost wake-up cannot stall.
const POLL: Duration = Duration::from_millis(250);

/// Longest tool name shown.
const TOOL_CHARS: usize = 40;

/// A line for the status bar and the Messages Log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Info(String),
    Warn(String),
}

/// The endpoint, when it is on, and the calls that wait for the owner's attention.
pub struct Live {
    ctx: egui::Context,
    /// The run folders to try, best first (empty: there is nowhere to listen).
    dirs: Vec<PathBuf>,
    server: Option<LinkServer>,
    /// Jobs taken from the link that have not run yet (a mutating call waits for a free moment).
    pending: Vec<Job>,
    /// The `Document::generation` the connections were made under.
    generation: u64,
    /// Who is connected, for the status-bar chip.
    clients: Vec<ConnInfo>,
}

impl Live {
    pub fn new(ctx: egui::Context, dirs: Vec<PathBuf>) -> Live {
        Live { ctx, dirs, server: None, pending: Vec::new(), generation: 0, clients: Vec::new() }
    }

    /// True while the endpoint is listening.
    pub fn active(&self) -> bool {
        self.server.is_some()
    }

    /// The assistants that are connected.
    pub fn clients(&self) -> &[ConnInfo] {
        &self.clients
    }

    /// Calls that wait for a dialog to close or an edit to end.
    #[cfg(test)]
    pub fn waiting(&self) -> usize {
        self.pending.len()
    }

    /// Starts listening. `generation` is the project's `Document::generation` now.
    pub fn start(&mut self, generation: u64) -> Result<(), StartError> {
        if self.server.is_some() {
            return Ok(());
        }
        let mut cfg = LinkConfig::new(env!("CARGO_PKG_VERSION"));
        let ctx = self.ctx.clone();
        cfg.wake = Some(std::sync::Arc::new(move || ctx.request_repaint()));
        self.server = Some(LinkServer::start_first(&self.dirs, &cfg)?);
        self.generation = generation;
        Ok(())
    }

    /// Turns the link off: waiting calls are answered with `the owner turned the link off`, the
    /// clients are told and disconnected, and the files are removed. Takes at most about two seconds.
    pub fn stop(&mut self) {
        // The server answers the calls it knows; those taken into `pending` share their state.
        if let Some(server) = self.server.take() {
            server.stop();
        }
        self.pending.clear();
        self.clients.clear();
    }

    /// The work of one frame: notes who came and went, drops the connections when the owner replaced
    /// the project, tidies the waiting calls and runs what may run now.
    ///
    /// A call that changes the project waits while `blocked` (a dialog is open or a cell is being
    /// edited) and at most one runs per frame; the others run at once, in the order they came. Each
    /// call runs through `Job::execute`, which sends the reply. `recovery_dir` is closed to new files,
    /// as are the project's own files (`files`) and the run folder.
    pub fn step(
        &mut self,
        doc: &mut Document,
        blocked: bool,
        files: &OpenFiles,
        recovery_dir: Option<&Path>,
        now: Instant,
        app_context: &dyn Fn(&Document) -> serde_json::Value,
    ) -> Vec<Note> {
        let mut notes = Vec::new();
        let Some(server) = &self.server else { return notes };
        for event in server.poll_events() {
            notes.push(Note::Info(match event {
                Event::Connected(c) => format!("AI assistant connected: {}", describe(&c)),
                Event::Disconnected(c) => format!("AI assistant disconnected: {}", sanitize(&c.client, 40)),
            }));
        }
        self.pending.extend(server.poll_jobs());
        if doc.generation() != self.generation {
            // The owner opened or created another project: names the assistant knows are stale.
            self.generation = doc.generation();
            let open = server.connections().len();
            server.drop_connections(NoticeKind::Replaced);
            if open > 0 {
                notes.push(Note::Info(format!("The project was replaced: {open} AI connection(s) closed")));
            }
        }
        sweep(&mut self.pending, MAX_WAIT, now);

        let mut folders: Vec<PathBuf> = recovery_dir.map(Path::to_path_buf).into_iter().collect();
        folders.push(server.dir().to_path_buf());
        let mut changed = false;
        let mut ran = false;
        for job in std::mem::take(&mut self.pending) {
            let mutates = job.mutates();
            if mutates && (blocked || changed) {
                self.pending.push(job);
                continue;
            }
            let before = doc.revision();
            let Some(done) = job.execute_with_app_context(doc, protected_set(files, &folders), app_context) else {
                continue;
            };
            ran = true;
            let tool = sanitize(&done.tool, TOOL_CHARS);
            match done.failed {
                Some(why) => notes.push(Note::Warn(format!("AI: {tool} failed: {why}"))),
                None if mutates && doc.revision() != before => notes.push(Note::Info(done.label)),
                None => {}
            }
            changed |= mutates;
        }
        self.clients = server.connections();
        if ran || (changed && !self.pending.is_empty()) {
            // The views were drawn before the job ran, and a job that waited for the next frame is due in it.
            self.ctx.request_repaint();
        } else if !self.clients.is_empty() || !self.pending.is_empty() {
            self.ctx.request_repaint_after(POLL);
        }
        notes
    }
}

/// `name (folders: a, b)` for the Messages Log; the text is sanitised already.
fn describe(c: &ConnInfo) -> String {
    let roots = if c.roots.is_empty() { "no folders".to_string() } else { c.roots.join(", ") };
    format!("{} (may use: {roots})", c.client)
}

/// The tooltip of the status-bar chip: each assistant with the folders it named, and what the owner
/// can and cannot undo.
pub fn tooltip(clients: &[ConnInfo]) -> String {
    let mut lines: Vec<String> = clients.iter().map(|c| format!("{}\n    {}", c.client, folders(c))).collect();
    lines.push(String::new());
    lines.push("Its changes are undo steps named \"AI: ...\" (within the undo limit).".to_string());
    lines.push("Files it writes are not undone. Untick Tools > AI Assistants (MCP) to disconnect.".to_string());
    lines.join("\n")
}

fn folders(c: &ConnInfo) -> String {
    if c.roots.is_empty() { "no folders".to_string() } else { format!("folders: {}", c.roots.join(", ")) }
}

impl Drop for Live {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(client: &str, roots: &[&str]) -> ConnInfo {
        ConnInfo { id: 1, client: client.to_string(), roots: roots.iter().map(|r| r.to_string()).collect() }
    }

    #[test]
    fn the_tooltip_names_each_client_with_its_folders() {
        let tip = tooltip(&[info("claude-code", &["/data/a", "/data/b"]), info("other", &[])]);
        assert!(tip.contains("claude-code\n    folders: /data/a, /data/b"), "{tip}");
        assert!(tip.contains("other\n    no folders"), "{tip}");
        assert!(tip.contains("not undone"), "{tip}");
    }

    #[test]
    fn a_connection_is_described_with_what_it_may_use() {
        assert_eq!(describe(&info("x", &["/r"])), "x (may use: /r)");
        assert_eq!(describe(&info("x", &[])), "x (may use: no folders)");
    }
}
