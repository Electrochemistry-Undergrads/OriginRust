//! The sheet tab's Sheet Name to Show and Duplicate Without Data (T90), as Origin 2016's. Measured on
//! 2026-11-16 (`docs/spec/workbook-model.md`, "Sheet tab names"):
//!
//! - A sheet has a short name and a long name (`wks.longname$`). Sheet Name to Show offers Long Name if
//!   available, Short Name (ticked for a new sheet) and Long Name; the choice is per sheet (choosing
//!   Long Name on Sheet1 left Sheet2's tab as "Sheet2" although Sheet2 had a long name).
//! - Duplicate Without Data adds the copy after the last sheet, named as a new sheet (Sheet3 next to
//!   Sheet1 and Sheet2), with the long name followed by " 1" ("Potential scan" gave "Potential scan 1"),
//!   and makes it the active sheet. It keeps the columns with their names, designations, labels and
//!   formulas; the cells are empty.

use crate::analysis::free_sheet_name;
use crate::column::ColumnData;
use crate::ids::WindowId;
use crate::project::{Project, ProjectError};
use crate::worksheet::{Workbook, Worksheet};
use serde::{Deserialize, Serialize};

/// What a sheet's tab shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SheetLabel {
    /// The long name, or the short name when there is none.
    LongIfAvailable,
    #[default]
    Short,
    /// The long name (the short name when it is empty, so the tab can still be found; Origin's
    /// empty case was not measured).
    Long,
}

impl SheetLabel {
    pub const ALL: [SheetLabel; 3] = [SheetLabel::LongIfAvailable, SheetLabel::Short, SheetLabel::Long];

    pub fn is_default(&self) -> bool {
        *self == SheetLabel::Short
    }

    /// The menu text, as Origin's.
    pub fn name(self) -> &'static str {
        match self {
            SheetLabel::LongIfAvailable => "Long Name if available",
            SheetLabel::Short => "Short Name",
            SheetLabel::Long => "Long Name",
        }
    }
}

impl Worksheet {
    /// The text on the sheet's tab.
    pub fn tab_text(&self) -> &str {
        match self.tab_label {
            SheetLabel::Short => &self.name,
            SheetLabel::LongIfAvailable | SheetLabel::Long if self.long_name.trim().is_empty() => &self.name,
            SheetLabel::LongIfAvailable | SheetLabel::Long => &self.long_name,
        }
    }
}

/// Where Navigate Worksheets' Actions move the selected sheets (T180, Origin's Move Before, Move to
/// Behind and Move to End).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetMove {
    /// One place towards the first sheet.
    Before,
    /// One place towards the last sheet.
    Behind,
    /// After every other sheet, in their present order.
    End,
}

impl Workbook {
    /// The sheet order (old indices) after moving the sheets `picked` as `to` says, for
    /// [`Workbook::reorder_sheets`]. A sheet does not pass another picked sheet or the ends, so
    /// moving a block keeps it together; unknown indices are ignored.
    pub fn moved_order(&self, picked: &[usize], to: SheetMove) -> Vec<usize> {
        let n = self.sheets.len();
        let is_picked = |i: usize| picked.contains(&i);
        let mut order: Vec<usize> = (0..n).collect();
        match to {
            SheetMove::Before => {
                for pos in 1..n {
                    if is_picked(order[pos]) && !is_picked(order[pos - 1]) {
                        order.swap(pos, pos - 1);
                    }
                }
            }
            SheetMove::Behind => {
                for pos in (0..n.saturating_sub(1)).rev() {
                    if is_picked(order[pos]) && !is_picked(order[pos + 1]) {
                        order.swap(pos, pos + 1);
                    }
                }
            }
            SheetMove::End => {
                order = (0..n).filter(|&i| !is_picked(i)).chain((0..n).filter(|&i| is_picked(i))).collect();
            }
        }
        order
    }

    /// Reorders whole worksheets by their old indices, keeping the active worksheet selected.
    /// Returns whether the order changed. A malformed permutation leaves the workbook intact.
    pub fn reorder_sheets(&mut self, order: &[usize]) -> bool {
        let len = self.sheets.len();
        if order.len() != len || order.iter().copied().eq(0..len) {
            return false;
        }
        let mut seen = vec![false; len];
        for &index in order {
            if index >= len || seen[index] {
                return false;
            }
            seen[index] = true;
        }
        let active = self.active_sheet.min(len - 1);
        self.active_sheet = order.iter().position(|&index| index == active).expect("validated permutation");
        let mut sheets: Vec<_> = std::mem::take(&mut self.sheets).into_iter().map(Some).collect();
        self.sheets = order.iter().map(|&index| sheets[index].take().expect("validated permutation")).collect();
        true
    }
}

impl Project {
    /// Duplicate Without Data: a copy of sheet `sheet`'s columns and labels without values, added after
    /// the last sheet. Returns the new sheet's index.
    pub fn duplicate_sheet_without_data(&mut self, book: WindowId, sheet: usize) -> Result<usize, ProjectError> {
        let (mut copy, name, long) = {
            let b = self.workbook(book).ok_or(ProjectError::NoSuchWindow(book))?;
            let src = b.sheets.get(sheet).ok_or(ProjectError::NoSuchLayer(sheet))?;
            let long = if src.long_name.is_empty() {
                String::new()
            } else {
                (1..)
                    .map(|n| format!("{} {n}", src.long_name))
                    .find(|l| !b.sheets.iter().any(|s| &s.long_name == l))
                    .expect("the sequence is unbounded")
            };
            (src.clone(), free_sheet_name(&b.sheets, "Sheet"), long)
        };
        copy.name = name;
        copy.long_name = long;
        copy.filter = None;
        copy.notes.clear();
        copy.comparison_source = None;
        let mut renamed = std::collections::HashMap::new();
        for c in &mut copy.columns {
            let new = self.new_column_id();
            renamed.insert(c.id, new);
            c.id = new;
            c.data = match c.data {
                ColumnData::Text(_) => ColumnData::text(Vec::new()),
                ColumnData::Mixed { .. } => ColumnData::mixed_numbers(Vec::new()),
                ColumnData::Numeric(_) => ColumnData::numeric(Vec::new()),
            };
            c.mask.clear();
            c.analysis = None;
        }
        copy.remap_cond_format_columns(&|c| renamed.get(&c).copied());
        copy.remap_format_columns(&|c| renamed.get(&c).copied());
        let b = self.workbook_mut(book).ok_or(ProjectError::NoSuchWindow(book))?;
        b.sheets.push(copy);
        b.active_sheet = b.sheets.len() - 1;
        Ok(b.active_sheet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Designation;

    /// Origin's probe book: Sheet1 "Potential scan" with A (X), B (Bee, mA), C (Y error, F(x) i*2),
    /// and an empty Sheet2.
    fn book() -> (Project, WindowId) {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let bb = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0, 30.0, 40.0])).unwrap();
        let c = p.add_column(b, 0, Designation::YError, ColumnData::numeric(vec![2.0, 4.0, 6.0, 8.0])).unwrap();
        p.column_mut(bb).unwrap().long_name = "Bee".into();
        p.column_mut(bb).unwrap().units = "mA".into();
        p.column_mut(c).unwrap().formula = Some("i*2".into());
        let wb = p.workbook_mut(b).unwrap();
        wb.sheets[0].long_name = "Potential scan".into();
        wb.sheets.push(Worksheet::new("Sheet2"));
        (p, b)
    }

    #[test]
    fn duplicate_without_data_as_origin() {
        let (mut p, b) = book();
        let at = p.duplicate_sheet_without_data(b, 0).unwrap();
        let wb = p.workbook(b).unwrap();
        assert_eq!(at, 2, "after the last sheet");
        assert_eq!(wb.active_sheet, 2);
        let s = &wb.sheets[2];
        assert_eq!((s.name.as_str(), s.long_name.as_str()), ("Sheet3", "Potential scan 1"));
        let names: Vec<(&str, Designation)> =
            s.columns.iter().map(|c| (c.short_name.as_str(), c.designation)).collect();
        assert_eq!(names, [("A", Designation::X), ("B", Designation::Y), ("C", Designation::YError)]);
        assert_eq!((s.columns[1].long_name.as_str(), s.columns[1].units.as_str()), ("Bee", "mA"));
        assert_eq!(s.columns[2].formula.as_deref(), Some("i*2"));
        assert!(s.columns.iter().all(|c| c.data.is_empty()), "no values");
        // New column ids: the original is untouched.
        assert_ne!(s.columns[0].id, wb.sheets[0].columns[0].id);
        assert_eq!(wb.sheets[0].columns[1].numeric().unwrap(), [10.0, 20.0, 30.0, 40.0]);
        // A second copy takes the next long name.
        p.duplicate_sheet_without_data(b, 0).unwrap();
        assert_eq!(p.workbook(b).unwrap().sheets[3].long_name, "Potential scan 2");
    }

    #[test]
    fn reordering_sheets_preserves_their_contents_and_the_active_sheet() {
        let (mut project, book_id) = book();
        let book = project.workbook_mut(book_id).unwrap();
        book.sheets[0].comments = "Calibration notes".into();
        book.sheets.push(Worksheet::new("Sheet3"));
        let original = book.clone();
        for bad in [&[0, 1][..], &[2, 2, 0], &[1, 2, 3], &[0, 1, 2]] {
            assert!(!book.reorder_sheets(bad));
            assert_eq!(*book, original);
        }
        assert!(book.reorder_sheets(&[1, 2, 0]));
        assert_eq!(book.active_sheet, 2);
        assert_eq!(book.sheets, [original.sheets[1].clone(), original.sheets[2].clone(), original.sheets[0].clone()]);
    }

    #[test]
    fn the_tab_shows_the_chosen_name_per_sheet() {
        let (mut p, b) = book();
        let wb = p.workbook_mut(b).unwrap();
        wb.sheets[1].long_name = "Second".into();
        assert_eq!(wb.sheets[0].tab_text(), "Sheet1", "Short Name for a new sheet");
        wb.sheets[0].tab_label = SheetLabel::Long;
        assert_eq!((wb.sheets[0].tab_text(), wb.sheets[1].tab_text()), ("Potential scan", "Sheet2"));
        wb.sheets[1].tab_label = SheetLabel::LongIfAvailable;
        assert_eq!(wb.sheets[1].tab_text(), "Second");
        wb.sheets[1].long_name.clear();
        assert_eq!(wb.sheets[1].tab_text(), "Sheet2");
        let json = serde_json::to_string(&wb.sheets[0]).unwrap();
        let back: Worksheet = serde_json::from_str(&json).unwrap();
        assert_eq!((back.long_name.as_str(), back.tab_label), ("Potential scan", SheetLabel::Long));
        assert!(!serde_json::to_string(&Worksheet::new("S")).unwrap().contains("tab_label"));
    }

    #[test]
    fn moving_sheets_keeps_picked_blocks_together_and_stops_at_the_ends() {
        let mut b = Workbook::new(WindowId(1), "Book1");
        b.sheets = (1..=5).map(|k| crate::Worksheet::new(format!("Sheet{k}"))).collect();
        assert_eq!(b.moved_order(&[2], SheetMove::Before), [0, 2, 1, 3, 4]);
        assert_eq!(b.moved_order(&[0, 3], SheetMove::Before), [0, 1, 3, 2, 4]);
        assert_eq!(b.moved_order(&[1, 2], SheetMove::Before), [1, 2, 0, 3, 4]);
        assert_eq!(b.moved_order(&[3, 4], SheetMove::Behind), [0, 1, 2, 3, 4]);
        assert_eq!(b.moved_order(&[1, 3], SheetMove::Behind), [0, 2, 1, 4, 3]);
        assert_eq!(b.moved_order(&[0, 2], SheetMove::End), [1, 3, 4, 0, 2]);
        assert_eq!(b.moved_order(&[9], SheetMove::End), [0, 1, 2, 3, 4]);
        b.active_sheet = 2;
        assert!(b.reorder_sheets(&b.moved_order(&[2], SheetMove::End)));
        assert_eq!(b.sheets[4].name, "Sheet3");
        assert_eq!(b.active_sheet, 4, "the active sheet stays active");
    }
}
