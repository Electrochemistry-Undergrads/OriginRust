//! Opening and saving projects.

use anyhow::{Context, Result};
use serde::Serialize;
use sheaf_core::Project;
use std::path::{Path, PathBuf};

/// True for the files that open as a project: `.sheaf`, `.opj` and `.opju`.
pub fn is_project(path: &Path) -> bool {
    sheaf_io::is_project_file(path)
}

/// A project read from a file.
#[derive(Debug)]
pub struct Loaded {
    pub project: Project,
    /// Things that were skipped or need a look.
    pub warnings: Vec<String>,
    /// The file is an Origin project of which Sheaf reads only part: saving over it replaces it
    /// with a partial rewrite.
    pub foreign: bool,
    /// The companion `x.opj.sheaf` was opened instead of `x.opj`.
    pub companion_used: bool,
}

/// Opens a `.sheaf` project or reads an Origin `.opj` project.
pub fn load_project(path: &Path) -> Result<Loaded> {
    let opened =
        sheaf_io::open_project(path).with_context(|| format!("could not open project '{}'", path.display()))?;
    Ok(Loaded {
        project: opened.project,
        warnings: opened.warnings,
        foreign: opened.foreign,
        companion_used: opened.companion_used,
    })
}

/// What a save wrote.
#[derive(Clone, Debug, Serialize)]
pub struct SaveReport {
    /// The file asked for (with `.opj` added when the name had no extension).
    pub path: PathBuf,
    /// The companion `<name>.opj.sheaf` written, when the `.opj` cannot hold everything.
    pub companion: Option<PathBuf>,
    /// A companion of an earlier save that was deleted because nothing is lost any more.
    pub companion_removed: Option<PathBuf>,
    /// The copy kept of the Origin file that was overwritten.
    pub backup: Option<PathBuf>,
    /// What only the companion holds.
    pub warnings: Vec<String>,
    /// Why the `.opj` was not written. Only the companion exists then, and the caller should report
    /// an error because the file asked for is missing.
    pub refusal: Option<String>,
}

/// True when both paths name the same file (or the same missing path).
pub fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Saves the project: `.opj` by default (a path without extension gets `.opj`), with a companion
/// `<name>.opj.sheaf` when the Origin file cannot hold everything; an explicit `.sheaf` path writes
/// a Sheaf project. When the Origin writer refuses the project only the companion is written and
/// `refusal` says why. With `backup_if_foreign` a copy of the file about to be replaced is kept
/// first (set it when that file is an Origin project this run read).
pub fn save_project(project: &Project, path: &Path, backup_if_foreign: bool) -> Result<SaveReport> {
    let target = sheaf_io::save::default_extension(path);
    let backup = if backup_if_foreign {
        sheaf_io::save::backup_original(&target)
            .with_context(|| format!("could not keep a copy of '{}'", target.display()))?
    } else {
        None
    };
    let outcome =
        sheaf_io::save::save_project(project, path).with_context(|| format!("could not save '{}'", path.display()))?;
    Ok(SaveReport {
        path: outcome.path,
        companion: outcome.companion,
        companion_removed: outcome.companion_removed,
        backup,
        warnings: outcome.warnings,
        refusal: outcome.refusal,
    })
}
