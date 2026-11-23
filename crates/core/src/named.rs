//! Named cells (T396, Origin 2022's named ranges, the single-cell case): a workbook can give a cell a name, and a
//! formula of any column of that workbook reads the name as the cell's value (`col(B) * Rate`). The formula then
//! depends on the cell's column, so it is calculated again when that column changes. Rules:
//! `docs/spec/expression-language.md`, "Named cells".
//!
//! - A name starts with a letter and has letters, digits and underscores, up to 32 characters; it is unique in
//!   its workbook regardless of letter case, and `i`, `j` and `pi` (which formulas already use) are refused.
//! - A text cell, an empty cell or a cell past the column's end reads as missing; a name whose column was
//!   deleted no longer resolves (a formula using it then fails as with any unknown name).

use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::worksheet::Workbook;
use serde::{Deserialize, Serialize};

/// One named cell of a workbook.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedCell {
    pub name: String,
    pub column: ColumnId,
    /// 0-based row.
    pub row: usize,
}

/// Names a formula already gives a meaning to.
const RESERVED: [&str; 3] = ["i", "j", "pi"];
const MAX_LEN: usize = 32;

/// Whether `name` can name a cell; the reason when not.
pub fn check_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else { return Err("a name cannot be empty".into()) };
    if !first.is_ascii_alphabetic() {
        return Err(format!("{name:?} must start with a letter"));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("{name:?} may hold only letters, digits and underscores"));
    }
    if name.len() > MAX_LEN {
        return Err(format!("{name:?} is longer than {MAX_LEN} characters"));
    }
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(name)) {
        return Err(format!("{name:?} already means something in formulas"));
    }
    Ok(())
}

impl Workbook {
    /// The named cell called `name` (letter case ignored).
    pub fn named_cell(&self, name: &str) -> Option<&NamedCell> {
        self.names.iter().find(|n| n.name.eq_ignore_ascii_case(name))
    }

    /// The value of the named cell `name`: its number, missing for text, an empty cell or a row past the end;
    /// `None` when there is no such name or its column is gone.
    pub fn named_value(&self, name: &str) -> Option<f64> {
        let n = self.named_cell(name)?;
        let column = self.sheets.iter().find_map(|s| s.columns.iter().find(|c| c.id == n.column))?;
        Some(column.numeric().and_then(|v| v.get(n.row).copied()).unwrap_or(f64::NAN))
    }
}

impl Project {
    /// Gives the cell at `row` of `column` the name `name` in its workbook, replacing the cell an existing name of
    /// the same spelling pointed at.
    pub fn define_name(&mut self, name: &str, column: ColumnId, row: usize) -> Result<WindowId, ProjectError> {
        check_name(name).map_err(ProjectError::BadName)?;
        let book = self.locate(column).map(|(b, _, _)| b.id).ok_or(ProjectError::NoSuchColumn(column))?;
        let b = self.workbook_mut(book).ok_or(ProjectError::NoSuchColumn(column))?;
        b.names.retain(|n| !n.name.eq_ignore_ascii_case(name));
        b.names.push(NamedCell { name: name.to_string(), column, row });
        Ok(book)
    }

    /// Removes the name `name` from workbook `book`; false when there was none.
    pub fn delete_name(&mut self, book: WindowId, name: &str) -> bool {
        let Some(b) = self.workbook_mut(book) else { return false };
        let before = b.names.len();
        b.names.retain(|n| !n.name.eq_ignore_ascii_case(name));
        b.names.len() < before
    }

    /// The name of the cell at `row` of `column`, if it has one.
    pub fn name_of_cell(&self, column: ColumnId, row: usize) -> Option<&str> {
        let (b, _, _) = self.locate(column)?;
        b.names.iter().find(|n| n.column == column && n.row == row).map(|n| n.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn names_follow_the_rules() {
        assert!(check_name("Rate").is_ok() && check_name("k_2").is_ok());
        for bad in ["", "2x", "a-b", "i", "J", "Pi", &"a".repeat(33)] {
            assert!(check_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_formula_reads_a_named_cell_and_follows_it() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 0.5])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![])).unwrap();
        p.define_name("Rate", a, 1).unwrap();
        assert_eq!(p.name_of_cell(a, 1), Some("Rate"));
        p.set_column_values_with(c, "col(B)*rate", None, crate::column::RecalcMode::Auto).unwrap();
        assert_eq!(p.numeric(c).unwrap(), [0.5, 1.0, 1.5], "a name is read regardless of case");
        // Changing the named cell changes the result (the formula depends on its column).
        let before = p.clone();
        p.column_mut(a).unwrap().numeric_mut().unwrap()[1] = 10.0;
        p.recalculate_since(&before);
        assert_eq!(p.numeric(c).unwrap(), [10.0, 20.0, 30.0]);
        // Same name again: the latest cell; a bad name is refused.
        p.define_name("RATE", a, 0).unwrap();
        assert_eq!(p.workbook(book).unwrap().names.len(), 1);
        assert!(p.define_name("j", a, 0).is_err());
        assert!(p.delete_name(book, "rate"));
        let _ = b;
    }
}
