//! The application window: menus, project explorer, the workspace with its
//! workbook and graph child windows, dialogs, and the one place where user actions change the
//! project.

use crate::actions::{
    Action, CellOp, CopyKind, DataEdit, DataTool, GraphOp, GraphTool, ImportPick, LabelRow, SheetOp, ViewOp,
};
use crate::autosave::{Autosave, Due};
use crate::dialogs::add_layer::LayerChoice;
use crate::dialogs::analysis::{AnalysisDialog, AnalysisKind, AnalysisOutcome, FilterDialog, FilterOutcome};
use crate::dialogs::autosave::{AutosaveDialog, AutosaveOutcome};
use crate::dialogs::cell_format::CellFormatDialog;
use crate::dialogs::cell_note::CellNoteDialog;
use crate::dialogs::column_props::ColumnPropsDialog;
use crate::dialogs::comparison_plot::{ComparisonPlotDialog, Outcome as ComparisonPlotOutcome};
use crate::dialogs::cond_format::CondFormatDialog;
use crate::dialogs::cond_manager::{CondManagerDialog, ManagerOutcome};
use crate::dialogs::cv_pick::{CvPickDialog, CvPickOutcome};

/// What Column > Add Column to All Sheets... edits (T359).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ColumnAcross {
    pub book: WindowId,
    pub long_name: String,
    pub units: String,
    pub formula: String,
    pub recalc: sheaf_core::column::RecalcMode,
    /// Every workbook of the project instead of this one.
    pub project: bool,
    /// Every workbook of this book's project folder and its subfolders (T412); wins over `project`.
    pub folder: bool,
    /// Insert right after this column of each sheet (T412); empty: at the end.
    pub after: String,
    /// The sheets chosen one by one (T418), as (workbook, sheet index); when set it wins over the other choices.
    pub chosen: Option<Vec<(WindowId, usize)>>,
    pub problem: Option<String>,
}

impl ColumnAcross {
    fn new(book: WindowId) -> Self {
        ColumnAcross {
            book,
            long_name: String::new(),
            units: String::new(),
            formula: String::new(),
            recalc: sheaf_core::column::RecalcMode::None,
            project: false,
            folder: false,
            after: String::new(),
            chosen: None,
            problem: None,
        }
    }
}

/// What the Integrate gadget's Preferences window edits (T357).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GadgetPrefs {
    pub graph: WindowId,
    pub from: f64,
    pub to: f64,
    pub baseline: bool,
    pub absolute: bool,
}
use crate::dialogs::error_bars::{ErrorBarsDialog, Outcome as ErrorBarsOutcome};
use crate::dialogs::export::{ExportDialog, ExportOutcome};
use crate::dialogs::export_excel::ExcelExportDialog;
use crate::dialogs::export_sheet::SheetExportDialog;
use crate::dialogs::find::{FindDialog, FindOutcome};
use crate::dialogs::fit::{FitDialog, FitDialogOutcome, FitKind};
use crate::dialogs::fit_more::{FitMoreDialog, FitMoreKind, FitMoreOutcome};
use crate::dialogs::go_to::{GoToDialog, GoToOutcome};
use crate::dialogs::graph_props::{GraphPropsDialog, PropsTab};
use crate::dialogs::import::{ImportDialog, ImportOutcome, ImportRequest};
use crate::dialogs::import_excel::{ExcelImportDialog, split_excel};
use crate::dialogs::import_theme::Themed;
use crate::dialogs::import_wizard::{ImportWizard, WizardPage};
use crate::dialogs::label_panels::{LabelPanelsDialog, Outcome as LabelPanelsOutcome};
use crate::dialogs::mask_condition::MaskConditionDialog;
use crate::dialogs::merge_graphs::{MergeGraphsDialog, Outcome as MergeGraphsOutcome};
use crate::dialogs::multi_y::{MultiYDialog, Outcome as MultiYOutcome};
use crate::dialogs::multipeak::{MultiPeakDialog, MultiPeakOutcome};
use crate::dialogs::pivot::PivotDialog;
use crate::dialogs::power::{PowerDialog, PowerOutcome};
use crate::dialogs::printing::{PageSetupDialog, PageSetupOutcome, PreviewOutcome, PrintPreview};
use crate::dialogs::range_pick::{RangePickDialog, RangePickOutcome};
use crate::dialogs::recovery::{RecoveryChoice, RecoveryOffer};
use crate::dialogs::reshape::{ReshapeDialog, ReshapeOutcome};
use crate::dialogs::row_filter::RowFilterDialog;
use crate::dialogs::set_values::SetValuesDialog;
use crate::dialogs::shape_object::ShapeDialog;
use crate::dialogs::simsurface::{Outcome as SimulateSurfaceOutcome, SimulateSurfaceDialog};
use crate::dialogs::simulate::{Outcome as SimulateOutcome, SimulateDialog};
use crate::dialogs::slope_object::SlopeDialog;
use crate::dialogs::sort::{SortDialog, SortOutcome};
use crate::dialogs::stack_panels::{Outcome as StackPanelsOutcome, StackPanelsDialog};
use crate::dialogs::straight_line::StraightLineDialog;
use crate::dialogs::subtract_pick::{SubtractPickDialog, SubtractPickOutcome};
use crate::dialogs::text_object::TextDialog;
use crate::dialogs::trendline::{Outcome as TrendlineOutcome, TrendlineDialog};
use crate::dialogs::worksheet_color::WorksheetColorDialog;
use crate::dialogs::worksheet_fonts::WorksheetFontsDialog;
use crate::graph_scale::{self, ScaleTool};
use crate::graph_view::{self, GraphUi};
use crate::grid::{self, BookUi, Sel};
use crate::live::{self, Live, Note};
use crate::menus::{self, MenuContext, WindowKind};
use crate::panels::logs::Logs;
use crate::panels::{self, PanelData, PanelId, Panels, explorer, objects};
use crate::recent::{self, Recent, RecentKind};
use crate::theme;
use crate::toolbar::{self, Bar, Toolbars};
use crate::workspace::{WindowInfo, Workspace};
use eframe::egui::{self, RichText};
use sheaf_core::clipboard::CopyOptions;

/// Copy Page's size factor: the picture is this fraction of the page size. Origin's option is
/// 40 % up to 2021a and 100 % from 2021b, so 40 % is the 2016 default (graphing.md 9.6).
const COPY_PAGE_SIZE_FACTOR: f64 = 0.4;
use sheaf_core::graph::{LabelForm, PageGrid, TickLabelSource};
use sheaf_core::project::{Folder, Window};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation, Document, Project, WindowId};
use sheaf_io::batch::{ImportMode, PlaceOptions, Target, place_workbook_with};
use sheaf_io::project_file;
use sheaf_io::recent::{ExportEntry, ExportKind};
use sheaf_io::recovery::{self, AutosaveSettings, Session};
use sheaf_io::window_file::{self, WindowFileKind};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

mod no_data;

pub const APP_NAME: &str = "Sheaf";

/// What to do at start-up, from the command line.
#[derive(Default, Debug)]
pub struct Startup {
    /// A project to open, or data files to import with default settings.
    pub paths: Vec<PathBuf>,
    /// Save a picture of the window to this file after a few frames, then quit.
    /// This is how the user interface is checked without a person at the screen.
    pub screenshot: Option<PathBuf>,
    /// Things to open before the picture is taken, for example `graph-settings:x`.
    pub show: Vec<String>,
    /// The folder of the recovery files. `None` runs without recovery: nothing is scanned, written or
    /// removed (the tests, `--no-recovery`).
    pub recovery_dir: Option<PathBuf>,
    /// Where the autosave switch and interval are kept; `None` keeps the defaults and writes nothing.
    pub settings_path: Option<PathBuf>,
    /// The file of the recent projects and imports (`recent.json`); `None` keeps no lists and writes
    /// nothing (the tests pass a temporary file).
    pub recent_path: Option<PathBuf>,
    /// Private format clipboard snapshot; None keeps it in memory only (including tests).
    pub format_clipboard_path: Option<PathBuf>,
    /// Reusable graph custom-color wells; None keeps only an in-memory collection.
    pub custom_colors_path: Option<PathBuf>,
    /// The folder of saved dialog settings (`import-themes`: importer files and separate graph-export
    /// files); `None` hides the theme rows and writes nothing (tests pass a temporary folder).
    pub themes_dir: Option<PathBuf>,
    /// The folder of the user graph templates (File > Save Template As, Plot > User Templates);
    /// `None` greys both and writes nothing (tests pass a temporary folder).
    pub templates_dir: Option<PathBuf>,
    /// Where graph themes are kept (Save Format as Theme, Theme Organizer); `None` greys them out.
    pub graph_themes_dir: Option<PathBuf>,
    /// The Peak Analyzer themes file (T169); `None` hides Save as Theme and lists no theme.
    pub peak_themes_path: Option<PathBuf>,
    /// Where the Fitting Function Builder saves its `.FDF` files (T132); `None` saves nothing.
    pub fit_functions_dir: Option<PathBuf>,
    /// Turn the AI link on at start (`--mcp`). It is never saved: a run starts with it off.
    pub mcp: bool,
    /// The run folders the AI link may listen in, best first. `None` has no folder, so the link
    /// cannot start (the tests; they pass a temporary folder of their own).
    pub mcp_dirs: Option<Vec<PathBuf>>,
}

/// Something that needs an answer about unsaved changes first.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Pending {
    New,
    /// Open the project in `pending_path` (dropped onto the window or given at start-up).
    OpenFile,
    /// Open the newest recovery of an earlier run (the Recover button).
    Recover,
    /// Duplicate Project without Data (it replaces the open project).
    DuplicateNoData,
    /// Open the sample project in `pending_sample` (T133).
    Sample,
    Quit,
}

/// More new windows than this at once are not all opened.
const MAX_TABS_AT_ONCE: usize = 8;
pub(crate) const MAX_ADDED_COLUMNS: usize = 1000;

struct RenameDialog {
    window: WindowId,
    long_name: String,
    /// The window's short name (Origin's Window Properties have both); cleaned by Origin's rule on OK.
    short_name: String,
    /// Why the last OK was refused (a short name another window has).
    error: Option<String>,
    /// Set when a folder is renamed instead of a window: its path.
    folder: Option<Vec<usize>>,
    /// The window's Comments (T249).
    comments: String,
}

/// The draft of a sheet's names and comments.
struct SheetNameDialog {
    book: WindowId,
    sheet: usize,
    name: String,
    long_name: String,
    comments: String,
}

pub struct SheafApp {
    doc: Document,
    path: Option<PathBuf>,
    /// Find in Project Files' Open (T449): the file and the search, to go to its first hit once it has opened.
    jump_after_open: Option<(PathBuf, crate::project_find::FindSpec)>,
    /// The search to run on the project just opened (T449), done on the next frame.
    jump_now: Option<crate::project_find::FindSpec>,
    /// The Origin file the project was imported from; saving over it first keeps a `.bak` copy.
    foreign_source: Option<PathBuf>,
    workspace: Workspace,
    /// Windows that were already shown once. A window the user closed is not reopened by itself.
    known: HashSet<WindowId>,
    active: Option<WindowId>,
    books: HashMap<WindowId, BookUi>,
    graphs: HashMap<WindowId, GraphUi>,
    /// Notes windows (T154).
    notes: HashMap<WindowId, crate::notes_view::NotesUi>,
    /// Matrix windows (T192).
    matrices: HashMap<WindowId, crate::matrix_view::MatrixUi>,
    /// Layout pages (T237).
    layouts: HashMap<WindowId, crate::layout_view::LayoutUi>,
    /// Layout > Add Graph's window (T237).
    layout_add_graph: Option<crate::dialogs::layout_add_graph::LayoutAddGraph>,
    /// Layout > Page Setup's window (T237 slice 4).
    layout_page_setup: Option<crate::dialogs::layout_page_setup::LayoutPageSetup>,
    /// Layout > Add Text's window (T237 slice 8).
    layout_text: Option<crate::dialogs::layout_text::LayoutText>,
    /// A drawn layout object's properties window (T237 slice 11).
    layout_shape: Option<crate::dialogs::layout_shape::LayoutShapeDialog>,
    import: Option<ImportDialog>,
    import_excel: Option<ExcelImportDialog>,
    import_wizard: Option<ImportWizard>,
    graph_props: Option<GraphPropsDialog>,
    layer_contents: Option<crate::dialogs::layer_contents::LayerContentsDialog>,
    layer_management: Option<crate::dialogs::layer_management::LayerManagementDialog>,
    add_layer_dialog: Option<crate::dialogs::add_layer::AddLayerDialog>,
    fit_page: Option<crate::dialogs::fit_page::FitPageDialog>,
    /// File > Import > Binary 2D Array's options window (T199).
    import_bin2d: Option<crate::dialogs::import_bin2d::ImportBin2dDialog>,
    /// The Matrix menu's Set Matrix Values, Set Dimensions, Expand and Shrink windows (T192).
    matrix_dialog: Option<crate::dialogs::matrix_ops::MatrixOpDialog>,
    axis3d: Option<crate::dialogs::axis3d::Axis3dDialog>,
    plot_setup: Option<crate::dialogs::plot_setup::PlotSetupDialog>,
    error_bars: Option<ErrorBarsDialog>,
    trendline: Option<TrendlineDialog>,
    stack_panels: Option<StackPanelsDialog>,
    merge_graphs: Option<MergeGraphsDialog>,
    label_panels: Option<LabelPanelsDialog>,
    simulate: Option<SimulateDialog>,
    simulate_surface: Option<SimulateSurfaceDialog>,
    column_props: Option<ColumnPropsDialog>,
    text_dialog: Option<TextDialog>,
    shape_dialog: Option<ShapeDialog>,
    slope_dialog: Option<SlopeDialog>,
    apply_formats: Option<crate::format_clip_ui::ApplyFormats>,
    straight_line_dialog: Option<StraightLineDialog>,
    comparison_plot: Option<ComparisonPlotDialog>,
    multi_y: Option<MultiYDialog>,
    set_values: Option<SetValuesDialog>,
    /// Each workbook's selection while Set Column Values is open (T423): a new selection is a picked reference.
    formula_pick: Option<std::collections::HashMap<WindowId, (crate::grid::Sel, Vec<ColumnId>, usize)>>,
    analysis: Option<AnalysisDialog>,
    fit: Option<FitDialog>,
    fit_more: Option<FitMoreDialog>,
    filter: Option<FilterDialog>,
    row_filter: Option<RowFilterDialog>,
    mask_condition: Option<MaskConditionDialog>,
    cond_format: Option<CondFormatDialog>,
    cond_manager: Option<CondManagerDialog>,
    pub(crate) cell_note: Option<CellNoteDialog>,
    pub(crate) cell_format: Option<CellFormatDialog>,
    pub(crate) worksheet_fonts: Option<WorksheetFontsDialog>,
    pub(crate) worksheet_color: Option<WorksheetColorDialog>,
    reshape: Option<ReshapeDialog>,
    pivot: Option<PivotDialog>,
    multi_peak: Option<MultiPeakDialog>,
    subtract_pick: Option<SubtractPickDialog>,
    cv_pick: Option<CvPickDialog>,
    /// File > Find in Project Files... (T361).
    pub(crate) find_files: Option<crate::dialogs::find_files::FindFilesDialog>,
    /// The last analysis run from its window, with its menu label, for Repeat Last Analysis (T360).
    last_analysis: Option<(sheaf_core::analysis::Tool, &'static str)>,
    /// Column > Add Column to All Sheets... (T359).
    pub(crate) column_across: Option<ColumnAcross>,
    /// The Integrate gadget's Preferences window (T357).
    pub(crate) gadget_prefs: Option<GadgetPrefs>,
    range_pick: Option<RangePickDialog>,
    /// The graph whose Data Reader the Peak Analyzer's anchor picking turned on (T351).
    anchor_graph: Option<WindowId>,
    export: Option<ExportDialog>,
    export_sheet: Option<SheetExportDialog>,
    export_excel: Option<ExcelExportDialog>,
    page_setup: Option<PageSetupDialog>,
    print_preview: Option<PrintPreview>,
    rename: Option<RenameDialog>,
    go_to: Option<GoToDialog>,
    /// File > Save Template As: the name prompt.
    template_prompt: Option<crate::dialogs::user_templates::SaveTemplatePrompt>,
    /// Plot > User Templates > Manage.
    template_manager: Option<crate::dialogs::user_templates::TemplateManager>,
    /// File > Batch Processing... (T165).
    batch: Option<crate::dialogs::batch_process::BatchDialog>,
    /// Add Graph as Sheet... (T190).
    graph_sheet_picker: Option<crate::dialogs::graph_sheet::GraphSheetPicker>,
    /// Send Graphs to PowerPoint... (T232).
    send_ppt: Option<crate::dialogs::send_ppt::SendPptDialog>,
    /// A data cell's Define Name... (T396).
    name_cell: Option<crate::dialogs::name_cell::NameCellDialog>,
    /// Worksheet > Name Manager... (T434).
    name_manager: Option<crate::dialogs::name_cell::NameManager>,
    /// A data cell's Insert Sparklines... (T200).
    cell_sparkline_picker: Option<crate::dialogs::cell_sparkline::CellSparklinePicker>,
    /// Batch Peak Analysis Using Theme... (T169).
    batch_peaks: Option<crate::dialogs::batch_peaks::BatchPeaksDialog>,
    /// Column > Fill Column With > A set of Numbers / Date/Time values / Text&Numeric values.
    fill_pattern: Option<crate::dialogs::fill_pattern::FillPatternDialog>,
    /// Column > Set Sampling Interval... and Show X Column....
    sampling: Option<crate::dialogs::sampling::SamplingDialog>,
    /// The column header's Copy Columns to....
    copy_columns: Option<crate::dialogs::copy_columns::CopyColumnsDialog>,
    /// The sheet tab's Navigate....
    sheet_navigate: Option<crate::dialogs::sheet_navigate::SheetNavigateDialog>,
    /// Edit > Find in Project, and the settings of the last search (Origin keeps them).
    find_project: Option<crate::dialogs::find_project::FindProjectDialog>,
    find_spec: crate::project_find::FindSpec,
    /// Window > Duplicate (Batch Plotting).
    batch_plot: Option<crate::dialogs::batch_plot::BatchPlotDialog>,
    qc_chart: Option<crate::dialogs::qc_chart::QcDialog>,
    /// Plot > Statistics > Grouped Histogram's window (T271).
    grouped_hist: Option<crate::dialogs::grouped_hist::GroupedHistDialog>,
    /// Plot > Statistics > Run Chart's window (T270).
    run_chart: Option<crate::dialogs::run_chart::RunChartDialog>,
    function_graph: Option<crate::dialogs::function_graph::FunctionGraphDialog>,
    pub(crate) graph_slicer: Option<crate::dialogs::graph_slicer::GraphSlicerDialog>,
    pub(crate) browser_graph: Option<crate::dialogs::browser_graph::BrowserGraphDialog>,
    pub(crate) flash_fill: Option<crate::dialogs::flash_fill::FlashFillDialog>,
    pub(crate) project_browser: Option<crate::dialogs::project_browser::ProjectBrowserDialog>,
    pub(crate) op_params: Option<crate::dialogs::op_params::OpParamsDialog>,
    pub(crate) auto_refresh: Option<crate::dialogs::auto_refresh::AutoRefreshDialog>,
    /// The Formula Bar of each workbook (T448).
    formula_bars: HashMap<WindowId, crate::formula_bar::FormulaBar>,
    /// View > Formula Bar is on (off to start with).
    pub(crate) show_formula_bar: bool,
    /// Auto Refresh (T442): per workbook, when its next check is due (seconds of the UI clock) and the source
    /// file's modification time at the last read.
    refresh_clock: HashMap<WindowId, (f64, Option<std::time::SystemTime>)>,
    /// The open function window is File > New > Function Plot's: OK makes the graph (T213).
    function_graph_new: bool,
    /// The Audit Log window is open (T131).
    audit_log: bool,
    fit_builder: Option<crate::dialogs::fit_builder::FitBuilder>,
    sample_projects: Option<crate::dialogs::sample_projects::SampleProjects>,
    /// The sample project to open once the question about unsaved changes is answered (T133).
    pending_sample: Option<String>,
    /// Plot > Statistics > Scatter Matrix's options (T114).
    scatter_matrix: Option<crate::dialogs::scatter_matrix::ScatterMatrixDialog>,
    /// The explorer folder's Properties....
    folder_props: Option<crate::dialogs::folder_props::FolderPropsDialog>,
    /// Save Format as Theme (with the copied axis format) and the Theme Organizer (T102).
    save_theme: Option<(crate::dialogs::graph_themes::SaveThemeDialog, sheaf_core::format_clip::FormatClip)>,
    theme_organizer: Option<crate::dialogs::graph_themes::ThemeOrganizer>,
    /// Plot > System Templates....
    system_templates: Option<crate::dialogs::system_templates::SystemTemplatesDialog>,
    /// Tools > Fitting Function Organizer.
    fit_organizer: Option<crate::dialogs::fit_organizer::FitOrganizer>,
    /// Tools > Import Filters Manager.
    filter_manager: Option<crate::dialogs::filter_manager::FilterManager>,
    /// The Project Explorer's active folder when last checked, so that a change brings a window of the
    /// new folder forward (View Windows, T100).
    last_folder: Vec<usize>,
    show_x: Option<crate::dialogs::sampling::ShowXDialog>,
    /// The saved user templates as last read from the folder (at start and after a change).
    user_templates: Vec<String>,
    /// The saved user workbook templates, read with them (T160).
    user_book_templates: Vec<String>,
    power: Option<PowerDialog>,
    /// Statistics > Acceptance Sampling... (T369).
    acceptance: Option<crate::dialogs::acceptance::AcceptanceDialog>,
    find: Option<FindDialog>,
    sort: Option<SortDialog>,
    sheet_name: Option<SheetNameDialog>,
    /// Add New Columns (T305): the workbook and how many columns to add.
    add_columns: Option<(WindowId, usize)>,
    /// Add User Parameter Row (T309): the workbook, the sheet and the name typed so far.
    /// T310: the last item is the row being renamed (`None` adds a row).
    param_row: Option<(WindowId, usize, String, Option<String>)>,
    /// The text of the last Copy or Cut (the system clipboard has it too).
    copied: String,
    /// The block last copied in Sheaf (columns and rows), for Paste Link (T211).
    copied_cells: Option<(Vec<ColumnId>, std::ops::Range<usize>)>,
    /// Edit > Paste Transpose asked for the clipboard: the next pasted text is transposed.
    paste_transposed: bool,
    worksheet_clipboard: crate::wks_clipboard::WorksheetClipboard,
    /// The pixel size of the last page put on the clipboard by Copy Page.
    copied_page: Option<[usize; 2]>,
    about: bool,
    /// Which toolbars are shown.
    toolbars: Toolbars,
    worksheet_format: crate::worksheet_format::Cache,
    /// View > Status Bar turned the status bar off.
    status_bar_hidden: bool,
    /// View > Full Screen is on (T66): one graph's page over the whole screen until Esc.
    full_screen: Option<crate::view_modes::FullScreen>,
    /// The Full Screen reminder was switched off with its tick (kept for this run only).
    full_screen_reminder_off: bool,
    /// The side panels: Project Explorer, Object Manager and the logs.
    panels: Panels,
    /// What the status bar and the message boxes said, and the reports of the tools.
    logs: Logs,
    /// The message box text that is already in the Messages Log.
    logged_message: Option<(String, String)>,
    /// A message the user has to dismiss (title, text).
    message: Option<(String, String)>,
    status: String,
    next_token: u64,
    pending: Option<Pending>,
    pending_path: Option<PathBuf>,
    /// Window files that came with the project in `pending_path`; they join it once it is open.
    pending_windows: Vec<PathBuf>,
    /// Folder of the last imported Origin project, used as the start folder of file choosers.
    import_folder: Option<PathBuf>,
    quit_confirmed: bool,
    /// A top-level menu that is held open (`--show menu:NAME`).
    open_menu: Option<String>,
    shown_title: String,
    startup: Startup,
    frame: u64,
    /// The timer and worker that write the recovery file.
    autosave: Autosave,
    autosave_dialog: Option<AutosaveDialog>,
    options: Option<crate::dialogs::options::OptionsDialog>,
    table_editor: Option<crate::dialogs::table_editor::TableEditor>,
    /// Work that an earlier run left behind, offered at start-up.
    recovery_offer: Option<RecoveryOffer>,
    /// The file the recovered project belonged to: suggested by Save, since the project has no name.
    recovered_from: Option<PathBuf>,
    /// The endpoint of `sheaf-mcp --attach`, when Tools > AI Assistants (MCP) is ticked.
    live: Live,
    /// The recent projects and imports (File > Recent Projects, File > Import > Recent Imports).
    recent: Recent,
    /// The Window Switcher (Ctrl+Tab) while it is open.
    switcher: Option<crate::switcher::Switcher>,
}

type ShapeLayerSnapshot = (WindowId, usize, Option<Vec<sheaf_core::shape::Shape>>);

impl SheafApp {
    pub fn new(cc: &eframe::CreationContext<'_>, startup: Startup) -> Self {
        crate::format_clip_ui::initialize(startup.format_clipboard_path.clone());
        crate::custom_colors::initialize(startup.custom_colors_path.clone());
        theme::apply(&cc.egui_ctx);
        let startup_dirs = startup.mcp_dirs.clone().unwrap_or_default();
        let recent_path = startup.recent_path.clone();
        let mut app = SheafApp {
            doc: Document::default(),
            path: None,
            jump_after_open: None,
            jump_now: None,
            foreign_source: None,
            workspace: Workspace::default(),
            known: HashSet::new(),
            active: None,
            books: HashMap::new(),
            graphs: HashMap::new(),
            notes: HashMap::new(),
            matrices: HashMap::new(),
            layouts: HashMap::new(),
            layout_add_graph: None,
            layout_page_setup: None,
            layout_text: None,
            layout_shape: None,
            import: None,
            import_excel: None,
            import_wizard: None,
            graph_props: None,
            layer_contents: None,
            layer_management: None,
            add_layer_dialog: None,
            fit_page: None,
            matrix_dialog: None,
            import_bin2d: None,
            axis3d: None,
            plot_setup: None,
            error_bars: None,
            trendline: None,
            stack_panels: None,
            merge_graphs: None,
            label_panels: None,
            simulate: None,
            simulate_surface: None,
            column_props: None,
            text_dialog: None,
            shape_dialog: None,
            slope_dialog: None,
            apply_formats: None,
            straight_line_dialog: None,
            comparison_plot: None,
            multi_y: None,
            set_values: None,
            formula_pick: None,
            analysis: None,
            fit: None,
            fit_more: None,
            filter: None,
            row_filter: None,
            mask_condition: None,
            cond_format: None,
            cond_manager: None,
            cell_note: None,
            cell_format: None,
            worksheet_fonts: None,
            worksheet_color: None,
            reshape: None,
            pivot: None,
            multi_peak: None,
            subtract_pick: None,
            cv_pick: None,
            find_files: None,
            last_analysis: None,
            column_across: None,
            gadget_prefs: None,
            range_pick: None,
            anchor_graph: None,
            export: None,
            export_sheet: None,
            export_excel: None,
            page_setup: None,
            print_preview: None,
            rename: None,
            go_to: None,
            template_prompt: None,
            template_manager: None,
            batch: None,
            graph_sheet_picker: None,
            send_ppt: None,
            name_cell: None,
            name_manager: None,
            cell_sparkline_picker: None,
            batch_peaks: None,
            fill_pattern: None,
            sampling: None,
            copy_columns: None,
            sheet_navigate: None,
            find_project: None,
            find_spec: Default::default(),
            batch_plot: None,
            qc_chart: None,
            grouped_hist: None,
            run_chart: None,
            function_graph: None,
            graph_slicer: None,
            browser_graph: None,
            flash_fill: None,
            project_browser: None,
            op_params: None,
            auto_refresh: None,
            formula_bars: HashMap::new(),
            show_formula_bar: false,
            refresh_clock: HashMap::new(),
            function_graph_new: false,
            audit_log: false,
            fit_builder: None,
            sample_projects: None,
            pending_sample: None,
            scatter_matrix: None,
            folder_props: None,
            save_theme: None,
            theme_organizer: None,
            system_templates: None,
            fit_organizer: None,
            filter_manager: None,
            last_folder: Vec::new(),
            show_x: None,
            user_templates: startup.templates_dir.as_deref().map(sheaf_io::user_templates::names).unwrap_or_default(),
            user_book_templates: startup
                .templates_dir
                .as_deref()
                .map(|d| sheaf_io::user_templates::names_of(d, sheaf_io::user_templates::TemplateKind::Workbook))
                .unwrap_or_default(),
            power: None,
            acceptance: None,
            find: None,
            sort: None,
            sheet_name: None,
            add_columns: None,
            param_row: None,
            copied: String::new(),
            copied_cells: None,
            paste_transposed: false,
            worksheet_clipboard: crate::wks_clipboard::WorksheetClipboard::default(),
            copied_page: None,
            about: false,
            toolbars: Toolbars::default(),
            worksheet_format: Default::default(),
            status_bar_hidden: false,
            full_screen: None,
            full_screen_reminder_off: false,
            panels: Panels::default(),
            logs: Logs::default(),
            logged_message: None,
            message: None,
            status: "Ready".to_string(),
            next_token: 1,
            pending: None,
            pending_path: None,
            pending_windows: Vec::new(),
            import_folder: None,
            quit_confirmed: false,
            open_menu: None,
            shown_title: String::new(),
            startup,
            frame: 0,
            autosave: Autosave::new(None, AutosaveSettings::default(), None),
            autosave_dialog: None,
            options: None,
            table_editor: None,
            recovery_offer: None,
            recovered_from: None,
            live: Live::new(cc.egui_ctx.clone(), startup_dirs),
            recent: Recent::new(recent_path),
            switcher: None,
        };
        app.start_recovery();
        let paths = std::mem::take(&mut app.startup.paths);
        app.open_paths(paths, false);
        app.sync_windows();
        app.run_startup_show(&cc.egui_ctx);
        if app.startup.mcp {
            app.set_link(true);
        }
        app
    }

    // ----- autosave and recovery -----------------------------------------

    /// Reads the autosave settings, drops recovery files older than 30 days, lists what earlier runs
    /// left behind and starts this run's own session. Without a recovery folder nothing is done.
    fn start_recovery(&mut self) {
        let settings_path = self.startup.settings_path.clone();
        let settings = settings_path.as_deref().map(AutosaveSettings::load).unwrap_or_default();
        let mut session = None;
        let mut orphans = Vec::new();
        if let Some(dir) = self.startup.recovery_dir.clone() {
            let dropped = recovery::prune(&dir, recovery::MAX_AGE, std::time::SystemTime::now());
            if dropped > 0 {
                self.logs.info(format!("Dropped {dropped} recovery file(s) older than 30 days"));
            }
            orphans = recovery::list_orphans(&dir);
            match Session::start(&dir) {
                Ok(s) => session = Some(s),
                Err(e) => self.warn(format!("Autosave is off, the recovery folder cannot be used: {e}")),
            }
        }
        self.autosave = Autosave::new(session, settings, settings_path);
        if !orphans.is_empty() {
            self.say(format!("Found {} project(s) from an earlier run that was not closed normally", orphans.len()));
            self.recovery_offer = Some(RecoveryOffer::new(orphans));
        }
    }

    /// The Recover button: opens the newest recovery as a new, modified project that has no file
    /// name. This run's own recovery copy is written first and only then is the old one removed, so
    /// the work is never held by nothing.
    fn recover_newest(&mut self) {
        let Some(newest) = self.recovery_offer.as_ref().and_then(|o| o.orphans.first().cloned()) else { return };
        let project = match recovery::load(&newest) {
            Ok(p) => p,
            Err(e) => {
                self.message = Some(("Could not recover the project".to_string(), e.to_string()));
                return;
            }
        };
        let original = newest.sidecar.as_ref().and_then(|s| s.original_path());
        self.doc.replace(project);
        self.doc.mark_unsaved();
        self.reset_views();
        self.path = None;
        // A project that came from an Origin file keeps its `.bak` copy when it is saved over it.
        self.foreign_source =
            original.clone().filter(|p| p.exists() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("opj")));
        self.recovered_from = original;
        let copy = self.autosave.write_now(self.doc.project(), self.recovered_from.as_deref(), self.doc.revision());
        let name = crate::dialogs::recovery::name(&newest);
        match copy.and_then(|()| recovery::discard(&newest).map_err(|e| e.to_string())) {
            Ok(()) => self.say(format!("Recovered {name}; it has no file name yet, so Save asks for one")),
            Err(e) => {
                self.warn(format!("Recovered {name}, but the old recovery file stays in the recovery folder: {e}"))
            }
        }
        let others = self.recovery_offer.take().map_or(0, |o| o.orphans.len().saturating_sub(1));
        if others > 0 {
            self.logs.info(format!("{others} more recovery file(s) are kept and offered at the next start"));
        }
    }

    /// The Discard button: removes the newest recovery; the next one, if any, is offered.
    fn discard_newest(&mut self) {
        let Some(offer) = &mut self.recovery_offer else { return };
        if offer.orphans.is_empty() {
            return;
        }
        let newest = offer.orphans.remove(0);
        if offer.orphans.is_empty() {
            self.recovery_offer = None;
        }
        let name = crate::dialogs::recovery::name(&newest);
        match recovery::discard(&newest) {
            Ok(()) => self.say(format!("Discarded the recovery of {name}")),
            Err(e) => self.warn(format!("Could not discard the recovery of {name}: {e}")),
        }
    }

    /// Once per frame: shows what a finished write said, removes the recovery files when the
    /// project is clean, and starts a write when the interval has passed.
    fn autosave_step(&mut self, ctx: &egui::Context) {
        match self.autosave.poll() {
            Some(Ok(())) => self.autosave.report_success(),
            Some(Err(e)) => self.autosave_failed(&e),
            None => {}
        }
        if !self.doc.dirty {
            if let Err(e) = self.autosave.clear() {
                self.autosave_failed(&e);
                ctx.request_repaint_after(std::time::Duration::from_secs(1));
            } else if self.autosave.write_pending() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            return;
        }
        match self.autosave.due(self.doc.dirty, self.doc.revision()) {
            Due::Never => {}
            Due::In(wait) => ctx.request_repaint_after(wait),
            Due::Now => {
                let project = self.autosave_snapshot();
                let original = self.recovery_original();
                self.autosave.start_write(project, original.as_deref(), self.doc.revision());
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
    }

    /// A copy of the project with the current window places, for the recovery file. It adds no undo
    /// step and does not change the modified flag.
    fn autosave_snapshot(&self) -> Project {
        let mut project = self.doc.project().clone();
        project.window_places = self.workspace.places();
        project
    }

    /// The file the recovery should name as the project's own.
    fn recovery_original(&self) -> Option<PathBuf> {
        self.path.clone().or_else(|| self.foreign_source.clone()).or_else(|| self.recovered_from.clone())
    }

    fn autosave_failed(&mut self, error: &str) {
        if self.autosave.report_error() {
            self.warn(format!("Autosave could not write the recovery file: {error}"));
        }
    }

    /// The Autosave window's OK: applies the settings now and keeps them in the settings file.
    fn apply_autosave_settings(&mut self, settings: AutosaveSettings) {
        self.autosave.settings = settings;
        if let Some(path) = self.autosave.settings_path.clone()
            && let Err(e) = settings.save(&path)
        {
            self.message = Some(("Could not keep the autosave settings".to_string(), e.to_string()));
        }
        if settings.enabled {
            self.say(format!("Autosave is on, every {} minute(s)", settings.minutes));
        } else {
            self.say("Autosave is off");
        }
    }

    /// The end of this run: a normal exit removes the recovery files. Closing while changes are
    /// unsaved and nobody was asked (the system ended the program) writes them one last time and
    /// leaves them for the next start.
    fn end_autosave(&mut self) {
        let clean = !self.doc.dirty || self.quit_confirmed;
        if !clean && self.autosave.settings.enabled {
            let project = self.autosave_snapshot();
            let original = self.recovery_original();
            let _ = self.autosave.write_now(&project, original.as_deref(), self.doc.revision());
        }
        if let Err(e) = self.autosave.end(clean) {
            eprintln!("could not remove the recovery files: {e}");
        }
    }

    // ----- files ---------------------------------------------------------

    /// Opens a project file, or imports data files. With `ask`, data files go
    /// through the import dialog; without, they are imported with default settings.
    fn open_paths(&mut self, paths: Vec<PathBuf>, ask: bool) {
        if paths.is_empty() {
            return;
        }
        // A window file (T54) never replaces the project: its windows are added, as File > Append does.
        // It is routed before the project check, which refuses a window file.
        let (windows, paths): (Vec<PathBuf>, Vec<PathBuf>) =
            paths.into_iter().partition(|p| window_file::is_window_file(p));
        if let Some(project) = paths.iter().find(|p| sheaf_io::is_project_file(p)) {
            if self.doc.dirty {
                self.pending_path = Some(project.clone());
                self.pending_windows = windows;
                self.pending = Some(Pending::OpenFile);
            } else {
                if self.load_project(project.clone()) {
                    self.append_window_files(&windows);
                }
            }
            return;
        }
        self.append_window_files(&windows);
        if paths.is_empty() {
            return;
        }
        // Excel workbooks have their own import window and are not graphed on their own.
        let (workbooks, paths) = split_excel(paths);
        // PDFs (T463) are read at once, without the text import window, and not graphed.
        let (pdfs, paths): (Vec<PathBuf>, Vec<PathBuf>) = paths.into_iter().partition(|p| is_pdf(p));
        if !pdfs.is_empty() {
            self.import_now(pdfs, false);
        }
        if ask {
            if !workbooks.is_empty() {
                self.import_excel = Some(self.excel_dialog(workbooks));
            }
            if !paths.is_empty() {
                self.import = Some(self.import_dialog(paths));
            }
            return;
        }
        for (files, plot) in [(paths, true), (workbooks, false)] {
            if !files.is_empty() {
                self.import_now(files, plot);
            }
        }
    }

    /// Imports `files` with the default settings into new workbooks, graphing them when `plot`.
    fn import_now(&mut self, files: Vec<PathBuf>, plot: bool) {
        let (chi, ascii) = (sheaf_io::chi::ChiOptions::default(), sheaf_io::ascii::AsciiOptions::default());
        let mut items = Vec::new();
        let mut errors = Vec::new();
        for p in &files {
            match sheaf_io::import_file(p, sheaf_io::ImportKind::Auto, &chi, &ascii) {
                Ok(i) => items.push(i),
                Err(e) => errors.push(e.to_string()),
            }
        }
        self.finish_import(ImportRequest {
            items,
            errors,
            plot,
            template: None,
            mode: ImportMode::NewBook,
            place: PlaceOptions::default(),
        });
    }

    /// Opens a `.sheaf` project or imports an Origin `.opj` project, replacing the current one.
    /// File > Append: the windows of the project in `path` join this project in a folder named
    /// after the file, in one undo step. A window file goes to [`SheafApp::append_window_file`].
    fn append_project_file(&mut self, path: &Path) {
        if window_file::is_window_file(path) {
            self.append_window_file(path);
            return;
        }
        match sheaf_io::open_project(path) {
            Ok(opened) => {
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Appended").to_string();
                let mut added = (Vec::new(), 0);
                self.doc.apply("Append project", |p| added = p.append(opened.project, &name));
                let (ids, dropped) = added;
                for id in ids.iter().take(MAX_TABS_AT_ONCE) {
                    self.focus(*id);
                }
                self.say(format!("Appended {} window(s) from {}", ids.len(), path.display()));
                let mut notes = opened.warnings;
                if dropped > 0 {
                    notes.push(format!(
                        "{dropped} analysis result(s) of the appended project no longer recalculate; their values are kept."
                    ));
                }
                if !notes.is_empty() {
                    self.message = Some(("Notes about the appended project".to_string(), notes.join("\n\n")));
                }
            }
            Err(e) => self.message = Some(("Could not append the project".to_string(), e.to_string())),
        }
    }

    fn append_window_files(&mut self, paths: &[PathBuf]) {
        for path in paths {
            self.append_window_file(path);
        }
    }

    /// Opening a window file (`.sheafbook`, `.sheafgraph`; `docs/spec/window-files.md`): its windows join
    /// the project in the root folder in one undo step, renamed where a name is taken, and the saved
    /// window comes to the front. Never replaces the project. The file goes to the top of Recent
    /// Books or Recent Graphs.
    fn append_window_file(&mut self, path: &Path) {
        let file = match window_file::load_window(path) {
            Ok(file) => file,
            Err(e) => {
                self.warn(format!("Could not open the window file: {}", path.display()));
                self.message = Some(("Could not open the window file".to_string(), e.to_string()));
                return;
            }
        };
        let wanted: Vec<String> = file.project.windows.iter().map(|w| w.name().to_string()).collect();
        let primary = file.primary;
        let kind = file.kind;
        let mut added = (Vec::new(), 0);
        self.doc.apply("Open window file", |p| added = p.append_windows(file.project));
        let (ids, dropped) = added;
        if let Some(id) = ids.get(primary) {
            self.focus(*id);
        }
        let mut notes = Vec::new();
        for (id, was) in ids.iter().zip(&wanted) {
            if let Some(now) = self.doc.project().window(*id).map(|w| w.name().to_string())
                && now != *was
            {
                notes.push(format!("{was} is renamed {now}, because the project already has a window of that name."));
            }
        }
        if dropped > 0 {
            notes.push(format!("{dropped} analysis result(s) no longer recalculate; their values are kept."));
        }
        self.say(format!("Opened {} window(s) from {}", ids.len(), path.display()));
        self.note_window(kind, path);
        for note in &notes {
            self.logs.warn(note.clone());
        }
        if !notes.is_empty() {
            self.message = Some(("Notes about the opened window file".to_string(), notes.join("\n\n")));
        }
    }

    /// File > Save Window As...: asks for a name (the extension of the window's kind, the window's
    /// name to start with) and writes the window.
    fn save_window_as(&mut self, id: WindowId) {
        let Some(window) = self.doc.project().window(id) else { return };
        let Some(kind) = WindowFileKind::of_window(window) else {
            self.say("A Notes window cannot be saved as a window file yet; save the project instead");
            return;
        };
        let (title, filter) = match kind {
            WindowFileKind::Book => ("Save workbook as", "Sheaf workbook (*.sheafbook)"),
            WindowFileKind::Graph => ("Save graph as", "Sheaf graph (*.sheafgraph)"),
        };
        let suggested = format!("{}.{}", window.name(), kind.extension());
        let mut chooser =
            rfd::FileDialog::new().set_title(title).add_filter(filter, &[kind.extension()]).set_file_name(suggested);
        if let Some(dir) = self.folder() {
            chooser = chooser.set_directory(dir);
        }
        if let Some(path) = chooser.save_file() {
            self.save_window_to(id, path);
        }
    }

    /// Writes window `id` to `path`. A name without the window kind's extension gets it added (so the
    /// file of another kind, a project among them, is never written over by a mistyped name). Returns
    /// `true` when the file was written; then it is at the top of Recent Books or Recent Graphs.
    fn save_window_to(&mut self, id: WindowId, path: PathBuf) -> bool {
        let Some(window) = self.doc.project().window(id) else { return false };
        let Some(kind) = WindowFileKind::of_window(window) else { return false };
        let path = window_file::default_extension(&path, kind);
        match window_file::save_window(self.doc.project(), id, &path) {
            Ok(saved) => {
                self.say(format!("Saved {} window(s) to {}", saved.windows, path.display()));
                self.note_window(kind, &path);
                for note in &saved.notes {
                    self.logs.warn(note.clone());
                }
                if !saved.notes.is_empty() {
                    self.message = Some(("Notes about the saved window".to_string(), saved.notes.join("\n\n")));
                }
                true
            }
            Err(e) => {
                self.message = Some(("Could not save the window".to_string(), e.to_string()));
                false
            }
        }
    }

    /// A window file was opened or saved: it goes to the top of Recent Books or Recent Graphs.
    fn note_window(&mut self, kind: WindowFileKind, path: &Path) {
        if let Some(why) = self.recent.push_window(kind, path) {
            self.logs.warn(why);
        }
    }

    /// Replaces the project with the one in `path`. Returns `false` (and keeps the project) when the
    /// file cannot be opened.
    fn load_project(&mut self, path: PathBuf) -> bool {
        match sheaf_io::open_project(&path) {
            Ok(opened) => {
                // Opened from Find in Project Files (T449): its search runs on this project next.
                if let Some((p, spec)) = self.jump_after_open.take()
                    && p == path
                {
                    self.jump_now = Some(spec);
                }
                self.note_project(&path);
                self.doc.replace(opened.project);
                self.reset_views();
                self.recovered_from = None;
                self.foreign_source = opened.foreign.then(|| path.clone());
                if opened.foreign {
                    // Not saved back to the Origin file by itself (Sheaf reads only part of what
                    // Origin keeps there): the project has no file name yet.
                    self.say(format!(
                        "Imported {}; use Save Project As to keep changes as an Origin project",
                        path.display()
                    ));
                    self.import_folder = path.parent().map(Path::to_path_buf);
                    self.path = None;
                } else if opened.companion_used {
                    self.say(format!("Opened the companion of {} (graphs included)", path.display()));
                    self.path = opened.save_path;
                } else {
                    self.say(format!("Opened {}", path.display()));
                    self.path = opened.save_path;
                }
                if !opened.warnings.is_empty() {
                    self.message = Some((
                        "Notes about this project".to_string(),
                        opened.warnings.join(
                            "

",
                        ),
                    ));
                }
                true
            }
            Err(e) => {
                self.message = Some(("Could not open the project".to_string(), e.to_string()));
                false
            }
        }
    }

    /// File > Open's chooser: projects and window files.
    fn choose_project_file(&self) -> Option<PathBuf> {
        let mut chooser = rfd::FileDialog::new()
            .set_title("Open project")
            .add_filter(
                "Projects and window files",
                &[
                    project_file::EXTENSION,
                    "opj",
                    "opju",
                    WindowFileKind::Book.extension(),
                    WindowFileKind::Graph.extension(),
                ],
            )
            .add_filter("Projects (Sheaf, Origin)", &[project_file::EXTENSION, "opj", "opju"])
            .add_filter("Sheaf window files", &[WindowFileKind::Book.extension(), WindowFileKind::Graph.extension()])
            .add_filter("Sheaf project", &[project_file::EXTENSION])
            .add_filter("Origin project", &["opj"])
            .add_filter("Origin Unicode project (workbooks only)", &["opju"]);
        if let Some(dir) = self.folder() {
            chooser = chooser.set_directory(dir);
        }
        chooser.pick_file()
    }

    /// A file picked in File > Open. A window file is added and never asks about unsaved changes (it
    /// does not replace the project); a project asks first when the project has changes.
    fn open_picked(&mut self, path: PathBuf) {
        if window_file::is_window_file(&path) {
            self.append_window_file(&path);
        } else if self.doc.dirty {
            self.pending_path = Some(path);
            self.pending_windows.clear();
            self.pending = Some(Pending::OpenFile);
        } else {
            self.load_project(path);
        }
    }

    fn reset_views(&mut self) {
        self.workspace = Workspace::default();
        self.known.clear();
        self.books.clear();
        self.graphs.clear();
        self.graph_props = None;
        self.error_bars = None;
        self.trendline = None;
        self.stack_panels = None;
        self.merge_graphs = None;
        self.layer_management = None;
        self.add_layer_dialog = None;
        self.fit_page = None;
        self.matrix_dialog = None;
        self.layout_add_graph = None;
        self.layout_page_setup = None;
        self.layout_text = None;
        self.layout_shape = None;
        self.import_bin2d = None;
        self.axis3d = None;
        self.label_panels = None;
        self.simulate = None;
        self.simulate_surface = None;
        self.column_props = None;
        self.set_values = None;
        self.export = None;
        self.export_sheet = None;
        self.export_excel = None;
        self.page_setup = None;
        self.print_preview = None;
        self.rename = None;
        self.go_to = None;
        self.find = None;
        self.row_filter = None;
        self.mask_condition = None;
        self.cond_format = None;
        self.cond_manager = None;
        self.cell_note = None;
        self.cell_format = None;
        self.worksheet_fonts = None;
        self.worksheet_color = None;
        self.reshape = None;
        self.pivot = None;
        self.multi_peak = None;
        self.subtract_pick = None;
        self.cv_pick = None;
        self.range_pick = None;
        self.sort = None;
        self.sheet_name = None;
        self.add_columns = None;
        self.param_row = None;
        self.import = None;
        self.import_excel = None;
        self.import_wizard = None;
    }

    /// Saves to the current file, asking for a name when there is none or
    /// `ask` is set. Returns `true` when the project was saved.
    fn save(&mut self, ask: bool) -> bool {
        let path = match (&self.path, ask) {
            (Some(p), false) => p.clone(),
            _ => {
                // A recovered project has no file name; it suggests the one it had.
                let named = self.path.as_ref().or(self.recovered_from.as_ref());
                let suggested = named
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Untitled.opj".to_string());
                // The default format is .opj; a project that came from a .sheaf file offers
                // .sheaf first because it keeps saving there.
                let keeps_sheaf = named.and_then(|p| p.extension()).is_some_and(|e| e.eq_ignore_ascii_case("sheaf"));
                let origin = ("Origin project (*.opj)", "opj");
                let sheaf = ("Sheaf project (*.sheaf)", project_file::EXTENSION);
                let (first, second) = if keeps_sheaf { (sheaf, origin) } else { (origin, sheaf) };
                let mut chooser = rfd::FileDialog::new()
                    .set_title("Save project")
                    .add_filter(first.0, &[first.1])
                    .add_filter(second.0, &[second.1])
                    .set_file_name(suggested);
                if let Some(dir) = self.folder() {
                    chooser = chooser.set_directory(dir);
                }
                let Some(p) = chooser.save_file() else {
                    return false;
                };
                p
            }
        };
        self.save_to(path)
    }

    /// Writes the project to `path`. Only `.opj` and `.sheaf` names are written as they are; any
    /// other name gets `.opj` added, so a file of another program is never written over. The
    /// rules (companion file, refusal) are in `sheaf_io::save`.
    fn save_to(&mut self, mut path: PathBuf) -> bool {
        let known = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("opj") || e.eq_ignore_ascii_case(project_file::EXTENSION));
        if !known {
            let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
            name.push(".opj");
            path.set_file_name(name);
        }
        let mut backup = None;
        if self.foreign_source.as_ref().is_some_and(|src| same_file(src, &path)) {
            match sheaf_io::save::backup_original(&path) {
                Ok(b) => backup = b,
                Err(e) => {
                    self.message = Some(("Could not keep a copy of the original".to_string(), e.to_string()));
                    return false;
                }
            }
        }
        // Where the windows are goes into the project file too (view state: no undo step).
        let places = self.workspace.places();
        if self.doc.project().window_places != places {
            self.doc.adjust_view(|p| p.window_places = places);
        }
        match sheaf_io::save::save_project(self.doc.project(), &path) {
            Ok(outcome) => {
                // The project is safe on disk either way: in the .opj, or in the companion when
                // the .opj was refused.
                self.doc.mark_saved();
                let mut said = outcome.status();
                if let Some(b) = &backup {
                    said.push_str(&format!("; the original Origin file is kept as {}", b.display()));
                }
                self.say(said);
                // Only a file that exists is listed: the .opj, or the companion when it was refused.
                if outcome.main_written {
                    self.note_project(&path);
                } else if let Some(companion) = &outcome.companion {
                    self.note_project(companion);
                }
                self.path = Some(path);
                self.recovered_from = None;
                if let Some(text) = outcome.details() {
                    let title = if outcome.refusal.is_some() {
                        "The Origin project could not be written"
                    } else {
                        "Saved with a companion file"
                    };
                    self.message = Some((title.to_string(), text));
                }
                true
            }
            Err(e) => {
                self.message = Some(("Could not save the project".to_string(), e.to_string()));
                false
            }
        }
    }

    /// A project was opened or saved: it goes to the top of the recent projects.
    fn note_project(&mut self, path: &Path) {
        if let Some(why) = self.recent.push_project(path) {
            self.logs.warn(why);
        }
    }

    /// A click on an entry of the recent lists. The file is checked now (the menu never touches it):
    /// a missing file, a folder or a link to one, or a file of the wrong kind is reported and
    /// dropped from the lists. Otherwise it goes the way a dropped file does, so the question about
    /// unsaved changes comes first for a project.
    fn follow_recent(&mut self, path: PathBuf, kind: RecentKind) {
        match recent::check_target(&path, kind) {
            Ok(()) => self.open_paths(vec![path], false),
            Err(why) => {
                let title = match kind {
                    RecentKind::Projects => "Could not open the project",
                    RecentKind::Imports => "Could not import the file",
                    RecentKind::Books => "Could not open the workbook file",
                    RecentKind::Graphs => "Could not open the graph file",
                };
                self.warn(format!("{title}: {}", path.display()));
                if let Some(failed) = self.recent.forget(&path) {
                    self.logs.warn(failed);
                }
                self.message = Some((
                    title.to_string(),
                    format!("{}\n\n{why}\n\nIt was removed from the recent files.", path.display()),
                ));
            }
        }
    }

    /// The folder file choosers start in: next to the project, else next to the last imported file.
    fn folder(&self) -> Option<PathBuf> {
        if let Some(p) = &self.path {
            return p.parent().map(Path::to_path_buf);
        }
        if let Some(dir) = self.recovered_from.as_ref().and_then(|p| p.parent()).filter(|d| d.is_dir()) {
            return Some(dir.to_path_buf());
        }
        if self.import_folder.is_some() {
            return self.import_folder.clone();
        }
        self.doc
            .project()
            .workbooks()
            .filter_map(|b| b.source.as_ref())
            .last()
            .and_then(|s| Path::new(&s.path).parent().map(Path::to_path_buf))
    }

    fn finish_import(&mut self, request: ImportRequest) {
        let ImportRequest { items, errors, plot, template, mode, place } = request;
        let count = items.len();
        let sources: Vec<PathBuf> = items.iter().map(|i| i.path.clone()).collect();
        let excel_sheets: usize = items.iter().filter(|i| i.format == "excel").map(|i| i.workbook.sheets.len()).sum();
        let mut notes: Vec<String> = errors;
        for i in &items {
            for w in &i.warnings {
                notes.push(format!("{}: {w}", i.workbook.long_name));
            }
        }
        // The modes other than "New book" put every file into the sheet that was active when the
        // import started. Only the first file may replace its data; the others become new sheets.
        let target = self.import_target().map(|t| t.0);
        if count > 0 {
            let placement_notes = self.doc.apply("Import data", move |p| {
                let mut placement_notes = Vec::new();
                for (n, imported) in items.into_iter().enumerate() {
                    let t = template.unwrap_or(imported.suggested_template);
                    let this_mode = if n > 0 && mode == ImportMode::ReplaceSheet { ImportMode::NewSheet } else { mode };
                    let placed = place_workbook_with(p, imported.workbook, this_mode, target, place);
                    placement_notes.extend(placed.warnings);
                    let book = placed.book;
                    if plot {
                        let sheet = placed.sheet;
                        let ys = p.default_plot_columns(book, sheet, t);
                        if !ys.is_empty()
                            && let Err(e) = p.plot_columns(t, &ys)
                        {
                            placement_notes.push(format!("No graph was made: {e}"));
                        }
                    }
                }
                placement_notes
            });
            notes.extend(placement_notes);
            let sheets = if excel_sheets > 0 { format!(", {excel_sheets} Excel sheet(s)") } else { String::new() };
            self.say(format!("Imported {count} file(s){sheets}"));
            if let Some(why) = self.recent.push_imports(&sources) {
                self.logs.warn(why);
            }
        }
        for note in &notes {
            self.logs.warn(note.clone());
        }
        if !notes.is_empty() {
            self.message = Some(("Import notes".to_string(), notes.join("\n\n")));
        }
    }

    // ----- windows -------------------------------------------------------

    /// Gives every new window of the project a place in the workspace and drops those that are gone.
    fn sync_windows(&mut self) {
        let ids: Vec<(WindowId, bool)> =
            self.doc.project().windows.iter().map(|w| (w.id(), matches!(w, Window::Graph(_)))).collect();
        let set: HashSet<WindowId> = ids.iter().map(|(id, _)| *id).collect();
        self.workspace.retain(|id| set.contains(&id));
        self.known.retain(|k| set.contains(k));
        self.books.retain(|k, _| set.contains(k));
        self.graphs.retain(|k, _| set.contains(k));
        // A project with very many windows (an imported Origin project can have dozens) opens
        // only its first few; the others are opened from the project explorer.
        let new: Vec<(WindowId, bool)> = ids.into_iter().filter(|(id, _)| self.known.insert(*id)).collect();
        let shown = if new.len() > MAX_TABS_AT_ONCE { 1 } else { new.len() };
        let added: HashSet<WindowId> = new.iter().map(|(id, _)| *id).collect();
        for (k, (id, graph)) in new.into_iter().enumerate() {
            self.workspace.add(id, graph, k < shown);
        }
        // A project that was saved with its window places opens with them; windows it does not
        // list, and windows that were already open, keep their places.
        let places: Vec<_> =
            self.doc.project().window_places.iter().filter(|p| added.contains(&p.window)).copied().collect();
        if !places.is_empty() {
            self.workspace.restore(&places);
        }
    }

    /// Copy Page (Ctrl+J): the graph as a picture on the clipboard, to paste into a document or a
    /// presentation. As in Origin the picture is 40 % of the page size (the "size factor"
    /// before 2021b) and its margin follows the Copy Page margin setting, whose default `Auto` is
    /// the box round everything drawn plus a border (see `docs/spec/graphing.md` 9.6). It is a
    /// 300 dpi bitmap; on Windows an enhanced metafile of the same picture goes with it
    /// (`AddEMFInCopyPage=1`), so a vector editor pastes lines and text, not pixels.
    fn copy_page(&mut self, graph: WindowId, ctx: &egui::Context) {
        const DPI: f64 = 300.0;
        let size = sheaf_plot::ExportSize { ratio: Some(COPY_PAGE_SIZE_FACTOR), ..sheaf_plot::ExportSize::default() };
        let layout = sheaf_plot::ExportLayout { margin: sheaf_plot::Margin::Auto, border_px: None };
        match sheaf_plot::export_raster(self.doc.project(), graph, DPI, size, layout) {
            Ok(r) => {
                let px = [r.width as usize, r.height as usize];
                let metafile = sheaf_plot::export_bytes_with(self.doc.project(), graph, "emf", DPI, size, layout);
                // Under test the real clipboard is left alone.
                let both = match &metafile {
                    Ok(emf) if cfg!(windows) && !cfg!(test) => crate::winclip::put_bitmap_and_emf(
                        &crate::winclip::dib_from_rgba(r.width, r.height, &r.rgba),
                        emf,
                    ),
                    Ok(_) => Err("not on Windows".to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let with_emf = both.is_ok();
                if !with_emf {
                    ctx.copy_image(egui::ColorImage::from_rgba_unmultiplied(px, &r.rgba));
                }
                self.copied_page = Some(px);
                let form = if with_emf { " and a metafile" } else { "" };
                self.say(format!(
                    "Copied the page as a picture{form} of {} x {} pixels (300 dpi, 40 % size)",
                    px[0], px[1]
                ));
            }
            Err(e) => self.warn(format!("Could not copy the page: {e}")),
        }
    }

    /// Format > Fit Page To Printer: the page gets the printable size of the default printer for
    /// Page Setup's paper in the page's orientation (the Page Setup paper less the measured edges
    /// when no printer answers, and always under test); the layers keep their frames, which are
    /// fractions of the page. One undo step.
    /// Opens the 3D Axis window (T164) on axis `dim` of a 3D layer.
    fn open_axis3d(&mut self, graph: WindowId, layer: usize, dim: usize) {
        match crate::dialogs::axis3d::Axis3dDialog::new(self.doc.project(), graph, layer, dim) {
            Some(d) => {
                self.axis3d = Some(d);
                self.focus(graph);
                self.graphs.entry(graph).or_default().active_layer = layer;
            }
            None => self.warn("The 3D Axis window needs a 3D layer"),
        }
    }

    fn fit_page_to_printer(&mut self, graph: WindowId) {
        let setup = self.doc.project().page_setup;
        let Some(page) = self.doc.project().graph(graph).map(|g| (g.page.width_pt, g.page.height_pt)) else { return };
        #[cfg(windows)]
        let (w, h) = if cfg!(test) { None } else { crate::winprint::printable_size_pt(&setup, page) }
            .unwrap_or_else(|| setup.fallback_printable_pt(page.0, page.1));
        #[cfg(not(windows))]
        let (w, h) = setup.fallback_printable_pt(page.0, page.1);
        self.doc.apply("Fit Page to Printer", |p| {
            if let Some(g) = p.graph_mut(graph) {
                g.resize_page(w, h, false);
            }
        });
        self.say("Fitted the page to the printer");
    }

    /// File > Print... (Ctrl+P): the active window printed as a vector drawing through the
    /// system's print dialog. A graph page is scaled into the paper of Page Setup as File > Print
    /// Preview shows; a workbook's active sheet prints as a table, split over as many pages as it
    /// needs (graphing.md 19.8). Windows plays the metafiles into the printer; macOS shows PDFKit's panel for a graph; elsewhere,
    /// and for a sheet on macOS, the PDF goes to `lp`.
    fn print_graph(&mut self, window: WindowId) {
        let project = self.doc.project();
        let mut setup = project.page_setup;
        // (title, page size in points, the pages as metafiles, the same as one PDF)
        let (title, page, what) = if let Some(g) = project.graph(window) {
            (g.name.clone(), (g.page.width_pt, g.page.height_pt), "graph")
        } else if let Some(l) = project.layout(window) {
            (l.name.clone(), (l.width_pt, l.height_pt), "layout page")
        } else if let Some(b) = project.workbook(window) {
            // A sheet's orientation follows the paper (Automatic means portrait, as in Print
            // Preview), never the printed area, which can be wider than tall.
            setup.orientation = setup.orientation_for(0.0, 1.0);
            let area = sheaf_plot::sheet_pages(b.sheet(), &setup).map(|p| p.area).unwrap_or((0.0, 0.0));
            (b.name.clone(), area, "sheet")
        } else {
            return;
        };
        if cfg!(test) {
            // Tests never reach a printer.
            self.say("Printing is skipped under test");
            return;
        }
        let is_graph = what == "graph";
        #[cfg(windows)]
        {
            let pages = if is_graph {
                sheaf_plot::render_emf(project, window).map(|e| vec![e])
            } else if what == "layout page" {
                sheaf_plot::render_layout_emf(project, window, 300.0).map(|e| vec![e])
            } else {
                project
                    .workbook(window)
                    .map_or(Ok(Vec::new()), |b| sheaf_plot::render_sheet_emf_pages(b.sheet(), &setup))
            };
            match pages {
                Ok(pages) => match crate::winprint::print_with_dialog(&pages, page, &setup, &title) {
                    Ok(crate::winprint::Outcome::Printed) => self.say(format!("Printed {title}")),
                    Ok(crate::winprint::Outcome::Cancelled) => {}
                    Err(e) => self.message = Some((format!("Could not print the {what}"), e)),
                },
                Err(e) => self.warn(format!("Could not print the {what}: {e}")),
            }
        }
        #[cfg(target_os = "macos")]
        if is_graph {
            match sheaf_plot::render_print_pdf(project, window) {
                Ok(pdf) => match crate::macprint::print_with_dialog(&pdf, &setup, page, &title) {
                    Ok(crate::macprint::Outcome::Printed) => self.say(format!("Printed {title}")),
                    Ok(crate::macprint::Outcome::CancelledOrFailed) => {}
                    Err(e) => self.message = Some(("Could not print the graph".to_string(), e)),
                },
                Err(e) => self.warn(format!("Could not print the graph: {e}")),
            }
            return;
        }
        #[cfg(not(windows))]
        {
            let pdf = if is_graph {
                sheaf_plot::render_pdf(project, window)
            } else if what == "layout page" {
                sheaf_plot::render_layout_image(project, window, "pdf", 300.0)
            } else {
                project.workbook(window).map_or(Ok(Vec::new()), |b| sheaf_plot::render_sheet_pdf(b.sheet(), &setup))
            };
            match pdf {
                Ok(pdf) => match crate::winprint::print_pdf(&pdf, &setup, page, &title) {
                    Ok(()) => self.say(format!("Sent {title} to the default printer")),
                    Err(e) => self.message = Some((format!("Could not print the {what}"), e)),
                },
                Err(e) => self.warn(format!("Could not print the {what}: {e}")),
            }
        }
    }

    /// Gadgets > Integrate: the gadget goes on the selected plot of the active layer, or its first
    /// visible plot with numbers, over the middle half of the X axis.
    fn start_gadget(&mut self, graph: WindowId, kind: crate::gadget::GadgetKind) {
        let project = self.doc.project();
        let Some(g) = project.graph(graph) else { return };
        let state = self.graphs.entry(graph).or_default();
        let layer = state.active_layer.min(g.layers.len().saturating_sub(1));
        // 2D Integrate (T189) goes on a layer with a heat map or contour plot: the active one, else the first.
        if kind == crate::gadget::GadgetKind::Integrate2d {
            let has_field = |i: &usize| g.layers.get(*i).is_some_and(|l| l.field.is_some() && !l.display.hidden);
            let Some(layer) = Some(layer).filter(has_field).or_else(|| (0..g.layers.len()).find(has_field)) else {
                self.warn("2D Integrate needs a contour or heat map plot".to_string());
                return;
            };
            let Some(geom) = sheaf_plot::layout_geometry(project, graph).ok().and_then(|v| v.into_iter().nth(layer))
            else {
                return;
            };
            let (x0, x1) = (geom.x.from_unit(0.25), geom.x.from_unit(0.75));
            let (y0, y1) = (geom.y.from_unit(0.25), geom.y.from_unit(0.75));
            state.active_layer = layer;
            state.gadget = Some(crate::gadget::Integrator {
                kind,
                layer,
                plot: 0,
                x0,
                x1,
                y0,
                y1,
                end_point_baseline: false,
                circle: false,
                absolute: false,
            });
            self.focus(graph);
            return;
        }
        let numeric = |i: &usize| {
            g.layers
                .get(layer)
                .and_then(|l| l.plots.get(*i))
                .is_some_and(|p| p.visible && project.column(p.y).is_some_and(|c| c.numeric().is_some()))
        };
        let selected = match state.selected {
            Some((l, sheaf_plot::scene::ObjectKind::Plot(p))) if l == layer => Some(p).filter(numeric),
            _ => None,
        };
        let count = g.layers.get(layer).map_or(0, |l| l.plots.len());
        let Some(plot) = selected.or_else(|| (0..count).find(numeric)) else {
            self.warn("Nothing to measure: the active layer has no plot of numbers".to_string());
            return;
        };
        let Some(geom) = sheaf_plot::layout_geometry(project, graph).ok().and_then(|v| v.into_iter().nth(layer)) else {
            return;
        };
        let placement =
            sheaf_plot::stack_places(project, graph, layer).ok().and_then(|p| p.get(plot).copied()).unwrap_or_default();
        // A Vertical Cursor (T144) is a thin band in the middle: its centre is the cursor.
        let (u0, u1) = if kind == crate::gadget::GadgetKind::VerticalCursor { (0.49, 0.51) } else { (0.25, 0.75) };
        let (x0, x1) = (placement.source_x(geom.x.from_unit(u0)), placement.source_x(geom.x.from_unit(u1)));
        // A Cluster (T146) is a rectangle over the middle half of both axes.
        let (y0, y1) = (placement.source_y(geom.y.from_unit(0.25)), placement.source_y(geom.y.from_unit(0.75)));
        state.gadget = Some(crate::gadget::Integrator {
            kind,
            layer,
            plot,
            x0,
            x1,
            y0,
            y1,
            end_point_baseline: false,
            circle: false,
            absolute: false,
        });
        self.focus(graph);
    }

    /// A command of a graph's Integrate gadget. New Output runs the analysis tool Integrate on the
    /// same rows, so the result lands in a sheet as from Analysis > Integrate.
    fn gadget(&mut self, graph: WindowId, op: crate::gadget::GadgetOp) {
        use crate::gadget::GadgetOp;
        let Some(state) = self.graphs.get_mut(&graph) else { return };
        let Some(g) = &mut state.gadget else { return };
        match op {
            GadgetOp::ToggleBaseline => g.end_point_baseline = !g.end_point_baseline,
            GadgetOp::Preferences => {
                let (from, to) = (g.x0.min(g.x1), g.x0.max(g.x1));
                self.gadget_prefs =
                    Some(GadgetPrefs { graph, from, to, baseline: g.end_point_baseline, absolute: g.absolute });
            }
            GadgetOp::SetRegion(circle) => g.circle = circle,
            GadgetOp::Close => state.gadget = None,
            // 2D Integrate (T198): a row of the results table.
            GadgetOp::NewOutput if g.kind == crate::gadget::GadgetKind::Integrate2d => {
                let g = *g;
                let Some(v) = g.volume(self.doc.project(), graph) else {
                    self.warn("No cells in the region".to_string());
                    return;
                };
                let rect = [g.x0.min(g.x1), g.x0.max(g.x1), g.y0.min(g.y1), g.y0.max(g.y1)];
                self.doc.apply("2D Integrate", |p| crate::gadget::write_volume(p, rect, &v));
                self.say(format!("The volume was added to {}", crate::gadget::VOLUME_BOOK));
            }
            GadgetOp::NewOutput => {
                let g = *g;
                let column = self
                    .doc
                    .project()
                    .graph(graph)
                    .and_then(|gr| gr.layers.get(g.layer))
                    .and_then(|l| l.plots.get(g.plot))
                    .map(|p| p.y);
                let Some(column) = column else { return };
                let request = sheaf_core::analysis::Request {
                    columns: vec![column],
                    rows: sheaf_core::analysis::RowRange::XBetween(g.x0, g.x1),
                    tool: sheaf_core::analysis::Tool::Integrate(sheaf_core::analysis::IntegrateSpec {
                        area_type: if g.absolute {
                            sheaf_core::analysis::AreaType::Absolute
                        } else {
                            sheaf_core::analysis::AreaType::Mathematical
                        },
                        end_point_baseline: g.end_point_baseline,
                        cumulative: false,
                    }),
                };
                match self.doc.edit("Integrate", |p| p.analyze(&request)) {
                    Ok(_) => self.say("The integration result was added to the workbook of the plot's data"),
                    Err(e) => self.warn(format!("Could not integrate: {e}")),
                }
            }
        }
    }

    /// Graph > New Layer(Axes): add one layer in a single undo step.
    fn add_layer(&mut self, graph: WindowId, active: usize, choice: LayerChoice, activate: bool, auto_offset: bool) {
        let Some(added) = self.doc.apply(sheaf_core::layers::ADD_LAYER_UNDO, |p| {
            let g = p.graph_mut(graph)?;
            match choice {
                LayerChoice::Preset(kind) if auto_offset => g.add_layer_with_auto_offset(kind, active),
                LayerChoice::Preset(kind) => g.add_layer(kind, active),
                LayerChoice::UserDefined(options) => g.add_user_defined_layer(options, active, auto_offset),
            }
        }) else {
            return;
        };
        let state = self.graphs.entry(graph).or_default();
        state.active_layer = if activate { added } else { active };
        state.selected = None;
    }

    /// Delete Layer: one undo step. The active layer, the selection and the windows that name a
    /// layer by its index are repaired for the layers that moved up.
    fn delete_layer(&mut self, graph: WindowId, layer: usize) {
        let Some(g) = self.doc.project().graph(graph) else { return };
        if layer >= g.layers.len() || g.layers.len() < 2 {
            return;
        }
        let resolved = sheaf_plot::layout_geometry(self.doc.project(), graph).ok();
        self.doc.apply(sheaf_core::layers::DELETE_LAYER_UNDO, |p| {
            graph_view::apply_op(p, graph, GraphOp::DeleteLayer { layer }, resolved.as_deref())
        });
        let remaining = self.doc.project().graph(graph).map_or(1, |g| g.layers.len());
        let state = self.graphs.entry(graph).or_default();
        state.active_layer = if state.active_layer == layer {
            sheaf_core::layers::active_after_delete(layer, remaining)
        } else {
            sheaf_core::layers::index_after_delete(state.active_layer, layer).unwrap_or(0)
        };
        state.selected =
            state.selected.and_then(|(l, kind)| sheaf_core::layers::index_after_delete(l, layer).map(|l| (l, kind)));
        state.reset_layer_interactions();
        // Windows that edit a layer by its index close rather than edit the wrong layer.
        if self.graph_props.as_ref().is_some_and(|d| d.graph == graph) {
            self.graph_props = None;
        }
        if self.layer_contents.as_ref().is_some_and(|d| d.graph == graph) {
            self.layer_contents = None;
        }
        if self.plot_setup.as_ref().is_some_and(|d| d.graph == graph) {
            self.plot_setup = None;
        }
        self.shape_dialog = None;
    }

    /// Applies a change made from a graph window (right-click menus, Delete, Ctrl+R) as one undo
    /// step.
    fn edit_graph(&mut self, graph: WindowId, op: GraphOp) {
        if let GraphOp::HidePlot { layer, plot } = op {
            self.doc.apply("Hide data plot", |p| objects::set_plot_visible(p, graph, layer, plot, false));
            return;
        }
        if let GraphOp::SetStack { layer, stack } = op {
            let same = self
                .doc
                .project()
                .graph(graph)
                .and_then(|g| g.layers.get(layer))
                .is_none_or(|l| std::mem::discriminant(&l.stack) == std::mem::discriminant(&stack));
            if !same {
                self.doc.apply("Offset grouped data in layer", |p| graph_view::apply_op(p, graph, op, None));
            }
            return;
        }
        let label = match op {
            GraphOp::SetPlotKind { .. } => "Change plot type",
            GraphOp::HideAxis { .. } => "Hide axis",
            GraphOp::ToggleGrids { .. } => "Show or hide grids",
            GraphOp::Rescale { .. } => "Rescale to show all",
            GraphOp::ExchangeAxes { .. } => "Exchange X-Y axes",
            GraphOp::AxisZoom { zoom_in: true, .. } => "Axis zoom in",
            GraphOp::AxisZoom { .. } => "Axis zoom out",
            GraphOp::AxisCenter { .. } => "Axis center",
            GraphOp::HideLegend { .. } => "Delete legend",
            GraphOp::OrderTitle { front: true, .. } => "Bring axis title to front",
            GraphOp::OrderTitle { front: false, .. } => "Push axis title to back",
            GraphOp::OrderLegend { front: true, .. } => "Bring legend to front",
            GraphOp::OrderLegend { front: false, .. } => "Push legend to back",
            GraphOp::LegendBehindData { behind: true, .. } => "Push legend to back of data",
            GraphOp::LegendBehindData { behind: false, .. } => "Bring legend to front of data",
            GraphOp::LegendTextColorFollowsPlot { .. } => "Text Color Follows Plot",
            GraphOp::ReconstructLegend { .. } => "Reconstruct legend",
            GraphOp::UpdateLegend { .. } => "Update legend",
            GraphOp::HideTitle { .. } => "Delete axis title",
            GraphOp::DeleteLabel { .. } => "Delete text",
            GraphOp::DeleteShape { .. } => "Delete drawn object",
            GraphOp::DeleteSlope { .. } => "Delete slope annotation",
            GraphOp::DeleteStraightLine { .. } => "Delete straight line",
            GraphOp::AddScale { kind, .. } => match kind {
                sheaf_core::layer_scale::LayerScaleKind::Color => "Add color scale",
                sheaf_core::layer_scale::LayerScaleKind::Bubble => "Add bubble scale",
            },
            GraphOp::DeleteScale { .. } => "Delete scale",
            GraphOp::OrderShape { front: true, .. } => "Bring drawn object to front",
            GraphOp::OrderShape { front: false, .. } => "Push drawn object to back",
            GraphOp::OrderLabel { front: true, .. } => "Bring text to front",
            GraphOp::OrderLabel { front: false, .. } => "Push text to back",
            GraphOp::HideLayer { .. } => "Hide layer",
            GraphOp::HideOtherLayers { .. } => "Hide other layers",
            GraphOp::DataMarkers { op, .. } => match op {
                crate::actions::MarkerOp::Mark { .. } => "Mark data range",
                crate::actions::MarkerOp::Move { .. } => "Move data marker",
                crate::actions::MarkerOp::Clear => "Clear data markers",
                crate::actions::MarkerOp::SetRange => "Set display range",
                crate::actions::MarkerOp::ResetRange => "Reset to full range",
                crate::actions::MarkerOp::Show(_) | crate::actions::MarkerOp::Size(_) => "Analysis markers",
            },
            GraphOp::AddLayer { .. } => sheaf_core::layers::ADD_LAYER_UNDO,
            GraphOp::DeleteLayer { .. } => sheaf_core::layers::DELETE_LAYER_UNDO,
            GraphOp::MovePlotToAxis { right: true, .. } => "Move plot to right Y axis",
            GraphOp::MovePlotToAxis { .. } => "Move plot to left Y axis",
            GraphOp::MovePlotToLayer { .. } => "Move plot to layer",
            GraphOp::RemoveTrendline { .. } => "Remove trendline",
            GraphOp::SetPlotColumn { y: false, .. } => "Change X",
            GraphOp::SetPlotColumn { .. } => "Change Y",
            GraphOp::Ungroup { .. } => "Ungroup",
            GraphOp::HidePlot { .. } | GraphOp::SetStack { .. } => unreachable!("handled above"),
        };
        if let GraphOp::AddLayer { layer, kind } = op {
            self.add_layer(graph, layer, LayerChoice::Preset(kind), true, false);
            return;
        }
        if let GraphOp::DeleteLayer { layer } = op {
            self.delete_layer(graph, layer);
            return;
        }
        // The present axis ranges, for the zoom (an automatic axis has no stored limits).
        let resolved = sheaf_plot::layout_geometry(self.doc.project(), graph).ok();
        let shapes = self.shape_dialog_shapes();
        let selected = self.snapshot_shape_interactions();
        self.doc.apply(label, |p| graph_view::apply_op(p, graph, op, resolved.as_deref()));
        // Keep the moved object selected at its new index; clear any other selection in a changed
        // shape stack, whose old index may now identify a different object.
        let mut preserved = None;
        if let GraphOp::OrderShape { layer, shape, front } = op
            && let Some((_, _, Some(before))) = selected.iter().find(|(id, l, _)| *id == graph && *l == layer)
            && let Some(after) = self.doc.project().graph(graph).and_then(|g| g.layers.get(layer)).map(|l| &l.shapes)
            && before != after
            && shape < before.len()
            && let Some(state) = self.graphs.get_mut(&graph)
            && state.selected == Some((layer, sheaf_plot::scene::ObjectKind::Shape(shape)))
        {
            state.selected =
                Some((layer, sheaf_plot::scene::ObjectKind::Shape(if front { after.len() - 1 } else { 0 })));
            preserved = Some((graph, layer));
        }
        // A label moved in its layer's list: a selected label of that layer keeps pointing at the same label.
        if let GraphOp::OrderLabel { layer, label, front } = op
            && let Some(state) = self.graphs.get_mut(&graph)
            && let Some((l, sheaf_plot::scene::ObjectKind::Label(k))) = state.selected
            && l == layer
        {
            let len = self.doc.project().graph(graph).and_then(|g| g.layers.get(layer)).map_or(0, |l| l.labels.len());
            let moved = |k: usize| match (k == label, front) {
                (true, true) => len.saturating_sub(1),
                (true, false) => 0,
                (false, true) if k > label => k - 1,
                (false, false) if k < label => k + 1,
                _ => k,
            };
            state.selected = Some((layer, sheaf_plot::scene::ObjectKind::Label(moved(k))));
        }
        self.clear_changed_shape_selections(selected, preserved);
        self.close_shape_dialog_if_changed(shapes);
    }

    /// Snapshot layers with index-based shape interactions so a changed layer cannot leave an
    /// index pointing at a different object.
    fn snapshot_shape_interactions(&self) -> Vec<ShapeLayerSnapshot> {
        self.graphs
            .iter()
            .flat_map(|(id, state)| {
                state.shape_interaction_layers().into_iter().map(move |layer| {
                    let shapes =
                        self.doc.project().graph(*id).and_then(|g| g.layers.get(layer)).map(|l| l.shapes.clone());
                    (*id, layer, shapes)
                })
            })
            .collect()
    }

    fn clear_changed_shape_selections(
        &mut self,
        before: Vec<ShapeLayerSnapshot>,
        preserved: Option<(WindowId, usize)>,
    ) {
        for (id, layer, shapes) in before {
            let current = self.doc.project().graph(id).and_then(|g| g.layers.get(layer)).map(|l| &l.shapes);
            if shapes.as_ref() != current
                && let Some(state) = self.graphs.get_mut(&id)
            {
                state.invalidate_shape_interaction(layer, preserved == Some((id, layer)));
            }
        }
    }

    /// The drawn objects of the layer the drawn-object window edits, if it is open.
    fn shape_dialog_shapes(&self) -> Option<Vec<sheaf_core::shape::Shape>> {
        self.shape_dialog.as_ref().and_then(|d| d.layer_shapes(&self.doc)).map(<[_]>::to_vec)
    }

    /// Closes the drawn-object window when Delete, Undo or Redo changed the objects of its layer:
    /// its index may now name another object, and Apply would edit that one.
    fn close_shape_dialog_if_changed(&mut self, before: Option<Vec<sheaf_core::shape::Shape>>) {
        if let Some(d) = &self.shape_dialog
            && d.layer_shapes(&self.doc) != before.as_deref()
        {
            self.shape_dialog = None;
        }
    }

    /// Opens the Window Switcher on the windows of the Project Explorer's selected folder, most
    /// recently used first. With fewer than two windows nothing opens.
    fn open_switcher(&mut self, ctx: &egui::Context, backward: bool, sticky: bool) {
        let ids = explorer::windows_in(self.doc.project(), &self.panels.explorer.folder);
        let ids = self.workspace.recent_order(&ids);
        let workspace = &self.workspace;
        self.switcher = crate::switcher::Switcher::open(
            ctx,
            self.doc.project(),
            &ids,
            |id| !workspace.is_shown(id),
            workspace.active(),
            backward,
            sticky,
        );
    }

    /// The keys of the Window Switcher: Ctrl+Tab opens it, Tab and the arrows move, releasing
    /// Ctrl or Enter activates the selection, Esc closes it. Runs before the grid reads keys.
    fn switcher_keys(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        use crate::switcher::Nav;
        let open = self.switcher.is_some();
        let keys = crate::switcher::take_keys(ctx, open, !self.dialog_open());
        for nav in keys {
            match &mut self.switcher {
                None => {
                    if matches!(nav, Nav::Next | Nav::Prev) {
                        self.open_switcher(ctx, nav == Nav::Prev, false);
                    }
                }
                Some(s) => match nav {
                    Nav::Confirm => {
                        actions.push(Action::Focus(s.selected()));
                        self.switcher = None;
                    }
                    Nav::Cancel => self.switcher = None,
                    other => s.go(other),
                },
            }
        }
        if let Some(s) = &self.switcher
            && !s.sticky
            && !crate::switcher::ctrl_down(ctx)
        {
            actions.push(Action::Focus(s.selected()));
            self.switcher = None;
        }
    }

    fn focus(&mut self, id: WindowId) {
        if self.doc.project().window(id).is_some() {
            if !self.workspace.contains(id) {
                let graph = self.doc.project().graph(id).is_some();
                self.workspace.add(id, graph, false);
            }
            self.workspace.activate(id);
        }
    }

    /// A change from a graph's Data menu tools. A column that a formula keeps up to date refuses it, as the
    /// worksheet refuses a typed value.
    fn data_edit(&mut self, edit: DataEdit) {
        let result = match edit {
            DataEdit::Mask { cells, masked } => self.doc.mask_points(&cells, masked),
            DataEdit::RemoveBad { cells } => self.doc.remove_bad_points(&cells),
            DataEdit::Move { y, x } => self.doc.move_point(y, x),
        };
        match result {
            Ok(report) if report.locked > 0 => {
                self.warn("That column is calculated from a formula; change the formula in Set Column Values");
            }
            Ok(_) => {}
            Err(e) => self.warn(format!("Could not change the data: {e}")),
        }
    }

    /// Shows a message in the status bar and keeps it in the Messages Log.
    fn say(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.logs.info(text.clone());
        self.status = text;
    }

    /// Like [`SheafApp::say`] for something that did not work or was left out.
    fn warn(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.logs.warn(text.clone());
        self.status = text;
    }

    // ----- the AI link ---------------------------------------------------

    /// Turns the endpoint of `sheaf-mcp --attach` on or off (Tools > AI Assistants (MCP)). A start
    /// that fails (another Sheaf has it, no folder can be used) is a warning, and the link stays off.
    fn set_link(&mut self, on: bool) {
        if on == self.live.active() {
            return;
        }
        if on {
            match self.live.start(self.doc.generation()) {
                Ok(()) => self.say("AI link on: an assistant started with sheaf-mcp --attach can connect"),
                Err(e) => self.warn(format!("The AI link could not start: {e}")),
            }
        } else {
            self.live.stop();
            self.say("AI link off");
        }
    }

    /// Once a frame, after the owner's actions and before the new windows are opened: runs the calls
    /// of the connected assistants. A call that changes the project waits for a dialog to close and
    /// for a cell edit to end.
    fn live_step(&mut self) {
        if !self.live.active() {
            return;
        }
        let editing = self.books.values().any(|b| b.editing.is_some() || b.editing_label.is_some());
        let dialog_open = self.dialog_open();
        let blocked = editing || dialog_open;
        let context = live::Context::new(self.active, &self.doc, &self.books, &self.graphs, editing, dialog_open);
        let app_context = |doc: &Document| context.snapshot(doc);
        let files = sheaf_mcp::files::OpenFiles {
            project: self.path.clone(),
            origin: self.foreign_source.clone(),
            recovered_from: self.recovered_from.clone(),
        };
        let now = std::time::Instant::now();
        let recovery = self.startup.recovery_dir.as_deref();
        for note in self.live.step(&mut self.doc, blocked, &files, recovery, now, &app_context) {
            match note {
                Note::Info(t) => self.say(t),
                Note::Warn(t) => self.warn(t),
            }
        }
    }

    fn token(&mut self) -> u64 {
        self.next_token += 1;
        self.next_token
    }

    /// Only completed AI calls at the top of history are eligible. An active owner edit or dialog
    /// must finish first; the handler checks this again in case an enabled menu became stale.
    fn can_undo_ai_changes(&self) -> bool {
        self.doc.undo_label().is_some_and(|label| label.starts_with("AI: "))
            && !self.books.values().any(|book| book.editing.is_some() || book.editing_label.is_some())
            && !self.dialog_open()
    }

    fn undo_ai_changes(&mut self, ctx: &egui::Context) {
        if !self.can_undo_ai_changes() {
            return;
        }
        let mut count = 0;
        while self.doc.undo_label().is_some_and(|label| label.starts_with("AI: ")) {
            // Reuse ordinary Undo, including its existing cleanup hooks and per-step log entry.
            self.apply(Action::Undo, ctx);
            count += 1;
        }
        self.say(format!("Undone {count} AI change{}", if count == 1 { "" } else { "s" }));
    }

    /// A snapshot of the selected whole text object. Dialog editors own their own selection.
    fn text_format(&self) -> Option<crate::text_format::Snapshot> {
        if self.dialog_open() || self.books.values().any(|b| b.editing.is_some() || b.editing_label.is_some()) {
            return None;
        }
        let graph = self.active?;
        if let Some(book) = self.doc.project().workbook(graph) {
            let sheet = book.active_sheet.min(book.sheets.len().checked_sub(1)?);
            let range = grid::format_target(self.books.get(&graph)?, &book.sheets[sheet]).ok()?;
            return self.worksheet_format.snapshot(&self.doc, graph, sheet, range);
        }
        let (layer, object) = self.graphs.get(&graph)?.selected?;
        crate::text_format::Snapshot::graph(&self.doc, graph, layer, object)
    }

    /// The selected object's non-font Format-bar controls (markup, Fill and Line Color), gated
    /// like `text_format`.
    fn object_format(&self) -> Option<crate::object_format::Snapshot> {
        if self.dialog_open() || self.books.values().any(|b| b.editing.is_some() || b.editing_label.is_some()) {
            return None;
        }
        let graph = self.active?;
        let (layer, object) = self.graphs.get(&graph)?.selected?;
        crate::object_format::Snapshot::graph(&self.doc, graph, layer, object)
    }

    // ----- actions -------------------------------------------------------

    fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::NewProject => {
                if self.doc.dirty {
                    self.pending = Some(Pending::New);
                } else {
                    self.do_pending(Pending::New, ctx);
                }
            }
            Action::NewGraph => self.new_graph(),
            Action::SaveProjectWithoutData => self.save_project_without_data(),
            Action::DuplicateProjectWithoutData => {
                if self.doc.dirty {
                    self.pending = Some(Pending::DuplicateNoData);
                } else {
                    self.do_pending(Pending::DuplicateNoData, ctx);
                }
            }
            Action::OpenProject => {
                // The chooser comes first: only a project replaces this one, so only a project asks.
                if let Some(path) = self.choose_project_file() {
                    self.open_picked(path);
                }
            }
            Action::SaveProject => {
                self.save(false);
            }
            Action::SaveProjectAs => {
                self.save(true);
            }
            Action::Import(path) => {
                let pick = if path.is_some() { ImportPick::Single } else { ImportPick::Multiple };
                self.start_import(path.map(|p| vec![p]), pick);
            }
            Action::ImportPick(pick) => self.start_import(None, pick),
            Action::GoToCell(book) => self.open_go_to(book),
            Action::Power(kind) => self.power = Some(PowerDialog::new(kind)),
            Action::AcceptanceSampling => self.acceptance = Some(Default::default()),
            Action::FindInSheets(book) => self.open_find(book, false),
            Action::ReplaceInSheets(book) => self.open_find(book, true),
            Action::ImportWizard => {
                // An open wizard keeps its settings: the command only brings it to the front.
                if self.import_wizard.is_some() {
                    ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, egui::Id::new("import_wizard")));
                } else {
                    self.import_wizard = Some(self.wizard(Vec::new()));
                }
            }
            Action::Undo => {
                let selected = self.snapshot_shape_interactions();
                let shapes = self.shape_dialog_shapes();
                if let Some(label) = self.doc.undo() {
                    self.say(format!("Undone: {label}"));
                }
                self.close_shape_dialog_if_changed(shapes);
                self.clear_changed_shape_selections(selected, None);
            }
            Action::Redo => {
                let selected = self.snapshot_shape_interactions();
                let shapes = self.shape_dialog_shapes();
                if let Some(label) = self.doc.redo() {
                    self.say(format!("Redone: {label}"));
                }
                self.close_shape_dialog_if_changed(shapes);
                self.clear_changed_shape_selections(selected, None);
            }
            Action::UndoAiChanges => self.undo_ai_changes(ctx),
            Action::WorksheetColorDialog(dialog) => {
                if self.text_format().is_some_and(|state| dialog.matches(&state)) {
                    self.worksheet_color = Some(*dialog);
                }
            }
            Action::FormatText(request) => {
                if self.text_format().is_some_and(|state| state.target == request.target) {
                    crate::text_format::apply(&mut self.doc, request);
                }
            }
            Action::FormatObject(request) => {
                if self.object_format().is_some_and(|state| state.target == request.target) {
                    crate::object_format::apply(&mut self.doc, request);
                }
            }
            Action::ImportImage => {
                let picked = rfd::FileDialog::new()
                    .set_title("Import Image")
                    .add_filter("Pictures", &sheaf_io::image_import::EXTENSIONS)
                    .pick_file();
                if let Some(path) = picked {
                    match crate::matrix_view::import_image(&mut self.doc, &path) {
                        Ok(_) => self.say(format!("Imported {} into a new matrix book", path.display())),
                        Err(e) => self.warn(e),
                    }
                }
            }
            Action::ImportBinary2d => {
                let picked = rfd::FileDialog::new().set_title("Import Binary 2D Array").pick_file();
                if let Some(path) = picked {
                    // Into the active matrix sheet, as Origin's Output; else a new matrix book.
                    let target = self.active.and_then(|a| {
                        self.doc
                            .project()
                            .matrix_book(a)
                            .map(|b| (a, b.active_sheet.min(b.sheets.len().saturating_sub(1))))
                    });
                    match crate::dialogs::import_bin2d::ImportBin2dDialog::new(path, target) {
                        Ok(d) => self.import_bin2d = Some(d),
                        Err(e) => self.warn(e),
                    }
                }
            }
            Action::NewLayout => {
                let mut new = None;
                self.doc.apply("New layout", |p| new = Some(p.add_layout()));
                if let Some(id) = new {
                    self.focus(id);
                }
            }
            Action::LayoutAddShape { layout, shape, arrow } => {
                self.doc.apply("Add drawing to layout", |p| {
                    if let Some(l) = p.layout_mut(layout) {
                        l.add_shape(shape, arrow);
                    }
                });
            }
            Action::LayoutShape { layout, index } => {
                self.layout_shape = crate::dialogs::layout_shape::LayoutShapeDialog::new(&self.doc, layout, index);
            }
            Action::LayoutText { layout, index } => {
                use crate::dialogs::layout_text::LayoutText;
                self.layout_text = match index {
                    None => Some(LayoutText::new(layout)),
                    Some(k) => LayoutText::edit(&self.doc, layout, k),
                };
            }
            Action::LayoutPageSetup { layout } => {
                self.layout_page_setup = crate::dialogs::layout_page_setup::LayoutPageSetup::new(&self.doc, layout);
            }
            Action::LayoutExport { layout } => {
                let name = self.doc.project().layout(layout).map(|l| l.name.clone()).unwrap_or_default();
                let picked = rfd::FileDialog::new()
                    .set_title("Export layout page")
                    .add_filter("PNG image", &["png"])
                    .add_filter("JPEG image", &["jpg", "jpeg"])
                    .add_filter("TIFF image", &["tif", "tiff"])
                    .add_filter("Windows bitmap", &["bmp"])
                    .add_filter("SVG drawing", &["svg"])
                    .add_filter("PDF document", &["pdf"])
                    .add_filter("Enhanced Metafile", &["emf"])
                    .set_file_name(format!("{name}.png"))
                    .save_file();
                if let Some(path) = picked {
                    match crate::layout_view::write_layout_image(self.doc.project(), layout, &path) {
                        Ok(written) => self.say(format!("Exported {}", written.display())),
                        Err(e) => self.warn(e),
                    }
                }
            }
            Action::LayoutSetItem { layout, index, rect, resize } => {
                let label = if resize { "Resize layout item" } else { "Move layout item" };
                self.doc.apply(label, |p| {
                    if let Some(it) = p.layout_mut(layout).and_then(|l| l.items.get_mut(index)) {
                        (it.x, it.y, it.w, it.h) = rect;
                    }
                });
            }
            Action::LayoutDeleteItem { layout, index } => {
                self.doc.apply("Delete layout item", |p| {
                    if let Some(l) = p.layout_mut(layout)
                        && index < l.items.len()
                    {
                        l.items.remove(index);
                    }
                });
            }
            Action::LayoutAddGraph { layout, sheets } => {
                match crate::dialogs::layout_add_graph::LayoutAddGraph::new(&self.doc, layout, sheets) {
                    Some(d) => self.layout_add_graph = Some(d),
                    None if sheets => self.warn("The project has no worksheet to add"),
                    None => self.warn("The project has no graph to add"),
                }
            }
            Action::NewMatrix => {
                self.doc.apply("New matrix", |p| {
                    p.add_matrix_book("");
                });
            }
            Action::SetMatrixCell { id, sheet, row, col, value } => {
                let mut failed = None;
                self.doc.apply("Edit matrix cell", |p| {
                    if let Ok(s) = p.matrix_sheet_mut(id, sheet)
                        && let Err(e) = s.set_value(row, col, value.unwrap_or(f64::NAN))
                    {
                        failed = Some(e.to_string());
                    }
                });
                if let Some(e) = failed {
                    self.say(e);
                }
            }
            Action::MatrixCmd { id, cmd } => self.matrix_cmd(id, cmd),
            Action::ImageCmd { id, cmd } => self.image_cmd(id, cmd),
            Action::ConvertToMatrix { book, columns, xyz } => {
                let sheet = self.doc.project().workbook(book).map_or(0, |b| b.active_sheet);
                let made = self.doc.edit("Convert to Matrix", |p| match (xyz, columns.as_slice()) {
                    (true, [x, y, z]) => p.xyz_to_matrix(*x, *y, *z),
                    (true, _) => Err(sheaf_core::project::ProjectError::NoSuchWindow(book)),
                    (false, []) => p.worksheet_to_matrix(book, sheet, None, &Default::default()),
                    (false, cols) => p.worksheet_to_matrix(book, sheet, Some(cols), &Default::default()),
                });
                match made {
                    Ok(_) => self.say("Converted to a matrix".to_string()),
                    Err(e) => self.warn(format!("Convert to Matrix: {e}")),
                }
            }
            Action::PlotField { template, matrix, xyz } => {
                let made = self.doc.edit("New graph", |p| match (matrix, xyz) {
                    (Some(id), _) => {
                        let sheet = p.matrix_book(id).map_or(0, |b| b.active_sheet);
                        p.plot_matrix(template, id, sheet)
                    }
                    (None, Some((x, y, z))) => p.plot_xyz(template, x, y, z),
                    (None, None) => Err(sheaf_core::project::ProjectError::NoSuchWindow(sheaf_core::WindowId(0))),
                });
                if let Err(e) = made {
                    self.warn(e.to_string());
                }
            }
            Action::SetMatrixSheet { id, sheet } => {
                self.doc.adjust_view(|p| {
                    if let Some(b) = p.matrix_book_mut(id)
                        && sheet < b.sheets.len()
                    {
                        b.active_sheet = sheet;
                    }
                });
            }
            Action::NewNotes => {
                self.doc.apply("New notes", |p| {
                    p.add_notes();
                });
            }
            Action::DefineName { column, row } => {
                // Several selected cells of this column: a named range (T400); of several columns: a block (T432).
                let (from, to, first, last) = self.selected_block_of(column, row).unwrap_or((column, column, row, row));
                let current = self.doc.project().name_of_block(from, to, first, last).map(str::to_string);
                let mut d = crate::dialogs::name_cell::NameCellDialog::new(from, first, last, current.as_deref());
                d.last_column = (to != from).then_some(to);
                self.name_cell = Some(d);
            }
            Action::NameManager(book) => self.name_manager = Some(crate::dialogs::name_cell::NameManager { book }),
            Action::DeleteName { book, name } => {
                let label = format!("Delete Name {name}");
                if self.doc.apply(&label, |p| p.delete_name(book, &name)) {
                    self.say(format!("The name {name} was deleted"));
                }
            }
            Action::SetCellName { column, row, last, last_column, name } => {
                let label = format!("Define Name {name}");
                let to = last_column.unwrap_or(column);
                match self.doc.edit(&label, |p| p.define_block(&name, column, to, row, last)) {
                    Ok(_) => {
                        self.name_cell = None;
                        self.say(format!("Formulas of this workbook can read {name}"));
                    }
                    Err(e) => {
                        if let Some(d) = &mut self.name_cell {
                            d.problem = Some(e.to_string());
                        } else {
                            self.warn(e.to_string());
                        }
                    }
                }
            }
            Action::PickCellGraph { column, row } => {
                let graphs: Vec<(WindowId, String)> =
                    self.doc.project().graphs().map(|g| (g.id, Window::Graph(g.clone()).title())).collect();
                self.graph_sheet_picker = Some(crate::dialogs::graph_sheet::GraphSheetPicker::new(
                    crate::dialogs::graph_sheet::PickTarget::Cell { column, row },
                    graphs,
                ));
            }
            Action::PickCellImage { column, row } => {
                let picked = rfd::FileDialog::new()
                    .set_title("Insert Images from Files")
                    .add_filter("PNG image", &["png"])
                    .pick_file();
                let Some(path) = picked else { return };
                match std::fs::read(&path) {
                    Ok(bytes) if image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).is_ok() => {
                        let png = Some(std::sync::Arc::new(bytes));
                        self.apply(Action::SetCellImage { column, row, png }, ctx);
                    }
                    Ok(_) => self.warn(format!("{} is not a PNG picture Sheaf can read", path.display())),
                    Err(e) => self.warn(format!("{}: {e}", path.display())),
                }
            }
            Action::SetRichText { book, on } => match self.format_target(book) {
                Ok(target) => {
                    self.doc.apply("Set Data Style", |p| {
                        if let Some(b) = p.workbook_mut(book) {
                            let at = b.active_sheet.min(b.sheets.len().saturating_sub(1));
                            if let Some(ws) = b.sheets.get_mut(at) {
                                let style =
                                    sheaf_core::cellformat::CellFormat { rich_text: Some(on), ..Default::default() };
                                ws.apply_cell_format(target, style);
                            }
                        }
                    });
                }
                Err(e) => self.warn(e),
            },
            Action::SetFloat { book, on } => match self.format_target(book) {
                Ok(target) => {
                    self.doc.apply("Set Data Style", |p| {
                        if let Some(b) = p.workbook_mut(book) {
                            let at = b.active_sheet.min(b.sheets.len().saturating_sub(1));
                            if let Some(ws) = b.sheets.get_mut(at) {
                                let style =
                                    sheaf_core::cellformat::CellFormat { float: Some(on), ..Default::default() };
                                ws.apply_cell_format(target, style);
                            }
                        }
                    });
                }
                Err(e) => self.warn(e),
            },
            Action::SetCellArrow { column, row, double } => {
                let label = if double.is_some() { "Insert Arrow" } else { "Remove Arrow" };
                self.doc.apply(label, |p| {
                    p.set_cell_arrow(column, row, double);
                });
            }
            Action::SetCellImage { column, row, png } => {
                let label = if png.is_some() { "Insert Image" } else { "Remove Image" };
                self.doc.apply(label, |p| {
                    p.set_cell_image(column, row, png);
                });
            }
            Action::PickCellSparkline { column, row } => {
                let sources: Vec<(ColumnId, String)> = self
                    .doc
                    .project()
                    .locate(column)
                    .map(|(_, sheet, _)| {
                        sheet
                            .columns
                            .iter()
                            .filter(|c| c.data.is_numeric())
                            .map(|c| {
                                let name = if c.long_name.is_empty() {
                                    c.short_name.clone()
                                } else {
                                    format!("{} - {}", c.short_name, c.long_name)
                                };
                                (c.id, name)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.cell_sparkline_picker =
                    Some(crate::dialogs::cell_sparkline::CellSparklinePicker::new(column, row, sources));
            }
            Action::SetCellSparkline { column, row, source } => {
                let label = if source.is_some() { "Insert Sparklines" } else { "Remove Sparkline" };
                self.doc.apply(label, |p| {
                    p.set_cell_sparkline(column, row, source);
                });
            }
            Action::SetCellGraph { column, row, graph } => {
                let label = if graph.is_some() { "Insert Graph" } else { "Remove Graph" };
                self.doc.apply(label, |p| {
                    p.set_cell_graph(column, row, graph);
                });
            }
            Action::AddGraphSheet { book, graph } => {
                let name = self.doc.project().graph(graph).map(|g| g.name.clone()).unwrap_or_default();
                self.doc.apply("Add Graph as Sheet", |p| {
                    if let Some(b) = p.workbook_mut(book) {
                        b.add_graph_sheet(graph, &name);
                    }
                });
                if let Some(state) = self.books.get_mut(&book) {
                    state.clear_selection();
                }
            }
            Action::SetNotesSheetText { book, sheet, text } => {
                self.doc.apply("Edit notes", |p| {
                    if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet))
                        && s.notes_text.is_some()
                    {
                        s.notes_text = Some(text);
                    }
                });
            }
            Action::SetNotesText { id, text } => {
                self.doc.apply("Edit notes", |p| {
                    if let Some(n) = p.notes_mut(id) {
                        n.text = text;
                    }
                });
            }
            Action::NewWorkbook => {
                self.doc.apply("New workbook", |p| {
                    let book = p.add_workbook("");
                    let _ = p.add_column(book, 0, Designation::X, ColumnData::default());
                    let _ = p.add_column(book, 0, Designation::Y, ColumnData::default());
                });
            }
            Action::AddColumn(book) => self.add_columns(book, 1),
            Action::RenameParamRow { book, name } => {
                let sheet = self.doc.project().workbook(book).map_or(0, |b| b.active_sheet);
                self.param_row = Some((book, sheet, name.clone(), Some(name)));
            }
            Action::DeleteParamRow { book, name } => {
                let sheet = self.doc.project().workbook(book).map_or(0, |b| b.active_sheet);
                self.doc.apply("Delete parameter row", |p| {
                    p.workbook_mut(book)
                        .and_then(|b| b.sheets.get_mut(sheet))
                        .is_some_and(|ws| ws.remove_parameter_row(&name))
                });
            }
            Action::MoveSheet { book, from, to } => {
                // T306: the dragged sheet goes where the tab was dropped and stays the active one.
                let Some(n) = self.doc.project().workbook(book).map(|b| b.sheets.len()) else { return };
                if from >= n || to >= n || from == to {
                    return;
                }
                let mut order: Vec<usize> = (0..n).filter(|&i| i != from).collect();
                order.insert(to, from);
                self.doc.apply("Move worksheet", |p| {
                    p.workbook_mut(book).is_some_and(|w| {
                        let moved = w.reorder_sheets(&order);
                        w.active_sheet = to;
                        moved
                    })
                });
                if let Some(state) = self.books.get_mut(&book) {
                    state.clear_selection();
                }
            }
            Action::AddColumnsDialog(book) => self.add_columns = Some((book, 1)),
            Action::AddColumnAcross(book) => self.column_across = Some(ColumnAcross::new(book)),
            Action::RepeatAnalysis { book, columns } => self.repeat_analysis(book, columns),
            Action::RepeatAnalysisAllSheets(book) => self.repeat_analysis_all_sheets(book),
            Action::DeleteColumns(columns) => {
                self.doc.apply("Delete columns", |p| delete_columns(p, &columns));
            }
            Action::MoveColumns { columns, to } => {
                let ids = columns;
                let mut moved = false;
                self.doc.apply("Move columns", |p| {
                    let Some(first) = ids.first().copied() else { return };
                    if let Some((book, sheet_index)) = p
                        .locate(first)
                        .and_then(|(b, s, _)| Some((b.id, b.sheets.iter().position(|x| std::ptr::eq(x, s))?)))
                        && let Some(ws) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet_index))
                    {
                        moved = ws.move_columns(&ids, to);
                    }
                });
                if !moved {
                    self.say("The columns are already there");
                }
            }
            Action::ResetFullRange(book) => {
                let ids: Vec<ColumnId> = self
                    .doc
                    .project()
                    .workbook(book)
                    .and_then(|b| b.sheets.get(b.active_sheet))
                    .map(|s| s.columns.iter().map(|c| c.id).collect())
                    .unwrap_or_default();
                let had = self
                    .doc
                    .project()
                    .workbook(book)
                    .and_then(|b| b.sheets.get(b.active_sheet))
                    .is_some_and(|s| s.columns.iter().any(|c| c.data_range.is_some()));
                if had {
                    let mut n = 0;
                    self.doc.apply("Reset to full range", |p| n = p.reset_data_range(&ids));
                    self.say(format!("{n} column(s) use all their rows again"));
                } else {
                    self.say("Every column already uses all its rows");
                }
            }
            Action::ExportWorksheetImage(book) => self.export_worksheet_image(book),
            Action::Reimport { book, choose } => self.reimport(book, choose),
            Action::AutoRefresh { book } => {
                self.auto_refresh =
                    self.doc.project().workbook(book).and_then(crate::dialogs::auto_refresh::AutoRefreshDialog::new);
            }
            Action::SaveAxisTheme { .. } | Action::SaveLegendTheme { .. } => {
                let Some(dir) = self.startup.graph_themes_dir.clone() else {
                    self.warn("Sheaf has no settings folder for graph themes");
                    return;
                };
                let clip = match action {
                    Action::SaveAxisTheme { graph, layer, y } => self.doc.project().graph(graph).and_then(|g| {
                        sheaf_core::format_clip::copy_axis(g, layer, y, sheaf_core::format_clip::AxisPart::All)
                    }),
                    Action::SaveLegendTheme { graph, layer } => self
                        .doc
                        .project()
                        .graph(graph)
                        .and_then(|g| sheaf_core::format_clip::copy_legend_styles(g, layer)),
                    _ => None,
                };
                if let Some(clip) = clip {
                    let existing = sheaf_io::graph_themes::list(&dir).into_iter().map(|e| e.name).collect();
                    let name = sheaf_io::graph_themes::next_name(&dir);
                    self.save_theme = Some((crate::dialogs::graph_themes::SaveThemeDialog::new(name, existing), clip));
                }
            }
            Action::NewFromTemplate(name) => self.new_from_template(name),
            Action::NewBookFromTemplate(name) => {
                let Some(dir) = self.startup.templates_dir.clone() else { return };
                match sheaf_io::user_templates::load_book(&dir, &name) {
                    Ok(t) => self.new_book_from_template(&t),
                    Err(e) => self.message = Some(("Could not read the template".to_string(), e.to_string())),
                }
            }
            Action::FitOrganizer => self.fit_organizer = Some(Default::default()),
            Action::FilterManager => {
                self.filter_manager =
                    Some(crate::dialogs::filter_manager::FilterManager::new(self.startup.themes_dir.clone()));
            }
            Action::SystemTemplates { book, columns } => {
                self.system_templates =
                    Some(crate::dialogs::system_templates::SystemTemplatesDialog::new(book, columns));
            }
            Action::ThemeOrganizer => {
                let entries =
                    self.startup.graph_themes_dir.as_deref().map(sheaf_io::graph_themes::list).unwrap_or_default();
                let graph = self.active.is_some_and(|a| self.doc.project().graph(a).is_some());
                self.theme_organizer =
                    Some(crate::dialogs::graph_themes::ThemeOrganizer { entries, selected: None, graph });
            }
            Action::ViewWindows(mode) => {
                if self.doc.project().view_windows != mode {
                    self.doc.apply("View windows", |p| p.view_windows = mode);
                }
                self.follow_folder();
            }
            Action::GoToFolder(path) => {
                if explorer::folder_at(&self.doc.project().root, &path).is_some() {
                    self.panels.explorer.folder = path;
                    self.panels.explorer.clear_selection();
                    self.follow_folder();
                }
            }
            Action::FolderProperties(path) => {
                if let Some(f) = explorer::folder_at(&self.doc.project().root, &path) {
                    self.folder_props = Some(crate::dialogs::folder_props::FolderPropsDialog::new(path, f));
                }
            }
            Action::BrowserGraph { graph, layer } => {
                match crate::dialogs::browser_graph::BrowserGraphDialog::new(self.doc.project(), graph, layer) {
                    Some(d) => self.browser_graph = Some(d),
                    None => self.warn("The layer has no plot to browse"),
                }
            }
            Action::BrowseStep { graph, next } => {
                let g = self.doc.project().graph(graph);
                let step = |current: usize, n: usize| {
                    if next { (current + 1 < n).then_some(current + 1) } else { current.checked_sub(1) }
                };
                if let Some(b) = g.and_then(|g| g.browser.as_ref()) {
                    if let Some(to) = step(b.current, b.items.len()) {
                        self.apply(Action::BrowseGraph { graph, to }, ctx);
                    }
                } else if let Some(e) = g.and_then(|g| g.explorer) {
                    if let Some(to) = step(e.current, e.intervals) {
                        self.apply(Action::ExploreInterval { graph, to }, ctx);
                    }
                } else {
                    self.warn("The graph has no browser or explorer to step: see Graph > Browser Graph...");
                }
            }
            Action::BrowseGraph { graph, to } => {
                // Steps one after another make one undo step.
                let token = 0x7410_0000_0000 + graph.0;
                if let Err(e) = self.doc.apply_merging("Browse Graph", token, |p| p.browse_graph(graph, to)) {
                    self.warn(e.to_string());
                }
            }
            Action::ExploreInterval { graph, to } => {
                // Steps one after another (Play included) make one undo step.
                let token = 0x7416_0000_0000 + graph.0;
                if let Err(e) = self.doc.apply_merging("Explore Interval", token, |p| p.explore_interval(graph, to)) {
                    self.warn(e.to_string());
                }
            }
            Action::ExplorerLength { graph, length } => {
                let token = 0x7453_0000_0000 + graph.0;
                if let Err(e) =
                    self.doc.apply_merging("Explorer Intervals", token, |p| p.set_explorer_length(graph, length))
                {
                    self.warn(e.to_string());
                }
            }
            Action::ExplorerIntervals { graph, intervals } => {
                let token = 0x7416_1000_0000 + graph.0;
                if let Err(e) =
                    self.doc.apply_merging("Explorer Intervals", token, |p| p.set_explorer_intervals(graph, intervals))
                {
                    self.warn(e.to_string());
                }
            }
            Action::GraphSlicer { graph } => {
                match crate::dialogs::graph_slicer::GraphSlicerDialog::new(self.doc.project(), graph) {
                    Some(d) => self.graph_slicer = Some(d),
                    None => self.warn("The graph plots no worksheet column to slice"),
                }
            }
            Action::AddFunctionGraph { graph, layer } => {
                // Origin's Auto From/To: the range the layer's X axis shows (T322).
                let range = sheaf_plot::layout_geometry(self.doc.project(), graph)
                    .ok()
                    .and_then(|geoms| geoms.get(layer).map(|g| (g.x.min, g.x.max)));
                self.function_graph =
                    Some(crate::dialogs::function_graph::FunctionGraphDialog::over(graph, layer, range));
                self.function_graph_new = false;
            }
            Action::Profile { book, graph } => {
                use crate::dialogs::matrix_ops::{Form, MatrixOpDialog};
                // The matrix: the given book, or the one that the graph's contour layer shows.
                let target = book.map(|b| (b, None)).or_else(|| {
                    let g = self.doc.project().graph(graph?)?;
                    g.layers.iter().find_map(|l| match l.field.as_ref()?.source {
                        sheaf_core::graph::FieldSource::Matrix { window, sheet } => Some((window, Some(sheet))),
                        _ => None,
                    })
                });
                match target {
                    Some((b, sheet)) => {
                        self.matrix_dialog =
                            MatrixOpDialog::new(&self.doc, b, Form::Profile { horizontal: true, index: 1 });
                        if let (Some(d), Some(s)) = (self.matrix_dialog.as_mut(), sheet) {
                            d.sheet = s;
                        }
                    }
                    None => self.warn("This graph shows no matrix; a profile needs a matrix contour or image"),
                }
            }
            Action::TernaryContour { a, b, c, z } => {
                match self.doc.edit("New graph", |p| p.plot_ternary_contour(a, b, c, z)) {
                    Ok(graph) => self.focus(graph),
                    Err(e) => self.warn(e.to_string()),
                }
            }
            Action::PolarContour { theta, r, z } => {
                match self.doc.edit("New graph", |p| p.plot_xyz_polar(theta, r, z)) {
                    Ok(graph) => self.focus(graph),
                    Err(e) => self.warn(e.to_string()),
                }
            }
            Action::NewFunctionPlot => {
                // The graph is made on OK, in the same undo step as its function (T213).
                self.function_graph = Some(crate::dialogs::function_graph::FunctionGraphDialog::new(WindowId(0), 0));
                self.function_graph_new = true;
            }
            Action::BatchPlot { graph, columns } => {
                match crate::dialogs::batch_plot::BatchPlotDialog::new(self.doc.project(), graph, columns) {
                    Some(d) => self.batch_plot = Some(d),
                    None => self.warn("The graph plots no worksheet column"),
                }
            }
            Action::FindInProjectFiles => {
                let folder = self.path.as_deref().and_then(|p| p.parent()).map(std::path::Path::to_path_buf);
                self.find_files = Some(crate::dialogs::find_files::FindFilesDialog::new(folder.as_deref()));
            }
            Action::SetAnalysisMode { index, mode } => {
                let label = match mode {
                    sheaf_core::column::RecalcMode::None => "Stop Following",
                    _ => "Recalculate Mode",
                };
                if let Err(e) = self.doc.edit(label, |p| p.set_analysis_mode(index, mode)) {
                    self.warn(e.to_string());
                }
            }
            Action::OperationParams(index) => {
                let link = self.doc.project().analysis_links.get(index);
                self.op_params = link.and_then(|l| {
                    crate::dialogs::op_params::OpParamsDialog::new(
                        index,
                        sheaf_core::operations::tool_name(&l.tool),
                        &l.tool,
                    )
                });
            }
            Action::GoToColumn(column) => {
                if let Some(t) = crate::dialogs::project_browser::target(
                    self.doc.project(),
                    sheaf_core::project_browser::ItemRef::Column(column),
                ) {
                    self.go_to_found(t, ctx);
                }
            }
            Action::ExportReport(notes) => {
                let name = self.doc.project().notes_window(notes).map_or_else(|| "report".into(), |n| n.name.clone());
                let mut chooser = rfd::FileDialog::new()
                    .set_title("Export Report")
                    .add_filter("Web page (*.html)", &["html", "htm"])
                    .add_filter("Markdown (*.md)", &["md", "markdown"])
                    .set_file_name(format!("{name}.html"));
                if let Some(dir) = self.folder() {
                    chooser = chooser.set_directory(dir);
                }
                if let Some(path) = chooser.save_file() {
                    self.apply(Action::ExportReportTo { notes, path }, ctx);
                }
            }
            Action::ExportReportTo { notes, path } => {
                match crate::notes_view::export_report(self.doc.project(), notes, &path) {
                    Ok(n) => self.say(format!("Exported the report to {} with {n} figure(s)", path.display())),
                    Err(e) => self.warn(format!("Could not export the report: {e}")),
                }
            }
            Action::ProjectBrowser => {
                self.project_browser.get_or_insert_with(Default::default);
            }
            Action::FindInProject => {
                self.find_project = Some(crate::dialogs::find_project::FindProjectDialog::new(self.find_spec.clone()));
            }
            Action::CopyColumnsTo(columns) => {
                self.copy_columns =
                    Some(crate::dialogs::copy_columns::CopyColumnsDialog::new(columns, self.doc.project()));
            }
            Action::SwapColumns(a, b) => {
                let swapped = self.doc.edit("Swap columns", |p| {
                    let (book, sheet_index) = p
                        .locate(a)
                        .and_then(|(b, s, _)| Some((b.id, b.sheets.iter().position(|x| std::ptr::eq(x, s))?)))
                        .ok_or("the column is gone")?;
                    let ws =
                        p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet_index)).ok_or("the sheet is gone")?;
                    if ws.swap_columns(a, b) { Ok(()) } else { Err("select two columns of the same sheet") }
                });
                match swapped {
                    Ok(()) => self.say("Swapped the two columns"),
                    Err(e) => self.warn(format!("Could not swap: {e}")),
                }
            }
            Action::HideColumns(columns) => {
                self.doc.apply("Hide columns", |p| {
                    for id in &columns {
                        if let Some(c) = p.column_mut(*id) {
                            c.hidden = true;
                        }
                    }
                });
                // The hidden columns are no longer on screen, so nothing of them stays selected.
                if let Some(book) = columns.first().and_then(|c| self.doc.project().locate(*c)).map(|(b, ..)| b.id)
                    && let Some(state) = self.books.get_mut(&book)
                {
                    state.clear_selection();
                }
            }
            Action::UnhideColumns(book) => {
                self.doc.apply("Unhide columns", |p| {
                    if let Some(b) = p.workbook_mut(book) {
                        for c in &mut b.sheet_mut().columns {
                            c.hidden = false;
                        }
                    }
                });
            }
            Action::SetCategorical(columns) => {
                let all = columns.iter().all(|id| self.doc.project().column(*id).is_some_and(|c| c.categorical));
                self.doc.apply("Set as categorical", |p| {
                    for id in &columns {
                        if let Some(c) = p.column_mut(*id) {
                            c.categorical = !all;
                        }
                    }
                });
                self.say(if all { "Categorical mark removed" } else { "Set as categorical" });
            }
            Action::AddSparklines(columns) => {
                self.doc.apply("Add or Update Sparklines", |p| {
                    for id in &columns {
                        if let Some(c) = p.column_mut(*id) {
                            c.sparkline = true;
                        }
                    }
                    let place = columns.first().and_then(|id| {
                        let (book, sheet, _) = p.locate(*id)?;
                        Some((book.id, book.sheets.iter().position(|s| std::ptr::eq(s, sheet))?))
                    });
                    if let Some((book, sheet)) = place
                        && let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet))
                    {
                        s.view.sparklines = true;
                    }
                });
                self.say("Added sparklines");
            }
            Action::SetDesignation { columns, designation } => {
                self.doc.apply("Set column designation", |p| {
                    for id in &columns {
                        if let Some(c) = p.column_mut(*id) {
                            c.designation = designation;
                        }
                    }
                });
            }
            Action::Recalculate(columns) => {
                let columns: Vec<ColumnId> = columns
                    .into_iter()
                    .filter(|c| self.doc.project().column(*c).is_some_and(|c| c.is_locked() || c.analysis.is_some()))
                    .collect();
                let result = self.doc.edit("Recalculate", |p| {
                    let mut done: Vec<usize> = Vec::new();
                    for c in &columns {
                        // The result columns of an analysis run again as a whole, once.
                        if let Some(link) = p.analysis_link_of(*c) {
                            if !done.contains(&link) {
                                p.recalculate_analysis(link).map_err(|e| e.to_string())?;
                                done.push(link);
                            }
                        } else {
                            p.recalculate(*c).map_err(|e| e.to_string())?;
                        }
                    }
                    Ok::<(), String>(())
                });
                match result {
                    Ok(()) if columns.is_empty() => self.warn("No selected column recalculates"),
                    Ok(()) => self.say("Recalculated"),
                    Err(e) => self.warn(format!("Could not recalculate: {e}")),
                }
            }
            Action::SetCell { column, .. } if self.doc.project().column(column).is_some_and(|c| c.is_locked()) => {
                self.warn("That column is calculated from a formula; change the formula in Set Column Values");
            }
            // A text starting with `=` makes a cell formula (T448).
            Action::SetCell { column, row, text } if text.trim_start().starts_with('=') && text.trim().len() > 1 => {
                let formula = text.trim_start()[1..].to_string();
                let done = self.doc.edit("Cell Formula", |p| {
                    p.drop_cell_link(column, row);
                    p.set_cell_formula(column, row, &formula)
                });
                if let Err(e) = done {
                    self.warn(format!("That cell formula cannot be read: {e}"));
                }
            }
            Action::SetCell { column, row, text } => {
                let beyond_end = self.doc.project().column(column).is_none_or(|c| row >= c.len());
                if !(beyond_end && text.trim().is_empty()) {
                    let mut understood = true;
                    self.doc.apply("Edit cell", |p| {
                        // Typing into a linked cell replaces its value and ends the link (T211), and the formula of a
                        // formula cell (T448).
                        p.drop_cell_link(column, row);
                        p.drop_cell_formula(column, row);
                        if let Some(c) = p.column_mut(column) {
                            understood = c.set_cell_text(row, &text);
                        }
                    });
                    if !understood {
                        self.warn(format!("'{text}' is not a number; the cell was left empty"));
                    }
                }
            }
            Action::SelectSheet { book, sheet } => {
                let changed = self.doc.project().workbook(book).is_some_and(|b| b.active_sheet != sheet);
                if changed && let Some(state) = self.books.get_mut(&book) {
                    state.clear_selection();
                }
                self.doc.adjust_view(|p| {
                    if let Some(b) = p.workbook_mut(book) {
                        b.active_sheet = sheet;
                    }
                });
            }
            Action::Cells { book, op } => self.cell_op(book, op, ctx),
            Action::SetLabel { column, row, text } => {
                self.doc.apply("Edit label", |p| {
                    if let Some(c) = p.column_mut(column) {
                        match row {
                            LabelRow::LongName => c.long_name = text,
                            LabelRow::Units => c.units = text,
                            LabelRow::Comments => c.comments = text,
                            LabelRow::Formula
                            | LabelRow::SamplingInterval
                            | LabelRow::Sparklines
                            | LabelRow::Categories => {}
                            LabelRow::Param(name) => {
                                c.params.insert(name, text);
                            }
                        }
                    }
                });
            }
            Action::Sheet { book, op } => self.sheet_op(book, op),
            Action::ColumnProperties(column) => {
                self.column_props = ColumnPropsDialog::new(self.doc.project(), column);
            }
            Action::FlashFill(column) => match self.doc.project().flash_fill(column) {
                Ok(fill) => {
                    self.flash_fill =
                        Some(crate::dialogs::flash_fill::FlashFillDialog::new(self.doc.project(), column, fill));
                }
                Err(e) => self.warn(e.to_string()),
            },
            Action::SetColumnValues(column) => {
                self.set_values = SetValuesDialog::new(self.doc.project(), column);
            }
            Action::SetMultipleColumnsValues(columns) => {
                self.set_values = SetValuesDialog::new_multiple(self.doc.project(), &columns);
                if self.set_values.is_none() {
                    self.say("Select two or more columns of one sheet first");
                }
            }
            Action::PlotColumns { book, columns, template } => self.plot_columns(book, columns, template),
            Action::GroupedHistogram { book, columns } => {
                match crate::dialogs::grouped_hist::GroupedHistDialog::new(self.doc.project(), book, &columns) {
                    Some(d) => self.grouped_hist = Some(d),
                    None => self.warn("A grouped histogram needs a numeric column and a group column".to_string()),
                }
            }
            Action::RunChart { book, columns } => {
                let project = self.doc.project();
                let numeric = |id: &ColumnId| project.column(*id).is_some_and(|c| c.data.is_numeric());
                let column = columns.iter().copied().find(numeric).or_else(|| {
                    let b = project.workbook(book)?;
                    b.sheets
                        .get(b.active_sheet)?
                        .columns
                        .iter()
                        .find(|c| c.designation == sheaf_core::Designation::Y && c.data.is_numeric())
                        .map(|c| c.id)
                });
                match column.and_then(|c| crate::dialogs::run_chart::RunChartDialog::new(project, c)) {
                    Some(d) => self.run_chart = Some(d),
                    None => self.warn("Select a numeric column for the run chart".to_string()),
                }
            }
            Action::PlotLineSeries { book, columns } => self.plot_line_series(book, columns),
            Action::PlotYError { book, columns, x } => self.plot_y_error(book, columns, x),
            Action::PlotPanels { book, columns, rows, cols } => self.plot_panels(book, columns, rows, cols),
            Action::StackPanels { book, columns } => {
                let columns = self.plot_y_columns(book, columns, Template::Line);
                let rows = self.selected_sort_rows(book).map(|r| (r.start, r.end));
                self.stack_panels = StackPanelsDialog::new(self.doc.project(), book, columns, rows);
                if self.stack_panels.is_none() {
                    self.warn("Select at least one numeric Y column to stack.");
                }
            }
            Action::MergeGraphs => {
                let project = self.doc.project();
                if project.graphs().next().is_none() {
                    self.warn("Choose a source graph to merge");
                } else {
                    self.merge_graphs =
                        Some(MergeGraphsDialog::new(project, &self.panels.explorer.folder, self.active));
                }
            }
            Action::LabelPanels { book, columns } => {
                let columns = self.plot_y_columns(book, columns, Template::Line);
                let rows = self.selected_sort_rows(book).map(|r| (r.start, r.end));
                self.label_panels = LabelPanelsDialog::new(self.doc.project(), book, columns, rows);
                if self.label_panels.is_none() {
                    self.warn("Select at least one numeric Y column to plot by label.");
                }
            }
            Action::MultiYAxes { book, columns } => {
                let columns = self.plot_y_columns(book, columns, Template::DoubleY);
                if columns.is_empty() {
                    self.warn("Nothing to plot: select one or more numeric Y columns");
                } else {
                    self.multi_y = Some(MultiYDialog::new(columns));
                }
            }
            Action::ComparisonPlot { book, sheet } => {
                let sheet = sheet.or_else(|| self.doc.project().workbook(book).map(|b| b.active_sheet));
                match sheet {
                    Some(sheet) if self.doc.project().can_comparison_plot(book, sheet) => {
                        self.comparison_plot = Some(ComparisonPlotDialog::new(book, sheet));
                    }
                    _ => self.warn(
                        "Open the report sheet of a one-way ANOVA with a means comparison or of a two-sample t-test first.",
                    ),
                }
            }
            Action::Analyze { book, columns, kind } => self.open_analysis(book, columns, kind),
            Action::InterpolateFromColumn { book, columns } => {
                self.open_analysis(book, columns, AnalysisKind::Interpolate);
                if let Some(d) = &mut self.analysis {
                    d.set_interp_from_column();
                }
            }
            Action::InterpolateWith { book, columns, method } => {
                self.open_analysis(book, columns, AnalysisKind::Interpolate);
                if let Some(d) = &mut self.analysis {
                    d.set_interp_method(method);
                }
            }
            Action::Fit { book, column, kind } => {
                let column = column.filter(|c| self.doc.project().column(*c).is_some_and(|c| c.data.is_numeric()));
                self.fit = FitDialog::new(self.doc.project(), book, column, kind);
                if self.fit.is_none() {
                    self.warn("Nothing to fit: choose a column that holds numbers and is set as Y".to_string());
                }
            }
            Action::FitMore { book, column, kind } => {
                let column = column.filter(|c| self.doc.project().column(*c).is_some_and(|c| c.data.is_numeric()));
                self.fit_more = FitMoreDialog::new(self.doc.project(), book, column, kind);
                if self.fit_more.is_none() {
                    self.warn("Nothing to fit: choose a column that holds numbers and is set as Y".to_string());
                }
            }
            Action::SimulateCurve { book } => self.simulate = Some(SimulateDialog::new(self.doc.project(), book)),
            Action::SimulateSurface { book } => {
                self.simulate_surface = Some(SimulateSurfaceDialog::new(self.doc.project(), book));
            }
            Action::SortSheet { column, descending } => self.sort_sheet(column, descending),
            Action::SortRange { columns, descending } => self.sort_selected(columns, descending, true),
            Action::SortColumns { columns, descending } => self.sort_selected(columns, descending, false),
            Action::SortCustom { book, columns } => {
                let rows = if columns.is_empty() { None } else { self.selected_sort_rows(book) };
                self.sort = SortDialog::nested_in_rows(self.doc.project(), book, columns, rows);
            }
            Action::PivotTable { book, columns } => self.pivot = PivotDialog::new(&self.doc, book, columns),
            Action::ExtractRegion { graph, layer, shape } => self.extract_region(graph, layer, vec![shape]),
            Action::MaskRegion { graph, layer, shape, mask } => self.mask_region(graph, layer, vec![shape], mask),
            Action::ExtractAllRegions { graph, layer } => {
                let shapes = sheaf_plot::region::region_shapes(self.doc.project(), graph, layer);
                self.extract_region(graph, layer, shapes)
            }
            Action::MaskAllRegions { graph, layer, mask } => {
                let shapes = sheaf_plot::region::region_shapes(self.doc.project(), graph, layer);
                self.mask_region(graph, layer, shapes, mask)
            }
            Action::MultiPeakFit { graph, y, layer } => {
                let graph = match graph {
                    Some(g) => Some(g),
                    // From a worksheet: a line graph of the column first, as Origin makes one.
                    None => self
                        .doc
                        .edit("Plot", |p| p.plot_columns(sheaf_core::template::Template::Line, &[y]))
                        .map_err(|e| self.warn(e.to_string()))
                        .ok(),
                };
                if let Some(graph) = graph {
                    self.multi_peak = MultiPeakDialog::new(self.doc.project(), graph, y, layer);
                    if self.multi_peak.is_none() {
                        self.warn("The graph has no plot of that column");
                    }
                }
            }
            Action::IntegrateRangesPick { graph, y, layer } => {
                self.range_pick = RangePickDialog::new(self.doc.project(), graph, y, layer);
                if self.range_pick.is_none() {
                    self.warn("The graph has no plot of that column");
                } else {
                    self.graphs.entry(graph).or_default().reader = true;
                    self.focus(graph);
                }
            }
            Action::CvChargePick { graph, y, layer } => {
                let rate = sheaf_core::analysis::CvChargeSpec::default().scan_rate;
                self.cv_pick = CvPickDialog::new(self.doc.project(), graph, y, layer, rate);
                if self.cv_pick.is_none() {
                    self.warn("The graph has no plot of that column");
                } else {
                    self.graphs.entry(graph).or_default().reader = true;
                    self.focus(graph);
                }
            }
            Action::SubtractLinePick { graph, y, layer } => {
                self.subtract_pick = SubtractPickDialog::new(self.doc.project(), graph, y, layer);
                if self.subtract_pick.is_none() {
                    self.warn("The graph has no plot of that column");
                } else {
                    self.graphs.entry(graph).or_default().reader = true;
                    self.focus(graph);
                }
            }
            Action::Reshape { book, columns, kind } => {
                self.reshape = ReshapeDialog::new(&self.doc, book, kind, columns);
            }
            Action::SortByLabel(book) => self.sort = Some(SortDialog::by_label(book)),
            Action::ClearWorksheet(book) => {
                if let Some(sheet) = self.doc.project().workbook(book).map(|b| b.active_sheet) {
                    let mut n = 0;
                    self.doc.apply("Clear worksheet", |p| n = p.clear_sheet(book, sheet));
                    self.say(format!("Cleared {n} column(s); Edit > Undo brings the values back"));
                }
            }
            Action::ReverseColumns(columns) => {
                let mut locked = 0;
                self.doc.apply("Reverse order", |p| locked = p.reverse_columns(&columns));
                if locked > 0 {
                    self.warn(format!("{locked} column(s) calculated from a formula were left alone"));
                }
            }
            Action::AddErrorBars { graph, layer } => self.add_error_bars(graph, layer),
            Action::Trendline { graph, layer, plot, edit } => {
                self.trendline = TrendlineDialog::new(&self.doc, graph, layer, plot, edit);
            }
            Action::Refresh(active) => self.refresh(active),
            Action::RowFilterDialog { book, column } => {
                self.row_filter = RowFilterDialog::new(&self.doc, book, column);
            }
            Action::MaskColor => self.doc.apply("Change mask color", |p| p.cycle_mask_color()),
            Action::ToggleMasking => {
                self.doc.apply("Disable or enable masking", |p| {
                    let now = p.masking_disabled();
                    p.set_masking_disabled(!now);
                });
            }
            Action::ToggleHideMasked => {
                self.doc.apply("Hide or show masked points", |p| p.hide_masked = !p.hide_masked);
            }
            Action::MaskByConditionDialog { book, columns } => {
                self.mask_condition = MaskConditionDialog::new(&self.doc, book, columns);
            }
            Action::ToggleRowFilter(book) => {
                let sheet = self.doc.project().workbook(book).map(|b| b.active_sheet);
                let has = sheet.is_some_and(|s| {
                    self.doc.project().workbook(book).and_then(|b| b.sheets.get(s)).is_some_and(|s| s.filter.is_some())
                });
                if has {
                    self.doc.apply("Enable or disable filter", |p| {
                        if let Some(f) = p.workbook_mut(book).and_then(|b| b.sheet_mut().filter.as_mut()) {
                            f.enabled = !f.enabled;
                        }
                    });
                } else {
                    self.warn("This sheet has no filter: use Add or Remove Filter first");
                }
            }
            Action::CondFormatDialog { book, kind } => {
                let range = self.cond_range(book);
                match range {
                    Ok(range) => self.cond_format = CondFormatDialog::new(&self.doc, book, kind, range),
                    Err(why) => self.warn(why),
                }
            }
            Action::WorksheetFontsDialog(book) => {
                self.worksheet_fonts = WorksheetFontsDialog::new(&self.doc, book, self.books.get(&book));
            }
            Action::UpdateWorksheetViewport { book, sheet, minimum_rows, fitted } => {
                // The grid retries after Properties closes, keeping its draft on the same revision.
                if self.worksheet_fonts.is_some() {
                    return;
                }
                let Some(worksheet) = self
                    .doc
                    .project()
                    .workbook(book)
                    .filter(|workbook| workbook.active_sheet == sheet)
                    .and_then(|workbook| workbook.sheets.get(sheet))
                else {
                    return;
                };
                let grow = worksheet.view.automatic_rows() && minimum_rows > grid::shown_rows(worksheet);
                let fitted = fitted.filter(|_| worksheet.view.fit_sheet_to_window);
                if grow || fitted.is_some() {
                    self.doc.adjust_view(|project| {
                        let worksheet = &mut project.workbook_mut(book).unwrap().sheets[sheet];
                        if grow {
                            worksheet.view.allocated_rows = Some(minimum_rows);
                            worksheet.view.auto_add_rows = Some(true);
                        }
                        if let Some(cells) = fitted {
                            worksheet.view.fitted_cells = cells;
                            if cells.column_width().is_some() {
                                for column in worksheet.columns.iter_mut().filter(|column| !column.hidden) {
                                    column.width = None;
                                }
                            }
                        }
                    });
                    ctx.request_repaint();
                }
            }
            Action::FormatCellsDialog(book) => match self.format_target(book) {
                Ok(target) => self.cell_format = CellFormatDialog::new(&self.doc, book, target),
                Err(why) => self.warn(why),
            },
            Action::NoteDialog(book) => match self.note_cell(book) {
                Ok((column, row)) => self.cell_note = CellNoteDialog::new(&self.doc, book, column, row),
                Err(why) => self.warn(why),
            },
            Action::DeleteNote(book) => match self.note_cell(book) {
                Ok((column, row)) => {
                    let has = self.doc.project().workbook(book).is_some_and(|b| b.sheet().note(column, row).is_some());
                    if has {
                        self.doc.apply("Delete note", |p| {
                            if let Some(b) = p.workbook_mut(book) {
                                b.sheet_mut().remove_note(column, row);
                            }
                        });
                    } else {
                        self.warn("The selected cell has no note");
                    }
                }
                Err(why) => self.warn(why),
            },
            Action::CondFormatManager(book) => self.cond_manager = CondManagerDialog::new(&self.doc, book),
            Action::RemoveCondFormat(book) => match self.cond_range(book) {
                Ok(range) => {
                    let has = self
                        .doc
                        .project()
                        .workbook(book)
                        .is_some_and(|b| b.sheet().cond_formats.iter().any(|r| r.range.intersects(&range)));
                    if has {
                        self.doc.apply("Remove conditional format", |p| {
                            if let Some(b) = p.workbook_mut(book) {
                                b.sheet_mut().cond_formats.retain(|r| !r.range.intersects(&range));
                            }
                        });
                    } else {
                        self.warn("No conditional format touches the selection");
                    }
                }
                Err(why) => self.warn(why),
            },
            Action::FreezePanes(book) => {
                let Some(sheet) = self.doc.project().workbook(book).map(|b| b.sheet()) else { return };
                let (next, label) = if sheet.frozen.is_none() {
                    match self.books.get(&book).map(|s| grid::freeze_counts(s, sheet)) {
                        Some(Ok(frozen)) => (frozen, "Freeze panes"),
                        Some(Err(why)) => {
                            self.warn(why);
                            return;
                        }
                        None => return,
                    }
                } else {
                    (sheaf_core::Frozen::default(), "Unfreeze panes")
                };
                self.doc.apply(label, |p| {
                    if let Some(b) = p.workbook_mut(book) {
                        b.sheet_mut().frozen = next;
                    }
                });
            }
            Action::ReapplyRowFilter(book) => {
                let sheet = self.doc.project().workbook(book).map(|b| b.active_sheet).unwrap_or(0);
                match self.doc.edit("Reapply filter", |p| p.reapply_row_filter(book, sheet)) {
                    Ok(()) => self.say("Filter applied again"),
                    Err(e) => self.warn(format!("Could not apply the filter: {e}")),
                }
            }
            Action::FilterRows(book) => {
                self.filter = FilterDialog::new(self.doc.project(), book);
            }
            Action::PlotSetup { graph, layer } => {
                let token = self.token();
                self.plot_setup =
                    Some(crate::dialogs::plot_setup::PlotSetupDialog::new(graph, layer, token, self.doc.project()));
                self.focus(graph);
            }
            Action::AddPlotToLayer { graph, layer, kind } => {
                let token = self.token();
                self.layer_contents =
                    Some(crate::dialogs::layer_contents::LayerContentsDialog::new(graph, layer, token).with_kind(kind));
                self.focus(graph);
                self.graphs.entry(graph).or_default().active_layer = layer;
            }
            Action::LayerContents { graph, layer } => {
                let token = self.token();
                self.layer_contents =
                    Some(crate::dialogs::layer_contents::LayerContentsDialog::new(graph, layer, token));
                self.focus(graph);
                self.graphs.entry(graph).or_default().active_layer = layer;
            }
            Action::FitPageToPrinter { graph } => self.fit_page_to_printer(graph),
            Action::FitPageToLayers { graph } => {
                self.fit_page = Some(crate::dialogs::fit_page::FitPageDialog::new(graph));
                self.focus(graph);
            }
            Action::LayerManagement { graph, layer } => {
                self.layer_management =
                    crate::dialogs::layer_management::LayerManagementDialog::new(self.doc.project(), graph, layer);
                if self.layer_management.is_some() {
                    self.focus(graph);
                } else {
                    self.warn("Layer Management needs a graph with layers");
                }
            }
            Action::Axis3d { graph, layer, dim } => self.open_axis3d(graph, layer, dim),
            // The X and Y Axis windows of a 3D layer are the 3D Axis window (T164): the layer's own
            // axes are the hidden frame of the view.
            Action::GraphProperties { graph, layer, tab: tab @ (PropsTab::XAxis | PropsTab::YAxis), plot: None }
                if self
                    .doc
                    .project()
                    .graph(graph)
                    .and_then(|g| g.layers.get(layer))
                    .is_some_and(|l| l.coords.space3d().is_some()) =>
            {
                self.open_axis3d(graph, layer, if tab == PropsTab::XAxis { 0 } else { 1 });
            }
            Action::GraphProperties { graph, layer, tab, plot } => {
                let token = self.token();
                let mut dialog = GraphPropsDialog::new(graph, layer, tab, token);
                if let Some(plot) = plot {
                    dialog.select_plot(plot);
                }
                self.graph_props = Some(dialog);
                self.focus(graph);
                self.graphs.entry(graph).or_default().active_layer = layer;
            }
            Action::AddLayerDialog { graph, layer } => {
                self.add_layer_dialog = crate::dialogs::add_layer::AddLayerDialog::new(&self.doc, graph, layer);
                if self.add_layer_dialog.is_some() {
                    self.focus(graph);
                }
            }
            Action::GraphEdit { graph, op } => self.edit_graph(graph, op),
            Action::ActivateLayer { graph, layer } => {
                self.graphs.entry(graph).or_default().active_layer = layer;
            }
            Action::GraphTool { graph, tool } => {
                // Mark Data Range puts the markers on the active layer's first plot before its tool starts.
                if tool == GraphTool::Data(DataTool::MarkRange) {
                    let layer = self.graphs.get(&graph).map_or(0, |s| s.active_layer);
                    let op = crate::actions::MarkerOp::Mark { plot: 0 };
                    self.edit_graph(graph, GraphOp::DataMarkers { layer, op });
                }
                let state = self.graphs.entry(graph).or_default();
                state.reader = tool == GraphTool::Reader;
                state.text_tool = tool == GraphTool::Text;
                state.slope_tool = tool == GraphTool::Slope;
                state.slope_first = None;
                state.zoom_in_tool = tool == GraphTool::ZoomIn;
                state.pan_tool = tool == GraphTool::Pan;
                state.scale.tool = match tool {
                    GraphTool::ScaleIn => Some(ScaleTool::In),
                    GraphTool::ScaleOut => Some(ScaleTool::Out),
                    _ => None,
                };
                state.shape_tool = match tool {
                    GraphTool::Draw(kind) => Some(kind),
                    _ => None,
                };
                state.data_tool = match tool {
                    GraphTool::Data(t) => Some(t),
                    _ => None,
                };
                state.reader_point = None;
                match tool {
                    GraphTool::Reader => {
                        self.say("Data Reader: click a data point; Left and Right step through the rows; Esc ends")
                    }
                    GraphTool::Text => self.say("Text: click where the text goes; Esc ends"),
                    GraphTool::Slope => self.say("Slope: click near a data point; Esc cancels"),
                    GraphTool::Draw(kind) => {
                        self.say(format!("{}: drag on the graph (Shift constrains); Esc ends", kind.name()))
                    }
                    GraphTool::Data(tool) => self.say(crate::data_tools::hint(tool)),
                    GraphTool::ZoomIn => self.say("Zoom In: click the page where to zoom; Esc ends"),
                    GraphTool::Pan => self.say("Pan: drag the page to move the view; Esc ends"),
                    GraphTool::ScaleIn => self.say("Scale In: drag a rectangle to magnify the axes; Esc ends"),
                    GraphTool::ScaleOut => self.say("Scale Out: click to step back to the previous axes; Esc ends"),
                    GraphTool::Pointer => {}
                }
            }
            Action::GraphView { graph, op } => {
                let state = self.graphs.entry(graph).or_default();
                match op {
                    ViewOp::ZoomOut => state.zoom_out(),
                    ViewOp::SetZoom(percent) => state.set_zoom_percent(percent),
                    ViewOp::WholePage => state.whole_page(),
                    ViewOp::ZoomAll => state.zoom_all(),
                }
                let percent = (state.zoom.factor * 100.0).round();
                self.say(format!("Zoom {percent}%"));
            }
            Action::ScaleIn { graph, layer, x, y } => {
                let state = &mut self.graphs.entry(graph).or_default().scale;
                if graph_scale::scale_in(&mut self.doc, state, graph, layer, x, y) {
                    self.say("Scale In");
                }
            }
            Action::ScaleOut { graph, layer } => {
                let state = &mut self.graphs.entry(graph).or_default().scale;
                if graph_scale::scale_out(&mut self.doc, state, graph, layer) {
                    self.say("Scale Out");
                } else {
                    self.say("Scale Out: no earlier scale to go back to");
                }
            }
            Action::SetViewMode { graph, mode } => {
                if self.doc.project().graph(graph).is_some() && self.doc.project().view_mode(graph) != mode {
                    self.doc.adjust_view(|p| p.set_view_mode(graph, mode));
                }
            }
            Action::SetShowElements { graph, labels, data } => {
                let what = if labels.is_some() { "Show Labels" } else { "Show Data" };
                self.doc.apply(what, |p| {
                    if let Some(g) = p.graph_mut(graph) {
                        for l in &mut g.layers {
                            if let Some(on) = labels {
                                l.display.show_labels = on;
                            }
                            if let Some(on) = data {
                                l.display.show_data = on;
                            }
                        }
                    }
                });
            }
            Action::SetPageSwitch { graph, switch, on } => {
                use crate::actions::PageSwitch;
                let label = match switch {
                    PageSwitch::ActivateByIconOnly => "Set Active Layer By Layer Icon Only",
                    PageSwitch::IconDoubleClickPlotSetup => "Double click to open Plot Setup",
                    PageSwitch::ActiveLayerIndicator => "Show Active Layer Indicator",
                    PageSwitch::AxisLayerIcons => "Show Axis Layer Icons",
                    PageSwitch::ObjectGrid => "Show Object Grid",
                    PageSwitch::LayerGrid => "Show Layer Grid",
                    PageSwitch::MasterItemsOnScreen => "Show Master Items on Screen",
                };
                self.doc.apply(label, |p| {
                    if let Some(g) = p.graph_mut(graph) {
                        match switch {
                            PageSwitch::ActivateByIconOnly => g.page.style.activate_by_icon_only = on,
                            PageSwitch::IconDoubleClickPlotSetup => g.page.style.icon_double_click_plot_setup = on,
                            PageSwitch::ActiveLayerIndicator => g.page.style.active_layer_indicator = on,
                            PageSwitch::AxisLayerIcons => g.page.style.axis_layer_icons = on,
                            PageSwitch::ObjectGrid => {
                                g.page.style.grid = if on { PageGrid::Object } else { PageGrid::Off }
                            }
                            PageSwitch::LayerGrid => {
                                g.page.style.grid = if on { PageGrid::Layer } else { PageGrid::Off }
                            }
                            PageSwitch::MasterItemsOnScreen => g.page.style.master_items_on_screen = on,
                        }
                    }
                });
            }
            Action::SetLockPosition { graph, position } => {
                if self.doc.project().graph(graph).is_some_and(|g| g.page.style.lock_position != position) {
                    self.doc.apply("Lock position", |p| {
                        if let Some(g) = p.graph_mut(graph) {
                            g.page.style.lock_position = position;
                        }
                    });
                }
            }
            Action::SetLayerIcons { graph, on } => {
                self.doc.apply("Show Layer Icons", |p| {
                    if let Some(g) = p.graph_mut(graph) {
                        g.page.style.layer_icons = on;
                    }
                });
            }
            Action::SetLayerFrame { graph, layer, on } => {
                self.doc.apply("Show Frame", |p| {
                    if let Some(layer) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer))
                        && layer.coords.is_cartesian()
                    {
                        layer.display.show_frame = on;
                    }
                });
            }
            Action::FullScreen(graph) if self.full_screen.is_none() => {
                self.full_screen = Some(crate::view_modes::FullScreen::new(graph, !self.full_screen_reminder_off));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
            Action::FullScreen(_) => {}
            Action::SlideShow(columns) if self.full_screen.is_none() => {
                let graphs = crate::view_modes::dependent_graphs(self.doc.project(), &columns);
                if graphs.is_empty() {
                    self.warn("No graph plots the selected column");
                } else {
                    self.full_screen = Some(crate::view_modes::FullScreen::slide_show(graphs));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                }
            }
            Action::SlideShow(_) => {}
            Action::AddFavorite(id) => {
                if self.doc.project().favorites.contains(&id) {
                    self.say("The window is in Favorites already".to_string());
                } else {
                    let _ = self.doc.edit("Add shortcut to Favorites", |p| Ok::<_, String>(p.add_favorite(id)));
                }
            }
            Action::AuditLog => self.audit_log = true,
            Action::SampleProjects => {
                self.sample_projects.get_or_insert_with(Default::default);
            }
            Action::OpenSample(name) => {
                self.pending_sample = Some(name);
                if self.doc.dirty {
                    self.pending = Some(Pending::Sample);
                } else {
                    self.do_pending(Pending::Sample, ctx);
                }
            }
            Action::FitBuilder => {
                self.fit_builder.get_or_insert_with(Default::default);
            }
            Action::RemoveFavorite(id) => {
                let _ = self.doc.edit("Remove from Favorites", |p| Ok::<_, String>(p.remove_favorite(id)));
            }
            Action::DataEdit(edit) => self.data_edit(edit),
            Action::AddText { graph, layer, at } => {
                let state = self.graphs.entry(graph).or_default();
                state.text_tool = false;
                let layer = layer.unwrap_or(state.active_layer);
                self.text_dialog = TextDialog::new_label(&self.doc, graph, layer, at.unwrap_or((0.5, 0.5)));
            }
            Action::EditText { graph, layer, label } => {
                // A table (T136) is edited cell by cell.
                let table = self
                    .doc
                    .project()
                    .graph(graph)
                    .and_then(|g| g.layers.get(layer))
                    .and_then(|l| l.labels.get(label))
                    .filter(|l| l.table)
                    .map(|l| l.cells());
                match table {
                    Some(cells) => {
                        self.table_editor =
                            Some(crate::dialogs::table_editor::TableEditor::edit(graph, layer, label, cells));
                    }
                    None => self.text_dialog = TextDialog::edit(&self.doc, graph, layer, label),
                }
            }
            Action::NewTable { graph, layer } => {
                self.table_editor = Some(crate::dialogs::table_editor::TableEditor::new_table(graph, layer));
            }
            Action::EditTitle { graph, layer, vertical } => {
                self.text_dialog = TextDialog::axis_title(&self.doc, graph, layer, vertical);
            }
            Action::EditLegendText { graph, layer } => {
                self.text_dialog = TextDialog::legend(&self.doc, graph, layer);
            }
            Action::AddShape { graph, layer, shape } => {
                self.doc.apply("Add drawn object", |p| {
                    if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                        l.shapes.push(shape);
                    }
                });
            }
            Action::ArrangeObjects { graph, op } => {
                let changes = self
                    .graphs
                    .get(&graph)
                    .and_then(|s| s.arrange_members(self.doc.project(), graph))
                    .map(|(reference, others)| crate::obj_align::plan(op, &reference, &others));
                match changes {
                    None => self.say("Select two or more drawn objects or text labels first (Shift+click adds one)"),
                    Some(c) if c.is_empty() => self.say("The objects already match"),
                    Some(c) => self.doc.apply(op.undo_label(), |p| crate::obj_align::apply(p, graph, &c)),
                }
            }
            Action::GroupObjects { graph, ungroup } => {
                let selection = self.graphs.get(&graph).map(|s| s.multi_selection()).unwrap_or_default();
                let Some(g) = self.doc.project().graph(graph) else { return };
                if ungroup && crate::obj_align::can_ungroup(g, &selection) {
                    self.doc.apply("Ungroup objects", |p| crate::obj_align::ungroup(p, graph, &selection));
                } else if !ungroup && crate::obj_align::can_group(g, &selection) {
                    self.doc.apply("Group objects", |p| crate::obj_align::group(p, graph, &selection));
                }
            }
            Action::MoveObjects { graph, changes } => {
                self.doc.apply("Move objects", |p| crate::obj_align::apply(p, graph, &changes));
            }
            Action::MoveShape { graph, layer, shape, dx, dy } => {
                self.doc.apply("Move drawn object", |p| {
                    if let Some(s) =
                        p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.shapes.get_mut(shape))
                    {
                        s.translate(dx, dy);
                    }
                });
            }
            Action::AddSlope { graph, layer, slope } => {
                self.doc.apply("Add slope annotation", |p| {
                    if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                        l.slopes.push(slope);
                    }
                });
                self.say("Slope annotation added");
            }
            Action::EditSlope { graph, layer, slope } => {
                self.slope_dialog = SlopeDialog::edit(&self.doc, graph, layer, slope);
            }
            Action::AddStraightLine { graph, layer } => {
                self.straight_line_dialog = StraightLineDialog::new_line(&self.doc, graph, layer);
            }
            Action::EditStraightLine { graph, layer, line } => {
                self.straight_line_dialog = StraightLineDialog::edit(&self.doc, graph, layer, line);
            }
            Action::Hint(text) => self.say(text),
            Action::ResizeShape { graph, layer, shape, before, after } => {
                let dialogs = self.shape_dialog_shapes();
                self.doc.apply("Resize drawn object", |p| {
                    if let Some(s) =
                        p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.shapes.get_mut(shape))
                        && *s == before
                    {
                        *s = after;
                    }
                });
                self.close_shape_dialog_if_changed(dialogs);
            }
            Action::EditShape { graph, layer, shape } => {
                self.shape_dialog = ShapeDialog::edit(&self.doc, graph, layer, shape);
            }
            Action::MoveLabel { graph, layer, label, x, y } => {
                self.doc.apply("Move text", |p| {
                    if let Some(l) =
                        p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.labels.get_mut(label))
                    {
                        l.x = x;
                        l.y = y;
                    }
                });
            }
            Action::MoveAxisTitle { graph, layer, y, dx, dy } => {
                self.doc.apply("Move axis title", |p| {
                    if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                        let axis = if y { &mut l.y_axis } else { &mut l.x_axis };
                        axis.title_offset_pt = (axis.title_offset_pt.0 + dx, axis.title_offset_pt.1 + dy);
                    }
                });
            }
            Action::MoveLegend { graph, layer, x, y } => {
                self.doc.apply("Move legend", |p| {
                    if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                        l.legend.position = sheaf_core::graph::LegendPosition::Manual { x, y };
                    }
                });
            }
            Action::CopyPage(graph) => self.copy_page(graph, ctx),
            Action::CopyGraphFormat { graph, part } => {
                if crate::format_clip_ui::copy_graph(&self.doc, graph, part) {
                    self.say("Format copied");
                }
            }
            Action::EditCopyFormat { graph, part } => {
                use sheaf_core::format_clip::{AxisPart, GraphPart};
                use sheaf_plot::scene::ObjectKind;
                let axis = match self.graphs.get(&graph).and_then(|s| s.selected) {
                    Some((layer, ObjectKind::XAxis)) => Some((layer, false)),
                    Some((layer, ObjectKind::YAxis)) => Some((layer, true)),
                    _ => None,
                };
                if let Some((layer, y)) = axis {
                    // Origin copies an axis format here; only the parts an axis holds are measured.
                    let part = match part {
                        GraphPart::All => Some(AxisPart::All),
                        GraphPart::AllStyle => Some(AxisPart::Styles),
                        GraphPart::Scales => Some(AxisPart::Scales),
                        _ => None,
                    };
                    if let Some(part) = part {
                        self.apply(Action::CopyAxisFormat { graph, layer, y, part }, ctx);
                    }
                } else {
                    self.apply(Action::CopyGraphFormat { graph, part }, ctx);
                }
            }
            Action::CopyAxisFormat { graph, layer, y, part } => {
                if crate::format_clip_ui::copy_axis(&self.doc, graph, layer, y, part) {
                    self.say("Format copied");
                }
            }
            Action::PasteGraphFormat { graph } => {
                // Every group the clipboard holds (a Page Background copy holds only the page colour).
                crate::format_clip_ui::paste_graph(&mut self.doc, graph, sheaf_core::format_clip::EVERY_GROUP);
            }
            Action::PasteAxesFormat { graph, targets, part } => {
                crate::format_clip_ui::paste_axes(&mut self.doc, graph, targets, part);
            }
            Action::PasteAxisFormat { graph, layer, y, part } => {
                crate::format_clip_ui::paste_axis(&mut self.doc, graph, layer, y, part);
            }
            Action::CopyLegendStyles { graph, layer } => {
                if crate::format_clip_ui::copy_legend_styles(&self.doc, graph, layer) {
                    self.say("Format copied");
                }
            }
            Action::CopyLegendFormat { graph, layer, part } => {
                if crate::format_clip_ui::copy_legend(&self.doc, graph, layer, part) {
                    self.say("Format copied");
                }
            }
            Action::PasteLegendFormat { graph, layer, part } => {
                crate::format_clip_ui::paste_legend(&mut self.doc, graph, layer, part);
            }
            Action::PasteLegendStyles { graph, layer } => {
                crate::format_clip_ui::paste_legend_styles(&mut self.doc, graph, layer);
            }
            Action::PasteFormatDialog { graph, layer } => {
                self.apply_formats = Some(crate::format_clip_ui::ApplyFormats::new(graph, layer));
            }
            Action::StartIntegrate { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::Integrate),
            Action::StartRiseTime { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::RiseTime),
            Action::StartIntersect { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::Intersect),
            Action::StartQuickPeaks { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::QuickPeaks),
            Action::StartSigmoid { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::Sigmoid),
            Action::StartFft { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::Fft),
            Action::StartStatisticsGadget { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::Statistics),
            Action::StartQuickFit { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::QuickFit),
            Action::StartDifferentiate { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::Differentiate),
            Action::StartInterpolate { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::Interpolate),
            Action::StartCluster { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::Cluster),
            Action::StartIntegrate2d { graph } => self.start_gadget(graph, crate::gadget::GadgetKind::Integrate2d),
            Action::StartVerticalCursor { graph } => {
                self.start_gadget(graph, crate::gadget::GadgetKind::VerticalCursor)
            }
            Action::Gadget { graph, op } => self.gadget(graph, op),
            Action::ExportGraph(graph) => {
                self.export = Some(
                    ExportDialog::new(graph, self.folder())
                        .with_themes(self.startup.themes_dir.clone())
                        .with_explorer_folder(self.doc.project(), &self.panels.explorer.folder),
                );
            }
            Action::SendGraphsToPowerPoint(graph) => {
                let p = self.doc.project();
                let name = p.graph(graph).map(|g| g.name.clone()).unwrap_or_default();
                let all = p.graphs().map(|g| g.id).collect();
                self.send_ppt = Some(crate::dialogs::send_ppt::SendPptDialog::new((graph, name), all));
            }
            Action::WriteGraphDocument { graphs, path } => {
                // 300 dpi pictures, as Export Graphs' default resolution.
                let project = self.doc.project();
                let pictures = graphs
                    .iter()
                    .map(|&g| {
                        let png = sheaf_plot::render_png(project, g, 300.0).map_err(|e| e.to_string())?;
                        let name = project.graph(g).map(|g| g.name.clone()).unwrap_or_default();
                        Ok((png, name))
                    })
                    .collect::<Result<Vec<_>, String>>();
                let word = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("docx"));
                let written = pictures
                    .and_then(|p| {
                        if word {
                            let p: Vec<_> =
                                p.into_iter().map(|(png, name)| sheaf_io::docx::DocxPicture { png, name }).collect();
                            sheaf_io::docx::docx_bytes(&p)
                        } else {
                            let p: Vec<_> =
                                p.into_iter().map(|(png, name)| sheaf_io::pptx::PptxSlide { png, name }).collect();
                            sheaf_io::pptx::pptx_bytes(&p)
                        }
                    })
                    .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
                match written {
                    Ok(()) => {
                        let n = graphs.len();
                        let s = if n == 1 { "" } else { "s" };
                        self.say(format!("Sent {n} graph{s} to {}", path.display()));
                    }
                    Err(e) => self.warn(format!("Could not write {}: {e}", path.display())),
                }
            }
            Action::ExportProjectGraphs(what) => {
                use crate::dialogs::export::ExportFormat;
                use crate::menus::ProjectGraphsExport;
                // The graph that was active last starts the window (it names the first file), else the first.
                let graphs: Vec<WindowId> = self.doc.project().graphs().map(|g| g.id).collect();
                let Some(&first) = graphs.first() else {
                    self.warn("The project has no graph to export");
                    return;
                };
                let start = self.workspace.recent_order(&graphs).into_iter().next().unwrap_or(first);
                let (format, multipage) = match what {
                    ProjectGraphsExport::Graphs | ProjectGraphsExport::Image => (ExportFormat::Png, false),
                    ProjectGraphsExport::MultiPagePdf => (ExportFormat::Pdf, true),
                };
                self.export = Some(
                    ExportDialog::new(start, self.folder())
                        .with_themes(self.startup.themes_dir.clone())
                        .with_explorer_folder(self.doc.project(), &self.panels.explorer.folder)
                        .for_all_graphs(format, multipage),
                );
            }
            Action::ExportData(book) => self.export_data(book),
            Action::ExportExcel(book) => {
                if let Some(b) = self.doc.project().workbook(book) {
                    self.export_excel = Some(ExcelExportDialog::new(book, b.sheets.len(), self.folder()));
                }
            }
            Action::Print(graph) => self.print_graph(graph),
            Action::PrintPreview(graph) => self.print_preview = Some(PrintPreview::new(graph)),
            Action::PageSetup => self.page_setup = Some(PageSetupDialog::new(self.doc.project().page_setup)),
            Action::Focus(id) => self.focus(id),
            Action::OpenSwitcher => self.open_switcher(ctx, false, true),
            Action::HideWindow(id) => self.workspace.hide(id),
            Action::AppendProject(path) => {
                let path = path.or_else(|| {
                    let mut chooser = rfd::FileDialog::new().set_title("Append project").add_filter(
                        "Projects and window files",
                        &[
                            project_file::EXTENSION,
                            "opj",
                            "opju",
                            WindowFileKind::Book.extension(),
                            WindowFileKind::Graph.extension(),
                        ],
                    );
                    if let Some(dir) = self.folder() {
                        chooser = chooser.set_directory(dir);
                    }
                    chooser.pick_file()
                });
                if let Some(path) = path {
                    self.append_project_file(&path);
                }
            }
            Action::SaveWindowAs(id) => self.save_window_as(id),
            Action::SaveTemplateAs(window) => {
                use sheaf_io::user_templates::TemplateKind;
                let project = self.doc.project();
                let (kind, name, existing) = if let Some(b) = project.workbook(window) {
                    (TemplateKind::Workbook, b.name.clone(), &self.user_book_templates)
                } else if let Some(g) = project.graph(window) {
                    (TemplateKind::Graph, g.name.clone(), &self.user_templates)
                } else {
                    return;
                };
                self.template_prompt = Some(crate::dialogs::user_templates::SaveTemplatePrompt::new(
                    window,
                    kind,
                    &name,
                    existing.clone(),
                ));
            }
            Action::SaveAnalysisTemplateAs(book) => {
                let Some(name) = self.doc.project().workbook(book).map(|b| b.name.clone()) else { return };
                self.template_prompt = Some(crate::dialogs::user_templates::SaveTemplatePrompt::analysis(
                    book,
                    &name,
                    self.user_book_templates.clone(),
                ));
            }
            Action::BatchPeaks { book, columns } => self.open_batch_peaks(book, columns),
            Action::BatchProcessing => {
                // T233: each template's sheet names, for the Summary Sheet list.
                let dir = self.startup.templates_dir.clone();
                let templates = self
                    .user_book_templates
                    .iter()
                    .map(|name| {
                        let sheets = dir
                            .as_deref()
                            .and_then(|d| sheaf_io::user_templates::load_book(d, name).ok())
                            .map(|t| t.book.sheets.iter().map(|s| s.name.clone()).collect())
                            .unwrap_or_default();
                        (name.clone(), sheets)
                    })
                    .collect();
                self.batch = Some(crate::dialogs::batch_process::BatchDialog::new(templates));
            }
            Action::ManageTemplates => {
                self.template_manager = Some(crate::dialogs::user_templates::TemplateManager::new(
                    self.user_templates.clone(),
                    self.user_book_templates.clone(),
                ));
            }
            Action::PlotUserTemplate { book, columns, name } => self.plot_user_template(book, columns, &name),
            Action::FillPattern { kind, columns } => {
                self.fill_pattern = Some(crate::dialogs::fill_pattern::FillPatternDialog::new(kind, columns));
            }
            Action::CopyShape { graph, layer, shape, cut } => {
                let copied = self.doc.project().graph(graph).is_some_and(|g| crate::shape_clip::copy(g, layer, shape));
                if copied && cut {
                    self.edit_graph(graph, crate::actions::GraphOp::DeleteShape { layer, shape });
                }
            }
            Action::ObjectProperties { graph } => {
                if let Some((layer, kind)) = self.graphs.get(&graph).and_then(|s| s.selected) {
                    self.apply(graph_view::properties_of(graph, layer, kind), ctx);
                }
            }
            Action::GraphClipboard { graph, op } => {
                use crate::menus::GraphClip;
                use sheaf_plot::scene::ObjectKind as K;
                let Some(state) = self.graphs.get_mut(&graph) else { return };
                match op {
                    GraphClip::Cut | GraphClip::Copy => {
                        let cut = op == GraphClip::Cut;
                        let next = match state.selected {
                            Some((layer, K::Shape(shape))) => Some(Action::CopyShape { graph, layer, shape, cut }),
                            Some((layer, K::Label(label))) => Some(Action::CopyLabel { graph, layer, label, cut }),
                            Some((layer, K::Legend)) => Some(Action::CopyLegend { graph, layer, cut }),
                            Some((layer, K::XTitle)) => Some(Action::CopyTitle { graph, layer, y: false, cut }),
                            Some((layer, K::YTitle)) => Some(Action::CopyTitle { graph, layer, y: true, cut }),
                            _ => None,
                        };
                        if cut && next.is_some() {
                            state.selected = None;
                        }
                        if let Some(next) = next {
                            self.apply(next, ctx);
                        }
                    }
                    GraphClip::Clear => {
                        if let Some(op) = state.delete_selected() {
                            self.edit_graph(graph, op);
                        }
                    }
                    GraphClip::Paste => {
                        let layer = state.active_layer;
                        let clicked = state.last_click;
                        let page = clicked.or_else(|| {
                            let g = self.doc.project().graph(graph)?;
                            let f = g.absolute_frame(layer)?;
                            Some((
                                (f.left + f.width / 2.0) * g.page.width_pt,
                                (f.top + f.height / 2.0) * g.page.height_pt,
                            ))
                        });
                        if let Some(page) = page {
                            self.apply(Action::PasteShape { graph, layer, page }, ctx);
                        }
                    }
                }
            }
            Action::CopyTitle { graph, layer, y, cut } => {
                let project = self.doc.project();
                let copied = sheaf_plot::axis_title(project, graph, layer, y).ok().is_some_and(|text| {
                    project.graph(graph).is_some_and(|g| crate::shape_clip::copy_title(g, layer, y, text))
                });
                if copied && cut {
                    self.edit_graph(graph, crate::actions::GraphOp::HideTitle { layer, y });
                }
            }
            Action::CopyLegend { graph, layer, cut } => {
                let project = self.doc.project();
                let copied = sheaf_plot::legend_text(project, graph, layer).ok().is_some_and(|text| {
                    project.graph(graph).is_some_and(|g| crate::shape_clip::copy_legend(g, layer, text))
                });
                if copied && cut {
                    self.edit_graph(graph, crate::actions::GraphOp::HideLegend { layer });
                }
            }
            Action::CopyLabel { graph, layer, label, cut } => {
                let copied =
                    self.doc.project().graph(graph).is_some_and(|g| crate::shape_clip::copy_label(g, layer, label));
                if copied && cut {
                    self.edit_graph(graph, crate::actions::GraphOp::DeleteLabel { layer, label });
                }
            }
            Action::PickGraphImage { graph, layer, page } => {
                let picked = rfd::FileDialog::new()
                    .set_title("Insert Images From Files")
                    .add_filter("PNG image", &["png"])
                    .pick_file();
                let Some(path) = picked else { return };
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        let png = std::sync::Arc::new(bytes);
                        self.apply(Action::InsertGraphImage { graph, layer, page, png }, ctx);
                    }
                    Err(e) => self.warn(format!("{}: {e}", path.display())),
                }
            }
            Action::InsertGraphImage { graph, layer, page, png } => {
                let Ok(img) = image::load_from_memory_with_format(&png, image::ImageFormat::Png) else {
                    self.warn("That file is not a PNG picture Sheaf can read".to_string());
                    return;
                };
                let picture = sheaf_core::shape::Picture { png, width_px: img.width(), height_px: img.height() };
                let shape = self
                    .doc
                    .project()
                    .graph(graph)
                    .and_then(|g| crate::shape_clip::picture_at(g, layer, page, picture));
                if let Some(shape) = shape {
                    self.doc.apply("Insert Image", |p| {
                        if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                            l.shapes.push(shape);
                        }
                    });
                }
            }
            Action::PasteShape { graph, layer, page } => {
                use crate::shape_clip::Pasted;
                if let Some(pasted) =
                    self.doc.project().graph(graph).and_then(|g| crate::shape_clip::placed(g, layer, page))
                {
                    let what = if matches!(pasted, Pasted::Shape(_)) { "Paste drawn object" } else { "Paste text" };
                    self.doc.apply(what, |p| {
                        if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                            match pasted {
                                Pasted::Shape(s) => l.shapes.push(s),
                                Pasted::Label(t) => l.labels.push(t),
                            }
                        }
                    });
                }
            }
            Action::QuickFill { kind, columns } => {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |t| t.as_nanos() as u64);
                let filled = self.doc.edit("Fill column", |p| {
                    let mut rows = 0;
                    for (k, c) in columns.iter().enumerate() {
                        rows = p.fill_quick(*c, kind, seed.wrapping_add(k as u64))?;
                    }
                    Ok::<usize, sheaf_core::fill_pattern::FillError>(rows)
                });
                match filled {
                    Ok(rows) => self.say(format!("{}: {} column(s), {rows} rows each", kind.label(), columns.len())),
                    Err(e) => self.logs.warn(e.to_string()),
                }
            }
            Action::SetSamplingInterval { columns } => {
                let current =
                    columns.first().and_then(|c| self.doc.project().column(*c)).and_then(|c| c.sampling.as_ref());
                let mut dialog = crate::dialogs::sampling::SamplingDialog::new(columns.clone(), current);
                dialog.from_x = columns.first().and_then(|c| sampling_from_x(self.doc.project(), *c));
                self.sampling = Some(dialog);
            }
            Action::ShowXColumn { columns } => {
                self.show_x = Some(crate::dialogs::sampling::ShowXDialog::new(columns));
            }
            Action::ReloadRecent => {
                self.recent.reload();
                ctx.request_repaint();
            }
            Action::OpenRecent(path, kind) => self.follow_recent(path, kind),
            Action::ImportRecent(path) => self.follow_recent(path, RecentKind::Imports),
            Action::ClearRecent(kind) => {
                let failed = match kind {
                    RecentKind::Projects => self.recent.clear_projects(),
                    RecentKind::Imports => self.recent.clear_imports(),
                    RecentKind::Books => self.recent.clear_books(),
                    RecentKind::Graphs => self.recent.clear_graphs(),
                };
                if let Some(why) = failed {
                    self.logs.warn(why);
                }
            }
            Action::UsedCommand { menu, cmd } => {
                let kind = self.menu_context().kind;
                if let Some(key) = crate::mru::key_of(crate::menus::bar(kind), menu, &cmd)
                    && let Some(why) = self.recent.push_command(&menu.list_name(), &key)
                {
                    self.logs.warn(why);
                }
            }
            Action::OpenRecentExport(entry) => self.open_recent_export(&entry),
            Action::ClearRecentExports => {
                if let Some(why) = self.recent.clear_exports() {
                    self.logs.warn(why);
                }
            }
            Action::DuplicateWindow(id) => {
                let mut new = None;
                self.doc.apply("Duplicate window", |p| new = p.duplicate_window(id));
                if let Some(new) = new {
                    self.focus(new);
                }
            }
            Action::MoveWindow { window, folder } => {
                if explorer::home(&self.doc.project().root, window) != folder
                    && explorer::folder_at(&self.doc.project().root, &folder).is_some()
                {
                    self.doc.apply("Move window", |p| {
                        explorer::unlist(&mut p.root, window);
                        if let Some(f) = explorer::folder_at_mut(&mut p.root, &folder) {
                            f.windows.push(window);
                        }
                    });
                }
            }
            Action::NewFolder(path) => {
                let mut made = None;
                self.doc.apply("New folder", |p| {
                    if let Some(f) = explorer::folder_at_mut(&mut p.root, &path) {
                        let name = (1..)
                            .map(|n| format!("Folder{n}"))
                            .find(|n| !f.folders.iter().any(|c| c.name.eq_ignore_ascii_case(n)))
                            .expect("the name sequence is unbounded");
                        f.folders.push(Folder { name, ..Folder::default() });
                        made = Some(f.folders.len() - 1);
                    }
                });
                if let Some(i) = made {
                    let mut selected = path;
                    selected.push(i);
                    self.panels.explorer.folder = selected;
                }
            }
            Action::RenameFolder(path) => {
                if let Some(f) = explorer::folder_at(&self.doc.project().root, &path).filter(|_| !path.is_empty()) {
                    self.rename = Some(RenameDialog {
                        window: WindowId(0),
                        long_name: f.name.clone(),
                        short_name: String::new(),
                        error: None,
                        folder: Some(path),
                        comments: String::new(),
                    });
                }
            }
            Action::DeleteFolder(path) => {
                if !path.is_empty()
                    && let Some(f) = explorer::folder_at(&self.doc.project().root, &path)
                {
                    let windows = explorer::subtree_windows(f);
                    let (last, parent) = path.split_last().expect("the path is not empty");
                    let (last, parent) = (*last, parent.to_vec());
                    self.doc.apply("Delete folder", |p| {
                        for id in windows {
                            let columns: Vec<ColumnId> = p
                                .workbook(id)
                                .map(|b| b.sheets.iter().flat_map(|s| s.columns.iter().map(|c| c.id)).collect())
                                .unwrap_or_default();
                            delete_columns(p, &columns);
                            p.remove_window(id);
                        }
                        if let Some(f) = explorer::folder_at_mut(&mut p.root, &parent)
                            && last < f.folders.len()
                        {
                            f.folders.remove(last);
                        }
                    });
                    self.panels.explorer.folder = parent;
                }
            }
            Action::SetPlotVisible { graph, layer, plot, visible } => {
                let label = if visible { "Show plot" } else { "Hide plot" };
                self.doc.apply(label, |p| objects::set_plot_visible(p, graph, layer, plot, visible));
            }
            Action::SetLayerVisible { graph, layer, visible } => {
                let label = if visible { "Show layer" } else { "Hide layer" };
                self.doc.apply(label, |p| objects::set_layer_visible(p, graph, layer, visible));
            }
            Action::SelectPlot { graph, layer, plot } => {
                self.panels.objects.selected = Some((layer, Some(plot)));
                self.graphs.entry(graph).or_default().active_layer = layer;
                if let Some(d) = &mut self.graph_props
                    && d.layer == layer
                    && d.graph == graph
                {
                    d.select_plot(plot);
                }
            }
            Action::TogglePanel(id) => self.panels.toggle(id),
            Action::ClearLog(PanelId::Results) => self.logs.results.clear(),
            Action::ClearLog(_) => self.logs.messages.clear(),
            Action::DeleteWindow(id) => {
                self.doc.apply("Delete window", |p| {
                    let columns: Vec<ColumnId> = p
                        .workbook(id)
                        .map(|b| b.sheets.iter().flat_map(|s| s.columns.iter().map(|c| c.id)).collect())
                        .unwrap_or_default();
                    delete_columns(p, &columns);
                    p.remove_window(id);
                });
            }
            Action::AddStickyNote { window, sheet } => {
                use sheaf_core::sticky::NoteTarget;
                let target = match self.doc.project().workbook(window) {
                    Some(b) if sheet => NoteTarget::Sheet(window, b.active_sheet.min(b.sheets.len().saturating_sub(1))),
                    _ => NoteTarget::Window(window),
                };
                if let Err(e) = self.doc.edit("Add Sticky Note", |p| p.add_sticky_note(target)) {
                    self.warn(e.to_string());
                }
            }
            Action::EditStickyNote { target, change } => {
                use crate::actions::StickyChange;
                let done = match change {
                    StickyChange::Text(text) => {
                        self.doc.apply_merging("Edit Sticky Note", crate::sticky_view::typing_token(target), |p| {
                            p.edit_sticky_note(target, |n| n.text = text)
                        })
                    }
                    StickyChange::Color(c) => {
                        self.doc.edit("Sticky Note Color", |p| p.edit_sticky_note(target, |n| n.color = c))
                    }
                    StickyChange::Collapsed(on) => {
                        self.doc.edit(if on { "Collapse Sticky Note" } else { "Expand Sticky Note" }, |p| {
                            p.edit_sticky_note(target, |n| n.collapsed = on)
                        })
                    }
                };
                if let Err(e) = done {
                    self.warn(e.to_string());
                }
            }
            Action::RemoveStickyNote { target } => {
                let _ = self
                    .doc
                    .edit("Delete Sticky Note", |p| Ok::<_, std::convert::Infallible>(p.remove_sticky_note(target)));
            }
            Action::RenameWindow(id) => {
                if let Some(w) = self.doc.project().window(id) {
                    let long_name = match w {
                        Window::Workbook(b) => b.long_name.clone(),
                        Window::Graph(g) => g.long_name.clone(),
                        Window::Notes(n) => n.long_name.clone(),
                        Window::Matrix(m) => m.long_name.clone(),
                        Window::Layout(l) => l.long_name.clone(),
                    };
                    let short_name = w.name().to_string();
                    let comments = self.doc.project().window_comments(id).to_string();
                    self.rename =
                        Some(RenameDialog { window: id, long_name, short_name, error: None, folder: None, comments });
                }
            }
            Action::Cascade => self.workspace.cascade(),
            Action::TileHorizontally => self.workspace.tile_horizontally(),
            Action::TileVertically => self.workspace.tile_vertically(),
            Action::ArrangeIcons => match self.workspace.arrange_icons() {
                0 => self.say("Arrange Icons: no window is minimised"),
                n => self.say(format!("Arrange Icons: {n} minimised window(s) lined up along the bottom")),
            },
            Action::About => self.about = true,
            Action::Toolbars => self.toolbars.dialog = true,
            Action::Options => {
                let s = &self.startup;
                let settings_dir = s.settings_path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
                let locations = vec![
                    ("Settings", settings_dir),
                    ("Recovery files", s.recovery_dir.clone()),
                    ("User templates", s.templates_dir.clone()),
                    ("Graph themes", s.graph_themes_dir.clone()),
                    ("Dialog themes", s.themes_dir.clone()),
                    ("Fitting functions", s.fit_functions_dir.clone()),
                ];
                self.options = Some(crate::dialogs::options::OptionsDialog::new(
                    self.autosave.settings,
                    self.autosave.has_session(),
                    locations,
                ));
            }
            Action::Autosave => {
                self.autosave_dialog = Some(AutosaveDialog::new(self.autosave.settings, self.autosave.has_session()));
            }
            Action::ToggleStatusBar => self.status_bar_hidden = !self.status_bar_hidden,
            Action::ToggleFormulaBar => self.show_formula_bar = !self.show_formula_bar,
            Action::FillFormulaDown { column, row } => {
                let last = self.doc.project().locate(column).map_or(0, |(_, s, _)| s.row_count()).saturating_sub(1);
                if last <= row {
                    self.say("There are no rows with data below this cell to fill".to_string());
                } else {
                    match self.doc.edit("Fill Down", |p| p.fill_cell_formula_down(column, row, last)) {
                        Ok(n) => self.say(format!("Filled the formula into {n} cell(s) below")),
                        Err(e) => self.warn(e.to_string()),
                    }
                }
            }
            Action::ToggleAiLink => self.set_link(!self.live.active()),
            Action::Quit => {
                if self.doc.dirty {
                    self.pending = Some(Pending::Quit);
                } else {
                    self.do_pending(Pending::Quit, ctx);
                }
            }
        }
    }

    fn do_pending(&mut self, pending: Pending, ctx: &egui::Context) {
        match pending {
            Pending::New => {
                self.doc.replace(Project::new());
                self.path = None;
                self.recovered_from = None;
                self.reset_views();
                self.say("New project".to_string());
            }
            Pending::OpenFile => {
                if let Some(path) = self.pending_path.take() {
                    let windows = std::mem::take(&mut self.pending_windows);
                    if self.load_project(path) {
                        self.append_window_files(&windows);
                    }
                }
            }
            Pending::DuplicateNoData => self.duplicate_project_without_data(),
            Pending::Sample => {
                let name = self.pending_sample.take().unwrap_or_default();
                match sheaf_core::samples::sample_project(&name) {
                    Some(project) => {
                        self.doc.replace(project);
                        self.path = None;
                        self.recovered_from = None;
                        self.reset_views();
                        self.say(format!("Opened the sample project {name}"));
                    }
                    None => self.warn(format!("There is no sample project {name}")),
                }
            }
            Pending::Recover => self.recover_newest(),
            Pending::Quit => {
                self.quit_confirmed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// Copy, cut, paste and clear on the selected cells of a workbook. Cut, paste and clear
    /// are one undo step each.
    fn cell_op(&mut self, book: WindowId, op: CellOp, ctx: &egui::Context) {
        if let Some(Sel::Label { column, row }) = self.books.get(&book).map(|s| s.sel.clone()) {
            if row == LabelRow::SamplingInterval && !matches!(op, CellOp::Copy(_)) {
                self.warn("Change the interval in Set Sampling Interval");
                return;
            }
            if row == LabelRow::Sparklines {
                self.warn("Sparklines are drawn from the column's values");
                return;
            }
            if row == LabelRow::Categories && !matches!(op, CellOp::Copy(_)) {
                self.warn("The categories come from the column's values");
                return;
            }
            let Some(c) = self.doc.project().column(column) else { return };
            let original = grid::label_text_full(c, &row);
            let replacement = match op {
                CellOp::Copy(_) | CellOp::Cut => {
                    self.worksheet_clipboard.write_plain(ctx, original.clone());
                    self.copied = original;
                    if op == CellOp::Cut { Some(String::new()) } else { None }
                }
                CellOp::Clear => Some(String::new()),
                CellOp::Paste => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    None
                }
                CellOp::PasteText(text) | CellOp::Fill(text) => Some(text),
                _ => {
                    self.warn("Select data cells to insert, delete, or mask worksheet data");
                    None
                }
            };
            if let Some(text) = replacement {
                if row == LabelRow::Formula {
                    self.warn("Change the formula in Set Column Values");
                } else {
                    self.apply(Action::SetLabel { column, row, text }, ctx);
                }
            }
            return;
        }
        let Some(b) = self.doc.project().workbook(book) else {
            return;
        };
        let sheet_index = b.active_sheet.min(b.sheets.len().saturating_sub(1));
        let Some(sheet) = b.sheets.get(sheet_index) else {
            return;
        };
        let target = self.books.get(&book).and_then(|s| s.target(sheet));
        let Some(t) = target else {
            if matches!(op, CellOp::Paste | CellOp::PasteTranspose | CellOp::PasteText(_)) {
                self.warn("Click a cell first: the pasted text starts there".to_string());
            }
            return;
        };
        match op {
            CellOp::Copy(_) | CellOp::Cut => {
                let kind = if let CellOp::Copy(k) = op { k } else { CopyKind::Plain };
                let part = if let CopyKind::Part(p) = kind { p } else { sheaf_core::clipboard::CopyPart::Values };
                let options = CopyOptions {
                    // Copy never drops digits: a copy and paste inside Sheaf must give back the same numbers.
                    full_precision: true,
                    label_rows: kind == CopyKind::WithLabels,
                    part,
                };
                let text = sheet.copy_text(&t.columns, t.rows.clone(), options);
                let external = sheet.copy_external_text(
                    &t.columns,
                    t.rows.clone(),
                    CopyOptions {
                        full_precision: kind == CopyKind::FullPrecision,
                        label_rows: options.label_rows,
                        part,
                    },
                );
                self.worksheet_clipboard.write(ctx, &external, &text);
                self.copied = text;
                // Paste Link (T211) links to the copied cells; a Cut leaves nothing to link to.
                self.copied_cells = (!matches!(op, CellOp::Cut)).then(|| {
                    (t.columns.iter().filter_map(|&i| sheet.columns.get(i).map(|c| c.id)).collect(), t.rows.clone())
                });
                let what = format!("{} row(s) of {} column(s)", t.rows.len(), t.columns.len());
                if matches!(op, CellOp::Cut) {
                    self.clear_cells(book, sheet_index, &t, "Cut cells");
                    self.say(format!("Cut {what}"));
                } else {
                    self.say(format!("Copied {what}"));
                }
            }
            CellOp::Clear => self.clear_cells(book, sheet_index, &t, "Clear cells"),
            CellOp::PasteLink(transpose) => {
                let Some((sources, rows)) = self.copied_cells.clone() else {
                    self.warn("Copy cells first: Paste Link links to the cells last copied".to_string());
                    return;
                };
                let (anchor_col, anchor_row) = (t.columns.first().copied().unwrap_or(0), t.rows.start);
                let mut links = Vec::new();
                let mut left_out = 0;
                for (i, src_row) in rows.clone().enumerate() {
                    for (j, src) in sources.iter().enumerate() {
                        let (dc, dr) = if transpose { (i, j) } else { (j, i) };
                        match sheet.columns.get(anchor_col + dc) {
                            Some(c) if !c.is_locked() => links.push((c.id, anchor_row + dr, *src, src_row)),
                            _ => left_out += 1,
                        }
                    }
                }
                let label = if transpose { "Paste Link Transpose" } else { "Paste Link" };
                let n = links.len();
                self.doc.apply(label, |p| {
                    if let Some(ws) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet_index)) {
                        for (column, row, source, source_row) in links {
                            ws.set_cell_link(column, row, Some((source, source_row)));
                        }
                    }
                });
                if left_out > 0 {
                    self.warn(format!(
                        "{n} cell(s) linked; {left_out} left out (past the last column, or a column calculated from a formula)"
                    ));
                } else {
                    self.say(format!("{n} cell(s) linked"));
                }
            }
            CellOp::RemoveLinks => {
                let ids: Vec<ColumnId> = t.columns.iter().filter_map(|&i| sheet.columns.get(i).map(|c| c.id)).collect();
                let mut removed = 0;
                self.doc.apply("Remove Links", |p| {
                    if let Some(ws) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet_index)) {
                        removed = ws.remove_cell_links(&ids, Some(t.rows.clone()));
                    }
                });
                self.say(format!("{removed} link(s) removed; the cells keep their values"));
            }
            CellOp::Fill(text) => {
                let (mut locked, mut not_understood) = (0, 0);
                self.doc.apply("Fill cells", |p| {
                    let Some(ws) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet_index)) else { return };
                    for c in &t.columns {
                        let Some(column) = ws.columns.get_mut(*c) else { continue };
                        if column.is_locked() {
                            locked += 1;
                            continue;
                        }
                        for row in t.rows.clone() {
                            if !column.set_cell_text(row, &text) {
                                not_understood += 1;
                            }
                        }
                    }
                });
                if locked > 0 {
                    self.warn(format!(
                        "{locked} column(s) calculated from a formula were left alone; change the formula in Set Column Values"
                    ));
                } else if not_understood > 0 {
                    self.warn(format!("'{text}' is not a number; {not_understood} cell(s) were left empty"));
                }
            }
            CellOp::Mask(m) => {
                let mut changed = 0;
                self.doc.apply("Mask cells", |p| {
                    if let Some(ws) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet_index)) {
                        changed = ws.mask_cells(&t.columns, t.rows.clone(), m);
                    }
                });
                self.say(format!("{changed} cell(s) changed"));
            }
            CellOp::Insert | CellOp::Delete => self.insert_or_delete(book, sheet_index, &t, op == CellOp::Insert),
            CellOp::SetRangeEnd(which) => {
                let ids: Vec<ColumnId> = t.columns.iter().filter_map(|&i| sheet.columns.get(i).map(|c| c.id)).collect();
                let row = match which {
                    sheaf_core::data_range::RangeEnd::Begin => t.rows.start,
                    sheaf_core::data_range::RangeEnd::End => t.rows.end.saturating_sub(1),
                };
                let label = match which {
                    sheaf_core::data_range::RangeEnd::Begin => "Set as begin",
                    sheaf_core::data_range::RangeEnd::End => "Set as end",
                };
                match self.doc.edit(label, |p| p.set_data_range_end(&ids, which, row)) {
                    Ok(()) => self.say(format!(
                        "Row {} is the {} of {} column(s)",
                        row + 1,
                        if label == "Set as begin" { "first row" } else { "last row" },
                        ids.len()
                    )),
                    Err(e) => self.warn(e.to_string()),
                }
            }
            CellOp::Paste | CellOp::PasteTranspose => {
                // The system answers with the text in a later frame (an `Event::Paste`).
                self.paste_transposed = op == CellOp::PasteTranspose;
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
            CellOp::PasteText(text) => {
                let text = self.worksheet_clipboard.for_paste(text);
                let (col, row) = t.top_left;
                let transposed = std::mem::take(&mut self.paste_transposed);
                let label = if transposed { "Paste transpose" } else { "Paste" };
                match self.doc.edit(label, |p| {
                    if transposed {
                        p.paste_text_transposed(book, sheet_index, col, row, &text)
                    } else {
                        p.paste_text(book, sheet_index, col, row, &text)
                    }
                }) {
                    Ok(report) => {
                        let mut status = format!("Pasted {} row(s) of {} column(s)", report.rows, report.columns);
                        if report.columns_added > 0 {
                            status.push_str(&format!(", {} new column(s)", report.columns_added));
                        }
                        if report.locked_columns > 0 {
                            status.push_str(&format!(
                                "; {} column(s) calculated from a formula were left alone",
                                report.locked_columns
                            ));
                        }
                        if report.not_numbers > 0 {
                            status.push_str(&format!(
                                "; {} value(s) are not numbers and were left empty",
                                report.not_numbers
                            ));
                        }
                        self.say(status);
                        // The pasted block becomes the selection.
                        let project = self.doc.project();
                        if let Some(sheet) = project.workbook(book).and_then(|b| b.sheets.get(sheet_index))
                            && report.rows > 0
                            && let Some(state) = self.books.get_mut(&book)
                        {
                            let last = col + report.columns.max(1) - 1;
                            state.select_cells(sheet, (col, row), (last, row + report.rows - 1));
                        }
                    }
                    Err(e) => self.message = Some(("Could not paste".to_string(), e.to_string())),
                }
            }
        }
    }

    /// Edit > Insert and Edit > Delete on the selection: whole columns are inserted (left of the
    /// first selected one) or deleted; rows and blocks of cells insert blank cells or remove
    /// cells, moving the cells below them, in the columns the selection covers.
    fn insert_or_delete(&mut self, book: WindowId, sheet_index: usize, t: &grid::Target, insert: bool) {
        let by_columns = self.books.get(&book).is_some_and(|s| s.sel == grid::Sel::Columns);
        if by_columns {
            let ids: Vec<ColumnId> = self
                .doc
                .project()
                .workbook(book)
                .and_then(|b| b.sheets.get(sheet_index))
                .map(|s| t.columns.iter().filter_map(|i| s.columns.get(*i).map(|c| c.id)).collect())
                .unwrap_or_default();
            if insert {
                let (at, count) = (t.columns.first().copied().unwrap_or(0), t.columns.len());
                let mut added = Vec::new();
                self.doc.apply("Insert columns", |p| {
                    added = p.insert_columns(book, sheet_index, at, count).unwrap_or_default();
                });
                if let Some(state) = self.books.get_mut(&book) {
                    state.selected = added;
                }
                self.say(format!("Inserted {count} column(s)"));
            } else {
                self.doc.apply("Delete columns", |p| delete_columns(p, &ids));
            }
            return;
        }
        let mut locked = 0;
        let rows = t.rows.clone();
        let label = if insert { "Insert rows" } else { "Delete rows" };
        self.doc.apply(label, |p| {
            if let Some(ws) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet_index)) {
                locked = if insert {
                    ws.insert_rows(&t.columns, rows.start, rows.len())
                } else {
                    ws.delete_rows(&t.columns, rows.clone())
                };
            }
            let _ = p.reapply_row_filter(book, sheet_index);
        });
        if locked > 0 {
            self.warn(format!("{locked} column(s) calculated from a formula were left alone"));
        } else {
            let what = if insert { "Inserted" } else { "Deleted" };
            self.say(format!("{what} {} row(s) in {} column(s)", rows.len(), t.columns.len()));
        }
    }

    fn clear_cells(&mut self, book: WindowId, sheet: usize, t: &grid::Target, label: &str) {
        let mut locked = 0;
        self.doc.apply(label, |p| {
            if let Some(ws) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
                locked = ws.clear_cells(&t.columns, t.rows.clone());
            }
        });
        if locked > 0 {
            self.warn(format!("{locked} column(s) calculated from a formula were left alone"));
        }
    }

    /// Adds `count` Y columns at the end of the workbook's active sheet in one undo step (T305).
    fn add_columns(&mut self, book: WindowId, count: usize) {
        let label = if count == 1 { "Add column" } else { "Add columns" };
        self.doc.apply(label, |p| {
            let sheet = p.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
            for _ in 0..count {
                let _ = p.add_column(book, sheet, Designation::Y, ColumnData::default());
            }
        });
    }

    /// Add, insert, delete, duplicate and rename sheets of a workbook (the sheet tab's menu).
    fn sheet_op(&mut self, book: WindowId, op: SheetOp) {
        use sheaf_core::Worksheet;
        let Some(b) = self.doc.project().workbook(book) else {
            return;
        };
        let active = b.active_sheet.min(b.sheets.len().saturating_sub(1));
        let free_name = |sheets: &[Worksheet], base: &str| -> String {
            let mut n = sheets.len() + 1;
            loop {
                let name = format!("{base}{n}");
                if !sheets.iter().any(|s| s.name == name) {
                    return name;
                }
                n += 1;
            }
        };
        match op {
            SheetOp::Add | SheetOp::Insert => {
                let at = if op == SheetOp::Add { b.sheets.len() } else { active };
                let name = free_name(&b.sheets, "Sheet");
                self.doc.apply(if op == SheetOp::Add { "Add sheet" } else { "Insert sheet" }, |p| {
                    if let Some(b) = p.workbook_mut(book) {
                        b.sheets.insert(at, Worksheet::new(name));
                        b.active_sheet = at;
                    }
                    // As a new workbook's sheet (T212): columns A(X) and B(Y) to type into.
                    let _ = p.add_column(book, at, Designation::X, ColumnData::default());
                    let _ = p.add_column(book, at, Designation::Y, ColumnData::default());
                });
                if let Some(state) = self.books.get_mut(&book) {
                    state.clear_selection();
                }
            }
            SheetOp::Delete => {
                if b.sheets.len() < 2 {
                    self.warn("A workbook keeps at least one sheet".to_string());
                    return;
                }
                let ids: Vec<ColumnId> = b.sheets[active].columns.iter().map(|c| c.id).collect();
                self.doc.apply("Delete sheet", |p| {
                    delete_columns(p, &ids);
                    if let Some(b) = p.workbook_mut(book) {
                        b.sheets.remove(active);
                        b.active_sheet = active.min(b.sheets.len() - 1);
                    }
                });
                if let Some(state) = self.books.get_mut(&book) {
                    state.clear_selection();
                }
            }
            SheetOp::Duplicate => {
                self.doc.apply("Duplicate sheet", |p| duplicate_sheet(p, book, active));
                if let Some(state) = self.books.get_mut(&book) {
                    state.clear_selection();
                }
            }
            SheetOp::DuplicateWithoutData => {
                match self.doc.edit("Duplicate sheet without data", |p| p.duplicate_sheet_without_data(book, active)) {
                    Ok(_) => {
                        if let Some(state) = self.books.get_mut(&book) {
                            state.clear_selection();
                        }
                    }
                    Err(e) => self.warn(e.to_string()),
                }
            }
            SheetOp::ActivateInFolder => {
                // T277: the books of the folder the book is in (not its subfolders), the book too.
                let project = self.doc.project();
                let folder = crate::panels::explorer::home(&project.root, book);
                let books: Vec<WindowId> = crate::panels::explorer::windows_in(project, &folder)
                    .into_iter()
                    .filter(|id| project.workbook(*id).is_some())
                    .collect();
                let name = b.sheets[active].name.clone();
                let mut changed = Vec::new();
                self.doc.adjust_view(|p| changed = p.activate_sheet_named(&books, &name));
                for id in &changed {
                    if let Some(state) = self.books.get_mut(id) {
                        state.clear_selection();
                    }
                }
                let without = books.iter().filter(|id| {
                    self.doc
                        .project()
                        .workbook(**id)
                        .is_some_and(|b| !b.sheets.iter().any(|s| s.name.eq_ignore_ascii_case(&name)))
                });
                let without = without.count();
                let mut msg = format!("Sheet {name} is active in {} more workbook(s) of the folder", changed.len());
                if without > 0 {
                    msg.push_str(&format!("; {without} workbook(s) have no sheet of that name"));
                }
                self.say(msg);
            }
            SheetOp::Navigate => {
                self.sheet_navigate = Some(crate::dialogs::sheet_navigate::SheetNavigateDialog::new(book, active));
            }
            SheetOp::ToggleView(switch) => {
                self.doc.apply("Worksheet view", |p| {
                    if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(active)) {
                        s.toggle_view_switch(switch);
                    }
                });
            }
            SheetOp::ShowName(label) => {
                if b.sheets[active].tab_label != label {
                    self.doc.apply("Sheet name to show", |p| {
                        if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(active)) {
                            s.tab_label = label;
                        }
                    });
                }
            }
            SheetOp::AddGraph => {
                let project = self.doc.project();
                let graphs: Vec<(WindowId, String)> =
                    project.graphs().map(|g| (g.id, Window::Graph(g.clone()).title())).collect();
                self.graph_sheet_picker = Some(crate::dialogs::graph_sheet::GraphSheetPicker::new(
                    crate::dialogs::graph_sheet::PickTarget::Sheet(book),
                    graphs,
                ));
            }
            SheetOp::AddParamRow => self.param_row = Some((book, active, String::new(), None)),
            SheetOp::AddNotes => {
                self.doc.apply("Add Notes as Sheet", |p| {
                    if let Some(b) = p.workbook_mut(book) {
                        b.add_notes_sheet();
                    }
                });
                if let Some(state) = self.books.get_mut(&book) {
                    state.clear_selection();
                }
            }
            SheetOp::RenameDialog => {
                let sheet = &b.sheets[active];
                self.sheet_name = Some(SheetNameDialog {
                    book,
                    sheet: active,
                    name: sheet.name.clone(),
                    long_name: sheet.long_name.clone(),
                    comments: sheet.comments.clone(),
                });
            }
        }
    }

    fn open_find(&mut self, book: WindowId, replacing: bool) {
        let text = self
            .books
            .get(&book)
            .and_then(|state| state.active_cell())
            .and_then(|(column, row)| {
                let workbook = self.doc.project().workbook(book)?;
                workbook.sheets.get(workbook.active_sheet)?.columns.get(column).map(|column| column.shown_text(row))
            })
            .unwrap_or_default();
        let mut dialog = if replacing { FindDialog::replace(book) } else { FindDialog::new(book) };
        if let Some(selection) = self.books.get(&book).and_then(|state| {
            if matches!(state.sel, Sel::Cells { anchor, end } if anchor == end) {
                return None;
            }
            let workbook = self.doc.project().workbook(book)?;
            state.target(workbook.sheets.get(workbook.active_sheet)?)
        }) {
            dialog = dialog.in_selection(selection.columns, selection.rows);
        }
        self.find = Some(dialog.initial_text(text));
    }

    fn open_go_to(&mut self, book: WindowId) {
        let can_extend = self
            .books
            .get(&book)
            .is_some_and(|b| matches!(b.sel, Sel::Cells { .. } | Sel::Columns | Sel::Rows { .. } | Sel::All));
        if let Some(sheet) = self.doc.project().workbook(book).and_then(|b| b.sheets.get(b.active_sheet)) {
            let state = self.books.entry(book).or_default();
            let current = state.target(sheet).map(|t| t.top_left);
            self.go_to = Some(GoToDialog::new(book, can_extend, state.row_capacity(sheet), current));
        }
    }

    /// The columns a plot command takes: the numeric Y columns among the selected ones, or the
    /// columns a plot of the active sheet would show when none is selected.
    fn plot_y_columns(&self, book: WindowId, columns: Vec<ColumnId>, template: Template) -> Vec<ColumnId> {
        let project = self.doc.project();
        if columns.is_empty() {
            let sheet = project.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
            return project.default_plot_columns(book, sheet, template);
        }
        columns
            .into_iter()
            .filter(|id| {
                project.column(*id).is_some_and(|c| {
                    c.data.is_numeric()
                        && matches!(c.designation, Designation::Y | Designation::Z | Designation::Disregard)
                })
            })
            .collect()
    }

    fn plot_columns(&mut self, book: WindowId, columns: Vec<ColumnId>, template: Template) {
        // An UpSet plot (T403): the selected columns are the sets (else the 0/1 columns of the active sheet).
        if template == Template::UpSet {
            let project = self.doc.project();
            let sets = if columns.len() >= 2 {
                columns
            } else {
                project
                    .workbook(book)
                    .and_then(|b| b.sheets.get(b.active_sheet))
                    .map(sheaf_core::project::flag_columns)
                    .unwrap_or_default()
            };
            match self.doc.edit("Plot columns", |p| p.plot_upset(&sets)) {
                Ok(_) => self.say(format!("UpSet plot of {} sets", sets.len())),
                Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
            }
            return;
        }
        // A network plot (T383) or a chord diagram (T402): the first two selected columns of text are the ends of
        // the edges (else the first two of the active sheet), the last selected numeric one weighs them (else every
        // row counts 1).
        if matches!(template, Template::Network | Template::Chord) {
            let project = self.doc.project();
            let is_text = |id: &ColumnId| project.column(*id).is_some_and(|c| !c.data.is_numeric());
            let mut ends: Vec<ColumnId> = columns.iter().copied().filter(is_text).collect();
            if ends.len() < 2 {
                ends = project
                    .workbook(book)
                    .and_then(|b| b.sheets.get(b.active_sheet))
                    .map(|s| s.columns.iter().filter(|c| !c.data.is_numeric()).map(|c| c.id).collect())
                    .unwrap_or_default();
            }
            let weight = columns.iter().rev().copied().find(|id| project.numeric(*id).is_some());
            let made = match ends[..] {
                [source, target, ..] if template == Template::Chord => {
                    self.doc.edit("Plot columns", |p| p.plot_chord(source, target, weight))
                }
                [source, target, ..] => self.doc.edit("Plot columns", |p| p.plot_network(source, target, weight)),
                _ => Err(sheaf_core::project::ProjectError::NetworkColumns),
            };
            match made {
                Ok(_) => self.say(template.label().to_string()),
                Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
            }
            return;
        }
        // An alluvial diagram (T381): the selected columns of text are the stages (else every text column of the
        // active sheet), the last selected numeric one weights the rows (else every row counts 1).
        if template == Template::Alluvial {
            let project = self.doc.project();
            let is_text = |id: &ColumnId| project.column(*id).is_some_and(|c| !c.data.is_numeric());
            let mut stages: Vec<ColumnId> = columns.iter().copied().filter(is_text).collect();
            if stages.len() < 2 {
                stages = project
                    .workbook(book)
                    .and_then(|b| b.sheets.get(b.active_sheet))
                    .map(|s| s.columns.iter().filter(|c| !c.data.is_numeric()).map(|c| c.id).collect())
                    .unwrap_or_default();
            }
            let value = columns.iter().rev().copied().find(|id| project.numeric(*id).is_some());
            let n = stages.len();
            match self.doc.edit("Plot columns", |p| p.plot_alluvial(&stages, value)) {
                Ok(_) => self.say(format!("Alluvial diagram of {n} stages")),
                Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
            }
            return;
        }
        // A cause-and-effect diagram (T375): the selected columns, of any kind, are the categories (else every
        // column of the active sheet); the effect text is edited on the graph afterwards.
        if template == Template::Fishbone {
            let categories: Vec<ColumnId> = if columns.is_empty() {
                let project = self.doc.project();
                project
                    .workbook(book)
                    .and_then(|b| b.sheets.get(b.active_sheet))
                    .map(|s| s.columns.iter().map(|c| c.id).collect())
                    .unwrap_or_default()
            } else {
                columns
            };
            let n = categories.len();
            match self.doc.edit("Plot columns", |p| p.plot_fishbone(&categories, sheaf_core::fishbone::DEFAULT_EFFECT))
            {
                Ok(_) => self.say(format!("Cause-and-effect diagram of {n} categories")),
                Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
            }
            return;
        }
        // A Parallel Index is grouped by the selected column of text (else the first text column of the sheet).
        let group = if template == Template::ParallelIndex {
            columns.iter().copied().find(|id| self.doc.project().column(*id).is_some_and(|c| !c.data.is_numeric()))
        } else {
            None
        };
        // A Treemap (or Sunburst plot) nests by the selected columns of text (in selection order; else every text column of the
        // sheet) and sizes by the last numeric one.
        let categories: Vec<ColumnId> = if matches!(template, Template::Treemap | Template::Sunburst | Template::Sankey)
        {
            let project = self.doc.project();
            columns.iter().copied().filter(|id| project.column(*id).is_some_and(|c| !c.data.is_numeric())).collect()
        } else {
            Vec::new()
        };
        // An X bar R chart asks for the subgroup size of the one selected numeric column first (T111).
        if template == Template::QcXbarR {
            let project = self.doc.project();
            let numeric: Vec<ColumnId> =
                columns.iter().copied().filter(|id| project.column(*id).is_some_and(|c| c.data.is_numeric())).collect();
            match numeric[..] {
                [column] => self.qc_chart = crate::dialogs::qc_chart::QcDialog::new(project, column),
                _ => self.warn("Select one numeric column for the X bar R chart".to_string()),
            }
            return;
        }
        // A scatter matrix takes every selected column, whatever its designation (else the sheet's numeric Y
        // columns), refuses columns of text, and asks for its options first (T114).
        if template == Template::ScatterMatrix {
            let project = self.doc.project();
            if columns.iter().any(|id| project.column(*id).is_some_and(|c| !c.data.is_numeric())) {
                self.warn("A scatter matrix plots numbers: leave the text columns out of the selection".to_string());
                return;
            }
            let numeric: Vec<ColumnId> =
                if columns.is_empty() { self.plot_y_columns(book, columns, template) } else { columns };
            let max = sheaf_core::scatter_matrix::MAX_VARIABLES;
            if (2..=max).contains(&numeric.len()) {
                self.scatter_matrix = Some(crate::dialogs::scatter_matrix::ScatterMatrixDialog::new(project, numeric));
            } else {
                self.warn(format!("Select 2 to {max} numeric columns for the scatter matrix"));
            }
            return;
        }
        // A Pareto chart counts the cells of the one selected column, text or numbers (Origin's Raw Data); with
        // two columns selected, the first holds the categories and the second their counts (Binned Data, T319).
        if template == Template::Pareto {
            if let [data, counts] = columns[..] {
                match self.doc.edit("Plot columns", |p| p.plot_pareto_binned(data, counts)) {
                    Ok(_) => self.say("Plotted a Pareto chart of binned data".to_string()),
                    Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
                }
                return;
            }
            let [column] = columns[..] else {
                self.warn("Select one column whose values are counted, or a category and a count column, for the Pareto chart".to_string());
                return;
            };
            match self.doc.edit("Plot columns", |p| p.plot_pareto(column)) {
                Ok(_) => self.say("Plotted a Pareto chart".to_string()),
                Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
            }
            return;
        }
        // Grouped columns / grouped box charts of indexed data: the one numeric Y column is the data, the other
        // selected columns (one or two) are the groups, in selection order.
        if matches!(template, Template::GroupedColumns | Template::GroupedBox) {
            let project = self.doc.project();
            let is_data = |id: &ColumnId| {
                project.column(*id).is_some_and(|c| c.data.is_numeric() && c.designation == Designation::Y)
            };
            let data = columns.iter().filter(|id| is_data(id)).count();
            let groups = columns.len() - data;
            if data != 1 || groups == 0 {
                self.warn("Select one or two group columns and one data column (a numeric group column must be designated X or Group)".to_string());
                return;
            }
            match self.doc.edit("Plot columns", |p| p.plot_columns(template, &columns)) {
                Ok(_) => self.say(format!("Plotted {} by {} group column(s)", data, groups)),
                Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
            }
            return;
        }
        let ys = self.plot_y_columns(book, columns, template);
        // Text columns only: a nested frequency treemap or sunburst counts the rows (T392).
        if ys.is_empty() && matches!(template, Template::Treemap | Template::Sunburst) && !categories.is_empty() {
            match self.doc.edit("Plot columns", |p| p.plot_columns(template, &categories)) {
                Ok(_) => self.say(format!("Counted the rows of {} category column(s)", categories.len())),
                Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
            }
            return;
        }
        if ys.is_empty() {
            let what = if matches!(template, Template::Treemap | Template::Sunburst | Template::Sankey) {
                "Nothing to plot: select one or more text category columns and a numeric value column"
            } else {
                "Nothing to plot: select one or more numeric Y columns"
            };
            self.warn(what.to_string());
            return;
        }
        let made = self.doc.edit("Plot columns", |p| match group {
            Some(g) => p.plot_parallel(template, &ys, Some(g)),
            None if matches!(template, Template::Treemap | Template::Sunburst | Template::Sankey) => {
                p.plot_columns(template, &[categories.as_slice(), &ys].concat())
            }
            None => p.plot_columns(template, &ys).inspect(|g| sheaf_plot::place_zoom_regions(p, *g)),
        });
        match made {
            Ok(_) => self.say(format!("Plotted {} column(s)", ys.len())),
            Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
        }
    }

    /// Plot > User Templates > a saved template: the selected Y columns in its look (T56,
    /// graphing.md 24). The template is read from its file at the time of the click.
    /// File > New > From Template (T103): an empty graph in a user template's look, one undo step.
    fn new_from_template(&mut self, name: Option<String>) {
        let template = match name {
            Some(name) => {
                let Some(dir) = self.startup.templates_dir.clone() else { return };
                sheaf_io::user_templates::load(&dir, &name).map_err(|e| e.to_string())
            }
            None => {
                use sheaf_io::user_templates::{BOOK_EXTENSION, EXTENSION, TemplateKind};
                let mut chooser = rfd::FileDialog::new()
                    .set_title("New from template")
                    .add_filter("Sheaf template", &[EXTENSION, BOOK_EXTENSION])
                    .add_filter("Sheaf graph template", &[EXTENSION])
                    .add_filter("Sheaf workbook template", &[BOOK_EXTENSION]);
                if let Some(dir) = self.startup.templates_dir.as_deref().filter(|d| d.is_dir()) {
                    chooser = chooser.set_directory(dir);
                }
                let Some(path) = chooser.pick_file() else { return };
                if sheaf_io::user_templates::kind_of_file(&path) == Some(TemplateKind::Workbook) {
                    match sheaf_io::user_templates::load_book_file(&path) {
                        Ok(t) => self.new_book_from_template(&t),
                        Err(e) => self.message = Some(("Could not read the template".to_string(), e.to_string())),
                    }
                    return;
                }
                sheaf_io::user_templates::load_file(&path).map_err(|e| e.to_string())
            }
        };
        let template = match template {
            Ok(t) => t,
            Err(e) => {
                self.message = Some(("Could not read the template".to_string(), e));
                return;
            }
        };
        match self.doc.edit("New graph from template", |p| p.plot_with_user_template(&template, &[])) {
            Ok(id) => {
                self.focus(id);
                self.say(format!("New graph from the template {}", template.name));
            }
            Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
        }
    }

    /// File > New > From Template with a workbook template (T160): an empty workbook built like the
    /// template, one undo step.
    fn new_book_from_template(&mut self, template: &sheaf_core::BookTemplate) {
        let id = self.doc.apply("New workbook from template", |p| p.new_workbook_from_template(template));
        self.focus(id);
        self.say(format!("New workbook from the template {}", template.name));
    }

    fn plot_user_template(&mut self, book: WindowId, columns: Vec<ColumnId>, name: &str) {
        let Some(dir) = self.startup.templates_dir.clone() else { return };
        let template = match sheaf_io::user_templates::load(&dir, name) {
            Ok(t) => t,
            Err(e) => {
                self.message = Some(("Could not read the template".to_string(), e.to_string()));
                return;
            }
        };
        let ys = self.plot_y_columns(book, columns, Template::Line);
        if ys.is_empty() {
            self.warn("Nothing to plot: select one or more numeric Y columns".to_string());
            return;
        }
        match self.doc.edit("Plot columns", |p| p.plot_with_user_template(&template, &ys)) {
            Ok(_) => self.say(format!("Plotted {} column(s) with the template {}", ys.len(), template.name)),
            Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
        }
    }

    /// After the active folder or View Windows changed: when the active window is no longer shown, the
    /// most recent window of the folder comes forward (as Origin brings the folder's window up).
    fn follow_folder(&mut self) {
        let Some(shown) = explorer::shown_windows(self.doc.project(), &self.panels.explorer.folder) else { return };
        if self.active.is_some_and(|a| shown.contains(&a)) {
            return;
        }
        let recent = self.workspace.recent_order(&shown);
        if let Some(id) = recent.into_iter().find(|id| self.workspace.contains(*id)) {
            self.focus(id);
        }
    }

    /// Save Format as Theme and the Theme Organizer (T102).
    fn show_graph_themes(&mut self, ctx: &egui::Context) {
        use crate::dialogs::graph_themes::{OrganizerOutcome, SaveThemeOutcome};
        let dir = self.startup.graph_themes_dir.clone();
        if let Some((d, clip)) = &mut self.save_theme {
            match d.show(ctx) {
                SaveThemeOutcome::Open => {}
                SaveThemeOutcome::Closed => self.save_theme = None,
                SaveThemeOutcome::Save { name, description } => {
                    let theme = sheaf_io::graph_themes::GraphTheme {
                        name,
                        description,
                        saved: String::new(),
                        format: clip.clone(),
                    };
                    match dir.as_deref().map(|dir| sheaf_io::graph_themes::save(dir, theme)) {
                        Some(Ok(path)) => {
                            self.save_theme = None;
                            self.say(format!("Saved the theme {}", path.display()));
                        }
                        Some(Err(e)) => d.error = Some(e),
                        None => self.save_theme = None,
                    }
                }
            }
        }
        let Some(o) = &mut self.theme_organizer else { return };
        match o.show(ctx) {
            OrganizerOutcome::Open => {}
            OrganizerOutcome::Closed => self.theme_organizer = None,
            OrganizerOutcome::Delete(name) => {
                if let Some(dir) = dir.as_deref() {
                    sheaf_io::graph_themes::delete(dir, &name);
                    o.entries = sheaf_io::graph_themes::list(dir);
                    o.selected = None;
                }
            }
            OrganizerOutcome::Apply(name) => {
                let Some(graph) = self.active.filter(|a| self.doc.project().graph(*a).is_some()) else { return };
                let theme = dir
                    .as_deref()
                    .ok_or_else(|| "no theme folder".to_string())
                    .and_then(|d| sheaf_io::graph_themes::load(d, &name));
                match theme {
                    Ok(t) => {
                        self.doc.apply("Apply theme", |p| {
                            if let Some(g) = p.graph_mut(graph) {
                                apply_graph_theme(g, &t.format);
                            }
                        });
                        self.say(format!("Applied the theme {name}"));
                    }
                    Err(e) => self.warn(e),
                }
            }
        }
    }

    /// Folder Properties: OK renames the folder and keeps its comments, one undo step.
    fn show_folder_props(&mut self, ctx: &egui::Context) {
        use crate::dialogs::folder_props::FolderOutcome;
        let Some(d) = &mut self.folder_props else { return };
        match d.show(ctx, self.doc.project()) {
            FolderOutcome::Open => {}
            FolderOutcome::Closed => self.folder_props = None,
            FolderOutcome::Set { path, name, comments } => {
                self.folder_props = None;
                self.doc.apply("Folder properties", |p| {
                    if let Some(f) = explorer::folder_at_mut(&mut p.root, &path) {
                        if !path.is_empty() && !name.trim().is_empty() {
                            f.name = name.trim().to_string();
                        }
                        f.comments = comments;
                    }
                });
            }
        }
    }

    /// Duplicate (Batch Plotting): OK makes the copies, one undo step, and brings the last forward.
    /// File > Open Sample Projects (T133): Open closes the window and opens the chosen sample.
    fn show_sample_projects(&mut self, ctx: &egui::Context) {
        use crate::dialogs::sample_projects::SampleOutcome;
        let Some(d) = &mut self.sample_projects else { return };
        match d.show(ctx) {
            SampleOutcome::Open => {}
            SampleOutcome::Closed => self.sample_projects = None,
            SampleOutcome::Load(name) => {
                self.sample_projects = None;
                self.apply(Action::OpenSample(name), ctx);
            }
        }
    }

    /// Tools > Fitting Function Builder (T132): Save writes `<name>.FDF` into the fitting-function folder.
    fn show_fit_builder(&mut self, ctx: &egui::Context) {
        use crate::dialogs::fit_builder::BuilderOutcome;
        let Some(b) = &mut self.fit_builder else { return };
        match b.show(ctx) {
            BuilderOutcome::Open => {}
            BuilderOutcome::Closed => self.fit_builder = None,
            BuilderOutcome::Save { name, text } => {
                let Some(dir) = self.startup.fit_functions_dir.clone() else {
                    self.warn("Sheaf has no folder for fitting functions".to_string());
                    return;
                };
                let path = dir.join(format!("{name}.FDF"));
                let written = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text));
                match written {
                    Ok(()) => self.say(format!(
                        "Saved {}; Analysis > Fitting > Nonlinear Curve Fit loads it with the .FDF category",
                        path.display()
                    )),
                    Err(e) => self.warn(format!("Could not save {}: {e}", path.display())),
                }
            }
        }
    }

    /// Tools > Protection > Audit Log (T131): the switch and the list of logged changes.
    fn show_audit_log(&mut self, ctx: &egui::Context) {
        if !self.audit_log {
            return;
        }
        let mut open = true;
        let mut on = self.doc.project().audit.is_some();
        let was = on;
        egui::Window::new("Audit Log").id(egui::Id::new("audit_log")).collapsible(false).open(&mut open).show(
            ctx,
            |ui| {
                ui.checkbox(&mut on, "Record the changes made to this project");
                ui.separator();
                match &self.doc.project().audit {
                    None => {
                        ui.weak("The audit log is off.");
                    }
                    Some(log) => {
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            egui::Grid::new("audit_log_grid").striped(true).num_columns(2).show(ui, |ui| {
                                ui.strong("Time (UTC)");
                                ui.strong("Change");
                                ui.end_row();
                                for e in &log.entries {
                                    ui.label(sheaf_core::audit::format_time(e.time));
                                    ui.label(&e.change);
                                    ui.end_row();
                                }
                            });
                        });
                    }
                }
            },
        );
        if on != was {
            self.doc.set_audit(on);
        }
        if !open {
            self.audit_log = false;
        }
    }

    /// Graph > Add Function Graph... (T119): OK adds the curve; an expression error keeps the window open.
    fn show_auto_refresh(&mut self, ctx: &egui::Context) {
        use crate::dialogs::auto_refresh::RefreshOutcome;
        let Some(d) = &mut self.auto_refresh else { return };
        match d.show(ctx) {
            RefreshOutcome::Open => {}
            RefreshOutcome::Closed => self.auto_refresh = None,
            RefreshOutcome::Set { book, refresh } => {
                self.auto_refresh = None;
                // A new setting starts its timer afresh.
                self.refresh_clock.remove(&book);
                match self.doc.edit("Auto Refresh", |p| p.set_auto_refresh(book, refresh)) {
                    Ok(()) => self.say(match refresh {
                        Some(r) if r.paused => "Auto refresh is paused".to_string(),
                        Some(r) => format!("The workbook re-imports its file every {} s", r.every_seconds),
                        None => "Auto refresh stopped".to_string(),
                    }),
                    Err(e) => self.warn(e.to_string()),
                }
            }
        }
    }

    /// Auto Refresh (T442) at `now` (seconds of the UI clock): re-imports each refreshing workbook whose check is
    /// due, when its file changed (or always, as set). A failed read pauses the book's refresh. Returns the
    /// seconds until the next check, when any book refreshes.
    pub(crate) fn run_auto_refresh(&mut self, now: f64) -> Option<f64> {
        let books = self.doc.project().refreshing_books();
        self.refresh_clock.retain(|id, _| books.iter().any(|(b, _)| b == id));
        let modified = |project: &sheaf_core::Project, book: WindowId| {
            let path = project.workbook(book)?.source.as_ref()?.path.clone();
            std::fs::metadata(path).and_then(|m| m.modified()).ok()
        };
        let mut due = Vec::new();
        for &(book, r) in &books {
            let every = f64::from(r.every_seconds);
            let mtime = modified(self.doc.project(), book);
            let clock = self.refresh_clock.entry(book).or_insert((now + every, mtime));
            if now >= clock.0 {
                clock.0 = now + every;
                if !r.only_when_changed || mtime != clock.1 {
                    clock.1 = mtime;
                    due.push((book, r));
                }
            }
        }
        for (book, r) in due {
            let Some(b) = self.doc.project().workbook(book) else { continue };
            let Some((source, kind)) = b.source.clone().zip(reimport_kind(b)) else { continue };
            match reimport_book(&mut self.doc, book, Path::new(&source.path), kind) {
                Ok(rep) => self.say(format!("Auto refresh: re-imported {} ({} row(s))", source.path, rep.rows)),
                Err(e) => {
                    let paused = Some(sheaf_core::auto_refresh::AutoRefresh { paused: true, ..r });
                    let _ = self.doc.edit("Auto Refresh", |p| p.set_auto_refresh(book, paused));
                    self.warn(format!("Auto refresh paused: could not read {}: {e}", source.path));
                }
            }
        }
        self.refresh_clock.values().map(|c| c.0 - now).reduce(f64::min)
    }

    fn show_op_params(&mut self, ctx: &egui::Context) {
        use crate::dialogs::op_params::ParamsOutcome;
        let Some(d) = &mut self.op_params else { return };
        match d.show(ctx) {
            ParamsOutcome::Open => {}
            ParamsOutcome::Closed => self.op_params = None,
            ParamsOutcome::Apply { index, tool } => {
                match self.doc.edit("Operation Parameters", |p| p.set_operation_tool(index, *tool)) {
                    Ok(()) => {
                        self.op_params = None;
                        self.say("The operation ran again with its new settings".to_string());
                    }
                    Err(e) => {
                        if let Some(d) = &mut self.op_params {
                            d.error = Some(e.to_string());
                        }
                    }
                }
            }
        }
    }

    fn show_project_browser(&mut self, ctx: &egui::Context) {
        use crate::dialogs::project_browser::BrowserOutcome;
        let Some(d) = &mut self.project_browser else { return };
        match d.show(ctx, self.doc.project()) {
            BrowserOutcome::Open => {}
            BrowserOutcome::Closed => self.project_browser = None,
            // The browser stays open, as Find in Project does.
            BrowserOutcome::GoTo(target) => self.go_to_found(target, ctx),
            BrowserOutcome::Set { items, field, value } => {
                let label = format!("Set {}", field.name());
                let n = self.doc.apply(&label, |p| p.set_metadata(&items, field, &value));
                self.say(format!("Set the {} of {n} item(s)", field.name()));
            }
        }
    }

    fn show_flash_fill(&mut self, ctx: &egui::Context) {
        use crate::dialogs::flash_fill::FlashOutcome;
        let Some(d) = &mut self.flash_fill else { return };
        match d.show(ctx) {
            FlashOutcome::Open => {}
            FlashOutcome::Closed => self.flash_fill = None,
            FlashOutcome::Fill { column, fill } => {
                self.flash_fill = None;
                match self.doc.edit("Flash Fill", |p| p.apply_flash_fill(column, &fill)) {
                    Ok(()) => self.say("Flash Fill filled the column".to_string()),
                    Err(e) => self.warn(e.to_string()),
                }
            }
            FlashOutcome::Formula { column, formula } => {
                self.flash_fill = None;
                let mode = sheaf_core::column::RecalcMode::Auto;
                match self.doc.edit("Flash Fill", |p| p.set_column_values_with(column, &formula, None, mode)) {
                    Ok(()) => self.say(format!("Flash Fill set the formula {formula}")),
                    Err(e) => self.warn(e.to_string()),
                }
            }
        }
    }

    fn show_browser_graph(&mut self, ctx: &egui::Context) {
        use crate::dialogs::browser_graph::BrowserOutcome;
        let Some(d) = &mut self.browser_graph else { return };
        match d.show(ctx) {
            BrowserOutcome::Open => {}
            BrowserOutcome::Closed => self.browser_graph = None,
            BrowserOutcome::Apply { graph, layer, plot, by } => {
                match self.doc.edit("Browser Graph", |p| p.set_graph_browser(graph, layer, plot, by)) {
                    Ok(n) => {
                        self.browser_graph = None;
                        self.say(format!("The graph browses {n} datasets"));
                    }
                    Err(e) => self.warn(e.to_string()),
                }
            }
            BrowserOutcome::Remove { graph } => {
                self.browser_graph = None;
                let _ = self
                    .doc
                    .edit("Remove Browser", |p| Ok::<_, std::convert::Infallible>(p.clear_graph_browser(graph)));
                self.say("Removed the graph browser".to_string());
            }
        }
    }

    fn show_graph_slicer(&mut self, ctx: &egui::Context) {
        use crate::dialogs::graph_slicer::SlicerOutcome;
        let Some(d) = &mut self.graph_slicer else { return };
        match d.show(ctx, self.doc.project()) {
            SlicerOutcome::Open => {}
            SlicerOutcome::Closed => self.graph_slicer = None,
            SlicerOutcome::Apply { graph, column, values, layer, plot, extract } => {
                let label = if extract { "Extract Slice" } else { "Graph Slicer" };
                let done = self.doc.edit(label, |p| {
                    p.set_graph_slicer(graph, column, values)?;
                    p.set_graph_slicer_layer(graph, layer)?;
                    p.set_graph_slicer_plot(graph, plot)?;
                    if extract { p.extract_slice(graph).map(Some) } else { Ok(None) }
                });
                match done {
                    Ok(book) => {
                        self.graph_slicer = None;
                        match book {
                            Some(b) => {
                                self.apply(Action::Focus(b), ctx);
                                self.say("Copied the rows the slicer shows to a new workbook".to_string());
                            }
                            None => self.say("Sliced the graph".to_string()),
                        }
                    }
                    Err(e) => self.warn(e.to_string()),
                }
            }
            SlicerOutcome::Remove { graph } => {
                self.graph_slicer = None;
                let _ = self
                    .doc
                    .edit("Remove Graph Slicer", |p| Ok::<_, std::convert::Infallible>(p.clear_graph_slicer(graph)));
                self.say("Removed the graph slicer".to_string());
            }
        }
    }

    fn show_function_graph(&mut self, ctx: &egui::Context) {
        use crate::dialogs::function_graph::FunctionOutcome;
        let Some(d) = &mut self.function_graph else { return };
        let FunctionOutcome::Add { graph, layer, expr, from, to, points, kind } = (match d.show(ctx) {
            FunctionOutcome::Open => return,
            FunctionOutcome::Closed => {
                self.function_graph = None;
                self.function_graph_new = false;
                return;
            }
            add => add,
        }) else {
            return;
        };
        let new = std::mem::take(&mut self.function_graph_new);
        let label = if new { "New function plot" } else { "Add function graph" };
        let made = self.doc.edit(label, |p| -> Result<WindowId, sheaf_core::project::ProjectError> {
            let graph = if new { p.add_graph(sheaf_core::template::Template::default()) } else { graph };
            p.add_function_graph(graph, layer, &expr, (from, to), points)?;
            if let Some(plot) =
                p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.plots.last_mut())
            {
                plot.kind = kind;
            }
            Ok(graph)
        });
        match made {
            Ok(graph) => {
                self.function_graph = None;
                if new {
                    self.focus(graph);
                }
                self.say(format!("Added y = {expr}"));
            }
            Err(e) => {
                self.function_graph_new = new;
                self.warn(e.to_string());
            }
        }
    }

    /// Plot > Statistics > QC (X bar R) Chart's subgroup prompt (T111).
    fn show_qc_chart(&mut self, ctx: &egui::Context) {
        use crate::dialogs::qc_chart::QcOutcome;
        let Some(d) = &mut self.qc_chart else { return };
        let (column, size) = match d.show(ctx) {
            QcOutcome::Open => return,
            QcOutcome::Closed => {
                self.qc_chart = None;
                return;
            }
            QcOutcome::Make { column, size } => (column, size),
        };
        self.qc_chart = None;
        match self.doc.edit("QC (X bar R) Chart", |p| p.plot_qc_xbar_r(column, size)) {
            Ok((_, graph)) => {
                self.focus(graph);
                self.say(format!("Made an X bar R chart in subgroups of {size}"));
            }
            Err(e) => self.warn(e.to_string()),
        }
    }

    /// Plot > Statistics > Run Chart's window (T270): OK makes the workbook and graph, one undo step.
    fn show_run_chart(&mut self, ctx: &egui::Context) {
        use crate::dialogs::run_chart::RunChartOutcome;
        let Some(d) = &mut self.run_chart else { return };
        let (column, size, stat, points) = match d.show(ctx) {
            RunChartOutcome::Open => return,
            RunChartOutcome::Closed => {
                self.run_chart = None;
                return;
            }
            RunChartOutcome::Make { column, size, stat, points } => (column, size, stat, points),
        };
        self.run_chart = None;
        match self.doc.edit("Run Chart", |p| p.plot_run_chart(column, size, stat, points)) {
            Ok((_, graph)) => {
                self.focus(graph);
                self.say(format!("Made a run chart in subgroups of {size}"));
            }
            Err(e) => self.warn(e.to_string()),
        }
    }

    /// Plot > Statistics > Grouped Histogram's window (T271): OK makes the workbook and graph, one undo step.
    fn show_grouped_hist(&mut self, ctx: &egui::Context) {
        use crate::dialogs::grouped_hist::GroupedHistOutcome;
        let Some(d) = &mut self.grouped_hist else { return };
        let (data, group, separate) = match d.show(ctx) {
            GroupedHistOutcome::Open => return,
            GroupedHistOutcome::Closed => {
                self.grouped_hist = None;
                return;
            }
            GroupedHistOutcome::Make { data, group, separate } => (data, group, separate),
        };
        self.grouped_hist = None;
        match self.doc.edit("Grouped Histogram", |p| p.plot_grouped_histogram(data, group, separate)) {
            Ok((_, graph)) => {
                self.focus(graph);
                self.say("Made a grouped histogram".to_string());
            }
            Err(e) => self.warn(e.to_string()),
        }
    }

    /// Plot > Statistics > Scatter Matrix's options (T114): OK makes the graph, one undo step.
    fn show_scatter_matrix(&mut self, ctx: &egui::Context) {
        use crate::dialogs::scatter_matrix::ScatterMatrixOutcome;
        let Some(d) = &mut self.scatter_matrix else { return };
        let (columns, options) = match d.show(ctx) {
            ScatterMatrixOutcome::Open => return,
            ScatterMatrixOutcome::Closed => {
                self.scatter_matrix = None;
                return;
            }
            ScatterMatrixOutcome::Make { columns, options } => (columns, options),
        };
        self.scatter_matrix = None;
        match self.doc.edit("Scatter Matrix", |p| p.plot_scatter_matrix(&columns, &options)) {
            Ok(graph) => {
                self.focus(graph);
                self.say(format!("Plotted a scatter matrix of {} columns", columns.len()));
            }
            Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
        }
    }

    fn show_batch_plot(&mut self, ctx: &egui::Context) {
        use crate::dialogs::batch_plot::BatchOutcome;
        let Some(d) = &mut self.batch_plot else { return };
        let made = match d.show(ctx) {
            BatchOutcome::Open => return,
            BatchOutcome::Closed => {
                self.batch_plot = None;
                return;
            }
            BatchOutcome::Sheets { graph, sheets, by } => {
                self.doc.edit("Duplicate (batch plotting)", |p| p.batch_plot_sheets(graph, &sheets, by))
            }
            BatchOutcome::Columns { graph, columns, associated_x } => {
                self.doc.edit("Duplicate (batch plotting)", |p| p.batch_plot_columns(graph, &columns, associated_x))
            }
        };
        self.batch_plot = None;
        match made {
            Ok(ids) => {
                for id in &ids {
                    self.focus(*id);
                }
                self.say(format!("Made {} graph(s)", ids.len()));
            }
            Err(e) => self.warn(e.to_string()),
        }
    }

    /// Activates what a Find in Project hit or a Project Browser row leads to: its folder, window, sheet or column.
    fn go_to_found(&mut self, target: crate::project_find::FindTarget, ctx: &egui::Context) {
        use crate::project_find::FindTarget;
        if let Some(window) = target.window_id() {
            let project = self.doc.project();
            if project.window(window).is_none() {
                return;
            }
            if explorer::shown_windows(project, &self.panels.explorer.folder)
                .is_some_and(|shown| !shown.contains(&window))
            {
                let folder = explorer::home(&project.root, window);
                self.apply(Action::GoToFolder(folder), ctx);
            }
            self.apply(Action::Focus(window), ctx);
        }
        match target {
            FindTarget::Folder(path) => self.apply(Action::GoToFolder(path), ctx),
            FindTarget::Window(_) => {}
            FindTarget::Sheet { book, sheet } => self.apply(Action::SelectSheet { book, sheet }, ctx),
            FindTarget::Column { book, sheet, column } => {
                self.apply(Action::SelectSheet { book, sheet }, ctx);
                if let Some(state) = self.books.get_mut(&book) {
                    state.sel = Sel::Columns;
                    state.selected = vec![column];
                }
            }
        }
    }

    /// Find in Project stays open when a hit activates its folder, window, sheet or column.
    fn show_find_project(&mut self, ctx: &egui::Context) {
        use crate::dialogs::find_project::FindProjectOutcome;
        let Some(d) = &mut self.find_project else { return };
        let outcome = d.show(ctx, self.doc.project(), &self.panels.explorer.folder);
        self.find_spec = d.spec.clone();
        match outcome {
            FindProjectOutcome::Open => {}
            FindProjectOutcome::Closed => self.find_project = None,
            FindProjectOutcome::Go(target) => self.go_to_found(target, ctx),
        }
    }

    /// Navigate Worksheets can activate a sheet or commit the displayed tab order.
    fn show_sheet_navigate(&mut self, ctx: &egui::Context) {
        use crate::dialogs::sheet_navigate::NavigateOutcome;
        let Some(mut dialog) = self.sheet_navigate.take() else { return };
        // Other dialogs may still hold worksheet indices from the current order.
        let can_reorder = !self.dialog_open();
        match dialog.show(ctx, self.doc.project(), can_reorder) {
            NavigateOutcome::Open => self.sheet_navigate = Some(dialog),
            NavigateOutcome::Closed => {}
            NavigateOutcome::Go { book, sheet } => {
                self.apply(Action::SelectSheet { book, sheet }, ctx);
            }
            NavigateOutcome::Reorder { book, order } => {
                if self.doc.apply("Reorder worksheets", |project| {
                    project.workbook_mut(book).is_some_and(|workbook| workbook.reorder_sheets(&order))
                }) {
                    dialog.reordered(&order);
                }
                self.sheet_navigate = Some(dialog);
            }
            NavigateOutcome::Act { book, sheets, action } => {
                self.navigate_action(book, &sheets, action, &mut dialog, ctx);
                self.sheet_navigate = Some(dialog);
            }
        }
    }

    /// An item of Navigate Worksheets' Actions menu (T180) on `sheets`; each is one undo step.
    fn navigate_action(
        &mut self,
        book: WindowId,
        sheets: &[usize],
        action: crate::dialogs::sheet_navigate::NavAction,
        dialog: &mut crate::dialogs::sheet_navigate::SheetNavigateDialog,
        ctx: &egui::Context,
    ) {
        use crate::dialogs::sheet_navigate::NavAction;
        let Some(b) = self.doc.project().workbook(book) else { return };
        let count = b.sheets.len();
        let sheets: Vec<usize> = sheets.iter().copied().filter(|&s| s < count).collect();
        if sheets.is_empty() {
            return;
        }
        match action {
            NavAction::Activate => {
                self.apply(Action::SelectSheet { book, sheet: sheets[0] }, ctx);
                dialog.reset(sheets[0]);
                return;
            }
            NavAction::Move(to) => {
                let order = b.moved_order(&sheets, to);
                if self.doc.apply("Move worksheets", |p| p.workbook_mut(book).is_some_and(|w| w.reorder_sheets(&order)))
                {
                    dialog.reordered(&order);
                }
                return;
            }
            NavAction::Duplicate => {
                // From the last, so that each copy lands right after its sheet.
                self.doc.apply("Duplicate sheets", |p| {
                    for &s in sheets.iter().rev() {
                        duplicate_sheet(p, book, s);
                    }
                });
            }
            NavAction::DuplicateWithoutData => {
                if let Err(e) = self.doc.edit("Duplicate sheets without data", |p| {
                    sheets.iter().try_for_each(|&s| p.duplicate_sheet_without_data(book, s).map(|_| ()))
                }) {
                    self.warn(e.to_string());
                }
            }
            NavAction::MoveToNewBook | NavAction::DuplicateToNewBook => {
                let moving = action == NavAction::MoveToNewBook;
                let (label, done) = if moving {
                    ("Move sheets to new book", "Moved")
                } else {
                    ("Duplicate sheets to new book", "Copied")
                };
                match self.doc.edit(label, |p| {
                    if moving {
                        p.move_sheets_to_new_book(book, &sheets)
                    } else {
                        p.duplicate_sheets_to_new_book(book, &sheets)
                    }
                }) {
                    Ok(new) => {
                        let name = self.doc.project().workbook(new).map(|w| w.name.clone()).unwrap_or_default();
                        self.say(format!("{done} {} sheet(s) to {name}", sheets.len()));
                    }
                    Err(e) => {
                        self.warn(e.to_string());
                        return;
                    }
                }
            }
            NavAction::MoveToBook(to) => {
                match self.doc.edit("Move sheets to book", |p| p.move_sheets_to_book(book, &sheets, to)) {
                    Ok(()) => {
                        let name = self.doc.project().workbook(to).map(|w| w.name.clone()).unwrap_or_default();
                        self.say(format!("Moved {} sheet(s) to {name}", sheets.len()));
                    }
                    Err(e) => {
                        self.warn(e.to_string());
                        return;
                    }
                }
            }
            NavAction::ExcludeFromPlotting => {
                let all = sheets.iter().all(|&s| b.sheets[s].exclude_from_plotting);
                self.doc.apply("Exclude from plotting dialogs", |p| {
                    if let Some(w) = p.workbook_mut(book) {
                        for &s in &sheets {
                            w.sheets[s].exclude_from_plotting = !all;
                        }
                    }
                });
                return;
            }
            NavAction::ClearData => {
                self.doc.apply("Clear worksheet data", |p| {
                    for &s in &sheets {
                        p.clear_sheet(book, s);
                    }
                });
            }
            NavAction::Delete => {
                if sheets.len() >= count {
                    self.warn("A workbook keeps at least one sheet".to_string());
                    return;
                }
                let ids: Vec<ColumnId> =
                    sheets.iter().flat_map(|&s| b.sheets[s].columns.iter().map(|c| c.id)).collect();
                self.doc.apply("Delete sheets", |p| {
                    delete_columns(p, &ids);
                    if let Some(w) = p.workbook_mut(book) {
                        let active = w.active_sheet;
                        for &s in sheets.iter().rev() {
                            w.sheets.remove(s);
                        }
                        // The active sheet stays active when it is kept; else the sheet now in its place.
                        let before = sheets.iter().filter(|&&s| s < active).count();
                        w.active_sheet = if sheets.contains(&active) {
                            active.saturating_sub(before).min(w.sheets.len() - 1)
                        } else {
                            active - before
                        };
                    }
                });
            }
        }
        if let Some(state) = self.books.get_mut(&book) {
            state.clear_selection();
        }
        let active = self.doc.project().workbook(book).map_or(0, |w| w.active_sheet);
        dialog.reset(active);
    }

    /// Copy Columns to: OK copies the columns, one undo step.
    fn show_copy_columns(&mut self, ctx: &egui::Context) {
        use crate::dialogs::copy_columns::CopyColumnsOutcome;
        let Some(d) = &mut self.copy_columns else { return };
        match d.show(ctx, self.doc.project()) {
            CopyColumnsOutcome::Open => {}
            CopyColumnsOutcome::Closed => self.copy_columns = None,
            CopyColumnsOutcome::Copy { columns, dest, parts } => {
                match self.doc.edit("Copy columns", |p| p.copy_columns_to(&columns, dest, parts)) {
                    Ok((book, ids)) => {
                        self.copy_columns = None;
                        let name = self.doc.project().workbook(book).map(|b| b.name.clone()).unwrap_or_default();
                        self.say(format!("Copied {} column(s) to {name}", ids.len()));
                    }
                    Err(e) => d.set_problem(e.to_string()),
                }
            }
        }
    }

    /// Set Sampling Interval and Show X Column: OK changes every chosen column, one undo step.
    fn show_sampling(&mut self, ctx: &egui::Context) {
        use crate::dialogs::sampling::{SamplingOutcome, ShowXOutcome};
        if let Some(d) = &mut self.sampling {
            match d.show(ctx) {
                SamplingOutcome::Open => {}
                SamplingOutcome::Closed => self.sampling = None,
                SamplingOutcome::Set { columns, x0, inc, long_name, units } => {
                    let set = self.doc.edit("Set sampling interval", |p| {
                        p.set_sampling_interval(&columns, x0, inc, &long_name, &units)
                    });
                    match set {
                        Ok(()) => {
                            self.sampling = None;
                            let what = if x0 == 1.0 && inc == 1.0 {
                                "Cleared the sampling interval"
                            } else {
                                "Set the sampling interval"
                            };
                            self.say(format!("{what} of {} column(s)", columns.len()));
                        }
                        Err(e) => d.set_problem(e.to_string()),
                    }
                }
            }
        }
        if let Some(d) = &mut self.show_x {
            match d.show(ctx) {
                ShowXOutcome::Open => {}
                ShowXOutcome::Closed => self.show_x = None,
                ShowXOutcome::Show { columns, clear } => {
                    let shown = self.doc.edit("Show X column", |p| {
                        for c in &columns {
                            p.show_x_column(*c, clear)?;
                        }
                        Ok::<(), sheaf_core::sampling::SamplingError>(())
                    });
                    match shown {
                        Ok(()) => {
                            self.show_x = None;
                            self.say(format!("Showed the X of {} column(s)", columns.len()));
                        }
                        Err(e) => d.set_problem(e.to_string()),
                    }
                }
            }
        }
    }

    /// The Fill Column With pattern window: OK fills every chosen column, one undo step.
    fn show_fill_pattern(&mut self, ctx: &egui::Context) {
        use crate::dialogs::fill_pattern::FillOutcome;
        let Some(d) = &mut self.fill_pattern else { return };
        match d.show(ctx) {
            FillOutcome::Open => {}
            FillOutcome::Closed => self.fill_pattern = None,
            FillOutcome::Fill(columns, pattern) => {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |t| t.as_nanos() as u64);
                let filled = self.doc.edit("Fill column", |p| {
                    let mut rows = 0;
                    for (k, c) in columns.iter().enumerate() {
                        rows = p.fill_pattern(*c, &pattern, seed.wrapping_add(k as u64))?;
                    }
                    Ok::<usize, sheaf_core::fill_pattern::FillError>(rows)
                });
                match filled {
                    Ok(rows) => {
                        self.fill_pattern = None;
                        self.say(format!("Filled {} column(s), {rows} rows each", columns.len()));
                    }
                    Err(e) => d.set_problem(e.to_string()),
                }
            }
        }
    }

    /// Reads the template names again after a change.
    fn refresh_user_templates(&mut self) {
        use sheaf_io::user_templates::{TemplateKind, names_of};
        let dir = self.startup.templates_dir.as_deref();
        self.user_templates = dir.map(|d| names_of(d, TemplateKind::Graph)).unwrap_or_default();
        self.user_book_templates = dir.map(|d| names_of(d, TemplateKind::Workbook)).unwrap_or_default();
    }

    /// The Save Template As prompt and the Manage window.
    /// Batch Peak Analysis Using Theme... (T169): the window on the given curves, or the curves a plot of
    /// the active sheet would show; columns without numbers are left out.
    fn open_batch_peaks(&mut self, book: WindowId, columns: Vec<ColumnId>) {
        let project = self.doc.project();
        let columns: Vec<ColumnId> = if columns.is_empty() {
            let sheet = project.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
            project.default_plot_columns(book, sheet, Template::Line)
        } else {
            columns.into_iter().filter(|id| project.column(*id).is_some_and(|c| c.data.is_numeric())).collect()
        };
        let names: Vec<String> =
            columns.iter().filter_map(|id| project.column(*id).map(|c| c.short_name.clone())).collect();
        let themes = self.startup.peak_themes_path.as_deref().map(sheaf_io::peak_themes::names).unwrap_or_default();
        self.batch_peaks = Some(crate::dialogs::batch_peaks::BatchPeaksDialog::new(themes, columns, names.join(", ")));
    }

    fn show_batch_peaks(&mut self, ctx: &egui::Context) {
        use crate::dialogs::batch_peaks::BatchPeaksOutcome;
        let Some(d) = &mut self.batch_peaks else { return };
        match d.show(ctx) {
            BatchPeaksOutcome::Open => {}
            BatchPeaksOutcome::Closed => self.batch_peaks = None,
            BatchPeaksOutcome::Run(name) => {
                let columns = d.columns.clone();
                let theme =
                    self.startup.peak_themes_path.as_deref().and_then(|p| sheaf_io::peak_themes::load(p, &name));
                let Some(theme) = theme else {
                    d.set_error(format!("The theme {name} is no longer saved"));
                    return;
                };
                let request = sheaf_core::analysis::Request {
                    columns,
                    rows: sheaf_core::analysis::RowRange::All,
                    tool: sheaf_core::analysis::Tool::PeakChain(theme.spec),
                };
                let made = self.doc.edit("Batch peak analysis", |p| {
                    p.analyze_to(&request, &sheaf_core::analysis::OutputTarget::default())
                });
                match made {
                    Ok(report) => {
                        self.batch_peaks = None;
                        self.say(format!("Batch peak analysis with the theme {}: {}", theme.name, report.message));
                    }
                    Err(e) => {
                        if let Some(d) = &mut self.batch_peaks {
                            d.set_error(e.to_string());
                        }
                    }
                }
            }
        }
    }

    /// A data cell's Insert Sparklines... (T200): the column picker.
    fn show_cell_sparkline_picker(&mut self, ctx: &egui::Context) {
        use crate::dialogs::cell_sparkline::CellSparklineOutcome;
        let Some(p) = &mut self.cell_sparkline_picker else { return };
        match p.show(ctx) {
            CellSparklineOutcome::Open => {}
            CellSparklineOutcome::Closed => self.cell_sparkline_picker = None,
            CellSparklineOutcome::Pick(source) => {
                let (column, row) = (p.column, p.row);
                self.cell_sparkline_picker = None;
                self.apply(Action::SetCellSparkline { column, row, source: Some(source) }, ctx);
            }
        }
    }

    /// Add Graph as Sheet... (T190): the graph picker.
    fn show_graph_sheet_picker(&mut self, ctx: &egui::Context) {
        use crate::dialogs::graph_sheet::GraphSheetOutcome;
        let Some(p) = &mut self.graph_sheet_picker else { return };
        match p.show(ctx) {
            GraphSheetOutcome::Open => {}
            GraphSheetOutcome::Closed => self.graph_sheet_picker = None,
            GraphSheetOutcome::Pick(graph) => {
                let target = p.target;
                self.graph_sheet_picker = None;
                use crate::dialogs::graph_sheet::PickTarget;
                let action = match target {
                    PickTarget::Sheet(book) => Action::AddGraphSheet { book, graph },
                    PickTarget::Cell { column, row } => Action::SetCellGraph { column, row, graph: Some(graph) },
                };
                self.apply(action, ctx);
            }
        }
    }

    /// Worksheet > Name Manager... (T434).
    fn show_name_manager(&mut self, ctx: &egui::Context) {
        use crate::dialogs::name_cell::NameManagerOutcome;
        let Some(m) = &mut self.name_manager else { return };
        let Some(w) = self.doc.project().workbook(m.book) else {
            self.name_manager = None;
            return;
        };
        let names: Vec<(String, String)> = w
            .names
            .iter()
            .map(|n| (n.name.clone(), w.name_reference(n).unwrap_or_else(|| "(its column is gone)".to_string())))
            .collect();
        let title = w.name.clone();
        let book = m.book;
        match m.show(ctx, &title, &names) {
            NameManagerOutcome::Open => {}
            NameManagerOutcome::Closed => self.name_manager = None,
            NameManagerOutcome::Delete(name) => self.apply(Action::DeleteName { book, name }, ctx),
        }
    }

    /// A data cell's Define Name... (T396).
    fn show_name_cell(&mut self, ctx: &egui::Context) {
        use crate::dialogs::name_cell::NameCellOutcome;
        let Some(d) = &mut self.name_cell else { return };
        match d.show(ctx) {
            NameCellOutcome::Open => {}
            NameCellOutcome::Closed => self.name_cell = None,
            NameCellOutcome::Define(name) => {
                let (column, row, last, last_column) = (d.column, d.row, d.last, d.last_column);
                self.apply(Action::SetCellName { column, row, last, last_column, name }, ctx);
            }
        }
    }

    /// The first and last selected rows when the selection is several cells of `column` alone and holds
    /// `row` (T400).
    fn selected_block_of(&self, column: ColumnId, row: usize) -> Option<(ColumnId, ColumnId, usize, usize)> {
        // T432: the selected block of cells when it holds this cell and is more than one cell: its first and last
        // columns and rows.
        let project = self.doc.project();
        let (book, _, _) = project.locate(column)?;
        let sheet = book.sheets.get(book.active_sheet)?;
        let index = sheet.column_index(column)?;
        let target = self.books.get(&book.id)?.target(sheet)?;
        let rows = target.rows;
        let (lo, hi) = (*target.columns.iter().min()?, *target.columns.iter().max()?);
        let contiguous = target.columns.len() == hi - lo + 1;
        let inside = contiguous && (lo..=hi).contains(&index) && rows.contains(&row);
        let more = rows.len() > 1 || hi > lo;
        (inside && more).then(|| (sheet.columns[lo].id, sheet.columns[hi].id, rows.start, rows.end - 1))
    }

    /// Send Graphs to PowerPoint... (T232): the window, then the file picker and the file.
    fn show_send_ppt(&mut self, ctx: &egui::Context) {
        use crate::dialogs::send_ppt::SendPptOutcome;
        let Some(d) = &mut self.send_ppt else { return };
        match d.show(ctx) {
            SendPptOutcome::Open => {}
            SendPptOutcome::Closed => self.send_ppt = None,
            SendPptOutcome::Send { graphs, word } => {
                self.send_ppt = None;
                let (filter, ext) = if word { ("Word document", "docx") } else { ("PowerPoint presentation", "pptx") };
                let picked = rfd::FileDialog::new()
                    .set_title("Send Graphs to PowerPoint")
                    .add_filter(filter, &[ext])
                    .set_file_name(format!("Graphs.{ext}"))
                    .save_file();
                if let Some(path) = picked {
                    self.apply(Action::WriteGraphDocument { graphs, path }, ctx);
                }
            }
        }
    }

    /// File > Batch Processing... (T165): the window, its file picker and the run.
    fn show_batch(&mut self, ctx: &egui::Context) {
        use crate::dialogs::batch_process::BatchOutcome;
        let Some(d) = &mut self.batch else { return };
        match d.show(ctx) {
            BatchOutcome::Open => {}
            BatchOutcome::Closed => self.batch = None,
            BatchOutcome::AddFiles => {
                if let Some(files) = rfd::FileDialog::new().set_title("Batch Processing: data files").pick_files() {
                    d.add_files(files);
                }
            }
            BatchOutcome::Run { template, files, summary } => match self.run_batch(&template, &files, summary.as_ref())
            {
                Ok(()) => self.batch = None,
                Err(e) => {
                    if let Some(d) = &mut self.batch {
                        d.set_error(e);
                    }
                }
            },
        }
    }

    /// Reads every file and makes one book per file from the workbook template `name`, then with a
    /// `summary` (T233) the summary book (and deletes the books made when asked), all in one undo
    /// step ("Batch processing"). Files that cannot be read are skipped and named in the status
    /// line; an error only when no book could be made.
    pub(crate) fn run_batch(
        &mut self,
        name: &str,
        files: &[PathBuf],
        summary: Option<&crate::dialogs::batch_process::BatchSummary>,
    ) -> Result<(), String> {
        let dir = self.startup.templates_dir.clone().ok_or("there is no template folder")?;
        let template = sheaf_io::user_templates::load_book(&dir, name).map_err(|e| e.to_string())?;
        let mut read = Vec::new();
        let mut failed = Vec::new();
        for path in files {
            match sheaf_io::import_file(path, sheaf_io::ImportKind::Auto, &Default::default(), &Default::default()) {
                Ok(imported) => {
                    let title = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    read.push((title, imported.workbook));
                }
                Err(e) => failed.push(format!("{}: {e}", path.display())),
            }
        }
        if read.is_empty() {
            return Err(format!("No file could be read. {}", failed.join("; ")));
        }
        let request = summary.map(|s| sheaf_core::BatchSummaryRequest {
            sheet: s.sheet.clone(),
            identifier: s.identifier.clone(),
            delete_books: s.delete_books,
        });
        let run = self.doc.apply("Batch processing", |p| p.batch_process(&template, &read, request.as_ref()));
        let (made, summary_book) = (run.books, run.summary);
        if let Some(book) = summary_book.or(made.last().map(|m| m.book)) {
            self.focus(book);
        }
        let problems: Vec<String> = made.iter().flat_map(|m| m.problems.iter().cloned()).collect();
        let mut text = format!("Batch processing: {} workbook(s) from the template {}", made.len(), template.name);
        if let Some(s) = summary {
            match summary_book.and_then(|b| self.doc.project().workbook(b)) {
                Some(b) if s.delete_books => {
                    text.push_str(&format!("; summary of {} in {} (the books were deleted)", s.sheet, b.name));
                }
                Some(b) => text.push_str(&format!("; summary of {} in {}", s.sheet, b.name)),
                None => text.push_str(&format!("; no book has the sheet {}, so there is no summary", s.sheet)),
            }
        }
        if !failed.is_empty() {
            text.push_str(&format!("; not read: {}", failed.join("; ")));
        }
        if !problems.is_empty() {
            text.push_str(&format!("; analysis problems: {}", problems.join("; ")));
        }
        if failed.is_empty() && problems.is_empty() && (summary.is_none() || summary_book.is_some()) {
            self.say(text);
        } else {
            self.warn(text);
        }
        Ok(())
    }

    fn show_template_windows(&mut self, ctx: &egui::Context) {
        use crate::dialogs::user_templates::{ManagerOutcome, PromptOutcome};
        let dir = self.startup.templates_dir.clone();
        if let Some(prompt) = &mut self.template_prompt {
            match prompt.show(ctx) {
                PromptOutcome::Open => {}
                PromptOutcome::Closed => self.template_prompt = None,
                PromptOutcome::Save(name) => {
                    let window = prompt.window;
                    let book = prompt.kind == sheaf_io::user_templates::TemplateKind::Workbook;
                    let analysis = prompt.analysis;
                    let mut runs = 0;
                    let project = self.doc.project();
                    let made =
                        dir.as_deref().ok_or_else(|| "there is no template folder".to_string()).and_then(|dir| {
                            if book {
                                let b = project.workbook(window).ok_or("the workbook was closed")?;
                                let t = if analysis {
                                    sheaf_core::BookTemplate::analysis_from_workbook(&name, project, window)
                                        .ok_or("the workbook was closed")?
                                } else {
                                    sheaf_core::BookTemplate::from_workbook(&name, b)
                                };
                                runs = t.links.len();
                                sheaf_io::user_templates::save_book(dir, &t).map_err(|e| e.to_string())
                            } else {
                                let g = project.graph(window).ok_or("the graph was closed")?;
                                let t = sheaf_core::UserTemplate::from_graph(&name, g).map_err(|e| e.to_string())?;
                                sheaf_io::user_templates::save(dir, &t).map_err(|e| e.to_string())
                            }
                        });
                    match made {
                        Ok(saved) => {
                            self.template_prompt = None;
                            self.refresh_user_templates();
                            let verb = if saved.replaced { "Replaced" } else { "Saved" };
                            let place = if book { "File > New > From Template" } else { "Plot > User Templates" };
                            if analysis {
                                self.say(format!(
                                    "{verb} the analysis template {} with {runs} recalculating analysis run(s) ({place})",
                                    saved.name
                                ));
                            } else {
                                self.say(format!("{verb} the template {} ({place})", saved.name));
                            }
                        }
                        Err(e) => prompt.set_error(e),
                    }
                }
            }
        }
        if let Some(m) = &mut self.template_manager {
            let outcome = m.show(ctx, dir.as_deref());
            let Some(dir) = dir else { return };
            match outcome {
                ManagerOutcome::Open => {}
                ManagerOutcome::Closed => self.template_manager = None,
                ManagerOutcome::Rename { kind, from, to } => {
                    match sheaf_io::user_templates::rename_of(&dir, &from, &to, kind) {
                        Ok(new) => {
                            self.refresh_user_templates();
                            if let Some(m) = &mut self.template_manager {
                                m.set_names(self.user_templates.clone(), self.user_book_templates.clone());
                                m.select(kind, &new);
                            }
                        }
                        Err(e) => m.set_error(e.to_string()),
                    }
                }
                ManagerOutcome::Delete(kind, name) => match sheaf_io::user_templates::delete_of(&dir, &name, kind) {
                    Ok(_) => {
                        self.refresh_user_templates();
                        if let Some(m) = &mut self.template_manager {
                            m.set_names(self.user_templates.clone(), self.user_book_templates.clone());
                        }
                    }
                    Err(e) => m.set_error(e.to_string()),
                },
            }
        }
    }

    /// Plot > Line + Symbol > Line Series (T82): the selected Y columns (else the sheet's), two or more.
    fn plot_line_series(&mut self, book: WindowId, columns: Vec<ColumnId>) {
        let ys = self.plot_y_columns(book, columns, Template::LineSymbol);
        let made = self.doc.edit("Line Series", |p| p.plot_line_series(&ys));
        match made {
            Ok(_) => self.say(format!("Line Series of {} column(s)", ys.len())),
            Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
        }
    }

    /// Plot > Symbol > Y Error: a scatter plot; the error bars come from the Y error column that
    /// follows each Y column (Column > Set As > Y Error).
    fn plot_y_error(&mut self, book: WindowId, columns: Vec<ColumnId>, x: bool) {
        let ys = self.plot_y_columns(book, columns, Template::Scatter);
        if ys.is_empty() {
            self.warn("Nothing to plot: select one or more numeric Y columns".to_string());
            return;
        }
        let project = self.doc.project();
        let count = |xerr: bool| {
            ys.iter()
                .filter(|y| {
                    project.locate(**y).is_some_and(|(_, sheet, _)| {
                        sheet.column_index(**y).is_some_and(|i| {
                            if xerr { sheet.x_error_for(i).is_some() } else { sheet.y_error_for(i).is_some() }
                        })
                    })
                })
                .count()
        };
        let (with_y, with_x) = (count(false), if x { count(true) } else { 0 });
        let what = if x { "X or Y error" } else { "Y error" };
        let made = self.doc.edit("Plot error bars", |p| {
            let graph = p.plot_columns(Template::Scatter, &ys)?;
            if !x {
                // Y Error shows no X error bars even when the sheet has an X error column.
                if let Some(g) = p.graph_mut(graph) {
                    g.layers.iter_mut().flat_map(|l| l.plots.iter_mut()).for_each(|pl| pl.x_error = None);
                }
            }
            Ok::<_, sheaf_core::project::ProjectError>(graph)
        });
        match made {
            Ok(_) if with_y + with_x == 0 => {
                self.warn(format!("Plotted without error bars: no {what} column follows the selected columns"))
            }
            Ok(_) => {
                self.say(format!("Plotted {} column(s), {with_y} with Y and {with_x} with X error bars", ys.len()))
            }
            Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
        }
    }

    /// Plot > Multi-Panel > Vertical 2 Panel, Horizontal 2 Panel, 4 Panel, 9 Panel: one panel per
    /// Y column, in a grid of `rows` x `cols` panels (more columns than panels wrap around).
    fn plot_panels(&mut self, book: WindowId, columns: Vec<ColumnId>, rows: usize, cols: usize) {
        let ys = self.plot_y_columns(book, columns, Template::Line);
        if ys.is_empty() {
            self.warn("Nothing to plot: select one or more numeric Y columns".to_string());
            return;
        }
        let frame = Template::Line.new_graph(WindowId(0), "").layers[0].frame;
        let layout = sheaf_core::panels::PanelLayout::grid(rows, cols, &frame);
        let groups: Vec<Vec<ColumnId>> = ys.iter().map(|y| vec![*y]).collect();
        match self.doc.edit("Plot panels", |p| p.plot_panels(Template::Line, &groups, &layout)) {
            Ok(_) => self.say(format!("Plotted {} column(s) in a {rows} x {cols} grid of panels", ys.len())),
            Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
        }
    }

    /// Graph > Add Error Bars: choose a plot and an existing error column, without changing
    /// any other plot of the layer.
    fn add_error_bars(&mut self, graph: WindowId, layer: usize) {
        let selected = self.graphs.get(&graph).and_then(|state| match state.selected {
            Some((l, sheaf_plot::scene::ObjectKind::Plot(i))) if l == layer => Some(i),
            _ => None,
        });
        self.error_bars = ErrorBarsDialog::new(self.doc.project(), graph, layer, selected);
        if self.error_bars.is_none() {
            self.warn("No error bars added: the active layer has no plots");
        }
    }

    /// Window > Refresh: recalculates the columns of the active workbook that wait for it.
    fn refresh(&mut self, active: Option<WindowId>) {
        let Some(book) = active.filter(|id| self.doc.project().workbook(*id).is_some()) else {
            self.say("Refreshed");
            return;
        };
        let sheets = self.doc.project().workbook(book).map(|b| b.sheets.len()).unwrap_or(0);
        let pending = self.doc.project().workbook(book).is_some_and(|b| {
            b.sheets.iter().any(|s| {
                s.columns.iter().any(|c| {
                    c.formula_state == sheaf_core::column::FormulaState::Pending
                        || c.analysis.is_some_and(|m| m.state != sheaf_core::column::FormulaState::Current)
                })
            })
        });
        if !pending {
            self.say("Refreshed");
            return;
        }
        let (problems, analysis_problems) = self.doc.apply("Refresh", |p| {
            let formulas = (0..sheets).flat_map(|s| p.recalculate_pending(book, s)).collect::<Vec<_>>();
            (formulas, p.recalculate_pending_analyses(book))
        });
        match (problems.first(), analysis_problems.first()) {
            (Some(first), _) => self.warn(format!("Refreshed; a column could not be recalculated: {}", first.message)),
            (None, Some(first)) => self.warn(format!("Refreshed; an analysis could not be recalculated: {first}")),
            (None, None) => self.say("Refreshed and recalculated"),
        }
    }

    fn selected_sort_rows(&self, book: WindowId) -> Option<std::ops::Range<usize>> {
        let state = self.books.get(&book)?;
        if !matches!(state.sel, grid::Sel::Cells { .. } | grid::Sel::Rows { .. }) {
            return None;
        }
        let sheet = self.doc.project().workbook(book)?.sheet();
        state.target(sheet).map(|t| t.rows)
    }

    /// Worksheet > Sort Range (`range`: the rows of the selected columns move together, ordered
    /// by the leftmost of them) and Sort Columns (each selected column is sorted on its own).
    fn sort_selected(&mut self, columns: Vec<ColumnId>, descending: bool, range: bool) {
        let project = self.doc.project();
        let Some(first) = columns.first().and_then(|c| project.locate(*c)) else { return };
        let (book_id, sheet) = (first.0.id, first.1);
        let rows = self.selected_sort_rows(book_id).unwrap_or(0..usize::MAX);
        let Some(sheet_index) = first.0.sheets.iter().position(|s| std::ptr::eq(s, sheet)) else { return };
        let mut on_sheet: Vec<(usize, ColumnId)> =
            columns.iter().filter_map(|c| Some((sheet.column_index(*c)?, *c))).collect();
        on_sheet.sort();
        let ids: Vec<ColumnId> = on_sheet.iter().map(|(_, c)| *c).collect();
        let Some(&key) = ids.first() else { return };
        let order = if descending { "descending" } else { "ascending" };
        let result = self.doc.edit(if range { "Sort range" } else { "Sort columns" }, |p| {
            if range {
                p.sort_range_in_rows(book_id, sheet_index, key, &ids, descending, rows)
            } else {
                p.sort_columns_in_rows(book_id, sheet_index, &ids, descending, rows)
            }
        });
        match result {
            Ok(()) if range => self.say(format!("Sorted {} column(s) as a range, {order}", ids.len())),
            Ok(()) => self.say(format!("Sorted {} column(s), each on its own, {order}", ids.len())),
            Err(e) => self.message = Some(("Could not sort".to_string(), e.to_string())),
        }
    }

    /// Opens the window of an analysis tool. With no columns given, the tool gets the columns a
    /// plot of the active sheet would show; columns that hold text are left out.
    fn open_analysis(&mut self, book: WindowId, columns: Vec<ColumnId>, kind: AnalysisKind) {
        let project = self.doc.project();
        // The proportion tests read no data: one column of the sheet only says where the results go.
        let host_only = matches!(
            kind,
            AnalysisKind::PropTestOne
                | AnalysisKind::PropTestTwo
                | AnalysisKind::PoissonOne
                | AnalysisKind::PoissonTwo
                | AnalysisKind::FactorialDesign
                | AnalysisKind::MixtureDesign
                | AnalysisKind::MonteCarlo
                | AnalysisKind::SplitPlotDesign
        );
        let columns: Vec<ColumnId> = if host_only {
            let sheet = project.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
            let all = project.workbook(book).and_then(|b| b.sheets.get(sheet)).map(|s| &s.columns);
            let first = columns.first().copied().or_else(|| all.and_then(|c| c.first()).map(|c| c.id));
            first.into_iter().collect()
        } else if columns.is_empty() {
            let sheet = project.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
            if kind == AnalysisKind::Crosstab {
                // Categorical columns are not Y columns: start with the first two of the sheet.
                let all = project.workbook(book).and_then(|b| b.sheets.get(sheet)).map(|s| &s.columns);
                all.map(|c| c.iter().take(2).map(|c| c.id).collect()).unwrap_or_default()
            } else if matches!(kind, AnalysisKind::RandomForest | AnalysisKind::Qpcr) {
                // The response (last) may be a text column of classes, which an import makes a Label column: every
                // Y and Label column of the sheet, in order (T377).
                let all = project.workbook(book).and_then(|b| b.sheets.get(sheet)).map(|s| &s.columns);
                let wanted = |c: &&sheaf_core::Column| matches!(c.designation, Designation::Y | Designation::Label);
                all.map(|c| c.iter().filter(wanted).map(|c| c.id).collect()).unwrap_or_default()
            } else {
                project.default_plot_columns(book, sheet, Template::Line)
            }
        } else {
            // These tools read categorical columns (factors, subjects), which may hold text.
            let any_type = matches!(
                kind,
                AnalysisKind::Crosstab
                    | AnalysisKind::Anova3
                    | AnalysisKind::RmAnova1
                    | AnalysisKind::RmAnova2
                    | AnalysisKind::RandomForest
                    | AnalysisKind::Qpcr
            );
            columns
                .into_iter()
                .filter(|id| project.column(*id).is_some_and(|c| any_type || c.data.is_numeric()))
                .collect()
        };
        if columns.is_empty() {
            self.warn("Nothing to analyze: select one or more columns that hold numbers".to_string());
            return;
        }
        self.analysis = AnalysisDialog::new(project, kind, columns);
        if let Some(d) = &mut self.analysis {
            d.peak_themes = self.startup.peak_themes_path.clone();
        }
    }

    fn sort_sheet(&mut self, column: ColumnId, descending: bool) {
        let project = self.doc.project();
        let Some((book, _, c)) = project.locate(column) else {
            return;
        };
        let (book_id, name) = (book.id, c.short_name.clone());
        let Some(sheet) = book.sheets.iter().position(|s| s.column(column).is_some()) else {
            return;
        };
        match self.doc.edit("Sort sheet", |p| p.sort_sheet(book_id, sheet, column, descending)) {
            Ok(()) => {
                let order = if descending { "descending" } else { "ascending" };
                self.say(format!("Sorted the sheet by column {name}, {order}"));
            }
            Err(e) => self.message = Some(("Could not sort the sheet".to_string(), e.to_string())),
        }
    }

    /// Shows a sheet of a workbook (a result sheet that a tool just added).
    /// The range of the selection of a workbook's grid, for conditional formatting.
    fn cond_range(&self, book: WindowId) -> Result<sheaf_core::condformat::CondRange, &'static str> {
        let sheet = self.doc.project().workbook(book).map(|b| b.sheet()).ok_or("This workbook no longer exists")?;
        let state = self.books.get(&book).ok_or("Select cells, rows or columns of the data first")?;
        grid::cond_range(state, sheet)
    }

    fn format_target(&self, book: WindowId) -> Result<sheaf_core::cellformat::FormatTarget, &'static str> {
        let sheet = self.doc.project().workbook(book).map(|b| b.sheet()).ok_or("This workbook no longer exists")?;
        let state = self.books.get(&book).ok_or("Select cells first")?;
        grid::format_target(state, sheet)
    }

    fn note_cell(&self, book: WindowId) -> Result<(ColumnId, usize), &'static str> {
        let sheet = self.doc.project().workbook(book).map(|b| b.sheet()).ok_or("This workbook no longer exists")?;
        let state = self.books.get(&book).ok_or("Select a cell first")?;
        grid::note_cell(state, sheet)
    }

    /// Multiple Peak Fit: while picking, the graph's Data Reader is on and its pinned point becomes a
    /// peak centre on a double-click (outside the Get Points window) or Enter.
    fn multi_peak_frame(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.multi_peak else { return };
        if d.picking {
            let (enter, double) = ctx.input(|i| {
                let double = i.pointer.button_double_clicked(egui::PointerButton::Primary)
                    && i.pointer.interact_pos().is_some_and(|p| !d.window_rect.is_some_and(|r| r.contains(p)));
                (i.key_pressed(egui::Key::Enter), double)
            });
            let state = self.graphs.entry(d.graph).or_default();
            state.reader = true;
            if (enter || double)
                && let Some(point) = state.reader_point
            {
                d.pick(self.doc.project(), point);
            }
        }
        let graph = d.graph;
        match d.show(ctx, &mut self.doc) {
            MultiPeakOutcome::Open => {}
            MultiPeakOutcome::StartPicking => {
                self.graphs.entry(graph).or_default().reader = true;
                self.focus(graph);
            }
            MultiPeakOutcome::Closed => {
                self.multi_peak = None;
                if let Some(s) = self.graphs.get_mut(&graph) {
                    s.reader = false;
                    s.reader_point = None;
                }
            }
            MultiPeakOutcome::Done(report) => {
                self.multi_peak = None;
                if let Some(s) = self.graphs.get_mut(&graph) {
                    s.reader = false;
                    s.reader_point = None;
                }
                if let Some(sheet) = report.new_sheet {
                    self.show_sheet(report.book, sheet);
                }
                self.focus(graph);
                self.logs.result(report.message.clone(), report.notes.clone());
                self.panels.reveal(PanelId::Results);
                self.say(report.message);
            }
        }
    }

    /// Subtract Straight Line on a graph (T348): the Data Reader is on and its pinned point becomes an end of the
    /// line on a double-click (outside the window) or Enter.
    fn subtract_pick_frame(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.subtract_pick else { return };
        let (enter, double) = ctx.input(|i| {
            let double = i.pointer.button_double_clicked(egui::PointerButton::Primary)
                && i.pointer.interact_pos().is_some_and(|p| !d.window_rect.is_some_and(|r| r.contains(p)));
            (i.key_pressed(egui::Key::Enter), double)
        });
        let state = self.graphs.entry(d.graph).or_default();
        state.reader = true;
        if (enter || double)
            && let Some(point) = state.reader_point
        {
            d.pick(self.doc.project(), point);
        }
        let graph = d.graph;
        let outcome = d.show(ctx, &mut self.doc);
        if !matches!(outcome, SubtractPickOutcome::Open) {
            self.subtract_pick = None;
            if let Some(s) = self.graphs.get_mut(&graph) {
                s.reader = false;
                s.reader_point = None;
            }
        }
        if let SubtractPickOutcome::Done(report) = outcome {
            self.focus(graph);
            self.logs.result(report.message.clone(), report.notes.clone());
            self.panels.reveal(PanelId::Results);
            self.say(report.message);
        }
    }

    /// CV Charge on a graph (T349): the Data Reader is on and its pinned point becomes a foot of the peak on a
    /// double-click (outside the window) or Enter.
    fn cv_pick_frame(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.cv_pick else { return };
        let (enter, double) = ctx.input(|i| {
            let double = i.pointer.button_double_clicked(egui::PointerButton::Primary)
                && i.pointer.interact_pos().is_some_and(|p| !d.window_rect.is_some_and(|r| r.contains(p)));
            (i.key_pressed(egui::Key::Enter), double)
        });
        let state = self.graphs.entry(d.graph).or_default();
        state.reader = true;
        if (enter || double)
            && let Some(point) = state.reader_point
        {
            d.pick(self.doc.project(), point);
        }
        let graph = d.graph;
        let outcome = d.show(ctx, &mut self.doc);
        if !matches!(outcome, CvPickOutcome::Open) {
            self.cv_pick = None;
            if let Some(s) = self.graphs.get_mut(&graph) {
                s.reader = false;
                s.reader_point = None;
            }
        }
        if let CvPickOutcome::Done(report) = outcome {
            self.focus(graph);
            self.logs.result(report.message.clone(), report.notes.clone());
            self.panels.reveal(PanelId::Results);
            self.say(report.message);
        }
    }

    /// The Peak Analyzer's "Pick on the graph" anchors (T351): while the window picks, the Data Reader of a graph
    /// that plots the input is on and its pinned point becomes an anchor on a double-click (outside the window)
    /// or Enter; the reader goes off again when the picking stops or the window closes.
    fn anchor_pick_frame(&mut self, ctx: &egui::Context) {
        let picking = self.analysis.as_ref().and_then(|d| d.anchor_column().filter(|_| d.anchor_picking));
        let Some(y) = picking else {
            if let Some(g) = self.anchor_graph.take()
                && let Some(s) = self.graphs.get_mut(&g)
            {
                s.reader = false;
                s.reader_point = None;
            }
            return;
        };
        let project = self.doc.project();
        let shows = |g: &sheaf_core::graph::Graph| {
            g.layers.iter().enumerate().find_map(|(l, layer)| layer.plots.iter().position(|p| p.y == y).map(|i| (l, i)))
        };
        let target = self
            .anchor_graph
            .and_then(|id| project.graph(id))
            .or_else(|| project.graphs().find(|g| shows(g).is_some()))
            .and_then(|g| shows(g).map(|(l, i)| (g.id, l, i)));
        let Some((graph, layer, plot)) = target else { return };
        self.anchor_graph = Some(graph);
        let rect = self.analysis.as_ref().and_then(|d| d.window_rect);
        let (enter, double) = ctx.input(|i| {
            let double = i.pointer.button_double_clicked(egui::PointerButton::Primary)
                && i.pointer.interact_pos().is_some_and(|p| !rect.is_some_and(|r| r.contains(p)));
            (i.key_pressed(egui::Key::Enter), double)
        });
        let state = self.graphs.entry(graph).or_default();
        state.reader = true;
        if (enter || double)
            && let Some(point) = state.reader_point.filter(|p| p.layer == layer && p.plot == plot)
            && let Some((xs, ys)) = crate::gadget::plot_xy(project, graph, layer, plot)
            && let (Some(&x), Some(&v)) = (xs.get(point.row), ys.get(point.row))
            && let Some(d) = &mut self.analysis
        {
            d.pick_anchor(x, v);
        }
    }

    /// Integrate Ranges on a graph (T353): the Data Reader is on and its pinned point becomes a range limit on a
    /// double-click (outside the window) or Enter.
    fn range_pick_frame(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.range_pick else { return };
        let (enter, double) = ctx.input(|i| {
            let double = i.pointer.button_double_clicked(egui::PointerButton::Primary)
                && i.pointer.interact_pos().is_some_and(|p| !d.window_rect.is_some_and(|r| r.contains(p)));
            (i.key_pressed(egui::Key::Enter), double)
        });
        let state = self.graphs.entry(d.graph).or_default();
        state.reader = true;
        if (enter || double)
            && let Some(point) = state.reader_point
        {
            d.pick(self.doc.project(), point);
        }
        let graph = d.graph;
        let outcome = d.show(ctx, &mut self.doc);
        if !matches!(outcome, RangePickOutcome::Open) {
            self.range_pick = None;
            if let Some(s) = self.graphs.get_mut(&graph) {
                s.reader = false;
                s.reader_point = None;
            }
        }
        if let RangePickOutcome::Done(report) = outcome {
            self.focus(graph);
            self.logs.result(report.message.clone(), report.notes.clone());
            self.panels.reveal(PanelId::Results);
            self.say(report.message);
        }
    }

    /// Mask / Unmask Data in Region (T413): the Y cells of the layer's data points inside the drawn rectangle or
    /// ellipse are masked (or unmasked), as one undo step.
    fn mask_region(&mut self, graph: WindowId, layer: usize, shapes: Vec<usize>, mask: bool) {
        let found = match sheaf_plot::region::points_in_shapes(self.doc.project(), graph, layer, &shapes) {
            Ok(Some(found)) => found,
            Ok(None) => return self.warn("Only a rectangle or an ellipse marks a region"),
            Err(e) => return self.warn(e.to_string()),
        };
        let Some(l) = self.doc.project().graph(graph).and_then(|g| g.layers.get(layer)) else { return };
        let cells: Vec<(ColumnId, Vec<usize>)> = found.iter().map(|f| (l.plots[f.plot].y, f.rows.clone())).collect();
        let changed: usize = cells
            .iter()
            .map(|(y, rows)| {
                let masked = self.doc.project().column(*y).map(|c| rows.iter().filter(|r| c.mask.contains(r)).count());
                let masked = masked.unwrap_or(0);
                if mask { rows.len() - masked } else { masked }
            })
            .sum();
        if changed == 0 {
            let what = if found.is_empty() { "No data points inside the object" } else { "Nothing to change" };
            return self.say(what.to_string());
        }
        let label = if mask { "Mask data in region" } else { "Unmask data in region" };
        self.doc.apply(label, |p| {
            for (y, rows) in &cells {
                if let Some(c) = p.column_mut(*y) {
                    for r in rows {
                        if mask {
                            c.mask.insert(*r);
                        } else {
                            c.mask.remove(r);
                        }
                    }
                }
            }
        });
        self.say(format!("{} {changed} data point(s) in the region", if mask { "Masked" } else { "Unmasked" }));
    }

    /// Extract Data in Region (T355): the data points of the layer inside the drawn rectangle or ellipse go to a
    /// new workbook, three columns per plot with points inside (X, Y and the source row), as one undo step.
    fn extract_region(&mut self, graph: WindowId, layer: usize, shapes: Vec<usize>) {
        let found = match sheaf_plot::region::points_in_shapes(self.doc.project(), graph, layer, &shapes) {
            Ok(Some(found)) => found,
            Ok(None) => return self.warn("Only a rectangle or an ellipse marks a region"),
            Err(e) => return self.warn(e.to_string()),
        };
        if found.is_empty() {
            return self.say("No data points inside the object");
        }
        let project = self.doc.project();
        let Some(l) = project.graph(graph).and_then(|g| g.layers.get(layer)) else { return };
        let names: Vec<(String, String, String, String)> = found
            .iter()
            .map(|f| {
                let plot = &l.plots[f.plot];
                let y = project.column(plot.y);
                let x = plot.x.and_then(|x| project.column(x));
                (
                    x.map_or("Row".to_string(), |c| c.display_name().to_string()),
                    x.map_or(String::new(), |c| c.units.clone()),
                    y.map_or(String::new(), |c| c.display_name().to_string()),
                    y.map_or(String::new(), |c| c.units.clone()),
                )
            })
            .collect();
        let labels: Vec<String> = found.iter().map(|f| project.plot_label(&l.plots[f.plot])).collect();
        let total: usize = found.iter().map(|f| f.rows.len()).sum();
        let made = self.doc.edit("Extract data in region", |p| {
            let book = p.add_workbook("Extracted from region");
            for ((f, (xn, xu, yn, yu)), label) in found.iter().zip(&names).zip(&labels) {
                let comment = format!("in the region of {label}");
                let columns = [
                    (Designation::X, xn.clone(), xu.clone(), f.x.clone()),
                    (Designation::Y, yn.clone(), yu.clone(), f.y.clone()),
                    (
                        Designation::Disregard,
                        "Source row".to_string(),
                        String::new(),
                        f.rows.iter().map(|r| (r + 1) as f64).collect(),
                    ),
                ];
                for (designation, name, units, values) in columns {
                    let id =
                        p.add_column(book, 0, designation, ColumnData::numeric(values)).map_err(|e| e.to_string())?;
                    if let Some(c) = p.column_mut(id) {
                        c.long_name = name;
                        c.units = units;
                        c.comments = comment.clone();
                    }
                }
            }
            Ok::<_, String>(book)
        });
        match made {
            Ok(book) => {
                self.focus(book);
                self.say(format!("{total} data point(s) in the region copied to a new workbook"));
            }
            Err(e) => self.warn(e),
        }
    }

    /// The Integrate gadget's Preferences window (T357): X From and To, the baseline and the area type; OK applies
    /// them to the gadget (a view setting, not an undo step).
    fn gadget_prefs_frame(&mut self, ctx: &egui::Context) {
        let Some(p) = &mut self.gadget_prefs else { return };
        let mut open = true;
        let mut done = None;
        egui::Window::new("Integrate Gadget Preferences")
            .id(egui::Id::new("integrate_gadget_prefs"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("integrate_gadget_prefs_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("X From");
                    ui.add(egui::DragValue::new(&mut p.from).speed(0.01));
                    ui.end_row();
                    ui.label("X To");
                    ui.add(egui::DragValue::new(&mut p.to).speed(0.01));
                    ui.end_row();
                    ui.label("Baseline");
                    ui.vertical(|ui| {
                        ui.radio_value(&mut p.baseline, false, "Zero (Y = 0)");
                        ui.radio_value(&mut p.baseline, true, "Straight line through the end points");
                    });
                    ui.end_row();
                    ui.label("Area Type");
                    ui.vertical(|ui| {
                        ui.radio_value(&mut p.absolute, false, "Mathematical Area");
                        ui.radio_value(&mut p.absolute, true, "Absolute Area");
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                done = crate::dialogs::frame::button_row(ui, false);
            });
        let p = *p;
        match done {
            Some(crate::dialogs::frame::DialogButton::Ok)
                if p.from != p.to && p.from.is_finite() && p.to.is_finite() =>
            {
                if let Some(g) = self.graphs.get_mut(&p.graph).and_then(|s| s.gadget.as_mut()) {
                    (g.x0, g.x1, g.end_point_baseline, g.absolute) = (p.from, p.to, p.baseline, p.absolute);
                }
                self.gadget_prefs = None;
            }
            Some(crate::dialogs::frame::DialogButton::Ok) => self.warn("X From and X To must differ"),
            Some(crate::dialogs::frame::DialogButton::Cancel) => self.gadget_prefs = None,
            _ if !open => self.gadget_prefs = None,
            _ => {}
        }
    }

    /// Column > Add Column to All Sheets... (T359): one column, with an optional formula, at the end of every sheet
    /// of the workbook (or of every workbook), as one undo step.
    fn column_across_frame(&mut self, ctx: &egui::Context) {
        // Every sheet of every workbook, for choosing them one by one (T418).
        let all_sheets: Vec<((WindowId, usize), String)> = self
            .doc
            .project()
            .workbooks()
            .flat_map(|b| {
                b.sheets.iter().enumerate().map(move |(i, s)| ((b.id, i), format!("{} / {}", b.name, s.name)))
            })
            .collect();
        let Some(d) = &mut self.column_across else { return };
        let mut open = true;
        let mut pressed = None;
        egui::Window::new("Add Column to All Sheets")
            .id(egui::Id::new("column_across"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("column_across_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("Long Name");
                    ui.text_edit_singleline(&mut d.long_name);
                    ui.end_row();
                    ui.label("Units");
                    ui.text_edit_singleline(&mut d.units);
                    ui.end_row();
                    ui.label("Formula");
                    ui.text_edit_singleline(&mut d.formula).on_hover_text(
                        "Optional Set Column Values formula, run in each sheet against that sheet's columns, for example col(B)*2",
                    );
                    ui.end_row();
                    ui.label("Recalculate");
                    ui.horizontal(|ui| {
                        use sheaf_core::column::RecalcMode;
                        ui.radio_value(&mut d.recalc, RecalcMode::None, "None");
                        ui.radio_value(&mut d.recalc, RecalcMode::Auto, "Auto");
                        ui.radio_value(&mut d.recalc, RecalcMode::Manual, "Manual");
                    });
                    ui.end_row();
                    ui.label("After Column");
                    ui.text_edit_singleline(&mut d.after).on_hover_text(
                        "Optional short or long name: the column goes right after it in each sheet that has it, \
                         else at the end",
                    );
                    ui.end_row();
                    ui.label("Sheets");
                    ui.vertical(|ui| {
                        let chosen = d.chosen.is_some();
                        if ui.radio(!chosen && !d.project && !d.folder, "Every sheet of this workbook").clicked() {
                            (d.project, d.folder, d.chosen) = (false, false, None);
                        }
                        if ui.radio(!chosen && d.folder, "Every sheet of every workbook in this folder").clicked() {
                            (d.project, d.folder, d.chosen) = (false, true, None);
                        }
                        if ui.radio(!chosen && d.project && !d.folder, "Every sheet of every workbook").clicked() {
                            (d.project, d.folder, d.chosen) = (true, false, None);
                        }
                        if ui.radio(chosen, "Only the ticked sheets").clicked() && !chosen {
                            // Start from the sheets of this workbook.
                            let book = d.book;
                            d.chosen = Some(all_sheets.iter().map(|(k, _)| *k).filter(|(b, _)| *b == book).collect());
                        }
                        if let Some(list) = &mut d.chosen {
                            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                                for (key, label) in &all_sheets {
                                    let mut on = list.contains(key);
                                    if ui.checkbox(&mut on, label).changed() {
                                        if on {
                                            list.push(*key);
                                        } else {
                                            list.retain(|k| k != key);
                                        }
                                    }
                                }
                            });
                        }
                    });
                    ui.end_row();
                });
                if let Some(p) = &d.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                pressed = crate::dialogs::frame::button_row(ui, false);
            });
        match pressed {
            Some(crate::dialogs::frame::DialogButton::Ok) => {
                let d = d.clone();
                let spec = sheaf_core::across::AcrossColumn {
                    long_name: d.long_name.trim().to_string(),
                    units: d.units.trim().to_string(),
                    formula: Some(d.formula.trim().to_string()).filter(|f| !f.is_empty()),
                    recalc: d.recalc,
                    after: Some(d.after.trim().to_string()).filter(|a| !a.is_empty()),
                    ..Default::default()
                };
                let scope = if let Some(list) = &d.chosen {
                    sheaf_core::across::AcrossScope::Sheets(list.clone())
                } else if d.folder {
                    sheaf_core::across::AcrossScope::Folder(self.doc.project().folder_of(d.book))
                } else if d.project {
                    sheaf_core::across::AcrossScope::Project
                } else {
                    sheaf_core::across::AcrossScope::Workbook(d.book)
                };
                match self.doc.edit("Add column to all sheets", |p| p.add_column_across_sheets(scope, &spec)) {
                    Ok(made) => {
                        self.column_across = None;
                        self.say(format!("A column was added to {} sheet(s)", made.len()));
                    }
                    Err(e) => {
                        if let Some(d) = &mut self.column_across {
                            d.problem = Some(e.to_string());
                        }
                    }
                }
            }
            Some(crate::dialogs::frame::DialogButton::Cancel) => self.column_across = None,
            _ if !open => self.column_across = None,
            _ => {}
        }
    }

    /// Analysis > Repeat Last Analysis (T360): the last tool run from its window, with the same options, on the
    /// chosen columns (the active sheet's Y columns when none is chosen), as one undo step named after the tool.
    fn repeat_analysis(&mut self, book: WindowId, columns: Vec<ColumnId>) {
        let Some((tool, label)) = self.last_analysis.clone() else {
            return self.say("No analysis to repeat yet: run one from its window first");
        };
        let columns = if columns.is_empty() {
            let Some(sheet) = self.doc.project().workbook(book).map(|b| b.sheet()) else { return };
            sheet.columns.iter().filter(|c| c.designation == Designation::Y).map(|c| c.id).collect()
        } else {
            columns
        };
        if columns.is_empty() {
            return self.warn("Choose the columns to repeat the analysis on");
        }
        let request =
            sheaf_core::analysis::Request { columns, rows: sheaf_core::analysis::RowRange::All, tool: tool.clone() };
        match self.doc.edit(tool.name(), |p| p.analyze(&request)) {
            Ok(report) => {
                if let Some(sheet) = report.new_sheet {
                    self.show_sheet(report.book, sheet);
                }
                if let Some(graph) = report.new_graph {
                    self.focus(graph);
                }
                self.logs.result(report.message.clone(), report.notes.clone());
                self.panels.reveal(PanelId::Results);
                self.say(format!("{} repeated: {}", label.trim_end_matches("..."), report.message));
            }
            Err(e) => self.warn(format!("Could not repeat {}: {e}", label.trim_end_matches("..."))),
        }
    }

    /// Interactive formula references (T423): while Set Column Values is open, a new selection in a sheet (one
    /// column, one cell, or cells of one column) is inserted into the formula at its cursor as `col(B)`, `col(B)[3]`
    /// or `col(B)[2:5]`, with `Sheet2!` or `[Book2]Sheet1!` in front when it is not on the window's sheet.
    fn pick_formula_reference(&mut self, ctx: &egui::Context) {
        let Some(d) = &self.set_values else {
            self.formula_pick = None;
            return;
        };
        let now: std::collections::HashMap<WindowId, (crate::grid::Sel, Vec<ColumnId>, usize)> = self
            .books
            .iter()
            .map(|(id, b)| {
                let sheet = self.doc.project().workbook(*id).map_or(0, |w| w.active_sheet);
                (*id, (b.sel.clone(), b.selected.clone(), sheet))
            })
            .collect();
        let Some(before) = self.formula_pick.replace(now.clone()) else { return };
        let target = d.target();
        let project = self.doc.project();
        let Some((home_book, home_sheet, _)) = project.locate(target) else { return };
        let home = (home_book.id, home_sheet as *const sheaf_core::Worksheet);
        let mut picked: Option<String> = None;
        for (book, (sel, selected, sheet_index)) in &now {
            if before.get(book) == Some(&(sel.clone(), selected.clone(), *sheet_index)) {
                continue;
            }
            let Some(wb) = project.workbook(*book) else { continue };
            let Some(sheet) = wb.sheets.get(*sheet_index) else { continue };
            let (column, rows) = match sel {
                crate::grid::Sel::Columns if selected.len() == 1 => (sheet.column(selected[0]), None),
                crate::grid::Sel::Cells { anchor, end } if anchor.0 == end.0 => {
                    let (lo, hi) = (anchor.1.min(end.1), anchor.1.max(end.1));
                    (sheet.columns.get(anchor.0), Some((lo + 1, hi + 1)))
                }
                _ => continue,
            };
            let Some(column) = column else { continue };
            let mut text = String::new();
            if (wb.id, sheet as *const sheaf_core::Worksheet) != home {
                if wb.id != home.0 {
                    text.push_str(&format!("[{}]", wb.name));
                }
                text.push_str(&format!("{}!", sheet.name));
            }
            text.push_str(&format!("col({})", column.short_name));
            match rows {
                Some((a, b)) if a == b => text.push_str(&format!("[{a}]")),
                Some((a, b)) => text.push_str(&format!("[{a}:{b}]")),
                None => {}
            }
            picked = Some(text);
        }
        if let (Some(text), Some(d)) = (picked, &mut self.set_values) {
            d.insert_reference(ctx, &text);
        }
    }

    /// Analysis > Repeat Last Analysis on All Sheets (T414): the last tool, with the same options, on the Y columns of
    /// every sheet the workbook has when the command starts (not the sheets the runs add), as one undo step. A sheet
    /// without Y columns, or where the tool fails, is skipped and named.
    fn repeat_analysis_all_sheets(&mut self, book: WindowId) {
        let Some((tool, label)) = self.last_analysis.clone() else {
            return self.say("No analysis to repeat yet: run one from its window first");
        };
        let Some(b) = self.doc.project().workbook(book) else { return };
        let sheets: Vec<(String, Vec<ColumnId>)> = b
            .sheets
            .iter()
            .map(|s| {
                let ys = s.columns.iter().filter(|c| c.designation == Designation::Y).map(|c| c.id).collect();
                (s.name.clone(), ys)
            })
            .collect();
        let name = label.trim_end_matches("...").to_string();
        let mut skipped: Vec<String> = Vec::new();
        let result = self.doc.edit(tool.name(), |p| {
            let mut runs = 0;
            for (sheet, columns) in &sheets {
                if columns.is_empty() {
                    skipped.push(format!("{sheet} (no Y columns)"));
                    continue;
                }
                let request = sheaf_core::analysis::Request {
                    columns: columns.clone(),
                    rows: sheaf_core::analysis::RowRange::All,
                    tool: tool.clone(),
                };
                match p.analyze(&request) {
                    Ok(_) => runs += 1,
                    Err(e) => skipped.push(format!("{sheet} ({e})")),
                }
            }
            if runs == 0 { Err(format!("{name} ran on no sheet")) } else { Ok(runs) }
        });
        match result {
            Ok(runs) => {
                self.panels.reveal(PanelId::Results);
                let mut msg = format!("{name} repeated on {runs} sheet(s)");
                if !skipped.is_empty() {
                    msg.push_str(&format!("; skipped: {}", skipped.join(", ")));
                }
                self.say(msg);
            }
            Err(e) => {
                let detail = if skipped.is_empty() { String::new() } else { format!(": {}", skipped.join(", ")) };
                self.warn(format!("Could not repeat {name}: {e}{detail}"));
            }
        }
    }

    /// File > Find in Project Files... (T361): Open closes the window and opens the chosen project.
    fn find_files_frame(&mut self, ctx: &egui::Context) {
        // The project opened from the window (T449): go to its first hit, as Find in Project's Go does.
        if let Some(spec) = self.jump_now.take() {
            let spec = crate::project_find::FindSpec { within: crate::project_find::FindScope::EntireProject, ..spec };
            let hits = crate::project_find::find_in_project(self.doc.project(), &spec, &[]);
            if let Some(first) = hits.first() {
                let (n, what) = (hits.len(), format!("{} {} {}", first.element, first.attribute, first.range));
                self.go_to_found(first.target.clone(), ctx);
                self.say(format!("{n} hit(s) for \"{}\"; the first: {what}", spec.text));
            }
        }
        let Some(d) = &mut self.find_files else { return };
        match d.show(ctx) {
            crate::dialogs::find_files::FindFilesOutcome::Open => {}
            crate::dialogs::find_files::FindFilesOutcome::Closed => self.find_files = None,
            crate::dialogs::find_files::FindFilesOutcome::OpenFile(path) => {
                let spec = self.find_files.as_ref().map(|d| d.spec.clone());
                self.find_files = None;
                self.jump_after_open = spec.map(|s| (path.clone(), s));
                self.open_picked(path);
            }
        }
    }

    fn show_sheet(&mut self, book: WindowId, sheet: usize) {
        self.doc.adjust_view(|p| {
            if let Some(b) = p.workbook_mut(book) {
                b.active_sheet = sheet.min(b.sheets.len().saturating_sub(1));
            }
        });
        if let Some(state) = self.books.get_mut(&book) {
            state.clear_selection();
        }
        self.focus(book);
    }

    /// Puts the files an export wrote into Recent Exports, each with the short name of its window (unique, and unchanged by a
    /// rename of the long name or title display).
    fn record_exports(&mut self, kind: ExportKind, files: Vec<(WindowId, PathBuf)>) {
        for (id, path) in files.into_iter().rev() {
            let Some(window) = self.doc.project().windows.iter().find(|w| w.id() == id).map(|w| w.name().to_string())
            else {
                continue;
            };
            if let Some(why) = self.recent.push_export(&ExportEntry { path, kind, window }) {
                self.logs.warn(why);
            }
        }
    }

    /// File > Export > Recent Exports: opens the export that wrote the file again, on the window it
    /// was made from and in the file's folder. A window that is not in the project any more is
    /// reported and nothing opens (Sheaf's choice, docs/spec/recent-lists.md 3).
    fn open_recent_export(&mut self, entry: &ExportEntry) {
        let title = "Recent Exports".to_string();
        let project = self.doc.project();
        let Some(id) = project.windows.iter().find(|w| w.name() == entry.window).map(|w| w.id()) else {
            let text = format!("The window '{}' is not in this project, so its export cannot be opened.", entry.window);
            self.message = Some((title, text));
            return;
        };
        let folder = entry.path.parent().map(Path::to_path_buf).or_else(|| self.folder());
        match entry.kind {
            ExportKind::Graph if project.graph(id).is_some() => {
                self.export = Some(ExportDialog::new(id, folder).with_themes(self.startup.themes_dir.clone()));
            }
            ExportKind::Sheet if project.workbook(id).is_some() => {
                self.export_sheet = Some(SheetExportDialog::new(id, folder));
            }
            ExportKind::Excel if project.workbook(id).is_some() => {
                let sheets = project.workbook(id).map_or(0, |b| b.sheets.len());
                self.export_excel = Some(ExcelExportDialog::new(id, sheets, folder));
            }
            _ => {
                let text = format!("'{}' is not the kind of window that wrote {}.", entry.window, entry.path.display());
                self.message = Some((title, text));
                return;
            }
        }
        self.focus(id);
    }

    /// File > Import > Re-Import Directly and Re-Import... (T98): reads the book's source file (or, for
    /// Re-Import..., a file picked starting from it) again and replaces the data, one undo step.
    /// The Matrix menu (T192 slice 3): the operations without settings run at once as one undo step;
    /// the others open their window.
    fn matrix_cmd(&mut self, id: WindowId, cmd: crate::menus::MatrixCmd) {
        use crate::dialogs::matrix_ops::{Form, MatrixOpDialog};
        use crate::menus::MatrixCmd as M;
        use sheaf_core::matrix::{FlipDirection, Rotation};
        use sheaf_core::matrix_project::MatrixOp;
        let Some(sheet) = self.doc.project().matrix_book(id).map(|b| b.active_sheet) else { return };
        let form = match cmd {
            M::SetValues => Some(Form::Values { formula: String::new() }),
            M::Dimensions => Some(Form::Dimensions { cols: 0, rows: 0, x: (0.0, 0.0), y: (0.0, 0.0) }),
            M::Expand => Some(Form::Expand { cols: 2, rows: 2 }),
            M::Shrink => Some(Form::Shrink { cols: 2, rows: 2 }),
            _ => None,
        };
        if let Some(form) = form {
            self.matrix_dialog = MatrixOpDialog::new(&self.doc, id, form);
            return;
        }
        let op = match cmd {
            M::Transpose => MatrixOp::Transpose,
            M::RotateCcw => MatrixOp::Rotate(Rotation::Ccw90),
            M::RotateCw => MatrixOp::Rotate(Rotation::Cw90),
            M::Rotate180 => MatrixOp::Rotate(Rotation::Half),
            M::FlipHorizontal => MatrixOp::Flip { direction: FlipDirection::Horizontal, coordinates: false },
            M::FlipVertical => MatrixOp::Flip { direction: FlipDirection::Vertical, coordinates: false },
            M::SurfaceFit => {
                // The active sheet becomes X, Y and Z columns, then Origin's surface-fit window opens
                // on them (T221).
                use sheaf_core::matrix_convert::{M2wMethod, XyzOrder};
                let method = M2wMethod::Xyz { order: XyzOrder::XConstFirst, skip_missing: true };
                match self.doc.edit("Matrix to XYZ for surface fit", |p| p.matrix_to_worksheet(id, sheet, &method)) {
                    Ok(book) => {
                        self.fit =
                            FitDialog::new(self.doc.project(), book, None, crate::dialogs::fit::FitKind::Surface);
                        if self.fit.is_none() {
                            self.warn("The matrix has no numbers to fit");
                        }
                    }
                    Err(e) => self.warn(e.to_string()),
                }
                return;
            }
            M::Fft2 | M::Ifft2 => {
                let inverse = cmd == M::Ifft2;
                let label = if inverse { "2D IFFT" } else { "2D FFT" };
                match self.doc.edit(label, |p| p.matrix_fft2(id, sheet, inverse)) {
                    Ok(_) => self.say(format!("{label}: the result is in a new matrix book")),
                    Err(e) => self.warn(e.to_string()),
                }
                return;
            }
            M::Inverse => {
                match self.doc.edit("Matrix Inverse", |p| p.matrix_inverse(id, sheet)) {
                    Ok((_, false)) => self.say("Inverted the matrix into a new matrix book".to_string()),
                    // Origin warns in a message box; Sheaf on the status bar.
                    Ok((_, true)) => self.warn(
                        "The matrix is not square or not of full rank: its pseudoinverse is in a new matrix book"
                            .to_string(),
                    ),
                    Err(e) => self.warn(e.to_string()),
                }
                return;
            }
            M::ToWorksheet => {
                use sheaf_core::matrix_convert::{LabelSource, M2wMethod, XyLayout};
                let method = M2wMethod::Direct {
                    layout: XyLayout::None,
                    labels: LabelSource::None,
                    coords_in_first_column: false,
                };
                if let Err(e) = self.doc.edit("Convert to Worksheet", |p| p.matrix_to_worksheet(id, sheet, &method)) {
                    self.warn(e.to_string());
                }
                return;
            }
            _ => return,
        };
        if let Err(e) = crate::dialogs::matrix_ops::apply(&mut self.doc, id, sheet, &op) {
            self.warn(e);
        }
    }

    /// The Image menu (T207): the spatial filters run at once; the others open their window.
    fn image_cmd(&mut self, id: WindowId, cmd: crate::menus::ImageCmd) {
        use crate::dialogs::matrix_ops::{Form, ImageForm, MatrixOpDialog};
        use crate::menus::ImageCmd as I;
        use sheaf_core::matrix::{FlipDirection, Rotation};
        use sheaf_core::matrix_image::{ImageOp, MathOp};
        let Some(b) = self.doc.project().matrix_book(id) else { return };
        let sheet = b.active_sheet;
        let mean = b.sheets.get(sheet).map_or(0.0, |s| {
            let v: Vec<f64> = s.values().iter().copied().filter(|x| x.is_finite()).collect();
            v.iter().sum::<f64>() / v.len().max(1) as f64
        });
        let max = b.sheets.get(sheet).and_then(|s| s.z_range()).map_or(mean, |r| r.1);
        let form = match cmd {
            I::Brightness => ImageForm::Brightness(10.0),
            I::Contrast => ImageForm::Contrast(1.5),
            I::Gamma => ImageForm::Gamma(1.5),
            I::Math => ImageForm::Math(MathOp::Add, 0.0),
            I::ToBinary => ImageForm::ToBinary(mean, max),
            I::Rotate => ImageForm::Rotate(Rotation::Ccw90),
            I::Flip => ImageForm::Flip(FlipDirection::Horizontal),
            // Origin 2016's imgMedian default window is 2 (T284).
            I::Median => ImageForm::Median(2),
            I::Levels => {
                let (lo, hi) = b.sheets.get(sheet).and_then(|s| s.z_range()).unwrap_or((0.0, 255.0));
                ImageForm::Levels { input: (lo, hi), output: (0.0, 255.0) }
            }
            I::Crop => {
                let (rows, cols) = b.sheets.get(sheet).map_or((1, 1), |s| (s.rows.max(1), s.cols.max(1)));
                ImageForm::Crop { rows: (1, rows), cols: (1, cols) }
            }
            I::Resize => {
                let (rows, cols) = b.sheets.get(sheet).map_or((1, 1), |s| (s.rows.max(1), s.cols.max(1)));
                ImageForm::Resize { rows, cols, bilinear: false }
            }
            I::Subtract => {
                let choices: Vec<(WindowId, String)> =
                    self.doc.project().matrix_books().filter(|m| m.id != id).map(|m| (m.id, m.name.clone())).collect();
                ImageForm::Subtract { choices, pick: 0 }
            }
            I::ColorToGray | I::RgbSplit => {
                use sheaf_core::matrix_image::{color_to_gray, rgb_sheets};
                let split = cmd == I::RgbSplit;
                let book_name = b.name.clone();
                let Some(rgb) = rgb_sheets(&b.sheets).map(|s| s.map(|x| x.clone())) else {
                    self.warn("This matrix book has no Red, Green and Blue sheets of a colour picture");
                    return;
                };
                let label = if split { "RGB Split" } else { "Color to Gray" };
                let _ = self.doc.edit(label, |p| {
                    if split {
                        for s in &rgb {
                            p.adopt_matrix_book(&format!("{} of {book_name}", s.name), vec![s.clone()]);
                        }
                    } else {
                        let [r, g, bl] = &rgb;
                        p.adopt_matrix_book(&format!("Gray of {book_name}"), vec![color_to_gray([r, g, bl])]);
                    }
                    Ok::<_, String>(())
                });
                return;
            }
            // Origin's imgEdge threshold defaults to 0 (T285); its default filter (Laplace) is not built.
            I::EdgeDetection => ImageForm::Edge(sheaf_core::matrix_image::EdgeFilter::SobelBoth, 0.0),
            I::Smooth | I::Sharpen => {
                let op = if cmd == I::Smooth { ImageOp::Smooth } else { ImageOp::Sharpen };
                if let Err(e) = crate::dialogs::matrix_ops::apply_image(&mut self.doc, id, sheet, &op) {
                    self.warn(e);
                }
                return;
            }
        };
        self.matrix_dialog = MatrixOpDialog::new(&self.doc, id, Form::Image(form));
    }

    fn reimport(&mut self, book: WindowId, choose: bool) {
        let Some(b) = self.doc.project().workbook(book) else { return };
        let Some((source, kind)) = b.source.clone().zip(reimport_kind(b)) else {
            self.warn("This workbook was not imported from a file");
            return;
        };
        let mut path = PathBuf::from(&source.path);
        if choose {
            let mut chooser = rfd::FileDialog::new().set_title("Re-Import");
            if let Some(dir) = path.parent().filter(|d| d.is_dir()) {
                chooser = chooser.set_directory(dir);
            }
            if let Some(name) = path.file_name() {
                chooser = chooser.set_file_name(name.to_string_lossy());
            }
            let Some(picked) = chooser.pick_file() else { return };
            path = picked;
        }
        match reimport_book(&mut self.doc, book, &path, kind) {
            Ok(r) => {
                let mut s = format!("Re-imported {}: {} column(s), {} row(s)", path.display(), r.columns, r.rows);
                if r.ignored > 0 {
                    s.push_str(&format!("; {} more column(s) in the file were not added", r.ignored));
                }
                if r.emptied > 0 {
                    s.push_str(&format!("; {} column(s) the file no longer has are empty", r.emptied));
                }
                self.say(s);
            }
            Err(e) => self.message = Some(("Could not re-import".to_string(), e)),
        }
    }

    /// File > Export > Worksheet as Image (T97): the active sheet in the supported image formats.
    fn export_worksheet_image(&mut self, book: WindowId) {
        let Some(b) = self.doc.project().workbook(book) else { return };
        let sheet = b.sheet();
        let stem = if b.long_name.is_empty() { b.name.clone() } else { b.long_name.clone() };
        let mut chooser = rfd::FileDialog::new()
            .set_title("Export worksheet as image")
            .add_filter("PNG image", &["png"])
            .add_filter("JPEG image", &["jpg", "jpeg"])
            .add_filter("TIFF image", &["tif", "tiff"])
            .add_filter("Windows bitmap", &["bmp"])
            .add_filter("GIF image", &["gif"])
            .add_filter("Targa image", &["tga"])
            .add_filter("PCX image", &["pcx"])
            .add_filter("Photoshop document", &["psd"])
            .add_filter("PDF", &["pdf"])
            .add_filter("SVG", &["svg"])
            .add_filter("Encapsulated PostScript", &["eps"])
            .add_filter("Enhanced Metafile", &["emf"])
            .set_file_name(format!("{stem}_{}.png", sheet.name));
        if let Some(dir) = self.folder() {
            chooser = chooser.set_directory(dir);
        }
        let Some(path) = chooser.save_file() else { return };
        match write_worksheet_image(sheet, &path) {
            Ok(path) => self.say(format!("Exported the sheet to {}", path.display())),
            Err(e) => self.message = Some(("Could not export the sheet".to_string(), e)),
        }
    }

    fn export_data(&mut self, book: WindowId) {
        if self.doc.project().workbook(book).is_some() {
            // T247: the active sheet's selection, for Selected cells only.
            let selection = self.doc.project().workbook(book).and_then(|b| {
                let t = self.books.get(&book)?.target(b.sheet())?;
                Some((t.columns, t.rows))
            });
            self.export_sheet = Some(SheetExportDialog::new(book, self.folder()).with_selection(selection));
        }
    }

    // ----- drawing -------------------------------------------------------

    /// What the menus need to know about the program right now.
    fn menu_context(&self) -> MenuContext<'_> {
        let project = self.doc.project();
        let book = self.active.filter(|id| project.workbook(*id).is_some());
        let graph = self.active.filter(|id| project.graph(*id).is_some());
        let kind = match (book, graph) {
            (Some(_), _) => WindowKind::Workbook,
            (_, Some(_)) => WindowKind::Graph,
            _ if self.active.is_some_and(|id| project.matrix_book(id).is_some()) => WindowKind::Matrix,
            _ if self.active.is_some_and(|id| project.layout(id).is_some()) => WindowKind::Layout,
            _ => WindowKind::None,
        };
        let selected: &[ColumnId] = book.and_then(|b| self.books.get(&b)).map(|s| s.selected.as_slice()).unwrap_or(&[]);
        let layer = graph.and_then(|g| self.graphs.get(&g)).map_or(0, |s| s.active_layer);
        MenuContext {
            kind,
            book,
            graph,
            active: self.active,
            selected,
            sorting_columns: book
                .and_then(|id| {
                    let sheet = project.workbook(id)?.sheet();
                    let target = self.books.get(&id)?.target(sheet)?;
                    Some(target.columns.iter().map(|&i| sheet.columns[i].id).collect())
                })
                .unwrap_or_default(),
            cells: book.and_then(|b| self.books.get(&b)).is_some_and(|s| s.sel != Sel::None),
            whole_columns: book
                .and_then(|b| self.books.get(&b))
                .is_some_and(|s| matches!(s.sel, Sel::Columns | Sel::All)),
            imported: book.and_then(|b| project.workbook(b)).is_some_and(|b| reimport_kind(b).is_some()),
            undo: self.doc.undo_label(),
            redo: self.doc.redo_label(),
            undo_ai: self.can_undo_ai_changes(),
            windows: {
                let shown = explorer::shown_windows(project, &self.panels.explorer.folder);
                project
                    .windows
                    .iter()
                    .filter(|w| shown.as_ref().is_none_or(|s| s.contains(&w.id())))
                    .map(|w| (w.id(), w.title()))
                    .collect()
            },
            folders: explorer::folder_list(&project.root),
            folder: self.panels.explorer.folder.clone(),
            view_windows: project.view_windows,
            open: self.open_menu.as_deref(),
            recent_projects: self.recent.projects(),
            recent_imports: self.recent.imports(),
            recent_books: self.recent.books(),
            recent_graphs: self.recent.graphs(),
            recent_commands: self.recent.commands(),
            recent_exports: self.recent.exports(),
            user_templates: &self.user_templates,
            user_book_templates: &self.user_book_templates,
            templates: self.startup.templates_dir.is_some(),
            panels: self.panels.shown_flags(),
            ai_link: self.live.active(),
            layer,
            tool: graph.and_then(|g| self.graphs.get(&g)).map_or(GraphTool::Pointer, |s| s.tool()),
            curve: graph.and_then(|g| self.graph_curve(g)),
            graph_pick: graph.and_then(|g| self.graphs.get(&g)).and_then(|s| s.selected).map(|(_, k)| k),
            graph_paste: crate::shape_clip::has(),
            layer_curves: graph.map(|g| self.graph_layer_curves(g)).unwrap_or_default(),
            stack: graph.and_then(|g| {
                let layer = self.graphs.get(&g).map_or(0, |s| s.active_layer);
                project.graph(g)?.layers.get(layer).map(|l| l.stack)
            }),
            selected_shape: graph.and_then(|g| {
                let (layer, sheaf_plot::scene::ObjectKind::Shape(shape)) = self.graphs.get(&g)?.selected? else {
                    return None;
                };
                Some((layer, shape, project.graph(g)?.layers.get(layer)?.shapes.len()))
            }),
            multi_selected: graph.and_then(|g| self.graphs.get(&g)).map_or(0, |s| s.multi_selection().len()),
            can_group: graph.is_some_and(|g| {
                let selection = self.graphs.get(&g).map(|s| s.multi_selection()).unwrap_or_default();
                project.graph(g).is_some_and(|gr| crate::obj_align::can_group(gr, &selection))
            }),
            can_ungroup: graph.is_some_and(|g| {
                let selection = self.graphs.get(&g).map(|s| s.multi_selection()).unwrap_or_default();
                project.graph(g).is_some_and(|gr| crate::obj_align::can_ungroup(gr, &selection))
            }),
            view_mode: graph.map(|g| project.view_mode(g)).unwrap_or_default(),
            layer_icons: graph.and_then(|g| project.graph(g)).is_none_or(|g| g.page.style.layer_icons),
            active_layer_indicator: graph
                .and_then(|g| project.graph(g))
                .is_some_and(|g| g.page.style.active_layer_indicator),
            axis_layer_icons: graph.and_then(|g| project.graph(g)).is_some_and(|g| g.page.style.axis_layer_icons),
            page_grid: graph.and_then(|g| project.graph(g)).map(|g| g.page.style.grid).unwrap_or_default(),
            master_items_on_screen: graph.and_then(|id| {
                let g = project.graph(id)?;
                project.master_graph_for(g).map(|_| g.page.style.master_items_on_screen)
            }),
            show_frame: graph
                .and_then(|g| project.graph(g)?.layers.get(layer))
                .filter(|l| l.coords.is_cartesian())
                .map(|l| l.display.show_frame),
            lock_position: graph.and_then(|g| project.graph(g)).map(|g| g.page.style.lock_position).unwrap_or_default(),
            page_breaks: book
                .and_then(|b| project.workbook(b))
                .and_then(|b| b.sheets.get(b.active_sheet))
                .is_some_and(|s| s.view.page_breaks),
            activate_by_icon_only: graph
                .and_then(|g| project.graph(g))
                .is_some_and(|g| g.page.style.activate_by_icon_only),
            icon_double_click_plot_setup: graph
                .and_then(|g| project.graph(g))
                .is_some_and(|g| g.page.style.icon_double_click_plot_setup),
            show_labels: graph
                .and_then(|g| project.graph(g))
                .is_none_or(|g| g.layers.iter().all(|l| l.display.show_labels)),
            show_data: graph
                .and_then(|g| project.graph(g))
                .is_none_or(|g| g.layers.iter().all(|l| l.display.show_data)),
            active_layer_only: graph.and_then(|id| {
                let g = project.graph(id)?;
                let layer = self.graphs.get(&id).map_or(0, |s| s.active_layer);
                if g.layers.len() < 2 || layer >= g.layers.len() {
                    return None;
                }
                Some(g.layers.iter().enumerate().all(|(i, l)| l.display.hidden == (i != layer)))
            }),
            space3d: graph.is_some_and(|g| {
                let layer = self.graphs.get(&g).map_or(0, |s| s.active_layer);
                project.graph(g).and_then(|gr| gr.layers.get(layer)).is_some_and(|l| l.coords.space3d().is_some())
            }),
            can_color_scale: graph.is_some_and(|g| {
                let layer = self.graphs.get(&g).map_or(0, |s| s.active_layer);
                project.graph(g).and_then(|gr| gr.layers.get(layer)).is_some_and(|l| {
                    sheaf_core::layer_scale::LayerScale::applies(l, sheaf_core::layer_scale::LayerScaleKind::Color)
                })
            }),
            can_cumulate: graph.is_some_and(|g| {
                let layer = self.graphs.get(&g).map_or(0, |s| s.active_layer);
                project.graph(g).and_then(|gr| gr.layers.get(layer)).is_some_and(sheaf_core::stack::layer_cumulates)
            }),
        }
    }

    /// The data the Analysis menu works on in graph `g` (see `MenuContext::curve`): the selected
    /// plot, or else the first visible plot of numbers in the active layer; with the workbook that
    /// holds its Y column.
    fn graph_curve(&self, g: WindowId) -> Option<(WindowId, ColumnId)> {
        let project = self.doc.project();
        let graph = project.graph(g)?;
        let state = self.graphs.get(&g);
        let selected = state.and_then(|s| match s.selected {
            Some((layer, sheaf_plot::scene::ObjectKind::Plot(i))) => graph.layers.get(layer)?.plots.get(i).map(|p| p.y),
            _ => None,
        });
        let layer = state.map_or(0, |s| s.active_layer);
        let y = selected.or_else(|| {
            graph.layers.get(layer)?.plots.iter().find(|p| p.visible && project.numeric(p.y).is_some()).map(|p| p.y)
        })?;
        let (book, _, _) = project.locate(y)?;
        Some((book.id, y))
    }

    /// The Y columns of the visible plots of numbers in the active layer of graph `g` that sit in the
    /// same worksheet as the plot the Analysis menu works on (see `MenuContext::layer_curves`); that
    /// plot comes first.
    fn graph_layer_curves(&self, g: WindowId) -> Vec<ColumnId> {
        let project = self.doc.project();
        let (Some(graph), Some((_, first))) = (project.graph(g), self.graph_curve(g)) else {
            return Vec::new();
        };
        let layer = self.graphs.get(&g).map_or(0, |s| s.active_layer);
        let sheet_of = |y: ColumnId| {
            project.locate(y).and_then(|(b, s, _)| b.sheets.iter().position(|w| std::ptr::eq(w, s)).map(|i| (b.id, i)))
        };
        let here = sheet_of(first);
        let mut out = vec![first];
        if let Some(l) = graph.layers.get(layer) {
            for p in &l.plots {
                if p.visible
                    && p.y != first
                    && project.numeric(p.y).is_some()
                    && sheet_of(p.y) == here
                    && !out.contains(&p.y)
                {
                    out.push(p.y);
                }
            }
        }
        out
    }

    /// Cell-format keys take precedence over graph-only keys such as Ctrl+I.
    fn worksheet_format_keys(&mut self, ctx: &egui::Context) {
        use crate::text_format::Change;
        while let Some((index, key)) = ctx.input(|i| {
            i.events.iter().enumerate().find_map(|(index, event)| match event {
                egui::Event::Key { key, pressed: true, modifiers, .. }
                    if matches!(key, egui::Key::B | egui::Key::I | egui::Key::U)
                        && modifiers.command
                        && !modifiers.shift
                        && !modifiers.alt =>
                {
                    Some((index, *key))
                }
                _ => None,
            })
        }) {
            let Some(state) = self.text_format().filter(|s| s.worksheet.is_some()) else { break };
            // Consume individual presses so repeated/different shortcuts keep their event order.
            ctx.input_mut(|i| {
                i.events.remove(index);
            });
            let change = match key {
                egui::Key::B => Change::Bold(!state.font.bold),
                egui::Key::I => Change::Italic(!state.font.italic),
                egui::Key::U => Change::Underline(!state.worksheet.expect("worksheet key target").underline),
                _ => unreachable!(),
            };
            self.apply(Action::FormatText(state.request(change)), ctx);
        }
    }

    /// Menu shortcuts and contextual worksheet cell-format keys.
    fn shortcuts(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        // A widget with the keyboard is a text field, except the grid itself (it takes the
        // keyboard when a cell is clicked so that Tab and the arrows stay with it).
        self.switcher_keys(ctx, actions);
        let grids: HashSet<egui::Id> = self.books.keys().map(|b| grid::grid_id(*b)).collect();
        let typing = ctx.memory(|m| m.focused()).is_some_and(|id| !grids.contains(&id));
        // Delete on a graph removes its selected object (a legend, a title, a text label).
        if !typing
            && let Some(graph) = self.active.filter(|id| self.doc.project().graph(*id).is_some())
            && let Some(state) = self.graphs.get_mut(&graph)
            && state.selected.is_some()
            && ctx.input(|i| i.key_pressed(egui::Key::Delete))
        {
            if let Some(op) = state.delete_selected() {
                actions.push(Action::GraphEdit { graph, op });
            }
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
        }
        // Delete on a layout page removes its selected item (T237 slice 2).
        if !typing
            && let Some(layout) = self.active.filter(|id| self.doc.project().layout(*id).is_some())
            && let Some(state) = self.layouts.get_mut(&layout)
            && let Some(index) = state.selected
            && ctx.input(|i| i.key_pressed(egui::Key::Delete))
        {
            state.selected = None;
            actions.push(Action::LayoutDeleteItem { layout, index });
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
        }
        // Ctrl+X, Ctrl+C and Ctrl+V on a graph cut, copy and paste its selected object (T149); they
        // reach the program as events of their own.
        if !typing && !self.dialog_open() && self.active.is_some_and(|id| self.doc.project().graph(id).is_some()) {
            for event in ctx.input(|i| i.events.clone()) {
                let op = match event {
                    egui::Event::Cut => menus::GraphClip::Cut,
                    egui::Event::Copy => menus::GraphClip::Copy,
                    egui::Event::Paste(_) => menus::GraphClip::Paste,
                    _ => continue,
                };
                if let Ok(action) = menus::resolve(&menus::Cmd::GraphClipboard(op), &self.menu_context()) {
                    actions.push(action);
                }
            }
        }
        // F2, Delete and Ctrl+A belong to the Project Explorer while it has the keyboard.
        if !typing && !self.dialog_open() {
            explorer::handle_keys(ctx, &mut self.panels.explorer, self.doc.project(), actions);
        }
        if !typing && !self.panels.explorer.focus && !egui::Popup::is_any_open(ctx) && !self.dialog_open() {
            self.worksheet_format_keys(ctx);
        }
        // The cursor keys, Tab, Enter and typing belong to the cells of the active worksheet,
        // unless a text field, a menu or a dialog has the keyboard.
        if !typing
            && !self.panels.explorer.focus
            && !egui::Popup::is_any_open(ctx)
            && !self.dialog_open()
            && let Some(book) = self.active.filter(|id| self.doc.project().workbook(*id).is_some())
            && let Some(state) = self.books.get_mut(&book)
        {
            grid::handle_keys(ctx, &self.doc, book, state, actions);
        }
        let menu_ctx = self.menu_context();
        // Ctrl+C, Ctrl+X and Ctrl+V reach the program as events of their own, not as keys. They
        // belong to the cells unless a text field has the keyboard.
        if !typing && let Some(book) = menu_ctx.book {
            let events = ctx.input(|i| i.events.clone());
            // winit consumes Ctrl+Alt+C and Ctrl+Shift+C as Copy events too, without a C
            // key event. Keep Origin's variants reachable through the native shortcuts.
            let copy_kind = ctx.input(|i| {
                if i.modifiers.alt {
                    CopyKind::FullPrecision
                } else if i.modifiers.shift {
                    CopyKind::WithLabels
                } else {
                    CopyKind::Plain
                }
            });
            for event in events {
                let op = match event {
                    egui::Event::Copy => CellOp::Copy(copy_kind),
                    egui::Event::Cut => CellOp::Cut,
                    egui::Event::Paste(text) => CellOp::PasteText(text),
                    _ => continue,
                };
                if menu_ctx.cells {
                    actions.push(Action::Cells { book, op });
                }
            }
        }
        ctx.input_mut(|i| {
            for (keys, cmd) in menus::shortcuts() {
                if typing && !menus::works_while_typing(&cmd) {
                    continue;
                }
                // Ctrl+G is Go to in a workbook and Export Graphs in a graph, Ctrl+J Copy formula
                // and Copy Page (as in Origin): a key goes to the command that can run now.
                let shared = matches!(
                    cmd,
                    menus::Cmd::GoToCell
                        | menus::Cmd::ExportGraph
                        | menus::Cmd::CopyPage
                        | menus::Cmd::Copy(crate::actions::CopyKind::Part(sheaf_core::clipboard::CopyPart::Formula))
                );
                let action = menus::resolve(&cmd, &menu_ctx);
                if shared && action.is_err() {
                    continue;
                }
                if i.consume_shortcut(&keys)
                    && let Ok(action) = action
                {
                    actions.push(action);
                }
            }
        });
    }

    /// True while a dialog or a message box is open.
    fn dialog_open(&self) -> bool {
        self.import.is_some()
            || self.import_excel.is_some()
            || self.import_wizard.is_some()
            || self.graph_props.is_some()
            || self.layer_contents.is_some()
            || self.layer_management.is_some()
            || self.add_layer_dialog.is_some()
            || self.fit_page.is_some()
            || self.matrix_dialog.is_some()
            || self.layout_add_graph.is_some()
            || self.layout_page_setup.is_some()
            || self.layout_text.is_some()
            || self.layout_shape.is_some()
            || self.import_bin2d.is_some()
            || self.axis3d.is_some()
            || self.plot_setup.is_some()
            || self.error_bars.is_some()
            || self.trendline.is_some()
            || self.stack_panels.is_some()
            || self.merge_graphs.is_some()
            || self.label_panels.is_some()
            || self.simulate.is_some()
            || self.simulate_surface.is_some()
            || self.column_props.is_some()
            || self.text_dialog.is_some()
            || self.shape_dialog.is_some()
            || self.slope_dialog.is_some()
            || self.apply_formats.is_some()
            || self.straight_line_dialog.is_some()
            || self.comparison_plot.is_some()
            || self.multi_y.is_some()
            || self.set_values.is_some()
            || self.analysis.is_some()
            || self.fit.is_some()
            || self.fit_more.is_some()
            || self.filter.is_some()
            || self.row_filter.is_some()
            || self.mask_condition.is_some()
            || self.cond_format.is_some()
            || self.cond_manager.is_some()
            || self.cell_note.is_some()
            || self.cell_format.is_some()
            || self.worksheet_fonts.is_some()
            || self.worksheet_color.is_some()
            || self.reshape.is_some()
            || self.pivot.is_some()
            || self.multi_peak.is_some()
            || self.subtract_pick.is_some()
            || self.cv_pick.is_some()
            || self.range_pick.is_some()
            || self.export.is_some()
            || self.export_sheet.is_some()
            || self.export_excel.is_some()
            || self.page_setup.is_some()
            || self.print_preview.is_some()
            || self.rename.is_some()
            || self.go_to.is_some()
            || self.template_prompt.is_some()
            || self.template_manager.is_some()
            || self.fill_pattern.is_some()
            || self.sampling.is_some()
            || self.copy_columns.is_some()
            || self.sheet_navigate.is_some()
            || self.find_project.is_some()
            || self.batch_plot.is_some()
            || self.folder_props.is_some()
            || self.save_theme.is_some()
            || self.theme_organizer.is_some()
            || self.system_templates.is_some()
            || self.fit_organizer.is_some()
            || self.filter_manager.is_some()
            || self.show_x.is_some()
            || self.power.is_some()
            || self.find.is_some()
            || self.sort.is_some()
            || self.sheet_name.is_some()
            || self.add_columns.is_some()
            || self.param_row.is_some()
            || self.about
            || self.message.is_some()
            || self.pending.is_some()
            || self.autosave_dialog.is_some()
            || self.recovery_offer.is_some()
            || self.switcher.is_some()
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let menu_ctx = self.menu_context();
        menus::set_masking_disabled(ui.ctx(), self.doc.project().masking_disabled());
        menus::set_masked_hidden(ui.ctx(), self.doc.project().hide_masked);
        menus::show_bar(ui, &menu_ctx, actions);
    }

    /// What the status bar says about the selected cells of the active workbook: the average, the
    /// sum and the count of its numbers.
    fn selection_stats(&self) -> Option<grid::Stats> {
        let book = self.doc.project().workbook(self.active?)?;
        let sheet = book.sheets.get(book.active_sheet.min(book.sheets.len().checked_sub(1)?))?;
        let target = self.books.get(&book.id)?.target(sheet)?;
        grid::selection_stats(sheet, &target)
    }

    /// The reference of the active object, as Origin writes it in the status bar.
    fn active_reference(&self) -> String {
        let project = self.doc.project();
        match self.active.and_then(|id| project.window(id)) {
            Some(Window::Workbook(b)) => {
                let sheet = b.sheets.get(b.active_sheet).map(|s| s.name.as_str()).unwrap_or_default();
                let part = b
                    .sheets
                    .get(b.active_sheet)
                    .and_then(|s| Some((s, self.books.get(&b.id)?.target(s)?)))
                    .map(|(s, t)| {
                        let name = |i: usize| s.columns.get(i).map(|c| c.short_name.clone()).unwrap_or_default();
                        let (first, last) =
                            (t.columns.first().copied().unwrap_or(0), t.columns.last().copied().unwrap_or(0));
                        let cols = if first == last { name(first) } else { format!("{}:{}", name(first), name(last)) };
                        if t.rows.is_empty() { cols } else { format!("{cols}[{}:{}]", t.rows.start + 1, t.rows.end) }
                    })
                    .unwrap_or_default();
                format!("1: [{}]{sheet}!{part}", b.name)
            }
            Some(Window::Graph(g)) => format!("1: [{}]1!", g.name),
            Some(Window::Notes(n)) => n.name.clone(),
            Some(Window::Matrix(m)) => format!("[{}]{}!", m.name, m.sheet().name),
            Some(Window::Layout(l)) => l.name.clone(),
            None => String::new(),
        }
    }

    /// The bar at the bottom: the hint of what the mouse is on (or the last message) at the left,
    /// then boxes for the statistics of the selected cells, the auto-update state, the active
    /// object and the angle unit.
    fn status_bar(&self, ui: &mut egui::Ui) {
        let hint = panels::take_hint(ui.ctx());
        let stats = self.selection_stats();
        let reference = self.active_reference();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let boxed = |ui: &mut egui::Ui, text: &str, tip: &str| {
                ui.add_space(4.0);
                ui.label(text).on_hover_text(tip);
                ui.add_space(4.0);
                ui.separator();
            };
            // Origin's trigonometric functions take radians, and so do ours (`sin`, `cos`, ...).
            boxed(ui, "Radian", "Angle unit of the trigonometric functions in formulas");
            if !self.live.clients().is_empty() {
                let text = format!("AI: connected ({})", self.live.clients().len());
                boxed(ui, &text, &live::tooltip(self.live.clients()));
            }
            if !reference.is_empty() {
                boxed(ui, &reference, "The active sheet or graph layer");
            }
            boxed(ui, "AU : ON", "Auto update: plots and automatic columns follow changes of their data");
            if let Some(s) = stats {
                let n = |v: f64| sheaf_core::format::format_general(v, 5);
                let text = format!("Average={} Sum={} Count={}", n(s.average()), n(s.sum), s.count);
                boxed(ui, &text, "Statistics of the selected cells");
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(hint.as_deref().unwrap_or(&self.status)).truncate());
            });
        });
    }

    /// An import window on `files`, started from `<last used>` (or `<default>`) of the saved settings.
    fn import_dialog(&self, files: Vec<PathBuf>) -> ImportDialog {
        let mut d = ImportDialog::new(files);
        d.attach_themes(self.startup.themes_dir.clone());
        d
    }

    fn excel_dialog(&self, files: Vec<PathBuf>) -> ExcelImportDialog {
        let mut d = ExcelImportDialog::new(files);
        d.attach_themes(self.startup.themes_dir.clone());
        d
    }

    fn wizard(&self, files: Vec<PathBuf>) -> ImportWizard {
        let mut w = ImportWizard::new(files);
        w.attach_themes(self.startup.themes_dir.clone());
        w
    }

    /// Opens the import dialog on `files`, or on what the user picks when there are none.
    fn start_import(&mut self, files: Option<Vec<PathBuf>>, pick: ImportPick) {
        let files = files.or_else(|| {
            let mut chooser = rfd::FileDialog::new().set_title("Import data files");
            chooser = if pick == ImportPick::Excel {
                chooser.set_title("Import Excel workbooks").add_filter("Excel workbooks", &sheaf_io::excel::EXTENSIONS)
            } else if pick == ImportPick::Pdf {
                chooser.set_title("Import PDF tables").add_filter("PDF files", &["pdf"])
            } else {
                chooser.add_filter("Data files", &["txt", "csv", "tsv", "dat", "asc", "prn", "bin"])
            };
            chooser = chooser.add_filter("All files", &["*"]);
            if let Some(dir) = self.folder() {
                chooser = chooser.set_directory(dir);
            }
            match pick {
                ImportPick::Single => chooser.pick_file().map(|f| vec![f]),
                ImportPick::Multiple | ImportPick::Csv | ImportPick::Excel | ImportPick::Pdf => chooser.pick_files(),
            }
        });
        if let Some(files) = files {
            // PDFs (T463) are read at once.
            let (pdfs, files): (Vec<PathBuf>, Vec<PathBuf>) = files.into_iter().partition(|p| is_pdf(p));
            if !pdfs.is_empty() {
                self.import_now(pdfs, false);
            }
            // An Excel workbook chosen with an ASCII item still goes to the Excel window.
            let (workbooks, files) = split_excel(files);
            if !workbooks.is_empty() {
                self.import_excel = Some(self.excel_dialog(workbooks));
            }
            if !files.is_empty() {
                let mut dialog = self.import_dialog(files);
                if pick == ImportPick::Csv {
                    dialog.preset_comma();
                }
                self.import = Some(dialog);
            }
        }
    }

    fn welcome(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.28);
            ui.label(RichText::new(APP_NAME).size(30.0).strong());
            ui.label("Import instrument data, make figures, analyse.");
            ui.add_space(14.0);
            if ui.button("Import data files...").clicked() {
                actions.push(Action::Import(None));
            }
            if ui.button("Open a project...").clicked() {
                actions.push(Action::OpenProject);
            }
            if ui.button("New workbook").clicked() {
                actions.push(Action::NewWorkbook);
            }
            ui.add_space(10.0);
            ui.weak("You can also drop data files or a project onto this window.");
        });
    }

    /// The sheet that data is imported into when the import window is not set to "New book": the
    /// active sheet of the active workbook, with a label for the window.
    fn import_target(&self) -> Option<(Target, String)> {
        let id = self.active?;
        let book = self.doc.project().workbook(id)?;
        let sheet = book.active_sheet.min(book.sheets.len().checked_sub(1)?);
        Some((Target { book: id, sheet }, format!("{} / {}", book.name, book.sheets[sheet].name)))
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let import_target = self.import_target();
        if let Some(d) = &mut self.import {
            match d.show(ctx, import_target.as_ref().map(|t| t.1.as_str())) {
                ImportOutcome::Open => {}
                ImportOutcome::Cancelled => self.import = None,
                ImportOutcome::Done(request) => {
                    if let Some(warning) = d.take_theme_warning() {
                        self.logs.warn(warning);
                    }
                    self.import = None;
                    self.finish_import(request);
                }
            }
        }
        if let Some(d) = &mut self.import_excel {
            match d.show(ctx, import_target.as_ref().map(|t| t.1.as_str())) {
                ImportOutcome::Open => {}
                ImportOutcome::Cancelled => self.import_excel = None,
                ImportOutcome::Done(request) => {
                    if let Some(warning) = d.take_theme_warning() {
                        self.logs.warn(warning);
                    }
                    self.import_excel = None;
                    self.finish_import(request);
                }
            }
        }
        if let Some(d) = &mut self.import_wizard {
            match d.show(ctx, import_target.as_ref().map(|t| t.1.as_str())) {
                ImportOutcome::Open => {}
                ImportOutcome::Cancelled => self.import_wizard = None,
                ImportOutcome::Done(request) => {
                    if let Some(warning) = d.take_theme_warning() {
                        self.logs.warn(warning);
                    }
                    self.import_wizard = None;
                    self.finish_import(request);
                }
            }
        }
        if let Some(d) = &mut self.graph_props
            && !d.show(ctx, &mut self.doc)
        {
            self.graph_props = None;
        }
        if let Some(d) = &mut self.layer_contents {
            use crate::dialogs::layer_contents::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed => self.layer_contents = None,
                Outcome::LayerProperties(graph, layer) => {
                    let token = self.token();
                    self.graph_props = Some(GraphPropsDialog::new(graph, layer, PropsTab::Plots, token));
                }
                Outcome::PlotSetup(graph, layer) => {
                    // Origin replaces Layer Contents with Plot Setup.
                    self.layer_contents = None;
                    let token = self.token();
                    self.plot_setup =
                        Some(crate::dialogs::plot_setup::PlotSetupDialog::new(graph, layer, token, self.doc.project()));
                }
            }
        }
        if let Some(d) = &mut self.plot_setup
            && matches!(d.show(ctx, &mut self.doc), crate::dialogs::plot_setup::Outcome::Closed)
        {
            self.plot_setup = None;
        }
        if let Some(d) = &mut self.error_bars {
            match d.show(ctx, &mut self.doc) {
                ErrorBarsOutcome::Open => {}
                ErrorBarsOutcome::Closed => self.error_bars = None,
                ErrorBarsOutcome::Added => {
                    self.error_bars = None;
                    self.say("Updated Y error bars for the selected plot");
                }
            }
        }
        if let Some(d) = &mut self.trendline {
            match d.show(ctx, &mut self.doc) {
                TrendlineOutcome::Open => {}
                TrendlineOutcome::Closed => self.trendline = None,
                TrendlineOutcome::Done => self.trendline = None,
            }
        }
        if let Some(d) = &mut self.stack_panels {
            match d.show(ctx, &mut self.doc) {
                StackPanelsOutcome::Open => {}
                StackPanelsOutcome::Closed => self.stack_panels = None,
                StackPanelsOutcome::Created(graph) => {
                    self.stack_panels = None;
                    self.focus(graph);
                    self.say("Created stacked panels");
                }
            }
        }
        if let Some(d) = &mut self.axis3d {
            use crate::dialogs::axis3d::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed => self.axis3d = None,
                Outcome::Done => {
                    self.axis3d = None;
                    self.say("Set the 3D axis scales");
                }
            }
        }
        if let Some(d) = &mut self.import_bin2d {
            use crate::dialogs::import_bin2d::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed => self.import_bin2d = None,
                Outcome::Done(partial) => {
                    self.import_bin2d = None;
                    if partial {
                        // Origin fills the row the same way and reports the import as failed.
                        self.warn("The file ends inside the last row of the array; that row is left at 0".to_string());
                    } else {
                        self.say("Imported the binary 2D array".to_string());
                    }
                }
            }
        }
        if let Some(d) = &mut self.layout_shape {
            use crate::dialogs::layout_shape::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed | Outcome::Done => self.layout_shape = None,
            }
        }
        if let Some(d) = &mut self.layout_text {
            use crate::dialogs::layout_text::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed | Outcome::Done => self.layout_text = None,
            }
        }
        if let Some(d) = &mut self.layout_page_setup {
            use crate::dialogs::layout_page_setup::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed | Outcome::Done => self.layout_page_setup = None,
            }
        }
        if let Some(d) = &mut self.layout_add_graph {
            use crate::dialogs::layout_add_graph::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed | Outcome::Done => self.layout_add_graph = None,
            }
        }
        if let Some(d) = &mut self.matrix_dialog {
            use crate::dialogs::matrix_ops::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed | Outcome::Done => self.matrix_dialog = None,
            }
        }
        if let Some(d) = &mut self.fit_page {
            use crate::dialogs::fit_page::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed => self.fit_page = None,
                Outcome::Done => {
                    self.fit_page = None;
                    self.say("Fitted the page to the layers");
                }
            }
        }
        if let Some(d) = &mut self.add_layer_dialog {
            use crate::dialogs::add_layer::Outcome;
            match d.show(ctx, &self.doc) {
                Outcome::Open => {}
                Outcome::Closed => self.add_layer_dialog = None,
                Outcome::Stale => {
                    self.add_layer_dialog = None;
                    self.warn("The project changed. Open New Layer (Axes) again.");
                }
                Outcome::Add { graph, layer, choice, activate, auto_offset } => {
                    self.add_layer_dialog = None;
                    self.add_layer(graph, layer, choice, activate, auto_offset);
                    self.focus(graph);
                }
            }
        }
        if let Some(d) = &mut self.layer_management {
            use crate::dialogs::layer_management::Outcome;
            match d.show(ctx, &mut self.doc) {
                Outcome::Open => {}
                Outcome::Closed => self.layer_management = None,
                Outcome::Applied => {
                    self.layer_management = None;
                    self.say("Updated the layers");
                }
            }
        }
        if let Some(d) = &mut self.merge_graphs {
            match d.show(ctx, &mut self.doc, &self.workspace) {
                MergeGraphsOutcome::Open => {}
                MergeGraphsOutcome::Closed => self.merge_graphs = None,
                MergeGraphsOutcome::Created(graph) => {
                    self.merge_graphs = None;
                    self.focus(graph);
                    self.say("Merged the graph windows");
                }
            }
        }
        if let Some(d) = &mut self.label_panels {
            match d.show(ctx, &mut self.doc) {
                LabelPanelsOutcome::Open => {}
                LabelPanelsOutcome::Closed => self.label_panels = None,
                LabelPanelsOutcome::Created(graph) => {
                    self.label_panels = None;
                    self.focus(graph);
                    self.say("Created panels by label");
                }
            }
        }
        if let Some(d) = &mut self.multi_y {
            match d.show(ctx, &mut self.doc) {
                MultiYOutcome::Open => {}
                MultiYOutcome::Closed => self.multi_y = None,
                MultiYOutcome::Created(graph) => {
                    self.multi_y = None;
                    self.focus(graph);
                    self.say("Created multiple Y axes graph");
                }
            }
        }
        if let Some(d) = &mut self.comparison_plot {
            match d.show(ctx, &mut self.doc) {
                ComparisonPlotOutcome::Open => {}
                ComparisonPlotOutcome::Closed => self.comparison_plot = None,
                ComparisonPlotOutcome::Created(graph) => {
                    self.comparison_plot = None;
                    self.focus(graph);
                    self.say("Created multiple comparisons plot");
                }
            }
        }
        if let Some(d) = &mut self.simulate {
            match d.show(ctx, &mut self.doc) {
                SimulateOutcome::Open => {}
                SimulateOutcome::Closed => self.simulate = None,
                SimulateOutcome::Done { book, graph, message } => {
                    self.simulate = None;
                    self.focus(book);
                    if let Some(graph) = graph {
                        self.focus(graph);
                    }
                    self.say(message);
                }
            }
        }
        if let Some(d) = &mut self.simulate_surface {
            match d.show(ctx, &mut self.doc) {
                SimulateSurfaceOutcome::Open => {}
                SimulateSurfaceOutcome::Closed => self.simulate_surface = None,
                SimulateSurfaceOutcome::Done { book, message } => {
                    self.simulate_surface = None;
                    self.focus(book);
                    self.say(message);
                }
            }
        }
        if let Some(column) = self.graph_props.as_mut().and_then(|d| d.take_show_workbook())
            && let Some((book, _, _)) = self.doc.project().locate(column)
        {
            let id = book.id;
            self.focus(id);
        }
        if let Some(d) = &mut self.column_props
            && !d.show(ctx, &mut self.doc)
        {
            self.column_props = None;
        }
        if let Some(d) = &mut self.text_dialog
            && !d.show(ctx, &mut self.doc)
        {
            self.text_dialog = None;
        }
        if let Some(d) = &mut self.shape_dialog
            && !d.show(ctx, &mut self.doc)
        {
            self.shape_dialog = None;
        }
        if let Some(d) = &mut self.slope_dialog
            && !d.show(ctx, &mut self.doc)
        {
            self.slope_dialog = None;
        }
        if let Some(d) = &mut self.apply_formats
            && !d.show(ctx, &mut self.doc)
        {
            self.apply_formats = None;
        }
        if let Some(d) = &mut self.straight_line_dialog
            && !d.show(ctx, &mut self.doc)
        {
            self.straight_line_dialog = None;
        }
        self.pick_formula_reference(ctx);
        if let Some(d) = &mut self.set_values
            && !d.show(ctx, &mut self.doc)
        {
            self.set_values = None;
        }
        if let Some(d) = &mut self.analysis {
            // OK closes the window; Apply leaves it open for the next run.
            let outcome = d.show(ctx, &mut self.doc);
            if matches!(outcome, AnalysisOutcome::Done(_) | AnalysisOutcome::Applied(_)) {
                self.last_analysis = d.repeat_tool();
            }
            let (report, close) = match outcome {
                AnalysisOutcome::Open => (None, false),
                AnalysisOutcome::Closed => (None, true),
                AnalysisOutcome::Done(report) => (Some(report), true),
                AnalysisOutcome::Applied(report) => (Some(report), false),
                AnalysisOutcome::AutoApplied(report) => {
                    // The tool ran again after an option changed: the status line says so, the
                    // sheet and the results panel stay as they are.
                    let message = report.message.clone();
                    self.say(message);
                    (None, false)
                }
            };
            if close {
                self.analysis = None;
            }
            if let Some(report) = report {
                if let Some(sheet) = report.new_sheet {
                    self.show_sheet(report.book, sheet);
                }
                if let Some(graph) = report.new_graph {
                    self.focus(graph);
                }
                self.logs.result(report.message.clone(), report.notes.clone());
                self.panels.reveal(PanelId::Results);
                self.say(report.message);
            }
        }
        if let Some(d) = &mut self.fit {
            match d.show(ctx, &mut self.doc) {
                FitDialogOutcome::Open => {}
                FitDialogOutcome::Closed => self.fit = None,
                FitDialogOutcome::Done(outcome, new_graph) => {
                    self.fit = None;
                    // Show the parameters; a graph made for the fit opens as its own tab.
                    self.show_sheet(outcome.book, outcome.param_sheet);
                    if let Some(g) = new_graph.or(outcome.graphs.first().copied()) {
                        self.focus(g);
                    }
                    self.logs.result(outcome.message.clone(), fit_lines(&outcome));
                    self.panels.reveal(PanelId::Results);
                    self.say(outcome.message);
                }
                FitDialogOutcome::DoneMulti(multi, new_graph) => {
                    self.fit = None;
                    let first = &multi.outcomes[0];
                    self.show_sheet(first.book, multi.summary_sheet.unwrap_or(first.param_sheet));
                    if let Some(g) = new_graph.or(first.graphs.first().copied()) {
                        self.focus(g);
                    }
                    let mut lines = Vec::new();
                    for o in &multi.outcomes {
                        lines.push(o.message.clone());
                        lines.extend(fit_lines(o));
                    }
                    self.logs.result(multi.message.clone(), lines);
                    self.panels.reveal(PanelId::Results);
                    self.say(multi.message.clone());
                }
            }
        }
        if let Some(d) = &mut self.fit_more {
            match d.show(ctx, &mut self.doc) {
                FitMoreOutcome::Open => {}
                FitMoreOutcome::Closed => self.fit_more = None,
                FitMoreOutcome::Multiple(o) => {
                    self.fit_more = None;
                    self.show_sheet(o.book, o.param_sheet);
                    self.logs.result(o.message.clone(), fit_report_lines(&o.report));
                    self.panels.reveal(PanelId::Results);
                    self.say(o.message);
                }
                FitMoreOutcome::Compare(o) => {
                    self.fit_more = None;
                    self.show_sheet(o.book, o.sheet);
                    let mut lines = o.comparison.warnings.clone();
                    lines.splice(0..0, o.reports.iter().flat_map(fit_report_lines));
                    self.logs.result(o.message.clone(), lines);
                    self.panels.reveal(PanelId::Results);
                    self.say(o.message);
                }
                FitMoreOutcome::CompareDatasets(o) => {
                    self.fit_more = None;
                    self.show_sheet(o.book, o.sheet);
                    let mut lines = o.comparison.warnings.clone();
                    lines.splice(0..0, o.reports.iter().flat_map(fit_report_lines));
                    self.logs.result(o.message.clone(), lines);
                    self.panels.reveal(PanelId::Results);
                    self.say(o.message);
                }
                FitMoreOutcome::Implicit(o) => {
                    self.fit_more = None;
                    self.show_sheet(o.book, o.param_sheet);
                    self.logs.result(o.message.clone(), fit_report_lines(&o.report));
                    self.panels.reveal(PanelId::Results);
                    self.say(o.message);
                }
                FitMoreOutcome::BestSubset(o) => {
                    self.fit_more = None;
                    self.show_sheet(o.book, o.sheet);
                    self.logs.result(o.message.clone(), Vec::new());
                    self.panels.reveal(PanelId::Results);
                    self.say(o.message);
                }
                FitMoreOutcome::Rank(o) => {
                    self.fit_more = None;
                    self.show_sheet(o.book, o.sheet);
                    let lines = o.skipped.iter().map(|(f, e)| format!("Skipped {f}: {e}")).collect();
                    self.logs.result(o.message.clone(), lines);
                    self.panels.reveal(PanelId::Results);
                    self.say(o.message);
                }
            }
        }
        self.multi_peak_frame(ctx);
        self.subtract_pick_frame(ctx);
        self.cv_pick_frame(ctx);
        self.find_files_frame(ctx);
        self.column_across_frame(ctx);
        self.gadget_prefs_frame(ctx);
        self.range_pick_frame(ctx);
        self.anchor_pick_frame(ctx);
        if let Some(d) = &mut self.pivot {
            let book = d.book;
            match d.show(ctx, &mut self.doc) {
                ReshapeOutcome::Open => {}
                ReshapeOutcome::Closed => self.pivot = None,
                ReshapeOutcome::Done(sheet) => {
                    self.pivot = None;
                    self.show_sheet(book, sheet);
                }
            }
        }
        if let Some(d) = &mut self.reshape {
            let book = d.book;
            match d.show(ctx, &mut self.doc) {
                ReshapeOutcome::Open => {}
                ReshapeOutcome::Closed => self.reshape = None,
                ReshapeOutcome::Done(sheet) => {
                    self.reshape = None;
                    self.show_sheet(book, sheet);
                }
            }
        }
        if let Some(d) = &mut self.cond_manager {
            match d.show(ctx, &mut self.doc) {
                ManagerOutcome::Open => {}
                ManagerOutcome::Closed => self.cond_manager = None,
                ManagerOutcome::EditRule { sheet, index } => {
                    let book = d.book;
                    self.cond_format = CondFormatDialog::edit(&self.doc, book, sheet, index);
                }
            }
        }
        if let Some(d) = &mut self.cond_format
            && !d.show(ctx, &mut self.doc)
        {
            self.cond_format = None;
        }
        if let Some(d) = &mut self.cell_note
            && !d.show(ctx, &mut self.doc)
        {
            self.cell_note = None;
        }
        if let Some(d) = &mut self.cell_format
            && !d.show(ctx, &mut self.doc)
        {
            self.cell_format = None;
        }
        if let Some(d) = &mut self.worksheet_fonts
            && !d.show(ctx, &mut self.doc)
        {
            self.worksheet_fonts = None;
        }
        if let Some(d) = &mut self.worksheet_color {
            let (open, request) = d.show(ctx);
            if !open {
                self.worksheet_color = None;
            }
            if let Some(request) = request {
                self.apply(Action::FormatText(request), ctx);
            }
        }
        if let Some(d) = &mut self.row_filter
            && !d.show(ctx, &mut self.doc)
        {
            self.row_filter = None;
        }
        if let Some(d) = &mut self.mask_condition
            && !d.show(ctx, &mut self.doc)
        {
            self.mask_condition = None;
        }
        if let Some(d) = &mut self.filter {
            match d.show(ctx, &mut self.doc) {
                FilterOutcome::Open => {}
                FilterOutcome::Closed => self.filter = None,
                FilterOutcome::Done(kept, sheet) => {
                    let book = d.book;
                    self.filter = None;
                    self.show_sheet(book, sheet);
                    self.say(format!("Filter rows: {kept} row(s) kept"));
                }
            }
        }
        if let Some(d) = &mut self.export {
            let outcome = d.show(ctx, &self.doc);
            let written = d.take_written();
            self.record_exports(ExportKind::Graph, written);
            match outcome {
                ExportOutcome::Open => {}
                ExportOutcome::Closed => self.export = None,
                ExportOutcome::Saved(path) => {
                    self.say(format!("Wrote {}", path.display()));
                    self.export = None;
                }
                ExportOutcome::SavedWithNotes(path, _) => {
                    self.say(format!("Wrote {}", path.display()));
                    self.export = None;
                }
                ExportOutcome::Applied(text) => self.say(text),
                ExportOutcome::Failed(e) => self.message = Some(("Could not export the graph".to_string(), e)),
            }
        }
        if let Some(d) = &mut self.page_setup {
            match d.show(ctx) {
                PageSetupOutcome::Open => {}
                PageSetupOutcome::Closed => self.page_setup = None,
                PageSetupOutcome::Apply(setup) => {
                    self.page_setup = None;
                    if setup != self.doc.project().page_setup {
                        self.doc.apply("Page Setup", |p| p.page_setup = setup);
                    }
                }
            }
        }
        if let Some(d) = &mut self.print_preview {
            let graph = d.graph;
            match d.show(ctx, &self.doc) {
                PreviewOutcome::Open => {}
                PreviewOutcome::Closed => self.print_preview = None,
                PreviewOutcome::Print => {
                    self.print_preview = None;
                    self.print_graph(graph);
                }
                PreviewOutcome::Setup => self.page_setup = Some(PageSetupDialog::new(self.doc.project().page_setup)),
            }
        }
        if let Some(d) = &mut self.export_sheet {
            let book = d.book;
            match d.show(ctx, &self.doc) {
                ExportOutcome::Open => {}
                ExportOutcome::Closed => self.export_sheet = None,
                ExportOutcome::Saved(path) => {
                    self.record_exports(ExportKind::Sheet, vec![(book, path.clone())]);
                    self.say(format!("Wrote {}", path.display()));
                    self.export_sheet = None;
                }
                ExportOutcome::SavedWithNotes(path, _) => {
                    self.record_exports(ExportKind::Sheet, vec![(book, path.clone())]);
                    self.say(format!("Wrote {}", path.display()));
                    self.export_sheet = None;
                }
                ExportOutcome::Applied(_) => {}
                ExportOutcome::Failed(e) => self.message = Some(("Could not export the worksheet".to_string(), e)),
            }
        }
        if let Some(d) = &mut self.export_excel {
            let book = d.book;
            match d.show(ctx, &self.doc) {
                ExportOutcome::Open | ExportOutcome::Applied(_) => {}
                ExportOutcome::Closed => self.export_excel = None,
                ExportOutcome::Saved(path) => {
                    self.record_exports(ExportKind::Excel, vec![(book, path.clone())]);
                    self.say(format!("Wrote {}", path.display()));
                    self.export_excel = None;
                }
                ExportOutcome::SavedWithNotes(path, notes) => {
                    self.record_exports(ExportKind::Excel, vec![(book, path.clone())]);
                    self.say(format!("Wrote {}", path.display()));
                    self.export_excel = None;
                    for note in &notes {
                        self.logs.warn(format!("Excel export: {note}"));
                    }
                    if !notes.is_empty() {
                        self.message = Some(("Export notes".to_string(), notes.join("\n\n")));
                    }
                }
                ExportOutcome::Failed(e) => self.message = Some(("Could not export to Excel".to_string(), e)),
            }
        }
        if let Some(d) = &mut self.acceptance {
            use crate::dialogs::acceptance::AcceptanceOutcome;
            match d.show(ctx, &mut self.doc) {
                AcceptanceOutcome::Open => {}
                AcceptanceOutcome::Closed => self.acceptance = None,
                AcceptanceOutcome::Done(report) => {
                    self.acceptance = None;
                    if let Some(sheet) = report.new_sheet {
                        self.show_sheet(report.book, sheet);
                    }
                    self.logs.result(report.message.clone(), report.notes.clone());
                    self.panels.reveal(PanelId::Results);
                    self.say(report.message);
                }
            }
        }
        if let Some(d) = &mut self.power {
            match d.show(ctx, &mut self.doc) {
                PowerOutcome::Open => {}
                PowerOutcome::Closed => self.power = None,
                PowerOutcome::Done(report) => {
                    self.power = None;
                    if let Some(sheet) = report.new_sheet {
                        self.show_sheet(report.book, sheet);
                    }
                    self.logs.result(report.message.clone(), report.notes.clone());
                    self.panels.reveal(PanelId::Results);
                    self.say(report.message);
                }
            }
        }
        if let Some(d) = &mut self.go_to {
            let book = d.book;
            let outcome = d.show(ctx, self.doc.project());
            match outcome {
                GoToOutcome::Open => {}
                GoToOutcome::Closed => self.go_to = None,
                GoToOutcome::Go { column, row, extend } => {
                    if let (Some(sheet), Some(state)) = (
                        self.doc.project().workbook(book).and_then(|b| b.sheets.get(b.active_sheet)),
                        self.books.get_mut(&book),
                    ) {
                        state.go_to_selection(sheet, column, row, extend);
                    }
                    self.go_to = None;
                }
            }
        }
        self.show_template_windows(ctx);
        self.show_batch(ctx);
        self.show_graph_sheet_picker(ctx);
        self.show_send_ppt(ctx);
        self.show_name_cell(ctx);
        self.show_name_manager(ctx);
        self.show_cell_sparkline_picker(ctx);
        self.show_batch_peaks(ctx);
        self.show_fill_pattern(ctx);
        self.show_sampling(ctx);
        self.show_copy_columns(ctx);
        self.show_sheet_navigate(ctx);
        self.show_find_project(ctx);
        self.show_batch_plot(ctx);
        self.show_qc_chart(ctx);
        self.show_grouped_hist(ctx);
        self.show_run_chart(ctx);
        self.show_function_graph(ctx);
        self.show_graph_slicer(ctx);
        self.show_browser_graph(ctx);
        self.show_flash_fill(ctx);
        self.show_project_browser(ctx);
        self.show_op_params(ctx);
        self.show_auto_refresh(ctx);
        let now = ctx.input(|i| i.time);
        if let Some(wait) = self.run_auto_refresh(now) {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait.max(0.5)));
        }
        self.show_audit_log(ctx);
        self.show_fit_builder(ctx);
        self.show_sample_projects(ctx);
        self.show_scatter_matrix(ctx);
        self.show_folder_props(ctx);
        self.show_graph_themes(ctx);
        if let Some(o) = &mut self.fit_organizer
            && !o.show(ctx)
        {
            self.fit_organizer = None;
        }
        if let Some(m) = &mut self.filter_manager
            && !m.show(ctx)
        {
            self.filter_manager = None;
        }
        if let Some(d) = &mut self.system_templates {
            use crate::dialogs::system_templates::SystemTemplatesOutcome;
            match d.show(ctx) {
                SystemTemplatesOutcome::Open => {}
                SystemTemplatesOutcome::Closed => self.system_templates = None,
                SystemTemplatesOutcome::Plot(template) => {
                    let (book, columns) = (d.book, d.columns.clone());
                    self.system_templates = None;
                    self.apply(Action::PlotColumns { book, columns, template }, ctx);
                }
            }
        }
        if self.panels.explorer.folder != self.last_folder {
            self.last_folder = self.panels.explorer.folder.clone();
            self.follow_folder();
        }
        if let Some(d) = &mut self.find {
            let book = d.book;
            match d.show(ctx) {
                FindOutcome::Open => {}
                FindOutcome::Closed => self.find = None,
                FindOutcome::Next => {
                    if let (Some(sheet), Some(state)) = (
                        self.doc.project().workbook(book).and_then(|b| b.sheets.get(b.active_sheet)),
                        self.books.get_mut(&book),
                    ) && let Some((column, row)) = d.search(sheet, state.active_cell())
                    {
                        state.go_to(sheet, column, row);
                    }
                }
                outcome @ (FindOutcome::Replace | FindOutcome::ReplaceAll) => {
                    let project = self.doc.project();
                    if let Some(sheet) = project.workbook(book).and_then(|b| b.sheets.get(b.active_sheet)) {
                        let cursor = self.books.get(&book).and_then(|s| s.active_cell());
                        let edits: Vec<(usize, usize, String)> = match outcome {
                            FindOutcome::ReplaceAll => d.replace_all(sheet),
                            _ => cursor
                                .and_then(|(c, r)| d.replaced(sheet, (c, r)).map(|t| (c, r, t)))
                                .into_iter()
                                .collect(),
                        };
                        // Columns kept up to date by a formula are left alone, as when typing.
                        let (edits, locked): (Vec<_>, Vec<_>) =
                            edits.into_iter().partition(|(c, _, _)| !sheet.columns[*c].is_locked());
                        let edits: Vec<(sheaf_core::ColumnId, usize, String)> =
                            edits.into_iter().map(|(c, r, t)| (sheet.columns[c].id, r, t)).collect();
                        if !edits.is_empty() {
                            let label = if edits.len() == 1 { "Replace" } else { "Replace all" };
                            self.doc.apply(label, |p| {
                                for (id, row, text) in &edits {
                                    if let Some(c) = p.column_mut(*id) {
                                        c.set_cell_text(*row, text);
                                    }
                                }
                            });
                        }
                        if !locked.is_empty() {
                            self.warn(format!(
                                "{} cell(s) in columns calculated from a formula were left alone",
                                locked.len()
                            ));
                        }
                    }
                    if matches!(outcome, FindOutcome::Replace)
                        && let Some(d) = &mut self.find
                        && let (Some(sheet), Some(state)) = (
                            self.doc.project().workbook(book).and_then(|b| b.sheets.get(b.active_sheet)),
                            self.books.get_mut(&book),
                        )
                        && let Some((column, row)) = d.search(sheet, state.active_cell())
                    {
                        state.go_to(sheet, column, row);
                    }
                }
            }
        }
        if let Some(d) = &mut self.sort {
            match d.show(ctx, self.doc.project()) {
                SortOutcome::Open => {}
                SortOutcome::Closed => self.sort = None,
                SortOutcome::Nested { book, columns, keys, rows } => {
                    let sheet =
                        self.doc.project().workbook(book).map(|b| b.active_sheet.min(b.sheets.len().saturating_sub(1)));
                    if let Some(sheet) = sheet {
                        match self.doc.edit("Sort (nested)", |p| {
                            p.sort_nested_in_rows(book, sheet, &keys, &columns, rows.unwrap_or(0..usize::MAX))
                        }) {
                            Ok(()) => self.say(format!("Sorted by {} column(s)", keys.len())),
                            Err(e) => self.message = Some(("Could not sort".to_string(), e.to_string())),
                        }
                    }
                    self.sort = None;
                }
                SortOutcome::ByLabel { book, label, descending } => {
                    let mut moved = false;
                    self.doc.apply("Sort columns by label", |p| {
                        if let Some(b) = p.workbook_mut(book) {
                            moved = b.sheet_mut().sort_columns_by_label(label, descending);
                        }
                    });
                    if !moved {
                        self.say("The columns are already in that order");
                    }
                    self.sort = None;
                }
            }
        }
        if let Some(d) = &mut self.rename {
            let mut open = true;
            let mut done = false;
            let (title, label) =
                if d.folder.is_some() { ("Rename folder", "Name") } else { ("Rename window", "Title") };
            egui::Window::new(title).open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.add(egui::TextEdit::singleline(&mut d.long_name).desired_width(260.0));
                });
                if d.folder.is_none() {
                    ui.horizontal(|ui| {
                        ui.label("Short Name");
                        ui.add(egui::TextEdit::singleline(&mut d.short_name).desired_width(200.0));
                    });
                    ui.weak("Letters and digits, at most 13 characters (as in Origin).");
                    // T249: Origin's Window Properties have Comments.
                    ui.label("Comments");
                    ui.add(egui::TextEdit::multiline(&mut d.comments).desired_width(320.0).desired_rows(3));
                    // Dependents (T220): what uses this window, and the books a graph plots.
                    let project = self.doc.project();
                    let titles = |ids: Vec<WindowId>| -> String {
                        let names: Vec<String> =
                            ids.iter().filter_map(|w| project.window(*w).map(|w| w.name().to_string())).collect();
                        if names.is_empty() { "none".to_string() } else { names.join(", ") }
                    };
                    if project.workbook(d.window).is_some() {
                        ui.label(format!("Dependents: {}", titles(project.dependents(d.window))));
                    } else if project.graph(d.window).is_some() {
                        ui.label(format!("Plots data of: {}", titles(project.plotted_books(d.window))));
                    }
                    if let Some(e) = &d.error {
                        ui.colored_label(ui.visuals().error_fg_color, e);
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        let (id, name) = (d.window, d.long_name.clone());
                        let short = d.short_name.clone();
                        let comments = d.comments.clone();
                        if let Some(path) = d.folder.clone() {
                            let name = name.trim().to_string();
                            if !name.is_empty() {
                                self.doc.apply("Rename folder", move |p| {
                                    if let Some(f) = explorer::folder_at_mut(&mut p.root, &path) {
                                        f.name = name;
                                    }
                                });
                            }
                            done = true;
                        } else {
                            // The short name first: a name another window has keeps the window open.
                            let result = self.doc.edit("Rename window", move |p| {
                                let current = p.window(id).map(|w| w.name().to_string()).unwrap_or_default();
                                if short.trim() != current {
                                    p.rename_window(id, &short)?;
                                }
                                if let Some(b) = p.workbook_mut(id) {
                                    b.long_name = name;
                                } else if let Some(g) = p.graph_mut(id) {
                                    g.long_name = name;
                                } else if let Some(n) = p.notes_mut(id) {
                                    n.long_name = name;
                                }
                                p.set_window_comments(id, &comments);
                                Ok::<(), String>(())
                            });
                            match result {
                                Ok(()) => done = true,
                                Err(e) => d.error = Some(e),
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        done = true;
                    }
                });
            });
            if done || !open {
                self.rename = None;
            }
        }
        // T309: Add User Parameter Row asks the name of the new row.
        if let Some((book, sheet, name, renaming)) = &mut self.param_row {
            let (book, sheet, renaming) = (*book, *sheet, renaming.clone());
            let (mut open, mut close, mut add) = (true, false, None);
            let title = if renaming.is_some() { "Rename Parameter Row" } else { "Add User Parameter Row" };
            egui::Window::new(title).open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let label = ui.label("Name of the row");
                    ui.add(egui::TextEdit::singleline(name).desired_width(160.0)).labelled_by(label.id);
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        add = Some(name.clone());
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if let Some(name) = add {
                let mut problem = None;
                let label = if renaming.is_some() { "Rename parameter row" } else { "Add parameter row" };
                self.doc.apply(label, |p| {
                    if let Some(ws) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
                        let done = match &renaming {
                            Some(old) => ws.rename_parameter_row(old, &name),
                            None => ws.add_parameter_row(&name),
                        };
                        if let Err(e) = done {
                            problem = Some(e);
                        }
                    }
                });
                match problem {
                    // The window stays open so that the name can be corrected.
                    Some(e) => {
                        self.logs.warn(&e);
                        self.status = e;
                    }
                    None => close = true,
                }
            }
            if close || !open {
                self.param_row = None;
            }
        }
        // T305: Origin's Add New Columns window, how many columns to add.
        if let Some((book, count)) = &mut self.add_columns {
            let (book, mut open, mut add) = (*book, true, None);
            let mut close = false;
            egui::Window::new("Add New Columns").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let label = ui.label("Number of columns to add");
                    ui.add(egui::DragValue::new(count).range(1..=MAX_ADDED_COLUMNS)).labelled_by(label.id);
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        add = Some(*count);
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if close || !open {
                self.add_columns = None;
            }
            if let Some(n) = add {
                self.add_columns(book, n);
            }
        }
        if let Some(d) = &mut self.sheet_name {
            let mut open = true;
            let mut done = false;
            egui::Window::new("Name and Comments").open(&mut open).collapsible(false).resizable(false).show(
                ctx,
                |ui| {
                    egui::Grid::new("sheet_name_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(200.0));
                        ui.end_row();
                        ui.label("Long Name");
                        ui.add(egui::TextEdit::singleline(&mut d.long_name).desired_width(200.0));
                        ui.end_row();
                        let label = ui.label("Comments");
                        ui.add(egui::TextEdit::multiline(&mut d.comments).desired_width(200.0).desired_rows(4))
                            .labelled_by(label.id);
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            let (book, sheet, name, long) =
                                (d.book, d.sheet, d.name.clone(), d.long_name.trim().to_string());
                            let name = name.trim().to_string();
                            let taken = self.doc.project().workbook(book).is_none_or(|b| {
                                b.sheets
                                    .iter()
                                    .enumerate()
                                    .any(|(i, s)| i != sheet && s.name.eq_ignore_ascii_case(&name))
                            });
                            if name.is_empty() || taken {
                                // Not `self.warn`: the window's own state is borrowed here.
                                let text = "A sheet needs a name that no other sheet of the workbook has";
                                self.logs.warn(text);
                                self.status = text.to_string();
                            } else {
                                let label = self
                                    .doc
                                    .project()
                                    .workbook(book)
                                    .and_then(|b| b.sheets.get(sheet))
                                    .filter(|s| s.name == name && s.long_name == long)
                                    .map_or("Rename sheet", |_| "Edit sheet comments");
                                let comments = d.comments.clone();
                                self.doc.apply(label, |p| {
                                    if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
                                        s.name = name;
                                        s.long_name = long;
                                        s.comments = comments;
                                    }
                                });
                            }
                            done = true;
                        }
                        if ui.button("Cancel").clicked() {
                            done = true;
                        }
                    });
                },
            );
            if done || !open {
                self.sheet_name = None;
            }
        }
        self.toolbars.show_dialog(ctx);
        if self.about {
            egui::Window::new(format!("About {APP_NAME}"))
                .open(&mut self.about)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(RichText::new(format!("{APP_NAME} {}", env!("CARGO_PKG_VERSION"))).strong());
                    ui.label("Data analysis and graphing for lab data.");
                    ui.label("Reads CH Instruments text files and delimited text; exports PNG, SVG and PDF.");
                });
        }
        if let Some((title, text)) = &self.message {
            let mut close = false;
            egui::Modal::new(egui::Id::new("message")).show(ctx, |ui| {
                ui.set_max_width(520.0);
                ui.label(RichText::new(title).strong());
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    ui.label(text);
                });
                ui.add_space(8.0);
                if ui.button("OK").clicked() {
                    close = true;
                }
            });
            if close {
                self.message = None;
            }
        }
        if let Some(d) = &mut self.table_editor {
            use crate::dialogs::table_editor::TableOutcome;
            match d.show(ctx) {
                TableOutcome::Open => {}
                TableOutcome::Closed => self.table_editor = None,
                TableOutcome::Apply { graph, layer, label, cells } => {
                    self.table_editor = None;
                    let what = if label.is_some() { "Edit table" } else { "New table" };
                    let done = self.doc.edit(what, |p| {
                        let l = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).ok_or("the layer is gone")?;
                        match label.and_then(|k| l.labels.get_mut(k)) {
                            Some(t) => t.text = sheaf_core::graph::table_text(&cells),
                            None => {
                                let font = l.new_label_font();
                                l.labels.push(sheaf_core::graph::TextLabel::table(&cells, 0.1, 0.1, font));
                            }
                        }
                        Ok::<_, &str>(())
                    });
                    if let Err(e) = done {
                        self.warn(e.to_string());
                    }
                }
            }
        }
        if let Some(d) = &mut self.options {
            use crate::dialogs::options::OptionsOutcome;
            match d.show(ctx) {
                OptionsOutcome::Open => {}
                OptionsOutcome::Cancelled => self.options = None,
                OptionsOutcome::Done(settings) => {
                    self.options = None;
                    self.apply_autosave_settings(settings);
                }
            }
        }
        if let Some(d) = &mut self.autosave_dialog {
            match d.show(ctx) {
                AutosaveOutcome::Open => {}
                AutosaveOutcome::Cancelled => self.autosave_dialog = None,
                AutosaveOutcome::Done(settings) => {
                    self.autosave_dialog = None;
                    self.apply_autosave_settings(settings);
                }
            }
        }
        // The offer waits while the question about unsaved changes is open.
        if self.pending.is_none()
            && let Some(offer) = &self.recovery_offer
        {
            match offer.show(ctx, std::time::SystemTime::now()) {
                RecoveryChoice::Open => {}
                RecoveryChoice::Later => {
                    self.recovery_offer = None;
                    self.say("The recovered project is kept for the next start");
                }
                RecoveryChoice::Discard => self.discard_newest(),
                RecoveryChoice::Recover => {
                    if self.doc.dirty {
                        self.pending = Some(Pending::Recover);
                    } else {
                        self.recover_newest();
                    }
                }
            }
        }
        if let Some(pending) = self.pending {
            let mut answer = None;
            egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
                ui.label(RichText::new("The project has unsaved changes.").strong());
                ui.label("Save them first?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        answer = Some(0);
                    }
                    if ui.button("Don't save").clicked() {
                        answer = Some(1);
                    }
                    if ui.button("Cancel").clicked() {
                        answer = Some(2);
                    }
                });
            });
            match answer {
                Some(0) => {
                    self.pending = None;
                    if self.save(false) {
                        self.do_pending(pending, ctx);
                    }
                }
                Some(1) => {
                    self.pending = None;
                    self.do_pending(pending, ctx);
                }
                Some(_) => {
                    self.pending = None;
                    self.pending_windows.clear();
                }
                None => {}
            }
        }
    }

    // ----- checking the interface without a person ------------------------

    /// Opens the things named by `--show` on the command line.
    fn run_startup_show(&mut self, ctx: &egui::Context) {
        let shows = self.startup.show.clone();
        for what in shows {
            let (name, arg) = what.split_once(':').unwrap_or((what.as_str(), ""));
            let project = self.doc.project();
            let first_graph = project.graphs().next().map(|g| g.id);
            let first_column = project.workbooks().next().and_then(|b| b.sheet().columns.first()).map(|c| c.id);
            let named = project.window_by_name(arg).map(Window::id);
            match name {
                "menu" => self.open_menu = Some(arg.to_string()),
                // The Window Switcher, held open (as from the menu).
                "switcher" => self.open_switcher(ctx, false, true),
                // panel:explorer (slid out), panel:explorer-docked, panel:explorer-hidden; the
                // names are explorer, objects, messages and results.
                "panel" => {
                    let (panel, mode) = arg.split_once('-').unwrap_or((arg, ""));
                    if let Some(id) = PanelId::from_name(panel) {
                        match mode {
                            "docked" => self.panels.set_pinned(id, true),
                            "hidden" => self.panels.hide(id),
                            _ => self.panels.open(id),
                        }
                    }
                }
                "select-column" | "column-menu" => {
                    let book = project.workbooks().next();
                    let found = book.and_then(|b| b.sheet().column_by_name(arg).map(|c| (b.id, c.id)));
                    if let Some((book, column)) = found {
                        let state = self.books.entry(book).or_default();
                        state.selected = vec![column];
                        state.sel = Sel::Columns;
                        if name == "column-menu" {
                            state.open_header_menu = Some(arg.to_string());
                        }
                        self.focus(book);
                    }
                }
                // hide-plot:N hides the N-th plot (from 1) of the first graph's first layer.
                "hide-plot" => {
                    if let (Some(graph), Ok(n)) = (first_graph, arg.parse::<usize>()) {
                        self.doc
                            .apply("Hide plot", |p| objects::set_plot_visible(p, graph, 0, n.saturating_sub(1), false));
                    }
                }
                "zoom" => {
                    // the first graph's window zoomed by the factor, its view scrolled to the page's centre
                    if let (Some(graph), Ok(f)) = (first_graph, arg.parse::<f32>()) {
                        let page = self.doc.project().graph(graph).map(|g| (g.page.width_pt, g.page.height_pt));
                        let factor = f.clamp(1.0, 32.0);
                        let (w, h) = page.unwrap_or((0.0, 0.0));
                        // the view shows 1/factor of the page; this top left centres it
                        let k = (1.0 - 1.0 / factor) / 2.0;
                        let scroll = egui::vec2(w as f32 * k, h as f32 * k);
                        self.graphs.entry(graph).or_default().zoom = crate::graph_zoom::Zoom { factor, scroll };
                    }
                }
                "merge-graphs" if first_graph.is_some() => {
                    let active = self.active.filter(|&id| project.graph(id).is_some()).or(first_graph);
                    self.merge_graphs = Some(MergeGraphsDialog::new(project, &self.panels.explorer.folder, active));
                }
                "add-layer" => {
                    if let Some(graph) = first_graph {
                        self.add_layer_dialog = crate::dialogs::add_layer::AddLayerDialog::new(&self.doc, graph, 0);
                        self.focus(graph);
                    }
                }
                "layer-management" => {
                    if let Some(graph) = first_graph {
                        self.layer_management = crate::dialogs::layer_management::LayerManagementDialog::new(
                            self.doc.project(),
                            graph,
                            None,
                        );
                    }
                }
                "fit-page" => {
                    if let Some(graph) = first_graph {
                        self.fit_page = Some(crate::dialogs::fit_page::FitPageDialog::new(graph));
                    }
                }
                "autosave" => {
                    self.autosave_dialog =
                        Some(AutosaveDialog::new(self.autosave.settings, self.autosave.has_session()));
                }
                "focus" => {
                    if let Some(id) = named {
                        self.focus(id);
                    }
                }
                "reader" => {
                    if let Some(id) = first_graph {
                        let state = self.graphs.entry(id).or_default();
                        state.reader = true;
                        state.reader_point = Some(crate::reader::Point { layer: 0, plot: 0, row: 2 });
                    }
                }
                // data-tool:mask, data-tool:move or data-tool:remove turns that Data menu tool on in the
                // first graph; mask-points:1,2 masks those rows (from 1) of its first plot.
                "data-tool" => {
                    let tool = match arg {
                        "mask" => Some(DataTool::Mask),
                        "move" => Some(DataTool::Move),
                        "remove" => Some(DataTool::RemoveBad),
                        _ => None,
                    };
                    if let (Some(id), Some(tool)) = (first_graph, tool) {
                        self.graphs.entry(id).or_default().data_tool = Some(tool);
                        self.say(crate::data_tools::hint(tool));
                        self.focus(id);
                    }
                }
                "mask-points" => {
                    let y = first_graph
                        .and_then(|g| project.graph(g))
                        .and_then(|g| g.layers.first()?.plots.first())
                        .map(|p| p.y);
                    if let Some(y) = y {
                        let cells: Vec<_> = arg
                            .split(',')
                            .filter_map(|r| r.trim().parse::<usize>().ok())
                            .map(|r| sheaf_core::DataCell::new(y, r.saturating_sub(1)))
                            .collect();
                        self.data_edit(DataEdit::Mask { cells, masked: true });
                    }
                }
                // text:N opens label N; text:x/y/legend opens existing title/legend text.
                "text" => {
                    if let Some(id) = first_graph {
                        let object = match arg {
                            "x" | "y" => {
                                self.text_dialog = TextDialog::axis_title(&self.doc, id, 0, arg == "y");
                                Some(if arg == "y" {
                                    sheaf_plot::scene::ObjectKind::YTitle
                                } else {
                                    sheaf_plot::scene::ObjectKind::XTitle
                                })
                            }
                            "legend" => {
                                self.text_dialog = TextDialog::legend(&self.doc, id, 0);
                                Some(sheaf_plot::scene::ObjectKind::Legend)
                            }
                            _ => arg.parse::<usize>().ok().map(|n| {
                                let label = n.saturating_sub(1);
                                self.text_dialog = TextDialog::edit(&self.doc, id, 0, label);
                                sheaf_plot::scene::ObjectKind::Label(label)
                            }),
                        };
                        if let Some(object) = object {
                            self.graphs.entry(id).or_default().selected = Some((0, object));
                            self.focus(id);
                        }
                    }
                }
                // shape:N opens the properties window of the N-th drawn object (from 1) of the
                // first graph's first layer and selects it.
                "shape" => {
                    if let (Some(id), Ok(n)) = (first_graph, arg.parse::<usize>()) {
                        let shape = n.saturating_sub(1);
                        self.shape_dialog = ShapeDialog::edit(&self.doc, id, 0, shape);
                        self.graphs.entry(id).or_default().selected =
                            Some((0, sheaf_plot::scene::ObjectKind::Shape(shape)));
                        self.focus(id);
                    }
                }
                "integrate" => {
                    if let Some(id) = first_graph {
                        self.start_gadget(id, crate::gadget::GadgetKind::Integrate);
                    }
                }
                "plot-setup" => {
                    if let Some(id) = first_graph {
                        self.plot_setup = Some(crate::dialogs::plot_setup::PlotSetupDialog::new(
                            id,
                            0,
                            1_000_000,
                            self.doc.project(),
                        ));
                        self.focus(id);
                    }
                }
                "layer-contents" => {
                    if let Some(id) = first_graph {
                        self.layer_contents =
                            Some(crate::dialogs::layer_contents::LayerContentsDialog::new(id, 0, 1_000_000));
                        self.focus(id);
                    }
                }
                "find" | "replace" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let Some(book) = book {
                        self.open_find(book, name == "replace");
                        self.focus(book);
                    }
                }
                "go-to" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let Some(book) = book {
                        self.open_go_to(book);
                        self.focus(book);
                    }
                }
                "stack-panels" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let Some(book) = book {
                        let columns = self.plot_y_columns(book, Vec::new(), Template::Line);
                        self.stack_panels = StackPanelsDialog::new(self.doc.project(), book, columns, None);
                        self.focus(book);
                    }
                }
                // simulate or simulate:FUNCTION opens Analysis > Fitting > Simulate Curve on the first workbook.
                "simulate" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    let mut d = SimulateDialog::new(self.doc.project(), book);
                    if !arg.is_empty() {
                        d.choose_function_named(arg);
                    }
                    self.simulate = Some(d);
                }
                // simulate-surface or simulate-surface:FUNCTION opens Analysis > Fitting > Simulate Surface.
                "simulate-surface" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    let mut d = SimulateSurfaceDialog::new(self.doc.project(), book);
                    if !arg.is_empty() {
                        d.choose_function_named(arg);
                    }
                    self.simulate_surface = Some(d);
                }
                "label-panels" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let Some(book) = book {
                        let columns = self.plot_y_columns(book, Vec::new(), Template::Line);
                        self.label_panels = LabelPanelsDialog::new(self.doc.project(), book, columns, None);
                        self.focus(book);
                    }
                }
                "error-bars" => {
                    if let Some(id) = first_graph {
                        self.add_error_bars(id, 0);
                        self.focus(id);
                    }
                }
                "graph-settings" => {
                    if let Some(id) = first_graph {
                        let tab = match arg {
                            "y" => PropsTab::YAxis,
                            "plots" => PropsTab::Plots,
                            "legend" => PropsTab::Legend,
                            "page" => PropsTab::Page,
                            _ => PropsTab::XAxis,
                        };
                        self.graph_props = Some(GraphPropsDialog::new(id, 0, tab, 1_000_000));
                        self.focus(id);
                    }
                }
                "column" => {
                    if let Some(c) = first_column {
                        self.column_props = ColumnPropsDialog::new(self.doc.project(), c);
                    }
                }
                "set-values" => {
                    if let Some(c) = first_column {
                        self.set_values = SetValuesDialog::new(self.doc.project(), c);
                    }
                }
                // set-values-multiple or set-values-multiple:B,C,D opens Set Multiple Columns Values
                // on those columns of the first workbook (every column but the first by default).
                "set-values-multiple" => {
                    if let Some(book) = project.workbooks().next() {
                        let sheet = book.sheet();
                        let columns: Vec<ColumnId> = if arg.is_empty() {
                            sheet.columns.iter().skip(1).map(|c| c.id).collect()
                        } else {
                            arg.split(',').filter_map(|n| sheet.column_by_name(n.trim())).map(|c| c.id).collect()
                        };
                        self.set_values = SetValuesDialog::new_multiple(self.doc.project(), &columns);
                    }
                }
                "analysis" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let (Some(book), Some(kind)) = (book, AnalysisKind::from_name(arg)) {
                        self.open_analysis(book, Vec::new(), kind);
                        self.focus(book);
                    }
                }
                // The same window with Auto Preview ticked (for pictures of the preview pane).
                "analysis-preview" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let (Some(book), Some(kind)) = (book, AnalysisKind::from_name(arg)) {
                        self.open_analysis(book, Vec::new(), kind);
                        if let Some(d) = &mut self.analysis {
                            d.tick_auto_preview();
                        }
                        self.focus(book);
                    }
                }
                "fit" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let (Some(book), Some(kind)) = (book, FitKind::from_name(arg)) {
                        self.fit = FitDialog::new(self.doc.project(), book, None, kind);
                        self.focus(book);
                    }
                }
                "fitmore" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let (Some(book), Some(kind)) = (book, FitMoreKind::from_name(arg)) {
                        self.fit_more = FitMoreDialog::new(self.doc.project(), book, None, kind);
                        self.focus(book);
                    }
                }
                "filter" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let Some(book) = book {
                        self.filter = FilterDialog::new(self.doc.project(), book);
                        self.focus(book);
                    }
                }
                "export" => {
                    if let Some(id) = first_graph {
                        self.export = Some(ExportDialog::new(id, None).with_themes(self.startup.themes_dir.clone()));
                    }
                }
                "import-wizard" => {
                    let files = if arg.is_empty() { Vec::new() } else { vec![PathBuf::from(arg)] };
                    self.import_wizard = Some(self.wizard(files));
                }
                // import-wizard-page:header, variables, file, data, selection or save (after import-wizard).
                "import-wizard-page" => {
                    let page = match arg {
                        "header" => WizardPage::HeaderLines,
                        "variables" => WizardPage::VariableExtraction,
                        "selection" => WizardPage::DataSelection,
                        "file" => WizardPage::FileNameOptions,
                        "data" => WizardPage::DataColumns,
                        "save" => WizardPage::SaveFilters,
                        _ => WizardPage::Source,
                    };
                    if let Some(w) = &mut self.import_wizard {
                        w.show_page(page);
                    }
                }
                "import-excel" => {
                    if !arg.is_empty() {
                        self.import_excel = Some(self.excel_dialog(vec![PathBuf::from(arg)]));
                    }
                }
                // import-excel-page:mode, header, columns, partial or misc (after import-excel).
                "import-excel-page" => {
                    if let Some(d) = &mut self.import_excel {
                        d.show_page(arg);
                    }
                }
                "export-excel" => {
                    if let Some(book) = project.workbooks().next().map(|b| (b.id, b.sheets.len())) {
                        self.export_excel = Some(ExcelExportDialog::new(book.0, book.1, None));
                    }
                }
                "import" | "import-data-options" => {
                    if !arg.is_empty() {
                        let mut d = self.import_dialog(vec![PathBuf::from(arg)]);
                        if name == "import-data-options" {
                            d.show_data_import_options();
                        }
                        self.import = Some(d);
                    }
                }
                _ => eprintln!("unknown --show item '{what}'"),
            }
        }
    }

    fn screenshot_step(&mut self, ctx: &egui::Context) {
        let Some(path) = self.startup.screenshot.clone() else {
            return;
        };
        ctx.request_repaint();
        if self.frame == 20 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let image = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_srgba_unmultiplied()).collect();
            let result = image::save_buffer(
                &path,
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ExtendedColorType::Rgba8,
            );
            if let Err(e) = result {
                eprintln!("could not write the screenshot: {e}");
            }
            self.quit_confirmed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// The report of a fit as plain lines for the Results Log: the equation, the parameters with their
/// standard errors, the fit statistics and any warnings.
fn fit_lines(outcome: &sheaf_core::fitting::FitOutcome) -> Vec<String> {
    let n = |v: f64| sheaf_core::format::format_general(v, 6);
    let mut lines = fit_report_lines(&outcome.report);
    let at = lines.iter().position(|l| l.starts_with("  Note: ")).unwrap_or(lines.len());
    lines.splice(at..at, outcome.derived.iter().map(|(name, value)| format!("  {name} = {}", n(*value))));
    lines
}

/// The lines of [`fit_lines`] that come from the report alone.
fn fit_report_lines(r: &sheaf_core::fitting::Report) -> Vec<String> {
    let n = |v: f64| sheaf_core::format::format_general(v, 6);
    let mut lines = vec![format!("{}: {}", r.model, r.equation)];
    for p in &r.params {
        lines.push(if p.fixed {
            format!("  {} = {} (fixed)", p.name, n(p.value))
        } else {
            format!("  {} = {} +/- {}", p.name, n(p.value), n(p.std_error))
        });
    }
    let st = &r.stats;
    lines.push(format!(
        "  N = {}, R-Square (COD) = {}, Adj. R-Square = {}, Reduced Chi-Sqr = {}",
        st.n_points,
        n(st.r_squared),
        n(st.adj_r_squared),
        n(st.reduced_chi_sqr)
    ));
    lines.extend(r.warnings.iter().map(|w| format!("  Note: {w}")));
    lines
}

/// Removes columns from every sheet, and from every graph the plots that used them.
/// Duplicate: a copy of sheet `sheet` with its data right after it, active; its columns get new
/// ids (their formats, notes and conditional formats follow them).
fn duplicate_sheet(p: &mut Project, book: WindowId, sheet: usize) {
    let Some(b) = p.workbook(book) else { return };
    let Some(src) = b.sheets.get(sheet).cloned() else { return };
    let mut n = b.sheets.len() + 1;
    let name = loop {
        let name = format!("{}_{n}", src.name);
        if !b.sheets.iter().any(|s| s.name == name) {
            break name;
        }
        n += 1;
    };
    let mut copy = p.sheet_copy(&src);
    copy.name = name;
    if let Some(b) = p.workbook_mut(book) {
        b.sheets.insert(sheet + 1, copy);
        b.active_sheet = sheet + 1;
    }
}

pub(crate) fn delete_columns(project: &mut Project, columns: &[ColumnId]) {
    if columns.is_empty() {
        return;
    }
    for w in &mut project.windows {
        match w {
            Window::Workbook(b) => {
                for s in &mut b.sheets {
                    s.columns.retain(|c| !columns.contains(&c.id));
                    s.remap_cond_format_columns(&|c| (!columns.contains(&c)).then_some(c));
                    s.remap_note_columns(&|c| (!columns.contains(&c)).then_some(c));
                    s.remap_format_columns(&|c| (!columns.contains(&c)).then_some(c));
                }
            }
            Window::Notes(_) => {}
            Window::Matrix(_) | Window::Layout(_) => {}
            Window::Graph(g) => {
                for l in &mut g.layers {
                    l.plots.retain(|p| !columns.contains(&p.y));
                    for p in &mut l.plots {
                        if p.x.is_some_and(|x| columns.contains(&x)) {
                            p.x = None;
                        }
                        if p.y_error.is_some_and(|e| columns.contains(&e)) {
                            p.y_error = None;
                        }
                        if p.x_error.is_some_and(|e| columns.contains(&e)) {
                            p.x_error = None;
                        }
                        if p.y_base.is_some_and(|b| columns.contains(&b)) {
                            p.y_base = None;
                        }
                        // Data labels that read a deleted column go back to the Y value.
                        if let LabelForm::Column { column } = p.decor.labels.form
                            && columns.contains(&column)
                        {
                            p.decor.labels.form = LabelForm::Y;
                        }
                    }
                    // Stack settings and tick labels that name a deleted column are dropped too.
                    l.plot_offsets.retain(|o| !columns.contains(&o.column));
                    for o in &mut l.plot_offsets {
                        if o.follows.is_some_and(|f| columns.contains(&f)) {
                            o.follows = None;
                        }
                    }
                    for axis in [&mut l.x_axis, &mut l.y_axis] {
                        if let TickLabelSource::Column { column } = axis.tick_label_source
                            && columns.contains(&column)
                        {
                            axis.tick_label_source = TickLabelSource::Auto;
                        }
                    }
                }
            }
        }
    }
}

impl eframe::App for SheafApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame += 1;
        let mut actions: Vec<Action> = Vec::new();

        // Files dropped onto the window.
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            self.open_paths(dropped, true);
        }
        // Closing the window with unsaved changes asks first.
        if ctx.input(|i| i.viewport().close_requested()) && self.doc.dirty && !self.quit_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Quit);
        }
        self.active = self.workspace.active();
        let full = self.full_screen.is_some();
        // Full Screen shows only the page: the editing shortcuts stay off while it is up.
        if !full {
            self.shortcuts(ctx, &mut actions);
        }

        if !full {
            egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| self.menu_bar(ui, &mut actions));
        }
        if !full {
            let c = self.menu_context();
            let format = self.text_format();
            let object = self.object_format();
            let tb = &self.toolbars;
            let bar_frame = || egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(3, 1));
            if tb.shown(Bar::Standard) {
                let zoom = c.graph.map(|id| self.graphs.get(&id).map_or(1.0, |state| state.zoom.factor));
                egui::TopBottomPanel::top("toolbar_standard")
                    .frame(bar_frame())
                    .show(ctx, |ui| toolbar::show_standard(ui, &c, zoom, &mut actions));
            }
            let second = [Bar::Edit, Bar::Format, Bar::Graph];
            if tb.any(&second) {
                egui::TopBottomPanel::top("toolbar_second").frame(bar_frame()).show(ctx, |ui| {
                    toolbar::show_row_with_format(ui, &second, tb, &c, format.as_ref(), object.as_ref(), &mut actions)
                });
            }
            if !self.status_bar_hidden {
                egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| self.status_bar(ui));
            }
            if tb.shown(Bar::Graphs2D) {
                egui::TopBottomPanel::bottom("toolbar_2d_graphs")
                    .frame(bar_frame())
                    .show(ctx, |ui| toolbar::show_row(ui, &[Bar::Graphs2D], tb, &c, &mut actions));
            }
        }
        if !full {
            let data = panel_data(&self.doc, &self.logs, self.active, &self.path, &self.graph_props);
            self.panels.strips_and_docks(ctx, &data, &mut actions);
        }
        if !full {
            let c = self.menu_context();
            let tb = &self.toolbars;
            let side_frame = || egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(1, 3));
            if tb.shown(Bar::Tools) {
                egui::SidePanel::left("toolbar_tools").frame(side_frame()).resizable(false).exact_width(24.0).show(
                    ctx,
                    |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        toolbar::show_bar(ui, Bar::Tools, &c, &mut actions);
                    },
                );
            }
            if tb.shown(Bar::ObjectEdit) {
                egui::SidePanel::right("toolbar_object_edit")
                    .frame(side_frame())
                    .resizable(false)
                    .exact_width(24.0)
                    .show(ctx, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        toolbar::show_bar(ui, Bar::ObjectEdit, &c, &mut actions);
                    });
            }
        }
        if let Some(fs) = &mut self.full_screen {
            if fs.show(ctx, self.doc.project(), self.doc.revision()) == crate::view_modes::Event::End {
                self.full_screen_reminder_off |= fs.never_remind;
                self.full_screen = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
        } else {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::WORKSPACE_GREY)).show(ctx, |ui| {
                if self.doc.project().windows.is_empty() {
                    self.welcome(ui, &mut actions);
                } else {
                    let doc = &self.doc;
                    let books = &mut self.books;
                    let formula_bars = &mut self.formula_bars;
                    let show_formula_bar = self.show_formula_bar;
                    let graphs = &mut self.graphs;
                    let notes = &mut self.notes;
                    let matrices = &mut self.matrices;
                    let layouts = &mut self.layouts;
                    let actions = &mut actions;
                    // View > View Windows: windows outside the active folder are not drawn (T100).
                    let shown = explorer::shown_windows(doc.project(), &self.panels.explorer.folder);
                    let info = |id: WindowId| {
                        if shown.as_ref().is_some_and(|s| !s.contains(&id)) {
                            return None;
                        }
                        doc.project()
                            .window(id)
                            .map(|w| WindowInfo { title: w.title(), graph: matches!(w, Window::Graph(_)) })
                    };
                    let mut content = |id: WindowId, ui: &mut egui::Ui| match doc.project().window(id) {
                        Some(Window::Workbook(b)) => {
                            // Sticky notes (T421): the book's, then the shown sheet's.
                            use sheaf_core::sticky::NoteTarget;
                            crate::sticky_view::strip(ui, doc, NoteTarget::Window(id), "Book note", actions);
                            let sheet = b.active_sheet;
                            crate::sticky_view::strip(ui, doc, NoteTarget::Sheet(id, sheet), "Sheet note", actions);
                            // The Formula Bar (T448), when View > Formula Bar is on.
                            if show_formula_bar {
                                let bar = formula_bars.entry(id).or_default();
                                crate::formula_bar::show(ui, doc, id, books.get(&id), bar, actions);
                            }
                            // The grid fills its ui's whole rectangle: give it only the room below the notes.
                            let rest = ui.available_rect_before_wrap();
                            ui.scope_builder(egui::UiBuilder::new().max_rect(rest), |ui| {
                                grid::show(ui, doc, id, books.entry(id).or_default(), actions);
                            });
                        }
                        Some(Window::Graph(_)) => {
                            use sheaf_core::sticky::NoteTarget;
                            crate::sticky_view::strip(ui, doc, NoteTarget::Window(id), "Note", actions);
                            graph_view::show(ui, doc, id, graphs.entry(id).or_default(), actions);
                        }
                        Some(Window::Notes(_)) => {
                            crate::notes_view::show(ui, doc, id, notes.entry(id).or_default(), actions);
                        }
                        // T192 slice 1 ports the matrix model, files, plots and command line; the
                        // matrix window itself comes with the next slice.
                        Some(Window::Matrix(_)) => {
                            // Sticky note (T460), then the matrix below it.
                            let target = sheaf_core::sticky::NoteTarget::Window(id);
                            crate::sticky_view::strip(ui, doc, target, "Note", actions);
                            let rest = ui.available_rect_before_wrap();
                            ui.scope_builder(egui::UiBuilder::new().max_rect(rest), |ui| {
                                crate::matrix_view::show(ui, doc, id, matrices.entry(id).or_default(), actions);
                            });
                        }
                        Some(Window::Layout(_)) => {
                            let target = sheaf_core::sticky::NoteTarget::Window(id);
                            crate::sticky_view::strip(ui, doc, target, "Note", actions);
                            let rest = ui.available_rect_before_wrap();
                            ui.scope_builder(egui::UiBuilder::new().max_rect(rest), |ui| {
                                crate::layout_view::show(ui, doc, id, layouts.entry(id).or_default(), actions);
                            });
                        }
                        None => {}
                    };
                    self.workspace.show(ui, doc.dirty, &info, &mut content);
                }
            });
        }

        if !full {
            let data = panel_data(&self.doc, &self.logs, self.active, &self.path, &self.graph_props);
            self.panels.flyouts(ctx, &data, &mut actions);
        }
        self.dialogs(ctx);
        if let Some(s) = &mut self.switcher {
            match s.show(ctx) {
                Some(crate::switcher::Click::Window(id)) => {
                    actions.push(Action::Focus(id));
                    self.switcher = None;
                }
                Some(crate::switcher::Click::Dismiss) => self.switcher = None,
                None => {}
            }
        }
        for action in actions {
            self.apply(action, ctx);
        }
        self.live_step();
        self.sync_windows();
        self.autosave_step(ctx);
        match &self.message {
            Some(m) if self.logged_message.as_ref() != Some(m) => {
                self.logs.warn(format!("{}: {}", m.0, m.1.replace('\n', " ")));
                self.logged_message = Some(m.clone());
            }
            None => self.logged_message = None,
            Some(_) => {}
        }

        let title = main_title(
            self.path.as_deref(),
            self.doc.dirty,
            self.active
                .filter(|id| self.workspace.mode(*id) == Some(crate::workspace::Mode::Maximised))
                .and_then(|id| self.doc.project().window(id))
                .map(|w| w.title()),
        );
        if title != self.shown_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.shown_title = title;
        }
        self.screenshot_step(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.live.stop();
        self.end_autosave();
    }
}

/// The title of the main window as Origin writes it: `<App> - <path> *`, and `- [Window]` when
/// a child window is maximised. The star marks unsaved changes.
fn main_title(path: Option<&Path>, dirty: bool, maximised: Option<String>) -> String {
    let star = if dirty { " *" } else { "" };
    let path = path.map_or_else(|| "UNTITLED".to_string(), |p| p.display().to_string());
    match maximised {
        Some(w) => format!("{APP_NAME} - {path}{star} - [{w}{star}]"),
        None => format!("{APP_NAME} - {path}{star}"),
    }
}

/// What the side panels read, borrowed from the parts of the application that they need (so that
/// the panels themselves can be borrowed mutably at the same time).
fn panel_data<'a>(
    doc: &'a Document,
    logs: &'a Logs,
    active: Option<WindowId>,
    path: &Option<PathBuf>,
    props: &Option<GraphPropsDialog>,
) -> PanelData<'a> {
    let project_name = path
        .as_ref()
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "UNTITLED".to_string());
    let props_plot = props.as_ref().filter(|d| Some(d.graph) == active).map(GraphPropsDialog::selected_plot);
    PanelData { project: doc.project(), active, project_name, props_plot, logs }
}

/// Theme Organizer's Apply Now on a graph (T102): an axis theme goes to every axis of every layer, a
/// graph theme to the whole graph, a legend theme (T167) to every shown legend.
pub(crate) fn apply_graph_theme(g: &mut sheaf_core::graph::Graph, format: &sheaf_core::format_clip::FormatClip) {
    use sheaf_core::format_clip::{AxisPart, FormatClip, paste_axis, paste_graph};
    match format {
        FormatClip::Axis(_) => {
            for layer in 0..g.layers.len() {
                for y in [false, true] {
                    paste_axis(g, layer, y, format, AxisPart::All);
                }
            }
        }
        FormatClip::Graph(f) => {
            let groups = f.groups().to_vec();
            paste_graph(g, format, &groups);
        }
        FormatClip::LegendStyles(_) => {
            for layer in 0..g.layers.len() {
                sheaf_core::format_clip::paste_legend_styles(g, layer, format);
            }
        }
    }
}

/// Set Sampling Interval's "From X Column" (T228): the X column the column is plotted against, checked
/// for even spacing (Origin's `vinc`); `None` when it has no X column.
fn sampling_from_x(project: &sheaf_core::Project, column: ColumnId) -> Option<crate::dialogs::sampling::FromX> {
    use sheaf_core::sampling::{INCREMENT_TOLERANCE, check_increment};
    let (_, sheet, _) = project.locate(column)?;
    let x = sheet.x_for(sheet.column_index(column)?)?;
    let values = x.numeric().unwrap_or(&[]);
    let finite_end = values.iter().rposition(|v| v.is_finite()).map_or(0, |i| i + 1);
    Some(match check_increment(&values[..finite_end], INCREMENT_TOLERANCE) {
        Some(c) if c.uniform => Ok((c.x0, c.inc, x.units.clone(), x.long_name.clone())),
        Some(c) => Err(format!(
            "X column {} is not evenly spaced: a step differs from the mean step {} by {}",
            x.short_name, c.inc, c.max_deviation
        )),
        None => Err(format!("X column {} needs at least two numbers and no gaps", x.short_name)),
    })
}

/// The importer a book's source file is read again with (Re-Import, T98); `None` for a book not
/// imported from a data file (made in Sheaf, or opened from a project).
/// A PDF file, by its extension (T463).
pub(crate) fn is_pdf(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

pub(crate) fn reimport_kind(book: &sheaf_core::Workbook) -> Option<sheaf_io::ImportKind> {
    match book.source.as_ref()?.importer.as_str() {
        "ascii" => Some(sheaf_io::ImportKind::Ascii),
        "chi" => Some(sheaf_io::ImportKind::Chi),
        "excel" | "pdf" => Some(sheaf_io::ImportKind::Auto),
        _ => None,
    }
}

/// Reads `path` again with `kind` (default options) and replaces the data of `book` with it, one undo
/// step; the book's source becomes `path`.
pub(crate) fn reimport_book(
    doc: &mut sheaf_core::Document,
    book: WindowId,
    path: &Path,
    kind: sheaf_io::ImportKind,
) -> Result<sheaf_core::reimport::ReimportReport, String> {
    let fresh =
        sheaf_io::import_file(path, kind, &Default::default(), &Default::default()).map_err(|e| e.to_string())?;
    doc.edit("Re-import", |p| {
        let r = p.replace_book_data(book, &fresh.workbook)?;
        if let Some(b) = p.workbook_mut(book) {
            b.source = fresh.workbook.source.clone();
        }
        Ok::<_, sheaf_core::project::ProjectError>(r)
    })
    .map_err(|e| e.to_string())
}

/// Writes the worksheet picture of File > Export > Worksheet as Image to `path`, the kind chosen by
/// its extension (PNG at 300 dpi when there is none). Returns the path written.
pub(crate) fn write_worksheet_image(sheet: &sheaf_core::Worksheet, path: &Path) -> Result<PathBuf, String> {
    let mut path = path.to_path_buf();
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    if ext.is_none() {
        path.set_extension("png");
    }
    let bytes =
        sheaf_plot::sheet_image_bytes(sheet, ext.as_deref().unwrap_or("png"), 300.0).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod ui_tests;

/// True when both paths name the same file (or, if one is missing, the same path text).
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}
