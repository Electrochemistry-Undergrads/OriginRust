//! Cell links (Edit > Paste Link, Paste Link Transpose, Remove Links; T211, ADR 0014): a worksheet
//! cell that takes the value of another cell of the project. The link is a record on the target sheet;
//! [`Project::refresh_cell_links`] copies each source value into its target cell, and
//! [`crate::Document`] runs it inside every edit's undo step.

use crate::column::{Column, ColumnData};
use crate::ids::ColumnId;
use crate::project::{Project, Window};
use crate::worksheet::Worksheet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A linked cell: the cell of `column` at `row` shows the cell of `source` at `source_row`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellLink {
    pub column: ColumnId,
    pub row: usize,
    pub source: ColumnId,
    pub source_row: usize,
}

/// A cell's value as a link copies it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Number(f64),
    Text(String),
}

fn read(c: &Column, row: usize) -> Value {
    if let Some(t) = c.data.text_cell(row) {
        return Value::Text(t.to_string());
    }
    match &c.data {
        ColumnData::Numeric(v) => Value::Number(v.get(row).copied().unwrap_or(f64::NAN)),
        ColumnData::Mixed { numbers, .. } => Value::Number(numbers.get(row).copied().unwrap_or(f64::NAN)),
        ColumnData::Text(v) => Value::Text(v.get(row).cloned().unwrap_or_default()),
    }
}

fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.to_bits() == y.to_bits(),
        (a, b) => a == b,
    }
}

/// Writes `value` into a cell; false when the cell's value did not change (a text that a numeric column
/// cannot hold stays a missing value).
pub(crate) fn write(c: &mut Column, row: usize, value: &Value) -> bool {
    let before = read(c, row);
    if same(&before, value) {
        return false;
    }
    write_value(c, row, value);
    !same(&before, &read(c, row))
}

fn write_value(c: &mut Column, row: usize, value: &Value) {
    match (&mut c.data, value) {
        (ColumnData::Numeric(v), Value::Number(x)) => {
            let v = Arc::make_mut(v);
            if row >= v.len() {
                v.resize(row + 1, f64::NAN);
            }
            v[row] = *x;
        }
        (ColumnData::Mixed { numbers, texts }, Value::Number(x)) => {
            let numbers = Arc::make_mut(numbers);
            if row >= numbers.len() {
                numbers.resize(row + 1, f64::NAN);
            }
            numbers[row] = *x;
            if texts.contains_key(&row) {
                Arc::make_mut(texts).remove(&row);
            }
        }
        (ColumnData::Text(_), Value::Number(x)) => {
            let text = if x.is_finite() { format!("{x}") } else { String::new() };
            c.set_cell_text(row, &text);
        }
        (_, Value::Text(t)) => {
            c.set_cell_text(row, t);
        }
    }
}

impl Worksheet {
    /// The link of a cell (T211).
    pub fn cell_link(&self, column: ColumnId, row: usize) -> Option<CellLink> {
        self.cell_links.iter().find(|l| l.column == column && l.row == row).copied()
    }

    /// Links a cell to a source cell, replacing its link; `None` drops the link (the value stays).
    pub fn set_cell_link(&mut self, column: ColumnId, row: usize, source: Option<(ColumnId, usize)>) {
        self.cell_links.retain(|l| !(l.column == column && l.row == row));
        if let Some((source, source_row)) = source {
            self.cell_links.push(CellLink { column, row, source, source_row });
        }
    }

    /// Remove Links: drops the links of the cells of `columns` in `rows` (all rows with `None`).
    /// Returns how many links went.
    pub fn remove_cell_links(&mut self, columns: &[ColumnId], rows: Option<std::ops::Range<usize>>) -> usize {
        let before = self.cell_links.len();
        self.cell_links.retain(|l| !(columns.contains(&l.column) && rows.as_ref().is_none_or(|r| r.contains(&l.row))));
        before - self.cell_links.len()
    }

    /// Follows new column ids for the target and source columns; a link whose target or source column
    /// the map drops is dropped (so a sheet copied on its own loses links to other sheets). Called
    /// through [`Worksheet::remap_note_columns`].
    pub(crate) fn remap_cell_link_columns(&mut self, map: &dyn Fn(ColumnId) -> Option<ColumnId>) {
        self.cell_links.retain_mut(|l| match (map(l.column), map(l.source)) {
            (Some(c), Some(s)) => {
                l.column = c;
                l.source = s;
                true
            }
            _ => false,
        });
    }
}

impl Project {
    /// Copies every linked cell's source value into it (T211). Returns true when a cell changed. A link
    /// whose source column is gone leaves its cell as it is.
    pub fn refresh_cell_links(&mut self) -> bool {
        let mut updates: Vec<(ColumnId, usize, Value)> = Vec::new();
        for w in &self.windows {
            let Window::Workbook(b) = w else { continue };
            for s in &b.sheets {
                for l in &s.cell_links {
                    if let Some(src) = self.column(l.source) {
                        updates.push((l.column, l.row, read(src, l.source_row)));
                    }
                }
            }
        }
        let mut changed = false;
        for (column, row, value) in updates {
            if let Some(c) = self.column_mut(column) {
                changed |= write(c, row, &value);
            }
        }
        changed
    }

    /// Drops the link of one cell (typing into a linked cell, T211).
    pub fn drop_cell_link(&mut self, column: ColumnId, row: usize) {
        for w in &mut self.windows {
            if let Window::Workbook(b) = w
                && let Some(s) = b.sheets.iter_mut().find(|s| s.column(column).is_some())
            {
                s.set_cell_link(column, row, None);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Designation;

    fn book() -> (Project, crate::WindowId, ColumnId, ColumnId, ColumnId) {
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        let a = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.5, 2.5, 3.5])).unwrap();
        let t = p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["x".into(), "y".into()])).unwrap();
        let target = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![])).unwrap();
        (p, b, a, t, target)
    }

    #[test]
    fn linked_cells_take_numbers_and_text_and_follow_their_source() {
        let (mut p, b, a, t, target) = book();
        let s = &mut p.workbook_mut(b).unwrap().sheets[0];
        s.set_cell_link(target, 0, Some((a, 1)));
        s.set_cell_link(target, 1, Some((t, 0)));
        assert!(p.refresh_cell_links());
        assert!(!p.refresh_cell_links(), "nothing changes the second time");
        let c = p.column(target).unwrap();
        assert_eq!(c.numeric().unwrap()[0], 2.5);
        assert!(c.numeric().unwrap()[1].is_nan(), "text into a numeric column is a missing value");
        p.column_mut(a).unwrap().set_cell_text(1, "9");
        assert!(p.refresh_cell_links());
        assert_eq!(p.column(target).unwrap().numeric().unwrap()[0], 9.0);
    }

    #[test]
    fn remove_links_keeps_the_values_and_deleted_columns_drop_links() {
        let (mut p, b, a, _, target) = book();
        let s = &mut p.workbook_mut(b).unwrap().sheets[0];
        s.set_cell_link(target, 0, Some((a, 0)));
        s.set_cell_link(target, 1, Some((a, 1)));
        p.refresh_cell_links();
        let s = &mut p.workbook_mut(b).unwrap().sheets[0];
        assert_eq!(s.remove_cell_links(&[target], Some(0..1)), 1);
        assert!(s.cell_link(target, 0).is_none() && s.cell_link(target, 1).is_some());
        p.column_mut(a).unwrap().set_cell_text(0, "7");
        p.refresh_cell_links();
        assert_eq!(p.column(target).unwrap().numeric().unwrap()[0], 1.5, "an unlinked cell keeps its value");
        let s = &mut p.workbook_mut(b).unwrap().sheets[0];
        s.remap_note_columns(&|c| (c != a).then_some(c));
        assert!(s.cell_links.is_empty(), "a deleted source column within the sheet drops the link");
        let json = serde_json::to_string(&p.workbook(b).unwrap().sheets[0]).unwrap();
        assert!(!json.contains("cell_links"), "no links, nothing written");
    }
}
