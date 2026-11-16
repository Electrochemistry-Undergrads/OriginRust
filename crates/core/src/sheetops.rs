//! Worksheet operations that change the shape of a sheet: inserting and deleting rows and
//! columns, moving and hiding columns.

use crate::analysis::{AnalysisError, row_comparator};
use crate::column::{Column, ColumnData, Designation};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::worksheet::Worksheet;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
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

/// What Column > Mask does to the selected cells.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MaskOp {
    Apply,
    Remove,
    /// Masked cells become unmasked and the others masked.
    Swap,
}

/// Column > Mask Cells by Condition (Origin's `colmask`): which cells of a column to mask.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MaskCondition {
    /// Cells further than this many sample standard deviations from the mean (Origin's default
    /// condition "Outliers by Std. Deviation", multiple 1 to 6).
    OutliersBySd(u8),
    Greater(f64),
    GreaterOrEqual(f64),
    Less(f64),
    LessOrEqual(f64),
    Equal(f64),
}

impl MaskCondition {
    /// The rows of `values` (missing values never match) that meet the condition; `absolute` compares
    /// `|y|` with the absolute value (ignored by the outlier test). Measured against Origin 2016's `colmask`
    /// (workbook-model 13.1a): the outlier test is `|y - mean| > k s` with the sample standard
    /// deviation `s` (divisor n - 1) of all the numbers of the column.
    pub fn rows(&self, values: &[f64], absolute: bool) -> Vec<usize> {
        let ok: Vec<(usize, f64)> = values.iter().copied().enumerate().filter(|(_, v)| v.is_finite()).collect();
        match *self {
            MaskCondition::OutliersBySd(k) => {
                let n = ok.len();
                if n < 2 {
                    return Vec::new();
                }
                let mean = ok.iter().map(|p| p.1).sum::<f64>() / n as f64;
                let var = ok.iter().map(|p| (p.1 - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
                let limit = f64::from(k) * var.sqrt();
                ok.iter().filter(|p| (p.1 - mean).abs() > limit).map(|p| p.0).collect()
            }
            _ => ok
                .iter()
                .filter(|&&(_, v)| {
                    // Origin compares |y| with |value| (masking "|y| > -1" leaves 0 and 1 unmasked).
                    let (y, f) = if absolute {
                        (v.abs(), f64::abs as fn(f64) -> f64)
                    } else {
                        (v, std::convert::identity as fn(f64) -> f64)
                    };
                    match *self {
                        MaskCondition::Greater(t) => y > f(t),
                        MaskCondition::GreaterOrEqual(t) => y >= f(t),
                        MaskCondition::Less(t) => y < f(t),
                        MaskCondition::LessOrEqual(t) => y <= f(t),
                        MaskCondition::Equal(t) => y == f(t),
                        MaskCondition::OutliersBySd(_) => false,
                    }
                })
                .map(|p| p.0)
                .collect(),
        }
    }
}

impl Worksheet {
    /// Masks the cells of these columns (positions) that meet `condition`. Without `keep` the
    /// columns' existing masks are replaced, with it the new cells are added (Origin's Keep Existing
    /// Mask, off by default). Returns the number of cells whose mask changed.
    pub fn mask_by_condition(
        &mut self,
        columns: &[usize],
        condition: MaskCondition,
        absolute: bool,
        keep: bool,
    ) -> usize {
        let mut changed = 0;
        for &i in columns {
            let Some(c) = self.columns.get_mut(i) else { continue };
            let Some(values) = c.numeric() else { continue };
            let hit: BTreeSet<usize> = condition.rows(values, absolute).into_iter().collect();
            let new: BTreeSet<usize> = if keep { c.mask.union(&hit).copied().collect() } else { hit };
            changed += c.mask.symmetric_difference(&new).count();
            c.mask = new;
        }
        changed
    }

    /// Masks, unmasks or swaps the mask of the cells of these columns (positions) and rows.
    /// Cells below the data of a column cannot be masked. Returns the number of cells changed.
    pub fn mask_cells(&mut self, columns: &[usize], rows: Range<usize>, op: MaskOp) -> usize {
        let mut changed = 0;
        for &i in columns {
            let Some(c) = self.columns.get_mut(i) else { continue };
            for r in rows.start..rows.end.min(c.len()) {
                let was = c.mask.contains(&r);
                let now = match op {
                    MaskOp::Apply => true,
                    MaskOp::Remove => false,
                    MaskOp::Swap => !was,
                };
                if now != was {
                    changed += 1;
                    if now {
                        c.mask.insert(r);
                    } else {
                        c.mask.remove(&r);
                    }
                }
            }
        }
        changed
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
        let mut done = Vec::new();
        for &i in columns {
            let Some(c) = self.columns.get_mut(i) else { continue };
            if c.is_locked() {
                locked += 1;
                continue;
            }
            done.push(c.id);
            if at < c.len() {
                c.mask = c.mask.iter().map(|r| if *r >= at { r + count } else { *r }).collect();
            }
            c.data.insert_blank_rows(at, count);
        }
        self.cond_formats_insert_rows(columns, at, count);
        self.notes_insert_rows(&done, at, count);
        self.formats_insert_rows(&done, at, count);
        locked
    }

    /// Deletes these rows from the columns at these positions; the cells below move up.
    /// Returns how many formula columns were skipped.
    pub fn delete_rows(&mut self, columns: &[usize], rows: Range<usize>) -> usize {
        let mut locked = 0;
        let mut done = Vec::new();
        for &i in columns {
            let Some(c) = self.columns.get_mut(i) else { continue };
            if c.is_locked() {
                locked += 1;
                continue;
            }
            done.push(c.id);
            c.mask = c
                .mask
                .iter()
                .filter(|r| !rows.contains(r))
                .map(|r| if *r >= rows.end { r - rows.len() } else { *r })
                .collect();
            c.data.remove_rows(rows.clone());
            c.trim_missing_tail();
        }
        self.cond_formats_delete_rows(columns, rows.clone());
        self.notes_delete_rows(&done, rows.clone());
        self.formats_delete_rows(&done, rows);
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

/// One level of a nested sort: a column and its order.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SortKey {
    pub column: ColumnId,
    pub descending: bool,
}

/// The label row that Worksheet > Sort Columns by Label orders the columns by.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColumnLabel {
    LongName,
    Units,
    Comments,
    ShortName,
}

impl Worksheet {
    /// Puts the columns in the order of one of their labels (case ignored, an empty label first;
    /// columns with equal labels keep their order). Returns false when nothing moved.
    pub fn sort_columns_by_label(&mut self, label: ColumnLabel, descending: bool) -> bool {
        let before: Vec<ColumnId> = self.columns.iter().map(|c| c.id).collect();
        let text = |c: &Column| {
            match label {
                ColumnLabel::LongName => &c.long_name,
                ColumnLabel::Units => &c.units,
                ColumnLabel::Comments => &c.comments,
                ColumnLabel::ShortName => &c.short_name,
            }
            .to_lowercase()
        };
        self.columns.sort_by(|a, b| {
            let o = text(a).cmp(&text(b));
            if descending { o.reverse() } else { o }
        });
        before != self.columns.iter().map(|c| c.id).collect::<Vec<_>>()
    }
}

impl Project {
    /// Nested sort: the rows of these columns (all columns of the sheet when `columns` is empty)
    /// are put in the order of the first key, ties by the second key, and so on. The other
    /// columns of the sheet keep their rows.
    pub fn sort_nested(
        &mut self,
        book: WindowId,
        sheet: usize,
        keys: &[SortKey],
        columns: &[ColumnId],
    ) -> Result<(), AnalysisError> {
        self.sort_nested_in_rows(book, sheet, keys, columns, 0..usize::MAX)
    }

    /// Nested sort restricted to a row interval. Rows outside it keep their values and masks.
    pub fn sort_nested_in_rows(
        &mut self,
        book: WindowId,
        sheet: usize,
        keys: &[SortKey],
        columns: &[ColumnId],
        rows: Range<usize>,
    ) -> Result<(), AnalysisError> {
        let ws = self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(AnalysisError::NoSuchSheet)?;
        let mut levels = Vec::new();
        for k in keys {
            levels.push((ws.column(k.column).ok_or(AnalysisError::NoSuchColumn(k.column))?, k.descending));
        }
        for c in columns {
            ws.column(*c).ok_or(AnalysisError::NoSuchColumn(*c))?;
        }
        let n = ws.row_count();
        let comparators: Vec<_> = levels.iter().map(|(c, d)| row_comparator(&c.data, n, *d)).collect();
        let mut order: Vec<usize> = (0..n).collect();
        let end = rows.end.min(n);
        let start = rows.start.min(end);
        order[start..end].sort_by(|a, b| {
            comparators.iter().map(|cmp| cmp(*a, *b)).find(|o| o.is_ne()).unwrap_or(std::cmp::Ordering::Equal)
        });
        drop(comparators);
        let ws = &mut self.workbook_mut(book).expect("checked above").sheets[sheet];
        for c in &mut ws.columns {
            if columns.is_empty() || columns.contains(&c.id) {
                c.reorder_rows(&order, n);
            }
        }
        ws.notes_reorder_rows(&|c| columns.is_empty() || columns.contains(&c), &order, n);
        ws.formats_reorder_rows(&|c| columns.is_empty() || columns.contains(&c), &order, n);
        // The rows moved, so a filter is applied again to the new order.
        let _ = self.reapply_row_filter(book, sheet);
        Ok(())
    }

    /// Empties every column of a sheet (Worksheet > Clear Worksheet): the columns, their labels,
    /// designations and formulas stay; the values go. Returns how many columns were cleared.
    pub fn clear_sheet(&mut self, book: WindowId, sheet: usize) -> usize {
        let Some(ws) = self.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) else { return 0 };
        for c in &mut ws.columns {
            c.data = match &c.data {
                ColumnData::Numeric(_) => ColumnData::numeric(Vec::new()),
                ColumnData::Text(_) => ColumnData::text(Vec::new()),
                ColumnData::Mixed { .. } => ColumnData::mixed_numbers(Vec::new()),
            };
        }
        ws.columns.len()
    }

    /// Reverses the order of the values of each of these columns on its own (Column > Reverse
    /// Order). Columns kept up to date by a formula are left alone; returns how many.
    pub fn reverse_columns(&mut self, ids: &[ColumnId]) -> usize {
        let mut locked = 0;
        let mut reversed = Vec::new();
        for id in ids {
            let Some(c) = self.column_mut(*id) else { continue };
            if c.is_locked() {
                locked += 1;
                continue;
            }
            let n = c.len();
            let order: Vec<usize> = (0..n).rev().collect();
            c.reorder_rows(&order, n);
            reversed.push((*id, order, n));
        }
        for (id, order, n) in reversed {
            for w in &mut self.windows {
                if let crate::project::Window::Workbook(b) = w {
                    for sheet in &mut b.sheets {
                        sheet.notes_reorder_rows(&|c| c == id, &order, n);
                        sheet.formats_reorder_rows(&|c| c == id, &order, n);
                    }
                }
            }
        }
        locked
    }

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

    #[test]
    fn the_mask_follows_its_rows_through_inserts_deletes_and_sorts() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let k = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![3.0, 1.0, 2.0])).unwrap();
        {
            let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
            assert_eq!(ws.mask_cells(&[0], 0..2, MaskOp::Apply), 2);
            assert_eq!(ws.mask_cells(&[0], 1..9, MaskOp::Swap), 2, "only cells with data swap");
            assert_eq!(ws.columns[0].mask.iter().copied().collect::<Vec<_>>(), vec![0, 2]);
            ws.insert_rows(&[0], 1, 1);
            assert_eq!(ws.columns[0].mask.iter().copied().collect::<Vec<_>>(), vec![0, 3]);
            ws.delete_rows(&[0], 1..2);
            assert_eq!(ws.columns[0].mask.iter().copied().collect::<Vec<_>>(), vec![0, 2]);
        }
        // Values 3 (masked), 1, 2 (masked): sorting ascending moves the masks with the values.
        p.sort_sheet(book, 0, k, false).unwrap();
        assert_eq!(p.numeric(k).unwrap(), [1.0, 2.0, 3.0]);
        let c = p.column(k).unwrap();
        assert_eq!(c.mask.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
        assert!(c.numeric_unmasked().unwrap()[1].is_nan() && c.numeric_unmasked().unwrap()[0] == 1.0);
        let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
        assert_eq!(ws.mask_cells(&[0], 0..3, MaskOp::Remove), 2);
        assert!(ws.columns[0].mask.is_empty());
    }

    #[test]
    fn notes_follow_their_rows_through_sorts_and_reverse() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let k = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![3.0, 1.0, 2.0])).unwrap();
        let o = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![30.0, 10.0, 20.0])).unwrap();
        {
            let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
            ws.set_note(k, 0, "three");
            ws.set_note(o, 1, "ten");
        }
        p.sort_sheet(book, 0, k, false).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!((ws.note(k, 2), ws.note(o, 0)), (Some("three"), Some("ten")));
        // Sorting one column alone moves only that column's notes.
        p.sort_columns(book, 0, &[k], true).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!((ws.note(k, 0), ws.note(o, 0)), (Some("three"), Some("ten")));
        assert_eq!(ws.notes.len(), 2);
        // Sort Range on one column and the key keeps the other columns' notes in place.
        p.sort_range(book, 0, k, &[k], false).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!((ws.note(k, 2), ws.note(o, 0)), (Some("three"), Some("ten")));
        p.reverse_columns(&[o]);
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!((ws.note(k, 2), ws.note(o, 2)), (Some("three"), Some("ten")));
    }

    #[test]
    fn cell_formats_follow_their_rows_through_sorts_inserts_and_deletes() {
        use crate::cellformat::{CellFormat, FormatTarget};
        let bold = CellFormat { bold: Some(true), ..Default::default() };
        let mut p = Project::new();
        let book = p.add_workbook("");
        let k = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![3.0, 1.0, 2.0])).unwrap();
        let o = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![30.0, 10.0, 20.0])).unwrap();
        {
            let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
            ws.apply_cell_format(FormatTarget::cells(vec![k], 0, Some(0)), bold.clone());
            ws.apply_cell_format(FormatTarget::cells(vec![o], 1, Some(1)), bold.clone());
            ws.apply_cell_format(
                FormatTarget::whole_columns(vec![k]),
                CellFormat { italic: Some(true), ..Default::default() },
            );
        }
        let at = |p: &Project, c, r| p.workbook(book).unwrap().sheets[0].cell_format_at(c, r).bold;
        p.sort_sheet(book, 0, k, false).unwrap();
        assert!(at(&p, k, 2) && !at(&p, k, 0) && at(&p, o, 0) && !at(&p, o, 1));
        assert!(p.workbook(book).unwrap().sheets[0].cell_format_at(k, 1).italic);
        // Sorting one column alone moves only that column's formats.
        p.sort_columns(book, 0, &[k], true).unwrap();
        assert!(at(&p, k, 0) && !at(&p, k, 2) && at(&p, o, 0));
        p.reverse_columns(&[o]);
        assert!(at(&p, o, 2) && !at(&p, o, 0));
        // Insert rows above and delete rows.
        let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
        ws.insert_rows(&[0, 1], 0, 2);
        assert!(at(&p, k, 2) && at(&p, o, 4));
        let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
        ws.delete_rows(&[0, 1], 0..3);
        assert!(!at(&p, k, 2) && at(&p, o, 1) && !at(&p, o, 0));
    }

    #[test]
    fn disable_masking_is_one_switch_that_new_columns_follow() {
        let mut doc = crate::history::Document::new(Project::new());
        let (book, a) = doc.apply("data", |p| {
            let book = p.add_workbook("");
            let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
            p.column_mut(a).unwrap().mask.insert(0);
            (book, a)
        });
        doc.apply("off", |p| p.set_masking_disabled(true));
        assert!(doc.project().masking_disabled());
        assert!(doc.project().column(a).unwrap().active_mask().is_none());
        // a column made later (as by Unstack or an import) follows the switch too
        let b = doc.apply("new column", |p| {
            let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![3.0, 4.0])).unwrap();
            p.column_mut(b).unwrap().mask.insert(1);
            b
        });
        assert!(doc.project().column(b).unwrap().active_mask().is_none(), "new columns follow the switch");
        doc.apply("on", |p| p.set_masking_disabled(false));
        assert!(doc.project().column(b).unwrap().numeric_unmasked().unwrap()[1].is_nan());
    }

    #[test]
    fn mask_by_condition_follows_origins_colmask() {
        // Origin 2016 `colmask` on 2 4 9 3 5 -7 1 6 8 0 (rows as Origin reported, 0-based here).
        let y = [2.0, 4.0, 9.0, 3.0, 5.0, -7.0, 1.0, 6.0, 8.0, 0.0];
        let r = |c: MaskCondition, abs: bool| c.rows(&y, abs);
        assert_eq!(r(MaskCondition::OutliersBySd(1), false), vec![2, 5, 8]);
        assert_eq!(r(MaskCondition::OutliersBySd(2), false), vec![5]);
        assert!(r(MaskCondition::OutliersBySd(3), false).is_empty());
        assert_eq!(r(MaskCondition::Greater(5.0), false), vec![2, 7, 8]);
        assert_eq!(r(MaskCondition::Greater(5.0), true), vec![2, 5, 7, 8]);
        assert_eq!(r(MaskCondition::GreaterOrEqual(5.0), false), vec![2, 4, 7, 8]);
        assert_eq!(r(MaskCondition::Less(3.0), false), vec![0, 5, 6, 9]);
        assert_eq!(r(MaskCondition::Less(3.0), true), vec![0, 6, 9]);
        assert_eq!(r(MaskCondition::LessOrEqual(3.0), false), vec![0, 3, 5, 6, 9]);
        assert_eq!(r(MaskCondition::Equal(5.0), false), vec![4]);
        assert_eq!(r(MaskCondition::Greater(-1.0), true), vec![0, 1, 2, 3, 4, 5, 7, 8]);
        // nine zeros and a 10: |10 - 1| = 9 > 2 s (s = sqrt(10)) but not > 3 s, as in Origin
        let z = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 10.0];
        assert_eq!(MaskCondition::OutliersBySd(2).rows(&z, false), vec![9]);
        assert!(MaskCondition::OutliersBySd(3).rows(&z, false).is_empty());
        // Keep Existing Mask: off replaces the old mask, on adds to it
        let mut p = Project::new();
        let book = p.add_workbook("");
        let k = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y.to_vec())).unwrap();
        p.column_mut(k).unwrap().mask.insert(0);
        let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
        assert_eq!(ws.mask_by_condition(&[0], MaskCondition::Greater(8.0), false, true), 1);
        assert_eq!(ws.columns[0].mask.iter().copied().collect::<Vec<_>>(), vec![0, 2]);
        assert_eq!(ws.mask_by_condition(&[0], MaskCondition::Greater(8.0), false, false), 1);
        assert_eq!(ws.columns[0].mask.iter().copied().collect::<Vec<_>>(), vec![2]);
    }

    #[test]
    fn nested_sort_breaks_ties_with_the_next_key_and_keeps_other_columns_in_place() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![2.0, 1.0, 2.0, 1.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 9.0, 3.0, 5.0])).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0, 30.0, 40.0])).unwrap();
        let keys = [SortKey { column: a, descending: false }, SortKey { column: b, descending: true }];
        p.sort_nested(book, 0, &keys, &[]).unwrap();
        assert_eq!(p.numeric(a).unwrap(), [1.0, 1.0, 2.0, 2.0]);
        assert_eq!(p.numeric(b).unwrap(), [9.0, 5.0, 3.0, 1.0]);
        assert_eq!(p.numeric(c).unwrap(), [20.0, 40.0, 30.0, 10.0]);
        // As a range of two columns, column C stays.
        p.sort_nested(book, 0, &[SortKey { column: b, descending: false }], &[a, b]).unwrap();
        assert_eq!(p.numeric(b).unwrap(), [1.0, 3.0, 5.0, 9.0]);
        assert_eq!(p.numeric(c).unwrap(), [20.0, 40.0, 30.0, 10.0]);
    }

    #[test]
    fn reversing_a_column_turns_it_round_and_columns_sort_by_a_label() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, f64::NAN, 4.0])).unwrap();
        assert_eq!(p.reverse_columns(&[a]), 0);
        let v = p.numeric(a).unwrap();
        assert_eq!((v[0], v[2], v[3]), (4.0, 2.0, 1.0));
        assert!(v[1].is_nan());

        let ws = &mut p.workbook_mut(book).unwrap().sheets[0];
        ws.columns
            .push(Column::new(ColumnId(900), "B", Designation::Y, ColumnData::default()).with_labels("alpha", "V", ""));
        ws.columns[0].long_name = "Zeta".into();
        assert!(ws.sort_columns_by_label(ColumnLabel::LongName, false));
        assert_eq!(order(ws), "BA");
        assert!(!ws.sort_columns_by_label(ColumnLabel::LongName, false));
        assert!(ws.sort_columns_by_label(ColumnLabel::LongName, true));
        assert_eq!(order(ws), "AB");
    }
}
