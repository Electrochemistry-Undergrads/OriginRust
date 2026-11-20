//! Workbooks and worksheets. A workbook is a window that holds one or more
//! worksheets; a worksheet is an ordered list of columns.

use crate::column::{Column, Designation, default_short_name};
use crate::ids::{ColumnId, WindowId};
use serde::{Deserialize, Serialize};

/// The most rows or columns that can be frozen (Origin: "an index between 1 and 10").
pub const MAX_FROZEN: u8 = 10;

/// View > Freeze Panes: how many displayed rows and visible columns stay in place while the rest
/// of the sheet scrolls. A view setting of the sheet, saved with it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frozen {
    pub rows: u8,
    pub columns: u8,
}

impl Frozen {
    pub fn is_none(&self) -> bool {
        self.rows == 0 && self.columns == 0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Worksheet {
    pub name: String,
    /// The sheet's long name (Origin's `wks.longname$`), shown on the tab when [`Worksheet::tab_label`]
    /// asks for it (T90).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    /// Plain-text comments about the whole sheet, edited from its tab (T90).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comments: String,
    /// Navigate Worksheets' Exclude from Plotting dialogs (T184): Plot Setup and Layer Contents do not
    /// list the sheet's columns.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclude_from_plotting: bool,
    /// The sheet tab's Sheet Name to Show, per sheet as in Origin 2016.
    #[serde(default, skip_serializing_if = "crate::sheet_tab::SheetLabel::is_default")]
    pub tab_label: crate::sheet_tab::SheetLabel,
    /// The corner menu's View switches: label rows, row labels and grid lines (T92).
    #[serde(default, skip_serializing_if = "crate::sheet_view::SheetView::is_default")]
    pub view: crate::sheet_view::SheetView,
    pub columns: Vec<Column>,
    /// Column > Filter: hides the rows that fail a condition (see [`crate::rowfilter`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<crate::rowfilter::RowFilter>,
    /// View > Freeze Panes (see [`Frozen`]).
    #[serde(default, skip_serializing_if = "Frozen::is_none")]
    pub frozen: Frozen,
    /// Worksheet > Conditional Formatting (see [`crate::condformat`]); the earlier rule wins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cond_formats: Vec<crate::condformat::CondFormat>,
    /// Cell notes (see [`crate::notes`]), sorted by column and row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<crate::notes::CellNote>,
    /// Cell formats (see [`crate::cellformat`]): ranges of columns and rows; later ranges win.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cell_formats: Vec<crate::cellformat::FormatRange>,
    /// Category fonts under per-range formatting (T59); absent in older project files.
    #[serde(default, skip_serializing_if = "crate::sheet_fonts::WorksheetFonts::is_empty")]
    pub font_defaults: crate::sheet_fonts::WorksheetFonts,
    /// Set on the report sheet of a one-way ANOVA with a means comparison and of a two-sample t-test: the groups
    /// the report was made from, so that "Multiple Comparisons Plot" (T64) can draw them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison_source: Option<crate::compplot::ComparisonSource>,
    /// Add Notes as Sheet (T188): `Some` makes this a Notes sheet, a page of free text with no columns,
    /// shown as a text box in place of the grid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes_text: Option<String>,
    /// Add Graph as Sheet (T190): `Some` makes this a graph sheet that shows that graph window of the
    /// project in place of the grid. It has no columns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_sheet: Option<WindowId>,
    /// Graphs shown in data cells (the cell menu's Insert Graph..., T194), at most one per cell.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cell_graphs: Vec<CellGraph>,
}

/// A graph shown in a data cell (T194): the cell holds a picture of graph window `graph`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellGraph {
    pub column: ColumnId,
    pub row: usize,
    pub graph: WindowId,
}

impl Worksheet {
    pub fn new(name: impl Into<String>) -> Self {
        Worksheet {
            name: name.into(),
            long_name: String::new(),
            comments: String::new(),
            exclude_from_plotting: false,
            tab_label: Default::default(),
            view: Default::default(),
            columns: Vec::new(),
            filter: None,
            frozen: Frozen::default(),
            cond_formats: Vec::new(),
            notes: Vec::new(),
            cell_formats: Vec::new(),
            font_defaults: Default::default(),
            comparison_source: None,
            notes_text: None,
            graph_sheet: None,
            cell_graphs: Vec::new(),
        }
    }

    /// The graph shown in a data cell (T194).
    pub fn cell_graph(&self, column: ColumnId, row: usize) -> Option<WindowId> {
        self.cell_graphs.iter().find(|g| g.column == column && g.row == row).map(|g| g.graph)
    }

    /// Puts graph `graph` in a data cell, replacing the one there; `None` takes it out (T194).
    pub fn set_cell_graph(&mut self, column: ColumnId, row: usize, graph: Option<WindowId>) {
        self.cell_graphs.retain(|g| !(g.column == column && g.row == row));
        if let Some(graph) = graph {
            self.cell_graphs.push(CellGraph { column, row, graph });
        }
    }

    /// Follows new column ids (or drops the cell graphs of deleted columns); called by
    /// [`Worksheet::remap_note_columns`], which every column re-id and deletion goes through.
    pub(crate) fn remap_cell_graph_columns(&mut self, map: &dyn Fn(ColumnId) -> Option<ColumnId>) {
        self.cell_graphs.retain_mut(|g| match map(g.column) {
            Some(c) => {
                g.column = c;
                true
            }
            None => false,
        });
    }

    /// True for a Notes sheet (Add Notes as Sheet, T188).
    pub fn is_notes(&self) -> bool {
        self.notes_text.is_some()
    }

    /// Number of rows: the length of the longest column.
    pub fn row_count(&self) -> usize {
        self.columns.iter().map(Column::len).max().unwrap_or(0)
    }

    pub fn column_index(&self, id: ColumnId) -> Option<usize> {
        self.columns.iter().position(|c| c.id == id)
    }

    pub fn column(&self, id: ColumnId) -> Option<&Column> {
        self.columns.iter().find(|c| c.id == id)
    }

    pub fn column_mut(&mut self, id: ColumnId) -> Option<&mut Column> {
        self.columns.iter_mut().find(|c| c.id == id)
    }

    /// Finds a column by short name (case-insensitive), then by long name.
    pub fn column_by_name(&self, name: &str) -> Option<&Column> {
        self.columns
            .iter()
            .find(|c| c.short_name.eq_ignore_ascii_case(name))
            .or_else(|| self.columns.iter().find(|c| c.long_name == name))
    }

    /// A short name that no column of this sheet uses yet: the first free
    /// one in the sequence `A`, `B`, ..., `Z`, `AA`, ...
    pub fn next_short_name(&self) -> String {
        (0..)
            .map(default_short_name)
            .find(|n| !self.columns.iter().any(|c| c.short_name.eq_ignore_ascii_case(n)))
            .expect("the name sequence is unbounded")
    }

    /// The X column that the column at `index` is plotted against: the
    /// nearest X column to its left, or failing that the first X column in
    /// the sheet. `None` means "plot against the row number".
    pub fn x_for(&self, index: usize) -> Option<&Column> {
        let left = self.columns[..index.min(self.columns.len())].iter().rev().find(|c| c.designation == Designation::X);
        left.or_else(|| self.columns.iter().find(|c| c.designation == Designation::X))
    }

    /// The Y-error column that belongs to the Y column at `index`: an
    /// error column directly after it, before the next X or Y column.
    pub fn y_error_for(&self, index: usize) -> Option<&Column> {
        self.columns
            .iter()
            .skip(index + 1)
            .take_while(|c| !matches!(c.designation, Designation::X | Designation::Y))
            .find(|c| c.designation == Designation::YError)
    }

    /// The X-error column that belongs to the Y column at `index`: an X error column directly
    /// after it, before the next X or Y column.
    pub fn x_error_for(&self, index: usize) -> Option<&Column> {
        self.columns
            .iter()
            .skip(index + 1)
            .take_while(|c| !matches!(c.designation, Designation::X | Designation::Y))
            .find(|c| c.designation == Designation::XError)
    }

    /// Indices of all Y columns, in sheet order.
    pub fn y_columns(&self) -> Vec<usize> {
        self.columns.iter().enumerate().filter(|(_, c)| c.designation == Designation::Y).map(|(i, _)| i).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workbook {
    pub id: WindowId,
    /// Short name, unique among all windows of the project (for example `Book1`).
    pub name: String,
    /// Free-form title, often the imported file name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    pub sheets: Vec<Worksheet>,
    #[serde(default)]
    pub active_sheet: usize,
    /// Where the data came from (file path and importer settings), kept so a
    /// file can be imported again with the same settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ImportSource>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportSource {
    pub path: String,
    /// Name of the importer that read the file, for example `chi` or `ascii`.
    pub importer: String,
}

impl Workbook {
    /// Add Graph as Sheet (T190): a new sheet after the last one that shows graph `graph`, named
    /// `name` (or `name2`, `name3`, ... when a sheet has that name), made the active sheet. Returns its
    /// index.
    pub fn add_graph_sheet(&mut self, graph: WindowId, name: &str) -> usize {
        let base = if name.trim().is_empty() { "Graph" } else { name.trim() };
        let name = std::iter::once(base.to_string())
            .chain((2..).map(|n| format!("{base}{n}")))
            .find(|n| !self.sheets.iter().any(|s| s.name.eq_ignore_ascii_case(n)))
            .expect("the name sequence is unbounded");
        let mut sheet = Worksheet::new(name);
        sheet.graph_sheet = Some(graph);
        self.sheets.push(sheet);
        self.active_sheet = self.sheets.len() - 1;
        self.active_sheet
    }

    /// Add Notes as Sheet (T188): a new empty Notes sheet after the last sheet, named `Notes` (then
    /// `Notes2`, `Notes3`, ... when taken), made the active sheet. Returns its index.
    pub fn add_notes_sheet(&mut self) -> usize {
        let name = std::iter::once("Notes".to_string())
            .chain((2..).map(|n| format!("Notes{n}")))
            .find(|n| !self.sheets.iter().any(|s| s.name.eq_ignore_ascii_case(n)))
            .expect("the name sequence is unbounded");
        let mut sheet = Worksheet::new(name);
        sheet.notes_text = Some(String::new());
        self.sheets.push(sheet);
        self.active_sheet = self.sheets.len() - 1;
        self.active_sheet
    }

    /// The title a graph made from this book takes: the book's long name, except that an imported
    /// book titled with its whole file name (`data.csv`, as Origin titles imports) gives the name
    /// without the extension, so graph titles and exported file names stay `data`.
    pub fn title_for_graph(&self) -> String {
        if let Some(src) = &self.source {
            let path = std::path::Path::new(&src.path);
            if path.file_name().is_some_and(|n| n.to_string_lossy() == self.long_name)
                && let Some(stem) = path.file_stem()
            {
                return stem.to_string_lossy().into_owned();
            }
        }
        self.long_name.clone()
    }

    pub fn new(id: WindowId, name: impl Into<String>) -> Self {
        Workbook {
            id,
            name: name.into(),
            long_name: String::new(),
            sheets: vec![Worksheet::new("Sheet1")],
            active_sheet: 0,
            source: None,
        }
    }

    pub fn sheet(&self) -> &Worksheet {
        &self.sheets[self.active_sheet.min(self.sheets.len().saturating_sub(1))]
    }

    pub fn sheet_mut(&mut self) -> &mut Worksheet {
        let i = self.active_sheet.min(self.sheets.len().saturating_sub(1));
        &mut self.sheets[i]
    }

    pub fn sheet_by_name(&self, name: &str) -> Option<&Worksheet> {
        self.sheets.iter().find(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// The window title: `Book1 - long name` when a long name is set.
    pub fn title(&self) -> String {
        if self.long_name.is_empty() { self.name.clone() } else { format!("{} - {}", self.name, self.long_name) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::ColumnData;

    #[test]
    fn worksheet_comments_default_to_empty_and_preserve_multiline_text() {
        let mut sheet: Worksheet = serde_json::from_str(r#"{"name":"Sheet1","columns":[]}"#).unwrap();
        assert!(sheet.comments.is_empty());
        assert!(serde_json::to_value(&sheet).unwrap().get("comments").is_none());
        sheet.comments = "  Run at 25 °C\nSecond sweep.\n".into();
        let saved = serde_json::to_string(&sheet).unwrap();
        let opened: Worksheet = serde_json::from_str(&saved).unwrap();
        assert_eq!(opened, sheet);
    }

    fn col(id: u64, name: &str, d: Designation) -> Column {
        Column::new(ColumnId(id), name, d, ColumnData::numeric(vec![0.0; id as usize]))
    }

    #[test]
    fn y_columns_find_the_x_on_their_left() {
        let mut ws = Worksheet::new("Sheet1");
        ws.columns = vec![
            col(1, "A", Designation::X),
            col(2, "B", Designation::Y),
            col(3, "C", Designation::X),
            col(4, "D", Designation::Y),
            col(5, "E", Designation::YError),
        ];
        assert_eq!(ws.x_for(1).unwrap().short_name, "A");
        assert_eq!(ws.x_for(3).unwrap().short_name, "C");
        assert_eq!(ws.y_error_for(3).unwrap().short_name, "E");
        assert!(ws.y_error_for(1).is_none());
        assert_eq!(ws.y_columns(), vec![1, 3]);
        assert_eq!(ws.row_count(), 5);
        assert_eq!(ws.next_short_name(), "F");
    }

    #[test]
    fn a_y_column_left_of_every_x_uses_the_first_x() {
        let mut ws = Worksheet::new("Sheet1");
        ws.columns = vec![col(1, "A", Designation::Y), col(2, "B", Designation::X)];
        assert_eq!(ws.x_for(0).unwrap().short_name, "B");
        ws.columns[1].designation = Designation::Y;
        assert!(ws.x_for(0).is_none());
    }

    #[test]
    fn frozen_panes_round_trip_and_old_files_load_unfrozen() {
        let mut ws = Worksheet::new("Sheet1");
        let plain = serde_json::to_string(&ws).unwrap();
        assert!(!plain.contains("frozen"), "an unfrozen sheet writes nothing: {plain}");
        let back: Worksheet = serde_json::from_str(&plain).unwrap();
        assert_eq!(back.frozen, Frozen::default());
        ws.frozen = Frozen { rows: 3, columns: 2 };
        let text = serde_json::to_string(&ws).unwrap();
        let back: Worksheet = serde_json::from_str(&text).unwrap();
        assert_eq!(back.frozen, Frozen { rows: 3, columns: 2 });
    }
}

#[cfg(test)]
mod title_tests {
    use super::*;

    #[test]
    fn a_graph_of_an_imported_book_is_titled_without_the_extension() {
        let mut b = Workbook::new(WindowId(1), "Book1");
        b.long_name = "data.csv".into();
        b.source = Some(ImportSource { path: "C:/runs/data.csv".into(), importer: "ascii".into() });
        assert_eq!(b.title_for_graph(), "data");
        // a title the user changed, or a book that was not imported, is used as it is
        b.long_name = "My data.csv run".into();
        assert_eq!(b.title_for_graph(), "My data.csv run");
        b.source = None;
        b.long_name = "notes.txt".into();
        assert_eq!(b.title_for_graph(), "notes.txt");
    }

    #[test]
    fn a_notes_sheet_is_added_after_the_last_sheet_with_a_free_name() {
        let mut b = Workbook::new(WindowId(1), "Book1");
        let first = b.add_notes_sheet();
        assert_eq!((first, b.sheets[first].name.as_str()), (1, "Notes"));
        assert!(b.sheets[first].is_notes() && b.sheets[first].columns.is_empty());
        let second = b.add_notes_sheet();
        assert_eq!(b.sheets[second].name, "Notes2");
        assert_eq!(b.active_sheet, second);
        let json = serde_json::to_string(&b).unwrap();
        let back: Workbook = serde_json::from_str(&json).unwrap();
        assert!(back.sheets[1].is_notes() && !back.sheets[0].is_notes());
    }

    #[test]
    fn a_graph_sheet_names_itself_after_the_graph_and_round_trips() {
        let mut b = Workbook::new(WindowId(1), "Book1");
        let i = b.add_graph_sheet(WindowId(7), "Graph1");
        let j = b.add_graph_sheet(WindowId(8), "Graph1");
        assert_eq!((b.sheets[i].name.as_str(), b.sheets[j].name.as_str()), ("Graph1", "Graph12"));
        assert_eq!((b.sheets[i].graph_sheet, b.active_sheet), (Some(WindowId(7)), j));
        let back: Workbook = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!(back.sheets[j].graph_sheet, Some(WindowId(8)));
        assert_eq!(back.sheets[0].graph_sheet, None);
    }

    #[test]
    fn a_cell_holds_one_graph_and_follows_its_column() {
        let mut s = Worksheet::new("S");
        s.set_cell_graph(ColumnId(1), 2, Some(WindowId(9)));
        s.set_cell_graph(ColumnId(1), 2, Some(WindowId(10)));
        assert_eq!((s.cell_graphs.len(), s.cell_graph(ColumnId(1), 2)), (1, Some(WindowId(10))));
        s.remap_note_columns(&|c| Some(ColumnId(c.0 + 100)));
        assert_eq!(s.cell_graph(ColumnId(101), 2), Some(WindowId(10)));
        s.remap_note_columns(&|_| None);
        assert!(s.cell_graphs.is_empty(), "a deleted column takes its graphs");
        s.set_cell_graph(ColumnId(3), 0, Some(WindowId(4)));
        s.set_cell_graph(ColumnId(3), 0, None);
        assert!(s.cell_graphs.is_empty());
    }
}

impl crate::project::Project {
    /// Puts graph `graph` in the data cell of `column` at `row`, or takes it out with `None` (T194).
    /// False when there is no such column.
    pub fn set_cell_graph(&mut self, column: ColumnId, row: usize, graph: Option<WindowId>) -> bool {
        for w in &mut self.windows {
            if let crate::project::Window::Workbook(b) = w
                && let Some(s) = b.sheets.iter_mut().find(|s| s.column(column).is_some())
            {
                s.set_cell_graph(column, row, graph);
                return true;
            }
        }
        false
    }
}
