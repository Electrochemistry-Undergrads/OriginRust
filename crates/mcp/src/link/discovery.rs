//! The run folder, the lock and `mcp.json`: how the app announces its endpoint and how a client finds it
//! (docs/spec/mcp.md section 11.2).

use super::transport::Endpoint;
use serde_json::{Value, json};
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const LOCK_NAME: &str = "mcp.lock";
pub const INFO_NAME: &str = "mcp.json";

/// What `mcp.json` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Info {
    pub pid: u32,
    pub app: String,
    pub endpoint: Endpoint,
}

impl Info {
    pub fn to_json(&self) -> Value {
        json!({"v": 1, "pid": self.pid, "app": self.app, "endpoint": self.endpoint.to_json()})
    }

    pub fn from_json(v: &Value) -> Result<Info, String> {
        if v.get("v").and_then(Value::as_u64) != Some(1) {
            return Err("mcp.json is from another version of Sheaf".to_string());
        }
        let pid = v.get("pid").and_then(Value::as_u64).and_then(|p| u32::try_from(p).ok()).unwrap_or(0);
        let app = v.get("app").and_then(Value::as_str).unwrap_or_default().to_string();
        let endpoint = Endpoint::from_json(v.get("endpoint").ok_or("mcp.json has no endpoint")?)?;
        Ok(Info { pid, app, endpoint })
    }
}

// ----- the folder -------------------------------------------------------------------

/// The id of the user the process runs as, found as the owner of a file the process makes in `dir` (and
/// removes at once).
#[cfg(unix)]
fn own_uid(dir: &Path) -> io::Result<u32> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let probe = dir.join(format!(".probe.{}.{n}", std::process::id()));
    let file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&probe)?;
    let uid = file.metadata().map(|m| m.uid());
    drop(file);
    let _ = std::fs::remove_file(&probe);
    uid
}

/// Checks the run folder `dir`, and with `create` makes it (and every missing level above it) first.
///
/// On Unix each folder made has mode 0700. An existing `dir` must be a real folder, not a link, owned by
/// the user the process runs as; if it is owned by the user and open to the group or others it is set
/// to 0700 and looked at again; if it is not, or still is open, it is refused. On Windows the folder
/// keeps the rights it inherits from the user's profile and only the link check applies.
pub fn check_dir(dir: &Path, create: bool) -> Result<(), String> {
    #[cfg(unix)]
    {
        check_dir_with(dir, create, &own_uid)
    }
    #[cfg(not(unix))]
    {
        check_dir_plain(dir, create)
    }
}

#[cfg(not(unix))]
fn check_dir_plain(dir: &Path, create: bool) -> Result<(), String> {
    match std::fs::symlink_metadata(dir) {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound && create => {
            std::fs::create_dir_all(dir).map_err(|e| format!("could not create the folder: {e}"))?;
        }
        Err(e) if e.kind() == ErrorKind::NotFound => return Err("does not exist".to_string()),
        Err(e) => return Err(format!("cannot be read: {e}")),
    }
    let m = std::fs::symlink_metadata(dir).map_err(|e| format!("cannot be read: {e}"))?;
    if m.file_type().is_symlink() {
        return Err("is a link, not a folder".to_string());
    }
    if !m.is_dir() {
        return Err("is not a folder".to_string());
    }
    Ok(())
}

/// [`check_dir`] with the user's id given by `me`, so that a test can play another owner.
#[cfg(unix)]
pub(crate) fn check_dir_with(dir: &Path, create: bool, me: &dyn Fn(&Path) -> io::Result<u32>) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    match std::fs::symlink_metadata(dir) {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound && create => {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(dir)
                .map_err(|e| format!("could not create the folder: {e}"))?;
        }
        Err(e) if e.kind() == ErrorKind::NotFound => return Err("does not exist".to_string()),
        Err(e) => return Err(format!("cannot be read: {e}")),
    }
    // Look again after the folder may have been made: this is the one that counts.
    let meta = std::fs::symlink_metadata(dir).map_err(|e| format!("cannot be read: {e}"))?;
    if meta.file_type().is_symlink() {
        return Err("is a link, not a folder".to_string());
    }
    if !meta.is_dir() {
        return Err("is not a folder".to_string());
    }
    let uid = me(dir).map_err(|e| format!("cannot be checked: {e}"))?;
    if meta.uid() != uid {
        return Err("belongs to another user".to_string());
    }
    if meta.mode() & 0o077 != 0 {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("is open to other users and could not be closed: {e}"))?;
        let again = std::fs::symlink_metadata(dir).map_err(|e| format!("cannot be read: {e}"))?;
        if again.mode() & 0o077 != 0 || again.uid() != uid {
            return Err("is open to other users".to_string());
        }
    }
    Ok(())
}

// ----- the lock ---------------------------------------------------------------------

fn lock_error(e: TryLockError) -> io::Error {
    match e {
        TryLockError::Error(e) => e,
        TryLockError::WouldBlock => io::Error::from(ErrorKind::WouldBlock),
    }
}

/// Why the app could not take the lock.
#[derive(Debug)]
pub enum LockError {
    /// Another process holds it: an app is listening.
    Busy,
    Other(String),
}

/// Takes the exclusive lock of `dir/mcp.lock` and keeps it as long as the file is held. The system drops
/// it when the process ends, also after a crash. A client that is only looking holds a shared lock for a
/// moment, so a refusal is tried again a few times before it counts as busy.
pub fn acquire_lock(dir: &Path) -> Result<File, LockError> {
    let path = dir.join(LOCK_NAME);
    match std::fs::symlink_metadata(&path) {
        Ok(m) if !m.is_file() => return Err(LockError::Other(format!("'{}' is not a plain file", path.display()))),
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(LockError::Other(format!("cannot read '{}': {e}", path.display()))),
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&path).map_err(|e| LockError::Other(format!("cannot open '{}': {e}", path.display())))?;
    for attempt in 0..4 {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) if attempt < 3 => std::thread::sleep(Duration::from_millis(50)),
            Err(TryLockError::WouldBlock) => return Err(LockError::Busy),
            Err(TryLockError::Error(e)) => {
                return Err(LockError::Other(format!("cannot lock '{}': {e}", path.display())));
            }
        }
    }
    Err(LockError::Busy)
}

// ----- mcp.json ---------------------------------------------------------------------

/// Writes `mcp.json` (mode 0600) in `dir`: to a new temporary file first, then renamed into place.
pub fn write_info(dir: &Path, info: &Info) -> io::Result<()> {
    let tmp = dir.join(format!(".{INFO_NAME}.{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = options
        .open(&tmp)
        .and_then(|mut f| f.write_all(info.to_json().to_string().as_bytes()).and_then(|()| f.sync_all()))
        .and_then(|()| std::fs::rename(&tmp, dir.join(INFO_NAME)));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Removes `mcp.json` and the socket, a stale one included. A folder or link at those names is left.
pub fn remove_endpoint_files(dir: &Path) {
    for name in [INFO_NAME, super::transport::SOCKET_NAME] {
        let path = dir.join(name);
        if let Ok(m) = std::fs::symlink_metadata(&path)
            && !m.is_dir()
        {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// The pid in `mcp.json`, when it can be read.
pub fn read_pid(dir: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(dir.join(INFO_NAME)).ok()?;
    Info::from_json(&serde_json::from_str(&text).ok()?).ok().map(|i| i.pid).filter(|p| *p != 0)
}

// ----- the client's side ------------------------------------------------------------

/// What a client learns from one folder: where the app listens, or why it does not.
///
/// The folder is checked as the app checks it, except that it is never created. Then the lock is
/// tried: a shared lock that succeeds means no app holds it, and the old `mcp.json` is ignored. When
/// the lock is held, `mcp.json` is read; the app writes it just after the lock, so a missing file is
/// waited for up to `wait`.
pub fn probe(dir: &Path, wait: Duration) -> Result<Info, String> {
    check_dir(dir, false)?;
    let lock_path = dir.join(LOCK_NAME);
    let lock = match File::open(&lock_path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => return Err("no lock".to_string()),
        Err(e) => return Err(format!("cannot open the lock: {e}")),
    };
    match lock.try_lock_shared() {
        Ok(()) => {
            let _ = lock.unlock();
            return Err("no lock".to_string());
        }
        Err(TryLockError::WouldBlock) => {}
        Err(e) => return Err(format!("cannot test the lock: {}", lock_error(e))),
    }
    drop(lock);
    let start = Instant::now();
    loop {
        match std::fs::read_to_string(dir.join(INFO_NAME)) {
            Ok(text) => {
                let info = serde_json::from_str::<Value>(&text)
                    .map_err(|e| format!("mcp.json cannot be read: {e}"))
                    .and_then(|v| Info::from_json(&v))?;
                if let Endpoint::Unix(path) = &info.endpoint
                    && !same_folder(path.parent(), dir)
                {
                    return Err("mcp.json names a socket outside the folder".to_string());
                }
                return Ok(info);
            }
            Err(e) if e.kind() == ErrorKind::NotFound && start.elapsed() < wait => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) if e.kind() == ErrorKind::NotFound => return Err("mcp.json is missing".to_string()),
            Err(e) => return Err(format!("mcp.json cannot be read: {e}")),
        }
    }
}

fn same_folder(a: Option<&Path>, b: &Path) -> bool {
    let Some(a) = a else { return false };
    a == b || matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
}

/// `dir` and the reasons, as the message of a client that found nothing.
pub fn no_listener(looked: &[(PathBuf, String)]) -> String {
    let places: Vec<String> = looked.iter().map(|(d, why)| format!("{}: {why}", d.display())).collect();
    let list = if places.is_empty() { "no folder to look in".to_string() } else { places.join("; ") };
    format!(
        "no Sheaf is listening (looked in: {list}); turn on Tools > AI Assistants (MCP) in Sheaf, or start it with --mcp"
    )
}

#[cfg(test)]
mod tests;
