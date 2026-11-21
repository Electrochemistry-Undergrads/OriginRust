//! File > Import > Re-Import Directly and Re-Import... (T98): the data of a workbook read again from its
//! source file, as Origin 2016's `reimport`. Measured on 2026-11-16 (`docs/spec/workbook-model.md`,
//! "Re-Import"):
//!
//! - The values are replaced by the file's (three rows became four); the graphs of the book follow.
//! - The book keeps its columns: a column the file now has in addition was not added (the book stayed at
//!   three columns when the file grew a fourth).
//! - Labels the user gave that the file does not hold stay (a comment and units typed on column B were
//!   still there); the long name read from the file's header stays the file's.
//!
//! Sheaf's rules for what was not measured: a column the file no longer has is emptied; extra sheets are
//! left alone.

use crate::column::ColumnData;
use crate::ids::WindowId;
use crate::project::{Project, ProjectError};
use crate::worksheet::Workbook;

/// What a re-import changed, for the status line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReimportReport {
    pub columns: usize,
    pub rows: usize,
    /// Columns of the file beyond the book's, which are not added.
    pub ignored: usize,
    /// Columns of the book the file no longer has, now empty.
    pub emptied: usize,
}

impl Project {
    /// Replaces the data of `book` with that of `fresh` (the same file read again), sheet by sheet and
    /// column by column, keeping the book's columns, their ids (so plots keep their data) and the labels
    /// the file does not give.
    pub fn replace_book_data(&mut self, book: WindowId, fresh: &Workbook) -> Result<ReimportReport, ProjectError> {
        let b = self.workbook_mut(book).ok_or(ProjectError::NoSuchWindow(book))?;
        let mut report = ReimportReport::default();
        for (old, new) in b.sheets.iter_mut().zip(&fresh.sheets) {
            report.ignored += new.columns.len().saturating_sub(old.columns.len());
            for (j, col) in old.columns.iter_mut().enumerate() {
                match new.columns.get(j) {
                    Some(src) => {
                        col.data = src.data.clone();
                        for (mine, theirs) in [
                            (&mut col.long_name, &src.long_name),
                            (&mut col.units, &src.units),
                            (&mut col.comments, &src.comments),
                        ] {
                            if !theirs.is_empty() {
                                mine.clone_from(theirs);
                            }
                        }
                        report.columns += 1;
                        report.rows = report.rows.max(src.data.len());
                    }
                    None => {
                        col.data = match col.data {
                            ColumnData::Text(_) => ColumnData::text(Vec::new()),
                            _ => ColumnData::numeric(Vec::new()),
                        };
                        report.emptied += 1;
                    }
                }
                col.formula = None;
                col.before_formula = None;
                col.mask.clear();
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{Column, Designation};
    use crate::ids::ColumnId;
    use crate::template::Template;
    use crate::worksheet::Worksheet;

    fn file(cols: &[(&str, Vec<f64>)]) -> Workbook {
        let mut b = Workbook::new(WindowId(0), "a");
        let mut s = Worksheet::new("a");
        for (k, (name, v)) in cols.iter().enumerate() {
            let d = if k == 0 { Designation::X } else { Designation::Y };
            let mut c = Column::new(
                ColumnId(k as u64 + 1),
                crate::column::default_short_name(k),
                d,
                ColumnData::numeric(v.clone()),
            );
            c.long_name = (*name).into();
            s.columns.push(c);
        }
        b.sheets = vec![s];
        b
    }

    #[test]
    fn values_change_columns_and_user_labels_stay_as_origin() {
        // Origin's probe: x,y,z with three rows; the user typed a comment and units on B and plotted it;
        // the file then held x,y,z,w with four rows.
        let mut p = Project::new();
        let first =
            file(&[("x", vec![1.0, 2.0, 3.0]), ("y", vec![10.0, 20.0, 30.0]), ("z", vec![100.0, 200.0, 300.0])]);
        let book = p.adopt_workbook(first);
        let ids: Vec<ColumnId> = p.workbook(book).unwrap().sheets[0].columns.iter().map(|c| c.id).collect();
        p.column_mut(ids[1]).unwrap().comments = "my note".into();
        p.column_mut(ids[1]).unwrap().units = "mA".into();
        let g = p.plot_columns(Template::Line, &[ids[1]]).unwrap();
        let again = file(&[
            ("x", vec![1.0, 2.0, 3.0, 4.0]),
            ("y", vec![11.0, 21.0, 31.0, 41.0]),
            ("z", vec![101.0, 201.0, 301.0, 401.0]),
            ("w", vec![7.0, 8.0, 9.0, 10.0]),
        ]);
        let r = p.replace_book_data(book, &again).unwrap();
        assert_eq!(r, ReimportReport { columns: 3, rows: 4, ignored: 1, emptied: 0 });
        let s = &p.workbook(book).unwrap().sheets[0];
        assert_eq!(s.columns.len(), 3, "the fourth column is not added");
        assert_eq!(s.columns[1].numeric().unwrap(), [11.0, 21.0, 31.0, 41.0]);
        let b = &s.columns[1];
        assert_eq!((b.long_name.as_str(), b.comments.as_str(), b.units.as_str()), ("y", "my note", "mA"));
        assert_eq!(b.id, ids[1], "the plot keeps its column");
        assert_eq!(p.graph(g).unwrap().layers[0].plots[0].y, ids[1]);
        // A file with fewer columns empties the rest.
        let fewer = file(&[("x", vec![1.0]), ("y", vec![5.0])]);
        let r = p.replace_book_data(book, &fewer).unwrap();
        assert_eq!((r.columns, r.emptied), (2, 1));
        assert!(p.workbook(book).unwrap().sheets[0].columns[2].data.is_empty());
    }
}
