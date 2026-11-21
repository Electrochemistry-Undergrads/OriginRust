//! Matrix windows: a matrix book holds one or more matrix sheets, and a matrix sheet is a grid of
//! numbers (`rows` x `cols`) whose columns and rows are mapped onto an X range and a Y range.
//! The rules are in `docs/spec/matrices.md`.
//!
//! Cells are addressed by 0-based `(row, col)` in this API; the formulas of
//! [`MatrixSheet::set_values`] count from 1 as Origin does (`i` is the row, `j` the column).

use crate::column::ColumnData;
use crate::ids::WindowId;
use serde::{Deserialize, Serialize};
use sheaf_expr::{ColumnRef, Expr, ExprError, Host};
use std::sync::Arc;

/// Origin's size for a new matrix sheet, in both directions.
pub const DEFAULT_SIZE: usize = 32;
/// The default X and Y range of a new matrix sheet (Origin's new matrix maps its 32 columns and
/// rows onto 1 to 10, whatever their number).
pub const DEFAULT_RANGE: (f64, f64) = (1.0, 10.0);
/// The most cells a matrix sheet may have (a guard against a typo that asks for billions).
pub const MAX_CELLS: usize = 200_000_000;

/// Long name, units and comments of one of the three coordinates of a matrix sheet.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MatrixLabel {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub units: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comments: String,
}

impl MatrixLabel {
    pub fn is_empty(&self) -> bool {
        self.long_name.is_empty() && self.units.is_empty() && self.comments.is_empty()
    }

    /// Long name followed by the units in parentheses, as an axis title is built.
    pub fn title(&self) -> String {
        match (self.long_name.is_empty(), self.units.is_empty()) {
            (true, true) => String::new(),
            (false, true) => self.long_name.clone(),
            (true, false) => format!("({})", self.units),
            (false, false) => format!("{} ({})", self.long_name, self.units),
        }
    }
}

/// Why a matrix operation failed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MatrixError {
    #[error("a matrix of {rows} x {cols} cells is too large (the limit is {MAX_CELLS} cells)")]
    TooLarge { rows: usize, cols: usize },
    #[error("the matrix sheet has no cell ({row}, {col})")]
    NoSuchCell { row: usize, col: usize },
    #[error("{0}")]
    Formula(String),
    #[error("{0}")]
    Invalid(String),
}

impl From<ExprError> for MatrixError {
    fn from(e: ExprError) -> Self {
        MatrixError::Formula(e.to_string())
    }
}

/// One sheet of a matrix book.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixSheet {
    /// Sheet name, for example `MSheet1`; unique within the book.
    pub name: String,
    /// Identifies the sheet's numbers in the project file (`data/m<id>.f64`). Unique in the
    /// project, never reused, shared with the identifiers of windows and columns.
    pub data_id: u64,
    pub rows: usize,
    pub cols: usize,
    /// The X coordinate of the first and of the last column.
    pub x_from: f64,
    pub x_to: f64,
    /// The Y coordinate of the first and of the last row.
    pub y_from: f64,
    pub y_to: f64,
    #[serde(default, skip_serializing_if = "MatrixLabel::is_empty")]
    pub x: MatrixLabel,
    #[serde(default, skip_serializing_if = "MatrixLabel::is_empty")]
    pub y: MatrixLabel,
    /// The labels of the cell values (the Z coordinate).
    #[serde(default, skip_serializing_if = "MatrixLabel::is_empty")]
    pub z: MatrixLabel,
    /// The numbers, row by row (`rows * cols` of them); a missing value is NaN. Not part of the
    /// JSON description: the project file stores them in `data/m<id>.f64`.
    #[serde(skip)]
    pub data: ColumnData,
}

/// Statistics of the cells of a matrix sheet (Origin's `mstats`).
#[derive(Clone, Debug, PartialEq)]
pub struct MatrixStats {
    /// Number of cells that hold a number.
    pub n: usize,
    /// Number of missing cells.
    pub missing: usize,
    pub mean: f64,
    /// Sample standard deviation (divisor n - 1); NaN for fewer than two numbers.
    pub sd: f64,
    pub min: f64,
    pub max: f64,
    pub sum: f64,
}

/// Which way [`MatrixSheet::flip`] mirrors the data.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FlipDirection {
    /// Mirror left to right: the first column becomes the last.
    Horizontal,
    /// Mirror top to bottom: the first row becomes the last.
    Vertical,
}

/// Which way [`MatrixSheet::rotate90`] turns the data.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Rotation {
    Cw90,
    Ccw90,
    Half,
}

pub(crate) fn check_cells(rows: usize, cols: usize) -> Result<(), MatrixError> {
    check_size(rows, cols)
}

fn check_size(rows: usize, cols: usize) -> Result<(), MatrixError> {
    match rows.checked_mul(cols) {
        Some(n) if n <= MAX_CELLS => Ok(()),
        _ => Err(MatrixError::TooLarge { rows, cols }),
    }
}

/// The coordinate of position `k` of `n` evenly spaced positions from `from` to `to` (position
/// 0 is `from`, position `n - 1` is `to`; a single position is `from`).
pub fn spaced(from: f64, to: f64, k: usize, n: usize) -> f64 {
    if n <= 1 { from } else { from + (to - from) * k as f64 / (n - 1) as f64 }
}

impl MatrixSheet {
    /// A sheet of `rows` x `cols` missing values with Origin's default ranges.
    pub fn new(name: impl Into<String>, data_id: u64, rows: usize, cols: usize) -> Self {
        MatrixSheet {
            name: name.into(),
            data_id,
            rows,
            cols,
            x_from: DEFAULT_RANGE.0,
            x_to: DEFAULT_RANGE.1,
            y_from: DEFAULT_RANGE.0,
            y_to: DEFAULT_RANGE.1,
            x: MatrixLabel::default(),
            y: MatrixLabel::default(),
            z: MatrixLabel::default(),
            data: ColumnData::numeric(vec![f64::NAN; rows * cols]),
        }
    }

    /// A sheet that holds `values` (row by row) and maps its columns and rows onto the given ranges.
    pub fn from_values(
        name: impl Into<String>,
        data_id: u64,
        rows: usize,
        cols: usize,
        values: Vec<f64>,
        x: (f64, f64),
        y: (f64, f64),
    ) -> Result<Self, MatrixError> {
        check_size(rows, cols)?;
        if values.len() != rows * cols {
            return Err(MatrixError::Invalid(format!(
                "{} numbers do not fill a matrix of {rows} x {cols}",
                values.len()
            )));
        }
        let mut s = MatrixSheet::new(name, data_id, 0, 0);
        s.rows = rows;
        s.cols = cols;
        s.x_from = x.0;
        s.x_to = x.1;
        s.y_from = y.0;
        s.y_to = y.1;
        s.data = ColumnData::numeric(values);
        Ok(s)
    }

    /// The numbers, row by row. Empty when the stored data does not match the size (a damaged file).
    pub fn values(&self) -> &[f64] {
        match &self.data {
            ColumnData::Numeric(v) if v.len() == self.rows * self.cols => v.as_slice(),
            _ => &[],
        }
    }

    /// Mutable access to the numbers; the data is copied first when an undo snapshot shares it.
    /// The data are made the right size first.
    pub fn values_mut(&mut self) -> &mut Vec<f64> {
        let n = self.rows * self.cols;
        if !matches!(&self.data, ColumnData::Numeric(v) if v.len() == n) {
            self.data = ColumnData::numeric(vec![f64::NAN; n]);
        }
        match &mut self.data {
            ColumnData::Numeric(v) => Arc::make_mut(v),
            ColumnData::Text(_) | ColumnData::Mixed { .. } => unreachable!("replaced by numbers above"),
        }
    }

    /// The value at 0-based `(row, col)`; NaN (missing) outside the matrix.
    pub fn value(&self, row: usize, col: usize) -> f64 {
        if row < self.rows && col < self.cols {
            self.values().get(row * self.cols + col).copied().unwrap_or(f64::NAN)
        } else {
            f64::NAN
        }
    }

    pub fn set_value(&mut self, row: usize, col: usize, v: f64) -> Result<(), MatrixError> {
        if row >= self.rows || col >= self.cols {
            return Err(MatrixError::NoSuchCell { row, col });
        }
        let cols = self.cols;
        self.values_mut()[row * cols + col] = v;
        Ok(())
    }

    /// The X coordinate of 0-based column `col`.
    pub fn x_at(&self, col: usize) -> f64 {
        spaced(self.x_from, self.x_to, col, self.cols)
    }

    /// The Y coordinate of 0-based row `row`.
    pub fn y_at(&self, row: usize) -> f64 {
        spaced(self.y_from, self.y_to, row, self.rows)
    }

    /// Smallest and largest number in the matrix; `None` when it holds none.
    pub fn z_range(&self) -> Option<(f64, f64)> {
        let mut r: Option<(f64, f64)> = None;
        for &v in self.values().iter().filter(|v| v.is_finite()) {
            r = Some(match r {
                None => (v, v),
                Some((lo, hi)) => (lo.min(v), hi.max(v)),
            });
        }
        r
    }

    /// Changes the number of columns and rows (Origin's `mdim`). The X and Y ranges stay as they
    /// are, so the spacing changes; cells that exist before and after keep their values and new
    /// cells are missing. `ranges` sets the ranges too (`x_from, x_to, y_from, y_to`).
    pub fn set_dimensions(
        &mut self,
        rows: usize,
        cols: usize,
        ranges: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), MatrixError> {
        if rows == 0 || cols == 0 {
            return Err(MatrixError::Invalid("a matrix needs at least one column and one row".into()));
        }
        check_size(rows, cols)?;
        let mut out = vec![f64::NAN; rows * cols];
        for r in 0..rows.min(self.rows) {
            for c in 0..cols.min(self.cols) {
                out[r * cols + c] = self.value(r, c);
            }
        }
        self.rows = rows;
        self.cols = cols;
        self.data = ColumnData::numeric(out);
        if let Some((x1, x2, y1, y2)) = ranges {
            self.x_from = x1;
            self.x_to = x2;
            self.y_from = y1;
            self.y_to = y2;
        }
        Ok(())
    }

    /// Fills the matrix from a formula in `i` (row, from 1), `j` (column, from 1), `x` and `y`
    /// (the cell's coordinates) and everything the expression language offers (Origin's Set Matrix
    /// Values, `msetvalue`). A result that is not a finite number is stored as missing. On an
    /// error the matrix is left as it was.
    pub fn set_values(&mut self, formula: &str) -> Result<(), MatrixError> {
        let expr = Expr::parse(formula)?;
        let (rows, cols) = (self.rows, self.cols);
        let mut out = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            let y = self.y_at(r);
            for c in 0..cols {
                let host = CellHost { i: (r + 1) as f64, j: (c + 1) as f64, x: self.x_at(c), y };
                let v = expr.eval(&host)?;
                out.push(if v.is_finite() { v } else { f64::NAN });
            }
        }
        self.data = ColumnData::numeric(out);
        Ok(())
    }

    /// Statistics of the numbers in the matrix; `mean`, `sd`, `min` and `max` are NaN when there
    /// are none.
    pub fn stats(&self) -> MatrixStats {
        let vals: Vec<f64> = self.values().iter().copied().filter(|v| !v.is_nan()).collect();
        let n = vals.len();
        let sum: f64 = vals.iter().sum();
        let mean = if n > 0 { sum / n as f64 } else { f64::NAN };
        let sd = if n > 1 {
            (vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1) as f64).sqrt()
        } else {
            f64::NAN
        };
        let min = vals.iter().copied().fold(f64::NAN, f64::min);
        let max = vals.iter().copied().fold(f64::NAN, f64::max);
        MatrixStats { n, missing: self.rows * self.cols - n, mean, sd, min, max, sum }
    }

    pub(crate) fn like(&self, rows: usize, cols: usize, data: Vec<f64>) -> MatrixSheet {
        MatrixSheet { rows, cols, data: ColumnData::numeric(data), ..self.clone() }
    }

    /// Rows become columns (Origin's `mtranspose`). The X range of the result is the old Y range
    /// and the other way round, and so are the X and Y labels.
    pub fn transpose(&self) -> MatrixSheet {
        let mut out = vec![f64::NAN; self.rows * self.cols];
        for r in 0..self.rows {
            for c in 0..self.cols {
                out[c * self.rows + r] = self.value(r, c);
            }
        }
        let mut s = self.like(self.cols, self.rows, out);
        s.x_from = self.y_from;
        s.x_to = self.y_to;
        s.y_from = self.x_from;
        s.y_to = self.x_to;
        s.x = self.y.clone();
        s.y = self.x.clone();
        s
    }

    /// The inverse of the matrix as Origin 2016's `minverse` makes it (Analysis > Mathematics > Inverse,
    /// T193, measured by T290; `sheaf_calc::minverse`): the inverse of a square matrix of full rank, else
    /// the pseudoinverse (`cols x rows`), with `true` beside it. A missing cell counts as Origin's missing
    /// number (in effect 0). The result is a fresh sheet named "Inversed1" on the default ranges, 1 to the
    /// number of columns and 1 to the number of rows, without labels, as Origin's.
    pub fn inverse(&self) -> Result<(MatrixSheet, bool), MatrixError> {
        let (rows, cols) = (self.rows, self.cols);
        let a: Vec<f64> = (0..rows * cols).map(|k| self.value(k / cols, k % cols)).collect();
        let r = sheaf_calc::minverse::minverse(&a, rows, cols).ok_or_else(|| {
            MatrixError::Invalid("the matrix is empty or holds an infinite value; it has no inverse".into())
        })?;
        let s = MatrixSheet::from_values(
            "Inversed1",
            0,
            r.rows,
            r.cols,
            r.values,
            (1.0, r.cols as f64),
            (1.0, r.rows as f64),
        )?;
        Ok((s, r.pseudo))
    }

    /// Mirrors the data (Origin's `mflip`). With `coordinates` the X (horizontal) or Y (vertical)
    /// range is mirrored too, so every value keeps its coordinate; without, the values move
    /// to other coordinates.
    pub fn flip(&self, direction: FlipDirection, coordinates: bool) -> MatrixSheet {
        let mut out = vec![f64::NAN; self.rows * self.cols];
        for r in 0..self.rows {
            for c in 0..self.cols {
                let (sr, sc) = match direction {
                    FlipDirection::Horizontal => (r, self.cols - 1 - c),
                    FlipDirection::Vertical => (self.rows - 1 - r, c),
                };
                out[r * self.cols + c] = self.value(sr, sc);
            }
        }
        let mut s = self.like(self.rows, self.cols, out);
        if coordinates {
            match direction {
                FlipDirection::Horizontal => std::mem::swap(&mut s.x_from, &mut s.x_to),
                FlipDirection::Vertical => std::mem::swap(&mut s.y_from, &mut s.y_to),
            }
        }
        s
    }

    /// Turns the data a quarter turn clockwise or counter-clockwise, or a half turn (Origin's
    /// `mrotate90`). The coordinates travel with the data: after a quarter turn the new X range
    /// is the old Y range (reversed for a clockwise turn) and the new Y range is the old X range
    /// (reversed for a counter-clockwise turn); after a half turn both ranges are reversed.
    pub fn rotate90(&self, rotation: Rotation) -> MatrixSheet {
        let (nr, nc) = (self.rows, self.cols);
        match rotation {
            Rotation::Half => {
                let mut out = vec![f64::NAN; nr * nc];
                for r in 0..nr {
                    for c in 0..nc {
                        out[r * nc + c] = self.value(nr - 1 - r, nc - 1 - c);
                    }
                }
                let mut s = self.like(nr, nc, out);
                std::mem::swap(&mut s.x_from, &mut s.x_to);
                std::mem::swap(&mut s.y_from, &mut s.y_to);
                s
            }
            Rotation::Cw90 | Rotation::Ccw90 => {
                // The result has nc rows and nr columns.
                let mut out = vec![f64::NAN; nr * nc];
                for r in 0..nc {
                    for c in 0..nr {
                        out[r * nr + c] = match rotation {
                            Rotation::Cw90 => self.value(nr - 1 - c, r),
                            _ => self.value(c, nc - 1 - r),
                        };
                    }
                }
                let mut s = self.like(nc, nr, out);
                s.x = self.y.clone();
                s.y = self.x.clone();
                if rotation == Rotation::Cw90 {
                    s.x_from = self.y_to;
                    s.x_to = self.y_from;
                    s.y_from = self.x_from;
                    s.y_to = self.x_to;
                } else {
                    s.x_from = self.y_from;
                    s.x_to = self.y_to;
                    s.y_from = self.x_to;
                    s.y_to = self.x_from;
                }
                s
            }
        }
    }

    /// Makes the matrix finer (Origin's `mexpand`): `cols_factor` times as many columns and
    /// `rows_factor` times as many rows, with the new values found by bilinear interpolation. The
    /// first and last columns and rows stay where they are, so the X and Y ranges do not change.
    pub fn expand(&self, cols_factor: usize, rows_factor: usize) -> Result<MatrixSheet, MatrixError> {
        if cols_factor == 0 || rows_factor == 0 {
            return Err(MatrixError::Invalid("the expand factors must be at least 1".into()));
        }
        let (nr, nc) = (self.rows * rows_factor, self.cols * cols_factor);
        check_size(nr, nc)?;
        let mut out = Vec::with_capacity(nr * nc);
        for r in 0..nr {
            let (r0, tr) = interp_position(r, nr, self.rows);
            for c in 0..nc {
                let (c0, tc) = interp_position(c, nc, self.cols);
                let r1 = (r0 + 1).min(self.rows - 1);
                let c1 = (c0 + 1).min(self.cols - 1);
                let top = lerp(self.value(r0, c0), self.value(r0, c1), tc);
                let bottom = lerp(self.value(r1, c0), self.value(r1, c1), tc);
                out.push(lerp(top, bottom, tr));
            }
        }
        Ok(self.like(nr, nc, out))
    }

    /// Makes the matrix coarser (Origin's `mshrink`): every block of `cols_factor` x `rows_factor`
    /// cells becomes one cell holding their mean. The matrix gets `ceil(cols / cols_factor)` columns
    /// and `ceil(rows / rows_factor)` rows; blocks at the edge that are cut short average the cells
    /// they have. Missing cells are left out of a mean (a block of only missing cells is missing).
    /// The X and Y ranges do not change.
    pub fn shrink(&self, cols_factor: usize, rows_factor: usize) -> Result<MatrixSheet, MatrixError> {
        if cols_factor == 0 || rows_factor == 0 {
            return Err(MatrixError::Invalid("the shrink factors must be at least 1".into()));
        }
        let nr = self.rows.div_ceil(rows_factor);
        let nc = self.cols.div_ceil(cols_factor);
        let mut out = Vec::with_capacity(nr * nc);
        for r in 0..nr {
            for c in 0..nc {
                let (mut sum, mut n) = (0.0, 0usize);
                for rr in r * rows_factor..((r + 1) * rows_factor).min(self.rows) {
                    for cc in c * cols_factor..((c + 1) * cols_factor).min(self.cols) {
                        let v = self.value(rr, cc);
                        if !v.is_nan() {
                            sum += v;
                            n += 1;
                        }
                    }
                }
                out.push(if n > 0 { sum / n as f64 } else { f64::NAN });
            }
        }
        Ok(self.like(nr, nc, out))
    }
}

/// For output position `k` of `n` that spans the same range as `old` input positions: the input
/// index below it and the fraction of the way to the next one.
fn interp_position(k: usize, n: usize, old: usize) -> (usize, f64) {
    if n <= 1 || old <= 1 {
        return (0, 0.0);
    }
    let pos = k as f64 * (old - 1) as f64 / (n - 1) as f64;
    let i = (pos.floor() as usize).min(old - 1);
    (i, pos - i as f64)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    if t == 0.0 { a } else { a + (b - a) * t }
}

/// The variables of a matrix formula.
struct CellHost {
    i: f64,
    j: f64,
    x: f64,
    y: f64,
}

impl Host for CellHost {
    fn variable(&self, name: &str) -> Option<f64> {
        match name {
            "i" => Some(self.i),
            "j" => Some(self.j),
            "x" => Some(self.x),
            "y" => Some(self.y),
            _ => None,
        }
    }

    fn cell(&self, _col: &ColumnRef, _row: usize) -> Option<f64> {
        None
    }

    fn has_column(&self, _col: &ColumnRef) -> bool {
        false
    }
}

/// A window that holds matrix sheets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixBook {
    pub id: WindowId,
    /// Short name, unique among all windows of the project (for example `MBook1`).
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    pub sheets: Vec<MatrixSheet>,
    #[serde(default)]
    pub active_sheet: usize,
}

impl MatrixBook {
    /// A book with one sheet `MSheet1` of Origin's default size, with the given data id.
    pub fn new(id: WindowId, name: impl Into<String>, data_id: u64) -> Self {
        MatrixBook {
            id,
            name: name.into(),
            long_name: String::new(),
            sheets: vec![MatrixSheet::new("MSheet1", data_id, DEFAULT_SIZE, DEFAULT_SIZE)],
            active_sheet: 0,
        }
    }

    pub fn sheet(&self) -> &MatrixSheet {
        &self.sheets[self.active_sheet.min(self.sheets.len().saturating_sub(1))]
    }

    pub fn sheet_mut(&mut self) -> &mut MatrixSheet {
        let i = self.active_sheet.min(self.sheets.len().saturating_sub(1));
        &mut self.sheets[i]
    }

    pub fn sheet_by_name(&self, name: &str) -> Option<&MatrixSheet> {
        self.sheets.iter().find(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// The window title: `MBook1 - long name` when a long name is set.
    pub fn title(&self) -> String {
        if self.long_name.is_empty() { self.name.clone() } else { format!("{} - {}", self.name, self.long_name) }
    }
}

#[cfg(test)]
mod inverse_tests {
    use super::*;

    #[test]
    fn the_inverse_of_a_matrix_as_origins_minverse() {
        let mut m = MatrixSheet::new("M", 1, 2, 2);
        for (r, c, v) in [(0, 0, 4.0), (0, 1, 7.0), (1, 0, 2.0), (1, 1, 6.0)] {
            m.set_value(r, c, v).unwrap();
        }
        m.x_from = 10.0;
        m.x_to = 20.0;
        let (inv, pseudo) = m.inverse().unwrap();
        assert!(!pseudo);
        for (r, c, v) in [(0, 0, 0.6), (0, 1, -0.7), (1, 0, -0.2), (1, 1, 0.4)] {
            assert!((inv.value(r, c) - v).abs() < 1e-12, "{r},{c}: {}", inv.value(r, c));
        }
        // Origin's output sheet: "Inversed1" on the default ranges, whatever the input's.
        assert_eq!(inv.name, "Inversed1");
        assert_eq!((inv.x_from, inv.x_to, inv.y_from, inv.y_to), (1.0, 2.0, 1.0, 2.0));
        // A pivot of zero in the first column needs a row swap.
        let mut p = MatrixSheet::new("P", 1, 2, 2);
        for (r, c, v) in [(0, 0, 0.0), (0, 1, 1.0), (1, 0, 1.0), (1, 1, 0.0)] {
            p.set_value(r, c, v).unwrap();
        }
        let (pi, _) = p.inverse().unwrap();
        assert_eq!((pi.value(0, 1), pi.value(1, 0)), (1.0, 1.0));
        // Singular: the pseudoinverse (measured in Origin 2016).
        let mut s = MatrixSheet::new("S", 1, 2, 2);
        for (r, c, v) in [(0, 0, 1.0), (0, 1, 2.0), (1, 0, 2.0), (1, 1, 4.0)] {
            s.set_value(r, c, v).unwrap();
        }
        let (si, pseudo) = s.inverse().unwrap();
        assert!(pseudo);
        assert!((si.value(1, 1) - 0.16).abs() < 1e-13);
        // Not square: the pseudoinverse is cols x rows, on ranges to match.
        let mut w = MatrixSheet::new("W", 1, 2, 3);
        for k in 0..6 {
            w.set_value(k / 3, k % 3, (k + 1) as f64).unwrap();
        }
        let (wi, pseudo) = w.inverse().unwrap();
        assert!(pseudo);
        assert_eq!((wi.rows, wi.cols, wi.x_to, wi.y_to), (3, 2, 2.0, 3.0));
        assert!((wi.value(0, 0) + 17.0 / 18.0).abs() < 1e-12);
        // A missing cell counts as Origin's missing number, in effect 0 (measured).
        let mut n = MatrixSheet::new("N", 1, 2, 2);
        for (r, c, v) in [(0, 0, 1.0), (1, 0, 3.0), (1, 1, 4.0)] {
            n.set_value(r, c, v).unwrap();
        }
        let (ni, pseudo) = n.inverse().unwrap();
        assert!(!pseudo);
        for (k, want) in [1.0, 0.0, -0.75, 0.25].into_iter().enumerate() {
            assert!((ni.value(k / 2, k % 2) - want).abs() < 1e-15, "{k}");
        }
    }
}
