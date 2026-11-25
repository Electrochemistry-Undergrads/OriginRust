//! Saving a project in its default format, `.opj`, without losing data.
//!
//! The `.opj` writer cannot store everything a project holds (graph details, characters outside
//! Windows-1252 and so on; folders and column formulas are stored). When it leaves something out, a companion file `<name>.opj.sheaf` is written next
//! to the `.opj` with the whole project, and the caller is told what only the companion holds.
//! The companion is marked in its manifest, so a later save that loses nothing deletes it, while a
//! `.sheaf` file the user made is never touched. When the writer refuses the project (a name Origin
//! cannot store), no `.opj` is written at all; the companion alone keeps the data. The rules are
//! in `docs/adr/0008-opj-default-with-companion.md`.

use crate::error::IoError;
use crate::{opj, project_file};
use sheaf_core::Project;
use std::path::{Path, PathBuf};

/// What a call of [`save_project`] did.
#[derive(Clone, Debug, Default)]
pub struct SaveOutcome {
    /// The file the caller asked for (with `.opj` added when the name had no extension).
    pub path: PathBuf,
    /// True when `path` was written. False when the writer refused the project.
    pub main_written: bool,
    /// The companion file written, if any.
    pub companion: Option<PathBuf>,
    /// A companion of an earlier save that this save deleted because nothing is lost any more.
    pub companion_removed: Option<PathBuf>,
    /// What only the companion holds (the writer's warnings).
    pub warnings: Vec<String>,
    /// Why the `.opj` could not be written, when it was not.
    pub refusal: Option<String>,
}

impl SaveOutcome {
    /// One short line for a status bar.
    pub fn status(&self) -> String {
        match (&self.refusal, &self.companion) {
            (Some(_), Some(c)) => {
                format!("Could not write {}; saved the whole project as {} instead", self.path.display(), c.display())
            }
            (None, Some(c)) => {
                format!("Saved {}; graphs and other content it cannot hold are in {}", self.path.display(), c.display())
            }
            _ => format!("Saved {}", self.path.display()),
        }
    }

    /// The longer explanation for a message window, or `None` when the save was complete.
    pub fn details(&self) -> Option<String> {
        let companion = self.companion.as_ref()?;
        let mut text = String::new();
        if let Some(reason) = &self.refusal {
            text.push_str(&format!(
                "The Origin project {} was not written, so it was left as it was: {reason}\n\nRename the project's windows, sheets or columns as the message says, or pick another file name, and save again. Meanwhile the whole project is safe in {}.",
                self.path.display(),
                companion.display()
            ));
        } else {
            text.push_str(&format!(
                "An Origin project cannot hold everything this project has. {} holds all of it; {} holds the rest. Keep the two files together. Sheaf opens the .opj.sheaf file when you open the .opj, unless the .opj was changed in Origin afterwards.\n\nOnly {} holds:",
                companion.display(),
                self.path.display(),
                companion.display()
            ));
            for w in &self.warnings {
                text.push_str(&format!("\n- {w}"));
            }
        }
        Some(text)
    }
}

/// `x.opj.sheaf`: the companion file name of `x.opj`.
pub fn companion_path(opj: &Path) -> PathBuf {
    let mut name = opj.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(format!(".{}", project_file::EXTENSION));
    opj.with_file_name(name)
}

/// If `path` is `x.opj.sheaf`, the `x.opj` it accompanies.
pub fn companion_target(path: &Path) -> Option<PathBuf> {
    let stem = Path::new(path.file_stem()?);
    (extension_is(path, project_file::EXTENSION) && extension_is(stem, "opj")).then(|| path.with_file_name(stem))
}

fn extension_is(path: &Path, wanted: &str) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(wanted))
}

/// The path a save goes to: a name without extension gets `.opj`; every other name is kept.
pub fn default_extension(path: &Path) -> PathBuf {
    if path.extension().is_none() {
        let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
        name.push(".opj");
        path.with_file_name(name)
    } else {
        path.to_path_buf()
    }
}

/// Saves `project` to `path`. A `.opj` path (or one without extension, which becomes `.opj`) gets
/// an Origin project plus, when needed, the companion; any other path gets a `.sheaf` project.
pub fn save_project(project: &Project, path: &Path) -> Result<SaveOutcome, IoError> {
    let path = default_extension(path);
    if extension_is(&path, "opju") {
        // Nothing is written: the .opju writer does not exist (opju-format.md section 10).
        return Err(IoError::OpjuWrite { path });
    }
    let mut outcome = SaveOutcome { path: path.clone(), ..SaveOutcome::default() };
    if !extension_is(&path, "opj") {
        project_file::save(project, &path)?;
        outcome.main_written = true;
        return Ok(outcome);
    }
    let companion = companion_path(&path);
    let name = path.file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty()).unwrap_or("Project");
    let mut written = match opj::write_named(project, name, opj::julian_day_now()) {
        Ok(w) => w,
        Err(IoError::OpjWrite { reason }) => {
            ensure_companion_free(&companion)?;
            // Nothing of the .opj is written; the companion keeps the data.
            project_file::save_companion(project, &companion)?;
            outcome.refusal = Some(reason);
            outcome.companion = Some(companion);
            return Ok(outcome);
        }
        Err(e) => return Err(e),
    };
    // T283: the comments of workbooks and graphs go in their .opj labels; the other windows are not
    // written to the .opj, so their comments need the companion.
    let unwritten = project.window_comments.iter().filter(|c| {
        !matches!(
            project.window(c.window),
            Some(sheaf_core::project::Window::Workbook(_) | sheaf_core::project::Window::Graph(_))
        )
    });
    if unwritten.count() > 0 {
        written.warnings.push(
            "Window comments (Window Properties) of notes, layout and matrix windows are not stored in an .opj file."
                .to_string(),
        );
    }
    if project.page_setup != sheaf_core::page_setup::PageSetup::default() {
        written.warnings.push("Page Setup (paper, orientation and margins) is not stored in an .opj file.".to_string());
    }
    if !written.warnings.is_empty() {
        // Checked before anything is written, so a refusal leaves every file as it was.
        ensure_companion_free(&companion)?;
    }
    write_atomically(&path, &written.bytes)?;
    outcome.main_written = true;
    if written.warnings.is_empty() {
        if companion.exists() && project_file::is_companion(&companion) {
            std::fs::remove_file(&companion).map_err(|e| IoError::io(&companion, e))?;
            outcome.companion_removed = Some(companion);
        }
    } else {
        // Written after the .opj, so the companion is never older than the .opj it accompanies.
        project_file::save_companion(project, &companion)?;
        outcome.companion = Some(companion);
        outcome.warnings = written.warnings;
    }
    Ok(outcome)
}

/// Fails when `companion` is a file Sheaf did not write (the user's own file).
fn ensure_companion_free(companion: &Path) -> Result<(), IoError> {
    if companion.exists() && !project_file::is_companion(companion) {
        return Err(IoError::CompanionExists { path: companion.to_path_buf() });
    }
    Ok(())
}

/// Before an Origin file that Sheaf only partly reads is replaced by Sheaf's own output, keeps
/// the original: copies `path` to `<path>.bak` unless that copy exists already (the first copy is
/// the true original). Returns the copy when one was made.
pub fn backup_original(path: &Path) -> Result<Option<PathBuf>, IoError> {
    if !path.is_file() {
        return Ok(None);
    }
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".bak");
    let backup = path.with_file_name(name);
    // Copying through a symlink (even a dangling one) would write wherever it points.
    match backup.symlink_metadata() {
        // A link to an existing file counts as a backup that is already there, as it always has.
        Ok(m) if m.file_type().is_symlink() && !backup.exists() => {
            let e = std::io::Error::other("the backup path is a symbolic link; remove it first");
            return Err(IoError::io(&backup, e));
        }
        Ok(_) => return Ok(None),
        Err(_) => {}
    }
    let copy = std::fs::read(path).map_err(|e| IoError::io(path, e)).and_then(|bytes| {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup)
            .map_err(|e| IoError::io(&backup, e))?;
        f.write_all(&bytes).map_err(|e| IoError::io(&backup, e))?;
        // fs::copy kept the permissions of the original; so does the backup.
        if let Ok(m) = std::fs::metadata(path) {
            let _ = std::fs::set_permissions(&backup, m.permissions());
        }
        Ok(())
    });
    if let Err(e) = copy {
        if matches!(&e, IoError::Io { source, .. } if source.kind() == std::io::ErrorKind::AlreadyExists) {
            return Ok(None);
        }
        return Err(e);
    }
    Ok(Some(backup))
}

/// Writes to a temporary file next to `path` and renames it over `path`. Each write has its own
/// temporary file (process id and a count), so two writes of one file at once never share one and
/// the later rename finds its file (T230: a shared name made it fail with NotFound).
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), IoError> {
    static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "project".into());
    let n = WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_file_name(format!(".{name}.{}.{n}.tmp", std::process::id()));
    let result = std::fs::write(&tmp, bytes).map_err(|e| IoError::io(&tmp, e)).and_then(|()| rename_over(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// How long [`rename_over`] keeps trying while Windows refuses the rename (T454: one second ran out with eight
/// writers on a loaded machine; a virus scanner can hold a file briefly too). The crate's own tests, whose writers
/// race on purpose while other test runs load the machine, wait 30 s.
const RENAME_WAIT: std::time::Duration =
    if cfg!(test) { std::time::Duration::from_secs(30) } else { std::time::Duration::from_secs(3) };

/// Renames `from` over `to`. Windows refuses (access denied) while another rename over `to` is
/// under way, so that is tried again for a short while.
fn rename_over(from: &Path, to: &Path) -> Result<(), IoError> {
    let start = std::time::Instant::now();
    loop {
        match std::fs::rename(from, to) {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && start.elapsed() < RENAME_WAIT => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            r => return r.map_err(|e| IoError::io(to, e)),
        }
    }
}

#[cfg(test)]
mod tests;
