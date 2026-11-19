//! The application window: menus, project explorer, the workspace with its
//! workbook and graph child windows, dialogs, and the one place where user actions change the
//! project.

use crate::actions::{
    Action, CellOp, CopyKind, DataEdit, DataTool, GraphOp, GraphTool, ImportPick, LabelRow, SheetOp, ViewOp,
};
use crate::autosave::{Autosave, Due};
use crate::dialogs::analysis::{AnalysisDialog, AnalysisKind, AnalysisOutcome, FilterDialog, FilterOutcome};
use crate::dialogs::autosave::{AutosaveDialog, AutosaveOutcome};
use crate::dialogs::cell_format::CellFormatDialog;
use crate::dialogs::cell_note::CellNoteDialog;
use crate::dialogs::column_props::ColumnPropsDialog;
use crate::dialogs::comparison_plot::{ComparisonPlotDialog, Outcome as ComparisonPlotOutcome};
use crate::dialogs::cond_format::CondFormatDialog;
use crate::dialogs::cond_manager::{CondManagerDialog, ManagerOutcome};
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
use sheaf_core::graph::{LabelForm, TickLabelSource};
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

struct RenameDialog {
    window: WindowId,
    long_name: String,
    /// The window's short name (Origin's Window Properties have both); cleaned by Origin's rule on OK.
    short_name: String,
    /// Why the last OK was refused (a short name another window has).
    error: Option<String>,
    /// Set when a folder is renamed instead of a window: its path.
    folder: Option<Vec<usize>>,
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
    /// The Origin file the project was imported from; saving over it first keeps a `.bak` copy.
    foreign_source: Option<PathBuf>,
    workspace: Workspace,
    /// Windows that were already shown once. A window the user closed is not reopened by itself.
    known: HashSet<WindowId>,
    active: Option<WindowId>,
    books: HashMap<WindowId, BookUi>,
    graphs: HashMap<WindowId, GraphUi>,
    import: Option<ImportDialog>,
    import_excel: Option<ExcelImportDialog>,
    import_wizard: Option<ImportWizard>,
    graph_props: Option<GraphPropsDialog>,
    layer_contents: Option<crate::dialogs::layer_contents::LayerContentsDialog>,
    layer_management: Option<crate::dialogs::layer_management::LayerManagementDialog>,
    fit_page: Option<crate::dialogs::fit_page::FitPageDialog>,
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
    function_graph: Option<crate::dialogs::function_graph::FunctionGraphDialog>,
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
    power: Option<PowerDialog>,
    find: Option<FindDialog>,
    sort: Option<SortDialog>,
    sheet_name: Option<SheetNameDialog>,
    /// The text of the last Copy or Cut (the system clipboard has it too).
    copied: String,
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
            foreign_source: None,
            workspace: Workspace::default(),
            known: HashSet::new(),
            active: None,
            books: HashMap::new(),
            graphs: HashMap::new(),
            import: None,
            import_excel: None,
            import_wizard: None,
            graph_props: None,
            layer_contents: None,
            layer_management: None,
            fit_page: None,
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
            export: None,
            export_sheet: None,
            export_excel: None,
            page_setup: None,
            print_preview: None,
            rename: None,
            go_to: None,
            template_prompt: None,
            template_manager: None,
            fill_pattern: None,
            sampling: None,
            copy_columns: None,
            sheet_navigate: None,
            find_project: None,
            find_spec: Default::default(),
            batch_plot: None,
            qc_chart: None,
            function_graph: None,
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
            power: None,
            find: None,
            sort: None,
            sheet_name: None,
            copied: String::new(),
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
            if files.is_empty() {
                continue;
            }
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
        let kind = WindowFileKind::of_window(window);
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
        let kind = WindowFileKind::of_window(window);
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
        self.fit_page = None;
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
        self.sort = None;
        self.sheet_name = None;
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
        state.gadget = Some(crate::gadget::Integrator { kind, layer, plot, x0, x1, y0, y1, end_point_baseline: false });
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
            GadgetOp::Close => state.gadget = None,
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
                        area_type: Default::default(),
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

    /// Graph > New Layer(Axes): one undo step; the new layer becomes the active layer.
    fn add_layer(&mut self, graph: WindowId, active: usize, kind: sheaf_core::layers::NewLayerKind) {
        let Some(g) = self.doc.project().graph(graph) else { return };
        if active >= g.layers.len() {
            return;
        }
        self.doc.apply(sheaf_core::layers::ADD_LAYER_UNDO, |p| {
            graph_view::apply_op(p, graph, GraphOp::AddLayer { layer: active, kind }, None)
        });
        let count = self.doc.project().graph(graph).map_or(0, |g| g.layers.len());
        let state = self.graphs.entry(graph).or_default();
        state.active_layer = count.saturating_sub(1);
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
            self.add_layer(graph, layer, kind);
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
            Action::NewWorkbook => {
                self.doc.apply("New workbook", |p| {
                    let book = p.add_workbook("");
                    let _ = p.add_column(book, 0, Designation::X, ColumnData::default());
                    let _ = p.add_column(book, 0, Designation::Y, ColumnData::default());
                });
            }
            Action::AddColumn(book) => {
                self.doc.apply("Add column", |p| {
                    let sheet = p.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
                    let _ = p.add_column(book, sheet, Designation::Y, ColumnData::default());
                });
            }
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
            Action::SaveAxisTheme { graph, layer, y } => {
                let Some(dir) = self.startup.graph_themes_dir.clone() else {
                    self.warn("Sheaf has no settings folder for graph themes");
                    return;
                };
                let clip = self.doc.project().graph(graph).and_then(|g| {
                    sheaf_core::format_clip::copy_axis(g, layer, y, sheaf_core::format_clip::AxisPart::All)
                });
                if let Some(clip) = clip {
                    let existing = sheaf_io::graph_themes::list(&dir).into_iter().map(|e| e.name).collect();
                    let name = sheaf_io::graph_themes::next_name(&dir);
                    self.save_theme = Some((crate::dialogs::graph_themes::SaveThemeDialog::new(name, existing), clip));
                }
            }
            Action::NewFromTemplate(name) => self.new_from_template(name),
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
                    self.panels.explorer.selected = None;
                    self.follow_folder();
                }
            }
            Action::FolderProperties(path) => {
                if let Some(f) = explorer::folder_at(&self.doc.project().root, &path) {
                    self.folder_props = Some(crate::dialogs::folder_props::FolderPropsDialog::new(path, f));
                }
            }
            Action::AddFunctionGraph { graph, layer } => {
                self.function_graph = Some(crate::dialogs::function_graph::FunctionGraphDialog::new(graph, layer));
            }
            Action::BatchPlot { graph, columns } => {
                match crate::dialogs::batch_plot::BatchPlotDialog::new(self.doc.project(), graph, columns) {
                    Some(d) => self.batch_plot = Some(d),
                    None => self.warn("The graph plots no worksheet column"),
                }
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
            Action::SetCell { column, row, text } => {
                let beyond_end = self.doc.project().column(column).is_none_or(|c| row >= c.len());
                if !(beyond_end && text.trim().is_empty()) {
                    let mut understood = true;
                    self.doc.apply("Edit cell", |p| {
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
                            LabelRow::Formula | LabelRow::SamplingInterval => {}
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
                if self.doc.project().graphs().count() < 2 {
                    self.warn("Merge Graph Windows needs at least two graphs");
                } else {
                    self.merge_graphs = Some(MergeGraphsDialog::new(self.doc.project(), &self.panels.explorer.folder));
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
                self.worksheet_fonts = WorksheetFontsDialog::new(&self.doc, book);
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
            Action::SetLayerIcons { graph, on } => {
                self.doc.apply("Show Layer Icons", |p| {
                    if let Some(g) = p.graph_mut(graph) {
                        g.page.style.layer_icons = on;
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
                use sheaf_core::format_clip::GraphPart;
                crate::format_clip_ui::paste_graph(&mut self.doc, graph, GraphPart::All.groups());
            }
            Action::PasteAxisFormat { graph, layer, y, part } => {
                crate::format_clip_ui::paste_axis(&mut self.doc, graph, layer, y, part);
            }
            Action::CopyLegendStyles { graph, layer } => {
                if crate::format_clip_ui::copy_legend_styles(&self.doc, graph, layer) {
                    self.say("Format copied");
                }
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
            Action::StartVerticalCursor { graph } => {
                self.start_gadget(graph, crate::gadget::GadgetKind::VerticalCursor)
            }
            Action::Gadget { graph, op } => self.gadget(graph, op),
            Action::ExportGraph(graph) => {
                self.export =
                    Some(ExportDialog::new(graph, self.folder()).with_themes(self.startup.themes_dir.clone()));
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
            Action::SaveTemplateAs(graph) => {
                let name = self.doc.project().graph(graph).map(|g| g.name.clone()).unwrap_or_default();
                self.template_prompt = Some(crate::dialogs::user_templates::SaveTemplatePrompt::new(
                    graph,
                    &name,
                    self.user_templates.clone(),
                ));
            }
            Action::ManageTemplates => {
                self.template_manager =
                    Some(crate::dialogs::user_templates::TemplateManager::new(self.user_templates.clone()));
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
            Action::CopyLabel { graph, layer, label, cut } => {
                let copied =
                    self.doc.project().graph(graph).is_some_and(|g| crate::shape_clip::copy_label(g, layer, label));
                if copied && cut {
                    self.edit_graph(graph, crate::actions::GraphOp::DeleteLabel { layer, label });
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
                self.sampling = Some(crate::dialogs::sampling::SamplingDialog::new(columns.clone(), current));
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
            Action::RenameWindow(id) => {
                if let Some(w) = self.doc.project().window(id) {
                    let long_name = match w {
                        Window::Workbook(b) => b.long_name.clone(),
                        Window::Graph(g) => g.long_name.clone(),
                    };
                    let short_name = w.name().to_string();
                    self.rename = Some(RenameDialog { window: id, long_name, short_name, error: None, folder: None });
                }
            }
            Action::Cascade => self.workspace.cascade(),
            Action::TileHorizontally => self.workspace.tile_horizontally(),
            Action::TileVertically => self.workspace.tile_vertically(),
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
                let what = format!("{} row(s) of {} column(s)", t.rows.len(), t.columns.len());
                if matches!(op, CellOp::Cut) {
                    self.clear_cells(book, sheet_index, &t, "Cut cells");
                    self.say(format!("Cut {what}"));
                } else {
                    self.say(format!("Copied {what}"));
                }
            }
            CellOp::Clear => self.clear_cells(book, sheet_index, &t, "Clear cells"),
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
                let mut copy = b.sheets[active].clone();
                copy.name = free_name(&b.sheets, &format!("{}_", copy.name));
                self.doc.apply("Duplicate sheet", |p| {
                    let mut renamed = std::collections::HashMap::new();
                    for c in &mut copy.columns {
                        let new = p.new_column_id();
                        renamed.insert(c.id, new);
                        c.id = new;
                    }
                    copy.remap_cond_format_columns(&|c| renamed.get(&c).copied());
                    copy.remap_note_columns(&|c| renamed.get(&c).copied());
                    copy.remap_format_columns(&|c| renamed.get(&c).copied());
                    if let Some(b) = p.workbook_mut(book) {
                        b.sheets.insert(active + 1, copy);
                        b.active_sheet = active + 1;
                    }
                });
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
        // A Parallel Index is grouped by the selected column of text (else the first text column of the sheet).
        let group = if template == Template::ParallelIndex {
            columns.iter().copied().find(|id| self.doc.project().column(*id).is_some_and(|c| !c.data.is_numeric()))
        } else {
            None
        };
        // A Treemap (or Sunburst plot) nests by the selected columns of text (in selection order; else every text column of the
        // sheet) and sizes by the last numeric one.
        let categories: Vec<ColumnId> = if matches!(template, Template::Treemap | Template::Sunburst) {
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
        // A Pareto chart counts the cells of the one selected column, text or numbers.
        if template == Template::Pareto {
            let [column] = columns[..] else {
                self.warn("Select one column whose values are counted for the Pareto chart".to_string());
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
        if ys.is_empty() {
            let what = if matches!(template, Template::Treemap | Template::Sunburst) {
                "Nothing to plot: select one or more text category columns and a numeric value column"
            } else {
                "Nothing to plot: select one or more numeric Y columns"
            };
            self.warn(what.to_string());
            return;
        }
        let made = self.doc.edit("Plot columns", |p| match group {
            Some(g) => p.plot_parallel(template, &ys, Some(g)),
            None if matches!(template, Template::Treemap | Template::Sunburst) => {
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
                let mut chooser = rfd::FileDialog::new()
                    .set_title("New from template")
                    .add_filter("Sheaf graph template", &["sheaftemplate"]);
                if let Some(dir) = self.startup.templates_dir.as_deref().filter(|d| d.is_dir()) {
                    chooser = chooser.set_directory(dir);
                }
                let Some(path) = chooser.pick_file() else { return };
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
    fn show_function_graph(&mut self, ctx: &egui::Context) {
        use crate::dialogs::function_graph::FunctionOutcome;
        let Some(d) = &mut self.function_graph else { return };
        let FunctionOutcome::Add { graph, layer, expr, from, to, points } = (match d.show(ctx) {
            FunctionOutcome::Open => return,
            FunctionOutcome::Closed => {
                self.function_graph = None;
                return;
            }
            add => add,
        }) else {
            return;
        };
        match self.doc.edit("Add function graph", |p| p.add_function_graph(graph, layer, &expr, (from, to), points)) {
            Ok(_) => {
                self.function_graph = None;
                self.say(format!("Added y = {expr}"));
            }
            Err(e) => self.warn(e.to_string()),
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

    /// Find in Project stays open when a hit activates its folder, window, sheet or column.
    fn show_find_project(&mut self, ctx: &egui::Context) {
        use crate::dialogs::find_project::FindProjectOutcome;
        use crate::project_find::FindTarget;
        let Some(d) = &mut self.find_project else { return };
        let outcome = d.show(ctx, self.doc.project(), &self.panels.explorer.folder);
        self.find_spec = d.spec.clone();
        match outcome {
            FindProjectOutcome::Open => {}
            FindProjectOutcome::Closed => self.find_project = None,
            FindProjectOutcome::Go(target) => {
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
        }
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
        self.user_templates =
            self.startup.templates_dir.as_deref().map(sheaf_io::user_templates::names).unwrap_or_default();
    }

    /// The Save Template As prompt and the Manage window.
    fn show_template_windows(&mut self, ctx: &egui::Context) {
        use crate::dialogs::user_templates::{ManagerOutcome, PromptOutcome};
        let dir = self.startup.templates_dir.clone();
        if let Some(prompt) = &mut self.template_prompt {
            match prompt.show(ctx) {
                PromptOutcome::Open => {}
                PromptOutcome::Closed => self.template_prompt = None,
                PromptOutcome::Save(name) => {
                    let graph = prompt.graph;
                    let made = self
                        .doc
                        .project()
                        .graph(graph)
                        .ok_or_else(|| "the graph was closed".to_string())
                        .and_then(|g| sheaf_core::UserTemplate::from_graph(&name, g).map_err(|e| e.to_string()))
                        .and_then(|t| {
                            let dir = dir.as_deref().ok_or_else(|| "there is no template folder".to_string())?;
                            sheaf_io::user_templates::save(dir, &t).map_err(|e| e.to_string())
                        });
                    match made {
                        Ok(saved) => {
                            self.template_prompt = None;
                            self.refresh_user_templates();
                            let verb = if saved.replaced { "Replaced" } else { "Saved" };
                            self.say(format!("{verb} the template {} (Plot > User Templates)", saved.name));
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
                ManagerOutcome::Rename { from, to } => match sheaf_io::user_templates::rename(&dir, &from, &to) {
                    Ok(new) => {
                        let names = sheaf_io::user_templates::names(&dir);
                        m.set_names(names.clone());
                        m.select(&new);
                        self.user_templates = names;
                    }
                    Err(e) => m.set_error(e.to_string()),
                },
                ManagerOutcome::Delete(name) => match sheaf_io::user_templates::delete(&dir, &name) {
                    Ok(_) => {
                        let names = sheaf_io::user_templates::names(&dir);
                        m.set_names(names.clone());
                        self.user_templates = names;
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
        let host_only = matches!(kind, AnalysisKind::PropTestOne | AnalysisKind::PropTestTwo);
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
            } else {
                project.default_plot_columns(book, sheet, Template::Line)
            }
        } else {
            // These tools read categorical columns (factors, subjects), which may hold text.
            let any_type = matches!(
                kind,
                AnalysisKind::Crosstab | AnalysisKind::Anova3 | AnalysisKind::RmAnova1 | AnalysisKind::RmAnova2
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

    /// File > Export > Worksheet as Image (T97): the active sheet as PNG (300 dpi), PDF or SVG.
    fn export_worksheet_image(&mut self, book: WindowId) {
        let Some(b) = self.doc.project().workbook(book) else { return };
        let sheet = b.sheet();
        let stem = if b.long_name.is_empty() { b.name.clone() } else { b.long_name.clone() };
        let mut chooser = rfd::FileDialog::new()
            .set_title("Export worksheet as image")
            .add_filter("PNG image", &["png"])
            .add_filter("PDF", &["pdf"])
            .add_filter("SVG", &["svg"])
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
            self.export_sheet = Some(SheetExportDialog::new(book, self.folder()));
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
            _ => WindowKind::None,
        };
        let selected: &[ColumnId] = book.and_then(|b| self.books.get(&b)).map(|s| s.selected.as_slice()).unwrap_or(&[]);
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
            templates: self.startup.templates_dir.is_some(),
            panels: self.panels.shown_flags(),
            ai_link: self.live.active(),
            layer: graph.and_then(|g| self.graphs.get(&g)).map_or(0, |s| s.active_layer),
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
            view_mode: graph.map(|g| project.view_mode(g)).unwrap_or_default(),
            layer_icons: graph.and_then(|g| project.graph(g)).is_none_or(|g| g.page.style.layer_icons),
            active_layer_only: graph.and_then(|id| {
                let g = project.graph(id)?;
                let layer = self.graphs.get(&id).map_or(0, |s| s.active_layer);
                if g.layers.len() < 2 || layer >= g.layers.len() {
                    return None;
                }
                Some(g.layers.iter().enumerate().all(|(i, l)| l.display.hidden == (i != layer)))
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
            || self.fit_page.is_some()
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
            } else {
                chooser.add_filter("Data files", &["txt", "csv", "tsv", "dat", "asc", "prn", "bin"])
            };
            chooser = chooser.add_filter("All files", &["*"]);
            if let Some(dir) = self.folder() {
                chooser = chooser.set_directory(dir);
            }
            match pick {
                ImportPick::Single => chooser.pick_file().map(|f| vec![f]),
                ImportPick::Multiple | ImportPick::Csv | ImportPick::Excel => chooser.pick_files(),
            }
        });
        if let Some(files) = files {
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
            match d.show(ctx, &mut self.doc) {
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
        if let Some(d) = &mut self.set_values
            && !d.show(ctx, &mut self.doc)
        {
            self.set_values = None;
        }
        if let Some(d) = &mut self.analysis {
            // OK closes the window; Apply leaves it open for the next run.
            let (report, close) = match d.show(ctx, &mut self.doc) {
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
        self.show_fill_pattern(ctx);
        self.show_sampling(ctx);
        self.show_copy_columns(ctx);
        self.show_sheet_navigate(ctx);
        self.show_find_project(ctx);
        self.show_batch_plot(ctx);
        self.show_qc_chart(ctx);
        self.show_function_graph(ctx);
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
                    if let Some(e) = &d.error {
                        ui.colored_label(ui.visuals().error_fg_color, e);
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        let (id, name) = (d.window, d.long_name.clone());
                        let short = d.short_name.clone();
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
                                }
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
                "merge-graphs" if self.doc.project().graphs().count() >= 2 => {
                    self.merge_graphs = Some(MergeGraphsDialog::new(self.doc.project(), &self.panels.explorer.folder));
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
fn delete_columns(project: &mut Project, columns: &[ColumnId]) {
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
                    let graphs = &mut self.graphs;
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
                        Some(Window::Workbook(_)) => {
                            grid::show(ui, doc, id, books.entry(id).or_default(), actions);
                        }
                        Some(Window::Graph(_)) => {
                            graph_view::show(ui, doc, id, graphs.entry(id).or_default(), actions);
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
/// graph theme to the whole graph.
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
        FormatClip::LegendStyles(_) => {}
    }
}

/// The importer a book's source file is read again with (Re-Import, T98); `None` for a book not
/// imported from a data file (made in Sheaf, or opened from a project).
pub(crate) fn reimport_kind(book: &sheaf_core::Workbook) -> Option<sheaf_io::ImportKind> {
    match book.source.as_ref()?.importer.as_str() {
        "ascii" => Some(sheaf_io::ImportKind::Ascii),
        "chi" => Some(sheaf_io::ImportKind::Chi),
        "excel" => Some(sheaf_io::ImportKind::Auto),
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
    let bytes = match ext.as_deref() {
        Some("pdf") => sheaf_plot::sheet_image_pdf(sheet),
        Some("svg") => sheaf_plot::sheet_image_svg(sheet).map(String::into_bytes),
        Some("png") => sheaf_plot::sheet_image_png(sheet, 300.0),
        None => {
            path.set_extension("png");
            sheaf_plot::sheet_image_png(sheet, 300.0)
        }
        Some(other) => return Err(format!("Sheaf writes worksheet pictures as PNG, PDF or SVG, not .{other}")),
    }
    .map_err(|e| e.to_string())?;
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
