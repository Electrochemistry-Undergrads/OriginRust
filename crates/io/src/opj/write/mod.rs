//! Writer of Origin project files (`.opj`, the format Origin 2016 / version 9.3 saves).
//!
//! It writes the workbooks of a project: sheets, numeric and text columns with missing values,
//! short and long names, units, comments and column roles, and the project tree with every window
//! in one folder, and the graphs (`graph.rs`: axes, plots, titles, text, legend). Whatever the file
//! cannot hold is named in [`OpjWrite::warnings`]. The layout is described in
//! `docs/spec/opj-format.md`, section 13.
//!
//! The file is built in memory in the order Origin writes it: signature, version header, one
//! block triple per column (the data sets), then the windows with their layers and column
//! records, then the sections after the window list. Graph windows come from [`Writer::graph`].

#[cfg(test)]
mod feature_tests;
mod glayout;
mod graph;
#[cfg(test)]
mod graph_feature_tests;
#[cfg(test)]
mod graph_tests;
#[rustfmt::skip]
mod graph_tables;
mod formulas;
mod layout;
mod tail;
#[cfg(test)]
mod tests;

use super::dataset::MISSING;
use super::strings::encode_cp1252;
use crate::error::IoError;
use layout::{ColumnSpec, DatasetSpec, LayerSpec, WindowSpec, framed};
use sheaf_core::Project;
use sheaf_core::column::{Column, ColumnData, Designation, FormulaState, RecalcMode};
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::project::{Folder, Window};
use sheaf_core::worksheet::{Workbook, Worksheet};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tail::{Dependency, FolderOut, TailSpec};

/// The bytes of an Origin project and what was left out of it.
#[derive(Clone, Debug)]
pub struct OpjWrite {
    pub bytes: Vec<u8>,
    /// Plain-English notes about everything of the project that is not in the file.
    pub warnings: Vec<String>,
}

/// Writes `project` as an Origin 2016 project, named after the project's root folder.
pub fn write(project: &Project) -> Result<OpjWrite, IoError> {
    write_named(project, &project.root.name, julian_day_now())
}

/// Writes `project` to `path` (an existing file is replaced) and returns the warnings. The project
/// is named after the file.
pub fn write_file(project: &Project, path: &Path) -> Result<Vec<String>, IoError> {
    let name = path.file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty()).unwrap_or("Project");
    let out = write_named(project, name, julian_day_now())?;
    std::fs::write(path, &out.bytes).map_err(|e| IoError::io(path, e))?;
    Ok(out.warnings)
}

/// Like [`write`], with the project name and the time stamp given (the time is a Julian day number).
pub fn write_named(project: &Project, name: &str, time: f64) -> Result<OpjWrite, IoError> {
    let mut w = Writer::new(time);
    w.project(project, name)?;
    Ok(w.finish())
}

/// The current time as a Julian day number, the unit of the time stamps in the file.
pub fn julian_day_now() -> f64 {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
    secs / 86_400.0 + 2_440_587.5
}

fn fail<T>(reason: impl Into<String>) -> Result<T, IoError> {
    Err(IoError::OpjWrite { reason: reason.into() })
}

/// Longest data-set name, `Window_Column@n`, in bytes (the field holds 25 with its terminator).
const MAX_DATASET_NAME: usize = 24;
/// Longest sheet name in bytes.
const MAX_SHEET_NAME: usize = layout::SHEET_NAME_WIDTH - 1;
/// Longest text of one cell in bytes: the cell size is stored in one byte and holds the text, two
/// header bytes and a terminator.
const MAX_CELL_TEXT: usize = 252;
/// The column record points to its data set with 16 bits.
const MAX_COLUMNS: usize = u16::MAX as usize;
/// The first number given to a column or a formula that takes part in a recalculation (the first
/// that Origin gave in the files examined). The column record keeps its number in 16 bits.
const OBJECT_NUMBER_START: u32 = 0x4000;
/// The longest formula text that is stored, in bytes (the reader keeps blocks up to 64 KiB).
const MAX_FORMULA_TEXT: usize = 60_000;
/// Deepest nesting of folders that is written.
const MAX_FOLDER_DEPTH: usize = 32;
/// Longest folder name in bytes.
const MAX_FOLDER_NAME: usize = 100;

/// Where the data set of a column is in the file, for the curves of graphs.
#[derive(Clone, Copy, Debug)]
struct SetInfo {
    /// 1-based position in the data-set list.
    position: u16,
    /// Position of the first value and after the last one (0 and 0 for an empty column).
    first: u32,
    last: u32,
}

/// The range of cells that hold a value, as `dataset()` stores it.
fn used_range(column: &Column) -> (u32, u32) {
    let used: Vec<usize> = match &column.data {
        ColumnData::Numeric(v) => v.iter().enumerate().filter(|(_, x)| !x.is_nan()).map(|(i, _)| i).collect(),
        ColumnData::Text(v) => v.iter().enumerate().filter(|(_, t)| !t.is_empty()).map(|(i, _)| i).collect(),
    };
    match (used.first(), used.last()) {
        (Some(f), Some(l)) => (*f as u32, *l as u32 + 1),
        _ => (0, 0),
    }
}

#[derive(Default)]
struct Notes {
    lossy_texts: usize,
    flattened_breaks: usize,
    /// Formulas whose text could not be stored.
    formulas_dropped: usize,
    /// Formulas that should recalculate but are stored with recalculate mode None.
    formula_modes_dropped: usize,
    /// Formulas whose state (out of date, broken) is not stored.
    formula_states_dropped: usize,
    demoted_labels: usize,
    /// Everything a graph holds that the file does not, one sentence each.
    graph_notes: Vec<String>,
}

/// What of the formulas of one sheet goes into the file.
struct SheetFormulas {
    /// The stored text of the formula of each column, if it can be stored.
    texts: Vec<Option<Vec<u8>>>,
    /// The number of each column in the recalculation records; 0 for a column outside them.
    object_ids: Vec<u16>,
}

struct Writer {
    time: f64,
    datasets: Vec<u8>,
    windows: Vec<u8>,
    /// Data sets written so far.
    columns: usize,
    /// Sheets written so far, over all workbooks.
    sheets: usize,
    written_windows: usize,
    /// Graph windows written so far.
    graphs: usize,
    /// A counter that Origin gives to the objects of a graph (curves, text).
    object_serial: u32,
    notes: Notes,
    /// The formulas that recalculate, in the order of their columns.
    dependencies: Vec<Dependency>,
    /// The next number to give to a column or a formula that takes part in a recalculation.
    next_object: u32,
    /// The folder tree, with the windows of every folder as positions in the window list.
    tree: FolderOut,
}

impl Writer {
    fn new(time: f64) -> Self {
        Writer {
            time,
            datasets: Vec::new(),
            windows: Vec::new(),
            columns: 0,
            sheets: 0,
            written_windows: 0,
            graphs: 0,
            object_serial: 0x0325,
            notes: Notes::default(),
            dependencies: Vec::new(),
            next_object: OBJECT_NUMBER_START,
            tree: FolderOut::default(),
        }
    }

    fn project(&mut self, project: &Project, name: &str) -> Result<(), IoError> {
        self.check_text("project name", name)?;
        self.tree = self.folder_tree(project, name)?;
        let mut seen: HashSet<String> = HashSet::new();
        // the data-set position of every column, as the workbooks will number them
        let mut sets: HashMap<ColumnId, SetInfo> = HashMap::new();
        let mut next = 0usize;
        for book in project.workbooks() {
            for column in book.sheets.iter().flat_map(|s| s.columns.iter()) {
                next += 1;
                let (first, last) = used_range(column);
                sets.insert(column.id, SetInfo { position: next.min(usize::from(u16::MAX)) as u16, first, last });
            }
        }
        for window in &project.windows {
            if !seen.insert(window.name().to_lowercase()) {
                return fail(format!("two windows are named '{}'; Origin needs distinct window names", window.name()));
            }
            self.window(project, window, &sets)?;
        }
        Ok(())
    }

    /// Writes one window: a workbook or a graph.
    fn window(&mut self, project: &Project, window: &Window, sets: &HashMap<ColumnId, SetInfo>) -> Result<(), IoError> {
        match window {
            Window::Workbook(book) => self.workbook(book),
            Window::Graph(graph) => self.graph(project, graph, sets),
        }
    }

    fn workbook(&mut self, book: &Workbook) -> Result<(), IoError> {
        self.check_name("workbook name", &book.name, MAX_DATASET_NAME)?;
        let label = self.label_text(&book.long_name, "workbook long name")?;
        let spec = WindowSpec {
            name: &book.name,
            label: &label,
            ordinal: self.written_windows,
            active_sheet: book.active_sheet.min(book.sheets.len().saturating_sub(1)),
            time: self.time,
        };
        self.windows.extend(framed(&layout::window_header(&spec)));
        let mut sheet_names: HashSet<String> = HashSet::new();
        for (index, sheet) in book.sheets.iter().enumerate() {
            self.check_name("sheet name", &sheet.name, MAX_SHEET_NAME)?;
            if !sheet_names.insert(sheet.name.to_lowercase()) {
                return fail(format!("workbook '{}' has two sheets named '{}'", book.name, sheet.name));
            }
            self.sheet(book, index, sheet)?;
        }
        // the end of the layer list
        self.windows.extend(framed(&[]));
        self.written_windows += 1;
        Ok(())
    }

    fn sheet(&mut self, book: &Workbook, index: usize, sheet: &Worksheet) -> Result<(), IoError> {
        let layer =
            LayerSpec { name: &sheet.name, columns: sheet.columns.len(), index_in_book: index, ordinal: self.sheets };
        self.sheets += 1;
        self.windows.extend(framed(&layout::layer_header(&layer)));
        // The only annotations of a written sheet hold the formulas of columns, the last column
        // first as Origin lists them; then the empty block that ends the list.
        let formulas = self.formula_plans(sheet);
        for (i, text) in formulas.texts.iter().enumerate().rev() {
            if let Some(text) = text {
                self.windows.extend(tail::formula_annotation(&sheet.columns[i].short_name, text));
            }
        }
        self.windows.extend(framed(&[]));
        // Origin stores every column of a sheet with the same number of cells, 32 for a new sheet
        let longest = sheet.columns.iter().map(Column::len).max().unwrap_or(0);
        let rows = if longest == 0 { 32 } else { longest };
        let first_dataset = self.columns;
        let suffix = if index == 0 { String::new() } else { format!("@{}", index + 1) };
        let mut names: HashSet<String> = HashSet::new();
        let mut records = Vec::new();
        // Origin 2016 opens a label column as unused when the sheet has no Y column, so write it so
        let has_y = sheet.columns.iter().any(|c| c.designation == Designation::Y);
        let roles: Vec<Designation> = sheet
            .columns
            .iter()
            .map(|c| {
                if c.designation == Designation::Label && !has_y {
                    self.notes.demoted_labels += 1;
                    Designation::Disregard
                } else {
                    c.designation
                }
            })
            .collect();
        for (i, column) in sheet.columns.iter().enumerate() {
            let dataset_name = format!("{}_{}{}", book.name, column.short_name, suffix);
            self.check_name("column name", &column.short_name, MAX_DATASET_NAME)?;
            if dataset_name.len() > MAX_DATASET_NAME {
                return fail(format!(
                    "the name of column '{}' in workbook '{}' is too long: Origin keeps at most {MAX_DATASET_NAME} \
                     bytes for the workbook name, an underscore, the column name and the sheet number",
                    column.short_name, book.name
                ));
            }
            if !names.insert(column.short_name.to_lowercase()) {
                return fail(format!(
                    "sheet '{}' of workbook '{}' has two columns named '{}'",
                    sheet.name, book.name, column.short_name
                ));
            }
            if self.columns >= MAX_COLUMNS {
                return fail(format!("a project file holds at most {MAX_COLUMNS} columns"));
            }
            let (block, used) =
                self.dataset(&dataset_name, column, roles[i], rows, parent_of(&roles, i, first_dataset))?;
            self.datasets.extend(block);
            let spec = ColumnSpec {
                dataset: (self.columns + 1) as u16,
                designation: designation_code(roles[i]),
                short_name: &column.short_name,
                type_code: type_code(roles[i]),
                first: used.0,
                last: used.1,
                parent: parent_of(&roles, i, first_dataset),
                index: i,
                object_id: formulas.object_ids[i],
            };
            records.extend(framed(&layout::column_record(&spec)));
            let long = self.label_text(&column.long_name, "long name")?;
            let units = self.label_text(&column.units, "units")?;
            let comments = self.comment_text(&column.comments)?;
            records.extend(framed(&layout::column_note(&long, &units, &comments)));
            self.columns += 1;
        }
        self.windows.extend(records);
        // the end of the column records, the extra block, the three (empty) groups of axis blocks
        for _ in 0..5 {
            self.windows.extend(framed(&[]));
        }
        Ok(())
    }

    /// The three blocks of one data set (header, values, mask) and the used range of the column.
    fn dataset(
        &mut self,
        name: &str,
        column: &Column,
        role: Designation,
        rows: usize,
        parent: u16,
    ) -> Result<(Vec<u8>, (u32, u32)), IoError> {
        let (cell_size, data, used) = match &column.data {
            ColumnData::Numeric(values) => {
                let mut data = Vec::with_capacity(rows * 10);
                let mut used: Option<(usize, usize)> = None;
                for row in 0..rows {
                    let x = values.get(row).copied().filter(|x| !x.is_nan());
                    if x.is_some() {
                        used = Some(used.map_or((row, row + 1), |(f, _)| (f, row + 1)));
                    }
                    data.extend_from_slice(&[0, 0]);
                    data.extend_from_slice(&x.unwrap_or(MISSING).to_le_bytes());
                }
                (10u8, data, used)
            }
            ColumnData::Text(texts) => {
                let mut encoded = Vec::with_capacity(rows);
                for row in 0..rows {
                    let text = texts.get(row).map_or("", String::as_str);
                    let (bytes, lossy) = encode_cp1252(text);
                    self.notes.lossy_texts += usize::from(lossy);
                    if bytes.contains(&0) {
                        return fail(format!("a cell of column '{}' contains a NUL character", column.short_name));
                    }
                    if bytes.len() > MAX_CELL_TEXT {
                        return fail(format!(
                            "a cell of column '{}' holds {} bytes of text; Origin cells hold at most {MAX_CELL_TEXT}",
                            column.short_name,
                            bytes.len()
                        ));
                    }
                    encoded.push(bytes);
                }
                let longest = encoded.iter().map(Vec::len).max().unwrap_or(0);
                let size = (longest + 3).max(10);
                let mut data = Vec::with_capacity(rows * size);
                let mut used: Option<(usize, usize)> = None;
                for (row, bytes) in encoded.iter().enumerate() {
                    if bytes.is_empty() {
                        // an empty cell is a missing number
                        data.extend_from_slice(&[0, 0]);
                        data.extend_from_slice(&MISSING.to_le_bytes());
                        data.resize(data.len() + size - 10, 0);
                    } else {
                        used = Some(used.map_or((row, row + 1), |(f, _)| (f, row + 1)));
                        data.extend_from_slice(&[1, 0]);
                        data.extend_from_slice(bytes);
                        data.resize(data.len() + size - 2 - bytes.len(), 0);
                    }
                }
                (size as u8, data, used)
            }
        };
        let (first, last) = used.map_or((0, 0), |(f, l)| (f as u32, l as u32));
        let spec = DatasetSpec {
            name,
            type_code: type_code(role),
            cell_size,
            rows: rows as u32,
            first,
            last,
            parent: if role == Designation::Disregard { 0 } else { parent.min(255) as u8 },
        };
        let mut block = framed(&layout::dataset_header(&spec));
        block.extend(framed(&data));
        block.extend(framed(&[]));
        Ok((block, (first, last)))
    }

    /// Decides what of the formulas of one sheet goes into the file: the text of every formula that
    /// can be stored, and for the formulas that recalculate the numbers that tie them to the
    /// columns they read (spec section 13.10). Records the recalculation for the tail.
    fn formula_plans(&mut self, sheet: &Worksheet) -> SheetFormulas {
        let n = sheet.columns.len();
        let mut out = SheetFormulas { texts: vec![None; n], object_ids: vec![0; n] };
        for (i, column) in sheet.columns.iter().enumerate() {
            let Some(formula) = column.formula.as_deref().filter(|f| !f.trim().is_empty()) else { continue };
            let text = formulas::stored_text(formula, column.formula_rows.as_ref());
            let (bytes, lossy) = encode_cp1252(&text);
            if bytes.contains(&0)
                || bytes.len() > MAX_FORMULA_TEXT
                || text.contains("@${")
                || formula.contains(formulas::TAIL_MARK)
            {
                self.notes.formulas_dropped += 1;
                continue;
            }
            self.notes.lossy_texts += usize::from(lossy);
            out.texts[i] = Some(bytes);
            if column.recalc == RecalcMode::None {
                continue;
            }
            self.notes.formula_states_dropped += usize::from(column.formula_state != FormulaState::Current);
            // A formula recalculates only through the columns it reads: one that reads none, reads
            // itself, reads a column that is not there or cannot be parsed is stored as plain.
            let inputs = formulas::inputs_of(sheet, formula).filter(|v| !v.is_empty() && !v.contains(&i));
            let Some(inputs) = inputs else {
                self.notes.formula_modes_dropped += 1;
                continue;
            };
            let fresh = inputs.iter().chain(std::iter::once(&i)).filter(|&&j| out.object_ids[j] == 0).count();
            if self.next_object as usize + fresh >= usize::from(u16::MAX) {
                self.notes.formula_modes_dropped += 1;
                continue;
            }
            let id = self.next_object;
            self.next_object += 1;
            for &j in inputs.iter().chain(std::iter::once(&i)) {
                if out.object_ids[j] == 0 {
                    out.object_ids[j] = self.next_object as u16;
                    self.next_object += 1;
                }
            }
            self.dependencies.push(Dependency {
                inputs: inputs.iter().map(|&j| u32::from(out.object_ids[j])).collect(),
                output: u32::from(out.object_ids[i]),
                auto: column.recalc == RecalcMode::Auto,
                id,
            });
        }
        out
    }

    /// The folder tree of the file: the folders of the project with the position of each window in
    /// the window list. Windows that no folder lists are in the root folder, as are those the root
    /// lists; a window listed by several folders stays in the first one.
    fn folder_tree(&self, project: &Project, root_name: &str) -> Result<FolderOut, IoError> {
        let position: HashMap<WindowId, u32> =
            project.windows.iter().enumerate().map(|(i, w)| (w.id(), i as u32)).collect();
        let mut claimed: HashSet<WindowId> = HashSet::new();
        let mut folders = Vec::new();
        let mut names: HashSet<String> = HashSet::new();
        for child in &project.root.folders {
            folders.push(self.folder_out(child, 1, &position, &mut claimed, &mut names, "")?);
        }
        let windows =
            project.windows.iter().enumerate().filter(|(_, w)| !claimed.contains(&w.id())).map(|(i, _)| i as u32);
        Ok(FolderOut { name: root_name.to_string(), windows: windows.collect(), folders })
    }

    fn folder_out(
        &self,
        folder: &Folder,
        depth: usize,
        position: &HashMap<WindowId, u32>,
        claimed: &mut HashSet<WindowId>,
        siblings: &mut HashSet<String>,
        parent: &str,
    ) -> Result<FolderOut, IoError> {
        let path = format!("{parent}/{}", folder.name);
        self.check_folder_name(&folder.name, &path)?;
        if !siblings.insert(folder.name.to_lowercase()) {
            return fail(format!(
                "two folders in '{}' are named '{}'; Origin needs distinct folder names",
                if parent.is_empty() { "/" } else { parent },
                folder.name
            ));
        }
        if depth > MAX_FOLDER_DEPTH {
            return fail(format!("the folder '{path}' is nested more than {MAX_FOLDER_DEPTH} levels deep"));
        }
        let mut windows: Vec<u32> = folder
            .windows
            .iter()
            .filter(|id| claimed.insert(**id))
            .filter_map(|id| position.get(id).copied())
            .collect();
        windows.sort_unstable();
        let mut names: HashSet<String> = HashSet::new();
        let mut folders = Vec::new();
        for child in &folder.folders {
            folders.push(self.folder_out(child, depth + 1, position, claimed, &mut names, &path)?);
        }
        Ok(FolderOut { name: folder.name.clone(), windows, folders })
    }

    /// A folder name that Origin can store: not empty, no control characters, nothing outside
    /// Windows-1252, no slash or backslash (which separate the folders of a path).
    fn check_folder_name(&self, name: &str, path: &str) -> Result<(), IoError> {
        if name.trim().is_empty() {
            return fail(format!("a folder inside '{}' has no name", path.rsplit_once('/').map_or("", |(p, _)| p)));
        }
        if encode_cp1252(name).1 {
            return fail(format!(
                "the folder '{path}' has characters that Origin cannot store in a name (Windows-1252 only)"
            ));
        }
        if name.chars().any(|c| c.is_control()) {
            return fail(format!("the folder '{path}' has a control character in its name"));
        }
        if name.contains(['/', '\\']) {
            return fail(format!(
                "the folder name '{name}' contains a slash or backslash, which Origin uses between folders"
            ));
        }
        if name.len() > MAX_FOLDER_NAME {
            return fail(format!(
                "the folder name '{name}' is longer than the {MAX_FOLDER_NAME} bytes that Origin stores"
            ));
        }
        Ok(())
    }

    /// A name that Origin can store: not empty, no control characters, nothing outside Windows-1252, no
    /// `@` (which separates the sheet number in a data-set name), at most `max` bytes.
    fn check_name(&self, what: &str, name: &str, max: usize) -> Result<(), IoError> {
        if name.is_empty() {
            return fail(format!("a {what} is empty"));
        }
        let (bytes, lossy) = encode_cp1252(name);
        if lossy {
            return fail(format!(
                "the {what} '{name}' has characters that Origin cannot store in a name (Windows-1252 only)"
            ));
        }
        if bytes.len() > max {
            return fail(format!("the {what} '{name}' is longer than the {max} bytes that Origin stores"));
        }
        if name.chars().any(|c| c.is_control()) {
            return fail(format!("the {what} '{name}' contains a control character"));
        }
        if name.contains('@') {
            return fail(format!("the {what} '{name}' contains '@', which Origin uses inside data-set names"));
        }
        Ok(())
    }

    fn check_text(&self, what: &str, text: &str) -> Result<(), IoError> {
        if text.chars().any(|c| c.is_control()) {
            return fail(format!("the {what} '{text}' contains a control character"));
        }
        Ok(())
    }

    /// Text for a long name or units: one line, nothing that Origin would read as storage.
    fn label_text(&mut self, text: &str, what: &str) -> Result<String, IoError> {
        if text.contains("@${") {
            return fail(format!(
                "the {what} '{text}' contains '@${{', which Origin reads as the start of stored data"
            ));
        }
        let mut out = text.to_string();
        if out.contains(['\r', '\n']) {
            self.notes.flattened_breaks += 1;
            out = out.replace("\r\n", " ").replace(['\r', '\n'], " ");
        }
        self.count_lossy(&out);
        Ok(out)
    }

    /// Text for comments: line breaks stay (Origin writes them as CR LF).
    fn comment_text(&mut self, text: &str) -> Result<String, IoError> {
        if text.contains("@${") {
            return fail(format!(
                "the comment '{text}' contains '@${{', which Origin reads as the start of stored data"
            ));
        }
        self.count_lossy(text);
        Ok(text.replace("\r\n", "\n").replace('\n', "\r\n"))
    }

    fn count_lossy(&mut self, text: &str) {
        self.notes.lossy_texts += usize::from(encode_cp1252(text).1);
    }

    fn finish(self) -> OpjWrite {
        let mut bytes = layout::SIGNATURE.to_vec();
        bytes.extend(framed(&layout::header()));
        bytes.extend(framed(&[]));
        bytes.extend(self.datasets);
        bytes.extend(framed(&[]));
        bytes.extend(self.windows);
        bytes.extend(framed(&[]));
        bytes.extend(tail::tail(&TailSpec { root: &self.tree, dependencies: &self.dependencies, time: self.time }));
        let n = &self.notes;
        let mut warnings = Vec::new();
        warnings.extend(n.graph_notes.iter().cloned());
        if n.lossy_texts > 0 {
            warnings.push(format!(
                "{} text(s) had characters outside Windows-1252, which an Origin 2016 project cannot store; they were written as '?'.",
                n.lossy_texts
            ));
        }
        if n.flattened_breaks > 0 {
            warnings.push(format!(
                "{} long name(s) or unit(s) had line breaks; they were replaced by spaces.",
                n.flattened_breaks
            ));
        }
        if n.demoted_labels > 0 {
            warnings.push(format!(
                "{} label column(s) are in sheets without a Y column; Origin opens those as unused, so they were written as unused.",
                n.demoted_labels
            ));
        }
        if n.formulas_dropped > 0 {
            warnings.push(format!(
                "{} column formula(s) could not be stored in the file (text too long, with a NUL or an Origin storage mark); only their values were written.",
                n.formulas_dropped
            ));
        }
        if n.formula_modes_dropped > 0 {
            warnings.push(format!(
                "{} column formula(s) that recalculate read no column of their sheet, read themselves or could not be understood; the formula text was written with recalculate mode None.",
                n.formula_modes_dropped
            ));
        }
        if n.formula_states_dropped > 0 {
            warnings.push(format!(
                "{} recalculating column(s) were out of date or broken; the file does not store that state, so they read as up to date.",
                n.formula_states_dropped
            ));
        }
        OpjWrite { bytes, warnings }
    }
}

/// The role code of a column (spec section 4.3).
fn designation_code(d: Designation) -> u8 {
    match d {
        Designation::X => 3,
        Designation::Y => 0,
        Designation::Z => 5,
        Designation::YError => 2,
        Designation::XError => 6,
        Designation::Label => 4,
        Designation::Disregard => 1,
    }
}

/// The type word of a column: the high byte tells X (0x51), Y (0x61) and unused (0x41) columns apart,
/// the low byte the error (0x30), Z and label (0x20) and ordinary (0x21) columns.
fn type_code(d: Designation) -> u16 {
    match d {
        Designation::X => 0x5121,
        Designation::Y => 0x6121,
        Designation::Z | Designation::Label => 0x6120,
        Designation::YError | Designation::XError => 0x6130,
        Designation::Disregard => 0x4121,
    }
}

/// The 1-based data-set position of the column that column `i` belongs to: a Y column to the nearest X
/// column on its left, an error, Z or label column to the nearest Y column on its left, an unused column
/// to the nearest X; 0 when there is none. `first` is the number of data sets before the sheet.
fn parent_of(roles: &[Designation], i: usize, first: usize) -> u16 {
    let wanted: &[Designation] = match roles[i] {
        Designation::X => return 0,
        Designation::Y | Designation::Disregard => &[Designation::X],
        Designation::YError | Designation::XError | Designation::Z | Designation::Label => &[Designation::Y],
    };
    roles[..i].iter().rposition(|c| wanted.contains(c)).map_or(0, |p| (first + p + 1) as u16)
}
