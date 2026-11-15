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
    /// Set on the report sheet of a one-way ANOVA with a means comparison and of a two-sample t-test: the groups
    /// the report was made from, so that "Multiple Comparisons Plot" (T64) can draw them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison_source: Option<crate::compplot::ComparisonSource>,
}

impl Worksheet {
    pub fn new(name: impl Into<String>) -> Self {
        Worksheet {
            name: name.into(),
            columns: Vec::new(),
            filter: None,
            frozen: Frozen::default(),
            cond_formats: Vec::new(),
            notes: Vec::new(),
            comparison_source: None,
        }
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
