//! Worksheet operations that change the shape of a sheet: inserting and deleting rows and
//! columns, moving and hiding columns.

use crate::column::{Column, ColumnData, Designation};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::worksheet::Worksheet;
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

impl ColumnData {
    /// Inserts `count` missing cells before row `at`, moving the cells from there on down.
    /// Nothing happens when `at` is beyond the data (there is nothing to move).
    pub fn insert_blank_rows(&mut self, at: usize, count: usize) {
        if count == 0 || at >= self.len() {
            return;
        }
        match self {
            ColumnData::Numeric(v) => {
                Arc::make_mut(v).splice(at..at, std::iter::repeat_n(f64::NAN, count));
            }
            ColumnData::Text(v) => {
                Arc::make_mut(v).splice(at..at, std::iter::repeat_n(String::new(), count));
            }
            ColumnData::Mixed { numbers, texts } => {
                let n = Arc::make_mut(numbers);
                if at < n.len() {
                    n.splice(at..at, std::iter::repeat_n(f64::NAN, count));
                }
                let moved: BTreeMap<usize, String> =
                    texts.iter().map(|(r, t)| (if *r >= at { r + count } else { *r }, t.clone())).collect();
                *texts = Arc::new(moved);
            }
        }
    }

    /// Removes the cells of these rows, moving the cells below them up.
    pub fn remove_rows(&mut self, rows: Range<usize>) {
        let end = rows.end.min(self.len());
        if rows.start >= end {
            return;
        }
        let count = end - rows.start;
        match self {
            ColumnData::Numeric(v) => {
                Arc::make_mut(v).drain(rows.start..end);
            }
            ColumnData::Text(v) => {
                Arc::make_mut(v).drain(rows.start..end);
            }
            ColumnData::Mixed { numbers, texts } => {
                let n = Arc::make_mut(numbers);
                let stop = end.min(n.len());
                if rows.start < stop {
                    n.drain(rows.start..stop);
                }
                let moved: BTreeMap<usize, String> = texts
                    .iter()
                    .filter(|(r, _)| **r < rows.start || **r >= end)
                    .map(|(r, t)| (if *r >= end { r - count } else { *r }, t.clone()))
                    .collect();
                *texts = Arc::new(moved);
            }
        }
    }
}

/// Where Column > Move Columns puts the selected columns.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MoveTo {
    First,
    Left,
    Right,
    Last,
}

impl Worksheet {
    /// Inserts `count` blank rows before row `at` in the columns at these positions; the cells
    /// below move down. Columns kept up to date by a formula are left alone. Returns how many
    /// such columns were skipped.
    pub fn insert_rows(&mut self, columns: &[usize], at: usize, count: usize) -> usize {
        let mut locked = 0;
        for &i in columns {
            let Some(c) = self.columns.get_mut(i) else { continue };
            if c.is_locked() {
                locked += 1;
                continue;
            }
            c.data.insert_blank_rows(at, count);
        }
        locked
    }

    /// Deletes these rows from the columns at these positions; the cells below move up.
    /// Returns how many formula columns were skipped.
    pub fn delete_rows(&mut self, columns: &[usize], rows: Range<usize>) -> usize {
        let mut locked = 0;
        for &i in columns {
            let Some(c) = self.columns.get_mut(i) else { continue };
            if c.is_locked() {
                locked += 1;
                continue;
            }
            c.data.remove_rows(rows.clone());
            c.trim_missing_tail();
        }
        locked
    }

    /// Moves the columns with these ids by one step or to an end of the sheet, keeping their
    /// order among themselves. Returns false when nothing moved.
    pub fn move_columns(&mut self, ids: &[ColumnId], to: MoveTo) -> bool {
        let before = self.columns.iter().map(|c| c.id).collect::<Vec<_>>();
        let selected = |c: &Column| ids.contains(&c.id);
        match to {
            MoveTo::First | MoveTo::Last => {
                let (mut moving, mut rest): (Vec<Column>, Vec<Column>) =
                    std::mem::take(&mut self.columns).into_iter().partition(selected);
                if to == MoveTo::First {
                    moving.append(&mut rest);
                    self.columns = moving;
                } else {
                    rest.append(&mut moving);
                    self.columns = rest;
                }
            }
            MoveTo::Left => {
                // Each selected column swaps with the one on its left unless that is selected
                // too, so a block moves as a block and stops at the edge.
                for i in 1..self.columns.len() {
                    if selected(&self.columns[i]) && !selected(&self.columns[i - 1]) {
                        self.columns.swap(i - 1, i);
                    }
                }
            }
            MoveTo::Right => {
                for i in (0..self.columns.len().saturating_sub(1)).rev() {
                    if selected(&self.columns[i]) && !selected(&self.columns[i + 1]) {
                        self.columns.swap(i, i + 1);
                    }
                }
            }
        }
        before != self.columns.iter().map(|c| c.id).collect::<Vec<_>>()
    }
}

impl Project {
    /// Inserts `count` new empty columns before position `at` of a sheet (Origin puts new
    /// columns to the left of the selected one). Returns their ids.
    pub fn insert_columns(
        &mut self,
        book: WindowId,
        sheet: usize,
        at: usize,
        count: usize,
    ) -> Result<Vec<ColumnId>, ProjectError> {
        let ids: Vec<ColumnId> = (0..count).map(|_| ColumnId(self.fresh_id())).collect();
        let ws = self
            .workbook_mut(book)
            .ok_or(ProjectError::NoSuchWindow(book))?
            .sheets
            .get_mut(sheet)
            .ok_or(ProjectError::NoSuchLayer(sheet))?;
        let at = at.min(ws.columns.len());
        for (k, id) in ids.iter().enumerate() {
            let name = ws.next_short_name();
            ws.columns.insert(at + k, Column::new(*id, name, Designation::Y, ColumnData::default()));
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet() -> Worksheet {
        let mut ws = Worksheet::new("Sheet1");
        for (i, name) in ["A", "B", "C"].iter().enumerate() {
            let v: Vec<f64> = (0..4).map(|r| (i * 10 + r) as f64).collect();
            ws.columns.push(Column::new(ColumnId(i as u64 + 1), *name, Designation::Y, ColumnData::numeric(v)));
        }
        ws
    }

    fn nums(ws: &Worksheet, i: usize) -> Vec<f64> {
        ws.columns[i].numeric().unwrap().to_vec()
    }

    #[test]
    fn inserted_rows_are_blank_and_push_the_cells_down_in_the_chosen_columns_only() {
        let mut ws = sheet();
        ws.insert_rows(&[0, 1], 1, 2);
        let a = nums(&ws, 0);
        assert_eq!(a.len(), 6);
        assert!(a[1].is_nan() && a[2].is_nan());
        assert_eq!((a[0], a[3], a[5]), (0.0, 1.0, 3.0));
        assert_eq!(nums(&ws, 2), vec![20.0, 21.0, 22.0, 23.0]);
    }

    #[test]
    fn inserting_below_the_data_changes_nothing() {
        let mut ws = sheet();
        ws.insert_rows(&[0], 4, 3);
        assert_eq!(nums(&ws, 0).len(), 4);
    }

    #[test]
    fn deleted_rows_close_the_gap() {
        let mut ws = sheet();
        ws.delete_rows(&[0, 2], 1..3);
        assert_eq!(nums(&ws, 0), vec![0.0, 3.0]);
        assert_eq!(nums(&ws, 2), vec![20.0, 23.0]);
        assert_eq!(nums(&ws, 1).len(), 4);
    }

    #[test]
    fn text_cells_of_a_mixed_column_move_with_their_rows() {
        let mut texts = BTreeMap::new();
        texts.insert(2, "two".to_string());
        let mut d = ColumnData::mixed(vec![1.0, 2.0, f64::NAN, 4.0], texts);
        d.insert_blank_rows(1, 1);
        assert_eq!(d.text_cell(3), Some("two"));
        d.remove_rows(0..2);
        assert_eq!(d.text_cell(1), Some("two"));
        assert_eq!(d.len(), 3);
    }

    #[test]
    fn formula_columns_are_not_shifted() {
        let mut ws = sheet();
        ws.columns[1].formula = Some("col(A)*2".into());
        ws.columns[1].recalc = crate::column::RecalcMode::Auto;
        assert_eq!(ws.delete_rows(&[0, 1], 0..1), 1);
        assert_eq!(nums(&ws, 1).len(), 4);
        assert_eq!(nums(&ws, 0).len(), 3);
    }

    fn order(ws: &Worksheet) -> String {
        ws.columns.iter().map(|c| c.short_name.as_str()).collect()
    }

    #[test]
    fn columns_move_to_the_ends_and_by_one_step_as_a_block() {
        let mut ws = sheet();
        assert!(ws.move_columns(&[ColumnId(3)], MoveTo::First));
        assert_eq!(order(&ws), "CAB");
        assert!(!ws.move_columns(&[ColumnId(3)], MoveTo::Left));
        assert!(ws.move_columns(&[ColumnId(3), ColumnId(1)], MoveTo::Right));
        assert_eq!(order(&ws), "BCA");
        assert!(ws.move_columns(&[ColumnId(2)], MoveTo::Last));
        assert_eq!(order(&ws), "CAB");
        assert!(!ws.move_columns(&[ColumnId(2)], MoveTo::Right));
    }
}
