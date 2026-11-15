//! Things the user asked for during one frame.
//!
//! Widgets do not change the project while they are being drawn (they only
//! read it). They push an [`Action`]; the application applies all actions
//! after the frame's drawing is done. This keeps borrowing simple and gives
//! every change one place where it becomes an undo step.

use crate::dialogs::analysis::AnalysisKind;
use crate::dialogs::fit::FitKind;
use crate::dialogs::fit_more::FitMoreKind;
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
    /// Store the same text in every selected cell (Ctrl+Enter in the cell editor).
    Fill(String),
    /// Column > Mask > Apply, Remove or Swap on the selected cells.
    Mask(sheaf_core::sheetops::MaskOp),
    /// Edit > Insert: blank rows above the selected rows (only in the selected columns for a
    /// block of cells), or new columns left of the selected columns.
    Insert,
    /// Edit > Delete: the selected rows (only in the selected columns for a block of cells) or
    /// the selected columns.
    Delete,
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
    /// Exchange X-Y Axes: the settings of the two axes and the X and Y columns of every plot trade places.
    /// A layer with a plot against the row number (no X column) is left as it is.
    ExchangeAxes { layer: usize },
    /// Axis Zoom In and Axis Zoom Out: the axis range shrinks or grows about its middle.
    AxisZoom { layer: usize, y: bool, zoom_in: bool },
    /// Delete on the legend: it is hidden.
    HideLegend { layer: usize },
    /// Reconstruct Legend: the legend text is made again from the plots (the automatic text).
    ReconstructLegend { layer: usize },
    /// Update Legend: custom legend text is kept; lines of plots that are gone are dropped and
    /// lines for plots that the text does not mention are added. Automatic text needs no update.
    UpdateLegend { layer: usize },
    /// Delete on an axis title: it is hidden.
    HideTitle { layer: usize, y: bool },
    /// Delete on a free text label.
    DeleteLabel { layer: usize, label: usize },
    /// Delete on a drawn line, arrow, rectangle or ellipse.
    DeleteShape { layer: usize, shape: usize },
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
    /// A drawing tool (Line, Arrow, Rectangle, Circle): a drag on the graph draws one object.
    Draw(sheaf_core::shape::ShapeKind),
}

/// Which File > Import item opened the file chooser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportPick {
    /// One file (Ctrl+K).
    Single,
    /// Several files.
    Multiple,
    /// Several files, with the comma preset as the delimiter.
    Csv,
    /// Excel workbooks (File > Import > Excel).
    Excel,
}

#[derive(Debug, Clone)]
pub enum Action {
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    /// Ask for files and open the import dialog; with a path, skip the file chooser.
    Import(Option<PathBuf>),
    /// File > Import > Single ASCII, Multiple ASCII or Comma Delimited: the file chooser for that item.
    ImportPick(ImportPick),
    /// File > Import > Import Wizard (Ctrl+3).
    ImportWizard,
    /// Edit > Go to... (Ctrl+G) in a workbook.
    GoToCell(WindowId),
    /// Statistics > Power and Sample Size > a test.
    Power(sheaf_core::analysis::PowerKind),
    /// Edit > Find in Sheets... in this workbook.
    FindInSheets(WindowId),
    /// Edit > Replace... in this workbook.
    ReplaceInSheets(WindowId),
    Undo,
    Redo,
    NewWorkbook,
    AddColumn(WindowId),
    DeleteColumns(Vec<ColumnId>),
    /// Column > Move Columns: these columns go to the first or last position or one step left or right.
    MoveColumns {
        columns: Vec<ColumnId>,
        to: sheaf_core::sheetops::MoveTo,
    },
    /// Column > Hide/Unhide Columns > Hide Selected.
    HideColumns(Vec<ColumnId>),
    /// Column > Hide/Unhide Columns > Unhide All, on the active sheet of the workbook.
    UnhideColumns(WindowId),
    /// Column > Set as Categorical: marks the columns as categorical, or, when they all are
    /// already, takes the mark off.
    SetCategorical(Vec<ColumnId>),
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
    /// The Interpolate window with this method chosen (Mathematics > Linear, Spline or B-Spline
    /// Interpolation).
    /// The Interpolate window taking its X values from a column (Interpolate/Extrapolate Y from X).
    InterpolateFromColumn {
        book: WindowId,
        columns: Vec<ColumnId>,
    },
    InterpolateWith {
        book: WindowId,
        columns: Vec<ColumnId>,
        method: sheaf_core::analysis::InterpMethod,
    },
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
    /// Open Multiple Linear Regression, Compare Models or Rank Models for a column.
    FitMore {
        book: WindowId,
        column: Option<ColumnId>,
        kind: FitMoreKind,
    },
    /// Sort the rows of the sheet that holds this column by the column's values.
    SortSheet {
        column: ColumnId,
        descending: bool,
    },
    /// Worksheet > Sort Range: sort the rows of the selected columns by the first of them; the
    /// other columns of the sheet keep their rows.
    SortRange {
        columns: Vec<ColumnId>,
        descending: bool,
    },
    /// Worksheet > Sort Columns: sort each selected column on its own.
    SortColumns {
        columns: Vec<ColumnId>,
        descending: bool,
    },
    /// Worksheet > Sort Worksheet > Custom... (no columns) or Sort Range > Custom...: open the
    /// nested sort window.
    SortCustom {
        book: WindowId,
        columns: Vec<ColumnId>,
    },
    /// Worksheet > Sort Columns by Label...
    SortByLabel(WindowId),
    /// Worksheet > Transpose, Stack Columns, Unstack Columns, Split Columns, Reduce Rows: open
    /// the window; the result goes to a new sheet.
    Reshape {
        book: WindowId,
        columns: Vec<ColumnId>,
        kind: crate::dialogs::reshape::ReshapeKind,
    },
    /// Column > Reverse Order: turn the values of each selected column round.
    ReverseColumns(Vec<ColumnId>),
    /// Empty every column of the active sheet of this workbook (one undo step).
    ClearWorksheet(WindowId),
    /// Column > Filter > Add or Remove Filter: the window for the sheet filter, with presets for
    /// this column.
    RowFilterDialog {
        book: WindowId,
        column: Option<ColumnId>,
    },
    /// Column > Filter > Enable/Disable Filter.
    ToggleRowFilter(WindowId),
    /// Column > Mask > Change Color: the next colour of the project's mask colour.
    MaskColor,
    /// Column > Mask > Disable Masking / Enable Masking, for the whole project.
    ToggleMasking,
    /// Column > Mask Cells by Condition...: the window for these columns.
    MaskByConditionDialog {
        book: WindowId,
        columns: Vec<ColumnId>,
    },
    /// View > Freeze/Unfreeze Panes (a toggle): freezes the rows above and the columns left of the
    /// selection, or unfreezes what is frozen.
    FreezePanes(WindowId),
    /// Worksheet > Conditional Formatting > Highlight Min Max / Highlight Top Bottom: the window
    /// for a rule on the selected cells.
    CondFormatDialog {
        book: WindowId,
        kind: crate::dialogs::cond_format::CondKind,
    },
    /// Worksheet > Conditional Formatting > Remove Conditional Format: removes the rules that
    /// touch the selection.
    RemoveCondFormat(WindowId),
    /// Column > Filter > Reapply Filter.
    ReapplyRowFilter(WindowId),
    /// Open the window that keeps only the rows matching a condition.
    FilterRows(WindowId),
    PlotColumns {
        book: WindowId,
        columns: Vec<ColumnId>,
        template: Template,
    },
    /// Plot > Symbol > Y Error: a scatter plot of the Y columns that shows the error bars of
    /// their Y error columns; with `x` (XY Error) also those of their X error columns.
    PlotYError {
        book: WindowId,
        columns: Vec<ColumnId>,
        x: bool,
    },
    /// Plot > Multi-Panel: one panel per selected Y column on a rows x columns grid.
    PlotPanels {
        book: WindowId,
        columns: Vec<ColumnId>,
        rows: usize,
        cols: usize,
    },
    /// Plot > Multi-Panel > Stack: choose grouping and layout before making the graph.
    StackPanels {
        book: WindowId,
        columns: Vec<ColumnId>,
    },
    LabelPanels {
        book: WindowId,
        columns: Vec<ColumnId>,
    },
    /// Graph > Add Error Bars: choose an existing Y-error column for one plot of the layer.
    AddErrorBars {
        graph: WindowId,
        layer: usize,
    },
    /// Window > Refresh (F5): recalculates the columns of the active workbook that are waiting
    /// for it; a graph is redrawn every frame anyway.
    Refresh(Option<WindowId>),
    /// Opens Plot Setup (Origin's window for the data of the plots) for one layer.
    PlotSetup {
        graph: WindowId,
        layer: usize,
    },
    /// Opens Layer Contents (Origin's add, remove, group and order window) for one layer.
    LayerContents {
        graph: WindowId,
        layer: usize,
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
    /// Adds a drawn object to a layer: one undo step.
    AddShape {
        graph: WindowId,
        layer: usize,
        shape: sheaf_core::shape::Shape,
    },
    /// Moves a drawn object by `(dx, dy)` fractions of the layer frame: one undo step.
    MoveShape {
        graph: WindowId,
        layer: usize,
        shape: usize,
        dx: f64,
        dy: f64,
    },
    /// Opens the properties window of a drawn object (double click, Properties).
    EditShape {
        graph: WindowId,
        layer: usize,
        shape: usize,
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
    /// File > Export > Worksheet as Excel...: the sheets of this book as an `.xlsx` file.
    ExportExcel(WindowId),
    /// File > Print...: the graph page through the system print dialog.
    Print(WindowId),
    PrintPreview(WindowId),
    PageSetup,
    /// Bring a window's tab to the front, reopening it if it was closed.
    Focus(WindowId),
    /// Close a window: it leaves the workspace, its data stays (Hide in the project explorer).
    HideWindow(WindowId),
    /// Window > Duplicate: copy this workbook or graph into a new window.
    DuplicateWindow(WindowId),
    /// File > Append: bring the windows of another project file into this one (the file, or `None`
    /// to ask for it).
    AppendProject(Option<std::path::PathBuf>),
    /// Remove a window from the project.
    DeleteWindow(WindowId),
    /// Put a window into a folder of the project explorer (the path of indices; empty is the root).
    MoveWindow {
        window: WindowId,
        folder: Vec<usize>,
    },
    /// A new empty folder inside the folder at this path.
    NewFolder(Vec<usize>),
    /// Rename the folder at this path (the folder menu and F2 on a folder).
    RenameFolder(Vec<usize>),
    /// Delete the folder at this path with the windows in it (undoable as one step).
    DeleteFolder(Vec<usize>),
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
    /// Open the window of Tools > Autosave.
    Autosave,
    /// View > Status Bar: show or hide the status bar.
    ToggleStatusBar,
    Quit,
}
