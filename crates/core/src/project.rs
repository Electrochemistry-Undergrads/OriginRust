//! The project: every workbook and graph, plus the folder tree that
//! organizes them.

use crate::column::{Column, ColumnData, Designation};
use crate::graph::{Graph, Plot};
use crate::ids::{ColumnId, WindowId};
use crate::template::Template;
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

/// A folder of the project explorer. Windows that no folder lists belong to
/// the root folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Folder {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<Folder>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<WindowId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Windows in creation order.
    pub windows: Vec<Window>,
    #[serde(default)]
    pub root: Folder,
    /// The next unused identifier (shared by windows and columns).
    next_id: u64,
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
    #[error("column '{0}' was not found")]
    NoSuchColumnName(String),
    #[error("the graph has no layer {0}")]
    NoSuchLayer(usize),
}

impl Project {
    pub fn new() -> Self {
        Project { windows: Vec::new(), root: Folder { name: "Project".into(), ..Folder::default() }, next_id: 1 }
    }

    fn fresh_id(&mut self) -> u64 {
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
            for col in &mut sheet.columns {
                col.id = ColumnId(self.fresh_id());
            }
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
    /// Y-error column are found from the column designations of the sheet.
    /// The plot takes the layer's default style and its next palette color.
    pub fn add_plot(&mut self, graph: WindowId, layer: usize, y: ColumnId) -> Result<(), ProjectError> {
        let (_, sheet, _) = self.locate(y).ok_or(ProjectError::NoSuchColumn(y))?;
        let index = sheet.column_index(y).expect("column is in this sheet");
        let x = sheet.x_for(index).map(|c| c.id);
        let y_error = sheet.y_error_for(index).map(|c| c.id);
        self.add_plot_xy(graph, layer, x, y, y_error)
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
        });
        l.color_newest_plot();
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

    /// The Y columns a new graph of `template` should show for a sheet.
    ///
    /// That is every numeric Y column when they all share one unit. When the
    /// units differ, one Y axis cannot show them all, so only one group is
    /// plotted: the columns in mA if there are any (an imported CH Instruments
    /// file has the current in A and in mA plus, for current-vs-time files,
    /// the charge; figures use mA), otherwise the columns that share the unit
    /// of the first Y column.
    pub fn default_plot_columns(&self, book: WindowId, sheet: usize, _template: Template) -> Vec<ColumnId> {
        let Some(ws) = self.workbook(book).and_then(|b| b.sheets.get(sheet)) else {
            return Vec::new();
        };
        let ys: Vec<&Column> =
            ws.columns.iter().filter(|c| c.designation == Designation::Y && c.data.is_numeric()).collect();
        let Some(first) = ys.first() else {
            return Vec::new();
        };
        let unit = if ys.iter().any(|c| c.units == "mA") { "mA" } else { first.units.as_str() };
        ys.iter().filter(|c| c.units == unit).map(|c| c.id).collect()
    }

    /// Creates a graph of `template` that plots the given Y columns in one
    /// layer, and returns its id. The graph's long name is taken from the
    /// workbook that holds the first column.
    pub fn plot_columns(&mut self, template: Template, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        for y in ys {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        let long_name = ys.first().and_then(|y| self.locate(*y)).map(|(b, _, _)| b.long_name.clone());
        let graph = self.add_graph(template);
        for y in ys {
            self.add_plot(graph, 0, *y)?;
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
        p.graph_mut(g).unwrap().layers[0].plots[0].line.color = chosen;
        p.add_plot(g, 0, y2).unwrap();
        let plots = &p.graph(g).unwrap().layers[0].plots;
        assert_eq!(plots[0].line.color, chosen);
        assert_ne!(plots[1].line.color, chosen);
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
}
