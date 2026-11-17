//! The project: every workbook and graph, plus the folder tree that
//! organizes them.

use crate::column::{Column, ColumnData, Designation};
use crate::graph::{ErrorStyle, Graph, Indexed, IndexedError, IntervalError, LegendScope, Plot, StatPlot};
use crate::ids::{ColumnId, WindowId};
use crate::multi_y::{MultiYError, MultiYKind, MultiYOptions, MultiYPlan};
use crate::panels::{PanelError, PanelLayout, PanelPage, panel_layers};
use crate::template::{Template, right_y_layer};
use crate::worksheet::{Workbook, Worksheet};
use serde::{Deserialize, Serialize};

/// A window of the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Window {
    Workbook(Workbook),
    Graph(Graph),
}

impl Window {
    pub fn id(&self) -> WindowId {
        match self {
            Window::Workbook(b) => b.id,
            Window::Graph(g) => g.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Window::Workbook(b) => &b.name,
            Window::Graph(g) => &g.name,
        }
    }

    pub fn title(&self) -> String {
        match self {
            Window::Workbook(b) => b.title(),
            Window::Graph(g) => g.title(),
        }
    }
}

/// View > View Windows (T100): which windows the workspace shows when the Project Explorer's active
/// folder changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewWindows {
    /// Every window, whatever folder is active.
    None,
    /// Only the windows of the active folder (Origin's default).
    #[default]
    ActiveFolder,
    /// The windows of the active folder and of the folders inside it.
    ActiveFolderAndSubfolders,
}

impl ViewWindows {
    pub const ALL: [ViewWindows; 3] =
        [ViewWindows::None, ViewWindows::ActiveFolder, ViewWindows::ActiveFolderAndSubfolders];

    pub fn is_default(&self) -> bool {
        *self == ViewWindows::default()
    }

    /// The menu text, as Origin's.
    pub fn name(self) -> &'static str {
        match self {
            ViewWindows::None => "None",
            ViewWindows::ActiveFolder => "Windows in Active Folder",
            ViewWindows::ActiveFolderAndSubfolders => "Active Folder and Subfolders",
        }
    }
}

/// A folder of the project explorer. Windows that no folder lists belong to
/// the root folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Folder {
    pub name: String,
    /// The folder's Comments (Folder Properties, T100).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comments: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<Folder>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<WindowId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Windows in creation order.
    pub windows: Vec<Window>,
    /// Which windows the workspace shows for the Project Explorer's active folder (View > View
    /// Windows, T100; Origin 2016 starts with Windows in Active Folder).
    #[serde(default, skip_serializing_if = "ViewWindows::is_default")]
    pub view_windows: ViewWindows,
    #[serde(default)]
    pub root: Folder,
    /// Where the windows were in the workspace when the project was saved, bottom of the stack
    /// first. Empty when unknown (a new project, or one read from an Origin file); windows that are
    /// not listed are placed by the cascade. The desktop app fills it just before saving and reads
    /// it on opening; it is view state, not data, so editing it is never an undo step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub window_places: Vec<WindowPlace>,
    /// Analysis runs whose results follow their input (Recalculate Auto or Manual).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub analysis_links: Vec<crate::analysis_link::AnalysisLink>,
    /// Page Setup (File > Page Setup): paper, orientation and margins for printing. Saved with the
    /// project; changing it is an undo step like any other setting.
    #[serde(default, skip_serializing_if = "is_default_page_setup")]
    pub page_setup: crate::page_setup::PageSetup,
    /// Colour of masked points: an index into Origin's colour list ([`crate::template::ORIGIN_COLOR_LIST`]),
    /// 1 (red) at first. Column > Mask > Change Color steps it on (Origin's `@MC`, a global setting
    /// there; saved with the project here).
    #[serde(default = "default_mask_color", skip_serializing_if = "is_default_mask_color")]
    pub mask_color: u8,
    /// Column > Mask > Disable Masking, one switch for the whole project as in Origin (`@MM`). The
    /// columns carry a copy ([`crate::column::Column::mask_disabled`], what the readers use), which
    /// [`Project::apply_mask_switch`] brings back in line after every edit, so new columns follow it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    masking_off: bool,
    /// Hide masked points (Origin's Mask toolbar switch `@MP`): graphs leave masked points out, as
    /// gaps like missing values, instead of drawing them in the mask colour. Analysis is unchanged.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_masked: bool,
    /// The view mode of graph windows that are not in Page View (View > Print View, Window View,
    /// Draft View); a graph that is not listed is in Page View. View state: it is saved with the
    /// project but setting it is never an undo step (see [`Project::set_view_mode`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub view_modes: Vec<(WindowId, ViewMode)>,
    /// The next unused identifier (shared by windows and columns).
    next_id: u64,
}

/// Origin's rule for the short name of a window (measured on workbooks in Origin 2016 with `win -r`,
/// workbook-model.md 1): only ASCII letters and digits are kept (spaces, `_`, `-`, `.`, `/`, brackets
/// and the like are dropped); a name that is then empty or starts with a digit gets an `A` in front;
/// the result is cut to 13 characters without a message (`12345678901234` becomes `A123456789012`).
pub fn clean_window_name(text: &str) -> String {
    let mut name: String = text.chars().filter(char::is_ascii_alphanumeric).collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, 'A');
    }
    name.chars().take(13).collect()
}

/// `formula` with every `[old]` (any case) replaced by `[new]`.
fn replace_book_reference(formula: &str, old: &str, new: &str) -> String {
    let needle = format!("[{}]", old.to_ascii_lowercase());
    let lower = formula.to_ascii_lowercase();
    let mut out = String::with_capacity(formula.len());
    let mut i = 0;
    while let Some(k) = lower[i..].find(&needle) {
        out.push_str(&formula[i..i + k]);
        out.push('[');
        out.push_str(new);
        out.push(']');
        i += k + needle.len();
    }
    out.push_str(&formula[i..]);
    out
}

/// `formula` with every `[old]` (any case) of `renames` replaced by `[new]`, in one pass over the
/// text, so a name that is the new name of another is not changed twice.
fn replace_book_references(formula: &str, renames: &[(String, String)]) -> String {
    let mut out = String::with_capacity(formula.len());
    let mut rest = formula;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &after[..close];
        match renames.iter().find(|(old, _)| old.eq_ignore_ascii_case(name)) {
            Some((_, new)) => {
                out.push('[');
                out.push_str(new);
                out.push(']');
            }
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

fn default_mask_color() -> u8 {
    1
}

fn is_default_mask_color(c: &u8) -> bool {
    *c == 1
}

fn is_default_page_setup(s: &crate::page_setup::PageSetup) -> bool {
    *s == crate::page_setup::PageSetup::default()
}

/// The place of one window in the desktop app's workspace (ADR 0007 point 1).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowPlace {
    pub window: WindowId,
    /// Top-left corner relative to the workspace's top-left corner, and size, in screen points,
    /// of the window in its normal (not maximised) state.
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub state: WindowState,
}

/// How a window was shown.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowState {
    #[default]
    Normal,
    Minimised,
    Maximised,
    /// Closed: listed in the project explorer only.
    Hidden,
}

/// How a graph window shows its page (View > Print View, Page View, Window View, Draft View; measured
/// in Origin 2016, graphing spec 22). The numbers are LabTalk's `page.viewmode`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    /// 1: the page at its own size (looked the same as Page View).
    Print,
    /// 2: the page at its own size on a grey ground; the default.
    #[default]
    Page,
    /// 3: the page stretched over the whole window.
    Window,
    /// 4: as Window View, with plain text.
    Draft,
}

impl ViewMode {
    pub const ALL: [ViewMode; 4] = [ViewMode::Print, ViewMode::Page, ViewMode::Window, ViewMode::Draft];

    /// LabTalk's `page.viewmode`.
    pub fn number(self) -> u8 {
        match self {
            ViewMode::Print => 1,
            ViewMode::Page => 2,
            ViewMode::Window => 3,
            ViewMode::Draft => 4,
        }
    }

    /// The mode of a `page.viewmode` value (other values are ignored by Origin).
    pub fn from_number(n: u8) -> Option<ViewMode> {
        ViewMode::ALL.into_iter().find(|m| m.number() == n)
    }

    /// Window and Draft View stretch the page over the whole window.
    pub fn fills_window(self) -> bool {
        matches!(self, ViewMode::Window | ViewMode::Draft)
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProjectError {
    #[error("there is no window with id {0:?}")]
    NoSuchWindow(WindowId),
    #[error("there is no window named '{0}'")]
    NoSuchWindowName(String),
    #[error("window '{0}' is not a workbook")]
    NotAWorkbook(String),
    #[error("window '{0}' is not a graph")]
    NotAGraph(String),
    #[error("there is no column with id {0:?}")]
    NoSuchColumn(ColumnId),
    #[error("stacked histograms need at least one Y column that holds numbers")]
    StackedHistogramsNeedNumbers,
    #[error("column '{0}' was not found")]
    NoSuchColumnName(String),
    #[error("a Line Series needs two or more Y columns, not {0}")]
    LineSeriesNeedsTwoColumns(usize),
    #[error("the columns of a Line Series must all be of one sheet")]
    LineSeriesOneSheet,
    #[error("the graph has no layer {0}")]
    NoSuchLayer(usize),
    #[error("the layer has no plot {0}")]
    NoSuchPlot(usize),
    #[error("{0}")]
    Panels(#[from] PanelError),
    #[error("{0}")]
    MultiY(#[from] MultiYError),
    #[error("the {0} template puts one Y column on each axis by itself; leave out --y2 and the left/right split")]
    MultiYNoRightAxis(&'static str),
    #[error("{0}")]
    VectorColumns(&'static str),
    #[error("{0}")]
    StockColumns(&'static str),
    #[error("{0}")]
    MarginalColumns(&'static str),
    #[error("a scatter matrix needs 2 to {max} numeric columns (got {0}); select two or more columns", max = crate::scatter_matrix::MAX_VARIABLES)]
    ScatterMatrixColumns(usize),
    #[error("a scatter matrix plots numbers; column {0} holds text")]
    ScatterMatrixText(String),
    #[error("the gap between scatter matrix cells must be from 0 to 10 percent of the page")]
    ScatterMatrixGap,
    #[error("an X bar R chart takes subgroups of 2 to 10 values and a column with numbers (asked for {0})")]
    QcSubgroup(usize),
    #[error("a zoom graph needs a Y column; select one or more")]
    ZoomNeedsColumn,
    #[error("{0}")]
    WindRoseColumns(&'static str),
    #[error("a 2D kernel density needs one Y column with an X column")]
    Kde2dColumns,
    #[error("a Piper diagram needs six Y columns: Ca, Mg, Na+K, Cl, SO4, HCO3+CO3 (got {0})")]
    PiperColumns(usize),
    #[error("a lollipop plot needs two or more Y columns (got {0}); select at least two Y columns")]
    LollipopNeedsTwoColumns(usize),
    #[error("a lollipop plot cannot have a right Y axis; use one Y axis or another template")]
    LollipopNoRightAxis,
    #[error("a violin plot cannot have a right Y axis; use one Y axis or another template")]
    ViolinNoRightAxis,
    #[error("a ridgeline chart cannot have a right Y axis; use one Y axis or another template")]
    RidgelineNoRightAxis,
    #[error("a hexbin plot cannot have a right Y axis; use one Y axis or another template")]
    HexbinNoRightAxis,
    #[error("a parallel plot needs two or more numeric columns (got {0}); select at least two Y columns")]
    ParallelNeedsTwoColumns(usize),
    #[error(
        "a parallel index needs three or more numeric columns (got {0}) and a group column; select at least three Y columns"
    )]
    ParallelIndexNeedsThreeColumns(usize),
    #[error(
        "a parallel index needs a group column (a column of text, such as a species name); select one or name it with --group"
    )]
    ParallelIndexNeedsGroup,
    #[error("a parallel plot cannot have a right Y axis; use one Y axis or another template")]
    ParallelNoRightAxis,
    #[error("a treemap needs a category column (a column of text) and a numeric value column; select both")]
    TreemapNeedsCategory,
    #[error("a treemap needs a numeric value column; select one")]
    TreemapNeedsValue,
    #[error("a treemap cannot have a right Y axis; use one Y axis or another template")]
    TreemapNoRightAxis,
    #[error("a sunburst plot needs a category column (a column of text) and a numeric value column; select both")]
    SunburstNeedsCategory,
    #[error("a sunburst plot needs a numeric value column; select one")]
    SunburstNeedsValue,
    #[error("a sunburst plot cannot have a right Y axis; use one Y axis or another template")]
    SunburstNoRightAxis,
    #[error(
        "a grouped plot of indexed data needs exactly one numeric data column (designated Y; a numeric group column must be designated X or Group); select one"
    )]
    IndexedNeedsData,
    #[error(
        "a grouped plot of indexed data needs one or two group columns with levels besides the data column; select them"
    )]
    IndexedNeedsGroup,
    #[error(
        "a grouped plot of indexed data takes at most two group columns (got {0}); deeper hierarchies are not made yet"
    )]
    IndexedTooManyGroups(usize),
    #[error("a grouped plot of indexed data cannot have a right Y axis; use one Y axis or another template")]
    IndexedNoRightAxis,
    #[error("an interval plot needs at least one numeric data column; select one")]
    IntervalNeedsData,
    #[error("an interval plot cannot have a right Y axis; use one Y axis or another template")]
    IntervalNoRightAxis,
    #[error("a Pareto chart needs exactly one column with cells to count; select one")]
    ParetoNeedsColumn,
    #[error("a Pareto chart has its own right Y axis (the cumulative percent); use one column and no right axis")]
    ParetoNoRightAxis,
    #[error("a stacked histograms graph cannot have a right Y axis; use one Y axis or another template")]
    StackedHistogramsNoRightAxis,
}

impl Project {
    /// The colour masked points are drawn in.
    pub fn mask_color_rgb(&self) -> crate::Color {
        let list = &crate::template::ORIGIN_COLOR_LIST;
        let (r, g, b) = list[usize::from(self.mask_color) % list.len()];
        crate::Color::rgb(r, g, b)
    }

    /// Column > Mask > Change Color: the next colour of Origin's list, back to red (1) after the
    /// 24th (index 23), so black is never chosen (Origin's rule for `@MC`, measured 2026-11-15).
    pub fn cycle_mask_color(&mut self) {
        self.mask_color = if self.mask_color > 22 { 1 } else { self.mask_color + 1 };
    }

    /// Whether masking is disabled (Column > Mask > Disable Masking); the menu then offers Enable
    /// Masking.
    pub fn masking_disabled(&self) -> bool {
        self.masking_off
    }

    /// Switches masking off or on for the whole project.
    pub fn set_masking_disabled(&mut self, disabled: bool) {
        self.masking_off = disabled;
        self.apply_mask_switch();
    }

    /// Gives every column the project's masking switch (columns made after the switch, by an import,
    /// Unstack or a new sheet, start with masking on). Called after each edit and on loading.
    pub fn apply_mask_switch(&mut self) {
        let disabled = self.masking_off;
        for w in &mut self.windows {
            if let Window::Workbook(b) = w {
                for c in b.sheets.iter_mut().flat_map(|s| &mut s.columns) {
                    c.mask_disabled = disabled;
                }
            }
        }
    }

    pub fn new() -> Self {
        Project {
            windows: Vec::new(),
            view_windows: ViewWindows::default(),
            root: Folder { name: "Project".into(), ..Folder::default() },
            window_places: Vec::new(),
            analysis_links: Vec::new(),
            page_setup: crate::page_setup::PageSetup::default(),
            mask_color: default_mask_color(),
            masking_off: false,
            hide_masked: false,
            view_modes: Vec::new(),
            next_id: 1,
        }
    }

    /// The view mode of a graph window (Page View unless set).
    pub fn view_mode(&self, graph: WindowId) -> ViewMode {
        self.view_modes.iter().find(|(w, _)| *w == graph).map_or(ViewMode::Page, |(_, m)| *m)
    }

    /// Sets the view mode of a graph window; Page View, the default, is not stored.
    pub fn set_view_mode(&mut self, graph: WindowId, mode: ViewMode) {
        self.view_modes.retain(|(w, _)| *w != graph);
        if mode != ViewMode::Page {
            self.view_modes.push((graph, mode));
        }
    }

    pub(crate) fn fresh_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn new_column_id(&mut self) -> ColumnId {
        ColumnId(self.fresh_id())
    }

    /// Makes sure future identifiers do not collide with any identifier that
    /// is already in use. Call this after loading a project from a file.
    pub fn repair_ids(&mut self) {
        let mut max = 0;
        for w in &self.windows {
            max = max.max(w.id().0);
            if let Window::Workbook(b) = w {
                for s in &b.sheets {
                    for c in &s.columns {
                        max = max.max(c.id.0);
                    }
                }
            }
        }
        self.next_id = self.next_id.max(max + 1);
    }

    /// Gives window `id` the short name `wanted`, cleaned by Origin's rule ([`clean_window_name`]).
    /// Refused (with a message) when another window has the cleaned name, in any case. Column
    /// formulas that name the old window (`[Old]...`) are changed to the new name. Returns the name
    /// the window got.
    pub fn rename_window(&mut self, id: WindowId, wanted: &str) -> Result<String, String> {
        let new = clean_window_name(wanted);
        let old = self.window(id).map(|w| w.name().to_string()).ok_or_else(|| "no such window".to_string())?;
        if self.windows.iter().any(|w| w.id() != id && w.name().eq_ignore_ascii_case(&new)) {
            return Err(format!("The name {new} is already used by another window."));
        }
        match self.windows.iter_mut().find(|w| w.id() == id) {
            Some(Window::Workbook(b)) => b.name = new.clone(),
            Some(Window::Graph(g)) => g.name = new.clone(),
            None => {}
        }
        if old != new {
            for w in &mut self.windows {
                if let Window::Workbook(b) = w {
                    for c in b.sheets.iter_mut().flat_map(|s| &mut s.columns) {
                        if let Some(f) = &mut c.formula {
                            *f = replace_book_reference(f, &old, &new);
                        }
                    }
                }
            }
        }
        Ok(new)
    }

    /// A window name that is not in use: `prefix` followed by the smallest
    /// free number, for example `Book1`, `Graph3`.
    pub fn unique_name(&self, prefix: &str) -> String {
        (1..)
            .map(|n| format!("{prefix}{n}"))
            .find(|name| !self.windows.iter().any(|w| w.name().eq_ignore_ascii_case(name)))
            .expect("the name sequence is unbounded")
    }

    // ----- windows -------------------------------------------------------

    pub fn window(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().find(|w| w.id() == id)
    }

    pub fn window_by_name(&self, name: &str) -> Option<&Window> {
        self.windows.iter().find(|w| w.name().eq_ignore_ascii_case(name))
    }

    pub fn remove_window(&mut self, id: WindowId) -> Option<Window> {
        let i = self.windows.iter().position(|w| w.id() == id)?;
        fn forget(f: &mut Folder, id: WindowId) {
            f.windows.retain(|w| *w != id);
            f.folders.iter_mut().for_each(|c| forget(c, id));
        }
        forget(&mut self.root, id);
        self.window_places.retain(|p| p.window != id);
        Some(self.windows.remove(i))
    }

    pub fn workbooks(&self) -> impl Iterator<Item = &Workbook> {
        self.windows.iter().filter_map(|w| match w {
            Window::Workbook(b) => Some(b),
            Window::Graph(_) => None,
        })
    }

    pub fn graphs(&self) -> impl Iterator<Item = &Graph> {
        self.windows.iter().filter_map(|w| match w {
            Window::Graph(g) => Some(g),
            Window::Workbook(_) => None,
        })
    }

    pub fn workbook(&self, id: WindowId) -> Option<&Workbook> {
        self.workbooks().find(|b| b.id == id)
    }

    pub fn workbook_mut(&mut self, id: WindowId) -> Option<&mut Workbook> {
        self.windows.iter_mut().find_map(|w| match w {
            Window::Workbook(b) if b.id == id => Some(b),
            _ => None,
        })
    }

    pub fn graph(&self, id: WindowId) -> Option<&Graph> {
        self.graphs().find(|g| g.id == id)
    }

    pub fn graph_mut(&mut self, id: WindowId) -> Option<&mut Graph> {
        self.windows.iter_mut().find_map(|w| match w {
            Window::Graph(g) if g.id == id => Some(g),
            _ => None,
        })
    }

    /// Adds an empty workbook (one empty sheet) and returns its id.
    pub fn add_workbook(&mut self, long_name: &str) -> WindowId {
        let id = WindowId(self.fresh_id());
        let mut book = Workbook::new(id, self.unique_name("Book"));
        book.long_name = long_name.to_string();
        self.windows.push(Window::Workbook(book));
        id
    }

    /// Adds a workbook that an importer built. The importer does not know
    /// which identifiers and names are free, so the workbook and all its
    /// columns get fresh ones here. Returns the new window id.
    pub fn adopt_workbook(&mut self, mut book: Workbook) -> WindowId {
        let id = WindowId(self.fresh_id());
        book.id = id;
        book.name = self.unique_name("Book");
        for sheet in &mut book.sheets {
            let mut renamed = std::collections::HashMap::new();
            for col in &mut sheet.columns {
                let new = ColumnId(self.fresh_id());
                renamed.insert(col.id, new);
                col.id = new;
            }
            sheet.remap_cond_format_columns(&|c| renamed.get(&c).copied());
            sheet.remap_note_columns(&|c| renamed.get(&c).copied());
            sheet.remap_format_columns(&|c| renamed.get(&c).copied());
        }
        self.windows.push(Window::Workbook(book));
        id
    }

    /// Like [`Project::adopt_workbook`], but keeps the workbook's own short
    /// name when no other window uses it (a project imported from another
    /// program keeps its window names). A name that is taken or empty is
    /// replaced by a free `Book<n>` name.
    pub fn adopt_named_workbook(&mut self, book: Workbook) -> WindowId {
        let wanted = book.name.clone();
        let free = !wanted.is_empty() && !self.windows.iter().any(|w| w.name().eq_ignore_ascii_case(&wanted));
        let id = self.adopt_workbook(book);
        if free && let Some(b) = self.workbook_mut(id) {
            b.name = wanted;
        }
        id
    }

    /// Adds a graph that an importer built. It gets a fresh window id; its name is kept when no
    /// other window uses it and replaced by a free `Graph<n>` name otherwise.
    pub fn adopt_graph(&mut self, mut graph: Graph) -> WindowId {
        let id = WindowId(self.fresh_id());
        graph.id = id;
        let wanted = graph.name.clone();
        let free = !wanted.is_empty() && !self.windows.iter().any(|w| w.name().eq_ignore_ascii_case(&wanted));
        if !free {
            graph.name = self.unique_name("Graph");
        }
        self.windows.push(Window::Graph(graph));
        id
    }

    /// File > Append: brings every window of `other` into this project, in a new folder named
    /// `folder` that holds `other`'s folder tree. Window and column ids of `other` are moved past
    /// this project's ids (graphs keep plotting their own columns); window names that are taken
    /// here get free names, and the formulas of the appended workbooks follow the new names.
    /// `other`'s analysis results that recalculate are kept as plain values.
    /// Returns the new windows' ids and how many recalculation links were dropped.
    pub fn append(&mut self, other: Project, folder: &str) -> (Vec<WindowId>, usize) {
        self.append_into(other, Some(folder))
    }

    /// Opening a window file (File > Save Window As): like [`Project::append`], but the windows
    /// join this project's root folder and no folder is made (Origin adds a window to the active
    /// folder; Sheaf has no active folder yet). `other`'s folders are dropped.
    pub fn append_windows(&mut self, other: Project) -> (Vec<WindowId>, usize) {
        self.append_into(other, None)
    }

    fn append_into(&mut self, mut other: Project, folder: Option<&str>) -> (Vec<WindowId>, usize) {
        let shift = self.next_id;
        let w = |id: WindowId| WindowId(id.0 + shift);
        let c = |id: ColumnId| ColumnId(id.0 + shift);
        let dropped_links = other.analysis_links.len();
        let mut new_ids = Vec::new();
        for window in &mut other.windows {
            match window {
                Window::Workbook(book) => {
                    book.id = w(book.id);
                    for sheet in &mut book.sheets {
                        for col in &mut sheet.columns {
                            col.id = c(col.id);
                            col.analysis = None;
                        }
                        sheet.remap_cond_format_columns(&|id| Some(c(id)));
                        sheet.remap_note_columns(&|id| Some(c(id)));
                        sheet.remap_format_columns(&|id| Some(c(id)));
                    }
                }
                Window::Graph(graph) => {
                    graph.id = w(graph.id);
                    graph.remap_columns(&c);
                }
            }
        }
        fn shift_folder(f: &mut Folder, w: &dyn Fn(WindowId) -> WindowId) {
            for id in &mut f.windows {
                *id = w(*id);
            }
            for sub in &mut f.folders {
                shift_folder(sub, w);
            }
        }
        shift_folder(&mut other.root, &w);
        self.next_id = shift + other.next_id;
        let first_new = self.windows.len();
        let mut renames: Vec<(String, String)> = Vec::new();
        for window in other.windows {
            let wanted = window.name().to_string();
            let taken = self.windows.iter().any(|x| x.name().eq_ignore_ascii_case(&wanted));
            let mut window = window;
            if taken {
                let prefix = if matches!(window, Window::Graph(_)) { "Graph" } else { "Book" };
                let name = self.unique_name(prefix);
                if matches!(window, Window::Workbook(_)) {
                    renames.push((wanted, name.clone()));
                }
                match &mut window {
                    Window::Workbook(b) => b.name = name,
                    Window::Graph(g) => g.name = name,
                }
            }
            new_ids.push(window.id());
            self.windows.push(window);
        }
        if !renames.is_empty() {
            for window in &mut self.windows[first_new..] {
                let Window::Workbook(book) = window else { continue };
                for col in book.sheets.iter_mut().flat_map(|s| &mut s.columns) {
                    if let Some(f) = &mut col.formula {
                        *f = replace_book_references(f, &renames);
                    }
                }
            }
        }
        if let Some(folder) = folder {
            let mut sub = other.root;
            sub.name = folder.to_string();
            self.root.folders.push(sub);
        }
        (new_ids, dropped_links)
    }

    /// Window > Duplicate: a copy of a workbook or graph as a new window with a free name. A
    /// workbook's copy has its own columns (fresh ids; a copied analysis result is plain data, no
    /// longer linked to its source); a graph's copy plots the same columns. Returns the new id.
    pub fn duplicate_window(&mut self, id: WindowId) -> Option<WindowId> {
        match self.window(id)?.clone() {
            Window::Workbook(mut book) => {
                for col in book.sheets.iter_mut().flat_map(|s| s.columns.iter_mut()) {
                    col.analysis = None;
                }
                Some(self.adopt_workbook(book))
            }
            Window::Graph(graph) => Some(self.adopt_graph(graph)),
        }
    }

    /// Adds an empty graph in the style of `template` and returns its id.
    pub fn add_graph(&mut self, template: Template) -> WindowId {
        let id = WindowId(self.fresh_id());
        let name = self.unique_name("Graph");
        self.windows.push(Window::Graph(template.new_graph(id, &name)));
        id
    }

    // ----- columns -------------------------------------------------------

    /// Appends a column to a sheet and returns its id.
    pub fn add_column(
        &mut self,
        book: WindowId,
        sheet: usize,
        designation: Designation,
        data: ColumnData,
    ) -> Result<ColumnId, ProjectError> {
        let id = ColumnId(self.fresh_id());
        let ws = self
            .workbook_mut(book)
            .ok_or(ProjectError::NoSuchWindow(book))?
            .sheets
            .get_mut(sheet)
            .ok_or(ProjectError::NoSuchLayer(sheet))?;
        let name = ws.next_short_name();
        ws.columns.push(Column::new(id, name, designation, data));
        Ok(id)
    }

    /// Finds a column anywhere in the project.
    pub fn column(&self, id: ColumnId) -> Option<&Column> {
        self.locate(id).map(|(_, _, c)| c)
    }

    /// Finds a column together with the workbook and sheet that hold it.
    pub fn locate(&self, id: ColumnId) -> Option<(&Workbook, &Worksheet, &Column)> {
        for b in self.workbooks() {
            for s in &b.sheets {
                if let Some(c) = s.column(id) {
                    return Some((b, s, c));
                }
            }
        }
        None
    }

    pub fn column_mut(&mut self, id: ColumnId) -> Option<&mut Column> {
        for w in &mut self.windows {
            if let Window::Workbook(b) = w {
                for s in &mut b.sheets {
                    if let Some(c) = s.column_mut(id) {
                        return Some(c);
                    }
                }
            }
        }
        None
    }

    /// The numbers of a numeric column, or `None` if the column does not
    /// exist or holds text.
    pub fn numeric(&self, id: ColumnId) -> Option<&[f64]> {
        self.column(id).and_then(Column::numeric)
    }

    // ----- plots ---------------------------------------------------------

    /// Adds a plot of column `y` to a layer of a graph. The X column and a
    /// Y-error column are found from the column designations of the sheet; a column with a
    /// sampling interval gets no X column, its plot uses the interval (as Origin's does).
    /// The plot takes the layer's default style and its next palette color.
    pub fn add_plot(&mut self, graph: WindowId, layer: usize, y: ColumnId) -> Result<(), ProjectError> {
        let (_, sheet, _) = self.locate(y).ok_or(ProjectError::NoSuchColumn(y))?;
        let index = sheet.column_index(y).expect("column is in this sheet");
        let x = if sheet.columns[index].sampling.is_some() { None } else { sheet.x_for(index).map(|c| c.id) };
        let y_error = sheet.y_error_for(index).map(|c| c.id);
        let x_error = sheet.x_error_for(index).map(|c| c.id);
        self.add_plot_xy(graph, layer, x, y, y_error)?;
        if let Some(p) = self.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.plots.last_mut()) {
            p.x_error = x_error;
        }
        Ok(())
    }

    /// The Z column that belongs to the Y column `y`: a Z column after it, before the next X or Y
    /// column.
    pub fn z_column_for(&self, y: ColumnId) -> Option<ColumnId> {
        let (_, sheet, _) = self.locate(y)?;
        let index = sheet.column_index(y)?;
        sheet
            .columns
            .iter()
            .skip(index + 1)
            .take_while(|c| !matches!(c.designation, Designation::X | Designation::Y))
            .find(|c| c.designation == Designation::Z)
            .map(|c| c.id)
    }

    /// The column that holds the third part of a ternary plot of the Y column `y`: its Z column,
    /// or, when the sheet has none, the numeric column right after it (a sheet of three plain
    /// columns X, Y, Y).
    pub fn ternary_z_for(&self, y: ColumnId) -> Option<ColumnId> {
        self.z_column_for(y).or_else(|| {
            let (_, sheet, _) = self.locate(y)?;
            let next = sheet.columns.get(sheet.column_index(y)? + 1)?;
            (next.data.is_numeric() && next.designation != Designation::X).then_some(next.id)
        })
    }

    /// Adds a plot with explicitly chosen X and error columns.
    pub fn add_plot_xy(
        &mut self,
        graph: WindowId,
        layer: usize,
        x: Option<ColumnId>,
        y: ColumnId,
        y_error: Option<ColumnId>,
    ) -> Result<(), ProjectError> {
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let l = g.layers.get_mut(layer).ok_or(ProjectError::NoSuchLayer(layer))?;
        l.plots.push(Plot {
            x,
            y,
            y_error,
            rows: None,
            kind: l.default_kind,
            line: l.default_line.clone(),
            symbol: l.default_symbol.clone(),
            label: None,
            visible: true,
            auto_color: true,
            group: None,
            x_error: None,
            y_base: None,
            error: ErrorStyle::default(),
            hist: Default::default(),
            ridge: Default::default(),
            hexbin: Default::default(),
            parallel: Default::default(),
            treemap: Default::default(),
            indexed: None,
            stat: None,
            sunburst: Default::default(),
            size_from: None,
            color_from: None,
            decor: Default::default(),
        });
        l.recolor_automatic();
        Ok(())
    }

    /// The legend text of a plot: its own label if set, otherwise the Y
    /// column's comments, long name or short name (the first that is not empty).
    pub fn plot_label(&self, plot: &Plot) -> String {
        if let Some(l) = &plot.label {
            return l.clone();
        }
        match self.column(plot.y) {
            Some(c) if !c.comments.is_empty() => c.comments.lines().next().unwrap_or_default().to_string(),
            Some(c) => c.display_name().to_string(),
            None => "(missing column)".to_string(),
        }
    }

    /// The plot as Origin's Plot Details tree names it (measured, graphing spec 18):
    /// `[Book1]Sheet1! A(X), B(Y) [1*:11*]`: the short names of the book, sheet and columns and the
    /// rows, where a star marks an end that follows the data; a set row range is shown without
    /// stars. An error column is listed as `C(yEr±)` (not measured).
    pub fn plot_data_name(&self, plot: &Plot) -> String {
        let Some((book, sheet, y)) = self.locate(plot.y) else {
            return self.plot_label(plot);
        };
        let mut cols = Vec::new();
        if let Some(x) = plot.x.and_then(|x| self.column(x)) {
            cols.push(format!("{}(X)", x.short_name));
        }
        cols.push(format!("{}(Y)", y.short_name));
        if let Some(e) = plot.y_error.and_then(|e| self.column(e)) {
            cols.push(format!("{}(yEr±)", e.short_name));
        }
        let rows = match plot.rows {
            Some((a, b)) => format!("[{}:{}]", a + 1, b),
            None => format!("[1*:{}*]", y.len()),
        };
        format!("[{}]{}! {} {rows}", book.name, sheet.name, cols.join(", "))
    }

    /// The Y columns a new graph of `template` should show for a sheet.
    ///
    /// That is every numeric Y column when they all share one unit. When the
    /// units differ, one Y axis cannot show them all, so only one group is
    /// plotted: the columns in mA if there are any (an imported CH Instruments
    /// file has the current in A and in mA plus, for current-vs-time files,
    /// the charge; figures use mA), otherwise the columns that share the unit
    /// of the first Y column.
    ///
    /// A double-Y graph is made for columns with different units, so for
    /// [`Template::DoubleY`] every numeric Y column is returned.
    pub fn default_plot_columns(&self, book: WindowId, sheet: usize, template: Template) -> Vec<ColumnId> {
        let Some(ws) = self.workbook(book).and_then(|b| b.sheets.get(sheet)) else {
            return Vec::new();
        };
        let ys: Vec<&Column> =
            ws.columns.iter().filter(|c| c.designation == Designation::Y && c.data.is_numeric()).collect();
        // A scatter matrix compares variables of any unit (T114).
        if template == Template::DoubleY || template == Template::ScatterMatrix || template.multi_y().is_some() {
            return ys.iter().map(|c| c.id).collect();
        }
        if template == Template::Ternary {
            // The plots that have a third column; without a Z column the first Y column and the
            // column after it.
            let with_z: Vec<ColumnId> = ys.iter().map(|c| c.id).filter(|&y| self.z_column_for(y).is_some()).collect();
            if !with_z.is_empty() {
                return with_z;
            }
            return ys.iter().map(|c| c.id).find(|&y| self.ternary_z_for(y).is_some()).into_iter().collect();
        }
        let Some(first) = ys.first() else {
            return Vec::new();
        };
        let unit = if ys.iter().any(|c| c.units == "mA") { "mA" } else { first.units.as_str() };
        ys.iter().filter(|c| c.units == unit).map(|c| c.id).collect()
    }

    /// The columns a fit takes when none are named: those of `default_plot_columns`, without the
    /// "Fitted ..." and "Residual of ..." columns that earlier fits wrote (ADR 0006).
    pub fn default_fit_columns(&self, book: WindowId, sheet: usize) -> Vec<ColumnId> {
        let Some(ws) = self.workbook(book).and_then(|b| b.sheets.get(sheet)) else {
            return Vec::new();
        };
        let ys: Vec<&Column> = ws
            .columns
            .iter()
            .filter(|c| c.designation == Designation::Y && c.data.is_numeric() && !c.fit_result)
            .collect();
        let Some(first) = ys.first() else {
            return Vec::new();
        };
        let unit = if ys.iter().any(|c| c.units == "mA") { "mA" } else { first.units.as_str() };
        ys.iter().filter(|c| c.units == unit).map(|c| c.id).collect()
    }

    /// Creates a graph of `template` that plots the given Y columns in one
    /// layer, and returns its id. The graph's long name is taken from the
    /// workbook that holds the first column.
    ///
    /// For [`Template::DoubleY`] the columns alternate between the axes as in
    /// Origin's Double-Y command: the 1st, 3rd, 5th ... go on the left axis and
    /// the 2nd, 4th ... on the right axis.
    pub fn plot_columns(&mut self, template: Template, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        if let Some(kind) = template.multi_y() {
            return self.plot_multi_y_template(kind, ys);
        }
        if template == Template::DoubleY {
            let left: Vec<ColumnId> = ys.iter().step_by(2).copied().collect();
            let right: Vec<ColumnId> = ys.iter().skip(1).step_by(2).copied().collect();
            return self.plot_columns_double_y(template, &left, &right);
        }
        for y in ys {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        if template == Template::StackedHistograms {
            return self.plot_stacked_histograms(ys);
        }
        if matches!(template, Template::VectorXyam | Template::VectorXyxy) {
            return self.plot_vectors(template, ys);
        }
        if matches!(template, Template::HighLowClose | Template::Candlestick | Template::OhlcBar | Template::OhlcVolume)
        {
            return self.plot_stock(template, ys);
        }
        if matches!(template, Template::MarginalHistograms | Template::MarginalBoxCharts) {
            return self.plot_marginal(template, ys);
        }
        // An X bar R chart of the first column in Origin's default subgroups of 3 (the app asks for the size).
        if template == Template::Piper {
            return self.plot_piper(ys);
        }
        if template == Template::Kde2d {
            return self.plot_kde2d(ys);
        }
        if matches!(template, Template::WindRoseBinned | Template::WindRoseRaw) {
            return self.plot_wind_rose(template, ys);
        }
        if template == Template::Zoom {
            return self.plot_zoom(ys);
        }
        if template == Template::QcXbarR {
            let y = *ys.first().ok_or(ProjectError::QcSubgroup(crate::qc::DEFAULT_GROUP))?;
            return self.plot_qc_xbar_r(y, crate::qc::DEFAULT_GROUP).map(|(_, g)| g);
        }
        if template == Template::ScatterMatrix {
            return self.plot_scatter_matrix(ys, &crate::scatter_matrix::ScatterMatrixOptions::default());
        }
        if template == Template::Lollipop && ys.len() < 2 {
            return Err(ProjectError::LollipopNeedsTwoColumns(ys.len()));
        }
        if template.is_parallel() {
            // The numeric columns are the axes; for a parallel index a column of text among them is the group
            // (else the first text column of the sheet), for a parallel plot the last axis colours the lines.
            let axes: Vec<ColumnId> = ys.iter().copied().filter(|y| self.numeric(*y).is_some()).collect();
            let group = ys.iter().copied().find(|y| self.numeric(*y).is_none());
            let color = if template == Template::ParallelIndex {
                Some(
                    group
                        .or_else(|| self.parallel_group_default(&axes))
                        .ok_or(ProjectError::ParallelIndexNeedsGroup)?,
                )
            } else {
                axes.last().copied()
            };
            return self.plot_parallel(template, &axes, color);
        }
        if template == Template::Treemap {
            // The category columns are the columns of text, in selection order (else those of the value column's
            // sheet); the value is the last numeric column.
            let value =
                ys.iter().rev().copied().find(|y| self.numeric(*y).is_some()).ok_or(ProjectError::TreemapNeedsValue)?;
            let mut categories: Vec<ColumnId> = ys.iter().copied().filter(|y| self.numeric(*y).is_none()).collect();
            if categories.is_empty() {
                categories = self.treemap_default_categories(value);
            }
            return self.plot_treemap(&categories, value);
        }
        if template == Template::IntervalPlot {
            return self.plot_interval(ys);
        }
        if template == Template::Pareto {
            let [y] = ys else { return Err(ProjectError::ParetoNeedsColumn) };
            return self.plot_pareto(*y);
        }
        if matches!(template, Template::GroupedColumns | Template::GroupedBox) {
            // The data is the one numeric column designated Y; every other given column is a group column.
            let is_data = |p: &Self, c: ColumnId| {
                p.numeric(c).is_some() && p.column(c).is_some_and(|c| c.designation == Designation::Y)
            };
            let data: Vec<ColumnId> = ys.iter().copied().filter(|c| is_data(self, *c)).collect();
            let [y] = data[..] else { return Err(ProjectError::IndexedNeedsData) };
            let groups: Vec<ColumnId> = ys.iter().copied().filter(|c| *c != y).collect();
            return self.plot_indexed(template, &groups, y);
        }
        if template == Template::Sunburst {
            // Like the treemap: the columns of text are the rings, in selection order (else those of the value
            // column's sheet); the value is the last numeric column.
            let value = ys
                .iter()
                .rev()
                .copied()
                .find(|y| self.numeric(*y).is_some())
                .ok_or(ProjectError::SunburstNeedsValue)?;
            let mut categories: Vec<ColumnId> = ys.iter().copied().filter(|y| self.numeric(*y).is_none()).collect();
            if categories.is_empty() {
                categories = self.treemap_default_categories(value);
            }
            return self.plot_sunburst(&categories, value);
        }
        // One hexbin plot per layer: the colour scale and the hexagons of a second plot would cover the first's.
        let ys = if template == Template::Hexbin { &ys[..ys.len().min(1)] } else { ys };
        let long_name = ys.first().and_then(|y| self.locate(*y)).map(|(b, _, _)| b.title_for_graph());
        let graph = self.add_graph(template);
        if matches!(template, Template::FloatingColumn | Template::FloatingBar) {
            // Origin chains the columns: each plot is a bar from the column before it to its own
            // column, which names it (measured: B, C, D, E gave the bars B-C, C-D and D-E).
            for pair in ys.windows(2) {
                self.add_plot(graph, 0, pair[1])?;
                if let Some(p) = self.graph_mut(graph).and_then(|g| g.layers[0].plots.last_mut()) {
                    p.y_base = Some(pair[0]);
                }
            }
        } else if template == Template::FillArea {
            // Columns go in pairs: the first of a pair is the lower edge of the filled area, the
            // second the upper edge (and names the plot, as in Origin). A last odd column is left
            // out.
            for pair in ys.as_chunks::<2>().0 {
                self.add_plot(graph, 0, pair[1])?;
                if let Some(p) = self.graph_mut(graph).and_then(|g| g.layers[0].plots.last_mut()) {
                    p.y_base = Some(pair[0]);
                }
            }
        } else {
            for y in ys {
                self.add_plot(graph, 0, *y)?;
                // Bubble and colour mapped plots take their third variable from the Z column.
                let z = self.z_column_for(*y);
                let ternary_z = if template == Template::Ternary { self.ternary_z_for(*y) } else { None };
                let z_name = ternary_z.and_then(|z| self.column(z)).map(|c| c.display_name().to_string());
                let (size, color) = match template {
                    Template::Bubble => (true, false),
                    Template::ColorMapped => (false, true),
                    Template::BubbleColorMapped => (true, true),
                    _ => (false, false),
                };
                if let Some(p) = self.graph_mut(graph).and_then(|g| g.layers[0].plots.last_mut()) {
                    p.size_from = z.filter(|_| size);
                    p.color_from = z.filter(|_| color);
                    if template == Template::Ternary {
                        // The Z column of a ternary plot, the third part of each row; Origin's
                        // legend names the plot after it (measured).
                        p.size_from = ternary_z;
                        p.label = z_name;
                    }
                }
            }
        }
        if template == Template::Lollipop {
            self.make_lollipop(graph, ys);
        }
        // Origin's DROPLINE and ColumnLabel templates (graphing 18.12): the scatter with vertical drop lines,
        // the column plot with its values written above.
        if matches!(template, Template::VerticalDropLine | Template::ColumnLabel)
            && let Some(g) = self.graph_mut(graph)
        {
            if template == Template::VerticalDropLine {
                // DROPLINE steps the symbol shape through the group (square, circle, ...), the plain
                // scatter does not (both measured).
                for l in &mut g.layers {
                    l.group_style.symbol_type = crate::graph::Increment::ByOne;
                    l.apply_group_increments();
                }
            }
            for p in g.layers.iter_mut().flat_map(|l| l.plots.iter_mut()) {
                if template == Template::VerticalDropLine {
                    p.decor.drop.vertical.on = true;
                } else {
                    p.decor.labels.enable = true;
                    p.decor.labels.position = crate::graph::LabelPosition::Above;
                }
            }
        }
        if let Some(g) = self.graph_mut(graph) {
            // A new graph has no colors chosen by the user, so spread the palette over all plots.
            if let Some(l) = g.layers.first_mut() {
                l.apply_palette();
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }

    /// The category columns of a treemap of `value` when none are named: every column of text of its sheet, in
    /// sheet order.
    pub fn treemap_default_categories(&self, value: ColumnId) -> Vec<ColumnId> {
        let Some((_, sheet, _)) = self.locate(value) else { return Vec::new() };
        sheet.columns.iter().filter(|c| !c.data.is_numeric() && c.id != value).map(|c| c.id).collect()
    }

    /// Creates a treemap of `value` (a numeric column; the areas) nested by the `categories` columns (the first
    /// the top level) and returns its id. One plot, the value column's; the categories live in its style.
    pub fn plot_treemap(&mut self, categories: &[ColumnId], value: ColumnId) -> Result<WindowId, ProjectError> {
        for c in categories.iter().chain([&value]) {
            self.column(*c).ok_or(ProjectError::NoSuchColumn(*c))?;
        }
        if self.numeric(value).is_none() {
            return Err(ProjectError::TreemapNeedsValue);
        }
        let mut cats: Vec<ColumnId> = Vec::new();
        for c in categories {
            if *c != value && !cats.contains(c) {
                cats.push(*c);
            }
        }
        if cats.is_empty() {
            return Err(ProjectError::TreemapNeedsCategory);
        }
        let long_name = self.locate(value).map(|(b, _, _)| b.title_for_graph());
        let graph = self.add_graph(Template::Treemap);
        self.add_plot(graph, 0, value)?;
        if let Some(g) = self.graph_mut(graph) {
            if let Some(l) = g.layers.first_mut() {
                l.apply_palette();
                for p in &mut l.plots {
                    p.treemap.categories = cats.clone();
                }
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }

    /// Creates an interval plot ([`Template::IntervalPlot`], T83): one plot per numeric column in `ys`, each the
    /// mean of its column at the next position 1, 2, ... with a 95 % confidence interval as error bar.
    pub fn plot_interval(&mut self, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        for y in ys {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        if ys.is_empty() || ys.iter().any(|y| self.numeric(*y).is_none()) {
            return Err(ProjectError::IntervalNeedsData);
        }
        let long_name = self.locate(ys[0]).map(|(b, _, _)| b.title_for_graph());
        let graph = self.add_graph(Template::IntervalPlot);
        for y in ys {
            if let Err(e) = self.add_plot(graph, 0, *y) {
                let _ = self.remove_window(graph);
                return Err(e);
            }
            if let Some(p) = self.graph_mut(graph).and_then(|g| g.layers[0].plots.last_mut()) {
                p.x = None;
                p.y_error = None;
                p.x_error = None;
                p.stat = Some(StatPlot::Interval { error: IntervalError::Ci, level: 95.0 });
            }
        }
        if let Some(g) = self.graph_mut(graph) {
            g.layers[0].apply_palette();
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }

    /// Creates a Pareto chart ([`Template::Pareto`], T83) of the cells of column `y` (text or numbers): columns of
    /// the count of each distinct value, most frequent first, in the left layer, and the cumulative percent as a
    /// line with symbols in a right-axis layer fixed to 0 - 100.
    pub fn plot_pareto(&mut self, y: ColumnId) -> Result<WindowId, ProjectError> {
        let column = self.column(y).ok_or(ProjectError::NoSuchColumn(y))?;
        if crate::indexed::levels(column).is_empty() {
            return Err(ProjectError::ParetoNeedsColumn);
        }
        let long_name = self.locate(y).map(|(b, _, _)| b.title_for_graph());
        let graph = self.add_graph(Template::Pareto);
        let right = self.add_right_y_layer(graph, 0)?;
        for layer in [0, right] {
            if let Err(e) = self.add_plot(graph, layer, y) {
                let _ = self.remove_window(graph);
                return Err(e);
            }
            if let Some(g) = self.graph_mut(graph) {
                let l = &mut g.layers[layer];
                if let Some(p) = l.plots.last_mut() {
                    p.x = None;
                    p.y_error = None;
                    p.x_error = None;
                    p.stat = Some(if layer == 0 { StatPlot::Pareto } else { StatPlot::ParetoCumulative });
                    p.label = Some(if layer == 0 { "Count" } else { "Cumulative %" }.to_string());
                    if layer != 0 {
                        p.kind = crate::graph::PlotKind::LineSymbol;
                    }
                }
                l.y_axis.title = Some(if layer == 0 { "Count" } else { "Cumulative %" }.to_string());
                if layer != 0 {
                    l.y_axis.min = Some(0.0);
                    l.y_axis.max = Some(100.0);
                }
            }
        }
        if let Some(g) = self.graph_mut(graph) {
            for layer in [0, right] {
                g.layers[layer].apply_palette();
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }

    /// Creates a grouped column plot ([`Template::GroupedColumns`]) or grouped box chart ([`Template::GroupedBox`])
    /// of indexed data (T76): the rows of the numeric column `y` are split by the levels of one or two `groups`
    /// columns. One group column gives one plot; with two, there is one plot per sorted level of the second, named
    /// after the level, so the legend lists the subgroups and the first column's levels are on the X axis.
    pub fn plot_indexed(
        &mut self,
        template: Template,
        groups: &[ColumnId],
        y: ColumnId,
    ) -> Result<WindowId, ProjectError> {
        for c in groups.iter().chain([&y]) {
            self.column(*c).ok_or(ProjectError::NoSuchColumn(*c))?;
        }
        if self.numeric(y).is_none() {
            return Err(ProjectError::IndexedNeedsData);
        }
        let mut cols: Vec<ColumnId> = Vec::new();
        for c in groups {
            if *c != y && !cols.contains(c) {
                cols.push(*c);
            }
        }
        match cols.len() {
            0 => return Err(ProjectError::IndexedNeedsGroup),
            1 | 2 => {}
            n => return Err(ProjectError::IndexedTooManyGroups(n)),
        }
        // One plot for the whole data, or one per level of the second group column.
        let subsets: Vec<Option<(ColumnId, String)>> = match cols.get(1) {
            None => vec![None],
            Some(second) => {
                let levels = self.column(*second).map(crate::indexed::levels).unwrap_or_default();
                levels.into_iter().map(|l| Some((*second, l))).collect()
            }
        };
        if subsets.is_empty()
            || cols.first().and_then(|c| self.column(*c)).is_none_or(|c| crate::indexed::levels(c).is_empty())
        {
            return Err(ProjectError::IndexedNeedsGroup);
        }
        let long_name = self.locate(y).map(|(b, _, _)| b.title_for_graph());
        let graph = self.add_graph(template);
        for subset in subsets {
            if let Err(e) = self.add_plot(graph, 0, y) {
                let _ = self.remove_window(graph);
                return Err(e);
            }
            if let Some(p) = self.graph_mut(graph).and_then(|g| g.layers[0].plots.last_mut()) {
                // The categories are the group levels, not an X column; error columns do not apply.
                p.x = None;
                p.y_error = None;
                p.x_error = None;
                p.label = subset.as_ref().map(|(_, l)| l.clone());
                p.indexed = Some(Indexed { group: cols[0], subset, error: IndexedError::None });
            }
        }
        if let Some(g) = self.graph_mut(graph) {
            if let Some(l) = g.layers.first_mut() {
                l.apply_palette();
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }

    /// Creates a sunburst plot of `value` (a numeric column; the angles) with a ring per `categories` column (the
    /// first innermost) and returns its id. One plot, the value column's; the categories live in its style.
    pub fn plot_sunburst(&mut self, categories: &[ColumnId], value: ColumnId) -> Result<WindowId, ProjectError> {
        for c in categories.iter().chain([&value]) {
            self.column(*c).ok_or(ProjectError::NoSuchColumn(*c))?;
        }
        if self.numeric(value).is_none() {
            return Err(ProjectError::SunburstNeedsValue);
        }
        let mut cats: Vec<ColumnId> = Vec::new();
        for c in categories {
            if *c != value && !cats.contains(c) {
                cats.push(*c);
            }
        }
        if cats.is_empty() {
            return Err(ProjectError::SunburstNeedsCategory);
        }
        let long_name = self.locate(value).map(|(b, _, _)| b.title_for_graph());
        let graph = self.add_graph(Template::Sunburst);
        self.add_plot(graph, 0, value)?;
        if let Some(g) = self.graph_mut(graph) {
            if let Some(l) = g.layers.first_mut() {
                l.apply_palette();
                for p in &mut l.plots {
                    p.sunburst.categories = cats.clone();
                }
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }

    /// The column that groups the rows of a parallel index when none is named: the first column of the axes' sheet
    /// that is set as Group and holds text, else the first one that holds text (never one of the axes).
    pub fn parallel_group_default(&self, axes: &[ColumnId]) -> Option<ColumnId> {
        let (_, sheet, _) = self.locate(*axes.first()?)?;
        let texty = |c: &&Column| !c.data.is_numeric() && !axes.contains(&c.id);
        let by_designation = sheet.columns.iter().filter(texty).find(|c| c.designation == Designation::Group);
        by_designation.or_else(|| sheet.columns.iter().find(texty)).map(|c| c.id)
    }

    /// Creates a Parallel Plot or Parallel Index (`template`) with one vertical axis per column of `axes`, and
    /// returns its id. `color` is a Parallel Plot's numeric colour column or a Parallel Index's group column (a
    /// parallel index without one is refused; a parallel plot without one draws every line in the plot colour).
    /// A parallel plot needs two axes, a parallel index three.
    pub fn plot_parallel(
        &mut self,
        template: Template,
        axes: &[ColumnId],
        color: Option<ColumnId>,
    ) -> Result<WindowId, ProjectError> {
        for y in axes.iter().chain(&color) {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        match template {
            Template::Parallel if axes.len() < 2 => return Err(ProjectError::ParallelNeedsTwoColumns(axes.len())),
            Template::ParallelIndex if axes.len() < 3 => {
                return Err(ProjectError::ParallelIndexNeedsThreeColumns(axes.len()));
            }
            Template::ParallelIndex if color.is_none() => return Err(ProjectError::ParallelIndexNeedsGroup),
            Template::Parallel | Template::ParallelIndex => {}
            _ => return self.plot_columns(template, axes),
        }
        let long_name = axes.first().and_then(|y| self.locate(*y)).map(|(b, _, _)| b.title_for_graph());
        let graph = self.add_graph(template);
        for y in axes {
            self.add_plot(graph, 0, *y)?;
        }
        if let Some(g) = self.graph_mut(graph) {
            if let Some(l) = g.layers.first_mut() {
                l.apply_palette();
                for p in &mut l.plots {
                    p.parallel.color = color;
                }
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }

    /// Turns the plots of a new graph into a lollipop plot: vertical drop lines to the next data
    /// plot and, as Origin's template exchanges X and Y, each plot's X and Y columns traded (with
    /// the two axes) when every plot has an X column. Plots against the row number cannot be
    /// exchanged here (Sheaf's own limit, unmeasured), so those lollipops stand upright.
    fn make_lollipop(&mut self, graph: WindowId, ys: &[ColumnId]) {
        let labels: Vec<String> =
            ys.iter().map(|y| self.column(*y).map(|c| c.display_name().to_string()).unwrap_or_default()).collect();
        let Some(l) = self.graph_mut(graph).and_then(|g| g.layers.first_mut()) else { return };
        for p in &mut l.plots {
            p.decor.drop.vertical.on = true;
            p.decor.drop.vertical.target = crate::graph::DropTarget::NextDataPlot;
        }
        if l.plots.iter().all(|p| p.x.is_some()) {
            std::mem::swap(&mut l.x_axis, &mut l.y_axis);
            for (p, name) in l.plots.iter_mut().zip(labels) {
                if let Some(x) = p.x {
                    // The plot keeps the name of its Y column in the legend.
                    p.label = Some(name);
                    p.x = Some(p.y);
                    p.y = x;
                }
                std::mem::swap(&mut p.y_error, &mut p.x_error);
            }
        }
    }

    /// Creates a graph of `template` with the `left` columns on its left Y axis and the `right`
    /// columns on a right Y axis (added with [`Project::add_right_y_layer`] unless the template
    /// already has one), and returns its id. The long name comes from the first column's workbook.
    pub fn plot_columns_double_y(
        &mut self,
        template: Template,
        left: &[ColumnId],
        right: &[ColumnId],
    ) -> Result<WindowId, ProjectError> {
        if template.multi_y().is_some() {
            return Err(ProjectError::MultiYNoRightAxis(template.name()));
        }
        if template == Template::Lollipop {
            return Err(ProjectError::LollipopNoRightAxis);
        }
        if template.is_violin() {
            return Err(ProjectError::ViolinNoRightAxis);
        }
        if template == Template::Ridgeline {
            return Err(ProjectError::RidgelineNoRightAxis);
        }
        if template == Template::Hexbin {
            return Err(ProjectError::HexbinNoRightAxis);
        }
        if template.is_parallel() {
            return Err(ProjectError::ParallelNoRightAxis);
        }
        if template == Template::Treemap {
            return Err(ProjectError::TreemapNoRightAxis);
        }
        if template == Template::Sunburst {
            return Err(ProjectError::SunburstNoRightAxis);
        }
        if matches!(template, Template::GroupedColumns | Template::GroupedBox) {
            return Err(ProjectError::IndexedNoRightAxis);
        }
        if template == Template::IntervalPlot {
            return Err(ProjectError::IntervalNoRightAxis);
        }
        if template == Template::Pareto {
            return Err(ProjectError::ParetoNoRightAxis);
        }
        if template == Template::StackedHistograms {
            return Err(ProjectError::StackedHistogramsNoRightAxis);
        }
        for y in left.iter().chain(right) {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        let first = left.first().or(right.first());
        let long_name = first.and_then(|y| self.locate(*y)).map(|(b, _, _)| b.title_for_graph());
        let graph = self.add_graph(template);
        let right_layer = self.add_right_y_layer(graph, 0)?;
        for y in left {
            self.add_plot(graph, 0, *y)?;
        }
        for y in right {
            self.add_plot(graph, right_layer, *y)?;
        }
        if let Some(g) = self.graph_mut(graph) {
            for layer in [0, right_layer] {
                g.layers[layer].apply_palette();
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }

    /// Creates one of the 3Ys / 4Ys graphs (graphing spec 21.5): Y column *i* (from 0) goes into layer
    /// *i* mod the number of layers, so that with fewer columns than layers the last layers stay empty,
    /// as in Origin. At least one column is needed.
    pub fn plot_multi_y_template(&mut self, kind: MultiYKind, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        if ys.is_empty() {
            return Err(MultiYError::NoColumns.into());
        }
        let n = kind.layers();
        let assignment: Vec<usize> = (0..ys.len()).map(|i| i % n).collect();
        let id = self.plot_multi_y_plan(&kind.plan(), Template::Line, ys, &assignment)?;
        kind.apply_legend_defaults(self.graph_mut(id).expect("new multi-Y graph"));
        Ok(id)
    }

    /// Creates a graph from Plot > Multi-Y > Multiple Y Axes... : the columns in order fill the layers with
    /// the numbers of plots of `options` (one each when none are given).
    pub fn plot_multi_y(&mut self, options: &MultiYOptions, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        let per_layer = options.plots_per_layer(ys.len())?;
        let plan = options.plan(per_layer.len())?;
        let assignment: Vec<usize> =
            per_layer.iter().enumerate().flat_map(|(l, &k)| std::iter::repeat_n(l, k)).collect();
        self.plot_multi_y_plan(&plan, options.plot_type.template(), ys, &assignment)
    }

    fn plot_multi_y_plan(
        &mut self,
        plan: &MultiYPlan,
        base: Template,
        ys: &[ColumnId],
        assignment: &[usize],
    ) -> Result<WindowId, ProjectError> {
        for y in ys {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        let long_name = ys.first().and_then(|y| self.locate(*y)).map(|(b, _, _)| b.long_name.clone());
        let id = WindowId(self.fresh_id());
        let name = self.unique_name("Graph");
        self.windows.push(Window::Graph(plan.graph(base, id, &name)));
        for (y, &layer) in ys.iter().zip(assignment) {
            self.add_plot(id, layer, *y)?;
        }
        if let Some(g) = self.graph_mut(id) {
            for l in &mut g.layers {
                l.apply_palette();
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(id)
    }

    /// Gives layer `parent` of a graph a right Y axis and returns the index of the layer that
    /// carries it. The axis belongs to the panel `parent` is drawn in (the first layer of
    /// [`Graph::same_frame`]). When that panel already has such a layer, that layer is returned and
    /// nothing changes. Otherwise a new layer (see [`right_y_layer`]) is appended, linked to the
    /// panel's root with the panel's X scale and covering the panel; the panel then no longer draws
    /// a line or ticks on its right edge, and its legend lists the plots of both layers.
    ///
    /// A panel of a multi-panel graph whose X scale is not shared with the first panel cannot get
    /// a right Y axis, because a link always goes to a root layer.
    pub fn add_right_y_layer(&mut self, graph: WindowId, parent: usize) -> Result<usize, ProjectError> {
        self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?.add_right_y_layer(parent)
    }

    /// Creates a multi-panel graph of `template` (graphing spec section 15.4) and returns its id.
    ///
    /// Each group of Y columns goes into one panel, in order; with more groups than cells, group g
    /// goes into panel `g mod cells`, as Origin's panel templates deal the columns out. Every panel
    /// is a copy of the template's layer, placed by `layout`, linked to the first panel when the
    /// layout links an axis, and with the labels at joints without a gap hidden. Each panel gets
    /// the template's palette from its first color, as Origin starts the colors again in every
    /// layer. The page follows `layout.page`; the long name comes from the first column's workbook.
    pub fn plot_panels(
        &mut self,
        template: Template,
        groups: &[Vec<ColumnId>],
        layout: &PanelLayout,
    ) -> Result<WindowId, ProjectError> {
        let cells = layout.frames()?.len();
        if groups.is_empty() {
            return Err(PanelError::NoGroups.into());
        }
        for y in groups.iter().flatten() {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        let mut g = template.new_graph(WindowId(0), "");
        if g.layers.len() != 1 {
            return Err(PanelError::Template(template.name()).into());
        }
        let base = g.layers[0].clone();
        let count = groups.len().min(cells);
        let first = groups.iter().flatten().next();
        let long_name = first.and_then(|y| self.locate(*y)).map(|(b, _, _)| b.title_for_graph());
        let (w, h) = (g.page.width_pt, g.page.height_pt);
        match layout.page {
            PanelPage::Keep => {}
            PanelPage::Upright => {
                if w > h {
                    (g.page.width_pt, g.page.height_pt) = (h, w);
                }
            }
            PanelPage::Tiled => {
                g.page.width_pt = w * layout.cols as f64;
                g.page.height_pt = h * layout.rows as f64;
            }
        }
        panel_layers(&mut g, &base, count, layout)?;
        let graph = WindowId(self.fresh_id());
        g.id = graph;
        g.name = self.unique_name("Graph");
        self.windows.push(Window::Graph(g));
        for (k, group) in groups.iter().enumerate() {
            for y in group {
                self.add_plot(graph, k % cells, *y)?;
            }
        }
        if let Some(g) = self.graph_mut(graph) {
            for layer in &mut g.layers {
                layer.apply_palette();
            }
            if let Some(name) = long_name {
                g.long_name = name;
            }
        }
        Ok(graph)
    }

    /// Stacked Histograms (graphing spec 13.16): one histogram panel for each Y column that holds numbers, from the
    /// top in the order of `ys`, linked to share one X scale, and drawn with the same bins in every panel: the
    /// bins the usual rule gives for all the values together (Sheaf's choice, not measured in Origin). Each
    /// panel has its own count axis. Columns without numbers are left out; with none left it is an error.
    pub fn plot_stacked_histograms(&mut self, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        use crate::graph::HistogramStyle;
        use crate::panels::{PanelLayout, PanelOrder};
        let columns: Vec<ColumnId> = ys.iter().copied().filter(|y| self.numeric(*y).is_some()).collect();
        // The bins cover the values each panel's histogram uses: the rows that show (passing the sheet's filter,
        // and not masked while Hide Masked Points is on).
        let all: Vec<f64> = columns.iter().flat_map(|y| self.shown_values(*y)).filter(|v| v.is_finite()).collect();
        let opts = sheaf_calc::freq::FreqOptions {
            min: None,
            max: None,
            size: sheaf_calc::freq::BinSize::Auto,
            below: false,
            above: false,
        };
        let freq = sheaf_calc::freq::freq_counts(&all, &opts).ok_or(ProjectError::StackedHistogramsNeedNumbers)?;
        let end = freq.rows.last().map_or(freq.bin_start + freq.bin_size, |r| r.end);
        let style = HistogramStyle { bin_size: Some(freq.bin_size), begin: Some(freq.bin_start), end: Some(end) };
        let groups: Vec<Vec<ColumnId>> = columns.iter().map(|y| vec![*y]).collect();
        let mut layout = PanelLayout::stack(groups.len());
        layout.order = PanelOrder::TopDown;
        let graph = self.plot_panels(Template::StackedHistograms, &groups, &layout)?;
        if let Some(g) = self.graph_mut(graph) {
            for (k, layer) in g.layers.iter_mut().enumerate() {
                if !layer.palette.colors.is_empty() {
                    let n = layer.palette.colors.len();
                    layer.palette.colors.rotate_left(k % n);
                    layer.apply_palette();
                }
                for plot in &mut layer.plots {
                    plot.hist = style;
                }
            }
        }
        Ok(graph)
    }

    /// Moves plot `index` of layer `from` to the end of layer `to` (for example from the left Y
    /// axis to the right one). Automatic colors are handed out again in both layers; a color
    /// chosen by hand stays.
    pub fn move_plot_to_layer(
        &mut self,
        graph: WindowId,
        from: usize,
        index: usize,
        to: usize,
    ) -> Result<(), ProjectError> {
        self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?.move_plot_to_layer(from, index, to)
    }
}

impl Graph {
    /// [`Project::add_right_y_layer`] on the graph itself (Plot Details edits a copy of it).
    pub fn add_right_y_layer(&mut self, parent: usize) -> Result<usize, ProjectError> {
        let g = self;
        if parent >= g.layers.len() {
            return Err(ProjectError::NoSuchLayer(parent));
        }
        let panel = g.same_frame(parent)[0];
        if let Some(existing) = g.right_y_member(panel) {
            return Ok(existing);
        }
        let root = g.root_of(panel);
        if g.valid_link(panel).is_some_and(|l| l.x != crate::graph::ScaleLink::Straight) {
            return Err(PanelError::RightAxis.into());
        }
        let mut layer = right_y_layer(&g.layers[panel], root);
        layer.frame = g.place(panel);
        let p = &mut g.layers[panel];
        p.y_axis.show_opposite = false;
        p.y_axis.opposite_ticks = false;
        p.legend.scope = LegendScope::Graph;
        g.layers.push(layer);
        Ok(g.layers.len() - 1)
    }

    /// [`Project::move_plot_to_layer`] on the graph itself.
    pub fn move_plot_to_layer(&mut self, from: usize, index: usize, to: usize) -> Result<(), ProjectError> {
        let g = self;
        for layer in [from, to] {
            if layer >= g.layers.len() {
                return Err(ProjectError::NoSuchLayer(layer));
            }
        }
        if index >= g.layers[from].plots.len() {
            return Err(ProjectError::NoSuchPlot(index));
        }
        if from == to {
            return Ok(());
        }
        let plot = g.layers[from].plots.remove(index);
        // The curve's stack settings (its own offset, or the curve it follows) go with it, unless
        // another curve of the same column stays behind.
        let column = plot.y;
        if !g.layers[from].plots.iter().any(|p| p.y == column)
            && let Some(i) = g.layers[from].plot_offsets.iter().position(|o| o.column == column)
        {
            let settings = g.layers[from].plot_offsets.remove(i);
            *g.layers[to].plot_offset_mut(column) = settings;
        }
        // Its slope annotations go with it, unless another plot left behind is also theirs.
        let (stay, go): (Vec<_>, Vec<_>) = std::mem::take(&mut g.layers[from].slopes)
            .into_iter()
            .partition(|s| !s.matches(&plot) || g.layers[from].plots.iter().any(|p| s.matches(p)));
        g.layers[from].slopes = stay;
        g.layers[to].slopes.extend(go);
        g.layers[to].plots.push(plot);
        g.layers[from].recolor_automatic();
        g.layers[to].recolor_automatic();
        Ok(())
    }

    /// Puts plot `index` of layer `layer` on the right Y axis of its panel (`right`) or on the
    /// left one. The right Y layer is added when the panel has none; the left axis belongs to the
    /// panel's first layer. A plot already on that axis stays where it is. Returns the plot's
    /// layer and index after the move. A layer left without plots is kept, as Origin keeps it.
    pub fn set_plot_y_axis(&mut self, layer: usize, index: usize, right: bool) -> Result<(usize, usize), ProjectError> {
        if layer >= self.layers.len() {
            return Err(ProjectError::NoSuchLayer(layer));
        }
        if index >= self.layers[layer].plots.len() {
            return Err(ProjectError::NoSuchPlot(index));
        }
        let to = if right { self.add_right_y_layer(layer)? } else { self.same_frame(layer)[0] };
        if to == layer {
            return Ok((layer, index));
        }
        self.move_plot_to_layer(layer, index, to)?;
        Ok((to, self.layers[to].plots.len() - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Project, WindowId, ColumnId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("demo");
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 4.0, 9.0])).unwrap();
        (p, book, x, y)
    }

    #[test]
    fn a_plot_is_named_as_in_origins_plot_details_tree() {
        let (mut p, book, x, y) = sample();
        let g = p.plot_columns(crate::template::Template::Line, &[y]).unwrap();
        let sheet = p.workbook(book).unwrap().sheets[0].name.clone();
        let plot = p.graph(g).unwrap().layers[0].plots[0].clone();
        assert_eq!(plot.x, Some(x));
        assert_eq!(p.plot_data_name(&plot), format!("[Book1]{sheet}! A(X), B(Y) [1*:3*]"));
        let mut part = plot;
        part.rows = Some((1, 3));
        assert_eq!(p.plot_data_name(&part), format!("[Book1]{sheet}! A(X), B(Y) [2:3]"));
    }

    #[test]
    fn a_lollipop_is_refused_a_right_y_axis() {
        let (mut p, _, _, y) = sample();
        let graphs = p.graphs().count();
        let r = p.plot_columns_double_y(crate::template::Template::Lollipop, &[y], &[y]);
        assert!(matches!(r, Err(ProjectError::LollipopNoRightAxis)));
        assert_eq!(p.graphs().count(), graphs, "no graph is left behind");
    }

    #[test]
    fn a_violin_is_refused_a_right_y_axis() {
        let (mut p, _, _, y) = sample();
        let graphs = p.graphs().count();
        let r = p.plot_columns_double_y(crate::template::Template::Violin, &[y], &[y]);
        assert!(matches!(r, Err(ProjectError::ViolinNoRightAxis)));
        assert_eq!(p.graphs().count(), graphs, "no graph is left behind");
    }

    #[test]
    fn names_and_ids_are_unique() {
        let (mut p, book, x, y) = sample();
        assert_eq!(p.workbook(book).unwrap().name, "Book1");
        let book2 = p.add_workbook("");
        assert_eq!(p.workbook(book2).unwrap().name, "Book2");
        assert_ne!(x, y);
        let g = p.add_graph(Template::Line);
        assert_eq!(p.graph(g).unwrap().name, "Graph1");
        p.remove_window(book2);
        assert_eq!(p.unique_name("Book"), "Book2");
    }

    #[test]
    fn adding_a_plot_keeps_the_colors_the_user_chose() {
        let (mut p, book, _, y) = sample();
        let y2 = p
            .add_column(book, 0, crate::column::Designation::Y, crate::column::ColumnData::numeric(vec![1.0]))
            .unwrap();
        let g = p.add_graph(Template::Line);
        p.add_plot(g, 0, y).unwrap();
        let chosen = crate::Color::rgb(1, 2, 3);
        {
            // What the graph settings window does when the user picks a color.
            let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
            plot.line.color = chosen;
            plot.auto_color = false;
        }
        p.add_plot(g, 0, y2).unwrap();
        let plots = &p.graph(g).unwrap().layers[0].plots;
        assert_eq!(plots[0].line.color, chosen);
        assert_ne!(plots[1].line.color, chosen);
    }

    /// With a gradient palette (the cyclic voltammetry template) every automatic color moves
    /// along when a plot is added, so scans added one at a time get distinct colors; a color
    /// chosen by hand stays. Adding only the newest plot's color gave two scans the same color.
    #[test]
    fn adding_plots_one_at_a_time_keeps_a_gradient_distinct() {
        let (mut p, book, _, y) = sample();
        let mut ys = vec![y];
        for _ in 0..3 {
            let c = crate::column::ColumnData::numeric(vec![1.0]);
            ys.push(p.add_column(book, 0, crate::column::Designation::Y, c).unwrap());
        }
        let g = p.add_graph(Template::CyclicVoltammetry);
        assert_eq!(p.graph(g).unwrap().layers[0].palette.mode, crate::graph::PaletteMode::Spread);
        for &c in &ys[..3] {
            p.add_plot(g, 0, c).unwrap();
        }
        let colors = |p: &Project| -> Vec<crate::Color> {
            p.graph(g).unwrap().layers[0].plots.iter().map(|q| q.line.color).collect()
        };
        let three = colors(&p);
        assert!(three[0] != three[1] && three[1] != three[2] && three[0] != three[2], "{three:?}");
        let chosen = crate::Color::rgb(200, 0, 0);
        {
            let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[1];
            plot.line.color = chosen;
            plot.auto_color = false;
        }
        p.add_plot(g, 0, ys[3]).unwrap();
        let four = colors(&p);
        assert_eq!(four[1], chosen);
        assert!(four[0] != four[2] && four[2] != four[3] && four[0] != four[3], "{four:?}");
    }

    #[test]
    fn add_plot_finds_x_from_designations() {
        let (mut p, _, x, y) = sample();
        let g = p.add_graph(Template::Line);
        p.add_plot(g, 0, y).unwrap();
        let plot = &p.graph(g).unwrap().layers[0].plots[0];
        assert_eq!(plot.x, Some(x));
        assert_eq!(plot.y, y);
        assert_eq!(p.plot_label(plot), "B");
        assert_eq!(p.numeric(y).unwrap(), &[1.0, 4.0, 9.0]);
    }

    #[test]
    fn default_plot_columns_keep_one_unit() {
        let (mut p, book, _, y) = sample();
        // Same (empty) unit everywhere: every Y column is plotted.
        let y2 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 3])).unwrap();
        assert_eq!(p.default_plot_columns(book, 0, Template::Line), vec![y, y2]);
        // Mixed units without mA: the unit of the first Y column wins.
        p.column_mut(y2).unwrap().units = "C".into();
        assert_eq!(p.default_plot_columns(book, 0, Template::Line), vec![y]);
        // Current in A and in mA plus charge: only the mA column.
        p.column_mut(y).unwrap().units = "A".into();
        let ma = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 3])).unwrap();
        p.column_mut(ma).unwrap().units = "mA".into();
        assert_eq!(p.default_plot_columns(book, 0, Template::Line), vec![ma]);
        // Text columns are never plotted.
        let t = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["a".into()])).unwrap();
        assert!(!p.default_plot_columns(book, 0, Template::Line).contains(&t));
    }

    #[test]
    fn window_short_names_follow_origins_rule_and_formulas_follow_a_rename() {
        // Origin 2016 `win -r` (workbook-model.md 1)
        assert_eq!(clean_window_name("ABCDEFGHIJKLMN"), "ABCDEFGHIJKLM");
        assert_eq!(clean_window_name("abcdefghijklmnopqrstuvwxyz"), "abcdefghijklm");
        for (typed, got) in [
            ("a b", "ab"),
            ("a_b", "ab"),
            ("a-b", "ab"),
            ("a.b", "ab"),
            ("x/y", "xy"),
            ("ab(c)", "abc"),
            ("Ab#1", "Ab1"),
        ] {
            assert_eq!(clean_window_name(typed), got, "{typed}");
        }
        assert_eq!(clean_window_name("1abc"), "A1abc");
        assert_eq!(clean_window_name("123"), "A123");
        assert_eq!(clean_window_name("12345678901234"), "A123456789012");
        assert_eq!(clean_window_name("___"), "A");
        let mut p = Project::new();
        let a = p.add_workbook("Data");
        let b = p.add_workbook("Other");
        let col = p.add_column(b, 0, crate::Designation::Y, crate::ColumnData::numeric(vec![1.0])).unwrap();
        p.column_mut(col).unwrap().formula = Some("[Book1]Sheet1!col(A) * 2 + [book1]Sheet1!col(B)".into());
        let first = p.window(a).unwrap().name().to_string();
        assert_eq!(first, "Book1");
        assert_eq!(p.rename_window(a, "Raw data 2026"), Ok("Rawdata2026".to_string()));
        assert_eq!(
            p.column(col).unwrap().formula.as_deref(),
            Some("[Rawdata2026]Sheet1!col(A) * 2 + [Rawdata2026]Sheet1!col(B)")
        );
        let taken = p.window(b).unwrap().name().to_string();
        assert!(p.rename_window(a, &taken.to_lowercase()).is_err(), "another window's name, in any case");
        assert_eq!(p.window(a).unwrap().name(), "Rawdata2026");
    }

    #[test]
    fn adopt_named_workbook_keeps_a_free_name() {
        let (mut p, _, _, _) = sample();
        let a = p.adopt_named_workbook(Workbook::new(WindowId(0), "Rates"));
        let b = p.adopt_named_workbook(Workbook::new(WindowId(0), "rates"));
        let c = p.adopt_named_workbook(Workbook::new(WindowId(0), ""));
        assert_eq!(p.workbook(a).unwrap().name, "Rates");
        assert_eq!(p.workbook(b).unwrap().name, "Book2", "a taken name (ignoring case) is replaced");
        assert_eq!(p.workbook(c).unwrap().name, "Book3");
    }

    #[test]
    fn adopt_workbook_assigns_fresh_ids() {
        let (mut p, _, _, y) = sample();
        let mut book = Workbook::new(WindowId(0), "whatever");
        book.sheets[0].columns.push(Column::new(ColumnId(0), "A", Designation::X, ColumnData::numeric(vec![1.0])));
        book.sheets[0].columns.push(Column::new(ColumnId(0), "B", Designation::Y, ColumnData::numeric(vec![2.0])));
        let id = p.adopt_workbook(book);
        let b = p.workbook(id).unwrap();
        assert_eq!(b.name, "Book2");
        let ids: Vec<_> = b.sheets[0].columns.iter().map(|c| c.id).collect();
        assert!(ids[0] != ids[1] && ids[0].0 > y.0);
    }

    #[test]
    fn json_round_trip_keeps_everything_but_data() {
        let (mut p, _, _, y) = sample();
        let g = p.add_graph(Template::CyclicVoltammetry);
        p.add_plot(g, 0, y).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let mut back: Project = serde_json::from_str(&json).unwrap();
        back.repair_ids();
        assert_eq!(back.graph(g), p.graph(g));
        assert_eq!(back.column(y).unwrap().short_name, "B");
        assert!(back.column(y).unwrap().is_empty(), "data is stored outside the JSON");
        assert_eq!(back.new_column_id().0, p.new_column_id().0);
    }

    /// A sheet with X and the Y columns B, C, D, E.
    fn four_ys() -> (Project, Vec<ColumnId>) {
        let (mut p, book, _, y) = sample();
        let mut ys = vec![y];
        for k in 0..3 {
            let c = ColumnData::numeric(vec![f64::from(k); 3]);
            ys.push(p.add_column(book, 0, Designation::Y, c).unwrap());
        }
        (p, ys)
    }

    fn plot_ys(p: &Project, g: WindowId, layer: usize) -> Vec<ColumnId> {
        p.graph(g).unwrap().layers[layer].plots.iter().map(|q| q.y).collect()
    }

    #[test]
    fn add_right_y_layer_adds_one_linked_layer_and_reuses_it() {
        let (mut p, _, _, y) = sample();
        let g = p.add_graph(Template::Line);
        p.add_plot(g, 0, y).unwrap();
        assert_eq!(p.add_right_y_layer(g, 0), Ok(1));
        assert_eq!(p.add_right_y_layer(g, 0), Ok(1), "no third layer");
        assert_eq!(p.add_right_y_layer(g, 1), Ok(1), "asking from the right layer finds itself");
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers.len(), 2);
        assert_eq!(graph.link_parent(1), Some(0));
        assert_eq!(graph.layers[0].legend.scope, LegendScope::Graph);
        assert!(graph.layers[1].plots.is_empty());
        assert_eq!(p.add_right_y_layer(g, 5), Err(ProjectError::NoSuchLayer(5)));
        // The double-Y template already has its right layer.
        let d = p.add_graph(Template::DoubleY);
        assert_eq!(p.add_right_y_layer(d, 0), Ok(1));
        assert_eq!(p.graph(d).unwrap().layers.len(), 2);
    }

    #[test]
    fn moving_a_plot_takes_its_slope_annotations_along() {
        let (mut p, ys) = four_ys();
        let g = p.plot_columns(Template::Line, &ys[..2]).unwrap();
        let right = p.add_right_y_layer(g, 0).unwrap();
        {
            let l = &mut p.graph_mut(g).unwrap().layers[0];
            let (a, b) = (l.plots[0].clone(), l.plots[1].clone());
            l.slopes.push(crate::graph::SlopeAnnotation::for_plot(&a, 0, 2));
            l.slopes.push(crate::graph::SlopeAnnotation::for_plot(&b, 1, 2));
        }
        p.move_plot_to_layer(g, 0, 0, right).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers[right].slopes.len(), 1);
        assert_eq!(graph.layers[right].slopes[0].plot, ys[0]);
        assert_eq!(graph.layers[0].slopes.len(), 1);
        assert_eq!(graph.layers[0].slopes[0].plot, ys[1]);
    }

    #[test]
    fn multi_y_templates_deal_the_columns_out_round_and_round() {
        let (mut p, mut ys) = four_ys();
        let book = p.workbooks().next().unwrap().id;
        ys.push(p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![9.0; 3])).unwrap());
        let g = p.plot_columns(Template::Multi3YYy, &ys).unwrap();
        assert_eq!(p.graph(g).unwrap().layers.len(), 3);
        assert_eq!(plot_ys(&p, g, 0), vec![ys[0], ys[3]]);
        assert_eq!(plot_ys(&p, g, 1), vec![ys[1], ys[4]]);
        assert_eq!(plot_ys(&p, g, 2), vec![ys[2]]);
        // The first plot of a layer has the layer's template colour; the second the next one of its list.
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers[1].plots[0].line.color, crate::Color::rgb(255, 0, 0));
        assert_ne!(graph.layers[1].plots[1].line.color, crate::Color::rgb(255, 0, 0));
        let g = p.plot_columns(Template::Multi4YyYy, &ys[..2]).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers.len(), 4, "all layers are made whatever the number of columns");
        assert!(graph.layers[2].plots.is_empty() && graph.layers[3].plots.is_empty());
    }

    #[test]
    fn multi_y_needs_a_column_and_no_right_axis_split() {
        let (mut p, ys) = four_ys();
        assert_eq!(p.plot_columns(Template::Multi4YYyy, &[]), Err(MultiYError::NoColumns.into()));
        assert_eq!(
            p.plot_columns_double_y(Template::Multi3YYY, &ys[..1], &ys[1..2]),
            Err(ProjectError::MultiYNoRightAxis("3ys-y-y-y"))
        );
    }

    #[test]
    fn multiple_y_axes_fills_the_layers_with_the_given_numbers() {
        let (mut p, ys) = four_ys();
        let o = MultiYOptions { per_layer: vec![2, 1, 1], ..MultiYOptions::default() };
        let g = p.plot_multi_y(&o, &ys).unwrap();
        assert_eq!(plot_ys(&p, g, 0), vec![ys[0], ys[1]]);
        assert_eq!(plot_ys(&p, g, 2), vec![ys[3]]);
        let bad = MultiYOptions { per_layer: vec![2, 1], ..MultiYOptions::default() };
        assert_eq!(p.plot_multi_y(&bad, &ys), Err(MultiYError::PlotsPerLayer { sum: 3, columns: 4 }.into()));
    }

    #[test]
    fn moving_a_plot_recolors_both_layers() {
        let (mut p, ys) = four_ys();
        let g = p.plot_columns(Template::Line, &ys[..3]).unwrap();
        let right = p.add_right_y_layer(g, 0).unwrap();
        p.move_plot_to_layer(g, 0, 1, right).unwrap();
        assert_eq!(plot_ys(&p, g, 0), vec![ys[0], ys[2]]);
        assert_eq!(plot_ys(&p, g, right), vec![ys[1]]);
        let graph = p.graph(g).unwrap();
        // Left: the first two palette colors; right: its own first color.
        assert_eq!(graph.layers[0].plots[1].line.color, graph.layers[0].palette.colors[1]);
        assert_eq!(graph.layers[right].plots[0].line.color, graph.layers[right].palette.colors[0]);
        assert_eq!(p.move_plot_to_layer(g, 0, 7, right), Err(ProjectError::NoSuchPlot(7)));
        assert_eq!(p.move_plot_to_layer(g, 0, 0, 9), Err(ProjectError::NoSuchLayer(9)));
    }

    #[test]
    fn set_plot_y_axis_makes_the_right_layer_and_moves_back_leaving_it_empty() {
        let (mut p, ys) = four_ys();
        let g = p.plot_columns(Template::Line, &ys[..3]).unwrap();
        let graph = p.graph_mut(g).unwrap();
        assert_eq!(graph.set_plot_y_axis(0, 1, false), Ok((0, 1)), "already left");
        assert_eq!(graph.layers.len(), 1);
        assert_eq!(graph.set_plot_y_axis(0, 1, true), Ok((1, 0)));
        assert_eq!(graph.layers.len(), 2);
        assert_eq!(graph.right_y_member(0), Some(1));
        assert_eq!(graph.set_plot_y_axis(1, 0, true), Ok((1, 0)), "already right");
        assert_eq!(graph.set_plot_y_axis(0, 1, true), Ok((1, 1)), "the second Y column joins it");
        assert_eq!(graph.layers.len(), 2, "the right layer is reused");
        assert_eq!(graph.set_plot_y_axis(1, 0, false), Ok((0, 1)));
        assert_eq!(graph.set_plot_y_axis(1, 0, false), Ok((0, 2)));
        assert_eq!(graph.layers.len(), 2, "the empty right layer stays");
        assert!(graph.layers[1].plots.is_empty());
        assert_eq!(graph.set_plot_y_axis(0, 9, true), Err(ProjectError::NoSuchPlot(9)));
        assert_eq!(graph.set_plot_y_axis(5, 0, true), Err(ProjectError::NoSuchLayer(5)));
    }

    #[test]
    fn a_moved_plot_keeps_its_stack_settings() {
        let (mut p, ys) = four_ys();
        let g = p.plot_columns(Template::Line, &ys[..2]).unwrap();
        let right = p.add_right_y_layer(g, 0).unwrap();
        p.graph_mut(g).unwrap().layers[0].plot_offset_mut(ys[1]).y = 7.0;
        p.move_plot_to_layer(g, 0, 1, right).unwrap();
        let graph = p.graph(g).unwrap();
        assert!(graph.layers[0].plot_offset(ys[1]).is_none());
        assert_eq!(graph.layers[right].plot_offset(ys[1]).map(|o| o.y), Some(7.0));
    }

    #[test]
    fn double_y_plots_alternate_between_the_axes() {
        let (mut p, ys) = four_ys();
        let book = p.workbooks().next().unwrap().id;
        // Mixed units: a double-Y graph still takes every Y column by default.
        p.column_mut(ys[1]).unwrap().units = "C".into();
        assert_eq!(p.default_plot_columns(book, 0, Template::DoubleY), ys);
        let g = p.plot_columns(Template::DoubleY, &ys).unwrap();
        assert_eq!(plot_ys(&p, g, 0), vec![ys[0], ys[2]]);
        assert_eq!(plot_ys(&p, g, 1), vec![ys[1], ys[3]]);
        assert_eq!(p.graph(g).unwrap().long_name, "demo");
        // The same as naming the axes explicitly on the line template.
        let h = p.plot_columns_double_y(Template::Line, &[ys[0], ys[2]], &[ys[1], ys[3]]).unwrap();
        let (a, b) = (p.graph(g).unwrap(), p.graph(h).unwrap());
        for layer in 0..2 {
            assert_eq!(a.layers[layer].plots, b.layers[layer].plots);
        }
        let missing = ColumnId(999);
        assert_eq!(p.plot_columns_double_y(Template::Line, &[missing], &[]), Err(ProjectError::NoSuchColumn(missing)));
        assert_eq!(
            p.plot_columns_double_y(Template::StackedHistograms, &[missing], &[missing]),
            Err(ProjectError::StackedHistogramsNoRightAxis)
        );
    }

    /// A sheet with X and the Y columns B to G.
    fn six_ys() -> (Project, Vec<ColumnId>) {
        let (mut p, mut ys) = four_ys();
        let book = p.workbooks().next().unwrap().id;
        for k in 0..2 {
            ys.push(p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![f64::from(k); 3])).unwrap());
        }
        (p, ys)
    }

    fn singles(ys: &[ColumnId]) -> Vec<Vec<ColumnId>> {
        ys.iter().map(|y| vec![*y]).collect()
    }

    #[test]
    fn a_stack_links_every_panel_to_the_bottom_one_and_hides_the_inner_x_labels() {
        use crate::graph::{LayerLink, ScaleLink};
        let (mut p, ys) = four_ys();
        let g = p.plot_panels(Template::Line, &singles(&ys[..3]), &PanelLayout::stack(3)).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers.len(), 3);
        // The Line template's page (6.4 x 4.8 inch) turned upright.
        let line = Template::Line.new_graph(WindowId(0), "").page;
        assert_eq!((graph.page.width_pt, graph.page.height_pt), (line.height_pt, line.width_pt));
        assert_eq!(graph.layers[0].link, None);
        for k in [1, 2] {
            assert_eq!(graph.layers[k].link, Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None }));
            assert_eq!(graph.link_parent(k), Some(0));
        }
        for (k, y) in ys[..3].iter().enumerate() {
            assert_eq!(plot_ys(&p, g, k), vec![*y]);
        }
        let x_shown: Vec<bool> =
            graph.layers.iter().map(|l| l.x_axis.show_tick_labels && l.x_axis.show_title).collect();
        assert_eq!(x_shown, vec![true, false, false]);
        let end_hidden: Vec<bool> = graph.layers.iter().map(|l| l.y_axis.hide_end_label).collect();
        assert_eq!(end_hidden, vec![true, true, false]);
        // Every panel starts the palette again, as Origin does in every layer.
        let first = graph.layers[0].palette.colors[0];
        assert!(graph.layers.iter().all(|l| l.plots[0].line.color == first));
        assert_eq!(graph.long_name, "demo");
        assert_eq!(graph.panels(), vec![0, 1, 2]);
        // With a gap of 1, every panel shows its X labels and nothing is hidden.
        let gap = PanelLayout { v_gap: 1.0, ..PanelLayout::stack(3) };
        let g = p.plot_panels(Template::Line, &singles(&ys[..3]), &gap).unwrap();
        let graph = p.graph(g).unwrap();
        assert!(graph.layers.iter().all(|l| l.x_axis.show_tick_labels && l.x_axis.show_title));
        assert!(graph.layers.iter().all(|l| !l.y_axis.hide_end_label));
    }

    #[test]
    fn a_cv_stack_keeps_the_cv_style_in_every_panel() {
        use crate::graph::{AutoRange, TickDirection};
        let (mut p, ys) = four_ys();
        let g = p.plot_panels(Template::CyclicVoltammetry, &singles(&ys[..2]), &PanelLayout::stack(2)).unwrap();
        let graph = p.graph(g).unwrap();
        let cv = Template::CyclicVoltammetry.new_graph(WindowId(0), "").page;
        assert_eq!((graph.page.width_pt, graph.page.height_pt), (cv.height_pt, cv.width_pt));
        assert!((graph.page.width_pt - 259.2).abs() < 1e-9 && (graph.page.height_pt - 360.0).abs() < 1e-9);
        for l in &graph.layers {
            assert_eq!(l.y_axis.auto_range, AutoRange::SymmetricZero);
            assert!(l.x_axis.show_opposite && l.y_axis.show_opposite);
            assert_eq!((l.x_axis.major_tick, l.y_axis.major_tick), (TickDirection::In, TickDirection::In));
        }
    }

    #[test]
    fn a_grid_deals_extra_groups_out_in_turn_like_origins_4_panel() {
        let (mut p, ys) = six_ys();
        let layout = PanelLayout::grid(2, 2, &Template::Line.new_graph(WindowId(0), "").layers[0].frame);
        let g = p.plot_panels(Template::Line, &singles(&ys), &layout).unwrap();
        assert_eq!(plot_ys(&p, g, 0), vec![ys[0], ys[4]]);
        assert_eq!(plot_ys(&p, g, 1), vec![ys[1], ys[5]]);
        assert_eq!(plot_ys(&p, g, 2), vec![ys[2]]);
        assert_eq!(plot_ys(&p, g, 3), vec![ys[3]]);
        let graph = p.graph(g).unwrap();
        // Twice the Line template's page in both directions; no links.
        let line = Template::Line.new_graph(WindowId(0), "").page;
        assert_eq!((graph.page.width_pt, graph.page.height_pt), (2.0 * line.width_pt, 2.0 * line.height_pt));
        assert!(graph.layers.iter().all(|l| l.link.is_none() && l.x_axis.show_tick_labels));
        // The second plot of a panel takes the palette's second color.
        let layer = &graph.layers[0];
        assert_eq!(layer.plots[1].line.color, layer.palette.colors[1]);
        // Three groups in a 2 x 2 grid give three panels; the fourth cell stays empty.
        let g = p.plot_panels(Template::Line, &singles(&ys[..3]), &layout).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers.len(), 3);
        let tops: Vec<f64> = graph.layers.iter().map(|l| l.frame.top).collect();
        assert!(tops[0] == tops[1] && tops[2] > tops[0], "{tops:?}");
    }

    #[test]
    fn panel_graphs_need_groups_existing_columns_and_a_one_layer_template() {
        let (mut p, ys) = four_ys();
        let before = p.windows.len();
        let stack = PanelLayout::stack(2);
        assert_eq!(p.plot_panels(Template::Line, &[], &stack), Err(ProjectError::Panels(PanelError::NoGroups)));
        let missing = ColumnId(999);
        assert_eq!(
            p.plot_panels(Template::Line, &[vec![ys[0]], vec![missing]], &stack),
            Err(ProjectError::NoSuchColumn(missing))
        );
        assert_eq!(
            p.plot_panels(Template::DoubleY, &singles(&ys[..2]), &stack),
            Err(ProjectError::Panels(PanelError::Template("double-y")))
        );
        let bad = PanelLayout { margin_left: 60.0, margin_right: 40.0, ..stack };
        assert_eq!(
            p.plot_panels(Template::Line, &singles(&ys[..2]), &bad),
            Err(ProjectError::Panels(PanelError::NoRoom))
        );
        assert_eq!(p.windows.len(), before, "no graph is left behind");
        // A group may hold several columns; an empty group gives an empty panel.
        let g = p.plot_panels(Template::Line, &[vec![ys[0], ys[1]], vec![]], &stack).unwrap();
        assert_eq!(plot_ys(&p, g, 0), vec![ys[0], ys[1]]);
        assert!(plot_ys(&p, g, 1).is_empty());
    }

    #[test]
    fn a_right_y_axis_belongs_to_the_panel_it_is_added_to() {
        let (mut p, ys) = four_ys();
        let g = p.plot_panels(Template::Line, &singles(&ys[..2]), &PanelLayout::stack(2)).unwrap();
        let r = p.add_right_y_layer(g, 1).unwrap();
        assert_eq!(r, 2);
        assert_eq!(p.add_right_y_layer(g, 1), Ok(2), "asked again, the same layer");
        assert_eq!(p.add_right_y_layer(g, 2), Ok(2), "asked from the right layer itself");
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.link_parent(2), Some(0), "links go to the root");
        assert_eq!(graph.layers[2].frame, graph.layers[1].frame, "it covers panel 1");
        assert_eq!(graph.same_frame(1), vec![1, 2]);
        assert_eq!(graph.right_y_member(0), None, "panel 0 has no right axis");
        assert_eq!(graph.layers[1].legend.scope, LegendScope::Graph);
        assert_eq!(graph.layers[0].legend.scope, LegendScope::Layer);
        assert_eq!(graph.panels(), vec![0, 1]);
        // A panel that does not share X with the root cannot get one.
        let side = PanelLayout { link_x: false, link_y: true, ..PanelLayout::stack(2) };
        let g = p.plot_panels(Template::Line, &singles(&ys[..2]), &side).unwrap();
        assert_eq!(p.add_right_y_layer(g, 1), Err(ProjectError::Panels(PanelError::RightAxis)));
        assert_eq!(p.add_right_y_layer(g, 0), Ok(2), "the root can");
    }

    #[test]
    fn json_round_trip_keeps_linked_layers() {
        let (mut p, ys) = four_ys();
        let g = p.plot_columns(Template::DoubleY, &ys).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(back.graph(g), p.graph(g));
        assert_eq!(back.graph(g).unwrap().right_y_member(0), Some(1));
    }

    #[test]
    fn append_brings_the_windows_over_with_fresh_ids_and_graphs_keep_their_data() {
        let mut here = Project::new();
        let b = here.add_workbook("");
        here.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
        let mine = here.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![5.0, 6.0])).unwrap();
        here.plot_columns(Template::Line, &[mine]).unwrap();

        let mut other = Project::new();
        let ob = other.add_workbook("XRD");
        other.add_column(ob, 0, Designation::X, ColumnData::numeric(vec![10.0, 20.0, 30.0])).unwrap();
        let theirs = other.add_column(ob, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        other.plot_columns(Template::Line, &[theirs]).unwrap();

        let before = here.windows.len();
        let (new, dropped) = here.append(other, "XRD project");
        assert_eq!((new.len(), dropped), (2, 0));
        assert_eq!(here.windows.len(), before + 2);
        let names: Vec<&str> = here.windows.iter().map(|w| w.name()).collect();
        let unique: std::collections::HashSet<String> = names.iter().map(|n| n.to_lowercase()).collect();
        assert_eq!(unique.len(), names.len(), "names stay unique: {names:?}");
        let graph = here.graph(new[1]).unwrap();
        let y = graph.layers[0].plots[0].y;
        assert_eq!(here.numeric(y).unwrap(), [1.0, 2.0, 3.0], "the appended graph plots its own column");
        assert_eq!(here.numeric(mine).unwrap(), [5.0, 6.0], "this project's data is untouched");
        assert_eq!(here.root.folders.last().unwrap().name, "XRD project");
        let fresh = here.new_column_id();
        assert!(fresh.0 > y.0, "new ids come after the appended ones");
    }

    #[test]
    fn append_windows_puts_the_windows_in_the_root_folder_without_a_new_folder() {
        let mut here = Project::new();
        here.add_workbook("");
        let mut other = Project::new();
        let ob = other.add_workbook("");
        other.add_column(ob, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        let (ids, dropped) = here.append_windows(other);
        assert_eq!((ids.len(), dropped), (1, 0));
        assert!(here.root.folders.is_empty());
        assert_eq!(here.windows.len(), 2);
        assert_ne!(here.windows[1].name(), here.windows[0].name(), "a clash is renamed");
    }

    #[test]
    fn append_renames_the_books_that_formulas_of_the_appended_windows_read() {
        let mut here = Project::new();
        here.add_workbook("");
        here.add_workbook("");
        let mut other = Project::new();
        let a = other.add_workbook("");
        let b = other.add_workbook("");
        other.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let ca = other.add_column(a, 0, Designation::Y, ColumnData::numeric(vec![0.0, 0.0])).unwrap();
        other.set_column_values(ca, "[Book2]Sheet1!col(A) * 10", None).unwrap();
        let (ids, _) = here.append(other, "x");
        let names: Vec<String> = ids.iter().map(|i| here.window(*i).unwrap().name().to_string()).collect();
        assert_eq!(names, ["Book3", "Book4"]);
        let formula = here
            .windows
            .iter()
            .flat_map(|w| match w {
                Window::Workbook(b) => {
                    b.sheets.iter().flat_map(|s| s.columns.iter()).filter_map(|c| c.formula.clone()).collect()
                }
                Window::Graph(_) => Vec::new(),
            })
            .next();
        assert_eq!(formula.as_deref(), Some("[Book4]Sheet1!col(A) * 10"));
    }

    #[test]
    fn replacing_book_names_in_one_pass_never_changes_a_name_twice() {
        let renames = [("Book1".to_string(), "Book2".to_string()), ("Book2".to_string(), "Book3".to_string())];
        assert_eq!(
            replace_book_references("[book1]S!col(A) + [Book2]S!col(A) + [Other]S!col(A) + [x", &renames),
            "[Book2]S!col(A) + [Book3]S!col(A) + [Other]S!col(A) + [x"
        );
    }

    fn book_with_rule(p: &mut Project) -> (WindowId, ColumnId) {
        let b = p.add_workbook("");
        let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let sheet = &mut p.workbook_mut(b).unwrap().sheets[0];
        let rule = crate::condformat::CondFormat::new(
            sheet,
            crate::condformat::CondRange::whole_columns(vec![c]),
            crate::condformat::Rule::min_max(),
        );
        sheet.cond_formats.push(rule);
        (b, c)
    }

    fn rule_columns(p: &Project, b: WindowId) -> Vec<ColumnId> {
        p.workbook(b).unwrap().sheets[0].cond_formats[0].range.columns.clone()
    }

    #[test]
    fn duplicate_window_and_append_remap_the_rules() {
        let mut p = Project::new();
        let (b, _) = book_with_rule(&mut p);
        let copy = p.duplicate_window(b).unwrap();
        let col = p.workbook(copy).unwrap().sheets[0].columns[0].id;
        assert_eq!(rule_columns(&p, copy), vec![col]);

        let mut other = Project::new();
        let (ob, oc) = book_with_rule(&mut other);
        let (new, _) = p.append(other, "f");
        let _ = (ob, oc);
        let col = p.workbook(new[0]).unwrap().sheets[0].columns[0].id;
        assert_eq!(rule_columns(&p, new[0]), vec![col]);
    }

    #[test]
    fn duplicate_window_and_append_remap_the_notes() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        p.workbook_mut(b).unwrap().sheets[0].set_note(c, 1, "n");
        let copy = p.duplicate_window(b).unwrap();
        let col = p.workbook(copy).unwrap().sheets[0].columns[0].id;
        assert_ne!(col, c);
        assert_eq!(p.workbook(copy).unwrap().sheets[0].note(col, 1), Some("n"));
        assert_eq!(p.workbook(b).unwrap().sheets[0].note(c, 1), Some("n"));

        let mut other = Project::new();
        let ob = other.add_workbook("");
        let oc = other.add_column(ob, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        other.workbook_mut(ob).unwrap().sheets[0].set_note(oc, 0, "o");
        let (new, _) = p.append(other, "f");
        let col = p.workbook(new[0]).unwrap().sheets[0].columns[0].id;
        assert_eq!(p.workbook(new[0]).unwrap().sheets[0].note(col, 0), Some("o"));
    }

    #[test]
    fn duplicate_window_and_append_remap_the_cell_formats() {
        use crate::cellformat::{CellFormat, FormatTarget, LabelKind};
        let bold = CellFormat { bold: Some(true), ..Default::default() };
        let mut p = Project::new();
        let b = p.add_workbook("");
        let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let s = &mut p.workbook_mut(b).unwrap().sheets[0];
        s.apply_cell_format(FormatTarget::cells(vec![c], 1, None), bold.clone());
        s.apply_cell_format(FormatTarget::label_rows(vec![c], vec![LabelKind::Units]), bold.clone());
        let copy = p.duplicate_window(b).unwrap();
        let sheet = &p.workbook(copy).unwrap().sheets[0];
        let col = sheet.columns[0].id;
        assert_ne!(col, c);
        assert!(sheet.cell_format_at(col, 1).bold && !sheet.cell_format_at(col, 0).bold);
        assert!(sheet.format_index().resolve_label(sheet, col, &LabelKind::Units).bold);
        assert!(p.workbook(b).unwrap().sheets[0].cell_format_at(c, 1).bold, "the original keeps its own ids");

        let mut other = Project::new();
        let ob = other.add_workbook("");
        let oc = other.add_column(ob, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        other.workbook_mut(ob).unwrap().sheets[0].apply_cell_format(FormatTarget::whole_columns(vec![oc]), bold);
        let (new, _) = p.append(other, "f");
        let sheet = &p.workbook(new[0]).unwrap().sheets[0];
        assert!(sheet.cell_format_at(sheet.columns[0].id, 0).bold);
    }

    #[test]
    fn append_remaps_the_column_ids_inside_plot_styles() {
        use crate::graph::PlotKind;
        let mut here = Project::new();
        let hb = here.add_workbook("");
        for i in 0..5 {
            here.add_column(hb, 0, Designation::Y, ColumnData::numeric(vec![100.0 + i as f64])).unwrap();
        }

        let mut other = Project::new();
        let ob = other.add_workbook("");
        let mut cols = Vec::new();
        for i in 0..4 {
            let d = ColumnData::numeric(vec![i as f64, i as f64 + 0.5, i as f64 + 1.0]);
            cols.push(other.add_column(ob, 0, Designation::Y, d).unwrap());
        }
        let g = other.plot_columns(Template::Line, &[cols[0]]).unwrap();
        {
            let plot = &mut other.graph_mut(g).unwrap().layers[0].plots[0];
            plot.kind = PlotKind::Treemap;
            plot.treemap.categories = vec![cols[1], cols[2]];
            plot.treemap.order_column = Some(cols[3]);
            plot.sunburst.categories = vec![cols[2], cols[1]];
            plot.parallel.color = Some(cols[3]);
        }
        {
            let l = &mut other.graph_mut(g).unwrap().layers[0];
            l.x_axis.rug.plot = Some(cols[0]);
            l.y_axis.rug_opposite.plot = Some(cols[0]);
            let mut sl = crate::graph::SlopeAnnotation::new(cols[0], 0, 2);
            sl.x = Some(cols[1]);
            l.slopes.push(sl);
        }
        let (new, _) = here.append(other, "f");
        let graph = here.graph(new[1]).unwrap();
        let l = &graph.layers[0];
        let plot = &l.plots[0];
        let data = |id: ColumnId| here.numeric(id).unwrap().to_vec();
        let want = |i: usize| vec![i as f64, i as f64 + 0.5, i as f64 + 1.0];
        assert_eq!(data(plot.y), want(0));
        assert_eq!(plot.treemap.categories.iter().map(|&c| data(c)).collect::<Vec<_>>(), vec![want(1), want(2)]);
        assert_eq!(data(plot.treemap.order_column.unwrap()), want(3));
        assert_eq!(plot.sunburst.categories.iter().map(|&c| data(c)).collect::<Vec<_>>(), vec![want(2), want(1)]);
        assert_eq!(data(plot.parallel.color.unwrap()), want(3));
        assert_eq!(data(l.x_axis.rug.plot.unwrap()), want(0));
        assert_eq!(data(l.y_axis.rug_opposite.plot.unwrap()), want(0));
        assert_eq!(l.x_axis.rug_opposite.plot, None, "an unset rug choice stays unset");
        assert_eq!(data(l.slopes[0].plot), want(0), "a slope annotation follows its plot's column");
        assert_eq!(data(l.slopes[0].x.unwrap()), want(1), "and its plot's X column");
    }
}
