//! Edit > Set As Begin, Set As End and Reset to Full Range (T87): a column's data range, as Origin
//! 2016's (LabTalk `set col(B) -b n` / `-e n`). Measured on 2026-11-16 (`docs/spec/workbook-model.md`,
//! "Column data range"):
//!
//! - The range is the rows from `begin` to `end` of one column; the values outside stay stored.
//! - The worksheet shows the cells outside the range blank (B begun at row 3 showed rows 1 and 2
//!   empty; C ended at row 4 showed rows 5 and 6 empty).
//! - Plots and statistics use only the rows inside it (B from row 2 to 4: `stats` gave n = 3, mean
//!   30, and its plot's X axis went from 1.5 to 4.5).
//! - Setting the begin to the first row and the end to the last gives the full range back.

use crate::ids::ColumnId;
use crate::project::{Project, ProjectError};
use serde::{Deserialize, Serialize};

/// The rows a column uses: `begin` to `end` (0-based, both included); no `end` means up to the
/// last row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRange {
    #[serde(default)]
    pub begin: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<usize>,
}

impl DataRange {
    /// Whether row `r` (0-based) is inside the range.
    pub fn contains(&self, r: usize) -> bool {
        r >= self.begin && self.end.is_none_or(|e| r <= e)
    }

    /// The rows of `start..end` inside the range, as a half-open pair.
    pub fn clip(&self, start: usize, end: usize) -> (usize, usize) {
        let s = start.max(self.begin);
        let e = self.end.map_or(end, |last| end.min(last + 1));
        (s, e.max(s))
    }

    /// `None` for the full range.
    fn normalized(self) -> Option<DataRange> {
        (self.begin != 0 || self.end.is_some()).then_some(self)
    }
}

/// Which end of the range a command sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeEnd {
    Begin,
    End,
}

impl Project {
    /// Set As Begin / Set As End: row `row` (0-based) becomes the first or last row of each column's
    /// range. A begin after the end (or an end before the begin) drops the other end.
    pub fn set_data_range_end(
        &mut self,
        columns: &[ColumnId],
        which: RangeEnd,
        row: usize,
    ) -> Result<(), ProjectError> {
        for &c in columns {
            self.column(c).ok_or(ProjectError::NoSuchColumn(c))?;
        }
        for &c in columns {
            let col = self.column_mut(c).expect("checked");
            let mut r = col.data_range.unwrap_or(DataRange { begin: 0, end: None });
            match which {
                RangeEnd::Begin => {
                    r.begin = row;
                    if r.end.is_some_and(|e| e < row) {
                        r.end = None;
                    }
                }
                RangeEnd::End => {
                    r.end = Some(row);
                    if r.begin > row {
                        r.begin = 0;
                    }
                }
            }
            col.data_range = r.normalized();
        }
        Ok(())
    }

    /// Reset to Full Range: the columns use all their rows again. Returns how many had a range.
    pub fn reset_data_range(&mut self, columns: &[ColumnId]) -> usize {
        let mut n = 0;
        for &c in columns {
            if let Some(col) = self.column_mut(c)
                && col.data_range.take().is_some()
            {
                n += 1;
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::template::Template;

    fn book() -> (Project, ColumnId, ColumnId) {
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        p.add_column(b, 0, Designation::X, ColumnData::numeric((1..=6).map(f64::from).collect())).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric((1..=6).map(|k| 10.0 * k as f64).collect()));
        let z = p.add_column(b, 0, Designation::Y, ColumnData::numeric((1..=6).map(|k| 5.0 * k as f64).collect()));
        (p, y.unwrap(), z.unwrap())
    }

    #[test]
    fn begin_and_end_set_the_rows_as_origins_set_b_and_e() {
        let (mut p, b, _) = book();
        // Origin's `set col(B) -b 2; set col(B) -e 4` (1-based): rows 1 to 3 here.
        p.set_data_range_end(&[b], RangeEnd::Begin, 1).unwrap();
        p.set_data_range_end(&[b], RangeEnd::End, 3).unwrap();
        let r = p.column(b).unwrap().data_range.unwrap();
        assert_eq!(r, DataRange { begin: 1, end: Some(3) });
        assert_eq!(r.clip(0, 6), (1, 4), "three rows, as Origin's stats n = 3");
        assert!(!r.contains(0) && r.contains(1) && r.contains(3) && !r.contains(4));
        // The values stay stored.
        assert_eq!(p.numeric(b).unwrap().len(), 6);
        // Begin 1 and end 6 (Origin) is the full range again.
        p.set_data_range_end(&[b], RangeEnd::Begin, 0).unwrap();
        p.set_data_range_end(&[b], RangeEnd::End, 5).unwrap();
        assert_eq!(p.column(b).unwrap().data_range, Some(DataRange { begin: 0, end: Some(5) }));
        assert_eq!(p.reset_data_range(&[b]), 1);
        assert_eq!(p.column(b).unwrap().data_range, None);
    }

    #[test]
    fn a_begin_after_the_end_drops_the_end() {
        let (mut p, b, _) = book();
        p.set_data_range_end(&[b], RangeEnd::End, 2).unwrap();
        p.set_data_range_end(&[b], RangeEnd::Begin, 4).unwrap();
        assert_eq!(p.column(b).unwrap().data_range, Some(DataRange { begin: 4, end: None }));
        p.set_data_range_end(&[b], RangeEnd::End, 1).unwrap();
        assert_eq!(p.column(b).unwrap().data_range, Some(DataRange { begin: 0, end: Some(1) }));
    }

    #[test]
    fn the_range_is_part_of_the_column_description() {
        let (mut p, b, z) = book();
        p.set_data_range_end(&[b], RangeEnd::Begin, 2).unwrap();
        let json = serde_json::to_string(p.column(b).unwrap()).unwrap();
        let back: crate::column::Column = serde_json::from_str(&json).unwrap();
        assert_eq!(back.data_range, Some(DataRange { begin: 2, end: None }));
        assert!(!serde_json::to_string(p.column(z).unwrap()).unwrap().contains("data_range"));
        // Plots are made as before; the drawing leaves the rows out (sheaf-plot).
        p.plot_columns(Template::Scatter, &[b]).unwrap();
    }
}
