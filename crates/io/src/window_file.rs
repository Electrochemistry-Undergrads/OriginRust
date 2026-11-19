//! Window files: one workbook or one graph in a file of its own (File > Save Window As).
//!
//! A window file is the `.sheaf` archive of a small project (see `project_file`) with the key
//! `"window": "workbook"` or `"graph"` in its manifest and its own extension, `.sheafbook` or
//! `.sheafgraph`. A graph file also holds the workbooks its plots read, so that it opens with its
//! data. Opening one adds its windows to the open project with [`sheaf_core::Project::append_windows`];
//! it never replaces the project, and [`project_file::load`] refuses it.
//!
//! The rules are in `docs/spec/window-files.md`. Origin's own window files (`.ogw`, `.ogg`) are never
//! read or written.

use crate::error::IoError;
use crate::project_file;
use sheaf_core::{Project, Window, WindowId};
use std::path::{Path, PathBuf};

/// Extension of a workbook file, without the dot.
pub const BOOK_EXTENSION: &str = "sheafbook";
/// Extension of a graph file, without the dot.
pub const GRAPH_EXTENSION: &str = "sheafgraph";

/// Which kind of window a window file holds.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WindowFileKind {
    /// A workbook, in `.sheafbook`.
    Book,
    /// A graph with the workbooks it reads, in `.sheafgraph`.
    Graph,
}

impl WindowFileKind {
    /// The extension without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            WindowFileKind::Book => BOOK_EXTENSION,
            WindowFileKind::Graph => GRAPH_EXTENSION,
        }
    }

    /// The value of the manifest key: `workbook` or `graph`.
    pub fn marker(self) -> &'static str {
        match self {
            WindowFileKind::Book => "workbook",
            WindowFileKind::Graph => "graph",
        }
    }

    /// The kind that a manifest value names; `None` for an unknown value.
    pub fn from_marker(marker: &str) -> Option<Self> {
        match marker {
            "workbook" => Some(WindowFileKind::Book),
            "graph" => Some(WindowFileKind::Graph),
            _ => None,
        }
    }

    /// The kind that a file name's extension names (any letter case); `None` for any other name.
    pub fn from_extension(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        [WindowFileKind::Book, WindowFileKind::Graph].into_iter().find(|k| ext.eq_ignore_ascii_case(k.extension()))
    }

    /// The kind of a window; `None` for a Notes window, which has no window file yet (T154).
    pub fn of_window(window: &Window) -> Option<Self> {
        match window {
            Window::Workbook(_) => Some(WindowFileKind::Book),
            Window::Graph(_) => Some(WindowFileKind::Graph),
            Window::Notes(_) => None,
        }
    }
}

/// True for a file that [`load_window`] is meant for: its extension is `.sheafbook` or `.sheafgraph`
/// (any letter case), or it is a `.sheaf` file whose manifest carries the window marker (a window
/// file that was renamed). Cheap: only a `.sheaf` name makes it read the file's manifest. Use this
/// before [`crate::is_project_file`] when routing a file the owner opened or dropped.
pub fn is_window_file(path: &Path) -> bool {
    if WindowFileKind::from_extension(path).is_some() {
        return true;
    }
    let is_sheaf =
        path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(project_file::EXTENSION));
    is_sheaf && project_file::window_marker(path).is_some()
}

/// The kind that the manifest of `path` names; `None` when the file is no window file or cannot be
/// read. Unlike [`is_window_file`] this looks inside the file, whatever its name.
pub fn window_file_kind(path: &Path) -> Option<WindowFileKind> {
    project_file::window_marker(path).and_then(|m| WindowFileKind::from_marker(&m))
}

/// The path a save goes to: the name is kept when it already has the extension of `kind` (any letter
/// case); every other name gets that extension added after it, so `keep.sheaf` becomes
/// `keep.sheaf.sheafgraph` and a project file is never written over by a mistyped name.
pub fn default_extension(path: &Path, kind: WindowFileKind) -> PathBuf {
    if WindowFileKind::from_extension(path) == Some(kind) {
        return path.to_path_buf();
    }
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".");
    name.push(kind.extension());
    path.with_file_name(name)
}

/// What [`save_window`] wrote.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedWindow {
    pub kind: WindowFileKind,
    /// How many windows the file holds: 1 for a workbook, the graph plus its source books for a graph.
    pub windows: usize,
    /// What was dropped because it pointed outside the file (see `Project::extract_window`).
    pub notes: Vec<String>,
}

/// Saves the window `window` of `project` to `path` as a window file: a workbook alone, a graph with
/// the workbooks it reads. The file is written to a temporary file next to `path` and renamed over
/// it, so a crash never leaves half a file. `path` is used as given; see [`default_extension`].
pub fn save_window(project: &Project, window: WindowId, path: &Path) -> Result<SavedWindow, IoError> {
    let extract = project.extract_window(window).map_err(|_| IoError::NoSuchWindow { id: window.0 })?;
    let kind = WindowFileKind::of_window(&extract.project.windows[extract.primary])
        .ok_or(IoError::NoSuchWindow { id: window.0 })?;
    project_file::save_window_file(&extract.project, path, kind.marker())?;
    Ok(SavedWindow { kind, windows: extract.project.windows.len(), notes: extract.notes })
}

/// A window file read by [`load_window`].
#[derive(Clone, Debug)]
pub struct WindowFile {
    pub kind: WindowFileKind,
    /// The windows of the file (the saved window and, for a graph, its source books) with their
    /// ids as saved. Append it with `Project::append_windows`.
    pub project: Project,
    /// The position of the saved window in `project.windows`: the first window of the kind
    /// (the graph of a graph file).
    pub primary: usize,
}

/// Reads a window file. Refused with [`IoError::NotAWindowFile`] when the manifest has no window
/// marker (a project), and with [`IoError::Corrupt`] when the marker is unknown or the file holds no
/// window of its kind. Never replaces anything: the caller appends the result.
pub fn load_window(path: &Path) -> Result<WindowFile, IoError> {
    let (project, marker) = project_file::load_window_file(path)?;
    let corrupt =
        |detail: String| IoError::Corrupt { path: path.to_path_buf(), member: "manifest.json".into(), detail };
    let kind = WindowFileKind::from_marker(&marker)
        .ok_or_else(|| corrupt(format!("names the window kind '{marker}', which this program does not know")))?;
    let primary = project
        .windows
        .iter()
        .position(|w| WindowFileKind::of_window(w) == Some(kind))
        .ok_or_else(|| corrupt(format!("says the file holds a {marker}, but it holds none")))?;
    Ok(WindowFile { kind, project, primary })
}

#[cfg(test)]
mod tests;
