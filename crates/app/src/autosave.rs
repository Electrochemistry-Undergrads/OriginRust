//! Autosave: the timer and the worker that write the modified project to this instance's recovery
//! file (`sheaf_io::recovery`). The application asks [`Autosave::due`] every frame and, when it is
//! time, hands a snapshot of the project to [`Autosave::start_write`], which saves it on a worker
//! thread so a large project does not freeze the window.

use sheaf_core::Project;
use sheaf_io::recovery::{AutosaveSettings, Session};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How often a running write is looked at.
const POLL: Duration = Duration::from_millis(100);

/// What the timer says.
#[derive(Debug, PartialEq, Eq)]
pub enum Due {
    /// Nothing to write and no need to look again until something changes.
    Never,
    /// Look again after this long.
    In(Duration),
    /// Write now.
    Now,
}

/// The state of this instance's autosave.
pub struct Autosave {
    session: Option<Arc<Session>>,
    pub settings: AutosaveSettings,
    pub settings_path: Option<PathBuf>,
    /// Added to the clock; the tests move time forward with it.
    offset: Duration,
    /// When the project became modified, while it is modified.
    dirty_since: Option<Instant>,
    /// When the last write started.
    last_write: Option<Instant>,
    /// The document revision that is on disk (or on its way there).
    written: Option<u64>,
    /// Recovery files of this session may exist.
    has_files: bool,
    inflight: Option<JoinHandle<Result<(), String>>>,
    /// How many background writes were started (the tests count them).
    #[cfg_attr(not(test), allow(dead_code))]
    started: usize,
    /// The last write failed and said so; the next success clears it, so a broken folder is
    /// reported once and not on every tick.
    error_reported: bool,
}

impl Autosave {
    pub fn new(session: Option<Session>, settings: AutosaveSettings, settings_path: Option<PathBuf>) -> Self {
        Autosave {
            session: session.map(Arc::new),
            settings: settings.clamped(),
            settings_path,
            offset: Duration::ZERO,
            dirty_since: None,
            last_write: None,
            written: None,
            has_files: false,
            inflight: None,
            started: 0,
            error_reported: false,
        }
    }

    /// True when this instance has a recovery session (a recovery folder that could be used).
    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }

    /// The clock the timer reads.
    pub fn now(&self) -> Instant {
        Instant::now() + self.offset
    }

    /// Moves the clock forward (tests).
    #[cfg(test)]
    pub fn advance(&mut self, by: Duration) {
        self.offset += by;
    }

    /// True when no write is running.
    #[cfg(test)]
    pub fn idle(&self) -> bool {
        self.inflight.is_none()
    }

    /// How many background writes were started.
    #[cfg(test)]
    pub fn writes_started(&self) -> usize {
        self.started
    }

    /// The recovery file of this session, when there is one.
    #[cfg(test)]
    pub fn project_path(&self) -> Option<PathBuf> {
        self.session.as_ref().map(|s| s.project_path())
    }

    /// Tells the timer whether the project is modified and which revision it is at, and answers
    /// whether to write now.
    pub fn due(&mut self, dirty: bool, revision: u64) -> Due {
        let now = self.now();
        if !dirty {
            self.dirty_since = None;
            return Due::Never;
        }
        let since = *self.dirty_since.get_or_insert(now);
        if self.session.is_none() || !self.settings.enabled {
            return Due::Never;
        }
        if self.inflight.is_some() {
            return Due::In(POLL);
        }
        if self.written == Some(revision) {
            return Due::Never;
        }
        let start = self.last_write.map_or(since, |w| w.max(since));
        let at = start + self.settings.interval();
        if now >= at { Due::Now } else { Due::In(at - now) }
    }

    /// Writes `project` (a snapshot) on a worker thread. `original` is the file the project belongs
    /// to. Only one write runs at a time; a second call while one runs does nothing.
    pub fn start_write(&mut self, project: Project, original: Option<&Path>, revision: u64) {
        let Some(session) = self.session.clone() else { return };
        if self.inflight.is_some() {
            return;
        }
        let original = original.map(Path::to_path_buf);
        self.last_write = Some(self.now());
        self.written = Some(revision);
        self.has_files = true;
        self.started += 1;
        self.inflight = Some(std::thread::spawn(move || {
            session.save(&project, original.as_deref()).map(|_| ()).map_err(|e| e.to_string())
        }));
    }

    /// Writes `project` at once, on this thread (the recovered project, which must be safe before
    /// its orphan goes).
    pub fn write_now(&mut self, project: &Project, original: Option<&Path>, revision: u64) -> Result<(), String> {
        let Some(session) = self.session.clone() else { return Err("no recovery folder".to_string()) };
        self.finish_write();
        self.last_write = Some(self.now());
        self.has_files = true;
        match session.save(project, original) {
            Ok(_) => {
                self.written = Some(revision);
                Ok(())
            }
            Err(e) => {
                self.written = None;
                Err(e.to_string())
            }
        }
    }

    /// The result of a write that has finished, once; `None` while it runs or when there is none.
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        if self.inflight.as_ref().is_some_and(JoinHandle::is_finished) { self.finish_write() } else { None }
    }

    /// Waits for a running write and returns its result.
    pub fn finish_write(&mut self) -> Option<Result<(), String>> {
        let result = self.inflight.take()?.join().unwrap_or_else(|_| Err("the autosave thread stopped".to_string()));
        if result.is_err() {
            // Try the same revision again at the next interval.
            self.written = None;
        }
        Some(result)
    }

    /// Records that an error was shown; false when it was already shown (so it is said once).
    pub fn report_error(&mut self) -> bool {
        !std::mem::replace(&mut self.error_reported, true)
    }

    /// A write went through: the next error is news again.
    pub fn report_success(&mut self) {
        self.error_reported = false;
    }

    /// The project is clean: removes this session's recovery files. A write that is still running
    /// is not waited for (that would freeze the window); this returns `Ok(false)` and is called
    /// again on the next frame, which removes the files once the write has ended, so a late write
    /// cannot bring them back. `Ok(true)` when files were removed. After a failure the files are
    /// still counted, so the next call tries again.
    pub fn clear(&mut self) -> Result<bool, String> {
        self.dirty_since = None;
        self.written = None;
        self.last_write = None;
        if self.inflight.as_ref().is_some_and(|h| !h.is_finished()) {
            return Ok(false);
        }
        self.finish_write();
        if !self.has_files {
            return Ok(false);
        }
        match &self.session {
            Some(s) => {
                s.clear().map_err(|e| e.to_string())?;
                self.has_files = false;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Ends the session. With `clean` the recovery files go (a normal exit); without, they stay
    /// for the next start (an exit while changes are unsaved and nobody was asked).
    pub fn end(&mut self, clean: bool) -> Result<(), String> {
        self.finish_write();
        let Some(session) = self.session.take() else { return Ok(()) };
        if !clean {
            return Ok(());
        }
        match Arc::into_inner(session) {
            Some(s) => s.finish().map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }
}
