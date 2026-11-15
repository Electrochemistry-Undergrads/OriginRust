//! The recently opened projects and the recently imported files, kept in `recent.json` in the
//! settings folder (next to the autosave settings) so that they survive a restart and are shared by
//! every running instance.
//!
//! A [`RecentList`] holds two lists, newest first, each capped at [`MAX_ENTRIES`]. A path is stored in
//! its canonical form and counts once, however it was spelled and (on Windows and macOS) in whatever
//! letter case. Changes go through [`update`], which takes a lock, re-reads the file, applies the
//! change and writes the file again, so two instances never wipe each other's entries.
//!
//! The rules are in `docs/spec/recent-files.md`.

use crate::error::IoError;
use crate::{recovery, save};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The most entries each list keeps (Sheaf's choice, not measured in Origin).
pub const MAX_ENTRIES: usize = 10;
/// The name of the file in the settings folder.
pub const FILE_NAME: &str = "recent.json";

/// How long [`update`] waits for another instance to finish its change before going on without the lock.
const LOCK_WAIT: Duration = Duration::from_secs(2);

/// Whether two spellings that differ only in letter case name the same file (Windows and macOS).
const FOLD_CASE: bool = cfg!(any(windows, target_os = "macos"));

/// Where the lists are kept: `recent.json` in the folder of the autosave settings
/// (`recovery::default_settings_path`). `None` when the environment gives no folder.
pub fn default_path() -> Option<PathBuf> {
    recovery::default_settings_path().map(|p| p.with_file_name(FILE_NAME))
}

/// The recent projects and recent imports, each newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecentList {
    /// Projects opened or saved, as canonical paths.
    pub projects: Vec<PathBuf>,
    /// Data files imported, as canonical paths.
    pub imports: Vec<PathBuf>,
}

impl RecentList {
    /// Reads the lists. A missing, unreadable or damaged file gives empty lists; a damaged entry is
    /// skipped and the others are kept. Entries that are not absolute paths are dropped, repeats are
    /// dropped, and each list is cut to [`MAX_ENTRIES`]. Nothing here touches the listed files.
    pub fn load(path: &Path) -> RecentList {
        let value = std::fs::read(path).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok());
        let Some(Value::Object(map)) = value else { return RecentList::default() };
        let read = |name: &str| -> Vec<PathBuf> {
            let mut out: Vec<PathBuf> = Vec::new();
            let Some(Value::Array(items)) = map.get(name) else { return out };
            for item in items {
                let Some(p) = serde_json::from_value::<Entry>(item.clone()).ok().and_then(Entry::into_path) else {
                    continue;
                };
                if p.is_absolute() && !out.iter().any(|q| same(q, &p)) {
                    out.push(p);
                }
            }
            out.truncate(MAX_ENTRIES);
            out
        };
        RecentList { projects: read("projects"), imports: read("imports") }
    }

    /// Writes the lists (creating the folder) without taking the lock; use [`update`] to change the
    /// file while other instances run.
    pub fn save(&self, path: &Path) -> Result<(), IoError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| IoError::io(parent, e))?;
        }
        let entries = |list: &[PathBuf]| Value::Array(list.iter().map(|p| Entry::from_path(p).to_value()).collect());
        let mut map = serde_json::Map::new();
        map.insert("projects".into(), entries(&self.projects));
        map.insert("imports".into(), entries(&self.imports));
        let json = serde_json::to_vec_pretty(&Value::Object(map)).map_err(|e| IoError::Json {
            path: path.to_path_buf(),
            member: "recent files".into(),
            source: e,
        })?;
        save::write_atomically(path, &json)
    }

    /// Puts a project at the top of the projects. The path is made canonical; an earlier entry for
    /// the same file goes away.
    pub fn push_project(&mut self, path: &Path) {
        push(&mut self.projects, normalize(path));
    }

    /// Puts imported files at the top of the imports, one after the other, so the last path of
    /// `paths` ends up first. Each path is made canonical; earlier entries for the same file go away.
    pub fn push_import<P: AsRef<Path>>(&mut self, paths: &[P]) {
        for p in paths {
            push(&mut self.imports, normalize(p.as_ref()));
        }
    }

    /// Removes a file from both lists, however it is spelled.
    pub fn remove(&mut self, path: &Path) {
        let given = absolute(path);
        let canonical = normalize(path);
        for list in [&mut self.projects, &mut self.imports] {
            list.retain(|q| !same(q, &given) && !same(q, &canonical));
        }
    }

    /// Empties the projects.
    pub fn clear_projects(&mut self) {
        self.projects.clear();
    }

    /// Empties the imports.
    pub fn clear_imports(&mut self) {
        self.imports.clear();
    }
}

/// Changes the lists in the file at `path`: waits for the file lock (up to two seconds, then goes on
/// without it), reads the file as it is now, applies `change`, writes the file and returns the new
/// lists. Entries another instance added in the meantime are kept.
pub fn update(path: &Path, change: impl FnOnce(&mut RecentList)) -> Result<RecentList, IoError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| IoError::io(parent, e))?;
    }
    let _lock = lock(path);
    let mut list = RecentList::load(path);
    change(&mut list);
    list.save(path)?;
    Ok(list)
}

/// Records a project opened or saved: [`RecentList::push_project`] through [`update`].
pub fn record_project(file: &Path, project: &Path) -> Result<RecentList, IoError> {
    update(file, |l| l.push_project(project))
}

/// Records imported files: [`RecentList::push_import`] through [`update`].
pub fn record_imports<P: AsRef<Path>>(file: &Path, imported: &[P]) -> Result<RecentList, IoError> {
    update(file, |l| l.push_import(imported))
}

/// Drops a file from both lists: [`RecentList::remove`] through [`update`].
pub fn forget(file: &Path, path: &Path) -> Result<RecentList, IoError> {
    update(file, |l| l.remove(path))
}

/// Empties the projects: [`RecentList::clear_projects`] through [`update`].
pub fn clear_projects(file: &Path) -> Result<RecentList, IoError> {
    update(file, RecentList::clear_projects)
}

/// Empties the imports: [`RecentList::clear_imports`] through [`update`].
pub fn clear_imports(file: &Path) -> Result<RecentList, IoError> {
    update(file, RecentList::clear_imports)
}

/// The path in the form the lists store: absolute and, as far as the disk allows, canonical (links
/// and `..` resolved; on Windows without the `\\?\` prefix). A file that does not exist keeps its
/// name and gets the canonical form of its folder; with no folder either, only the absolute form.
pub fn normalize(path: &Path) -> PathBuf {
    let abs = absolute(path);
    if let Ok(c) = std::fs::canonicalize(&abs) {
        return strip_verbatim(c);
    }
    if let (Some(parent), Some(name)) = (abs.parent(), abs.file_name())
        && let Ok(p) = std::fs::canonicalize(parent)
    {
        return strip_verbatim(p).join(name);
    }
    abs
}

fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Windows turns `C:\x` into `\\?\C:\x` when it resolves a path; this puts it back.
fn strip_verbatim(path: PathBuf) -> PathBuf {
    if cfg!(windows)
        && let Some(text) = path.to_str().and_then(strip_verbatim_text)
    {
        return PathBuf::from(text);
    }
    path
}

/// `\\?\C:\x` gives `C:\x` and `\\?\UNC\server\share\x` gives `\\server\share\x`; anything else gives `None`.
fn strip_verbatim_text(text: &str) -> Option<String> {
    let rest = text.strip_prefix(r"\\?\")?;
    if let Some(unc) = rest.strip_prefix(r"UNC\") {
        return Some(format!(r"\\{unc}"));
    }
    let mut chars = rest.chars();
    let drive = chars.next()?;
    (drive.is_ascii_alphabetic() && chars.next() == Some(':')).then(|| rest.to_string())
}

/// The bytes that decide whether two paths are the same entry.
fn key(path: &Path, fold: bool) -> Vec<u8> {
    match path.to_str() {
        Some(s) if fold => s.to_lowercase().into_bytes(),
        _ => path.as_os_str().as_encoded_bytes().to_vec(),
    }
}

fn same(a: &Path, b: &Path) -> bool {
    key(a, FOLD_CASE) == key(b, FOLD_CASE)
}

/// Puts `path` first, drops its earlier entry and cuts the list.
fn push(list: &mut Vec<PathBuf>, path: PathBuf) {
    list.retain(|q| !same(q, &path));
    list.insert(0, path);
    list.truncate(MAX_ENTRIES);
}

/// Holds the lock file next to the lists; dropping it lets the next instance in.
pub(crate) fn lock(path: &Path) -> Option<std::fs::File> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let lock_path = path.with_file_name(format!("{name}.lock"));
    let file = OpenOptions::new().create(true).truncate(false).write(true).open(lock_path).ok()?;
    let start = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Some(file),
            Err(TryLockError::WouldBlock) if start.elapsed() < LOCK_WAIT => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}

/// One path in the file: plain text when it is valid text, else the raw units of the system.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Entry {
    Text(String),
    Bytes { bytes: Vec<u8> },
    Wide { wide: Vec<u16> },
}

impl Entry {
    fn from_path(path: &Path) -> Entry {
        if let Some(s) = path.to_str() {
            return Entry::Text(s.to_string());
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            Entry::Bytes { bytes: path.as_os_str().as_bytes().to_vec() }
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            Entry::Wide { wide: path.as_os_str().encode_wide().collect() }
        }
        #[cfg(not(any(unix, windows)))]
        {
            Entry::Text(path.to_string_lossy().into_owned())
        }
    }

    fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// The path, or `None` for the other system's raw form.
    fn into_path(self) -> Option<PathBuf> {
        match self {
            Entry::Text(s) => Some(PathBuf::from(s)),
            #[cfg(unix)]
            Entry::Bytes { bytes } => {
                use std::os::unix::ffi::OsStringExt;
                Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
            }
            #[cfg(windows)]
            Entry::Wide { wide } => {
                use std::os::windows::ffi::OsStringExt;
                Some(PathBuf::from(std::ffi::OsString::from_wide(&wide)))
            }
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
