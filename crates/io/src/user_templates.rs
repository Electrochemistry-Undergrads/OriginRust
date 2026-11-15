//! User graph templates on disk (File > Save Template As, Plot > User Templates).
//!
//! One file per template, `<name>.sheaftemplate`, in the folder `templates` of the settings folder
//! (see [`default_dir`]). The file is the `.sheaf` archive of a project that holds one graph without
//! data, with the manifest key `"template": "graph"` (as a window file has `"window"`); it never
//! holds a workbook. The name of a template is its file name without the extension, so renaming is
//! renaming the file, and two instances never disagree about a list kept in a file of its own.
//! Names are matched without regard to letter case.
//!
//! A file that is damaged, from a newer version or not a template is left out of [`list`] when its
//! manifest cannot be read and reported by [`load`] otherwise; one such file never hides the others.
//! The rules are in `docs/spec/graphing.md`, section 21. Origin's own templates (`.otp`, `.otpu`)
//! are never read or written.

use crate::error::IoError;
use crate::{project_file, recovery};
use sheaf_core::{Project, UserTemplate, Window};
use std::path::{Path, PathBuf};

/// Extension of a template file, without the dot.
pub const EXTENSION: &str = "sheaftemplate";
/// The name of the folder, inside the settings folder, that holds the files.
pub const FOLDER_NAME: &str = "templates";
/// The most characters a template name keeps (Sheaf's choice, not measured in Origin).
pub const MAX_NAME_CHARS: usize = 60;
/// The manifest value that marks a graph template.
const MARKER: &str = "graph";

/// The folder of the template files: `templates` in the folder of the autosave settings
/// (`recovery::default_settings_path`). `None` when the environment gives no folder.
pub fn default_dir() -> Option<PathBuf> {
    recovery::default_settings_path().and_then(|p| p.parent().map(|d| d.join(FOLDER_NAME)))
}

/// Why a template name is refused.
#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("a template needs a name")]
    Empty,
}

/// What went wrong with a template.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error(transparent)]
    Name(#[from] NameError),
    #[error(transparent)]
    Io(#[from] IoError),
    /// No template has this name; `available` lists the saved ones.
    #[error("no user template named '{name}'{}", list_saved(.available))]
    Unknown { name: String, available: Vec<String> },
    /// A template of that name exists already (renaming onto another template).
    #[error("a user template named '{name}' exists already")]
    Exists { name: String },
}

fn list_saved(names: &[String]) -> String {
    if names.is_empty() {
        "; no templates are saved".to_string()
    } else {
        format!("; saved templates: {}", names.join(", "))
    }
}

/// Names that Windows keeps for devices (with or without an extension).
fn is_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|p| stem.strip_prefix(p).is_some_and(|n| matches!(n.as_bytes(), [b'1'..=b'9'])))
}

/// Makes a name fit as a template name and as a file name: control characters go, runs of white
/// space become one space, the ends are trimmed, `/ \ : * ? " < > |` become `_`, dots at either
/// end go (no hidden or dotted-out names), a name that Windows keeps for a device gets `_` added,
/// and the name is cut to [`MAX_NAME_CHARS`]. An empty result is refused. Two names are the same
/// template when this form matches in lower case.
pub fn clean_name(raw: &str) -> Result<String, NameError> {
    let mut out = String::new();
    let mut pending_space = false;
    for c in raw.chars() {
        if c.is_whitespace() {
            pending_space = !out.is_empty();
        } else if !c.is_control() {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(if "/\\:*?\"<>|".contains(c) { '_' } else { c });
        }
    }
    let cut: String = out.chars().take(MAX_NAME_CHARS).collect();
    let mut name = cut.trim_matches(|c: char| c == '.' || c.is_whitespace()).to_string();
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if is_device_name(&name) {
        name.push('_');
    }
    Ok(name)
}

fn fold(name: &str) -> String {
    name.to_lowercase()
}

/// A saved template as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateEntry {
    /// The name: the file name without the extension.
    pub name: String,
    pub path: PathBuf,
}

/// The path that a template of this name is saved to in `dir` (the name is cleaned first). The file
/// need not exist.
pub fn file_path(dir: &Path, name: &str) -> Result<PathBuf, NameError> {
    Ok(dir.join(format!("{}.{EXTENSION}", clean_name(name)?)))
}

fn has_extension(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

/// True for a file that is a user template: it has the extension `.sheaftemplate` (any letter
/// case) or `.sheaf`, and its manifest carries the template marker. Reads the manifest.
pub fn is_template_file(path: &Path) -> bool {
    let sheaf = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("sheaf"));
    (has_extension(path) || sheaf) && project_file::template_marker(path).as_deref() == Some(MARKER)
}

/// The saved templates of `dir`, sorted by name without regard to letter case. A missing folder
/// gives an empty list. Files without the extension, files that are no Sheaf archive and archives
/// without the template marker are left out; only the manifest of each file is read.
pub fn list(dir: &Path) -> Vec<TemplateEntry> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<TemplateEntry> = read
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| has_extension(p) && p.is_file() && is_template_file(p))
        .filter_map(|p| {
            let name = p.file_stem()?.to_string_lossy().into_owned();
            Some(TemplateEntry { name, path: p })
        })
        .collect();
    out.sort_by(|a, b| fold(&a.name).cmp(&fold(&b.name)).then_with(|| a.name.cmp(&b.name)));
    out
}

/// The names of the saved templates, in the order of [`list`].
pub fn names(dir: &Path) -> Vec<String> {
    list(dir).into_iter().map(|e| e.name).collect()
}

/// The saved template with this name (letter case does not matter; the name is cleaned first).
pub fn find(dir: &Path, name: &str) -> Option<TemplateEntry> {
    let key = fold(&clean_name(name).ok()?);
    list(dir).into_iter().find(|e| fold(&e.name) == key)
}

/// True when a template of this name is saved.
pub fn exists(dir: &Path, name: &str) -> bool {
    find(dir, name).is_some()
}

/// What [`save`] wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedTemplate {
    /// The name as stored (cleaned, spelled as typed).
    pub name: String,
    pub path: PathBuf,
    /// True when a template of that name was there and has been replaced.
    pub replaced: bool,
}

/// Saves the template under `template.name` (cleaned: see [`clean_name`]), creating the folder. A
/// template of the same name (letter case does not matter) is replaced and takes the new
/// spelling. The file is written to a temporary file and renamed over, so a crash never leaves half
/// a file. The file holds the look only: no workbook, no column.
pub fn save(dir: &Path, template: &UserTemplate) -> Result<SavedTemplate, TemplateError> {
    let name = clean_name(&template.name)?;
    std::fs::create_dir_all(dir).map_err(|e| IoError::io(dir, e))?;
    let path = file_path(dir, &name)?;
    let old = find(dir, &name);
    if let Some(old) = old.as_ref().filter(|o| o.path.file_name() != path.file_name()) {
        // Another spelling of the same name: on a file system that ignores letter case this is the
        // same file, so it goes before the new one is written.
        std::fs::remove_file(&old.path).map_err(|e| IoError::io(&old.path, e))?;
    }
    let mut graph = template.graph.clone();
    graph.name.clone_from(&name);
    let mut project = Project::new();
    project.windows.push(Window::Graph(graph));
    project_file::save_template_file(&project, &path, MARKER)?;
    Ok(SavedTemplate { name, path, replaced: old.is_some() })
}

/// Reads a template file. The name is the file name without the extension. Refused with
/// [`IoError::NotATemplateFile`] when the manifest has no template marker (a project or a window
/// file), and with [`IoError::Corrupt`] when the marker is unknown or the file holds no graph.
pub fn load_file(path: &Path) -> Result<UserTemplate, IoError> {
    let (project, marker) = project_file::load_template_file(path)?;
    let corrupt =
        |detail: String| IoError::Corrupt { path: path.to_path_buf(), member: "manifest.json".into(), detail };
    if marker != MARKER {
        return Err(corrupt(format!("names the template kind '{marker}', which this program does not know")));
    }
    let graph = project
        .windows
        .into_iter()
        .find_map(|w| match w {
            Window::Graph(g) => Some(g),
            Window::Workbook(_) => None,
        })
        .ok_or_else(|| corrupt("says the file holds a graph template, but it holds no graph".into()))?;
    let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| graph.name.clone());
    Ok(UserTemplate { name, graph })
}

/// Reads the saved template with this name (letter case does not matter). The error for an unknown
/// name lists the saved templates.
pub fn load(dir: &Path, name: &str) -> Result<UserTemplate, TemplateError> {
    match find(dir, name) {
        Some(entry) => Ok(load_file(&entry.path)?),
        None => Err(TemplateError::Unknown { name: name.trim().to_string(), available: names(dir) }),
    }
}

/// Renames a template (the file). Returns the new name as stored. Changing only the letter case is
/// allowed; a name that another template has is refused with [`TemplateError::Exists`].
pub fn rename(dir: &Path, old: &str, new: &str) -> Result<String, TemplateError> {
    let entry = find(dir, old)
        .ok_or_else(|| TemplateError::Unknown { name: old.trim().to_string(), available: names(dir) })?;
    let new = clean_name(new)?;
    if find(dir, &new).is_some_and(|other| fold(&other.name) != fold(&entry.name)) {
        return Err(TemplateError::Exists { name: new });
    }
    let to = file_path(dir, &new)?;
    if to != entry.path {
        std::fs::rename(&entry.path, &to).map_err(|e| IoError::io(&to, e))?;
    }
    Ok(new)
}

/// Deletes a template; false when there was none of that name.
pub fn delete(dir: &Path, name: &str) -> Result<bool, IoError> {
    let Some(entry) = find(dir, name) else { return Ok(false) };
    std::fs::remove_file(&entry.path).map_err(|e| IoError::io(&entry.path, e))?;
    Ok(true)
}

#[cfg(test)]
mod tests;
