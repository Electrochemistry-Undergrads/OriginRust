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
    /// Copy (formula), Copy (label rows), Copy (formula and label rows), Copy All (T88): whole
    /// columns only, as Origin's.
    Part(sheaf_core::clipboard::CopyPart),
}

/// What to do with the selected cells of a worksheet.
#[derive(Debug, Clone, PartialEq)]
pub enum CellOp {
    Copy(CopyKind),
    Cut,
    /// Paste what is on the clipboard: asks the system for the text, which comes back as `PasteText`.
    Paste,
    /// Edit > Paste Transpose: as `Paste`, and the text that comes back is pasted transposed.
    PasteTranspose,
    PasteText(String),
    /// Edit > Set As Begin / Set As End: the top (or bottom) selected row becomes the first (or
    /// last) row of each selected column's data range.
    SetRangeEnd(sheaf_core::data_range::RangeEnd),
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
    /// The read-only summary of a column's own X values.
    SamplingInterval,
    /// The Sparklines row (T175): a small line of the column's values, read-only.
    Sparklines,
    /// The Categories row (T187): the categories of a categorical column, read-only.
    Categories,
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
    /// Duplicate Without Data: the columns and labels of the sheet in a new last sheet (T90).
    DuplicateWithoutData,
    /// Sheet Name to Show: what the sheet's tab shows (T90).
    ShowName(sheaf_core::sheet_tab::SheetLabel),
    /// The corner menu's View switches: show or hide a label row, the row labels or grid lines (T92).
    ToggleView(sheaf_core::sheet_view::ViewSwitch),
    /// Navigate...: the window that lists the sheets and goes to one (T93).
    Navigate,
    /// Open the window that asks for a new name.
    RenameDialog,
    /// Add Notes as Sheet: a Notes sheet after the last sheet (T188).
    AddNotes,
}

/// An edit of one object of a graph, from the graph window (Origin's right-click menus).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphOp {
    /// Change Plot to: line, scatter, line + symbol, column.
    SetPlotKind { layer: usize, plot: usize, kind: PlotKind },
    /// Move to Right Y Axis / Move to Left Y Axis: the plot goes to the right Y layer of its
    /// panel (made when there is none) or back to the panel's own layer.
    MovePlotToAxis { layer: usize, plot: usize, right: bool },
    /// Move to Layer N: the plot goes to another layer of the graph, with that layer's axes (T61).
    MovePlotToLayer { layer: usize, plot: usize, to: usize },
    /// Hide Data Plot.
    HidePlot { layer: usize, plot: usize },
    /// Change X / Change Y: the plot reads another column of its sheet (`y` false for X). The axis
    /// limits are not touched.
    SetPlotColumn { layer: usize, plot: usize, y: bool, column: sheaf_core::ColumnId },
    /// Ungroup: the plots of the plot's group become single plots, keeping their look.
    Ungroup { layer: usize, plot: usize },
    /// Remove Trendline: the plot's last trendline goes.
    RemoveTrendline { layer: usize, plot: usize },
    /// Hide Axis (`y` false for the X axis).
    HideAxis { layer: usize, y: bool },
    /// Hide Grids, or show the major grid lines again.
    ToggleGrids { layer: usize, y: bool },
    /// Rescale To Show All: automatic limits on both axes of the layer.
    Rescale { layer: usize },
    /// Exchange X-Y Axes: the settings of the two axes and the X and Y columns of every plot trade places.
    /// A layer with a plot against the row number (no X column) is left as it is.
    ExchangeAxes { layer: usize },
    /// Graph > Offset Grouped Data in Layer: the layer's stack mode. Choosing the mode the layer
    /// already has changes nothing and makes no undo step.
    SetStack { layer: usize, stack: sheaf_core::graph::Stack },
    /// Axis Zoom In and Axis Zoom Out: the axis range shrinks or grows about its middle.
    AxisZoom { layer: usize, y: bool, zoom_in: bool },
    /// Axis Center: the axis keeps its span and moves so that the value at `page` (the right-clicked
    /// page point, in points) becomes the middle of its range. The range becomes fixed.
    AxisCenter { layer: usize, y: bool, page: (f64, f64) },
    /// Delete on the legend: it is hidden.
    HideLegend { layer: usize },
    /// The legend's Bring to the front (`front`) / Push to the back, over or under the labels and drawn
    /// objects (T152).
    OrderLegend { layer: usize, front: bool },
    /// An axis title's Bring to the front (`front`) / Push to the back (T153).
    OrderTitle { layer: usize, y: bool, front: bool },
    /// The legend's Push to back of data (`behind`) / Bring to front of data (T152).
    LegendBehindData { layer: usize, behind: bool },
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
    /// Delete on a slope annotation.
    DeleteSlope { layer: usize, slope: usize },
    /// Delete on a straight line.
    DeleteStraightLine { layer: usize, line: usize },
    /// Add Color Scale / Add Bubble Scale (T80): a scale of the layer's mapped plot beside the frame.
    AddScale { layer: usize, kind: sheaf_core::layer_scale::LayerScaleKind },
    /// Delete on a colour scale or bubble scale.
    DeleteScale { layer: usize, scale: usize },
    /// Move a drawn object to the top or bottom of its own layer's shape stack.
    OrderShape { layer: usize, shape: usize, front: bool },
    /// A text label's Bring to the front / Push to the back (T137): the top or bottom of its layer's labels,
    /// which are drawn in order.
    OrderLabel { layer: usize, label: usize, front: bool },
    /// Hide Layer (every plot of the layer is hidden).
    HideLayer { layer: usize },
    /// Hide Other Layers.
    HideOtherLayers { layer: usize },
    /// The Data menu's data markers and display range (T118); `layer` is the active layer.
    DataMarkers { layer: usize, op: MarkerOp },
    /// Graph > New Layer(Axes) and the Graph toolbar: a layer of `kind` next to the active layer `layer`.
    AddLayer { layer: usize, kind: sheaf_core::layers::NewLayerKind },
    /// Delete Layer of the layer icon: the layer goes, its children become independent.
    DeleteLayer { layer: usize },
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
    /// The Slope tool: two clicks on data points of one plot mark the slope between them.
    Slope,
    /// One of the Data menu's tools (Mask, Move, Remove Bad Data Points).
    Data(DataTool),
    /// Zoom In (a mode): the next click on the page zooms in at that point.
    ZoomIn,
    /// The Pan tool: a drag on the page moves the view.
    Pan,
    /// The Tools-bar Scale In: a rectangle drag rescales the active layer's axes (T69).
    ScaleIn,
    /// The Tools-bar Scale Out: a click or drag steps back to the previous scale (T69).
    ScaleOut,
}

/// A change of a graph window's view (screen-only; the graph is not changed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewOp {
    /// Zoom Out: half the zoom, down to 25 percent.
    ZoomOut,
    /// The Standard toolbar's zoom combo, as a whole percentage of the fitted page.
    SetZoom(u32),
    /// Whole Page: the page fits the window.
    WholePage,
    /// Zoom All: the page and all objects fit the window (T69).
    ZoomAll,
}

/// The tools of a graph's Data menu that act on the points of a plot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTool {
    /// Data > Mask Data Points: a drag masks the points in a rectangle, a click on a masked point
    /// unmasks it.
    Mask,
    /// Data > Move Data Points: a drag moves a point, changing its X and Y cells.
    Move,
    /// Data > Remove Bad Data Points: a click sets the Y cell of a point missing.
    RemoveBad,
    /// Data > Mark Data Range: a click on a point moves the nearer data marker of its plot there (T118).
    MarkRange,
}

/// A change of the data markers or the display range of a graph (T118).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerOp {
    /// Mark Data Range: markers at the ends of plot `plot` of the layer.
    Mark { plot: usize },
    /// A marker moved: the plot's markers are now on rows `a` and `b`.
    Move { plot: usize, a: usize, b: usize },
    /// Clear Data Markers.
    Clear,
    /// Set Display Range.
    SetRange,
    /// Reset to Full Range.
    ResetRange,
    /// Data > Analysis Markers > Hide, Show Partial Ranges, Show All (T135), for every layer of the graph.
    Show(sheaf_core::data_markers::MarkerShow),
    /// Data > Analysis Markers > Tiny, Small, Medium, Large (T135).
    Size(sheaf_core::data_markers::MarkerSize),
}

/// A change of worksheet cells made from a graph's Data menu tools (one undo step each).
#[derive(Debug, Clone, PartialEq)]
pub enum DataEdit {
    /// Masks or unmasks the Y cells of points.
    Mask { cells: Vec<sheaf_core::DataCell>, masked: bool },
    /// Sets the Y cells of points missing.
    RemoveBad { cells: Vec<sheaf_core::DataCell> },
    /// Moves a point: its Y cell and, when the plot has an X column, its X cell.
    Move { y: (sheaf_core::DataCell, f64), x: Option<(sheaf_core::DataCell, f64)> },
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
    /// File > New > Graph and the Standard toolbar button: an empty graph.
    NewGraph,
    /// File > Save Project without Data...: a copy without values to a file of its own.
    SaveProjectWithoutData,
    /// File > Duplicate Project without Data: an unsaved copy without values replaces the project.
    DuplicateProjectWithoutData,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    /// File > Save Window As...: ask for a name and write this window (a workbook, or a graph with
    /// the workbooks it reads) to a window file.
    SaveWindowAs(WindowId),
    /// File > Save Template As...: ask for a name and save the graph's look, or the workbook's
    /// structure (T160), as a user template.
    SaveTemplateAs(WindowId),
    /// File > Save Workbook As Analysis Template...: ask for a name and save the workbook's structure
    /// with its recalculating analysis runs (T163).
    SaveAnalysisTemplateAs(WindowId),
    /// Plot > User Templates > Manage...: rename and delete the saved templates.
    ManageTemplates,
    /// File > Batch Processing...: the window that runs a workbook template on data files (T165).
    BatchProcessing,
    /// Batch Peak Analysis Using Theme...: the window that runs a Peak Analyzer theme on these curves
    /// (none: the curves a plot of the active sheet would show) (T169).
    BatchPeaks {
        book: WindowId,
        columns: Vec<ColumnId>,
    },
    /// File > New > Notes and the Standard toolbar's New Notes (T154).
    NewNotes,
    /// File > New > Matrix and the Standard toolbar's New Matrix (T179): a matrix book with one
    /// sheet of Origin's default size (32 x 32 missing values), one undo step.
    NewMatrix,
    /// A typed matrix cell (T179): `None` clears it (missing). One undo step.
    SetMatrixCell {
        id: WindowId,
        sheet: usize,
        row: usize,
        col: usize,
        value: Option<f64>,
    },
    /// A matrix book's sheet tab was clicked (view state, no undo step).
    SetMatrixSheet {
        id: WindowId,
        sheet: usize,
    },
    /// The text of a Notes window after editing: one undo step.
    SetNotesText {
        id: WindowId,
        text: String,
    },
    /// The text of a Notes sheet after editing (T188): one undo step.
    SetNotesSheetText {
        book: WindowId,
        sheet: usize,
        text: String,
    },
    /// Column > Fill Column With > a pattern: open its window for these columns.
    FillPattern {
        kind: crate::dialogs::fill_pattern::FillKind,
        columns: Vec<ColumnId>,
    },
    /// A drawn object's right-click Copy (and Cut, which also deletes it, one undo step; T125).
    CopyShape {
        graph: WindowId,
        layer: usize,
        shape: usize,
        cut: bool,
    },
    /// An axis title's right-click Copy as a text label (and Cut, which also deletes the title, one
    /// undo step; T148).
    CopyTitle {
        graph: WindowId,
        layer: usize,
        y: bool,
        cut: bool,
    },
    /// Format > Object Properties...: the selected object's properties, as a double-click (T150).
    ObjectProperties {
        graph: WindowId,
    },
    /// The graph Edit menu's Cut, Copy, Paste and Clear (and Ctrl+X, C, V) on the selected object;
    /// Paste goes to the last clicked point of the active layer (T149).
    GraphClipboard {
        graph: WindowId,
        op: crate::menus::GraphClip,
    },
    /// The legend's right-click Copy as a free text label with its text and font (and Cut, which
    /// also hides the legend, one undo step; T178).
    CopyLegend {
        graph: WindowId,
        layer: usize,
        cut: bool,
    },
    /// A text label's right-click Copy (and Cut, which also deletes it, one undo step; T147).
    CopyLabel {
        graph: WindowId,
        layer: usize,
        label: usize,
        cut: bool,
    },
    /// The page's right-click Paste: the copied drawn object or label at page point `page`, one
    /// undo step.
    PasteShape {
        graph: WindowId,
        layer: usize,
        page: (f64, f64),
    },
    /// Column > Fill Column With > Row Numbers / Uniform / Normal Random Numbers: fill these columns
    /// at once, one undo step.
    QuickFill {
        kind: sheaf_core::fill_pattern::QuickFill,
        columns: Vec<ColumnId>,
    },
    /// Column > Set Sampling Interval...: open its window for these columns.
    SetSamplingInterval {
        columns: Vec<ColumnId>,
    },
    /// Column > Show X Column...: open its window for these columns.
    ShowXColumn {
        columns: Vec<ColumnId>,
    },
    /// Plot > User Templates > a saved one: plot the columns in its look.
    PlotUserTemplate {
        book: WindowId,
        columns: Vec<ColumnId>,
        name: String,
    },
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
    /// Owner-only: undo consecutive completed AI calls, stopping at a manual history entry.
    UndoAiChanges,
    /// A change captured by the shared Format toolbar.
    FormatText(crate::text_format::Request),
    WorksheetColorDialog(Box<crate::dialogs::worksheet_color::WorksheetColorDialog>),
    FormatObject(crate::object_format::Request),
    NewWorkbook,
    AddColumn(WindowId),
    DeleteColumns(Vec<ColumnId>),
    /// Column > Move Columns: these columns go to the first or last position or one step left or right.
    MoveColumns {
        columns: Vec<ColumnId>,
        to: sheaf_core::sheetops::MoveTo,
    },
    /// The column header's Swap Columns...: the two columns exchange places.
    SwapColumns(ColumnId, ColumnId),
    /// Edit > Find in Project (F3): open the Find window.
    FindInProject,
    /// File > Export > Worksheet as Image...: save the active sheet as a picture.
    ExportWorksheetImage(WindowId),
    /// View > View Windows (T100).
    ViewWindows(sheaf_core::project::ViewWindows),
    /// An axis's Save Format as Theme...: ask for a name and save the axis's format as a graph theme.
    SaveAxisTheme {
        graph: WindowId,
        layer: usize,
        y: bool,
    },
    /// A legend's Save Format as Theme...: ask for a name and save the legend's styles as a graph theme (T167).
    SaveLegendTheme {
        graph: WindowId,
        layer: usize,
    },
    /// Tools > Theme Organizer...: list the graph themes and apply one to the active graph.
    ThemeOrganizer,
    /// Tools > Fitting Function Organizer...: browse the built-in fit functions.
    FitOrganizer,
    /// Tools > Import Filters Manager...: the import file types and the saved import settings.
    FilterManager,
    /// Plot > System Templates...: the window of the built-in templates, for these columns.
    SystemTemplates {
        book: WindowId,
        columns: Vec<ColumnId>,
    },
    /// File > New > From Template: an empty graph from the saved template of this name, or (None,
    /// More...) from a template file picked with the system dialog.
    NewFromTemplate(Option<String>),
    /// File > New > From Template: an empty workbook from the saved workbook template of this name (T160).
    NewBookFromTemplate(String),
    /// Window > Folders: make this folder the Project Explorer's active one.
    GoToFolder(Vec<usize>),
    /// The explorer folder's Properties...: its name, comments and information.
    FolderProperties(Vec<usize>),
    /// Graph > Add Function Graph...: open its window for the active layer (T119).
    AddFunctionGraph {
        graph: WindowId,
        layer: usize,
    },
    /// Window > Duplicate (Batch Plotting): open the window that picks sheets or columns.
    BatchPlot {
        graph: WindowId,
        columns: bool,
    },
    /// File > Import > Re-Import Directly / Re-Import...: read the book's source file again.
    Reimport {
        book: WindowId,
        choose: bool,
    },
    /// The column header's Copy Columns to...: open its window for these columns.
    CopyColumnsTo(Vec<ColumnId>),
    /// Edit > Reset to Full Range: every column of the workbook's active sheet uses all its rows.
    ResetFullRange(WindowId),
    /// Column > Hide/Unhide Columns > Hide Selected.
    HideColumns(Vec<ColumnId>),
    /// Column > Hide/Unhide Columns > Unhide All, on the active sheet of the workbook.
    UnhideColumns(WindowId),
    /// Column > Set as Categorical: marks the columns as categorical, or, when they all are
    /// already, takes the mark off.
    SetCategorical(Vec<ColumnId>),
    /// Column header > Add or Update Sparklines... (T175): gives the columns sparklines and shows
    /// the sheet's Sparklines row, as one undo step.
    AddSparklines(Vec<ColumnId>),
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
    /// The Set Values window for several columns of one sheet at once (Ctrl+Shift+Q).
    SetMultipleColumnsValues(Vec<ColumnId>),
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
    /// Analysis > Fitting > Simulate Curve: the window, with the active workbook if there is one.
    SimulateCurve {
        book: Option<WindowId>,
    },
    /// Analysis > Fitting > Simulate Surface: the window, with the active workbook if there is one.
    SimulateSurface {
        book: Option<WindowId>,
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
    /// Analysis > Peaks and Baseline > Multiple Peak Fit on the active data plot of `graph`, or on
    /// the Y column `y` of a workbook (a graph of it is made first, as Origin does).
    MultiPeakFit {
        graph: Option<WindowId>,
        y: ColumnId,
        layer: usize,
    },
    /// Worksheet > Pivot Table: open the window; the table goes to a new sheet.
    PivotTable {
        book: WindowId,
        columns: Vec<ColumnId>,
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
    /// Column > Mask > Hide/Show Masked Points for the whole project.
    ToggleHideMasked,
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
    /// Worksheet > Conditional Formatting > Conditional Format Manager: the list of the active
    /// sheet's rules.
    CondFormatManager(WindowId),
    /// Worksheet > Conditional Formatting > Remove Conditional Format: removes the rules that
    /// touch the selection.
    RemoveCondFormat(WindowId),
    /// Format Cells... (cell, column and row menus; Format > Cell...): the window for the
    /// formats of the selection.
    FormatCellsDialog(WindowId),
    /// Format > Worksheet (F4): worksheet-wide Data font defaults.
    WorksheetFontsDialog(WindowId),
    /// Cell menu > Add Note... / Edit Note...: the note editor for the single selected cell.
    NoteDialog(WindowId),
    /// Cell menu > Delete Note: removes the note of the single selected cell.
    DeleteNote(WindowId),
    /// Column > Filter > Reapply Filter.
    ReapplyRowFilter(WindowId),
    /// Open the window that keeps only the rows matching a condition.
    FilterRows(WindowId),
    PlotColumns {
        book: WindowId,
        columns: Vec<ColumnId>,
        template: Template,
    },
    /// Plot > Line + Symbol > Line Series: one plot of the rows of the selected Y columns, k-point segments.
    PlotLineSeries {
        book: WindowId,
        columns: Vec<ColumnId>,
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
    /// Graph > Merge Graph Windows...
    MergeGraphs,
    LabelPanels {
        book: WindowId,
        columns: Vec<ColumnId>,
    },
    /// Plot > Multi-Y > Multiple Y Axes... (T61) for the selected columns (none: the sheet's Y columns).
    MultiYAxes {
        book: WindowId,
        columns: Vec<ColumnId>,
    },
    /// Multiple Comparisons Plot... (T64) of a report sheet (`None`: the workbook's active sheet).
    ComparisonPlot {
        book: WindowId,
        sheet: Option<usize>,
    },
    /// A plot's menu: Add Trendline... or Edit Trendline... (the plot's last one).
    Trendline {
        graph: WindowId,
        layer: usize,
        plot: usize,
        edit: bool,
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
    /// Opens Layer Contents for one layer with this plot type chosen (Graph > Add Plot to Layer).
    AddPlotToLayer {
        graph: WindowId,
        layer: usize,
        kind: sheaf_core::graph::PlotKind,
    },
    /// Opens Layer Contents (Origin's add, remove, group and order window) for one layer.
    LayerContents {
        graph: WindowId,
        layer: usize,
    },
    /// Fit Page to Layers...: opens its window.
    FitPageToLayers {
        graph: WindowId,
    },
    /// The 3D Axis window of a 3D layer (T164) on its X, Y or Z data axis (`dim` 0, 1 or 2).
    Axis3d {
        graph: WindowId,
        layer: usize,
        dim: usize,
    },
    /// Format > Fit Page To Printer: the page takes the default printer's printable size at once.
    FitPageToPrinter {
        graph: WindowId,
    },
    /// Opens Layer Management (arrange, size/position and link the layers) for a graph, with a layer selected when given.
    LayerManagement {
        graph: WindowId,
        layer: Option<usize>,
    },
    /// Opens the graph settings on a tab, for one layer, with a plot chosen in the Plots tab.
    GraphProperties {
        graph: WindowId,
        layer: usize,
        tab: PropsTab,
        plot: Option<usize>,
    },
    /// A change of a graph made from its window (right-click menus, the Delete key, Ctrl+R).
    /// Graph > New Layer(Axes) > Open Dialog...: capture the input graph and active layer.
    AddLayerDialog {
        graph: WindowId,
        layer: usize,
    },
    GraphEdit {
        graph: WindowId,
        op: GraphOp,
    },
    /// Switches a graph's tool.
    GraphTool {
        graph: WindowId,
        tool: GraphTool,
    },
    /// View > Print View, Page View, Window View, Draft View: the graph window's mode (view state, no undo step).
    SetViewMode {
        graph: WindowId,
        mode: sheaf_core::project::ViewMode,
    },
    /// View > Show > Layer Icons.
    /// View > Show > Labels or Data (T177): sets the flag on every layer of the graph, one undo step.
    SetShowElements {
        graph: WindowId,
        labels: Option<bool>,
        data: Option<bool>,
    },
    SetLayerIcons {
        graph: WindowId,
        on: bool,
    },
    /// A switch of the layer icon menu, saved with the page (T183).
    SetPageSwitch {
        graph: WindowId,
        switch: PageSwitch,
        on: bool,
    },
    /// View > Full Screen: the graph's page over the whole screen until Esc.
    FullScreen(WindowId),
    /// Add Shortcut to Favorites (T130): the window gets a shortcut in the Project Explorer's Favorites.
    AddFavorite(WindowId),
    /// Tools > Protection > Audit Log (T131): open the project's audit log window.
    AuditLog,
    /// Tools > Fitting Function Builder... (F8, T132).
    FitBuilder,
    /// Tools > Options... (T134).
    Options,
    /// Graph > New Table (T136): the cell editor for a new table in the active layer.
    NewTable {
        graph: WindowId,
        layer: usize,
    },
    /// File > Open Sample Projects (T133): the list of samples.
    SampleProjects,
    /// Open a sample project by name (it replaces the project, after asking about unsaved changes).
    OpenSample(String),
    /// Remove from Favorites (T130).
    RemoveFavorite(WindowId),
    /// Column header > Slide Show of Dependent Graphs (T129): the graphs that plot any of the columns, one
    /// after another over the whole screen.
    SlideShow(Vec<sheaf_core::ColumnId>),
    /// Zoom Out or Whole Page of a graph window.
    GraphView {
        graph: WindowId,
        op: ViewOp,
    },
    /// Scale In finished: the new ends of the active layer's axes.
    ScaleIn {
        graph: WindowId,
        layer: usize,
        x: (f64, f64),
        y: (f64, f64),
    },
    /// Scale Out: back to the previous scale of the active layer.
    ScaleOut {
        graph: WindowId,
        layer: usize,
    },
    /// A change that a Data menu tool made to the cells of a plot's worksheet.
    DataEdit(DataEdit),
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
    /// Edits the existing axis title with text-selection formatting.
    EditTitle {
        graph: WindowId,
        layer: usize,
        vertical: bool,
    },
    /// Edits the existing legend content, retaining plot substitutions.
    EditLegendText {
        graph: WindowId,
        layer: usize,
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
    /// Object Edit Align / Make Same Width / Make Same Height on the graph's selected drawn objects
    /// and text labels (T159): one undo step.
    ArrangeObjects {
        graph: WindowId,
        op: crate::obj_align::Arrange,
    },
    /// Object Edit Group / Ungroup on the graph's selected drawn objects and text labels (T162):
    /// one undo step.
    GroupObjects {
        graph: WindowId,
        ungroup: bool,
    },
    /// Moves several drawn objects and text labels together (a drag of a selection or a group,
    /// T162): one undo step.
    MoveObjects {
        graph: WindowId,
        changes: Vec<crate::obj_align::Change>,
    },
    /// Moves a drawn object by `(dx, dy)` fractions of the layer frame: one undo step.
    MoveShape {
        graph: WindowId,
        layer: usize,
        shape: usize,
        dx: f64,
        dy: f64,
    },
    /// Adds a slope annotation to a layer: one undo step.
    AddSlope {
        graph: WindowId,
        layer: usize,
        slope: sheaf_core::graph::SlopeAnnotation,
    },
    /// Opens the properties window of a slope annotation (double click, Properties).
    EditSlope {
        graph: WindowId,
        layer: usize,
        slope: usize,
    },
    /// Graph > Add Straight Line...: opens the window for a new line in `layer`.
    AddStraightLine {
        graph: WindowId,
        layer: usize,
    },
    /// Opens the window of a straight line (double click, Properties).
    EditStraightLine {
        graph: WindowId,
        layer: usize,
        line: usize,
    },
    /// Writes a hint on the status bar.
    Hint(String),
    /// Resize a drawn object, if its original geometry still exists.
    ResizeShape {
        graph: WindowId,
        layer: usize,
        shape: usize,
        before: sheaf_core::shape::Shape,
        after: sheaf_core::shape::Shape,
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
    /// Copy Format of a whole graph (T72).
    CopyGraphFormat {
        graph: WindowId,
        part: sheaf_core::format_clip::GraphPart,
    },
    /// Edit > Copy Format: the format of the selected axis when an axis is selected (Origin does
    /// the same), else of the whole graph.
    EditCopyFormat {
        graph: WindowId,
        part: sheaf_core::format_clip::GraphPart,
    },
    /// Copy Format of one axis.
    CopyAxisFormat {
        graph: WindowId,
        layer: usize,
        y: bool,
        part: sheaf_core::format_clip::AxisPart,
    },
    /// Paste Format of a graph format onto the graph, every group it holds.
    PasteGraphFormat {
        graph: WindowId,
    },
    /// The page's Paste Format To: an axis format onto several axes of the graph (T174).
    PasteAxesFormat {
        graph: WindowId,
        targets: sheaf_core::format_clip::AxisTargets,
        part: sheaf_core::format_clip::AxisPart,
    },
    /// Paste Format To: an axis format onto one axis.
    PasteAxisFormat {
        graph: WindowId,
        layer: usize,
        y: bool,
        part: sheaf_core::format_clip::AxisPart,
    },
    CopyLegendStyles {
        graph: WindowId,
        layer: usize,
    },
    PasteLegendStyles {
        graph: WindowId,
        layer: usize,
    },
    /// A legend's Copy Format > Dimensions or All (T179).
    CopyLegendFormat {
        graph: WindowId,
        layer: usize,
        part: sheaf_core::format_clip::LegendPart,
    },
    /// A legend's Paste Format To > Dimensions or All (T179).
    PasteLegendFormat {
        graph: WindowId,
        layer: usize,
        part: sheaf_core::format_clip::LegendPart,
    },
    /// Edit > Paste Format...: the Apply Formats window.
    PasteFormatDialog {
        graph: WindowId,
        layer: usize,
    },
    /// Gadgets > Integrate on a graph.
    StartIntegrate {
        graph: WindowId,
    },
    /// Gadgets > Rise Time on a graph (T138).
    StartRiseTime {
        graph: WindowId,
    },
    /// Gadgets > Intersect on a graph (T139).
    StartIntersect {
        graph: WindowId,
    },
    /// Gadgets > Quick Peaks on a graph (T140).
    StartQuickPeaks {
        graph: WindowId,
    },
    /// Gadgets > Quick Sigmoidal Fit on a graph (T141).
    StartSigmoid {
        graph: WindowId,
    },
    /// Gadgets > FFT on a graph (T142).
    StartFft {
        graph: WindowId,
    },
    /// Gadgets > Statistics on a graph (T143).
    StartStatisticsGadget {
        graph: WindowId,
    },
    /// Gadgets > Quick Fit on a graph (T143).
    StartQuickFit {
        graph: WindowId,
    },
    /// Gadgets > Vertical Cursor on a graph (T144).
    StartVerticalCursor {
        graph: WindowId,
    },
    /// Gadgets > Differentiate on a graph (T145).
    StartDifferentiate {
        graph: WindowId,
    },
    /// Gadgets > Interpolate on a graph (T145).
    StartInterpolate {
        graph: WindowId,
    },
    /// Gadgets > Cluster on a graph (T146).
    StartCluster {
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
    /// File > Export without an active graph: the Export Graphs window on every graph of the project (T182).
    ExportProjectGraphs(crate::menus::ProjectGraphsExport),
    ExportData(WindowId),
    /// File > Export > Worksheet as Excel...: the sheets of this book as an `.xlsx` file.
    ExportExcel(WindowId),
    /// File > Print...: the graph page through the system print dialog.
    Print(WindowId),
    PrintPreview(WindowId),
    PageSetup,
    /// Bring a window's tab to the front, reopening it if it was closed.
    Focus(WindowId),
    /// Window > Window Switcher: the switcher stays open until a window is chosen.
    OpenSwitcher,
    /// Close a window: it leaves the workspace, its data stays (Hide in the project explorer).
    HideWindow(WindowId),
    /// Window > Duplicate: copy this workbook or graph into a new window.
    DuplicateWindow(WindowId),
    /// File > Append: bring the windows of another project file into this one (the file, or `None`
    /// to ask for it).
    AppendProject(Option<std::path::PathBuf>),
    /// The File menu was opened: read the recent files again.
    ReloadRecent,
    /// File > Recent Projects, Recent Books or Recent Graphs: open this entry the way Open does.
    OpenRecent(std::path::PathBuf, crate::recent::RecentKind),
    /// File > Import > Recent Imports: import this entry again with the default settings.
    ImportRecent(std::path::PathBuf),
    /// Clear Recent Projects / Imports / Books / Graphs.
    ClearRecent(crate::recent::RecentKind),
    /// A command ran from the Plot, Worksheet, Analysis or Statistics menu (or its Most Recently
    /// Used list): it goes to the top of that menu's list.
    UsedCommand {
        menu: crate::mru::MruMenu,
        cmd: crate::menus::Cmd,
    },
    /// File > Export > Recent Exports: open the export of that file again on its window.
    OpenRecentExport(sheaf_io::recent::ExportEntry),
    /// Clear Recent Exports.
    ClearRecentExports,
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
    /// Tools > AI Assistants (MCP): turn the endpoint of `sheaf-mcp --attach` on or off.
    ToggleAiLink,
    Quit,
}

/// The page switches of the layer icon menu (T183).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSwitch {
    ActivateByIconOnly,
    IconDoubleClickPlotSetup,
}
