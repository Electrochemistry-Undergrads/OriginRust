//! Things the user asked for during one frame.
//!
//! Widgets do not change the project while they are being drawn (they only
//! read it). They push an [`Action`]; the application applies all actions
//! after the frame's drawing is done. This keeps borrowing simple and gives
//! every change one place where it becomes an undo step.

use crate::dialogs::analysis::AnalysisKind;
use crate::dialogs::fit::FitKind;
use crate::dialogs::graph_props::PropsTab;
use crate::panels::PanelId;
use sheaf_core::graph::PlotKind;
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

/// An edit of one object of a graph, from the graph window (Origin's right-click menus).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphOp {
    /// Change Plot to: line, scatter, line + symbol, column.
    SetPlotKind { layer: usize, plot: usize, kind: PlotKind },
    /// Move to Right Y Axis / Move to Left Y Axis: the plot goes to the right Y layer of its
    /// panel (made when there is none) or back to the panel's own layer.
    MovePlotToAxis { layer: usize, plot: usize, right: bool },
    /// Hide Data Plot.
    HidePlot { layer: usize, plot: usize },
    /// Hide Axis (`y` false for the X axis).
    HideAxis { layer: usize, y: bool },
    /// Hide Grids, or show the major grid lines again.
    ToggleGrids { layer: usize, y: bool },
    /// Rescale To Show All: automatic limits on both axes of the layer.
    Rescale { layer: usize },
    /// Axis Zoom In and Axis Zoom Out: the axis range shrinks or grows about its middle.
    AxisZoom { layer: usize, y: bool, zoom_in: bool },
    /// Delete on the legend: it is hidden.
    HideLegend { layer: usize },
    /// Reconstruct Legend: the legend text is made again from the plots (the automatic text).
    ReconstructLegend { layer: usize },
    /// Delete on an axis title: it is hidden.
    HideTitle { layer: usize, y: bool },
    /// Delete on a free text label.
    DeleteLabel { layer: usize, label: usize },
    /// Hide Layer (every plot of the layer is hidden).
    HideLayer { layer: usize },
    /// Hide Other Layers.
    HideOtherLayers { layer: usize },
}

/// The tool of a graph window (the Tools toolbar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphTool {
    #[default]
    Pointer,
    /// The Data Reader.
    Reader,
    /// The Text tool: the next click on the page places a text label.
    Text,
}

#[derive(Debug, Clone)]
pub enum Action {
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    /// Ask for files and open the import dialog; with a path, skip the file chooser.
    Import(Option<PathBuf>),
    /// File > Import > Import Wizard (Ctrl+3).
    ImportWizard,
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
    /// Opens the graph settings on a tab, for one layer, with a plot chosen in the Plots tab.
    GraphProperties {
        graph: WindowId,
        layer: usize,
        tab: PropsTab,
        plot: Option<usize>,
    },
    /// A change of a graph made from its window (right-click menus, the Delete key, Ctrl+R).
    GraphEdit {
        graph: WindowId,
        op: GraphOp,
    },
    /// Switches a graph's tool.
    GraphTool {
        graph: WindowId,
        tool: GraphTool,
    },
    /// Add Text: opens the text window for a new label in a layer (the active one when `layer`
    /// is `None`) at a point given as fractions of the layer frame (the middle when `None`).
    AddText {
        graph: WindowId,
        layer: Option<usize>,
        at: Option<(f64, f64)>,
    },
    /// Opens the text window for an existing label (double click, Properties).
    EditText {
        graph: WindowId,
        layer: usize,
        label: usize,
    },
    /// Moves a label to a new anchor (fractions of the layer frame): one undo step.
    MoveLabel {
        graph: WindowId,
        layer: usize,
        label: usize,
        x: f64,
        y: f64,
    },
    /// Moves the legend to a new centre (fractions of the layer frame): it becomes manually
    /// placed; one undo step.
    MoveLegend {
        graph: WindowId,
        layer: usize,
        x: f64,
        y: f64,
    },
    /// Copy Page: the graph as a picture on the clipboard.
    CopyPage(WindowId),
    /// Gadgets > Integrate on a graph.
    StartIntegrate {
        graph: WindowId,
    },
    /// A command of a graph's open gadget.
    Gadget {
        graph: WindowId,
        op: crate::gadget::GadgetOp,
    },
    /// Makes a layer of a graph the active one (a click on its layer icon or inside its frame).
    ActivateLayer {
        graph: WindowId,
        layer: usize,
    },
    ExportGraph(WindowId),
    ExportData(WindowId),
    /// Bring a window's tab to the front, reopening it if it was closed.
    Focus(WindowId),
    /// Close a window: it leaves the workspace, its data stays (Hide in the project explorer).
    HideWindow(WindowId),
    /// Remove a window from the project.
    DeleteWindow(WindowId),
    /// Put a window into a folder of the project explorer (the path of indices; empty is the root).
    MoveWindow {
        window: WindowId,
        folder: Vec<usize>,
    },
    /// A new empty folder inside the folder at this path.
    NewFolder(Vec<usize>),
    /// Show or hide one plot of a graph (the tick boxes of the Object Manager).
    SetPlotVisible {
        graph: WindowId,
        layer: usize,
        plot: usize,
        visible: bool,
    },
    /// Show or hide all plots of a layer.
    SetLayerVisible {
        graph: WindowId,
        layer: usize,
        visible: bool,
    },
    /// Select a plot in the Object Manager (and in Plot Details when that is open).
    SelectPlot {
        graph: WindowId,
        layer: usize,
        plot: usize,
    },
    /// Show or hide a side panel (View > Project Explorer and the others).
    TogglePanel(PanelId),
    /// Empty the Messages Log or the Results Log.
    ClearLog(PanelId),
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
