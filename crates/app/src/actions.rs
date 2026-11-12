//! Things the user asked for during one frame.
//!
//! Widgets do not change the project while they are being drawn (they only
//! read it). They push an [`Action`]; the application applies all actions
//! after the frame's drawing is done. This keeps borrowing simple and gives
//! every change one place where it becomes an undo step.

use crate::dialogs::analysis::AnalysisKind;
use crate::dialogs::fit::FitKind;
use crate::dialogs::graph_props::PropsTab;
use sheaf_core::template::Template;
use sheaf_core::{ColumnId, Designation, WindowId};
use std::path::PathBuf;

/// How Copy writes the cells (the Copy submenu of the Edit menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyKind {
    /// What the cells show.
    Plain,
    /// Every digit of the numbers.
    FullPrecision,
    /// The label rows above the values, which are written as shown.
    WithLabels,
}

/// What to do with the selected cells of a worksheet.
#[derive(Debug, Clone, PartialEq)]
pub enum CellOp {
    Copy(CopyKind),
    Cut,
    /// Paste what is on the clipboard: asks the system for the text, which comes back as `PasteText`.
    Paste,
    PasteText(String),
    /// Empty the cells (the Delete key).
    Clear,
}

/// One of the label rows above the data of a column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelRow {
    LongName,
    Units,
    Comments,
    /// The F(x)= row: editing it opens Set Column Values.
    Formula,
    /// A user parameter row, by name.
    Param(String),
}

/// What to do with a sheet tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetOp {
    /// A new empty sheet after the last one.
    Add,
    /// A new empty sheet before the active one.
    Insert,
    Delete,
    Duplicate,
    /// Open the window that asks for a new name.
    RenameDialog,
}

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
    SetDesignation {
        columns: Vec<ColumnId>,
        designation: Designation,
    },
    SetCell {
        column: ColumnId,
        row: usize,
        text: String,
    },
    SelectSheet {
        book: WindowId,
        sheet: usize,
    },
    /// Copy, cut, paste or clear the selected cells of a workbook.
    Cells {
        book: WindowId,
        op: CellOp,
    },
    /// Change one label row of a column (a double click on the label cell).
    SetLabel {
        column: ColumnId,
        row: LabelRow,
        text: String,
    },
    /// Add, insert, delete, duplicate or rename the active sheet of a workbook.
    Sheet {
        book: WindowId,
        op: SheetOp,
    },
    ColumnProperties(ColumnId),
    SetColumnValues(ColumnId),
    /// Recalculate these columns now. Columns without a recalculating formula are skipped.
    Recalculate(Vec<ColumnId>),
    /// Open the window of an analysis tool for these columns of a workbook; without columns,
    /// for the columns a plot of the active sheet would show.
    Analyze {
        book: WindowId,
        columns: Vec<ColumnId>,
        kind: AnalysisKind,
    },
    /// Open the Fit window for a column of a workbook (or, without one, the first Y column of
    /// the active sheet), starting with a model of this kind.
    Fit {
        book: WindowId,
        column: Option<ColumnId>,
        kind: FitKind,
    },
    /// Sort the rows of the sheet that holds this column by the column's values.
    SortSheet {
        column: ColumnId,
        descending: bool,
    },
    /// Open the window that keeps only the rows matching a condition.
    FilterRows(WindowId),
    PlotColumns {
        book: WindowId,
        columns: Vec<ColumnId>,
        template: Template,
    },
    GraphProperties {
        graph: WindowId,
        tab: PropsTab,
    },
    ExportGraph(WindowId),
    ExportData(WindowId),
    /// Bring a window's tab to the front, reopening it if it was closed.
    Focus(WindowId),
    /// Remove a window from the project.
    DeleteWindow(WindowId),
    RenameWindow(WindowId),
    /// Window > Cascade, Tile Horizontally, Tile Vertically.
    Cascade,
    TileHorizontally,
    TileVertically,
    About,
    /// Open the window of View > Toolbars.
    Toolbars,
    Quit,
}
