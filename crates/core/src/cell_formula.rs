//! Cell formulas (T448; Origin 2019's cell formulas and Origin 2021's Formula Bar, Sheaf's own rules): a worksheet
//! cell typed as `=expression` keeps the expression and shows its value, recalculated inside every edit's undo step
//! (as cell links are, T211). The expression is a column formula (`docs/spec/expression-language.md`) in which a
//! spreadsheet reference names a cell of the sheet:
//!
//! - `B3` is column B, row 3 (`col(B)[3]`); `A1:A5` is rows 1 to 5 of A (`col(A)[1:5]`, for `sum`, `mean` ...);
//! - `Sheet2!B3` is a cell of another sheet of the same workbook;
//! - a reference is a run of capital letters naming a column of the sheet followed by a row number, not inside quotes
//!   and not followed by `(` (so `LOG10(x)` stays a call);
//! - `sum(...)` is the total, as in a spreadsheet (`total`; in a column formula `sum` is Origin's running sum);
//! - `$A$1`, `A$1` and `$A1` are the same cell as `A1`; the `$` before the row keeps it when the formula is filled
//!   down (T455: Fill Down copies a cell's formula to the rows below with its other row numbers shifted).
//!
//! Sheaf's choices (Origin 2019 not measured here): formulas that read other formula cells are worked out in up to
//! ten passes, so chains settle; a formula that cannot be read or run leaves its cell empty; typing a value over the
//! cell drops its formula. A formula cell moves with its row when rows are inserted, deleted or sorted (T462).
//! Its references to the sheet's own cells follow rows inserted or deleted there, as in a spreadsheet (T491): a
//! reference below the change moves with its row, a range grows or shrinks, a reference to a deleted cell becomes
//! `#REF!` (the formula then leaves its cell empty); `$` does not hold a reference still here, it only keeps its row in
//! Fill Down. Sorting rewrites no reference, and references from other sheets stay as typed.

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
    let formula = strip_dollars(formula);
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
                // An explicit range stays a range even over one row (`sum(A1:A1)`).
                let (rows_text, end) = match range_end(&chars, end, &reference, &letters) {
                    Some((last, end)) => (format!("{row}:{last}"), end),
                    None => (rows(row, row), end),
                };
                out.push_str(&format!("[{book}]{sheet}!col({letters})[{rows_text}]"));
                i = end;
                continue;
            }
            if let Some((letters, row, end)) = reference(i).filter(|(l, _, _)| has_column(None, l)) {
                let (rows_text, end) = match range_end(&chars, end, &reference, &letters) {
                    Some((last, end)) => (format!("{row}:{last}"), end),
                    None => (rows(row, row), end),
                };
                out.push_str(&format!("col({letters})[{rows_text}]"));
                i = end;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// `formula` without the `$` markers of its references (`$A$1` is `A1`); a `$` before anything but a capital
/// letter or a digit (`col(A)$`, `$(...)`), after `)`, or inside quotes stays.
fn strip_dollars(formula: &str) -> String {
    let chars: Vec<char> = formula.chars().collect();
    let mut out = String::new();
    let mut quoted = false;
    for (i, &c) in chars.iter().enumerate() {
        if c == '"' {
            quoted = !quoted;
        }
        let marker = c == '$'
            && !quoted
            && chars.get(i + 1).is_some_and(|n| n.is_ascii_uppercase() || n.is_ascii_digit())
            && (i == 0 || chars[i - 1] != ')');
        if !marker {
            out.push(c);
        }
    }
    out
}

/// `formula` with the row number of each reference moved by `offset` (Fill Down), except where `$` stands before
/// the row. `has_column(name)` says whether the formula's sheet has a column of that short name (references to
/// other sheets move too when that sheet has the column; the check uses the formula's own sheet).
pub fn shift_rows(formula: &str, offset: i64, has_column: &dyn Fn(&str) -> bool) -> String {
    let chars: Vec<char> = formula.chars().collect();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = String::new();
    let mut i = 0;
    let mut quoted = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            quoted = !quoted;
        }
        let starts = !quoted && (i == 0 || !(word(chars[i - 1]) || chars[i - 1] == '$'));
        if starts {
            // [$]LETTERS[$]DIGITS, not followed by a word character or `(`.
            let mut k = i;
            let col_abs = chars.get(k) == Some(&'$');
            if col_abs {
                k += 1;
            }
            let l0 = k;
            while k < chars.len() && chars[k].is_ascii_uppercase() {
                k += 1;
            }
            let letters: String = chars[l0..k].iter().collect();
            let row_abs = chars.get(k) == Some(&'$');
            if row_abs {
                k += 1;
            }
            let d0 = k;
            while k < chars.len() && chars[k].is_ascii_digit() {
                k += 1;
            }
            let ends = k == chars.len() || !(word(chars[k]) || chars[k] == '(');
            if !letters.is_empty() && k > d0 && ends && has_column(&letters) {
                let row: i64 = chars[d0..k].iter().collect::<String>().parse().unwrap_or(1);
                let row = if row_abs { row } else { (row + offset).max(1) };
                out.push_str(&format!(
                    "{}{letters}{}{row}",
                    if col_abs { "$" } else { "" },
                    if row_abs { "$" } else { "" }
                ));
                i = k;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// A change of a sheet's rows that references to its cells follow (T491); rows from 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RowEdit {
    /// `count` rows inserted before row `at`.
    Insert { at: usize, count: usize },
    /// These rows deleted.
    Delete(std::ops::Range<usize>),
}

impl RowEdit {
    /// Where the row numbered `r` (from 1) goes; `None` when it was deleted.
    fn row(&self, r: usize) -> Option<usize> {
        match self {
            RowEdit::Insert { at, count } => Some(if r > *at { r + count } else { r }),
            RowEdit::Delete(d) if r <= d.start => Some(r),
            RowEdit::Delete(d) if r > d.end => Some(r - d.len()),
            RowEdit::Delete(_) => None,
        }
    }

    /// Where the range of rows `first..=last` (from 1) goes: rows inserted inside it make it longer, deleted rows
    /// shorter; `None` when every row of it was deleted.
    fn range(&self, first: usize, last: usize) -> Option<(usize, usize)> {
        match self {
            RowEdit::Insert { .. } => Some((self.row(first)?, self.row(last)?)),
            RowEdit::Delete(d) => {
                let f = self.row(first).unwrap_or(d.start + 1);
                let l = self.row(last).unwrap_or(d.start);
                (l >= f && l >= 1).then_some((f, l))
            }
        }
    }
}

/// A reference `[$]LETTERS[$]DIGITS` at `i` (not inside a word, not followed by a word character or `(`): whether
/// `$` stands before the letters, the letters, whether `$` stands before the row, the row (from 1) and its end.
fn reference_at(chars: &[char], i: usize) -> Option<(bool, String, bool, usize, usize)> {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    if i > 0 && (word(chars[i - 1]) || chars[i - 1] == '$') {
        return None;
    }
    let mut k = i;
    let col_abs = chars.get(k) == Some(&'$');
    k += usize::from(col_abs);
    let l0 = k;
    while k < chars.len() && chars[k].is_ascii_uppercase() {
        k += 1;
    }
    let letters: String = chars[l0..k].iter().collect();
    let row_abs = chars.get(k) == Some(&'$');
    k += usize::from(row_abs);
    let d0 = k;
    while k < chars.len() && chars[k].is_ascii_digit() {
        k += 1;
    }
    if letters.is_empty() || k == d0 || (k < chars.len() && (word(chars[k]) || chars[k] == '(')) {
        return None;
    }
    let row: usize = chars[d0..k].iter().collect::<String>().parse().ok()?;
    (row >= 1).then_some((col_abs, letters, row_abs, row, k))
}

/// `formula` with its references to moved cells rewritten for `edit` (T491). `moved(sheet, letters)` says whether
/// the column of that short name (in the sheet named, `None` the formula's own) is one whose rows changed. A
/// reference to a deleted cell, or a range all of whose rows were deleted, becomes `#REF!`.
pub(crate) fn move_references(formula: &str, edit: &RowEdit, moved: &dyn Fn(Option<&str>, &str) -> bool) -> String {
    let chars: Vec<char> = formula.chars().collect();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let text = |abs: bool, letters: &str, row_abs: bool, row: usize| {
        format!("{}{letters}{}{row}", if abs { "$" } else { "" }, if row_abs { "$" } else { "" })
    };
    let mut out = String::new();
    let mut i = 0;
    let mut quoted = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            quoted = !quoted;
        }
        if !quoted && (i == 0 || !(word(chars[i - 1]) || chars[i - 1] == '$')) {
            // An optional `Sheet!` before the reference.
            let bang = (i..chars.len()).take_while(|&k| word(chars[k]) || chars[k] == '!').find(|&k| chars[k] == '!');
            let (sheet, at) = match bang {
                Some(b) if b > i => (Some(chars[i..b].iter().collect::<String>()), b + 1),
                _ => (None, i),
            };
            if let Some((ca, letters, ra, row, end)) = reference_at(&chars, at)
                && moved(sheet.as_deref(), &letters)
            {
                let prefix = sheet.as_ref().map(|s| format!("{s}!")).unwrap_or_default();
                // A range of the same column: `A1:A5`.
                let range = (chars.get(end) == Some(&':'))
                    .then(|| reference_at(&chars, end + 1))
                    .flatten()
                    .filter(|(_, l2, _, r2, _)| *l2 == letters && *r2 >= row);
                match range {
                    Some((ca2, _, ra2, row2, end2)) => {
                        match edit.range(row, row2) {
                            Some((f, l)) => out.push_str(&format!(
                                "{prefix}{}:{}",
                                text(ca, &letters, ra, f),
                                text(ca2, &letters, ra2, l)
                            )),
                            None => out.push_str("#REF!"),
                        }
                        i = end2;
                    }
                    None => {
                        match edit.row(row) {
                            Some(r) => out.push_str(&format!("{prefix}{}", text(ca, &letters, ra, r))),
                            None => out.push_str("#REF!"),
                        }
                        i = end;
                    }
                }
                continue;
            }
            // Not a moved reference: copy the sheet name and the reference as they are.
            if let Some((.., end)) = reference_at(&chars, at) {
                out.extend(&chars[i..end]);
                i = end;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Spreadsheet function names that differ in Sheaf's cell formulas (T499; `AVERAGE` T492), any case. `INT` and
/// `MOD` are left alone: Sheaf has functions of those names (`int` truncates, `mod` keeps the dividend's sign).
const SPREADSHEET_NAMES: [(&str, &str); 8] = [
    ("average", "mean"),
    ("stdev", "sd"),
    ("stdev.s", "sd"),
    ("stdevp", "stddevp"),
    ("stdev.p", "stddevp"),
    ("var.s", "var"),
    ("power", "pow"),
    ("trunc", "int"),
];

/// A spreadsheet formula in the spelling of Sheaf's cell formulas (T492, T499): the function names of
/// [`SPREADSHEET_NAMES`] replaced, `PI()` written `pi`, the comparisons `=` and `<>` written `==` and `!=`; other
/// names are the same in both (any case: IF, MEDIAN, VAR, SQRT ...), text in quotes stays.
pub fn spreadsheet_names(formula: &str) -> String {
    let chars: Vec<char> = formula.chars().collect();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = String::new();
    let mut i = 0;
    let mut quoted = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            quoted = !quoted;
        }
        if !quoted && c == '<' && chars.get(i + 1) == Some(&'>') {
            out.push_str("!=");
            i += 2;
            continue;
        }
        // A lone `=` (not part of `==`, `<=`, `>=`, `!=`) is a comparison.
        if !quoted
            && c == '='
            && chars.get(i + 1) != Some(&'=')
            && (i == 0 || !matches!(chars[i - 1], '<' | '>' | '!' | '='))
        {
            out.push_str("==");
            i += 1;
            continue;
        }
        if !quoted && c.is_alphabetic() && (i == 0 || !word(chars[i - 1])) {
            let mut k = i;
            while k < chars.len() && word(chars[k]) {
                k += 1;
            }
            // `STDEV.S`: a dotted spreadsheet name.
            if chars.get(k) == Some(&'.') && chars.get(k + 1).is_some_and(|c| c.is_alphabetic()) {
                let mut e = k + 1;
                while e < chars.len() && word(chars[e]) {
                    e += 1;
                }
                let dotted: String = chars[i..e].iter().collect();
                if SPREADSHEET_NAMES.iter().any(|(n, _)| dotted.eq_ignore_ascii_case(n)) {
                    k = e;
                }
            }
            let name: String = chars[i..k].iter().collect();
            let mut after = k;
            while chars.get(after).is_some_and(|c| c.is_whitespace()) {
                after += 1;
            }
            if chars.get(after) == Some(&'(') {
                if let Some((_, sheaf)) = SPREADSHEET_NAMES.iter().find(|(n, _)| name.eq_ignore_ascii_case(n)) {
                    out.push_str(sheaf);
                    i = k;
                    continue;
                }
                let mut close = after + 1;
                while chars.get(close).is_some_and(|c| c.is_whitespace()) {
                    close += 1;
                }
                if name.eq_ignore_ascii_case("pi") && chars.get(close) == Some(&')') {
                    out.push_str("pi");
                    i = close + 1;
                    continue;
                }
            }
            out.extend(&chars[i..k]);
            i = k;
            continue;
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

impl crate::worksheet::Worksheet {
    /// Gives the formula cells the columns' new ids (`None` drops a cell's formula), as an importer's workbook gets
    /// fresh ids when it joins a project (T464).
    pub(crate) fn remap_formula_columns(&mut self, map: &dyn Fn(ColumnId) -> Option<ColumnId>) {
        let old = std::mem::take(&mut self.cell_formulas);
        self.cell_formulas = old
            .into_iter()
            .filter_map(|mut f| {
                f.column = map(f.column)?;
                Some(f)
            })
            .collect();
    }

    /// Moves the formula cells of these columns for `count` rows inserted before row `at` (T462), and their
    /// references to the cells of these columns (T491).
    pub(crate) fn formulas_insert_rows(&mut self, columns: &[ColumnId], at: usize, count: usize) {
        for f in &mut self.cell_formulas {
            if columns.contains(&f.column) && f.row >= at {
                f.row += count;
            }
        }
        self.formulas_follow_rows(columns, &RowEdit::Insert { at, count });
    }

    /// Rewrites every formula's references to the cells of these columns of this sheet for `edit` (T491).
    fn formulas_follow_rows(&mut self, columns: &[ColumnId], edit: &RowEdit) {
        if self.cell_formulas.is_empty() {
            return;
        }
        let names: Vec<String> =
            self.columns.iter().filter(|c| columns.contains(&c.id)).map(|c| c.short_name.clone()).collect();
        let me = self.name.clone();
        let moved = |sheet: Option<&str>, letters: &str| {
            sheet.is_none_or(|s| s.eq_ignore_ascii_case(&me)) && names.iter().any(|n| n == letters)
        };
        for f in &mut self.cell_formulas {
            f.formula = move_references(&f.formula, edit, &moved);
        }
    }

    /// Drops the formulas on the deleted `rows` of these columns and moves up those below (T462).
    pub(crate) fn formulas_delete_rows(&mut self, columns: &[ColumnId], rows: std::ops::Range<usize>) {
        self.cell_formulas.retain(|f| !(columns.contains(&f.column) && rows.contains(&f.row)));
        for f in &mut self.cell_formulas {
            if columns.contains(&f.column) && f.row >= rows.end {
                f.row -= rows.len();
            }
        }
        self.formulas_follow_rows(columns, &RowEdit::Delete(rows));
    }

    /// Puts the formula cells of these columns in the order `order` (new row i takes old row `order[i]`); formulas
    /// on rows not in `order` are dropped (T462).
    pub(crate) fn formulas_reorder_rows(&mut self, columns: &dyn Fn(ColumnId) -> bool, order: &[usize], rows: usize) {
        if self.cell_formulas.is_empty() {
            return;
        }
        let mut new_row = std::collections::HashMap::new();
        for (i, src) in order.iter().enumerate() {
            new_row.entry(*src).or_insert(i);
        }
        let old = std::mem::take(&mut self.cell_formulas);
        self.cell_formulas = old
            .into_iter()
            .filter_map(|mut f| {
                if columns(f.column) && f.row < rows {
                    f.row = *new_row.get(&f.row)?;
                }
                Some(f)
            })
            .collect();
    }
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

    /// Fill Down (T455): the formula of the cell of `column` at `row` copied to the rows below it up to `last`
    /// (0-based), each with its relative row numbers shifted by its distance. Returns how many cells were filled.
    pub fn fill_cell_formula_down(&mut self, column: ColumnId, row: usize, last: usize) -> Result<usize, ProjectError> {
        let formula = self.cell_formula(column, row).ok_or(ProjectError::NoCellFormula)?.to_string();
        let (_, sheet, _) = self.locate(column).ok_or(ProjectError::NoSuchColumn(column))?;
        let names: Vec<String> = sheet.columns.iter().map(|c| c.short_name.clone()).collect();
        let has = |n: &str| names.iter().any(|c| c == n);
        let filled: Vec<CellFormula> = (row + 1..=last)
            .map(|r| CellFormula { column, row: r, formula: shift_rows(&formula, (r - row) as i64, &has) })
            .collect();
        let n = filled.len();
        let sheet = self.sheet_of_mut(column).ok_or(ProjectError::NoSuchColumn(column))?;
        sheet.cell_formulas.retain(|f| !(f.column == column && f.row > row && f.row <= last));
        sheet.cell_formulas.extend(filled);
        self.refresh_cell_formulas();
        Ok(n)
    }

    /// Pasted spreadsheet formulas (T492): each `(column, row, formula, text)` whose formula (without `=`) can be
    /// read, after [`spreadsheet_names`], becomes that cell's formula; the others are written as their pasted `text`,
    /// as typed. Every cell formula is then worked out once. Returns how many became formulas.
    pub fn paste_cell_formulas(&mut self, cells: Vec<(ColumnId, usize, String, String)>) -> usize {
        let mut n = 0;
        for (column, row, formula, text) in cells {
            let formula = spreadsheet_names(&formula);
            let readable = self.translated(column, &formula).is_some_and(|src| sheaf_expr::Expr::parse(&src).is_ok());
            if readable && let Some(sheet) = self.sheet_of_mut(column) {
                sheet.cell_formulas.retain(|f| !(f.column == column && f.row == row));
                sheet.cell_formulas.push(CellFormula { column, row, formula });
                n += 1;
            } else if let Some(c) = self.column_mut(column) {
                c.set_cell_text(row, &text);
            }
        }
        self.refresh_cell_formulas();
        n
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
        assert_eq!(translate("$A$1 + A$2 + $B3", "Book1", &has), "col(A)[1] + col(A)[2] + col(B)[3]", "T455");
        assert_eq!(translate("col(A)$ + \"$A1\"", "Book1", &has), "col(A)$ + \"$A1\"", "a text suffix, quotes");
        let cols = |n: &str| ["A", "B"].contains(&n);
        assert_eq!(shift_rows("A1*$A$1 + B$2 + sum(A1:A3)", 2, &cols), "A3*$A$1 + B$2 + sum(A3:A5)");
        assert_eq!(shift_rows("LOG10(A1) + Z1", 1, &cols), "LOG10(A2) + Z1", "a call and an unknown column stay");
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
        // Fill Down (T455): C2 = A2 * $A$1 ... down to row 3.
        let d = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![f64::NAN; 3])).unwrap();
        p.set_cell_formula(d, 0, "A1*$A$1").unwrap();
        assert_eq!(p.fill_cell_formula_down(d, 0, 2).unwrap(), 2);
        assert_eq!(p.cell_formula(d, 2), Some("A3*$A$1"));
        assert_eq!(texts(&p, d), ["1", "2", "10"]);
        // Rows move (T462): a row inserted above moves the formula cell down and its references with their rows
        // (T491); deleting it moves them back.
        let sheet = &mut p.workbook_mut(b).unwrap().sheets[0];
        sheet.insert_rows(&[0, 1, 2], 0, 1);
        assert_eq!(p.cell_formula(d, 3), Some("A4*$A$2"), "row 3 became row 4");
        let sheet = &mut p.workbook_mut(b).unwrap().sheets[0];
        sheet.delete_rows(&[0, 1, 2], 0..1);
        assert_eq!(p.cell_formula(d, 2), Some("A3*$A$1"));
        // Typed over: the value stays, the formula goes.
        assert!(p.drop_cell_formula(c, 0));
        assert!(p.cell_formula(c, 0).is_none());
        assert!(p.set_cell_formula(c, 2, "A1 +* 2").is_err(), "not a formula");
    }

    #[test]
    fn pasted_spreadsheet_formulas_become_cell_formulas() {
        assert_eq!(spreadsheet_names("AVERAGE(A1:A3) + Pi() * average (B1:B2)"), "mean(A1:A3) + pi * mean (B1:B2)");
        // T499: comparisons and more names; Sheaf's own spelling stays.
        assert_eq!(spreadsheet_names("IF(A1=2, STDEV.S(B1:B3), A1<>3)"), "IF(A1==2, sd(B1:B3), A1!=3)");
        assert_eq!(
            spreadsheet_names("A1==2 + (A1<=1) + (A1>=1) + (A1!=1) + POWER(A1, 2)"),
            "A1==2 + (A1<=1) + (A1>=1) + (A1!=1) + pow(A1, 2)"
        );
        assert_eq!(spreadsheet_names("\"a=b\" + MOD(A1, 2)"), "\"a=b\" + MOD(A1, 2)", "quotes and Sheaf's mod stay");
        assert_eq!(
            spreadsheet_names("\"AVERAGE(x)\" + MYAVERAGE(A1) + SQRT(A1)"),
            "\"AVERAGE(x)\" + MYAVERAGE(A1) + SQRT(A1)"
        );
        let mut p = Project::new();
        let b = p.add_workbook("");
        let a = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![9.0; 3])).unwrap();
        p.set_cell_formula(c, 2, "A1*1000").unwrap();
        // Two rows into B: a formula, a number, Excel names, a formula Sheaf cannot read (stays as typed text).
        let r = p.paste_text(b, 0, 1, 0, "=A1*10\n5\n=AVERAGE(A1:A3)+PI()-pi").unwrap();
        assert_eq!(r.formulas, 2);
        assert_eq!(p.cell_formula(c, 0), Some("A1*10"));
        assert_eq!(p.cell_formula(c, 1), None);
        assert_eq!(p.cell_formula(c, 2), Some("mean(A1:A3)+pi-pi"), "the old formula of B3 was replaced");
        assert_eq!(texts(&p, c), ["10", "5", "2"]);
        // A pasted value over a formula cell drops the formula; an unreadable formula is pasted as text.
        p.paste_text(b, 0, 1, 0, "7\n=A1 +* 2").unwrap();
        assert_eq!(p.cell_formula(c, 0), None);
        assert_eq!(texts(&p, c)[0], "7");
        assert_eq!(p.cell_formula(c, 1), None, "not a formula");
        // T499: IF with an Excel comparison, and a sample standard deviation.
        p.paste_text(b, 0, 1, 0, "=IF(A1=1, STDEV(A1:A3), -1)").unwrap();
        assert_eq!(p.cell_formula(c, 0), Some("IF(A1==1, sd(A1:A3), -1)"));
        assert_eq!(texts(&p, c)[0], "1");
        // A pasted formula reads the cells pasted with it.
        p.paste_text(b, 0, 0, 0, "4\t=A1+1").unwrap();
        assert_eq!(texts(&p, c)[0], "5");
        assert_eq!(p.column(a).unwrap().numeric().unwrap()[0], 4.0);
    }

    #[test]
    fn references_follow_inserted_and_deleted_rows() {
        let moved = |sheet: Option<&str>, l: &str| ["A", "B"].contains(&l) && sheet.is_none_or(|s| s == "Sheet1");
        let ins = RowEdit::Insert { at: 2, count: 2 };
        // Rows 3 on move down 2; a range across the insertion grows; one above stays; C is not moved.
        assert_eq!(
            move_references("A1 + A3 + $B$5 + sum(A2:A4) + C9 + Sheet1!B3 + Sheet2!B3", &ins, &moved),
            "A1 + A5 + $B$7 + sum(A2:A6) + C9 + Sheet1!B5 + Sheet2!B3"
        );
        assert_eq!(move_references("sum(A3:A4) + \"A3\" + LOG10(A3)", &ins, &moved), "sum(A5:A6) + \"A3\" + LOG10(A5)");
        // Rows 3 and 4 deleted: below moves up, a range shrinks, a deleted cell or range is #REF!.
        let del = RowEdit::Delete(2..4);
        assert_eq!(
            move_references("A2 + A3 + A6 + sum(A1:A5) + sum(A3:A6) + sum(A3:A4)", &del, &moved),
            "A2 + #REF! + A4 + sum(A1:A3) + sum(A3:A4) + sum(#REF!)"
        );
        assert_eq!(move_references("sum(A1:A3)", &del, &moved), "sum(A1:A2)");
    }

    #[test]
    fn formula_cells_keep_reading_their_inputs_when_rows_move() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let a = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![f64::NAN; 4])).unwrap();
        p.set_cell_formula(c, 0, "A4*10").unwrap();
        p.set_cell_formula(c, 1, "sum(A2:A4)").unwrap();
        assert_eq!(texts(&p, c), ["40", "9", "", ""]);
        // A whole row inserted before row 3: A4 is now A5 and the sum covers the new row too.
        p.workbook_mut(b).unwrap().sheets[0].insert_rows(&[0, 1], 2, 1);
        p.column_mut(a).unwrap().set_cell_text(2, "100");
        p.refresh_cell_formulas();
        assert_eq!(p.cell_formula(c, 0), Some("A5*10"));
        assert_eq!(p.cell_formula(c, 1), Some("sum(A2:A5)"));
        assert_eq!(texts(&p, c)[..2], ["40", "109"]);
        // Cells of A only inserted: B's own cells stay, its references to A still move.
        p.workbook_mut(b).unwrap().sheets[0].insert_rows(&[0], 0, 1);
        assert_eq!(p.cell_formula(c, 0), Some("A6*10"));
        // The row A6 is on deleted: that reference is lost and its cell empty; the sum shrinks.
        p.workbook_mut(b).unwrap().sheets[0].delete_rows(&[0], 5..6);
        p.refresh_cell_formulas();
        assert_eq!(p.cell_formula(c, 0), Some("#REF!*10"));
        assert_eq!(p.cell_formula(c, 1), Some("sum(A3:A5)"));
        assert_eq!(texts(&p, c)[..2], ["", "105"]);
    }
}
