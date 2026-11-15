//! The rules for files the assistant writes while attached to the desktop app (docs/spec/mcp.md
//! section 11.11): never replace a file, write atomically, and never touch the files that belong to the
//! open project. The headless server keeps its phase-1 behaviour and does not use them.

use crate::roots::canonical;
use anyhow::{Context, Result, bail};
use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Names ending like these are never written: a planted `.bak` would stop the backup of an Origin
/// project, and the others are the app's own project and lock files.
const RESERVED_SUFFIXES: [&str; 5] = [".opj", ".opju", ".sheaf", ".bak", ".lock"];

/// The files the live document is tied to, as the app knows them at the time of a call.
#[derive(Clone, Debug, Default)]
pub struct OpenFiles {
    /// The path the project was opened from or last saved to.
    pub project: Option<PathBuf>,
    /// The Origin file the project was read from (saving over it keeps a `.bak` copy).
    pub origin: Option<PathBuf>,
    /// The original path of a project that was recovered from an autosave.
    pub recovered_from: Option<PathBuf>,
}

/// Names that a new file may not take. Build it for each call with [`protected_set`].
#[derive(Clone, Debug, Default)]
pub struct Protected {
    /// Canonical parent folder and file name, the name lower-cased where the file system ignores case.
    names: Vec<(PathBuf, String)>,
    /// Canonical folders nothing may be written into (the recovery folder, the run folder).
    folders: Vec<PathBuf>,
}

/// Whether the platform's usual file systems ignore case (macOS and Windows).
const fn ignores_case() -> bool {
    cfg!(any(target_os = "macos", windows))
}

fn fold(name: &str) -> String {
    if ignores_case() { name.to_lowercase() } else { name.to_string() }
}

fn fold_path(path: &Path) -> PathBuf {
    if ignores_case() { PathBuf::from(path.to_string_lossy().to_lowercase()) } else { path.to_path_buf() }
}

/// The canonical parent folder of `path` and its file name; the folder need not exist.
fn split(path: &Path) -> Option<(PathBuf, String)> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => std::env::current_dir().ok()?,
    };
    let parent = canonical(&parent).unwrap_or(parent);
    Some((crate::roots::plain(parent), name))
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    crate::roots::with_suffix(path, suffix)
}

/// The protected names of the open files and the folders that are closed to the assistant, computed for
/// one call. `folders` are the app's recovery folder and run folder.
///
/// Protected: each file of `open` with its `.sheaf` companion and its `.bak` copy, and their temporary
/// names (`.<name>.<pid>.tmp`, as `sheaf_io` writes them). Names ending in `.opj`, `.opju`, `.sheaf`,
/// `.bak` or `.lock`, and anything inside `folders`, are refused by [`Protected::check`] as well.
pub fn protected_set(open: &OpenFiles, folders: &[PathBuf]) -> Protected {
    let mut out = Protected::default();
    let pid = std::process::id();
    for base in [&open.project, &open.origin, &open.recovered_from].into_iter().flatten() {
        let absolute =
            if base.is_absolute() { base.clone() } else { std::env::current_dir().unwrap_or_default().join(base) };
        let companion = suffixed(&absolute, ".sheaf");
        for path in [absolute.clone(), companion.clone(), suffixed(&absolute, ".bak")] {
            out.add(&path);
        }
        for path in [&absolute, &companion] {
            if let Some(name) = path.file_name() {
                out.add(&path.with_file_name(format!(".{}.{pid}.tmp", name.to_string_lossy())));
            }
        }
    }
    for f in folders {
        let canon = canonical(f).unwrap_or_else(|| f.clone());
        out.folders.push(fold_path(&crate::roots::plain(canon)));
    }
    out
}

impl Protected {
    fn add(&mut self, path: &Path) {
        if let Some((parent, name)) = split(path) {
            self.names.push((fold_path(&parent), fold(&name)));
        }
    }

    /// Refuses a name the assistant may not create. The path should be absolute.
    pub fn check(&self, path: &Path) -> Result<()> {
        let Some((parent, name)) = split(path) else {
            bail!("'{}' is not a file name", path.display());
        };
        // Windows drops a trailing dot or space and reads `name:stream` as a stream of `name`, so
        // `X.opj.bak.` would become the protected `X.opj.bak`. Such names are refused on every platform.
        if name.ends_with(['.', ' ']) || name.contains(':') {
            bail!("a file name may not end in a dot or a space or contain ':': {}", path.display());
        }
        let (parent_key, name_key) = (fold_path(&parent), fold(&name));
        if self.names.iter().any(|(p, n)| *p == parent_key && *n == name_key) {
            bail!("that name belongs to the project open in Sheaf: {}", path.display());
        }
        let lower = name.to_lowercase();
        if RESERVED_SUFFIXES.iter().any(|s| lower.ends_with(s)) {
            bail!(
                "that name is kept for project, backup and lock files (.opj, .opju, .sheaf, .bak, .lock): {}",
                path.display()
            );
        }
        if self.folders.iter().any(|f| parent_key.starts_with(f)) {
            bail!("that folder is kept for Sheaf's own recovery and link files: {}", path.display());
        }
        Ok(())
    }
}

fn will_not_overwrite(path: &Path) -> anyhow::Error {
    anyhow::anyhow!("will not overwrite: '{}' exists; choose a new file name", path.display())
}

/// Fails when anything is at `path`, a link (even one that points nowhere) and a file that is not a
/// regular file included: the check does not follow links.
fn refuse_existing(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(will_not_overwrite(path)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("could not check '{}'", path.display())),
    }
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A name for the temporary file: in the target's folder, never matching a project's temporary name,
/// hard to guess, and ending in the target's extension because the writers choose the format from it.
fn temp_name(path: &Path, n: u64) -> OsString {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() ^ (d.as_secs() as u32).rotate_left(16));
    let mut name = OsString::from(format!(".sheaf-mcp-{}-{n}-{nanos:08x}.tmp", std::process::id()));
    if let Some(ext) = path.extension() {
        name.push(".");
        name.push(ext);
    }
    name
}

/// Creates the temporary file next to `path` (`create_new`, so no link is followed) and returns its
/// path and the open handle.
fn create_temp(path: &Path) -> Result<(PathBuf, fs::File)> {
    for _ in 0..100 {
        let tmp = path.with_file_name(temp_name(path, COUNTER.fetch_add(1, Ordering::Relaxed)));
        match fs::OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(f) => return Ok((tmp, f)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e).with_context(|| format!("could not write '{}'", path.display())),
        }
    }
    bail!("could not make a temporary file next to '{}'", path.display())
}

/// Puts the finished temporary file at `path` without replacing anything: a hard link, which fails when
/// the name exists. Where the file system has no hard links it checks again and renames, which leaves a
/// small race. The temporary file is removed in every case.
fn publish(tmp: &Path, path: &Path, link: impl Fn(&Path, &Path) -> std::io::Result<()>) -> Result<()> {
    let result = match link(tmp, path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Err(will_not_overwrite(path)),
        Err(_) => refuse_existing(path)
            .and_then(|()| fs::rename(tmp, path).with_context(|| format!("could not write '{}'", path.display()))),
    };
    let _ = fs::remove_file(tmp);
    result
}

/// Fails early when `path` could not be written as a new file: a protected name, a name that exists, or
/// a folder that does not.
pub fn check_new(path: &Path, protected: &Protected) -> Result<()> {
    if !path.is_absolute() {
        bail!("'{}' is not an absolute path", path.display());
    }
    protected.check(path)?;
    refuse_existing(path)?;
    match path.parent().map(fs::metadata) {
        Some(Ok(m)) if m.is_dir() => Ok(()),
        _ => bail!("could not write '{}': the folder does not exist", path.display()),
    }
}

/// Writes `bytes` to a new file at `path`: never replaces a file, never follows a link, leaves no
/// partial file, refuses the names of `protected`.
pub fn write_new_bytes(path: &Path, bytes: &[u8], protected: &Protected) -> Result<()> {
    write_new_bytes_with(path, bytes, protected, |a, b| fs::hard_link(a, b))
}

fn write_new_bytes_with(
    path: &Path,
    bytes: &[u8],
    protected: &Protected,
    link: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<()> {
    check_new(path, protected)?;
    let (tmp, mut file) = create_temp(path)?;
    let written = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .with_context(|| format!("could not write '{}'", path.display()));
    drop(file);
    match written {
        Ok(()) => publish(&tmp, path, link),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// A temporary name that nothing is at yet, for a writer that creates the file itself.
fn free_temp(path: &Path) -> Result<PathBuf> {
    for _ in 0..100 {
        let tmp = path.with_file_name(temp_name(path, COUNTER.fetch_add(1, Ordering::Relaxed)));
        if refuse_existing(&tmp).is_ok() {
            return Ok(tmp);
        }
    }
    bail!("could not make a temporary name next to '{}'", path.display())
}

/// Like [`write_new_bytes`] for a writer that takes a path (the figure and Excel writers choose the
/// format from the extension, which the temporary name keeps). `write` gets a temporary path that is
/// free when it is handed out; the writer creates the file itself (the Excel writer refuses to
/// replace a file that is not a workbook, so the name cannot be reserved with an empty file).
pub fn write_new_with<T>(path: &Path, protected: &Protected, write: impl FnOnce(&Path) -> Result<T>) -> Result<T> {
    check_new(path, protected)?;
    let tmp = free_temp(path)?;
    match write(&tmp) {
        Ok(value) => publish(&tmp, path, |a, b| fs::hard_link(a, b)).map(|()| value),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            // The writer named the temporary file; the caller knows only the real one.
            let text = format!("{e:#}").replace(&tmp.display().to_string(), &path.display().to_string());
            Err(anyhow::anyhow!(text))
        }
    }
}

#[cfg(test)]
mod tests;
