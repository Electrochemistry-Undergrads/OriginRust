//! Column formulas ("set column values"): fill a column from an expression
//! over the other columns of its sheet, for example `col(B)*1000`.
//!
//! The expression language is in the `sheaf-expr` crate and described in
//! `docs/spec/expression-language.md`. This module connects it to worksheets.

use crate::column::{Column, ColumnData};
use crate::ids::ColumnId;
use crate::project::Project;
use crate::worksheet::Worksheet;
use sheaf_expr::{ColumnRef, Expr, ExprError, Host};
use std::ops::Range;

/// The most rows a formula may fill; a larger end row is refused instead of allocating that much.
pub const MAX_FORMULA_ROWS: usize = 10_000_000;

#[derive(Debug, thiserror::Error)]
pub enum FormulaError {
    #[error("there is no column with id {0:?}")]
    NoSuchColumn(ColumnId),
    #[error("the sheet has no rows yet; give a row range or enter data first")]
    NoRows,
    #[error("row {0} is beyond the limit of {MAX_FORMULA_ROWS} rows for a formula")]
    TooManyRows(usize),
    #[error("{0}")]
    Expr(#[from] ExprError),
}

/// Lets a formula read the columns of one worksheet.
///
/// `col(A)` finds a column by short name (ignoring case) and, if there is
/// none, by long name; `col("Long Name")` by long name; `col(3)` by position.
/// Text columns read as missing values.
struct SheetHost<'a> {
    sheet: &'a Worksheet,
}

impl<'a> SheetHost<'a> {
    fn find(&self, r: &ColumnRef) -> Option<&'a Column> {
        let cols = &self.sheet.columns;
        match r {
            ColumnRef::Short(name) => cols
                .iter()
                .find(|c| c.short_name.eq_ignore_ascii_case(name))
                .or_else(|| cols.iter().find(|c| c.long_name == *name)),
            ColumnRef::Long(name) => cols.iter().find(|c| c.long_name == *name),
            ColumnRef::Index(n) => n.checked_sub(1).and_then(|i| cols.get(i)),
        }
    }
}

impl Host for SheetHost<'_> {
    fn variable(&self, _name: &str) -> Option<f64> {
        None
    }

    fn cell(&self, col: &ColumnRef, row: usize) -> Option<f64> {
        let c = self.find(col)?;
        match c.numeric() {
            Some(v) => v.get(row).copied(),
            None => (row < c.len()).then_some(f64::NAN),
        }
    }

    fn has_column(&self, col: &ColumnRef) -> bool {
        self.find(col).is_some()
    }

    fn column_data(&self, col: &ColumnRef) -> Option<&[f64]> {
        self.find(col)?.numeric()
    }

    fn column_len(&self, col: &ColumnRef) -> usize {
        self.find(col).map_or(0, Column::len)
    }
}

/// Evaluates an expression for the first `rows` rows of a sheet.
pub(crate) fn eval_on_sheet(expr: &Expr, sheet: &Worksheet, rows: usize) -> Result<Vec<f64>, ExprError> {
    expr.eval_rows(&SheetHost { sheet }, rows)
}

impl Project {
    /// Fills a column from a formula and remembers the formula on the column.
    ///
    /// `rows` is the 0-based row range to fill; `None` fills every row of
    /// the sheet (the length of its longest column). Rows of the column
    /// outside the range keep their values. A text column becomes numeric.
    /// On error nothing is changed.
    pub fn set_column_values(
        &mut self,
        column: ColumnId,
        formula: &str,
        rows: Option<Range<usize>>,
    ) -> Result<(), FormulaError> {
        let expr = Expr::parse(formula)?;
        let (_, sheet, target) = self.locate(column).ok_or(FormulaError::NoSuchColumn(column))?;
        let range = rows.unwrap_or(0..sheet.row_count());
        if range.is_empty() {
            return Err(FormulaError::NoRows);
        }
        if range.end > MAX_FORMULA_ROWS {
            return Err(FormulaError::TooManyRows(range.end));
        }
        let computed = expr.eval_rows(&SheetHost { sheet }, range.end)?;
        let mut values: Vec<f64> = target.numeric().map(<[f64]>::to_vec).unwrap_or_default();
        if values.len() < range.end {
            values.resize(range.end, f64::NAN);
        }
        values[range.clone()].copy_from_slice(&computed[range]);

        let c = self.column_mut(column).expect("located above");
        c.data = ColumnData::numeric(values);
        c.formula = Some(formula.trim().to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Designation;

    fn sample() -> (Project, ColumnId, ColumnId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.5, 0.25, f64::NAN])).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
        p.column_mut(b).unwrap().long_name = "Current".into();
        (p, a, b, c)
    }

    #[test]
    fn an_absurd_end_row_is_refused_without_allocating() {
        let (mut p, _, _, c) = sample();
        let err = p.set_column_values(c, "i", Some(0..usize::MAX)).unwrap_err();
        assert!(matches!(err, FormulaError::TooManyRows(_)));
        assert!(p.numeric(c).unwrap().is_empty());
    }

    #[test]
    fn the_row_limit_is_the_exact_boundary() {
        let (mut p, _, _, c) = sample();
        let err = p.set_column_values(c, "1", Some(0..MAX_FORMULA_ROWS + 1)).unwrap_err();
        assert!(matches!(err, FormulaError::TooManyRows(n) if n == MAX_FORMULA_ROWS + 1));
        // A range of a few rows is still fine.
        p.set_column_values(c, "1", Some(0..3)).unwrap();
        assert_eq!(p.numeric(c).unwrap()[..3], [1.0, 1.0, 1.0]);
    }

    #[test]
    fn fills_every_row_of_the_sheet_and_keeps_the_formula() {
        let (mut p, _, _, c) = sample();
        p.set_column_values(c, " col(a)*1000 + i ", None).unwrap();
        assert_eq!(p.numeric(c).unwrap(), &[1001.0, 2002.0, 3003.0, 4004.0]);
        assert_eq!(p.column(c).unwrap().formula.as_deref(), Some("col(a)*1000 + i"));
    }

    #[test]
    fn finds_columns_by_long_name_and_position_and_passes_missing_values_on() {
        let (mut p, _, _, c) = sample();
        p.set_column_values(c, "col(\"Current\")*2 + col(Current)*0 + col(1)*0", None).unwrap();
        let v = p.numeric(c).unwrap();
        assert_eq!(&v[..2], &[1.0, 0.5]);
        assert!(v[2].is_nan(), "a missing input gives a missing result");
        assert!(v[3].is_nan(), "a row past the end of the input is missing");
    }

    #[test]
    fn a_row_range_leaves_other_rows_alone() {
        let (mut p, _, b, _) = sample();
        p.set_column_values(b, "col(A)*10", Some(1..3)).unwrap();
        assert_eq!(p.numeric(b).unwrap(), &[0.5, 20.0, 30.0]);
        // A range past the end grows the column with missing values in between.
        p.set_column_values(b, "7", Some(5..6)).unwrap();
        let v = p.numeric(b).unwrap();
        assert_eq!((v.len(), v[5]), (6, 7.0));
        assert!(v[3].is_nan() && v[4].is_nan());
    }

    #[test]
    fn a_column_can_refer_to_itself_and_text_becomes_numeric() {
        let (mut p, a, _, _) = sample();
        p.set_column_values(a, "col(A)^2", None).unwrap();
        assert_eq!(p.numeric(a).unwrap(), &[1.0, 4.0, 9.0, 16.0]);
        let book = p.workbooks().next().unwrap().id;
        let t = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["x".into()])).unwrap();
        p.set_column_values(t, "i", None).unwrap();
        assert_eq!(p.numeric(t).unwrap(), &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn errors_change_nothing() {
        let (mut p, _, b, c) = sample();
        let before = p.clone();
        assert!(matches!(p.set_column_values(c, "col(Nope)+1", None), Err(FormulaError::Expr(_))));
        assert!(matches!(p.set_column_values(c, "1 +", None), Err(FormulaError::Expr(_))));
        assert!(matches!(p.set_column_values(ColumnId(999), "1", None), Err(FormulaError::NoSuchColumn(_))));
        assert_eq!(p, before);
        let mut empty = Project::new();
        let book = empty.add_workbook("");
        let e = empty.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
        assert!(matches!(empty.set_column_values(e, "1", None), Err(FormulaError::NoRows)));
        let _ = b;
    }
}
