//! Delimited text import (CSV, TSV, whitespace-aligned columns).
//!
//! Everything is detected automatically by default: the delimiter, where the
//! data starts, which header lines hold names, units and comments, and which
//! columns are numbers. The dialog can show the result of [`detect`] and let
//! the user override any part through [`AsciiOptions`].

use crate::error::IoError;
use sheaf_core::column::default_short_name;
use sheaf_core::datetime::{self, DateOrder};
use sheaf_core::format::NumberFormat;
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

/// What to do with columns of dates and times of day.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum DateImport {
    /// Import them as numbers (Origin's date numbers) with a date or time display format.
    #[default]
    Detect,
    /// Keep the text exactly as written.
    AsText,
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
    /// Whether date and time columns become numbers or stay text.
    pub dates: DateImport,
    /// Which order `11/12/2026` is read in when the file does not settle it
    /// (a day above 12 anywhere in the column does settle it).
    pub date_order: DateOrder,
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
            dates: DateImport::Detect,
            date_order: DateOrder::Mdy,
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

/// True for the strings that instruments and spreadsheets write for "no value":
/// `NA`, `N/A`, `#N/A`, `n.d.`, `nd`, `--`, `----`, error codes such as `#DIV/0!`,
/// and "below detection limit" entries such as `<LOD` or `<0.05`. In a numeric
/// column such a cell is a missing value; in a text column it is ordinary text.
fn is_missing_marker(field: &str) -> bool {
    let t = field.trim();
    if t.is_empty() {
        return false;
    }
    if t.chars().all(|c| matches!(c, '-' | '\u{2013}' | '\u{2014}')) {
        return true;
    }
    if t.len() > 1 && t.starts_with('<') {
        return true;
    }
    matches!(
        t.to_ascii_lowercase().as_str(),
        "na" | "n/a"
            | "#n/a"
            | "n.a."
            | "n.a"
            | "n.d."
            | "n.d"
            | "nd"
            | "bdl"
            | "#value!"
            | "#div/0!"
            | "#ref!"
            | "#num!"
    )
}

/// How one cell looks to the numeric-column logic.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Cell {
    Empty,
    Number,
    /// A date or a time of day.
    Date,
    Marker,
    Text,
}

fn classify(field: &str, decimal_comma: bool) -> Cell {
    if field.trim().is_empty() {
        Cell::Empty
    } else if parse_number(field, decimal_comma).is_some() {
        Cell::Number
    } else if looks_like_date(field) {
        Cell::Date
    } else if is_missing_marker(field) {
        Cell::Marker
    } else {
        Cell::Text
    }
}

fn looks_like_date(field: &str) -> bool {
    field.trim_start().starts_with(|c: char| c.is_ascii_digit())
        && (datetime::parse(field, DateOrder::Mdy).is_some() || datetime::parse(field, DateOrder::Dmy).is_some())
}

/// No cell is plain text (missing-value markers are allowed) and there is at
/// least one real number.
fn is_numeric_row(fields: &[String], decimal_comma: bool) -> bool {
    let mut any = false;
    for f in fields {
        match classify(f, decimal_comma) {
            Cell::Text => return false,
            Cell::Number | Cell::Date => any = true,
            Cell::Empty | Cell::Marker => {}
        }
    }
    any
}

/// Splits the text into records. A record normally is a line, but a quoted
/// field may contain line breaks, so the open-quote state is carried across
/// lines. A quote opens a quoted field only at the start of a field. If a
/// quote is still open at the end of the text (a stray quote character), the
/// text is split into plain lines instead.
fn split_records(text: &str) -> Vec<&str> {
    let mut records = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let mut field_start = true;
    let mut chars = text.char_indices().peekable();
    fn cut(s: &str) -> &str {
        s.strip_suffix('\r').unwrap_or(s)
    }
    while let Some((i, c)) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek().is_some_and(|(_, n)| *n == '"') {
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            continue;
        }
        match c {
            '\n' => {
                records.push(cut(&text[start..i]));
                start = i + 1;
                field_start = true;
            }
            '"' if field_start => {
                in_quotes = true;
                field_start = false;
            }
            ',' | ';' | '\t' | ' ' => field_start = true,
            _ => field_start = false,
        }
    }
    if in_quotes {
        return text.lines().collect();
    }
    if start < text.len() {
        records.push(cut(&text[start..]));
    }
    records
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
        return Some(extend_back(rows, i, decimal_comma));
    }
    // 2. Ragged numeric rows: every line is numeric but the field counts differ.
    if let Some(i) =
        (0..full_runs).find(|&i| rows[i..i + DATA_RUN].iter().all(|(_, f)| is_numeric_row(f, decimal_comma)))
    {
        return Some(extend_back(rows, i, decimal_comma));
    }
    // 3. Fewer than three lines left: all of them must fit.
    if let Some(i) = (full_runs..rows.len()).find(|&i| run_ok(i, rows.len() - i)) {
        return Some(extend_back(rows, i, decimal_comma));
    }
    // Files with a text column or with odd text cells inside numeric columns. A row
    // starts the data when it holds at least one number or missing-value marker, its
    // other fields are text, and the next rows have the same field count with at
    // most half of the cells in its numeric columns being text.
    for i in 0..rows.len() {
        let first = &rows[i].1;
        let numeric: Vec<bool> = first
            .iter()
            .map(|f| matches!(classify(f, decimal_comma), Cell::Number | Cell::Date | Cell::Marker))
            .collect();
        let others_are_text = first.iter().zip(&numeric).all(|(f, n)| *n || !f.trim().is_empty());
        if !numeric.contains(&true) || !others_are_text {
            continue;
        }
        let need = DATA_RUN.min(rows.len() - i);
        let block = &rows[i..i + need];
        if !block.iter().all(|(_, f)| f.len() == first.len()) {
            continue;
        }
        let (mut odd, mut filled) = (0usize, 0usize);
        for (_, f) in block {
            for (v, n) in f.iter().zip(&numeric) {
                if !*n {
                    continue;
                }
                match classify(v, decimal_comma) {
                    Cell::Empty => {}
                    Cell::Text => {
                        odd += 1;
                        filled += 1;
                    }
                    Cell::Number | Cell::Date | Cell::Marker => filled += 1,
                }
            }
        }
        if odd * 2 <= filled {
            return Some(i);
        }
    }
    // Last resort: the first row that is all numbers.
    rows.iter().position(|(_, f)| is_numeric_row(f, decimal_comma))
}

/// Moves the data start up over rows that belong to the data block although a
/// cell in them is text (for example `3,trace` above numeric rows). Such a row
/// has the field count of the block and at least as many numbers (or
/// missing-value markers) as text cells; a name row has no numbers at all, so
/// it stays a header line.
fn extend_back(rows: &[(usize, Vec<String>)], start: usize, decimal_comma: bool) -> usize {
    let width = rows[start].1.len();
    let mut s = start;
    while s > 0 {
        let f = &rows[s - 1].1;
        if f.len() != width {
            break;
        }
        let (mut good, mut marks, mut odd) = (0, 0, 0);
        for v in f {
            match classify(v, decimal_comma) {
                Cell::Number | Cell::Date => good += 1,
                Cell::Marker => marks += 1,
                Cell::Text => odd += 1,
                Cell::Empty => {}
            }
        }
        // A row made only of markers (a units row such as `-,-`) is a header
        // line, not data.
        if (good == 0 && odd == 0) || good + marks < odd {
            break;
        }
        s -= 1;
    }
    s
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
    let lines: Vec<&str> = split_records(text);
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
/// Rows that consist only of empty fields are skipped. Quoted fields may span
/// lines; line numbers in the options then count records, not physical lines.
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
        if opts.dates == DateImport::Detect
            && let Some(dc) = detect_date_column(&cells, opts.date_order)
        {
            if dc.ambiguous {
                warnings.push(format!(
                    "column {}: the dates can be read day first or month first; read {}",
                    default_short_name(j),
                    if dc.order == DateOrder::Mdy { "month first (M/d/yyyy)" } else { "day first (dd/MM/yyyy)" }
                ));
            }
            let designation = if j == 0 && opts.first_column_x { Designation::X } else { Designation::Y };
            let mut col =
                Column::new(ColumnId(j as u64 + 1), default_short_name(j), designation, ColumnData::numeric(dc.values));
            col.format = dc.format;
            col.long_name = long_names[j].clone();
            col.units = units[j].clone();
            col.comments = comments[j].clone();
            sheet.columns.push(col);
            continue;
        }
        let numbers: Vec<Option<f64>> = cells.iter().map(|c| parse_number(c, opts.decimal_comma)).collect();
        let parsed = numbers.iter().filter(|n| n.is_some()).count();
        // Missing-value markers (`n.d.`, `<LOD`, `NA`, ...) are neither numbers nor evidence of text.
        let markers = cells.iter().zip(&numbers).filter(|(c, n)| n.is_none() && is_missing_marker(c)).count();
        let non_empty = cells.iter().filter(|c| !c.trim().is_empty()).count() - markers;
        let short = default_short_name(j);
        // A column of markers only (names such as `NA` or `nd`) keeps its text.
        let numeric = if non_empty == 0 { markers == 0 } else { parsed as f64 >= NUMERIC_SHARE * non_empty as f64 };
        let (designation, data) = if numeric {
            let bad = non_empty - parsed;
            if bad > 0 {
                let plural = if bad == 1 { "cell is not a number" } else { "cells are not numbers" };
                warnings.push(format!(
                    "column {short}: {bad} {plural} and {} read as missing",
                    if bad == 1 { "was" } else { "were" }
                ));
            }
            if markers > 0 {
                let plural = if markers == 1 { "cell holds" } else { "cells hold" };
                warnings.push(format!(
                    "column {short}: {markers} {plural} a no-value marker (n.d., NA, <LOD, ...) and read as missing"
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

/// A column whose cells are all dates, or all times of day, or all both.
struct DateColumn {
    /// NaN for an empty cell or a no-value marker.
    values: Vec<f64>,
    format: NumberFormat,
    /// Some cell reads differently with the other day and month order, and both orders fit every cell.
    ambiguous: bool,
    order: DateOrder,
}

/// Reads a whole column as dates when every filled cell is a date or time of the same kind.
/// The order that fits every cell is used; when both fit, `prefer` decides and the column is
/// marked ambiguous if that changes any value.
fn detect_date_column(cells: &[&str], prefer: DateOrder) -> Option<DateColumn> {
    let filled: Vec<&str> = cells.iter().copied().filter(|c| !c.trim().is_empty() && !is_missing_marker(c)).collect();
    if filled.is_empty() {
        return None;
    }
    let read_all = |order: DateOrder| -> Option<Vec<datetime::Parsed>> {
        filled.iter().map(|c| datetime::parse(c, order)).collect()
    };
    let other = if prefer == DateOrder::Mdy { DateOrder::Dmy } else { DateOrder::Mdy };
    let (order, parsed, both) = match (read_all(prefer), read_all(other)) {
        (Some(p), Some(_)) => (prefer, p, true),
        (Some(p), None) => (prefer, p, false),
        (None, Some(p)) => (other, p, false),
        (None, None) => return None,
    };
    let kind = |f: &NumberFormat| std::mem::discriminant(f);
    if !parsed.iter().all(|p| kind(&p.format) == kind(&parsed[0].format)) {
        return None;
    }
    // One display format for the column: the first date pattern and the most precise time pattern.
    let format = match parsed[0].format {
        NumberFormat::Date(d) => NumberFormat::Date(d),
        NumberFormat::Time(_) => NumberFormat::Time(
            parsed.iter().filter_map(|p| if let NumberFormat::Time(t) = p.format { Some(t) } else { None }).max()?,
        ),
        NumberFormat::DateTime(d, _) => NumberFormat::DateTime(
            d,
            parsed
                .iter()
                .filter_map(|p| if let NumberFormat::DateTime(_, t) = p.format { Some(t) } else { None })
                .max()?,
        ),
        _ => return None,
    };
    let mut it = parsed.iter();
    let values = cells
        .iter()
        .map(|c| {
            if c.trim().is_empty() || is_missing_marker(c) { f64::NAN } else { it.next().map_or(f64::NAN, |p| p.value) }
        })
        .collect();
    let ambiguous = both && parsed.iter().any(|p| p.ambiguous);
    Some(DateColumn { values, format, ambiguous, order })
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
    fn text_cell_in_first_rows_does_not_turn_data_into_header() {
        let imp = parse("x,y\n1,2\n3,n.d.\n5,6\n7,8\n9,10\n");
        assert_eq!(imp.detected.header_lines, 1);
        assert_eq!(col(&imp, 0).numeric().unwrap(), &[1.0, 3.0, 5.0, 7.0, 9.0]);
        let y = col(&imp, 1).numeric().unwrap();
        assert_eq!((y.len(), y[0], y[2]), (5, 2.0, 6.0));
        assert!(y[1].is_nan());
        assert_eq!(col(&imp, 0).long_name, "x");
        // a text cell that is not a known marker, in the first and second data rows
        let imp = parse("x,y\n1,trace\n3,4\n5,6\n7,8\n9,10\n");
        assert_eq!(col(&imp, 0).numeric().unwrap().len(), 5);
        assert!(col(&imp, 1).numeric().unwrap()[0].is_nan());
        let imp = parse("x,y\n1,2\n3,trace\n5,trace\n7,8\n9,10\n");
        assert_eq!(col(&imp, 0).numeric().unwrap().len(), 5);
        assert_eq!(imp.detected.header_lines, 1);
    }

    #[test]
    fn missing_markers_keep_a_column_numeric() {
        let imp = parse("a,b\n1,NA\n2,N/A\n3,--\n4,n.d.\n5,#N/A\n6,nan\n7,----\n8,9\n");
        let b = col(&imp, 1).numeric().expect("numeric column");
        assert_eq!(b.iter().filter(|v| v.is_nan()).count(), 7);
        assert_eq!(b[7], 9.0);
        assert_eq!(col(&imp, 0).numeric().unwrap().len(), 8);
        // a text column is not changed by markers
        let imp = parse("id,v\nNA,1\nbob,2\nann,3\n");
        assert_eq!(col(&imp, 0).text().unwrap(), ["NA", "bob", "ann"]);
    }

    #[test]
    fn quoted_fields_may_contain_line_breaks() {
        let text = "name,value\n\"line one\nline two\",1\n\"b\",2\n\"c, \"\"q\"\"\",3\n";
        let imp = parse(text);
        assert_eq!(col(&imp, 0).text().unwrap(), ["line one\nline two", "b", "c, \"q\""]);
        assert_eq!(col(&imp, 1).numeric().unwrap(), &[1.0, 2.0, 3.0]);
        // CRLF line ends, and a stray quote falls back to plain lines
        let imp = parse("a,b\r\n\"x\r\ny\",1\r\n\"z\",2\r\nw,3\r\n");
        assert_eq!(col(&imp, 1).numeric().unwrap(), &[1.0, 2.0, 3.0]);
        assert_eq!(split_records("1,2\n5\" pipe,3\n7,8\n").len(), 3);
        assert_eq!(split_records("a\n\"open\nb\n").len(), 3);
    }

    #[test]
    fn icp_oes_like_file() {
        let text = "Sample,Cu 324.754,Zn 213.857,Pb 220.353\nName,mg/L,mg/L,mg/L\n\
                    S1,<LOD,0.52,n.d.\nS2,1.25,0.48,<LOD\nS3,n.d.,0.61,0.03\nS4,2.10,NA,0.04\nS5,0.95,0.55,----\n";
        let imp = parse(text);
        let d = &imp.detected;
        assert_eq!((d.header_lines, d.long_name_line, d.units_line, d.columns), (2, 1, 2, 4));
        assert_eq!(col(&imp, 0).designation, Designation::Label);
        assert_eq!(col(&imp, 0).text().unwrap(), ["S1", "S2", "S3", "S4", "S5"]);
        for j in 1..4 {
            assert!(col(&imp, j).numeric().is_some(), "column {j} numeric");
            assert_eq!(col(&imp, j).units, "mg/L");
            assert_eq!(col(&imp, j).numeric().unwrap().len(), 5);
        }
        let cu = col(&imp, 1).numeric().unwrap();
        assert!(cu[0].is_nan() && cu[2].is_nan());
        assert_eq!((cu[1], cu[3], cu[4]), (1.25, 2.1, 0.95));
        assert_eq!(col(&imp, 3).numeric().unwrap()[3], 0.04);
        // the first data row has only markers in its numeric cells and is still data
        let imp = parse("Sample,Cu,Zn\nName,mg/L,mg/L\nS1,<LOD,<LOD\nS2,1,2\nS3,3,4\n");
        assert_eq!(col(&imp, 0).text().unwrap().len(), 3);
    }

    #[test]
    fn dash_units_row_stays_a_header_line() {
        let imp = parse("Name,Value\n-,-\n1,2\n3,4\n5,6\n");
        assert_eq!(col(&imp, 0).numeric().unwrap(), [1.0, 3.0, 5.0]);
        assert_eq!(col(&imp, 0).units, "-");
        let imp = parse("Sample,Cu\nName,mg/L\n-,-\n1,2\n3,4\n5,6\n");
        assert_eq!(col(&imp, 1).numeric().unwrap(), [2.0, 4.0, 6.0]);
        assert_eq!(col(&imp, 1).units, "mg/L"); // the dash row is a third header line (comments), not data
    }

    #[test]
    fn marker_only_columns_keep_their_text() {
        let imp = parse("id,v\nNA,1\nND,2\nnd,3\n");
        assert_eq!(col(&imp, 0).text().unwrap(), ["NA", "ND", "nd"]);
        assert!(col(&imp, 1).numeric().is_some());
    }

    const ICP: &str = "Time,Cu 324.754,Zn 213.857,Sample\n\
        2026-11-11 13:45:10,1.25,0.50,blank\n\
        2026-11-11 13:47:42,12.5,<LOD,std 1\n\
        2026-11-11 13:50:15,24.9,5.10,std 2\n\
        2026-11-11 13:52:47,n.d.,10.2,std 3\n";

    #[test]
    fn an_icp_like_file_gets_a_date_time_column() {
        use sheaf_core::datetime::{DatePattern, TimePattern};
        let imp = parse(ICP);
        let t = col(&imp, 0);
        assert_eq!(t.long_name, "Time");
        assert_eq!(t.designation, Designation::X);
        assert_eq!(t.format, NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms));
        let v = t.numeric().unwrap();
        assert_eq!(v.len(), 4);
        // Same number Origin 2016 computes for 2026-11-11 13:45:10.
        assert!((v[0] - 2_461_355.573_032_407_6).abs() < 1e-9);
        assert!((v[1] - v[0] - 152.0 / 86_400.0).abs() < 1e-9);
        assert_eq!(t.cell_text(2), "2026-11-11 13:50:15");
        assert!(col(&imp, 1).numeric().is_some());
        assert_eq!(col(&imp, 3).text().unwrap()[0], "blank");
        assert!(imp.warnings.iter().all(|w| !w.contains("day first")), "{:?}", imp.warnings);
    }

    #[test]
    fn ratios_such_as_1_2_stay_text() {
        let imp = parse(
            "ratio,v
1:1,1
1:2,2
2:3,3
",
        );
        assert!(col(&imp, 0).text().is_some());
        assert_eq!(col(&imp, 0).text().unwrap()[1], "1:2");
    }

    #[test]
    fn dates_can_be_kept_as_text() {
        let opts = AsciiOptions { dates: DateImport::AsText, ..AsciiOptions::default() };
        let imp = parse_ascii(ICP, "f", &opts).unwrap();
        assert_eq!(col(&imp, 0).text().unwrap()[0], "2026-11-11 13:45:10");
        assert_eq!(col(&imp, 0).format, NumberFormat::General);
    }

    #[test]
    fn the_origin_sample_strings_import_as_origin_numbers() {
        // The same strings were imported by Origin 2016 and read back (see docs/spec/import-ascii.md).
        let us = parse("when,v\n11/11/2026 1:45:10 PM,1\n11/12/2026 1:05:00 AM,2\n11/13/2026 1:05:00 AM,3\n");
        let v = col(&us, 0).numeric().unwrap();
        assert!((v[0] - 2_461_355.573_032_407_6).abs() < 1e-9);
        assert!((v[1] - 2_461_356.045_138_889).abs() < 1e-9);
        let named = parse("when,v\n11-Nov-2026,1\n12-Nov-2026,2\n13-Nov-2026,3\n");
        assert_eq!(col(&named, 0).numeric().unwrap(), [2_461_355.0, 2_461_356.0, 2_461_357.0]);
        assert_eq!(col(&named, 0).cell_text(0), "11-Nov-2026");
        let clock = parse("when,v\n13:45,1\n01:05,2\n02:05,3\n");
        let v = col(&clock, 0).numeric().unwrap();
        assert!((v[0] - 0.572_916_666_666_666_6).abs() < 1e-15);
        assert_eq!(col(&clock, 0).cell_text(1), "01:05");
        let ms = parse("when,v\n13:45:10.250,1\n01:05:00,2\n02:05:00,3\n");
        assert!((col(&ms, 0).numeric().unwrap()[0] - 0.573_035_300_925_925_9).abs() < 1e-12);
        assert_eq!(col(&ms, 0).cell_text(0), "13:45:10.250");
    }

    #[test]
    fn day_and_month_order_is_taken_from_the_rows_that_decide_it() {
        // 13 and 27 cannot be months: the column is day first, with no warning.
        let imp = parse("d,v\n13/11/2026,1\n02/12/2026,2\n27/12/2026,3\n");
        let v = col(&imp, 0).numeric().unwrap();
        assert_eq!(v[1], sheaf_core::datetime::date_value(2026, 12, 2).unwrap());
        assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
        // Nothing decides: month first (the default), with a warning.
        let imp = parse("d,v\n02/12/2026,1\n03/12/2026,2\n04/12/2026,3\n");
        assert_eq!(col(&imp, 0).numeric().unwrap()[0], sheaf_core::datetime::date_value(2026, 2, 12).unwrap());
        assert!(imp.warnings.iter().any(|w| w.contains("month first")), "{:?}", imp.warnings);
        // ... and the option can choose day first.
        let opts = AsciiOptions { date_order: DateOrder::Dmy, ..AsciiOptions::default() };
        let imp = parse_ascii("d,v\n02/12/2026,1\n03/12/2026,2\n04/12/2026,3\n", "f", &opts).unwrap();
        assert_eq!(col(&imp, 0).numeric().unwrap()[0], sheaf_core::datetime::date_value(2026, 12, 2).unwrap());
        assert!(imp.warnings.iter().any(|w| w.contains("day first")), "{:?}", imp.warnings);
        // Only month first fits: no warning.
        let imp = parse("d,v\n11/13/2026,1\n11/14/2026,2\n11/15/2026,3\n");
        assert_eq!(col(&imp, 0).numeric().unwrap()[0], sheaf_core::datetime::date_value(2026, 11, 13).unwrap());
        assert!(imp.warnings.is_empty());
    }

    #[test]
    fn text_with_a_few_odd_cells_stays_text_and_missing_dates_are_blank() {
        let imp = parse("d,v\n2026-11-11,1\nsoon,2\n2026-11-13,3\n");
        assert!(col(&imp, 0).text().is_some());
        let imp = parse("d,v\n2026-11-11,1\n,2\nn.d.,4\n2026-11-13,3\n");
        let v = col(&imp, 0).numeric().unwrap();
        assert!(v[1].is_nan() && v[2].is_nan() && v[3].is_finite());
        assert_eq!(col(&imp, 0).cell_text(1), "--");
    }

    #[test]
    fn mixed_time_precision_uses_the_most_precise_pattern() {
        let imp = parse("t,v\n13:45,1\n13:46:30,2\n13:47,3\n");
        assert_eq!(col(&imp, 0).cell_text(0), "13:45:00");
    }

    #[test]
    fn formulas_use_date_columns_as_numbers() {
        use sheaf_core::Project;
        let imp = parse(ICP);
        let mut p = Project::new();
        p.adopt_workbook(imp.workbook);
        let ids: Vec<_> = p.workbooks().next().unwrap().sheets[0].columns.iter().map(|c| c.id).collect();
        // Minutes since the first row, using the date column in arithmetic.
        p.set_column_values(ids[2], "(col(A) - col(A)[1]) * 1440", None).unwrap();
        let v = p.numeric(ids[2]).unwrap();
        assert!(v[0].abs() < 1e-9);
        assert!((v[1] - 152.0 / 60.0).abs() < 1e-6, "{v:?}");
    }

    #[test]
    fn long_sheet_names_are_truncated() {
        let imp = parse_ascii("1\n", &"x".repeat(50), &AsciiOptions::default()).unwrap();
        assert_eq!(imp.workbook.sheets[0].name.chars().count(), 31);
        assert_eq!(parse_ascii("1\n", "", &AsciiOptions::default()).unwrap().workbook.sheets[0].name, "Sheet1");
    }
}
