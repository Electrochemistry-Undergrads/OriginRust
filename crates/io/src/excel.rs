//! Excel workbooks: import of `.xls`, `.xlsx` and `.xlsm`, export of `.xlsx`.
//!
//! One Excel file is one Sheaf workbook and each Excel sheet is a worksheet of it. The sheet's
//! rows are read like the lines of a text file: the same detection finds the first data row and
//! the Long Name, Units and Comments lines above it, and the same options choose them
//! ([`ExcelOptions`] holds the ones that apply to cells). What is stored in a cell decides its type:
//! numbers stay numbers, text stays text, dates and times become date and time columns, a formula
//! gives the value Excel stored with it. Origin's behaviour is not measured for any of this; the
//! choices made are listed in `docs/spec/import-excel.md`.

mod dates;
mod formulas;
#[cfg(test)]
mod tests;
mod write;

use crate::ascii::{self, AsciiOptions, DateImport, ImportSelection, NumStyle, Thousands};
use crate::error::IoError;
use calamine::{Cell, Data, Range, Reader, SheetType, SheetVisible, Sheets, StyleRange};
use dates::{Converted, DateKind};
use sheaf_core::datetime::{self, DateOrder, DatePattern, TimePattern};
use sheaf_core::format::NumberFormat;
use sheaf_core::worksheet::ImportSource;
use sheaf_core::{ColumnData, WindowId, Workbook, Worksheet};
use std::collections::BTreeMap;
use std::path::Path;

pub use write::{ExcelExportOptions, ExportOutcome, write_workbook_xlsx, write_xlsx};

/// File extensions that [`is_excel_file`] accepts.
pub const EXTENSIONS: [&str; 3] = ["xls", "xlsx", "xlsm"];

/// True for the names of Excel workbooks that [`read_workbook`] reads in the app and the command
/// line: `.xls`, `.xlsx` and `.xlsm`, whatever the case.
pub fn is_excel_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// True for a file that is an Excel workbook: its name ends in `.xls`, `.xlsx` or `.xlsm` and its
/// first bytes are those of a zip or of a compound file. A tab separated or HTML text saved as
/// `.xls` (instruments and web pages do this) is not one; it is read as text. A file that cannot be
/// read is not one either, so that the text import reports the error.
pub fn is_excel_workbook(path: &Path) -> bool {
    use std::io::Read;
    if !is_excel_file(path) {
        return false;
    }
    let mut magic = [0u8; 4];
    let Ok(mut file) = std::fs::File::open(path) else { return false };
    let n = file.read(&mut magic).unwrap_or(0);
    magic[n..].fill(0);
    &magic[..2] == b"PK" || magic == OLE_MAGIC
}

/// The first bytes of a compound file, the container of `.xls`.
const OLE_MAGIC: [u8; 4] = [0xD0, 0xCF, 0x11, 0xE0];

/// How many non-empty rows the header detection looks at (fewer for very wide sheets).
const DETECTION_ROWS: usize = 5000;
/// Cells that the header detection looks at at most.
const DETECTION_CELLS: usize = 2_000_000;
/// The most cells (the area from the first to the last used cell) that one sheet may span. A file
/// that spans more is refused: a single value in `XFD1048576` would otherwise take gigabytes.
const MAX_SHEET_CELLS: u64 = 40_000_000;
/// The most bytes that one part of an `.xlsx` zip may hold once unpacked.
const MAX_PART_BYTES: u64 = 1 << 30;
/// The most bytes that all parts of an `.xlsx` zip may hold once unpacked.
const MAX_UNPACKED_BYTES: u64 = 2 << 30;
/// Parts above this size must not be packed more tightly than `MAX_PACK_RATIO` to one.
const RATIO_FROM_BYTES: u64 = 64 << 20;
const MAX_PACK_RATIO: u64 = 250;
/// What the writer puts in the first cell of a label row that has no label, so that the row exists
/// and the rows below it keep their roles (long name, units, comments). It is a word joiner, which
/// shows nothing in Excel; the reader treats it as an empty cell once the header is analysed.
const EMPTY_LABEL: &str = "\u{2060}";
/// Integers above this lose digits as 64-bit floats.
const EXACT_INTEGER: i64 = 1 << 53;

/// One sheet of a workbook, by name or by position.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SheetRef {
    /// The sheet's name (letter case does not matter).
    Name(String),
    /// The 1-based position among all the sheets of the file, hidden ones included.
    Index(usize),
    /// A whole number typed by a person: the sheet of that name if there is one (sheets named
    /// `2023` are common), otherwise the sheet at that position.
    NameOrIndex(String),
}

impl SheetRef {
    /// Reads `Results` as a name and `2` as a name that falls back to a position.
    pub fn parse(text: &str) -> SheetRef {
        let t = text.trim();
        match t.parse::<usize>() {
            Ok(n) if n > 0 => SheetRef::NameOrIndex(t.to_string()),
            _ => SheetRef::Name(t.to_string()),
        }
    }

    /// Reads a comma separated list such as `1,Results,3`; empty entries are dropped.
    pub fn parse_list(text: &str) -> Vec<SheetRef> {
        text.split(',').map(str::trim).filter(|t| !t.is_empty()).map(SheetRef::parse).collect()
    }
}

/// Settings of [`read_workbook`]. Header lines and rows are counted in Excel's own row numbers:
/// line 1 is the first row of the sheet. A header line of 0 means "there is none".
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ExcelOptions {
    /// The sheets to read, in this order. Empty: every sheet, in the file's order (hidden ones
    /// only when `include_hidden` is set). A sheet named here is read even when it is hidden.
    pub sheets: Vec<SheetRef>,
    /// Read hidden sheets too when `sheets` is empty. Default true, as Origin's `impExcel` imports
    /// hidden sheets (import-excel.md 2).
    pub include_hidden: bool,
    /// Number of rows before the first data row. `None` detects it, as for text files.
    pub header_lines: Option<usize>,
    /// 1-based row holding the column long names (`None` detects).
    pub long_name_line: Option<usize>,
    /// 1-based row holding the units (`None` detects).
    pub units_line: Option<usize>,
    /// 1-based row holding the comments (`None` detects).
    pub comments_line: Option<usize>,
    /// The header rows fill long name, units and comments. When false the columns keep empty labels.
    pub rename_columns: bool,
    /// 1-based row whose cells become the short names; 0 keeps `A`, `B`, ...
    pub short_name_line: usize,
    /// Give the book the file name, with its extension, as its long name (as Origin). Default true.
    pub rename_book: bool,
    /// Rows (1-based) whose cells become named parameter rows, see
    /// [`ascii::AsciiOptions::param_lines`].
    pub param_lines: Vec<usize>,
    /// A parameter row takes its name from its first cell.
    pub param_name_from_first_cell: bool,
    /// The first column is X, the others Y. Default true.
    pub first_column_x: bool,
    /// Data rows to skip at the start.
    pub skip_rows: usize,
    /// Stop after this many data rows.
    pub max_rows: Option<usize>,
    /// Data rows to import after the skip and maximum above; positions are 1-based.
    pub rows: ImportSelection,
    /// Columns to import, counted from column A of the sheet; positions are 1-based.
    pub columns: ImportSelection,
    /// Whether Excel dates and times become date and time columns (`Detect`) or text.
    pub dates: DateImport,
}

impl Default for ExcelOptions {
    fn default() -> Self {
        let a = AsciiOptions::default();
        ExcelOptions {
            sheets: Vec::new(),
            include_hidden: true,
            header_lines: a.header_lines,
            long_name_line: a.long_name_line,
            units_line: a.units_line,
            comments_line: a.comments_line,
            rename_columns: a.rename_columns,
            short_name_line: a.short_name_line,
            rename_book: a.rename_book,
            param_lines: a.param_lines,
            param_name_from_first_cell: a.param_name_from_first_cell,
            first_column_x: a.first_column_x,
            skip_rows: a.skip_rows,
            max_rows: a.max_rows,
            rows: a.rows,
            columns: a.columns,
            dates: a.dates,
        }
    }
}

impl ExcelOptions {
    /// A copy that imports every row and column and keeps the rest. Used for the first file of a
    /// partial batch import.
    pub fn without_partial_import(&self) -> Self {
        ExcelOptions {
            rows: ImportSelection::default(),
            columns: ImportSelection::default(),
            skip_rows: 0,
            max_rows: None,
            ..self.clone()
        }
    }

    /// The text-import options that carry the same header and selection settings.
    fn ascii(&self) -> AsciiOptions {
        AsciiOptions {
            header_lines: self.header_lines,
            long_name_line: self.long_name_line,
            units_line: self.units_line,
            comments_line: self.comments_line,
            rename_columns: self.rename_columns,
            short_name_line: self.short_name_line,
            rename_book: self.rename_book,
            param_lines: self.param_lines.clone(),
            param_name_from_first_cell: self.param_name_from_first_cell,
            first_column_x: self.first_column_x,
            skip_rows: self.skip_rows,
            max_rows: self.max_rows,
            rows: self.rows.clone(),
            columns: self.columns.clone(),
            dates: self.dates,
            decimal_comma: false,
            thousands: Thousands::Off,
            date_order: DateOrder::Mdy,
            ..AsciiOptions::default()
        }
    }

    fn validate(&self) -> Result<(), IoError> {
        for (name, selection) in [("row", &self.rows), ("column", &self.columns)] {
            if !selection.valid() {
                return Err(IoError::InvalidOption(format!(
                    "the {name} selection needs first and read counts above 0, and last must be 0 or at least first"
                )));
            }
        }
        Ok(())
    }
}

/// What the importer decided about one sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetLayout {
    /// Rows before the first data row (Excel row numbers; line 1 is the first row).
    pub header_lines: usize,
    pub long_name_line: usize,
    pub units_line: usize,
    pub comments_line: usize,
    /// 1-based column of the sheet (A is 1) that the first imported column came from.
    pub first_column: usize,
    /// Columns imported.
    pub columns: usize,
    /// Data rows imported.
    pub rows: usize,
}

/// What became of one sheet of the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetReport {
    pub name: String,
    /// 1-based position in the file.
    pub index: usize,
    pub hidden: bool,
    /// False for a sheet that was left out (hidden, no data, a chart sheet); a warning says why.
    pub imported: bool,
    pub layout: Option<SheetLayout>,
}

/// The result of [`read_workbook`].
#[derive(Clone, Debug)]
pub struct ExcelImport {
    /// One workbook with placeholder identifiers (see `Project::adopt_workbook`), one sheet per
    /// imported Excel sheet, named as in Excel. The book's long name is the file name with its
    /// extension, as Origin names it (unless `rename_book` is off).
    pub workbook: Workbook,
    /// One entry per sheet that was looked at, in the order read.
    pub sheets: Vec<SheetReport>,
    /// Sheets left out, cells that could not be read, formulas without a stored value.
    pub warnings: Vec<String>,
}

/// What kind of sheet an Excel sheet is.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SheetKind {
    Worksheet,
    /// A chart on a sheet of its own: it has no cells.
    Chart,
    /// A dialog, macro or VBA sheet.
    Other,
}

/// A sheet of a file, as [`list_sheets`] reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetInfo {
    pub name: String,
    /// 1-based position in the file.
    pub index: usize,
    pub kind: SheetKind,
    pub hidden: bool,
}

/// The first rows of a sheet as text, for a dialog. See [`preview_sheet`].
#[derive(Clone, Debug)]
pub struct SheetPreview {
    pub name: String,
    /// 1-based Excel row of `rows[0]`.
    pub first_row: usize,
    /// 1-based Excel column of the first cell of every row (empty columns before it are left out).
    pub first_column: usize,
    /// The cells as text: numbers in shortest form, dates as `yyyy-mm-dd hh:mm:ss`, errors empty.
    pub rows: Vec<Vec<String>>,
    /// What the importer would do with this sheet; `None` when it has no data.
    pub layout: Option<SheetLayout>,
}

fn open(path: &Path) -> Result<Sheets<std::io::BufReader<std::fs::File>>, IoError> {
    let mut magic = [0u8; 4];
    {
        use std::io::Read;
        let mut f = std::fs::File::open(path).map_err(|e| IoError::io(path, e))?;
        let n = f.read(&mut magic).map_err(|e| IoError::io(path, e))?;
        magic[n..].fill(0);
    }
    let fail = |e: &dyn std::fmt::Display| IoError::ExcelRead { path: path.to_path_buf(), reason: e.to_string() };
    // The bytes decide, not the extension: a `.xls` that is really a zip (or the other way round)
    // is common. `.xlsb` and `.ods` are zips too and keep their own readers.
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if &magic[..2] == b"PK" {
        check_archive(path).map_err(|e| fail(&e))?;
    }
    if &magic[..2] == b"PK" && !matches!(ext.as_str(), "xlsb" | "ods") {
        let book: calamine::Xlsx<_> = calamine::open_workbook(path).map_err(|e| fail(&e))?;
        return Ok(Sheets::Xlsx(book));
    }
    if magic == OLE_MAGIC {
        let book: calamine::Xls<_> = calamine::open_workbook(path).map_err(|e| fail(&e))?;
        return Ok(Sheets::Xls(book));
    }
    calamine::open_workbook_auto(path).map_err(|e| match e {
        calamine::Error::Io(io) => IoError::io(path, io),
        other => fail(&other),
    })
}

/// Refuses a zip whose parts unpack to far more than they weigh (a "zip bomb") before any part is
/// read. The sizes are the ones the archive declares.
fn check_archive(path: &Path) -> Result<(), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    // An archive that cannot be opened is left to the workbook reader for its message.
    let Ok(mut zip) = zip::ZipArchive::new(file) else { return Ok(()) };
    let mut total = 0u64;
    for i in 0..zip.len() {
        let Ok(part) = zip.by_index_raw(i) else { continue };
        let (size, packed) = (part.size(), part.compressed_size().max(1));
        total = total.saturating_add(size);
        if size > MAX_PART_BYTES
            || total > MAX_UNPACKED_BYTES
            || (size > RATIO_FROM_BYTES && size / packed > MAX_PACK_RATIO)
        {
            return Err(format!(
                "the part '{}' unpacks to {} MB, which is far more than a workbook holds; the file is refused",
                part.name().map(|n| n.into_owned()).unwrap_or_default(),
                size >> 20
            ));
        }
    }
    Ok(())
}

/// Runs a reader step and turns a panic inside the workbook reader (a damaged file can make it
/// index out of range) into an error that names the file.
fn guarded<T>(path: &Path, step: impl FnOnce() -> Result<T, IoError>) -> Result<T, IoError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(step)).unwrap_or_else(|_| {
        Err(IoError::ExcelRead {
            path: path.to_path_buf(),
            reason: "the file is damaged and cannot be read".to_string(),
        })
    })
}

/// One xlsx sheet as a range, read cell by cell so that the area it spans is known before the
/// range is allocated.
fn xlsx_range<RS: std::io::Read + std::io::Seek>(
    book: &mut calamine::Xlsx<RS>,
    name: &str,
) -> Result<Range<Data>, String> {
    let mut reader = match book.worksheet_cells_reader(name) {
        Ok(r) => r,
        Err(calamine::XlsxError::NotAWorksheet(_)) => return Ok(Range::default()),
        Err(e) => return Err(e.to_string()),
    };
    let mut cells: Vec<Cell<Data>> = Vec::new();
    let mut bounds = Bounds::default();
    while let Some(cell) = reader.next_cell().map_err(|e| e.to_string())? {
        if matches!(cell.get_value(), calamine::DataRef::Empty) {
            continue;
        }
        let pos = cell.get_position();
        bounds.add(pos);
        bounds.check()?;
        cells.push(Cell::new(pos, Data::from(cell.get_value().clone())));
    }
    Ok(Range::from_sparse(cells))
}

/// The formula texts of one xlsx sheet, with the same area check as [`xlsx_range`].
fn xlsx_formulas<RS: std::io::Read + std::io::Seek>(
    book: &mut calamine::Xlsx<RS>,
    name: &str,
) -> Result<Range<String>, String> {
    let Ok(mut reader) = book.worksheet_cells_reader(name) else { return Ok(Range::default()) };
    let mut cells: Vec<Cell<String>> = Vec::new();
    let mut bounds = Bounds::default();
    while let Some(cell) = reader.next_formula().map_err(|e| e.to_string())? {
        if cell.get_value().is_empty() {
            continue;
        }
        bounds.add(cell.get_position());
        bounds.check()?;
        cells.push(cell);
    }
    Ok(Range::from_sparse(cells))
}

/// The area from the first to the last cell seen.
struct Bounds {
    first: (u32, u32),
    last: (u32, u32),
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds { first: (u32::MAX, u32::MAX), last: (0, 0) }
    }
}

impl Bounds {
    fn add(&mut self, (r, c): (u32, u32)) {
        self.first = (self.first.0.min(r), self.first.1.min(c));
        self.last = (self.last.0.max(r), self.last.1.max(c));
    }

    fn check(&self) -> Result<(), String> {
        let rows = u64::from(self.last.0 - self.first.0) + 1;
        let cols = u64::from(self.last.1 - self.first.1) + 1;
        if rows * cols > MAX_SHEET_CELLS {
            return Err(format!(
                "its used area spans {rows} rows by {cols} columns, more than the {} million cells that can be read",
                MAX_SHEET_CELLS / 1_000_000
            ));
        }
        Ok(())
    }
}

/// The cells of one sheet. An xlsx sheet is read sparsely and refused when its area is too large;
/// the other formats cannot span more than a few million cells.
fn sheet_range<RS: std::io::Read + std::io::Seek>(book: &mut Sheets<RS>, name: &str) -> Result<Range<Data>, String> {
    match book {
        Sheets::Xlsx(x) => xlsx_range(x, name),
        other => other.worksheet_range(name).map_err(|e| e.to_string()),
    }
}

fn sheet_formulas<RS: std::io::Read + std::io::Seek>(book: &mut Sheets<RS>, name: &str) -> Option<Range<String>> {
    match book {
        Sheets::Xlsx(x) => xlsx_formulas(x, name).ok(),
        other => other.worksheet_formula(name).ok(),
    }
}

/// The sheets of a file in order, without reading their cells.
pub fn list_sheets(path: &Path) -> Result<Vec<SheetInfo>, IoError> {
    guarded(path, || list_sheets_unguarded(path))
}

fn list_sheets_unguarded(path: &Path) -> Result<Vec<SheetInfo>, IoError> {
    let book = open(path)?;
    Ok(book
        .sheets_metadata()
        .iter()
        .enumerate()
        .map(|(i, s)| SheetInfo {
            name: s.name.clone(),
            index: i + 1,
            kind: match s.typ {
                SheetType::WorkSheet => SheetKind::Worksheet,
                SheetType::ChartSheet => SheetKind::Chart,
                _ => SheetKind::Other,
            },
            hidden: s.visible != SheetVisible::Visible,
        })
        .collect())
}

fn resolve(sheet: &SheetRef, names: &[String]) -> Result<usize, IoError> {
    let by_name = |name: &str| names.iter().position(|n| n.eq_ignore_ascii_case(name.trim()));
    let by_position = |n: usize| n.checked_sub(1).filter(|i| *i < names.len());
    let found = match sheet {
        SheetRef::Index(n) => by_position(*n),
        SheetRef::Name(name) => by_name(name),
        SheetRef::NameOrIndex(text) => by_name(text).or_else(|| text.trim().parse().ok().and_then(by_position)),
    };
    found.ok_or_else(|| {
        let shown = match sheet {
            SheetRef::Index(n) => format!("number {n}"),
            SheetRef::Name(n) => format!("'{n}'"),
            SheetRef::NameOrIndex(n) => format!("named or numbered '{n}'"),
        };
        IoError::InvalidOption(format!("the file has no sheet {shown}; its sheets are: {}", names.join(", ")))
    })
}

/// Reads sheets of an Excel file into one workbook.
///
/// Sheets that cannot give data (hidden ones by default, chart sheets, empty sheets) are left out
/// with a warning. It is an error when no sheet gives any data.
pub fn read_workbook(path: &Path, opts: &ExcelOptions) -> Result<ExcelImport, IoError> {
    guarded(path, || read_workbook_unguarded(path, opts))
}

fn read_workbook_unguarded(path: &Path, opts: &ExcelOptions) -> Result<ExcelImport, IoError> {
    opts.validate()?;
    let mut book = open(path)?;
    let meta = book.sheets_metadata().to_vec();
    let names: Vec<String> = meta.iter().map(|m| m.name.clone()).collect();
    let (chosen, explicit): (Vec<usize>, bool) = if opts.sheets.is_empty() {
        ((0..meta.len()).collect(), false)
    } else {
        let mut v = Vec::new();
        for s in &opts.sheets {
            let i = resolve(s, &names)?;
            if !v.contains(&i) {
                v.push(i);
            }
        }
        (v, true)
    };
    let ao = opts.ascii();
    let file_name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut sheets = Vec::new();
    let mut reports = Vec::new();
    let mut warnings = Vec::new();
    for i in chosen {
        let m = &meta[i];
        let hidden = m.visible != SheetVisible::Visible;
        let mut report = SheetReport { name: m.name.clone(), index: i + 1, hidden, imported: false, layout: None };
        if m.typ != SheetType::WorkSheet {
            warnings.push(format!(
                "sheet '{}' is a chart, dialog or macro sheet without cells and was not imported",
                m.name
            ));
            reports.push(report);
            continue;
        }
        if hidden && !explicit && !opts.include_hidden {
            warnings
                .push(format!("sheet '{}' is hidden and was not imported (include hidden sheets to read it)", m.name));
            reports.push(report);
            continue;
        }
        let range = match sheet_range(&mut book, &m.name) {
            Ok(r) => r,
            Err(e) => {
                warnings.push(format!("sheet '{}' could not be read and was not imported: {e}", m.name));
                reports.push(report);
                continue;
            }
        };
        if let Err(e) = check_size(&range) {
            warnings.push(format!("sheet '{}' was not imported: {e}", m.name));
            reports.push(report);
            continue;
        }
        let formulas = sheet_formulas(&mut book, &m.name);
        let styles = if has_dates(&range) { book.worksheet_style(&m.name).ok() } else { None };
        match read_sheet(&range, formulas.as_ref(), styles.as_ref(), &m.name, &ao) {
            Ok(done) => {
                warnings.extend(done.warnings.into_iter().map(|w| format!("sheet '{}': {w}", m.name)));
                report.imported = true;
                report.layout = Some(done.layout);
                sheets.push(done.sheet);
            }
            Err(why) => warnings.push(format!("sheet '{}' was not imported: {why}", m.name)),
        }
        reports.push(report);
    }
    if sheets.is_empty() {
        let detail = if warnings.is_empty() { "the file has no sheets".to_string() } else { warnings.join("; ") };
        return Err(IoError::NoData { source_name: path.display().to_string(), detail });
    }
    let mut workbook = Workbook::new(WindowId(0), "Book1");
    if opts.rename_book {
        workbook.long_name = file_name;
    }
    workbook.sheets = sheets;
    workbook.source = Some(ImportSource { path: path.display().to_string(), importer: "excel".to_string() });
    Ok(ExcelImport { workbook, sheets: reports, warnings })
}

/// Refuses a range that spans more cells than [`MAX_SHEET_CELLS`] (the xls, xlsb and ods readers
/// build the range before it can be asked).
fn check_size(range: &Range<Data>) -> Result<(), String> {
    let (rows, cols) = (range.height() as u64, range.width() as u64);
    if rows * cols > MAX_SHEET_CELLS {
        return Err(format!(
            "its used area spans {rows} rows by {cols} columns, more than the {} million cells that can be read",
            MAX_SHEET_CELLS / 1_000_000
        ));
    }
    Ok(())
}

/// The first `max_rows` rows of one sheet as text, with what the importer would make of it, for a
/// dialog that lets the user pick the header rows.
pub fn preview_sheet(
    path: &Path,
    sheet: &SheetRef,
    opts: &ExcelOptions,
    max_rows: usize,
) -> Result<SheetPreview, IoError> {
    guarded(path, || preview_sheet_unguarded(path, sheet, opts, max_rows))
}

fn preview_sheet_unguarded(
    path: &Path,
    sheet: &SheetRef,
    opts: &ExcelOptions,
    max_rows: usize,
) -> Result<SheetPreview, IoError> {
    opts.validate()?;
    let mut book = open(path)?;
    let names = book.sheet_names();
    let i = resolve(sheet, &names)?;
    let name = names[i].clone();
    let range =
        sheet_range(&mut book, &name).map_err(|reason| IoError::ExcelRead { path: path.to_path_buf(), reason })?;
    check_size(&range).map_err(|reason| IoError::ExcelRead { path: path.to_path_buf(), reason })?;
    let layout = read_sheet(&range, None, None, &name, &opts.ascii()).ok().map(|d| d.layout);
    let view = View::new(&range);
    let Some(used) = view.grid() else {
        return Ok(SheetPreview { name, first_row: 1, first_column: 1, rows: Vec::new(), layout });
    };
    let end = (used.row0 + max_rows).min(used.row0 + used.rows);
    let rows = (used.row0..end).map(|r| view.shown_row(&used, r)).collect();
    Ok(SheetPreview { name, first_row: used.row0 + 1, first_column: used.col0 + 1, rows, layout })
}

// ----- one sheet ---------------------------------------------------------------------

struct SheetRead {
    sheet: Worksheet,
    layout: SheetLayout,
    warnings: Vec<String>,
}

/// The cells of a sheet by absolute position (0-based row and column of the sheet).
struct View<'a> {
    rows: Vec<&'a [Data]>,
    row_start: usize,
    col_start: usize,
}

/// The part of a sheet that holds anything: absolute first row and column and the size.
struct Used {
    row0: usize,
    col0: usize,
    rows: usize,
    cols: usize,
}

static EMPTY: Data = Data::Empty;

fn has_content(d: &Data) -> bool {
    match d {
        Data::Empty | Data::Error(_) => false,
        Data::String(s) => !s.trim().is_empty(),
        _ => true,
    }
}

fn has_dates(range: &Range<Data>) -> bool {
    range.used_cells().any(|(_, _, d)| matches!(d, Data::DateTime(_)))
}

impl<'a> View<'a> {
    fn new(range: &'a Range<Data>) -> View<'a> {
        let (row_start, col_start) = range.start().map_or((0, 0), |(r, c)| (r as usize, c as usize));
        View { rows: range.rows().collect(), row_start, col_start }
    }

    fn cell(&self, row: usize, col: usize) -> &Data {
        row.checked_sub(self.row_start)
            .and_then(|r| self.rows.get(r))
            .and_then(|r| col.checked_sub(self.col_start).and_then(|c| r.get(c)))
            .unwrap_or(&EMPTY)
    }

    /// The bounding box of the cells that hold something (errors and blank text do not count).
    fn used(&self) -> Option<Used> {
        let (mut first_row, mut last_row) = (usize::MAX, 0);
        let (mut first_col, mut last_col) = (usize::MAX, 0);
        for (i, row) in self.rows.iter().enumerate() {
            let Some(first) = row.iter().position(has_content) else { continue };
            let last = row.iter().rposition(has_content).unwrap_or(first);
            first_row = first_row.min(i);
            last_row = i;
            first_col = first_col.min(first);
            last_col = last_col.max(last);
        }
        (first_row != usize::MAX).then(|| Used {
            row0: self.row_start + first_row,
            col0: self.col_start + first_col,
            rows: last_row - first_row + 1,
            cols: last_col - first_col + 1,
        })
    }

    /// The grid as Origin imports it: leading empty columns are kept, so it starts at column A
    /// (import-excel.md 2, probe `p4` `a2`). Rows above the first used row are still left out.
    fn grid(&self) -> Option<Used> {
        self.used().map(|u| Used { row0: u.row0, col0: 0, rows: u.rows, cols: u.col0 + u.cols })
    }

    fn render_row(&self, used: &Used, row: usize) -> Vec<String> {
        (0..used.cols).map(|c| render(self.cell(row, used.col0 + c))).collect()
    }

    /// The row as a dialog shows it: a placeholder label is an empty cell.
    fn shown_row(&self, used: &Used, row: usize) -> Vec<String> {
        let mut cells = self.render_row(used, row);
        cells.iter_mut().filter(|c| *c == EMPTY_LABEL).for_each(String::clear);
        cells
    }
}

/// The text of a cell for header detection and labels. Dates are ISO text, durations numbers.
fn render(d: &Data) -> String {
    match cx(d, &mut Tally::default()) {
        Cx::Empty | Cx::Error => String::new(),
        Cx::Num(n) => number_text(n),
        Cx::Text(s) => s.to_string(),
        Cx::Date(c) => date_text(c),
    }
}

/// A number the way a label shows it: whole numbers without a point, others in shortest form.
fn number_text(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{n:.0}") } else { format!("{n:?}") }
}

/// A date cell as text, ISO style.
fn date_text(c: Converted) -> String {
    match c.kind {
        DateKind::Date => datetime::format_date(c.value, DatePattern::Iso),
        DateKind::DateTime => datetime::format_date_time(c.value, DatePattern::Iso, TimePattern::Hms),
        DateKind::Time => datetime::format_time(c.value, TimePattern::Hms),
        DateKind::Duration => number_text(c.value),
    }
}

/// A cell as the importer sees it.
#[derive(Clone, Copy)]
enum Cx<'a> {
    Empty,
    /// An error value (`#DIV/0!`): a missing value.
    Error,
    Num(f64),
    /// Text, trimmed; never empty.
    Text(&'a str),
    Date(Converted),
}

/// Counts of what the cell mapping noticed.
#[derive(Default)]
struct Tally {
    errors: usize,
    wide_integers: usize,
    leap_bug_dates: usize,
}

fn cx<'a>(d: &'a Data, tally: &mut Tally) -> Cx<'a> {
    match d {
        Data::Empty => Cx::Empty,
        Data::Int(i) => {
            if i.unsigned_abs() > EXACT_INTEGER as u64 {
                tally.wide_integers += 1;
            }
            Cx::Num(*i as f64)
        }
        Data::Float(f) if f.is_finite() => Cx::Num(*f),
        Data::Float(_) => Cx::Empty,
        Data::Bool(b) => Cx::Num(f64::from(u8::from(*b))),
        Data::String(s) => {
            let t = s.trim();
            if t.is_empty() { Cx::Empty } else { Cx::Text(t) }
        }
        Data::DateTime(dt) => match dates::from_excel(dt) {
            Some(c) => {
                tally.leap_bug_dates += usize::from(c.leap_bug);
                Cx::Date(c)
            }
            None => Cx::Empty,
        },
        Data::DateTimeIso(s) => match parse_iso(s) {
            Some(c) => Cx::Date(c),
            None => Cx::Text(s.trim()),
        },
        Data::DurationIso(s) => Cx::Text(s.trim()),
        Data::Error(_) => {
            tally.errors += 1;
            Cx::Error
        }
    }
}

/// An ISO 8601 date, time or date-time as found in `t="d"` cells.
fn parse_iso(s: &str) -> Option<Converted> {
    let t = s.trim().trim_end_matches('Z').replacen('T', " ", 1);
    let p = datetime::parse(&t, DateOrder::Mdy)?;
    let kind = match p.format {
        NumberFormat::Date(_) => DateKind::Date,
        NumberFormat::Time(_) => DateKind::Time,
        _ => DateKind::DateTime,
    };
    Some(Converted::from_text(p.value, kind))
}

/// A row that holds one cell, in the first used column (`lead`) of a sheet with several used
/// columns.
fn is_title(fields: &[String], lead: usize, columns: usize) -> bool {
    columns > lead + 1
        && fields.get(lead).is_some_and(|f| !f.is_empty() && f != EMPTY_LABEL)
        && fields.iter().enumerate().all(|(i, f)| i == lead || f.is_empty())
}

/// "C5" for 0-based row 4 and column 2.
fn address(row: usize, col: usize) -> String {
    let mut letters = Vec::new();
    let mut n = col + 1;
    while n > 0 {
        letters.push(b'A' + ((n - 1) % 26) as u8);
        n = (n - 1) / 26;
    }
    letters.reverse();
    format!("{}{}", String::from_utf8_lossy(&letters), row + 1)
}

fn read_sheet(
    range: &Range<Data>,
    formulas: Option<&Range<String>>,
    styles: Option<&StyleRange>,
    name: &str,
    ao: &AsciiOptions,
) -> Result<SheetRead, String> {
    let view = View::new(range);
    let lead = view.used().ok_or("it holds no data")?.col0;
    let used = view.grid().ok_or("it holds no data")?;
    // The grid starts at column A, so a sheet whose cells lie far to the right is counted from there.
    if (used.rows as u64) * (used.cols as u64) > MAX_SHEET_CELLS {
        return Err(format!(
            "counted from column A its cells span {} rows by {} columns, more than the {} million cells that can be read",
            used.rows,
            used.cols,
            MAX_SHEET_CELLS / 1_000_000
        ));
    }
    let ns = NumStyle::new(false, Thousands::Off);

    // The rows that header detection looks at: the first non-empty rows, padded to the width of
    // the used part so that every row has the same field count (as the lines of a CSV file that
    // Excel saves do).
    let explicit = [ao.header_lines.unwrap_or(0), ao.long_name_line.unwrap_or(0), ao.units_line.unwrap_or(0)]
        .into_iter()
        .chain([ao.comments_line.unwrap_or(0), ao.short_name_line])
        .chain(ao.param_lines.iter().copied())
        .max()
        .unwrap_or(0);
    let window = (DETECTION_CELLS / used.cols.max(1)).clamp(100, DETECTION_ROWS);
    let end = (used.row0 + used.rows).min((used.row0 + window).max(explicit + 1));
    let records: Vec<(usize, Vec<String>)> = (used.row0..end)
        .map(|r| (r, view.render_row(&used, r)))
        .filter(|(_, f)| f.iter().any(|c| !c.is_empty()))
        .collect();
    let layout = ascii::find_layout(&records, ns, ao);
    // The top row of a sheet with a single cell, in the first column, is a title (a merged title
    // row reads this way). It keeps one field, so it is never taken for a row of labels; the data
    // start found above does not change, only the label lines are looked for again.
    let (records, layout) = match records.first() {
        Some((r, f)) if is_title(f, lead, used.cols) && layout.start.is_some() && *r < layout.header_lines => {
            let mut titled = records.clone();
            titled[0].1.truncate(lead + 1);
            let ao = AsciiOptions { header_lines: Some(layout.header_lines), ..ao.clone() };
            let again = ascii::find_layout(&titled, ns, &ao);
            (titled, again)
        }
        _ => (records, layout),
    };
    let start = layout.start.ok_or("it has no data rows below the header rows")?;
    let first_data = records[start].0;

    let last_row = used.row0 + used.rows;
    // Blank rows between data rows stay as rows of missing values, so a row keeps its position
    // against the other columns and sheets; only rows above the first data row and below the last
    // used row are not data.
    let kept: Vec<usize> = ascii::window_rows(first_data..last_row, ao).collect();
    // A column counts when a data row or one of the label rows uses it: a column that has a name
    // but no value (formulas without a stored value) is still a column.
    let label_rows = if ao.rename_columns {
        [layout.long_name_line, layout.units_line, layout.comments_line]
            .into_iter()
            .filter_map(|n| n.checked_sub(1))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let source_count = kept
        .iter()
        .chain(&label_rows)
        .map(|&r| (0..used.cols).rposition(|c| has_content(view.cell(r, used.col0 + c))).map_or(0, |p| p + 1))
        .max()
        .unwrap_or(0);
    let row_lines = ascii::pick_rows(kept, ao);
    let source_columns = ascii::pick_columns(source_count, ao);
    if row_lines.is_empty() {
        return Err("it has no data rows".to_string());
    }
    if source_columns.is_empty() {
        return Err("no columns match the import selection".to_string());
    }
    let first_column = used.col0 + source_columns[0] + 1;
    // The label rows are found; a placeholder label is an empty one from here on.
    let records: Vec<(usize, Vec<String>)> = records
        .into_iter()
        .map(|(r, f)| (r, f.into_iter().map(|c| if c == EMPTY_LABEL { String::new() } else { c }).collect()))
        .collect();
    let a = ascii::Analysis::for_grid(records, &layout, row_lines, source_columns, ns);

    let mut tally = Tally::default();
    let (mut sheet, mut warnings) = ascii::assemble(&a, ao, name.to_string(), |j, short, a, warnings| {
        let col = used.col0 + a.source_columns[j];
        let cells: Vec<Cx> = a.row_lines.iter().map(|&r| cx(view.cell(r, col), &mut tally)).collect();
        let code = a
            .row_lines
            .iter()
            .zip(&cells)
            .find(|(_, c)| matches!(c, Cx::Date(_)))
            .and_then(|(&r, _)| styles?.get((r as u32, col as u32))?.number_format.as_ref())
            .map(|f| f.format_code.as_str());
        build_column(&cells, short, ao.dates, code, warnings)
    });
    if tally.errors > 0 {
        warnings.push(format!("{} cells hold errors (#DIV/0!, #N/A, ...); they are missing values", tally.errors));
    }
    if tally.wide_integers > 0 {
        warnings.push(format!("{} integers are larger than 2^53 and lose their last digits", tally.wide_integers));
    }
    if tally.leap_bug_dates > 0 {
        warnings.push(format!(
            "{} cells hold Excel's non-existent date 1900-02-29; they were read as 1900-02-28",
            tally.leap_bug_dates
        ));
    }
    if let Some(f) = formulas {
        keep_formulas(&mut sheet, f, &view, used.col0, &a, &mut warnings);
    }
    if let Some(f) = formulas {
        let (row0, col0) = f.start().map_or((0, 0), |(r, c)| (r as usize, c as usize));
        let missing: Vec<(usize, usize)> = f
            .used_cells()
            .filter(|(_, _, text)| !text.is_empty())
            .map(|(r, c, _)| (row0 + r, col0 + c))
            .filter(|&(r, c)| matches!(view.cell(r, c), Data::Empty))
            .collect();
        if let Some(&(r, c)) = missing.first() {
            warnings.push(format!(
                "{} formula cells have no stored value and are missing values (the first is {}); open the file in Excel and save it to store the values",
                missing.len(),
                address(r, c)
            ));
        }
    }
    let layout = SheetLayout {
        header_lines: a.detected.header_lines,
        long_name_line: a.detected.long_name_line,
        units_line: a.detected.units_line,
        comments_line: a.detected.comments_line,
        first_column,
        columns: a.detected.columns,
        rows: a.row_lines.len(),
    };
    Ok(SheetRead { sheet, layout, warnings })
}

/// Columns whose data cells all hold one same-row Excel formula keep it as their column formula (T417, see
/// [`formulas`]); a column with formulas that cannot be kept gets a warning and keeps its stored values only.
fn keep_formulas(
    sheet: &mut Worksheet,
    f: &Range<String>,
    view: &View<'_>,
    used_col0: usize,
    a: &ascii::Analysis,
    warnings: &mut Vec<String>,
) {
    if sheet.columns.len() != a.source_columns.len() {
        return;
    }
    let (row0, col0) = f.start().map_or((0, 0), |(r, c)| (r as usize, c as usize));
    let text_at = |r: usize, c: usize| -> Option<&str> {
        let (r, c) = (r.checked_sub(row0)?, c.checked_sub(col0)?);
        f.get((r, c)).map(String::as_str).filter(|t| !t.trim().is_empty())
    };
    let places: Vec<usize> = a.source_columns.iter().map(|s| used_col0 + s).collect();
    let shorts: Vec<String> = sheet.columns.iter().map(|c| c.short_name.clone()).collect();
    let name_of = |c: usize| places.iter().position(|&p| p == c).map(|j| shorts[j].clone());
    for (j, &col) in places.iter().enumerate() {
        let with: Vec<(usize, &str)> = a.row_lines.iter().filter_map(|&r| text_at(r, col).map(|t| (r, t))).collect();
        if with.is_empty() {
            continue;
        }
        let plain_values = a.row_lines.iter().any(|&r| text_at(r, col).is_none() && has_content(view.cell(r, col)));
        let mut kept: Option<String> = None;
        let same = !plain_values
            && with.iter().all(|&(r, t)| match formulas::translate(t, r, &name_of) {
                Some(e) => kept.get_or_insert_with(|| e.clone()) == &e,
                None => false,
            });
        match kept.filter(|_| same) {
            Some(expr) => {
                let c = &mut sheet.columns[j];
                c.formula = Some(expr);
                c.recalc = sheaf_core::column::RecalcMode::Auto;
            }
            None => {
                // T464: each cell keeps its own formula as a cell formula where Sheaf can read it.
                let data_row = |r: usize| a.row_lines.iter().position(|&x| x == r);
                let column_id = sheet.columns[j].id;
                let mut missed = None;
                for &(r, t) in &with {
                    match (formulas::translate_cell(t, &data_row, &name_of), data_row(r)) {
                        (Some(formula), Some(row)) => sheet.cell_formulas.push(sheaf_core::cell_formula::CellFormula {
                            column: column_id,
                            row,
                            formula,
                        }),
                        _ => {
                            missed.get_or_insert((r, t));
                        }
                    }
                }
                if let Some((r, t)) = missed {
                    warnings.push(format!(
                        "column {}: some of its Excel formulas (the first is {}: ={}) cannot be read as Sheaf formulas; those cells keep only their stored values",
                        shorts[j],
                        address(r, col),
                        t.trim().trim_start_matches('=')
                    ));
                }
            }
        }
    }
}

/// A column whose cells are all dates or times: a date (or date-time) column when the number format
/// shows a date, or, without a format, when some cell is a date of day 1 or later; otherwise a time
/// column. The format settles what a cell below one day means: a moment on Excel's day 0 in a date
/// column, a time of day in a time column; a time column also takes counts over one day. `None`
/// when a cell cannot be read that way (a length of time among dates, text dates among times).
fn date_column(cells: &[Cx], code: Option<&str>) -> Option<ascii::BuiltColumn> {
    let (date, time) = code.map_or((None, None), dates::patterns_from_code);
    let calendar = match (date, time) {
        (Some(_), _) => true,
        (None, Some(_)) => false,
        (None, None) => {
            cells.iter().any(|c| matches!(c, Cx::Date(d) if matches!(d.kind, DateKind::Date | DateKind::DateTime)))
        }
    };
    let mut values = Vec::with_capacity(cells.len());
    let format = if calendar {
        let mut with_time = false;
        for c in cells {
            values.push(if let Cx::Date(d) = c {
                let (v, kind) = d.as_calendar()?;
                with_time |= kind == DateKind::DateTime;
                v
            } else {
                f64::NAN
            });
        }
        let pattern = date.unwrap_or(DatePattern::Iso);
        if with_time {
            NumberFormat::DateTime(pattern, time.unwrap_or(TimePattern::Hms))
        } else {
            NumberFormat::Date(pattern)
        }
    } else {
        for c in cells {
            values.push(if let Cx::Date(d) = c { d.as_clock()? } else { f64::NAN });
        }
        NumberFormat::Time(time.unwrap_or(TimePattern::Hms))
    };
    Some(ascii::BuiltColumn { data: ColumnData::numeric(values), format, text: false })
}

/// Turns the cells of one column into data: a date or time column when every cell is a date (or
/// every cell a time), a Text & Numeric column with the date display when the other cells are text
/// (as Origin), otherwise numbers with text cells kept as Text & Numeric, or a text column when
/// there is no number.
fn build_column(
    cells: &[Cx],
    short: &str,
    dates: DateImport,
    code: Option<&str>,
    warnings: &mut Vec<String>,
) -> ascii::BuiltColumn {
    let filled = cells.iter().filter(|c| !matches!(c, Cx::Empty | Cx::Error)).count();
    let date_count = cells.iter().filter(|c| matches!(c, Cx::Date(_))).count();
    if date_count > 0 && dates == DateImport::Detect {
        let plain_numbers = cells.iter().any(|c| matches!(c, Cx::Num(_)));
        if !plain_numbers && let Some(built) = date_column(cells, code) {
            if date_count == filled {
                return built;
            }
            // Dates among text: as Origin (import-excel.md 2, probe p11), the dates keep their
            // numbers and the column its date display, and the text cells stay text.
            let ColumnData::Numeric(numbers) = built.data else { unreachable!("date_column gives numbers") };
            let texts = cells
                .iter()
                .enumerate()
                .filter_map(|(i, c)| if let Cx::Text(s) = c { Some((i, (*s).to_string())) } else { None })
                .collect();
            let numbers = std::sync::Arc::unwrap_or_clone(numbers);
            return ascii::BuiltColumn { data: ColumnData::mixed(numbers, texts), format: built.format, text: false };
        }
        let others = if plain_numbers { "numbers" } else { "other values" };
        warnings.push(format!(
            "column {short} mixes dates or times with {others}; the dates and times were imported as text"
        ));
    }
    let mut numbers = vec![f64::NAN; cells.len()];
    let mut texts: BTreeMap<usize, String> = BTreeMap::new();
    let mut numeric_cells = 0usize;
    for (i, c) in cells.iter().enumerate() {
        match c {
            Cx::Num(n) => {
                numbers[i] = *n;
                numeric_cells += 1;
            }
            Cx::Text(s) => {
                texts.insert(i, (*s).to_string());
            }
            Cx::Date(d) => {
                texts.insert(i, date_text(*d));
            }
            Cx::Empty | Cx::Error => {}
        }
    }
    if numeric_cells > 0 || texts.is_empty() {
        ascii::BuiltColumn { data: ColumnData::mixed(numbers, texts), format: NumberFormat::General, text: false }
    } else {
        let mut column = vec![String::new(); cells.len()];
        for (i, t) in texts {
            column[i] = t;
        }
        ascii::BuiltColumn { data: ColumnData::text(column), format: NumberFormat::General, text: true }
    }
}
