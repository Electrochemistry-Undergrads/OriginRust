//! The application window: menus, project explorer, the workspace with its
//! workbook and graph child windows, dialogs, and the one place where user actions change the
//! project.

use crate::actions::{Action, CellOp, CopyKind, GraphOp, GraphTool, LabelRow, SheetOp};
use crate::dialogs::analysis::{AnalysisDialog, AnalysisKind, AnalysisOutcome, FilterDialog, FilterOutcome};
use crate::dialogs::column_props::ColumnPropsDialog;
use crate::dialogs::export::{ExportDialog, ExportOutcome};
use crate::dialogs::fit::{FitDialog, FitDialogOutcome, FitKind};
use crate::dialogs::graph_props::{GraphPropsDialog, PropsTab};
use crate::dialogs::import::{ImportDialog, ImportOutcome};
use crate::dialogs::import_wizard::{ImportWizard, WizardPage};
use crate::dialogs::set_values::SetValuesDialog;
use crate::dialogs::text_object::TextDialog;
use crate::graph_view::{self, GraphUi};
use crate::grid::{self, BookUi, Sel};
use crate::menus::{self, MenuContext, WindowKind};
use crate::panels::logs::Logs;
use crate::panels::{self, PanelData, PanelId, Panels, explorer, objects};
use crate::theme;
use crate::toolbar::{self, Bar, Toolbars};
use crate::workspace::{WindowInfo, Workspace};
use eframe::egui::{self, RichText};
use sheaf_core::clipboard::CopyOptions;
use sheaf_core::graph::TickLabelSource;
use sheaf_core::project::{Folder, Window};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation, Document, Project, WindowId};
use sheaf_io::batch::{ImportMode, Target, place_workbook};
use sheaf_io::project_file;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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
}

/// Something that needs an answer about unsaved changes first.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Pending {
    New,
    Open,
    /// Open the project in `pending_path` (dropped onto the window or given at start-up).
    OpenFile,
    Quit,
}

/// More new windows than this at once are not all opened.
const MAX_TABS_AT_ONCE: usize = 8;

struct RenameDialog {
    window: WindowId,
    long_name: String,
}

/// The window that asks for a new name of a sheet.
struct SheetNameDialog {
    book: WindowId,
    sheet: usize,
    name: String,
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
    import_wizard: Option<ImportWizard>,
    graph_props: Option<GraphPropsDialog>,
    column_props: Option<ColumnPropsDialog>,
    text_dialog: Option<TextDialog>,
    set_values: Option<SetValuesDialog>,
    analysis: Option<AnalysisDialog>,
    fit: Option<FitDialog>,
    filter: Option<FilterDialog>,
    export: Option<ExportDialog>,
    rename: Option<RenameDialog>,
    sheet_name: Option<SheetNameDialog>,
    /// The text of the last Copy or Cut (the system clipboard has it too).
    copied: String,
    /// The pixel size of the last page put on the clipboard by Copy Page.
    copied_page: Option<[usize; 2]>,
    about: bool,
    /// Which toolbars are shown.
    toolbars: Toolbars,
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
    /// Folder of the last imported Origin project, used as the start folder of file choosers.
    import_folder: Option<PathBuf>,
    quit_confirmed: bool,
    /// A top-level menu that is held open (`--show menu:NAME`).
    open_menu: Option<String>,
    shown_title: String,
    startup: Startup,
    frame: u64,
}

impl SheafApp {
    pub fn new(cc: &eframe::CreationContext<'_>, startup: Startup) -> Self {
        theme::apply(&cc.egui_ctx);
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
            import_wizard: None,
            graph_props: None,
            column_props: None,
            text_dialog: None,
            set_values: None,
            analysis: None,
            fit: None,
            filter: None,
            export: None,
            rename: None,
            sheet_name: None,
            copied: String::new(),
            copied_page: None,
            about: false,
            toolbars: Toolbars::default(),
            panels: Panels::default(),
            logs: Logs::default(),
            logged_message: None,
            message: None,
            status: "Ready".to_string(),
            next_token: 1,
            pending: None,
            pending_path: None,
            import_folder: None,
            quit_confirmed: false,
            open_menu: None,
            shown_title: String::new(),
            startup,
            frame: 0,
        };
        let paths = std::mem::take(&mut app.startup.paths);
        app.open_paths(paths, false);
        app.sync_windows();
        app.run_startup_show();
        app
    }

    // ----- files ---------------------------------------------------------

    /// Opens a project file, or imports data files. With `ask`, data files go
    /// through the import dialog; without, they are imported with default settings.
    fn open_paths(&mut self, paths: Vec<PathBuf>, ask: bool) {
        if paths.is_empty() {
            return;
        }
        if let Some(project) = paths.iter().find(|p| sheaf_io::is_project_file(p)) {
            if self.doc.dirty {
                self.pending_path = Some(project.clone());
                self.pending = Some(Pending::OpenFile);
            } else {
                self.load_project(project.clone());
            }
            return;
        }
        if ask {
            self.import = Some(ImportDialog::new(paths));
            return;
        }
        let (chi, ascii) = (sheaf_io::chi::ChiOptions::default(), sheaf_io::ascii::AsciiOptions::default());
        let mut items = Vec::new();
        let mut errors = Vec::new();
        for p in &paths {
            match sheaf_io::import_file(p, sheaf_io::ImportKind::Auto, &chi, &ascii) {
                Ok(i) => items.push(i),
                Err(e) => errors.push(e.to_string()),
            }
        }
        self.finish_import(items, errors, true, None, ImportMode::NewBook);
    }

    /// Opens a `.sheaf` project or imports an Origin `.opj` project, replacing the current one.
    fn load_project(&mut self, path: PathBuf) {
        match sheaf_io::open_project(&path) {
            Ok(opened) => {
                self.doc.replace(opened.project);
                self.reset_views();
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
            }
            Err(e) => self.message = Some(("Could not open the project".to_string(), e.to_string())),
        }
    }

    fn reset_views(&mut self) {
        self.workspace = Workspace::default();
        self.known.clear();
        self.books.clear();
        self.graphs.clear();
        self.graph_props = None;
        self.column_props = None;
        self.set_values = None;
        self.export = None;
        self.rename = None;
        self.sheet_name = None;
        self.import = None;
        self.import_wizard = None;
    }

    /// Saves to the current file, asking for a name when there is none or
    /// `ask` is set. Returns `true` when the project was saved.
    fn save(&mut self, ask: bool) -> bool {
        let path = match (&self.path, ask) {
            (Some(p), false) => p.clone(),
            _ => {
                let suggested = self
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Untitled.opj".to_string());
                // The default format is .opj; a project that came from a .sheaf file offers
                // .sheaf first because it keeps saving there.
                let keeps_sheaf =
                    self.path.as_ref().and_then(|p| p.extension()).is_some_and(|e| e.eq_ignore_ascii_case("sheaf"));
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
                self.doc.dirty = false;
                let mut said = outcome.status();
                if let Some(b) = &backup {
                    said.push_str(&format!("; the original Origin file is kept as {}", b.display()));
                }
                self.say(said);
                self.path = Some(path);
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

    /// The folder file choosers start in: next to the project, else next to the last imported file.
    fn folder(&self) -> Option<PathBuf> {
        if let Some(p) = &self.path {
            return p.parent().map(Path::to_path_buf);
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

    fn finish_import(
        &mut self,
        items: Vec<sheaf_io::Imported>,
        errors: Vec<String>,
        plot: bool,
        template: Option<Template>,
        mode: ImportMode,
    ) {
        let count = items.len();
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
                    let placed = place_workbook(p, imported.workbook, this_mode, target);
                    placement_notes.extend(placed.warnings);
                    let book = placed.book;
                    if plot {
                        let sheet = placed.sheet;
                        let ys = p.default_plot_columns(book, sheet, t);
                        if !ys.is_empty() {
                            let _ = p.plot_columns(t, &ys);
                        }
                    }
                }
                placement_notes
            });
            notes.extend(placement_notes);
            self.say(format!("Imported {count} file(s)"));
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

    /// Copy Page (Ctrl+J): the graph as a picture on the clipboard, at 300 dpi, to paste into a
    /// document or a presentation (Origin also offers a vector copy; Sheaf's vector forms are the
    /// SVG and PDF exports).
    fn copy_page(&mut self, graph: WindowId, ctx: &egui::Context) {
        const DPI: f64 = 300.0;
        match sheaf_plot::render_raster(self.doc.project(), graph, DPI / 72.0) {
            Ok(r) => {
                let size = [r.width as usize, r.height as usize];
                ctx.copy_image(egui::ColorImage::from_rgba_unmultiplied(size, &r.rgba));
                self.copied_page = Some(size);
                self.say(format!("Copied the page as a picture of {} x {} pixels (300 dpi)", size[0], size[1]));
            }
            Err(e) => self.warn(format!("Could not copy the page: {e}")),
        }
    }

    /// Gadgets > Integrate: the gadget goes on the selected plot of the active layer, or its first
    /// visible plot with numbers, over the middle half of the X axis.
    fn start_integrate(&mut self, graph: WindowId) {
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
            self.warn("Nothing to integrate: the active layer has no plot of numbers".to_string());
            return;
        };
        let Some(geom) = sheaf_plot::layout_geometry(project, graph).ok().and_then(|v| v.into_iter().nth(layer)) else {
            return;
        };
        let (x0, x1) = (geom.x.from_unit(0.25), geom.x.from_unit(0.75));
        state.gadget = Some(crate::gadget::Integrator { layer, plot, x0, x1, end_point_baseline: false });
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

    /// Applies a change made from a graph window (right-click menus, Delete, Ctrl+R) as one undo
    /// step.
    fn edit_graph(&mut self, graph: WindowId, op: GraphOp) {
        if let GraphOp::HidePlot { layer, plot } = op {
            self.doc.apply("Hide data plot", |p| objects::set_plot_visible(p, graph, layer, plot, false));
            return;
        }
        let label = match op {
            GraphOp::SetPlotKind { .. } => "Change plot type",
            GraphOp::HideAxis { .. } => "Hide axis",
            GraphOp::ToggleGrids { .. } => "Show or hide grids",
            GraphOp::Rescale { .. } => "Rescale to show all",
            GraphOp::AxisZoom { zoom_in: true, .. } => "Axis zoom in",
            GraphOp::AxisZoom { .. } => "Axis zoom out",
            GraphOp::HideLegend { .. } => "Delete legend",
            GraphOp::ReconstructLegend { .. } => "Reconstruct legend",
            GraphOp::HideTitle { .. } => "Delete axis title",
            GraphOp::DeleteLabel { .. } => "Delete text",
            GraphOp::HideLayer { .. } => "Hide layer",
            GraphOp::HideOtherLayers { .. } => "Hide other layers",
            GraphOp::MovePlotToAxis { right: true, .. } => "Move plot to right Y axis",
            GraphOp::MovePlotToAxis { .. } => "Move plot to left Y axis",
            GraphOp::HidePlot { .. } => unreachable!("handled above"),
        };
        // The present axis ranges, for the zoom (an automatic axis has no stored limits).
        let resolved = sheaf_plot::layout_geometry(self.doc.project(), graph).ok();
        self.doc.apply(label, |p| graph_view::apply_op(p, graph, op, resolved.as_deref()));
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

    fn token(&mut self) -> u64 {
        self.next_token += 1;
        self.next_token
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
            Action::OpenProject => {
                if self.doc.dirty {
                    self.pending = Some(Pending::Open);
                } else {
                    self.do_pending(Pending::Open, ctx);
                }
            }
            Action::SaveProject => {
                self.save(false);
            }
            Action::SaveProjectAs => {
                self.save(true);
            }
            Action::Import(path) => {
                let files = match path {
                    Some(p) => Some(vec![p]),
                    None => {
                        let mut chooser = rfd::FileDialog::new()
                            .set_title("Import data files")
                            .add_filter("Data files", &["txt", "csv", "tsv", "dat", "asc", "prn", "bin"])
                            .add_filter("All files", &["*"]);
                        if let Some(dir) = self.folder() {
                            chooser = chooser.set_directory(dir);
                        }
                        chooser.pick_files()
                    }
                };
                if let Some(files) = files {
                    self.import = Some(ImportDialog::new(files));
                }
            }
            Action::ImportWizard => {
                // An open wizard keeps its settings: the command only brings it to the front.
                if self.import_wizard.is_some() {
                    ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, egui::Id::new("import_wizard")));
                } else {
                    self.import_wizard = Some(ImportWizard::new(Vec::new()));
                }
            }
            Action::Undo => {
                if let Some(label) = self.doc.undo() {
                    self.say(format!("Undone: {label}"));
                }
            }
            Action::Redo => {
                if let Some(label) = self.doc.redo() {
                    self.say(format!("Redone: {label}"));
                }
            }
            Action::NewWorkbook => {
                self.doc.apply("New workbook", |p| {
                    let book = p.add_workbook("");
                    let _ = p.add_column(book, 0, Designation::X, ColumnData::numeric(Vec::new()));
                    let _ = p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new()));
                });
            }
            Action::AddColumn(book) => {
                self.doc.apply("Add column", |p| {
                    let sheet = p.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
                    let _ = p.add_column(book, sheet, Designation::Y, ColumnData::numeric(Vec::new()));
                });
            }
            Action::DeleteColumns(columns) => {
                self.doc.apply("Delete columns", |p| delete_columns(p, &columns));
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
                    .filter(|c| self.doc.project().column(*c).is_some_and(|c| c.is_locked()))
                    .collect();
                let result = self.doc.edit("Recalculate", |p| {
                    for c in &columns {
                        p.recalculate(*c)?;
                    }
                    Ok::<(), sheaf_core::formula::FormulaError>(())
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
                            LabelRow::Formula => {}
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
            Action::PlotColumns { book, columns, template } => self.plot_columns(book, columns, template),
            Action::Analyze { book, columns, kind } => self.open_analysis(book, columns, kind),
            Action::Fit { book, column, kind } => {
                let column = column.filter(|c| self.doc.project().column(*c).is_some_and(|c| c.data.is_numeric()));
                self.fit = FitDialog::new(self.doc.project(), book, column, kind);
                if self.fit.is_none() {
                    self.warn("Nothing to fit: choose a column that holds numbers and is set as Y".to_string());
                }
            }
            Action::SortSheet { column, descending } => self.sort_sheet(column, descending),
            Action::FilterRows(book) => {
                self.filter = FilterDialog::new(self.doc.project(), book);
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
                let state = self.graphs.entry(graph).or_default();
                state.reader = tool == GraphTool::Reader;
                state.text_tool = tool == GraphTool::Text;
                state.reader_point = None;
                match tool {
                    GraphTool::Reader => {
                        self.say("Data Reader: click a data point; Left and Right step through the rows; Esc ends")
                    }
                    GraphTool::Text => self.say("Text: click where the text goes; Esc ends"),
                    GraphTool::Pointer => {}
                }
            }
            Action::AddText { graph, layer, at } => {
                let state = self.graphs.entry(graph).or_default();
                state.text_tool = false;
                let layer = layer.unwrap_or(state.active_layer);
                self.text_dialog = TextDialog::new_label(&self.doc, graph, layer, at.unwrap_or((0.5, 0.5)));
            }
            Action::EditText { graph, layer, label } => {
                self.text_dialog = TextDialog::edit(&self.doc, graph, layer, label);
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
            Action::StartIntegrate { graph } => self.start_integrate(graph),
            Action::Gadget { graph, op } => self.gadget(graph, op),
            Action::ExportGraph(graph) => {
                self.export = Some(ExportDialog::new(graph, self.folder()));
            }
            Action::ExportData(book) => self.export_data(book),
            Action::Focus(id) => self.focus(id),
            Action::HideWindow(id) => self.workspace.hide(id),
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
                    self.rename = Some(RenameDialog { window: id, long_name });
                }
            }
            Action::Cascade => self.workspace.cascade(),
            Action::TileHorizontally => self.workspace.tile_horizontally(),
            Action::TileVertically => self.workspace.tile_vertically(),
            Action::About => self.about = true,
            Action::Toolbars => self.toolbars.dialog = true,
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
                self.reset_views();
                self.say("New project".to_string());
            }
            Pending::Open => {
                let mut chooser = rfd::FileDialog::new()
                    .set_title("Open project")
                    .add_filter("Projects (Sheaf, Origin)", &[project_file::EXTENSION, "opj"])
                    .add_filter("Sheaf project", &[project_file::EXTENSION])
                    .add_filter("Origin project", &["opj"]);
                if let Some(dir) = self.folder() {
                    chooser = chooser.set_directory(dir);
                }
                if let Some(path) = chooser.pick_file() {
                    self.load_project(path);
                }
            }
            Pending::OpenFile => {
                if let Some(path) = self.pending_path.take() {
                    self.load_project(path);
                }
            }
            Pending::Quit => {
                self.quit_confirmed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// Copy, cut, paste and clear on the selected cells of a workbook. Cut, paste and clear
    /// are one undo step each.
    fn cell_op(&mut self, book: WindowId, op: CellOp, ctx: &egui::Context) {
        let Some(b) = self.doc.project().workbook(book) else {
            return;
        };
        let sheet_index = b.active_sheet.min(b.sheets.len().saturating_sub(1));
        let Some(sheet) = b.sheets.get(sheet_index) else {
            return;
        };
        let target = self.books.get(&book).and_then(|s| s.target(sheet));
        let Some(t) = target else {
            if op == CellOp::Paste || matches!(op, CellOp::PasteText(_)) {
                self.warn("Click a cell first: the pasted text starts there".to_string());
            }
            return;
        };
        match op {
            CellOp::Copy(_) | CellOp::Cut => {
                let kind = if let CellOp::Copy(k) = op { k } else { CopyKind::Plain };
                let options = CopyOptions {
                    // Copy never drops digits: a copy and paste inside Sheaf must give back the same numbers.
                    full_precision: true,
                    label_rows: kind == CopyKind::WithLabels,
                };
                let text = sheet.copy_text(&t.columns, t.rows.clone(), options);
                ctx.copy_text(text.clone());
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
            CellOp::Paste => {
                // The system answers with the text in a later frame (an `Event::Paste`).
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
            CellOp::PasteText(text) => {
                let (col, row) = t.top_left;
                match self.doc.edit("Paste", |p| p.paste_text(book, sheet_index, col, row, &text)) {
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
                    for c in &mut copy.columns {
                        c.id = p.new_column_id();
                    }
                    if let Some(b) = p.workbook_mut(book) {
                        b.sheets.insert(active + 1, copy);
                        b.active_sheet = active + 1;
                    }
                });
                if let Some(state) = self.books.get_mut(&book) {
                    state.clear_selection();
                }
            }
            SheetOp::RenameDialog => {
                let name = b.sheets[active].name.clone();
                self.sheet_name = Some(SheetNameDialog { book, sheet: active, name });
            }
        }
    }

    fn plot_columns(&mut self, book: WindowId, columns: Vec<ColumnId>, template: Template) {
        let project = self.doc.project();
        let ys: Vec<ColumnId> = if columns.is_empty() {
            let sheet = project.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
            project.default_plot_columns(book, sheet, template)
        } else {
            columns
                .into_iter()
                .filter(|id| {
                    project.column(*id).is_some_and(|c| {
                        c.data.is_numeric()
                            && matches!(c.designation, Designation::Y | Designation::Z | Designation::Disregard)
                    })
                })
                .collect()
        };
        if ys.is_empty() {
            self.warn("Nothing to plot: select one or more numeric Y columns".to_string());
            return;
        }
        match self.doc.edit("Plot columns", |p| p.plot_columns(template, &ys)) {
            Ok(_) => self.say(format!("Plotted {} column(s)", ys.len())),
            Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
        }
    }

    /// Opens the window of an analysis tool. With no columns given, the tool gets the columns a
    /// plot of the active sheet would show; columns that hold text are left out.
    fn open_analysis(&mut self, book: WindowId, columns: Vec<ColumnId>, kind: AnalysisKind) {
        let project = self.doc.project();
        let columns: Vec<ColumnId> = if columns.is_empty() {
            let sheet = project.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
            project.default_plot_columns(book, sheet, Template::Line)
        } else {
            columns.into_iter().filter(|id| project.column(*id).is_some_and(|c| c.data.is_numeric())).collect()
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

    fn export_data(&mut self, book: WindowId) {
        let Some(b) = self.doc.project().workbook(book) else {
            return;
        };
        let sheet = b.sheet();
        let mut chooser = rfd::FileDialog::new()
            .set_title("Export worksheet")
            .add_filter("Comma-separated", &["csv"])
            .add_filter("Tab-separated", &["tsv", "txt"])
            .set_file_name(format!("{}.csv", if b.long_name.is_empty() { &b.name } else { &b.long_name }));
        if let Some(dir) = self.folder() {
            chooser = chooser.set_directory(dir);
        }
        let Some(path) = chooser.save_file() else {
            return;
        };
        let tabs = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("tsv") || e.eq_ignore_ascii_case("txt"));
        let opts = sheaf_io::export::AsciiExportOptions {
            delimiter: if tabs { '\t' } else { ',' },
            ..sheaf_io::export::AsciiExportOptions::default()
        };
        let result = std::fs::File::create(&path).map_err(|e| e.to_string()).and_then(|file| {
            let mut out = std::io::BufWriter::new(file);
            sheaf_io::export::write_delimited(sheet, &mut out, &opts).map_err(|e| e.to_string())
        });
        match result {
            Ok(()) => self.say(format!("Wrote {}", path.display())),
            Err(e) => self.message = Some(("Could not export the worksheet".to_string(), e)),
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
            cells: book.and_then(|b| self.books.get(&b)).is_some_and(|s| s.sel != Sel::None),
            undo: self.doc.undo_label(),
            redo: self.doc.redo_label(),
            windows: project.windows.iter().map(|w| (w.id(), w.title())).collect(),
            open: self.open_menu.as_deref(),
            panels: self.panels.shown_flags(),
            layer: graph.and_then(|g| self.graphs.get(&g)).map_or(0, |s| s.active_layer),
            tool: graph.and_then(|g| self.graphs.get(&g)).map_or(GraphTool::Pointer, |s| s.tool()),
            curve: graph.and_then(|g| self.graph_curve(g)),
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

    /// Keyboard shortcuts: the keys written in the menu table (Origin's keys for the commands
    /// Sheaf has), and nothing else.
    fn shortcuts(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let typing = ctx.wants_keyboard_input();
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
        let menu_ctx = self.menu_context();
        // Ctrl+C, Ctrl+X and Ctrl+V reach the program as events of their own, not as keys. They
        // belong to the cells unless a text field has the keyboard.
        if !typing && let Some(book) = menu_ctx.book {
            let events = ctx.input(|i| i.events.clone());
            for event in events {
                let op = match event {
                    egui::Event::Copy => CellOp::Copy(CopyKind::Plain),
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
                if i.consume_shortcut(&keys)
                    && let Ok(action) = menus::resolve(&cmd, &menu_ctx)
                {
                    actions.push(action);
                }
            }
        });
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let menu_ctx = self.menu_context();
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
                format!("[{}]{sheet}!", b.name)
            }
            Some(Window::Graph(g)) => format!("[{}]1!", g.name),
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
                ImportOutcome::Done { items, errors, plot, template, mode } => {
                    self.import = None;
                    self.finish_import(items, errors, plot, template, mode);
                }
            }
        }
        if let Some(d) = &mut self.import_wizard {
            match d.show(ctx, import_target.as_ref().map(|t| t.1.as_str())) {
                ImportOutcome::Open => {}
                ImportOutcome::Cancelled => self.import_wizard = None,
                ImportOutcome::Done { items, errors, plot, template, mode } => {
                    self.import_wizard = None;
                    self.finish_import(items, errors, plot, template, mode);
                }
            }
        }
        if let Some(d) = &mut self.graph_props
            && !d.show(ctx, &mut self.doc)
        {
            self.graph_props = None;
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
            }
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
            match d.show(ctx, &self.doc) {
                ExportOutcome::Open => {}
                ExportOutcome::Closed => self.export = None,
                ExportOutcome::Saved(path) => {
                    self.say(format!("Wrote {}", path.display()));
                    self.export = None;
                }
                ExportOutcome::Failed(e) => self.message = Some(("Could not export the graph".to_string(), e)),
            }
        }
        if let Some(d) = &mut self.rename {
            let mut open = true;
            let mut done = false;
            egui::Window::new("Rename window").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Title");
                    ui.add(egui::TextEdit::singleline(&mut d.long_name).desired_width(260.0));
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        let (id, name) = (d.window, d.long_name.clone());
                        self.doc.apply("Rename window", move |p| {
                            if let Some(b) = p.workbook_mut(id) {
                                b.long_name = name;
                            } else if let Some(g) = p.graph_mut(id) {
                                g.long_name = name;
                            }
                        });
                        done = true;
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
            egui::Window::new("Sheet name").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(200.0));
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        let (book, sheet, name) = (d.book, d.sheet, d.name.clone());
                        let name = name.trim().to_string();
                        let taken = self.doc.project().workbook(book).is_none_or(|b| {
                            b.sheets.iter().enumerate().any(|(i, s)| i != sheet && s.name.eq_ignore_ascii_case(&name))
                        });
                        if name.is_empty() || taken {
                            // Not `self.warn`: the window's own state is borrowed here.
                            let text = "A sheet needs a name that no other sheet of the workbook has";
                            self.logs.warn(text);
                            self.status = text.to_string();
                        } else {
                            self.doc.apply("Rename sheet", |p| {
                                if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
                                    s.name = name;
                                }
                            });
                        }
                        done = true;
                    }
                    if ui.button("Cancel").clicked() {
                        done = true;
                    }
                });
            });
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
                Some(_) => self.pending = None,
                None => {}
            }
        }
    }

    // ----- checking the interface without a person ------------------------

    /// Opens the things named by `--show` on the command line.
    fn run_startup_show(&mut self) {
        let shows = self.startup.show.clone();
        for what in shows {
            let (name, arg) = what.split_once(':').unwrap_or((what.as_str(), ""));
            let project = self.doc.project();
            let first_graph = project.graphs().next().map(|g| g.id);
            let first_column = project.workbooks().next().and_then(|b| b.sheet().columns.first()).map(|c| c.id);
            let named = project.window_by_name(arg).map(Window::id);
            match name {
                "menu" => self.open_menu = Some(arg.to_string()),
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
                // text:N opens the Text window of the N-th label (from 1) of the first graph's
                // first layer and selects it.
                "text" => {
                    if let (Some(id), Ok(n)) = (first_graph, arg.parse::<usize>()) {
                        let label = n.saturating_sub(1);
                        self.text_dialog = TextDialog::edit(&self.doc, id, 0, label);
                        self.graphs.entry(id).or_default().selected =
                            Some((0, sheaf_plot::scene::ObjectKind::Label(label)));
                        self.focus(id);
                    }
                }
                "integrate" => {
                    if let Some(id) = first_graph {
                        self.start_integrate(id);
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
                "filter" => {
                    let book = project.workbooks().next().map(|b| b.id);
                    if let Some(book) = book {
                        self.filter = FilterDialog::new(self.doc.project(), book);
                        self.focus(book);
                    }
                }
                "export" => {
                    if let Some(id) = first_graph {
                        self.export = Some(ExportDialog::new(id, None));
                    }
                }
                "import-wizard" => {
                    let files = if arg.is_empty() { Vec::new() } else { vec![PathBuf::from(arg)] };
                    self.import_wizard = Some(ImportWizard::new(files));
                }
                // import-wizard-page:header, names, file, data or save (after import-wizard).
                "import-wizard-page" => {
                    let page = match arg {
                        "header" => WizardPage::HeaderLines,
                        "names" => WizardPage::ColumnNames,
                        "file" => WizardPage::FileNameOptions,
                        "data" => WizardPage::DataColumns,
                        "save" => WizardPage::SaveFilters,
                        _ => WizardPage::Source,
                    };
                    if let Some(w) = &mut self.import_wizard {
                        w.show_page(page);
                    }
                }
                "import" => {
                    if !arg.is_empty() {
                        self.import = Some(ImportDialog::new(vec![PathBuf::from(arg)]));
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
    let r = &outcome.report;
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
    for (name, value) in &outcome.derived {
        lines.push(format!("  {name} = {}", n(*value)));
    }
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
        self.shortcuts(ctx, &mut actions);

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| self.menu_bar(ui, &mut actions));
        {
            let c = self.menu_context();
            let tb = &self.toolbars;
            let bar_frame = || egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(3, 1));
            if tb.shown(Bar::Standard) {
                egui::TopBottomPanel::top("toolbar_standard")
                    .frame(bar_frame())
                    .show(ctx, |ui| toolbar::show_row(ui, &[Bar::Standard], tb, &c, &mut actions));
            }
            let second = [Bar::Edit, Bar::Format, Bar::Graph];
            if tb.any(&second) {
                egui::TopBottomPanel::top("toolbar_second")
                    .frame(bar_frame())
                    .show(ctx, |ui| toolbar::show_row(ui, &second, tb, &c, &mut actions));
            }
            egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| self.status_bar(ui));
            if tb.shown(Bar::Graphs2D) {
                egui::TopBottomPanel::bottom("toolbar_2d_graphs")
                    .frame(bar_frame())
                    .show(ctx, |ui| toolbar::show_row(ui, &[Bar::Graphs2D], tb, &c, &mut actions));
            }
        }
        {
            let data = panel_data(&self.doc, &self.logs, self.active, &self.path, &self.graph_props);
            self.panels.strips_and_docks(ctx, &data, &mut actions);
        }
        {
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
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::WORKSPACE_GREY)).show(ctx, |ui| {
            if self.doc.project().windows.is_empty() {
                self.welcome(ui, &mut actions);
            } else {
                let doc = &self.doc;
                let books = &mut self.books;
                let graphs = &mut self.graphs;
                let actions = &mut actions;
                let info = |id: WindowId| {
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

        {
            let data = panel_data(&self.doc, &self.logs, self.active, &self.path, &self.graph_props);
            self.panels.flyouts(ctx, &data, &mut actions);
        }
        self.dialogs(ctx);
        for action in actions {
            self.apply(action, ctx);
        }
        self.sync_windows();
        match &self.message {
            Some(m) if self.logged_message.as_ref() != Some(m) => {
                self.logs.warn(format!("{}: {}", m.0, m.1.replace('\n', " ")));
                self.logged_message = Some(m.clone());
            }
            None => self.logged_message = None,
            Some(_) => {}
        }

        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string());
        let title = format!("{name}{} - {APP_NAME}", if self.doc.dirty { "*" } else { "" });
        if title != self.shown_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.shown_title = title;
        }
        self.screenshot_step(ctx);
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

#[cfg(test)]
mod ui_tests;

/// True when both paths name the same file (or, if one is missing, the same path text).
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}
