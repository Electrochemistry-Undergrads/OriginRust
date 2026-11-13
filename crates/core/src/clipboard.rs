//! Cell operations behind Copy, Cut, Paste and Clear in a worksheet.
//!
//! Copy turns cells into tab-separated text. Clear empties cells. Paste reads tab- or
//! comma-separated text into a sheet at a cell, adding rows and columns as needed. All of them
//! are plain functions on the model; the application wraps each call in one undo step.
//!
//! What a copied number looks like follows Origin's two commands: Copy writes what the cells
//! show (the column's number format), Copy full precision writes the shortest text that reads
//! back as exactly the same number. A missing value is an empty field in both. Date and time
//! columns always write the text they show, because their raw numbers mean nothing outside the
//! program.

use crate::column::{Column, ColumnData, Designation};
use crate::ids::WindowId;
use crate::project::{Project, ProjectError};
use crate::worksheet::Worksheet;
use std::ops::Range;
use std::sync::Arc;

/// What Copy writes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct CopyOptions {
    /// Numbers with every digit instead of the column's display format.
    pub full_precision: bool,
    /// Add the label rows (Long Name, Units, Comments, F(x)=, user parameters) above the values.
    pub label_rows: bool,
}

/// What a paste did, for the status line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PasteReport {
    /// Rows and columns of the pasted block.
    pub rows: usize,
    pub columns: usize,
    /// Columns added at the right end to make room.
    pub columns_added: usize,
    /// Columns that were not changed because a formula keeps them up to date.
    pub locked_columns: usize,
    /// Pasted texts that are not numbers but landed in a number column and became missing values.
    pub not_numbers: usize,
}

/// The shortest text that reads back as exactly this number, without a useless ".0".
fn full_text(x: f64) -> String {
    let text = format!("{x:?}");
    text.strip_suffix(".0").map(str::to_string).unwrap_or(text)
}

/// The text of one cell as Copy writes it.
fn copy_cell(column: &Column, row: usize, full_precision: bool) -> String {
    let clean = |s: &str| s.replace(['\t', '\r', '\n'], " ");
    match &column.data {
        ColumnData::Numeric(v) => match v.get(row) {
            Some(x) if x.is_nan() => String::new(),
            Some(x) if full_precision && !column.format.is_date_time() && x.is_finite() => full_text(*x),
            Some(_) => column.shown_text(row),
            None => String::new(),
        },
        ColumnData::Text(v) => v.get(row).map(|s| clean(s)).unwrap_or_default(),
        ColumnData::Mixed { numbers, .. } => match (column.data.text_cell(row), numbers.get(row)) {
            (Some(t), _) => clean(t),
            (None, Some(x)) if x.is_nan() => String::new(),
            (None, Some(x)) if full_precision && !column.format.is_date_time() && x.is_finite() => full_text(*x),
            (None, Some(_)) => column.shown_text(row),
            (None, None) => String::new(),
        },
    }
}

impl Worksheet {
    /// The cells of the columns at these positions (in this order), rows `rows`, as text with a
    /// tab between the cells of a row and a line break after every row.
    pub fn copy_text(&self, columns: &[usize], rows: Range<usize>, options: CopyOptions) -> String {
        let cols: Vec<&Column> = columns.iter().filter_map(|i| self.columns.get(*i)).collect();
        let mut out = String::new();
        let mut line = |cells: Vec<String>| {
            out.push_str(&cells.join("\t"));
            out.push('\n');
        };
        if options.label_rows {
            let clean = |s: &str| s.replace(['\t', '\r', '\n'], " ");
            line(cols.iter().map(|c| clean(&c.long_name)).collect());
            line(cols.iter().map(|c| clean(&c.units)).collect());
            line(cols.iter().map(|c| clean(&c.comments)).collect());
            if cols.iter().any(|c| c.formula.is_some()) {
                line(cols.iter().map(|c| clean(c.formula.as_deref().unwrap_or_default())).collect());
            }
            for name in self.parameter_names() {
                if cols.iter().any(|c| c.params.contains_key(&name)) {
                    line(cols.iter().map(|c| clean(c.params.get(&name).map_or("", String::as_str))).collect());
                }
            }
        }
        for row in rows {
            line(cols.iter().map(|c| copy_cell(c, row, options.full_precision)).collect());
        }
        out
    }

    /// The names of the user parameters (extra label rows) that any column of the sheet has, in
    /// alphabetical order.
    pub fn parameter_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.columns.iter().flat_map(|c| c.params.keys().cloned()).collect();
        names.sort();
        names.dedup();
        names
    }

    /// Empties the cells of the columns at these positions in rows `rows`: numbers become
    /// missing values and text becomes empty. Columns kept up to date by a formula are left
    /// alone. Missing values at the end of a column are removed, so that a cleared block does not
    /// leave rows of "--" behind. Returns the number of columns left alone because of a formula.
    pub fn clear_cells(&mut self, columns: &[usize], rows: Range<usize>) -> usize {
        let mut locked = 0;
        for &i in columns {
            let Some(c) = self.columns.get_mut(i) else {
                continue;
            };
            if c.is_locked() {
                locked += 1;
                continue;
            }
            let end = rows.end.min(c.len());
            if rows.start < end {
                match &mut c.data {
                    ColumnData::Numeric(v) => Arc::make_mut(v)[rows.start..end].fill(f64::NAN),
                    ColumnData::Text(v) => Arc::make_mut(v)[rows.start..end].fill(String::new()),
                    ColumnData::Mixed { numbers, texts } => {
                        let stop = end.min(numbers.len());
                        Arc::make_mut(numbers)[rows.start..stop].fill(f64::NAN);
                        Arc::make_mut(texts).retain(|r, _| *r < rows.start || *r >= end);
                    }
                }
                c.trim_missing_tail();
            }
        }
        locked
    }
}

impl Column {
    /// True when the cell holds no value: a missing number or empty text. Rows beyond the data
    /// count as empty.
    pub fn is_blank_at(&self, row: usize) -> bool {
        match &self.data {
            ColumnData::Numeric(v) => v.get(row).is_none_or(|x| x.is_nan()),
            ColumnData::Text(v) => v.get(row).is_none_or(|s| s.is_empty()),
            ColumnData::Mixed { numbers, .. } => {
                numbers.get(row).is_none_or(|x| x.is_nan()) && self.data.text_cell(row).is_none()
            }
        }
    }

    /// Removes missing values (empty text) from the end of the column.
    pub fn trim_missing_tail(&mut self) {
        let keep = (0..self.len()).rev().find(|r| !self.is_blank_at(*r)).map_or(0, |r| r + 1);
        if keep == self.len() {
            return;
        }
        match &mut self.data {
            ColumnData::Numeric(v) => Arc::make_mut(v).truncate(keep),
            ColumnData::Text(v) => Arc::make_mut(v).truncate(keep),
            ColumnData::Mixed { numbers, texts } => {
                Arc::make_mut(numbers).truncate(keep);
                Arc::make_mut(texts).retain(|r, _| *r < keep);
            }
        }
    }

    /// True when no cell outside `rows` holds a value.
    fn blank_outside(&self, rows: &Range<usize>) -> bool {
        (0..self.len()).filter(|r| !rows.contains(r)).all(|r| self.is_blank_at(r))
    }
}

/// Reads a number as typed or pasted: leading and trailing blanks are ignored.
fn parse_number(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|x| !x.is_nan())
}

/// True for a field that stands for a missing value.
fn is_missing(text: &str) -> bool {
    let t = text.trim();
    t.is_empty() || t == "--"
}

/// Splits pasted text into rows and fields. The delimiter is a tab when the text has any, else a
/// comma when the text has any, else every line is one field. A field in double quotes may hold
/// the delimiter; two quotes in a row stand for one. One line break at the very end is ignored.
pub fn parse_table(text: &str) -> Vec<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let text = text.strip_suffix("\r\n").or_else(|| text.strip_suffix('\n')).unwrap_or(text);
    if text.is_empty() {
        return Vec::new();
    }
    let delimiter = if text.contains('\t') {
        Some('\t')
    } else if text.contains(',') {
        Some(',')
    } else {
        None
    };
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    let mut field_start = true;
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(ch);
            }
            continue;
        }
        match ch {
            '"' if field_start && delimiter.is_some() => {
                quoted = true;
                field_start = false;
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                field_start = true;
            }
            c if Some(c) == delimiter => {
                row.push(std::mem::take(&mut field));
                field_start = true;
            }
            c => {
                field.push(c);
                field_start = false;
            }
        }
    }
    row.push(field);
    rows.push(row);
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    for r in &mut rows {
        r.resize(width, String::new());
    }
    rows
}

impl Project {
    /// Pastes text into a sheet with its top left cell at column position `col` and row `row`
    /// (both 0-based). Columns are added at the right end when the block is wider than the
    /// sheet; rows are added when it is longer. Numbers are stored as numbers and other text as
    /// text: a column that was empty before takes the type of what is pasted into it, a column
    /// with data keeps its type (text pasted into a number column becomes a missing value). A
    /// column that a formula keeps up to date is not changed.
    pub fn paste_text(
        &mut self,
        book: WindowId,
        sheet: usize,
        col: usize,
        row: usize,
        text: &str,
    ) -> Result<PasteReport, ProjectError> {
        let table = parse_table(text);
        let mut report =
            PasteReport { rows: table.len(), columns: table.first().map_or(0, Vec::len), ..Default::default() };
        if table.is_empty() {
            return Ok(report);
        }
        let have = self
            .workbook(book)
            .ok_or(ProjectError::NoSuchWindow(book))?
            .sheets
            .get(sheet)
            .ok_or(ProjectError::NoSuchLayer(sheet))?
            .columns
            .len();
        // A paste beyond the last column starts at the first free one.
        let col = col.min(have);
        for _ in have..col + report.columns {
            self.add_column(book, sheet, Designation::Y, ColumnData::default())?;
            report.columns_added += 1;
        }
        let ws =
            self.workbook_mut(book).ok_or(ProjectError::NoSuchWindow(book))?.sheets.get_mut(sheet).expect("checked");
        let block = row..row + table.len();
        for j in 0..report.columns {
            let column = &mut ws.columns[col + j];
            if column.is_locked() {
                report.locked_columns += 1;
                continue;
            }
            let cells: Vec<&str> = table.iter().map(|r| r[j].as_str()).collect();
            report.not_numbers += paste_column(column, &block, &cells);
        }
        Ok(report)
    }
}

/// Writes pasted texts into one column. Returns how many became missing values because they are
/// not numbers.
fn paste_column(column: &mut Column, block: &Range<usize>, cells: &[&str]) -> usize {
    let all_numbers = cells.iter().all(|c| is_missing(c) || parse_number(c).is_some());
    let entirely_blank = (0..column.len()).all(|r| column.is_blank_at(r));
    let date_like = column.format.is_date_time();
    let mut not_numbers = 0;
    // Decide the type of the column after the paste.
    // A Text & Numeric column keeps the text cells, so only a plain Numeric column turns to text.
    let become_text =
        matches!(column.data, ColumnData::Numeric(_)) && !all_numbers && !date_like && column.blank_outside(block);
    let become_numbers = !column.data.is_numeric() && all_numbers && entirely_blank;
    if become_text {
        let mut v: Vec<String> = vec![String::new(); column.len().max(block.end)];
        for (i, c) in cells.iter().enumerate() {
            v[block.start + i] = (*c).to_string();
        }
        column.data = ColumnData::text(v);
        column.trim_missing_tail();
        return 0;
    }
    if become_numbers {
        column.data = ColumnData::mixed_numbers(Vec::new());
    }
    match &mut column.data {
        ColumnData::Text(v) => {
            let v = Arc::make_mut(v);
            if v.len() < block.end {
                v.resize(block.end, String::new());
            }
            for (i, c) in cells.iter().enumerate() {
                v[block.start + i] = (*c).to_string();
            }
        }
        ColumnData::Numeric(_) | ColumnData::Mixed { .. } => {
            for (i, c) in cells.iter().enumerate() {
                if !column.set_cell_text(block.start + i, c) && !is_missing(c) {
                    not_numbers += 1;
                }
            }
        }
    }
    column.trim_missing_tail();
    not_numbers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::Document;

    fn book_with(columns: Vec<ColumnData>) -> (Project, WindowId) {
        let mut p = Project::new();
        let b = p.add_workbook("");
        for d in columns {
            p.add_column(b, 0, Designation::Y, d).unwrap();
        }
        (p, b)
    }

    fn numbers(p: &Project, b: WindowId, col: usize) -> Vec<f64> {
        p.workbook(b).unwrap().sheets[0].columns[col].numeric().unwrap().to_vec()
    }

    #[test]
    fn copy_writes_displayed_text_or_every_digit_and_leaves_missing_cells_empty() {
        let (mut p, b) = book_with(vec![
            ColumnData::numeric(vec![0.1 + 0.2, f64::NAN, 2.0]),
            ColumnData::text(vec!["a b".into(), "x\ty".into()]),
        ]);
        p.workbook_mut(b).unwrap().sheets[0].columns[0].format = crate::format::NumberFormat::Decimal(2);
        let ws = &p.workbook(b).unwrap().sheets[0];
        let shown = ws.copy_text(&[0, 1], 0..3, CopyOptions::default());
        assert_eq!(shown, "0.30\ta b\n\tx y\n2.00\t\n");
        let full = ws.copy_text(&[0], 0..3, CopyOptions { full_precision: true, ..Default::default() });
        assert_eq!(full, "0.30000000000000004\n\n2\n", "a whole number has no useless .0");
    }

    #[test]
    fn copy_can_add_the_label_rows() {
        let (mut p, b) = book_with(vec![ColumnData::numeric(vec![1.0]), ColumnData::numeric(vec![2.0])]);
        {
            let ws = &mut p.workbook_mut(b).unwrap().sheets[0];
            ws.columns[0].long_name = "Potential".into();
            ws.columns[0].units = "V".into();
            ws.columns[1].params.insert("Scan rate".into(), "0.1".into());
        }
        let ws = &p.workbook(b).unwrap().sheets[0];
        let text = ws.copy_text(&[0, 1], 0..1, CopyOptions { label_rows: true, ..Default::default() });
        assert_eq!(text, "Potential\t\nV\t\n\t\n\t0.1\n1\t2\n");
    }

    #[test]
    fn clear_empties_cells_trims_the_tail_and_spares_formula_columns() {
        let (mut p, b) = book_with(vec![
            ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0]),
            ColumnData::text(vec!["a".into(), "b".into(), "c".into()]),
            ColumnData::numeric(vec![5.0, 6.0]),
        ]);
        {
            let c = &mut p.workbook_mut(b).unwrap().sheets[0].columns[2];
            c.formula = Some("1+1".into());
            c.recalc = crate::column::RecalcMode::Auto;
        }
        let ws = &mut p.workbook_mut(b).unwrap().sheets[0];
        let locked = ws.clear_cells(&[0, 1, 2], 1..3);
        assert_eq!(locked, 1);
        let a = ws.columns[0].numeric().unwrap();
        assert_eq!(a.len(), 4);
        assert!(a[1].is_nan() && a[2].is_nan());
        assert_eq!(a[3], 4.0);
        assert_eq!(ws.columns[1].text().unwrap(), &["a".to_string()], "the empty tail of a text column is dropped");
        assert_eq!(ws.columns[2].numeric().unwrap(), &[5.0, 6.0]);
        // Clearing the last values shortens the column.
        ws.clear_cells(&[0], 3..4);
        assert_eq!(ws.columns[0].len(), 1);
    }

    #[test]
    fn paste_splits_on_tabs_then_commas_and_reads_quotes() {
        assert_eq!(parse_table("1\t2\n3\t4\n"), vec![vec!["1", "2"], vec!["3", "4"]]);
        assert_eq!(parse_table("1,2\r\n3,4"), vec![vec!["1", "2"], vec!["3", "4"]]);
        assert_eq!(parse_table("a b\nc"), vec![vec!["a b"], vec!["c"]]);
        assert_eq!(parse_table("\"x, y\",2\n\"say \"\"hi\"\"\",3"), vec![vec!["x, y", "2"], vec!["say \"hi\"", "3"]]);
        assert_eq!(parse_table("1\t\n\t2"), vec![vec!["1", ""], vec!["", "2"]]);
        assert!(parse_table("").is_empty());
        assert!(parse_table("\n").is_empty());
    }

    #[test]
    fn paste_into_empty_columns_takes_numbers_as_numbers_and_text_as_text() {
        let (mut p, b) = book_with(vec![ColumnData::default(), ColumnData::default()]);
        let r = p.paste_text(b, 0, 0, 0, "1.5\talpha\n2.5\tbeta\n").unwrap();
        assert_eq!((r.rows, r.columns, r.columns_added), (2, 2, 0));
        assert_eq!(numbers(&p, b, 0), vec![1.5, 2.5]);
        let ws = &p.workbook(b).unwrap().sheets[0];
        // A new column is Text & Numeric (Origin's default), so the words stay as text cells.
        assert_eq!((ws.columns[1].cell_text(0), ws.columns[1].cell_text(1)), ("alpha".into(), "beta".into()));
        assert!(ws.columns[1].data.is_numeric());
    }

    #[test]
    fn text_pasted_into_a_text_and_numeric_column_is_kept_and_copied_back() {
        let (mut p, b) = book_with(vec![ColumnData::mixed_numbers(vec![1.0, 2.0, 3.0, 4.0])]);
        let r = p.paste_text(b, 0, 0, 0, "10\n<LOD\n30").unwrap();
        assert_eq!(r.not_numbers, 0);
        let a = numbers(&p, b, 0);
        assert_eq!(a[0], 10.0);
        assert!(a[1].is_nan(), "numeric views see the text as missing");
        assert_eq!(a[3], 4.0);
        let ws = &p.workbook(b).unwrap().sheets[0];
        assert_eq!(ws.columns[0].cell_text(1), "<LOD");
        assert_eq!(ws.copy_text(&[0], 0..4, CopyOptions::default()), "10\n<LOD\n30\n4\n");
        // clearing removes the text
        let mut ws = ws.clone();
        ws.clear_cells(&[0], 1..2);
        assert_eq!(ws.columns[0].cell_text(1), "--");
        assert!(ws.columns[0].is_blank_at(1));
    }

    #[test]
    fn paste_grows_the_sheet_in_rows_and_columns() {
        let (mut p, b) = book_with(vec![ColumnData::numeric(vec![1.0, 2.0])]);
        let r = p.paste_text(b, 0, 0, 1, "9\t8\t7\n6\t5\t4\n3\t2\t1").unwrap();
        assert_eq!(r.columns_added, 2);
        let ws = &p.workbook(b).unwrap().sheets[0];
        assert_eq!(ws.columns.len(), 3);
        assert_eq!(ws.columns[2].short_name, "C");
        assert_eq!(numbers(&p, b, 0), vec![1.0, 9.0, 6.0, 3.0]);
        let c = numbers(&p, b, 2);
        assert!(c[0].is_nan());
        assert_eq!(&c[1..], &[7.0, 4.0, 1.0]);
        assert_eq!(p.workbook(b).unwrap().sheets[0].row_count(), 4);
    }

    #[test]
    fn text_pasted_into_a_column_with_numbers_becomes_a_missing_value_and_is_counted() {
        let (mut p, b) = book_with(vec![ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])]);
        let r = p.paste_text(b, 0, 0, 0, "10\nabc\n30").unwrap();
        assert_eq!(r.not_numbers, 1);
        let a = numbers(&p, b, 0);
        assert_eq!(a[0], 10.0);
        assert!(a[1].is_nan());
        assert_eq!(a[2], 30.0);
    }

    #[test]
    fn a_number_column_filled_only_by_the_paste_becomes_text_when_the_paste_has_words() {
        let (mut p, b) = book_with(vec![ColumnData::numeric(vec![1.0, 2.0])]);
        let r = p.paste_text(b, 0, 0, 0, "x\ny\nz").unwrap();
        assert_eq!(r.not_numbers, 0);
        assert_eq!(p.workbook(b).unwrap().sheets[0].columns[0].text().unwrap().len(), 3);
    }

    #[test]
    fn a_formula_column_is_not_pasted_over() {
        let (mut p, b) = book_with(vec![ColumnData::numeric(vec![1.0]), ColumnData::numeric(vec![2.0])]);
        {
            let c = &mut p.workbook_mut(b).unwrap().sheets[0].columns[0];
            c.formula = Some("col(B)".into());
            c.recalc = crate::column::RecalcMode::Manual;
        }
        let r = p.paste_text(b, 0, 0, 0, "7\t8").unwrap();
        assert_eq!(r.locked_columns, 1);
        assert_eq!(numbers(&p, b, 0), vec![1.0]);
        assert_eq!(numbers(&p, b, 1), vec![8.0]);
    }

    #[test]
    fn copy_then_paste_gives_the_same_numbers_and_each_step_undoes_in_one_go() {
        let (p, b) = book_with(vec![
            ColumnData::numeric(vec![0.1 + 0.2, 1.0 / 3.0, f64::NAN, 4.0]),
            ColumnData::text(vec!["u".into(), "v".into(), String::new(), "w".into()]),
        ]);
        let mut doc = Document::new(p);
        let text = doc.project().workbook(b).unwrap().sheets[0].copy_text(
            &[0, 1],
            0..4,
            CopyOptions { full_precision: true, ..Default::default() },
        );
        doc.apply("Add sheet columns", |p| {
            p.add_column(b, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
            p.add_column(b, 0, Designation::Y, ColumnData::text(Vec::new())).unwrap();
        });
        doc.apply("Paste", |p| p.paste_text(b, 0, 2, 0, &text).unwrap());
        let ws = &doc.project().workbook(b).unwrap().sheets[0];
        assert_eq!(ws.columns[2].numeric().unwrap()[..2], ws.columns[0].numeric().unwrap()[..2]);
        assert_eq!(ws.columns[3].text().unwrap(), ws.columns[1].text().unwrap());
        assert_eq!(doc.undo().as_deref(), Some("Paste"));
        assert!(doc.project().workbook(b).unwrap().sheets[0].columns[2].is_empty());
        assert_eq!(doc.redo().as_deref(), Some("Paste"));
        doc.apply("Clear", |p| p.workbook_mut(b).unwrap().sheets[0].clear_cells(&[2, 3], 0..4));
        assert!(doc.project().workbook(b).unwrap().sheets[0].columns[2].is_empty());
        doc.undo();
        assert_eq!(doc.project().workbook(b).unwrap().sheets[0].columns[2].len(), 4);
    }
}
