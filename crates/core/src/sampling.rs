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

/// What [`check_increment`] finds about a run of X values (Origin's `vinc` and `vinc_check`, T228;
/// measured in Origin 2016 by T371).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IncrementCheck {
    /// The first value.
    pub x0: f64,
    /// The mean step: (last - first) / (count - 1).
    pub inc: f64,
    /// The largest difference between one step and the mean step.
    pub max_deviation: f64,
    /// The sample standard deviation of the steps (`vinc`'s `sd`; 0 for a single step).
    pub sd: f64,
    /// The steps are equal within the tolerance: `max_deviation <= tolerance * |inc|`.
    pub uniform: bool,
}

/// The relative tolerance Set Sampling Interval's From X Column uses: Sheaf's choice, far below Origin's
/// default, because the interval then replaces the X values exactly.
pub const INCREMENT_TOLERANCE: f64 = 1e-6;

/// `vinc_check`'s default tolerance in Origin 2016 (measured, T371).
pub const ORIGIN_TOLERANCE: f64 = 0.05;

/// Checks whether `xs` are evenly spaced as Origin's `vinc_check` does (measured, T371): the mean step is
/// (last - first) / (count - 1), and the run is even when no step differs from it by more than
/// `tolerance` times its size (a relative test: steps 10, 10.4, 9.6 pass at 0.05 and fail at 0.039, as
/// steps 0.1, 0.104, 0.096 do; the largest deviation counts, not the standard deviation). `None` with
/// fewer than two values, a missing value (Origin reads a missing cell as about 0 instead), or a zero
/// mean step.
pub fn check_increment(xs: &[f64], tolerance: f64) -> Option<IncrementCheck> {
    if xs.len() < 2 || xs.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let (first, last) = (xs[0], xs[xs.len() - 1]);
    let inc = (last - first) / (xs.len() - 1) as f64;
    if inc == 0.0 {
        return None;
    }
    let max_deviation = xs.windows(2).map(|w| ((w[1] - w[0]) - inc).abs()).fold(0.0, f64::max);
    let steps = xs.len() - 1;
    let sd = if steps < 2 {
        0.0
    } else {
        let ss: f64 = xs.windows(2).map(|w| (w[1] - w[0] - inc).powi(2)).sum();
        (ss / (steps - 1) as f64).sqrt()
    };
    Some(IncrementCheck { x0: first, inc, max_deviation, sd, uniform: max_deviation <= tolerance * inc.abs() })
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
mod increment_tests {
    use super::*;

    #[test]
    fn check_increment_finds_even_steps_and_reports_uneven_ones() {
        let even = check_increment(&[2.0, 2.5, 3.0, 3.5], INCREMENT_TOLERANCE).unwrap();
        assert_eq!((even.x0, even.inc, even.uniform), (2.0, 0.5, true));
        let uneven = check_increment(&[0.0, 1.0, 3.0], INCREMENT_TOLERANCE).unwrap();
        assert!(!uneven.uniform);
        assert!((uneven.max_deviation - 0.5).abs() < 1e-12);
        assert!(check_increment(&[1.0], INCREMENT_TOLERANCE).is_none());
        assert!(check_increment(&[1.0, f64::NAN, 2.0], INCREMENT_TOLERANCE).is_none());
        assert!(check_increment(&[1.0, 1.0], INCREMENT_TOLERANCE).is_none());
        // Floating-point steps such as 0.1 still count as even.
        let xs: Vec<f64> = (0..50).map(|i| f64::from(i) * 0.1).collect();
        assert!(check_increment(&xs, INCREMENT_TOLERANCE).unwrap().uniform);
    }

    #[test]
    fn vinc_and_vinc_check_match_origin() {
        // Origin 2016, 2026-11-22 (T371, probes re-workspace/scratch/t371, local only).
        let c = check_increment(&[0.0, 1.0, 2.04, 3.0], ORIGIN_TOLERANCE).unwrap();
        assert!((c.inc, c.uniform) == (1.0, true) && (c.sd - 0.04).abs() < 1e-12);
        let d = check_increment(&[0.0, 1.0, 2.06, 3.0], ORIGIN_TOLERANCE).unwrap();
        assert!(!d.uniform && (d.sd - 0.06).abs() < 1e-12);
        assert!(check_increment(&[0.0, 1.0, 2.06, 3.0], 0.1).unwrap().uniform);
        assert!(!check_increment(&[0.0, 1.0, 2.04, 3.0], 0.01).unwrap().uniform);
        let b = check_increment(&[0.0, 1.0, 3.0], ORIGIN_TOLERANCE).unwrap();
        assert!((b.inc, b.uniform) == (1.5, false) && (b.sd - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
        let down = check_increment(&[3.0, 2.0, 1.0, 0.0], ORIGIN_TOLERANCE).unwrap();
        assert!((down.inc, down.sd, down.uniform) == (-1.0, 0.0, true));
        // The largest deviation counts, not the standard deviation (0.032 here).
        let a = [0.0, 1.0, 2.0, 3.0, 4.045, 5.0];
        let r = check_increment(&a, 0.044).unwrap();
        assert!(!r.uniform && (r.sd - 0.031_819_805_153_394_59).abs() < 1e-12);
        assert!(check_increment(&a, 0.046).unwrap().uniform);
        // Relative to the step.
        assert!(check_increment(&[0.0, 10.0, 20.4, 30.0], ORIGIN_TOLERANCE).unwrap().uniform);
        assert!(!check_increment(&[0.0, 10.0, 20.4, 30.0], 0.039).unwrap().uniform);
        assert!(!check_increment(&[0.0, 0.1, 0.204, 0.3], 0.039).unwrap().uniform);
    }
}
