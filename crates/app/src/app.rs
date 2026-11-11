//! The application window: menus, project explorer, docked workbook and
//! graph tabs, dialogs, and the one place where user actions change the
//! project.

use crate::actions::Action;
use crate::dialogs::column_props::ColumnPropsDialog;
use crate::dialogs::export::{ExportDialog, ExportOutcome};
use crate::dialogs::graph_props::{GraphPropsDialog, PropsTab};
use crate::dialogs::import::{ImportDialog, ImportOutcome};
use crate::dialogs::set_values::SetValuesDialog;
use crate::graph_view::{self, GraphUi};
use crate::grid::{self, BookUi};
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, RichText, Sense, Stroke};
use egui_dock::{DockArea, DockState, TabViewer};
use sheaf_core::project::Window;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation, Document, Project, WindowId};
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

/// More new windows than this at once are not all opened as tabs.
const MAX_TABS_AT_ONCE: usize = 8;

struct RenameDialog {
    window: WindowId,
    long_name: String,
}

pub struct SheafApp {
    doc: Document,
    path: Option<PathBuf>,
    dock: DockState<WindowId>,
    /// Windows that already got a tab once. A tab the user closed is not reopened by itself.
    known: HashSet<WindowId>,
    active: Option<WindowId>,
    books: HashMap<WindowId, BookUi>,
    graphs: HashMap<WindowId, GraphUi>,
    import: Option<ImportDialog>,
    graph_props: Option<GraphPropsDialog>,
    column_props: Option<ColumnPropsDialog>,
    set_values: Option<SetValuesDialog>,
    export: Option<ExportDialog>,
    rename: Option<RenameDialog>,
    about: bool,
    /// A message the user has to dismiss (title, text).
    message: Option<(String, String)>,
    status: String,
    next_token: u64,
    pending: Option<Pending>,
    pending_path: Option<PathBuf>,
    /// Folder of the last imported Origin project, used as the start folder of file choosers.
    import_folder: Option<PathBuf>,
    quit_confirmed: bool,
    shown_title: String,
    startup: Startup,
    frame: u64,
}

impl SheafApp {
    pub fn new(cc: &eframe::CreationContext<'_>, startup: Startup) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Light);
        cc.egui_ctx.style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 5.0);
            s.interaction.selectable_labels = false;
        });
        let mut app = SheafApp {
            doc: Document::default(),
            path: None,
            dock: DockState::new(Vec::new()),
            known: HashSet::new(),
            active: None,
            books: HashMap::new(),
            graphs: HashMap::new(),
            import: None,
            graph_props: None,
            column_props: None,
            set_values: None,
            export: None,
            rename: None,
            about: false,
            message: None,
            status: "Ready".to_string(),
            next_token: 1,
            pending: None,
            pending_path: None,
            import_folder: None,
            quit_confirmed: false,
            shown_title: String::new(),
            startup,
            frame: 0,
        };
        let paths = std::mem::take(&mut app.startup.paths);
        app.open_paths(paths, false);
        app.sync_tabs();
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
        self.finish_import(items, errors, true, None);
    }

    /// Opens a `.sheaf` project or imports an Origin `.opj` project, replacing the current one.
    fn load_project(&mut self, path: PathBuf) {
        match sheaf_io::open_project(&path) {
            Ok(opened) => {
                self.doc.replace(opened.project);
                self.reset_views();
                if opened.foreign {
                    // Not saved back to the Origin file: the project has no file name yet.
                    self.status = format!("Imported {}; save it as a Sheaf project to keep changes", path.display());
                    self.import_folder = path.parent().map(Path::to_path_buf);
                    self.path = None;
                } else {
                    self.status = format!("Opened {}", path.display());
                    self.path = Some(path);
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
        self.dock = DockState::new(Vec::new());
        self.known.clear();
        self.books.clear();
        self.graphs.clear();
        self.graph_props = None;
        self.column_props = None;
        self.set_values = None;
        self.export = None;
        self.rename = None;
        self.import = None;
    }

    /// Saves to the current file, asking for a name when there is none or
    /// `ask` is set. Returns `true` when the project was saved.
    fn save(&mut self, ask: bool) -> bool {
        let path = match (&self.path, ask) {
            (Some(p), false) => p.clone(),
            _ => {
                let mut chooser = rfd::FileDialog::new()
                    .set_title("Save project")
                    .add_filter("Sheaf project", &[project_file::EXTENSION])
                    .set_file_name(
                        self.path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| format!("Untitled.{}", project_file::EXTENSION)),
                    );
                if let Some(dir) = self.folder() {
                    chooser = chooser.set_directory(dir);
                }
                let Some(mut p) = chooser.save_file() else {
                    return false;
                };
                // Always a Sheaf project: never write over a file of another program.
                let is_sheaf = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case(project_file::EXTENSION));
                if !is_sheaf {
                    let mut name = p.file_name().map(|n| n.to_os_string()).unwrap_or_default();
                    name.push(format!(".{}", project_file::EXTENSION));
                    p.set_file_name(name);
                }
                p
            }
        };
        match project_file::save(self.doc.project(), &path) {
            Ok(()) => {
                self.doc.dirty = false;
                self.status = format!("Saved {}", path.display());
                self.path = Some(path);
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
    ) {
        let count = items.len();
        let mut notes: Vec<String> = errors;
        for i in &items {
            for w in &i.warnings {
                notes.push(format!("{}: {w}", i.workbook.long_name));
            }
        }
        if count > 0 {
            self.doc.apply("Import data", move |p| {
                for imported in items {
                    let t = template.unwrap_or(imported.suggested_template);
                    let book = p.adopt_workbook(imported.workbook);
                    if plot {
                        let sheet = p.workbook(book).map(|b| b.active_sheet).unwrap_or(0);
                        let ys = p.default_plot_columns(book, sheet, t);
                        if !ys.is_empty() {
                            let _ = p.plot_columns(t, &ys);
                        }
                    }
                }
            });
            self.status = format!("Imported {count} file(s)");
        }
        if !notes.is_empty() {
            self.message = Some(("Import notes".to_string(), notes.join("\n\n")));
        }
    }

    // ----- tabs ----------------------------------------------------------

    /// Gives every new window of the project a tab and drops tabs of windows that are gone.
    fn sync_tabs(&mut self) {
        let ids: Vec<WindowId> = self.doc.project().windows.iter().map(Window::id).collect();
        let set: HashSet<WindowId> = ids.iter().copied().collect();
        self.dock.retain_tabs(|t| set.contains(t));
        self.known.retain(|k| set.contains(k));
        self.books.retain(|k, _| set.contains(k));
        self.graphs.retain(|k, _| set.contains(k));
        // A project with very many windows (an imported Origin project can have dozens) opens
        // only its first few as tabs; the others are opened from the project explorer.
        let new: Vec<WindowId> = ids.into_iter().filter(|id| self.known.insert(*id)).collect();
        let shown = if new.len() > MAX_TABS_AT_ONCE { 1 } else { new.len() };
        for id in new.into_iter().take(shown) {
            self.dock.push_to_focused_leaf(id);
        }
    }

    fn focus(&mut self, id: WindowId) {
        if let Some((surface, node, tab)) = self.dock.find_tab(&id) {
            self.dock.set_active_tab((surface, node, tab));
            self.dock.set_focused_node_and_surface((surface, node));
        } else if self.doc.project().window(id).is_some() {
            self.dock.push_to_focused_leaf(id);
        }
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
                            .add_filter("Data files", &["txt", "csv", "tsv", "dat", "asc", "prn"])
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
            Action::Undo => {
                if let Some(label) = self.doc.undo() {
                    self.status = format!("Undone: {label}");
                }
            }
            Action::Redo => {
                if let Some(label) = self.doc.redo() {
                    self.status = format!("Redone: {label}");
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
                        self.status = format!("'{text}' is not a number; the cell was left empty");
                    }
                }
            }
            Action::SelectSheet { book, sheet } => {
                self.doc.adjust_view(|p| {
                    if let Some(b) = p.workbook_mut(book) {
                        b.active_sheet = sheet;
                    }
                });
            }
            Action::ColumnProperties(column) => {
                self.column_props = ColumnPropsDialog::new(self.doc.project(), column);
            }
            Action::SetColumnValues(column) => {
                self.set_values = SetValuesDialog::new(self.doc.project(), column);
            }
            Action::PlotColumns { book, columns, template } => self.plot_columns(book, columns, template),
            Action::GraphProperties { graph, tab } => {
                let token = self.token();
                self.graph_props = Some(GraphPropsDialog::new(graph, tab, token));
                self.focus(graph);
            }
            Action::ExportGraph(graph) => {
                self.export = Some(ExportDialog::new(graph, self.folder()));
            }
            Action::ExportData(book) => self.export_data(book),
            Action::Focus(id) => self.focus(id),
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
            Action::About => self.about = true,
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
                self.status = "New project".to_string();
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
            self.status = "Nothing to plot: select one or more numeric Y columns".to_string();
            return;
        }
        match self.doc.edit("Plot columns", |p| p.plot_columns(template, &ys)) {
            Ok(_) => self.status = format!("Plotted {} column(s)", ys.len()),
            Err(e) => self.message = Some(("Could not make the graph".to_string(), e.to_string())),
        }
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
            Ok(()) => self.status = format!("Wrote {}", path.display()),
            Err(e) => self.message = Some(("Could not export the worksheet".to_string(), e)),
        }
    }

    // ----- drawing -------------------------------------------------------

    fn shortcuts(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let cmd = |key| KeyboardShortcut::new(Modifiers::COMMAND, key);
        let cmd_shift = |key| KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, key);
        let typing = ctx.wants_keyboard_input();
        ctx.input_mut(|i| {
            if i.consume_shortcut(&cmd_shift(Key::S)) {
                actions.push(Action::SaveProjectAs);
            }
            if i.consume_shortcut(&cmd(Key::S)) {
                actions.push(Action::SaveProject);
            }
            if i.consume_shortcut(&cmd(Key::O)) {
                actions.push(Action::OpenProject);
            }
            if i.consume_shortcut(&cmd(Key::N)) {
                actions.push(Action::NewProject);
            }
            if i.consume_shortcut(&cmd(Key::I)) {
                actions.push(Action::Import(None));
            }
            if !typing {
                if i.consume_shortcut(&cmd_shift(Key::Z)) || i.consume_shortcut(&cmd(Key::Y)) {
                    actions.push(Action::Redo);
                }
                if i.consume_shortcut(&cmd(Key::Z)) {
                    actions.push(Action::Undo);
                }
            }
        });
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let project = self.doc.project();
        let active_book = self.active.filter(|id| project.workbook(*id).is_some());
        let active_graph = self.active.filter(|id| project.graph(*id).is_some());
        let selected: Vec<ColumnId> =
            active_book.and_then(|b| self.books.get(&b)).map(|s| s.selected.clone()).unwrap_or_default();
        let item = |ui: &mut egui::Ui, enabled: bool, text: &str, shortcut: &str| {
            let mut button = egui::Button::new(text);
            if !shortcut.is_empty() {
                button = button.shortcut_text(shortcut);
            }
            let clicked = ui.add_enabled(enabled, button).clicked();
            if clicked {
                ui.close();
            }
            clicked
        };

        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if item(ui, true, "New Project", "Ctrl+N") {
                    actions.push(Action::NewProject);
                }
                if item(ui, true, "Open Project...", "Ctrl+O") {
                    actions.push(Action::OpenProject);
                }
                if item(ui, true, "Save Project", "Ctrl+S") {
                    actions.push(Action::SaveProject);
                }
                if item(ui, true, "Save Project As...", "Ctrl+Shift+S") {
                    actions.push(Action::SaveProjectAs);
                }
                ui.separator();
                if item(ui, true, "Import Data...", "Ctrl+I") {
                    actions.push(Action::Import(None));
                }
                ui.separator();
                if item(ui, active_graph.is_some(), "Export Graph...", "")
                    && let Some(g) = active_graph
                {
                    actions.push(Action::ExportGraph(g));
                }
                if item(ui, active_book.is_some(), "Export Worksheet...", "")
                    && let Some(b) = active_book
                {
                    actions.push(Action::ExportData(b));
                }
                ui.separator();
                if item(ui, true, "Quit", "") {
                    actions.push(Action::Quit);
                }
            });
            ui.menu_button("Edit", |ui| {
                let undo = self.doc.undo_label().map(|l| format!("Undo {l}")).unwrap_or_else(|| "Undo".to_string());
                let redo = self.doc.redo_label().map(|l| format!("Redo {l}")).unwrap_or_else(|| "Redo".to_string());
                if item(ui, self.doc.undo_label().is_some(), &undo, "Ctrl+Z") {
                    actions.push(Action::Undo);
                }
                if item(ui, self.doc.redo_label().is_some(), &redo, "Ctrl+Y") {
                    actions.push(Action::Redo);
                }
            });
            ui.menu_button("Worksheet", |ui| {
                if item(ui, true, "New Workbook", "") {
                    actions.push(Action::NewWorkbook);
                }
                ui.separator();
                match active_book {
                    Some(book) => grid::column_menu(ui, book, &selected, actions),
                    None => {
                        ui.weak("Open a workbook to work with columns");
                    }
                }
            });
            ui.menu_button("Plot", |ui| match active_book {
                Some(book) => {
                    let what = if selected.is_empty() {
                        "all Y columns of the sheet".to_string()
                    } else {
                        format!("{} selected column(s)", selected.len())
                    };
                    ui.weak(format!("Plots {what}"));
                    for t in Template::ALL {
                        if item(ui, true, t.label(), "") {
                            actions.push(Action::PlotColumns { book, columns: selected.clone(), template: t });
                        }
                    }
                }
                None => {
                    ui.weak("Open a workbook to plot its columns");
                }
            });
            ui.menu_button("Graph", |ui| {
                let on = active_graph.is_some();
                for (text, tab) in [
                    ("X Axis...", PropsTab::XAxis),
                    ("Y Axis...", PropsTab::YAxis),
                    ("Plots...", PropsTab::Plots),
                    ("Legend...", PropsTab::Legend),
                    ("Page...", PropsTab::Page),
                ] {
                    if item(ui, on, text, "")
                        && let Some(graph) = active_graph
                    {
                        actions.push(Action::GraphProperties { graph, tab });
                    }
                }
                ui.separator();
                if item(ui, on, "Export...", "")
                    && let Some(g) = active_graph
                {
                    actions.push(Action::ExportGraph(g));
                }
            });
            ui.menu_button("Window", |ui| {
                if project.windows.is_empty() {
                    ui.weak("The project has no windows");
                }
                for w in &project.windows {
                    if item(ui, true, &w.title(), "") {
                        actions.push(Action::Focus(w.id()));
                    }
                }
            });
            ui.menu_button("Help", |ui| {
                if item(ui, true, &format!("About {APP_NAME}"), "") {
                    actions.push(Action::About);
                }
            });
        });
    }

    fn explorer(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.add_space(4.0);
        ui.label(RichText::new("Project").strong());
        ui.separator();
        let project = self.doc.project();
        if project.windows.is_empty() {
            ui.weak("No windows yet");
            return;
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            for w in &project.windows {
                let id = w.id();
                let is_graph = matches!(w, Window::Graph(_));
                let response = ui
                    .horizontal(|ui| {
                        window_icon(ui, is_graph);
                        ui.selectable_label(self.active == Some(id), w.title())
                    })
                    .inner;
                if response.clicked() {
                    actions.push(Action::Focus(id));
                }
                response.context_menu(|ui| {
                    if ui.button("Show").clicked() {
                        actions.push(Action::Focus(id));
                        ui.close();
                    }
                    if ui.button("Rename...").clicked() {
                        actions.push(Action::RenameWindow(id));
                        ui.close();
                    }
                    if ui.button("Export...").clicked() {
                        actions.push(if is_graph { Action::ExportGraph(id) } else { Action::ExportData(id) });
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() {
                        actions.push(Action::DeleteWindow(id));
                        ui.close();
                    }
                });
            }
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

    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(d) = &mut self.import {
            match d.show(ctx) {
                ImportOutcome::Open => {}
                ImportOutcome::Cancelled => self.import = None,
                ImportOutcome::Done { items, errors, plot, template } => {
                    self.import = None;
                    self.finish_import(items, errors, plot, template);
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
        if let Some(d) = &mut self.set_values
            && !d.show(ctx, &mut self.doc)
        {
            self.set_values = None;
        }
        if let Some(d) = &mut self.export {
            match d.show(ctx, &self.doc) {
                ExportOutcome::Open => {}
                ExportOutcome::Closed => self.export = None,
                ExportOutcome::Saved(path) => {
                    self.status = format!("Wrote {}", path.display());
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
                "focus" => {
                    if let Some(id) = named {
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
                        self.graph_props = Some(GraphPropsDialog::new(id, tab, 1_000_000));
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
                "export" => {
                    if let Some(id) = first_graph {
                        self.export = Some(ExportDialog::new(id, None));
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
                }
            }
        }
    }
}

/// A small drawn icon: a table for workbooks, a curve on axes for graphs.
fn window_icon(ui: &mut egui::Ui, graph: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
    let r = rect.shrink(1.0);
    let painter = ui.painter();
    let line = Stroke::new(1.0_f32, ui.visuals().text_color().gamma_multiply(0.75));
    if graph {
        painter.line_segment([r.left_top(), r.left_bottom()], line);
        painter.line_segment([r.left_bottom(), r.right_bottom()], line);
        let accent = Stroke::new(1.5_f32, egui::Color32::from_rgb(40, 110, 200));
        let pts: Vec<egui::Pos2> = [(0.1, 0.8), (0.35, 0.45), (0.6, 0.6), (0.95, 0.1)]
            .iter()
            .map(|(x, y)| egui::pos2(r.left() + x * r.width(), r.top() + y * r.height()))
            .collect();
        painter.add(egui::Shape::line(pts, accent));
    } else {
        painter.rect_stroke(r, 1.0, line, egui::StrokeKind::Inside);
        let third_w = r.width() / 3.0;
        let third_h = r.height() / 3.0;
        for k in 1..3 {
            let x = r.left() + third_w * k as f32;
            let y = r.top() + third_h * k as f32;
            painter.line_segment([egui::pos2(x, r.top()), egui::pos2(x, r.bottom())], line);
            painter.line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], line);
        }
    }
}

struct Tabs<'a> {
    doc: &'a Document,
    books: &'a mut HashMap<WindowId, BookUi>,
    graphs: &'a mut HashMap<WindowId, GraphUi>,
    actions: &'a mut Vec<Action>,
}

impl TabViewer for Tabs<'_> {
    type Tab = WindowId;

    fn title(&mut self, tab: &mut WindowId) -> egui::WidgetText {
        match self.doc.project().window(*tab) {
            Some(w) => w.title().into(),
            None => "(closed)".into(),
        }
    }

    fn id(&mut self, tab: &mut WindowId) -> egui::Id {
        egui::Id::new(("window_tab", tab.0))
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut WindowId) {
        match self.doc.project().window(*tab) {
            Some(Window::Workbook(_)) => {
                let state = self.books.entry(*tab).or_default();
                grid::show(ui, self.doc, *tab, state, self.actions);
            }
            Some(Window::Graph(_)) => {
                let state = self.graphs.entry(*tab).or_default();
                graph_view::show(ui, self.doc, *tab, state, self.actions);
            }
            None => {
                ui.label("This window no longer exists.");
            }
        }
    }

    fn scroll_bars(&self, _tab: &WindowId) -> [bool; 2] {
        [false, false]
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
        self.shortcuts(ctx, &mut actions);
        self.active = self.dock.find_active_focused().map(|(_, tab)| *tab);

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| self.menu_bar(ui, &mut actions));
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.doc.dirty {
                        ui.weak("unsaved changes");
                    }
                });
            });
        });
        egui::SidePanel::left("explorer")
            .resizable(true)
            .default_width(200.0)
            .width_range(120.0..=420.0)
            .show(ctx, |ui| self.explorer(ui, &mut actions));
        egui::CentralPanel::default().frame(egui::Frame::central_panel(&ctx.style()).inner_margin(0.0)).show(
            ctx,
            |ui| {
                if self.doc.project().windows.is_empty() {
                    self.welcome(ui, &mut actions);
                } else {
                    let mut tabs = Tabs {
                        doc: &self.doc,
                        books: &mut self.books,
                        graphs: &mut self.graphs,
                        actions: &mut actions,
                    };
                    DockArea::new(&mut self.dock)
                        .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                        .show_leaf_collapse_buttons(false)
                        .show_leaf_close_all_buttons(false)
                        .show_inside(ui, &mut tabs);
                }
            },
        );

        self.dialogs(ctx);
        for action in actions {
            self.apply(action, ctx);
        }
        self.sync_tabs();

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

#[cfg(test)]
mod ui_tests;
