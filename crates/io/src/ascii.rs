//! Delimited text import (CSV, TSV, whitespace-aligned columns).
//!
//! Everything is detected automatically by default: the delimiter, where the
//! data starts, which header lines hold names, units and comments, and which
//! columns are numbers. The dialog can show the result of [`detect`] and let
//! the user override any part through [`AsciiOptions`].

use crate::error::IoError;
use sheaf_core::column::{default_short_name, imported_short_name};
use sheaf_core::datetime::{self, DateOrder};
use sheaf_core::format::NumberFormat;
use sheaf_core::{Column, ColumnData, ColumnId, Designation, WindowId, Workbook, Worksheet};
use std::collections::HashMap;

/// Lines looked at when guessing the delimiter.
const DELIMITER_SAMPLE_LINES: usize = 200;
/// Consecutive numeric lines that mark the start of the data.
const DATA_RUN: usize = 3;
/// Longest worksheet name.
const SHEET_NAME_MAX: usize = 31;
/// How the fields of a line are separated.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// A string of two or more characters that separates the fields as a whole (`||`, `::`,
    /// `;;`). Origin's "Other" field accepts such a string (its option `MultipleDelimiters`).
    Multi(String),
    /// Fixed-width columns: the width of each column in characters, applied in order. The last
    /// width repeats for the columns after the list, so a single width applies to all columns
    /// (Origin's Fixed Width option). Quotes mean nothing here and fields are trimmed.
    Fixed(Vec<usize>),
}

impl Delimiter {
    /// Reads a delimiter typed in the "Other" field: one character is [`Delimiter::Other`], two or
    /// more are [`Delimiter::Multi`]; `None` for an empty string.
    pub fn from_text(text: &str) -> Option<Delimiter> {
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (None, _) => None,
            (Some(c), None) => Some(Delimiter::Other(c)),
            _ => Some(Delimiter::Multi(text.to_string())),
        }
    }
}

/// Reads a list of column widths such as `8, 12 10` (commas and spaces both separate). `None`
/// when the text has no number, or a number that is not a positive whole number.
pub fn parse_widths(text: &str) -> Option<Vec<usize>> {
    let widths: Option<Vec<usize>> = text
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<usize>().ok().filter(|w| *w > 0))
        .collect();
    widths.filter(|w| !w.is_empty())
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

/// Which character groups the digits of a number (`1,234.5`, `1.234,5`, `1 234,5`, `1'234.5`).
/// These are Origin's four number-separator choices plus "none" and "detect".
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Thousands {
    /// Use a separator when the file shows one unambiguously (a number such as `1,234,567`,
    /// `1,234.5`, `1.234,5` or `1 234`), none otherwise.
    #[default]
    Auto,
    /// No grouping character; `1,234` is not a number.
    Off,
    /// `1,234.5`: comma groups, dot is the decimal point.
    Comma,
    /// `1.234,5`: dot groups, comma is the decimal point.
    Dot,
    /// `1 234,5`: a space (also a no-break space) groups, comma is the decimal point.
    Space,
    /// `1'234.5`: apostrophe groups, dot is the decimal point.
    Apostrophe,
}

/// What to do with a file that holds several blocks of data separated by blank lines
/// (each block may repeat its header lines).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Blocks {
    /// One block: blank lines are skipped and everything is read as one table.
    #[default]
    One,
    /// Each block becomes its own sheet of the workbook.
    Sheets,
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
    /// Header lines fill the long name, units and comments of the columns (Origin's "rename
    /// columns" in Data Import Options). When false the columns keep empty labels; the header
    /// lines are still skipped.
    pub rename_columns: bool,
    /// 1-based line whose cells become the short names of the columns (letters, digits and
    /// underscores only; a cell that is empty, repeated or unusable keeps the automatic name).
    /// 0 keeps `A`, `B`, ... as short names.
    pub short_name_line: usize,
    /// Name the sheet after the file (without its extension). When false it is `Sheet1`.
    pub rename_sheet: bool,
    /// Give the book the file name as long name. When false the long name is empty.
    pub rename_book: bool,
    /// Numbers use `,` as decimal separator; `,` then cannot be the delimiter.
    pub decimal_comma: bool,
    /// The digit-grouping character. It is removed from numbers (`1,234.5` is 1234.5); dot and
    /// space also make the comma the decimal point.
    pub thousands: Thousands,
    /// Header lines (1-based) whose cells become named parameter rows of the columns, see
    /// [`AsciiOptions::param_name_from_first_cell`].
    pub param_lines: Vec<usize>,
    /// A parameter row takes its name from the first cell of its line (`Wavelength` in the first
    /// column, values in the others). When false the rows are named `Param1`, `Param2`, ... and
    /// the first column gets a value as well.
    pub param_name_from_first_cell: bool,
    /// What to do with a file that holds several blocks of data separated by blank lines.
    pub blocks: Blocks,
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
            rename_columns: true,
            short_name_line: 0,
            rename_sheet: true,
            rename_book: true,
            decimal_comma: false,
            thousands: Thousands::Auto,
            param_lines: Vec::new(),
            param_name_from_first_cell: true,
            blocks: Blocks::One,
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
    /// Numbers are read with a decimal comma (chosen, or implied by the thousands separator).
    pub decimal_comma: bool,
    /// The digit-grouping character in use; never `Auto`.
    pub thousands: Thousands,
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
fn split_fields(line: &str, delim: &Delimiter) -> Vec<String> {
    if let Delimiter::Fixed(widths) = delim {
        return split_fixed(line, widths);
    }
    let whitespace = matches!(delim, Delimiter::Whitespace);
    let sep: Vec<char> = match delim {
        Delimiter::Comma => vec![','],
        Delimiter::Semicolon => vec![';'],
        Delimiter::Other(c) => vec![*c],
        Delimiter::Multi(s) if !s.is_empty() => s.chars().collect(),
        _ => vec!['\t'],
    };
    let chars: Vec<char> = line.chars().collect();
    // The length of the separator that starts at `i`, if one does.
    let sep_at = |i: usize| -> Option<usize> {
        if whitespace {
            (chars[i] == ' ' || chars[i] == '\t').then_some(1)
        } else {
            chars[i..].starts_with(&sep).then_some(sep.len())
        }
    };
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut quoted = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_quotes {
            if c == '"' {
                if chars.get(i + 1) == Some(&'"') {
                    cur.push('"');
                    i += 1;
                } else {
                    in_quotes = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' && !quoted && cur.is_empty() && !sep.contains(&'"') {
            in_quotes = true;
            quoted = true;
        } else if let Some(len) = sep_at(i) {
            if !whitespace || quoted || !cur.is_empty() {
                fields.push(finish_field(&cur, quoted));
            }
            cur.clear();
            quoted = false;
            i += len - 1;
        } else if c.is_whitespace() && (cur.is_empty() || quoted) {
            // leading space, or space after a closing quote: ignored
        } else {
            cur.push(c);
        }
        i += 1;
    }
    if !whitespace || quoted || !cur.is_empty() {
        fields.push(finish_field(&cur, quoted));
    }
    fields
}

/// Cuts a line into fields of the given widths in characters; the last width repeats until the
/// line is used up. Fields are trimmed. A line shorter than the first width is one field.
fn split_fixed(line: &str, widths: &[usize]) -> Vec<String> {
    let chars: Vec<char> = line.trim_end_matches(['\r', '\n']).chars().collect();
    let mut fields = Vec::new();
    let mut at = 0;
    let mut k = 0;
    while at < chars.len() {
        let w = widths.get(k).or(widths.last()).copied().unwrap_or(1).max(1);
        let end = (at + w).min(chars.len());
        fields.push(chars[at..end].iter().collect::<String>().trim().to_string());
        at = end;
        k += 1;
    }
    fields
}

fn finish_field(cur: &str, quoted: bool) -> String {
    if quoted { cur.to_string() } else { cur.trim_end().to_string() }
}

/// Reads a number as the importer understands it: `1e-5`, `+3.2`, `.5`,
/// `-.5e3`, `NaN`, `inf`, and with `decimal_comma` also `1,5`.
fn parse_number(field: &str, ns: NumStyle) -> Option<f64> {
    let t = field.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(sep) = ns.thousands
        && let Some((_, dec)) = grouped_shape(t, sep)
        && dec.is_none_or(|d| d == ns.decimal)
    {
        let cleaned: String =
            t.chars().filter(|c| !is_group_char(*c, sep)).map(|c| if c == ns.decimal { '.' } else { c }).collect();
        return cleaned.parse().ok();
    }
    if ns.decimal == ',' && t.contains(',') {
        return t.replace(',', ".").parse().ok();
    }
    t.parse().ok()
}

/// How numbers are written in one file.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct NumStyle {
    decimal: char,
    thousands: Option<char>,
}

impl NumStyle {
    fn new(decimal_comma: bool, thousands: Thousands) -> NumStyle {
        let thousands_char = match thousands {
            Thousands::Auto | Thousands::Off => None,
            Thousands::Comma => Some(','),
            Thousands::Dot => Some('.'),
            Thousands::Space => Some(' '),
            Thousands::Apostrophe => Some('\''),
        };
        let comma = decimal_comma || thousands == Thousands::Dot;
        NumStyle { decimal: if comma { ',' } else { '.' }, thousands: thousands_char }
    }
}

fn is_group_char(c: char, sep: char) -> bool {
    match sep {
        ' ' => matches!(c, ' ' | '\u{a0}' | '\u{202f}'),
        '\'' => c == '\'' || c == '\u{2019}',
        _ => c == sep,
    }
}

/// Looks at `t` as `[sign] 1-3 digits, (separator + 3 digits)+, [decimal part and exponent]`.
/// Returns the number of 3-digit groups and the decimal character used after them, if any.
fn grouped_shape(t: &str, sep: char) -> Option<(usize, Option<char>)> {
    let t = t.strip_prefix(['+', '-']).unwrap_or(t);
    let mut chars = t.chars().peekable();
    let mut lead = 0;
    while chars.peek().is_some_and(char::is_ascii_digit) {
        chars.next();
        lead += 1;
    }
    if !(1..=3).contains(&lead) {
        return None;
    }
    let mut groups = 0;
    while chars.peek().is_some_and(|c| is_group_char(*c, sep)) {
        chars.next();
        for _ in 0..3 {
            if !chars.next().is_some_and(|c| c.is_ascii_digit()) {
                return None;
            }
        }
        groups += 1;
    }
    if groups == 0 {
        return None;
    }
    let rest: String = chars.collect();
    if rest.is_empty() {
        return Some((groups, None));
    }
    let dec = rest.chars().next()?;
    if !matches!(dec, '.' | ',') || is_group_char(dec, sep) {
        return None;
    }
    let digits = rest[1..].split(['e', 'E']).next().unwrap_or("");
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then_some((groups, Some(dec)))
}

/// Settles the number style of a file from the options and, for `Thousands::Auto`, from the cells:
/// a separator is adopted only when some number shows it unambiguously (`1,234,567`, `1,234.5`,
/// `1.234,5`, `1 234`, `1'234`); a lone `1,234` proves nothing, except in a comma-delimited file,
/// where a field holding it was quoted and so cannot be a decimal comma. Evidence for two different
/// separators cancels out. Returns the decimal-comma flag, the separator, and whether it was found
/// by detection.
fn resolve_style(opts: &AsciiOptions, rows: &[(usize, Vec<String>)], delimiter: &Delimiter) -> (bool, Thousands, bool) {
    if opts.thousands != Thousands::Auto {
        let comma = opts.decimal_comma || matches!(opts.thousands, Thousands::Dot | Thousands::Space);
        return (comma, opts.thousands, false);
    }
    let (mut comma, mut dot, mut space, mut apos, mut comma_decimal) = (false, false, false, false, false);
    for t in rows.iter().take(5000).flat_map(|(_, f)| f.iter()).map(|c| c.trim()).filter(|t| t.len() >= 5) {
        if let Some((g, dec)) = grouped_shape(t, ',')
            && (g >= 2 || dec == Some('.') || *delimiter == Delimiter::Comma)
        {
            comma = true;
        }
        if let Some((g, dec)) = grouped_shape(t, '.')
            && (g >= 2 || dec == Some(','))
        {
            dot = true;
        }
        if let Some((_, dec)) = grouped_shape(t, ' ') {
            space = true;
            comma_decimal |= dec == Some(',');
        }
        apos |= grouped_shape(t, '\'').is_some();
    }
    // With a decimal comma chosen, a comma or an apostrophe cannot group digits.
    let found: Vec<Thousands> = [
        (comma && !opts.decimal_comma, Thousands::Comma),
        (dot, Thousands::Dot),
        (space, Thousands::Space),
        (apos && !opts.decimal_comma, Thousands::Apostrophe),
    ]
    .into_iter()
    .filter_map(|(on, t)| on.then_some(t))
    .collect();
    match found.as_slice() {
        [Thousands::Space] => (opts.decimal_comma || comma_decimal, Thousands::Space, true),
        [one] => (opts.decimal_comma || *one == Thousands::Dot, *one, true),
        _ => (opts.decimal_comma, Thousands::Off, false),
    }
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

fn classify(field: &str, ns: NumStyle) -> Cell {
    if field.trim().is_empty() {
        Cell::Empty
    } else if parse_number(field, ns).is_some() {
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
fn is_numeric_row(fields: &[String], ns: NumStyle) -> bool {
    let mut any = false;
    for f in fields {
        match classify(f, ns) {
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
    /// The digit-grouping character was found by looking at the numbers, not chosen.
    thousands_found: bool,
    ns: NumStyle,
}

fn auto_delimiter(lines: &[&str], skip_comma: bool) -> Delimiter {
    let sample: Vec<&str> =
        lines.iter().copied().filter(|l| !l.trim().is_empty()).take(DELIMITER_SAMPLE_LINES).collect();
    let mut best: Option<(usize, Delimiter)> = None;
    for cand in [Delimiter::Tab, Delimiter::Comma, Delimiter::Semicolon, Delimiter::Whitespace] {
        if skip_comma && cand == Delimiter::Comma {
            continue;
        }
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for l in &sample {
            *counts.entry(split_fields(l, &cand).len()).or_default() += 1;
        }
        let top = counts.iter().filter(|(n, _)| **n > 1).map(|(n, freq)| (*freq, *n)).max();
        if let Some((freq, _)) = top
            && best.as_ref().is_none_or(|(b, _)| freq > *b)
        {
            best = Some((freq, cand));
        }
    }
    // A one-column file has no delimiter; tab keeps every line as one field.
    best.map_or(Delimiter::Tab, |(_, d)| d)
}

/// Index (into `rows`) of the first row of the data block.
fn find_data_start(rows: &[(usize, Vec<String>)], ns: NumStyle) -> Option<usize> {
    let run_ok = |i: usize, need: usize| {
        let n = rows[i].1.len();
        rows[i..i + need].iter().all(|(_, f)| f.len() == n && is_numeric_row(f, ns))
    };
    let full_runs = (rows.len() + 1).saturating_sub(DATA_RUN);
    // 1. A full run of lines with one field count, all numeric.
    if let Some(i) = (0..full_runs).find(|&i| run_ok(i, DATA_RUN)) {
        return Some(extend_back(rows, i, ns));
    }
    // 2. Ragged numeric rows: every line is numeric but the field counts differ.
    if let Some(i) = (0..full_runs).find(|&i| rows[i..i + DATA_RUN].iter().all(|(_, f)| is_numeric_row(f, ns))) {
        return Some(extend_back(rows, i, ns));
    }
    // 3. Fewer than three lines left: all of them must fit.
    if let Some(i) = (full_runs..rows.len()).find(|&i| run_ok(i, rows.len() - i)) {
        return Some(extend_back(rows, i, ns));
    }
    // Files with a text column or with odd text cells inside numeric columns. A row
    // starts the data when it holds at least one number or missing-value marker, its
    // other fields are text, and the next rows have the same field count with at
    // most half of the cells in its numeric columns being text.
    for i in 0..rows.len() {
        let first = &rows[i].1;
        let numeric: Vec<bool> =
            first.iter().map(|f| matches!(classify(f, ns), Cell::Number | Cell::Date | Cell::Marker)).collect();
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
                match classify(v, ns) {
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
    rows.iter().position(|(_, f)| is_numeric_row(f, ns))
}

/// Moves the data start up over rows that belong to the data block although a
/// cell in them is text (for example `3,trace` above numeric rows). Such a row
/// has the field count of the block and at least as many numbers (or
/// missing-value markers) as text cells; a name row has no numbers at all, so
/// it stays a header line.
fn extend_back(rows: &[(usize, Vec<String>)], start: usize, ns: NumStyle) -> usize {
    let width = rows[start].1.len();
    let mut s = start;
    while s > 0 {
        let f = &rows[s - 1].1;
        if f.len() != width {
            break;
        }
        let (mut good, mut marks, mut odd) = (0, 0, 0);
        for v in f {
            match classify(v, ns) {
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
    let delimiter = match opts.delimiter.clone() {
        Delimiter::Auto => {
            auto_delimiter(&lines, opts.decimal_comma || matches!(opts.thousands, Thousands::Dot | Thousands::Space))
        }
        d => d,
    };
    let all: Vec<(usize, Vec<String>)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i, split_fields(l, &delimiter)))
        .collect();
    let (decimal_comma, thousands, thousands_found) = resolve_style(opts, &all, &delimiter);
    let ns = NumStyle::new(decimal_comma, thousands);

    let (header_lines, start) = match opts.header_lines {
        Some(h) => (h, all.iter().position(|(i, _)| *i >= h)),
        None => match find_data_start(&all, ns) {
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
        detected: Detected {
            delimiter,
            header_lines,
            long_name_line,
            units_line,
            comments_line,
            columns,
            decimal_comma,
            thousands,
        },
        rows,
        thousands_found,
        ns,
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
    let comma_decimal = opts.decimal_comma || matches!(opts.thousands, Thousands::Dot | Thousands::Space);
    if comma_decimal && matches!(opts.delimiter, Delimiter::Comma | Delimiter::Other(',')) {
        return Err(IoError::InvalidOption(
            "the decimal separator and the field delimiter cannot both be a comma".to_string(),
        ));
    }
    if opts.decimal_comma && opts.thousands == Thousands::Comma {
        return Err(IoError::InvalidOption(
            "the decimal separator and the thousands separator cannot both be a comma".to_string(),
        ));
    }
    match &opts.delimiter {
        Delimiter::Multi(m) if m.is_empty() => {
            return Err(IoError::InvalidOption("the delimiter string is empty".to_string()));
        }
        Delimiter::Fixed(w) if w.is_empty() || w.contains(&0) => {
            return Err(IoError::InvalidOption(
                "fixed width needs at least one column width, and every width must be above 0".to_string(),
            ));
        }
        _ => {}
    }
    if opts.blocks == Blocks::Sheets {
        return parse_blocks(text, file_stem, opts);
    }
    parse_block(text, file_stem, opts)
}

/// Splits the text at blank lines into blocks that each hold data. A block of text without any
/// numeric row (a title, a header) belongs to the data block that follows it; one at the end is dropped.
fn split_blocks(text: &str, opts: &AsciiOptions) -> Vec<String> {
    let whole = analyze(text, opts);
    let ns = whole.ns;
    let delimiter = whole.detected.delimiter.clone();
    let mut blocks: Vec<Vec<&str>> = vec![Vec::new()];
    for rec in split_records(text) {
        if rec.trim().is_empty() {
            if !blocks.last().is_some_and(Vec::is_empty) {
                blocks.push(Vec::new());
            }
        } else if let Some(b) = blocks.last_mut() {
            b.push(rec);
        }
    }
    let mut out: Vec<String> = Vec::new();
    let mut pending: Vec<&str> = Vec::new();
    for b in blocks.into_iter().filter(|b| !b.is_empty()) {
        let numeric = b.iter().any(|l| is_numeric_row(&split_fields(l, &delimiter), ns));
        pending.extend(b);
        if numeric {
            out.push(pending.join("\n"));
            pending.clear();
        }
    }
    out
}

/// Reads each block of data (see [`Blocks`]) as its own sheet; sheets after the first are named
/// `name_2`, `name_3`, ...
fn parse_blocks(text: &str, file_stem: &str, opts: &AsciiOptions) -> Result<AsciiImport, IoError> {
    let chunks = split_blocks(text, opts);
    if chunks.len() < 2 {
        return parse_block(text, file_stem, opts);
    }
    let mut fixed = opts.clone();
    fixed.delimiter = analyze(text, opts).detected.delimiter;
    let mut first: Option<AsciiImport> = None;
    let mut sheets = Vec::new();
    let mut warnings = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let mut part = parse_block(chunk, file_stem, &fixed)?;
        let name = match (opts.rename_sheet, i) {
            (false, _) => format!("Sheet{}", i + 1),
            (true, 0) => sheet_name(file_stem),
            (true, _) => sheet_name(&format!("{file_stem}_{}", i + 1)),
        };
        for w in part.warnings.drain(..) {
            warnings.push(format!("block {}: {w}", i + 1));
        }
        let mut sheet = part.workbook.sheets.remove(0);
        sheet.name = name;
        sheets.push(sheet);
        first.get_or_insert(part);
    }
    let mut result = first.ok_or_else(|| IoError::NoData {
        source_name: file_stem.to_string(),
        detail: "no data rows were found".to_string(),
    })?;
    result.workbook.sheets = sheets;
    result.warnings = warnings;
    Ok(result)
}

fn parse_block(text: &str, file_stem: &str, opts: &AsciiOptions) -> Result<AsciiImport, IoError> {
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
            n.checked_sub(1).and_then(|i| a.lines.get(i)).map_or_else(Vec::new, |l| split_fields(l, &d.delimiter));
        f.resize(ncols, String::new());
        f
    };
    let label_line = |n: usize| if opts.rename_columns { n } else { 0 };
    let mut long_names = label_row(label_line(d.long_name_line));
    let mut units = label_row(label_line(d.units_line));
    let comments = label_row(label_line(d.comments_line));
    let short_names = short_names_from(&label_row(opts.short_name_line), ncols);
    if d.long_name_line > 0 && d.units_line == 0 {
        for (name, unit) in long_names.iter_mut().zip(units.iter_mut()) {
            if let Some((n, u)) = split_name_unit(name) {
                *name = n;
                *unit = u;
            }
        }
    }
    if a.thousands_found {
        let sep = match d.thousands {
            Thousands::Comma => "comma",
            Thousands::Dot => "dot",
            Thousands::Space => "space",
            _ => "apostrophe",
        };
        warnings.push(format!(
            "numbers with a {sep} as thousands separator were found; the separator was removed (see the thousands separator option)"
        ));
    }
    // Parameter rows: (name, cells, first column that gets a value).
    let params: Vec<(String, Vec<String>, usize)> = opts
        .param_lines
        .iter()
        .enumerate()
        .map(|(k, &line)| {
            let cells = label_row(line);
            let generic = format!("Param{}", k + 1);
            if opts.param_name_from_first_cell {
                let name = cells.first().map_or("", |c| c.trim().trim_end_matches(':').trim());
                (if name.is_empty() { generic } else { name.to_string() }, cells, 1)
            } else {
                (generic, cells, 0)
            }
        })
        .collect();
    let used: Vec<usize> = [d.long_name_line, d.units_line, d.comments_line]
        .into_iter()
        .chain(opts.param_lines.iter().copied())
        .chain([opts.short_name_line])
        .filter(|n| *n > 0)
        .collect();
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
    let mut sheet = Worksheet::new(if opts.rename_sheet { sheet_name(file_stem) } else { "Sheet1".to_string() });
    for j in 0..ncols {
        let cells: Vec<&str> = a.rows.iter().map(|r| r.get(j).map_or("", String::as_str)).collect();
        if opts.dates == DateImport::Detect
            && let Some(dc) = detect_date_column(&cells, opts.date_order)
        {
            if dc.ambiguous {
                warnings.push(format!(
                    "column {}: the dates can be read day first or month first; read {}",
                    short_names[j],
                    if dc.order == DateOrder::Mdy { "month first (M/d/yyyy)" } else { "day first (dd/MM/yyyy)" }
                ));
            }
            let designation = if j == 0 && opts.first_column_x { Designation::X } else { Designation::Y };
            let mut col = Column::new(
                ColumnId(j as u64 + 1),
                short_names[j].clone(),
                designation,
                ColumnData::numeric(dc.values),
            );
            col.format = dc.format;
            col.long_name = long_names[j].clone();
            col.units = units[j].clone();
            col.comments = comments[j].clone();
            sheet.columns.push(col);
            continue;
        }
        let numbers: Vec<Option<f64>> = cells.iter().map(|c| parse_number(c, a.ns)).collect();
        let parsed = numbers.iter().filter(|n| n.is_some()).count();
        let non_empty = cells.iter().filter(|c| !c.trim().is_empty()).count();
        let short = short_names[j].clone();
        // Origin keeps every imported column in its Text & Numeric format: a cell that is not a
        // number (a word, `<LOD`, `n.d.`) stays a text cell and the numbers see it as missing. There
        // is no ratio of numbers to text. A column with no number at all is a text column.
        let numeric = parsed > 0 || non_empty == 0;
        let (designation, data) = if numeric {
            let mut texts = std::collections::BTreeMap::new();
            for (row, (c, n)) in cells.iter().zip(&numbers).enumerate() {
                if n.is_none() && !c.trim().is_empty() {
                    texts.insert(row, c.trim().to_string());
                }
            }
            let designation = if j == 0 && opts.first_column_x { Designation::X } else { Designation::Y };
            (designation, ColumnData::mixed(numbers.into_iter().map(|n| n.unwrap_or(f64::NAN)).collect(), texts))
        } else {
            let text = cells.iter().map(|c| c.to_string()).collect();
            // Origin designates the columns of a text file by position, so a first column of sample
            // names is the X column: the Y columns are drawn against the names (graphing spec 14.6).
            let designation = if j == 0 && opts.first_column_x { Designation::X } else { Designation::Label };
            (designation, ColumnData::text(text))
        };
        let mut col = Column::new(ColumnId(j as u64 + 1), short, designation, data);
        col.long_name = long_names[j].clone();
        col.units = units[j].clone();
        col.comments = comments[j].clone();
        sheet.columns.push(col);
    }
    for (j, col) in sheet.columns.iter_mut().enumerate() {
        for (name, cells, from) in &params {
            if j >= *from
                && let Some(v) = cells.get(j).filter(|v| !v.trim().is_empty())
            {
                col.params.insert(name.clone(), v.trim().to_string());
            }
        }
    }

    let mut workbook = Workbook::new(WindowId(0), "Book1");
    if opts.rename_book {
        workbook.long_name = file_stem.to_string();
    }
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

/// The short name of every column, as Origin's importer makes them (measured 2026-11-13): the cell
/// of the short-name line with everything but letters, digits and dots dropped, a first digit or
/// dot made `A`, cut to 18 characters; the automatic name when the cell is missing, has nothing
/// left, or another column has the name already (compared without regard to case).
fn short_names_from(cells: &[String], ncols: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for j in 0..ncols {
        let wanted = cells.get(j).map(|c| imported_short_name(c)).unwrap_or_default();
        let usable = !wanted.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&wanted));
        out.push(if usable { wanted } else { default_short_name(j) });
    }
    // An automatic name can clash with a header cell that an earlier column took (`B` in the
    // header of column A); the later column then gets a numbered name.
    for j in 0..out.len() {
        if out[..j].iter().any(|o| o.eq_ignore_ascii_case(&out[j])) {
            let base = out[j].clone();
            let free = (2..).map(|k| format!("{base}_{k}")).find(|n| !out.iter().any(|o| o.eq_ignore_ascii_case(n)));
            out[j] = free.expect("the sequence is unbounded");
        }
    }
    out
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
    fn fixed_width_counts_characters_not_bytes() {
        assert_eq!(split_fixed("é1.5 ü2.5", &[4, 5]), vec!["é1.5", "ü2.5"]);
        assert_eq!(split_fixed("日本語abc", &[3]), vec!["日本語", "abc"]);
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
            assert_eq!(parse_number(s, NumStyle::new(false, Thousands::Off)), Some(v), "{s}");
        }
        assert!(parse_number("NaN", NumStyle::new(false, Thousands::Off)).unwrap().is_nan());
        assert_eq!(parse_number("1,5", NumStyle::new(false, Thousands::Off)), None);
        assert_eq!(parse_number("1,5", NumStyle::new(true, Thousands::Off)), Some(1.5));
        assert_eq!(parse_number("abc", NumStyle::new(false, Thousands::Off)), None);
        assert_eq!(parse_number("", NumStyle::new(false, Thousands::Off)), None);
    }

    fn with(thousands: Thousands) -> AsciiOptions {
        AsciiOptions { thousands, ..AsciiOptions::default() }
    }

    #[test]
    fn comma_thousands_are_found_when_a_number_shows_them() {
        let text = "name\tvalue\na\t1,234.5\nb\t2,345.25\nc\t1,000,000.5\nd\t7\n";
        let imp = parse(text);
        assert_eq!(imp.detected.thousands, Thousands::Comma);
        assert!(!imp.detected.decimal_comma);
        assert_eq!(col(&imp, 1).numeric().unwrap(), &[1234.5, 2345.25, 1_000_000.5, 7.0]);
        assert!(imp.warnings.iter().any(|w| w.contains("thousands")), "{:?}", imp.warnings);
        // Chosen by hand there is nothing to report.
        let chosen = parse_ascii(text, "f", &with(Thousands::Comma)).unwrap();
        assert!(chosen.warnings.is_empty(), "{:?}", chosen.warnings);
        assert_eq!(col(&chosen, 1).numeric().unwrap(), col(&imp, 1).numeric().unwrap());
    }

    #[test]
    fn a_lone_1_234_proves_nothing_until_the_option_says_so() {
        let text = "x\ty\n1\t1,234\n2\t2,345\n3\t3,456\n";
        let auto = parse(text);
        assert_eq!(auto.detected.thousands, Thousands::Off);
        assert!(col(&auto, 1).text().is_some(), "left as text, nothing is guessed");
        let comma = parse_ascii(text, "f", &with(Thousands::Comma)).unwrap();
        assert_eq!(col(&comma, 1).numeric().unwrap(), &[1234.0, 2345.0, 3456.0]);
        // The same cells are 1.234 ... with a decimal comma.
        let dec = parse_ascii(text, "f", &AsciiOptions { decimal_comma: true, ..AsciiOptions::default() }).unwrap();
        assert_eq!(col(&dec, 1).numeric().unwrap(), &[1.234, 2.345, 3.456]);
    }

    #[test]
    fn dot_thousands_with_decimal_comma_match_the_origin_run() {
        // Reference: Origin `impasc` with the "1.000,00" separator setting read a file of these
        // four cells (made by us) as 1234.5, 2345.25, 12000 and 7.
        let text = "x;y\n1;1.234,5\n2;2.345,25\n3;12.000\n4;7\n";
        let imp = parse(text);
        assert_eq!((imp.detected.thousands, imp.detected.decimal_comma), (Thousands::Dot, true));
        assert_eq!(col(&imp, 1).numeric().unwrap(), &[1234.5, 2345.25, 12000.0, 7.0]);
        let chosen = parse_ascii(text, "f", &with(Thousands::Dot)).unwrap();
        assert_eq!(col(&chosen, 1).numeric().unwrap(), &[1234.5, 2345.25, 12000.0, 7.0]);
    }

    #[test]
    fn decimal_comma_alone_matches_the_origin_run() {
        // Reference: with the "1.000,00" setting Origin read 1,5 / 2,25 / 3,5 / 7,125 as 1.5, 2.25, 3.5, 7.125.
        let imp = parse_ascii(
            "x\ty\n1\t1,5\n2\t2,25\n3\t3,5\n4\t7,125\n",
            "f",
            &AsciiOptions { decimal_comma: true, ..AsciiOptions::default() },
        )
        .unwrap();
        assert_eq!(col(&imp, 1).numeric().unwrap(), &[1.5, 2.25, 3.5, 7.125]);
        // A plain dot decimal stays a decimal point under a decimal comma.
        assert_eq!(parse_number("1.5", NumStyle::new(true, Thousands::Off)), Some(1.5));
    }

    #[test]
    fn space_and_apostrophe_thousands() {
        let sp = parse("x\ty\n1\t1 234,5\n2\t2\u{a0}345,25\n3\t12 000\n4\t7\n");
        assert_eq!((sp.detected.thousands, sp.detected.decimal_comma), (Thousands::Space, true));
        assert_eq!(col(&sp, 1).numeric().unwrap(), &[1234.5, 2345.25, 12000.0, 7.0]);
        let sp_dot = parse("x\ty\n1\t1 234.5\n2\t2 345.25\n3\t7\n");
        assert_eq!((sp_dot.detected.thousands, sp_dot.detected.decimal_comma), (Thousands::Space, false));
        assert_eq!(col(&sp_dot, 1).numeric().unwrap(), &[1234.5, 2345.25, 7.0]);
        let ap = parse("x\ty\n1\t1'234.5\n2\t2'345.25\n3\t1'000'000\n");
        assert_eq!(ap.detected.thousands, Thousands::Apostrophe);
        assert_eq!(col(&ap, 1).numeric().unwrap(), &[1234.5, 2345.25, 1_000_000.0]);
    }

    #[test]
    fn grouping_must_sit_between_groups_of_three() {
        let ns = NumStyle::new(false, Thousands::Comma);
        assert_eq!(parse_number("1,234", ns), Some(1234.0));
        assert_eq!(parse_number("-12,345,678.25", ns), Some(-12_345_678.25));
        assert_eq!(parse_number("1,234.5e3", ns), Some(1_234_500.0));
        assert_eq!(parse_number("1,23", ns), None);
        assert_eq!(parse_number("1234,567", ns), None);
        assert_eq!(parse_number("1,234,5", ns), None);
        assert_eq!(parse_number("1,234.5.6", ns), None);
        assert_eq!(parse_number("1234.5", ns), Some(1234.5), "no grouping at all is fine");
        let dot = NumStyle::new(false, Thousands::Dot);
        assert_eq!(parse_number("1.234", dot), Some(1234.0));
        assert_eq!(parse_number("12.5", dot), Some(12.5), "a short group after the dot is a decimal point");
    }

    #[test]
    fn two_kinds_of_evidence_cancel_out() {
        let imp = parse("x\ty\n1\t1,234.5\n2\t1.234,5\n3\t7\n");
        assert_eq!(imp.detected.thousands, Thousands::Off);
    }

    #[test]
    fn lone_quoted_grouped_values_in_a_comma_file_are_numbers() {
        let imp = parse(
            "a,b
\"1,234\",\"2,345\"
\"3,456\",\"1,000\"
",
        );
        assert_eq!(imp.detected.thousands, Thousands::Comma);
        assert_eq!(col(&imp, 0).numeric().unwrap(), &[1234.0, 3456.0]);
        assert_eq!(col(&imp, 1).numeric().unwrap(), &[2345.0, 1000.0]);
    }

    #[test]
    fn thousands_in_quoted_csv_fields() {
        let imp = parse("name,value\n\"a\",\"1,234.5\"\n\"b\",\"2,345.5\"\n\"c\",\"7\"\n");
        assert_eq!(imp.detected.delimiter, Delimiter::Comma);
        assert_eq!(col(&imp, 1).numeric().unwrap(), &[1234.5, 2345.5, 7.0]);
        let off =
            parse_ascii("name,value\n\"a\",\"1,234.5\"\n\"b\",\"2,345.5\"\n\"c\",\"7\"\n", "f", &with(Thousands::Off))
                .unwrap();
        // Without the separator the first two rows are not numbers, so they count as header lines.
        assert!(!col(&off, 1).numeric().unwrap().contains(&1234.5));
        let bad = AsciiOptions { decimal_comma: true, thousands: Thousands::Comma, ..AsciiOptions::default() };
        assert!(matches!(parse_ascii("1\n", "f", &bad), Err(IoError::InvalidOption(_))));
    }

    /// An export of an ICP-OES run: sample names, one line per element with its wavelength, a units
    /// line, a date-time column, and entries below the detection limit.
    const ICP_RUN: &str = "\
Sample Name,Fe 238.204,Cu 324.754,Zn 213.857,Measured\n\
Wavelength (nm),238.204,324.754,213.857,\n\
Units,mg/L,mg/L,mg/L,\n\
Blank,<LOD,0.021,n.d.,2026-05-04 09:15:22\n\
Std 1,1.02,0.98,1.10,2026-05-04 09:17:40\n\
Std 2,5.11,4.97,5.30,2026-05-04 09:19:58\n\
Sample A,12.40,0.045,<LOD,2026-05-04 09:22:16\n";

    #[test]
    fn icp_oes_export_with_parameter_rows() {
        let opts = AsciiOptions {
            header_lines: Some(3),
            long_name_line: Some(1),
            units_line: Some(3),
            comments_line: Some(0),
            param_lines: vec![2],
            ..AsciiOptions::default()
        };
        let imp = parse_ascii(ICP_RUN, "run1", &opts).unwrap();
        let sheet = &imp.workbook.sheets[0];
        assert_eq!(sheet.columns.len(), 5);
        let names = &sheet.columns[0];
        assert_eq!(names.text().unwrap(), ["Blank", "Std 1", "Std 2", "Sample A"]);
        assert_eq!(names.designation, Designation::X, "sample names in the first column are X");
        assert!(names.params.is_empty(), "the first cell of a parameter row is its name");
        let fe = &sheet.columns[1];
        assert_eq!((fe.long_name.as_str(), fe.units.as_str()), ("Fe 238.204", "mg/L"));
        assert_eq!(fe.params.get("Wavelength (nm)").map(String::as_str), Some("238.204"));
        let v = fe.numeric().unwrap();
        assert!(v[0].is_nan());
        assert_eq!(&v[1..], &[1.02, 5.11, 12.4]);
        assert!(sheet.columns[3].numeric().unwrap()[0].is_nan());
        // The date-time column holds Origin date numbers and displays as date-time.
        let t = &sheet.columns[4];
        assert!(matches!(t.format, NumberFormat::DateTime(..)), "{:?}", t.format);
        let s = t.numeric().unwrap();
        assert!((s[1] - s[0] - 138.0 / 86400.0).abs() < 1e-9);
        // No-value entries stay in the sheet as text cells (Origin's Text & Numeric), not as a warning.
        let joined = imp.warnings.join("\n");
        assert!(fe.data.is_mixed());
        assert!(!joined.contains("no-value marker"), "{joined}");
        assert!(imp.warnings.iter().all(|w| !w.contains("header line")), "{joined}");
    }

    #[test]
    fn icp_oes_export_is_detected_without_any_option() {
        let imp = parse(ICP_RUN);
        let sheet = &imp.workbook.sheets[0];
        assert_eq!(imp.detected.header_lines, 3);
        assert_eq!(sheet.columns[0].text().unwrap().len(), 4);
        assert_eq!(sheet.columns[1].numeric().unwrap().len(), 4);
    }

    #[test]
    fn parameter_rows_can_be_named_generically() {
        let text = "Sample,A,B\nWavelength,10,20\nPurpose,x,y\n1,2,3\n2,3,4\n3,4,5\n";
        let opts = AsciiOptions {
            header_lines: Some(3),
            long_name_line: Some(1),
            units_line: Some(0),
            comments_line: Some(0),
            param_lines: vec![2, 3],
            param_name_from_first_cell: false,
            ..AsciiOptions::default()
        };
        let imp = parse_ascii(text, "p", &opts).unwrap();
        let c = &imp.workbook.sheets[0].columns;
        assert_eq!(c[0].params.get("Param1").map(String::as_str), Some("Wavelength"));
        assert_eq!(c[2].params.get("Param1").map(String::as_str), Some("20"));
        assert_eq!(c[2].params.get("Param2").map(String::as_str), Some("y"));
        // By first cell: the first column carries no value and the rows are named from it.
        let named = parse_ascii(text, "p", &AsciiOptions { param_name_from_first_cell: true, ..opts }).unwrap();
        let c = &named.workbook.sheets[0].columns;
        assert!(c[0].params.is_empty());
        assert_eq!(c[1].params.get("Wavelength").map(String::as_str), Some("10"));
        assert_eq!(c[2].params.get("Purpose").map(String::as_str), Some("y"));
    }

    #[test]
    fn blocks_become_sheets_when_asked() {
        // The shape of a file we ran through Origin: a title line, then two tables with the same header.
        let text = "junk header line\nTime\tA\ns\tmV\n1\t10\n2\t20\n3\t30\n\nTime\tA\ns\tmV\n7\t70\n8\t80\n9\t90\n";
        let one = parse(text);
        assert_eq!(one.workbook.sheets.len(), 1);
        let two =
            parse_ascii(text, "blocks", &AsciiOptions { blocks: Blocks::Sheets, ..AsciiOptions::default() }).unwrap();
        let sheets = &two.workbook.sheets;
        assert_eq!(sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["blocks", "blocks_2"]);
        for (s, first) in sheets.iter().zip([1.0, 7.0]) {
            assert_eq!((s.columns[0].long_name.as_str(), s.columns[0].units.as_str()), ("Time", "s"));
            assert_eq!(s.columns[0].numeric().unwrap()[0], first);
            assert_eq!(s.columns[1].numeric().unwrap().len(), 3);
        }
        // Without a blank line there is one block and one sheet.
        let single = parse_ascii(
            "x,y\n1,2\n3,4\n5,6\n",
            "s",
            &AsciiOptions { blocks: Blocks::Sheets, ..AsciiOptions::default() },
        )
        .unwrap();
        assert_eq!(single.workbook.sheets.len(), 1);
    }

    #[test]
    fn quoted_fields() {
        assert_eq!(split_fields(r#"a,"b,c","say ""hi""", d "#, &Delimiter::Comma), ["a", "b,c", "say \"hi\"", "d"]);
        assert_eq!(split_fields(r#""two words" x  "y z""#, &Delimiter::Whitespace), ["two words", "x", "y z"]);
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
        // Origin has no ratio: one word among numbers stays as a text cell of a Text & Numeric column
        assert!(col(&imp, 2).numeric().unwrap()[5].is_nan());
        assert_eq!(col(&imp, 2).cell_text(5), "oops");
        assert!(col(&imp, 2).data.is_mixed());
        assert!(imp.warnings.iter().all(|w| !w.contains("column C")));
        // and so does a column that is mostly words
        let imp = parse("a,b\n1,x\n2,y\n3,4\n4,z\n5,w\n");
        assert_eq!(col(&imp, 1).designation, Designation::Y);
        assert_eq!(col(&imp, 1).cell_text(0), "x");
        assert_eq!(col(&imp, 1).numeric().unwrap()[2], 4.0);
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
        assert!(col(&imp, 0).text().is_some(), "no numbers: a text column");
    }

    #[test]
    fn a_first_column_of_names_is_x_and_later_text_columns_are_labels() {
        let imp = parse("Sample,Li,K,Mn,Note\nS1,1.2,0.5,0.2,a\nS2,2.5,0.9,0.3,b\n");
        let a = col(&imp, 0);
        assert_eq!(a.designation, Designation::X);
        assert_eq!(a.text().unwrap(), ["S1", "S2"]);
        assert_eq!(a.long_name, "Sample");
        for j in 1..4 {
            assert_eq!(col(&imp, j).designation, Designation::Y);
        }
        assert_eq!(col(&imp, 4).designation, Designation::Label, "a second text column stays a label column");
        let all_y = AsciiOptions { first_column_x: false, ..AsciiOptions::default() };
        let imp = parse_ascii("Sample,Li\nS1,1.2\nS2,2.5\n", "t", &all_y).unwrap();
        assert_eq!(imp.workbook.sheets[0].columns[0].designation, Designation::Label);
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
        assert_eq!(col(&imp, 0).designation, Designation::X);
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
        // Same number Origin computes for 2026-11-11 13:45:10.
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
        // The same strings were imported by Origin and read back (see docs/spec/import-ascii.md).
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

    fn parse_with(text: &str, opts: &AsciiOptions) -> AsciiImport {
        parse_ascii(text, "run1", opts).unwrap()
    }

    #[test]
    fn a_multi_character_delimiter_splits_as_a_whole_string() {
        let text = "t||i\n1||2.5\n2||3.5\n3||4.5\n4||5.5\n";
        let opts = AsciiOptions { delimiter: Delimiter::from_text("||").unwrap(), ..AsciiOptions::default() };
        let imp = parse_with(text, &opts);
        let cols = &imp.workbook.sheets[0].columns;
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[1].numeric().unwrap()[..], [2.5, 3.5, 4.5, 5.5]);
        assert_eq!(cols[1].long_name, "i");
        // A lone `|` inside a field is not a separator; a quoted field may hold the whole string.
        assert_eq!(split_fields("a|b||\"c||d\"||e", &Delimiter::Multi("||".into())), ["a|b", "c||d", "e"]);
        assert_eq!(Delimiter::from_text("|"), Some(Delimiter::Other('|')));
        assert_eq!(Delimiter::from_text(""), None);
        let empty = AsciiOptions { delimiter: Delimiter::Multi(String::new()), ..AsciiOptions::default() };
        assert!(parse_ascii(text, "x", &empty).is_err());
    }

    #[test]
    fn fixed_width_columns_are_cut_by_the_widths_and_the_last_width_repeats() {
        let text = "  x   y   z\n 1.0 2.5 3.5\n 2.0 3.5 4.5\n 3.0 4.5 5.5\n 4.0 5.5 6.5\n";
        let opts = AsciiOptions { delimiter: Delimiter::Fixed(vec![4]), ..AsciiOptions::default() };
        let imp = parse_with(text, &opts);
        let cols = &imp.workbook.sheets[0].columns;
        assert_eq!(cols.len(), 3, "the single width applies to every column");
        assert_eq!(cols[2].numeric().unwrap()[..], [3.5, 4.5, 5.5, 6.5]);
        assert_eq!(cols[0].long_name, "x");
        // Different widths, and a value that touches its neighbour (no delimiter at all).
        let glued = "ab12.5\ncd13.5\nef14.5\ngh15.5\n";
        let opts = AsciiOptions { delimiter: Delimiter::Fixed(vec![2, 2, 3]), ..AsciiOptions::default() };
        let imp = parse_with(glued, &opts);
        let cols = &imp.workbook.sheets[0].columns;
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[1].numeric().unwrap()[..], [12.0, 13.0, 14.0, 15.0]);
        assert_eq!(cols[2].numeric().unwrap()[..], [0.5; 4]);
        assert_eq!(parse_widths("8, 12 10"), Some(vec![8, 12, 10]));
        assert_eq!(parse_widths("8,x"), None);
        assert_eq!(parse_widths("0"), None);
        assert_eq!(parse_widths(" "), None);
        let bad = AsciiOptions { delimiter: Delimiter::Fixed(vec![]), ..AsciiOptions::default() };
        assert!(parse_ascii(glued, "x", &bad).is_err());
    }

    #[test]
    fn the_short_name_line_names_the_columns_and_unusable_cells_keep_the_automatic_name() {
        let text = "Pot (V),Cur-rent,Cur-rent,3rd\nvolt,amp,amp,x\n1,2,3,4\n2,3,4,5\n3,4,5,6\n4,5,6,7\n";
        let opts = AsciiOptions { short_name_line: 1, long_name_line: Some(2), ..AsciiOptions::default() };
        let imp = parse_with(text, &opts);
        let names: Vec<_> = imp.workbook.sheets[0].columns.iter().map(|c| c.short_name.as_str()).collect();
        // `Pot (V)` loses its spaces and brackets, a repeated name falls back to the automatic one
        // and a first digit becomes A.
        assert_eq!(names, ["PotV", "Current", "C", "Ard"]);
        assert_eq!(imp.workbook.sheets[0].columns[0].long_name, "volt");
        assert!(imp.warnings.iter().all(|w| !w.contains("header")), "{:?}", imp.warnings);
    }

    #[test]
    fn data_import_options_can_leave_the_labels_and_the_names_alone() {
        let text = "t,i\ns,A\n1,2\n2,3\n3,4\n4,5\n";
        let kept = parse_with(text, &AsciiOptions::default());
        assert_eq!(kept.workbook.sheets[0].columns[1].long_name, "i");
        assert_eq!(kept.workbook.sheets[0].name, "run1");
        assert_eq!(kept.workbook.long_name, "run1");
        let opts =
            AsciiOptions { rename_columns: false, rename_sheet: false, rename_book: false, ..Default::default() };
        let plain = parse_with(text, &opts);
        let cols = &plain.workbook.sheets[0].columns;
        assert_eq!((cols[1].long_name.as_str(), cols[1].units.as_str()), ("", ""));
        assert_eq!(cols[1].numeric().unwrap().len(), 4, "the header lines are still skipped");
        assert_eq!(plain.workbook.sheets[0].name, "Sheet1");
        assert_eq!(plain.workbook.long_name, "");
    }
}
