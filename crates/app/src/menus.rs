//! The menu bars: one table, three bars (ADR 0007, point 2).
//!
//! Origin swaps its menu bar when another kind of window is activated. This module holds the
//! trees of menus for "no window", a workbook and a graph, with Origin's top-level names, their
//! order and Origin's names for the items (the study is in `docs/spec/ui-command-map.md`,
//! section A, and the pictures behind it). It is the single source: the bar that is drawn, the
//! keyboard shortcuts and the tests all read these tables.
//!
//! An item either has a [`Cmd`], a command Sheaf has, or none. Items without a command are
//! drawn greyed with the tooltip "Not available in Sheaf yet", so that a user who knows Origin
//! sees where the command lives. A command that exists but cannot run right now (no column
//! selected, wrong window type) is greyed too, with a tooltip that says what is missing.
//!
//! Alt letters: every label marks its letter with `&` (Origin's notation). The letters are drawn
//! underlined, and Alt plus the letter of a top-level menu opens that menu. Letters inside the
//! open menus are not handled (egui has no way to pick an item by letter).

use crate::actions::{Action, CellOp, CopyKind, DataTool, GraphOp, GraphTool, ImportPick, MarkerOp, SheetOp, ViewOp};
use crate::app::APP_NAME;
use crate::dialogs::analysis::AnalysisKind;
use crate::dialogs::fill_pattern::FillKind;
use crate::dialogs::fit::FitKind;
use crate::dialogs::fit_more::FitMoreKind;
use crate::dialogs::graph_props::PropsTab;
use crate::dialogs::reshape::ReshapeKind;
use crate::gadget::GadgetOp;
use crate::mru::{self, MruMenu};
use crate::panels::PanelId;
use crate::recent::{self, RecentKind};
use eframe::egui::{
    self, Color32, Key, KeyboardShortcut, Modifiers, Stroke, TextStyle,
    text::{LayoutJob, TextFormat},
};
use sheaf_core::clipboard::CopyPart;
use sheaf_core::data_range::RangeEnd;
use sheaf_core::fill_pattern::QuickFill;
use sheaf_core::format_clip::{AxisPart, GraphPart, LegendPart};
use sheaf_core::graph::{LockPosition, PlotKind, Stack};
use sheaf_core::sheetops::{MaskOp, MoveTo};
use sheaf_core::template::Template;
use sheaf_core::{ColumnId, Designation, WindowId};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Tooltip of an item that Sheaf does not have.
pub const NOT_YET: &str = "Not available in Sheaf yet";
const NEED_BOOK: &str = "Activate a workbook window first";
pub const NO_COLOR_SCALE: &str = "New Color Scale needs a plot whose symbols take their colour from a Z column";
pub const NO_CUMULATIVE: &str = "Cumulative needs a line, scatter or line + symbol plot in the layer";
const NEED_GRAPH: &str = "Activate a graph window first";
const NEED_3D: &str = "The active layer is not a 3D layer";
const NEED_GRAPH_OR_BOOK: &str = "Activate a graph or a workbook first";
const NEED_SHAPE: &str = "Select a drawn object first";
const CANNOT_GROUP: &str = "Select two or more drawn objects or text labels of one layer first (Shift+click adds one)";
const NOT_GROUPED: &str = "Select a grouped object first";
const NEED_TWO_OBJECTS: &str = "Select two or more drawn objects or text labels first (Shift+click adds one)";
const NEED_PRINTABLE: &str = "Activate a graph or workbook window first";
const NEED_COLUMNS: &str = "Select one or more columns first";
const NEED_CELLS: &str = "Select cells, columns or rows first";
const NEED_TWO_COLUMNS: &str = "Select two or more columns first";
const NEED_EXACTLY_TWO_COLUMNS: &str = "Select exactly two columns to swap";
const NEED_WHOLE_COLUMNS: &str = "Select whole columns first (click their headers)";
const NOT_IMPORTED: &str = "This workbook was not imported from a file";
const NEED_ONE_COLUMN: &str = "Select exactly one column first";
const NEED_WINDOW: &str = "Activate a window first";
const NO_TEMPLATE_FOLDER: &str = "There is no folder for user templates (the settings folder is missing)";
const NO_UNDO: &str = "Nothing to undo";
const NO_REDO: &str = "Nothing to redo";
const NO_AI_UNDO: &str = "The latest change must be an AI change, with no edit or dialog open";
const ANALYSIS_ON_BOOK: &str = "The active layer has no data plot to analyse: plot a worksheet column first";

/// Which menu bar is shown.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WindowKind {
    /// No window is open or active.
    None,
    Workbook,
    Graph,
    /// A matrix window (T192).
    Matrix,
    /// A layout page (T237).
    Layout,
}

/// The Image menu's commands on the active matrix (T207, matrices spec 7i).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageCmd {
    Brightness,
    Contrast,
    Gamma,
    Math,
    ToBinary,
    Rotate,
    Flip,
    Smooth,
    Sharpen,
    EdgeDetection,
    Median,
    Levels,
    Subtract,
    Crop,
    Resize,
    ColorToGray,
    RgbSplit,
}

/// The Matrix menu's commands (T192 slice 3, matrices spec 7b).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixCmd {
    SetValues,
    Dimensions,
    Transpose,
    RotateCcw,
    RotateCw,
    Rotate180,
    FlipHorizontal,
    FlipVertical,
    Expand,
    Shrink,
    ToWorksheet,
    /// Analysis > Mathematics > Inverse (T193).
    Inverse,
    /// Analysis > Signal Processing > FFT > 2D FFT and 2D IFFT (T196).
    Fft2,
    Ifft2,
    /// Analysis > Fitting > Nonlinear Surface Fit on the active matrix (T221).
    SurfaceFit,
}

/// What the graph Edit menu's clipboard items do (T149).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphClip {
    Cut,
    Copy,
    Paste,
    Clear,
}

/// Why the graph Edit menu's Cut, Copy or Clear cannot run.
pub const NEED_GRAPH_OBJECT: &str = "select a drawn object, a text label or an axis title first";
/// Why Format > Object Properties... cannot run.
pub const NEED_SELECTED_OBJECT: &str = "select an object of the graph first";
/// Why the graph Edit menu's Paste cannot run.
pub const NOTHING_COPIED: &str = "copy a drawn object, a text label or an axis title first";

/// What File > Export starts the Export Graphs window with when no graph is active (T182).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ProjectGraphsExport {
    /// Graphs...: every graph, the last format used (PNG at first).
    Graphs,
    /// Graph as Image...: every graph as PNG pictures.
    Image,
    /// Multi-Page PDF...: every graph into one PDF, one page each.
    MultiPagePdf,
}

/// A command of Sheaf that a menu item runs. It is turned into an [`Action`] with the state of
/// the program by [`resolve`].
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    NewProject,
    NewGraph,
    SaveProjectNoData,
    DuplicateProjectNoData,
    NewWorkbook,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    /// File > Save Window As...: the active workbook or graph to a window file of its own (T54).
    SaveWindowAs,
    /// File > Save Template As... on a graph (T56).
    SaveTemplateAs,
    /// File > Save Workbook As Analysis Template... on a workbook (T163).
    SaveAnalysisTemplateAs,
    /// Plot > User Templates > Manage... (T56).
    ManageTemplates,
    /// File > Batch Processing... (T165).
    BatchProcessing,
    /// A data cell's Insert Graph... (T194).
    InsertCellGraph {
        column: ColumnId,
        row: usize,
    },
    /// A data cell's Remove Graph (T194).
    RemoveCellGraph {
        column: ColumnId,
        row: usize,
    },
    /// Edit > Paste Link (false) / Paste Link Transpose (true) (T211).
    PasteLink(bool),
    /// Edit > Remove Links (T211).
    RemoveLinks,
    /// A data cell's Set Data Style > Rich Text: turn it on or off for the selected cells (T208).
    SetRichText(bool),
    /// A data cell's Set Data Style > Float: turn it on or off for the selected cells (T214).
    SetFloat(bool),
    /// A data cell's Insert Arrow > Arrow / Double Arrow, or Remove Arrow with `None` (T205).
    SetCellArrow {
        column: ColumnId,
        row: usize,
        double: Option<bool>,
    },
    /// A data cell's Insert Images from Files... (T202).
    InsertCellImage {
        column: ColumnId,
        row: usize,
    },
    /// A data cell's Remove Image (T202).
    RemoveCellImage {
        column: ColumnId,
        row: usize,
    },
    /// A data cell's Insert Sparklines... (T200).
    InsertCellSparkline {
        column: ColumnId,
        row: usize,
    },
    /// A data cell's Remove Sparkline (T200).
    RemoveCellSparkline {
        column: ColumnId,
        row: usize,
    },
    /// Analysis > Peaks and Baseline > Batch Peak Analysis Using Theme... (T169).
    BatchPeaks,
    /// Column > Fill Column With > A set of Numbers / Date/Time values / Text&Numeric values (T74).
    FillPattern(FillKind),
    /// Format > Object Properties... on a graph: the selected object's properties (T150).
    ObjectProperties,
    /// File > New > Notes (T154).
    NewNotes,
    /// File > New > Matrix and the Standard toolbar's New Matrix (T192).
    NewMatrix,
    /// File > New > Layout and the Standard toolbar's New Layout (T237).
    NewLayout,
    /// Layout > Add Graph... on the active layout page (T237).
    LayoutAddGraph,
    /// Layout > Add Worksheet... on the active layout page (T237 slice 5).
    LayoutAddSheet,
    /// Layout > Add Text... on the active layout page (T237 slice 8).
    LayoutAddText,
    /// Layout > Add Rectangle / Ellipse / Line / Arrow (T237 slices 11 and 12): the object and
    /// whether it has an arrowhead.
    LayoutAddShape(sheaf_core::layout_page::LayoutShape, bool),
    /// Layout > Export Page as Image... (T237 slice 3).
    LayoutExport,
    /// Format > Layout Page... [F2] (T237 slices 4 and 6).
    LayoutPageSetup,
    /// File > Import > Binary 2D Array... (T199).
    ImportBinary2d,
    /// File > Import > Image... into a matrix book (T204).
    ImportImage,
    /// File > New > Function Plot (T213): the function window; OK makes a new graph.
    NewFunctionPlot,
    /// Plot > Contour/Heat Map > Polar Contour theta(X) r(Y) / r(X) theta(Y) of three selected
    /// columns (T225).
    PolarContour {
        theta_first: bool,
    },
    /// Plot > Contour/Heat Map > Ternary Contour of four selected columns A, B, C, Z (T226).
    TernaryContour,
    /// Plot > Profile > Contour Profiles / Image Profiles (T234).
    Profile {
        contour: bool,
    },
    /// The Matrix menu of a matrix window (T192 slice 3).
    Matrix(MatrixCmd),
    /// The Image menu on the active matrix (T207).
    Image(ImageCmd),
    /// Worksheet > Convert to Matrix > Direct (the selected columns, or the sheet) and XYZ Gridding
    /// (three selected columns X, Y, Z on a regular grid), T192 slice 4.
    ConvertToMatrix {
        xyz: bool,
    },
    /// Plot > Contour/Heat Map > Color Fill, B/W Lines + Labels, Heat Map: of the active matrix, or of
    /// three selected worksheet columns X, Y and Z (T192).
    PlotField(sheaf_core::template::FieldTemplate),
    /// The graph Edit menu's Cut, Copy, Paste and Clear on the selected object (T149).
    GraphClipboard(GraphClip),
    /// A drawn object's right-click Cut and Copy, and the page's Paste at a page point (T125).
    CutShape {
        layer: usize,
        shape: usize,
    },
    CopyShape {
        layer: usize,
        shape: usize,
    },
    PasteShape {
        layer: usize,
        page: (f64, f64),
    },
    /// The page's Insert Images From Files...: a picture with its corner at the clicked point (T216).
    InsertGraphImage {
        layer: usize,
        page: (f64, f64),
    },
    /// An axis title's right-click Cut and Copy (T148).
    CopyTitle {
        layer: usize,
        y: bool,
        cut: bool,
    },
    /// The legend's right-click Cut and Copy (T178): a free text label with the legend's text.
    CopyLegend {
        layer: usize,
        cut: bool,
    },
    /// A text label's right-click Cut and Copy (T147).
    CutLabel {
        layer: usize,
        label: usize,
    },
    CopyLabel {
        layer: usize,
        label: usize,
    },
    /// Column > Fill Column With > Row Numbers / Uniform / Normal Random Numbers (T63 group H).
    QuickFill(QuickFill),
    /// Column > Set Sampling Interval... and Show X Column... (T84).
    SetSamplingInterval,
    ShowXColumn,
    /// File > Import > Single ASCII (Ctrl+K).
    Import,
    ImportMultiple,
    ImportCsv,
    ImportWizard,
    /// File > Import > Excel (XLS, XLSX, XLSM)...
    ImportExcel,
    /// Edit > Go to... (Ctrl+G, in a workbook; in a graph the same keys export).
    GoToCell,
    /// Statistics > Power and Sample Size > one of the tests.
    Power(sheaf_core::analysis::PowerKind),
    /// Edit > Find in Sheets... (Ctrl+F) in a workbook.
    FindInSheets,
    /// Edit > Find in Project (F3, T95).
    FindInProject,
    /// File > Find in Project Files... (T361).
    FindInProjectFiles,
    /// File > Export > Worksheet as Image... (T97).
    ExportWorksheetImage,
    /// File > Import > Re-Import Directly (false) and Re-Import... (true, choose the file) (T98).
    Reimport(bool),
    /// Window > Duplicate (Batch Plotting) > with New Sheets/Books (false) or New Columns (true) (T99).
    BatchPlot(bool),
    /// Graph > Add Function Graph... (T119).
    AddFunctionGraph,
    /// View > View Windows (T100).
    ViewWindows(sheaf_core::project::ViewWindows),
    /// An axis's Save Format as Theme... (T102).
    SaveAxisTheme {
        layer: usize,
        y: bool,
    },
    /// A legend's Save Format as Theme... (T167).
    SaveLegendTheme {
        layer: usize,
    },
    /// Tools > Theme Organizer... (F7, T102).
    ThemeOrganizer,
    /// File > New > From Template > More...: pick a template file (T103).
    NewFromTemplateFile,
    /// Plot > System Templates... (T104).
    SystemTemplates,
    /// Tools > Fitting Function Organizer... (F9, T105).
    FitOrganizer,
    /// Tools > Import Filters Manager... (T106).
    FilterManager,
    /// Edit > Replace... (Ctrl+H) in a workbook.
    ReplaceInSheets,
    ExportGraph,
    /// The graph page's Send Graphs to PowerPoint... (T232).
    SendGraphsToPowerPoint,
    /// File > Export > Graphs..., Graph as Image... or Multi-Page PDF... without an active graph (T182).
    ExportProjectGraphs(ProjectGraphsExport),
    ExportWorksheet,
    /// File > Export > Worksheet as Excel...
    ExportExcel,
    /// File > Print... (Ctrl+P): the active graph page.
    Print,
    /// File > Print Preview.
    PrintPreview,
    /// File > Page Setup...
    PageSetup,
    Exit,
    Undo,
    Redo,
    UndoAiChanges,
    DeleteColumns,
    /// Column > Move Columns.
    MoveColumns(MoveTo),
    /// The column header's Swap Columns... (T86): the two selected columns exchange places.
    SwapColumns,
    /// The column header's Copy Columns to... (T89).
    CopyColumnsTo,
    /// Edit > Paste Transpose and the data cell's Paste Transpose (T86).
    PasteTranspose,
    /// Edit > Set As Begin / Set As End and the data cell's items (T87).
    SetRangeEnd(sheaf_core::data_range::RangeEnd),
    /// Edit > Reset to Full Range on a workbook (T87).
    ResetFullRange,
    /// Column > Hide/Unhide Columns > Hide Selected.
    HideColumns,
    /// Column > Hide/Unhide Columns > Unhide All.
    UnhideColumns,
    /// Column > Mask > Apply, Remove, Swap.
    Mask(MaskOp),
    /// Column > Mask > Change Color.
    MaskColor,
    /// Column > Mask > Disable Masking (Enable Masking while it is disabled).
    ToggleMasking,
    /// Column > Mask > Hide/Show Masked Points.
    ToggleHideMasked,
    /// Column > Mask Cells by Condition...
    MaskByCondition,
    /// Column > Set as Categorical.
    SetCategorical,
    /// Column header > Add or Update Sparklines... (T175).
    AddSparklines,
    /// Worksheet > Transpose, Stack, Unstack, Split Columns, Reduce Rows.
    Reshape(ReshapeKind),
    /// Worksheet > Pivot Table.
    PivotTable,
    /// Analysis > Peaks and Baseline > Multiple Peak Fit.
    MultiPeakFit,
    /// Column > Filter > Add or Remove Filter, Enable/Disable Filter, Reapply Filter.
    RowFilterDialog,
    ToggleRowFilter,
    ReapplyRowFilter,
    /// View > Freeze/Unfreeze Panes.
    FreezePanes,
    /// Worksheet > Conditional Formatting > Highlight Min Max / Highlight Top Bottom.
    CondFormat(crate::dialogs::cond_format::CondKind),
    /// Worksheet > Conditional Formatting > Remove Conditional Format.
    RemoveCondFormat,
    /// Worksheet > Conditional Formatting > Conditional Format Manager.
    CondFormatManager,
    /// Format Cells... for the selected cells, rows, columns or label cell.
    FormatCells,
    /// Format > Worksheet (F4): worksheet-wide Data font defaults.
    WorksheetFonts,
    /// Cell menu > Add Note... / Edit Note...: the note editor for the selected cell.
    EditNote,
    /// Cell menu > Delete Note.
    DeleteNote,
    /// Insert rows or columns at the selection (Edit > Insert and the cell menus).
    InsertCells,
    /// Delete the selected rows or columns (Edit > Delete and the cell menus).
    DeleteCells,
    /// Copy the selected cells (Edit > Copy and the Copy item of the cell menus).
    Copy(CopyKind),
    Cut,
    Paste,
    /// Empty the selected cells (the Delete key).
    Clear,
    /// A command of a sheet tab's menu, on the active sheet.
    Sheet(SheetOp),
    Plot(Template),
    /// Plot > Statistics > Grouped Histogram... (T271).
    GroupedHistogram,
    /// Plot > Statistics > Run Chart... (T270).
    RunChart,
    /// Plot > Line + Symbol > Line Series (T82).
    PlotLineSeries,
    /// Plot > Symbol > Y Error.
    PlotYError,
    /// Plot > Symbol > XY Error: a scatter plot with the X and Y error columns of each Y column.
    PlotXYError,
    /// Plot > Multi-Panel > a fixed grid of panels (Vertical 2 Panel is 2 x 1, 4 Panel 2 x 2).
    PlotPanels {
        rows: usize,
        cols: usize,
    },
    StackPanels,
    LabelPanels,
    /// Graph > Merge Graph Windows... (T60).
    MergeGraphs,
    /// Statistics > Multiple Comparisons Plot... (T64).
    ComparisonPlot,
    MultiYAxes,
    /// Add / Edit Trendline... on a plot of the active layer.
    Trendline {
        layer: usize,
        plot: usize,
        edit: bool,
    },
    /// Graph > Add Error Bars... on the active layer.
    AddErrorBars,
    /// Window > Refresh (F5).
    Refresh,
    SetAs(Designation),
    SetColumnValues,
    /// Column > Set Multiple Columns Values... (Ctrl+Shift+Q): two or more selected columns.
    SetMultipleColumnsValues,
    Recalculate,
    /// Column > Add New Columns... (Ctrl+D): the window that asks how many (T305).
    AddColumns,
    /// Column > Add Column to All Sheets... (T359).
    AddColumnAcross,
    /// Analysis > Repeat Last Analysis (T360): the last tool run from its window, on the current selection.
    RepeatAnalysis,
    /// The Standard toolbar's Add New Columns: one column at once, as Origin's button (T305).
    AddOneColumn,
    ColumnProperties,
    SortAscending,
    SortDescending,
    /// Worksheet > Sort Range, on the selected columns.
    SortRange {
        descending: bool,
    },
    /// Worksheet > Sort Columns: each selected column on its own.
    SortColumns {
        descending: bool,
    },
    /// Worksheet > Sort Worksheet > Custom... and Sort Range > Custom...
    SortCustom {
        range: bool,
    },
    SortByLabel,
    /// Worksheet > Clear Worksheet: empty every column of the active sheet.
    ClearWorksheet,
    /// Column > Reverse Order.
    ReverseOrder,
    FilterRows,
    Analyze(AnalysisKind),
    /// Mathematics > Linear, Spline or B-Spline Interpolation: the Interpolate window with that
    /// method chosen.
    InterpolateWith(sheaf_core::analysis::InterpMethod),
    /// Mathematics > Interpolate/Extrapolate Y from X: Interpolate at the X values of a column.
    InterpolateFromColumn,
    Fit(FitKind),
    FitMore(FitMoreKind),
    /// Analysis > Fitting > Simulate Curve...
    SimulateCurve,
    /// Analysis > Fitting > Simulate Surface...
    SimulateSurface,
    /// The graph settings of the active layer on a tab.
    GraphProps(PropsTab),
    /// Graph > Layer Contents... (F12): Origin's own window for the plots of a layer; `None` is the
    /// active layer.
    LayerContents(Option<usize>),
    /// Graph > Add Plot to Layer > Line... and the others: Layer Contents with this plot type chosen.
    AddPlotToLayer(PlotKind),
    /// Graph > Layer Management... (T60); the layer to select, if any.
    LayerManagement(Option<usize>),
    /// Graph > Fit Page to Layers... (the window) and Format > Fit Page To Layers... (T60).
    FitPageToLayers,
    /// Format > Axes > Z Axis...: the 3D Axis window on Z (T164); greyed outside a 3D layer.
    ZAxis,
    /// Format > Fit Page To Printer: the page takes the printable size of the default printer (T70).
    FitPageToPrinter,
    /// Graph > Plot Setup...: Origin's own window for the data of a layer's plots.
    PlotSetup(Option<usize>),
    /// The graph settings of a given layer, with a plot chosen (the graph's right-click menus).
    GraphPropsAt {
        layer: usize,
        tab: PropsTab,
        plot: Option<usize>,
    },
    /// A change made from a graph's right-click menus.
    GraphEdit(GraphOp),
    /// Graph > New Layer(Axes) or a Graph toolbar button: a layer of this kind next to the active layer.
    AddLayer(sheaf_core::layers::NewLayerKind),
    AddLayerDialog,
    /// Rescale to Show All on the active layer (Ctrl+R).
    Rescale,
    /// Graph > Exchange X-Y Axes on the active layer.
    ExchangeAxes,
    /// Graph > Offset Grouped Data in Layer: the active layer's stack mode. A Constant step or an
    /// Auto gap already stored on the layer is kept; a layer that is not in that mode starts from
    /// the value Plot Details' Stack tab offers.
    SetStack(sheaf_core::graph::Stack),
    /// Graph > Legend > Reconstruct Legend on the active layer (Ctrl+L).
    ReconstructLegend,
    /// Graph > Legend > Update Legend on the active layer.
    UpdateLegend,
    /// Makes a plot the active one (the plot list of the layer icon's menu).
    SelectPlot {
        layer: usize,
        plot: usize,
    },
    /// Go to the workbook a plot's data is in.
    GoTo(WindowId),
    /// The Pointer tool of the Tools toolbar (switches the Data Reader off).
    Pointer,
    /// The Data Reader tool of the Tools toolbar.
    DataReader,
    /// A tool of the Data menu (Mask, Move or Remove Bad Data Points).
    DataTool(DataTool),
    /// Column header > Slide Show of Dependent Graphs (T129).
    SlideShow,
    /// Graph page > Add Shortcut to Favorites (T130).
    AddFavorite,
    /// Tools > Protection > Audit Log (T131).
    AuditLog,
    /// Tools > Fitting Function Builder... (T132).
    FitBuilder,
    /// Tools > Options... (T134).
    Options,
    /// Graph > New Table (T136).
    NewTable,
    /// File > Open Sample Projects, Help > Open Sample Projects > Sample Projects (T133).
    SampleProjects,
    /// The Data menu's display range and data markers (T118).
    DataMarkers(MarkerOp),
    /// The Text tool of the Tools toolbar.
    TextTool,
    /// The Slope tool of the Tools toolbar.
    SlopeTool,
    /// Graph > Add Straight Line...
    AddStraightLine,
    /// Graph > New Color Scale: a colour scale of the active layer.
    NewColorScale,
    /// Properties of a straight line.
    EditStraightLine {
        layer: usize,
        line: usize,
    },
    /// View > Zoom In (Ctrl+I): the mode in which the next click on the page zooms in.
    ZoomIn,
    /// View > Zoom Out (Ctrl+M).
    ZoomOut,
    /// View > Whole Page (Ctrl+W).
    WholePage,
    /// View > Zoom All.
    ZoomAll,
    /// The Tools-bar Scale In tool.
    ScaleIn,
    /// The Tools-bar Scale Out tool.
    ScaleOut,
    /// View > Print View, Page View, Window View, Draft View.
    SetViewMode(sheaf_core::project::ViewMode),
    /// View > Show > Layer Icons.
    ToggleLayerIcons,
    ToggleActiveLayerIndicator,
    /// Data > Lock Position (T186): where the graph's analysis locks are drawn.
    LockPosition(sheaf_core::graph::LockPosition),
    /// The layer icon menu's Set Active Layer By Layer Icon Only (T183).
    ToggleActivateByIconOnly,
    /// The layer icon menu's Double click to open Plot Setup (T183).
    ToggleIconPlotSetup,
    /// View > Show > Labels and Data on a graph (T177): every layer's labels or data plots.
    ToggleShowLabels,
    ToggleShowData,
    /// View > Show > Active Layer Only uses the existing Hide Other Layers toggle.
    ToggleActiveLayerOnly,
    /// View > Full Screen (Ctrl+Shift+J) and the page's "View Full Screen".
    FullScreen,
    /// The Pan tool of the Tools toolbar.
    PanTool,
    /// Object Edit toolbar Bring to Front / Send to Back: reorders the selected drawn shape.
    OrderSelectedShape {
        front: bool,
    },
    /// Object Edit toolbar Align Left ... Make Same Height on the selected drawn objects and text
    /// labels, the last-selected one the reference (T159).
    ArrangeSelected(crate::obj_align::Arrange),
    /// Object Edit toolbar Group / Ungroup on the selected drawn objects and text labels (T162).
    GroupSelected {
        ungroup: bool,
    },
    /// Properties of a slope annotation.
    EditSlope {
        layer: usize,
        slope: usize,
    },
    /// A drawing tool of the Tools toolbar.
    ShapeTool(sheaf_core::shape::ShapeKind),
    /// Properties of a drawn object.
    EditShape {
        layer: usize,
        shape: usize,
    },
    /// The data points inside a drawn rectangle or ellipse, copied to a new workbook (T355).
    ExtractRegion {
        layer: usize,
        shape: usize,
    },
    /// Add Text...: a new text label in the active layer.
    AddText,
    /// Properties of a text label (its text window).
    EditText {
        layer: usize,
        label: usize,
    },
    /// Selection-format editor for a layer's existing axis title.
    EditTitle {
        layer: Option<usize>,
        vertical: bool,
    },
    /// Selection-format editor for the existing legend content.
    EditLegendText {
        layer: Option<usize>,
    },
    /// Edit > Copy Page: the graph as a picture on the clipboard.
    CopyPage,
    /// Copy Format of the whole graph (Edit menu, page menu): what the clipboard takes (T72).
    CopyGraphFormat(sheaf_core::format_clip::GraphPart),
    /// Edit > Copy Format: the graph format, or the format of the selected axis (T72).
    EditCopyFormat(sheaf_core::format_clip::GraphPart),
    /// Copy Format of one axis.
    CopyAxisFormat {
        layer: usize,
        y: bool,
        part: sheaf_core::format_clip::AxisPart,
    },
    /// Paste Format of the page menu: every group the graph format holds, at once.
    PasteGraphFormat,
    /// Paste Format To of an axis: the parts the axis format holds.
    PasteAxisFormat {
        layer: usize,
        y: bool,
        part: sheaf_core::format_clip::AxisPart,
    },
    /// The page's Paste Format To of an axis format onto several axes (T174).
    PasteAxesFormat {
        targets: sheaf_core::format_clip::AxisTargets,
        part: sheaf_core::format_clip::AxisPart,
    },
    CopyLegendStyles {
        layer: usize,
    },
    PasteLegendStyles {
        layer: usize,
    },
    /// A legend's Copy Format > Dimensions or All (T179).
    CopyLegendFormat {
        layer: usize,
        part: sheaf_core::format_clip::LegendPart,
    },
    /// A legend's Paste Format To > Dimensions or All (T179).
    PasteLegendFormat {
        layer: usize,
        part: sheaf_core::format_clip::LegendPart,
    },
    /// Edit > Paste Format...: the Apply Formats window.
    PasteFormatDialog,
    /// Gadgets > Integrate: starts the Integrate gadget on the active plot.
    IntegrateGadget,
    /// Gadgets > Rise Time... (T138).
    RiseTimeGadget,
    /// Gadgets > Intersect... (T139).
    IntersectGadget,
    /// Gadgets > Quick Peaks... (T140).
    QuickPeaksGadget,
    /// Gadgets > Quick Sigmoidal Fit... (T141).
    SigmoidGadget,
    /// Gadgets > FFT... (T142).
    FftGadget,
    /// Gadgets > Statistics... (T143).
    StatisticsGadget,
    /// Gadgets > Quick Fit... (T143).
    QuickFitGadget,
    /// Gadgets > Vertical Cursor... (T144).
    VerticalCursorGadget,
    /// Gadgets > Differentiate... (T145).
    DifferentiateGadget,
    /// Gadgets > Interpolate... (T145).
    InterpolateGadget,
    /// Gadgets > Cluster... (T146).
    ClusterGadget,
    /// Gadgets > 2D Integrate... (T189).
    Integrate2dGadget,
    /// A command of the open gadget's right-click menu.
    Gadget(GadgetOp),
    /// View > Toolbars: the window that shows and hides the bars.
    Toolbars,
    /// Tools > Autosave...: the window for the autosave switch and interval.
    Autosave,
    /// View > Status Bar: show or hide the status bar.
    ToggleStatusBar,
    /// Tools > AI Assistants (MCP): a tick item for the endpoint of `sheaf-mcp --attach`.
    ToggleAiLink,
    /// File > Close: close (hide) the active window, as its title bar's close button does.
    CloseWindow,
    /// Window > Duplicate: a copy of the active workbook or graph.
    DuplicateWindow,
    /// Window > Window Switcher (Ctrl+Tab, handled by `switcher.rs`; the menu entry opens it
    /// and keeps it open until a card is chosen).
    WindowSwitcher,
    /// File > Append...: bring the windows of another project into this one.
    AppendProject,
    /// Clear Recent Projects / Imports / Books / Graphs.
    ClearRecent(RecentKind),
    /// Clear Recent Exports.
    ClearRecentExports,
    /// View > Project Explorer and the other panels.
    Panel(PanelId),
    Cascade,
    TileHorizontally,
    TileVertically,
    /// Window > Arrange Icons (T257).
    ArrangeIcons,
    WindowProperties,
    About,
}

#[derive(Clone)]
pub enum Entry {
    Separator,
    Item(Item),
    Submenu(Submenu),
    /// The numbered list of the windows of the project, with a tick on the active one.
    Windows,
    /// Window > Folders: the project's folders, the active one ticked (T100).
    Folders,
    /// File > New > From Template: the saved user templates, each making an empty graph (T103), then
    /// the saved workbook templates, each making an empty workbook (T160).
    NewFromTemplates,
    /// The entries of File > Recent Projects, Recent Books, Recent Graphs or File > Import > Recent
    /// Imports, from the context.
    Recent(RecentKind),
    /// The saved user graph templates of Plot > User Templates (T56), from the context.
    UserTemplates,
    /// The numbered Most Recently Used commands of the menu (T78), from the context.
    Mru(MruMenu),
    /// The entries of Recent Exports (T78): files written by exports, from the context.
    Exports,
}

#[derive(Clone)]
pub struct Item {
    /// The text with `&` before the Alt letter (`&&` is a literal ampersand).
    pub label: String,
    pub keys: Option<KeyboardShortcut>,
    /// `None`: Sheaf does not have the command.
    pub cmd: Option<Cmd>,
}

#[derive(Clone)]
pub struct Submenu {
    pub label: String,
    pub entries: Vec<Entry>,
}

#[derive(Clone)]
pub struct Menu {
    pub label: String,
    pub entries: Vec<Entry>,
}

impl Menu {
    /// The name without the `&`.
    pub fn name(&self) -> String {
        plain(&self.label)
    }
}

/// What the menus need to know about the program to enable items and name the windows.
#[derive(Clone)]
pub struct MenuContext<'a> {
    pub kind: WindowKind,
    pub book: Option<WindowId>,
    pub graph: Option<WindowId>,
    pub active: Option<WindowId>,
    pub selected: &'a [ColumnId],
    /// Data columns within the sorting selection, including whole-row selections.
    pub sorting_columns: Vec<ColumnId>,
    /// True when whole columns (or the whole sheet) of the active workbook are selected.
    pub whole_columns: bool,
    /// True when the active workbook was imported from a data file (Re-Import, T98).
    pub imported: bool,
    /// True when the active workbook has selected cells, columns or rows (Copy, Cut, Paste and
    /// Clear act on them).
    pub cells: bool,
    pub undo: Option<&'a str>,
    pub redo: Option<&'a str>,
    /// The latest history entry is an AI call and no editor/dialog is active.
    pub undo_ai: bool,
    /// All windows of the project, in order, with their titles.
    pub windows: Vec<(WindowId, String)>,
    /// Window > Folders: every folder (path, `/Project/Sub/`) and the active one (T100).
    pub folders: Vec<(Vec<usize>, String)>,
    pub folder: Vec<usize>,
    /// View > View Windows: the project's choice, ticked in the menu.
    pub view_windows: sheaf_core::project::ViewWindows,
    /// A top-level menu to hold open (`--show menu:Plot`, for pictures of the menus); `File>Recent
    /// Projects` holds that submenu of File open too.
    pub open: Option<&'a str>,
    /// The recent projects, imports, workbook files and graph files, newest first (stored text; no
    /// file is touched to draw them).
    pub recent_projects: &'a [PathBuf],
    pub recent_imports: &'a [PathBuf],
    pub recent_books: &'a [PathBuf],
    pub recent_graphs: &'a [PathBuf],
    /// The Most Recently Used command keys per menu (list name to keys) and the recent exports.
    pub recent_commands: &'a std::collections::BTreeMap<String, Vec<String>>,
    pub recent_exports: &'a [sheaf_io::recent::ExportEntry],
    /// The saved user graph templates (Plot > User Templates), and whether there is a template
    /// folder at all (File > Save Template As and Manage are greyed without one).
    pub user_templates: &'a [String],
    /// The saved user workbook templates (File > New > From Template, T160).
    pub user_book_templates: &'a [String],
    pub templates: bool,
    /// Which side panels are shown, in the order of `PanelId::ALL` (the ticks of the View menu).
    pub panels: [bool; 4],
    /// The AI link is on (the tick of Tools > AI Assistants (MCP)).
    pub ai_link: bool,
    /// The active layer of the active graph (0 when there is none).
    pub layer: usize,
    /// The active graph's tool (the Tools toolbar shows it pressed).
    pub tool: crate::actions::GraphTool,
    /// On a graph: the data the Analysis menu works on, as Origin's menu does on the active data
    /// plot: the workbook and Y column of the selected plot, or else of the active layer's first
    /// visible plot of numbers. `None` when the graph has no such plot.
    pub curve: Option<(WindowId, ColumnId)>,
    /// On a graph: the kind of the selected object (the Edit menu's Cut, Copy and Clear act on it).
    pub graph_pick: Option<sheaf_plot::scene::ObjectKind>,
    /// A drawn object or a text label is on the graph clipboard (the Edit menu's Paste, T149).
    pub graph_paste: bool,
    /// On a graph: the Y columns of the active layer's visible plots of numbers that sit in the same
    /// worksheet as `curve`, the active plot first, then the others in plot order. The tools that
    /// compare curves (Average Multiple Curves, Intersect) start from these.
    pub layer_curves: Vec<ColumnId>,
    /// The stack mode of the active layer of the active graph (the tick of Graph > Offset Grouped
    /// Data in Layer); `None` without a graph.
    pub stack: Option<sheaf_core::graph::Stack>,
    /// True when the active layer has a plot that Cumulative stacking applies to (line, scatter, line + symbol).
    pub can_cumulate: bool,
    /// The active layer has a plot whose symbols take their colour from a Z column (New Color Scale).
    pub can_color_scale: bool,
    /// The active layer of the active graph is a 3D layer (T155): Format > Axes > Z Axis... works on it.
    pub space3d: bool,
    /// The selected drawn shape of the active graph as (layer, shape index, shapes in that layer).
    pub selected_shape: Option<(usize, usize, usize)>,
    /// How many drawn objects and text labels are selected on the active graph (Shift+click, T159).
    pub multi_selected: usize,
    /// The selected objects can be grouped: two or more, in one layer, not already one group (T162).
    pub can_group: bool,
    /// A selected object is in a group (T162).
    pub can_ungroup: bool,
    /// The view mode of the active graph (the tick of View > Print View, Page View, Window View, Draft View).
    pub view_mode: sheaf_core::project::ViewMode,
    /// The active graph shows its layer icons (the tick of View > Show > Layer Icons).
    pub layer_icons: bool,
    /// Whether View > Show > Active Layer Indicator is checked for this graph.
    pub active_layer_indicator: bool,
    /// The active graph's Lock Position (the tick of Data > Lock Position, T186).
    pub lock_position: sheaf_core::graph::LockPosition,
    /// The active sheet shows Page Break Preview Lines (the tick of the View menu's item, T185).
    pub page_breaks: bool,
    /// The active graph's page switches of the layer icon menu (T183), for their ticks.
    pub activate_by_icon_only: bool,
    pub icon_double_click_plot_setup: bool,
    /// Every layer of the active graph shows its labels, and its data (the ticks of View > Show >
    /// Labels and Data, T177).
    pub show_labels: bool,
    pub show_data: bool,
    /// None without a valid multi-layer target; otherwise the derived isolation check.
    pub active_layer_only: Option<bool>,
}

impl<'a> MenuContext<'a> {
    /// The entries of one recent list.
    pub fn recent(&self, kind: RecentKind) -> &'a [PathBuf] {
        match kind {
            RecentKind::Projects => self.recent_projects,
            RecentKind::Imports => self.recent_imports,
            RecentKind::Books => self.recent_books,
            RecentKind::Graphs => self.recent_graphs,
        }
    }
}

// ----- the table ---------------------------------------------------------------------------

/// The menus of a bar, in order.
pub fn bar(kind: WindowKind) -> &'static [Menu] {
    static BARS: OnceLock<[Vec<Menu>; 5]> = OnceLock::new();
    let bars = BARS.get_or_init(|| {
        [
            build(WindowKind::None),
            build(WindowKind::Workbook),
            build(WindowKind::Graph),
            build(WindowKind::Matrix),
            build(WindowKind::Layout),
        ]
    });
    &bars[match kind {
        WindowKind::None => 0,
        WindowKind::Workbook => 1,
        WindowKind::Graph => 2,
        WindowKind::Matrix => 3,
        WindowKind::Layout => 4,
    }]
}

fn build(kind: WindowKind) -> Vec<Menu> {
    let mut menus = base(kind);
    place_unplaced(&mut menus);
    for m in &mut menus {
        assign_mnemonics(&mut m.entries);
    }
    menus
}

/// The table before the fallback placement of commands that a menu does not name.
fn base(kind: WindowKind) -> Vec<Menu> {
    use WindowKind::*;
    let m = |label: &str, entries: Vec<Entry>| Menu { label: label.to_string(), entries };
    match kind {
        None => vec![
            m("&File", file_menu(kind)),
            m("&View", view_menu(kind)),
            m("&Tools", tools_menu()),
            m("&Window", window_menu(kind)),
            m("&Help", help_menu()),
        ],
        Workbook => vec![
            m("&File", file_menu(kind)),
            m("&Edit", edit_menu(kind)),
            m("&View", view_menu(kind)),
            m("&Plot", plot_menu()),
            m("&Column", column_menu()),
            m("Wor&ksheet", worksheet_menu()),
            m("&Analysis", analysis_menu()),
            m("&Statistics", statistics_menu()),
            m("&Image", image_menu()),
            m("&Tools", tools_menu()),
            m("F&ormat", format_menu(kind)),
            m("&Window", window_menu(kind)),
            m("&Help", help_menu()),
        ],
        Layout => vec![
            m("&File", file_menu(kind)),
            m("&View", view_menu(kind)),
            m(
                "&Layout",
                vec![
                    item("Add &Graph...", Cmd::LayoutAddGraph),
                    item("Add &Worksheet...", Cmd::LayoutAddSheet),
                    item("Add &Text...", Cmd::LayoutAddText),
                    item("Add &Rectangle", Cmd::LayoutAddShape(sheaf_core::layout_page::LayoutShape::Rectangle, false)),
                    item("Add &Ellipse", Cmd::LayoutAddShape(sheaf_core::layout_page::LayoutShape::Ellipse, false)),
                    item("Add &Line", Cmd::LayoutAddShape(sheaf_core::layout_page::LayoutShape::HLine, false)),
                    item("Add &Arrow", Cmd::LayoutAddShape(sheaf_core::layout_page::LayoutShape::HLine, true)),
                    sep(),
                    item("&Export Page as Image...", Cmd::LayoutExport),
                ],
            ),
            m("F&ormat", vec![keyed("&Layout Page...", plain_key(Key::F2), Cmd::LayoutPageSetup)]),
            m("&Tools", tools_menu()),
            m("&Window", window_menu(kind)),
            m("&Help", help_menu()),
        ],
        Matrix => vec![
            m("&File", file_menu(kind)),
            m("&View", view_menu(kind)),
            m("&Plot", vec![sub("Contour/&Heat Map", field_plot_items()), profile_menu()]),
            m("&Matrix", matrix_menu()),
            m("&Image", image_menu()),
            m(
                "&Analysis",
                vec![
                    sub("&Mathematics", vec![item("&Inverse...", Cmd::Matrix(MatrixCmd::Inverse))]),
                    sub("Fi&tting", vec![item("&Nonlinear Surface Fit...", Cmd::Matrix(MatrixCmd::SurfaceFit))]),
                    sub(
                        "Si&gnal Processing",
                        vec![sub(
                            "&FFT",
                            vec![
                                item("2D FFT...", Cmd::Matrix(MatrixCmd::Fft2)),
                                item("2D IFFT...", Cmd::Matrix(MatrixCmd::Ifft2)),
                            ],
                        )],
                    ),
                ],
            ),
            m("&Tools", tools_menu()),
            m("&Window", window_menu(kind)),
            m("&Help", help_menu()),
        ],
        Graph => vec![
            m("&File", file_menu(kind)),
            m("&Edit", edit_menu(kind)),
            m("&View", view_menu(kind)),
            m("&Graph", graph_menu()),
            m("&Data", data_menu()),
            m("&Analysis", analysis_menu()),
            m("Gadget&s", gadgets_menu()),
            m("&Tools", tools_menu()),
            m("F&ormat", format_menu(kind)),
            m("&Window", window_menu(kind)),
            m("&Help", help_menu()),
        ],
    }
}

/// The heat map and contour items of Plot > Contour/Heat Map: from the active matrix window, or
/// from three selected worksheet columns X, Y and Z (T192).
fn field_plot_items() -> Vec<Entry> {
    use sheaf_core::template::FieldTemplate;
    vec![
        item("Color Fill", Cmd::PlotField(FieldTemplate::ContourFill)),
        item("B/W Lines + Labels", Cmd::PlotField(FieldTemplate::ContourLines)),
        item("Gray Scale Map", Cmd::PlotField(FieldTemplate::GrayScale)),
        item("Heat Map", Cmd::PlotField(FieldTemplate::HeatMap)),
    ]
}

/// Plot > Profile (T234): a row or column cut of the active matrix, or of the matrix that the active
/// graph's contour shows.
fn profile_menu() -> Entry {
    sub(
        "Pro&file",
        vec![
            item("Contour Profiles", Cmd::Profile { contour: true }),
            item("Image Profiles", Cmd::Profile { contour: false }),
        ],
    )
}

/// The Matrix menu of a matrix window (T192 slice 3).
fn matrix_menu() -> Vec<Entry> {
    use MatrixCmd::*;
    vec![
        item("Set Matrix &Values...", Cmd::Matrix(SetValues)),
        item("Set &Dimensions/Labels...", Cmd::Matrix(Dimensions)),
        sep(),
        item("&Transpose", Cmd::Matrix(Transpose)),
        sub(
            "&Rotate90",
            vec![
                item("90 CCW", Cmd::Matrix(RotateCcw)),
                item("90 CW", Cmd::Matrix(RotateCw)),
                item("180", Cmd::Matrix(Rotate180)),
            ],
        ),
        sub(
            "&Flip",
            vec![item("Horizontal", Cmd::Matrix(FlipHorizontal)), item("Vertical", Cmd::Matrix(FlipVertical))],
        ),
        item("&Expand...", Cmd::Matrix(Expand)),
        item("&Shrink...", Cmd::Matrix(Shrink)),
        sep(),
        item("&Convert to Worksheet", Cmd::Matrix(ToWorksheet)),
    ]
}

// Small builders, so that the table reads like the menu.

fn sep() -> Entry {
    Entry::Separator
}

fn item(label: &str, cmd: Cmd) -> Entry {
    Entry::Item(Item { label: label.to_string(), keys: None, cmd: Some(cmd) })
}

fn keyed(label: &str, keys: KeyboardShortcut, cmd: Cmd) -> Entry {
    Entry::Item(Item { label: label.to_string(), keys: Some(keys), cmd: Some(cmd) })
}

fn grey(label: &str) -> Entry {
    Entry::Item(Item { label: label.to_string(), keys: None, cmd: None })
}

fn grey_keyed(label: &str, keys: KeyboardShortcut) -> Entry {
    Entry::Item(Item { label: label.to_string(), keys: Some(keys), cmd: None })
}

fn sub(label: &str, entries: Vec<Entry>) -> Entry {
    Entry::Submenu(Submenu { label: label.to_string(), entries })
}

/// Greyed items; a "-" in the list is a separator.
fn greys(labels: &[&str]) -> Vec<Entry> {
    labels.iter().map(|l| if *l == "-" { sep() } else { grey(l) }).collect()
}

/// A submenu of greyed items.
fn grey_sub(label: &str, labels: &[&str]) -> Entry {
    sub(label, greys(labels))
}

fn ctrl(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND, key)
}

fn ctrl_shift(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, key)
}

fn ctrl_alt(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::ALT, key)
}

fn alt(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::ALT, key)
}

fn plain_key(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::NONE, key)
}

/// No Most Recently Used commands (for contexts that keep none).
pub static NO_COMMANDS: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();

/// The Most Recently Used item at the end of the Plot, Worksheet, Analysis and Statistics menus
/// (a submenu in Origin's Plot menu; Sheaf uses one in all four, Sheaf's choice).
fn mru_submenu(label: &str, menu: MruMenu) -> Entry {
    sub(label, vec![Entry::Mru(menu)])
}

/// Recent Exports: a submenu of the files exports wrote, a separator and Clear.
fn exports_submenu() -> Entry {
    sub("Recent Exports", vec![Entry::Exports, sep(), item("Clear Recent Exports", Cmd::ClearRecentExports)])
}

/// A submenu of one recent list: the entries, a separator and Clear.
fn recent_submenu(label: &str, kind: RecentKind) -> Entry {
    sub(label, vec![Entry::Recent(kind), sep(), item(&format!("Clear {label}"), Cmd::ClearRecent(kind))])
}

fn file_menu(kind: WindowKind) -> Vec<Entry> {
    let mut v = vec![
        sub(
            "&New",
            [
                vec![
                    keyed("&Project", ctrl(Key::N), Cmd::NewProject),
                    item("&Worksheet...", Cmd::NewWorkbook),
                    item("Matrix...", Cmd::NewMatrix),
                    item("&Graph", Cmd::NewGraph),
                ],
                greys(&["Image"]),
                vec![sub(
                    "From Template",
                    vec![Entry::NewFromTemplates, sep(), item("More...", Cmd::NewFromTemplateFile)],
                )],
                greys(&["Excel"]),
                vec![item("Layout", Cmd::NewLayout)],
                vec![item("Notes", Cmd::NewNotes)],
                vec![item("Function Plot", Cmd::NewFunctionPlot)],
            ]
            .concat(),
        ),
        keyed("&Open...", ctrl(Key::O), Cmd::OpenProject),
        grey_keyed("Open &Excel...", ctrl(Key::E)),
        item("Open Sample Projects", Cmd::SampleProjects),
        item("Appen&d...", Cmd::AppendProject),
        item("&Close", Cmd::CloseWindow),
        sep(),
        keyed("&Save Project", ctrl(Key::S), Cmd::SaveProject),
        keyed("Save Project &As...", ctrl_shift(Key::S), Cmd::SaveProjectAs),
    ];
    if kind == WindowKind::None {
        // Origin's short bar has no Edit menu, but deleting the last window is an undoable
        // step, so Undo and Redo stay reachable with the mouse here.
        v.push(sep());
        v.push(keyed("&Undo", ctrl(Key::Z), Cmd::Undo));
        v.push(keyed("&Redo", ctrl_shift(Key::Z), Cmd::Redo));
        v.push(item("Undo AI Changes", Cmd::UndoAiChanges));
    }
    if kind != WindowKind::None {
        v.push(sep());
        v.push(item("Save &Window As...", Cmd::SaveWindowAs));
        if matches!(kind, WindowKind::Graph | WindowKind::Workbook) {
            v.push(item("Save Temp&late As...", Cmd::SaveTemplateAs));
        } else {
            v.push(grey("Save Template As..."));
        }
        if kind == WindowKind::Workbook {
            v.push(item("Save Workbook As Analysis Template...", Cmd::SaveAnalysisTemplateAs));
            v.push(grey("Add Word Bookmarks to Analysis Template..."));
        }
        v.push(item("Batch Processing...", Cmd::BatchProcessing));
    }
    v.extend([
        sep(),
        item("Save Project without Data...", Cmd::SaveProjectNoData),
        item("Duplicate Project without Data", Cmd::DuplicateProjectNoData),
        sep(),
        keyed("&Print...", ctrl(Key::P), Cmd::Print),
        item("Print Pre&view", Cmd::PrintPreview),
        item("Page Setup...", Cmd::PageSetup),
        sep(),
        sub(
            "&Import",
            [
                vec![
                    keyed("Import Wi&zard...", ctrl(Key::Num3), Cmd::ImportWizard),
                    keyed("Single &ASCII...", ctrl(Key::K), Cmd::Import),
                    item("&Multiple ASCII...", Cmd::ImportMultiple),
                    item("Comma &Delimited (CSV)...", Cmd::ImportCsv),
                    item("&Excel (XLS, XLSX, XLSM)...", Cmd::ImportExcel),
                ],
                vec![item("Image...", Cmd::ImportImage), item("Binary 2D Array...", Cmd::ImportBinary2d)],
                // Origin's graph File menu has no Re-Import items.
                if kind == WindowKind::Graph {
                    Vec::new()
                } else {
                    vec![
                        sep(),
                        keyed("Re-Import Directly", ctrl(Key::Num4), Cmd::Reimport(false)),
                        item("Re-Import...", Cmd::Reimport(true)),
                    ]
                },
                vec![sub(
                    "Recent Imports",
                    vec![
                        Entry::Recent(RecentKind::Imports),
                        sep(),
                        item("Clear Recent Imports", Cmd::ClearRecent(RecentKind::Imports)),
                    ],
                )],
            ]
            .concat(),
        ),
    ]);
    if kind == WindowKind::Graph {
        // Origin's graph menu has Export Graphs... where the workbook has the Export submenu.
        v.push(keyed("E&xport Graphs...", ctrl(Key::G), Cmd::ExportGraph));
        v.push(exports_submenu());
    } else {
        v.push(sub(
            "E&xport",
            [vec![
                item("&Graphs...", Cmd::ExportProjectGraphs(ProjectGraphsExport::Graphs)),
                item("&Worksheet as ASCII...", Cmd::ExportWorksheet),
                item("Worksheet as E&xcel...", Cmd::ExportExcel),
                item("Graph as Image...", Cmd::ExportProjectGraphs(ProjectGraphsExport::Image)),
                item("Worksheet as Image...", Cmd::ExportWorksheetImage),
                item("Multi-Page PDF...", Cmd::ExportProjectGraphs(ProjectGraphsExport::MultiPagePdf)),
                exports_submenu(),
            ]]
            .concat(),
        ));
    }
    v.push(sep());
    v.push(recent_submenu("Recent Books", RecentKind::Books));
    if kind == WindowKind::Graph {
        v.push(recent_submenu("Recent Graphs", RecentKind::Graphs));
    }
    v.extend([
        recent_submenu("Recent Projects", RecentKind::Projects),
        // Origin 2026's search of saved projects (T361); with any window type, also with no window open.
        item("Find in Project Files...", Cmd::FindInProjectFiles),
        sep(),
        item("Exi&t", Cmd::Exit),
    ]);
    v
}

fn edit_menu(kind: WindowKind) -> Vec<Entry> {
    let mut v = vec![
        keyed("&Undo", ctrl(Key::Z), Cmd::Undo),
        // Origin has no Redo; Sheaf keeps its own, with its own key.
        keyed("&Redo", ctrl_shift(Key::Z), Cmd::Redo),
        item("Und&o AI Changes", Cmd::UndoAiChanges),
        sep(),
    ];
    if kind == WindowKind::Workbook {
        v.push(keyed("Cu&t", ctrl(Key::X), Cmd::Cut));
        v.push(sub(
            "&Copy",
            vec![
                keyed("Copy", ctrl(Key::C), Cmd::Copy(CopyKind::Plain)),
                keyed("Copy full precision", ctrl_alt(Key::C), Cmd::Copy(CopyKind::FullPrecision)),
                keyed("Copy including label rows", ctrl_shift(Key::C), Cmd::Copy(CopyKind::WithLabels)),
                keyed("Copy formula", ctrl(Key::J), Cmd::Copy(CopyKind::Part(CopyPart::Formula))),
                // Origin lists Ctrl+Shift+J here too; Sheaf keeps that key for View > Full Screen.
                item("Copy label rows", Cmd::Copy(CopyKind::Part(CopyPart::LabelRows))),
                keyed(
                    "Copy formula and label rows",
                    ctrl_alt(Key::J),
                    Cmd::Copy(CopyKind::Part(CopyPart::FormulaAndLabels)),
                ),
                keyed("Copy All", ctrl_shift(Key::A), Cmd::Copy(CopyKind::Part(CopyPart::All))),
            ],
        ));
        v.push(keyed("Paste", ctrl(Key::V), Cmd::Paste));
        v.push(item("Paste Transpose", Cmd::PasteTranspose));
        v.push(keyed("Paste Link", ctrl_alt(Key::V), Cmd::PasteLink(false)));
        v.push(item("Paste Link Transpose", Cmd::PasteLink(true)));
        v.extend(greys(&["Paste Special", "-", "Select..."]));
        v.push(keyed("C&lear", plain_key(Key::Delete), Cmd::Clear));
        v.push(item("I&nsert", Cmd::InsertCells));
        v.push(item("&Delete", Cmd::DeleteCells));
        v.push(item("Remove Links", Cmd::RemoveLinks));
        v.push(sep());
        v.push(item("Set As Begin", Cmd::SetRangeEnd(RangeEnd::Begin)));
        // Every letter of its text is taken; it takes S from the greyed Paste Link.
        v.push(item("&Set As End", Cmd::SetRangeEnd(RangeEnd::End)));
        v.push(item("Reset to Full Range", Cmd::ResetFullRange));
        v.push(sep());
        v.push(keyed("Find in Project...", plain_key(Key::F3), Cmd::FindInProject));
        v.push(keyed("Find in Sheets...", ctrl(Key::F), Cmd::FindInSheets));
        v.push(keyed("R&eplace...", ctrl(Key::H), Cmd::ReplaceInSheets));
        v.push(keyed("Go to...", ctrl(Key::G), Cmd::GoToCell));
        v.extend(greys(&["-", "Button Edit Mode"]));
    } else {
        v.push(item("Cu&t", Cmd::GraphClipboard(GraphClip::Cut)));
        // Ctrl+X, C and V reach a graph as clipboard events (app.rs `shortcuts`), so these carry no
        // key of their own: the workbook's Cut, Copy and Paste have them.
        v.push(item("Copy", Cmd::GraphClipboard(GraphClip::Copy)));
        v.push(keyed("Copy Page", ctrl(Key::J), Cmd::CopyPage));
        v.push(item("Paste", Cmd::GraphClipboard(GraphClip::Paste)));
        v.extend(greys(&["Paste Link", "Paste Special"]));
        v.push(item("Clear", Cmd::GraphClipboard(GraphClip::Clear)));
        v.extend(greys(&["-", "Insert New Object", "Links", "Button Edit Mode", "Update Client"]));
        v.push(graph_copy_format_menu(
            true,
            &[
                ("All", Some(GraphPart::All)),
                ("All Style Formats", Some(GraphPart::AllStyle)),
                ("Colors", Some(GraphPart::Colors)),
                ("Fonts", Some(GraphPart::Fonts)),
                ("Background", Some(GraphPart::Background)),
                ("Symbol/Line/Fill", Some(GraphPart::SymbolLineFill)),
                ("Dimensions", Some(GraphPart::Dimensions)),
                ("Scales", Some(GraphPart::Scales)),
                ("Text", Some(GraphPart::Text)),
            ],
        ));
        v.push(item("Paste Format...", Cmd::PasteFormatDialog));
        v.push(sep());
        v.push(keyed("Find in Project", plain_key(Key::F3), Cmd::FindInProject));
    }
    v
}

fn view_menu(kind: WindowKind) -> Vec<Entry> {
    let mut v = vec![
        grey("Menu"),
        keyed("&Toolbars...", ctrl(Key::T), Cmd::Toolbars),
        item("Status Bar", Cmd::ToggleStatusBar),
        sep(),
    ];
    v.push(grey_keyed("Command Window", alt(Key::Num3)));
    v.push(grey_keyed("Code Builder", alt(Key::Num4)));
    v.push(grey_keyed("Quick Help", alt(Key::Num5)));
    v.push(keyed("Project Explorer", alt(Key::Num1), Cmd::Panel(PanelId::Explorer)));
    if kind == WindowKind::Graph {
        v.push(keyed("Object Manager", alt(Key::Num8), Cmd::Panel(PanelId::Objects)));
    }
    v.push(sub(
        "View Windows",
        sheaf_core::project::ViewWindows::ALL.iter().map(|m| item(m.name(), Cmd::ViewWindows(*m))).collect(),
    ));
    v.push(sep());
    v.push(keyed("Results Log", alt(Key::Num2), Cmd::Panel(PanelId::Results)));
    v.push(keyed("Messages Log", alt(Key::Num6), Cmd::Panel(PanelId::Messages)));
    v.push(grey_keyed("Smart Hint Log", alt(Key::Num7)));
    match kind {
        WindowKind::Workbook => {
            v.push(sep());
            v.push(keyed("&Freeze/Unfreeze Panes", ctrl_shift(Key::F), Cmd::FreezePanes));
            v.push(grey("Actively Update Plots"));
            v.push(item(
                "Page Break Preview Lines",
                Cmd::Sheet(SheetOp::ToggleView(sheaf_core::sheet_view::ViewSwitch::PageBreaks)),
            ));
        }
        WindowKind::Graph => {
            v.push(sep());
            use sheaf_core::project::ViewMode as VM;
            v.push(keyed("Print View", ctrl(Key::Num2), Cmd::SetViewMode(VM::Print)));
            v.push(keyed("Page View", ctrl(Key::Num1), Cmd::SetViewMode(VM::Page)));
            v.push(item("Window View", Cmd::SetViewMode(VM::Window)));
            v.push(keyed("Draft View", ctrl(Key::Num0), Cmd::SetViewMode(VM::Draft)));
            v.push(sep());
            v.push(keyed("Zoom In", ctrl(Key::I), Cmd::ZoomIn));
            v.push(keyed("Zoom Out", ctrl(Key::M), Cmd::ZoomOut));
            v.push(keyed("Whole Page", ctrl(Key::W), Cmd::WholePage));
            v.push(item("Zoom &All", Cmd::ZoomAll));
            v.push(sep());
            let mut show = vec![
                item("Layer Icons", Cmd::ToggleLayerIcons),
                item("Active Layer Indicator", Cmd::ToggleActiveLayerIndicator),
            ];
            show.extend(greys(&["Axis Layer Icons", "-", "Object Grid", "Layer Grid", "-", "Frame"]));
            show.push(item("Labels", Cmd::ToggleShowLabels));
            show.push(item("Data", Cmd::ToggleShowData));
            show.push(keyed("Active Layer Only", ctrl_alt(Key::D), Cmd::ToggleActiveLayerOnly));
            show.push(sep());
            show.push(grey("Master Items on Screen"));
            v.push(sub("Show", show));
            v.push(grey("Show Data Info"));
            v.push(keyed("Full Screen", ctrl_shift(Key::J), Cmd::FullScreen));
        }
        WindowKind::None | WindowKind::Matrix | WindowKind::Layout => {}
    }
    v
}

fn plot_menu() -> Vec<Entry> {
    let with = |first: Vec<Entry>, rest: &[&str]| [first, greys(rest)].concat();
    vec![
        sub(
            "&Line",
            with(
                vec![
                    item("&Line", Cmd::Plot(Template::Line)),
                    item("Horizontal Step", Cmd::Plot(Template::StepHorizontal)),
                    item("Vertical Step", Cmd::Plot(Template::StepVertical)),
                    item("Spline Connected", Cmd::Plot(Template::SplineConnected)),
                ],
                &[],
            ),
        ),
        sub(
            "&Symbol",
            [
                vec![
                    item("&Scatter", Cmd::Plot(Template::Scatter)),
                    item("Scatter + Rug", Cmd::Plot(Template::ScatterRug)),
                    item("Scatter Central", Cmd::Plot(Template::ScatterCentral)),
                    item("Column Scatter", Cmd::Plot(Template::ColumnScatter)),
                ],
                vec![item("Y Error", Cmd::PlotYError), item("XY Error", Cmd::PlotXYError)],
                vec![item("Vertical Drop Line", Cmd::Plot(Template::VerticalDropLine))],
                vec![item("Lollipop Plot", Cmd::Plot(Template::Lollipop))],
                vec![
                    item("Bubble", Cmd::Plot(Template::Bubble)),
                    item("Color Mapped", Cmd::Plot(Template::ColorMapped)),
                    item("Bubble + Color Mapped", Cmd::Plot(Template::BubbleColorMapped)),
                ],
            ]
            .concat(),
        ),
        sub(
            "Line + S&ymbol",
            vec![
                item("Line + &Symbol", Cmd::Plot(Template::LineSymbol)),
                item("2 Point Segment", Cmd::Plot(Template::Segment2)),
                item("3 Point Segment", Cmd::Plot(Template::Segment3)),
                item("Line Series", Cmd::PlotLineSeries),
            ],
        ),
        sub(
            "Column/Bar/&Pie",
            [
                vec![item("&Column", Cmd::Plot(Template::Column))],
                vec![item("Column + Label", Cmd::Plot(Template::ColumnLabel))],
                vec![item("Grouped Columns (indexed data)", Cmd::Plot(Template::GroupedColumns))],
                vec![
                    item("Bar", Cmd::Plot(Template::Bar)),
                    item("Stacked Column", Cmd::Plot(Template::StackedColumn)),
                    item("Stacked Bar", Cmd::Plot(Template::StackedBar)),
                    item("100% Stacked Column", Cmd::Plot(Template::Stacked100Column)),
                    item("100% Stacked Bar", Cmd::Plot(Template::Stacked100Bar)),
                    item("Floating Column", Cmd::Plot(Template::FloatingColumn)),
                    item("Floating Bar", Cmd::Plot(Template::FloatingBar)),
                ],
                vec![item("3D Color Pie", Cmd::Plot(Template::Pie3D)), item("2D B&&W Pie", Cmd::Plot(Template::PieBw))],
            ]
            .concat(),
        ),
        sep(),
        sub(
            "&Multi-Y",
            vec![
                item("&Double-Y", Cmd::Plot(Template::DoubleY)),
                item("3Ys Y-YY", Cmd::Plot(Template::Multi3YYy)),
                item("3Ys Y-Y-Y", Cmd::Plot(Template::Multi3YYY)),
                item("4Ys Y-YYY", Cmd::Plot(Template::Multi4YYyy)),
                item("4Ys YY-YY", Cmd::Plot(Template::Multi4YyYy)),
                item("Multiple Y Axes...", Cmd::MultiYAxes),
            ],
        ),
        sub(
            "Y-offset/&Waterfall",
            with(
                vec![
                    item("&Stacked Lines by Y Offsets", Cmd::Plot(Template::StackedLines)),
                    item("&Waterfall", Cmd::Plot(Template::Waterfall)),
                    item("Waterfall Y:Color Mapping", Cmd::Plot(Template::WaterfallY)),
                    item("Waterfall Z:Color Mapping", Cmd::Plot(Template::WaterfallZ)),
                    item("3D Waterfall", Cmd::Plot(Template::Waterfall3d)),
                ],
                &[],
            ),
        ),
        sub(
            "Multi-Pa&nel",
            [
                vec![
                    item("Vertical 2 Panel", Cmd::PlotPanels { rows: 2, cols: 1 }),
                    item("Horizontal 2 Panel", Cmd::PlotPanels { rows: 1, cols: 2 }),
                    item("4 Panel", Cmd::PlotPanels { rows: 2, cols: 2 }),
                    item("9 Panel", Cmd::PlotPanels { rows: 3, cols: 3 }),
                ],
                vec![item("Stack...", Cmd::StackPanels), item("Multiple Panels by Label...", Cmd::LabelPanels)],
            ]
            .concat(),
        ),
        sep(),
        sub(
            "3D &XYY",
            vec![
                item("XYY 3D Bars", Cmd::Plot(Template::Xyy3dBars)),
                item("3D Ribbons", Cmd::Plot(Template::Ribbons3d)),
                item("3D Walls", Cmd::Plot(Template::Walls3d)),
                item("3D Waterfall", Cmd::Plot(Template::Waterfall3d)),
            ],
        ),
        sub(
            "3D Su&rface",
            [
                vec![item("Color Fill Surface", Cmd::Plot(Template::ColorFillSurface3d))],
                vec![
                    item("X Constant with Base", Cmd::Plot(Template::XConstant3d)),
                    item("Y Constant with Base", Cmd::Plot(Template::YConstant3d)),
                ],
                vec![
                    item("Color Map Surface", Cmd::Plot(Template::ColorMapSurface3d)),
                    item("Colormap Surface with Projection", Cmd::Plot(Template::ColorMapProjection3d)),
                ],
                vec![
                    item("Wire Frame", Cmd::Plot(Template::WireFrame3d)),
                    item("Wire Surface", Cmd::Plot(Template::WireSurface3d)),
                ],
                vec![item("3D Ternary Colormap Surface", Cmd::Plot(Template::TernarySurface3d))],
            ]
            .concat(),
        ),
        sub(
            "3D Symbol/Bar/&Vector",
            [
                vec![
                    item("3D Bars", Cmd::Plot(Template::Bars3d)),
                    item("3D Scatter", Cmd::Plot(Template::Scatter3d)),
                    item("3D Trajectory", Cmd::Plot(Template::Trajectory3d)),
                    item("3D Scatter + Error Bar", Cmd::Plot(Template::Scatter3dError)),
                ],
                vec![
                    item("3D Vector XYZ XYZ", Cmd::Plot(Template::Vector3dXyz)),
                    item("3D Vector XYZ dXdYdZ", Cmd::Plot(Template::Vector3dDelta)),
                ],
            ]
            .concat(),
        ),
        sep(),
        sub(
            "S&tatistics",
            [
                vec![item("Box Chart", Cmd::Plot(Template::BoxChart))],
                vec![
                    item("Grouped Box Charts", Cmd::Plot(Template::GroupedBox)),
                    item("Interval Plot", Cmd::Plot(Template::IntervalPlot)),
                ],
                vec![
                    item("Violin Plot", Cmd::Plot(Template::Violin)),
                    item("Violin with Box", Cmd::Plot(Template::ViolinBox)),
                    item("Violin with Point", Cmd::Plot(Template::ViolinPoint)),
                    item("Violin with Quartile", Cmd::Plot(Template::ViolinQuartile)),
                    item("Violin with Stick", Cmd::Plot(Template::ViolinStick)),
                    item("Half Violin", Cmd::Plot(Template::HalfViolin)),
                ],
                vec![item("Ridgeline Chart", Cmd::Plot(Template::Ridgeline))],
                vec![item("Histogram", Cmd::Plot(Template::Histogram))],
                vec![
                    item("Histogram + Probabilities", Cmd::Plot(Template::HistogramProbabilities)),
                    item("Stacked Histograms", Cmd::Plot(Template::StackedHistograms)),
                    item("Grouped Histogram...", Cmd::GroupedHistogram),
                ],
                vec![item("Pareto", Cmd::Plot(Template::Pareto))],
                vec![
                    item("Marginal Histograms", Cmd::Plot(Template::MarginalHistograms)),
                    item("Marginal Box Charts", Cmd::Plot(Template::MarginalBoxCharts)),
                ],
                vec![item("2D Kernel Density...", Cmd::Plot(Template::Kde2d))],
                vec![item("QC (X bar R) Chart", Cmd::Plot(Template::QcXbarR)), item("&Run Chart...", Cmd::RunChart)],
                // Every letter of the label was taken; C moves Marginal Box Charts to n.
                vec![item("S&catter Matrix", Cmd::Plot(Template::ScatterMatrix))],
                // T151: the same windows as Statistics > Descriptive Statistics.
                vec![
                    // Q, -, l, o and t are taken here: Q-Q Plot takes P, Probability Plot y.
                    item("Probabilit&y Plot", Cmd::Analyze(AnalysisKind::ProbPlot)),
                    item("Q-Q &Plot", Cmd::Analyze(AnalysisKind::QQPlot)),
                ],
            ]
            .concat(),
        ),
        sub(
            "Cate&gorical",
            vec![
                item("Parallel Plot", Cmd::Plot(Template::Parallel)),
                item("Parallel Index", Cmd::Plot(Template::ParallelIndex)),
                item("Treemap", Cmd::Plot(Template::Treemap)),
                item("Sunburst Plot", Cmd::Plot(Template::Sunburst)),
            ],
        ),
        sub(
            "&Area",
            vec![
                item("&Area", Cmd::Plot(Template::Area)),
                item("Stacked Area", Cmd::Plot(Template::StackedArea)),
                item("100% Stacked Area", Cmd::Plot(Template::Stacked100Area)),
                item("Fill Area", Cmd::Plot(Template::FillArea)),
            ],
        ),
        sub(
            "Contour/&Heat Map",
            [
                field_plot_items(),
                vec![
                    item("Polar Contour theta(X) r(Y)", Cmd::PolarContour { theta_first: true }),
                    item("Polar Contour r(X) theta(Y)", Cmd::PolarContour { theta_first: false }),
                ],
                vec![item("Ternary Contour", Cmd::TernaryContour)],
                vec![item("Hexbin", Cmd::Plot(Template::Hexbin))],
            ]
            .concat(),
        ),
        profile_menu(),
        sub(
            "Specialize&d",
            [
                vec![
                    item("Polar theta(X) r(Y)", Cmd::Plot(Template::Polar)),
                    item("Polar r(X) theta(Y)", Cmd::Plot(Template::PolarRTheta)),
                    item("Wind Rose - Binned Data", Cmd::Plot(Template::WindRoseBinned)),
                    item("Wind Rose - Raw Data", Cmd::Plot(Template::WindRoseRaw)),
                ],
                vec![item("Ternary", Cmd::Plot(Template::Ternary))],
                vec![item("Piper", Cmd::Plot(Template::Piper))],
                vec![item("Smith Chart", Cmd::Plot(Template::SmithChart))],
                vec![item("Radar", Cmd::Plot(Template::Radar))],
                vec![
                    item("High-Low-Close", Cmd::Plot(Template::HighLowClose)),
                    item("Vector XYAM", Cmd::Plot(Template::VectorXyam)),
                    item("Vector XYXY", Cmd::Plot(Template::VectorXyxy)),
                ],
                vec![item("Zoom", Cmd::Plot(Template::Zoom))],
            ]
            .concat(),
        ),
        sub(
            "Stoc&k",
            [vec![
                item("High-Low-Close", Cmd::Plot(Template::HighLowClose)),
                item("Japanese Candlestick", Cmd::Plot(Template::Candlestick)),
                item("OHLC Bar Chart", Cmd::Plot(Template::OhlcBar)),
                item("OHLC-Volume", Cmd::Plot(Template::OhlcVolume)),
                item("Line", Cmd::Plot(Template::StockLine)),
            ]]
            .concat(),
        ),
        sep(),
        item("System Templates...", Cmd::SystemTemplates),
        // Sheaf's own graph style is the one template that has no Origin counterpart.
        sub(
            "&User Templates",
            vec![
                item("Cyclic voltammetry overlay", Cmd::Plot(Template::CyclicVoltammetry)),
                sep(),
                Entry::UserTemplates,
                sep(),
                item("&Manage...", Cmd::ManageTemplates),
            ],
        ),
        sep(),
        mru_submenu("Most Recently Used", MruMenu::Plot),
    ]
}

fn column_menu() -> Vec<Entry> {
    let set_as = [
        ("&X", Designation::X),
        ("&Y", Designation::Y),
        ("&Z", Designation::Z),
        ("&Label", Designation::Label),
        ("&Disregard", Designation::Disregard),
        ("Y E&rror", Designation::YError),
        ("X Err&or", Designation::XError),
        ("&Group", Designation::Group),
        ("&Subject", Designation::Subject),
    ];
    let set_entries: Vec<Entry> = set_as.iter().map(|(l, d)| item(l, Cmd::SetAs(*d))).collect();
    vec![
        sub("Set &As", set_entries),
        item("Set as Categorical", Cmd::SetCategorical),
        sep(),
        keyed("Set Column &Values...", ctrl(Key::Q), Cmd::SetColumnValues),
        keyed("Set Multiple Columns Values...", ctrl_shift(Key::Q), Cmd::SetMultipleColumnsValues),
        // Origin has this as a toolbar button and in the column's right-click menu.
        item("Re&calculate", Cmd::Recalculate),
        sub(
            "Fill Column With",
            vec![
                item("Row Numbers", Cmd::QuickFill(QuickFill::RowNumbers)),
                item("Uniform Random Numbers", Cmd::QuickFill(QuickFill::Uniform)),
                item("Normal Random Numbers", Cmd::QuickFill(QuickFill::Normal)),
                item("A set of Numbers...", Cmd::FillPattern(FillKind::Numbers)),
                item("A set of Date/Time values...", Cmd::FillPattern(FillKind::Dates)),
                item("Arbitrary set of Text&&Numeric values...", Cmd::FillPattern(FillKind::Text)),
            ],
        ),
        sep(),
        item("Set Sampling Interval...", Cmd::SetSamplingInterval),
        item("Show X Column...", Cmd::ShowXColumn),
        sep(),
        keyed("Add &New Columns...", ctrl(Key::D), Cmd::AddColumns),
        item("Add Column to All Sheets...", Cmd::AddColumnAcross),
        hide_menu(),
        sep(),
        filter_menu(),
        mask_menu(),
        item("Mask Cells by Condition...", Cmd::MaskByCondition),
    ]
}

/// The Mask submenu: Apply, Remove and Swap work on the selected cells; the rest is greyed.
fn mask_menu() -> Entry {
    let entries = vec![
        item("Apply", Cmd::Mask(MaskOp::Apply)),
        item("Remove", Cmd::Mask(MaskOp::Remove)),
        item("Swap", Cmd::Mask(MaskOp::Swap)),
        item("Change Color", Cmd::MaskColor),
        item("Disable Masking", Cmd::ToggleMasking),
        // Origin has this switch on its Mask toolbar (`@MP`); Sheaf puts it here.
        item("Hide Masked Points", Cmd::ToggleHideMasked),
    ];
    sub("Mask", entries)
}

/// Worksheet > Conditional Formatting (also in the right-click menu of selected cells).
fn cond_format_menu() -> Entry {
    use crate::dialogs::cond_format::CondKind;
    let mut entries = vec![
        item("Highlight Min Max...", Cmd::CondFormat(CondKind::MinMax)),
        item("Highlight Top Bottom...", Cmd::CondFormat(CondKind::TopBottom)),
        item("Conditional Highlight...", Cmd::CondFormat(CondKind::Highlight)),
        item("Conditional Heatmap...", Cmd::CondFormat(CondKind::Heatmap)),
        item("Conditional Pareto...", Cmd::CondFormat(CondKind::Pareto)),
        item("Conditional Outliers...", Cmd::CondFormat(CondKind::Outliers)),
    ];
    entries.extend(greys(&["-"]));
    entries.push(item("Remove Conditional Format", Cmd::RemoveCondFormat));
    entries.push(item("Conditional Format Manager...", Cmd::CondFormatManager));
    sub("Conditional Formatting", entries)
}

fn filter_menu() -> Entry {
    sub(
        "Filter",
        vec![
            item("Add or Remove Filter", Cmd::RowFilterDialog),
            item("Enable/Disable Filter", Cmd::ToggleRowFilter),
            item("Reapply Filter", Cmd::ReapplyRowFilter),
        ],
    )
}

fn hide_menu() -> Entry {
    sub("Hide/Unhide Columns", vec![item("Hide Selected", Cmd::HideColumns), item("Unhide All", Cmd::UnhideColumns)])
}

fn worksheet_menu() -> Vec<Entry> {
    vec![
        sub(
            "Sort &Range",
            vec![
                item("Ascending", Cmd::SortRange { descending: false }),
                item("Descending", Cmd::SortRange { descending: true }),
                item("Custom...", Cmd::SortCustom { range: true }),
            ],
        ),
        sub(
            "Sort &Columns",
            vec![
                item("Ascending", Cmd::SortRange { descending: false }),
                item("Descending", Cmd::SortRange { descending: true }),
                item("Each Column Ascending", Cmd::SortColumns { descending: false }),
                item("Each Column Descending", Cmd::SortColumns { descending: true }),
                item("Custom...", Cmd::SortCustom { range: true }),
            ],
        ),
        sub(
            "&Sort Worksheet",
            vec![
                item("&Ascending", Cmd::SortAscending),
                item("&Descending", Cmd::SortDescending),
                item("&Custom...", Cmd::SortCustom { range: false }),
            ],
        ),
        item("Sort Columns by Label...", Cmd::SortByLabel),
        sep(),
        cond_format_menu(),
        item("Clear Worksheet...", Cmd::ClearWorksheet),
        item("Split Columns...", Cmd::Reshape(ReshapeKind::Split)),
        item("Pivot Table...", Cmd::PivotTable),
        item("Time Series Pivot...", Cmd::Analyze(AnalysisKind::TimePivot)),
        item("Reduce Rows...", Cmd::Reshape(ReshapeKind::Reduce)),
        item("Transpose...", Cmd::Reshape(ReshapeKind::Transpose)),
        item("Stack Columns...", Cmd::Reshape(ReshapeKind::Stack)),
        item("Unstack Columns...", Cmd::Reshape(ReshapeKind::Unstack)),
        sub(
            "Convert to Matrix",
            vec![
                item("Direct", Cmd::ConvertToMatrix { xyz: false }),
                item("XYZ Gridding", Cmd::ConvertToMatrix { xyz: true }),
                grey("Extract"),
            ],
        ),
        sep(),
        grey("Worksheet Script..."),
        // Sheaf's "keep only the rows matching a condition" is Origin's worksheet query.
        item("Worksheet &Query...", Cmd::FilterRows),
        mru_submenu("Most Recently Used Items", MruMenu::Worksheet),
    ]
}

fn analysis_menu() -> Vec<Entry> {
    vec![
        sub(
            "&Mathematics",
            vec![
                item("Simple Curve Math...", Cmd::Analyze(AnalysisKind::CurveMath)),
                item("Simple Column Math...", Cmd::Analyze(AnalysisKind::ColumnMath)),
                sep(),
                item("Differentiate...", Cmd::Analyze(AnalysisKind::Differentiate)),
                item("Integrate...", Cmd::Analyze(AnalysisKind::Integrate)),
                // Sheaf's own: integrate several ranges in one go.
                item("Integrate Ranges...", Cmd::Analyze(AnalysisKind::IntegrateRanges)),
                // Sheaf's own (T296): the charge of each sweep of a cyclic voltammogram.
                item("CV Charge...", Cmd::Analyze(AnalysisKind::CvCharge)),
                sep(),
                item("Interpolate/Extrapolate...", Cmd::Analyze(AnalysisKind::Interpolate)),
                item("Interpolate/Extrapolate Y from X...", Cmd::InterpolateFromColumn),
                item("Trace Interpolation...", Cmd::Analyze(AnalysisKind::TraceInterp)),
                item("Linear Interpolation...", Cmd::InterpolateWith(sheaf_core::analysis::InterpMethod::Linear)),
                item("Spline Interpolation...", Cmd::InterpolateWith(sheaf_core::analysis::InterpMethod::CubicSpline)),
                item("B-Spline Interpolation...", Cmd::InterpolateWith(sheaf_core::analysis::InterpMethod::BSpline)),
                sep(),
                item("Intersect...", Cmd::Analyze(AnalysisKind::Intersect)),
                // Sheaf's own: Origin has the levelcrossing X-Function but no menu item for it.
                item("Level Crossing...", Cmd::Analyze(AnalysisKind::LevelCrossing)),
                item("Normalize Columns...", Cmd::Analyze(AnalysisKind::Normalize)),
                item("Average Multiple Curves...", Cmd::Analyze(AnalysisKind::AverageCurves)),
                item("Polygon Area...", Cmd::Analyze(AnalysisKind::PolygonArea)),
                item("XYZ Surface Area...", Cmd::Analyze(AnalysisKind::XyzArea)),
                item("Median Filter...", Cmd::Analyze(AnalysisKind::MedianFilter)),
                grey("Special Filter..."),
                item("In&verse...", Cmd::Matrix(MatrixCmd::Inverse)),
            ],
        ),
        sub(
            "&Data Manipulation",
            vec![
                item("Subtract Reference Data...", Cmd::Analyze(AnalysisKind::SubtractRef)),
                item("Subtract Straight Line...", Cmd::Analyze(AnalysisKind::SubtractLine)),
                item("Reduce Duplicate X Data...", Cmd::Analyze(AnalysisKind::ReduceDup)),
                item("Reduce to Evenly Spaced X...", Cmd::Analyze(AnalysisKind::ReduceX)),
                item("Curve Translate X...", Cmd::Analyze(AnalysisKind::TranslateX)),
                // Sheaf's own: the same window along Y.
                item("Curve Translate Y...", Cmd::Analyze(AnalysisKind::TranslateY)),
            ],
        ),
        sub(
            "&Fitting",
            vec![
                item("Linear Fit...", Cmd::Fit(FitKind::Linear)),
                item("Fit Linear with X Error...", Cmd::Fit(FitKind::LinearXError)),
                item("Polynomial Fit...", Cmd::Fit(FitKind::Polynomial)),
                item("Multiple Linear Regression...", Cmd::FitMore(FitMoreKind::Multiple)),
                item("Best Subset Selection...", Cmd::FitMore(FitMoreKind::BestSubset)),
                sep(),
                keyed("Nonlinear Curve Fit...", ctrl(Key::Y), Cmd::Fit(FitKind::BuiltIn)),
                item("Nonlinear Implicit Curve Fit...", Cmd::FitMore(FitMoreKind::Implicit)),
                item("Nonlinear Surface Fit...", Cmd::Fit(FitKind::Surface)),
                sep(),
                item("Simulate Curve...", Cmd::SimulateCurve),
                item("Simulate Surface...", Cmd::SimulateSurface),
                sep(),
                item("Exponential Fit...", Cmd::Fit(FitKind::Exponential)),
                item("Single Peak Fit...", Cmd::Fit(FitKind::SinglePeak)),
                item("Sigmoidal Fit...", Cmd::Fit(FitKind::Sigmoidal)),
                sep(),
                item("Compare Datasets...", Cmd::FitMore(FitMoreKind::CompareDatasets)),
                item("Compare Models...", Cmd::FitMore(FitMoreKind::Compare)),
                item("Rank Models...", Cmd::FitMore(FitMoreKind::Rank)),
            ],
        ),
        sub(
            "Si&gnal Processing",
            vec![
                item("S&mooth...", Cmd::Analyze(AnalysisKind::Smooth)),
                item("FFT Filters...", Cmd::Analyze(AnalysisKind::FftFilter)),
                sub(
                    "FFT",
                    vec![
                        item("FFT...", Cmd::Analyze(AnalysisKind::Fft)),
                        item("IFFT...", Cmd::Analyze(AnalysisKind::Ifft)),
                        item("2D FFT...", Cmd::Matrix(MatrixCmd::Fft2)),
                        item("2D IFFT...", Cmd::Matrix(MatrixCmd::Ifft2)),
                    ],
                ),
                item("Convolution...", Cmd::Analyze(AnalysisKind::Convolution)),
                item("Deconvolution...", Cmd::Analyze(AnalysisKind::Deconvolution)),
                item("Correlation...", Cmd::Analyze(AnalysisKind::SignalCorrelation)),
                item("Coherence...", Cmd::Analyze(AnalysisKind::Coherence)),
                item("Decimation...", Cmd::Analyze(AnalysisKind::Decimate)),
                item("Envelope...", Cmd::Analyze(AnalysisKind::Envelope)),
                item("Hilbert Transform...", Cmd::Analyze(AnalysisKind::Hilbert)),
                item("IIR Filter...", Cmd::Analyze(AnalysisKind::IirFilter)),
                item("FIR Filter...", Cmd::Analyze(AnalysisKind::FirFilter)),
                item("&STFT...", Cmd::Analyze(AnalysisKind::Stft)),
                sub(
                    "Wavelet",
                    vec![
                        item("Decompose...", Cmd::Analyze(AnalysisKind::WaveletDecompose)),
                        item("Reconstruction...", Cmd::Analyze(AnalysisKind::WaveletReconstruct)),
                        item("Denoise...", Cmd::Analyze(AnalysisKind::WaveletDenoise)),
                        item("Smooth...", Cmd::Analyze(AnalysisKind::WaveletSmooth)),
                        item("Continuous Wavelet...", Cmd::Analyze(AnalysisKind::ContinuousWavelet)),
                    ],
                ),
            ],
        ),
        sub(
            "&Peaks and Baseline",
            vec![
                item("Peak Analyzer...", Cmd::Analyze(AnalysisKind::PeakChain)),
                item("Find Peaks...", Cmd::Analyze(AnalysisKind::Peaks)),
                item("Subtract Baseline...", Cmd::Analyze(AnalysisKind::Baseline)),
                item("Multiple Peak Fit", Cmd::MultiPeakFit),
                item("Fourier Self-Deconvolution...", Cmd::Analyze(AnalysisKind::Fsd)),
                item("Batch Peak Analysis Using Theme...", Cmd::BatchPeaks),
            ],
        ),
        sep(),
        item("Repeat Last Analysis", Cmd::RepeatAnalysis),
        mru_submenu("Most Recently Used Items", MruMenu::Analysis),
    ]
}

fn statistics_menu() -> Vec<Entry> {
    vec![
        sub(
            "&Descriptive Statistics",
            vec![
                item("Statistics on &Columns...", Cmd::Analyze(AnalysisKind::Statistics)),
                item("Statistics on &Rows...", Cmd::Analyze(AnalysisKind::RowStatistics)),
                sep(),
                item("Graphical Summary...", Cmd::Analyze(AnalysisKind::GraphSummary)),
                item("Stem-and-Leaf Plot...", Cmd::Analyze(AnalysisKind::StemLeaf)),
                sep(),
                item("Cross Tabulation...", Cmd::Analyze(AnalysisKind::Crosstab)),
                sep(),
                item("Discrete Frequency...", Cmd::Analyze(AnalysisKind::DiscreteFreq)),
                item("Frequency Counts...", Cmd::Analyze(AnalysisKind::Histogram)),
                item("2D Frequency Counts/Binning...", Cmd::Analyze(AnalysisKind::Binning2D)),
                item("Binned Scatter...", Cmd::Analyze(AnalysisKind::BinScatter)),
                sep(),
                item("Distribution Fit...", Cmd::Analyze(AnalysisKind::DistFit)),
                item("Probability Plot...", Cmd::Analyze(AnalysisKind::ProbPlot)),
                item("Q-Q Plot...", Cmd::Analyze(AnalysisKind::QQPlot)),
                item("Normality Test...", Cmd::Analyze(AnalysisKind::Normality)),
                sep(),
                item("Correlation Coefficient...", Cmd::Analyze(AnalysisKind::Correlation)),
                item("Partial Correlation Coefficient...", Cmd::Analyze(AnalysisKind::PartialCorr)),
                sep(),
                item("Grubbs Test...", Cmd::Analyze(AnalysisKind::Grubbs)),
                item("Dixon's Q-test...", Cmd::Analyze(AnalysisKind::Dixon)),
            ],
        ),
        sub(
            "&Hypothesis Testing",
            vec![
                item("One-Sample t-Test...", Cmd::Analyze(AnalysisKind::TTest)),
                item("Pair-Sample t-Test...", Cmd::Analyze(AnalysisKind::TTest)),
                item("Two-Sample t-Test...", Cmd::Analyze(AnalysisKind::TTest)),
                item("One-Sample Test for Variance...", Cmd::Analyze(AnalysisKind::VarTestOne)),
                item("Two-Sample Test for Variance...", Cmd::Analyze(AnalysisKind::VarTestTwo)),
                sep(),
                item("One-Sample Proportion Test...", Cmd::Analyze(AnalysisKind::PropTestOne)),
                item("Two-Sample Proportion Test...", Cmd::Analyze(AnalysisKind::PropTestTwo)),
                sep(),
                item("One-Sample Poisson Rate Test...", Cmd::Analyze(AnalysisKind::PoissonOne)),
                item("Two-Sample Poisson Rate Test...", Cmd::Analyze(AnalysisKind::PoissonTwo)),
            ],
        ),
        sub(
            "AN&OVA",
            vec![
                item("One-Way ANOVA...", Cmd::Analyze(AnalysisKind::Anova)),
                item("Two-Way ANOVA...", Cmd::Analyze(AnalysisKind::Anova2)),
                item("Three-Way ANOVA...", Cmd::Analyze(AnalysisKind::Anova3)),
                item("One-Way Repeated Measures ANOVA...", Cmd::Analyze(AnalysisKind::RmAnova1)),
                item("Two-Way Repeated Measures ANOVA...", Cmd::Analyze(AnalysisKind::RmAnova2)),
            ],
        ),
        item("Multiple Comparisons Plot...", Cmd::ComparisonPlot),
        sub(
            "&Nonparametric Tests",
            vec![
                item("One-Sample Wilcoxon Signed Rank Test...", Cmd::Analyze(AnalysisKind::SignedRank)),
                item("Paired Sample Wilcoxon Signed Rank Test...", Cmd::Analyze(AnalysisKind::SignedRankPaired)),
                item("Mann-Whitney Test...", Cmd::Analyze(AnalysisKind::MannWhitney)),
                item("Two Sample Kolmogorov-Smirnov Test...", Cmd::Analyze(AnalysisKind::KsTwoSample)),
                item("Kruskal-Wallis ANOVA...", Cmd::Analyze(AnalysisKind::KruskalWallis)),
                item("Mood's Median Test...", Cmd::Analyze(AnalysisKind::MedianTest)),
                item("Friedman ANOVA...", Cmd::Analyze(AnalysisKind::Friedman)),
                item("Paired Sample Sign Test...", Cmd::Analyze(AnalysisKind::SignTest)),
            ],
        ),
        sub("&Survival Analysis", vec![item("Kaplan-Meier Estimator...", Cmd::Analyze(AnalysisKind::KaplanMeier))]),
        sub(
            "&Multivariate Analysis",
            vec![
                item("Principal Component Analysis...", Cmd::Analyze(AnalysisKind::Pca)),
                item("K-Means Cluster Analysis...", Cmd::Analyze(AnalysisKind::Kmeans)),
            ],
        ),
        sub(
            "&Power and Sample Size",
            vec![
                item("One-Sample t-Test...", Cmd::Power(sheaf_core::analysis::PowerKind::OneSampleT)),
                item("Two-Sample t-Test...", Cmd::Power(sheaf_core::analysis::PowerKind::TwoSampleT)),
                item("Paired Sample t-Test...", Cmd::Power(sheaf_core::analysis::PowerKind::PairedT)),
                item("One-Way ANOVA...", Cmd::Power(sheaf_core::analysis::PowerKind::Anova)),
            ],
        ),
        sep(),
        item("&ROC Curve...", Cmd::Analyze(AnalysisKind::Roc)),
        item("&Quantile Regression...", Cmd::Analyze(AnalysisKind::QuantReg)),
        item("&Change Point Analysis...", Cmd::Analyze(AnalysisKind::ChangePoint)),
        item("M&anhattan Plot...", Cmd::Analyze(AnalysisKind::Manhattan)),
        sep(),
        mru_submenu("Most Recently Used Items", MruMenu::Statistics),
    ]
}

fn image_menu() -> Vec<Entry> {
    use ImageCmd::*;
    let i = |label: &str, c: ImageCmd| item(label, Cmd::Image(c));
    vec![
        sub(
            "&Adjustments",
            vec![
                i("Brightness...", Brightness),
                i("Contrast...", Contrast),
                i("Gamma...", Gamma),
                i("Levels...", Levels),
            ],
        ),
        sub("Arithmetic &Transform", vec![i("Math...", Math), i("Subtract...", Subtract)]),
        sub(
            "&Conversion",
            vec![i("To Binary...", ToBinary), i("Color to Gray...", ColorToGray), i("RGB Split...", RgbSplit)],
        ),
        sub(
            "&Geometric Transform",
            vec![i("Crop...", Crop), i("Resize...", Resize), i("Rotate...", Rotate), i("Flip...", Flip)],
        ),
        sub(
            "&Spatial Filters",
            vec![
                i("Smooth...", Smooth),
                i("Median...", Median),
                i("Sharpen...", Sharpen),
                i("Edge Detection...", EdgeDetection),
            ],
        ),
    ]
}

fn graph_menu() -> Vec<Entry> {
    vec![
        // Layer Contents is Origin's own window; Plot Setup opens Sheaf's Plots tab (add and remove plots).
        keyed("Layer Contents...", plain_key(Key::F12), Cmd::LayerContents(None)),
        item("Plot Setup...", Cmd::PlotSetup(None)),
        // Origin opens Layer Contents with the plot type chosen (T63 group J).
        sub(
            "Add Plot to Layer",
            vec![
                item("Line...", Cmd::AddPlotToLayer(PlotKind::Line)),
                item("Scatter...", Cmd::AddPlotToLayer(PlotKind::Scatter)),
                item("Line + Symbol...", Cmd::AddPlotToLayer(PlotKind::LineSymbol)),
                item("Column...", Cmd::AddPlotToLayer(PlotKind::Column)),
                item("Area...", Cmd::AddPlotToLayer(PlotKind::Area)),
            ],
        ),
        sep(),
        item("Layer Management...", Cmd::LayerManagement(None)),
        item("Add Error Bars...", Cmd::AddErrorBars),
        item("Add Function Graph...", Cmd::AddFunctionGraph),
        item("Add Straight Line...", Cmd::AddStraightLine),
        item("Add Text...", Cmd::AddText),
        new_layer_menu(),
        item("Merge Graph Windows...", Cmd::MergeGraphs),
        sep(),
        keyed("Rescale to Show All", ctrl(Key::R), Cmd::Rescale),
        sub(
            "&Legend",
            vec![
                // Origin opens the legend's settings by double click; Sheaf also offers them here.
                item("Edit Legend Text...", Cmd::EditLegendText { layer: None }),
                item("Legend Properties...", Cmd::GraphProps(PropsTab::Legend)),
                keyed("Reconstruct Legend", ctrl(Key::L), Cmd::ReconstructLegend),
                item("Update Legend", Cmd::UpdateLegend),
                grey("Add Legend Title"),
            ],
        ),
        item("New Table", Cmd::NewTable),
        item("New Color Scale", Cmd::NewColorScale),
        sep(),
        grey("Fit Layer to Graph"),
        item("Fit Page to Layers...", Cmd::FitPageToLayers),
        item("Exchange X-Y Axes", Cmd::ExchangeAxes),
        sub(
            "Offset Grouped Data in Layer",
            vec![
                item("None", Cmd::SetStack(Stack::None)),
                item("Cumulative", Cmd::SetStack(Stack::Cumulative)),
                item("Constant", Cmd::SetStack(Stack::Constant { step: 1.0 })),
                item("Auto", Cmd::SetStack(Stack::Auto { gap_percent: 8.0 })),
                item("Individual", Cmd::SetStack(Stack::Individual)),
            ],
        ),
        grey("Convert to Standard Font Size"),
    ]
}

fn data_menu() -> Vec<Entry> {
    vec![
        item("Set Display Range", Cmd::DataMarkers(MarkerOp::SetRange)),
        item("Reset to Full Range", Cmd::DataMarkers(MarkerOp::ResetRange)),
        sep(),
        keyed("Mark Data Range", ctrl_alt(Key::M), Cmd::DataTool(DataTool::MarkRange)),
        keyed("Clear Data Markers", ctrl_alt(Key::N), Cmd::DataMarkers(MarkerOp::Clear)),
        sep(),
        {
            use sheaf_core::data_markers::{MarkerShow, MarkerSize};
            let show = |label, s| item(label, Cmd::DataMarkers(MarkerOp::Show(s)));
            let size = |label, s| item(label, Cmd::DataMarkers(MarkerOp::Size(s)));
            sub(
                "Analysis Markers",
                vec![
                    show("Hide", MarkerShow::Hide),
                    show("Show Partial Ranges", MarkerShow::PartialRanges),
                    show("Show All", MarkerShow::All),
                    sep(),
                    size("Tiny", MarkerSize::Tiny),
                    size("Small", MarkerSize::Small),
                    size("Medium", MarkerSize::Medium),
                    size("Large", MarkerSize::Large),
                ],
            )
        },
        sub(
            "Lock Position",
            vec![
                item("Layer Icon", Cmd::LockPosition(LockPosition::LayerIcon)),
                item("First Point", Cmd::LockPosition(LockPosition::FirstPoint)),
                item("Last Point", Cmd::LockPosition(LockPosition::LastPoint)),
            ],
        ),
        sep(),
        item("Mask Data Points", Cmd::DataTool(DataTool::Mask)),
        item("Move Data Points", Cmd::DataTool(DataTool::Move)),
        item("Remove Bad Data Points", Cmd::DataTool(DataTool::RemoveBad)),
    ]
}

fn gadgets_menu() -> Vec<Entry> {
    let mut v = vec![item("Statistics...", Cmd::StatisticsGadget), item("Quick Fit...", Cmd::QuickFitGadget)];
    v.push(item("Quick Sigmoidal Fit...", Cmd::SigmoidGadget));
    v.push(item("Quick Peaks...", Cmd::QuickPeaksGadget));
    v.push(item("Integrate...", Cmd::IntegrateGadget));
    v.push(item("Differentiate...", Cmd::DifferentiateGadget));
    v.push(item("Interpolate...", Cmd::InterpolateGadget));
    v.push(item("Intersect...", Cmd::IntersectGadget));
    v.push(item("Rise Time...", Cmd::RiseTimeGadget));
    v.push(item("Cluster...", Cmd::ClusterGadget));
    v.push(item("&FFT...", Cmd::FftGadget));
    v.push(item("2D Integrate...", Cmd::Integrate2dGadget));
    v.push(sep());
    v.push(item("Vertical Cursor...", Cmd::VerticalCursorGadget));
    v.extend(greys(&["Go To..."]));
    v
}

/// The right-click menu of the Integrate gadget.
/// The right-click menu of the Rise Time gadget (T138).
pub fn rise_time_gadget_menu() -> Vec<Entry> {
    vec![keyed("Delete", plain_key(Key::Escape), Cmd::Gadget(GadgetOp::Close))]
}

/// The 2D Integrate gadget's right-click menu (T198).
pub fn integrate_2d_gadget_menu(circle: bool, plane: bool) -> Vec<Entry> {
    let tick = |on: bool, name: &str| if on { format!("\u{2713} {name}") } else { name.to_string() };
    vec![
        sub(
            "Region",
            vec![
                item(&tick(!circle, "Rectangle"), Cmd::Gadget(GadgetOp::SetRegion(false))),
                item(&tick(circle, "Circle"), Cmd::Gadget(GadgetOp::SetRegion(true))),
            ],
        ),
        // T206: the base the volume is measured from.
        item(if plane { "Base: Zero" } else { "Base: Plane Through Boundary" }, Cmd::Gadget(GadgetOp::ToggleBaseline)),
        item("New Output", Cmd::Gadget(GadgetOp::NewOutput)),
        sep(),
        keyed("Delete", plain_key(Key::Escape), Cmd::Gadget(GadgetOp::Close)),
    ]
}

pub fn integrate_gadget_menu(end_point_baseline: bool) -> Vec<Entry> {
    let baseline = if end_point_baseline { "Baseline: Zero" } else { "Baseline: Line Through End Points" };
    vec![
        item(baseline, Cmd::Gadget(GadgetOp::ToggleBaseline)),
        item("New Output", Cmd::Gadget(GadgetOp::NewOutput)),
        item("Preferences...", Cmd::Gadget(GadgetOp::Preferences)),
        sep(),
        keyed("Delete", plain_key(Key::Escape), Cmd::Gadget(GadgetOp::Close)),
    ]
}

fn tools_menu() -> Vec<Entry> {
    vec![
        // Origin shows Ctrl+U here; Sheaf leaves it to Underline in worksheets and text (gui-look "Options").
        item("&Options...", Cmd::Options),
        item("&Autosave...", Cmd::Autosave),
        item("AI Assistants (&MCP)", Cmd::ToggleAiLink),
        grey("System Variables..."),
        grey("3D OpenGL Settings..."),
        sub("Protection", vec![grey("Protect Project..."), item("Audit Log", Cmd::AuditLog)]),
        grey("Apps on File Exchange..."),
        sep(),
        keyed("Fitting Function Builder...", plain_key(Key::F8), Cmd::FitBuilder),
        keyed("Fitting Function Organizer...", plain_key(Key::F9), Cmd::FitOrganizer),
        keyed("Theme Organizer...", plain_key(Key::F7), Cmd::ThemeOrganizer),
        item("Import Filters Manager...", Cmd::FilterManager),
        grey("Package Manager..."),
        grey("Custom Menu Organizer..."),
        grey("Virtual Matrix Manager..."),
        grey_keyed("X-Function Builder...", plain_key(Key::F10)),
        grey_keyed("X-Function Script Samples...", plain_key(Key::F11)),
        sep(),
        grey("R Console..."),
        grey("Rserve Console..."),
        grey("Python Console..."),
    ]
}

fn format_menu(kind: WindowKind) -> Vec<Entry> {
    match kind {
        WindowKind::Graph => vec![
            grey("Menu"),
            keyed("&Page Properties...", plain_key(Key::F2), Cmd::GraphProps(PropsTab::Page)),
            item("&Layer Properties...", Cmd::GraphProps(PropsTab::Layer)),
            item("Plot P&roperties...", Cmd::GraphProps(PropsTab::Plots)),
            sep(),
            grey("Snap Layers to Grid"),
            grey("Snap Objects to Grid"),
            item("Fit Page To Layers...", Cmd::FitPageToLayers),
            item("Fit Page To Printer", Cmd::FitPageToPrinter),
            sep(),
            item("&Axes...", Cmd::GraphProps(PropsTab::XAxis)),
            sub(
                "Axe&s",
                vec![
                    item("&X Axis...", Cmd::GraphProps(PropsTab::XAxis)),
                    item("&Y Axis...", Cmd::GraphProps(PropsTab::YAxis)),
                    item("&Z Axis...", Cmd::ZAxis),
                ],
            ),
            sub(
                "Axis Tick Labels",
                vec![
                    item("X Axis Tick Labels...", Cmd::GraphProps(PropsTab::XTickLabels)),
                    item("Y Axis Tick Labels...", Cmd::GraphProps(PropsTab::YTickLabels)),
                ],
            ),
            sub(
                "Axis Titles",
                vec![
                    item("X Axis Title...", Cmd::EditTitle { layer: None, vertical: false }),
                    item("Y Axis Title...", Cmd::EditTitle { layer: None, vertical: true }),
                ],
            ),
            sep(),
            grey("Programming Control..."),
            item("Object Properties...", Cmd::ObjectProperties),
        ],
        _ => vec![
            grey("Menu"),
            keyed("&Worksheet...", plain_key(Key::F4), Cmd::WorksheetFonts),
            item("&Column...", Cmd::ColumnProperties),
            item("Cell...", Cmd::FormatCells),
            sep(),
            grey("Snap to Grid"),
            grey_keyed("Merge cells", ctrl(Key::R)),
            sep(),
            grey("Programming Control..."),
            grey("Object Properties..."),
        ],
    }
}

fn window_menu(kind: WindowKind) -> Vec<Entry> {
    let mut v = vec![
        item("Cas&cade", Cmd::Cascade),
        item("Tile &Horizontally", Cmd::TileHorizontally),
        item("&Tile Vertically", Cmd::TileVertically),
        item("Arrange &Icons", Cmd::ArrangeIcons),
        sep(),
        keyed("&Refresh", plain_key(Key::F5), Cmd::Refresh),
        sep(),
        item("&Duplicate", Cmd::DuplicateWindow),
    ];
    match kind {
        WindowKind::Graph => v.push(sub(
            "Duplicate (&Batch Plotting)",
            vec![
                item("Duplicate with New Sheets/Books...", Cmd::BatchPlot(false)),
                item("Duplicate with New Columns...", Cmd::BatchPlot(true)),
            ],
        )),
        WindowKind::Workbook => v.push(grey("Sp&lit")),
        WindowKind::None | WindowKind::Matrix | WindowKind::Layout => {}
    }
    v.extend([
        sep(),
        keyed("&Properties...", alt(Key::Enter), Cmd::WindowProperties),
        sep(),
        grey_keyed("C&ommand Window", alt(Key::Num3)),
        Entry::Item(Item {
            label: "&Script Window".to_string(),
            keys: Some(KeyboardShortcut::new(Modifiers::ALT | Modifiers::SHIFT, Key::Num3)),
            cmd: None,
        }),
        sep(),
        sub("&Folders", vec![Entry::Folders]),
        sep(),
        keyed("Window S&witcher", KeyboardShortcut::new(Modifiers::CTRL, Key::Tab), Cmd::WindowSwitcher),
        sep(),
        Entry::Windows,
    ]);
    v
}

fn help_menu() -> Vec<Entry> {
    vec![
        grey("Use Online Help"),
        grey_sub("Origin", &["Help", "X-Functions", "Programming"]),
        grey_sub("Programming", &["LabTalk", "Origin C", "Python", "Automation Server"]),
        sub("Open Sample Projects", vec![item("Sample Projects", Cmd::SampleProjects)]),
        grey_sub("Online Resources", &["Web Links"]),
        sep(),
        grey("Check for Updates..."),
        item(&format!("&About {APP_NAME}..."), Cmd::About),
    ]
}

// ----- the right-click menus of the worksheet ------------------------------------------------
//
// The lists of Origin's context menus (`docs/spec/gui-look.md`, section 3), drawn with the same
// code as the menu bar so that greyed items, tooltips and shortcut texts look the same. Origin
// shows a short form with a chevron; Sheaf shows the long form (a deliberate difference, as for
// the menu bar).

fn copy_entries() -> Vec<Entry> {
    vec![
        keyed("Copy", ctrl(Key::C), Cmd::Copy(CopyKind::Plain)),
        keyed("Copy (full precision)", ctrl_alt(Key::C), Cmd::Copy(CopyKind::FullPrecision)),
        item("Copy including label rows", Cmd::Copy(CopyKind::WithLabels)),
    ]
}

/// The menu of a column header (the column is selected first).
pub fn column_header_menu() -> Vec<Entry> {
    let order = ["Ascending", "Descending", "Custom..."];
    let set_as: Vec<Entry> = [
        ("X", Designation::X),
        ("Y", Designation::Y),
        ("Z", Designation::Z),
        ("Label", Designation::Label),
        ("Disregard", Designation::Disregard),
        ("Y Error", Designation::YError),
        ("X Error", Designation::XError),
        ("Group", Designation::Group),
        ("Subject", Designation::Subject),
    ]
    .iter()
    .map(|(l, d)| item(l, Cmd::SetAs(*d)))
    .collect();
    let plots: Vec<Entry> = Template::ALL.iter().map(|t| item(t.label(), Cmd::Plot(*t))).collect();
    let mut v = vec![sub("Plot", plots), sep(), keyed("Cut", ctrl(Key::X), Cmd::Cut), sub("Copy", copy_entries())];
    v.push(item("Copy Columns to...", Cmd::CopyColumnsTo));
    v.push(keyed("Paste", ctrl(Key::V), Cmd::Paste));
    v.push(sep());
    v.push(item("Insert", Cmd::InsertCells));
    v.push(item("Delete", Cmd::DeleteColumns));
    v.push(keyed("Clear", plain_key(Key::Delete), Cmd::Clear));
    v.push(item("Remove Links", Cmd::RemoveLinks));
    v.push(sep());
    v.push(sub("Set As", set_as));
    v.push(item("Set As Categorical", Cmd::SetCategorical));
    v.push(sep());
    v.push(keyed("Set Column Values...", ctrl(Key::Q), Cmd::SetColumnValues));
    v.push(keyed("Set Multiple Columns Values...", ctrl_shift(Key::Q), Cmd::SetMultipleColumnsValues));
    v.push(item("Recalculate", Cmd::Recalculate));
    v.push(item("Add New Columns...", Cmd::AddColumns));
    v.push(sep());
    v.push(sub(
        "Fill Column with",
        vec![
            item("Row Numbers", Cmd::QuickFill(QuickFill::RowNumbers)),
            item("Uniform Random Numbers", Cmd::QuickFill(QuickFill::Uniform)),
            item("Normal Random Numbers", Cmd::QuickFill(QuickFill::Normal)),
            item("A set of Numbers...", Cmd::FillPattern(FillKind::Numbers)),
            item("A set of Date/Time values...", Cmd::FillPattern(FillKind::Dates)),
            item("Arbitrary set of Text&&Numeric values...", Cmd::FillPattern(FillKind::Text)),
        ],
    ));
    v.push(item("Set Sampling Interval...", Cmd::SetSamplingInterval));
    v.push(item("Show X Column...", Cmd::ShowXColumn));
    v.push(sep());
    v.push(filter_menu());
    v.push(mask_menu());
    v.push(item("Mask Cells by Condition...", Cmd::MaskByCondition));
    v.push(sep());
    v.push(sub(
        "Sort Column",
        vec![
            item(order[0], Cmd::SortColumns { descending: false }),
            item(order[1], Cmd::SortColumns { descending: true }),
            item(order[2], Cmd::SortCustom { range: true }),
        ],
    ));
    v.push(sub(
        "Sort Worksheet",
        vec![
            item("Ascending", Cmd::SortAscending),
            item("Descending", Cmd::SortDescending),
            item("Custom...", Cmd::SortCustom { range: false }),
        ],
    ));
    v.push(item("Sort Columns by Label...", Cmd::SortByLabel));
    v.push(item("Reverse Order", Cmd::ReverseOrder));
    v.push(sep());
    v.push(item("Normalize...", Cmd::Analyze(AnalysisKind::Normalize)));
    v.push(item("Frequency Count...", Cmd::Analyze(AnalysisKind::Histogram)));
    v.push(item("Statistics on Columns...", Cmd::Analyze(AnalysisKind::Statistics)));
    v.push(sep());
    v.push(hide_menu());
    v.push(sub(
        "Move Columns",
        vec![
            item("To First", Cmd::MoveColumns(MoveTo::First)),
            item("Left", Cmd::MoveColumns(MoveTo::Left)),
            item("Right", Cmd::MoveColumns(MoveTo::Right)),
            item("To Last", Cmd::MoveColumns(MoveTo::Last)),
        ],
    ));
    v.push(item("Swap Columns...", Cmd::SwapColumns));
    v.push(item("Column Width...", Cmd::ColumnProperties));
    v.push(sep());
    v.push(item("Slide Show of Dependent Graphs", Cmd::SlideShow));
    v.push(item("Add or Update Sparklines...", Cmd::AddSparklines));
    v.push(sep());
    v.push(item("Format Cells...", Cmd::FormatCells));
    v.push(item("Properties...", Cmd::ColumnProperties));
    v
}

/// Metadata cells have clipboard commands, without structural operations on worksheet data.
pub fn label_cell_menu(read_only: bool) -> Vec<Entry> {
    if read_only {
        return vec![sub("Copy", copy_entries())];
    }
    vec![
        keyed("Cut", ctrl(Key::X), Cmd::Cut),
        sub("Copy", copy_entries()),
        keyed("Paste", ctrl(Key::V), Cmd::Paste),
        keyed("Clear", plain_key(Key::Delete), Cmd::Clear),
    ]
}

/// The menu of a data cell; `noted` when the cell has a note (Edit/Delete Note instead of Add);
/// `cell` is the right-clicked cell, `graph` whether it shows a graph (T194) and `sparkline` whether it
/// draws a sparkline (T200); `image` whether it shows a picture (T202); `arrow` whether it has an arrow
/// (T205); `rich_text` and `float` whether the cell's Set Data Style > Rich Text (T208) and Float (T214)
/// are on.
pub fn cell_menu_for(
    noted: bool,
    cell: (ColumnId, usize),
    graph: bool,
    sparkline: bool,
    image: bool,
    arrow: bool,
    (rich_text, float): (bool, bool),
) -> Vec<Entry> {
    let (column, row) = cell;
    let mut v = vec![keyed("Cut", ctrl(Key::X), Cmd::Cut)];
    v.extend(copy_entries());
    v.push(keyed("Paste", ctrl(Key::V), Cmd::Paste));
    v.push(item("Paste Transpose", Cmd::PasteTranspose));
    v.push(item("Paste Link", Cmd::PasteLink(false)));
    v.push(item("Paste Link Transpose", Cmd::PasteLink(true)));
    v.push(sep());
    v.push(item("Insert", Cmd::InsertCells));
    v.push(item("Delete", Cmd::DeleteCells));
    v.push(keyed("Clear", plain_key(Key::Delete), Cmd::Clear));
    v.push(item("Remove Links", Cmd::RemoveLinks));
    v.push(sep());
    v.push(item("Set as Begin", Cmd::SetRangeEnd(RangeEnd::Begin)));
    v.push(item("Set as End", Cmd::SetRangeEnd(RangeEnd::End)));
    v.push(sep());
    v.push(sub(
        "Insert Arrow",
        vec![
            item("Arrow", Cmd::SetCellArrow { column, row, double: Some(false) }),
            item("Double Arrow", Cmd::SetCellArrow { column, row, double: Some(true) }),
        ],
    ));
    if arrow {
        v.push(item("Remove Arrow", Cmd::SetCellArrow { column, row, double: None }));
    }
    v.push(item("Insert Graph...", Cmd::InsertCellGraph { column, row }));
    if graph {
        v.push(item("Remove Graph", Cmd::RemoveCellGraph { column, row }));
    }
    v.push(item("Insert Images from Files...", Cmd::InsertCellImage { column, row }));
    if image {
        v.push(item("Remove Image", Cmd::RemoveCellImage { column, row }));
    }
    v.push(item("Insert Sparklines...", Cmd::InsertCellSparkline { column, row }));
    if sparkline {
        v.push(item("Remove Sparkline", Cmd::RemoveCellSparkline { column, row }));
    }
    v.extend(greys(&["Insert Variables...", "-"]));
    v.push(item("Format Cells...", Cmd::FormatCells));
    if noted {
        v.push(item("Edit Note...", Cmd::EditNote));
        v.push(item("Delete Note", Cmd::DeleteNote));
    } else {
        v.push(item("Add Note...", Cmd::EditNote));
    }
    v.push(mask_menu());
    v.push(cond_format_menu());
    v.push(sub(
        "Set Data Style",
        vec![
            item(if rich_text { "\u{2713} Rich Text" } else { "Rich Text" }, Cmd::SetRichText(!rich_text)),
            grey("Wrap Text"),
            item(if float { "\u{2713} Float" } else { "Float" }, Cmd::SetFloat(!float)),
            item("More...", Cmd::FormatCells),
        ],
    ));
    v
}

/// The menu of a row number: Origin's short form, the same items as the cell menu starts with.
/// `first` is the row's first visible cell, where Insert Graph... puts the graph (T205).
pub fn row_menu(first: Option<(ColumnId, usize)>) -> Vec<Entry> {
    let mut v = vec![keyed("Cut", ctrl(Key::X), Cmd::Cut), keyed("Copy", ctrl(Key::C), Cmd::Copy(CopyKind::Plain))];
    v.push(keyed("Paste", ctrl(Key::V), Cmd::Paste));
    v.push(item("Insert", Cmd::InsertCells));
    v.push(item("Delete", Cmd::DeleteCells));
    v.push(keyed("Clear", plain_key(Key::Delete), Cmd::Clear));
    v.push(match first {
        Some((column, row)) => item("Insert Graph...", Cmd::InsertCellGraph { column, row }),
        None => grey("Insert Graph..."),
    });
    v.push(item("Format Cells...", Cmd::FormatCells));
    v
}

/// The menu of a sheet tab (it acts on the sheet whose tab was clicked, which becomes the
/// active one first).
pub fn sheet_tab_menu(sheet: &sheaf_core::Worksheet) -> Vec<Entry> {
    let names = sheaf_core::sheet_tab::SheetLabel::ALL
        .iter()
        .map(|l| {
            let tick = if *l == sheet.tab_label { "\u{2713} " } else { "" };
            item(&format!("{tick}{}", l.name()), Cmd::Sheet(SheetOp::ShowName(*l)))
        })
        .collect();
    let mut menu = vec![
        item("Insert", Cmd::Sheet(SheetOp::Insert)),
        item("Delete", Cmd::Sheet(SheetOp::Delete)),
        item("Add", Cmd::Sheet(SheetOp::Add)),
        item("Add Graph as Sheet...", Cmd::Sheet(SheetOp::AddGraph)),
        item("Add Notes as Sheet", Cmd::Sheet(SheetOp::AddNotes)),
        sep(),
        item("Duplicate Without Data", Cmd::Sheet(SheetOp::DuplicateWithoutData)),
        item("Duplicate", Cmd::Sheet(SheetOp::Duplicate)),
        sep(),
        sub("Sheet Name to Show", names),
        item("Name and Comments...", Cmd::Sheet(SheetOp::RenameDialog)),
        sep(),
        item("Navigate...", Cmd::Sheet(SheetOp::Navigate)),
        item("Activate This Sheet in All Books in Folder", Cmd::Sheet(SheetOp::ActivateInFolder)),
    ];
    if !sheet.view.column_header || !sheet.view.row_header {
        use sheaf_core::sheet_view::ViewSwitch as V;
        menu.push(sub("View", vec![sheet_view_entry(sheet, V::ColumnHeader), sheet_view_entry(sheet, V::RowHeader)]));
    }
    menu
}

// ----- graph window menus (gui-look.md section 4) ----------------------------------------------

/// What Change X, Change Y and Ungroup of a plot's menu need: the columns of the plot's sheet
/// (id and the label `[Book1]Sheet1!A(X)`), the plot's own columns and whether it is in a group.
#[derive(Clone, Default)]
pub struct PlotColumns {
    pub list: Vec<(ColumnId, String)>,
    pub x: Option<ColumnId>,
    pub y: Option<ColumnId>,
    pub grouped: bool,
}

/// The menu of a plot: what was picked, so the commands act on it. `book` is the workbook the
/// plot's data is in, with its title.
pub fn graph_plot_menu(
    layer: usize,
    plot: usize,
    kind: PlotKind,
    on_right: bool,
    book: Option<(WindowId, String)>,
    trendlines: usize,
    layers: usize,
    cols: &PlotColumns,
) -> Vec<Entry> {
    // Change X / Change Y: the sheet's columns, the plot's own ticked, the one it uses on the other
    // axis greyed; a plot of a group offers none.
    let change_col = |label: &str, y: bool| {
        if cols.grouped || cols.list.is_empty() {
            return grey(label);
        }
        let (own, other) = if y { (cols.y, cols.x) } else { (cols.x, cols.y) };
        let entries = cols
            .list
            .iter()
            .map(|(id, name)| {
                let tick = if own == Some(*id) { "\u{2713} " } else { "" };
                let text = format!("{tick}{name}");
                if other == Some(*id) {
                    grey(&text)
                } else {
                    item(&text, Cmd::GraphEdit(GraphOp::SetPlotColumn { layer, plot, y, column: *id }))
                }
            })
            .collect();
        sub(label, entries)
    };
    let change = |label: &str, to: PlotKind| {
        if to == kind {
            grey(label)
        } else {
            item(label, Cmd::GraphEdit(GraphOp::SetPlotKind { layer, plot, kind: to }))
        }
    };
    let mut v = vec![
        sub(
            "Change Plot to",
            vec![
                change("Line", PlotKind::Line),
                change("Scatter", PlotKind::Scatter),
                change("Line + Symbol", PlotKind::LineSymbol),
                change("Column", PlotKind::Column),
                change("Area", PlotKind::Area),
                change("Stacked Area", PlotKind::StackedArea),
            ],
        ),
        change_col("Change X", false),
        change_col("Change Y", true),
        sep(),
        item(
            if on_right { "Move to Left Y Axis" } else { "Move to Right Y Axis" },
            Cmd::GraphEdit(GraphOp::MovePlotToAxis { layer, plot, right: !on_right }),
        ),
        item("Hide Data Plot", Cmd::GraphEdit(GraphOp::HidePlot { layer, plot })),
        sep(),
        item("Add Trendline...", Cmd::Trendline { layer, plot, edit: false }),
    ];
    if layers > 1 {
        // The plot takes the axes of the layer it goes to (the extra Y axes of a multi-Y graph, T61).
        let to_layer = (0..layers)
            .filter(|&to| to != layer)
            .map(|to| item(&format!("Layer {}", to + 1), Cmd::GraphEdit(GraphOp::MovePlotToLayer { layer, plot, to })))
            .collect();
        v.insert(5, sub("Move to Layer", to_layer));
    }
    if trendlines > 0 {
        v.push(item("Edit Trendline...", Cmd::Trendline { layer, plot, edit: true }));
        v.push(item("Remove Trendline", Cmd::GraphEdit(GraphOp::RemoveTrendline { layer, plot })));
    }
    match book {
        Some((id, name)) => v.push(item(&format!("Go to {name}"), Cmd::GoTo(id))),
        None => v.push(grey("Go to Book")),
    }
    v.extend([
        sep(),
        if cols.grouped { item("Ungroup", Cmd::GraphEdit(GraphOp::Ungroup { layer, plot })) } else { grey("Ungroup") },
        sep(),
        item("Plot Details...", Cmd::GraphPropsAt { layer, tab: PropsTab::Plots, plot: Some(plot) }),
    ]);
    v
}

/// A Copy Format submenu of a graph: an item per choice; `None` is a choice Sheaf has not measured
/// and keeps greyed. A "-" label is a separator.
fn graph_copy_format_menu(edit: bool, choices: &[(&str, Option<GraphPart>)]) -> Entry {
    sub(
        "Copy Format",
        choices
            .iter()
            .map(|(label, part)| match (*label, part) {
                ("-", _) => sep(),
                (_, Some(part)) if edit => item(label, Cmd::EditCopyFormat(*part)),
                (_, Some(part)) => item(label, Cmd::CopyGraphFormat(*part)),
                (_, None) => grey(label),
            })
            .collect(),
    )
}

/// The menu of an axis. `grids` tells whether the axis shows grid lines now.
/// `page` is the page point that was right-clicked (Center puts the value there in the middle).
pub fn graph_axis_menu(layer: usize, y: bool, grids: bool, page: (f64, f64)) -> Vec<Entry> {
    let tab = if y { PropsTab::YAxis } else { PropsTab::XAxis };
    let at = Cmd::GraphPropsAt { layer, tab, plot: None };
    let labels_tab = if y { PropsTab::YTickLabels } else { PropsTab::XTickLabels };
    let labels = Cmd::GraphPropsAt { layer, tab: labels_tab, plot: None };
    let mut menu = vec![
        item("Hide Axis", Cmd::GraphEdit(GraphOp::HideAxis { layer, y })),
        item(if grids { "Hide Grids" } else { "Show Grids" }, Cmd::GraphEdit(GraphOp::ToggleGrids { layer, y })),
        sep(),
        keyed("Rescale To Show All", ctrl(Key::R), Cmd::GraphEdit(GraphOp::Rescale { layer })),
        item("Axis Zoom In", Cmd::GraphEdit(GraphOp::AxisZoom { layer, y, zoom_in: true })),
        item("Axis Zoom Out", Cmd::GraphEdit(GraphOp::AxisZoom { layer, y, zoom_in: false })),
        item("Center", Cmd::GraphEdit(GraphOp::AxisCenter { layer, y, page })),
        sep(),
        axis_copy_format_menu(layer, y),
        item("Save Format as Theme...", Cmd::SaveAxisTheme { layer, y }),
        sep(),
        item("Scale...", at.clone()),
        item("Tick Labels...", labels),
        item("Properties...", at),
    ];
    // Paste Format To shows once an axis format is on the clipboard (Origin, spec 24).
    if crate::format_clip_ui::kind() == Some(crate::format_clip_ui::ClipKind::Axis) {
        menu.insert(9, axis_paste_format_menu(layer, y));
    }
    menu
}

/// Copy Format of an axis: Scales reuses the existing scale-only group (spec 26.9).
/// Styles and Dimensions stay greyed; All takes styles and scale (measured).
fn axis_copy_format_menu(layer: usize, y: bool) -> Entry {
    sub(
        "Copy Format",
        vec![
            item("Styles", Cmd::CopyAxisFormat { layer, y, part: AxisPart::Styles }),
            item("Scales", Cmd::CopyAxisFormat { layer, y, part: AxisPart::Scales }),
            grey("Dimensions"),
            sep(),
            item("All", Cmd::CopyAxisFormat { layer, y, part: AxisPart::All }),
        ],
    )
}

/// Paste Format To of an axis: existing All, Scales and Styles groups (spec 26.9/26.11).
fn axis_paste_format_menu(layer: usize, y: bool) -> Entry {
    sub("Paste Format To", axis_paste_groups(|part| Cmd::PasteAxisFormat { layer, y, part }))
}

/// The groups of an axis format's Paste Format To, each pasted by the command `make` gives.
fn axis_paste_groups(make: impl Fn(AxisPart) -> Cmd) -> Vec<Entry> {
    let paste = |label: &str, part: AxisPart| item(label, make(part));
    let mut v = vec![
        paste("Tick Direction", AxisPart::TickDirection),
        paste("Ticks", AxisPart::Ticks),
        paste("Line", AxisPart::Line),
        paste("Tick Labels", AxisPart::TickLabels),
        paste("Title Styles", AxisPart::TitleStyles),
        paste("Colors", AxisPart::Colors),
        paste("Thickness", AxisPart::Thickness),
        paste("Styles", AxisPart::Styles),
        sep(),
        paste("Scales", AxisPart::Scales),
        sep(),
    ];
    // Title Dimensions: Sheaf keeps no title position or size of its own on an axis yet.
    v.push(grey("Title Dimensions"));
    v.push(paste("Title Text", AxisPart::TitleText));
    v.push(sep());
    v.push(paste("All", AxisPart::All));
    v
}

/// The menu of an axis title.
pub fn graph_title_menu(layer: usize, y: bool) -> Vec<Entry> {
    let tab = if y { PropsTab::YAxis } else { PropsTab::XAxis };
    vec![
        item("Edit Text...", Cmd::EditTitle { layer: Some(layer), vertical: y }),
        keyed("Cut", ctrl(Key::X), Cmd::CopyTitle { layer, y, cut: true }),
        keyed("Copy", ctrl(Key::C), Cmd::CopyTitle { layer, y, cut: false }),
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::HideTitle { layer, y })),
        sep(),
        item("Bring to the front", Cmd::GraphEdit(GraphOp::OrderTitle { layer, y, front: true })),
        item("Push to the back", Cmd::GraphEdit(GraphOp::OrderTitle { layer, y, front: false })),
        sep(),
        item("Properties...", Cmd::GraphPropsAt { layer, tab, plot: None }),
    ]
}

/// The menu of a free text label.
pub fn graph_label_menu(layer: usize, label: usize) -> Vec<Entry> {
    vec![
        keyed("Cut", ctrl(Key::X), Cmd::CutLabel { layer, label }),
        keyed("Copy", ctrl(Key::C), Cmd::CopyLabel { layer, label }),
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::DeleteLabel { layer, label })),
        sep(),
        item("Bring to the front", Cmd::GraphEdit(GraphOp::OrderLabel { layer, label, front: true })),
        item("Push to the back", Cmd::GraphEdit(GraphOp::OrderLabel { layer, label, front: false })),
        sep(),
        item("Properties...", Cmd::EditText { layer, label }),
    ]
}

/// The menu of a drawn line, arrow, rectangle or ellipse.
pub fn graph_shape_menu(layer: usize, shape: usize, region: bool) -> Vec<Entry> {
    let mut entries = vec![
        keyed("Cut", ctrl(Key::X), Cmd::CutShape { layer, shape }),
        keyed("Copy", ctrl(Key::C), Cmd::CopyShape { layer, shape }),
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::DeleteShape { layer, shape })),
        sep(),
        item("Bring to the front", Cmd::GraphEdit(GraphOp::OrderShape { layer, shape, front: true })),
        item("Push to the back", Cmd::GraphEdit(GraphOp::OrderShape { layer, shape, front: false })),
        sep(),
    ];
    // A rectangle or an ellipse marks a region whose data points can be copied out (T355).
    if region {
        entries.push(item("Extract Data in Region", Cmd::ExtractRegion { layer, shape }));
        entries.push(sep());
    }
    entries.push(item("Properties...", Cmd::EditShape { layer, shape }));
    entries
}

/// The menu of a slope annotation.
pub fn graph_slope_menu(layer: usize, slope: usize) -> Vec<Entry> {
    vec![
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::DeleteSlope { layer, slope })),
        sep(),
        item("Properties...", Cmd::EditSlope { layer, slope }),
    ]
}

/// The menu of a straight line.
pub fn graph_straight_line_menu(layer: usize, line: usize) -> Vec<Entry> {
    vec![
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::DeleteStraightLine { layer, line })),
        sep(),
        item("Properties...", Cmd::EditStraightLine { layer, line }),
    ]
}

/// The menu of a colour scale or bubble scale.
pub fn graph_scale_menu(layer: usize, scale: usize) -> Vec<Entry> {
    vec![keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::DeleteScale { layer, scale }))]
}

/// The menu of the legend.
///
/// Add Color Scale and Add Bubble Scale are enabled when the layer has a plot of that mapping.
pub fn graph_legend_menu(layer: usize, can_color: bool, can_bubble: bool) -> Vec<Entry> {
    use sheaf_core::layer_scale::LayerScaleKind;
    let add = |label: &str, can: bool, kind| {
        if can { item(label, Cmd::GraphEdit(GraphOp::AddScale { layer, kind })) } else { grey(label) }
    };
    let mut menu = vec![
        item("Edit Text...", Cmd::EditLegendText { layer: Some(layer) }),
        item("Plot Details...", Cmd::GraphPropsAt { layer, tab: PropsTab::Plots, plot: None }),
        sub(
            "Legend",
            vec![
                item("Reconstruct Legend", Cmd::GraphEdit(GraphOp::ReconstructLegend { layer })),
                item("Update Legend", Cmd::GraphEdit(GraphOp::UpdateLegend { layer })),
                grey("Add Legend Title"),
            ],
        ),
        add("Add Color Scale", can_color, LayerScaleKind::Color),
        add("Add Bubble Scale", can_bubble, LayerScaleKind::Bubble),
        sep(),
        keyed("Cut", ctrl(Key::X), Cmd::CopyLegend { layer, cut: true }),
        keyed("Copy", ctrl(Key::C), Cmd::CopyLegend { layer, cut: false }),
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::HideLegend { layer })),
        sep(),
        item("Bring to the front", Cmd::GraphEdit(GraphOp::OrderLegend { layer, front: true })),
        item("Push to the back", Cmd::GraphEdit(GraphOp::OrderLegend { layer, front: false })),
        item("Bring to front of data", Cmd::GraphEdit(GraphOp::LegendBehindData { layer, behind: false })),
        item("Push to back of data", Cmd::GraphEdit(GraphOp::LegendBehindData { layer, behind: true })),
        sep(),
        grey("Programming Control..."),
        item("Properties...", Cmd::GraphPropsAt { layer, tab: PropsTab::Legend, plot: None }),
        sep(),
        sub(
            "Copy Format",
            vec![
                item("Styles", Cmd::CopyLegendStyles { layer }),
                item("Dimensions", Cmd::CopyLegendFormat { layer, part: LegendPart::Dimensions }),
                sep(),
                item("All", Cmd::CopyLegendFormat { layer, part: LegendPart::All }),
            ],
        ),
    ];
    if let Some((styles, dimensions)) = crate::format_clip_ui::legend_parts() {
        // Each entry pastes what the copied legend format holds (T179).
        let when = |held: bool, label: &str, cmd: Cmd| if held { item(label, cmd) } else { grey(label) };
        menu.push(sub(
            "Paste Format To",
            vec![
                when(styles, "Styles", Cmd::PasteLegendStyles { layer }),
                when(dimensions, "Dimensions", Cmd::PasteLegendFormat { layer, part: LegendPart::Dimensions }),
                sep(),
                item("All", Cmd::PasteLegendFormat { layer, part: LegendPart::All }),
            ],
        ));
    }
    menu.push(item("Save Format as Theme...", Cmd::SaveLegendTheme { layer }));
    menu
}

/// Graph > New Layer(Axes): the nine kinds and the preset facade of the `layadd` window.
fn new_layer_menu() -> Entry {
    use sheaf_core::layers::NewLayerKind;
    let mut entries: Vec<Entry> =
        NewLayerKind::ALL.iter().map(|&kind| item(kind.menu_label(), Cmd::AddLayer(kind))).collect();
    entries.push(sep());
    entries.push(item("Open Dialog...", Cmd::AddLayerDialog));
    sub("New Layer(Axes)", entries)
}

/// The menu of the page outside the plots and axes, or of the inside of a layer.
/// The menu of the page or of a layer's empty area; `page` is the clicked page point (Paste puts a
/// copied drawn object there, T125).
pub fn graph_page_menu(layer: usize, page: (f64, f64)) -> Vec<Entry> {
    let paste = crate::shape_clip::has().then(|| item("Paste", Cmd::PasteShape { layer, page }));
    let mut v: Vec<Entry> = paste.into_iter().collect();
    v.extend([
        item("Add Text...", Cmd::AddText),
        item("Insert Images From Files...", Cmd::InsertGraphImage { layer, page }),
        item("Layer Management...", Cmd::LayerManagement(None)),
        new_layer_menu(),
        item("Add Shortcut to Favorites", Cmd::AddFavorite),
        keyed("Copy Page", ctrl(Key::J), Cmd::CopyPage),
        item("Export Graphs...", Cmd::ExportGraph),
        item("Send Graphs to PowerPoint...", Cmd::SendGraphsToPowerPoint),
        keyed("View Full Screen", ctrl_shift(Key::J), Cmd::FullScreen),
        sep(),
        keyed("Rescale To Show All", ctrl(Key::R), Cmd::GraphEdit(GraphOp::Rescale { layer })),
        item("Page Properties...", Cmd::GraphPropsAt { layer, tab: PropsTab::Page, plot: None }),
        graph_copy_format_menu(
            false,
            &[
                ("Page Background", Some(GraphPart::PageBackground)),
                ("Legend Translation Mode", None),
                ("Dimensions", Some(GraphPart::Dimensions)),
                ("Scales", Some(GraphPart::Scales)),
                ("-", None),
                ("Colors", Some(GraphPart::Colors)),
                ("Fonts", Some(GraphPart::Fonts)),
                ("Background", Some(GraphPart::Background)),
                ("All Style Formats", Some(GraphPart::AllStyle)),
                ("-", None),
                ("All", Some(GraphPart::All)),
            ],
        ),
        page_paste_format_entry(),
        sub(
            "Duplicate (Batch Plotting)",
            vec![
                item("Duplicate", Cmd::DuplicateWindow),
                item("Duplicate with New Sheets/Books...", Cmd::BatchPlot(false)),
                item("Duplicate with New Columns...", Cmd::BatchPlot(true)),
            ],
        ),
    ]);
    v
}

/// Paste Format of the page menu: it acts at once for a graph format; for an axis format Origin
/// shows Paste Format To with Bottom Axes, All X Axes, All Axes and All; the first three paste the
/// axis groups onto those axes of every layer (T174, Sheaf's meanings), All stays greyed (Origin's
/// target is unmeasured and Sheaf cannot tell it from All Axes); with nothing copied it is greyed.
fn page_paste_format_entry() -> Entry {
    use crate::format_clip_ui::{ClipKind, kind};
    match kind() {
        Some(ClipKind::Graph) => item("Paste Format", Cmd::PasteGraphFormat),
        Some(ClipKind::Axis) => {
            use sheaf_core::format_clip::AxisTargets;
            let to = |label: &str, targets: AxisTargets| {
                sub(label, axis_paste_groups(|part| Cmd::PasteAxesFormat { targets, part }))
            };
            sub(
                "Paste Format To",
                vec![
                    to("Bottom Axes", AxisTargets::Bottom),
                    to("All X Axes", AxisTargets::AllX),
                    to("All Axes", AxisTargets::AllAxes),
                    grey("All"),
                ],
            )
        }
        Some(ClipKind::Legend) | None => grey("Paste Format"),
    }
}

/// The menu of a layer icon (the numbered button at the top left of the page). `plots` names the
/// layer's plots in order; `active` is the active plot, which gets a tick.
pub fn layer_icon_menu(layer: usize, layers: usize, plots: &[String], active: Option<usize>) -> Vec<Entry> {
    let others = if layers > 1 {
        item("Hide Other Layers", Cmd::GraphEdit(GraphOp::HideOtherLayers { layer }))
    } else {
        grey("Hide Other Layers")
    };
    let mut v = vec![
        item("Hide Layer", Cmd::GraphEdit(GraphOp::HideLayer { layer })),
        others,
        if layers > 1 {
            item("Show Active Layer Only", Cmd::ToggleActiveLayerOnly)
        } else {
            grey("Show Active Layer Only")
        },
        item("Set Active Layer By Layer Icon Only", Cmd::ToggleActivateByIconOnly),
        if layers > 1 {
            item("Delete Layer", Cmd::GraphEdit(GraphOp::DeleteLayer { layer }))
        } else {
            grey("Delete Layer")
        },
        sep(),
        keyed("Layer Contents...", plain_key(Key::F12), Cmd::LayerContents(Some(layer))),
        item("Plot Setup...", Cmd::PlotSetup(Some(layer))),
        item("Double click to open Plot Setup", Cmd::ToggleIconPlotSetup),
        sep(),
        item("Axis...", Cmd::GraphPropsAt { layer, tab: PropsTab::XAxis, plot: None }),
        item("Layer Properties...", Cmd::GraphPropsAt { layer, tab: PropsTab::Layer, plot: None }),
        item("Layer Management...", Cmd::LayerManagement(Some(layer))),
    ];
    if !plots.is_empty() {
        v.push(sep());
    }
    for (i, name) in plots.iter().enumerate() {
        let tick = if active == Some(i) { "\u{2713} " } else { "    " };
        v.push(item(&format!("{tick}{} {name}", i + 1), Cmd::SelectPlot { layer, plot: i }));
    }
    v
}

/// The menu of the corner above the row numbers: the switches of Origin 2016's corner View submenu
/// (T92), ticked when on.
pub fn corner_menu(sheet: &sheaf_core::Worksheet) -> Vec<Entry> {
    use sheaf_core::sheet_view::ViewSwitch as V;
    let switch = |s| sheet_view_entry(sheet, s);
    vec![
        switch(V::LongName),
        switch(V::Units),
        switch(V::Comments),
        grey("Parameters"),
        item("Add User Parameter Row...", Cmd::Sheet(SheetOp::AddParamRow)),
        switch(V::SamplingInterval),
        switch(V::Sparklines),
        grey("Filter"),
        switch(V::Formula),
        switch(V::Categories),
        sep(),
        switch(V::RowLabels),
        sep(),
        switch(V::ColumnHeader),
        switch(V::RowHeader),
        sep(),
        switch(V::ColumnGrid),
        switch(V::RowGrid),
        switch(V::PageBreaks),
    ]
}

fn sheet_view_entry(sheet: &sheaf_core::Worksheet, switch: sheaf_core::sheet_view::ViewSwitch) -> Entry {
    let tick = if sheet.shows_view_switch(switch) { "\u{2713} " } else { "" };
    item(&format!("{tick}{}", switch.name()), Cmd::Sheet(SheetOp::ToggleView(switch)))
}

/// Draws a list of entries into a context menu.
pub fn show_context(ui: &mut egui::Ui, entries: &[Entry], c: &MenuContext, actions: &mut Vec<Action>) {
    // Origin shows a short form with a chevron; the long list scrolls when the screen is low.
    let height = (ui.ctx().content_rect().height() - 40.0).max(200.0);
    egui::ScrollArea::vertical().max_height(height).min_scrolled_height(height).auto_shrink([true, true]).show(
        ui,
        |ui| {
            ui.set_min_width(200.0);
            // Close-set rows: the long lists of Origin's menus are about 17 px per item there too.
            ui.spacing_mut().interact_size.y = 16.0;
            ui.spacing_mut().item_spacing.y = 1.0;
            show_entries(ui, entries, c, actions, None);
        },
    );
}

// ----- checks and helpers on the table --------------------------------------------------------

/// Calls `f` for every item of the entries, submenus included.
pub(crate) fn walk<'a>(entries: &'a [Entry], f: &mut dyn FnMut(&'a Item)) {
    for e in entries {
        match e {
            Entry::Item(i) => f(i),
            Entry::Submenu(s) => walk(&s.entries, f),
            Entry::Separator
            | Entry::Windows
            | Entry::Folders
            | Entry::NewFromTemplates
            | Entry::Recent(_)
            | Entry::UserTemplates => {}
            Entry::Mru(_) | Entry::Exports => {}
        }
    }
}

/// Commands that Sheaf has but no menu names (because a kind of analysis or fit was added
/// after the table was written) are put at the end of Analysis > Data Manipulation and under
/// Fitting, so that they are never lost from the menus. A test makes sure the table itself
/// does name all of them.
fn place_unplaced(menus: &mut [Menu]) {
    let (analysis, fits) = unplaced(menus);
    let Some(menu) = menus.iter_mut().find(|m| m.name() == "Analysis") else { return };
    for e in &mut menu.entries {
        if let Entry::Submenu(s) = e {
            if s.label == "&Data Manipulation" {
                s.entries.extend(analysis.iter().map(|k| item(k.menu_label(), Cmd::Analyze(*k))));
            } else if s.label == "&Fitting" {
                for (k, label) in fits.iter() {
                    s.entries.push(item(label, Cmd::Fit(*k)));
                }
            }
        }
    }
}

/// The kinds of analysis and fit that the menus do not name.
fn unplaced(menus: &[Menu]) -> (Vec<AnalysisKind>, Vec<(FitKind, &'static str)>) {
    let mut cmds: Vec<&Cmd> = Vec::new();
    for m in menus {
        walk(&m.entries, &mut |i| cmds.extend(i.cmd.as_ref()));
    }
    let analysis = AnalysisKind::ALL.into_iter().filter(|k| !cmds.contains(&&Cmd::Analyze(*k))).collect();
    let fits = FitKind::MENU.into_iter().filter(|(k, _)| !cmds.contains(&&Cmd::Fit(*k))).collect();
    (analysis, fits)
}

/// Gives every item without an `&` the first letter of its label that no other item of the
/// same menu uses, so that every item has an Alt letter as in Origin. Letters already marked
/// by hand keep their place.
fn assign_mnemonics(entries: &mut [Entry]) {
    let mut used: Vec<char> = Vec::new();
    for e in entries.iter() {
        let label = match e {
            Entry::Item(i) => &i.label,
            Entry::Submenu(s) => &s.label,
            _ => continue,
        };
        used.extend(mnemonic(label));
    }
    for e in entries.iter_mut() {
        let label = match e {
            Entry::Item(i) => &mut i.label,
            Entry::Submenu(s) => {
                assign_mnemonics(&mut s.entries);
                &mut s.label
            }
            _ => continue,
        };
        if split(label).1.is_some() {
            continue;
        }
        let free = |c: char| c.is_alphanumeric() && !used.contains(&c.to_ascii_lowercase());
        let word_starts = label
            .char_indices()
            .filter(|(at, _)| *at == 0 || label[..*at].ends_with([' ', '/', '(']))
            .find(|(_, c)| free(*c));
        let pick = word_starts.or_else(|| label.char_indices().find(|(_, c)| free(*c)));
        if let Some((at, c)) = pick {
            used.push(c.to_ascii_lowercase());
            label.insert(at, '&');
        }
    }
}

/// Tells the menus whether the project has masking disabled (the Mask submenu then offers Enable
/// Masking); the program sets it every frame before it draws a menu.
pub fn set_masking_disabled(ctx: &egui::Context, disabled: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("masking_disabled"), disabled));
}

/// Tells the menus whether masked points are hidden (for the Hide/Show Masked Points label).
pub fn set_masked_hidden(ctx: &egui::Context, hidden: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("masked_hidden"), hidden));
}

fn masked_hidden(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp(egui::Id::new("masked_hidden"))).unwrap_or(false)
}

fn masking_disabled(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp(egui::Id::new("masking_disabled"))).unwrap_or(false)
}

/// The Alt letter of a label, lower case.
pub fn mnemonic(label: &str) -> Option<char> {
    parse(label).1.map(|c| c.to_ascii_lowercase())
}

/// The label without `&`.
pub fn plain(label: &str) -> String {
    parse(label).0
}

/// Splits a label into its text and the position of the Alt letter (in characters).
fn split(label: &str) -> (String, Option<usize>) {
    let mut text = String::new();
    let mut at = None;
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.peek() {
                Some('&') => {
                    chars.next();
                    text.push('&');
                }
                Some(_) if at.is_none() => at = Some(text.chars().count()),
                _ => {}
            }
        } else {
            text.push(c);
        }
    }
    (text, at)
}

fn parse(label: &str) -> (String, Option<char>) {
    let (text, at) = split(label);
    let letter = at.and_then(|k| text.chars().nth(k));
    (text, letter)
}

// ----- turning a command into an action -------------------------------------------------------

/// The action a command runs now, or the reason it cannot run.
pub fn resolve(cmd: &Cmd, c: &MenuContext) -> Result<Action, &'static str> {
    let book = c.book.ok_or(NEED_BOOK);
    let one = || match c.selected {
        [one] => Ok(*one),
        _ => Err(NEED_ONE_COLUMN),
    };
    let some = || if c.selected.is_empty() { Err(NEED_COLUMNS) } else { Ok(c.selected.to_vec()) };
    let sorting = || if c.sorting_columns.is_empty() { Err(NEED_COLUMNS) } else { Ok(c.sorting_columns.clone()) };
    Ok(match cmd {
        Cmd::NewProject => Action::NewProject,
        Cmd::NewGraph => Action::NewGraph,
        Cmd::SaveProjectNoData => Action::SaveProjectWithoutData,
        Cmd::DuplicateProjectNoData => Action::DuplicateProjectWithoutData,
        Cmd::NewWorkbook => Action::NewWorkbook,
        Cmd::OpenProject => Action::OpenProject,
        Cmd::SaveProject => Action::SaveProject,
        Cmd::SaveProjectAs => Action::SaveProjectAs,
        Cmd::SaveWindowAs => Action::SaveWindowAs(c.active.ok_or(NEED_WINDOW)?),
        Cmd::FillPattern(kind) => Action::FillPattern { kind: *kind, columns: some()? },
        Cmd::QuickFill(kind) => Action::QuickFill { kind: *kind, columns: some()? },
        Cmd::CutShape { layer, shape } => {
            Action::CopyShape { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, shape: *shape, cut: true }
        }
        Cmd::CopyShape { layer, shape } => {
            Action::CopyShape { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, shape: *shape, cut: false }
        }
        Cmd::NewNotes => Action::NewNotes,
        Cmd::NewMatrix => Action::NewMatrix,
        Cmd::NewLayout => Action::NewLayout,
        Cmd::LayoutPageSetup => match (c.kind, c.active) {
            (WindowKind::Layout, Some(id)) => Action::LayoutPageSetup { layout: id },
            _ => return Err("Activate a layout page first"),
        },
        Cmd::LayoutExport => match (c.kind, c.active) {
            (WindowKind::Layout, Some(id)) => Action::LayoutExport { layout: id },
            _ => return Err("Activate a layout page first"),
        },
        Cmd::LayoutAddGraph => match (c.kind, c.active) {
            (WindowKind::Layout, Some(id)) => Action::LayoutAddGraph { layout: id, sheets: false },
            _ => return Err("Activate a layout page first"),
        },
        Cmd::LayoutAddShape(shape, arrow) => match (c.kind, c.active) {
            (WindowKind::Layout, Some(id)) => Action::LayoutAddShape { layout: id, shape: *shape, arrow: *arrow },
            _ => return Err("Activate a layout page first"),
        },
        Cmd::LayoutAddText => match (c.kind, c.active) {
            (WindowKind::Layout, Some(id)) => Action::LayoutText { layout: id, index: None },
            _ => return Err("Activate a layout page first"),
        },
        Cmd::LayoutAddSheet => match (c.kind, c.active) {
            (WindowKind::Layout, Some(id)) => Action::LayoutAddGraph { layout: id, sheets: true },
            _ => return Err("Activate a layout page first"),
        },
        Cmd::ImportBinary2d => Action::ImportBinary2d,
        Cmd::ImportImage => Action::ImportImage,
        Cmd::NewFunctionPlot => Action::NewFunctionPlot,
        Cmd::Profile { contour } => match (c.kind, c.active, c.graph) {
            (WindowKind::Matrix, Some(id), _) => Action::Profile { book: Some(id), graph: None },
            (WindowKind::Graph, _, Some(g)) if *contour => Action::Profile { book: None, graph: Some(g) },
            _ if *contour => return Err("Activate a matrix window or a contour graph of a matrix first"),
            _ => return Err("Activate a matrix window first"),
        },
        Cmd::TernaryContour => {
            let [a, b, c3, z] = c.selected[..] else {
                return Err("Select the A, B and C composition columns and the Z column (four columns) first");
            };
            Action::TernaryContour { a, b, c: c3, z }
        }
        Cmd::PolarContour { theta_first } => {
            let [a, b, z] = c.selected[..] else {
                return Err("Select the X, Y and Z columns (three columns) first");
            };
            let (theta, r) = if *theta_first { (a, b) } else { (b, a) };
            Action::PolarContour { theta, r, z }
        }
        Cmd::ConvertToMatrix { xyz } => {
            let book = c.book.ok_or(NEED_BOOK)?;
            if *xyz && c.selected.len() != 3 {
                return Err("Select the X, Y and Z columns (three columns) first");
            }
            Action::ConvertToMatrix { book, columns: c.selected.to_vec(), xyz: *xyz }
        }
        Cmd::Image(cmd) => match (c.kind, c.active) {
            (WindowKind::Matrix, Some(id)) => Action::ImageCmd { id, cmd: *cmd },
            _ => return Err("Activate a matrix window first"),
        },
        Cmd::Matrix(cmd) => match (c.kind, c.active) {
            (WindowKind::Matrix, Some(id)) => Action::MatrixCmd { id, cmd: *cmd },
            _ => return Err("Activate a matrix window first"),
        },
        Cmd::PlotField(template) => match (c.kind, c.active) {
            (WindowKind::Matrix, Some(id)) => Action::PlotField { template: *template, matrix: Some(id), xyz: None },
            _ if c.book.is_some() && c.selected.len() >= 3 => Action::PlotField {
                template: *template,
                matrix: None,
                xyz: Some((c.selected[0], c.selected[1], c.selected[2])),
            },
            _ => return Err("Select the X, Y and Z columns (three columns), or activate a matrix window"),
        },
        Cmd::ObjectProperties => {
            let graph = c.graph.ok_or(NEED_GRAPH)?;
            c.graph_pick.ok_or(NEED_SELECTED_OBJECT)?;
            Action::ObjectProperties { graph }
        }
        Cmd::GraphClipboard(op) => {
            use sheaf_plot::scene::ObjectKind as K;
            let graph = c.graph.ok_or(NEED_GRAPH)?;
            let ok = match op {
                GraphClip::Cut | GraphClip::Copy => {
                    matches!(c.graph_pick, Some(K::Shape(_) | K::Label(_) | K::XTitle | K::YTitle | K::Legend))
                }
                GraphClip::Clear => matches!(
                    c.graph_pick,
                    Some(
                        K::Shape(_)
                            | K::Label(_)
                            | K::XTitle
                            | K::YTitle
                            | K::Legend
                            | K::Slope(_)
                            | K::StraightLine(_)
                            | K::Scale(_)
                    )
                ),
                GraphClip::Paste => c.graph_paste,
            };
            if !ok {
                return Err(match op {
                    GraphClip::Paste => NOTHING_COPIED,
                    _ => NEED_GRAPH_OBJECT,
                });
            }
            Action::GraphClipboard { graph, op: *op }
        }
        Cmd::CopyLegend { layer, cut } => {
            Action::CopyLegend { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, cut: *cut }
        }
        Cmd::CopyTitle { layer, y, cut } => {
            Action::CopyTitle { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, y: *y, cut: *cut }
        }
        Cmd::CutLabel { layer, label } => {
            Action::CopyLabel { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, label: *label, cut: true }
        }
        Cmd::CopyLabel { layer, label } => {
            Action::CopyLabel { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, label: *label, cut: false }
        }
        Cmd::InsertGraphImage { layer, page } => {
            Action::PickGraphImage { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, page: *page }
        }
        Cmd::PasteShape { layer, page } => {
            Action::PasteShape { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, page: *page }
        }
        Cmd::SetSamplingInterval => Action::SetSamplingInterval { columns: some()? },
        Cmd::ShowXColumn => Action::ShowXColumn { columns: some()? },
        Cmd::SaveTemplateAs => {
            if !c.templates {
                return Err(NO_TEMPLATE_FOLDER);
            }
            let window = match c.kind {
                WindowKind::Workbook => c.book,
                _ => c.graph,
            };
            Action::SaveTemplateAs(window.ok_or(NEED_GRAPH_OR_BOOK)?)
        }
        Cmd::SaveAnalysisTemplateAs => {
            if !c.templates {
                return Err(NO_TEMPLATE_FOLDER);
            }
            Action::SaveAnalysisTemplateAs(c.book.ok_or(NEED_BOOK)?)
        }
        Cmd::ManageTemplates => {
            if !c.templates {
                return Err(NO_TEMPLATE_FOLDER);
            }
            Action::ManageTemplates
        }
        Cmd::InsertCellGraph { column, row } => Action::PickCellGraph { column: *column, row: *row },
        Cmd::RemoveCellGraph { column, row } => Action::SetCellGraph { column: *column, row: *row, graph: None },
        Cmd::InsertCellSparkline { column, row } => Action::PickCellSparkline { column: *column, row: *row },
        Cmd::InsertCellImage { column, row } => Action::PickCellImage { column: *column, row: *row },
        Cmd::SetRichText(on) => Action::SetRichText { book: c.book.ok_or(NEED_BOOK)?, on: *on },
        Cmd::SetFloat(on) => Action::SetFloat { book: c.book.ok_or(NEED_BOOK)?, on: *on },
        Cmd::SetCellArrow { column, row, double } => {
            Action::SetCellArrow { column: *column, row: *row, double: *double }
        }
        Cmd::RemoveCellImage { column, row } => Action::SetCellImage { column: *column, row: *row, png: None },
        Cmd::RemoveCellSparkline { column, row } => {
            Action::SetCellSparkline { column: *column, row: *row, source: None }
        }
        Cmd::BatchProcessing => {
            if !c.templates {
                return Err(NO_TEMPLATE_FOLDER);
            }
            Action::BatchProcessing
        }
        Cmd::Import => Action::ImportPick(ImportPick::Single),
        Cmd::ImportMultiple => Action::ImportPick(ImportPick::Multiple),
        Cmd::ImportCsv => Action::ImportPick(ImportPick::Csv),
        Cmd::ImportWizard => Action::ImportWizard,
        Cmd::ImportExcel => Action::ImportPick(ImportPick::Excel),
        Cmd::GoToCell => Action::GoToCell(book?),
        Cmd::Power(kind) => Action::Power(*kind),
        Cmd::FindInSheets => Action::FindInSheets(book?),
        Cmd::FindInProject => Action::FindInProject,
        Cmd::FindInProjectFiles => Action::FindInProjectFiles,
        Cmd::ReplaceInSheets => Action::ReplaceInSheets(book?),
        Cmd::ExportGraph => Action::ExportGraph(c.graph.ok_or(NEED_GRAPH)?),
        Cmd::SendGraphsToPowerPoint => Action::SendGraphsToPowerPoint(c.graph.ok_or(NEED_GRAPH)?),
        Cmd::ExportProjectGraphs(what) => Action::ExportProjectGraphs(*what),
        Cmd::ExportWorksheet => Action::ExportData(book?),
        Cmd::ExportExcel => Action::ExportExcel(book?),
        Cmd::ExportWorksheetImage => Action::ExportWorksheetImage(book?),
        Cmd::ViewWindows(m) => Action::ViewWindows(*m),
        Cmd::SaveAxisTheme { layer, y } => {
            Action::SaveAxisTheme { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, y: *y }
        }
        Cmd::SaveLegendTheme { layer } => Action::SaveLegendTheme { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer },
        Cmd::ThemeOrganizer => Action::ThemeOrganizer,
        Cmd::NewFromTemplateFile => Action::NewFromTemplate(None),
        Cmd::FitOrganizer => Action::FitOrganizer,
        Cmd::FilterManager => Action::FilterManager,
        Cmd::SystemTemplates => Action::SystemTemplates { book: book?, columns: c.selected.to_vec() },
        Cmd::BatchPlot(columns) => Action::BatchPlot { graph: c.graph.ok_or(NEED_GRAPH)?, columns: *columns },
        Cmd::AddFunctionGraph => Action::AddFunctionGraph { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer },
        Cmd::Reimport(choose) => {
            let book = book?;
            c.imported.then_some(()).ok_or(NOT_IMPORTED)?;
            Action::Reimport { book, choose: *choose }
        }
        Cmd::Print => Action::Print(
            c.graph.or(c.book).or(c.active.filter(|_| c.kind == WindowKind::Layout)).ok_or(NEED_PRINTABLE)?,
        ),
        Cmd::PrintPreview => Action::PrintPreview(c.graph.or(c.book).ok_or(NEED_PRINTABLE)?),
        Cmd::PageSetup => Action::PageSetup,
        Cmd::Exit => Action::Quit,
        Cmd::Undo => {
            c.undo.ok_or(NO_UNDO)?;
            Action::Undo
        }
        Cmd::Redo => {
            c.redo.ok_or(NO_REDO)?;
            Action::Redo
        }
        Cmd::UndoAiChanges => {
            if !c.undo_ai {
                return Err(NO_AI_UNDO);
            }
            Action::UndoAiChanges
        }
        Cmd::DeleteColumns => {
            book?;
            Action::DeleteColumns(some()?)
        }
        Cmd::MoveColumns(to) => {
            book?;
            Action::MoveColumns { columns: some()?, to: *to }
        }
        Cmd::ResetFullRange => Action::ResetFullRange(book?),
        Cmd::CopyColumnsTo => {
            book?;
            Action::CopyColumnsTo(some()?)
        }
        Cmd::SwapColumns => {
            book?;
            match c.selected {
                [a, b] => Action::SwapColumns(*a, *b),
                _ => return Err(NEED_EXACTLY_TWO_COLUMNS),
            }
        }
        Cmd::HideColumns => {
            book?;
            Action::HideColumns(some()?)
        }
        Cmd::UnhideColumns => Action::UnhideColumns(book?),
        Cmd::Copy(kind) => {
            let book = book?;
            c.cells.then_some(()).ok_or(NEED_CELLS)?;
            // Origin greys these four unless whole columns are selected.
            if matches!(kind, CopyKind::Part(_)) && !c.whole_columns {
                return Err(NEED_WHOLE_COLUMNS);
            }
            Action::Cells { book, op: CellOp::Copy(*kind) }
        }
        Cmd::Cut
        | Cmd::Paste
        | Cmd::PasteTranspose
        | Cmd::PasteLink(_)
        | Cmd::RemoveLinks
        | Cmd::SetRangeEnd(_)
        | Cmd::Clear
        | Cmd::InsertCells
        | Cmd::DeleteCells
        | Cmd::Mask(_) => {
            let book = book?;
            c.cells.then_some(()).ok_or(NEED_CELLS)?;
            let op = match cmd {
                Cmd::Cut => CellOp::Cut,
                Cmd::Paste => CellOp::Paste,
                Cmd::PasteTranspose => CellOp::PasteTranspose,
                Cmd::PasteLink(transpose) => CellOp::PasteLink(*transpose),
                Cmd::RemoveLinks => CellOp::RemoveLinks,
                Cmd::SetRangeEnd(which) => CellOp::SetRangeEnd(*which),
                Cmd::InsertCells => CellOp::Insert,
                Cmd::Mask(m) => CellOp::Mask(*m),
                Cmd::DeleteCells => CellOp::Delete,
                _ => CellOp::Clear,
            };
            Action::Cells { book, op }
        }
        Cmd::MaskColor => Action::MaskColor,
        Cmd::ToggleMasking => Action::ToggleMasking,
        Cmd::ToggleHideMasked => Action::ToggleHideMasked,
        Cmd::MaskByCondition => {
            let book = book?;
            Action::MaskByConditionDialog { book, columns: some()? }
        }
        Cmd::Sheet(op) => Action::Sheet { book: book?, op: op.clone() },
        Cmd::Plot(template) => Action::PlotColumns { book: book?, columns: c.selected.to_vec(), template: *template },
        Cmd::GroupedHistogram => Action::GroupedHistogram { book: book?, columns: c.selected.to_vec() },
        Cmd::RunChart => Action::RunChart { book: book?, columns: c.selected.to_vec() },
        Cmd::SetAs(designation) => {
            book?;
            Action::SetDesignation { columns: some()?, designation: *designation }
        }
        Cmd::SetColumnValues => {
            book?;
            Action::SetColumnValues(one()?)
        }
        Cmd::SetMultipleColumnsValues => {
            book?;
            match c.selected {
                [_, _, ..] => Action::SetMultipleColumnsValues(c.selected.to_vec()),
                _ => return Err(NEED_TWO_COLUMNS),
            }
        }
        Cmd::Recalculate => {
            book?;
            Action::Recalculate(some()?)
        }
        Cmd::AddColumns => Action::AddColumnsDialog(book?),
        Cmd::AddColumnAcross => Action::AddColumnAcross(book?),
        Cmd::RepeatAnalysis => match (c.book, c.curve) {
            (Some(book), _) => Action::RepeatAnalysis { book, columns: c.selected.to_vec() },
            (None, Some((book, y))) if c.kind == WindowKind::Graph => Action::RepeatAnalysis { book, columns: vec![y] },
            (None, _) if c.kind == WindowKind::Graph => return Err(ANALYSIS_ON_BOOK),
            (None, _) => return Err(NEED_BOOK),
        },
        Cmd::AddOneColumn => Action::AddColumn(book?),
        Cmd::ColumnProperties => {
            book?;
            Action::ColumnProperties(one()?)
        }
        Cmd::SortAscending | Cmd::SortDescending => {
            book?;
            Action::SortSheet { column: one()?, descending: *cmd == Cmd::SortDescending }
        }
        Cmd::SortRange { descending } => {
            book?;
            Action::SortRange { columns: sorting()?, descending: *descending }
        }
        Cmd::SortColumns { descending } => {
            book?;
            Action::SortColumns { columns: sorting()?, descending: *descending }
        }
        Cmd::SortCustom { range } => {
            let book = book?;
            let columns = if *range { sorting()? } else { Vec::new() };
            Action::SortCustom { book, columns }
        }
        Cmd::SortByLabel => Action::SortByLabel(book?),
        Cmd::ClearWorksheet => Action::ClearWorksheet(book?),
        Cmd::SetCategorical => {
            book?;
            Action::SetCategorical(some()?)
        }
        Cmd::AddSparklines => {
            book?;
            Action::AddSparklines(some()?)
        }
        Cmd::Reshape(kind) => Action::Reshape { book: book?, columns: c.selected.to_vec(), kind: *kind },
        Cmd::PivotTable => Action::PivotTable { book: book?, columns: c.selected.to_vec() },
        Cmd::MultiPeakFit => match (c.book, c.curve) {
            (Some(_), _) => match c.selected {
                [y] => Action::MultiPeakFit { graph: None, y: *y, layer: 0 },
                _ => return Err("Select one Y column"),
            },
            (None, Some((_, y))) if c.kind == WindowKind::Graph => {
                Action::MultiPeakFit { graph: c.graph, y, layer: c.layer }
            }
            (None, _) if c.kind == WindowKind::Graph => return Err(ANALYSIS_ON_BOOK),
            (None, _) => return Err(NEED_BOOK),
        },
        Cmd::ReverseOrder => {
            book?;
            Action::ReverseColumns(some()?)
        }
        Cmd::PlotLineSeries => Action::PlotLineSeries { book: book?, columns: c.selected.to_vec() },
        Cmd::PlotYError => Action::PlotYError { book: book?, columns: c.selected.to_vec(), x: false },
        Cmd::PlotXYError => Action::PlotYError { book: book?, columns: c.selected.to_vec(), x: true },
        Cmd::PlotPanels { rows, cols } => {
            Action::PlotPanels { book: book?, columns: c.selected.to_vec(), rows: *rows, cols: *cols }
        }
        Cmd::MergeGraphs => Action::MergeGraphs,
        Cmd::StackPanels => Action::StackPanels { book: book?, columns: c.selected.to_vec() },
        Cmd::MultiYAxes => Action::MultiYAxes { book: book?, columns: c.selected.to_vec() },
        Cmd::ComparisonPlot => Action::ComparisonPlot { book: book?, sheet: None },
        Cmd::LabelPanels => Action::LabelPanels { book: book?, columns: c.selected.to_vec() },
        Cmd::Trendline { layer, plot, edit } => {
            Action::Trendline { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, plot: *plot, edit: *edit }
        }
        Cmd::AddErrorBars => Action::AddErrorBars { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer },
        Cmd::Refresh => Action::Refresh(c.active),
        Cmd::UpdateLegend => {
            Action::GraphEdit { graph: c.graph.ok_or(NEED_GRAPH)?, op: GraphOp::UpdateLegend { layer: c.layer } }
        }
        Cmd::FilterRows => Action::FilterRows(book?),
        Cmd::RowFilterDialog => Action::RowFilterDialog { book: book?, column: c.selected.first().copied() },
        Cmd::ToggleRowFilter => Action::ToggleRowFilter(book?),
        Cmd::ReapplyRowFilter => Action::ReapplyRowFilter(book?),
        Cmd::FreezePanes => Action::FreezePanes(book?),
        Cmd::CondFormat(kind) => {
            c.cells.then_some(()).ok_or(NEED_CELLS)?;
            Action::CondFormatDialog { book: book?, kind: *kind }
        }
        Cmd::WorksheetFonts => Action::WorksheetFontsDialog(book?),
        Cmd::FormatCells => {
            c.cells.then_some(()).ok_or(NEED_CELLS)?;
            Action::FormatCellsDialog(book?)
        }
        Cmd::EditNote => {
            c.cells.then_some(()).ok_or(NEED_CELLS)?;
            Action::NoteDialog(book?)
        }
        Cmd::DeleteNote => {
            c.cells.then_some(()).ok_or(NEED_CELLS)?;
            Action::DeleteNote(book?)
        }
        Cmd::RemoveCondFormat => {
            c.cells.then_some(()).ok_or(NEED_CELLS)?;
            Action::RemoveCondFormat(book?)
        }
        Cmd::CondFormatManager => Action::CondFormatManager(book?),
        // On a graph the tools take the active data plot, as Origin's Analysis menu does; the
        // results go to its workbook (ADR 0005), and a fit draws its curve on the graph (ADR 0006).
        Cmd::BatchPeaks => match (c.book, c.curve) {
            (Some(book), _) => Action::BatchPeaks { book, columns: c.selected.to_vec() },
            (None, Some((book, y))) if c.kind == WindowKind::Graph => {
                let columns = if c.layer_curves.is_empty() { vec![y] } else { c.layer_curves.clone() };
                Action::BatchPeaks { book, columns }
            }
            (None, _) if c.kind == WindowKind::Graph => return Err(ANALYSIS_ON_BOOK),
            (None, _) => return Err(NEED_BOOK),
        },
        Cmd::Analyze(kind) => match (c.book, c.curve) {
            (Some(book), _) => Action::Analyze { book, columns: c.selected.to_vec(), kind: *kind },
            // On a graph the limits of each range are picked there (T353).
            (None, Some((_, y))) if c.kind == WindowKind::Graph && *kind == AnalysisKind::IntegrateRanges => {
                match c.graph {
                    Some(graph) => Action::IntegrateRangesPick { graph, y, layer: c.layer },
                    None => return Err(ANALYSIS_ON_BOOK),
                }
            }
            // On a graph the feet of a CV peak are picked there (T349).
            (None, Some((_, y))) if c.kind == WindowKind::Graph && *kind == AnalysisKind::CvCharge => match c.graph {
                Some(graph) => Action::CvChargePick { graph, y, layer: c.layer },
                None => return Err(ANALYSIS_ON_BOOK),
            },
            // On a graph Subtract Straight Line is Origin's interactive tool: the points are picked there (T348).
            (None, Some((_, y))) if c.kind == WindowKind::Graph && *kind == AnalysisKind::SubtractLine => {
                match c.graph {
                    Some(graph) => Action::SubtractLinePick { graph, y, layer: c.layer },
                    None => return Err(ANALYSIS_ON_BOOK),
                }
            }
            (None, Some((book, y))) if c.kind == WindowKind::Graph => {
                let columns = match kind {
                    AnalysisKind::AverageCurves if c.layer_curves.len() >= 2 => c.layer_curves.clone(),
                    AnalysisKind::Intersect if c.layer_curves.len() >= 2 => c.layer_curves[..2].to_vec(),
                    _ => vec![y],
                };
                Action::Analyze { book, columns, kind: *kind }
            }
            (None, _) if c.kind == WindowKind::Graph => return Err(ANALYSIS_ON_BOOK),
            (None, _) => return Err(NEED_BOOK),
        },
        Cmd::InterpolateFromColumn => match (c.book, c.curve) {
            (Some(book), _) => Action::InterpolateFromColumn { book, columns: c.selected.to_vec() },
            (None, Some((book, y))) if c.kind == WindowKind::Graph => {
                Action::InterpolateFromColumn { book, columns: vec![y] }
            }
            (None, _) if c.kind == WindowKind::Graph => return Err(ANALYSIS_ON_BOOK),
            (None, _) => return Err(NEED_BOOK),
        },
        Cmd::InterpolateWith(method) => match (c.book, c.curve) {
            (Some(book), _) => Action::InterpolateWith { book, columns: c.selected.to_vec(), method: *method },
            (None, Some((book, y))) if c.kind == WindowKind::Graph => {
                Action::InterpolateWith { book, columns: vec![y], method: *method }
            }
            (None, _) if c.kind == WindowKind::Graph => return Err(ANALYSIS_ON_BOOK),
            (None, _) => return Err(NEED_BOOK),
        },
        Cmd::Fit(kind) => match (c.book, c.curve) {
            (Some(book), _) => Action::Fit { book, column: c.selected.first().copied(), kind: *kind },
            (None, Some((book, y))) if c.kind == WindowKind::Graph => {
                Action::Fit { book, column: Some(y), kind: *kind }
            }
            (None, _) if c.kind == WindowKind::Graph => return Err(ANALYSIS_ON_BOOK),
            (None, _) => return Err(NEED_BOOK),
        },
        // The window needs no data: with no workbook it makes a new one.
        Cmd::SimulateCurve => Action::SimulateCurve { book: c.book.or(c.curve.map(|(book, _)| book)) },
        Cmd::SimulateSurface => Action::SimulateSurface { book: c.book.or(c.curve.map(|(book, _)| book)) },
        Cmd::FitMore(kind) => match (c.book, c.curve) {
            (Some(book), _) => Action::FitMore { book, column: c.selected.first().copied(), kind: *kind },
            (None, Some((book, y))) if c.kind == WindowKind::Graph => {
                Action::FitMore { book, column: Some(y), kind: *kind }
            }
            (None, _) if c.kind == WindowKind::Graph => return Err(ANALYSIS_ON_BOOK),
            (None, _) => return Err(NEED_BOOK),
        },
        // F2 is the page properties of the active window: a layout page's own (T237 slice 6).
        Cmd::GraphProps(PropsTab::Page) if c.kind == WindowKind::Layout && c.active.is_some() => {
            Action::LayoutPageSetup { layout: c.active.ok_or(NEED_GRAPH)? }
        }
        Cmd::GraphProps(tab) => {
            Action::GraphProperties { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer, tab: *tab, plot: None }
        }
        Cmd::PlotSetup(layer) => {
            Action::PlotSetup { graph: c.graph.ok_or(NEED_GRAPH)?, layer: layer.unwrap_or(c.layer) }
        }
        Cmd::AddPlotToLayer(kind) => {
            Action::AddPlotToLayer { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer, kind: *kind }
        }
        Cmd::LayerContents(layer) => {
            Action::LayerContents { graph: c.graph.ok_or(NEED_GRAPH)?, layer: layer.unwrap_or(c.layer) }
        }
        Cmd::LayerManagement(layer) => Action::LayerManagement { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer },
        Cmd::FitPageToPrinter => Action::FitPageToPrinter { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::FitPageToLayers => Action::FitPageToLayers { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::GraphPropsAt { layer, tab, plot } => {
            Action::GraphProperties { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, tab: *tab, plot: *plot }
        }
        Cmd::GraphEdit(op) => Action::GraphEdit { graph: c.graph.ok_or(NEED_GRAPH)?, op: *op },
        Cmd::AddLayer(kind) => Action::GraphEdit {
            graph: c.graph.ok_or(NEED_GRAPH)?,
            op: GraphOp::AddLayer { layer: c.layer, kind: *kind },
        },
        Cmd::AddLayerDialog => Action::AddLayerDialog { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer },
        Cmd::Rescale => {
            Action::GraphEdit { graph: c.graph.ok_or(NEED_GRAPH)?, op: GraphOp::Rescale { layer: c.layer } }
        }
        Cmd::ExchangeAxes => {
            Action::GraphEdit { graph: c.graph.ok_or(NEED_GRAPH)?, op: GraphOp::ExchangeAxes { layer: c.layer } }
        }
        Cmd::SetStack(Stack::Cumulative) if !c.can_cumulate => return Err(NO_CUMULATIVE),
        Cmd::SetStack(stack) => Action::GraphEdit {
            graph: c.graph.ok_or(NEED_GRAPH)?,
            op: GraphOp::SetStack { layer: c.layer, stack: *stack },
        },
        Cmd::ReconstructLegend => {
            Action::GraphEdit { graph: c.graph.ok_or(NEED_GRAPH)?, op: GraphOp::ReconstructLegend { layer: c.layer } }
        }
        Cmd::SelectPlot { layer, plot } => {
            Action::SelectPlot { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, plot: *plot }
        }
        Cmd::GoTo(id) => Action::Focus(*id),
        Cmd::Pointer => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Pointer },
        Cmd::DataReader => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Reader },
        Cmd::DataTool(tool) => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Data(*tool) },
        Cmd::SlideShow => Action::SlideShow(some()?),
        Cmd::AddFavorite => Action::AddFavorite(c.graph.ok_or(NEED_GRAPH)?),
        Cmd::AuditLog => Action::AuditLog,
        Cmd::FitBuilder => Action::FitBuilder,
        Cmd::Options => Action::Options,
        Cmd::NewTable => Action::NewTable { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer },
        Cmd::SampleProjects => Action::SampleProjects,
        Cmd::DataMarkers(op) => Action::GraphEdit {
            graph: c.graph.ok_or(NEED_GRAPH)?,
            op: GraphOp::DataMarkers { layer: c.layer, op: *op },
        },
        Cmd::TextTool => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Text },
        Cmd::ShapeTool(kind) => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Draw(*kind) },
        Cmd::OrderSelectedShape { front } => {
            let (layer, shape, len) = c.selected_shape.ok_or(NEED_SHAPE)?;
            if (*front && shape + 1 >= len) || (!*front && shape == 0) {
                return Err(if *front { "Already at the front" } else { "Already at the back" });
            }
            Action::GraphEdit {
                graph: c.graph.ok_or(NEED_GRAPH)?,
                op: GraphOp::OrderShape { layer, shape, front: *front },
            }
        }
        Cmd::ArrangeSelected(_) if c.multi_selected < 2 => return Err(NEED_TWO_OBJECTS),
        Cmd::ArrangeSelected(op) => Action::ArrangeObjects { graph: c.graph.ok_or(NEED_GRAPH)?, op: *op },
        Cmd::GroupSelected { ungroup: false } if !c.can_group => return Err(CANNOT_GROUP),
        Cmd::GroupSelected { ungroup: true } if !c.can_ungroup => return Err(NOT_GROUPED),
        Cmd::GroupSelected { ungroup } => Action::GroupObjects { graph: c.graph.ok_or(NEED_GRAPH)?, ungroup: *ungroup },
        Cmd::SlopeTool => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Slope },
        Cmd::ZoomIn => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::ZoomIn },
        Cmd::PanTool => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Pan },
        Cmd::ZoomOut => Action::GraphView { graph: c.graph.ok_or(NEED_GRAPH)?, op: ViewOp::ZoomOut },
        Cmd::WholePage => Action::GraphView { graph: c.graph.ok_or(NEED_GRAPH)?, op: ViewOp::WholePage },
        Cmd::ZoomAll => Action::GraphView { graph: c.graph.ok_or(NEED_GRAPH)?, op: ViewOp::ZoomAll },
        Cmd::ScaleIn => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::ScaleIn },
        Cmd::ScaleOut => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::ScaleOut },
        Cmd::NewColorScale if !c.can_color_scale => return Err(NO_COLOR_SCALE),
        Cmd::ZAxis if !c.space3d => return Err(NEED_3D),
        Cmd::ZAxis => Action::Axis3d { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer, dim: 2 },
        Cmd::NewColorScale => Action::GraphEdit {
            graph: c.graph.ok_or(NEED_GRAPH)?,
            op: GraphOp::AddScale { layer: c.layer, kind: sheaf_core::layer_scale::LayerScaleKind::Color },
        },
        Cmd::AddStraightLine => Action::AddStraightLine { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer },
        Cmd::EditStraightLine { layer, line } => {
            Action::EditStraightLine { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, line: *line }
        }
        Cmd::SetViewMode(mode) => Action::SetViewMode { graph: c.graph.ok_or(NEED_GRAPH)?, mode: *mode },
        Cmd::ToggleLayerIcons => Action::SetLayerIcons { graph: c.graph.ok_or(NEED_GRAPH)?, on: !c.layer_icons },
        Cmd::ToggleActiveLayerIndicator => Action::SetPageSwitch {
            graph: c.graph.ok_or(NEED_GRAPH)?,
            switch: crate::actions::PageSwitch::ActiveLayerIndicator,
            on: !c.active_layer_indicator,
        },
        Cmd::LockPosition(position) => {
            Action::SetLockPosition { graph: c.graph.ok_or(NEED_GRAPH)?, position: *position }
        }
        Cmd::ToggleActivateByIconOnly => Action::SetPageSwitch {
            graph: c.graph.ok_or(NEED_GRAPH)?,
            switch: crate::actions::PageSwitch::ActivateByIconOnly,
            on: !c.activate_by_icon_only,
        },
        Cmd::ToggleIconPlotSetup => Action::SetPageSwitch {
            graph: c.graph.ok_or(NEED_GRAPH)?,
            switch: crate::actions::PageSwitch::IconDoubleClickPlotSetup,
            on: !c.icon_double_click_plot_setup,
        },
        Cmd::ToggleShowLabels => {
            Action::SetShowElements { graph: c.graph.ok_or(NEED_GRAPH)?, labels: Some(!c.show_labels), data: None }
        }
        Cmd::ToggleShowData => {
            Action::SetShowElements { graph: c.graph.ok_or(NEED_GRAPH)?, labels: None, data: Some(!c.show_data) }
        }
        Cmd::ToggleActiveLayerOnly => {
            let graph = c.graph.ok_or(NEED_GRAPH)?;
            c.active_layer_only.ok_or("Needs a valid active layer in a graph with at least two layers")?;
            Action::GraphEdit { graph, op: GraphOp::HideOtherLayers { layer: c.layer } }
        }
        Cmd::FullScreen => Action::FullScreen(c.graph.ok_or(NEED_GRAPH)?),
        Cmd::EditSlope { layer, slope } => {
            Action::EditSlope { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, slope: *slope }
        }
        Cmd::EditShape { layer, shape } => {
            Action::EditShape { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, shape: *shape }
        }
        Cmd::ExtractRegion { layer, shape } => {
            Action::ExtractRegion { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, shape: *shape }
        }
        Cmd::AddText => Action::AddText { graph: c.graph.ok_or(NEED_GRAPH)?, layer: Some(c.layer), at: None },
        Cmd::EditText { layer, label } => {
            Action::EditText { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, label: *label }
        }
        Cmd::EditTitle { layer, vertical } => Action::EditTitle {
            graph: c.graph.ok_or(NEED_GRAPH)?,
            layer: layer.unwrap_or(c.layer),
            vertical: *vertical,
        },
        Cmd::EditLegendText { layer } => {
            Action::EditLegendText { graph: c.graph.ok_or(NEED_GRAPH)?, layer: layer.unwrap_or(c.layer) }
        }
        Cmd::CopyPage => Action::CopyPage(c.graph.ok_or(NEED_GRAPH)?),
        Cmd::CopyGraphFormat(part) => Action::CopyGraphFormat { graph: c.graph.ok_or(NEED_GRAPH)?, part: *part },
        Cmd::EditCopyFormat(part) => Action::EditCopyFormat { graph: c.graph.ok_or(NEED_GRAPH)?, part: *part },
        Cmd::CopyAxisFormat { layer, y, part } => {
            Action::CopyAxisFormat { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, y: *y, part: *part }
        }
        Cmd::PasteGraphFormat => Action::PasteGraphFormat { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::PasteAxesFormat { targets, part } => {
            Action::PasteAxesFormat { graph: c.graph.ok_or(NEED_GRAPH)?, targets: *targets, part: *part }
        }
        Cmd::PasteAxisFormat { layer, y, part } => {
            Action::PasteAxisFormat { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, y: *y, part: *part }
        }
        Cmd::CopyLegendStyles { layer } => {
            Action::CopyLegendStyles { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer }
        }
        Cmd::PasteLegendStyles { layer } => {
            Action::PasteLegendStyles { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer }
        }
        Cmd::CopyLegendFormat { layer, part } => {
            Action::CopyLegendFormat { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, part: *part }
        }
        Cmd::PasteLegendFormat { layer, part } => {
            Action::PasteLegendFormat { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, part: *part }
        }
        Cmd::PasteFormatDialog => Action::PasteFormatDialog { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer },
        Cmd::IntegrateGadget => Action::StartIntegrate { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::RiseTimeGadget => Action::StartRiseTime { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::IntersectGadget => Action::StartIntersect { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::QuickPeaksGadget => Action::StartQuickPeaks { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::SigmoidGadget => Action::StartSigmoid { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::FftGadget => Action::StartFft { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::StatisticsGadget => Action::StartStatisticsGadget { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::QuickFitGadget => Action::StartQuickFit { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::VerticalCursorGadget => Action::StartVerticalCursor { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::DifferentiateGadget => Action::StartDifferentiate { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::InterpolateGadget => Action::StartInterpolate { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::ClusterGadget => Action::StartCluster { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::Integrate2dGadget => Action::StartIntegrate2d { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::Gadget(op) => Action::Gadget { graph: c.graph.ok_or(NEED_GRAPH)?, op: *op },
        Cmd::Toolbars => Action::Toolbars,
        Cmd::Autosave => Action::Autosave,
        Cmd::ToggleStatusBar => Action::ToggleStatusBar,
        Cmd::ToggleAiLink => Action::ToggleAiLink,
        Cmd::CloseWindow => Action::HideWindow(c.active.ok_or(NEED_WINDOW)?),
        Cmd::DuplicateWindow => Action::DuplicateWindow(c.active.ok_or(NEED_WINDOW)?),
        Cmd::WindowSwitcher => Action::OpenSwitcher,
        Cmd::AppendProject => Action::AppendProject(None),
        Cmd::ClearRecent(kind) => {
            if c.recent(*kind).is_empty() {
                return Err("The list is empty");
            }
            Action::ClearRecent(*kind)
        }
        Cmd::ClearRecentExports => {
            if c.recent_exports.is_empty() {
                return Err("The list is empty");
            }
            Action::ClearRecentExports
        }
        Cmd::Panel(p) => Action::TogglePanel(*p),
        Cmd::Cascade => Action::Cascade,
        Cmd::TileHorizontally => Action::TileHorizontally,
        Cmd::TileVertically => Action::TileVertically,
        Cmd::ArrangeIcons => Action::ArrangeIcons,
        Cmd::WindowProperties => Action::RenameWindow(c.active.ok_or(NEED_WINDOW)?),
        Cmd::About => Action::About,
    })
}

/// The keyboard shortcuts of all commands that Sheaf has, in all bars (so that, for example,
/// Undo still works when no window is open and the bar has no Edit menu), the more specific
/// key combinations first.
pub fn shortcuts() -> Vec<(KeyboardShortcut, Cmd)> {
    let mut out: Vec<(KeyboardShortcut, Cmd)> = Vec::new();
    for kind in [WindowKind::Workbook, WindowKind::Graph, WindowKind::None] {
        for m in bar(kind) {
            walk(&m.entries, &mut |i| {
                // Ctrl+Tab is read by the Window Switcher itself (it needs the key release).
                if let (Some(keys), Some(cmd)) = (i.keys, &i.cmd)
                    && *cmd != Cmd::WindowSwitcher
                    && !out.iter().any(|(k, c)| *k == keys && c == cmd)
                {
                    out.push((keys, cmd.clone()));
                }
            });
        }
    }
    let weight = |m: &Modifiers| [m.alt, m.ctrl, m.shift, m.mac_cmd, m.command].iter().filter(|b| **b).count();
    out.sort_by_key(|(k, _)| std::cmp::Reverse(weight(&k.modifiers)));
    out
}

/// Commands that stay available while a text field has the keyboard: the ones that do not
/// work on what the user is typing.
pub fn works_while_typing(cmd: &Cmd) -> bool {
    matches!(
        cmd,
        Cmd::NewProject
            | Cmd::OpenProject
            | Cmd::SaveProject
            | Cmd::SaveProjectAs
            | Cmd::Import
            | Cmd::ImportMultiple
            | Cmd::ImportCsv
            | Cmd::ImportWizard
            | Cmd::ImportExcel
            | Cmd::Exit
    )
}

// ----- drawing -------------------------------------------------------------------------------

/// The text as a layout job with the Alt letter underlined. `indent` leaves the room at the left
/// of an item in which a tick can be drawn.
fn label_job(ui: &egui::Ui, label: &str, indent: bool) -> LayoutJob {
    let (text, at) = split(label);
    let font_id = TextStyle::Button.resolve(ui.style());
    // The placeholder colour is replaced by the colour of the widget (enabled, hovered, weak).
    let normal = TextFormat { font_id, color: Color32::PLACEHOLDER, ..Default::default() };
    let underlined = TextFormat { underline: Stroke::new(1.0_f32, Color32::PLACEHOLDER), ..normal.clone() };
    let lead = if indent { 20.0 } else { 0.0 };
    let mut job = LayoutJob::default();
    match at {
        Some(k) => {
            let chars: Vec<char> = text.chars().collect();
            let before: String = chars[..k].iter().collect();
            let letter: String = chars[k..k + 1].iter().collect();
            let after: String = chars[k + 1..].iter().collect();
            job.append(&before, lead, normal.clone());
            job.append(&letter, 0.0, underlined);
            job.append(&after, 0.0, normal);
        }
        None => job.append(&text, lead, normal),
    }
    job
}

/// Draws the bar of the active window's kind and pushes the actions of clicked items.
pub fn show_bar(ui: &mut egui::Ui, c: &MenuContext, actions: &mut Vec<Action>) {
    egui::MenuBar::new().ui(ui, |ui| {
        for menu in bar(c.kind) {
            let response = ui.menu_button(label_job(ui, &menu.label, false), |ui| {
                ui.set_min_width(180.0);
                show_entries(ui, &menu.entries, c, actions, MruMenu::from_name(&menu.name()));
            });
            // The lists of the menus that show recent entries are read again each time one opens.
            if menu.name() == "File" || MruMenu::from_name(&menu.name()).is_some() {
                // The recent files are read again each time File opens, not every frame.
                let key = ui.id().with(("menu was open", menu.name()));
                let open = response.inner.is_some();
                let was = ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
                if open && !was {
                    actions.push(Action::ReloadRecent);
                }
                ui.data_mut(|d| d.insert_temp(key, open));
            }
            if c.open.is_some_and(|o| o.split('>').next() == Some(menu.name().as_str())) {
                egui::Popup::open_id(ui.ctx(), response.response.id.with("popup"));
            }
            // Alt and the letter opens the menu.
            if let Some(letter) = mnemonic(&menu.label)
                && let Some(key) = Key::from_name(&letter.to_ascii_uppercase().to_string())
                && ui.input_mut(|i| i.consume_key(Modifiers::ALT, key))
            {
                egui::Popup::open_id(ui.ctx(), response.response.id.with("popup"));
            }
        }
    });
}

/// `used` is the menu the entries belong to when it keeps a Most Recently Used list: a command run
/// from it is recorded there.
fn show_entries(
    ui: &mut egui::Ui,
    entries: &[Entry],
    c: &MenuContext,
    actions: &mut Vec<Action>,
    used: Option<MruMenu>,
) {
    for entry in entries {
        match entry {
            Entry::Separator => {
                ui.separator();
            }
            Entry::Submenu(s) => {
                let response = ui.menu_button(label_job(ui, &s.label, true), |ui| {
                    ui.set_min_width(160.0);
                    show_entries(ui, &s.entries, c, actions, used);
                });
                // `--show menu:File>Recent Projects` holds the submenu open for a picture.
                if c.open.is_some_and(|o| o.split('>').skip(1).any(|sub| sub == plain(&s.label))) {
                    use egui::containers::menu::{MenuState, SubMenu};
                    let id = SubMenu::id_from_widget_id(response.response.id);
                    MenuState::from_ui(ui, |state, _| state.open_item = Some(id));
                    // The menu forgets an open item that was not shown last frame.
                    MenuState::mark_shown(ui.ctx(), id);
                }
            }
            Entry::Recent(kind) => show_recent(ui, *kind, c, actions),
            Entry::UserTemplates => show_user_templates(ui, c, actions),
            Entry::NewFromTemplates => {
                if c.user_templates.is_empty() && c.user_book_templates.is_empty() {
                    ui.add_enabled(false, egui::Button::new(label_job(ui, "(no saved templates)", true)));
                }
                for name in c.user_templates {
                    if ui.add(egui::Button::new(label_job(ui, name, true))).clicked() {
                        actions.push(Action::NewFromTemplate(Some(name.clone())));
                        ui.close();
                    }
                }
                // Workbook templates (T160) follow the graph templates, after a line.
                if !c.user_templates.is_empty() && !c.user_book_templates.is_empty() {
                    ui.separator();
                }
                for name in c.user_book_templates {
                    if ui.add(egui::Button::new(label_job(ui, name, true))).clicked() {
                        actions.push(Action::NewBookFromTemplate(name.clone()));
                        ui.close();
                    }
                }
            }
            Entry::Mru(menu) => show_mru(ui, *menu, c, actions),
            Entry::Exports => show_exports(ui, c, actions),
            Entry::Folders => {
                for (n, (path, name)) in c.folders.iter().enumerate() {
                    let response = ui.add(egui::Button::new(label_job(ui, &format!("&{} {name}", n + 1), true)));
                    if *path == c.folder {
                        tick(ui, &response);
                    }
                    if response.clicked() {
                        actions.push(Action::GoToFolder(path.clone()));
                        ui.close();
                    }
                }
            }
            Entry::Windows => {
                for (n, (id, title)) in c.windows.iter().enumerate() {
                    let label = format!("&{} {title}", n + 1);
                    let active = c.active == Some(*id);
                    let response = ui.add(egui::Button::new(label_job(ui, &label, true)));
                    if active {
                        tick(ui, &response);
                    }
                    if response.clicked() {
                        actions.push(Action::Focus(*id));
                        ui.close();
                    }
                }
            }
            Entry::Item(it) => {
                let mut label = it.label.clone();
                // Undo and Redo name what they undo.
                match (&it.cmd, c.undo, c.redo) {
                    (Some(Cmd::Undo), Some(what), _) | (Some(Cmd::Redo), _, Some(what)) => {
                        label = format!("{label} {what}");
                    }
                    _ => {}
                }
                if matches!(it.cmd, Some(Cmd::ToggleMasking)) && masking_disabled(ui.ctx()) {
                    label = "Enable Masking".to_string();
                }
                if matches!(it.cmd, Some(Cmd::ToggleHideMasked)) && masked_hidden(ui.ctx()) {
                    label = "Show Masked Points".to_string();
                }
                let resolved = it.cmd.as_ref().map(|cmd| resolve(cmd, c));
                let enabled = matches!(resolved, Some(Ok(_)));
                let mut button = egui::Button::new(label_job(ui, &label, true));
                if let Some(keys) = &it.keys {
                    button = button.shortcut_text(ui.ctx().format_shortcut(keys));
                }
                let response = ui.add_enabled(enabled, button);
                if let Some(Cmd::Panel(p)) = &it.cmd
                    && c.panels[p.index()]
                {
                    tick(ui, &response);
                }
                if matches!(it.cmd, Some(Cmd::ToggleAiLink)) && c.ai_link {
                    tick(ui, &response);
                }
                if matches!(it.cmd, Some(Cmd::ToggleLayerIcons)) && c.layer_icons {
                    tick(ui, &response);
                }
                if matches!(it.cmd, Some(Cmd::ToggleActiveLayerIndicator)) && c.active_layer_indicator {
                    tick(ui, &response);
                }
                if let Some(Cmd::LockPosition(position)) = &it.cmd
                    && *position == c.lock_position
                {
                    tick(ui, &response);
                }
                if matches!(
                    it.cmd,
                    Some(Cmd::Sheet(SheetOp::ToggleView(sheaf_core::sheet_view::ViewSwitch::PageBreaks)))
                ) && c.page_breaks
                {
                    tick(ui, &response);
                }
                if (matches!(it.cmd, Some(Cmd::ToggleActivateByIconOnly)) && c.activate_by_icon_only)
                    || (matches!(it.cmd, Some(Cmd::ToggleIconPlotSetup)) && c.icon_double_click_plot_setup)
                {
                    tick(ui, &response);
                }
                if (matches!(it.cmd, Some(Cmd::ToggleShowLabels)) && c.show_labels)
                    || (matches!(it.cmd, Some(Cmd::ToggleShowData)) && c.show_data)
                {
                    tick(ui, &response);
                }
                if matches!(it.cmd, Some(Cmd::ToggleActiveLayerOnly)) && c.active_layer_only == Some(true) {
                    tick(ui, &response);
                }
                if let Some(Cmd::SetViewMode(mode)) = &it.cmd
                    && *mode == c.view_mode
                {
                    tick(ui, &response);
                }
                if let Some(Cmd::ViewWindows(mode)) = &it.cmd
                    && *mode == c.view_windows
                {
                    tick(ui, &response);
                }
                if let (Some(Cmd::SetStack(mode)), Some(current)) = (&it.cmd, c.stack)
                    && std::mem::discriminant(mode) == std::mem::discriminant(&current)
                {
                    tick(ui, &response);
                }
                if response.contains_pointer() {
                    crate::panels::set_hint(ui.ctx(), &plain(&it.label));
                }
                match resolved {
                    Some(Ok(action)) => {
                        if response.clicked() {
                            if let (Some(menu), Some(cmd)) = (used, &it.cmd)
                                && mru::recordable(cmd)
                            {
                                // First, so that it sees the window the command was run on.
                                actions.push(Action::UsedCommand { menu, cmd: cmd.clone() });
                            }
                            actions.push(action);
                            ui.close();
                        }
                    }
                    Some(Err(reason)) => {
                        response.on_disabled_hover_text(reason);
                    }
                    None => {
                        response.on_disabled_hover_text(NOT_YET);
                    }
                }
            }
        }
    }
}

/// The entries of a recent list as Origin 2016 shows them: numbered 1 to 9 and A to F, the path
/// without its extension shortened to 80 characters, the full path as the tooltip. An empty list
/// shows a greyed "(none)".
fn show_recent(ui: &mut egui::Ui, kind: RecentKind, c: &MenuContext, actions: &mut Vec<Action>) {
    let list = c.recent(kind);
    if list.is_empty() {
        ui.add_enabled(false, egui::Button::new(label_job(ui, "(none)", true)));
        return;
    }
    for (n, path) in list.iter().enumerate() {
        let job = label_job(ui, &recent::numbered(n, &recent::entry_text(path, RECENT_ENTRY_CHARS)), true);
        let full = path.display().to_string();
        let response = ui.add(egui::Button::new(job)).on_hover_text(&full);
        if response.contains_pointer() {
            crate::panels::set_hint(ui.ctx(), &full);
        }
        if response.clicked() {
            actions.push(match kind {
                RecentKind::Projects | RecentKind::Books | RecentKind::Graphs => Action::OpenRecent(path.clone(), kind),
                RecentKind::Imports => Action::ImportRecent(path.clone()),
            });
            ui.close();
        }
    }
}

/// The Most Recently Used commands of a menu: numbered, newest first, each looked up again in the
/// bar and resolved against the current selection (greyed with the reason when it cannot run now).
/// An empty list shows a greyed "(none)". Sheaf's choice, docs/spec/recent-lists.md 2.
fn show_mru(ui: &mut egui::Ui, menu: MruMenu, c: &MenuContext, actions: &mut Vec<Action>) {
    let keys = c.recent_commands.get(&menu.list_name()).map_or(&[][..], Vec::as_slice);
    let entries: Vec<(&String, &Item)> =
        keys.iter().filter_map(|k| mru::find(bar(c.kind), menu, k).map(|i| (k, i))).collect();
    if entries.is_empty() {
        ui.add_enabled(false, egui::Button::new(label_job(ui, "(none)", true)));
        return;
    }
    for (n, (key, it)) in entries.into_iter().enumerate() {
        let Some(cmd) = &it.cmd else { continue };
        let resolved = resolve(cmd, c);
        let job = label_job(ui, &recent::numbered(n, &plain(&it.label)), true);
        let response = ui.add_enabled(resolved.is_ok(), egui::Button::new(job));
        if response.contains_pointer() {
            crate::panels::set_hint(ui.ctx(), key);
        }
        match resolved {
            Ok(action) => {
                if response.on_hover_text(key).clicked() {
                    actions.push(Action::UsedCommand { menu, cmd: cmd.clone() });
                    actions.push(action);
                    ui.close();
                }
            }
            Err(reason) => {
                response.on_disabled_hover_text(reason);
            }
        }
    }
}

/// The entries of Recent Exports: the files exports wrote, numbered like the other recent lists,
/// the full path as the tooltip; a click opens the export again on the same window (Sheaf's choice).
fn show_exports(ui: &mut egui::Ui, c: &MenuContext, actions: &mut Vec<Action>) {
    if c.recent_exports.is_empty() {
        ui.add_enabled(false, egui::Button::new(label_job(ui, "(none)", true)));
        return;
    }
    for (n, e) in c.recent_exports.iter().enumerate() {
        // `entry_text` drops the extension, which an exported file needs; a dummy one takes its place.
        let shown = recent::entry_text(&PathBuf::from(format!("{}.x", e.path.display())), RECENT_ENTRY_CHARS);
        let full = e.path.display().to_string();
        let response = ui.add(egui::Button::new(label_job(ui, &recent::numbered(n, &shown), true)));
        let response = response.on_hover_text(format!("{full}\n{}", e.window));
        if response.contains_pointer() {
            crate::panels::set_hint(ui.ctx(), &full);
        }
        if response.clicked() {
            actions.push(Action::OpenRecentExport(e.clone()));
            ui.close();
        }
    }
}

/// The saved user templates: plotting the selected columns of the active workbook with one.
fn show_user_templates(ui: &mut egui::Ui, c: &MenuContext, actions: &mut Vec<Action>) {
    if c.user_templates.is_empty() {
        ui.add_enabled(false, egui::Button::new(label_job(ui, "(no saved templates)", true)));
        return;
    }
    for name in c.user_templates {
        let can = c.book.is_some() && !c.selected.is_empty();
        let response = ui.add_enabled(can, egui::Button::new(label_job(ui, name, true)));
        let response = if can { response } else { response.on_disabled_hover_text(NEED_COLUMNS) };
        if response.clicked()
            && let Some(book) = c.book
        {
            actions.push(Action::PlotUserTemplate { book, columns: c.selected.to_vec(), name: name.clone() });
            ui.close();
        }
    }
}

/// The longest folder text an entry shows; longer folders are cut from the left (Sheaf's choice).
/// Origin 2016 shortens a recent entry to 80 characters (recent-files.md 2).
const RECENT_ENTRY_CHARS: usize = 80;

/// A check mark in the room left of the label.
fn tick(ui: &egui::Ui, response: &egui::Response) {
    let color = ui.style().interact(response).text_color();
    let c = egui::pos2(response.rect.left() + 10.0, response.rect.center().y);
    let stroke = Stroke::new(1.5_f32, color);
    let p = ui.painter();
    p.line_segment([c + egui::vec2(-3.5, 0.0), c + egui::vec2(-1.0, 3.0)], stroke);
    p.line_segment([c + egui::vec2(-1.0, 3.0), c + egui::vec2(4.0, -3.5)], stroke);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = include_str!("../../../docs/spec/ui-command-map.md");

    /// The top-level menu names of a window type, from the table in section A0 of the spec.
    fn spec_names(resource: &str) -> Vec<String> {
        let row = SPEC
            .lines()
            .find(|l| l.starts_with(&format!("| {resource} |")))
            .unwrap_or_else(|| panic!("no row for resource {resource} in the spec"));
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        let list = cells[3];
        list.split(',').map(|s| s.trim().to_string()).collect()
    }

    fn names(kind: WindowKind) -> Vec<String> {
        bar(kind).iter().map(Menu::name).collect()
    }

    #[test]
    fn the_top_level_names_follow_the_spec_for_each_window_type() {
        assert_eq!(names(WindowKind::Workbook), spec_names("501"));
        assert_eq!(names(WindowKind::Graph), spec_names("503"));
        // The row of the spec for the menu without a window has a longer description.
        assert_eq!(names(WindowKind::None), spec_names("480"));
        // And the literal lists, so that a change of the spec text alone is noticed too.
        assert_eq!(
            names(WindowKind::Workbook),
            [
                "File",
                "Edit",
                "View",
                "Plot",
                "Column",
                "Worksheet",
                "Analysis",
                "Statistics",
                "Image",
                "Tools",
                "Format",
                "Window",
                "Help"
            ]
        );
        assert_eq!(
            names(WindowKind::Graph),
            ["File", "Edit", "View", "Graph", "Data", "Analysis", "Gadgets", "Tools", "Format", "Window", "Help"]
        );
        assert_eq!(names(WindowKind::None), ["File", "View", "Tools", "Window", "Help"]);
    }

    #[test]
    fn every_menu_has_distinct_alt_letters_and_every_label_has_one() {
        fn check(entries: &[Entry], path: &str) {
            let mut seen: Vec<char> = Vec::new();
            for e in entries {
                let label = match e {
                    Entry::Item(i) => &i.label,
                    Entry::Submenu(s) => {
                        check(&s.entries, &format!("{path} > {}", plain(&s.label)));
                        &s.label
                    }
                    _ => continue,
                };
                // Items that have a command must have a letter; a greyed one may lack it when all
                // the letters of its text are taken by others in the same menu.
                let Some(letter) = mnemonic(label) else {
                    let live = matches!(e, Entry::Item(i) if i.cmd.is_some());
                    assert!(!live, "{path} > {label} has no Alt letter");
                    continue;
                };
                assert!(!seen.contains(&letter), "{path}: the Alt letter {letter} is used twice");
                seen.push(letter);
            }
        }
        for kind in [WindowKind::None, WindowKind::Workbook, WindowKind::Graph] {
            check_top(bar(kind));
            for m in bar(kind) {
                check(&m.entries, &m.name());
            }
        }
        fn check_top(menus: &[Menu]) {
            let mut seen: Vec<char> = Vec::new();
            for m in menus {
                let letter = mnemonic(&m.label).expect("a top-level menu has an Alt letter");
                assert!(!seen.contains(&letter), "two top-level menus use Alt+{letter}");
                seen.push(letter);
            }
        }
    }

    #[test]
    fn the_table_names_every_analysis_fit_plot_and_designation() {
        let menus = base(WindowKind::Workbook);
        let (analysis, fits) = unplaced(&menus);
        assert!(analysis.is_empty(), "kinds of analysis without a place in the menus: {analysis:?}");
        assert!(fits.is_empty(), "kinds of fit without a place in the menus: {fits:?}");
        let mut cmds: Vec<Cmd> = Vec::new();
        for m in bar(WindowKind::Workbook) {
            walk(&m.entries, &mut |i| cmds.extend(i.cmd.clone()));
        }
        for t in Template::ALL {
            // The flat colour pie stays as a CLI name only: Origin has no "2D Color Pie".
            if t == Template::Pie {
                continue;
            }
            assert!(cmds.contains(&Cmd::Plot(t)), "{t:?} is not in the Plot menu");
        }
        for d in Designation::ALL {
            assert!(cmds.contains(&Cmd::SetAs(d)), "{d:?} is not in Column > Set As");
        }
    }

    #[test]
    fn the_horizontal_bar_items_of_the_plot_menu_are_live_and_make_the_right_graph() {
        use sheaf_core::graph::PlotKind;
        let mut items: Vec<(String, Cmd)> = Vec::new();
        for m in bar(WindowKind::Workbook) {
            walk(&m.entries, &mut |i| {
                if let Some(c) = &i.cmd {
                    items.push((i.label.clone(), c.clone()));
                }
            });
        }
        for (label, t, kind) in [
            ("Stacked Bar", Template::StackedBar, PlotKind::StackedBar),
            ("100% Stacked Bar", Template::Stacked100Bar, PlotKind::Stacked100Bar),
            ("Floating Bar", Template::FloatingBar, PlotKind::FloatingBar),
        ] {
            assert!(
                items.iter().any(|(l, c)| l.replace('&', "") == label && *c == Cmd::Plot(t)),
                "{label} is a live item"
            );
            let g = t.new_graph(sheaf_core::WindowId(0), "");
            assert_eq!(g.layers[0].default_kind, kind);
        }
    }

    #[test]
    fn all_three_bars_have_the_window_commands_and_the_window_list() {
        for kind in [WindowKind::None, WindowKind::Workbook, WindowKind::Graph] {
            let window = bar(kind).iter().find(|m| m.name() == "Window").expect("a Window menu");
            let mut cmds: Vec<Cmd> = Vec::new();
            walk(&window.entries, &mut |i| cmds.extend(i.cmd.clone()));
            for c in [Cmd::Cascade, Cmd::TileHorizontally, Cmd::TileVertically, Cmd::WindowProperties] {
                assert!(cmds.contains(&c), "{kind:?}: {c:?} is missing from Window");
            }
            assert!(window.entries.iter().any(|e| matches!(e, Entry::Windows)), "{kind:?}: no window list");
        }
    }

    #[test]
    fn no_key_combination_runs_two_commands_and_origins_keys_are_used() {
        let keys = shortcuts();
        for (k, cmd) in &keys {
            for (k2, cmd2) in &keys {
                let formula = Cmd::Copy(CopyKind::Part(CopyPart::Formula));
                let shared =
                    matches!((cmd, cmd2), (Cmd::GoToCell, Cmd::ExportGraph) | (Cmd::ExportGraph, Cmd::GoToCell))
                        || (*cmd == formula && *cmd2 == Cmd::CopyPage)
                        || (*cmd == Cmd::CopyPage && *cmd2 == formula);
                assert!(k != k2 || cmd == cmd2 || shared, "{k:?} runs both {cmd:?} and {cmd2:?}");
            }
        }
        let find = |ks: KeyboardShortcut| keys.iter().find(|(k, _)| *k == ks).map(|(_, c)| c.clone());
        assert_eq!(find(ctrl(Key::K)), Some(Cmd::Import), "Ctrl+K imports, as in Origin");
        assert_eq!(find(ctrl(Key::Q)), Some(Cmd::SetColumnValues));
        assert_eq!(find(ctrl_shift(Key::Q)), Some(Cmd::SetMultipleColumnsValues));
        assert_eq!(find(ctrl(Key::D)), Some(Cmd::AddColumns));
        assert_eq!(find(ctrl(Key::Y)), Some(Cmd::Fit(FitKind::BuiltIn)), "Ctrl+Y is Origin's, not redo");
        assert_eq!(find(ctrl(Key::N)), Some(Cmd::NewProject));
        assert_eq!(find(ctrl(Key::O)), Some(Cmd::OpenProject));
        assert_eq!(find(ctrl(Key::S)), Some(Cmd::SaveProject));
        assert_eq!(find(ctrl(Key::I)), Some(Cmd::ZoomIn), "Ctrl+I is Zoom In in Origin, no longer Import");
        assert_eq!(find(ctrl(Key::M)), Some(Cmd::ZoomOut));
        assert_eq!(find(ctrl(Key::W)), Some(Cmd::WholePage));
        assert_eq!(find(plain_key(Key::F2)), Some(Cmd::GraphProps(PropsTab::Page)));
        assert_eq!(find(alt(Key::Enter)), Some(Cmd::WindowProperties));
    }

    #[test]
    fn commands_say_what_is_missing_when_they_cannot_run() {
        let c = MenuContext {
            kind: WindowKind::Graph,
            book: None,
            graph: Some(WindowId(2)),
            active: Some(WindowId(2)),
            selected: &[],
            sorting_columns: Vec::new(),
            cells: false,
            whole_columns: false,
            imported: false,
            undo: None,
            redo: None,
            undo_ai: false,
            windows: Vec::new(),
            folders: Vec::new(),
            folder: Vec::new(),
            view_windows: Default::default(),
            open: None,
            recent_projects: &[],
            recent_imports: &[],
            recent_books: &[],
            recent_graphs: &[],
            recent_commands: &crate::menus::NO_COMMANDS,
            recent_exports: &[],
            user_templates: &[],
            user_book_templates: &[],
            templates: false,
            panels: [true; 4],
            ai_link: false,
            layer: 0,
            tool: Default::default(),
            curve: None,
            graph_pick: None,
            graph_paste: false,
            layer_curves: Vec::new(),
            stack: None,
            can_cumulate: false,
            can_color_scale: false,
            space3d: false,
            selected_shape: None,
            multi_selected: 0,
            can_group: false,
            can_ungroup: false,
            view_mode: Default::default(),
            layer_icons: true,
            active_layer_indicator: false,
            lock_position: Default::default(),
            page_breaks: false,
            activate_by_icon_only: false,
            icon_double_click_plot_setup: false,
            show_labels: true,
            show_data: true,
            active_layer_only: None,
        };
        assert_eq!(resolve(&Cmd::SetColumnValues, &c).unwrap_err(), NEED_BOOK);
        assert_eq!(resolve(&Cmd::Analyze(AnalysisKind::Smooth), &c).unwrap_err(), ANALYSIS_ON_BOOK);
        assert_eq!(resolve(&Cmd::Fit(FitKind::Linear), &c).unwrap_err(), ANALYSIS_ON_BOOK);
        // With a data plot on the graph, the Analysis menu works on it (Origin's active data plot).
        let on_plot = MenuContext { curve: Some((WindowId(5), ColumnId(9))), ..c.clone() };
        assert!(matches!(
            resolve(&Cmd::Analyze(AnalysisKind::Smooth), &on_plot),
            Ok(Action::Analyze { book: WindowId(5), ref columns, kind: AnalysisKind::Smooth }) if *columns == [ColumnId(9)]
        ));
        assert!(matches!(
            resolve(&Cmd::Fit(FitKind::Linear), &on_plot),
            Ok(Action::Fit { book: WindowId(5), column: Some(ColumnId(9)), kind: FitKind::Linear })
        ));
        assert!(matches!(resolve(&Cmd::ExportGraph, &c), Ok(Action::ExportGraph(WindowId(2)))));
        assert_eq!(resolve(&Cmd::Undo, &c).unwrap_err(), NO_UNDO);
        assert!(matches!(resolve(&Cmd::Import, &c), Ok(Action::ImportPick(ImportPick::Single))));
        let id = ColumnId(7);
        let sel = [id];
        let c = MenuContext { kind: WindowKind::Workbook, book: Some(WindowId(1)), graph: None, selected: &sel, ..c };
        assert!(matches!(resolve(&Cmd::SetColumnValues, &c), Ok(Action::SetColumnValues(x)) if x == id));
        assert_eq!(resolve(&Cmd::ExportGraph, &c).unwrap_err(), NEED_GRAPH);
        // Several columns need two or more.
        assert_eq!(resolve(&Cmd::SetMultipleColumnsValues, &c).unwrap_err(), NEED_TWO_COLUMNS);
        assert_eq!(resolve(&Cmd::SwapColumns, &c).unwrap_err(), NEED_EXACTLY_TWO_COLUMNS);
        // The Copy variants need whole columns (T88), as Origin greys them for a block of cells.
        let part = Cmd::Copy(CopyKind::Part(CopyPart::All));
        let block = MenuContext { cells: true, whole_columns: false, ..c.clone() };
        assert_eq!(resolve(&part, &block).unwrap_err(), NEED_WHOLE_COLUMNS);
        assert!(resolve(&Cmd::Copy(CopyKind::Plain), &block).is_ok());
        let columns = MenuContext { cells: true, whole_columns: true, ..c.clone() };
        assert!(resolve(&part, &columns).is_ok());
        let three = [ColumnId(7), ColumnId(8), ColumnId(9)];
        let c3 = MenuContext { selected: &three, ..c.clone() };
        assert_eq!(resolve(&Cmd::SwapColumns, &c3).unwrap_err(), NEED_EXACTLY_TWO_COLUMNS);
        let two = [ColumnId(7), ColumnId(8)];
        let c = MenuContext { selected: &two, ..c };
        assert!(matches!(resolve(&Cmd::SwapColumns, &c), Ok(Action::SwapColumns(ColumnId(7), ColumnId(8)))));
        assert!(
            matches!(resolve(&Cmd::SetMultipleColumnsValues, &c), Ok(Action::SetMultipleColumnsValues(x)) if x == two)
        );
        assert_eq!(resolve(&Cmd::SetColumnValues, &c).unwrap_err(), NEED_ONE_COLUMN);
    }

    #[test]
    fn labels_parse_the_ampersand_notation() {
        assert_eq!(plain("&File"), "File");
        assert_eq!(mnemonic("Wor&ksheet"), Some('k'));
        assert_eq!(plain("2D B&&W Pie"), "2D B&W Pie");
        assert_eq!(mnemonic("2D B&&W Pie"), None);
        assert_eq!(mnemonic("&1 Book1 - a&b"), Some('1'));
    }
}
