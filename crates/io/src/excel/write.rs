//! Writing worksheets as an `.xlsx` workbook.

use super::{EMPTY_LABEL, dates};
use crate::error::IoError;
use crate::save::write_atomically;
use rust_xlsxwriter::{Format, Workbook as XlsxBook};
use sheaf_core::format::NumberFormat;
use sheaf_core::{Column, ColumnData, Workbook, Worksheet};
use std::path::{Path, PathBuf};

/// Excel's limits.
const MAX_ROWS: usize = 1_048_576;
const MAX_COLUMNS: usize = 16_384;
const MAX_SHEET_NAME: usize = 31;
const MAX_TEXT: usize = 32_767;

/// What to write besides the data. Sheaf's choices (Origin's Excel export is not measured): a row
/// of long names, units and comments above the data, each only when some column has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcelExportOptions {
    /// A row of long names. On by default; left out when no column has one.
    pub long_names: bool,
    /// A row of units. On by default; left out when no column has one.
    pub units: bool,
    /// A row of comments. On by default; left out when no column has one.
    pub comments: bool,
    /// A first row of short (column) names. Off by default. The importer takes it for a long-name
    /// row unless told otherwise (`short_name_line`).
    pub short_names: bool,
}

impl Default for ExcelExportOptions {
    fn default() -> Self {
        ExcelExportOptions { long_names: true, units: true, comments: true, short_names: false }
    }
}

/// What [`write_xlsx`] did.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportOutcome {
    /// The file written (`.xlsx` is added when the name had no extension).
    pub path: PathBuf,
    /// The Excel sheet names, in order; they differ from the Sheaf names where Excel's rules
    /// required it (31 characters, none of `[]:*?/\`, unique).
    pub sheet_names: Vec<String>,
    /// Things that changed on the way (renamed sheets, values Excel cannot hold).
    pub warnings: Vec<String>,
}

/// Writes every sheet of `book` to an `.xlsx` file.
pub fn write_workbook_xlsx(book: &Workbook, path: &Path, opts: &ExcelExportOptions) -> Result<ExportOutcome, IoError> {
    let sheets: Vec<&Worksheet> = book.sheets.iter().collect();
    write_xlsx(&sheets, path, opts)
}

/// Writes the given sheets to an `.xlsx` file, one Excel sheet per Sheaf sheet.
///
/// Numbers are numbers (every digit), text is text, a missing value is an empty cell. A column
/// shown as a date, time or date-time is written as Excel dates with a number format of the same
/// pattern. The file is written to a temporary name and renamed, and a file that is there already
/// is replaced only when it is an `.xlsx` workbook; any other file is left alone and the call fails.
pub fn write_xlsx(sheets: &[&Worksheet], path: &Path, opts: &ExcelExportOptions) -> Result<ExportOutcome, IoError> {
    let path = with_extension(path)?;
    let fail = |reason: String| IoError::ExcelWrite { path: path.clone(), reason };
    if sheets.is_empty() {
        return Err(fail("there is no sheet to write".to_string()));
    }
    if path.exists() && !is_xlsx(&path) {
        return Err(IoError::ExportExists { path });
    }
    let mut warnings = Vec::new();
    let names = excel_names(sheets, &mut warnings);
    let mut book = XlsxBook::new();
    for (sheet, name) in sheets.iter().zip(&names) {
        let target = book.add_worksheet();
        target.set_name(name).map_err(|e| fail(e.to_string()))?;
        write_sheet(target, sheet, opts, &mut warnings).map_err(|e| fail(format!("sheet '{}': {e}", sheet.name)))?;
    }
    let bytes = book.save_to_buffer().map_err(|e| fail(e.to_string()))?;
    write_atomically(&path, &bytes)?;
    Ok(ExportOutcome { path, sheet_names: names, warnings })
}

/// Adds `.xlsx` to a name without extension; refuses another extension.
fn with_extension(path: &Path) -> Result<PathBuf, IoError> {
    match path.extension().and_then(|e| e.to_str()) {
        None => Ok(path.with_extension("xlsx")),
        Some(e) if e.eq_ignore_ascii_case("xlsx") => Ok(path.to_path_buf()),
        Some(e) => Err(IoError::InvalidOption(format!(
            "an Excel export is written as .xlsx, not .{e}; give the file the extension .xlsx"
        ))),
    }
}

/// True when the file is a zip archive that holds an Excel workbook part.
fn is_xlsx(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else { return false };
    zip::ZipArchive::new(file).is_ok_and(|mut z| z.by_name("xl/workbook.xml").is_ok())
}

/// Excel sheet names: at most 31 UTF-16 units (what Excel counts as characters), none of
/// `[]:*?/\`, not starting or ending with an apostrophe, not `History`, not empty, and different
/// from each other ignoring case (all of Unicode's case, as Excel does).
fn excel_names(sheets: &[&Worksheet], warnings: &mut Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (i, s) in sheets.iter().enumerate() {
        let cleaned: String = s.name.chars().map(|c| if "[]:*?/\\".contains(c) { '_' } else { c }).collect();
        let mut base = cut_utf16(cleaned.trim_matches('\'').trim(), MAX_SHEET_NAME);
        base = base.trim_matches('\'').to_string();
        if base.is_empty() {
            base = format!("Sheet{}", i + 1);
        }
        if base.to_lowercase() == "history" {
            base.push('_');
        }
        let taken = |name: &str, out: &[String]| out.iter().any(|o| o.to_lowercase() == name.to_lowercase());
        let mut name = base.clone();
        let mut k = 2;
        while taken(&name, &out) {
            let suffix = format!("_{k}");
            name = format!("{}{suffix}", cut_utf16(&base, MAX_SHEET_NAME - suffix.len()));
            k += 1;
        }
        if name != s.name {
            warnings.push(format!("sheet '{}' is called '{name}' in the Excel file", s.name));
        }
        out.push(name);
    }
    out
}

/// The longest start of `text` that is at most `max` UTF-16 units long, never splitting a character.
fn cut_utf16(text: &str, max: usize) -> String {
    let mut units = 0;
    let mut out = String::new();
    for c in text.chars() {
        units += c.len_utf16();
        if units > max {
            break;
        }
        out.push(c);
    }
    out
}

fn write_sheet(
    target: &mut rust_xlsxwriter::Worksheet,
    sheet: &Worksheet,
    opts: &ExcelExportOptions,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let cols = &sheet.columns;
    if cols.len() > MAX_COLUMNS {
        return Err(format!("it has {} columns and Excel holds {MAX_COLUMNS}", cols.len()));
    }
    // Long name, units and comments are told apart by their place: up to three rows right above
    // the data, in that order. A row that is written because a later one needs it, or that no
    // column fills, holds only the invisible `EMPTY_LABEL` in its first cell, so that the importer
    // still finds every label in its own row.
    let any = |text: fn(&Column) -> &str| cols.iter().any(|c| !text(c).is_empty());
    let comments_row = opts.comments && any(|c| &c.comments);
    let units_row = comments_row || (opts.units && any(|c| &c.units));
    let long_row = units_row || (opts.long_names && any(|c| &c.long_name));
    let mut labels: Vec<Vec<&str>> = Vec::new();
    if opts.short_names {
        labels.push(cols.iter().map(|c| c.short_name.as_str()).collect());
    }
    let label_rows: [(bool, bool, fn(&Column) -> &str); 3] = [
        (long_row, opts.long_names, |c| &c.long_name),
        (units_row, opts.units, |c| &c.units),
        (comments_row, opts.comments, |c| &c.comments),
    ];
    for (written, wanted, text) in label_rows {
        if !written {
            continue;
        }
        let mut line: Vec<&str> = cols.iter().map(|c| if wanted { text(c) } else { "" }).collect();
        if line.iter().all(|t| t.is_empty())
            && let Some(first) = line.first_mut()
        {
            *first = EMPTY_LABEL;
        }
        labels.push(line);
    }
    let rows = sheet.row_count();
    if labels.len() + rows > MAX_ROWS {
        return Err(format!("it has {rows} rows and Excel holds {MAX_ROWS}"));
    }
    let err = |e: rust_xlsxwriter::XlsxError| e.to_string();
    for (r, line) in labels.iter().enumerate() {
        for (c, text) in line.iter().enumerate() {
            if !text.is_empty() {
                target.write_string(r as u32, c as u16, clip(text)).map_err(err)?;
            }
        }
    }
    let top = labels.len() as u32;
    let mut notes = Notes::default();
    for (c, col) in cols.iter().enumerate() {
        let (format, width) = column_format(col);
        if let Some(w) = width {
            target.set_column_width(c as u16, w).map_err(err)?;
        }
        write_column(target, col, c as u16, top, format.as_ref(), &mut notes).map_err(err)?;
    }
    let leading = (0..rows).take_while(|&r| row_is_blank(cols, r)).count();
    let trailing = if leading == rows { 0 } else { (0..rows).rev().take_while(|&r| row_is_blank(cols, r)).count() };
    if leading + trailing > 0 {
        warnings.push(format!(
            "sheet '{}': {} rows at the start or end hold no value in any column; the import skips blank rows there, so they will not come back",
            sheet.name,
            leading + trailing
        ));
    }
    if notes.infinite > 0 {
        warnings.push(format!(
            "sheet '{}': {} infinite values cannot be stored in Excel and were left empty",
            sheet.name, notes.infinite
        ));
    }
    if notes.early_dates > 0 {
        warnings.push(format!(
            "sheet '{}': {} dates before 1900-01-01 cannot be Excel dates and were written as plain numbers",
            sheet.name, notes.early_dates
        ));
    }
    if notes.clipped > 0 {
        warnings.push(format!("sheet '{}': {} texts longer than 32767 characters were cut", sheet.name, notes.clipped));
    }
    Ok(())
}

/// True when no column writes a cell in this row.
fn row_is_blank(cols: &[Column], row: usize) -> bool {
    cols.iter().all(|c| match &c.data {
        ColumnData::Numeric(v) => v.get(row).is_none_or(|x| !x.is_finite()),
        ColumnData::Text(v) => v.get(row).is_none_or(String::is_empty),
        ColumnData::Mixed { numbers, texts } => {
            numbers.get(row).is_none_or(|x| !x.is_finite()) && texts.get(&row).is_none_or(String::is_empty)
        }
    })
}

#[derive(Default)]
struct Notes {
    infinite: usize,
    early_dates: usize,
    clipped: usize,
}

fn clip(text: &str) -> &str {
    match text.char_indices().nth(MAX_TEXT) {
        Some((i, _)) => &text[..i],
        None => text,
    }
}

/// How a column's numbers are shown in Excel, and a width when the default is too narrow.
fn column_format(col: &Column) -> (Option<Format>, Option<f64>) {
    let num = |code: String| Some(Format::new().set_num_format(code));
    match col.format {
        NumberFormat::Date(d) => (num(dates::date_code(d).to_string()), Some(14.0)),
        NumberFormat::DateTime(d, t) => {
            (num(format!("{} {}", dates::date_code(d), dates::time_code(t, false))), Some(22.0))
        }
        NumberFormat::Time(t) => {
            // A negative time needs the elapsed code: the clock code shows only 0 to 24 hours.
            let elapsed = max_abs(col) >= 1.0 || col.numeric().is_some_and(|v| v.iter().any(|x| *x < 0.0));
            (num(dates::time_code(t, elapsed).to_string()), Some(12.0))
        }
        NumberFormat::Decimal(n) => (num(decimals(n)), None),
        NumberFormat::DecimalGrouped(n) => (num(format!("#,##{}", decimals(n))), None),
        NumberFormat::Scientific(n) => (num(format!("{}E+00", decimals(n))), None),
        _ => (None, None),
    }
}

fn decimals(n: u8) -> String {
    if n == 0 { "0".to_string() } else { format!("0.{}", "0".repeat(usize::from(n))) }
}

fn max_abs(col: &Column) -> f64 {
    col.numeric().map_or(0.0, |v| v.iter().filter(|x| x.is_finite()).fold(0.0, |m, x| m.max(x.abs())))
}

fn write_column(
    target: &mut rust_xlsxwriter::Worksheet,
    col: &Column,
    c: u16,
    top: u32,
    format: Option<&Format>,
    notes: &mut Notes,
) -> Result<(), rust_xlsxwriter::XlsxError> {
    let is_date = matches!(col.format, NumberFormat::Date(_) | NumberFormat::DateTime(..));
    let mut put_number = |target: &mut rust_xlsxwriter::Worksheet, row: usize, x: f64| {
        if x.is_nan() {
            return Ok(());
        }
        if x.is_infinite() {
            notes.infinite += 1;
            return Ok(());
        }
        let (value, format) = if is_date {
            match dates::serial_from_date(x) {
                Some(s) => (s, format),
                None => {
                    notes.early_dates += 1;
                    (x, None)
                }
            }
        } else {
            (x, format)
        };
        let r = top + row as u32;
        match format {
            Some(f) => target.write_number_with_format(r, c, value, f).map(|_| ()),
            None => target.write_number(r, c, value).map(|_| ()),
        }
    };
    match &col.data {
        ColumnData::Numeric(v) => {
            for (row, x) in v.iter().enumerate() {
                put_number(target, row, *x)?;
            }
        }
        ColumnData::Text(v) => {
            for (row, s) in v.iter().enumerate().filter(|(_, s)| !s.is_empty()) {
                if s.chars().count() > MAX_TEXT {
                    notes.clipped += 1;
                }
                target.write_string(top + row as u32, c, clip(s))?;
            }
        }
        ColumnData::Mixed { numbers, texts } => {
            for (row, x) in numbers.iter().enumerate() {
                match texts.get(&row).filter(|_| x.is_nan()) {
                    Some(s) => {
                        if s.chars().count() > MAX_TEXT {
                            notes.clipped += 1;
                        }
                        target.write_string(top + row as u32, c, clip(s))?;
                    }
                    None => put_number(target, row, *x)?,
                }
            }
            // Text cells below the last number.
            for (row, s) in texts.range(numbers.len()..) {
                target.write_string(top + *row as u32, c, clip(s))?;
            }
        }
    }
    Ok(())
}
