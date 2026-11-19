//! User graph templates on disk (File > Save Template As, Plot > User Templates), and user
//! workbook templates (File > Save Template As on a workbook, File > New > From Template; T160).
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
//! The rules are in `docs/spec/graphing.md`, section 24. Origin's own templates (`.otp`, `.otpu`)
//! are never read or written.
//!
//! A workbook template is `<name>.sheafbooktemplate` in the same folder: the archive of a project
//! that holds one workbook without values, with the manifest key `"template": "workbook"`. Graph
//! and workbook templates have names of their own (a graph template and a workbook template may
//! share a name); the `*_book*` functions are the workbook versions of the graph ones.

use crate::error::IoError;
use crate::{project_file, recovery};
use sheaf_core::{BookTemplate, Project, UserTemplate, Window};
use std::path::{Path, PathBuf};

/// Extension of a template file, without the dot.
pub const EXTENSION: &str = "sheaftemplate";
/// The name of the folder, inside the settings folder, that holds the files.
pub const FOLDER_NAME: &str = "templates";
/// The most characters a template name keeps (Sheaf's choice, not measured in Origin).
pub const MAX_NAME_CHARS: usize = 60;
/// The manifest value that marks a graph template.
const MARKER: &str = "graph";
/// Extension of a workbook template file, without the dot.
pub const BOOK_EXTENSION: &str = "sheafbooktemplate";
/// The manifest value that marks a workbook template.
const BOOK_MARKER: &str = "workbook";

/// The two kinds of user template.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TemplateKind {
    Graph,
    Workbook,
}

impl TemplateKind {
    /// The extension of its files, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            TemplateKind::Graph => EXTENSION,
            TemplateKind::Workbook => BOOK_EXTENSION,
        }
    }

    fn marker(self) -> &'static str {
        match self {
            TemplateKind::Graph => MARKER,
            TemplateKind::Workbook => BOOK_MARKER,
        }
    }
}

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
    file_path_of(dir, name, TemplateKind::Graph)
}

/// The path of a template of this kind and name in `dir` (cleaned first; the file need not exist).
pub fn file_path_of(dir: &Path, name: &str, kind: TemplateKind) -> Result<PathBuf, NameError> {
    Ok(dir.join(format!("{}.{}", clean_name(name)?, kind.extension())))
}

fn has_extension(path: &Path, kind: TemplateKind) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(kind.extension()))
}

/// True for a file that is a user graph template: it has the extension `.sheaftemplate` (any letter
/// case) or `.sheaf`, and its manifest carries the template marker. Reads the manifest.
pub fn is_template_file(path: &Path) -> bool {
    kind_of_file(path) == Some(TemplateKind::Graph)
}

/// The kind of user template a file is: its manifest carries a template marker and its extension
/// is that kind's (any letter case) or `.sheaf`. `None` for anything else. Reads the manifest.
pub fn kind_of_file(path: &Path) -> Option<TemplateKind> {
    let sheaf = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("sheaf"));
    let marker = project_file::template_marker(path)?;
    [TemplateKind::Graph, TemplateKind::Workbook]
        .into_iter()
        .find(|k| marker == k.marker() && (sheaf || has_extension(path, *k)))
}

/// The saved graph templates of `dir`, sorted by name without regard to letter case. A missing folder
/// gives an empty list. Files without the extension, files that are no Sheaf archive and archives
/// without the template marker are left out; only the manifest of each file is read.
pub fn list(dir: &Path) -> Vec<TemplateEntry> {
    list_of(dir, TemplateKind::Graph)
}

/// The saved templates of one kind in `dir`, as [`list`] does for graph templates.
pub fn list_of(dir: &Path, kind: TemplateKind) -> Vec<TemplateEntry> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<TemplateEntry> = read
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| has_extension(p, kind) && p.is_file() && kind_of_file(p) == Some(kind))
        .filter_map(|p| {
            let name = p.file_stem()?.to_string_lossy().into_owned();
            Some(TemplateEntry { name, path: p })
        })
        .collect();
    out.sort_by(|a, b| fold(&a.name).cmp(&fold(&b.name)).then_with(|| a.name.cmp(&b.name)));
    out
}

/// The names of the saved graph templates, in the order of [`list`].
pub fn names(dir: &Path) -> Vec<String> {
    names_of(dir, TemplateKind::Graph)
}

/// The names of the saved templates of one kind, in the order of [`list_of`].
pub fn names_of(dir: &Path, kind: TemplateKind) -> Vec<String> {
    list_of(dir, kind).into_iter().map(|e| e.name).collect()
}

/// The saved graph template with this name (letter case does not matter; the name is cleaned first).
pub fn find(dir: &Path, name: &str) -> Option<TemplateEntry> {
    find_of(dir, name, TemplateKind::Graph)
}

/// The saved template of this kind and name, as [`find`].
pub fn find_of(dir: &Path, name: &str, kind: TemplateKind) -> Option<TemplateEntry> {
    let key = fold(&clean_name(name).ok()?);
    list_of(dir, kind).into_iter().find(|e| fold(&e.name) == key)
}

/// True when a graph template of this name is saved.
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

/// Writes `window` as the only window of a template file of `kind` named `name` (cleaned), replacing
/// a template of the same kind and name; `links` are the analysis runs of an analysis template.
fn save_window(
    dir: &Path,
    name: &str,
    kind: TemplateKind,
    mut window: Window,
    links: Vec<sheaf_core::analysis_link::AnalysisLink>,
) -> Result<SavedTemplate, TemplateError> {
    let name = clean_name(name)?;
    std::fs::create_dir_all(dir).map_err(|e| IoError::io(dir, e))?;
    let path = file_path_of(dir, &name, kind)?;
    let old = find_of(dir, &name, kind);
    if let Some(old) = old.as_ref().filter(|o| o.path.file_name() != path.file_name()) {
        // Another spelling of the same name: on a file system that ignores letter case this is the
        // same file, so it goes before the new one is written.
        std::fs::remove_file(&old.path).map_err(|e| IoError::io(&old.path, e))?;
    }
    match &mut window {
        Window::Graph(g) => g.name.clone_from(&name),
        Window::Workbook(b) => b.name.clone_from(&name),
        Window::Notes(n) => n.name.clone_from(&name),
    }
    let mut project = Project::new();
    project.windows.push(window);
    project.analysis_links = links;
    project_file::save_template_file(&project, &path, kind.marker())?;
    Ok(SavedTemplate { name, path, replaced: old.is_some() })
}

/// Saves the template under `template.name` (cleaned: see [`clean_name`]), creating the folder. A
/// template of the same name (letter case does not matter) is replaced and takes the new
/// spelling. The file is written to a temporary file and renamed over, so a crash never leaves half
/// a file. The file holds the look only: no workbook, no column.
pub fn save(dir: &Path, template: &UserTemplate) -> Result<SavedTemplate, TemplateError> {
    save_window(dir, &template.name, TemplateKind::Graph, Window::Graph(template.graph.clone()), Vec::new())
}

/// Saves a workbook template as [`save`] saves a graph template: `<name>.sheafbooktemplate`, a
/// workbook template of the same name replaced. Graph templates are never touched. The analysis
/// runs of an analysis template (T163) are saved with it.
pub fn save_book(dir: &Path, template: &BookTemplate) -> Result<SavedTemplate, TemplateError> {
    let window = Window::Workbook(template.book.clone());
    save_window(dir, &template.name, TemplateKind::Workbook, window, template.links.clone())
}

/// Reads a template file of `kind`: its project and its name (the file name without the extension).
fn read_kind(path: &Path, kind: TemplateKind) -> Result<(Project, String), IoError> {
    let (project, marker) = project_file::load_template_file(path)?;
    if marker != kind.marker() {
        let detail = if [MARKER, BOOK_MARKER].contains(&marker.as_str()) {
            format!("holds a {marker} template, not a {} template", kind.marker())
        } else {
            format!("names the template kind '{marker}', which this program does not know")
        };
        return Err(IoError::Corrupt { path: path.to_path_buf(), member: "manifest.json".into(), detail });
    }
    let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    Ok((project, name))
}

/// Reads a template file. The name is the file name without the extension. Refused with
/// [`IoError::NotATemplateFile`] when the manifest has no template marker (a project or a window
/// file), and with [`IoError::Corrupt`] when the marker is unknown or the file holds no graph.
pub fn load_file(path: &Path) -> Result<UserTemplate, IoError> {
    let (project, name) = read_kind(path, TemplateKind::Graph)?;
    let graph = project
        .windows
        .into_iter()
        .find_map(|w| match w {
            Window::Graph(g) => Some(g),
            Window::Workbook(_) | Window::Notes(_) => None,
        })
        .ok_or_else(|| IoError::Corrupt {
            path: path.to_path_buf(),
            member: "manifest.json".into(),
            detail: "says the file holds a graph template, but it holds no graph".into(),
        })?;
    let name = if name.is_empty() { graph.name.clone() } else { name };
    Ok(UserTemplate { name, graph })
}

/// Reads a workbook template file, as [`load_file`] reads a graph template.
pub fn load_book_file(path: &Path) -> Result<BookTemplate, IoError> {
    let (project, name) = read_kind(path, TemplateKind::Workbook)?;
    let mut project = project;
    let book = std::mem::take(&mut project.windows)
        .into_iter()
        .find_map(|w| match w {
            Window::Workbook(b) => Some(b),
            Window::Graph(_) | Window::Notes(_) => None,
        })
        .ok_or_else(|| IoError::Corrupt {
            path: path.to_path_buf(),
            member: "manifest.json".into(),
            detail: "says the file holds a workbook template, but it holds no workbook".into(),
        })?;
    let name = if name.is_empty() { book.name.clone() } else { name };
    Ok(BookTemplate { name, book, links: project.analysis_links })
}

fn unknown(dir: &Path, name: &str, kind: TemplateKind) -> TemplateError {
    TemplateError::Unknown { name: name.trim().to_string(), available: names_of(dir, kind) }
}

/// Reads the saved template with this name (letter case does not matter). The error for an unknown
/// name lists the saved templates.
pub fn load(dir: &Path, name: &str) -> Result<UserTemplate, TemplateError> {
    match find(dir, name) {
        Some(entry) => Ok(load_file(&entry.path)?),
        None => Err(unknown(dir, name, TemplateKind::Graph)),
    }
}

/// Reads the saved workbook template with this name, as [`load`].
pub fn load_book(dir: &Path, name: &str) -> Result<BookTemplate, TemplateError> {
    match find_of(dir, name, TemplateKind::Workbook) {
        Some(entry) => Ok(load_book_file(&entry.path)?),
        None => Err(unknown(dir, name, TemplateKind::Workbook)),
    }
}

/// Renames a template (the file). Returns the new name as stored. Changing only the letter case is
/// allowed; a name that another template has is refused with [`TemplateError::Exists`].
pub fn rename(dir: &Path, old: &str, new: &str) -> Result<String, TemplateError> {
    rename_of(dir, old, new, TemplateKind::Graph)
}

/// Renames a template of one kind, as [`rename`]; only templates of the same kind can clash.
pub fn rename_of(dir: &Path, old: &str, new: &str, kind: TemplateKind) -> Result<String, TemplateError> {
    let entry = find_of(dir, old, kind).ok_or_else(|| unknown(dir, old, kind))?;
    let new = clean_name(new)?;
    if find_of(dir, &new, kind).is_some_and(|other| fold(&other.name) != fold(&entry.name)) {
        return Err(TemplateError::Exists { name: new });
    }
    let to = file_path_of(dir, &new, kind)?;
    if to != entry.path {
        std::fs::rename(&entry.path, &to).map_err(|e| IoError::io(&to, e))?;
    }
    Ok(new)
}

/// Deletes a graph template; false when there was none of that name.
pub fn delete(dir: &Path, name: &str) -> Result<bool, IoError> {
    delete_of(dir, name, TemplateKind::Graph)
}

/// Deletes a template of one kind; false when there was none of that name.
pub fn delete_of(dir: &Path, name: &str, kind: TemplateKind) -> Result<bool, IoError> {
    let Some(entry) = find_of(dir, name, kind) else { return Ok(false) };
    std::fs::remove_file(&entry.path).map_err(|e| IoError::io(&entry.path, e))?;
    Ok(true)
}

#[cfg(test)]
mod tests;
