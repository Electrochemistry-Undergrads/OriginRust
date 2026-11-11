//! Reader for Origin project files (`.opj`, written by Origin 7.5 to 2016).
//!
//! It reads workbooks, their sheets and columns (names, long names, units, comments,
//! designations) and the column values. Graphs, matrices, notes, fits and everything
//! else are skipped and reported in [`OpjImport::warnings`]. The format is described in
//! `docs/spec/opj-format.md`.
//!
//! The file is read through a buffered reader: the data blocks are stepped over on the
//! first pass and read one column at a time on the second, so memory use is about the
//! size of the decoded numbers, not of the file.

mod dataset;
mod frame;
mod strings;
#[cfg(test)]
mod testutil;
mod window;

use crate::error::IoError;
use dataset::DataSetInfo;
use frame::{FrameError, Frames};
use sheaf_core::column::{Column, Designation};
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::worksheet::{ImportSource, Workbook, Worksheet};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

/// What [`read_opj`] found.
#[derive(Clone, Debug)]
pub struct OpjImport {
    /// The workbooks in file order, each with its sheets in file order. Ids are placeholders.
    pub workbooks: Vec<Workbook>,
    /// The Origin version that saved the file, for example `9.3`; `unknown` if the header is damaged.
    pub version: String,
    /// The first line of the file, for example `CPYA 4.3168 226 W64 #`.
    pub signature: String,
    /// Plain-English notes about what was skipped or could not be read.
    pub warnings: Vec<String>,
}

/// Reads an `.opj` file.
pub fn read_opj(path: &Path) -> Result<OpjImport, IoError> {
    let file = File::open(path).map_err(|e| IoError::io(path, e))?;
    read_opj_from(BufReader::with_capacity(1 << 20, file), path)
}

/// Reads an `.opj` project from any seekable byte source; `path` is used in messages and
/// recorded as the source of the workbooks.
pub fn read_opj_from<R: Read + Seek>(reader: R, path: &Path) -> Result<OpjImport, IoError> {
    let bad = |reason: String| IoError::OpjFormat { path: path.to_path_buf(), reason };
    let from_frame = |e: FrameError| match e {
        FrameError::Io(source) => IoError::io(path, source),
        other => bad(other.to_string()),
    };
    let mut f = Frames::new(reader).map_err(|e| IoError::io(path, e))?;

    let line = f.line(200).map_err(|_| bad("it does not start with the Origin signature 'CPYA'".into()))?;
    let signature = strings::decode_cp1252(&line).trim().to_string();
    if signature.starts_with("CPYUA") {
        return Err(bad("it is in the newer Unicode project format (.opju), which is not supported".into()));
    }
    if !signature.starts_with("CPYA") {
        return Err(bad("it does not start with the Origin signature 'CPYA'".into()));
    }
    let header = f.block().map_err(from_frame)?;
    let version = origin_version(&header);
    f.block().map_err(from_frame)?; // empty block that closes the header

    let mut warnings = Vec::new();
    let (sets, damage) = dataset::scan(&mut f);
    if let Some(why) = damage {
        if sets.is_empty() {
            return Err(bad(why));
        }
        warnings.push(format!("The data of the project is damaged after {} column(s): {why}.", sets.len()));
    }
    let (windows, damage) = if warnings.is_empty() { window::scan(&mut f) } else { (Vec::new(), None) };
    if let Some(why) = damage {
        warnings.push(format!("The window list is damaged ({why}); windows after that point are missing."));
    }

    let mut b = Builder { f, sets, warnings, next_column: 1, skipped: BTreeMap::new(), counts: Counts::default() };
    let mut workbooks = b.workbooks(&windows).map_err(from_frame)?;
    if workbooks.is_empty() && !b.sets.is_empty() {
        b.warnings.push(
            "No workbook could be matched to its columns; the columns were grouped by their stored names, \
             so sheet names, long names, units, comments and column roles are missing."
                .to_string(),
        );
        workbooks = b.fallback().map_err(from_frame)?;
    }
    let source = ImportSource { path: path.display().to_string(), importer: "opj".to_string() };
    for w in &mut workbooks {
        w.source = Some(source.clone());
    }
    b.finish_warnings(windows.len(), workbooks.len());
    if workbooks.is_empty() {
        b.warnings.push("The project holds no workbook data (only graphs, matrices or empty workbooks).".to_string());
    }
    Ok(OpjImport { workbooks, version, signature, warnings: b.warnings })
}

/// The version number stored in the header block, as `9.3`.
fn origin_version(header: &[u8]) -> String {
    let Some(bytes) = header.get(0x1B..0x23) else { return "unknown".to_string() };
    let v = f64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
    if !v.is_finite() || v <= 0.0 || v >= 100.0 {
        return "unknown".to_string();
    }
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[derive(Default)]
struct Counts {
    mixed_to_text: usize,
    stray: usize,
    time_columns: usize,
    unlinked: usize,
}

struct Builder<R> {
    f: Frames<R>,
    sets: Vec<DataSetInfo>,
    warnings: Vec<String>,
    next_column: u64,
    /// Reason a column was skipped -> how many.
    skipped: BTreeMap<String, usize>,
    counts: Counts,
}

impl<R: Read + Seek> Builder<R> {
    fn new_id(&mut self) -> ColumnId {
        let id = ColumnId(self.next_column);
        self.next_column += 1;
        id
    }

    /// Decodes one data set into a column of the given designation.
    fn column(&mut self, set: usize, short_name: &str, designation: Designation) -> Result<Option<Column>, FrameError> {
        let info = self.sets[set].clone();
        let bytes = self.f.read_extent(info.data)?;
        match dataset::decode(&info, &bytes) {
            Ok(d) => {
                self.counts.mixed_to_text += usize::from(d.mixed_to_text);
                self.counts.stray += usize::from(d.stray_bytes > 0);
                Ok(Some(Column::new(self.new_id(), short_name, designation, d.data)))
            }
            Err(reason) => {
                *self.skipped.entry(reason).or_insert(0) += 1;
                Ok(None)
            }
        }
    }

    fn workbooks(&mut self, windows: &[window::Window]) -> Result<Vec<Workbook>, FrameError> {
        // Every prefix before an underscore of a data-set name is a possible window name.
        let mut prefixes: HashSet<&str> = HashSet::new();
        for s in &self.sets {
            let base = s.name.split('@').next().unwrap_or("");
            for (i, c) in base.char_indices() {
                if c == '_' && i > 0 {
                    prefixes.insert(&base[..i]);
                }
            }
        }
        let is_book: Vec<bool> = windows.iter().map(|w| prefixes.contains(w.name.as_str())).collect();
        let mut out = Vec::new();
        for (w, book) in windows.iter().zip(is_book) {
            if !book {
                continue;
            }
            let mut wb = Workbook::new(WindowId(0), w.name.clone());
            wb.long_name = w.label.clone();
            wb.sheets.clear();
            let prefix = format!("{}_", w.name);
            for (n, layer) in w.layers.iter().enumerate() {
                let name = if layer.name.is_empty() { format!("Sheet{}", n + 1) } else { layer.name.clone() };
                let mut sheet = Worksheet::new(name);
                for rec in &layer.columns {
                    let linked = rec.dataset.checked_sub(1).filter(|&i| i < self.sets.len());
                    let Some(i) = linked.filter(|&i| self.sets[i].name.starts_with(&prefix)) else {
                        self.counts.unlinked += 1;
                        continue;
                    };
                    let short = {
                        let rest = &self.sets[i].name[prefix.len()..];
                        let rest = rest.split('@').next().unwrap_or("");
                        if rest.is_empty() { rec.short_name.clone() } else { rest.to_string() }
                    };
                    let designation = designation_of(rec.designation);
                    let Some(mut col) = self.column(i, &short, designation)? else { continue };
                    let (long, units, comments) = strings::column_labels(&rec.note);
                    col.long_name = long;
                    col.units = units;
                    col.comments = comments;
                    if is_time_type(rec.value_type) {
                        self.counts.time_columns += 1;
                    }
                    sheet.columns.push(col);
                }
                wb.sheets.push(sheet);
            }
            if wb.sheets.iter().all(|s| s.columns.is_empty()) {
                continue;
            }
            out.push(wb);
        }
        Ok(out)
    }

    /// Workbooks rebuilt from data-set names alone, for files whose window list is unusable.
    fn fallback(&mut self) -> Result<Vec<Workbook>, FrameError> {
        // window name -> sheet number -> data-set indices, windows in order of first appearance
        let mut order: Vec<String> = Vec::new();
        let mut groups: HashMap<String, BTreeMap<u32, Vec<usize>>> = HashMap::new();
        for (i, s) in self.sets.iter().enumerate() {
            let (base, sheet) = match s.name.split_once('@') {
                Some((b, n)) => (b, n.parse::<u32>().unwrap_or(1)),
                None => (s.name.as_str(), 1),
            };
            let Some((window, _)) = base.rsplit_once('_').filter(|(w, c)| !w.is_empty() && !c.is_empty()) else {
                continue;
            };
            if !groups.contains_key(window) {
                order.push(window.to_string());
            }
            groups.entry(window.to_string()).or_default().entry(sheet).or_default().push(i);
        }
        let mut out = Vec::new();
        for window in order {
            let mut wb = Workbook::new(WindowId(0), window.clone());
            wb.sheets.clear();
            for (n, (_, members)) in groups[&window].iter().enumerate() {
                let mut sheet = Worksheet::new(format!("Sheet{}", n + 1));
                for (k, &i) in members.iter().enumerate() {
                    let short =
                        self.sets[i].name.split('@').next().unwrap_or("").rsplit('_').next().unwrap_or("").to_string();
                    let designation = if k == 0 { Designation::X } else { Designation::Y };
                    if let Some(col) = self.column(i, &short, designation)? {
                        sheet.columns.push(col);
                    }
                }
                wb.sheets.push(sheet);
            }
            if wb.sheets.iter().any(|s| !s.columns.is_empty()) {
                out.push(wb);
            }
        }
        Ok(out)
    }

    fn finish_warnings(&mut self, windows: usize, books: usize) {
        let skipped_windows = windows.saturating_sub(books);
        if skipped_windows > 0 && books > 0 {
            self.warnings.push(format!(
                "{skipped_windows} window(s) that are not workbooks (graphs, matrices, notes, layouts) were not imported."
            ));
        }
        for (reason, n) in std::mem::take(&mut self.skipped) {
            self.warnings.push(format!("{n} column(s) were skipped because of {reason}."));
        }
        let c = &self.counts;
        if c.mixed_to_text > 0 {
            self.warnings
                .push(format!("{} column(s) held both text and numbers; they were imported as text.", c.mixed_to_text));
        }
        if c.stray > 0 {
            self.warnings.push(format!("{} column(s) had incomplete trailing data, which was ignored.", c.stray));
        }
        if c.time_columns > 0 {
            self.warnings.push(format!(
                "{} column(s) hold dates or times; their values are Origin's day numbers, not formatted dates.",
                c.time_columns
            ));
        }
        if c.unlinked > 0 {
            self.warnings
                .push(format!("{} column record(s) could not be matched to stored data and were skipped.", c.unlinked));
        }
    }
}

/// The column role from the stored code (0 Y, 3 X, 5 Z, 2 and 6 errors, 4 labels).
fn designation_of(code: u8) -> Designation {
    match code {
        0 => Designation::Y,
        3 => Designation::X,
        5 => Designation::Z,
        2 => Designation::YError,
        6 => Designation::XError,
        4 => Designation::Label,
        _ => Designation::Disregard,
    }
}

/// Column value types that mean a date, a time, a month or a weekday.
fn is_time_type(code: u8) -> bool {
    matches!(code, 0x02 | 0x03 | 0x33 | 0x04 | 0x34 | 0x05 | 0x35)
}

#[cfg(test)]
mod tests;
