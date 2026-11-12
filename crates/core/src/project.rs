//! The project: every workbook and graph, plus the folder tree that
//! organizes them.

use crate::column::{Column, ColumnData, Designation};
use crate::graph::{Graph, LegendScope, Plot};
use crate::ids::{ColumnId, WindowId};
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
    #[error("the layer has no plot {0}")]
    NoSuchPlot(usize),
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
            auto_color: true,
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
        if template == Template::DoubleY {
            return ys.iter().map(|c| c.id).collect();
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
        if template == Template::DoubleY {
            let left: Vec<ColumnId> = ys.iter().step_by(2).copied().collect();
            let right: Vec<ColumnId> = ys.iter().skip(1).step_by(2).copied().collect();
            return self.plot_columns_double_y(template, &left, &right);
        }
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

    /// Creates a graph of `template` with the `left` columns on its left Y axis and the `right`
    /// columns on a right Y axis (added with [`Project::add_right_y_layer`] unless the template
    /// already has one), and returns its id. The long name comes from the first column's workbook.
    pub fn plot_columns_double_y(
        &mut self,
        template: Template,
        left: &[ColumnId],
        right: &[ColumnId],
    ) -> Result<WindowId, ProjectError> {
        for y in left.iter().chain(right) {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        let first = left.first().or(right.first());
        let long_name = first.and_then(|y| self.locate(*y)).map(|(b, _, _)| b.long_name.clone());
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

    /// Gives layer `parent` of a graph a right Y axis and returns the index of the layer that
    /// carries it. When the parent's group already has such a layer, that layer is returned and
    /// nothing changes. Otherwise a new layer linked to the parent (or to the parent's own parent
    /// when `parent` is itself linked) is appended (see [`right_y_layer`]); the parent then no
    /// longer draws a line or ticks on its right edge, and its legend lists the plots of every
    /// layer.
    pub fn add_right_y_layer(&mut self, graph: WindowId, parent: usize) -> Result<usize, ProjectError> {
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        if parent >= g.layers.len() {
            return Err(ProjectError::NoSuchLayer(parent));
        }
        let root = g.root_of(parent);
        if let Some(existing) = g.right_y_member(root) {
            return Ok(existing);
        }
        let layer = right_y_layer(&g.layers[root], root);
        let p = &mut g.layers[root];
        p.y_axis.show_opposite = false;
        p.y_axis.opposite_ticks = false;
        p.legend.scope = LegendScope::Graph;
        g.layers.push(layer);
        Ok(g.layers.len() - 1)
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
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
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
        g.layers[to].plots.push(plot);
        g.layers[from].recolor_automatic();
        g.layers[to].recolor_automatic();
        Ok(())
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
}
