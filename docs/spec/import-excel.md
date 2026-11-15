# Excel import and export (xls, xlsx, xlsm)

Status: first implementation (task T30, 2026-11-14), written on a machine without Origin. **Nothing
in this file about Origin's Excel behaviour has been measured.** Source tags: **[W]** public OriginLab
documentation, paraphrased; **[S]** a choice made by Sheaf, Origin unmeasured; **[?]** unknown, to be
measured on the Windows machine (section 10). Where the brief says "measure", Sheaf uses the text
import's defaults ([`import-ascii.md`](import-ascii.md), sections 3, 9, 10 and 13) and says so here.

Decisions on libraries are in [ADR 0012](../adr/0012-excel-import-export.md).

## 1. Scope

- **In:** File > Import > Excel (XLS, XLSX, XLSM) reading one or more sheets of a workbook into one
  Sheaf book; the header rows become Long Name, Units and Comments; `sheaf import` and the drop of an
  Excel file on the window go the same way (`import_file` reads Excel by extension). File > Export
  writes a book or a sheet as `.xlsx`.
- **Out:** `File > Open Excel` (Ctrl+E), which opens the file as a live Excel window inside Origin and
  needs Excel itself. The menu item stays grey. `.xlsb` and `.ods` are not offered, although the
  library could read them. Charts, styles and the rest of section 8 are not read. Import of Excel
  files as a live link (re-import on change) is not part of this task.
- **Crate API** (`sheaf_io::excel`): `read_workbook(path, &ExcelOptions) -> ExcelImport`,
  `list_sheets(path)`, `preview_sheet(path, &SheetRef, &ExcelOptions, max_rows)`,
  `write_xlsx(&[&Worksheet], path, &ExcelExportOptions) -> ExportOutcome`,
  `write_workbook_xlsx(&Workbook, path, &ExcelExportOptions)`, `is_excel_file(path)`;
  `sheaf_io::import_file_with_excel` and `sheaf_io::batch::import_batch_with_excel` carry the options
  through the existing import and batch entry points.

## 2. Origin facts (unmeasured) [W]

From Origin's public documentation, paraphrased; none of it checked against a running Origin:

- File > Import > Excel uses the Excel data connector in recent versions; older versions and scripts
  use the `impExcel` X-Function.
- One book per Excel file, one sheet per Excel sheet. All sheets by default, or a chosen set.
- Header lines are chosen as main header and subheader rows, which map to Long Name, Units and
  Comments, as in the text import.
- A formula cell imports its cached value (the value Excel stored with it).
- A date cell becomes a Date column.
- A partial import selects rows and columns.
- Export to Excel is `expXLS` or the connector's save.

## 3. Sheaf's choices [S]

Each of these is a choice, not a measurement. The section number says where the detail is.

| Topic | Sheaf's choice, Origin unmeasured | Section |
|---|---|---|
| Header rows | The text import's detection and defaults, run on the sheet's rows (padded to equal width, as the lines of a CSV file saved by Excel are). Excel row numbers are the line numbers. | 5 |
| Which sheets | All visible worksheets, in file order. Hidden sheets are skipped with a warning unless asked for (a flag, or a sheet named in the list). Chart sheets and sheets without cells are skipped with a warning. A typed whole number is a sheet name if a sheet has that name, otherwise a position. | 6 |
| Book and sheet names | Book long name is the file name without extension (`rename_book`, as for text); sheets keep their Excel names. | 6 |
| Leading empty rows and columns | Empty leading rows are not imported but row numbers stay Excel's; empty leading columns are dropped, so the first used column becomes column `A` and the column selection counts from it. | 5 |
| Blank rows inside the data | Kept as rows of missing values, so a row keeps its place against the other columns and sheets. Rows above the first data row and below the last used row are not data (an export of a sheet that starts or ends with blank rows warns). | 5 |
| Top title row | A single cell in the first column of the top row of a multi-column sheet (a merged title) is a title and never a label row. | 5 |
| Booleans | Numbers, TRUE is 1 and FALSE is 0. | 4 |
| Error cells | Missing values, counted in a warning. | 4 |
| Text among numbers | As the text import (Text & Numeric column). | 4 |
| Dates | Date, Time and Date-Time columns; patterns from the Excel number format when the file is `.xlsx`, ISO date and `HH:mm:ss` otherwise. The number format (or, without one, the values) decides per column whether a cell below one day is a time of day or a moment on day 0. | 7 |
| 1900 leap-year bug | Followed: Excel day 60 does not exist and is read as 1900-02-28, with a warning. | 7 |
| Export labels | Long Name, Units and Comments rows when some column has that label, with an empty placeholder row for a label that comes before one that is written; a Short Name row only on request. | 9 |
| Export numeric formats | Date, time and a few number formats become Excel number formats; the rest is General. | 9 |
| Text named `.xls` | A file named `.xls`, `.xlsx` or `.xlsm` whose first bytes are neither a zip nor a compound file is read as text, with a warning. | 6 |
| Damaged and hostile files | A damaged file is an error, not a crash; a sheet that spans more than 40 million cells, or a zip that unpacks to far more than it weighs, is refused. | 8 |
| Existing file | Replaced only when it is an `.xlsx` workbook; any other file is left alone and the export fails. | 9 |

## 4. Cell-type mapping

What a cell holds in the file decides its type; the column type follows from its cells.

| Cell in the file | Sheaf cell | Notes |
|---|---|---|
| Number (floating point) | Number, bit for bit | Every digit survives (`0.1 + 0.2`, `5e-324`, `1.8e308`). |
| Integer (an `.xls` RK number, or an `.xlsx` cell with an integer text) | Number | Exact up to 2^53; above that a warning says the last digits are lost (Sheaf numbers are 64-bit floats). |
| Boolean | Number 1 or 0 | [S] A column of booleans is a numeric column. |
| Text | Text | Blanks at both ends are trimmed (as the text import trims fields); a text of only blanks is an empty cell. |
| Empty cell | Missing value | A numeric cell is NaN; a text cell is an empty string. |
| Error (`#DIV/0!`, `#N/A`, `#REF!`, ...) | Missing value | A warning counts them per sheet. [S] The text import keeps such strings as text in a numeric column; a file cannot tell an error from text there, Excel can. |
| Formula | The value Excel stored with it | When the file holds no stored value (a file written by a program that does not calculate), the cell is missing and a warning gives the number of such cells and the first one (`B2`). |
| Date, time, duration | Date, Time or Date-Time column | Section 7. |
| ISO text date in a `t="d"` cell | Date | Read like a date cell. |
| Merged range | Top-left value only | Other cells of the range are empty; the merge itself is not kept. |

**Columns.** A column whose cells are all dates (or all times) is a date (time) column. Otherwise, a
column with at least one number is a Text & Numeric column, so a text cell (`n.d.`, `<LOD`) stays a
text cell and numeric views see it as missing; a column with no number is a text column. This is the
rule of the text import ("a cell that is not a number stays a text cell"; no ratio of text to
numbers). **A column that mixes dates or times with other values is imported as text** (the dates
as `yyyy-mm-dd` or `yyyy-mm-dd HH:mm:ss`), with a warning [S]. Designations follow the text import:
the first column is X (`first_column_x`), numeric columns are Y, text columns Label.

A column that has a name in a label row but no value in any data row (formulas without a stored
value, for instance) is kept as an empty numeric column [S]. Trailing columns with neither are not
columns.

## 5. Header rows and layout

The importer builds, for each sheet, the same table of fields that a CSV file saved by Excel would
give (every row as wide as the used part of the sheet, cells as text) and runs the text import's
detection on it: `ascii::find_layout` finds the first data row and the Long Name, Units and Comments
lines above it, and `ascii::assemble` builds the columns with their labels, short names and
parameter rows. These are the same functions the text import uses; its tests run unchanged.

- **Line numbers are Excel's row numbers** (line 1 is the first row of the sheet), also when the sheet
  starts with empty rows. `header_lines`, `long_name_line`, `units_line`, `comments_line`,
  `short_name_line` and `param_lines` take them. 0 means none; `None` means detect.
- **Detection window.** The detection looks at the first 5000 non-empty rows (fewer for very wide
  sheets: at most 2 million cells), and always at the rows the options name.
- **Defaults are the text import's** (Long Name, Units, Comments in up to three adjacent lines above
  the data; a lone `Name (unit)` label split into name and unit when no units line exists;
  `rename_columns` on; first column X). Origin's own defaults for the Excel connector are [?].
- **Title row** [S]. A merged title over the header rows is common in lab sheets. The top row of the
  used range, when it holds a single cell in the first column and the sheet has several columns, is a
  title: it is not a label row (a warning says one header line was not imported) and the label lines
  below it are found as if it were not there. Anywhere else a single filled cell is an ordinary row.
  A one-column sheet has no title row.
- **Leading empty rows and columns** [S]. The first used cell sets the corner: rows above it are not
  imported (their numbers still count) and columns left of it are dropped, so data in `C4:D9` become
  columns `A` and `B`. The sheet's report gives `first_column` (3 here) so a dialog can say where the
  columns came from. Column selection (`columns`) counts from the first used column.
- **Blank rows in the data** [S]. A row with no value in any column, between the first data row and the
  last used row, is a data row of missing values (the text import drops blank lines; a sheet cannot,
  because a row number is a position that other columns and sheets share). Rows above the first data
  row, and below the last row that holds anything, are not data. An export that starts or ends with
  such rows says so, since they do not come back.
- **Partial import** [S]. `skip_rows`, `max_rows`, `rows` (first, last, read, skip) and `columns` work
  as in the text import ([`import-ascii.md`](import-ascii.md) section 9), except that the rows counted
  are all rows from the first data row to the last used row, blank ones included; columns are discovered before the rows are selected, so a column does not
  disappear because the chosen rows are empty in it. The first file of a batch can be read in full
  (`ExcelOptions::without_partial_import`).
- **Not offered** for Excel because they are about characters: delimiter, quote, decimal and thousands
  separators, block splitting. A text cell `1,5` stays text.

## 6. Sheets and books

- One Sheaf book per Excel file, one sheet per imported Excel sheet, in the order of `sheets` (or of
  the file). Book long name: the file name without extension. Sheet names are the Excel names (at most
  31 characters, the same limit as Sheaf's). `workbook.source` records the path and the importer
  `excel`.
- **Selection.** `ExcelOptions::sheets` lists sheets by name (case-insensitive) or 1-based position
  among all sheets (`SheetRef::parse_list("1,Results")`). A whole number typed by a person
  (`SheetRef::NameOrIndex`) is the sheet of that name when there is one (sheets called `2023` or `3`
  are common) and the sheet at that position otherwise; `SheetRef::Index` is always a position (the
  desktop window uses it). An unknown name or position is an error that lists the sheets. Empty means
  all.
- **Hidden sheets** (hidden and "very hidden") are skipped with a warning unless `include_hidden` is
  set; a hidden sheet named in `sheets` is read [S].
- **Chart sheets and dialog or macro sheets** are skipped with a warning. A worksheet that holds only
  a chart has no cells and is skipped with a warning.
- It is an error (`NoData`) when no sheet gives data; the message gives the reasons.
- **Batch modes.** In `batch`, an Excel file is read like any file: *new books* gives one book with
  all the sheets, *new sheets* adds every sheet of the file as a sheet of the target, *new columns* and
  *new rows* use the first sheet only and say so. A folder import lists `.xls`, `.xlsx` and `.xlsm`
  files, but not the `~$name.xlsx` lock files that Excel leaves beside an open workbook.
- A file is recognised by its bytes (zip or compound file), not only by its name, so an `.xlsx` saved
  with the extension `.xls` still reads. Anything else gives an error that names the file.
- **Text named like a workbook** [S]. Instruments and web pages save tab separated or HTML text as
  `.xls`. `import_file` (and so the command line, folders and the app) sends a file to the Excel reader
  only when its name is `.xls`, `.xlsx` or `.xlsm` and its first bytes are a zip (`PK`) or a compound
  file (`D0 CF 11 E0`); `sheaf_io::is_excel_workbook` is that test and `is_excel_file` the name test
  alone. Any other such file takes the text path and gets a warning that it was named like a workbook.
  A file that starts like a zip or compound file but is damaged stays an Excel error.

## 7. Dates, times and durations

Excel stores a date as a number of days since an epoch, with the time of day as the fraction. Origin
and Sheaf store days since a different epoch ([`workbook-model.md`](workbook-model.md)); the importer
converts through calendar fields, not through offsets, so both systems and the leap-year bug come out
right.

| Cell | Sheaf column | Value |
|---|---|---|
| Whole-day date | `Date(pattern)` | Sheaf date number at midnight |
| Date with time | `DateTime(date pattern, time pattern)` | Sheaf date number plus the fraction of the day |
| Time of day (value below 1) | `Time(pattern)` | The fraction of a day |
| Duration (`[h]:mm:ss`, value over 1 allowed) | `Time(pattern)` | The count of days (`1.5` is 36:00:00) |
| Negative number with a date or time format | `Time(pattern)` | The negative count of days (Excel shows no date for it; Sheaf keeps the value) |

- **One kind per column** [S]. A cell's value alone cannot say whether 0.25 is 06:00 or the first
  quarter of day 0, so the column decides, from the number format of its first date cell. A format
  that shows a date (`yyyy-mm-dd hh:mm`) makes a date or date-time column: a cell below one day is a
  moment on Excel's day 0 (1899-12-31 in the 1900 system, 1904-01-01 in the 1904 system), so a `0`
  that comes from a formula on a blank cell is 1899-12-31 and not a reason to turn the column into
  text. A format that shows only a time (`h:mm:ss`) makes a time column: its cells are counts of days,
  also over one day (0.25 and 1.5 stay 0.25 and 1.5) and below zero. Without a format (`.xls`, or a
  code the table does not know) the column is a date column when some cell is a date from day 1 on,
  otherwise a time column. A length of time among dates, or a text date among times, still makes a
  text column with a warning.
- **1900 and 1904 systems.** The file's date-system flag (`date1904`, or the BIFF record in `.xls`)
  is applied by the reader. The same moment in either system gives the same Sheaf number.
- **The 1900 leap-year bug.** Excel counts 1900 as a leap year, so day 60 is a 1900-02-29 that never
  existed.
  Sheaf follows Excel's display: day 1 is 1900-01-01, day 59 is 1900-02-28, day 61 is 1900-03-01. Day
  60 has no date; it is read as 1900-02-28 with a warning [S]. On export, dates from 1900-01-01 on are
  written with the same rule; earlier dates cannot be Excel dates and are written as plain numbers with
  a warning.
- **Precision.** Date cells are cut to the millisecond, which is all Excel shows.
- **Patterns.** In `.xlsx` the cell's number format code picks the Sheaf patterns (`yyyy-mm-dd` is ISO,
  `m/d/yyyy` is M/d/yyyy, `dd/mm/yyyy`, `dd-mm-yyyy`, `dd.mm.yyyy`, `d-mmm-yy`, `dddd, mmmm d, yyyy`,
  `ddd`, `yymmdd`, `h:mm:ss`, `hh:mm`, `h:mm AM/PM`, `[h]:mm:ss`, `h:mm:ss.00` and so on; table in
  `excel/dates.rs::patterns_from_code`), taken from the first date cell of the column. In `.xls`, and
  for a code the table does not know, the pattern is ISO `yyyy-MM-dd` and `HH:mm:ss` [S]. (The text
  import defaults to M/d/yyyy for ambiguous text; an Excel date has no ambiguity, so the unambiguous
  ISO form is the default.)
- **Dates kept as text.** `ExcelOptions::dates = DateImport::AsText` imports dates as ISO text.
- Text cells that look like dates stay text; only cells Excel itself holds as dates become dates.

## 8. What is not read

Charts, drawings and pictures; cell styles other than the number format of date cells (and only in
`.xlsx`); fonts, colours, borders, column widths, frozen panes; pivot tables; conditional formats and
data validation; comments and notes; hyperlinks; defined names; VBA (`.xlsm` is read as `.xlsx`, macros
are not kept); the shape of merged ranges; the formulas themselves (only the stored values); hidden
rows and columns are read as if shown; external links are not followed. A sheet that holds only a
chart gives no data and is reported.

**Limits and damaged files** [S]. The reader answers a file it cannot handle with an error that names the
file, never with a crash or an out-of-memory abort:

- A panic inside the workbook reader (a damaged `.xls` can make it index out of range) is caught
  around every reader call (`list_sheets`, `read_workbook`, `preview_sheet`) and becomes the error
  "the file is damaged and cannot be read". Rust's panic message may still appear on the error output.
- An `.xlsx` sheet is read cell by cell and refused, before any space is reserved, when the area from
  its first to its last used cell is above 40 million cells (a single value in `XFD1048576` would
  otherwise need gigabytes); the sheet is skipped with that reason, and a file with no other sheet is
  an error. The same limit is checked after reading `.xls`, `.xlsb` and `.ods` (they cannot hold more
  than a few million cells, but one that does is refused).
- A zip part that unpacks above 1 GiB, all parts above 2 GiB, or a part above 64 MiB that is packed
  more than 250 to 1, makes the file refused ("far more than a workbook holds"). The sizes are the
  ones the archive declares.

## 9. Export (`write_xlsx`)

All of this is Sheaf's choice [S]; Origin's Excel export is [?].

- **Sheets.** One Excel sheet per Sheaf sheet, in order. Names follow Excel's rules: at most 31
  characters counted as UTF-16 units (so at most 15 emoji; a character is never cut in two), none of
  `[ ] : * ? / \`, no apostrophe at either end, not `History`, not empty, and unique ignoring case
  with Unicode's case rules (`run`, `RUN_2`, `Ärger`, `ÄRGER_2`). A changed name is reported in the
  outcome's warnings.
- **Label rows.** Long Name, Units, Comments, each only when some column has it (and its option is on);
  Short Name first when `short_names` is on (off by default). The data start on the next row. A sheet
  with no labels starts with data in row 1. The importer tells the three labels apart by their place
  (up to three rows above the data, in that order), so a label that comes before a written one is
  written too, as a row whose first cell holds only U+2060 (WORD JOINER, invisible in Excel), and the
  importer reads that cell as empty. A sheet with units only is therefore written as an empty-looking
  row and the units row; one with comments only as two such rows and the comments. Every combination of
  the three labels reads back in its own field. (Short Name rows are the exception: read them back with
  `short_name_line`.)
- **Values.** Numbers as numbers with every digit; text as text; missing values as empty cells; the
  text cells of a Text & Numeric column as text. Infinite values cannot be stored and are left empty
  (warning). Texts above 32767 characters are cut (warning).
- **Dates.** A column shown as date, date-time or time is written as Excel serial numbers with a
  matching number format and a wider column. `Time` columns that reach a day or more, or hold a negative value, use `[hh]:mm:ss`.
  `Decimal`, `Decimal with thousands` and `Scientific` display formats become the Excel codes `0.00`,
  `#,##0.00` and `0.00E+00`; other formats are General.
- **Limits.** More than 16384 columns or 1048576 rows (labels included) is an error that says so.
- **Files.** The workbook is built in memory, written to a temporary file in the target folder and
  renamed (`save::write_atomically`). A name without extension gets `.xlsx`; another extension is an
  error. An existing file is replaced only if it is a zip holding `xl/workbook.xml`; otherwise the call
  fails (`ExportExists`) and the file is untouched.
- **Round trip.** What `write_xlsx` writes, `read_workbook` reads back with the same numbers (bit for
  bit), text, missing values (a row with no value between two rows with values stays a row), labels,
  designations (X for the first column, Label for text columns) and date formats. Not kept: blank rows
  before the first or after the last row with a value (a warning says how many), and the display
  patterns Excel cannot tell apart (a weekday letter reads back as the short weekday).

## 10. Measurements to do on the Windows machine

Run on `tools/origin-ref` (one Origin at a time, global lock), on synthetic `.xlsx` files written by
`write_xlsx` or by `tools/make-excel-fixtures.py` into `re-workspace/scratch/t26/`, with `impExcel` (or
the connector's LabTalk equivalent) and `expXLS`. Record each result here with the run's output file
name, and change section 3 where Origin differs. The owner's workbooks are not on the machine that
wrote this file, so the local check of step 6 of the brief was not done; run it where they are.

1. **Header rows.** A sheet with one, two and three label rows (names; names and units; names, units,
   comments) and with none. Which rows does the connector take as Long Name, Units, Comments by
   default; is a header row count asked or detected; what does it do with a title row above the labels
   (plain and merged).
2. **Empty leading rows and columns.** Data starting at C4. Does the book keep A and B as empty
   columns or start at the first used column; do row numbers in the header options count from the sheet
   or from the first used row?
3. **Merged cells.** A merged title; a merged block in the data (is the value repeated, or only in the
   top-left cell).
4. **Text in a numeric column.** `n.d.`, `<LOD`, a word, a blank string: text cell, missing value, or
   the column becomes text? Compare with section 4.
5. **Booleans and error cells.** TRUE/FALSE as 1/0 or as text; `#DIV/0!` and `#N/A` as missing values
   or as text.
6. **Dates and times.** Date, date-time, time-of-day and `[h]:mm:ss` columns: the column format Origin
   gives (display pattern per Excel format code), 1900 and 1904 workbooks, the 1900-02-29 cell, a date
   before 1900-03-01, a date column that contains one text cell. Compare the numbers with Sheaf's
   (section 7).
7. **Formulas.** A formula with a stored value, and one without (written by a program that does not calculate,
   such as `openpyxl`): value, missing, or an error.
8. **Hidden sheets, chart-only sheets.** Are hidden sheets imported by default; what does a chart sheet
   or a worksheet with only a chart give (an empty sheet, no sheet, an error).
9. **Several sheets.** Default sheet selection, sheet names in the book, long name of the book.
10. **Wide sheets.** More than 1000 columns: any limit, column names beyond `Z`.
11. **Partial import** of rows and columns in the connector: counts, 1-based or 0-based, and whether
    they count from the sheet or from the used range.
12. **Export.** The sheet names Origin writes for a book and for a sheet, the header rows (Long Name,
    Units, Comments; their labels in the first column or not), how missing values and text are
    written, how Date columns are written (serial number and number format), and what it does when
    the target file exists.

## 11. Command line and MCP

Both go through `sheaf_ops` (`ReadSettings`, `import_files`, `tabular::write_xlsx`), so they behave
the same. An Excel file is recognised by its extension (`.xls`, `.xlsx`, `.xlsm`, any letter case) and
its first bytes (a zip or compound file; text named `.xls` is read as text) in
`import`, `info`, `plot`, `fit`, `analyze` and `export`; a folder gives its Excel files with its text
files, and `~$` lock files are skipped.

- `sheaf import book.xlsx --into out.sheaf`: one Sheaf workbook per Excel file, one sheet per Excel
  sheet. `--mode sheets` puts every sheet of every file into one workbook; `--mode columns` and
  `--mode rows` use the first Excel sheet of each file and warn (the behaviour of `sheaf_io::batch`).
- `--sheets LIST`: sheet names or 1-based positions counted over all sheets, comma-separated, read in
  the order given (`--sheets "Results,3"`). A number is the sheet of that name if there is one
  (`--sheets 2023` for a sheet called 2023) and the sheet at that position otherwise; a sheet name that
  contains a comma cannot be given. A sheet named here is read even when hidden. `--hidden-sheets`:
  also read the hidden sheets when no list is given. Both are ignored for text files.
- The header and selection flags of the text import apply to Excel with Excel's own row numbers:
  `--header-lines`, `--long-name-line`, `--units-line`, `--comments-line`, `--short-name-line`,
  `--param-lines`, `--no-rename-columns`, `--no-rename-book`, `--first-row`, `--last-row`,
  `--read-rows`, `--skip-rows`, `--first-column`, `--last-column`, `--read-columns`,
  `--skip-columns` and `--dates-as-text`. The separator, number-format and block flags (`--delimiter*`,
  `--fixed-width`, `--decimal-comma`, `--thousands`, `--day-first`, `--blocks-as-sheets`,
  `--no-rename-sheet`) do not apply to a cell grid and are ignored. `--format ascii` forces the text
  reader on an Excel file.
- `sheaf export project.sheaf -o out.xlsx` and `sheaf export-data project.sheaf -o out.xlsx` write a
  workbook: `--book`, `--sheet` pick it (default: the first book, its active sheet),
  `--all-sheets` writes every sheet of the book (not with `--sheet`), `--no-long-names`,
  `--no-units`, `--no-comments` leave a label row out and `--short-names` adds a first row of short
  names (read it back with `--short-name-line 1`). `export` also takes a data file as input (an Excel
  or text file converts to `.xlsx`). Renamed sheets and values Excel cannot hold are printed as
  warnings. A target that is not an `.xlsx` workbook is never replaced; the command fails and the
  file is untouched.
- MCP: `import_files` takes `excel: {sheets: [...], include_hidden}` next to `ascii` (its header
  and row settings apply to Excel too); `export_data` writes `.xlsx` for a path ending in `.xlsx` or
  `format: "xlsx"`, with `all_sheets`. The `plot`, `fit` and `analyze` tools work on the project, so
  they read Excel data after `import_files`.

Tests: `crates/cli/tests/excel.rs`, `an_excel_file_is_imported_and_a_sheet_is_exported_to_xlsx`
in `crates/mcp/tests/hardening.rs`.

## 11b. Desktop application

The window layout, the sheet list and the notes below are Sheaf's own [S]; Origin's Excel import window
has not been seen (section 10).

- **Menus.** File > Import > "Excel (XLS, XLSX, XLSM)..." and File > Export > "Worksheet as Excel..." are
  on. File > Open Excel... (Ctrl+E) and File > New > Excel stay grey (they need Excel itself).
- **Routing.** An Excel file (by extension and first bytes: `sheaf_io::is_excel_workbook`, so a tab
  separated text named `.xls` goes to the ASCII window) dropped on the window, chosen in
  any Import item's file chooser, or given on the command line goes to Excel, never to the ASCII window.
  Dropped or chosen files open the Excel import window; files given on the command line import at once
  with default options (every visible sheet, detected header rows). A drop of Excel and text files
  together opens both windows. Excel imports do not make a graph by default (the checkbox on the Import
  Mode page turns it on), whereas text files from the command line are plotted as before.
- **Import window** (`crates/app/src/dialogs/import_excel.rs`), laid out like the ASCII window: a file
  list at the left top, a tree of pages and, below, a preview. Pages: *Sheets* (a checkbox per sheet,
  Select all, Clear; visible worksheets are on, hidden sheets are marked "(hidden)" and off, chart and
  macro sheets are marked and cannot be ticked), *Import Mode* (the ASCII modes: new books, new sheets,
  replace sheet, new columns, new rows; the target is the active sheet), *Header Rows*, *Column
  Designation*, *Partial Import* and *Miscellaneous* (dates as text, rename the book from the file name).
  The Header Rows and Partial Import controls and the Import Mode page are the same code as the ASCII
  window (`header_line_rows`, `selection_rows`, `mode_and_graph_controls` in `import.rs`). Line numbers
  are Excel's row numbers. The ticked sheets go to the importer by position, so a hidden sheet that is
  ticked is read.
- **Preview.** One sheet at a time (a list picks it): the first 40 rows and 40 columns of the cells as
  Excel holds them, with Excel's column letters and row numbers, and each row tagged Name, Units,
  Comment or Header as the importer will use it. A line above says how many columns and rows come out and
  from which column. The preview follows the header and partial-import options.
- **Result.** The import is one undo step, "Import data", as for text files. The status bar says
  "Imported N file(s), M Excel sheet(s)". Every warning of the reader (skipped hidden or chart sheets,
  formulas without a stored value, text among numbers, dates before 1900) goes to the Messages Log and to
  the "Import notes" box. A file with no ticked sheet is reported by name and not imported.
- **Export window** (`dialogs/export_excel.rs`): the sheets of the active book (all ticked) and the label
  rows to write (Short name off; Long name, Units, Comments on, each written only if some column has
  it). "Save as..." asks for an `.xlsx` name. A target that exists and is not an `.xlsx` workbook is not
  touched; the window stays open and shows the error so another name can be chosen. Warnings of the
  writer (renamed sheets, values Excel cannot hold) go to the Messages Log and a box.
- **Screenshots.** `--show import-excel:FILE` (with `import-excel-page:mode|header|columns|partial|misc`)
  and `--show export-excel`.
- **Not done in the app.** Multi-file preview of the sheets of every file at once (the sheet list is per
  file), an "Import hidden sheets" shortcut, and re-import.

Tests: `crates/app/src/app/ui_tests/excel.rs`.

## 12. Tests that guard this

`crates/io/src/excel/tests.rs`, with the fixtures in `crates/io/tests/fixtures/excel/` (generated by
`tools/make-excel-fixtures.py`, each below 10 kB, made up):

- `a_workbook_with_header_rows_imports_numbers_text_ints_and_dates` (`basic.xlsx`),
  `a_formula_cell_gives_its_stored_value_and_one_without_gives_a_warning`
- `a_legacy_xls_file_reads_like_an_xlsx_file` (`legacy.xls`)
- `the_1904_date_system_gives_the_same_dates_as_the_1900_system` (`date1904.xlsx`),
  `excels_1900_leap_year_bug_is_followed_and_the_missing_day_is_reported` (`leap1900.xlsx`)
- `merged_cells_keep_their_top_left_value_and_a_merged_title_is_not_a_label_row` (`merged.xlsx`)
- `hidden_sheets_are_skipped_unless_asked_for` (`hidden.xlsx`),
  `sheets_that_hold_only_a_chart_are_skipped_with_a_warning` (`charts.xlsx`)
- `text_among_numbers_stays_text_and_booleans_errors_and_big_integers_are_mapped` (`mixed.xlsx`),
  `data_that_starts_below_and_right_of_the_corner_drops_the_empty_rows_and_columns` (`leading.xlsx`)
- options: `sheets_can_be_chosen_by_name_and_by_position_and_a_wrong_name_is_an_error`,
  `header_rows_follow_the_options_as_in_a_text_import`, `rows_and_columns_can_be_imported_in_part`,
  `dates_can_be_kept_as_text`, `a_text_import_and_an_excel_import_of_the_same_table_agree`,
  `a_single_title_cell_is_not_a_label_row_but_two_adjacent_label_rows_are`,
  `a_file_is_read_by_its_bytes_not_its_name`, `sheets_are_listed_and_previewed_for_a_dialog`,
  `sheets_named_with_digits_are_chosen_by_name_before_position`
- writer and round trips: `a_written_workbook_reads_back_with_the_same_values_labels_and_types`,
  `label_rows_are_written_only_when_some_column_has_the_label`,
  `date_and_time_columns_are_written_as_excel_dates_and_read_back_with_their_format`,
  `a_date_before_1900_is_written_as_a_number_with_a_warning_and_infinity_is_left_out`,
  `sheet_names_follow_excels_rules_and_stay_unique`,
  `an_export_never_replaces_a_file_that_is_not_a_workbook`,
  `a_sheet_with_more_columns_than_excel_holds_is_refused`,
  `rows_without_a_value_between_data_rows_stay_rows_of_missing_values`,
  `blank_rows_at_the_start_or_end_of_the_data_are_reported_by_the_writer`,
  `every_combination_of_labels_comes_back_in_its_own_field`,
  `a_date_column_holding_day_zero_or_a_time_of_day_stays_a_date_column`,
  `a_time_column_with_negative_values_round_trips`,
  `weekday_and_compact_date_patterns_come_back_as_they_were_written`,
  `sheet_names_are_cut_by_utf16_units_and_compared_by_unicode_case`
- damaged and hostile files: `a_panic_inside_the_workbook_reader_becomes_an_error_that_names_the_file`,
  `a_legacy_file_with_damaged_bytes_gives_an_error_or_data_but_never_a_panic`,
  `a_sheet_that_spans_a_huge_area_is_refused_before_anything_is_allocated`,
  `a_workbook_whose_parts_unpack_to_far_more_than_they_weigh_is_refused`,
  `text_named_like_a_workbook_is_read_as_text_and_a_damaged_workbook_is_an_error`
- size: `a_sheet_with_more_than_a_thousand_columns_round_trips`,
  `a_sheet_with_a_hundred_thousand_rows_is_read_in_one_pass`
- entry points: `import_file_reads_an_excel_file_by_its_extension_unless_a_kind_is_forced`,
  `excel_files_go_through_the_batch_modes_like_text_files`,
  `a_folder_import_lists_excel_files_but_not_excels_lock_files`
- format codes and serial numbers: `excel_number_formats_map_to_sheaf_patterns`,
  `every_sheaf_date_and_time_pattern_has_an_excel_code_that_maps_back_where_excel_can_show_it`,
  `serial_numbers_follow_the_1900_system_and_come_back`

The text import's own tests (`crates/io/src/ascii/` tests and `crates/io/tests/`) run unchanged and
guard the shared stage.
