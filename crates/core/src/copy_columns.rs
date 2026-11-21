//! Copy Columns to... (T89), as Origin 2016's `colcopy` (Copy Data, Copy Format, Long Name, Units,
//! Comments, User-Defined Parameters; destination a new book, a new sheet of a book, or a column of
//! a sheet). Measured on 2026-11-16 (`docs/spec/workbook-model.md`, "Copy Columns to"):
//!
//! - With no destination the copies go to a new book, named A, B, ... from the first column, with
//!   their designation (a Y-error column stays one), long name, units, comments and display format;
//!   the formula is not copied (the values are).
//! - Into a column of a sheet the copies overwrite that column and the ones to its right, adding
//!   columns where the sheet ends; an overwritten column keeps its short name, a new one takes the
//!   next free letter. The sheet grows to the rows of the copies.
//! - To `<new>` the copies are added after the last column; to `<new>!<new>` they fill a new sheet.

use crate::analysis::{free_sheet_name, take_rows};
use crate::column::{Column, ColumnData};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::worksheet::Worksheet;
use std::collections::HashMap;

/// Where Copy Columns to puts the copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyDestination {
    /// A new workbook (Origin's default).
    NewBook,
    /// A new sheet added to this workbook.
    NewSheet(WindowId),
    /// Sheet `sheet` of a workbook, from column position `at` (overwriting); `None` adds them after
    /// the last column.
    Sheet { book: WindowId, sheet: usize, at: Option<usize> },
}

/// What is copied of each column (Origin's check boxes; all on by default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyParts {
    pub data: bool,
    /// Copy only source rows shown by the worksheet filter.
    pub ignore_hidden_rows: bool,
    pub format: bool,
    pub long_name: bool,
    pub units: bool,
    pub comments: bool,
    pub params: bool,
}

impl Default for CopyParts {
    fn default() -> Self {
        CopyParts {
            data: true,
            ignore_hidden_rows: true,
            format: true,
            long_name: true,
            units: true,
            comments: true,
            params: true,
        }
    }
}

impl Project {
    /// Copies `sources` (in this order) to `dest`. Returns the workbook and the ids of the columns
    /// written.
    pub fn copy_columns_to(
        &mut self,
        sources: &[ColumnId],
        dest: CopyDestination,
        parts: CopyParts,
    ) -> Result<(WindowId, Vec<ColumnId>), ProjectError> {
        // Snapshot all sources before touching the destination, which can overlap them.
        let mut shown_by_sheet = HashMap::new();
        let copies: Vec<Column> = sources
            .iter()
            .map(|&c| -> Result<Column, ProjectError> {
                let (_, sheet, source) = self.locate(c).ok_or(ProjectError::NoSuchColumn(c))?;
                let mut copy = source.clone();
                if parts.data
                    && parts.ignore_hidden_rows
                    && let Some(shown) = shown_by_sheet
                        .entry(std::ptr::from_ref(sheet))
                        .or_insert_with(|| sheet.shown_rows(sheet.row_count()))
                {
                    let len = source.data.len();
                    // The visible row order is shared by all columns of this sheet. A short
                    // column stops after its last stored row.
                    let end = shown.partition_point(|&r| r < len);
                    if end < len {
                        copy.data = take_rows(&source.data, &shown[..end], len);
                    }
                }
                Ok(copy)
            })
            .collect::<Result<_, _>>()?;
        let (book, sheet, at) = match dest {
            CopyDestination::NewBook => {
                let long = self.locate(sources[0]).map(|(b, _, _)| b.long_name.clone()).unwrap_or_default();
                (self.add_workbook(&long), 0, Some(0))
            }
            CopyDestination::NewSheet(book) => {
                let b = self.workbook_mut(book).ok_or(ProjectError::NoSuchWindow(book))?;
                let name = free_sheet_name(&b.sheets, "Sheet");
                b.sheets.push(Worksheet::new(&name));
                (book, b.sheets.len() - 1, Some(0))
            }
            CopyDestination::Sheet { book, sheet, at } => (book, sheet, at),
        };
        let mut ids = Vec::new();
        for (k, src) in copies.into_iter().enumerate() {
            let fresh = ColumnId(self.fresh_id());
            let ws = self
                .workbook_mut(book)
                .ok_or(ProjectError::NoSuchWindow(book))?
                .sheets
                .get_mut(sheet)
                .ok_or(ProjectError::NoSuchLayer(sheet))?;
            let pos = at.map(|a| a + k).filter(|&p| p < ws.columns.len());
            let target = match pos {
                Some(p) => &mut ws.columns[p],
                None => {
                    let name = ws.next_short_name();
                    ws.columns.push(Column::new(fresh, name, src.designation, ColumnData::default()));
                    ws.columns.last_mut().expect("just pushed")
                }
            };
            target.designation = src.designation;
            if parts.data {
                target.data = src.data.clone();
                target.formula = None;
                target.before_formula = None;
            }
            if parts.format {
                target.format = src.format;
                target.width = src.width;
            }
            if parts.long_name {
                target.long_name = src.long_name.clone();
            }
            if parts.units {
                target.units = src.units.clone();
            }
            if parts.comments {
                target.comments = src.comments.clone();
            }
            if parts.params {
                target.params = src.params.clone();
            }
            ids.push(target.id);
        }
        Ok((book, ids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Designation;

    /// Origin's probe book Src: A 1..4 (X), B 10..40 (Bee, mA), C = i*2 (Y error), D 7..9.
    fn src() -> (Project, WindowId, Vec<ColumnId>) {
        let mut p = Project::new();
        let b = p.add_workbook("Src");
        let a = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let bb = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0, 30.0, 40.0])).unwrap();
        let c = p.add_column(b, 0, Designation::YError, ColumnData::numeric(vec![2.0, 4.0, 6.0, 8.0])).unwrap();
        let d = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![7.0, 8.0, 9.0])).unwrap();
        {
            let col = p.column_mut(bb).unwrap();
            col.long_name = "Bee".into();
            col.units = "mA".into();
        }
        p.column_mut(c).unwrap().formula = Some("i*2".into());
        (p, b, vec![a, bb, c, d])
    }

    fn names(p: &Project, book: WindowId, sheet: usize) -> Vec<(String, Designation)> {
        p.workbook(book).unwrap().sheets[sheet].columns.iter().map(|c| (c.short_name.clone(), c.designation)).collect()
    }

    #[test]
    fn a_new_book_gets_the_columns_renamed_from_a_with_labels_but_no_formula() {
        let (mut p, _, c) = src();
        let (book, ids) = p.copy_columns_to(&[c[1], c[2]], CopyDestination::NewBook, CopyParts::default()).unwrap();
        assert_eq!(names(&p, book, 0), [("A".into(), Designation::Y), ("B".into(), Designation::YError)]);
        let first = p.column(ids[0]).unwrap();
        assert_eq!((first.long_name.as_str(), first.units.as_str()), ("Bee", "mA"));
        let second = p.column(ids[1]).unwrap();
        assert_eq!(second.formula, None, "Origin copies the values, not the formula");
        assert_eq!(p.numeric(ids[1]).unwrap(), [2.0, 4.0, 6.0, 8.0]);
    }

    #[test]
    fn into_a_column_the_copies_overwrite_rightward_and_add_columns() {
        let (mut p, _, c) = src();
        let dst = p.add_workbook("Dst");
        p.add_column(dst, 0, Designation::X, ColumnData::numeric(vec![100.0, 101.0, 102.0])).unwrap();
        p.add_column(dst, 0, Designation::Y, ColumnData::numeric(vec![200.0, 201.0, 202.0])).unwrap();
        let to_b = CopyDestination::Sheet { book: dst, sheet: 0, at: Some(1) };
        p.copy_columns_to(&[c[1], c[2]], to_b, CopyParts::default()).unwrap();
        // Origin: A 100..102, B 10..40 (Bee), C 2..8 (Y error).
        assert_eq!(
            names(&p, dst, 0),
            [("A".into(), Designation::X), ("B".into(), Designation::Y), ("C".into(), Designation::YError)]
        );
        let ws = &p.workbook(dst).unwrap().sheets[0];
        assert_eq!(ws.columns[1].numeric().unwrap(), [10.0, 20.0, 30.0, 40.0]);
        assert_eq!(ws.columns[1].long_name, "Bee");
        // <new>: after the last column, the next letter.
        let append = CopyDestination::Sheet { book: dst, sheet: 0, at: None };
        let (_, ids) = p.copy_columns_to(&[c[3]], append, CopyParts::default()).unwrap();
        assert_eq!(p.column(ids[0]).unwrap().short_name, "D");
        assert_eq!(p.numeric(ids[0]).unwrap(), [7.0, 8.0, 9.0]);
        // <new>!<new>: a new sheet.
        let (_, ids) = p.copy_columns_to(&[c[1]], CopyDestination::NewSheet(dst), CopyParts::default()).unwrap();
        let book = p.workbook(dst).unwrap();
        assert_eq!(book.sheets.len(), 2);
        assert_eq!(book.sheets[1].name, "Sheet2");
        assert_eq!(book.sheets[1].columns[0].id, ids[0]);
    }

    #[test]
    fn unticked_parts_are_left_as_they_were() {
        let (mut p, _, c) = src();
        let dst = p.add_workbook("Dst");
        let keep = p.add_column(dst, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        p.column_mut(keep).unwrap().long_name = "Mine".into();
        let parts = CopyParts { long_name: false, ..CopyParts::default() };
        p.copy_columns_to(&[c[1]], CopyDestination::Sheet { book: dst, sheet: 0, at: Some(0) }, parts).unwrap();
        let col = p.column(keep).unwrap();
        assert_eq!((col.long_name.as_str(), col.units.as_str()), ("Mine", "mA"));
        assert_eq!(col.numeric().unwrap(), [10.0, 20.0, 30.0, 40.0]);
    }

    #[test]
    fn visible_column_copy_keeps_numeric_text_and_mixed_rows_aligned() {
        use std::collections::BTreeMap;

        let (mut p, book, c) = src();
        p.column_mut(c[2]).unwrap().data = ColumnData::text(vec!["first".into(), "hidden".into(), "third".into()]);
        p.column_mut(c[3]).unwrap().data =
            ColumnData::mixed(vec![7.0, 8.0, f64::NAN, 9.0], BTreeMap::from([(2, "label".into())]));
        p.set_row_filter(book, 0, "col(A) != 2").unwrap();
        let source = p.workbook(book).unwrap().clone();

        let (_, copied) = p.copy_columns_to(&c[1..], CopyDestination::NewBook, CopyParts::default()).unwrap();
        assert_eq!(p.numeric(copied[0]).unwrap(), [10.0, 30.0, 40.0]);
        assert_eq!(p.column(copied[1]).unwrap().text().unwrap(), ["first", "third"]);
        let mixed = &p.column(copied[2]).unwrap().data;
        assert_eq!(mixed.len(), 3);
        assert_eq!(mixed.text_cell(1), Some("label"));
        assert_eq!(p.numeric(copied[2]).unwrap()[2], 9.0);
        assert_eq!(p.workbook(book), Some(&source));

        let all = CopyParts { ignore_hidden_rows: false, ..CopyParts::default() };
        let (_, copied) = p.copy_columns_to(&[c[1]], CopyDestination::NewBook, all).unwrap();
        assert_eq!(p.numeric(copied[0]).unwrap(), [10.0, 20.0, 30.0, 40.0]);
        p.workbook_mut(book).unwrap().sheets[0].filter.as_mut().unwrap().enabled = false;
        let (_, copied) = p.copy_columns_to(&[c[1]], CopyDestination::NewBook, CopyParts::default()).unwrap();
        assert_eq!(p.numeric(copied[0]).unwrap(), [10.0, 20.0, 30.0, 40.0]);
    }

    #[test]
    fn column_copy_uses_each_source_filter_before_overwriting_any_column() {
        let (mut p, book, c) = src();
        let other = p.add_workbook("Other");
        let y = p.add_column(other, 0, Designation::Y, ColumnData::numeric(vec![100.0, 200.0])).unwrap();
        p.set_row_filter(book, 0, "col(A) > 1").unwrap();
        p.set_row_filter(other, 0, "col(A) > 200").unwrap();
        let to_a = CopyDestination::Sheet { book, sheet: 0, at: Some(0) };
        p.copy_columns_to(&[c[1], c[0], y], to_a, CopyParts::default()).unwrap();
        assert_eq!(p.numeric(c[0]).unwrap(), [20.0, 30.0, 40.0]);
        assert_eq!(p.numeric(c[1]).unwrap(), [2.0, 3.0, 4.0]);
        assert!(p.numeric(c[2]).unwrap().is_empty());
        assert_eq!(p.numeric(y).unwrap(), [100.0, 200.0]);
    }
}
