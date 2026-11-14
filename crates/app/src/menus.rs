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

use crate::actions::{Action, CellOp, CopyKind, GraphOp, GraphTool, ImportPick, SheetOp};
use crate::app::APP_NAME;
use crate::dialogs::analysis::AnalysisKind;
use crate::dialogs::fit::FitKind;
use crate::dialogs::fit_more::FitMoreKind;
use crate::dialogs::graph_props::PropsTab;
use crate::dialogs::reshape::ReshapeKind;
use crate::gadget::GadgetOp;
use crate::panels::PanelId;
use eframe::egui::{
    self, Color32, Key, KeyboardShortcut, Modifiers, Stroke, TextStyle,
    text::{LayoutJob, TextFormat},
};
use sheaf_core::graph::PlotKind;
use sheaf_core::sheetops::{MaskOp, MoveTo};
use sheaf_core::template::Template;
use sheaf_core::{ColumnId, Designation, WindowId};
use std::sync::OnceLock;

/// Tooltip of an item that Sheaf does not have.
pub const NOT_YET: &str = "Not available in Sheaf yet";
const NEED_BOOK: &str = "Activate a workbook window first";
const NEED_GRAPH: &str = "Activate a graph window first";
const NEED_COLUMNS: &str = "Select one or more columns first";
const NEED_CELLS: &str = "Select cells, columns or rows first";
const NEED_ONE_COLUMN: &str = "Select exactly one column first";
const NEED_WINDOW: &str = "Activate a window first";
const NO_UNDO: &str = "Nothing to undo";
const NO_REDO: &str = "Nothing to redo";
const ANALYSIS_ON_BOOK: &str = "The active layer has no data plot to analyse: plot a worksheet column first";

/// Which menu bar is shown.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WindowKind {
    /// No window is open or active.
    None,
    Workbook,
    Graph,
}

/// A command of Sheaf that a menu item runs. It is turned into an [`Action`] with the state of
/// the program by [`resolve`].
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    NewProject,
    NewWorkbook,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    /// File > Import > Single ASCII (Ctrl+K).
    Import,
    ImportMultiple,
    ImportCsv,
    ImportWizard,
    /// Edit > Go to... (Ctrl+G, in a workbook; in a graph the same keys export).
    GoToCell,
    /// Edit > Find in Sheets... (Ctrl+F) in a workbook.
    FindInSheets,
    /// Edit > Replace... (Ctrl+H) in a workbook.
    ReplaceInSheets,
    ExportGraph,
    ExportWorksheet,
    Exit,
    Undo,
    Redo,
    DeleteColumns,
    /// Column > Move Columns.
    MoveColumns(MoveTo),
    /// Column > Hide/Unhide Columns > Hide Selected.
    HideColumns,
    /// Column > Hide/Unhide Columns > Unhide All.
    UnhideColumns,
    /// Column > Mask > Apply, Remove, Swap.
    Mask(MaskOp),
    /// Column > Set as Categorical.
    SetCategorical,
    /// Worksheet > Transpose, Stack, Unstack, Split Columns, Reduce Rows.
    Reshape(ReshapeKind),
    /// Column > Filter > Add or Remove Filter, Enable/Disable Filter, Reapply Filter.
    RowFilterDialog,
    ToggleRowFilter,
    ReapplyRowFilter,
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
    /// Plot > Symbol > Y Error.
    PlotYError,
    /// Plot > Symbol > XY Error: a scatter plot with the X and Y error columns of each Y column.
    PlotXYError,
    /// Plot > Multi-Panel > a fixed grid of panels (Vertical 2 Panel is 2 x 1, 4 Panel 2 x 2).
    PlotPanels {
        rows: usize,
        cols: usize,
    },
    /// Graph > Add Error Bars... on the active layer.
    AddErrorBars,
    /// Window > Refresh (F5).
    Refresh,
    SetAs(Designation),
    SetColumnValues,
    Recalculate,
    AddColumns,
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
    /// The graph settings of the active layer on a tab.
    GraphProps(PropsTab),
    /// The graph settings of a given layer, with a plot chosen (the graph's right-click menus).
    GraphPropsAt {
        layer: usize,
        tab: PropsTab,
        plot: Option<usize>,
    },
    /// A change made from a graph's right-click menus.
    GraphEdit(GraphOp),
    /// Rescale to Show All on the active layer (Ctrl+R).
    Rescale,
    /// Graph > Exchange X-Y Axes on the active layer.
    ExchangeAxes,
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
    /// The Text tool of the Tools toolbar.
    TextTool,
    /// A drawing tool of the Tools toolbar.
    ShapeTool(sheaf_core::shape::ShapeKind),
    /// Properties of a drawn object.
    EditShape {
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
    /// Edit > Copy Page: the graph as a picture on the clipboard.
    CopyPage,
    /// Gadgets > Integrate: starts the Integrate gadget on the active plot.
    IntegrateGadget,
    /// A command of the open gadget's right-click menu.
    Gadget(GadgetOp),
    /// View > Toolbars: the window that shows and hides the bars.
    Toolbars,
    /// View > Status Bar: show or hide the status bar.
    ToggleStatusBar,
    /// File > Close: close (hide) the active window, as its title bar's close button does.
    CloseWindow,
    /// Window > Duplicate: a copy of the active workbook or graph.
    DuplicateWindow,
    /// File > Append...: bring the windows of another project into this one.
    AppendProject,
    /// View > Project Explorer and the other panels.
    Panel(PanelId),
    Cascade,
    TileHorizontally,
    TileVertically,
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
    /// True when the active workbook has selected cells, columns or rows (Copy, Cut, Paste and
    /// Clear act on them).
    pub cells: bool,
    pub undo: Option<&'a str>,
    pub redo: Option<&'a str>,
    /// All windows of the project, in order, with their titles.
    pub windows: Vec<(WindowId, String)>,
    /// A top-level menu to hold open (`--show menu:Plot`, for pictures of the menus).
    pub open: Option<&'a str>,
    /// Which side panels are shown, in the order of `PanelId::ALL` (the ticks of the View menu).
    pub panels: [bool; 4],
    /// The active layer of the active graph (0 when there is none).
    pub layer: usize,
    /// The active graph's tool (the Tools toolbar shows it pressed).
    pub tool: crate::actions::GraphTool,
    /// On a graph: the data the Analysis menu works on, as Origin's menu does on the active data
    /// plot: the workbook and Y column of the selected plot, or else of the active layer's first
    /// visible plot of numbers. `None` when the graph has no such plot.
    pub curve: Option<(WindowId, ColumnId)>,
    /// On a graph: the Y columns of the active layer's visible plots of numbers that sit in the same
    /// worksheet as `curve`, the active plot first, then the others in plot order. The tools that
    /// compare curves (Average Multiple Curves, Intersect) start from these.
    pub layer_curves: Vec<ColumnId>,
}

// ----- the table ---------------------------------------------------------------------------

/// The menus of a bar, in order.
pub fn bar(kind: WindowKind) -> &'static [Menu] {
    static BARS: OnceLock<[Vec<Menu>; 3]> = OnceLock::new();
    let bars = BARS.get_or_init(|| [build(WindowKind::None), build(WindowKind::Workbook), build(WindowKind::Graph)]);
    &bars[match kind {
        WindowKind::None => 0,
        WindowKind::Workbook => 1,
        WindowKind::Graph => 2,
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

fn file_menu(kind: WindowKind) -> Vec<Entry> {
    let mut v = vec![
        sub(
            "&New",
            [
                vec![keyed("&Project", ctrl(Key::N), Cmd::NewProject), item("&Worksheet...", Cmd::NewWorkbook)],
                greys(&["Matrix...", "Graph", "Image", "From Template", "Excel", "Layout", "Notes", "Function Plot"]),
            ]
            .concat(),
        ),
        keyed("&Open...", ctrl(Key::O), Cmd::OpenProject),
        grey_keyed("Open &Excel...", ctrl(Key::E)),
        grey("Open Sample Projects"),
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
    }
    if kind != WindowKind::None {
        v.push(sep());
        v.push(grey("Save Window As..."));
        v.push(grey("Save Template As..."));
        if kind == WindowKind::Workbook {
            v.push(grey("Save Workbook As Analysis Template..."));
            v.push(grey("Add Word Bookmarks to Analysis Template..."));
        }
        v.push(grey("Batch Processing..."));
    }
    v.extend([
        sep(),
        grey("Save Project without Data..."),
        grey("Duplicate Project without Data"),
        sep(),
        grey_keyed("&Print...", ctrl(Key::P)),
        grey("Print Pre&view"),
        grey("Page Setup..."),
        sep(),
        sub(
            "&Import",
            [
                vec![
                    keyed("Import Wi&zard...", ctrl(Key::Num3), Cmd::ImportWizard),
                    keyed("Single &ASCII...", ctrl(Key::K), Cmd::Import),
                    item("&Multiple ASCII...", Cmd::ImportMultiple),
                    item("Comma &Delimited (CSV)...", Cmd::ImportCsv),
                ],
                greys(&["Excel (XLS, XLSX, XLSM)...", "Image...", "Binary 2D Array..."]),
                // Origin's graph File menu has no Re-Import items.
                if kind == WindowKind::Graph {
                    Vec::new()
                } else {
                    vec![sep(), grey_keyed("Re-Import Directly", ctrl(Key::Num4)), grey("Re-Import...")]
                },
                vec![grey("Recent Imports")],
            ]
            .concat(),
        ),
    ]);
    if kind == WindowKind::Graph {
        // Origin's graph menu has Export Graphs... where the workbook has the Export submenu.
        v.push(keyed("E&xport Graphs...", ctrl(Key::G), Cmd::ExportGraph));
        v.push(grey("Recent Exports"));
    } else {
        v.push(sub(
            "E&xport",
            [
                vec![item("&Graphs...", Cmd::ExportGraph), item("&Worksheet as ASCII...", Cmd::ExportWorksheet)],
                greys(&["Graph as Image...", "Worksheet as Image...", "Multi-Page PDF...", "Recent Exports"]),
            ]
            .concat(),
        ));
    }
    v.push(sep());
    v.push(grey("Recent Books"));
    if kind == WindowKind::Graph {
        v.push(grey("Recent Graphs"));
    }
    v.extend([grey("Recent Projects"), sep(), item("Exi&t", Cmd::Exit)]);
    v
}

fn edit_menu(kind: WindowKind) -> Vec<Entry> {
    let mut v = vec![
        keyed("&Undo", ctrl(Key::Z), Cmd::Undo),
        // Origin has no Redo; Sheaf keeps its own, with its own key.
        keyed("&Redo", ctrl_shift(Key::Z), Cmd::Redo),
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
                grey("Copy formula"),
                grey("Copy label rows"),
                grey("Copy formula and label rows"),
                grey("Copy All"),
            ],
        ));
        v.push(keyed("Paste", ctrl(Key::V), Cmd::Paste));
        v.extend(greys(&["Paste Transpose", "Paste Link", "Paste Link Transpose", "Paste Special", "-", "Select..."]));
        v.push(keyed("C&lear", plain_key(Key::Delete), Cmd::Clear));
        v.push(item("Insert", Cmd::InsertCells));
        v.push(item("&Delete", Cmd::DeleteCells));
        v.extend(greys(&["Remove Links", "-", "Set As Begin", "Set As End", "Reset to Full Range", "-"]));
        v.push(grey_keyed("Find in Project...", plain_key(Key::F3)));
        v.push(keyed("Find in Sheets...", ctrl(Key::F), Cmd::FindInSheets));
        v.push(keyed("R&eplace...", ctrl(Key::H), Cmd::ReplaceInSheets));
        v.push(keyed("Go to...", ctrl(Key::G), Cmd::GoToCell));
        v.extend(greys(&["-", "Button Edit Mode"]));
    } else {
        v.push(grey("Cu&t"));
        v.push(grey("Copy"));
        v.push(keyed("Copy Page", ctrl(Key::J), Cmd::CopyPage));
        v.extend(greys(&[
            "Paste",
            "Paste Link",
            "Paste Special",
            "Clear",
            "-",
            "Insert New Object",
            "Links",
            "Button Edit Mode",
            "Update Client",
        ]));
        v.push(grey_sub(
            "Copy Format",
            &[
                "All",
                "All Style Formats",
                "Colors",
                "Fonts",
                "Background",
                "Symbol/Line/Fill",
                "Dimensions",
                "Scales",
                "Text",
            ],
        ));
        v.extend(greys(&["Paste Format...", "-"]));
        v.push(grey_keyed("Find in Project", plain_key(Key::F3)));
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
    v.push(grey_sub("View Windows", &["None", "Windows in Active Folder", "Active Folder and Subfolders"]));
    v.push(sep());
    v.push(keyed("Results Log", alt(Key::Num2), Cmd::Panel(PanelId::Results)));
    v.push(keyed("Messages Log", alt(Key::Num6), Cmd::Panel(PanelId::Messages)));
    v.push(grey_keyed("Smart Hint Log", alt(Key::Num7)));
    match kind {
        WindowKind::Workbook => v.extend(greys(&["-", "Actively Update Plots", "Page Break Preview Lines"])),
        WindowKind::Graph => {
            v.push(sep());
            v.push(grey_keyed("Print View", ctrl(Key::Num2)));
            v.push(grey_keyed("Page View", ctrl(Key::Num1)));
            v.push(grey("Window View"));
            v.push(grey_keyed("Draft View", ctrl(Key::Num0)));
            v.push(sep());
            v.push(grey_keyed("Zoom In", ctrl(Key::I)));
            v.push(grey_keyed("Zoom Out", ctrl(Key::M)));
            v.push(grey_keyed("Whole Page", ctrl(Key::W)));
            v.push(grey("Zoom All"));
            v.push(sep());
            v.push(grey_sub(
                "Show",
                &[
                    "Layer Icons",
                    "Active Layer Indicator",
                    "Axis Layer Icons",
                    "Object Grid",
                    "Layer Grid",
                    "Frame",
                    "Labels",
                    "Data",
                    "Active Layer Only",
                    "Master Items on Screen",
                ],
            ));
            v.push(grey("Show Data Info"));
            v.push(grey_keyed("Full Screen", ctrl_shift(Key::J)));
        }
        WindowKind::None => {}
    }
    v
}

fn plot_menu() -> Vec<Entry> {
    let with = |first: Vec<Entry>, rest: &[&str]| [first, greys(rest)].concat();
    vec![
        sub(
            "&Line",
            with(
                vec![item("&Line", Cmd::Plot(Template::Line))],
                &["Horizontal Step", "Vertical Step", "Spline Connected"],
            ),
        ),
        sub(
            "&Symbol",
            [
                vec![item("&Scatter", Cmd::Plot(Template::Scatter)), grey("Scatter Central"), grey("Column Scatter")],
                vec![item("Y Error", Cmd::PlotYError), item("XY Error", Cmd::PlotXYError)],
                greys(&["Vertical Drop Line"]),
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
            with(
                vec![item("Line + &Symbol", Cmd::Plot(Template::LineSymbol))],
                &["Line Series", "2 Point Segment", "3 Point Segment"],
            ),
        ),
        sub(
            "Column/Bar/&Pie",
            [
                vec![item("&Column", Cmd::Plot(Template::Column))],
                greys(&["Column + Label", "Grouped Columns (indexed data)"]),
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
            with(
                vec![item("&Double-Y", Cmd::Plot(Template::DoubleY))],
                &["3Ys Y-YY", "3Ys Y-Y-Y", "4Ys Y-YYY", "4Ys YY-YY", "Multiple Y Axes..."],
            ),
        ),
        sub(
            "Y-offset/&Waterfall",
            with(
                vec![
                    item("&Stacked Lines by Y Offsets", Cmd::Plot(Template::StackedLines)),
                    item("&Waterfall", Cmd::Plot(Template::Waterfall)),
                ],
                &["Waterfall Y:Color Mapping", "Waterfall Z:Color Mapping", "3D Waterfall"],
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
                greys(&["Stack...", "Multiple Panels by Label..."]),
            ]
            .concat(),
        ),
        sep(),
        grey_sub("3D &XYY", &["XYY 3D Bars", "3D Ribbons", "3D Walls", "3D Waterfall"]),
        grey_sub(
            "3D Su&rface",
            &[
                "Color Fill Surface",
                "X Constant with Base",
                "Y Constant with Base",
                "Color Map Surface",
                "Colormap Surface with Projection",
                "Wire Frame",
                "Wire Surface",
                "3D Ternary Colormap Surface",
            ],
        ),
        grey_sub(
            "3D Symbol/Bar/&Vector",
            &[
                "3D Bars",
                "3D Scatter",
                "3D Trajectory",
                "3D Scatter + Error Bar",
                "3D Vector XYZ XYZ",
                "3D Vector XYZ dXdYdZ",
            ],
        ),
        sep(),
        sub(
            "S&tatistics",
            [
                vec![item("Box Chart", Cmd::Plot(Template::BoxChart))],
                greys(&["Grouped Box Charts", "Interval Plot"]),
                vec![item("Histogram", Cmd::Plot(Template::Histogram))],
                greys(&[
                    "Histogram + Probabilities",
                    "Stacked Histograms",
                    "Marginal Histograms",
                    "Marginal Box Charts",
                    "2D Kernel Density",
                    "QC (X bar R) Chart",
                    "Pareto",
                    "Scatter Matrix",
                    "Probability Plot",
                    "Q-Q Plot",
                ]),
            ]
            .concat(),
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
        grey_sub(
            "Contour/&Heat Map",
            &[
                "Color Fill",
                "B/W Lines + Labels",
                "Gray Scale Map",
                "Heat Map",
                "Polar Contour theta(X) r(Y)",
                "Polar Contour r(X) theta(Y)",
                "Ternary Contour",
            ],
        ),
        grey_sub("Pro&file", &["Contour Profiles", "Image Profiles"]),
        sub(
            "Specialize&d",
            [
                with(
                    vec![
                        item("Polar theta(X) r(Y)", Cmd::Plot(Template::Polar)),
                        item("Polar r(X) theta(Y)", Cmd::Plot(Template::PolarRTheta)),
                    ],
                    &["Wind Rose"],
                ),
                vec![item("Ternary", Cmd::Plot(Template::Ternary))],
                greys(&["Piper", "Smith Chart"]),
                vec![item("Radar", Cmd::Plot(Template::Radar))],
                greys(&["High-Low-Close", "Vector XYAM", "Vector XYXY", "Zoom"]),
            ]
            .concat(),
        ),
        grey_sub("Stoc&k", &["High-Low-Close", "Japanese Candlestick", "OHLC Bar Chart", "OHLC-Volume", "Line"]),
        sep(),
        grey("System Templates..."),
        // Sheaf's own graph style is the one template that has no Origin counterpart.
        sub("&User Templates", vec![item("Cyclic voltammetry overlay", Cmd::Plot(Template::CyclicVoltammetry))]),
        sep(),
        grey("Most Recently Used"),
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
        grey_keyed("Set Multiple Columns Values...", ctrl_shift(Key::Q)),
        // Origin has this as a toolbar button and in the column's right-click menu.
        item("Re&calculate", Cmd::Recalculate),
        grey_sub(
            "Fill Column With",
            &[
                "Row Numbers",
                "Uniform Random Numbers",
                "Normal Random Numbers",
                "A set of Numbers...",
                "A set of Date/Time values...",
                "Arbitrary set of Text&&Numeric values...",
            ],
        ),
        sep(),
        grey("Set Sampling Interval..."),
        grey("Show X Column..."),
        sep(),
        keyed("Add &New Columns...", ctrl(Key::D), Cmd::AddColumns),
        hide_menu(),
        sep(),
        filter_menu(),
        mask_menu(&["Change Color", "Disable Masking", "Mask Cells by Condition..."]),
    ]
}

/// The Mask submenu: Apply, Remove and Swap work on the selected cells; the rest is greyed.
fn mask_menu(grey_items: &[&str]) -> Entry {
    let mut entries = vec![
        item("Apply", Cmd::Mask(MaskOp::Apply)),
        item("Remove", Cmd::Mask(MaskOp::Remove)),
        item("Swap", Cmd::Mask(MaskOp::Swap)),
    ];
    entries.extend(greys(grey_items));
    sub("Mask", entries)
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
                item("Ascending", Cmd::SortColumns { descending: false }),
                item("Descending", Cmd::SortColumns { descending: true }),
                grey("Custom..."),
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
        item("Clear Worksheet...", Cmd::ClearWorksheet),
        item("Split Columns...", Cmd::Reshape(ReshapeKind::Split)),
        grey("Pivot Table..."),
        item("Reduce Rows...", Cmd::Reshape(ReshapeKind::Reduce)),
        item("Transpose...", Cmd::Reshape(ReshapeKind::Transpose)),
        item("Stack Columns...", Cmd::Reshape(ReshapeKind::Stack)),
        item("Unstack Columns...", Cmd::Reshape(ReshapeKind::Unstack)),
        grey_sub("Convert to Matrix", &["Direct", "XYZ Gridding", "Extract"]),
        sep(),
        grey("Worksheet Script..."),
        // Sheaf's "keep only the rows matching a condition" is Origin's worksheet query.
        item("Worksheet &Query...", Cmd::FilterRows),
        grey("Most Recently Used Items"),
    ]
}

fn analysis_menu() -> Vec<Entry> {
    vec![
        sub(
            "&Mathematics",
            vec![
                item("Simple Curve Math...", Cmd::Analyze(AnalysisKind::CurveMath)),
                grey("Simple Column Math..."),
                sep(),
                item("Differentiate...", Cmd::Analyze(AnalysisKind::Differentiate)),
                item("Integrate...", Cmd::Analyze(AnalysisKind::Integrate)),
                // Sheaf's own: integrate several ranges in one go.
                item("Integrate Ranges...", Cmd::Analyze(AnalysisKind::IntegrateRanges)),
                sep(),
                item("Interpolate/Extrapolate...", Cmd::Analyze(AnalysisKind::Interpolate)),
                item("Interpolate/Extrapolate Y from X...", Cmd::InterpolateFromColumn),
                grey("Trace Interpolation..."),
                item("Linear Interpolation...", Cmd::InterpolateWith(sheaf_core::analysis::InterpMethod::Linear)),
                item("Spline Interpolation...", Cmd::InterpolateWith(sheaf_core::analysis::InterpMethod::CubicSpline)),
                item("B-Spline Interpolation...", Cmd::InterpolateWith(sheaf_core::analysis::InterpMethod::BSpline)),
                sep(),
                item("Intersect...", Cmd::Analyze(AnalysisKind::Intersect)),
                item("Normalize Columns...", Cmd::Analyze(AnalysisKind::Normalize)),
                item("Average Multiple Curves...", Cmd::Analyze(AnalysisKind::AverageCurves)),
                item("Polygon Area...", Cmd::Analyze(AnalysisKind::PolygonArea)),
                grey("XYZ Surface Area..."),
                item("Median Filter...", Cmd::Analyze(AnalysisKind::MedianFilter)),
                grey("Special Filter..."),
                grey("Inverse..."),
            ],
        ),
        sub(
            "&Data Manipulation",
            vec![
                item("Subtract Reference Data...", Cmd::Analyze(AnalysisKind::SubtractRef)),
                item("Subtract Straight Line...", Cmd::Analyze(AnalysisKind::SubtractLine)),
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
                sep(),
                keyed("Nonlinear Curve Fit...", ctrl(Key::Y), Cmd::Fit(FitKind::BuiltIn)),
                grey("Nonlinear Implicit Curve Fit..."),
                grey("Nonlinear Surface Fit..."),
                sep(),
                grey("Simulate Curve..."),
                grey("Simulate Surface..."),
                sep(),
                item("Exponential Fit...", Cmd::Fit(FitKind::Exponential)),
                item("Single Peak Fit...", Cmd::Fit(FitKind::SinglePeak)),
                item("Sigmoidal Fit...", Cmd::Fit(FitKind::Sigmoidal)),
                sep(),
                grey("Compare Datasets..."),
                item("Compare Models...", Cmd::FitMore(FitMoreKind::Compare)),
                item("Rank Models...", Cmd::FitMore(FitMoreKind::Rank)),
            ],
        ),
        sub(
            "Si&gnal Processing",
            vec![
                item("Smooth...", Cmd::Analyze(AnalysisKind::Smooth)),
                grey("FFT Filters..."),
                grey_sub("FFT", &["FFT...", "IFFT...", "2D FFT...", "2D IFFT..."]),
                grey("Convolution..."),
                grey("Deconvolution..."),
                grey("Correlation..."),
                grey("Coherence..."),
                grey("Decimation..."),
                grey("Envelope..."),
                grey("Hilbert Transform..."),
                grey("IIR Filter..."),
                grey("STFT..."),
                grey_sub(
                    "Wavelet",
                    &["Decompose...", "Reconstruction...", "Denoise...", "Smooth...", "Continuous Wavelet..."],
                ),
            ],
        ),
        sub(
            "&Peaks and Baseline",
            vec![
                item("Peak Analyzer...", Cmd::Analyze(AnalysisKind::PeakChain)),
                item("Find Peaks...", Cmd::Analyze(AnalysisKind::Peaks)),
                item("Subtract Baseline...", Cmd::Analyze(AnalysisKind::Baseline)),
                grey("Multiple Peak Fit"),
                grey("Batch Peak Analysis Using Theme..."),
            ],
        ),
        sep(),
        grey("Most Recently Used Items"),
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
                grey("Cross Tabulation..."),
                sep(),
                grey("Discrete Frequency..."),
                item("Frequency Counts...", Cmd::Analyze(AnalysisKind::Histogram)),
                grey("2D Frequency Counts/Binning..."),
                sep(),
                grey("Distribution Fit..."),
                item("Normality Test...", Cmd::Analyze(AnalysisKind::Normality)),
                sep(),
                item("Correlation Coefficient...", Cmd::Analyze(AnalysisKind::Correlation)),
                grey("Partial Correlation Coefficient..."),
                sep(),
                item("Grubbs Test...", Cmd::Analyze(AnalysisKind::Grubbs)),
                item("Dixon's Q-test...", Cmd::Analyze(AnalysisKind::Dixon)),
            ],
        ),
        sub(
            "&Hypothesis Testing",
            vec![
                item("One-Sample t-Test...", Cmd::Analyze(AnalysisKind::TTest)),
                item("Two-Sample t-Test...", Cmd::Analyze(AnalysisKind::TTest)),
                item("Paired Sample t-Test...", Cmd::Analyze(AnalysisKind::TTest)),
            ],
        ),
        sub(
            "AN&OVA",
            vec![
                item("One-Way ANOVA...", Cmd::Analyze(AnalysisKind::Anova)),
                item("Two-Way ANOVA...", Cmd::Analyze(AnalysisKind::Anova2)),
                grey("Three-Way ANOVA..."),
                grey("Repeated Measures ANOVA..."),
            ],
        ),
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
        grey_sub("&Survival Analysis", &["Kaplan-Meier Estimator..."]),
        grey_sub("&Multivariate Analysis", &["Principal Component Analysis...", "K-Means Cluster Analysis..."]),
        grey_sub("&Power and Sample Size", &["Power and Sample Size..."]),
        sep(),
        grey("&ROC Curve..."),
        sep(),
        grey("Most Recently Used Items"),
    ]
}

fn image_menu() -> Vec<Entry> {
    vec![
        grey_sub("&Adjustments", &["Brightness...", "Contrast...", "Gamma...", "Levels..."]),
        grey_sub("Arithmetic &Transform", &["Math...", "Subtract..."]),
        grey_sub("&Conversion", &["To Binary...", "Color to Gray...", "RGB Split..."]),
        grey_sub("&Geometric Transform", &["Crop...", "Resize...", "Rotate...", "Flip..."]),
        grey_sub("&Spatial Filters", &["Smooth...", "Sharpen...", "Edge Detection..."]),
    ]
}

fn graph_menu() -> Vec<Entry> {
    vec![
        // Sheaf's Plots tab does what Layer Contents and Plot Setup do: add and remove plots.
        keyed("Layer Contents...", plain_key(Key::F12), Cmd::GraphProps(PropsTab::Plots)),
        item("Plot Setup...", Cmd::GraphProps(PropsTab::Plots)),
        grey_sub("Add Plot to Layer", &["Line...", "Scatter...", "Line + Symbol...", "Column...", "Area..."]),
        sep(),
        grey("Layer Management..."),
        item("Add Error Bars...", Cmd::AddErrorBars),
        grey("Add Function Graph..."),
        grey("Add Straight Line..."),
        item("Add Text...", Cmd::AddText),
        grey_sub("New Layer(Axes)", &["Horizontal Axis Layer...", "Vertical Axis Layer..."]),
        grey("Merge Graph Windows..."),
        sep(),
        keyed("Rescale to Show All", ctrl(Key::R), Cmd::Rescale),
        sub(
            "&Legend",
            vec![
                // Origin opens the legend's settings by double click; Sheaf also offers them here.
                item("Legend Properties...", Cmd::GraphProps(PropsTab::Legend)),
                keyed("Reconstruct Legend", ctrl(Key::L), Cmd::ReconstructLegend),
                item("Update Legend", Cmd::UpdateLegend),
                grey("Add Legend Title"),
            ],
        ),
        grey("New Table"),
        grey("New Color Scale"),
        sep(),
        grey("Fit Layer to Graph"),
        grey("Fit Page to Layers..."),
        item("Exchange X-Y Axes", Cmd::ExchangeAxes),
        grey_sub("Offset Grouped Data in Layer", &["None", "Cumulative", "Constant", "Auto", "Individual"]),
        grey("Convert to Standard Font Size"),
    ]
}

fn data_menu() -> Vec<Entry> {
    vec![
        grey("Set Display Range"),
        grey("Reset to Full Range"),
        sep(),
        grey_keyed("Mark Data Range", ctrl_alt(Key::M)),
        grey_keyed("Clear Data Markers", ctrl_alt(Key::N)),
        sep(),
        grey_sub("Analysis Markers", &["Hide", "Show Partial Ranges", "Show All", "Tiny", "Small", "Medium", "Large"]),
        grey_sub("Lock Position", &["Layer Icon", "First Point", "Last Point"]),
        sep(),
        grey("Mask Data Points"),
        grey("Move Data Points"),
        grey("Remove Bad Data Points"),
    ]
}

fn gadgets_menu() -> Vec<Entry> {
    let mut v = greys(&["Statistics...", "Quick Fit...", "Quick Sigmoidal Fit...", "Quick Peaks..."]);
    v.push(item("Integrate...", Cmd::IntegrateGadget));
    v.extend(greys(&[
        "Differentiate...",
        "Interpolate...",
        "Intersect...",
        "Rise Time...",
        "Cluster...",
        "FFT...",
        "2D Integrate...",
        "-",
        "Vertical Cursor...",
        "Go To...",
    ]));
    v
}

/// The right-click menu of the Integrate gadget.
pub fn integrate_gadget_menu(end_point_baseline: bool) -> Vec<Entry> {
    let baseline = if end_point_baseline { "Baseline: Zero" } else { "Baseline: Line Through End Points" };
    vec![
        item(baseline, Cmd::Gadget(GadgetOp::ToggleBaseline)),
        item("New Output", Cmd::Gadget(GadgetOp::NewOutput)),
        sep(),
        keyed("Delete", plain_key(Key::Escape), Cmd::Gadget(GadgetOp::Close)),
    ]
}

fn tools_menu() -> Vec<Entry> {
    vec![
        grey_keyed("&Options...", ctrl(Key::U)),
        grey("System Variables..."),
        grey("3D OpenGL Settings..."),
        grey_sub("Protection", &["Protect Project...", "Audit Log"]),
        grey("Apps on File Exchange..."),
        sep(),
        grey_keyed("Fitting Function Builder...", plain_key(Key::F8)),
        grey_keyed("Fitting Function Organizer...", plain_key(Key::F9)),
        grey_keyed("Theme Organizer...", plain_key(Key::F7)),
        grey("Import Filters Manager..."),
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
            grey("&Layer Properties..."),
            item("Plot P&roperties...", Cmd::GraphProps(PropsTab::Plots)),
            sep(),
            grey("Snap Layers to Grid"),
            grey("Snap Objects to Grid"),
            grey("Fit Page To Layers"),
            grey("Fit Page To Printer"),
            sep(),
            item("&Axes...", Cmd::GraphProps(PropsTab::XAxis)),
            sub(
                "Axe&s",
                vec![
                    item("&X Axis...", Cmd::GraphProps(PropsTab::XAxis)),
                    item("&Y Axis...", Cmd::GraphProps(PropsTab::YAxis)),
                    grey("&Z Axis..."),
                ],
            ),
            grey_sub("Axis Tick Labels", &["X Axis Tick Labels...", "Y Axis Tick Labels..."]),
            grey_sub("Axis Titles", &["X Axis Title...", "Y Axis Title..."]),
            sep(),
            grey("Programming Control..."),
            grey("Object Properties..."),
        ],
        _ => vec![
            grey("Menu"),
            grey_keyed("&Worksheet...", plain_key(Key::F4)),
            item("&Column...", Cmd::ColumnProperties),
            grey("Cell..."),
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
        grey("Arrange &Icons"),
        sep(),
        keyed("&Refresh", plain_key(Key::F5), Cmd::Refresh),
        sep(),
        item("&Duplicate", Cmd::DuplicateWindow),
    ];
    match kind {
        WindowKind::Graph => {
            v.push(grey_sub("Duplicate (&Batch Plotting)", &["With New Sheets/Books", "With New Columns"]))
        }
        WindowKind::Workbook => v.push(grey("Sp&lit")),
        WindowKind::None => {}
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
        grey_sub("&Folders", &["Folders List"]),
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
        grey_sub("Open Sample Projects", &["Sample Projects"]),
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
    v.push(grey("Copy Columns to..."));
    v.push(keyed("Paste", ctrl(Key::V), Cmd::Paste));
    v.push(sep());
    v.push(item("Insert", Cmd::InsertCells));
    v.push(item("Delete", Cmd::DeleteColumns));
    v.push(keyed("Clear", plain_key(Key::Delete), Cmd::Clear));
    v.push(grey("Remove Links"));
    v.push(sep());
    v.push(sub("Set As", set_as));
    v.push(item("Set As Categorical", Cmd::SetCategorical));
    v.push(sep());
    v.push(keyed("Set Column Values...", ctrl(Key::Q), Cmd::SetColumnValues));
    v.push(grey_keyed("Set Multiple Columns Values...", ctrl_shift(Key::Q)));
    v.push(item("Recalculate", Cmd::Recalculate));
    v.push(item("Add New Columns...", Cmd::AddColumns));
    v.push(sep());
    v.push(grey_sub(
        "Fill Column with",
        &[
            "Row Numbers",
            "Uniform Random Numbers",
            "Normal Random Numbers",
            "A set of Numbers...",
            "A set of Date/Time values...",
            "Arbitrary set of Text&&Numeric values...",
        ],
    ));
    v.extend(greys(&["Set Sampling Interval...", "Show X Column...", "-"]));
    v.push(filter_menu());
    v.push(mask_menu(&["Change Color", "Disable Masking"]));
    v.push(grey("Mask Cells by Condition..."));
    v.push(sep());
    v.push(sub(
        "Sort Column",
        vec![
            item(order[0], Cmd::SortColumns { descending: false }),
            item(order[1], Cmd::SortColumns { descending: true }),
            grey(order[2]),
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
    v.push(grey("Frequency Count..."));
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
    v.push(grey("Swap Columns..."));
    v.push(item("Column Width...", Cmd::ColumnProperties));
    v.extend(greys(&["-", "Slide Show of Dependent Graphs"]));
    v.extend(greys(&["Add or Update Sparklines...", "-", "Format Cells..."]));
    v.push(item("Properties...", Cmd::ColumnProperties));
    v
}

/// The menu of a cell.
pub fn cell_menu() -> Vec<Entry> {
    let mut v = vec![keyed("Cut", ctrl(Key::X), Cmd::Cut)];
    v.extend(copy_entries());
    v.push(keyed("Paste", ctrl(Key::V), Cmd::Paste));
    v.extend(greys(&["Paste Transpose", "Paste Link", "Paste Link Transpose", "-"]));
    v.push(item("Insert", Cmd::InsertCells));
    v.push(item("Delete", Cmd::DeleteCells));
    v.push(keyed("Clear", plain_key(Key::Delete), Cmd::Clear));
    v.extend(greys(&["Remove Links", "-", "Set as Begin", "Set as End", "-"]));
    v.push(grey_sub("Insert Arrow", &["Arrow", "Double Arrow"]));
    v.extend(greys(&[
        "Insert Graph...",
        "Insert Images from Files...",
        "Insert Notes",
        "Insert Sparklines...",
        "Insert Variables...",
        "-",
        "Format Cells...",
    ]));
    v.push(mask_menu(&[]));
    v.push(grey_sub("Set Data Style", &["Default"]));
    v
}

/// The menu of a row number: Origin's short form, the same items as the cell menu starts with.
pub fn row_menu() -> Vec<Entry> {
    let mut v = vec![keyed("Cut", ctrl(Key::X), Cmd::Cut), keyed("Copy", ctrl(Key::C), Cmd::Copy(CopyKind::Plain))];
    v.push(keyed("Paste", ctrl(Key::V), Cmd::Paste));
    v.push(item("Insert", Cmd::InsertCells));
    v.push(item("Delete", Cmd::DeleteCells));
    v.push(keyed("Clear", plain_key(Key::Delete), Cmd::Clear));
    v.extend(greys(&["Insert Graph...", "Format Cells..."]));
    v
}

/// The menu of a sheet tab (it acts on the sheet whose tab was clicked, which becomes the
/// active one first).
pub fn sheet_tab_menu() -> Vec<Entry> {
    vec![
        item("Insert", Cmd::Sheet(SheetOp::Insert)),
        item("Delete", Cmd::Sheet(SheetOp::Delete)),
        item("Add", Cmd::Sheet(SheetOp::Add)),
        grey("Add Graph as Sheet..."),
        grey("Add Notes as Sheet"),
        sep(),
        grey("Duplicate Without Data"),
        item("Duplicate", Cmd::Sheet(SheetOp::Duplicate)),
        sep(),
        grey_sub("Sheet Name to Show", &["Short Name", "Long Name", "Both"]),
        item("Name and Comments...", Cmd::Sheet(SheetOp::RenameDialog)),
        sep(),
        grey("Navigate..."),
    ]
}

// ----- graph window menus (gui-look.md section 4) ----------------------------------------------

/// The menu of a plot: what was picked, so the commands act on it. `book` is the workbook the
/// plot's data is in, with its title.
pub fn graph_plot_menu(
    layer: usize,
    plot: usize,
    kind: PlotKind,
    on_right: bool,
    book: Option<(WindowId, String)>,
) -> Vec<Entry> {
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
        grey("Change X"),
        grey("Change Y"),
        sep(),
        item(
            if on_right { "Move to Left Y Axis" } else { "Move to Right Y Axis" },
            Cmd::GraphEdit(GraphOp::MovePlotToAxis { layer, plot, right: !on_right }),
        ),
        item("Hide Data Plot", Cmd::GraphEdit(GraphOp::HidePlot { layer, plot })),
    ];
    match book {
        Some((id, name)) => v.push(item(&format!("Go to {name}"), Cmd::GoTo(id))),
        None => v.push(grey("Go to Book")),
    }
    v.extend([
        sep(),
        grey("Ungroup"),
        sep(),
        item("Plot Details...", Cmd::GraphPropsAt { layer, tab: PropsTab::Plots, plot: Some(plot) }),
    ]);
    v
}

/// The menu of an axis. `grids` tells whether the axis shows grid lines now.
pub fn graph_axis_menu(layer: usize, y: bool, grids: bool) -> Vec<Entry> {
    let tab = if y { PropsTab::YAxis } else { PropsTab::XAxis };
    let at = Cmd::GraphPropsAt { layer, tab, plot: None };
    vec![
        item("Hide Axis", Cmd::GraphEdit(GraphOp::HideAxis { layer, y })),
        item(if grids { "Hide Grids" } else { "Show Grids" }, Cmd::GraphEdit(GraphOp::ToggleGrids { layer, y })),
        sep(),
        keyed("Rescale To Show All", ctrl(Key::R), Cmd::GraphEdit(GraphOp::Rescale { layer })),
        item("Axis Zoom In", Cmd::GraphEdit(GraphOp::AxisZoom { layer, y, zoom_in: true })),
        item("Axis Zoom Out", Cmd::GraphEdit(GraphOp::AxisZoom { layer, y, zoom_in: false })),
        grey("Center"),
        sep(),
        grey_sub("Copy Format", &["All", "All Style Formats", "Axis"]),
        grey("Save Format as Theme..."),
        sep(),
        item("Scale...", at.clone()),
        item("Tick Labels...", at.clone()),
        item("Properties...", at),
    ]
}

/// The menu of an axis title.
pub fn graph_title_menu(layer: usize, y: bool) -> Vec<Entry> {
    let tab = if y { PropsTab::YAxis } else { PropsTab::XAxis };
    vec![
        grey_keyed("Cut", ctrl(Key::X)),
        grey_keyed("Copy", ctrl(Key::C)),
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::HideTitle { layer, y })),
        sep(),
        grey("Bring to the front"),
        grey("Push to the back"),
        sep(),
        item("Properties...", Cmd::GraphPropsAt { layer, tab, plot: None }),
    ]
}

/// The menu of a free text label.
pub fn graph_label_menu(layer: usize, label: usize) -> Vec<Entry> {
    vec![
        grey_keyed("Cut", ctrl(Key::X)),
        grey_keyed("Copy", ctrl(Key::C)),
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::DeleteLabel { layer, label })),
        sep(),
        grey("Bring to the front"),
        grey("Push to the back"),
        sep(),
        item("Properties...", Cmd::EditText { layer, label }),
    ]
}

/// The menu of a drawn line, arrow, rectangle or ellipse.
pub fn graph_shape_menu(layer: usize, shape: usize) -> Vec<Entry> {
    vec![
        grey_keyed("Cut", ctrl(Key::X)),
        grey_keyed("Copy", ctrl(Key::C)),
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::DeleteShape { layer, shape })),
        sep(),
        grey("Bring to the front"),
        grey("Push to the back"),
        sep(),
        item("Properties...", Cmd::EditShape { layer, shape }),
    ]
}

/// The menu of the legend.
pub fn graph_legend_menu(layer: usize) -> Vec<Entry> {
    vec![
        item("Plot Details...", Cmd::GraphPropsAt { layer, tab: PropsTab::Plots, plot: None }),
        sub(
            "Legend",
            vec![
                item("Reconstruct Legend", Cmd::GraphEdit(GraphOp::ReconstructLegend { layer })),
                item("Update Legend", Cmd::GraphEdit(GraphOp::UpdateLegend { layer })),
                grey("Add Legend Title"),
            ],
        ),
        grey("Add Color Scale"),
        grey("Add Bubble Scale"),
        sep(),
        grey_keyed("Cut", ctrl(Key::X)),
        grey_keyed("Copy", ctrl(Key::C)),
        keyed("Delete", plain_key(Key::Delete), Cmd::GraphEdit(GraphOp::HideLegend { layer })),
        sep(),
        grey("Bring to the front"),
        grey("Push to the back"),
        grey("Bring to front of data"),
        grey("Push to back of data"),
        sep(),
        grey("Programming Control..."),
        item("Properties...", Cmd::GraphPropsAt { layer, tab: PropsTab::Legend, plot: None }),
        sep(),
        grey_sub("Copy Format", &["All", "All Style Formats", "Text"]),
        grey("Save Format as Theme..."),
    ]
}

/// The menu of the page outside the plots and axes, or of the inside of a layer.
pub fn graph_page_menu(layer: usize) -> Vec<Entry> {
    vec![
        item("Add Text...", Cmd::AddText),
        grey("Layer Management..."),
        grey_sub("New Layer(Axes)", &["Horizontal Axis Layer...", "Vertical Axis Layer..."]),
        grey("Add Shortcut to Favorites"),
        keyed("Copy Page", ctrl(Key::J), Cmd::CopyPage),
        item("Export Graphs...", Cmd::ExportGraph),
        grey_keyed("View Full Screen", ctrl_shift(Key::J)),
        sep(),
        keyed("Rescale To Show All", ctrl(Key::R), Cmd::GraphEdit(GraphOp::Rescale { layer })),
        item("Page Properties...", Cmd::GraphPropsAt { layer, tab: PropsTab::Page, plot: None }),
        grey_sub("Copy Format", &["All", "All Style Formats"]),
        grey_sub("Duplicate (Batch Plotting)", &["Duplicate"]),
    ]
}

/// The menu of a layer icon (the numbered button at the top left of the page). `plots` names the
/// layer's plots in order; `active` is the active plot, which gets a tick.
pub fn layer_icon_menu(layer: usize, layers: usize, plots: &[String], active: Option<usize>) -> Vec<Entry> {
    let others = if layers > 1 {
        item("Hide Other Layers", Cmd::GraphEdit(GraphOp::HideOtherLayers { layer }))
    } else {
        grey("Hide Other Layers")
    };
    let contents = Cmd::GraphPropsAt { layer, tab: PropsTab::Plots, plot: None };
    let mut v = vec![
        item("Hide Layer", Cmd::GraphEdit(GraphOp::HideLayer { layer })),
        others,
        grey("Show Active Layer Only"),
        grey("Set Active Layer By Layer Icon Only"),
        grey("Delete Layer"),
        sep(),
        keyed("Layer Contents...", plain_key(Key::F12), contents.clone()),
        item("Plot Setup...", contents),
        grey("Double click to open Plot Setup"),
        sep(),
        item("Axis...", Cmd::GraphPropsAt { layer, tab: PropsTab::XAxis, plot: None }),
        grey("Layer Properties..."),
        grey("Layer Management..."),
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

/// The menu of the corner above the row numbers: switches for the parts of the grid.
pub fn corner_menu() -> Vec<Entry> {
    vec![
        grey_sub(
            "Row Labels for Column Label Rows",
            &["Long Name", "Units", "Comments", "F(x)=", "Sparklines", "Parameters"],
        ),
        sep(),
        grey("Column Header"),
        grey("Row Header"),
        grey("Column Grid"),
        grey("Row Grid"),
        grey("Page Break Preview Lines"),
    ]
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
            show_entries(ui, entries, c, actions);
        },
    );
}

// ----- checks and helpers on the table --------------------------------------------------------

/// Calls `f` for every item of the entries, submenus included.
fn walk<'a>(entries: &'a [Entry], f: &mut dyn FnMut(&'a Item)) {
    for e in entries {
        match e {
            Entry::Item(i) => f(i),
            Entry::Submenu(s) => walk(&s.entries, f),
            Entry::Separator | Entry::Windows => {}
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
    Ok(match cmd {
        Cmd::NewProject => Action::NewProject,
        Cmd::NewWorkbook => Action::NewWorkbook,
        Cmd::OpenProject => Action::OpenProject,
        Cmd::SaveProject => Action::SaveProject,
        Cmd::SaveProjectAs => Action::SaveProjectAs,
        Cmd::Import => Action::ImportPick(ImportPick::Single),
        Cmd::ImportMultiple => Action::ImportPick(ImportPick::Multiple),
        Cmd::ImportCsv => Action::ImportPick(ImportPick::Csv),
        Cmd::ImportWizard => Action::ImportWizard,
        Cmd::GoToCell => Action::GoToCell(book?),
        Cmd::FindInSheets => Action::FindInSheets(book?),
        Cmd::ReplaceInSheets => Action::ReplaceInSheets(book?),
        Cmd::ExportGraph => Action::ExportGraph(c.graph.ok_or(NEED_GRAPH)?),
        Cmd::ExportWorksheet => Action::ExportData(book?),
        Cmd::Exit => Action::Quit,
        Cmd::Undo => {
            c.undo.ok_or(NO_UNDO)?;
            Action::Undo
        }
        Cmd::Redo => {
            c.redo.ok_or(NO_REDO)?;
            Action::Redo
        }
        Cmd::DeleteColumns => {
            book?;
            Action::DeleteColumns(some()?)
        }
        Cmd::MoveColumns(to) => {
            book?;
            Action::MoveColumns { columns: some()?, to: *to }
        }
        Cmd::HideColumns => {
            book?;
            Action::HideColumns(some()?)
        }
        Cmd::UnhideColumns => Action::UnhideColumns(book?),
        Cmd::Copy(kind) => {
            let book = book?;
            c.cells.then_some(()).ok_or(NEED_CELLS)?;
            Action::Cells { book, op: CellOp::Copy(*kind) }
        }
        Cmd::Cut | Cmd::Paste | Cmd::Clear | Cmd::InsertCells | Cmd::DeleteCells | Cmd::Mask(_) => {
            let book = book?;
            c.cells.then_some(()).ok_or(NEED_CELLS)?;
            let op = match cmd {
                Cmd::Cut => CellOp::Cut,
                Cmd::Paste => CellOp::Paste,
                Cmd::InsertCells => CellOp::Insert,
                Cmd::Mask(m) => CellOp::Mask(*m),
                Cmd::DeleteCells => CellOp::Delete,
                _ => CellOp::Clear,
            };
            Action::Cells { book, op }
        }
        Cmd::Sheet(op) => Action::Sheet { book: book?, op: op.clone() },
        Cmd::Plot(template) => Action::PlotColumns { book: book?, columns: c.selected.to_vec(), template: *template },
        Cmd::SetAs(designation) => {
            book?;
            Action::SetDesignation { columns: some()?, designation: *designation }
        }
        Cmd::SetColumnValues => {
            book?;
            Action::SetColumnValues(one()?)
        }
        Cmd::Recalculate => {
            book?;
            Action::Recalculate(some()?)
        }
        Cmd::AddColumns => Action::AddColumn(book?),
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
            Action::SortRange { columns: some()?, descending: *descending }
        }
        Cmd::SortColumns { descending } => {
            book?;
            Action::SortColumns { columns: some()?, descending: *descending }
        }
        Cmd::SortCustom { range } => {
            let book = book?;
            let columns = if *range { some()? } else { Vec::new() };
            Action::SortCustom { book, columns }
        }
        Cmd::SortByLabel => Action::SortByLabel(book?),
        Cmd::ClearWorksheet => Action::ClearWorksheet(book?),
        Cmd::SetCategorical => {
            book?;
            Action::SetCategorical(some()?)
        }
        Cmd::Reshape(kind) => Action::Reshape { book: book?, columns: c.selected.to_vec(), kind: *kind },
        Cmd::ReverseOrder => {
            book?;
            Action::ReverseColumns(some()?)
        }
        Cmd::PlotYError => Action::PlotYError { book: book?, columns: c.selected.to_vec(), x: false },
        Cmd::PlotXYError => Action::PlotYError { book: book?, columns: c.selected.to_vec(), x: true },
        Cmd::PlotPanels { rows, cols } => {
            Action::PlotPanels { book: book?, columns: c.selected.to_vec(), rows: *rows, cols: *cols }
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
        // On a graph the tools take the active data plot, as Origin's Analysis menu does; the
        // results go to its workbook (ADR 0005), and a fit draws its curve on the graph (ADR 0006).
        Cmd::Analyze(kind) => match (c.book, c.curve) {
            (Some(book), _) => Action::Analyze { book, columns: c.selected.to_vec(), kind: *kind },
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
        Cmd::FitMore(kind) => match (c.book, c.curve) {
            (Some(book), _) => Action::FitMore { book, column: c.selected.first().copied(), kind: *kind },
            (None, Some((book, y))) if c.kind == WindowKind::Graph => {
                Action::FitMore { book, column: Some(y), kind: *kind }
            }
            (None, _) if c.kind == WindowKind::Graph => return Err(ANALYSIS_ON_BOOK),
            (None, _) => return Err(NEED_BOOK),
        },
        Cmd::GraphProps(tab) => {
            Action::GraphProperties { graph: c.graph.ok_or(NEED_GRAPH)?, layer: c.layer, tab: *tab, plot: None }
        }
        Cmd::GraphPropsAt { layer, tab, plot } => {
            Action::GraphProperties { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, tab: *tab, plot: *plot }
        }
        Cmd::GraphEdit(op) => Action::GraphEdit { graph: c.graph.ok_or(NEED_GRAPH)?, op: *op },
        Cmd::Rescale => {
            Action::GraphEdit { graph: c.graph.ok_or(NEED_GRAPH)?, op: GraphOp::Rescale { layer: c.layer } }
        }
        Cmd::ExchangeAxes => {
            Action::GraphEdit { graph: c.graph.ok_or(NEED_GRAPH)?, op: GraphOp::ExchangeAxes { layer: c.layer } }
        }
        Cmd::ReconstructLegend => {
            Action::GraphEdit { graph: c.graph.ok_or(NEED_GRAPH)?, op: GraphOp::ReconstructLegend { layer: c.layer } }
        }
        Cmd::SelectPlot { layer, plot } => {
            Action::SelectPlot { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, plot: *plot }
        }
        Cmd::GoTo(id) => Action::Focus(*id),
        Cmd::Pointer => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Pointer },
        Cmd::DataReader => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Reader },
        Cmd::TextTool => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Text },
        Cmd::ShapeTool(kind) => Action::GraphTool { graph: c.graph.ok_or(NEED_GRAPH)?, tool: GraphTool::Draw(*kind) },
        Cmd::EditShape { layer, shape } => {
            Action::EditShape { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, shape: *shape }
        }
        Cmd::AddText => Action::AddText { graph: c.graph.ok_or(NEED_GRAPH)?, layer: Some(c.layer), at: None },
        Cmd::EditText { layer, label } => {
            Action::EditText { graph: c.graph.ok_or(NEED_GRAPH)?, layer: *layer, label: *label }
        }
        Cmd::CopyPage => Action::CopyPage(c.graph.ok_or(NEED_GRAPH)?),
        Cmd::IntegrateGadget => Action::StartIntegrate { graph: c.graph.ok_or(NEED_GRAPH)? },
        Cmd::Gadget(op) => Action::Gadget { graph: c.graph.ok_or(NEED_GRAPH)?, op: *op },
        Cmd::Toolbars => Action::Toolbars,
        Cmd::ToggleStatusBar => Action::ToggleStatusBar,
        Cmd::CloseWindow => Action::HideWindow(c.active.ok_or(NEED_WINDOW)?),
        Cmd::DuplicateWindow => Action::DuplicateWindow(c.active.ok_or(NEED_WINDOW)?),
        Cmd::AppendProject => Action::AppendProject(None),
        Cmd::Panel(p) => Action::TogglePanel(*p),
        Cmd::Cascade => Action::Cascade,
        Cmd::TileHorizontally => Action::TileHorizontally,
        Cmd::TileVertically => Action::TileVertically,
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
                if let (Some(keys), Some(cmd)) = (i.keys, &i.cmd)
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
                show_entries(ui, &menu.entries, c, actions);
            });
            if c.open == Some(menu.name().as_str()) {
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

fn show_entries(ui: &mut egui::Ui, entries: &[Entry], c: &MenuContext, actions: &mut Vec<Action>) {
    for entry in entries {
        match entry {
            Entry::Separator => {
                ui.separator();
            }
            Entry::Submenu(s) => {
                ui.menu_button(label_job(ui, &s.label, true), |ui| {
                    ui.set_min_width(160.0);
                    show_entries(ui, &s.entries, c, actions);
                });
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
                if response.contains_pointer() {
                    crate::panels::set_hint(ui.ctx(), &plain(&it.label));
                }
                match resolved {
                    Some(Ok(action)) => {
                        if response.clicked() {
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
                let shared =
                    matches!((cmd, cmd2), (Cmd::GoToCell, Cmd::ExportGraph) | (Cmd::ExportGraph, Cmd::GoToCell));
                assert!(k != k2 || cmd == cmd2 || shared, "{k:?} runs both {cmd:?} and {cmd2:?}");
            }
        }
        let find = |ks: KeyboardShortcut| keys.iter().find(|(k, _)| *k == ks).map(|(_, c)| c.clone());
        assert_eq!(find(ctrl(Key::K)), Some(Cmd::Import), "Ctrl+K imports, as in Origin");
        assert_eq!(find(ctrl(Key::Q)), Some(Cmd::SetColumnValues));
        assert_eq!(find(ctrl(Key::D)), Some(Cmd::AddColumns));
        assert_eq!(find(ctrl(Key::Y)), Some(Cmd::Fit(FitKind::BuiltIn)), "Ctrl+Y is Origin's, not redo");
        assert_eq!(find(ctrl(Key::N)), Some(Cmd::NewProject));
        assert_eq!(find(ctrl(Key::O)), Some(Cmd::OpenProject));
        assert_eq!(find(ctrl(Key::S)), Some(Cmd::SaveProject));
        assert_eq!(find(ctrl(Key::I)), None, "Ctrl+I is Zoom In in Origin, no longer Import");
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
            cells: false,
            undo: None,
            redo: None,
            windows: Vec::new(),
            open: None,
            panels: [true; 4],
            layer: 0,
            tool: Default::default(),
            curve: None,
            layer_curves: Vec::new(),
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
