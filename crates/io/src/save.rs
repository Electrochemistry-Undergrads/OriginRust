//! Saving a project in its default format, `.opj`, without losing data.
//!
//! The `.opj` writer cannot store everything a project holds (graphs, folders, column formulas
//! and so on). When it leaves something out, a companion file `<name>.opj.sheaf` is written next
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
    let mut outcome = SaveOutcome { path: path.clone(), ..SaveOutcome::default() };
    if !extension_is(&path, "opj") {
        project_file::save(project, &path)?;
        outcome.main_written = true;
        return Ok(outcome);
    }
    let companion = companion_path(&path);
    let name = path.file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty()).unwrap_or("Project");
    let written = match opj::write_named(project, name, opj::julian_day_now()) {
        Ok(w) => w,
        Err(IoError::OpjWrite { reason }) => {
            // Nothing of the .opj is written; the companion keeps the data.
            project_file::save_companion(project, &companion)?;
            outcome.refusal = Some(reason);
            outcome.companion = Some(companion);
            return Ok(outcome);
        }
        Err(e) => return Err(e),
    };
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

/// Writes to a temporary file next to `path` and renames it over `path`.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), IoError> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "project".into());
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let result = std::fs::write(&tmp, bytes)
        .map_err(|e| IoError::io(&tmp, e))
        .and_then(|()| std::fs::rename(&tmp, path).map_err(|e| IoError::io(path, e)));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests;
