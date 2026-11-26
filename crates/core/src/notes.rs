//! Cell notes: a plain-text note attached to one data cell of a worksheet (Origin 2022 "Add Notes
//! to individual Worksheet Cells"). A note belongs to a column (by its stable id) and a data row
//! index, so it stays on its value through sorting, row inserts and deletes; Sheaf's rules for
//! those are its own choices and unmeasured, see docs/spec/gui-look.md "Cell notes (T36)".
//!
//! Notes are kept in a vector sorted by (column, row), so the saved form is deterministic and a
//! lookup is a binary search. A note may sit on an empty cell (Sheaf's choice; it follows that
//! row position like any other). Empty text is never stored.

use crate::ids::ColumnId;
use crate::worksheet::Worksheet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellNote {
    pub column: ColumnId,
    /// Zero-based data row.
    pub row: usize,
    pub text: String,
}

impl Worksheet {
    fn note_pos(&self, column: ColumnId, row: usize) -> Result<usize, usize> {
        self.notes.binary_search_by(|n| (n.column, n.row).cmp(&(column, row)))
    }

    /// The note of a cell, if it has one.
    pub fn note(&self, column: ColumnId, row: usize) -> Option<&str> {
        self.note_pos(column, row).ok().map(|i| self.notes[i].text.as_str())
    }

    /// Sets, replaces or (for empty text) removes the note of a cell. Returns true when the sheet
    /// changed.
    pub fn set_note(&mut self, column: ColumnId, row: usize, text: &str) -> bool {
        let text = text.trim_end();
        match (self.note_pos(column, row), text.trim().is_empty()) {
            (Ok(i), true) => {
                self.notes.remove(i);
                true
            }
            (Ok(i), false) => {
                if self.notes[i].text == text {
                    return false;
                }
                self.notes[i].text = text.to_string();
                true
            }
            (Err(_), true) => false,
            (Err(i), false) => {
                self.notes.insert(i, CellNote { column, row, text: text.to_string() });
                true
            }
        }
    }

    /// Removes the note of a cell; true when there was one.
    pub fn remove_note(&mut self, column: ColumnId, row: usize) -> bool {
        self.set_note(column, row, "")
    }

    fn sort_notes(&mut self) {
        self.notes.sort_by_key(|n| (n.column, n.row));
    }

    /// Renames the columns of the notes (`map` gives the new id, or `None` when the column is
    /// gone, which drops its notes). Used wherever a sheet's columns get new ids or are deleted.
    pub fn remap_note_columns(&mut self, map: &dyn Fn(ColumnId) -> Option<ColumnId>) {
        // The graphs in cells (T194) and the cell formulas (T464) follow the same column changes.
        self.remap_cell_graph_columns(map);
        self.remap_formula_columns(map);
        if self.notes.is_empty() {
            return;
        }
        let old = std::mem::take(&mut self.notes);
        self.notes = old
            .into_iter()
            .filter_map(|mut n| {
                n.column = map(n.column)?;
                Some(n)
            })
            .collect();
        self.sort_notes();
    }

    /// Moves the notes of these columns for `count` rows inserted before row `at`.
    pub fn notes_insert_rows(&mut self, columns: &[ColumnId], at: usize, count: usize) {
        // Cell formulas (T462) move with their cells too.
        self.formulas_insert_rows(columns, at, count);
        for n in &mut self.notes {
            if columns.contains(&n.column) && n.row >= at {
                n.row += count;
            }
        }
    }

    /// Drops the notes on the deleted `rows` of these columns and moves up those below.
    pub fn notes_delete_rows(&mut self, columns: &[ColumnId], rows: std::ops::Range<usize>) {
        if rows.is_empty() {
            return;
        }
        self.formulas_delete_rows(columns, rows.clone());
        self.notes.retain(|n| !(columns.contains(&n.column) && rows.contains(&n.row)));
        for n in &mut self.notes {
            if columns.contains(&n.column) && n.row >= rows.end {
                n.row -= rows.len();
            }
        }
    }

    /// Puts the rows of the notes of these columns in the order `order` (new row i takes old row
    /// `order[i]`, as [`crate::column::Column::reorder_rows`] does); notes on rows not in `order`
    /// are dropped, notes at or beyond `rows` stay. `rows` is the sheet length the order is for.
    pub fn notes_reorder_rows(&mut self, columns: &dyn Fn(ColumnId) -> bool, order: &[usize], rows: usize) {
        self.formulas_reorder_rows(columns, order, rows);
        if self.notes.is_empty() {
            return;
        }
        let mut new_row = std::collections::HashMap::new();
        for (i, src) in order.iter().enumerate() {
            new_row.entry(*src).or_insert(i);
        }
        let old = std::mem::take(&mut self.notes);
        self.notes = old
            .into_iter()
            .filter_map(|mut n| {
                if columns(n.column) && n.row < rows {
                    n.row = *new_row.get(&n.row)?;
                }
                Some(n)
            })
            .collect();
        self.sort_notes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{Column, ColumnData, Designation};

    fn sheet() -> Worksheet {
        let mut s = Worksheet::new("Sheet1");
        for i in 0..2u64 {
            s.columns.push(Column::new(
                ColumnId(i + 1),
                crate::column::default_short_name(i as usize),
                Designation::Y,
                ColumnData::numeric(vec![3.0, 1.0, 2.0]),
            ));
        }
        s
    }

    #[test]
    fn set_get_replace_and_remove() {
        let mut s = sheet();
        let a = ColumnId(1);
        assert!(s.set_note(a, 1, "hello"));
        assert_eq!(s.note(a, 1), Some("hello"));
        assert!(!s.set_note(a, 1, "hello"));
        assert!(s.set_note(a, 1, "bye\n"));
        assert_eq!(s.note(a, 1), Some("bye"));
        assert!(s.set_note(ColumnId(2), 0, "x"));
        assert!(s.set_note(a, 0, "first"));
        assert_eq!(s.notes[0].row, 0);
        assert!(!s.set_note(a, 2, "  \n"));
        assert!(s.set_note(a, 1, ""));
        assert_eq!(s.note(a, 1), None);
        assert!(!s.remove_note(a, 1));
        assert!(s.remove_note(a, 0));
    }

    #[test]
    fn old_json_loads_and_empty_notes_are_not_saved() {
        let s = sheet();
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("notes"));
        let back: Worksheet = serde_json::from_str(&json).unwrap();
        assert!(back.notes.is_empty());
        let mut s = sheet();
        s.set_note(ColumnId(1), 2, "n");
        let back: Worksheet = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.note(ColumnId(1), 2), Some("n"));
    }

    #[test]
    fn insert_and_delete_rows_shift_and_drop() {
        let mut s = sheet();
        let (a, b) = (ColumnId(1), ColumnId(2));
        s.set_note(a, 0, "r0");
        s.set_note(a, 2, "r2");
        s.set_note(b, 2, "b2");
        s.insert_rows(&[0], 1, 2);
        assert_eq!(s.note(a, 0), Some("r0"));
        assert_eq!(s.note(a, 4), Some("r2"));
        assert_eq!(s.note(b, 2), Some("b2"));
        s.delete_rows(&[0], 3..5);
        assert_eq!(s.note(a, 4), None);
        assert_eq!(s.note(a, 3), None);
        s.set_note(a, 3, "z");
        s.delete_rows(&[0], 1..3);
        assert_eq!(s.note(a, 1), Some("z"));
        assert_eq!(s.note(b, 2), Some("b2"));
    }

    #[test]
    fn reorder_moves_notes_with_their_rows_and_only_for_listed_columns() {
        let mut s = sheet();
        let (a, b) = (ColumnId(1), ColumnId(2));
        s.set_note(a, 0, "three");
        s.set_note(b, 0, "stay");
        s.notes_reorder_rows(&|c| c == a, &[1, 2, 0], 3);
        assert_eq!(s.note(a, 2), Some("three"));
        assert_eq!(s.note(b, 0), Some("stay"));
        s.notes_reorder_rows(&|_| true, &[2], 3);
        assert_eq!(s.note(a, 0), Some("three"));
        assert_eq!(s.note(b, 0), None);
    }

    #[test]
    fn remap_renames_and_drops() {
        let mut s = sheet();
        s.set_note(ColumnId(1), 0, "a");
        s.set_note(ColumnId(2), 0, "b");
        s.remap_note_columns(&|c| (c.0 == 2).then_some(ColumnId(1)));
        assert_eq!(s.notes.len(), 1);
        assert_eq!(s.note(ColumnId(1), 0), Some("b"));
    }
}
