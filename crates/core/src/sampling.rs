//! Column > Set Sampling Interval and Show X Column (T84), as Origin 2016's `colint` and
//! `colshowx`. Measured on 2026-11-16 (`docs/spec/workbook-model.md`, "Sampling interval"):
//!
//! - A column's interval gives row `i` (from 0) the X value `x0 + i * inc`. A plot of the column
//!   uses it even when an X column stands to its left (`plotxy` of a column with x0 = 0, inc = 0.5
//!   and five rows drew X from 0 to 2, not the X column's 1 to 5).
//! - Setting both the initial value and the increment to 1 clears the interval (the X is then the
//!   row number again, as without an interval).
//! - Show X Column inserts a new column directly left of the column, named as a new column is (the
//!   next free letter), with the interval's long name and units and as many rows as the column
//!   has. Without Clear Sampling Interval it is a Disregard column and the interval stays; with it,
//!   it is an X column and the interval is cleared, so the column now plots against it. Showing it
//!   again inserts another copy. A column without an interval shows nothing (Origin's `colshowx`
//!   fails).

use crate::column::{Column, ColumnData, Designation};
use crate::ids::ColumnId;
use crate::project::{Project, ProjectError};
use serde::{Deserialize, Serialize};

/// A column's own X: row `i` (from 0) is at `x0 + i * inc`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SamplingInterval {
    pub x0: f64,
    pub inc: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub units: String,
}

impl SamplingInterval {
    /// The X of row `i` (from 0).
    pub fn x_at(&self, i: usize) -> f64 {
        self.x0 + i as f64 * self.inc
    }

    /// The X of the first `rows` rows.
    pub fn values(&self, rows: usize) -> Vec<f64> {
        (0..rows).map(|i| self.x_at(i)).collect()
    }

    /// An axis title from the long name and units, as a column's: `Time (s)`.
    pub fn title(&self) -> String {
        match (self.long_name.is_empty(), self.units.is_empty()) {
            (_, true) => self.long_name.clone(),
            (true, false) => format!("({})", self.units),
            (false, false) => format!("{} ({})", self.long_name, self.units),
        }
    }
}

/// Why a sampling interval could not be set or shown.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SamplingError {
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("the initial X value and the increment must be numbers")]
    NotFinite,
    #[error("column {0} has no sampling interval to show")]
    NoInterval(String),
}

impl Project {
    /// Sets the sampling interval of `columns`; an initial value and increment of 1 clear it.
    pub fn set_sampling_interval(
        &mut self,
        columns: &[ColumnId],
        x0: f64,
        inc: f64,
        long_name: &str,
        units: &str,
    ) -> Result<(), SamplingError> {
        if !(x0.is_finite() && inc.is_finite()) {
            return Err(SamplingError::NotFinite);
        }
        for &c in columns {
            self.column(c).ok_or(ProjectError::NoSuchColumn(c))?;
        }
        let interval = (x0 != 1.0 || inc != 1.0).then(|| SamplingInterval {
            x0,
            inc,
            long_name: long_name.trim().to_string(),
            units: units.trim().to_string(),
        });
        for &c in columns {
            if let Some(col) = self.column_mut(c) {
                col.sampling = interval.clone();
            }
        }
        Ok(())
    }

    /// Show X Column: inserts the X values of `column`'s interval as a new column directly left of
    /// it (Disregard, or X when `clear` also clears the interval). Returns the new column.
    pub fn show_x_column(&mut self, column: ColumnId, clear: bool) -> Result<ColumnId, SamplingError> {
        let (book, sheet_index, at, rows, interval) = {
            let (b, sheet, col) = self.locate(column).ok_or(ProjectError::NoSuchColumn(column))?;
            let at = sheet.column_index(column).expect("the column is in its sheet");
            let si = b.sheets.iter().position(|s| std::ptr::eq(s, sheet)).expect("the sheet is in its book");
            let interval = col.sampling.clone().ok_or_else(|| SamplingError::NoInterval(col.short_name.clone()))?;
            (b.id, si, at, col.data.len(), interval)
        };
        let id = ColumnId(self.fresh_id());
        let ws = self
            .workbook_mut(book)
            .and_then(|b| b.sheets.get_mut(sheet_index))
            .ok_or(ProjectError::NoSuchColumn(column))?;
        let designation = if clear { Designation::X } else { Designation::Disregard };
        let mut x = Column::new(id, ws.next_short_name(), designation, ColumnData::numeric(interval.values(rows)));
        x.long_name = interval.long_name.clone();
        x.units = interval.units.clone();
        ws.columns.insert(at, x);
        if clear {
            ws.columns[at + 1].sampling = None;
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests;
