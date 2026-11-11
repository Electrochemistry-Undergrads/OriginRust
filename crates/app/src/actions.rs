//! Things the user asked for during one frame.
//!
//! Widgets do not change the project while they are being drawn (they only
//! read it). They push an [`Action`]; the application applies all actions
//! after the frame's drawing is done. This keeps borrowing simple and gives
//! every change one place where it becomes an undo step.

use crate::dialogs::graph_props::PropsTab;
use sheaf_core::template::Template;
use sheaf_core::{ColumnId, Designation, WindowId};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Action {
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    /// Ask for files and open the import dialog; with a path, skip the file chooser.
    Import(Option<PathBuf>),
    Undo,
    Redo,
    NewWorkbook,
    AddColumn(WindowId),
    DeleteColumns(Vec<ColumnId>),
    SetDesignation { columns: Vec<ColumnId>, designation: Designation },
    SetCell { column: ColumnId, row: usize, text: String },
    SelectSheet { book: WindowId, sheet: usize },
    ColumnProperties(ColumnId),
    SetColumnValues(ColumnId),
    PlotColumns { book: WindowId, columns: Vec<ColumnId>, template: Template },
    GraphProperties { graph: WindowId, tab: PropsTab },
    ExportGraph(WindowId),
    ExportData(WindowId),
    /// Bring a window's tab to the front, reopening it if it was closed.
    Focus(WindowId),
    /// Remove a window from the project.
    DeleteWindow(WindowId),
    RenameWindow(WindowId),
    About,
    Quit,
}
