//! Where imported data goes (Origin's import modes) and importing many files at once.
//!
//! [`place_workbook`] puts one imported workbook into a project in one of five ways: as a new
//! book, as new sheets of an existing book, over the data of a sheet, as columns appended to a
//! sheet, or as rows appended under the data of a sheet. [`import_batch`] reads a list of files and
//! places them with one of four layouts (one book per file, one sheet per file, all columns side
//! by side in one sheet, all rows stacked in one sheet). [`expand_inputs`] turns a folder or a
//! wildcard pattern into the list of files.

use crate::excel::{self, ExcelOptions};
use crate::{ImportKind, ascii, chi, import_file_with_excel};
use sheaf_core::{Column, ColumnData, Project, WindowId, Workbook, Worksheet};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// How an imported workbook is put into the project. These are Origin's modes "new books",
/// "new sheets", "replace existing data", "new columns" and "new rows".
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ImportMode {
    /// A new workbook.
    #[default]
    NewBook,
    /// A new sheet (or several) in the target workbook.
    NewSheet,
    /// The data of the target sheet is replaced by the imported data; the sheet takes the new name.
    ReplaceSheet,
    /// The imported columns are appended to the right of the target sheet.
    NewColumns,
    /// The imported rows are appended under the data of the target sheet, column by column in
    /// order. The labels of the file are not used (the target keeps its own).
    NewRows,
}

impl ImportMode {
    /// In the order of Origin's drop-down: replace first, then the four ways to add.
    pub const ALL: [ImportMode; 5] = [
        ImportMode::ReplaceSheet,
        ImportMode::NewBook,
        ImportMode::NewSheet,
        ImportMode::NewColumns,
        ImportMode::NewRows,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ImportMode::NewBook => "Start New Books",
            ImportMode::NewSheet => "Start New Sheets",
            ImportMode::ReplaceSheet => "Replace Existing Data",
            ImportMode::NewColumns => "Start New Columns",
            ImportMode::NewRows => "Start New Rows",
        }
    }
}

/// Options of [`place_workbook_with`] (Origin's "(Re)Naming Worksheet and Workbook" group).
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PlaceOptions {
    /// The sheet takes the name that the importer gave it (the file name). When false, a replaced
    /// sheet keeps its name and a new sheet gets the next free `Sheet<n>`.
    pub rename_sheet: bool,
}

impl Default for PlaceOptions {
    fn default() -> Self {
        PlaceOptions { rename_sheet: true }
    }
}

/// The sheet that the modes other than `NewBook` work on.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub book: WindowId,
    pub sheet: usize,
}

/// Where the data landed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub book: WindowId,
    /// The first sheet that was added or changed.
    pub sheet: usize,
    /// Things that did not fit (a column that changed type, extra sheets that were left out).
    pub warnings: Vec<String>,
}

/// Puts `book` (as an importer returns it) into the project. Without a valid `target`, every mode
/// makes a new book, so the first file of a batch can use the same call as the others.
pub fn place_workbook(project: &mut Project, book: Workbook, mode: ImportMode, target: Option<Target>) -> Placed {
    place_workbook_with(project, book, mode, target, PlaceOptions::default())
}

/// [`place_workbook`] with the naming options.
///
/// Replacing the data of a sheet keeps the identity of its columns by position: the first new
/// column takes over the identifier of the first old column, and so on. A graph that plots those
/// columns therefore keeps plotting them, now with the new data. As in Origin (measured, import-ascii.md
/// section 10) a plot of all rows follows the new length and a plot with a row range keeps its range.
pub fn place_workbook_with(
    project: &mut Project,
    book: Workbook,
    mode: ImportMode,
    target: Option<Target>,
    options: PlaceOptions,
) -> Placed {
    let valid = target.filter(|t| project.workbook(t.book).is_some_and(|b| t.sheet < b.sheets.len()));
    let (Some(t), false) = (valid, mode == ImportMode::NewBook) else {
        let id = project.adopt_workbook(book);
        return Placed { book: id, sheet: 0, warnings: Vec::new() };
    };
    let mut warnings = Vec::new();
    // Fresh column identifiers for everything that arrives.
    let mut sheets = book.sheets;
    for sheet in &mut sheets {
        for col in &mut sheet.columns {
            col.id = project.new_column_id();
        }
    }
    let Some(dest) = project.workbook_mut(t.book) else {
        unreachable!("the target was checked above");
    };
    let mut first = t.sheet;
    if !options.rename_sheet {
        for sheet in &mut sheets {
            sheet.name = String::new();
        }
    }
    match mode {
        ImportMode::NewBook => unreachable!("handled above"),
        ImportMode::NewSheet => {
            first = dest.sheets.len();
            for mut sheet in sheets {
                sheet.name = new_sheet_name(dest, &sheet.name);
                dest.sheets.push(sheet);
            }
            dest.active_sheet = first;
        }
        ImportMode::ReplaceSheet => {
            let mut it = sheets.into_iter();
            if let Some(new) = it.next() {
                let name = if new.name.is_empty() {
                    dest.sheets[t.sheet].name.clone()
                } else {
                    unique_sheet_name_except(dest, &new.name, t.sheet)
                };
                let mut columns = new.columns;
                for (old, col) in dest.sheets[t.sheet].columns.iter().zip(columns.iter_mut()) {
                    col.id = old.id;
                }
                dest.sheets[t.sheet] = Worksheet {
                    name,
                    long_name: dest.sheets[t.sheet].long_name.clone(),
                    comments: dest.sheets[t.sheet].comments.clone(),
                    exclude_from_plotting: dest.sheets[t.sheet].exclude_from_plotting,
                    tab_label: dest.sheets[t.sheet].tab_label,
                    view: dest.sheets[t.sheet].view,
                    columns,
                    filter: None,
                    frozen: Default::default(),
                    cond_formats: Vec::new(),
                    notes: Vec::new(),
                    cell_formats: Vec::new(),
                    font_defaults: Default::default(),
                    comparison_source: None,
                };
            }
            // Further sheets of the file (several data blocks) follow as new sheets.
            for mut sheet in it {
                sheet.name = new_sheet_name(dest, &sheet.name);
                dest.sheets.push(sheet);
            }
        }
        ImportMode::NewColumns => {
            if sheets.len() > 1 {
                warnings.push(format!("the file has {} sheets; only the first was added", sheets.len()));
            }
            if let Some(new) = sheets.into_iter().next() {
                let ws = &mut dest.sheets[t.sheet];
                for mut col in new.columns {
                    if ws.columns.iter().any(|c| c.short_name.eq_ignore_ascii_case(&col.short_name)) {
                        col.short_name = ws.next_short_name();
                    }
                    ws.columns.push(col);
                }
            }
        }
        ImportMode::NewRows => {
            if sheets.len() > 1 {
                warnings.push(format!("the file has {} sheets; only the first was added", sheets.len()));
            }
            if let Some(new) = sheets.into_iter().next() {
                append_rows(&mut dest.sheets[t.sheet], new, &mut warnings);
            }
        }
    }
    Placed { book: t.book, sheet: first, warnings }
}

/// The name of a sheet that is added to `book`: the wanted name made unique, or, when there is
/// none, the next free `Sheet<n>`.
fn new_sheet_name(book: &Workbook, wanted: &str) -> String {
    if wanted.is_empty() {
        let taken = |n: &str| book.sheets.iter().any(|s| s.name.eq_ignore_ascii_case(n));
        return (book.sheets.len() + 1..).map(|k| format!("Sheet{k}")).find(|n| !taken(n)).expect("unbounded");
    }
    unique_sheet_name(book, wanted)
}

fn unique_sheet_name(book: &Workbook, wanted: &str) -> String {
    unique_sheet_name_except(book, wanted, usize::MAX)
}

/// A sheet name that no other sheet of the book uses (ignoring the sheet `except`): the wanted name,
/// or the name followed by `_2`, `_3`, ...
fn unique_sheet_name_except(book: &Workbook, wanted: &str, except: usize) -> String {
    let wanted = if wanted.trim().is_empty() { "Sheet" } else { wanted };
    let taken = |n: &str| book.sheets.iter().enumerate().any(|(i, s)| i != except && s.name.eq_ignore_ascii_case(n));
    if !taken(wanted) {
        return wanted.to_string();
    }
    (2..).map(|k| format!("{wanted}_{k}")).find(|n| !taken(n)).expect("the sequence is unbounded")
}

/// Appends the rows of `new` under the rows of `ws`, column by column in order. A column that has
/// no partner is added (the rows above it are missing); a partner of another type gets missing
/// values for the new rows and a warning.
fn append_rows(ws: &mut Worksheet, new: Worksheet, warnings: &mut Vec<String>) {
    if ws.columns.is_empty() {
        ws.columns = new.columns;
        return;
    }
    let old_rows = ws.row_count();
    let added = new.row_count();
    for (j, col) in new.columns.into_iter().enumerate() {
        if j >= ws.columns.len() {
            let mut fresh = col;
            fresh.short_name = ws.next_short_name();
            pad_front(&mut fresh, old_rows);
            ws.columns.push(fresh);
            continue;
        }
        let dest = &mut ws.columns[j];
        pad_to(dest, old_rows);
        match (&mut dest.data, col.data) {
            (ColumnData::Numeric(d), ColumnData::Numeric(s)) => std::sync::Arc::make_mut(d).extend(s.iter().copied()),
            (ColumnData::Text(d), ColumnData::Text(s)) => std::sync::Arc::make_mut(d).extend(s.iter().cloned()),
            (ColumnData::Mixed { numbers, texts }, src) => {
                let offset = old_rows.max(numbers.len());
                let (nums, txt): (Vec<f64>, Vec<(usize, String)>) = match src {
                    ColumnData::Numeric(s) => (s.to_vec(), Vec::new()),
                    ColumnData::Text(s) => (
                        vec![f64::NAN; s.len()],
                        s.iter().enumerate().filter(|(_, t)| !t.is_empty()).map(|(i, t)| (i, t.clone())).collect(),
                    ),
                    ColumnData::Mixed { numbers: n, texts: t } => {
                        (n.to_vec(), t.iter().map(|(i, t)| (*i, t.clone())).collect())
                    }
                };
                let numbers = std::sync::Arc::make_mut(numbers);
                numbers.resize(offset, f64::NAN);
                numbers.extend(nums);
                let texts = std::sync::Arc::make_mut(texts);
                texts.extend(txt.into_iter().map(|(i, t)| (offset + i, t)));
            }
            (ColumnData::Numeric(d), ColumnData::Mixed { numbers: s, texts }) => {
                if !texts.is_empty() {
                    warnings.push(format!(
                        "column {} holds numbers but the new rows hold text; the text was left empty",
                        dest.short_name
                    ));
                }
                std::sync::Arc::make_mut(d).extend(s.iter().copied());
            }
            (ColumnData::Text(d), ColumnData::Mixed { numbers: s, texts }) => {
                if s.iter().any(|x| !x.is_nan()) {
                    warnings.push(format!(
                        "column {} holds text but the new rows hold numbers; they were left empty",
                        dest.short_name
                    ));
                }
                let d = std::sync::Arc::make_mut(d);
                d.extend((0..s.len()).map(|i| texts.get(&i).cloned().unwrap_or_default()));
            }
            (ColumnData::Numeric(d), ColumnData::Text(s)) => {
                warnings.push(format!(
                    "column {} holds numbers but the new rows are text; they were left empty",
                    dest.short_name
                ));
                std::sync::Arc::make_mut(d).extend(std::iter::repeat_n(f64::NAN, s.len()));
            }
            (ColumnData::Text(d), ColumnData::Numeric(s)) => {
                warnings.push(format!(
                    "column {} holds text but the new rows are numbers; they were left empty",
                    dest.short_name
                ));
                std::sync::Arc::make_mut(d).extend(std::iter::repeat_n(String::new(), s.len()));
            }
        }
    }
    let total = old_rows + added;
    for c in &mut ws.columns {
        pad_to(c, total);
    }
}

/// Fills a column up to `rows` rows with missing values.
fn pad_to(col: &mut Column, rows: usize) {
    match &mut col.data {
        ColumnData::Numeric(v) if v.len() < rows => std::sync::Arc::make_mut(v).resize(rows, f64::NAN),
        ColumnData::Text(v) if v.len() < rows => std::sync::Arc::make_mut(v).resize(rows, String::new()),
        ColumnData::Mixed { numbers, .. } if numbers.len() < rows => {
            std::sync::Arc::make_mut(numbers).resize(rows, f64::NAN)
        }
        _ => {}
    }
}

/// Puts `rows` missing values in front of a column's data.
fn pad_front(col: &mut Column, rows: usize) {
    match &mut col.data {
        ColumnData::Numeric(v) => {
            let mut all = vec![f64::NAN; rows];
            all.extend(v.iter().copied());
            *v = std::sync::Arc::new(all);
        }
        ColumnData::Text(v) => {
            let mut all = vec![String::new(); rows];
            all.extend(v.iter().cloned());
            *v = std::sync::Arc::new(all);
        }
        ColumnData::Mixed { numbers, texts } => {
            let mut all = vec![f64::NAN; rows];
            all.extend(numbers.iter().copied());
            *numbers = std::sync::Arc::new(all);
            *texts = std::sync::Arc::new(texts.iter().map(|(r, t)| (r + rows, t.clone())).collect());
        }
    }
}

// ----- many files ----------------------------------------------------------------------

/// How the files of a batch are laid out.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BatchLayout {
    /// One book per file.
    #[default]
    Books,
    /// One book; one sheet per file, named after the file.
    Sheets,
    /// One book and one sheet; the columns of the files side by side. The file name is kept in
    /// the comment row and in the `SourceFile` parameter row.
    Columns,
    /// One book and one sheet; the rows of the files stacked. Needs files of the same layout.
    Rows,
}

impl BatchLayout {
    pub const ALL: [BatchLayout; 4] =
        [BatchLayout::Books, BatchLayout::Sheets, BatchLayout::Columns, BatchLayout::Rows];

    pub fn label(self) -> &'static str {
        match self {
            BatchLayout::Books => "One book per file",
            BatchLayout::Sheets => "One sheet per file",
            BatchLayout::Columns => "Columns side by side",
            BatchLayout::Rows => "Rows one under the other",
        }
    }
}

/// Settings of [`import_batch`].
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BatchOptions {
    pub layout: BatchLayout,
    pub kind: ImportKind,
    pub chi: chi::ChiOptions,
    pub ascii: ascii::AsciiOptions,
    /// Long name of the shared book (layouts other than `Books`); empty means the folder name of
    /// the first file.
    pub book_name: String,
    /// Write the file name into the comment of every column that has none, and into the
    /// `SourceFile` parameter row. Always done for `Columns`.
    pub file_name_in_comments: bool,
    /// Import the first supplied file in full; apply ASCII row/column selection only to later
    /// files (Origin's partial-import scope). A failed first file does not shift this boundary.
    pub partial_except_first: bool,
}

impl Default for BatchOptions {
    fn default() -> Self {
        BatchOptions {
            layout: BatchLayout::Books,
            kind: ImportKind::Auto,
            chi: chi::ChiOptions::default(),
            ascii: ascii::AsciiOptions::default(),
            book_name: String::new(),
            file_name_in_comments: false,
            partial_except_first: false,
        }
    }
}

/// One file that was imported.
#[derive(Clone, Debug)]
pub struct BatchItem {
    pub path: PathBuf,
    pub book: WindowId,
    pub sheet: usize,
    pub format: &'static str,
    pub suggested_template: sheaf_core::template::Template,
    /// Notes about this file (import warnings and placement warnings).
    pub warnings: Vec<String>,
}

/// What [`import_batch`] did.
#[derive(Clone, Debug, Default)]
pub struct BatchResult {
    pub items: Vec<BatchItem>,
    /// Files that could not be read, with the reason. The others were imported.
    pub errors: Vec<String>,
}

/// Reads the files in order and puts them into the project with the chosen layout. A file that
/// cannot be read is reported in `errors` and skipped.
pub fn import_batch(project: &mut Project, files: &[PathBuf], opts: &BatchOptions) -> BatchResult {
    import_batch_with_excel(project, files, opts, &ExcelOptions::default())
}

/// [`import_batch`] with the options for Excel workbooks among the files (`.xls`, `.xlsx`,
/// `.xlsm`). An Excel file gives one book with a sheet per Excel sheet; the layouts and modes put
/// that book where they put any other imported book (a layout that adds columns or rows to one
/// sheet uses the first sheet of the Excel file and says so).
pub fn import_batch_with_excel(
    project: &mut Project,
    files: &[PathBuf],
    opts: &BatchOptions,
    excel_opts: &ExcelOptions,
) -> BatchResult {
    let mut result = BatchResult::default();
    let mut shared: Option<WindowId> = None;
    for (index, path) in files.iter().enumerate() {
        let ascii = if index == 0 && opts.partial_except_first {
            opts.ascii.without_partial_import()
        } else {
            opts.ascii.clone()
        };
        let excel_here = if index == 0 && opts.partial_except_first {
            excel_opts.without_partial_import()
        } else {
            excel_opts.clone()
        };
        let imported = match import_file_with_excel(path, opts.kind, &opts.chi, &ascii, &excel_here) {
            Ok(i) => i,
            Err(e) => {
                result.errors.push(e.to_string());
                continue;
            }
        };
        let (format, suggested_template) = (imported.format, imported.suggested_template);
        let mut book = imported.workbook;
        let mut warnings = imported.warnings;
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if opts.file_name_in_comments || opts.layout == BatchLayout::Columns {
            for sheet in &mut book.sheets {
                for col in &mut sheet.columns {
                    if col.comments.is_empty() {
                        col.comments = stem.clone();
                    }
                    col.params.entry("SourceFile".to_string()).or_insert_with(|| stem.clone());
                }
            }
        }
        let placed = match (opts.layout, shared) {
            (BatchLayout::Books, _) | (_, None) => {
                if opts.layout != BatchLayout::Books {
                    book.long_name = shared_book_name(opts, path);
                    // The one sheet of a stacked or side-by-side layout is not named after a single file.
                    if matches!(opts.layout, BatchLayout::Columns | BatchLayout::Rows)
                        && let Some(s) = book.sheets.first_mut()
                    {
                        s.name = "Sheet1".to_string();
                    }
                }
                let placed = place_workbook(project, book, ImportMode::NewBook, None);
                if opts.layout != BatchLayout::Books {
                    shared = Some(placed.book);
                }
                placed
            }
            (layout, Some(book_id)) => {
                let (mode, sheet) = match layout {
                    BatchLayout::Sheets => (ImportMode::NewSheet, 0),
                    BatchLayout::Columns => (ImportMode::NewColumns, 0),
                    _ => (ImportMode::NewRows, 0),
                };
                place_workbook(project, book, mode, Some(Target { book: book_id, sheet }))
            }
        };
        warnings.extend(placed.warnings);
        result.items.push(BatchItem {
            path: path.clone(),
            book: placed.book,
            sheet: placed.sheet,
            format,
            suggested_template,
            warnings,
        });
    }
    result
}

fn shared_book_name(opts: &BatchOptions, first: &Path) -> String {
    if !opts.book_name.trim().is_empty() {
        return opts.book_name.trim().to_string();
    }
    first
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported files".to_string())
}

/// Extensions that a folder import reads.
const FOLDER_EXTENSIONS: [&str; 11] = ["txt", "csv", "tsv", "dat", "asc", "prn", "tab", "bin", "xls", "xlsx", "xlsm"];

/// Turns one command-line or dialog argument into files: a folder gives its data files
/// (`.txt .csv .tsv .dat .asc .prn .tab .bin` and the Excel files `.xls .xlsx .xlsm`, but not the
/// `~$` lock files that Excel leaves beside an open workbook), a name with `*` or `?` in its last part gives
/// the matching files of that folder, anything else is the file itself. The result is in natural
/// order (`run2` before `run10`). A pattern or folder that finds nothing is an error.
pub fn expand_inputs(arg: &Path) -> Result<Vec<PathBuf>, String> {
    let name = arg.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut found: Vec<PathBuf> = Vec::new();
    if arg.is_dir() {
        found = list_folder(arg, |n| {
            !(n.starts_with("~$") && excel::is_excel_file(Path::new(n)))
                && Path::new(n)
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| FOLDER_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        })?;
        if found.is_empty() {
            return Err(format!(
                "the folder '{}' holds no data files (.txt, .csv, .tsv, .dat, .asc, .prn, .tab, .bin, .xls, .xlsx, .xlsm)",
                arg.display()
            ));
        }
    } else if name.contains(['*', '?']) {
        let dir = match arg.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        if dir.to_string_lossy().contains(['*', '?']) {
            return Err(format!("'{}': wildcards are only allowed in the file name, not in the folder", arg.display()));
        }
        found = list_folder(&dir, |n| wildcard_match(&name, n))?;
        if found.is_empty() {
            return Err(format!("no file matches '{}'", arg.display()));
        }
    }
    if found.is_empty() {
        return Ok(vec![arg.to_path_buf()]);
    }
    found.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
    Ok(found)
}

fn list_folder(dir: &Path, keep: impl Fn(&str) -> bool) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("cannot read the folder '{}': {e}", dir.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter(|e| keep(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect())
}

/// `*` matches any run of characters, `?` one character; letter case is ignored.
pub fn wildcard_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// Orders names so that numbers inside them compare as numbers: `run2` before `run10`.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars<'_>>| {
                    let mut s = String::new();
                    while let Some(ch) = it.next_if(char::is_ascii_digit) {
                        s.push(ch);
                    }
                    s
                };
                let (na, nb) = (take(&mut x), take(&mut y));
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let o = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(c), Some(d)) => {
                let o = c.to_lowercase().cmp(d.to_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnId, Designation};

    fn book(name: &str, cols: Vec<Column>) -> Workbook {
        let mut b = Workbook::new(WindowId(0), "Book1");
        b.long_name = name.to_string();
        b.sheets = vec![Worksheet { columns: cols, ..Worksheet::new(name) }];
        b
    }

    fn num(short: &str, long: &str, v: &[f64]) -> Column {
        let mut c = Column::new(ColumnId(1), short, Designation::Y, ColumnData::numeric(v.to_vec()));
        c.long_name = long.to_string();
        c
    }

    fn txt(short: &str, v: &[&str]) -> Column {
        Column::new(ColumnId(1), short, Designation::Label, ColumnData::text(v.iter().map(|s| s.to_string()).collect()))
    }

    fn project_with(b: Workbook) -> (Project, Target) {
        let mut p = Project::new();
        let id = p.adopt_workbook(b);
        (p, Target { book: id, sheet: 0 })
    }

    #[test]
    fn new_sheet_adds_sheets_with_free_names_and_fresh_ids() {
        let (mut p, t) = project_with(book("run", vec![num("A", "x", &[1.0])]));
        let placed = place_workbook(&mut p, book("run", vec![num("A", "x", &[2.0])]), ImportMode::NewSheet, Some(t));
        assert_eq!((placed.book, placed.sheet), (t.book, 1));
        let b = p.workbook(t.book).unwrap();
        assert_eq!(b.sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["run", "run_2"]);
        assert_eq!(b.active_sheet, 1);
        assert_ne!(b.sheets[0].columns[0].id, b.sheets[1].columns[0].id);
    }

    #[test]
    fn replace_keeps_the_book_and_swaps_the_data() {
        let (mut p, t) = project_with(book("old", vec![num("A", "x", &[1.0, 2.0]), num("B", "y", &[3.0, 4.0])]));
        p.workbook_mut(t.book).unwrap().sheets[t.sheet].comments = "Run notes".into();
        let placed =
            place_workbook(&mut p, book("new", vec![num("A", "z", &[9.0])]), ImportMode::ReplaceSheet, Some(t));
        assert_eq!(placed.book, t.book);
        let s = &p.workbook(t.book).unwrap().sheets[0];
        assert_eq!(s.name, "new");
        assert_eq!(s.comments, "Run notes");
        assert_eq!(s.columns.len(), 1);
        assert_eq!(s.columns[0].long_name, "z");
        assert_eq!(p.workbooks().count(), 1);
    }

    #[test]
    fn replacing_data_keeps_the_plots_and_their_row_ranges_like_origin() {
        use sheaf_core::template::Template;
        let (mut p, t) =
            project_with(book("old", vec![num("A", "x", &[1.0, 2.0, 3.0, 4.0]), num("B", "y", &[3.0, 4.0, 5.0, 6.0])]));
        let ids: Vec<ColumnId> = p.workbook(t.book).unwrap().sheets[0].columns.iter().map(|c| c.id).collect();
        // One plot of all rows and one limited to rows 1 to 4 (Origin: plot range 2..4 of 5).
        let all = p.plot_columns(Template::Line, &[ids[1]]).unwrap();
        let limited = p.plot_columns(Template::Line, &[ids[1]]).unwrap();
        p.graph_mut(limited).unwrap().layers[0].plots[0].rows = Some((0, 4));
        let new =
            book("new", vec![num("A", "x", &(1..=9).map(f64::from).collect::<Vec<_>>()), num("B", "y", &[1.0; 9])]);
        place_workbook(&mut p, new, ImportMode::ReplaceSheet, Some(t));
        let sheet = &p.workbook(t.book).unwrap().sheets[0];
        assert_eq!(sheet.columns[1].id, ids[1], "the plot still finds its column");
        let plot = &p.graph(all).unwrap().layers[0].plots[0];
        assert_eq!((plot.y, plot.rows), (ids[1], None), "a plot of all rows follows the new data");
        assert_eq!(p.numeric(plot.y).unwrap().len(), 9);
        let plot = &p.graph(limited).unwrap().layers[0].plots[0];
        assert_eq!(plot.rows, Some((0, 4)), "a plot with a row range keeps it");
    }

    #[test]
    fn not_renaming_the_sheet_keeps_a_replaced_name_and_numbers_new_sheets() {
        let (mut p, t) = project_with(book("old", vec![num("A", "x", &[1.0])]));
        let keep = PlaceOptions { rename_sheet: false };
        place_workbook_with(&mut p, book("new", vec![num("A", "x", &[2.0])]), ImportMode::ReplaceSheet, Some(t), keep);
        assert_eq!(p.workbook(t.book).unwrap().sheets[0].name, "old");
        place_workbook_with(&mut p, book("new", vec![num("A", "x", &[2.0])]), ImportMode::NewSheet, Some(t), keep);
        place_workbook_with(&mut p, book("new", vec![num("A", "x", &[2.0])]), ImportMode::NewSheet, Some(t), keep);
        let names: Vec<_> = p.workbook(t.book).unwrap().sheets.iter().map(|s| s.name.clone()).collect();
        assert_eq!(names, ["old", "Sheet2", "Sheet3"]);
    }

    #[test]
    fn the_modes_carry_origins_names_in_origins_order() {
        let labels: Vec<_> = ImportMode::ALL.iter().map(|m| m.label()).collect();
        assert_eq!(
            labels,
            ["Replace Existing Data", "Start New Books", "Start New Sheets", "Start New Columns", "Start New Rows"]
        );
    }

    #[test]
    fn new_columns_go_to_the_right_with_unique_short_names() {
        let (mut p, t) =
            project_with(book("a", vec![num("A", "t", &[1.0, 2.0, 3.0]), num("B", "a", &[10.0, 20.0, 30.0])]));
        place_workbook(
            &mut p,
            book("b", vec![num("A", "t", &[4.0, 5.0]), num("B", "b", &[40.0, 50.0])]),
            ImportMode::NewColumns,
            Some(t),
        );
        let s = &p.workbook(t.book).unwrap().sheets[0];
        let names: Vec<&str> = s.columns.iter().map(|c| c.short_name.as_str()).collect();
        assert_eq!(names, ["A", "B", "C", "D"]);
        assert_eq!(s.columns[3].long_name, "b");
        assert_eq!(s.row_count(), 3);
    }

    #[test]
    fn new_rows_stack_under_the_data_and_keep_the_first_labels() {
        // The same shape as the Origin run: two files with the labels Time/A and Time/B; Origin kept
        // the labels of the first file and put the second file's rows below.
        let (mut p, t) =
            project_with(book("a", vec![num("A", "Time", &[1.0, 2.0, 3.0]), num("B", "A", &[10.0, 20.0, 30.0])]));
        let placed = place_workbook(
            &mut p,
            book("b", vec![num("A", "Time", &[4.0, 5.0]), num("B", "B", &[40.0, 50.0])]),
            ImportMode::NewRows,
            Some(t),
        );
        assert!(placed.warnings.is_empty());
        let s = &p.workbook(t.book).unwrap().sheets[0];
        assert_eq!(s.columns[0].numeric().unwrap(), &[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(s.columns[1].numeric().unwrap(), &[10.0, 20.0, 30.0, 40.0, 50.0]);
        assert_eq!(s.columns[1].long_name, "A");
    }

    #[test]
    fn new_rows_with_other_shapes() {
        let (mut p, t) = project_with(book("a", vec![num("A", "x", &[1.0, 2.0]), txt("B", &["p", "q"])]));
        // A third column arrives, the second changes type, and the first file had two rows.
        let new = book("b", vec![num("A", "x", &[3.0]), num("B", "y", &[9.0]), num("C", "z", &[7.0])]);
        let placed = place_workbook(&mut p, new, ImportMode::NewRows, Some(t));
        assert_eq!(placed.warnings.len(), 1, "{:?}", placed.warnings);
        let s = &p.workbook(t.book).unwrap().sheets[0];
        assert_eq!(s.columns[0].numeric().unwrap(), &[1.0, 2.0, 3.0]);
        assert_eq!(s.columns[1].text().unwrap(), ["p", "q", ""]);
        let c = s.columns[2].numeric().unwrap();
        assert!(c[0].is_nan() && c[1].is_nan());
        assert_eq!(c[2], 7.0);
        assert_eq!(s.columns[2].short_name, "C");
        // Fewer columns than the target: the others are padded.
        let (mut p, t) = project_with(book("a", vec![num("A", "x", &[1.0]), num("B", "y", &[2.0])]));
        place_workbook(&mut p, book("b", vec![num("A", "x", &[5.0, 6.0])]), ImportMode::NewRows, Some(t));
        let s = &p.workbook(t.book).unwrap().sheets[0];
        assert_eq!(s.columns[0].len(), 3);
        assert_eq!(s.columns[1].len(), 3);
    }

    #[test]
    fn without_a_target_every_mode_makes_a_book() {
        let mut p = Project::new();
        for mode in ImportMode::ALL {
            let placed = place_workbook(&mut p, book("x", vec![num("A", "x", &[1.0])]), mode, None);
            assert_eq!(placed.sheet, 0);
        }
        assert_eq!(p.workbooks().count(), 5);
        // A target that no longer exists is the same as none.
        let gone = Target { book: WindowId(9999), sheet: 0 };
        place_workbook(&mut p, book("x", vec![num("A", "x", &[1.0])]), ImportMode::NewColumns, Some(gone));
        assert_eq!(p.workbooks().count(), 6);
    }

    #[test]
    fn wildcards_and_natural_order() {
        assert!(wildcard_match("*.csv", "Run1.CSV"));
        assert!(wildcard_match("icp_??.txt", "icp_07.txt"));
        assert!(!wildcard_match("icp_??.txt", "icp_7.txt"));
        assert!(wildcard_match("a*b*c", "aXXbYYc"));
        assert!(!wildcard_match("a*b", "abc"));
        let mut names = vec!["run10.txt", "run2.txt", "run1.txt", "Run3.txt", "run02.txt"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(names, ["run1.txt", "run2.txt", "run02.txt", "Run3.txt", "run10.txt"]);
    }

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sheaf-io-batch-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn folders_and_patterns_expand_to_sorted_files() {
        let dir = temp("expand");
        for n in ["s10.csv", "s2.csv", "s1.csv", "notes.md", "other.txt"] {
            std::fs::write(dir.join(n), "x,y\n1,2\n").unwrap();
        }
        std::fs::create_dir(dir.join("sub.csv")).unwrap();
        let names = |v: Vec<PathBuf>| {
            v.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>()
        };
        assert_eq!(names(expand_inputs(&dir.join("s*.csv")).unwrap()), ["s1.csv", "s2.csv", "s10.csv"]);
        assert_eq!(names(expand_inputs(&dir).unwrap()), ["other.txt", "s1.csv", "s2.csv", "s10.csv"]);
        assert_eq!(names(expand_inputs(&dir.join("other.txt")).unwrap()), ["other.txt"]);
        assert!(expand_inputs(&dir.join("zzz*.csv")).unwrap_err().contains("no file matches"));
        assert!(expand_inputs(&dir.join("*").join("x*.csv")).is_err());
        let empty = temp("expand-empty");
        assert!(expand_inputs(&empty).unwrap_err().contains("no data files"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
    }

    fn write_icp(dir: &Path, name: &str, base: f64) {
        let text = format!(
            "Sample Name,Fe 238.204,Cu 324.754,Measured\nUnits,mg/L,mg/L,\n\
             Blank,<LOD,0.02,2026-05-04 09:15:22\nStd,{base},{},2026-05-04 09:17:40\nSample,{},n.d.,2026-05-04 09:19:58\n",
            base + 1.0,
            base * 3.0
        );
        std::fs::write(dir.join(name), text).unwrap();
    }

    fn chi_file() -> String {
        let mut cv = String::from("Sept. 12, 2026\r\nCyclic Voltammetry\r\n\r\nPotential/V, Current/A\r\n\r\n");
        for e in [0.0, 0.5, 1.0, 0.5, 0.0, 0.5, 1.0] {
            cv.push_str(&format!("{e}, {:e}\r\n", e * 1e-5));
        }
        cv
    }

    #[test]
    fn a_series_of_icp_exports_goes_into_one_book_with_a_sheet_each() {
        let dir = temp("icp-sheets");
        for (i, n) in ["icp_1.csv", "icp_2.csv", "icp_10.csv"].iter().enumerate() {
            write_icp(&dir, n, 1.0 + i as f64);
        }
        let files = expand_inputs(&dir.join("icp_*.csv")).unwrap();
        let mut p = Project::new();
        let r = import_batch(&mut p, &files, &BatchOptions { layout: BatchLayout::Sheets, ..BatchOptions::default() });
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert_eq!(r.items.len(), 3);
        assert_eq!(p.workbooks().count(), 1);
        let b = p.workbooks().next().unwrap();
        assert_eq!(b.sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["icp_1", "icp_2", "icp_10"]);
        assert!(b.long_name.contains("icp-sheets"), "the book is named after the folder: {}", b.long_name);
        let fe = &b.sheets[1].columns[1];
        assert_eq!((fe.long_name.as_str(), fe.units.as_str()), ("Fe 238.204", "mg/L"));
        assert!(fe.numeric().unwrap()[0].is_nan(), "<LOD is a missing value for the numbers");
        assert_eq!(fe.cell_text(0), "<LOD", "and stays text in the cell");
        assert_eq!(fe.numeric().unwrap()[1], 2.0);
        // The text cells are kept, so there is nothing to report.
        assert!(r.items.iter().all(|i| i.warnings.iter().all(|w| !w.contains("no-value marker"))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn partial_selection_can_skip_only_the_first_supplied_file() {
        use crate::ascii::{Delimiter, ImportSelection};
        let dir = temp("partial-except-first");
        let text = "t|signal|other\n1|11|21\n2|12|22\n3|13|23\n4|14|24\n5|15|25\n";
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        std::fs::write(&first, text).unwrap();
        std::fs::write(&second, text).unwrap();
        let opts = BatchOptions {
            partial_except_first: true,
            ascii: ascii::AsciiOptions {
                delimiter: Delimiter::Other('|'),
                rows: ImportSelection { first: 2, last: 5, read: 1, skip: 1 },
                columns: ImportSelection { first: 2, last: 2, ..ImportSelection::default() },
                ..ascii::AsciiOptions::default()
            },
            ..BatchOptions::default()
        };
        let mut p = Project::new();
        let result = import_batch(&mut p, &[first, second.clone()], &opts);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let books: Vec<_> = p.workbooks().collect();
        assert_eq!(books[0].sheet().columns.len(), 3);
        assert_eq!(books[0].sheet().row_count(), 5);
        assert_eq!(books[1].sheet().columns.len(), 1);
        assert_eq!(books[1].sheet().columns[0].long_name, "signal");
        assert_eq!(books[1].sheet().columns[0].numeric().unwrap(), [12.0, 14.0]);
        // Failure does not promote the next file to "first": scope follows input order.
        let mut p = Project::new();
        let result = import_batch(&mut p, &[dir.join("missing.txt"), second], &opts);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(p.workbooks().next().unwrap().sheet().columns[0].numeric().unwrap(), [12.0, 14.0]);
    }

    #[test]
    fn side_by_side_keeps_the_file_name_in_comments() {
        let dir = temp("icp-cols");
        write_icp(&dir, "a.csv", 1.0);
        write_icp(&dir, "b.csv", 5.0);
        let files = expand_inputs(&dir.join("*.csv")).unwrap();
        let mut p = Project::new();
        let opts =
            BatchOptions { layout: BatchLayout::Columns, book_name: "ICP series".into(), ..BatchOptions::default() };
        let r = import_batch(&mut p, &files, &opts);
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let b = p.workbooks().next().unwrap();
        assert_eq!((b.sheets.len(), b.long_name.as_str()), (1, "ICP series"));
        let s = &b.sheets[0];
        assert_eq!(s.columns.len(), 8);
        let comments: Vec<&str> = s.columns.iter().map(|c| c.comments.as_str()).collect();
        assert_eq!(comments, ["a", "a", "a", "a", "b", "b", "b", "b"]);
        assert_eq!(s.columns[5].params.get("SourceFile").map(String::as_str), Some("b"));
        let shorts: std::collections::HashSet<&str> = s.columns.iter().map(|c| c.short_name.as_str()).collect();
        assert_eq!(shorts.len(), 8, "short names stay unique");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stacked_rows_and_separate_books() {
        let dir = temp("icp-rows");
        write_icp(&dir, "a.csv", 1.0);
        write_icp(&dir, "b.csv", 5.0);
        let files = expand_inputs(&dir.join("*.csv")).unwrap();
        let mut p = Project::new();
        import_batch(&mut p, &files, &BatchOptions { layout: BatchLayout::Rows, ..BatchOptions::default() });
        let s = &p.workbooks().next().unwrap().sheets[0];
        assert_eq!(s.row_count(), 6);
        assert_eq!(s.columns[0].text().unwrap()[3], "Blank");
        let mut p = Project::new();
        import_batch(&mut p, &files, &BatchOptions::default());
        assert_eq!(p.workbooks().count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chi_files_in_a_batch_use_the_chi_importer_and_bad_files_are_reported() {
        let dir = temp("mixed");
        std::fs::write(dir.join("a_cv.txt"), chi_file()).unwrap();
        write_icp(&dir, "b_icp.csv", 1.0);
        std::fs::write(dir.join("c_empty.txt"), "").unwrap();
        let files = expand_inputs(&dir).unwrap();
        let mut p = Project::new();
        let r = import_batch(&mut p, &files, &BatchOptions { layout: BatchLayout::Sheets, ..BatchOptions::default() });
        assert_eq!(r.items.iter().map(|i| i.format).collect::<Vec<_>>(), ["chi", "ascii"]);
        assert_eq!(r.errors.len(), 1);
        assert!(r.errors[0].contains("c_empty.txt"), "{:?}", r.errors);
        let b = p.workbooks().next().unwrap();
        assert!(b.sheets.len() >= 2, "the CHI sheets and the ICP sheet share one book");
        assert!(b.sheets.iter().any(|s| s.name == "b_icp"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
