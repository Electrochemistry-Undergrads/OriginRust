//! Cell formulas (T448; Origin 2019's cell formulas and Origin 2021's Formula Bar, Sheaf's own rules): a worksheet
//! cell typed as `=expression` keeps the expression and shows its value, recalculated inside every edit's undo step
//! (as cell links are, T211). The expression is a column formula (`docs/spec/expression-language.md`) in which a
//! spreadsheet reference names a cell of the sheet:
//!
//! - `B3` is column B, row 3 (`col(B)[3]`); `A1:A5` is rows 1 to 5 of A (`col(A)[1:5]`, for `sum`, `mean` ...);
//! - `Sheet2!B3` is a cell of another sheet of the same workbook;
//! - a reference is a run of capital letters naming a column of the sheet followed by a row number, not inside quotes
//!   and not followed by `(` (so `LOG10(x)` stays a call);
//! - `sum(...)` is the total, as in a spreadsheet (`total`; in a column formula `sum` is Origin's running sum).
//!
//! Sheaf's choices (Origin 2019 not measured here): formulas that read other formula cells are worked out in up to
//! ten passes, so chains settle; a formula that cannot be read or run leaves its cell empty; typing a value over the
//! cell drops its formula; moving rows or columns does not move the formulas (as cell links).

use crate::cell_links::{Value, write};
use crate::ids::ColumnId;
use crate::project::{Project, ProjectError, Window};
use serde::{Deserialize, Serialize};

/// A cell formula: the cell of `column` at `row` (0-based) shows `formula` (without the `=`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellFormula {
    pub column: ColumnId,
    pub row: usize,
    pub formula: String,
}

/// `formula` with its spreadsheet references written as column formula references. `has_column(sheet, name)` says
/// whether a sheet (`None` the formula's own) has a column of that short name; `book` is the workbook's name.
pub fn translate(formula: &str, book: &str, has_column: &dyn Fn(Option<&str>, &str) -> bool) -> String {
    let chars: Vec<char> = formula.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut quoted = false;
    let word = |c: char| c.is_alphanumeric() || c == '_';
    // A reference at `i`: capital letters then digits; returns (letters, row, end).
    let reference = |i: usize| -> Option<(String, usize, usize)> {
        if i > 0 && (word(chars[i - 1]) || chars[i - 1] == '$') {
            return None;
        }
        let mut k = i;
        while k < chars.len() && chars[k].is_ascii_uppercase() {
            k += 1;
        }
        let letters_end = k;
        while k < chars.len() && chars[k].is_ascii_digit() {
            k += 1;
        }
        if letters_end == i || k == letters_end || (k < chars.len() && (word(chars[k]) || chars[k] == '(')) {
            return None;
        }
        let row: usize = chars[letters_end..k].iter().collect::<String>().parse().ok()?;
        (row >= 1).then(|| (chars[i..letters_end].iter().collect(), row, k))
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            quoted = !quoted;
        }
        if !quoted {
            // `sum(` is the total in a cell, as in a spreadsheet (in a column formula it is Origin's running sum).
            let at_word_start = i == 0 || !word(chars[i - 1]);
            if at_word_start
                && chars.len() >= i + 4
                && chars[i..i + 3].iter().collect::<String>().eq_ignore_ascii_case("sum")
                && chars[i + 3] == '('
            {
                out.push_str("total(");
                i += 4;
                continue;
            }
            // `Sheet2!B3`: a sheet name, `!`, a reference.
            let sheet_ref = (|| {
                if i > 0 && word(chars[i - 1]) {
                    return None;
                }
                let bang =
                    (i..chars.len()).take_while(|&k| word(chars[k]) || chars[k] == '!').find(|&k| chars[k] == '!')?;
                let sheet: String = chars[i..bang].iter().collect();
                if sheet.is_empty() {
                    return None;
                }
                let (letters, row, end) = reference(bang + 1)?;
                has_column(Some(&sheet), &letters).then_some((sheet, letters, row, end))
            })();
            if let Some((sheet, letters, row, end)) = sheet_ref {
                let (last, end) = range_end(&chars, end, &reference, &letters).unwrap_or((row, end));
                out.push_str(&format!("[{book}]{sheet}!col({letters})[{}]", rows(row, last)));
                i = end;
                continue;
            }
            if let Some((letters, row, end)) = reference(i).filter(|(l, _, _)| has_column(None, l)) {
                let (last, end) = range_end(&chars, end, &reference, &letters).unwrap_or((row, end));
                out.push_str(&format!("col({letters})[{}]", rows(row, last)));
                i = end;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// `row` or `first:last`.
fn rows(first: usize, last: usize) -> String {
    if first == last { first.to_string() } else { format!("{first}:{last}") }
}

/// The end of a range `A1:A5` after its first reference ends at `end`: the last row and where the range ends.
fn range_end(
    chars: &[char],
    end: usize,
    reference: &dyn Fn(usize) -> Option<(String, usize, usize)>,
    letters: &str,
) -> Option<(usize, usize)> {
    if chars.get(end) != Some(&':') {
        return None;
    }
    let (l2, row2, end2) = reference(end + 1)?;
    (l2 == letters).then_some((row2, end2))
}

impl Project {
    /// The formula of a cell, if it has one.
    pub fn cell_formula(&self, column: ColumnId, row: usize) -> Option<&str> {
        let (_, sheet, _) = self.locate(column)?;
        sheet.cell_formulas.iter().find(|f| f.column == column && f.row == row).map(|f| f.formula.as_str())
    }

    /// Gives the cell of `column` at `row` the formula `formula` (without the `=`) and works out every cell formula.
    /// The formula is checked first: one that cannot be read is refused.
    pub fn set_cell_formula(&mut self, column: ColumnId, row: usize, formula: &str) -> Result<(), ProjectError> {
        let src = self.translated(column, formula).ok_or(ProjectError::NoSuchColumn(column))?;
        sheaf_expr::Expr::parse(&src).map_err(|e| ProjectError::CellFormula(e.to_string()))?;
        let sheet = self.sheet_of_mut(column).ok_or(ProjectError::NoSuchColumn(column))?;
        sheet.cell_formulas.retain(|f| !(f.column == column && f.row == row));
        sheet.cell_formulas.push(CellFormula { column, row, formula: formula.trim().to_string() });
        self.refresh_cell_formulas();
        Ok(())
    }

    /// Drops the formula of one cell (typing a value into it); its value stays.
    pub fn drop_cell_formula(&mut self, column: ColumnId, row: usize) -> bool {
        let Some(sheet) = self.sheet_of_mut(column) else { return false };
        let before = sheet.cell_formulas.len();
        sheet.cell_formulas.retain(|f| !(f.column == column && f.row == row));
        before != sheet.cell_formulas.len()
    }

    fn sheet_of_mut(&mut self, column: ColumnId) -> Option<&mut crate::worksheet::Worksheet> {
        self.windows.iter_mut().find_map(|w| match w {
            Window::Workbook(b) => b.sheets.iter_mut().find(|s| s.column(column).is_some()),
            _ => None,
        })
    }

    /// The formula of a cell of `column` as a column formula.
    fn translated(&self, column: ColumnId, formula: &str) -> Option<String> {
        let (book, sheet, _) = self.locate(column)?;
        let has = |other: Option<&str>, name: &str| {
            let s = match other {
                None => Some(sheet),
                Some(n) => book.sheets.iter().find(|s| s.name.eq_ignore_ascii_case(n)),
            };
            s.is_some_and(|s| s.columns.iter().any(|c| c.short_name == name))
        };
        Some(translate(formula, &book.name, &has))
    }

    /// Works out every cell formula of the project and writes the values; true when a cell changed. Formulas that
    /// read formula cells settle over up to ten passes.
    pub fn refresh_cell_formulas(&mut self) -> bool {
        let all: Vec<CellFormula> =
            self.workbooks().flat_map(|b| b.sheets.iter().flat_map(|s| s.cell_formulas.iter().cloned())).collect();
        if all.is_empty() {
            return false;
        }
        let mut changed = false;
        for _ in 0..10 {
            let mut pass = false;
            for f in &all {
                let value = self
                    .translated(f.column, &f.formula)
                    .and_then(|src| self.evaluate_cell(f.column, &src))
                    .unwrap_or(Value::Number(f64::NAN));
                if let Some(c) = self.column_mut(f.column) {
                    pass |= write(c, f.row, &value);
                }
            }
            changed |= pass;
            if !pass {
                break;
            }
        }
        changed
    }

    /// The value of a translated formula, read in `column`'s sheet.
    fn evaluate_cell(&self, column: ColumnId, src: &str) -> Option<Value> {
        let text = self.evaluate_in_sheet(column, src).ok()?;
        Some(match text {
            sheaf_expr::Value::Num(x) => Value::Number(x),
            sheaf_expr::Value::Text(t) => Value::Text(t),
        })
    }
}

/// A column's cells as text, for tests.
#[cfg(test)]
fn texts(p: &Project, c: ColumnId) -> Vec<String> {
    let col = p.column(c).unwrap();
    (0..col.len()).map(|r| crate::indexed::level_text(col, r).unwrap_or_default()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::worksheet::Worksheet;

    #[test]
    fn references_become_column_formula_references() {
        let has = |sheet: Option<&str>, name: &str| ["A", "B", "AB"].contains(&name) && sheet != Some("Nope");
        assert_eq!(translate("B3*2", "Book1", &has), "col(B)[3]*2");
        assert_eq!(translate("sum(A1:A5)/AB2", "Book1", &has), "total(col(A)[1:5])/col(AB)[2]");
        assert_eq!(translate("Sheet2!A1 + 1", "Book1", &has), "[Book1]Sheet2!col(A)[1] + 1");
        assert_eq!(translate("LOG10(A1)", "Book1", &has), "LOG10(col(A)[1])", "a call stays a call");
        assert_eq!(translate("\"A1\" + Z9 + A0", "Book1", &has), "\"A1\" + Z9 + A0", "quotes, no column Z, row 0");
    }

    #[test]
    fn a_cell_formula_follows_its_inputs_and_drops_when_typed_over() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let a = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![f64::NAN; 3])).unwrap();
        p.set_cell_formula(c, 0, "sum(A1:A3)").unwrap();
        p.set_cell_formula(c, 1, "B1*10").unwrap();
        assert_eq!(texts(&p, c), ["6", "60", ""]);
        assert_eq!(p.cell_formula(c, 1), Some("B1*10"));
        // An input changes: both follow (the second reads the first).
        p.column_mut(a).unwrap().set_cell_text(2, "10");
        p.refresh_cell_formulas();
        assert_eq!(texts(&p, c), ["13", "130", ""]);
        // Another sheet of the book.
        p.workbook_mut(b).unwrap().sheets.push(Worksheet::new("Sheet2"));
        let s2 = p.add_column(b, 1, Designation::Y, ColumnData::numeric(vec![7.0])).unwrap();
        p.set_cell_formula(s2, 0, "Sheet1!A1 + 100").unwrap();
        assert_eq!(texts(&p, s2), ["101"]);
        // Saved with the project.
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.cell_formula(c, 0), Some("sum(A1:A3)"));
        // Typed over: the value stays, the formula goes.
        assert!(p.drop_cell_formula(c, 0));
        assert!(p.cell_formula(c, 0).is_none());
        assert!(p.set_cell_formula(c, 2, "A1 +* 2").is_err(), "not a formula");
    }
}
