//! Autosave and recovery: a running app instance writes its unsaved or modified project to a recovery
//! file now and then; after a crash the next start finds what was left behind and offers it.
//!
//! The recovery files live in a folder of their own (never beside the user's project) and are always
//! in Sheaf's native `.sheaf` format, because that format stores everything. Per running instance
//! there are three files, all named by the instance's session id `<pid>-<unix seconds>-<n>`:
//!
//! - `<id>.lock` is held with an operating system file lock for as long as the instance runs. The
//!   system drops the lock when the process ends, however it ends, so a lock that is not held
//!   means that the instance is gone. (A process id alone cannot tell: ids are reused.)
//! - `<id>.sheaf` is the project as it was at the last autosave.
//! - `<id>.json` says where the project came from and when it was written.
//!
//! The rules are in `docs/spec/autosave.md`.

use crate::error::IoError;
use crate::{project_file, save};
use serde::{Deserialize, Serialize};
use sheaf_core::Project;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Recovery files older than this are dropped at start-up (Sheaf's choice).
pub const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// The shortest autosave interval a setting can have, in minutes.
pub const MIN_MINUTES: u32 = 1;
/// The longest autosave interval a setting can have, in minutes.
pub const MAX_MINUTES: u32 = 120;
/// The autosave interval when nothing is set, in minutes (Sheaf's choice).
pub const DEFAULT_MINUTES: u32 = 10;

/// Which family of operating systems decides where the folders are.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Platform {
    Windows,
    Mac,
    Other,
}

impl Platform {
    fn current() -> Self {
        if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::Mac
        } else {
            Platform::Other
        }
    }
}

/// A non-empty environment variable, as a path.
fn env_path(get: &dyn Fn(&str) -> Option<std::ffi::OsString>, name: &str) -> Option<PathBuf> {
    get(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Like [`env_path`], for the XDG variables, which must be absolute to count.
fn xdg_path(get: &dyn Fn(&str) -> Option<std::ffi::OsString>, name: &str) -> Option<PathBuf> {
    env_path(get, name).filter(|p| p.is_absolute())
}

/// The `Sheaf` folder of `platform` for data (the recovery files) or, with `config`, for settings.
fn base_dir(platform: Platform, get: &dyn Fn(&str) -> Option<std::ffi::OsString>, config: bool) -> Option<PathBuf> {
    match platform {
        Platform::Windows => {
            let root = env_path(get, if config { "APPDATA" } else { "LOCALAPPDATA" })?;
            Some(root.join("Sheaf"))
        }
        Platform::Mac => Some(env_path(get, "HOME")?.join("Library").join("Application Support").join("Sheaf")),
        Platform::Other => {
            let (var, rest) = if config { ("XDG_CONFIG_HOME", ".config") } else { ("XDG_DATA_HOME", ".local/share") };
            let root = xdg_path(get, var).or_else(|| env_path(get, "HOME").map(|h| h.join(rest)))?;
            Some(root.join("sheaf"))
        }
    }
}

fn recovery_dir_for(platform: Platform, get: &dyn Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    let dir = base_dir(platform, get, false)?;
    Some(dir.join(if platform == Platform::Other { "recovery" } else { "Recovery" }))
}

fn settings_path_for(platform: Platform, get: &dyn Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    Some(base_dir(platform, get, true)?.join("autosave.json"))
}

/// The folder the recovery files go to on this computer, from environment variables only: on
/// Windows `%LOCALAPPDATA%\Sheaf\Recovery`, on macOS `$HOME/Library/Application Support/Sheaf/Recovery`,
/// elsewhere `$XDG_DATA_HOME/sheaf/recovery` or `$HOME/.local/share/sheaf/recovery`. `None` when the
/// variables are missing; the app then runs without recovery.
pub fn default_dir() -> Option<PathBuf> {
    recovery_dir_for(Platform::current(), &|n| std::env::var_os(n))
}

/// Where the autosave settings are kept: on Windows `%APPDATA%\Sheaf\autosave.json`, on macOS
/// `$HOME/Library/Application Support/Sheaf/autosave.json`, elsewhere `$XDG_CONFIG_HOME/sheaf/autosave.json`
/// or `$HOME/.config/sheaf/autosave.json`.
pub fn default_settings_path() -> Option<PathBuf> {
    settings_path_for(Platform::current(), &|n| std::env::var_os(n))
}

/// The autosave switch and interval the user chose.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutosaveSettings {
    /// Whether the project is written to a recovery file now and then.
    pub enabled: bool,
    /// Minutes between two autosaves, 1 to 120.
    pub minutes: u32,
}

impl Default for AutosaveSettings {
    fn default() -> Self {
        AutosaveSettings { enabled: true, minutes: DEFAULT_MINUTES }
    }
}

impl AutosaveSettings {
    /// Reads the settings. A missing or unreadable file gives the defaults; the interval is brought
    /// into 1 to 120 minutes.
    pub fn load(path: &Path) -> AutosaveSettings {
        let loaded = std::fs::read(path).ok().and_then(|b| serde_json::from_slice::<AutosaveSettings>(&b).ok());
        loaded.unwrap_or_default().clamped()
    }

    /// Writes the settings (creating the folder), with the interval brought into 1 to 120 minutes.
    pub fn save(&self, path: &Path) -> Result<(), IoError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| IoError::io(parent, e))?;
        }
        let json = serde_json::to_vec_pretty(&self.clamped()).map_err(|e| IoError::Json {
            path: path.to_path_buf(),
            member: "autosave settings".into(),
            source: e,
        })?;
        save::write_atomically(path, &json)
    }

    /// The same settings with the interval inside 1 to 120 minutes.
    pub fn clamped(self) -> AutosaveSettings {
        AutosaveSettings { minutes: self.minutes.clamp(MIN_MINUTES, MAX_MINUTES), ..self }
    }

    /// The time between two autosaves.
    pub fn interval(&self) -> Duration {
        Duration::from_secs(u64::from(self.clamped().minutes) * 60)
    }
}

/// What `<id>.json` says about a recovery file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sidecar {
    /// The file the project was opened from or last saved to; `None` for a project never saved.
    /// Kept as text (a name that is not valid UTF-8 is stored with replacement characters).
    pub original: Option<String>,
    /// When the recovery file was written, in seconds since 1970.
    pub saved_at: u64,
    /// The Sheaf version that wrote it.
    pub sheaf_version: String,
    /// The process id of the instance that wrote it (for the user's information only).
    pub pid: u32,
}

impl Sidecar {
    /// The original file as a path.
    pub fn original_path(&self) -> Option<PathBuf> {
        self.original.as_ref().map(PathBuf::from)
    }
}

/// What is known about a lock file.
enum Probe {
    /// There is no lock file.
    Missing,
    /// The lock file exists and nobody holds it. The lock is ours until the file is dropped.
    Free(File),
    /// A running instance holds it (or it cannot be told, which counts the same: files of a
    /// possibly live instance are never touched).
    Held,
}

fn lock_probe(lock: &Path) -> Probe {
    match std::fs::symlink_metadata(lock) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Probe::Missing,
        Ok(m) if m.is_file() => {}
        _ => return Probe::Held,
    }
    let file = OpenOptions::new().read(true).write(true).open(lock).or_else(|_| File::open(lock));
    let Ok(file) = file else { return Probe::Held };
    match file.try_lock() {
        Ok(()) => Probe::Free(file),
        Err(_) => Probe::Held,
    }
}

/// Removes a regular file; a missing file is fine, a link or folder is left alone.
fn remove_regular(path: &Path) -> Result<(), IoError> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.is_file() => std::fs::remove_file(path).or_else(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Ok(()),
            _ => Err(e),
        }),
        _ => Ok(()),
    }
    .map_err(|e| IoError::io(path, e))
}

fn unix_seconds(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Counts sessions so that two in one process get different ids.
static SESSION_COUNTER: AtomicU32 = AtomicU32::new(0);

/// One running app instance's claim on the recovery folder. Create it at start-up, call [`save`]
/// while the project is modified, [`clear`] when it becomes clean and [`finish`] on a clean exit.
///
/// Dropping a session releases the lock but keeps the recovery files, so a panic (which unwinds)
/// leaves the work behind for the next start, just as a crash does. A session can be moved to and
/// used from a worker thread; wrap it in an `Arc` to share it.
///
/// [`save`]: Session::save
/// [`clear`]: Session::clear
/// [`finish`]: Session::finish
#[derive(Debug)]
pub struct Session {
    dir: PathBuf,
    id: String,
    lock: File,
}

impl Session {
    /// Creates the folder if needed, picks a session id and holds its lock file.
    pub fn start(dir: &Path) -> Result<Session, IoError> {
        std::fs::create_dir_all(dir).map_err(|e| IoError::io(dir, e))?;
        let pid = std::process::id();
        let now = unix_seconds(SystemTime::now());
        let mut last_error = None;
        for _ in 0..100 {
            let n = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
            let id = format!("{pid}-{now}-{n}");
            let lock_path = dir.join(format!("{id}.lock"));
            // create_new never follows a link and never reuses a file of another instance.
            let lock = match OpenOptions::new().read(true).write(true).create_new(true).open(&lock_path) {
                Ok(f) => f,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(IoError::io(&lock_path, e)),
            };
            match lock.try_lock() {
                Ok(()) => return Ok(Session { dir: dir.to_path_buf(), id, lock }),
                Err(e) => {
                    let _ = std::fs::remove_file(&lock_path);
                    let e = match e {
                        TryLockError::Error(e) => e,
                        TryLockError::WouldBlock => std::io::Error::from(std::io::ErrorKind::WouldBlock),
                    };
                    last_error = Some(IoError::io(&lock_path, e));
                }
            }
        }
        Err(last_error
            .unwrap_or_else(|| IoError::io(dir, std::io::Error::other("no free session name in the recovery folder"))))
    }

    /// The session id, `<pid>-<unix seconds>-<n>`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The recovery folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Where this session's project copy goes: `<dir>/<id>.sheaf`.
    pub fn project_path(&self) -> PathBuf {
        self.dir.join(format!("{}.sheaf", self.id))
    }

    fn sidecar_path(&self) -> PathBuf {
        self.dir.join(format!("{}.json", self.id))
    }

    fn lock_path(&self) -> PathBuf {
        self.dir.join(format!("{}.lock", self.id))
    }

    /// Writes `project` as `<id>.sheaf` (atomically, in the native format) and then the sidecar that
    /// names `original`, the file the user's project belongs to. Returns the project file's path.
    pub fn save(&self, project: &Project, original: Option<&Path>) -> Result<PathBuf, IoError> {
        let path = self.project_path();
        project_file::save(project, &path)?;
        let sidecar = Sidecar {
            original: original.map(|p| p.to_string_lossy().into_owned()),
            saved_at: unix_seconds(SystemTime::now()),
            sheaf_version: env!("CARGO_PKG_VERSION").to_string(),
            pid: std::process::id(),
        };
        let sidecar_path = self.sidecar_path();
        let json = serde_json::to_vec_pretty(&sidecar).map_err(|e| IoError::Json {
            path: sidecar_path.clone(),
            member: "recovery sidecar".into(),
            source: e,
        })?;
        save::write_atomically(&sidecar_path, &json)?;
        Ok(path)
    }

    /// Removes this session's project copy and sidecar, for when the project has become clean. The
    /// session goes on running and can save again later.
    pub fn clear(&self) -> Result<(), IoError> {
        let project = remove_regular(&self.project_path());
        let sidecar = remove_regular(&self.sidecar_path());
        project.and(sidecar)
    }

    /// A clean exit: removes the project copy, the sidecar and the lock file.
    pub fn finish(self) -> Result<(), IoError> {
        let cleared = self.clear();
        let lock_path = self.lock_path();
        drop(self.lock);
        cleared.and(remove_regular(&lock_path))
    }
}

/// A recovery left behind by an instance that is gone.
#[derive(Clone, Debug)]
pub struct Recovery {
    /// The session id, which is also the stem of the file names.
    pub id: String,
    /// The `<id>.sheaf` file.
    pub project: PathBuf,
    /// What `<id>.json` says; `None` when it is missing or unreadable.
    pub sidecar: Option<Sidecar>,
    /// When the project file was last written.
    pub modified: SystemTime,
}

impl Recovery {
    fn sibling(&self, extension: &str) -> PathBuf {
        self.project.with_extension(extension)
    }
}

/// A session id is exactly what [`Session::start`] makes: `<pid>-<unix seconds>-<n>`, three runs of
/// digits. Anything else in the folder (a user's own `thesis.sheaf`, a `config.json`) is not ours
/// and is never listed, offered or deleted.
fn valid_id(id: &str) -> bool {
    let mut parts = id.split('-');
    let digits = |p: Option<&str>| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    digits(parts.next()) && digits(parts.next()) && digits(parts.next()) && parts.next().is_none()
}

/// True for the temporary file of an atomic write of a recovery file: `.<id>.sheaf.<pid>.tmp` or
/// `.<id>.json.<pid>.tmp`.
fn is_recovery_temporary(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('.').and_then(|n| n.strip_suffix(".tmp")) else { return false };
    let Some((rest, pid)) = rest.rsplit_once('.') else { return false };
    let Some((id, ext)) = rest.rsplit_once('.') else { return false };
    !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit()) && matches!(ext, "sheaf" | "json") && valid_id(id)
}

fn read_sidecar(path: &Path) -> Option<Sidecar> {
    // A sidecar is a few hundred bytes; a link or a huge file is not one.
    std::fs::symlink_metadata(path).ok().filter(|m| m.is_file() && m.len() <= 1 << 20)?;
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Every recovery in `dir` whose instance is gone, newest first. A project file counts when its lock
/// file is missing or not held. Files of a running instance (this process included), temporary
/// files, links and anything that does not look like a recovery are not listed. A missing folder
/// gives an empty list.
pub fn list_orphans(dir: &Path) -> Vec<Recovery> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let Some(id) = name.strip_suffix(".sheaf").filter(|id| valid_id(id)) else { continue };
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if !meta.is_file() {
            continue;
        }
        match lock_probe(&dir.join(format!("{id}.lock"))) {
            Probe::Held => continue,
            Probe::Missing | Probe::Free(_) => {}
        }
        let sidecar = read_sidecar(&dir.join(format!("{id}.json")));
        let modified = meta.modified().unwrap_or(UNIX_EPOCH);
        found.push(Recovery { id: id.to_string(), project: path.clone(), sidecar, modified });
    }
    found.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| b.id.cmp(&a.id)));
    found
}

/// Removes a recovery's project file, sidecar and lock file. Fails, and removes nothing, when an
/// instance holds the lock.
pub fn discard(recovery: &Recovery) -> Result<(), IoError> {
    let lock_path = recovery.sibling("lock");
    let held = match lock_probe(&lock_path) {
        Probe::Held => {
            let e = std::io::Error::new(std::io::ErrorKind::WouldBlock, "a running Sheaf is using this recovery");
            return Err(IoError::io(&lock_path, e));
        }
        Probe::Free(f) => Some(f),
        Probe::Missing => None,
    };
    let project = remove_regular(&recovery.project);
    let sidecar = remove_regular(&recovery.sibling("json"));
    drop(held);
    let lock = remove_regular(&lock_path);
    project.and(sidecar).and(lock)
}

/// Reads the recovered project.
pub fn load(recovery: &Recovery) -> Result<Project, IoError> {
    project_file::load(&recovery.project)
}

/// Discards every recovery of a gone instance whose project file is older than `max_age` at `now`,
/// and returns how many. Files of running instances are never touched. Leftovers without a project
/// file (a lock, a sidecar or a temporary file) older than `max_age` are removed too.
pub fn prune(dir: &Path, max_age: Duration, now: SystemTime) -> usize {
    let too_old = |t: SystemTime| now.duration_since(t).is_ok_and(|age| age > max_age);
    let dropped = list_orphans(dir).iter().filter(|r| too_old(r.modified) && discard(r).is_ok()).count();
    let Ok(entries) = std::fs::read_dir(dir) else { return dropped };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if !meta.is_file() || !too_old(meta.modified().unwrap_or(now)) {
            continue;
        }
        let temporary = is_recovery_temporary(name);
        let stem = |ext: &str| name.strip_suffix(ext).filter(|id| valid_id(id));
        let leftover = match (stem(".lock"), stem(".json")) {
            (Some(id), _) | (_, Some(id)) => !dir.join(format!("{id}.sheaf")).exists(),
            _ => false,
        };
        if !(temporary || leftover) {
            continue;
        }
        if stem(".lock").is_some() && !matches!(lock_probe(&path), Probe::Free(_)) {
            continue;
        }
        let _ = remove_regular(&path);
    }
    dropped
}

#[cfg(test)]
mod tests;
