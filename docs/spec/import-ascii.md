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
2. Column type: a column is numeric when at least 80 percent of its cells that are neither empty nor markers are numbers. The remaining text cells become missing values, with a warning naming the column and the count. Otherwise the column is text and designated Label.
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
