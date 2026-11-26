//! Reader for Origin Unicode projects (`.opju`, Origin 2018 and later). The format is described in
//! `docs/spec/opju-format.md`.
//!
//! It reads workbooks (the sheets and the columns with their numbers and text). Graphs, matrices,
//! notes, the folder tree and the column labels (long names, units, comments, designations) are not
//! read yet; each is named in [`OpjImport::warnings`] so that nothing disappears silently.

mod numbers;
mod records;
#[cfg(test)]
mod tests;

use crate::error::IoError;
use crate::opj::OpjImport;
use records::{ColumnValues, WindowRecord, find_columns, find_windows};
use sheaf_core::column::{Column, ColumnData, Designation};
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::worksheet::{ImportSource, Workbook, Worksheet};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

pub use numbers::MISSING;

/// What a file holds, counted by the same scan that [`read_opju`] uses.
#[derive(Clone, Debug, Default)]
pub struct Summary {
    /// Window records found (workbooks, matrices, graphs and the rest).
    pub windows: usize,
    pub workbook_windows: usize,
    pub matrix_windows: usize,
    pub graph_windows: usize,
    /// Workbooks rebuilt from the names of their columns because no window record was found.
    pub windowless_workbooks: usize,
    /// Column records found in the file (including data behind graphs).
    pub column_records: usize,
    /// Column records that were decoded into a workbook.
    pub columns_read: usize,
    /// Column records of workbooks that were not decoded, by reason.
    pub columns_skipped: BTreeMap<&'static str, usize>,
    /// Column records that belong to no workbook (data behind graphs and matrices).
    pub columns_unmatched: usize,
    /// Columns cut short because the values after some row cannot be data, and the values dropped.
    pub columns_cut: usize,
    pub values_dropped: usize,
    /// The names of the first skipped records, with the reason and the first bytes of the block.
    pub examples: Vec<String>,
}

/// Where a column goes: the n-th workbook window, or a workbook known only by its columns.
enum Target {
    Window(usize),
    Named(String),
}

type Sheets = BTreeMap<usize, Vec<(String, ColumnData)>>;

/// True when the bytes start with the signature of a Unicode project.
pub fn has_signature(bytes: &[u8]) -> bool {
    bytes.starts_with(b"CPYUA")
}

/// Counts the windows and column records of the bytes of an `.opju` file.
pub fn summarize(bytes: &[u8]) -> Summary {
    build(bytes, Path::new("")).map(|(_, s)| s).unwrap_or_default()
}

/// Reads an `.opju` file.
pub fn read_opju(path: &Path) -> Result<OpjImport, IoError> {
    let bytes = std::fs::read(path).map_err(|e| IoError::io(path, e))?;
    read_opju_bytes(&bytes, path)
}

/// Reads an `.opju` project that is already in memory; `path` is used in messages.
pub fn read_opju_bytes(bytes: &[u8], path: &Path) -> Result<OpjImport, IoError> {
    build(bytes, path).map(|(import, _)| import)
}

/// The workbook window whose name, followed by an underscore, starts the data-set name `base`.
fn owner_of<'a>(books: &[&'a WindowRecord], base: &str) -> Option<(usize, &'a WindowRecord)> {
    books
        .iter()
        .enumerate()
        .filter(|(_, w)| {
            base.len() > w.name.len() + 1 && base.starts_with(w.name.as_str()) && base.as_bytes()[w.name.len()] == b'_'
        })
        .max_by_key(|(_, w)| w.name.len())
        .map(|(i, w)| (i, *w))
}

fn build(bytes: &[u8], path: &Path) -> Result<(OpjImport, Summary), IoError> {
    let bad = |reason: &str| IoError::OpjuFormat { path: path.to_path_buf(), reason: reason.to_string() };
    if !has_signature(bytes) {
        return Err(bad("it does not start with the Origin signature 'CPYUA'"));
    }
    let line_end = bytes.iter().position(|&c| c == b'\n').ok_or_else(|| bad("the signature line is not finished"))?;
    let signature = String::from_utf8_lossy(&bytes[..line_end.min(200)]).trim().to_string();
    let version = signature.split_whitespace().nth(1).unwrap_or("unknown").to_string();

    let windows = find_windows(bytes);
    let columns = find_columns(bytes);
    let mut warnings = Vec::new();
    let mut summary = Summary { windows: windows.len(), column_records: columns.len(), ..Summary::default() };

    let is_matrix = |w: &WindowRecord| w.sheets.iter().any(|s| s.starts_with("MSheet"));
    let book_windows: Vec<&WindowRecord> =
        windows.iter().filter(|w| w.has_origin_mark && !w.sheets.is_empty() && !is_matrix(w)).collect();
    summary.workbook_windows = book_windows.len();
    summary.matrix_windows = windows.iter().filter(|w| is_matrix(w)).count();
    summary.graph_windows = windows.len() - summary.workbook_windows - summary.matrix_windows;

    let mut by_book: BTreeMap<usize, Sheets> = BTreeMap::new();
    let mut by_named: BTreeMap<String, Sheets> = BTreeMap::new();
    for rec in columns {
        let base = rec.name.split('@').next().unwrap_or("");
        let sheet_no =
            rec.name.split_once('@').and_then(|(_, n)| n.parse::<usize>().ok()).filter(|&n| n >= 1).unwrap_or(1);
        let owner = owner_of(&book_windows, base);
        let in_matrix = owner.is_none()
            && windows.iter().any(|w| {
                is_matrix(w)
                    && base.len() > w.name.len()
                    && base.starts_with(w.name.as_str())
                    && base.as_bytes()[w.name.len()] == b'_'
            });
        if in_matrix {
            summary.columns_unmatched += 1;
            continue;
        }
        // A workbook without a preview picture has no window record that this reader finds; its
        // columns are grouped by the text before the first underscore. `CS_n` is data behind graphs.
        let fallback = match base.split_once('_') {
            Some((book, rest)) if owner.is_none() && !book.is_empty() && !rest.is_empty() && book != "CS" => {
                Some((book.to_string(), rest.to_string()))
            }
            _ => None,
        };
        let (target, short) = match (owner, fallback) {
            (Some((ix, w)), _) => (Target::Window(ix), base[w.name.len() + 1..].to_string()),
            (None, Some((book, rest))) => (Target::Named(book), rest),
            (None, None) => {
                summary.columns_unmatched += 1;
                continue;
            }
        };
        let data = match rec.values {
            ColumnValues::Numeric(v, dropped) => {
                if dropped > 0 {
                    summary.columns_cut += 1;
                    summary.values_dropped += dropped;
                    if summary.examples.len() < 400 {
                        summary.examples.push(format!("cut: {} kept {} dropped {}", rec.name, v.len(), dropped));
                    }
                }
                ColumnData::Numeric(Arc::new(v))
            }
            ColumnValues::Text(v) => ColumnData::Text(Arc::new(v)),
            ColumnValues::Unsupported(why) => {
                *summary.columns_skipped.entry(why).or_insert(0) += 1;
                if summary.examples.len() < 400 {
                    let head: Vec<String> = rec.head.iter().map(|b| format!("{b:02x}")).collect();
                    summary.examples.push(format!("{why}: {} [{}]", rec.name, head.join(" ")));
                }
                continue;
            }
        };
        summary.columns_read += 1;
        let sheets = match target {
            Target::Window(ix) => by_book.entry(ix).or_default(),
            Target::Named(book) => by_named.entry(book).or_default(),
        };
        sheets.entry(sheet_no).or_default().push((short, data));
    }

    summary.windowless_workbooks = by_named.len();
    let mut next_id = 1u64;
    let mut workbooks = Vec::new();
    let books = by_book
        .into_iter()
        .map(|(ix, sheets)| (book_windows[ix].name.clone(), Some(book_windows[ix]), sheets))
        .chain(by_named.into_iter().map(|(name, sheets)| (name, None, sheets)));
    for (book_name, window, sheets) in books {
        let mut wb = Workbook::new(WindowId(0), book_name);
        wb.long_name = window.map(|w| w.long_name.clone()).unwrap_or_default();
        wb.sheets.clear();
        for (sheet_no, cols) in sheets {
            let name =
                window.and_then(|w| w.sheets.get(sheet_no - 1).cloned()).unwrap_or_else(|| format!("Sheet{sheet_no}"));
            let mut sheet = Worksheet::new(name);
            for (k, (short, data)) in cols.into_iter().enumerate() {
                let designation = if k == 0 { Designation::X } else { Designation::Y };
                sheet.columns.push(Column::new(ColumnId(next_id), short, designation, data));
                next_id += 1;
            }
            wb.sheets.push(sheet);
        }
        wb.source = Some(ImportSource { path: path.display().to_string(), importer: "opju".to_string() });
        workbooks.push(wb);
    }

    if summary.windowless_workbooks > 0 {
        warnings.push(format!(
            "{} workbook(s) have no window record that this reader finds; they were rebuilt from the names of \
             their columns (no long name, sheets called Sheet1, Sheet2, ...).",
            summary.windowless_workbooks
        ));
    }
    if summary.graph_windows > 0 {
        warnings.push(format!(
            "{} graph window(s) were not imported: graphs of .opju files are not read yet.",
            summary.graph_windows
        ));
    }
    if summary.matrix_windows > 0 {
        warnings.push(format!(
            "{} matrix window(s) were not imported: matrices of .opju files are not read yet.",
            summary.matrix_windows
        ));
    }
    if summary.columns_unmatched > 0 {
        warnings.push(format!(
            "{} data block(s) belong to no workbook (data behind graphs and matrices) and were skipped.",
            summary.columns_unmatched
        ));
    }
    if summary.columns_cut > 0 {
        warnings.push(format!(
            "{} column(s) were cut short: {} value(s) at their end could not be read as numbers and were left out.",
            summary.columns_cut, summary.values_dropped
        ));
    }
    for (why, n) in &summary.columns_skipped {
        warnings.push(format!("{n} column(s) were skipped because they hold {why}."));
    }
    warnings.push(
        "Long names, units, comments and the X/Y roles of the columns, the sheet and workbook properties and the \
         folder tree of .opju files are not read yet; the first column of each sheet is X and the others are Y, \
         and every window is in the root folder."
            .to_string(),
    );
    let import = OpjImport {
        workbooks,
        graphs: Vec::new(),
        version,
        signature,
        folders: Vec::new(),
        warnings,
        comments: Vec::new(),
        notes: Vec::new(),
        matrices: Vec::new(),
    };
    Ok((import, summary))
}
