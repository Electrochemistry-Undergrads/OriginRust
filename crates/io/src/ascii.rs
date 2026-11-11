//! Delimited text import (CSV, TSV, whitespace-aligned columns).
//!
//! Everything is detected automatically by default: the delimiter, where the
//! data starts, which header lines hold names, units and comments, and which
//! columns are numbers. The dialog can show the result of [`detect`] and let
//! the user override any part through [`AsciiOptions`].

use crate::error::IoError;
use sheaf_core::column::default_short_name;
use sheaf_core::{Column, ColumnData, ColumnId, Designation, WindowId, Workbook, Worksheet};
use std::collections::HashMap;

/// Lines looked at when guessing the delimiter.
const DELIMITER_SAMPLE_LINES: usize = 200;
/// Consecutive numeric lines that mark the start of the data.
const DATA_RUN: usize = 3;
/// Share of non-empty cells that must be numbers for a numeric column.
const NUMERIC_SHARE: f64 = 0.8;
/// Longest worksheet name.
const SHEET_NAME_MAX: usize = 31;

/// How the fields of a line are separated.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Delimiter {
    /// Guess from the file.
    Auto,
    Comma,
    Tab,
    Semicolon,
    /// Runs of spaces and tabs count as one separator.
    Whitespace,
    /// Any other single character.
    Other(char),
}

/// Import settings. `None` means "detect"; for the line numbers, `Some(0)` means "there is none".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AsciiOptions {
    pub delimiter: Delimiter,
    /// Number of lines before the first data row.
    pub header_lines: Option<usize>,
    /// 1-based line number holding the column long names.
    pub long_name_line: Option<usize>,
    /// 1-based line number holding the units.
    pub units_line: Option<usize>,
    /// 1-based line number holding the comments.
    pub comments_line: Option<usize>,
    /// Numbers use `,` as decimal separator; `,` then cannot be the delimiter.
    pub decimal_comma: bool,
    /// The first column is X and the others Y (otherwise all are Y).
    pub first_column_x: bool,
    /// Data rows to skip at the start.
    pub skip_rows: usize,
    /// Stop after this many data rows.
    pub max_rows: Option<usize>,
}

impl Default for AsciiOptions {
    fn default() -> Self {
        AsciiOptions {
            delimiter: Delimiter::Auto,
            header_lines: None,
            long_name_line: None,
            units_line: None,
            comments_line: None,
            decimal_comma: false,
            first_column_x: true,
            skip_rows: 0,
            max_rows: None,
        }
    }
}

/// What [`detect`] decided. Line numbers are 1-based; 0 means "none".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detected {
    /// Never `Auto`.
    pub delimiter: Delimiter,
    /// Lines before the first data row.
    pub header_lines: usize,
    pub long_name_line: usize,
    pub units_line: usize,
    pub comments_line: usize,
    /// Number of columns the sheet will have.
    pub columns: usize,
}

/// The result of [`parse_ascii`].
#[derive(Clone, Debug)]
pub struct AsciiImport {
    /// One workbook with placeholder identifiers (see `Project::adopt_workbook`).
    pub workbook: Workbook,
    pub detected: Detected,
    pub warnings: Vec<String>,
}

// ----- splitting and numbers -------------------------------------------------

/// Splits one line into fields. Quoted fields may contain the delimiter, and
/// a doubled quote inside quotes is one literal quote. Fields are trimmed
/// (quoted ones keep their inner spaces). With `Whitespace`, runs of
/// separators are one separator and the line has no empty fields; with any
/// other delimiter, consecutive delimiters give empty fields.
fn split_fields(line: &str, delim: Delimiter) -> Vec<String> {
    let whitespace = matches!(delim, Delimiter::Whitespace);
    let sep = match delim {
        Delimiter::Comma => ',',
        Delimiter::Semicolon => ';',
        Delimiter::Other(c) => c,
        Delimiter::Tab | Delimiter::Auto | Delimiter::Whitespace => '\t',
    };
    let is_sep = |c: char| if whitespace { c == ' ' || c == '\t' } else { c == sep };
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' && !quoted && cur.is_empty() && sep != '"' {
            in_quotes = true;
            quoted = true;
        } else if is_sep(c) {
            if !whitespace || quoted || !cur.is_empty() {
                fields.push(finish_field(&cur, quoted));
            }
            cur.clear();
            quoted = false;
        } else if c.is_whitespace() && (cur.is_empty() || quoted) {
            // leading space, or space after a closing quote: ignored
        } else {
            cur.push(c);
        }
    }
    if !whitespace || quoted || !cur.is_empty() {
        fields.push(finish_field(&cur, quoted));
    }
    fields
}

fn finish_field(cur: &str, quoted: bool) -> String {
    if quoted { cur.to_string() } else { cur.trim_end().to_string() }
}

/// Reads a number as the importer understands it: `1e-5`, `+3.2`, `.5`,
/// `-.5e3`, `NaN`, `inf`, and with `decimal_comma` also `1,5`.
fn parse_number(field: &str, decimal_comma: bool) -> Option<f64> {
    let t = field.trim();
    if t.is_empty() {
        return None;
    }
    if decimal_comma && t.contains(',') {
        return t.replace(',', ".").parse().ok();
    }
    t.parse().ok()
}

/// Every non-empty field is a number, and there is at least one.
fn is_numeric_row(fields: &[String], decimal_comma: bool) -> bool {
    let mut any = false;
    for f in fields {
        if f.trim().is_empty() {
            continue;
        }
        if parse_number(f, decimal_comma).is_none() {
            return false;
        }
        any = true;
    }
    any
}

// ----- analysis ----------------------------------------------------------------

struct Analysis<'a> {
    lines: Vec<&'a str>,
    detected: Detected,
    /// The data rows after `skip_rows` / `max_rows`, as raw fields.
    rows: Vec<Vec<String>>,
}

fn auto_delimiter(lines: &[&str], decimal_comma: bool) -> Delimiter {
    let sample: Vec<&str> =
        lines.iter().copied().filter(|l| !l.trim().is_empty()).take(DELIMITER_SAMPLE_LINES).collect();
    let mut best: Option<(usize, Delimiter)> = None;
    for cand in [Delimiter::Tab, Delimiter::Comma, Delimiter::Semicolon, Delimiter::Whitespace] {
        if decimal_comma && cand == Delimiter::Comma {
            continue;
        }
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for l in &sample {
            *counts.entry(split_fields(l, cand).len()).or_default() += 1;
        }
        let top = counts.iter().filter(|(n, _)| **n > 1).map(|(n, freq)| (*freq, *n)).max();
        if let Some((freq, _)) = top
            && best.is_none_or(|(b, _)| freq > b)
        {
            best = Some((freq, cand));
        }
    }
    // A one-column file has no delimiter; tab keeps every line as one field.
    best.map_or(Delimiter::Tab, |(_, d)| d)
}

/// Index (into `rows`) of the first row of the data block.
fn find_data_start(rows: &[(usize, Vec<String>)], decimal_comma: bool) -> Option<usize> {
    let run_ok = |i: usize, need: usize| {
        let n = rows[i].1.len();
        rows[i..i + need].iter().all(|(_, f)| f.len() == n && is_numeric_row(f, decimal_comma))
    };
    let full_runs = (rows.len() + 1).saturating_sub(DATA_RUN);
    // 1. A full run of lines with one field count, all numeric.
    if let Some(i) = (0..full_runs).find(|&i| run_ok(i, DATA_RUN)) {
        return Some(i);
    }
    // 2. Ragged numeric rows: every line is numeric but the field counts differ.
    if let Some(i) =
        (0..full_runs).find(|&i| rows[i..i + DATA_RUN].iter().all(|(_, f)| is_numeric_row(f, decimal_comma)))
    {
        return Some(i);
    }
    // 3. Fewer than three lines left: all of them must fit.
    if let Some(i) = (full_runs..rows.len()).find(|&i| run_ok(i, rows.len() - i)) {
        return Some(i);
    }
    // Files with a text column: the numeric columns of the first data row stay numeric
    // (or empty) in the next rows, and the other fields of that row are text.
    for i in 0..rows.len() {
        let first = &rows[i].1;
        let numeric: Vec<bool> = first.iter().map(|f| parse_number(f, decimal_comma).is_some()).collect();
        let others_are_text = first.iter().zip(&numeric).all(|(f, n)| *n || !f.trim().is_empty());
        if !numeric.contains(&true) || !others_are_text {
            continue;
        }
        let need = DATA_RUN.min(rows.len() - i);
        let consistent = rows[i..i + need].iter().all(|(_, f)| {
            f.len() == first.len()
                && f.iter()
                    .zip(&numeric)
                    .all(|(v, n)| !*n || v.trim().is_empty() || parse_number(v, decimal_comma).is_some())
        });
        if consistent {
            return Some(i);
        }
    }
    // Last resort: the first row that is all numbers.
    rows.iter().position(|(_, f)| is_numeric_row(f, decimal_comma))
}

/// Detects the long-name, units and comments lines (1-based, 0 = none).
fn detect_labels(rows: &[(usize, Vec<String>)], header_lines: usize, ncols: usize) -> (usize, usize, usize) {
    let header: Vec<&(usize, Vec<String>)> = rows.iter().take_while(|(i, _)| *i < header_lines).collect();
    let Some(last) = header.iter().rposition(|(_, f)| f.len() == ncols) else {
        return (0, 0, 0);
    };
    let mut block = vec![header[last].0];
    let mut k = last;
    while block.len() < 3 && k > 0 {
        let (prev_line, prev_fields) = header[k - 1];
        if prev_fields.len() == ncols && prev_line + 1 == header[k].0 {
            block.push(*prev_line);
            k -= 1;
        } else {
            break;
        }
    }
    block.reverse();
    let line = |i: usize| block.get(i).map_or(0, |l| l + 1);
    (line(0), line(1), line(2))
}

fn analyze<'a>(text: &'a str, opts: &AsciiOptions) -> Analysis<'a> {
    let lines: Vec<&str> = text.lines().collect();
    let delimiter = match opts.delimiter {
        Delimiter::Auto => auto_delimiter(&lines, opts.decimal_comma),
        d => d,
    };
    let all: Vec<(usize, Vec<String>)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i, split_fields(l, delimiter)))
        .collect();

    let (header_lines, start) = match opts.header_lines {
        Some(h) => (h, all.iter().position(|(i, _)| *i >= h)),
        None => match find_data_start(&all, opts.decimal_comma) {
            Some(s) => (all[s].0, Some(s)),
            None => (0, if all.is_empty() { None } else { Some(0) }),
        },
    };
    let (ncols_start, data) = match start {
        Some(s) => (all[s].1.len(), &all[s..]),
        None => (0, &all[all.len()..]),
    };

    let (dl, du, dc) = detect_labels(&all, header_lines, ncols_start);
    let long_name_line = opts.long_name_line.unwrap_or(dl);
    let units_line = opts.units_line.unwrap_or(du);
    let comments_line = opts.comments_line.unwrap_or(dc);

    let mut rows: Vec<Vec<String>> = data
        .iter()
        .map(|(_, f)| f.clone())
        .filter(|f| f.iter().any(|c| !c.trim().is_empty()))
        .skip(opts.skip_rows)
        .take(opts.max_rows.unwrap_or(usize::MAX))
        .collect();
    // Trailing empty columns (a delimiter at the end of every line) are not columns.
    let columns =
        rows.iter().map(|r| r.iter().rposition(|c| !c.trim().is_empty()).map_or(0, |p| p + 1)).max().unwrap_or(0);
    for r in &mut rows {
        r.truncate(columns.max(1));
    }
    Analysis {
        lines,
        detected: Detected { delimiter, header_lines, long_name_line, units_line, comments_line, columns },
        rows,
    }
}

/// Works out every automatic setting for `text` and returns what will be used.
pub fn detect(text: &str, opts: &AsciiOptions) -> Detected {
    analyze(text, opts).detected
}

/// The raw fields of the first `max_rows` data rows, for the import dialog.
pub fn preview(text: &str, opts: &AsciiOptions, max_rows: usize) -> Vec<Vec<String>> {
    let mut a = analyze(text, opts);
    a.rows.truncate(max_rows);
    a.rows
}

// ----- parsing -------------------------------------------------------------------

/// Reads delimited text into a workbook with one sheet.
///
/// Rows that consist only of empty fields are skipped. Quoted fields cannot
/// span lines.
pub fn parse_ascii(text: &str, file_stem: &str, opts: &AsciiOptions) -> Result<AsciiImport, IoError> {
    if opts.decimal_comma && matches!(opts.delimiter, Delimiter::Comma | Delimiter::Other(',')) {
        return Err(IoError::InvalidOption(
            "the decimal separator and the field delimiter cannot both be a comma".to_string(),
        ));
    }
    let a = analyze(text, opts);
    if a.rows.is_empty() {
        let detail = if text.trim().is_empty() { "the file is empty" } else { "no data rows were found" };
        return Err(IoError::NoData { source_name: file_stem.to_string(), detail: detail.to_string() });
    }
    let d = a.detected.clone();
    let ncols = d.columns;
    let mut warnings = Vec::new();

    // Labels.
    let label_row = |n: usize| -> Vec<String> {
        let mut f =
            n.checked_sub(1).and_then(|i| a.lines.get(i)).map_or_else(Vec::new, |l| split_fields(l, d.delimiter));
        f.resize(ncols, String::new());
        f
    };
    let mut long_names = label_row(d.long_name_line);
    let mut units = label_row(d.units_line);
    let comments = label_row(d.comments_line);
    if d.long_name_line > 0 && d.units_line == 0 {
        for (name, unit) in long_names.iter_mut().zip(units.iter_mut()) {
            if let Some((n, u)) = split_name_unit(name) {
                *name = n;
                *unit = u;
            }
        }
    }
    let used: Vec<usize> = [d.long_name_line, d.units_line, d.comments_line].into_iter().filter(|n| *n > 0).collect();
    let dropped = a
        .lines
        .iter()
        .enumerate()
        .filter(|(i, l)| *i < d.header_lines && !l.trim().is_empty() && !used.contains(&(i + 1)))
        .count();
    if dropped > 0 {
        let plural = if dropped == 1 { "line was" } else { "lines were" };
        warnings.push(format!("{dropped} header {plural} not imported"));
    }

    // Columns.
    let mut sheet = Worksheet::new(sheet_name(file_stem));
    for j in 0..ncols {
        let cells: Vec<&str> = a.rows.iter().map(|r| r.get(j).map_or("", String::as_str)).collect();
        let non_empty = cells.iter().filter(|c| !c.trim().is_empty()).count();
        let numbers: Vec<Option<f64>> = cells.iter().map(|c| parse_number(c, opts.decimal_comma)).collect();
        let parsed = numbers.iter().filter(|n| n.is_some()).count();
        let short = default_short_name(j);
        let numeric = non_empty == 0 || parsed as f64 >= NUMERIC_SHARE * non_empty as f64;
        let (designation, data) = if numeric {
            if parsed < non_empty {
                let bad = non_empty - parsed;
                let plural = if bad == 1 { "cell is not a number" } else { "cells are not numbers" };
                warnings.push(format!(
                    "column {short}: {bad} {plural} and {} read as missing",
                    if bad == 1 { "was" } else { "were" }
                ));
            }
            let designation = if j == 0 && opts.first_column_x { Designation::X } else { Designation::Y };
            (designation, ColumnData::numeric(numbers.into_iter().map(|n| n.unwrap_or(f64::NAN)).collect()))
        } else {
            let text = cells.iter().map(|c| c.to_string()).collect();
            (Designation::Label, ColumnData::text(text))
        };
        let mut col = Column::new(ColumnId(j as u64 + 1), short, designation, data);
        col.long_name = long_names[j].clone();
        col.units = units[j].clone();
        col.comments = comments[j].clone();
        sheet.columns.push(col);
    }

    let mut workbook = Workbook::new(WindowId(0), "Book1");
    workbook.long_name = file_stem.to_string();
    workbook.sheets = vec![sheet];
    Ok(AsciiImport { workbook, detected: d, warnings })
}

fn sheet_name(stem: &str) -> String {
    let name: String = stem.chars().take(SHEET_NAME_MAX).collect();
    if name.trim().is_empty() { "Sheet1".to_string() } else { name }
}

/// Splits `Name (unit)`, `Name [unit]` and `Name/unit` into name and unit.
fn split_name_unit(label: &str) -> Option<(String, String)> {
    let t = label.trim();
    for (open, close) in [('(', ')'), ('[', ']')] {
        if let Some(inner_end) = t.strip_suffix(close)
            && let Some(p) = inner_end.rfind(open)
        {
            let (name, unit) = (inner_end[..p].trim(), inner_end[p + 1..].trim());
            if !name.is_empty() && !unit.is_empty() {
                return Some((name.to_string(), unit.to_string()));
            }
        }
    }
    if t.matches('/').count() == 1 {
        let (name, unit) = t.split_once('/')?;
        let (name, unit) = (name.trim(), unit.trim());
        if !name.is_empty() && !unit.is_empty() && !unit.contains(char::is_whitespace) {
            return Some((name.to_string(), unit.to_string()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> AsciiImport {
        parse_ascii(text, "file", &AsciiOptions::default()).unwrap()
    }

    fn col(imp: &AsciiImport, j: usize) -> &Column {
        &imp.workbook.sheets[0].columns[j]
    }

    #[test]
    fn detects_each_delimiter() {
        let cases = [
            ("1,2,3\n4,5,6\n7,8,9\n", Delimiter::Comma),
            ("1\t2\t3\n4\t5\t6\n7\t8\t9\n", Delimiter::Tab),
            ("1;2;3\n4;5;6\n7;8;9\n", Delimiter::Semicolon),
            ("1  2   3\n4 5 6\n  7 8 9\n", Delimiter::Whitespace),
            ("1.5\n2.5\n3.5\n", Delimiter::Tab),
        ];
        for (text, want) in cases {
            let d = detect(text, &AsciiOptions::default());
            assert_eq!(d.delimiter, want, "{text:?}");
            assert_eq!(d.header_lines, 0);
        }
        // comma and whitespace tie ("1, 2"): comma wins
        assert_eq!(detect("1, 2\n3, 4\n5, 6\n", &AsciiOptions::default()).delimiter, Delimiter::Comma);
    }

    #[test]
    fn header_names_and_units() {
        let text = "Comment line one\nPotential,Current,Time\nV,A,s\n1,2,3\n4,5,6\n7,8,9\n";
        let imp = parse(text);
        let d = &imp.detected;
        assert_eq!((d.header_lines, d.long_name_line, d.units_line, d.comments_line), (3, 2, 3, 0));
        assert_eq!(col(&imp, 0).long_name, "Potential");
        assert_eq!(col(&imp, 1).units, "A");
        assert_eq!(col(&imp, 0).designation, Designation::X);
        assert_eq!(col(&imp, 2).designation, Designation::Y);
        assert_eq!(col(&imp, 2).numeric().unwrap(), &[3.0, 6.0, 9.0]);
        assert_eq!(imp.warnings, vec!["1 header line was not imported".to_string()]);
        assert_eq!(imp.workbook.sheets[0].name, "file");
        assert_eq!(col(&imp, 1).short_name, "B");
    }

    #[test]
    fn three_label_lines_are_names_units_comments() {
        let imp = parse("x,y\nV,A\nfirst,second\n1,2\n3,4\n5,6\n");
        let d = &imp.detected;
        assert_eq!((d.long_name_line, d.units_line, d.comments_line), (1, 2, 3));
        assert_eq!(col(&imp, 1).comments, "second");
        assert!(imp.warnings.is_empty());
    }

    #[test]
    fn name_with_unit_is_split_only_without_units_line() {
        let imp = parse("Potential (V),Current [mA],Time/s,Plain text name\n1,2,3,4\n5,6,7,8\n9,1,2,3\n");
        let units: Vec<&str> = imp.workbook.sheets[0].columns.iter().map(|c| c.units.as_str()).collect();
        let names: Vec<&str> = imp.workbook.sheets[0].columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(units, ["V", "mA", "s", ""]);
        assert_eq!(names, ["Potential", "Current", "Time", "Plain text name"]);
        let with_units = parse("Potential (V),Current\nV,A\n1,2\n3,4\n5,6\n");
        assert_eq!(col(&with_units, 0).long_name, "Potential (V)");
    }

    #[test]
    fn decimal_comma_with_semicolons() {
        let opts = AsciiOptions { decimal_comma: true, ..AsciiOptions::default() };
        let imp = parse_ascii("a;b\n1,5;2,25\n3,5;-4,5e1\n5;6\n", "eu", &opts).unwrap();
        assert_eq!(imp.detected.delimiter, Delimiter::Semicolon);
        assert_eq!(col(&imp, 0).numeric().unwrap(), &[1.5, 3.5, 5.0]);
        assert_eq!(col(&imp, 1).numeric().unwrap(), &[2.25, -45.0, 6.0]);
        let bad = AsciiOptions { decimal_comma: true, delimiter: Delimiter::Comma, ..AsciiOptions::default() };
        assert!(matches!(parse_ascii("1,5\n", "eu", &bad), Err(IoError::InvalidOption(_))));
    }

    #[test]
    fn number_syntax() {
        for (s, v) in
            [("1e-5", 1e-5), ("1E5", 1e5), ("+3.2", 3.2), (".5", 0.5), ("-.5e3", -500.0), ("inf", f64::INFINITY)]
        {
            assert_eq!(parse_number(s, false), Some(v), "{s}");
        }
        assert!(parse_number("NaN", false).unwrap().is_nan());
        assert_eq!(parse_number("1,5", false), None);
        assert_eq!(parse_number("1,5", true), Some(1.5));
        assert_eq!(parse_number("abc", false), None);
        assert_eq!(parse_number("", false), None);
    }

    #[test]
    fn quoted_fields() {
        assert_eq!(split_fields(r#"a,"b,c","say ""hi""", d "#, Delimiter::Comma), ["a", "b,c", "say \"hi\"", "d"]);
        assert_eq!(split_fields(r#""two words" x  "y z""#, Delimiter::Whitespace), ["two words", "x", "y z"]);
        let imp = parse("name,value\n\"Smith, J\",1\n\"Doe, A\",2\n\"Roe, B\",3\n");
        assert_eq!(col(&imp, 0).text().unwrap(), ["Smith, J", "Doe, A", "Roe, B"]);
        assert_eq!(imp.detected.delimiter, Delimiter::Comma);
    }

    #[test]
    fn ragged_rows_and_empty_fields() {
        let imp = parse("1,2,3\n4,,6\n7,8\n9,10,11,12\n");
        let s = &imp.workbook.sheets[0];
        assert_eq!(s.columns.len(), 4);
        let c = |j: usize| s.columns[j].numeric().unwrap().to_vec();
        assert_eq!(c(0), [1.0, 4.0, 7.0, 9.0]);
        assert!(c(1)[1].is_nan() && c(2)[2].is_nan());
        assert_eq!(c(3).iter().filter(|v| v.is_nan()).count(), 3);
        assert_eq!(c(3)[3], 12.0);
    }

    #[test]
    fn text_column_and_missing_markers() {
        let imp = parse("id,label,val\n1,alpha,1.5\n2,beta,2.5\n3,gamma,3.5\n4,delta,4.5\n5,eps,5.5\n6,zeta,oops\n");
        assert_eq!(col(&imp, 1).designation, Designation::Label);
        assert_eq!(col(&imp, 1).text().unwrap()[2], "gamma");
        // one non-number in six is below the 20% limit: numeric with a missing value
        assert!(col(&imp, 2).numeric().unwrap()[5].is_nan());
        assert!(imp.warnings.iter().any(|w| w.contains("column C") && w.contains("1 cell")));
    }

    #[test]
    fn one_column_file() {
        let imp = parse("1.5\n2.5\n3.5\n");
        assert_eq!(imp.workbook.sheets[0].columns.len(), 1);
        assert_eq!(col(&imp, 0).numeric().unwrap(), &[1.5, 2.5, 3.5]);
        let titled = parse("Voltage\n1\n2\n3\n");
        assert_eq!(col(&titled, 0).long_name, "Voltage");
    }

    #[test]
    fn empty_and_headerless_garbage() {
        for t in ["", "\n \n\n"] {
            let e = parse_ascii(t, "empty", &AsciiOptions::default()).unwrap_err();
            assert!(matches!(e, IoError::NoData { .. }) && e.to_string().contains("empty"));
        }
        let imp = parse("just\nwords\nhere\n");
        assert_eq!(col(&imp, 0).designation, Designation::Label, "no numbers: a text column");
    }

    #[test]
    fn overrides_skip_and_limits() {
        let text = "a,b\n1,2\n3,4\n5,6\n7,8\n";
        let opts = AsciiOptions { skip_rows: 1, max_rows: Some(2), long_name_line: Some(0), ..AsciiOptions::default() };
        let imp = parse_ascii(text, "t", &opts).unwrap();
        assert_eq!(col(&imp, 0).numeric().unwrap(), &[3.0, 5.0]);
        assert_eq!(col(&imp, 0).long_name, "");
        let all_y = AsciiOptions { first_column_x: false, ..AsciiOptions::default() };
        assert_eq!(parse_ascii(text, "t", &all_y).unwrap().workbook.sheets[0].columns[0].designation, Designation::Y);
        let forced = AsciiOptions { header_lines: Some(2), ..AsciiOptions::default() };
        assert_eq!(parse_ascii(text, "t", &forced).unwrap().workbook.sheets[0].row_count(), 3);
    }

    #[test]
    fn preview_and_trailing_delimiters() {
        let rows = preview("h1,h2,\n1,2,\n3,4,\n5,6,\n7,8,\n", &AsciiOptions::default(), 2);
        assert_eq!(rows, vec![vec!["1", "2"], vec!["3", "4"]]);
    }

    #[test]
    fn long_sheet_names_are_truncated() {
        let imp = parse_ascii("1\n", &"x".repeat(50), &AsciiOptions::default()).unwrap();
        assert_eq!(imp.workbook.sheets[0].name.chars().count(), 31);
        assert_eq!(parse_ascii("1\n", "", &AsciiOptions::default()).unwrap().workbook.sheets[0].name, "Sheet1");
    }
}
