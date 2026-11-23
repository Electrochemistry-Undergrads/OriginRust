//! Writer of Origin project files (`.opj`, the format Origin / version 9.3 saves).
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

/// Writes `project` as an Origin project, named after the project's root folder.
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
    // Notes windows (T154) are not written yet: the file gets the project without them.
    // Notes sheets (T188) are not written either: a workbook keeps its other sheets (or one empty one).
    let notes: Vec<WindowId> = project.notes().map(|n| n.id).collect();
    let left_out = |s: &sheaf_core::Worksheet| s.is_notes() || s.graph_sheet.is_some();
    let notes_sheets: usize = project.workbooks().map(|b| b.sheets.iter().filter(|s| left_out(s)).count()).sum();
    let without;
    let project = if notes.is_empty() && notes_sheets == 0 {
        project
    } else {
        let mut p = project.clone();
        for id in &notes {
            p.remove_window(*id);
        }
        let books: Vec<WindowId> = p.workbooks().map(|b| b.id).collect();
        for id in books {
            if let Some(b) = p.workbook_mut(id) {
                if b.sheets.iter().all(left_out) {
                    b.sheets.truncate(1);
                    if let Some(s) = b.sheets.first_mut() {
                        s.notes_text = None;
                        s.graph_sheet = None;
                    }
                } else {
                    b.sheets.retain(|s| !left_out(s));
                }
                b.active_sheet = b.active_sheet.min(b.sheets.len().saturating_sub(1));
            }
        }
        without = p;
        &without
    };
    let mut w = Writer::new(time);
    w.project(project, name)?;
    let mut out = w.finish();
    if !notes.is_empty() {
        out.warnings.push(format!(
            "{} Notes window(s) were left out: this program does not write Notes windows to an Origin project yet.",
            notes.len()
        ));
    }
    if notes_sheets > 0 {
        out.warnings.push(format!(
            "{notes_sheets} Notes or graph sheet(s) were left out: this program does not write Notes sheets or graph sheets to an Origin project yet."
        ));
    }
    Ok(out)
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
        ColumnData::Mixed { numbers, texts } => {
            let mut rows: Vec<usize> =
                numbers.iter().enumerate().filter(|(_, x)| !x.is_nan()).map(|(i, _)| i).collect();
            rows.extend(texts.keys().copied().filter(|r| numbers.get(*r).is_none_or(|x| x.is_nan())));
            rows.sort_unstable();
            rows
        }
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
    /// Columns with user parameter rows that are not stored.
    params_dropped: usize,
    /// Columns with a display format other than General that is not stored.
    formats_dropped: usize,
    /// Hidden columns and columns with their own width: the file written here holds neither.
    hidden_dropped: usize,
    /// Columns with a sampling interval (Set Sampling Interval), which the file written here does
    /// not hold: Origin plots them against their X column or the row number.
    sampling_dropped: usize,
    /// Columns with a data range (Set As Begin / Set As End), which the file written here does not hold.
    ranges_dropped: usize,
    /// Columns whose formula has Before Formula Scripts (T307), which the file written here does not hold.
    scripts_dropped: usize,
    /// Sheets with a worksheet filter: the file written here holds no filter.
    filters_dropped: usize,
    /// Sheets with a long name or a tab showing it (T90), which the file written here does not hold.
    sheet_names_dropped: usize,
    /// Sheets with comments, which the file written here does not hold.
    sheet_comments_dropped: usize,
    /// Sheets marked Exclude from Plotting dialogs (T184), which the file written here does not hold.
    plot_exclusions_dropped: usize,
    /// Sheets with hidden label rows, row labels or grid lines (T92), which the file written here does not hold.
    sheet_views_dropped: usize,
    /// Sheets with frozen panes: the file written here holds none.
    frozen_dropped: usize,
    /// Sheets with conditional formatting rules: the file written here holds none.
    cond_formats_dropped: usize,
    /// Sheets with cell formats (bold, italic, underline, font, size, colours, alignment): the
    /// file written here holds none.
    cell_formats_dropped: usize,
    /// Cell notes: the file written here holds none (counted per note).
    cell_notes_dropped: usize,
    /// Analysis results that recalculate from their source (Recalculate Manual or Auto), not stored.
    analysis_links_dropped: usize,
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
    /// Where the window being written stood in the workspace, if the project knows.
    place: Option<sheaf_core::project::WindowPlace>,
    /// The data set being written is a Numeric-format column (8-byte cells, T337).
    numeric_cells: bool,
    /// The Comments of the window being written (T283).
    comments: String,
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
            place: None,
            numeric_cells: false,
            comments: String::new(),
        }
    }

    fn project(&mut self, project: &Project, name: &str) -> Result<(), IoError> {
        self.check_text("project name", name)?;
        self.notes.analysis_links_dropped = project.analysis_links.len();
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
            self.place = project.window_places.iter().find(|p| p.window == window.id()).copied();
            self.comments = project.window_comments(window.id()).to_string();
            self.window(project, window, &sets)?;
        }
        Ok(())
    }

    /// Writes one window: a workbook or a graph.
    fn window(&mut self, project: &Project, window: &Window, sets: &HashMap<ColumnId, SetInfo>) -> Result<(), IoError> {
        match window {
            Window::Workbook(book) => self.workbook(book),
            Window::Graph(graph) => self.graph(project, graph, sets),
            // Left out by `write_named`.
            Window::Notes(_) => Ok(()),
            Window::Layout(l) => {
                // Layout pages are not written yet (the companion file keeps them).
                self.notes.graph_notes.push(format!(
                    "layout page '{}' is not written to the Origin project (the writer does not store layout pages yet)",
                    l.name
                ));
                Ok(())
            }
            Window::Matrix(m) => {
                // Matrix windows are not written yet (the companion file keeps them).
                self.notes.graph_notes.push(format!(
                    "matrix book '{}' is not written to the Origin project (the writer does not store matrix windows yet)",
                    m.name
                ));
                Ok(())
            }
        }
    }

    fn workbook(&mut self, book: &Workbook) -> Result<(), IoError> {
        self.check_name("workbook name", &book.name, MAX_DATASET_NAME)?;
        let label = self.window_label(&book.long_name, "workbook long name")?;
        let spec = WindowSpec {
            name: &book.name,
            label: &label,
            ordinal: self.written_windows,
            active_sheet: book.active_sheet.min(book.sheets.len().saturating_sub(1)),
            time: self.time,
            place: self.place,
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
        // T343: the sheet's long name, as Origin 2016 stores it: an annotation named "__LONGNAME"
        // with the same header as a formula annotation and the text in its first block.
        let long = sheet.long_name.trim();
        if !long.is_empty() {
            let text = self.label_text(long, "sheet long name")?;
            self.windows.extend(tail::formula_annotation("__LONGNAME", &encode_cp1252(&text).0));
        }
        self.windows.extend(framed(&[]));
        // Origin stores every column of a sheet with the same number of cells, 32 for a new sheet
        let longest = sheet.columns.iter().map(Column::len).max().unwrap_or(0);
        let rows = if longest == 0 { 32 } else { longest };
        let first_dataset = self.columns;
        let suffix = if index == 0 { String::new() } else { format!("@{}", index + 1) };
        let mut names: HashSet<String> = HashSet::new();
        let mut records = Vec::new();
        // Origin opens a label column as unused when the sheet has no Y column, so write it so
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
        self.notes.filters_dropped += usize::from(sheet.filter.is_some());
        // T343: the long name is written; only a tab that shows another name is lost.
        self.notes.sheet_names_dropped += usize::from(!sheet.tab_label.is_default());
        self.notes.sheet_comments_dropped += usize::from(!sheet.comments.is_empty());
        self.notes.plot_exclusions_dropped += usize::from(sheet.exclude_from_plotting);
        self.notes.sheet_views_dropped += usize::from(!sheet.view.is_default());
        self.notes.frozen_dropped += usize::from(!sheet.frozen.is_none());
        self.notes.cond_formats_dropped += usize::from(!sheet.cond_formats.is_empty());
        self.notes.cell_formats_dropped += usize::from(
            !sheet.cell_formats.is_empty() || !sheet.font_defaults.is_empty() || !sheet.data_colors.is_empty(),
        );
        self.notes.cell_notes_dropped += sheet.notes.len();
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
            // T303: a column with a sampling interval belongs to no X column, as Origin writes it.
            let sampling = column
                .sampling
                .as_ref()
                .filter(|s| roles[i] != Designation::X && s.x0.is_finite() && s.inc.is_finite() && s.inc != 0.0);
            let parent = if sampling.is_some() { 0 } else { parent_of(&roles, i, first_dataset) };
            // T337: an X or Y column of numbers with a numeric display is written as Origin's Numeric
            // format (8-byte cells, type word ..01) with the display in its column record.
            let display = numeric_display(column.format)
                .filter(|_| matches!(roles[i], Designation::X | Designation::Y))
                .filter(|_| matches!(column.data, ColumnData::Numeric(_)));
            self.numeric_cells = display.is_some();
            let (block, used) = self.dataset(
                &dataset_name,
                column,
                roles[i],
                rows,
                parent,
                sampling.map_or((1.0, 1.0), |s| (s.x0, s.inc)),
            )?;
            self.datasets.extend(block);
            let spec = ColumnSpec {
                dataset: (self.columns + 1) as u16,
                designation: designation_code(roles[i]),
                short_name: &column.short_name,
                type_code: if display.is_some() {
                    (type_code(roles[i]) & 0xF000) | 0x0001
                } else {
                    type_code(roles[i])
                },
                first: used.0,
                last: used.1,
                parent,
                index: i,
                width: column.width,
                display,
                object_id: formulas.object_ids[i],
                sampling: sampling.map(|s| (s.x0, s.inc, parent_of(&roles, i, first_dataset) != 0)),
            };
            records.extend(framed(&layout::column_record(&spec)));
            self.notes.params_dropped += usize::from(!column.params.is_empty());
            self.notes.formats_dropped +=
                usize::from(column.format != sheaf_core::format::NumberFormat::General && display.is_none());
            self.notes.hidden_dropped += usize::from(column.hidden);
            // T303: the interval is written; only its X name and units are not.
            self.notes.sampling_dropped +=
                usize::from(column.sampling.as_ref().is_some_and(|s| !s.long_name.is_empty() || !s.units.is_empty()));
            self.notes.ranges_dropped += usize::from(column.data_range.is_some());
            self.notes.scripts_dropped += usize::from(column.before_formula.is_some());
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
        sampling: (f64, f64),
    ) -> Result<(Vec<u8>, (u32, u32)), IoError> {
        // Every cell is a number, a text or missing. A Text & Numeric column holds all three.
        let number_at = |row: usize| -> Option<f64> {
            match &column.data {
                ColumnData::Numeric(v) | ColumnData::Mixed { numbers: v, .. } => {
                    v.get(row).copied().filter(|x| !x.is_nan())
                }
                ColumnData::Text(_) => None,
            }
        };
        if self.numeric_cells {
            // T337: Origin's Numeric format: one 8-byte double per row, the missing value for an empty cell.
            let mut data = Vec::with_capacity(rows * 8);
            let mut used: Option<(usize, usize)> = None;
            for row in 0..rows {
                let x = number_at(row);
                if x.is_some() {
                    used = Some(used.map_or((row, row + 1), |(f, _)| (f, row + 1)));
                }
                data.extend_from_slice(&x.unwrap_or(MISSING).to_le_bytes());
            }
            let (first, last) = used.map_or((0, 0), |(f, l)| (f as u32, l as u32));
            let spec = DatasetSpec {
                name,
                type_code: (type_code(role) & 0xF000) | 0x0001,
                cell_size: 8,
                rows: rows as u32,
                first,
                last,
                parent: if role == Designation::Disregard { 0 } else { parent.min(255) as u8 },
                sampling,
            };
            let mut block = framed(&layout::dataset_header(&spec));
            block.extend(framed(&data));
            block.extend(framed(&mask_block(column, rows)));
            return Ok((block, (first, last)));
        }
        let mut encoded: Vec<Option<Vec<u8>>> = Vec::with_capacity(rows);
        for row in 0..rows {
            // A number written over a text wins, as in the worksheet.
            let text = if number_at(row).is_some() { None } else { column.data.text_cell(row) };
            let Some(text) = text else {
                encoded.push(None);
                continue;
            };
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
            encoded.push(Some(bytes));
        }
        let longest = encoded.iter().flatten().map(Vec::len).max().unwrap_or(0);
        let size = (longest + 3).max(10);
        let mut data = Vec::with_capacity(rows * size);
        let mut used: Option<(usize, usize)> = None;
        for (row, bytes) in encoded.iter().enumerate() {
            match bytes {
                Some(bytes) => {
                    used = Some(used.map_or((row, row + 1), |(f, _)| (f, row + 1)));
                    data.extend_from_slice(&[1, 0]);
                    data.extend_from_slice(bytes);
                    data.resize(data.len() + size - 2 - bytes.len(), 0);
                }
                None => {
                    // a number, or a missing number for an empty cell
                    let x = number_at(row);
                    if x.is_some() {
                        used = Some(used.map_or((row, row + 1), |(f, _)| (f, row + 1)));
                    }
                    data.extend_from_slice(&[0, 0]);
                    data.extend_from_slice(&x.unwrap_or(MISSING).to_le_bytes());
                    data.resize(data.len() + size - 10, 0);
                }
            }
        }
        let cell_size = size as u8;
        let (first, last) = used.map_or((0, 0), |(f, l)| (f as u32, l as u32));
        let spec = DatasetSpec {
            name,
            type_code: type_code(role),
            cell_size,
            rows: rows as u32,
            first,
            last,
            parent: if role == Designation::Disregard { 0 } else { parent.min(255) as u8 },
            sampling,
        };
        let mut block = framed(&layout::dataset_header(&spec));
        block.extend(framed(&data));
        block.extend(framed(&mask_block(column, rows)));
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

    /// The label of a window: its long name, then CR LF and its Comments when it has any, as Origin
    /// 2016 writes them (T283, measured).
    fn window_label(&mut self, long_name: &str, what: &str) -> Result<String, IoError> {
        let mut label = self.label_text(long_name, what)?;
        if !self.comments.is_empty() {
            let comments = std::mem::take(&mut self.comments);
            let text = self.comment_text(&comments)?;
            label.push_str("\r\n");
            label.push_str(&text);
            self.comments = comments;
        }
        Ok(label)
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
                "{} text(s) had characters outside Windows-1252, which an Origin project cannot store; they were written as '?'.",
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
        if n.params_dropped > 0 {
            warnings.push(format!(
                "{} column(s) have user parameter rows, which the project file written here does not hold.",
                n.params_dropped
            ));
        }
        if n.formats_dropped > 0 {
            warnings.push(format!(
                "{} column(s) have a display format (decimals, scientific, date or time) that the project file written here does not hold; Origin shows them in its default format.",
                n.formats_dropped
            ));
        }
        if n.scripts_dropped > 0 {
            warnings.push(format!(
                "{} column formula(s) use Before Formula Scripts, which the project file written here does not hold; Origin keeps the values but its formula misses the script's names.",
                n.scripts_dropped
            ));
        }
        if n.hidden_dropped > 0 {
            warnings.push(format!(
                "{} hidden column(s): the project file written here does not hold the hidden state, so Origin shows them.",
                n.hidden_dropped
            ));
        }
        if n.sampling_dropped > 0 {
            warnings.push(format!(
                "{} column(s) have a sampling interval with an X long name or units, which the project file written here does not hold (the start and the step are written).",
                n.sampling_dropped
            ));
        }
        if n.ranges_dropped > 0 {
            warnings.push(format!(
                "{} column(s) have a data range (Set As Begin or Set As End), which the project file written here does not hold; Origin shows and plots all their rows.",
                n.ranges_dropped
            ));
        }
        if n.sheet_views_dropped > 0 {
            warnings.push(format!(
                "{} sheet(s) hide label rows, row labels or grid lines, which the project file written here does not hold; Origin shows them.",
                n.sheet_views_dropped
            ));
        }
        if n.sheet_names_dropped > 0 {
            warnings.push(format!(
                "{} sheet(s) show their long name on their tab, which the project file written here does not hold; Origin shows their short names on the tabs.",
                n.sheet_names_dropped
            ));
        }
        if n.sheet_comments_dropped > 0 {
            warnings.push(format!(
                "{} sheet(s) have worksheet comments, which the project file written here does not hold.",
                n.sheet_comments_dropped
            ));
        }
        if n.plot_exclusions_dropped > 0 {
            warnings.push(format!(
                "{} sheet(s) are excluded from the plotting dialogs, which the project file written here does not hold.",
                n.plot_exclusions_dropped
            ));
        }
        if n.filters_dropped > 0 {
            warnings.push(format!(
                "{} sheet(s) have a worksheet filter; the project file written here does not hold it, so Origin shows all rows.",
                n.filters_dropped
            ));
        }
        if n.frozen_dropped > 0 {
            warnings.push(format!(
                "{} sheet(s) have frozen panes; the project file written here does not hold them, so Origin shows none.",
                n.frozen_dropped
            ));
        }
        if n.cond_formats_dropped > 0 {
            warnings.push(format!(
                "{} sheet(s) have conditional formatting; the project file written here does not hold it, so Origin shows none.",
                n.cond_formats_dropped
            ));
        }
        if n.cell_formats_dropped > 0 {
            warnings.push(format!(
                "{} sheet(s) have cell formatting (font, size, bold, italic, underline, colours or alignment); the project file written here does not hold it, so Origin shows plain cells.",
                n.cell_formats_dropped
            ));
        }
        if n.cell_notes_dropped > 0 {
            warnings.push(format!(
                "{} cell note(s) are not written: the project file written here does not hold them, so Origin shows none.",
                n.cell_notes_dropped
            ));
        }
        if n.analysis_links_dropped > 0 {
            warnings.push(format!(
                "{} analysis result(s) recalculate from their source (Recalculate Manual or Auto); the project file written here holds their values only, not the link.",
                n.analysis_links_dropped
            ));
        }
        OpjWrite { bytes, warnings }
    }
}

/// The role code of a column (spec section 4.3).
/// The value-type byte (`0x1E`: display in the high nibble, 0 = Numeric format in the low one) and the
/// digits byte (`0x1F`: decimal places + 3, `0x80 | n` significant digits, 0 default) of a numeric
/// display, as Origin 2016 writes them (T337, measured); `None` for the default display.
fn numeric_display(f: sheaf_core::format::NumberFormat) -> Option<(u8, u8)> {
    use sheaf_core::format::NumberFormat as F;
    let places = |d: u8| d.min(15) + 3;
    let sig = |n: u8| 0x80 | n.clamp(1, 15);
    Some(match f {
        F::Decimal(d) => (0x00, places(d)),
        F::Significant(n) => (0x00, sig(n)),
        F::Scientific(d) => (0x10, places(d)),
        F::ScientificAuto => (0x10, 0),
        F::Engineering(d) | F::EngineeringSi(d) => (0x20, places(d)),
        F::EngineeringSig(n) => (0x20, sig(n)),
        F::EngineeringAuto => (0x20, 0),
        F::DecimalGrouped(d) => (0x30, places(d)),
        _ => return None,
    })
}

fn designation_code(d: Designation) -> u8 {
    match d {
        Designation::X => 3,
        Designation::Y => 0,
        Designation::Z => 5,
        Designation::YError => 2,
        Designation::XError => 6,
        Designation::Label => 4,
        // T330: Origin 2016's codes of Group and Subject (LabTalk types 8 and 9).
        Designation::Group => 7,
        Designation::Subject => 8,
        Designation::Disregard => 1,
    }
}

/// The type word of a column: the high byte tells X (0x51), Y (0x61) and unused (0x41) columns apart,
/// the low byte the error (0x30), Z and label (0x20) and ordinary (0x21) columns.
fn type_code(d: Designation) -> u16 {
    match d {
        Designation::X => 0x5121,
        // Origin writes the Y type word for Group and Subject columns too (T330).
        Designation::Y | Designation::Group | Designation::Subject => 0x6121,
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
        Designation::Y | Designation::Disregard | Designation::Group | Designation::Subject => &[Designation::X],
        Designation::YError | Designation::XError | Designation::Z | Designation::Label => &[Designation::Y],
    };
    roles[..i].iter().rposition(|c| wanted.contains(c)).map_or(0, |p| (first + p + 1) as u16)
}

/// The mask block of a data set in Origin's layout (opj-format.md 3.2): empty without masked
/// cells; otherwise 4 bytes Origin leaves to chance (written as zeros), the block length, 32 zero
/// bytes, one byte per written row (1 = masked) and 4 zero bytes.
fn mask_block(column: &Column, rows: usize) -> Vec<u8> {
    if column.mask.range(..rows).next().is_none() {
        return Vec::new();
    }
    let len = 40 + rows + 4;
    let mut b = vec![0u8; len];
    b[4..8].copy_from_slice(&(len as u32).to_le_bytes());
    for &r in column.mask.range(..rows) {
        b[40 + r] = 1;
    }
    b
}
