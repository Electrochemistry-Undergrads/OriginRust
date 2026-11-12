# Text / CSV data import (clean-room spec)

Status: research draft. Source tags: **[F]** installed file (path given), **[W]** public OriginLab documentation (docs.originlab.com, paraphrased), **[I]** inference by the author, **[?]** unknown, needs a check against running Origin.

## Summary

- Origin's text import is one engine (the "ASCII import") used by two entry points: a general one (`impASC`, file types .txt/.dat/.asc and anything else) and a CSV one (`impCSV`, file type .csv). They share most options but have different defaults (see section 11).
- The engine reads a file in three zones: **main header lines** (free text, not column-aligned), **sub-header lines** (column-aligned: they become long name, units, comments, parameters), then **data rows**.
- By default it auto-detects the delimiter, the number of header and sub-header lines, and each column's type (numeric vs text vs date/time), by looking at up to 50 lines and requiring 4 consistent lines.
- Non-numeric text found in a numeric column is, by default, kept as text (the column becomes text) rather than aborting.
- Imported data goes into a workbook according to an import mode (replace, new columns, new rows, new books, new sheets). The sheet and book names come from the file name.
- Priority for the owner (P0): comma/tab/space/semicolon delimited text with a few header lines (CH Instruments style: free-text header, then a "Potential/V, Current/A" line, then numeric rows), CSV, auto-detection, long-name/units from header lines, importing many files into new sheets/books.
- Export (`expASC`) writes the worksheet back to text with optional label rows (section 12).

Priority legend: P0 = owner's daily use, P1 = common, P2 = later.

---

## 1. Entry points and files

| Item | Observation | Source |
|---|---|---|
| General import function | `impASC`, description "Import ASCII file/files". Input: file name(s); `options` tree of settings; output range `orng` (default: active sheet). A "Launch Option" chooses single or multiple file dialog. | [F] `X-Functions/Import and Export/impASC.OXF` |
| CSV import function | `impCSV`. Same shape (file name, options tree, output range). | [F] `.../impCSV.OXF` |
| Generic importer by filter | `impFile` takes a file name and an optional filter (.oif) name plus filter location (data folder, system filter folder, user filter folder, or stored in the target page). | [F] `.../impFile.OXF` |
| Stored settings | Filters are XML files (.oif) holding one saved set of import options. `ASCII.oif` is the default text filter (spec `*.txt;*.dat;*.asc`); `CSV.oif` is the CSV one (spec `*.CSV`). Other .oif files (Excel, HDF5, SPC, JCAMP ...) are other formats and are P2. | [F] `Filters/*.oif` |

For the Rust app: model "an import profile" as a struct holding the options in sections 2 to 10, serialisable to a settings file. A profile applies to a file extension.

## 2. File structure and delimiters (P0)

Option group name in Origin: `FileStruct`.

| Option | Meaning | Values | Default | Source |
|---|---|---|---|---|
| DataStruct | How a line is split into cells. | 0 fixed column widths; 1 delimited by a single character (or a set of single characters); 2 delimited by a multi-character string | 1 | [W] details-of-treenodes-in-import-ascii |
| Delimiter | Which single-character delimiter (only when DataStruct=1). | 0 unknown (auto-detect); 1 comma; 2 tab; 3 tab or space; 4 semicolon; 5 other (user supplied); 7 space | 0 (auto) | [W] same |
| OtherDelimiter | The user's set of delimiter characters when Delimiter=5. Several characters may be given; each one is separately a delimiter. | string | empty | [W] |
| CombineDelimiters | Treat a run of consecutive delimiters as one. Documented as applying to the "Other" delimiter option. | 0 no; 1 yes | 0 | [W] |
| MultipleDelimiters | The ordered multi-character delimiter when DataStruct=2. | string | empty | [W] |
| ColWidths | Widths in characters for fixed-width mode: comma list, applied in order; the last width repeats for remaining columns; a single width applies to all columns. | list of ints | none | [W] impasc-options |
| TransposeData | Swap rows and columns while importing. | 0/1 | 0 | [W] |

Observed in the legacy filter `ASCII.oif`: `iDelimited=1`, `iDelimiter=0`, `szFixedWidth` empty, `cDelimiters` empty. [F]

Behavior notes:
- Delimiter "tab or space" (3) means either character separates cells, and runs of spaces behave as one (I infer, since this mode exists to read space-aligned columns). **[I]**
- With Delimiter = comma, empty cells between two commas are empty (missing) cells; consecutive-delimiter combining is documented only for "Other". For a plain comma/tab/semicolon, assume NOT combined. **[I]**
- For space delimiting (7) and tab/space (3), whether runs collapse is **[?]**. Safest implementation: collapse runs of spaces for 3 and 7, because aligned-column text files are the common case.
- Auto-detect (Delimiter=0) "searches the file for a repeating delimiter". The candidates it tries are presumably comma, tab, semicolon, space. **[I]** The exact scoring is **[?]**; see section 13 for a proposed algorithm.

## 3. Header handling (P0)

Option group `Headers` (CSV import calls it `HeaderLines` with slightly different node names).

Concepts [W impwiz-asc-headlinepage]:
- **Main header lines**: text at the top of the file that is not data and does not follow the delimiter pattern (instrument name, date, settings). They are not turned into columns. Their content can be saved to the workbook as notes/variables.
- **Sub-header lines**: lines after the main header, before the first data row, which DO follow the delimiter pattern, so each cell lines up with a data column. These supply per-column labels.
- **Data rows**: everything after.

| Option | Meaning | Default (ASCII import) | Default (CSV import) | Source |
|---|---|---|---|---|
| AutoSubHeaderLines | Auto-determine both the main header count and sub-header count. It does not decide which sub-header line is the name vs the unit. | 1 (on) | 1 (on) | [W] |
| CountHeaderLines | How the main header is delimited: 0 fixed count, 1 by leading character(s) (Origin 2018b and later). | 0 | n/a | [W] |
| MainHeaderLines | Count of main header lines when fixed. | 0 | 0 | [W] |
| HeaderLeadingChar | Characters that mark a main header line when CountHeaderLines=1. | none | n/a | [W] |
| SubHeaderLines | Number of sub-header lines, used only when auto is off. | 0 | 0 | [W] |
| ByBottom | If 1, sub-header line numbers (used by the next rows) count upward from the bottom of the sub-header block instead of downward from its top. | 0 | 0 | [W] |
| ShortName | Sub-header line to use as column short name (hidden since 2016 successors; short names are alphabetic by default). | 0 (none) | 0 | [W] |
| LongName | Line number (1-based within the sub-header block) holding long names. | 1 | 0 | [W] details page; `ASCII.oif` nLongNames=1 [F] |
| Unit | Line number holding units. | 2 | 0 | [W]; `ASCII.oif` nUnits=2 [F] |
| CommentFrom / CommentTo | Range of lines that become the comment row(s). Multiple lines are concatenated into the comment (I infer, with a separator). | 3 / 15 | 0 / 0 | [W]; `ASCII.oif` iLabels=3, iMaxLabels=1 (older meaning) [F] |
| ParametersFrom / To | Lines for system parameter rows. | 0 / 0 (none) | 0 / 0 | [W] |
| UserParametersFrom / To | Lines for user-defined parameter rows. | 0 / 0 | 0 / 0 | [W] |
| Oneline | One header line (1-based index) that contains both name and unit; 0 = not used. | 0 | n/a | [W] |
| OneLine Expression | Pattern splitting that composite line; `<auto>` detects paired brackets ( ) [ ] { } and single separators slash, hyphen, number sign (name#unit, name#unit#, a run like name###unit counts as one separator). | `<auto>` | n/a | [W] |
| LabelSkipChars (legacy) | Number of leading characters to ignore on each header line. | 0 | n/a | [F] ASCII.oif `iLabelSkipChars`; [W] "characters to skip" |

Important note: the defaults above are the documented defaults of the import tree. They mean: when the file has sub-header lines, line 1 is the long name, line 2 is the units, and lines 3 and later are comments. If only one sub-header line is found, it becomes the long name and there are no units or comments. **[I]** Whether the auto mode only counts lines or also assigns "line 2 = unit" even when the file has just one text line is **[?]**.

Fields that appear in the legacy `ASCII.oif`: `iHeaderLines=0`, `iSubHeaderLines=0`, `iAutoSubHeaderLines=1`, `iAutoColTypes=1`, `iTestLines=4`, `iMaxTestLines=50`. **[F]** These match the tree defaults (4 and 50).

Where the header text goes in the workbook:

| Header concept | Destination |
|---|---|
| Long name line | column long name |
| Units line | column units |
| Comments lines | column comment row |
| System parameter lines | column parameter rows (P1) |
| User parameter lines | user-defined parameter rows (P1) |
| Main header lines | saved with the sheet as notes / "file info" (P1) |

## 4. Auto-detection of data start (P0)

What is documented [W]: with auto-determine on, Origin guesses the main-header count and sub-header count, shows the guess in a preview, and the user may override. A "structure" (delimiter, column count) is adopted once a minimum number of consecutive lines (default 4) share it, scanning at most a maximum number of lines (default 50).

| Option | Meaning | Default | Source |
|---|---|---|---|
| MinTestLns | Number of consecutive lines with an identical structure required to accept the structure for the whole file (used when auto is on). | 4 | [W]; `iTestLines=4` [F] |
| MaxTestLns | Upper limit of lines examined. Must be above the number of header lines and below the number of lines in the file. | 50 | [W]; `iMaxTestLines=50` [F] |
| AutoColTypes | Determine each column's type (text vs numeric) automatically. When off, the existing column formats of the target sheet are kept and the data must fit them. | on (1 in ASCII.oif; 2 in the newer tree meaning "yes") | [W][F] |

Proposed algorithm (inference; verify with running Origin, see unknowns):
1. Read up to MaxTestLns lines (50). Split each into cells with the chosen/trial delimiter.
2. Classify each cell as numeric, date/time (matches a known date format), or text. A cell is numeric if it parses as a number under the active numeric separators (after trimming blanks).
3. Find the first line L such that L and the next MinTestLns-1 lines have the same cell count and the same per-cell numeric/text pattern, and at least one cell is numeric. That is the start of data.
4. Lines before L are header lines. Of those, lines that split into the same cell count as the data are sub-header lines (counted from the bottom of the header block upwards: the sub-header lines are the contiguous block directly above L that matches the data's cell count); the lines above them are main header lines. **[I]**
5. If the file has no numeric rows at all, treat as all-text; allowed only if AllowAllText is on.
6. If delimiter is auto: try candidate delimiters on the test lines and pick the one giving the highest count of lines with the same (greater than 1) cell count. **[I]**

Edge cases to decide: a data row with fewer cells than the data structure (pad with missing); a trailing delimiter at the end of a line (CH Instruments files do not have one, but many CSVs do; an extra empty trailing column should be dropped **[I]**); blank lines within data (skipped or an empty row **[?]**).

## 5. Numeric separators (P0)

| Option | Meaning | Values | Default | Source |
|---|---|---|---|---|
| NumSep | Decimal/thousands convention for reading numbers. | -2 auto (use the program preference, which itself defaults to the operating system regional setting); 1 = "1,000.00" (comma thousands, dot decimal); 2 = "1.000,00"; 3 = "1 000,00" (space thousands, comma decimal); 4 = "1'000.00" | -2 | [W] |

Legacy file `ASCII.oif` stores `nNumSep=1`. [F] The documentation notes the underlying list has more than 15 alternatives; only these four are confirmed. **[?]** the rest.

Rust guidance: default to dot decimal, comma thousands, but let the profile specify the alternatives. Note that with comma as delimiter, the NumSep must not be "1.000,00" (conflict); do not try to be clever.

## 6. Text qualifier (P0 for CSV)

| Option | Meaning | Values | Default | Source |
|---|---|---|---|---|
| Qualifier | Character that wraps text which may contain delimiters. | 0 none; 1 double quote; 2 single quote | 0 | [W]; ASCII.oif `cQuote=0` [F] |
| RemoveQuotes | Strip the qualifier from imported values. Requires a qualifier. | 0/1 | 1 (ASCII tree); CSV.oif `RemoveQuotes=0` | [W][F] |

The CSV import is expected to handle quoted fields natively; the CSV filter file has the qualifier node absent. **[?]** exactly which quote it assumes (double quote is the sensible default).

## 7. Non-numeric cells, missing values, leading zeros

| Option | Meaning | Values | Default | Source |
|---|---|---|---|---|
| NonNum | What to do when a text cell is found in a column that has been decided numeric. | 0 terminate import; 1 read it as text (the cell keeps the text; column format becomes text/numeric mixed); 2 start a new worksheet; 3 start a new column; 4 start a new hidden workbook; 5 skip | 1 | [W]; ASCII.oif `iNonnumeric=1` [F] |
| AllowAllText | Allow a file that has no numeric data at all. | 0/1 | 1 | [W]; ASCII.oif `iAllowAllText=1` [F] |
| LeadingZeros | Remove leading zeros from numbers (so "007" is 7). | 0/1 | 1 | [W]; ASCII.oif `iLeadingZeroes=1` [F] |
| (Origin.ini) `[Import] CntrlByte=3` | bit0: continue reading past non-numeric cells in numeric columns until end of file (if clear, import stops at the first one); bit1: reset plot ranges to the imported data range. The value 3 means both are on. | 3 | [F] `OriginLab/2016/User Files/Origin.ini` lines 179-194 |

Missing values:
- An empty cell (two adjacent delimiters, or a missing trailing cell) imports as a blank cell. In modern versions blank cells stay blank (not filled with the special missing value); see the workbook spec. **[W]**
- The words "NaN", "NA", "--" and similar strings as numeric values are **[?]**. Safe rule: treat as missing in numeric columns.
- A text cell in a column typed numeric, with NonNum=1, converts the column to the mixed text-and-numeric format.

### What Sheaf implements (crates/io/src/ascii.rs)

None of the rules below were checked against a running Origin; they follow the documented NonNum=1 behaviour and the "safe rule" above. Sheaf columns are either all numbers or all text (`ColumnData`), so there is no mixed column. Where Origin keeps a text cell in a numeric column, Sheaf stores a missing value (NaN) and reports it in the import warnings.

1. No-value markers. These count as missing, not as text: `NA`, `N/A`, `#N/A`, `n.a.`, `n.d.`, `nd`, `bdl`, `#VALUE!`, `#DIV/0!`, `#REF!`, `#NUM!` (any letter case), any string made only of dashes (`-`, `--`, `----`, en and em dashes), and any string that starts with `<` followed by something (`<LOD`, `<0.05`). `NaN` and `inf` are real numbers. Markers do not count when deciding whether a column is numeric, so a column with many `<LOD` cells stays numeric. A column that holds only markers is numeric with all values missing. In a text column (sample names) the markers stay as they are.
2. Column type: a column is numeric when at least 80 percent of its cells that are neither empty nor markers are numbers. The remaining text cells become missing values, with a warning naming the column and the count. Otherwise the column is text. A text column is designated Label, except the first column when "first column is X" is on (the default): Origin designates the columns of a text file by position, so sample names in the first column are the X column and the Y columns are drawn against them (graphing.md section 14.6).
3. Data start. Markers do not break a run of numeric rows. When the first run of three numeric rows is found, the start moves up over directly preceding rows that have the same field count and at least as many numbers or markers as text cells, so `3,trace` above numeric rows stays data. A name row (no numbers) or a units row stays a header line. In files with a text column the first data row is the first row that has a number or marker, has text in its other fields, and is followed by rows with the same field count in which at most half of the cells in its numeric columns are text.
4. Quoted fields may contain line breaks. The text is cut into records, carrying the open-quote state across physical lines; a quote opens a field only at the start of a field (after a delimiter, a space or the start of a record). If a quote is still open at the end of the file, the file is cut into plain lines instead. Header line numbers in the options and in `Detected` count records, not physical lines. CRLF and LF both work, and a line break inside quotes is kept as a line feed in the cell.

Not decided: whether Origin treats `-` or `<LOD` as numeric-column text (it probably keeps them as text cells that are not plotted); a black-box run of impasc on a small file would settle this.

## 8. Date and time detection (P0 for time-stamped logs, P1 otherwise)

| Option | Meaning | Default | Source |
|---|---|---|---|
| DateFormat | A custom date pattern to match. If cells match, the column format becomes Date. | none | [W] |
| TimeFormat | A custom time pattern. A match makes the column Time. | none | [W] |
| KeepDestColFmt (CSV) | Keep the target column's format instead of re-detecting. | 0 | [W]; CSV.oif `KeepDestColFmt` [F] |

Origin keeps a menu of ready patterns (also stored in Origin.ini `[Column Custom Format List]`, observed [F]), using these token letters: `yyyy yy MM MMM dd d HH hh mm ss tt` and `###` for fractional seconds. The twelve entries:
`yyyy'-'MM'-'dd HH':'mm':'ss`, same with `'.'###`, `yyyy'-'MM'-'dd hh':'mm':'ss tt`, `dd'/'MM'/'yyyy`, `dd'-'MM'-'yy`, `MM'/'dd'/'yyyy`, `MM'-'dd'-'yy`, `yy'/'MM'/'dd`, `yy'.'MM'.'dd`, `dd'.'MM'.'yy`, `yyyyMMdd`, `dd'-'MMM'-'yy`.

Behavior [W wkscol-datetime]: unless a date format is given or recognised, date/time strings arrive as text (left-aligned) and the user converts the column by setting its format. Internal date value = a double, days (fraction = time of day) in a Julian-day-based system. Whether the automatic pass recognises ISO-like "2020-01-31 12:00:00" without a user pattern is **[?]**; plan to recognise ISO 8601 and the pattern list above.

### Observed in Origin 2016 (black-box runs of `impasc`, 2026-11-12) [F]

Files were small CSVs we wrote (US regional settings on the test machine). Origin's own automatic pass only turns some strings into dates; everything else arrives as text (column format 7) and stays left-aligned:

| String in the file | What the default import gave |
|---|---|
| `13:45`, `01:05` | Time column (format 3, `HH:mm`), value 0.5729166667 for 13:45 |
| `13:45:10.250` | Time column, fraction kept (0.5730353009) |
| `13/11/2026`, `02/12/2026` | Date column (format 4) read month first: row 1 (day 13) did not appear in the column, and row 2 was read as 12 February (value 2461083); Origin gave no warning |
| `2026-11-11 13:45:10`, `2026-11-11` | text |
| `11/11/2026 1:45:10 PM`, `11-Nov-2026` | text |

Setting the column to Date (format 4, subformat 11) afterwards converts text that matches the system pattern: `11/11/2026 1:45:10 PM` became 2461355.5730324076 and `11/12/2026 1:05:00 AM` became 2461356.045138889. The ISO strings turned into missing values. `11-Nov-2026` converted to the wrong date (11 November read as 12 November, and 12-Nov as 12 December), so Origin's conversion of day-month-name strings is not something to copy. The `Dates.*` option names we guessed for `impasc` were rejected; Origin's import wizard has a per-column date format that we did not drive.

Decision for Sheaf: recognise more than Origin's automatic pass does, because the owner's instruments write ISO and day-month-name strings. The numbers stored are the ones Origin uses (see `workbook-model.md`).

### What Sheaf implements (crates/io/src/ascii.rs)

- A column is a date column when every filled cell reads as a date, a time of day, or a date with time of one kind. Recognised: `yyyy-MM-dd` (also with `/` or `.`), a `T` or a space then a clock, `M/d/yyyy` and `d/M/yyyy` (also with `-`), `dd.MM.yyyy` (always day first), `11-Nov-2026`, `11 November 2026`, `13:45`, `13:45:10.250`, and 12-hour clocks with AM or PM. No-value markers (`n.d.`, `--`) and empty cells in the column become missing values.
- Display format: the first date pattern seen and the most precise clock pattern in the column, so the text shown matches the file.
- Day and month order: when only one order reads every cell (a day above 12 or a month above 12 somewhere), that order is used silently. When both fit, the option `date_order` decides (default month first, like Origin on a US setup) and the import warns that the dates are ambiguous. CLI: `--day-first`.
- A column with any cell that is not a date, such as a stray word, stays text; the choice `DateImport::AsText` (CLI `--dates-as-text`, import dialog "Keep as text") keeps every date column as text.
- Date cells also count as numbers when the importer looks for where the data starts, so a time stamp column does not hide the header.
- Export writes date columns as displayed (ISO date-time when that is the column's format), so the file reads back to the same numbers.

## 9. Partial import (P1)

Origin groups these under `Partials`; rows and columns each have: first, last, read-count and skip-count (read R, skip S, repeat), plus a custom list.

| Option | Meaning | Default | Source |
|---|---|---|---|
| Partial | 0 off; 1 apply to all files; 2 apply to all files except the first | 0 in CSV; the ASCII tree lists 1 as "apply to all" | [W] |
| FirstCol / LastCol | first and last column to import; LastCol 0 means to the end | 1 / 0 | [W] |
| ReadCols / SkipCols | read R columns, skip S columns, repeat; used when Skip is more than 0 | 1 / 0 | [W] |
| ColRanges | custom column list string | none | [W] |
| FirstRow / LastRow | first and last data row to import (rows counted after the header); LastRow 0 = to the end | 1 / 0 | [W] |
| ReadRows / SkipRows | read R rows, skip S rows, repeat | 1 / 0 | [W] |
| RowRanges | custom row list string | none | [W] |

Legacy ASCII.oif: `iPartialC1=0`, `iPartialC2=-1`, `iPartialR1=0`, `iPartialR2=-1` (zero-based, -1 = to the end). [F]

## 10. Import mode and naming (P0)

Modes (option `FirstMode` for the first file, `Mode` for later files in a multi-file import):

| Code | Name | Meaning |
|---|---|---|
| 0 | Replace existing data | overwrite the data of the target sheet (first file only) |
| 1 | New columns | append the file's columns to the right of the target sheet |
| 2 | New rows | append the file's rows under the existing data |
| 3 | New books | one new workbook per file |
| 4 | New sheets | one new sheet (in the same book) per file |

Defaults: `FirstMode=0` for both; `Mode` = 3 (new books) for ASCII [W], 4 (new sheets) for CSV [W]; `CSV.oif` agrees (`FirstMode=0`, `Mode=4`) [F]; `ASCII.oif` `iMode=0` [F] (older meaning). Behavior of multi-file import when Mode=0 is not allowed (only 1 to 4 for later files).

Other creation options: `PageTemplate` (template used for new books/sheets; default `<default>` prototype; `<clone>` means copy the active book) [W]; `PageType` 2 = workbook in the filters [F]; `Sparklines` 0 no, 1 yes, 2 yes only if fewer than 50 columns (default 2) [W]; `nAutoSize` 1 = auto-fit cell width (CSV tree: 0 none, 1 width, 2 height, 3 both) [W]; `DataWriteProtection` 0 none, 1 read-only, 2 sort-only [W]; `ColDesignations` string (empty means keep template; the CSV filter writes `XY`, i.e. first column X, the rest Y, repeating) [F]; `ColFormats` 9 in both filters [F].

Name derivation from file name (group `Names`):

| Option | Meaning | Default | Source |
|---|---|---|---|
| AutoNames | Use the default rules: sheet name = file name without extension; book name = file name with extension when the book is new/empty or mode is replace (CSV doc). | 0 | [W] |
| FNameToSht | Rename the sheet using (part of) the file name | 1 | [W] |
| FNameToShtFrom / To | 1-based character range of the file name to keep (To 0 = to the end). Negative numbers count from the end. | 1 / 0 | [W] |
| FNameToBk | Rename the book from the file name. | 1 | [W]; `CSV.oif` RenameBook=0 [F] (conflict, **[?]**) |
| FNameToBkFrom / To | same trimming; example -4 removes ".csv" | 1 / 0 | [W] |
| FNameToBkLnOnly | change only the book long name (the short name stays automatic) | 0 | [W]; CSV.oif RenameBookLongName=1 [F] |
| FPathToBk | include folder path in the book name | 0 | [W] |
| FNameToBkComm | append file name to book comment | 1 | [W] |
| FNameToWksComm | append file name to sheet comment | 0 | [W] |
| FNameToColComm | append file name to each column comment | 0 | [W] |
| FNameToUDL | write file name to a user parameter row named `SourceFile` | 0 | [W] |
| FPathToComm | include path and extension in appended names | 0 | [W] |

Name limits apply after derivation (book short name max 13 chars of letters/digits; sheet name 64 per the limits page; see workbook-model.md). When a book is made from a file name with spaces, punctuation or length over the limit, the short name is auto-generated and the file name goes into the long name. **[I]**

## 11. Encoding and misc (P1)

- Encoding: no encoding option exists in the 2016 import tree. **[W]** Behavior with UTF-8/UTF-16 files is **[?]**. Plan: detect BOM (UTF-8, UTF-16 LE/BE), else try UTF-8, else fall back to Windows-1252.
- `SaveFileInfo` (default 1) stores file name, size, date and the header lines in the workbook so the user can re-import. `reimport` is a related feature (P2). [W][F]
- `Script after each import / after all imports`: hooks for scripting, default none. P2.
- Drag and drop: graph or workbook as per `ASCII.oif` DragAndDrop flags. P2.
- Limits: maximum 18,002 columns and 200,000,000 rows per ASCII import. [W origin-limitation]

## 12. Comparison of the two entry points

| Setting | ASCII (`impASC`, ASCII.oif) | CSV (`impCSV`, CSV.oif) |
|---|---|---|
| File pattern | `*.txt;*.dat;*.asc` | `*.CSV` |
| Delimiter control | full set: auto, comma, tab, tab/space, semicolon, space, other, multi-char, fixed width | only comma, semicolon, or auto-detect (code 0, 1, 2; default 0 in docs, 2 in the .oif) |
| Header detection | auto on | auto off, 0 header lines in the filter file (but doc default for AutoSubHeaderLines is on) |
| Long name / units lines | 1 / 2 | 0 / 0 in the filter file |
| Multi-file mode | new books | new sheets |
| Column designations | none (template) | `XY` |

Because the .oif and docs differ, the Rust app should choose its own sane defaults: auto header detection ON, delimiter auto, long name = first sub-header line, units = second. **[I]**

## 13. Exporting text: expASC (P1)

Source: `X-Functions/Import and Export/expASC.OXF` [F]. Writes the worksheet to a text file.

| Option | Meaning | Values | Default |
|---|---|---|---|
| type | File type for the dialog | dat, text (.txt), csv, all | dat |
| path | Output file path | string | none |
| overwrite | Existing file handling | replace / append | 2 (append in the OXF numbering; GUI default replace **[?]**) |
| select | Export only the selected cells (may be non-contiguous) | 0/1 | 0 |
| separator | Cell separator for non-CSV types | tab, comma, space, comma+tab, comma+space | tab |
| csvsep | Separator when the type is csv | comma / semicolon | comma |
| shortname | Include the short-name heading row | 0/1 | 0 |
| longname | Include the long-name row | 0/1 | 1 |
| units | Include the units row | 0/1 | 1 |
| comment | Include comments row | 0/1 | 1 |
| userparam | Include user parameter rows | 0/1 | 0 |
| sampinterv | Include sampling interval row | 0/1 | 0 |
| parameter | Include parameter rows | 0/1 | 0 |
| rowlabel | Include row labels and index numbers | 0/1 | 0 |
| precision | Full precision output of numbers (not rounded to the display digits) | 0/1 | 1 |
| addxcol | Add a new X column computed from the sampling interval | 0/1 | 0 |
| missing | Write missing values as "--" | 0/1 | 1 |
| suffix | Text appended to numbers in text cells | string | empty |

Related exports (names only): expWks (workbook/Excel), expGraph / expImage / expPDFw (graphics), expMatASC (matrix text), expNITDM, expWAV. [F]

## 14. Worked example: CH Instruments cyclic voltammetry text file (P0)

Observed structure from the owner's domain knowledge (not from Origin): a CH Instruments .txt export has a few free-text header lines (date, file name, instrument model, technique, then parameters like Init E, High E, Low E, Scan rate), then a short column header line ("Potential/V, Current/A"), a blank line, then rows of two comma-separated numbers. With the auto rules above, the lines before the "Potential/V, Current/A" row are main header lines (they do not match two-cell structure), the column header is a sub-header line that becomes the long name (unit may need splitting from the name via the one-line pattern: name "/" unit), and the numeric rows are data. Column designations X, Y. **[I]** This example must be checked against real files; the blank line before the data must be tolerated.

## Unknowns that need a check against running Origin

1. Exact auto-detect rules for data start and delimiter choice (section 4).
2. Whether consecutive spaces collapse when Delimiter is space or tab/space.
3. Default behavior and effect of Qualifier/RemoveQuotes in `impCSV`, and which character it assumes.
4. Which strings (NaN, NA, "--", empty) become missing in numeric columns.
5. Whether ISO date strings are auto-recognised as Date without a custom pattern.
6. Encoding handling (UTF-8 with/without BOM, UTF-16, Windows-1252).
7. How the Comments range is joined when it spans several lines.
8. Behavior on ragged lines (fewer or more cells than the structure) and blank lines.
9. Discrepancy: CSV filter file has book renaming off and header lines 0, the documentation says otherwise.
10. Full list of numeric-separator options beyond the four confirmed.

Sheaf's own guard on clock times: minutes (and seconds, when present) must have two digits, so ratios or codes such as 1:2 or 2:3 stay text instead of becoming times of day. Date-format cells holding numbers outside about year -100000 to 100000 are shown as plain numbers rather than as a date.

## 15. Number separators, import modes, header rows, data blocks and batch import (P0)

### Observed in Origin 2016 (black-box runs of `impasc`, 2026-11-12) [F]

The scripts are `tools/origin-ref/import-*.lines.txt`, the input files `tools/origin-ref/import-inputs/` (written by us). The option names that the 2016 `impasc` accepts on the command line are short ones, not the ones in the documentation: `options.FileStruct.NumericSeparator`, `options.impmode`, `options.Hdr.UParamsFrom` / `UParamsTo`, `Units`, `CommsFrom` / `CommsTo`, `AutoSubHdr`, `SubHdrLns`, `MainHdrLns`. The documented names (`NumSep`, `Mode`, `Headers.*`) were refused.

- **Number separator.** `NumericSeparator` 0 is the "1,000.00" reading and 1 is the "1.000,00" reading. With 0, a file whose first rows hold `1,234.5`, `2,345.25`, `12,000` was not recognised from the first row: the automatic header detection took the rows with thousands separators as header text, and only the later rows (`7`, `1,000,000.5`) arrived as numbers. Under 0 a decimal comma was silently dropped: `1,5` became 15 and `7,125` became 7125. With 1, `1.234,5` became 1234.5, `12.000` became 12000, `1,5` became 1.5; `1,234.5` became 1.2345 (the dot was dropped) and `1,000,000.5` stayed text. Origin therefore does not check that the groups have three digits.
- **Import modes** (`impmode`, run on a second file after a first one was imported): 0 replaced the data of the sheet, and the sheet and book long name took the new file's name; 1 appended the columns to the right, each with its own long name and units; 2 appended the rows under the data and kept the labels of the first file (the new file's labels were not used); 3 made a new book; 4 added a second sheet to the book. The value 5 behaved like 3.
- **Header lines.** With four lines of text above the data (a name line, a "Wavelength" line, a units line and a comment line) the default import used line 1 as long name, line 2 as units, and lines 3 and 4 joined with a line break as the comment. Setting the user-parameter range to line 2 made one user parameter row, named `UserParameter0` (a generic name, not the text in the line), holding all cells of line 2 including the first one (`Wavelength`).
- **Several blocks.** A file with a title line, a table with a name and a units line, a blank line, and the same name and units lines followed by a second table arrived as the second table only (three rows); the first table was not imported.

### What Sheaf does (crates/io/src/ascii.rs, batch.rs; dialog; CLI)

- **Thousands separator** (`AsciiOptions::thousands`, CLI `--thousands`, import window "Thousands separator"). Choices: detect (default), none, comma (`1,234.5`), dot (`1.234,5`), space or no-break space (`1 234,5`), apostrophe (`1'234.5`). Dot and space make the comma the decimal separator (Origin's reading); comma and apostrophe keep the dot. A separator is accepted only between groups of three digits (`1,23` and `1234,567` are not numbers), which is stricter than Origin, so `1,5` under the comma setting is a missing value and a warning, not 15. Detection adopts a separator when some cell shows it without doubt: two or more groups (`1,234,567`), groups followed by the decimal character (`1,234.5`, `1.234,5`), or any grouped number with a space or an apostrophe. A lone `1,234` proves nothing and stays text (it is 1234 with the comma option and 1.234 with the decimal-comma option). Evidence for two different separators cancels out and nothing is removed. When the separator was found by detection the import warns once; when it was chosen it does not. Detection looks at all rows, so the rows that Origin took for headers are read as data. The comma as field delimiter cannot be combined with a decimal comma (error, as before); a comma thousands separator in a comma-delimited file works only inside quotes.
- **Import modes** (`batch::ImportMode`, `batch::place_workbook`): new book, new sheet in the target book (the sheet name is the file name, `_2`, `_3` ... on a clash), replace the data of the target sheet (the sheet takes the new name, as in Origin), new columns (right of the existing ones; a clashing short name gets the next free one), new rows (column by column in order, labels of the first file kept; a column that changes type, text against numbers, gets missing values for the new rows and a warning; extra columns are added with missing values above; short columns are padded). Without a target every mode makes a new book. As in Origin, only the first file of a multi-file import may replace data; the dialog makes the others new sheets.
- **Header rows to parameter rows** (`AsciiOptions::param_lines`, CLI `--param-lines 2,3`). Each listed header line becomes a named parameter row of the columns (`Column::params`): the name is the first cell of the line (a trailing colon is dropped; empty gives `Param<k>`) and the first column gets no value. With `param_name_from_first_cell = false` the rows are named `Param<k>` and the first column is included, which is closer to Origin's `UserParameter0`. Parameter lines are not reported as dropped header lines. The long name, units and comments lines are chosen as before (`--long-name-line`, `--units-line`, `--comments-line`). Known difference: the automatic label detection looks at three lines directly above the data at most, where Origin used a longer block and joined the comment lines.
- **Several blocks** (`AsciiOptions::blocks`, CLI `--blocks-as-sheets`). By default a blank line is skipped and the file is one table (Origin kept only the last block in the run above; Sheaf does not copy that). With `Blocks::Sheets` the text is cut at blank lines into blocks that hold at least one numeric row; a block of text only (a title or header) belongs to the data block after it. Each block is read with its own header detection and becomes a sheet named after the file, then `file_2`, `file_3`.
- **Batch import** (`batch::import_batch`, `batch::expand_inputs`, CLI `sheaf import <files, folders or patterns> --into project.sheaf --mode books|sheets|columns|rows`). A folder gives its `.txt .csv .tsv .dat .asc .prn .tab .bin` files; `*` and `?` in the last path part match file names without regard to letter case; the files are taken in natural order (`icp_2` before `icp_10`). Layouts: one book per file (default, the old behaviour); one book with a sheet per file named after the file; one sheet with the columns of all files side by side, each column's comment and its `SourceFile` parameter row holding the file name (`--file-name-comments` does this in the other layouts too); one sheet with the rows stacked. The shared book is named by `--book-name` or the folder name. Every file goes through `import_file`, so CHI text and binary files in the batch use the CHI importers and may add several sheets; in the columns and rows layouts only the first sheet of a file is used, with a warning. A file that cannot be read is reported and skipped; the others are saved and the exit code is non-zero.
- **Import window.** "Put the data in" (new book, new sheet, replace the data, new columns, new rows; the last four act on the sheet that is active in the project and are disabled when there is none) and "Thousands separator". File > Import and dropping already accepted several files.

Not done: file name trimming for sheet and book names (`FNameToShtFrom/To`), per-file modes (`FirstMode` against `Mode`), parameter and block options in the import window, joining several comment lines.
