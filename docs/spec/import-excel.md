# Excel import and export (xls, xlsx, xlsm)

Status: implemented (task T30, 2026-11-14, macOS); Origin's import measured on Windows on 2026-11-14
and 2026-11-15 (section 2). Source tags: **[F]** measured in Origin 2016 (black-box runs of the
`impExcel` X-Function through `tools/origin-ref/run-labtalk-lines.ps1`; probe files, scripts and
outputs named in each entry are kept in `re-workspace/scratch/t30/`, local only); **[W]** public
OriginLab documentation, paraphrased; **[S]** a choice made by Sheaf; **[?]** not measured (section
10). Where Origin was not measured, Sheaf uses the text import's defaults
([`import-ascii.md`](import-ascii.md), sections 3, 9, 10 and 13) and says so here.

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

## 2. What Origin does (measured) [F]

Recent Origin versions import Excel through a data connector [W]; Origin 2016, the version on the
measuring machine, has the `impExcel` X-Function, which File > Import > Excel runs. The probes are
synthetic workbooks written by Python `openpyxl` (generators `gen.py`, `genfix.py` and `gen9.py`),
imported with `impExcel` one Origin at a time; each script is `p<n>.lines.txt` with its output
`p<n>.out.txt`. The probes p1 to p8 were run by session 5a83f9b6 on 2026-11-14 for its own Excel
branch (`excel-io`, not merged; `main` took the macOS implementation), p9 and p10 on 2026-11-15.

The X-Function lists these variables (name, default): `excel` (use the Excel COM component, 1),
`cellfmts` (import cell formats, 1), `emptycols` (0), `excludeempty` (exclude empty sheets, 1),
`mainheader` (0), `subheader` (-1), `sname`, `lname`, `unit`, `cmt1`, `cmt2`, `para1`, `para2`,
`udp1`, `udp2` (all 0), `removerow` (1), `desig`, `allsheets` (0), `sheet`, `firstmode` (0),
`impmode` (3). Its menu text says it reads Excel 97-2003 files directly and Excel 2007 and later
files through Excel's COM component (`impExcel.OXF` of the installation).

| Fact | Origin | Probe | Sheaf |
|---|---|---|---|
| Reading `.xlsx` | needs Excel: with `excel:=0` an `.xlsx` gives no sheet at all; with the default it works because Excel is installed | `p3` (`v5`) | reads every format itself (ADR 0012) |
| Book names | long name is the file name with its extension (`a1_c.xlsx`); short name is the file name without the characters that are not letters or digits (`a1c`); the book comment is the full path | `p2`, `p8` (`m4`) | long name as Origin; the short name follows the project's naming (`Book1`, ...) and the path is kept in `workbook.source`, not in a comment |
| Sheets | one worksheet per Excel sheet, named like it, in file order | `p2` | same |
| Hidden sheet | imported | `p4` (`a3`) | same (`--skip-hidden-sheets` leaves them out) |
| Empty sheet, and a sheet that holds only a chart | not imported (`excludeempty` 1) | `p4` (`a3`) | same, with a warning |
| Leading empty rows and columns | kept: a table that starts in C4 comes in with empty columns A and B and three empty rows above it | `p4` (`a2`) | columns kept; the rows above the first used row are not data (section 5) |
| Merged cells | the value is in the first cell, the others are empty | `p4` (`a2`) | same |
| Designation | first column X, all others Y, a column of text too | `p2`, `p4` | first column X; a column without numbers is Label, as in the text import |
| Column format | every column Text & Numeric (`wks.col.format` 7), date columns too | `p4`, `p9` | numeric, Text & Numeric, text, or a date or time column (section 4) |
| Short names | `A`, `B`, ... for the first 26, then Origin's own scheme (column 1200 of a 1200-column sheet is `F1195`); all 1200 columns imported | `p6` (`a4`) | the text import's names (`AA`, `AB`, ...); all columns imported |
| Numbers | cell values as written (`0.30002999999999996` shows every digit of the double) | `p3` | same, bit for bit |
| Text in a numeric column | stays a text cell (`n.d.`) in a Text & Numeric column | `p3` | same |
| Booleans | numbers: TRUE is 1, FALSE is 0 | `p9`, `p10` (`b1`) | same |
| Error cells (`#N/A`, `#DIV/0!`) | missing values (`-1.23456789E-300`, shown empty) | `p9`, `p10` (`b1`) | same, with a warning that counts them |
| Dates | numbers in Origin's date system (`2461041` for 2026-01-01, `2461100.5210069446` for 2026-03-01 12:30:15), shown as `01/01/2026 00:00:00`, month first, in a Text & Numeric column | `p9`, `p10` (`b1`) | the same numbers, in a Date or Date-Time column |
| The 1900 leap-year bug | ignored: Excel's 1900-01-01 becomes 2415019, shown 12/31/1899, one day early (Origin adds a fixed offset to Excel's serial, which is right only from 1900-03-01 on) | `p9`, `p10` (`b1`) | Excel's date: 2415020, 1900-01-01 (section 7) |
| 1904 date system | applied: 1904-01-02 is 2416481 | `p9`, `p10` (`b2`) | same |
| Time of day | the fraction of a day as a plain number (`0.25` for 06:00:00) | `p9`, `p10` (`b1`) | the same number in a Time column |
| Duration (`[h]:mm:ss`) | the count of days (`1.5`) | `p9`, `p10` (`b1`) | same, in a Time column |
| Formula cells | the value Excel computes when Origin opens the file through it: the stored results of the probe file (10.5, 21, ...) were replaced by the recalculated sums; a formula that gives an error is an empty cell | `p3` (`v1`) | the stored value; nothing is recalculated (section 4) |
| `cellfmts:=0` | no difference in the values read back | `p3` | - |
| `sheet:="Second"` or `sheet:="2"` | did not restrict the import from LabTalk: both sheets were imported | `p3` (`v3`, `v4`) | `--sheets` restricts the import |
| Header rows | none by default: the first rows are data and the labels stay empty (2.1) | `p3`, `p4`, `p6`, `p7`, `p8` | detected, as in the text import (section 5) |

`crates/io/tests/fixtures/excel/origin-kinds.xlsx` holds the cells of `b1`; the test
`booleans_errors_dates_and_times_give_the_numbers_origin_gives` compares Sheaf's numbers with
Origin's.

### 2.1 Header rows

Without any header setting the first rows are data: a row of names is a row of text cells, and the
long name, units and comments stay empty (`p3`, `v1`). With the settings:

- The header is `mainheader` rows followed by `subheader` rows; both are left out of the data.
  `subheader` -1 (the default) is "as many as the highest label row needs" (`p6`: main 3, sub
  automatic, long name row 1 to 3 gives 6 header rows and the data start on row 7; `p7` `k5`).
- `sname`, `lname`, `unit`, `cmt1` and `cmt2` are 1-based positions among the sub-header rows,
  counted after the main header rows (`p7`: main 2, sub 4, long name 1, units 2, comments 3 gives
  rows 3, 4 and 5 of the sheet; main 0 gives rows 1, 2 and 3).
- Several comment rows (`cmt1` to `cmt2`) are joined with line breaks (`p7` `k6`, `k7`).
- A position beyond the sub-header rows is ignored, and with `subheader:=0` no label is read at all
  (`p7`, `p8`).
- `sname` makes the cell the short name of the column (`p8` `m1`).
- Parameter rows (`para1`, `para2`, `udp1`, `udp2`): the import ran, but reading the user parameters
  back from LabTalk gave nothing [?].

Sheaf's header options are the text import's absolute Excel row numbers (`header_lines`,
`long_name_line`, `units_line`, `comments_line`, `short_name_line`): Origin's `mainheader` *m* with
`lname` *k* is Sheaf's long-name line *m* + *k*, and *m* + `subheader` header rows are
`header_lines`. Sheaf detects the header rows unless told (Origin reads none); several comment rows
are not joined [S].

### 2.2 Export, not measurable from LabTalk

Origin 2016's worksheet export to Excel is not an X-Function of the installation: `expWks` is the
image export (its menu text says so), and the only Excel-related `.OXF` is `impExcel`. `save -i`
wrote an Origin window file (`o_i.xlsx.ogw`), and the variants `save -x`, `-xb`, `-xs`, `-e` and
`-w` stopped the run on a dialog (`p5`), so the export's sheet names, header rows and missing values
are not measured [?]. Sheaf writes the label rows of the text export (section 9).

## 3. Sheaf's choices

Each row says whether it copies a measurement of section 2 **[F]** or is Sheaf's own choice **[S]**.
The section number says where the detail is.

| Topic | Sheaf's choice | Section |
|---|---|---|
| Header rows | [S] The text import's detection and defaults, run on the sheet's rows (padded to equal width, as the lines of a CSV file saved by Excel are); Origin reads no header rows unless told (2.1). Excel row numbers are the line numbers. | 5 |
| Which sheets | [F] Every worksheet, hidden ones too, in file order (`--skip-hidden-sheets` or unticking leaves hidden ones out). Chart sheets and sheets without cells are skipped with a warning. [S] A typed whole number is a sheet name if a sheet has that name, otherwise a position. | 6 |
| Book and sheet names | [F] Book long name is the file name with its extension (`rename_book`; the text import uses the name without it); sheets keep their Excel names. | 6 |
| Leading empty rows and columns | [F] Empty leading columns are kept, so the column letters are Excel's and the column selection counts from column A. [S] Empty leading rows are not imported (Origin keeps them as rows, since it reads no header), but row numbers stay Excel's. | 5 |
| Blank rows inside the data | Kept as rows of missing values, so a row keeps its place against the other columns and sheets. Rows above the first data row and below the last used row are not data (an export of a sheet that starts or ends with blank rows warns). | 5 |
| Top title row | A single cell in the first column of the top row of a multi-column sheet (a merged title) is a title and never a label row. | 5 |
| Booleans | [F] Numbers, TRUE is 1 and FALSE is 0. | 4 |
| Error cells | [F] Missing values, counted in a warning. | 4 |
| Text among numbers | As the text import (Text & Numeric column). | 4 |
| Dates | Date, Time and Date-Time columns; patterns from the Excel number format when the file is `.xlsx`, ISO date and `HH:mm:ss` otherwise. The number format (or, without one, the values) decides per column whether a cell below one day is a time of day or a moment on day 0. | 7 |
| 1900 leap-year bug | [S] Followed: Excel day 60 does not exist and is read as 1900-02-28, with a warning; dates before 1900-03-01 are the dates Excel shows (Origin's are one day early, section 2). | 7 |
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
| Boolean | Number 1 or 0 | As Origin [F]. A column of booleans is a numeric column. |
| Text | Text | Blanks at both ends are trimmed (as the text import trims fields); a text of only blanks is an empty cell. |
| Empty cell | Missing value | A numeric cell is NaN; a text cell is an empty string. |
| Error (`#DIV/0!`, `#N/A`, `#REF!`, ...) | Missing value | As Origin [F]. A warning counts them per sheet. The text import keeps such strings as text in a numeric column; a file cannot tell an error from text there, Excel can. |
| Formula | The value Excel stored with it | When the file holds no stored value (a file written by a program that does not calculate), the cell is missing and a warning gives the number of such cells and the first one (`B2`). |
| Date, time, duration | Date, Time or Date-Time column | Section 7. |
| ISO text date in a `t="d"` cell | Date | Read like a date cell. |
| Merged range | Top-left value only | Other cells of the range are empty; the merge itself is not kept. |

**Columns.** A column whose cells are all dates (or all times) is a date (time) column. Otherwise, a
column with at least one number is a Text & Numeric column, so a text cell (`n.d.`, `<LOD`) stays a
text cell and numeric views see it as missing; a column with no number is a text column. This is the
rule of the text import ("a cell that is not a number stays a text cell"; no ratio of text to
numbers). **Dates among text** (a date column with an `n.d.` cell) make a Text & Numeric column with
the date display: the dates keep their numbers and the text cells stay text, as Origin keeps them
(probe `p11`) [F]. **Dates among plain numbers are imported as text** (the dates as `yyyy-mm-dd` or
`yyyy-mm-dd HH:mm:ss`), with a warning [S]: Origin keeps both as numbers with a display format per
cell, which a Sheaf column does not have. Designations follow the text import:
the first column is X (`first_column_x`), numeric columns are Y, text columns Label.

A column that has a name in a label row but no value in any data row (formulas without a stored
value, for instance) is kept as an empty numeric column [S]. Trailing columns with neither are not
columns.

**Formulas kept as column formulas (T417, Origin 2026's formula-preserving import) [S].** A column whose
data rows that hold a value all hold a formula, and whose formulas are one formula written for their own
row (`B2 =A2*2`, `B3 =A3*2`, ...), keeps it as its Set Column Values formula in Sheaf's language
(`col(A)*2`, the short names of the imported columns) with Recalculate Auto; its values stay as Excel
stored them (missing where none was stored) until an input changes. What can be read: numbers,
`+ - * / ^`, parentheses, same-row references to imported columns (`A2`, `$A2`) and SQRT, EXP, LN,
LOG10, ABS, SIN, COS, TAN, ASIN, ACOS, ATAN, SINH, COSH, TANH, ROUND and PI(); a unary minus next to
`^` is refused (Excel's `-2^2` is 4).

**Other formulas kept as cell formulas (T464) [S].** In a column whose formulas are not one same-row formula
(another row, a `$` row, a range such as a running `SUM($A$2:A2)`, formulas that differ by row, a typed
value among them), each cell's formula that Sheaf can read becomes that cell's cell formula
(workbook-model.md 13u): references name the imported columns (their short names, which must be capital
letters) and the data rows counted from 1 below the name rows (`$A$2` in a sheet with one name row is
`$A$1`); `$` markers are kept; a range must stay in one column; the functions above plus SUM (the total),
AVERAGE (`mean`), MIN and MAX, and (T499) the comparisons `= <> < > <= >=` (written `== != < > <= >=`), IF,
MEDIAN, STDEV and STDEV.S (`sd`), STDEVP and STDEV.P (`stddevp`), VAR and VAR.S (`var`), POWER (`pow`), TRUNC
(`int`), SIGN, INT (`floor`: Excel's INT rounds down) and MOD (`rmod`: the divisor's sign). The stored values stay
until the next edit recalculates the cell formulas. A formula that cannot be read (a cell in a name row or outside
the imported columns, another sheet, text, other functions such as COUNT, VLOOKUP, AND) keeps only its stored value, with a warning that names the column and its
first such formula (`column D: some of its Excel formulas (the first is D2: =A2*$G$1) cannot be read as
Sheaf formulas; those cells keep only their stored values`). `sheaf_io::excel::formulas`.

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
  used range, when it holds a single cell in the first used column and the sheet has several used
  columns, is a title: it is not a label row (a warning says one header line was not imported) and the label lines
  below it are found as if it were not there. Anywhere else a single filled cell is an ordinary row.
  A one-column sheet has no title row.
- **Leading empty rows and columns.** Columns left of the first used cell are kept as empty columns,
  as Origin keeps them [F], so data in `C4:D9` stay columns `C` and `D` behind empty `A` and `B`, and
  Sheaf's column letters are Excel's. Column selection (`columns`) counts from column A, and the
  sheet's report gives `first_column` (the first imported column, 1 unless a selection starts later).
  Rows above the first used row are not imported (their numbers still count) [S]: Origin keeps them,
  but it also reads no header rows by default. Since the grid is counted from column A, a sheet whose
  rows times columns from A1 are above the 40 million cell limit of section 8 is refused, with the
  sheet's name.
- **Blank rows in the data** [S]. A row with no value in any column, between the first data row and the
  last used row, is a data row of missing values (the text import drops blank lines; a sheet cannot,
  because a row number is a position that other columns and sheets share). Rows above the first data
  row, and below the last row that holds anything, are not data. An export that starts or ends with
  such rows says so, since they do not come back.
- **Partial import** [S]. `skip_rows`, `max_rows`, `rows` (first, last, read, skip) and `columns` work
  as in the text import ([`import-ascii.md`](import-ascii.md) section 9), except that the rows counted
  are all rows from the first data row to the last used row, blank ones included, and columns are counted from column A; columns are discovered before the rows are selected, so a column does not
  disappear because the chosen rows are empty in it. The first file of a batch can be read in full
  (`ExcelOptions::without_partial_import`).
- **Not offered** for Excel because they are about characters: delimiter, quote, decimal and thousands
  separators, block splitting. A text cell `1,5` stays text.

## 6. Sheets and books

- One Sheaf book per Excel file, one sheet per imported Excel sheet, in the order of `sheets` (or of
  the file). Book long name: the file name with its extension, as Origin [F]. Sheet names are the Excel names (at most
  31 characters, the same limit as Sheaf's). `workbook.source` records the path and the importer
  `excel`.
- **Selection.** `ExcelOptions::sheets` lists sheets by name (case-insensitive) or 1-based position
  among all sheets (`SheetRef::parse_list("1,Results")`). A whole number typed by a person
  (`SheetRef::NameOrIndex`) is the sheet of that name when there is one (sheets called `2023` or `3`
  are common) and the sheet at that position otherwise; `SheetRef::Index` is always a position (the
  desktop window uses it). An unknown name or position is an error that lists the sheets. Empty means
  all.
- **Hidden sheets** (hidden and "very hidden") are read, as Origin reads them [F]
  (`include_hidden`, on by default); with it off they are skipped with a warning, and a hidden sheet
  named in `sheets` is still read.
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
  otherwise a time column. A length of time among dates, or a date among times, still makes a
  text column with a warning.
- **1900 and 1904 systems.** The file's date-system flag (`date1904`, or the BIFF record in `.xls`)
  is applied by the reader. The same moment in either system gives the same Sheaf number.
- **The 1900 leap-year bug.** Excel counts 1900 as a leap year, so day 60 is a 1900-02-29 that never
  existed.
  Sheaf follows Excel's display: day 1 is 1900-01-01, day 59 is 1900-02-28, day 61 is 1900-03-01. Day
  60 has no date; it is read as 1900-02-28 with a warning [S]. Origin does not follow it: it adds a
  fixed offset to the serial, so its dates before 1900-03-01 are one day early (1900-01-01 is
  12/31/1899, section 2) [F]; from 1900-03-01 on Sheaf's and Origin's numbers are the same. On export, dates from 1900-01-01 on are
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
are not kept); the shape of merged ranges; formulas other than one same-row formula per column (section 4;
the others give only their stored values); hidden
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

## 10. Measurements: done and left

Done on Windows (Origin 2016, section 2): header rows with and without settings (1), leading empty
rows and columns (2), merged cells (3), text in a numeric column (4), booleans and error cells (5),
dates, times, durations, the 1904 system, the 1900 leap-year cells and a date column with a text cell
(6), formulas (7: Origin opens the file through Excel, so even a formula without a stored value gets
Excel's result), hidden and chart-only sheets (8), several sheets and the book's names (9), a sheet of
1200 columns (10). Left [?]:

1. **Origin's import window.** File > Import > Excel's dialog (its pages, defaults and the sheet
   list) was not opened; only the X-Function was run from LabTalk. `tools/origin-ref/origin-dialog.ps1
   -MenuClicks` can open it.
2. **A title row under Origin's header settings** (a merged title above the label rows with
   `mainheader` 1), and Origin's handling of a `sheet` restriction from its dialog (from LabTalk it
   did not restrict the import).
3. **Display of Excel's number formats.** Origin showed every date as `MM/dd/yyyy HH:mm:ss` whatever
   the cell's format code (`yyyy-mm-dd`, `dd/mm/yyyy`); whether `cellfmts` or the dialog change that is
   not known. Sheaf takes the pattern from the code (section 7).
4. **Partial import.** `impExcel` of Origin 2016 has no row or column range variables; the dialog's
   partial import, if any, is not measured. Sheaf's rows and columns follow the text import (section 5).
5. **User parameter rows** (`para1`, `udp1`): the readback gave nothing (2.1).
6. **Export** (2.2): needs the GUI (File > Export > Excel in a recent Origin, or Excel itself); the
   sheet names, label rows, missing values and Date columns Origin writes are unknown.

**The owner's workbooks** (step 6 of the brief, 2026-11-15, local only, no values kept): three
workbooks found on the Windows machine (five sheets, up to 36 000 rows and 24 columns) were imported
with `--header-lines 0`, written back as CSV and compared cell by cell with `openpyxl`
(`data_only=True`): all 307 707 number cells agree to 1e-9 relative, all 124 text
cells and every empty cell agree, and the 39 `#DIV/0!` cells are missing values with the warning that
counts them, as specified (section 4). The files hold no dates, booleans or hidden sheets.

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
  contains a comma cannot be given. A sheet named here is read even when hidden. Without a list
  hidden sheets are read, as in Origin; `--skip-hidden-sheets` leaves them out (`--hidden-sheets`, the
  default, is kept for older scripts and cannot be combined with it). All three are ignored for text
  files.
- The header and selection flags of the text import apply to Excel with Excel's own row numbers:
  `--header-lines`, `--long-name-line`, `--units-line`, `--comments-line`, `--short-name-line`,
  `--param-lines`, `--no-rename-columns`, `--no-rename-book`, `--first-row`, `--last-row`,
  `--read-rows`, `--skip-rows`, `--first-column`, `--last-column`, `--read-columns`,
  `--skip-columns` and `--dates-as-text`. The separator, number-format and block flags (`--delimiter*`,
  `--fixed-width`, `--decimal-comma`, `--thousands`, `--day-first`, `--blocks-as-sheets`, `--sparklines`,
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
- MCP: `import_files` takes `excel: {sheets: [...], include_hidden}` next to `ascii` (hidden sheets
  are read whether `include_hidden` is given or not; MCP has no switch to leave them out yet; its header
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
  with default options (every sheet, hidden ones too, detected header rows). A drop of Excel and text files
  together opens both windows. Excel imports do not make a graph by default (the checkbox on the Import
  Mode page turns it on), whereas text files from the command line are plotted as before.
- **Import window** (`crates/app/src/dialogs/import_excel.rs`), laid out like the ASCII window: a file
  list at the left top, a tree of pages and, below, a preview. Pages: *Sheets* (a checkbox per sheet,
  Select all, Clear; every worksheet is on, hidden sheets too (marked "(hidden)"), as Origin imports
  them; chart and
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
  file) and re-import.
- **Dialog theme (T46).** The window gets the Dialog Theme row of the text window: `<default>`, `<last used>`
  and saved names, with Save, Save As... and Delete. Origin's Excel window has not been seen, so the row is
  **[S]** (Sheaf's choice, not measured), as in `import-ascii.md` section 17. The theme holds the whole of
  `ExcelOptions` except `sheets` (the ticked sheets belong to the files, not to a theme), the import mode, the
  partial-import tick and the graph tick, in the `excel` store (`import-themes/excel.json`). OK records
  `<last used>`; Cancel does not.

Tests: `crates/app/src/app/ui_tests/excel.rs`.

## 12. Tests that guard this

`crates/io/src/excel/tests.rs`, with the fixtures in `crates/io/tests/fixtures/excel/` (generated by
`tools/make-excel-fixtures.py`, each below 10 kB, made up):

- `a_workbook_with_header_rows_imports_numbers_text_ints_and_dates` (`basic.xlsx`),
  `a_formula_cell_gives_its_stored_value_and_one_without_gives_a_warning`,
  `a_same_row_formula_column_keeps_its_formula`
- `formulas_that_are_not_one_same_row_formula_become_cell_formulas` (`formulas.xlsx`, T417/T464), and the
  translation rules in `crates/io/src/excel/formulas.rs`
- `a_legacy_xls_file_reads_like_an_xlsx_file` (`legacy.xls`)
- `the_1904_date_system_gives_the_same_dates_as_the_1900_system` (`date1904.xlsx`),
  `excels_1900_leap_year_bug_is_followed_and_the_missing_day_is_reported` (`leap1900.xlsx`)
- `merged_cells_keep_their_top_left_value_and_a_merged_title_is_not_a_label_row` (`merged.xlsx`)
- `hidden_sheets_are_read_as_origin_reads_them_unless_left_out` (`hidden.xlsx`),
  `sheets_that_hold_only_a_chart_are_skipped_with_a_warning` (`charts.xlsx`)
- `text_among_numbers_stays_text_and_booleans_errors_and_big_integers_are_mapped` (`mixed.xlsx`),
  `data_that_starts_below_and_right_of_the_corner_keeps_the_empty_columns_as_origin` (`leading.xlsx`),
  `a_merged_title_over_data_right_of_column_a_is_still_a_title`,
  `a_sheet_counted_from_column_a_above_the_cell_limit_is_refused`,
  `dates_among_plain_numbers_become_text_with_a_warning`
- Origin's numbers: `booleans_errors_dates_and_times_give_the_numbers_origin_gives`
  (`origin-kinds.xlsx`, the cells of Origin's probe `b1`)
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
