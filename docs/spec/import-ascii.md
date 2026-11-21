# Text / CSV data import

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

What Sheaf does (`Delimiter` in `crates/io/src/ascii.rs`, CLI `--delimiter`, `--fixed-width`, import window "Delimiter"):

- **Character set** (`Delimiter::AnyOf { chars, combine }`, import window **Other**, CLI `--delimiter-chars '|;'`): every listed character separates fields. With the default `combine = false`, `a|;b` has an empty middle field. **Treat Consecutive Delimiters as One** (CLI `--combine-delimiters`) collapses each adjacent run, so the same input has two fields. Explicit quoted empty fields stay empty fields even with combining. A leading/trailing run creates one empty field at that edge; this is a Sheaf choice for an edge case not measured in Origin. Unicode separator characters are supported. An empty set is rejected; a set containing comma is rejected when decimal comma is in use.
- **Multi-character delimiter** (`Delimiter::Multi`, import window **Multiple Characters**, CLI `--delimiter '||'`): fields split only at a complete, non-overlapping occurrence of the string (`||`, `::`). A lone character of the sequence is ordinary text. `Delimiter::from_text` and the existing CLI `--delimiter` keep their whole-string behavior. The import window now distinguishes this mode from **Other**, which uses a set. These options follow the published [FileStruct reference](https://docs.originlab.com/x-function/ref/details-of-treenodes-in-import-ascii/); no new installed-Origin probe is claimed. An empty delimiter string is rejected.
- **Fixed width** (`Delimiter::Fixed(widths)`, CLI `--fixed-width 8,12,10`, window "Fixed Width" with a "Widths" field): the line is cut every `w` characters (Unicode characters, not display columns, so wide CJK text counts one each), the last width repeats for the rest of the line (so one width applies to every column), fields are trimmed, quotes mean nothing, a shorter line has fewer fields. Header detection then works on the cut fields as for any delimiter. A width of 0 or an empty list is an error. Origin's ruler-style click-to-place column breaks are not offered; widths are typed. Widths are taken from Origin's public documentation (`ColWidths`), not measured; measure on the Windows machine with tools/origin-ref.

Quoted fields in explicit delimiter modes use the selected delimiter for record boundaries too, so a custom separator can precede quoted multiline text. A partial or overlapping match of a whole-string delimiter does not open a new quoted field. CRLF inside quotes is normalized to LF. Quotes used as a separator are literal separators, and Fixed Width never groups physical lines by quotes. Detection, preview, block splitting, and final import use the same record rules. Auto mode retains its earlier generic quote-boundary bootstrap before choosing among built-in separators; explicit selection is available for ambiguous files.

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

Header and naming options in Sheaf (`AsciiOptions`; the window nodes are described, as measured, at the end of section 10): `rename_columns` (default on; off leaves long name, units and comments empty and still skips the header lines; only the CLI flag `--no-rename-columns` sets it, the window has no such tick because Origin's window has none: setting Long Names, Units and Comments to `<none>` does the same); `short_name_line` (0 = none; a header line whose cells become the short names by the rules measured below; CLI `--short-name-line N`).

**Short names from a header line, measured in Origin (2026-11-13, `impasc ... options.Hdr.SNames:=n`, files `tools/origin-ref/import-inputs/shortn*.txt`).** The option of the Header Lines group is `SNames` (dialog label "Short Names", default `<none>`; `LNames` is the long-name line). For each cell: every character that is not a letter, digit or dot is dropped (`my_name_1` gives `myname1`, `Second Col` gives `SecondCol`, `3rd-col` gives `3rdcol`, `(paren)` gives `paren`); a first character that is a digit or a dot becomes `A` (`3rd-col` gives `Ardcol`, `9lives` `Alives`, `12x` `A2x`, `1.5` `A.5`, `.5x` `A5x`, `..a` `A.a`); the result is cut to **18 characters** (a 26-letter cell gives its first 18, so the limit is confirmed); an empty cell, a cell with nothing left and a name that another column has already (letters compared without regard to case: `Q1` then `q1`; after the cut too: a 19-letter cell whose first 18 letters repeat an earlier name) get the automatic name of their column (`F`, `G`, `O`, ...). A cell that equals the automatic name of another column (`B` in the 16th column) is accepted when no column has it. Typing a name in the Column Properties box or setting `wks.col1.name$` gives the same drops and the same cut to 18 (a longer name is cut without a message) but allows a first digit (`9abc` stays), and a name that another column of the sheet has is not taken: the name stays as it was.

**Measured in Origin 2016 (T299, 2026-11-21; `re-workspace/scratch/import-header/`).** A tab file with
three sub-header lines (`Potential (V)` ..., then two comment lines) through `impasc`:

- With the defaults (auto sub-header), line 1 became the long name, line 2 the units and line 3 the
  comment (the documented LongName 1, Unit 2, CommentFrom 3).
- `options.Hdr.AutoSubHdr:=0 SubHdrLns:=3 LNames:=1 Units:=0 CommsFrom:=2 CommsTo:=3` put lines 2 and 3
  into the comment **joined with a line break** (`first comment a` / `second comment a`), no units.
- `options.Hdr.ByBottom`, `options.Hdr.OneLine` and `options.Hdr.NameUnitLine` are refused by Origin
  2016's `impasc` (the command fails): counting the lines from the bottom and the one-line name+unit
  splitter of the documentation are later versions' options.

Sheaf (T299): the label lines detected above the data are a run of up to 15 lines (Origin's
CommentTo 15; the detection looked at three before); the comments are the lines from the comments
line to the last comments line, joined with line breaks (empty cells left out). `comments_to`
(window "Last line of comments", CLI `--comments-to N`) sets the last line; by default it is the end of
the detected label lines, or the comments line alone when that was given. The Excel import uses the
same detection (its window keeps one comments line).

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

**Measured in Origin 2016 (T312, 2026-11-21; `re-workspace/scratch/import-nonnum/`).** `impasc` with
`options.Miscellaneous.NonNum` on a tab file `x, y` / `1 10`, `2 20`, `3 oops`, `4 40`, `5 50`:

- 1 (the default): every row; `oops` stays a text cell in a Text & Numeric column.
- 0 (terminate) and 5: only the rows above the bad row (x 1, 2; y 10, 20); 5 does not skip the row.
- 2 (new worksheet): the book gets sheet `data1` with rows 1, 2 and sheet `data1_` with rows 4, 5; the
  bad row is left out.
- 3 (new column): one sheet of four columns, x 1, 2 | y 10, 20 | 4, 5 | 40, 50; the new columns start at
  row 1 and the bad row is left out.
- `options.Miscellaneous.LeadingZeros` exists in 2016 (not measured further).

Sheaf (T312, `AsciiOptions::non_numeric`, `NonNumeric`): Read as text (default), Stop (Origin's 0 and
5), New sheet (2; sheets named after the first with `_` added per cut, `data1_`, `data1__`), New
columns (3; short names continue C, D, ..., no labels). A column is numeric when its first non-empty,
non-marker data cell is a number or a date; a bad row is a data row with a text cell (a marker does not
count) in a numeric column. The header lines stay with the first part; a warning says how many rows
were cut out. Window: Miscellaneous > "Text in a numeric column"; CLI `--non-numeric
text|stop|new-sheet|new-columns`. Origin's 4 (new hidden workbook) is not built.

### What Sheaf implements (crates/io/src/ascii.rs)

Rules 1 and 2 were first written from the documented NonNum=1 behaviour; the measurement in section 15 ("Text & Numeric") replaced the 80 percent rule: a column keeps its text cells (`ColumnData::Mixed`), so nothing is turned into a missing value and no warning is needed for it. Rules 3 and 4 were not checked against a running Origin.

1. No-value markers. These are not numbers: `NA`, `N/A`, `#N/A`, `n.a.`, `n.d.`, `nd`, `bdl`, `#VALUE!`, `#DIV/0!`, `#REF!`, `#NUM!` (any letter case), any string made only of dashes (`-`, `--`, `----`, en and em dashes), and any string that starts with `<` followed by something (`<LOD`, `<0.05`). `NaN` and `inf` are real numbers. Since the measurement in section 15 a marker is a text cell like any other non-number: it stays in the cell as text (shown as typed) and the numeric views see it as missing. A column that holds only markers is a text column.
2. Column type (changed, see section 15): a column with at least one number is a Text & Numeric column (`ColumnData::Mixed`); every cell that is not a number stays a text cell. There is no ratio. A column without any number is a text column. A text column is designated Label, except the first column when "first column is X" is on (the default): Origin designates the columns of a text file by position, so sample names in the first column are the X column and the Y columns are drawn against them (graphing.md section 14.6).
3. Data start. Markers do not break a run of numeric rows. When the first run of three numeric rows is found, the start moves up over directly preceding rows that have the same field count and at least as many numbers or markers as text cells, so `3,trace` above numeric rows stays data. A name row (no numbers) or a units row stays a header line. In files with a text column the first data row is the first row that has a number or marker, has text in its other fields, and is followed by rows with the same field count in which at most half of the cells in its numeric columns are text.
4. Quoted fields may contain line breaks. The text is cut into records, carrying the open-quote state across physical lines; a quote opens a field only at the start of a field (after a delimiter, a space or the start of a record). If a quote is still open at the end of the file, the file is cut into plain lines instead. Header line numbers in the options and in `Detected` count records, not physical lines. CRLF and LF both work, and a line break inside quotes is kept as a line feed in the cell.

Not decided (but see section 15, where Sheaf keeps them as text cells as Origin keeps every non-number): whether Origin treats `-` or `<LOD` differently from other words.

## 8. Date and time detection (P0 for time-stamped logs, P1 otherwise)

| Option | Meaning | Default | Source |
|---|---|---|---|
| DateFormat | A custom date pattern to match. If cells match, the column format becomes Date. | none | [W] |
| TimeFormat | A custom time pattern. A match makes the column Time. | none | [W] |
| KeepDestColFmt (CSV) | Keep the target column's format instead of re-detecting. | 0 | [W]; CSV.oif `KeepDestColFmt` [F] |

Origin keeps a menu of ready patterns (also stored in Origin.ini `[Column Custom Format List]`, observed [F]), using these token letters: `yyyy yy MM MMM dd d HH hh mm ss tt` and `###` for fractional seconds. The twelve entries:
`yyyy'-'MM'-'dd HH':'mm':'ss`, same with `'.'###`, `yyyy'-'MM'-'dd hh':'mm':'ss tt`, `dd'/'MM'/'yyyy`, `dd'-'MM'-'yy`, `MM'/'dd'/'yyyy`, `MM'-'dd'-'yy`, `yy'/'MM'/'dd`, `yy'.'MM'.'dd`, `dd'.'MM'.'yy`, `yyyyMMdd`, `dd'-'MMM'-'yy`.

Behavior [W wkscol-datetime]: unless a date format is given or recognised, date/time strings arrive as text (left-aligned) and the user converts the column by setting its format. Internal date value = a double, days (fraction = time of day) in a Julian-day-based system. Whether the automatic pass recognises ISO-like "2020-01-31 12:00:00" without a user pattern is **[?]**; plan to recognise ISO 8601 and the pattern list above.

### Observed in Origin (black-box runs of `impasc`, 2026-11-12) [F]

Files were small CSVs we wrote (US regional settings on the test machine). Origin's own automatic pass only turns some strings into dates; everything else arrives as text (column format 7) and stays left-aligned:

| String in the file | What the default import gave |
|---|---|
| `13:45`, `01:05` | Time column (format 3, `HH:mm`), value 0.5729166667 for 13:45 |
| `13:45:10.250` | Time column, fraction kept (0.5730353009) |
| `13/11/2026`, `02/12/2026` | Date column (format 4) read month first: row 1 (day 13) did not appear in the column, and row 2 was read as 12 February (value 2461083); Origin gave no warning |
| `2026-11-11 13:45:10`, `2026-11-11` | text |
| `11/11/2026 1:45:10 PM`, `11-Nov-2026` | text |

Setting the column to Date (format 4, subformat 11) afterwards converts text that matches the system pattern: `11/11/2026 1:45:10 PM` became 2461355.5730324076 and `11/12/2026 1:05:00 AM` became 2461356.045138889. The ISO strings turned into missing values. `11-Nov-2026` converted to the wrong date (11 November read as 12 November, and 12-Nov as 12 December), so Origin's conversion of day-month-name strings is not something to copy. The `Dates.*` option names we guessed for `impasc` were rejected; Origin's import wizard has a per-column date format that we did not drive.

**Two-digit years, measured in Origin 2016 (T315, 2026-11-21; `re-workspace/scratch/import-yy/`).**
`impasc` on tab files of nine dates each, US regional settings:

| Strings | What the default import gave |
|---|---|
| `1/2/21`, `3/4/29`, `5/6/30`, `12/31/99`, `1/1/00` (M/d/yy) | Date column (format 4), shown as `1/2/2021` (M/d/yyyy); 00 to 44 became 2000 to 2044, 45 to 99 became 1945 to 1999 (`1/2/44` is 2044, `1/2/45` is 1945; values 2459216 for 1/2/21, 2451544 for 1/1/00) |
| `13/2/21` (d/M/yy), `13.02.21` (dd.MM.yy), `21-03-05` (yy-MM-dd), `05-Mar-21` (dd-MMM-yy) | text (format 7) |

Whether the 44/45 window moves with the current year was not measured (one machine date).

Decision for Sheaf: recognise more than Origin's automatic pass does, because the owner's instruments write ISO and day-month-name strings. The numbers stored are the ones Origin uses (see `workbook-model.md`).

### What Sheaf implements (crates/io/src/ascii.rs)

- A column is a date column when every filled cell reads as a date, a time of day, or a date with time of one kind. Recognised: `yyyy-MM-dd` (also with `/` or `.`), a `T` or a space then a clock, `M/d/yyyy` and `d/M/yyyy` (also with `-`), `dd.MM.yyyy` (always day first), `11-Nov-2026`, `11 November 2026`, `13:45`, `13:45:10.250`, and 12-hour clocks with AM or PM. No-value markers (`n.d.`, `--`) and empty cells in the column become missing values.
- Two-digit years (T315): every year-last form also takes a two-digit year, read with Origin's measured window (00 to 44 are the 2000s, 45 to 99 the 1900s): `1/2/21` and `13/2/21` (the day and month order rule below applies), and with two-digit day and month `13.02.21`, `05-03-21`, `05-Mar-21`. One-digit parts with dots or dashes (`1.2.21`, a version number) and year-first `yy-MM-dd` stay text. The column is shown with four-digit years (`1/2/2021`), as Origin shows it. Set Column Values, Fill and the text-to-date column conversion read the same forms (`sheaf_core::datetime::parse`).
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

### Implemented range and repeating-pattern selection

`AsciiOptions::rows` and `columns` use `ImportSelection { first, last, read, skip }`, following the documented [Partial_rows / Partial_cols options](https://docs.originlab.com/x-function/ref/details-of-treenodes-in-import-ascii/). Positions are 1-based and inclusive; `last = 0` means through the end. Starting at `first`, take `read` entries, skip `skip`, and repeat until `last`. The default `{ first: 1, last: 0, read: 1, skip: 0 }` imports everything. For example, first 2 / last 5 / read 1 / skip 1 imports positions 2 and 4. First and read must be positive; a nonzero last must be at least first. An out-of-range selection that imports nothing is reported as no data.

The Import window's Partial Import page and the wizard's shared Data Selection controls expose both axes. CLI flags are `--first-row`, `--last-row`, `--read-rows`, `--skip-rows`, `--first-column`, `--last-column`, `--read-columns`, and `--skip-columns`. Data-row positions exclude header records and blank records (the existing importer skips blank data records). Legacy API `skip_rows` and `max_rows` still define an initial window; the new row selection then applies within it. These legacy settings default to zero/no limit and are distinct from the CLI repeating `--skip-rows` flag.

Selected columns keep their source Long Name, Units, Comments, short-name header values, and parameter values. Parameter names still come from the original first field even when that column is excluded. Automatic short names are assigned consecutively to the imported columns; the first imported column is X when that option is enabled. These naming/designation choices preserve existing output conventions and have not been newly measured in installed Origin. Preview, detection, and final import agree on the selected data. Selections apply to every file by default and reset per block when importing blocks as sheets. The **All files except first** scope (CLI `sheaf import --partial-except-first`, API `BatchOptions::partial_except_first`) leaves the first supplied data file's rows and columns intact while selecting later files. Delimiters, header labels, number/date settings, and naming choices still apply to the first file. The first-file boundary follows input order after file-pattern expansion, even if that file cannot be read; a later file is not promoted to first. Import-window detection and preview use the same effective per-file options as the final import, and the wizard shares the scope control. `AsciiOptions::without_partial_import` clears both the new ranges/patterns and the legacy skip/max window. Custom range-list strings remain unimplemented.

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

Sheaf's naming options (`rename_sheet`, `rename_book` in `AsciiOptions`, both default on like Origin's "Rename Sheet with (Partial) Filename" and "Rename Book with (Partial) Filename"; CLI `--no-rename-sheet`, `--no-rename-book`): off gives the sheet the name `Sheet1` (`Sheet<n>` for a new sheet or a later block; a replaced sheet keeps its own name) and the book an empty long name. On, the book long name is the whole file name **with its extension**, as Origin (owner's decision 2026-11-15: an imported `data.csv` is titled `data.csv`; CH Instruments text and binary files too, `run1.txt`), and the sheet the name without it. `sheet_name_range` and `book_name_range` (`(from, to)`, default `(1, 0)`) cut them by Origin's Trim Filename rule below (`ascii::trim_file_name`, `ascii::parse_ascii_named`); the import window shows "Trim Filename From / To" under each tick. A graph made from an imported book whose title is its file name is titled without the extension (`Workbook::title_for_graph`), so graph titles and `<long name>` export file names stay `data` (Origin's graph titles here were not measured). The overlay legend of several files (CLI `plot` with several files) also uses the title without extension. Not yet offered: CLI flags for the ranges, so CLI import themes (`sheaf import --theme`) do not carry them.

**File name trimming, measured 2026-11-15** (Origin 2016, `impasc` on `abcdefgh.csv` with `AutoNames:=0`,
`FNameToSht:=1`, `FNameToBk:=1` and the same From/To for both; `re-workspace/scratch/fntrim`):

- The sheet name is cut from the file name **without** its extension (`abcdefgh`, 8 characters); the book long name
  from the file name **with** it (`abcdefgh.csv`, 12 characters). With From 1, To 0 the long name is `abcdefgh.csv`
  (Sheaf copies this since 2026-11-15, owner's decision).
- From and To are 1-based character positions, both included: (2, 5) gives `bcde` for both names; (3, 3) gives `c`.
- A negative number counts from the end, `len + n`: To -4 gives the sheet `abcd` (8 - 4) and the long name
  `abcdefgh` (12 - 4, which drops `.csv`); From -6 gives the sheet `bcdefgh` (from 8 - 6 = 2) and the long name
  `fgh.csv` (from 6); (-6, -4) gives `bcd` and `fgh`.
- From 0, or a From before the first character (-20), means the first character; To 0, a To past the end (20), or a
  To before From (5, 2) means the last character.
- A range that leaves nothing (From 9 on the 8-character sheet name) keeps the default sheet name (`Sheet2` in that
  run) and the default book short name (`Book1`); the long name still takes its own range (`.csv`).
- The book short name follows the sheet's text (the range applied to the name without extension), numbered when the
  name is taken (`abcdefgh1`, `abcdefgh2`).

"Rename Long Name for Book only", "Include File Path when Renaming Book", "Auto Rename Using Filename" and the three "Append Filename to ... Comment" options are not offered.

### The impASC window, measured in Origin (2026-11-13)

Read with `tools/origin-ref/origin-dialog.ps1` from `impasc fname:=... -d;` (control list, combo box items, pictures kept locally).

- **Buttons: OK and Cancel only. There is no Apply** (the earlier guess, from the general X-Function layout, was wrong). Title `ASCII: impASC`; at the top the Dialog Theme row and the description `Import ASCII file/files`; a tick `Results Log Output` (off); `File Name` with a browse button; at the bottom `Output` (`[Book1]Sheet1`).
- Tree of groups: `File Info`, then `Import Options` holding `Add Sparklines` (No, Yes, Yes(if less than 50 columns); default the last), `Import Mode` (list `Replace Existing Data`, `Start New Books`, `Start New Sheets`, `Start New Columns`, `Start New Rows`; default Replace Existing Data), `Template Name` (`<default>`, `<clone>`, ...), and the groups `File Structure`, `Columns`, `Header Lines`, `(Re)Naming Worksheet and Workbook`, `Partial Import`, `Miscellaneous`, `Script`. **There is no page called Data Import Options**, no tick "rename columns from the header" and no "apply new range to plots".
- File Structure: Data Structure (`Fixed column widths`, `Delimited - Single Character` (default), `Delimited - Multiple Characters`), Delimiter (`Unknown` (default), `Tab/Space`, `Tab`, `Comma(,)`, `Semicolon`, `Space`, `Other`), Delimiters (text), Numeric Separators (`1,000.00` (default), `1.000,00`, `1 000,00`, `1'000.00`), Custom Date Format (`<none>` and 12 patterns such as `yyyy-MM-dd HH:mm:ss`, `dd/MM/yyyy`, `yyyyMMdd`), Custom Time Format (`<none>`, `hh:mm:ss T`, `hh mm ss.##`), tick Transpose Data (off).
- Columns: Number of Columns (0), tick Auto Determine Column Types (on), Min Number of Consistent Lines to Determine Data Structure (4), Max Number ... (50), Column Designations (`<Unchanged>`, `None`, `2Y`, `X2Y`, `(XY)`, `(X2Y)`, `(X3Y)`, `(XYE)`, `XY2Z`, `(XYZ)`).
- Header Lines: Number of Main Header Lines (grey, 0), tick Auto Determine Subheader Lines (on), tick Line Number Start from Bottom (off), Number of Subheader Lines (grey, 0), tick Extract Long Names and Units from Same Line (off), **Short Names `<none>`**, Long Names `1`, Units `2`, Comments From `3`, Comments To `15`, System Parameters From `<none>`, To (grey 0), User Parameters From `<none>`, To (grey 0). The line lists are `<none>` and 1 to 10. Setting Long Names, Units and Comments to `<none>` is how the labels are left alone.
- **(Re)Naming Worksheet and Workbook** (all defaults): `Auto Rename Using Filename` off; `Rename Sheet with (Partial) Filename` **on** with `Trim Filename From` 1 and `To` 0; `Rename Book with (Partial) Filename` **on** with Trim From 1 and To 0; `Rename Long Name for Book only` off; `Include File Path when Renaming Book` off; `Append Filename to Workbook Comment` on; `Append Filename to Worksheet Comment` off; `Append Filename to Column Comment` off; `Include File Path when Appending Filename` on.
- Partial Import: `Partial Import` (`None` default, with From/Read/Skip/To Column and Row boxes grey until it is chosen). Miscellaneous: Text Qualifier (`NONE`, `"`, `'`), tick Remove Text Qualifier from Quoted Data (grey, ticked), tick Remove Leading Zeros from Numbers (on), When Non-numeric is Found in Numeric Field (`Terminate Import`, `Read as Text` (default), `Start New hidden Workbook`, `Start New Worksheet`, `Start New Column`), ticks Allow Import All Text Data (on), Keep Target Column Format (off), Save File Info. in Workbook (on). Script: two text boxes (after each file, after all files).

**Apply the new range to plots.** Origin has no such option; what it does when the data of a sheet is replaced (`impasc` into the active sheet that holds the data of two graphs, measured through `get %C -b` and `-e`): a plot of all rows follows the new data (5 rows replaced by 12: the plot's range goes from 1..5 to 1..12), and a plot that was limited to a row range keeps its range (2..4 stayed 2..4). Sheaf now does the same: `PlaceOptions` has no range option any more; replacing the data keeps the identifiers of the columns by position so that graphs keep plotting them, a plot with `rows: None` shows all new rows and a plot with a row range keeps it (this replaces the earlier option that reset the ranges, default on).

Sheaf's window follows this: OK and Cancel (the Apply button and `ImportOutcome::Apply` are gone); the mode list reads `Replace Existing Data`, `Start New Books`, `Start New Sheets`, `Start New Columns`, `Start New Rows` (`ImportMode::label`); the node that used to be called "Data Import Options" is `(Re)Naming Worksheet and Workbook` with `Rename Sheet with (Partial) Filename` and `Rename Book with (Partial) Filename`, both on; the short-name line is on the Header Lines node (`Line with the short names`, none by default).

**Add Sparklines (T231, 2026-11-20, not measured beyond the list).** The Import Options node has Origin's
`Add Sparklines` list with its three texts: `No`, `Yes`, `Yes(if less than 50 columns)`
(`AsciiOptions::sparklines`, `AddSparklines`; saved in dialog themes; CLI `sheaf import --sparklines
no|yes|auto`). Yes marks every imported column for a sparkline and turns the sheet's Sparklines row on
(workbook-model T175); the third does so only for a sheet of fewer than 50 columns (49 columns get them,
50 do not), per sheet when the blocks of a file become sheets. Imported into an existing sheet, the
arriving columns keep the mark and the target sheet's row is turned on (Start New Columns marks only the
new columns; Start New Rows marks the columns the rows went into). **Sheaf's default is `No`**, not
Origin's `Yes(if less than 50 columns)`: the `.opj` writer does not keep sparklines yet, so a default
import saved as `.opj` would always need a companion file. Auto column width (`nAutoSize`) and write
protection (`DataWriteProtection`) are not implemented.

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

### 13.1 Measured in Origin (2026-11-12)

Run through `tools/origin-ref/run-labtalk-lines.ps1`; inputs and Origin's raw output files are in
`re-workspace/scratch/sheet-export/` (`probe1.lines.txt` .. `probe6.lines.txt`, the files in `out/` .. `out5/`,
local only). Call form: `expASC iw:=[Book1]1! type:=csv path:="FOLDERile.csv" ...`. The `path` is the
file name; Origin adds the extension of the type (`.csv`, `.dat`, `.txt`) when the name has none of its own
(`base.out` became `base.out.csv`).

- **Defaults (nothing but `type` and `path` given):** one row each of long names, units and comments (a row is
  left out when no column has text for it), no short-name row, no user-parameter rows, no row labels, numbers in
  full precision, missing values as `--`. Lines end in CR LF, also after the last row. The sheet written is the
  active sheet when `iw` names only the book (a book with two sheets, second active, wrote the second);
  `[Book]1!` writes sheet 1. The 2016 defaults of the OXF table above for `comment` (1) and `userparam` (0) are
  what happens; `suffix:=x` changed nothing.
- **Row order** with everything on: short names, long names, units, comments, then one row per user parameter
  (in the order the parameters were created), then data. The parameter rows carry only the values, no name,
  unless `rowlabel:=1`.
- **`rowlabel:=1`:** a first column; label rows begin with `Short Name`, `Long Name`, `Units`, `Comments`, or the
  parameter's name; data rows begin with the row number 1, 2, 3...
- **Separators:** `type:=csv` comma; `csvsep:=1` semicolon, and then every number is written with a **decimal comma**
  (`1,5;2,5`, also `40000:12:00:00,00` for a date). `type:=dat` / `text` tab; `separator:=1` comma, `2` space,
  `3` comma+tab (`,	`), `4` comma+space (`, `). The file extension does not choose the separator.
- **Text cells:** a quote becomes two quotes. In the csv type a text that holds the separator (`a,b`; `a;b` when
  the separator is a semicolon) or a quote is also wrapped in quotes and its quotes doubled again, so `q"x`
  is written `"q""""x"` (in the dat type: `q""x`, no wrapping, `a,b` plain even with comma separator). Spaces at
  the ends and an apostrophe are written as they are. The label rows go through the same rule.
- **Missing values:** `--` inside a column (numbers and text), nothing past the last value of a column (the
  field is empty). `missing:=0` writes an empty field inside a column too.
- **Full precision (`precision:=1`, default):** 15 significant digits, trailing zeros dropped (`0.333333333333333`,
  `1.23456789012346E17`). Plain notation from 0.001 up to 1E15 inclusive (`0.001`, `1000000000000000`, `123456789012345`),
  otherwise scientific with capital E, no plus sign and no leading zeros in the exponent: `1E-7`, `-1.23456E-4`,
  `1.2E-4`, `1.5E15`, `2E15`, `1E16`, `1E300`. Zero is `0`; -0 is `0`.
- **Displayed digits (`precision:=0`):** each cell as its column shows it, e.g. General `0.12346`, `0.33333`,
  `1.23457E17`, `1E-4`; significant digits 2 gave `3.1` and `1.2E+06`; decimal places 3 gave `3.142` and `1.235E+06`
  for 1234567.891 (so a value of 1E6 or more is shown in scientific notation, the Options threshold power 6),
  decimal 2 `1234.50`.
- **LabTalk column format (for the probes):** `wks.colN.digitMode` 0 = significant digits, 1 = decimal places;
  `wks.colN.digits` the count; do not set `wks.colN.format` to 1 (that is Text).
- **Not measured:** how dates are written under full precision (the probe's date column came out as displayed
  either way, `40000:12:00:00.00`), `parameter` and `sampinterv` (no visible effect on a plain sheet), `select`,
  `overwrite` (a second call on the same path appended), `addxcol`.

Sheaf (`crates/io/src/export.rs`, `write_delimited`, Export window `dialogs/export_sheet.rs`) follows these.
**Selected cells only (T247, `select`; measured in Origin 2016 on 2026-11-21, T273: `worksheet -s 2 2 3 3`
then `expASC type:=csv select:=1 [rowlabel:=1]`, `re-workspace/scratch/t273`).** The Export window's **Range** row has
**Selected cells only**, enabled when the active sheet had a selection as the window opened (cells,
whole rows, whole columns or the whole sheet). It writes the selected columns in sheet order and the
selected rows (`write_selection`): the label rows are those of the selected columns, and with row
labels each data line starts with its row number in the sheet (not 1, 2, ...). A selected row past
the end of a column gives an empty field, as in a whole-sheet export. Measured: with long names LA,
LB, LC and C's unit uC, the block B2:C3 gives `LB,LC` / `,uC` / `20,200` / `30,300`, and with row
labels `Long Name,LB,LC` / `Units,,uC` / `2,20,200` / `3,30,300` (the sheet's row numbers); Sheaf
writes the same bytes. Origin quirk not copied: for a whole selected column its Long Name line ends
with one more empty field than the data lines. Origin's handling of a
non-contiguous selection is not copied (Sheaf's selection is one block or a set of whole columns). Known
differences: user-parameter rows are written in alphabetical order (Sheaf does not keep creation order); a text
cell that is empty between values is written as `--`; numbers 15 digits long do not read back to the last bit.

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

### Observed in Origin (black-box runs of `impasc`, 2026-11-12) [F]

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

### Text & Numeric columns (Origin, measured 2026-11-12, `impasc`) [F]

Origin has no ratio of numbers to text cells. A column whose cells are 95 percent numbers and a column whose cells are 55 percent numbers both came out with `wks.col.format` 7 (Text & Numeric, Origin's default column format), and every text cell stayed in the sheet as text (`cell(11,2)$` read `four`). The text cells count as missing in calculations (workbook-model.md section 6). Typing text into a number cell of such a column keeps the text too, unlike the Numeric format (format 1), where text typed in becomes missing. Inputs were synthetic files we wrote.

What Sheaf does (`ColumnData::Mixed`, `crates/core/src/column.rs`):

- A Text & Numeric column stores the numbers (NaN for a missing value and for a text cell) and a map from row to text. `Column::numeric()` gives the numbers, so plots, analysis, fits and formulas see a text cell as missing and are unchanged. Display, copy, export and `.opj` writing use the text. A number written over a text cell wins.
- `ColumnData::default()` (a new column, a new workbook, a pasted-into column beyond the data) is an empty Text & Numeric column, as in Origin. Sheaf's explicit Numeric format (`ColumnData::Numeric`) stays for result columns and date/time columns, and still turns typed text into a missing value (Origin's format 1).
- ASCII import: a column with at least one number is Text & Numeric, the other non-empty cells (words, `<LOD`, `n.d.`, `NA`) are kept as text. A column with no number at all is text (Label designation, or X for the first column), as before. Designation stays by position (X for the first column when "first column is X", then Y). The warnings "N cells are not numbers" and "N cells hold a no-value marker" are gone; nothing is lost. Date and time columns are unchanged (Numeric with a date format). CH Instruments importers produce Numeric columns as before.
- Typing and pasting text into a Text & Numeric cell keeps the text. A Numeric column that is pasted text into still turns to text when it is empty outside the pasted block, as before.
- `.sheaf` files store the numbers as before and the text cells in `data/<id>.mixed.json` (an object row -> text); the member marks the column as Text & Numeric, and older files without it load as Numeric.
- `.opj`: Origin stores every column as text-or-number cells (opj-format.md section 3.4), so a Text & Numeric column is written natively: text cells as text cells, numbers as number cells, one cell size for the column (longest text plus 3, at least 10). The reader keeps text cells of columns that also hold numbers (before, such a column became text with a warning); a column of cells that are all text is still a text column.
- Worksheet export and Copy write the text cells (quoted when needed in export) next to the numbers.
- Sort uses the numbers (text cells sort as missing, last). Set Column Values on a Text & Numeric column stays Text & Numeric and keeps the text cells outside the filled rows.

Not done: file name trimming for sheet and book names (`FNameToShtFrom/To`), per-file modes (`FirstMode` against `Mode`), parameter and block options in the import window, joining several comment lines.

## 16. Import Wizard pages (measured 2026-11-13)

Origin's Import Wizard (File > Import > Import Wizard..., Ctrl+3; opened with `origin-dialog.ps1
-MenuClicks "300,300;K:^3"`, control lists in `re-workspace/scratch/t19wiz/w4`, `w5`) titles its window
"Import Wizard - <page>". For an ASCII file the pages are, in order:

1. Source: Data Type ASCII, Binary, User Defined; Data Source File (with a browse button) or Clipboard;
   Import Filter (List filters applicable to both Data Type and file name, on; Import Filters for current
   Data Type `<None>` and the filters of Origin's folder; Description); Target Window Worksheet, Matrix,
   None; Template `<default>`, `<clone>`, `excel.otw`; Import Mode Replace Existing Data (default), Start
   New Books, Start New Sheets, Start New Columns, Start New Rows.
2. Header Lines: Number of main header lines (exclude subheader lines) and Number of subheader lines (0 to
   10), Auto determine header lines, Line number start from bottom, Long Names, Units, Short Names,
   Comments (from one line to another), System Parameters and User Parameters (line ranges), Extract Long
   Names and Units from Same Line, Column Label Assignment from Subheader Lines (prefixes S, L, U, P, C, MH,
   SH), Characters to skip on each line, Preview Lines (10 to 500), Preview Font.
3. Variable Extraction: Extract variables from file names and file headers, by delimiters, by character
   positions, by a scanned delimiter or by an Origin C function; Add variables to page info; Create User
   Parameter rows for each variable; File Info: Save file info. in workbook (on).
4. File Name Options: Rename worksheet with filename (on), workbook with file name (on), Include path when
   rename workbook, file name to workbook comments (on), file name to column comments, Include path when
   appending file name (on), worksheet or workbook with a variable, Rename Long Name for Book only, and
   Rename, Append or Auto for multiple files.
5. Data Columns: Delimiter (Tab/Space, Tab, Comma, Semicolon, Space, Other) or Fixed Width, Treat
   consecutive delimiters as one, Number of columns, Text Qualifier `<None>`, `"` or `'` with Remove,
   Column Designations (`<Unchanged>`, 2Y, X2Y, (XY), (X2Y), (X3Y), (XYE), XY2Z, (XYZ)), Custom Date Format
   and Custom Time Format lists, Keep target column format, Remove leading zeroes from numbers (on), Force
   Rows to same size, Numeric Separator (1,000.00 and three others), Add Sparklines (Yes if less than 50
   columns), a column preview with Add Column, Delete Column and Column Separator.
6. Data Selection: Partial Import From Column / To, From Row / To with Skip Rows and Read Rows, Non-numeric
   data in a numeric field (Read as text by default; Terminate import, Start new (hidden) book, Start new
   worksheet, Start new column), and for several files None, All Files or From 2nd File On.
7. Save Filters: Save filter in the data file folder, the User Files folder (default) or the window; filter
   file name; data file names to associate; description; Show Filter in File:Open List; Specify advanced
   filter options.

Data Selection rules, measured through the wizard on a 5 column by 12 row file (`re-workspace/scratch/t19wiz/ps2`,
`ps3`): From Column 2 To 4 imports columns b, c, d; From Row 3 To 10 imports data rows 3 to 10 (counted after
the header, both ends included), the header still naming the columns; Read Rows 2 and Skip Rows 1 inside that
range keep rows 3, 4, 6, 7, 9, 10, and the same for columns keeps b, c (the pattern starts with a read). The
wizard remembers these settings for the next import. Sheaf's `ImportSelection` gives the same rows and columns
(test `partial_import_matches_origins_data_selection`).

Sheaf's wizard has these seven pages with these titles (`import_wizard.rs`): Header Lines holds the header
line count and the Long Names, Units, Comments and Short Names lines; Variable Extraction and Save Filters
are shown greyed; Data Selection holds Sheaf's partial import (rows to skip and to read); Data Columns the
delimiter, designations and number and date settings. The other controls listed above are not offered.

### 16.1 Quoted fields and the CSV import (measured 2026-11-14)

On a file with quoted fields (`"Smith, J",1.5,"a ""quoted"" word"` and `'single, quote'`,
`re-workspace/scratch/partimp`): `impASC` (File > Import > Single ASCII) has no text qualifier by default and
splits inside the quotes; `impCSV` (File > Import > Comma Delimited (CSV)) treats `"` as the qualifier, removes
it and reads `""` as one quote, but not `'`. Run from a script with no options, `impCSV` keeps every header line
as data (Text & Numeric columns, no long names), while `impASC` makes the header into long names and units.
Sheaf splits fields like `impCSV` (double quotes are a qualifier) and finds header lines like `impASC`; no change
was made.

## 17. Dialog themes (saved import settings, T46)

The mechanism is in `crates/io/src/themes.rs`; the theme row and the command-line flags are built by the app
and CLI parts. Labels: **[F]** measured in Origin, **[S]** Sheaf's own choice, not measured (no Origin was
available when this was written), **[M]** to measure.

### 17.1 What is known

- **[F]** The impASC window has a Dialog Theme row at the top, above the description (section 10).
- **[F]** The other X-Function windows (NLFit, Peak Analyzer, Export Graphs) show the row as a label, a wide box
  for a theme name and a small arrow button that opens a menu of theme actions (`gui-look.md`). The items of
  the menu were not seen.
- **[F]** The Import Wizard has a separate "Import Filter" list (`gui-look.md`; section 16). Origin keeps
  those filters in `.oif` files. Sheaf does not read or write `.oif` files (clean-room; the Import Filters
  Manager is left out).

### 17.2 What Sheaf does

All of this is **[S]** unless it says otherwise.

- **The row.** Each import window (text, CH Instruments options, Excel) gets a Dialog Theme row at the top: a
  combo box and a small menu with Save, Save As... and Delete. The combo lists `<default>`, `<last used>` and the
  saved names, in that order; the saved names keep the order in which they were first saved.
- **`<default>`.** The built-in defaults of the window (`AsciiSettings::default()` and so on).
- **`<last used>`.** The settings of the last OK in that kind of window. Cancel records nothing. The list shows
  `<last used>` before the first OK, and choosing it then gives the defaults.
- **A fresh window.** Opens with `<last used>` when it exists, otherwise `<default>`. A file dropped on the
  window or passed on the command line without `--theme` is not changed by this.
- **What a theme stores.** Every option of the window except the file list and the target sheet or book: the
  text options (`AsciiOptions`), the data type choice, the CHI options (`ChiOptions`), the Excel options
  (`ExcelOptions`, without the list of ticked sheets, which belong to the files), the import mode, the
  partial-import tick, the graph tick, and the text of the boxes (other delimiter, fixed widths). The graph
  template is not stored. The import mode is stored, but a window with no target sheet still falls back to
  new books: the window keeps the mode that was chosen (shown as New Book while there is no target) and only the
  import itself uses a new book, so Save and OK store the theme's mode and not the fallback.
- **Save, Save As, Delete.** Save writes the current settings over the selected saved theme (it asks for a
  name when `<default>` or `<last used>` is selected). Save As asks for a name. Delete removes the selected saved
  theme; the two built-in entries cannot be deleted. After Save or Save As the combo shows the saved theme;
  changing a control afterwards leaves the name shown but does not change the stored theme.
- **Names.** Leading and trailing white space is cut, runs of white space become one space, control characters
  are dropped, and a name is at most 60 characters (longer names are cut). A name must not be empty and must not
  be in angle brackets (`<...>` is for the built-in entries). Two names that differ only in letter case are the
  same theme: saving under a taken name replaces that theme in its place and takes the new spelling.
- **One store per importer kind.** `ascii`, `chi` and `excel`, each with its own `<last used>` and its own
  saved themes. A theme saved in the Excel window is not offered in the text window. The CH Instruments options
  are in the `chi` store; the text window with the data type set to CH Instruments (or detected as such)
  applies the `chi` store to the CHI options and the `ascii` store to the rest.
- **Where.** One file per kind in the folder `import-themes` inside the settings folder (the folder of
  `autosave.json`, `recovery::default_settings_path`): `ascii.json`, `chi.json`, `excel.json`. The file is JSON:
  `{"version": 1, "last_used": {...}, "themes": [{"name": "...", "settings": {...}}]}`. It is per user, not
  per project, so a theme is available in every project.
- **Compatibility.** Every option struct is read with defaults for fields that are missing (a file from an
  older Sheaf keeps loading and the new options get their defaults) and ignores fields it does not know (a file
  from a newer Sheaf loads). Field names of the option structs are therefore part of the file format and are
  not renamed. A theme whose values this version cannot read at all (a new variant of an enum) is listed but
  cannot be chosen, and is kept when the file is written again. A file whose `version` is above 1 is also
  kept as `<name>.json.bad` before this version writes it again (what this version can read is carried over;
  the fields it does not know are lost from the new file, which is why the backup is made).
- **A damaged file.** A file that is missing, unreadable or not a JSON object (or whose `themes` is not a list)
  gives empty lists. The next change keeps a damaged file as `<name>.json.bad` before writing a new one. A file
  that exists but cannot be read (no permission, a sharing violation) is not replaced: the change fails with
  the error, which the window shows under the row. Only a missing file starts an empty store. A damaged entry (no name, no settings,
  a bad or repeated name) is skipped and the others are kept.
- **Two instances.** Changes take a lock file, read the file again, apply the change and write it atomically
  (as the recent files, `recent-files.md`), so themes saved by another running instance are not lost.
  The `<last used>` of the instance that confirms last wins.
- **Command line** (`crates/ops/src/themes.rs`, flags in `ReadOptions`, so every command that reads a data file
  has them: `import`, `info`, `plot`, `export`, `fit` and `analyze ...`; the MCP server does not
  offer them). `--theme NAME` and `--last-used` (same as `--theme "<last used>"`; the two cannot be combined)
  start from the stored settings, and options typed on the command line win. Details:
  - *Which store.* The entry is looked up in the text, CH Instruments and Excel stores, in that order, and every
    store that has it sets its part: the text store the text options, the data type (`--format`) and the options
    the readers share; the CH Instruments store the `--no-split-scans`-style CHI options; the Excel store the
    hidden-sheet choice and the options the readers share, for Excel files. So a name (or `<last used>`) that
    both the text and the Excel window saved reads a text file with the text window's settings and a workbook
    with the Excel window's, as each window's own `<last used>` does (`ReadSettings::excel_shared`). When only
    the Excel store has the name, text files take its shared options too (Sheaf's choice; there is nothing
    else to read them with). Two things stay one value per run: the partial-import tick (the text store's when
    it has the name, else the Excel store's) and the data type. `<default>` is the built-in settings (the
    same as giving no flag).
  - *Typed options win.* An option counts as typed only when it appears on the command line; a value that equals
    its default (`--first-row 1`) still wins. The four delimiter options (`--delimiter`, `--delimiter-chars`,
    `--combine-delimiters`, `--fixed-width`) act as one, so any of them replaces the theme's delimiter whole. A typed option also
    replaces the Excel store's value of that option for Excel files. A delimiter that a theme holds as a
    character or a whole string is used as it is, not read as a keyword: a stored `tab` (three characters)
    stays the string `tab`, where `--delimiter tab` typed on the command line means a tab.
    A switch such as `--decimal-comma` can only turn an option on: to run without an option that a theme turns
    on, save a theme without it.
  - *What is not applied.* The import mode and the graph tick belong to the window (a target sheet, a template)
    and are not applied: use `--mode` and `--plot`. The partial-import tick is applied (the theme turns on
    `--partial-except-first`; the flag cannot turn it off). The few options with no flag (first column is X,
    parameter names from the first cell, the legacy skip and maximum rows) come from the theme, so the window and
    the command line read a file the same way.
  - *Read only.* The command line never writes the stores: it does not record `<last used>` (that is the
    window's OK) and does not save themes. Without `--theme` or `--last-used` the folder is not read.
  - *Where.* `--themes-dir DIR`, else the environment variable `SHEAF_THEMES_DIR`, else the folder of
    section 17.2 "Where". Tests give a folder of their own.
  - *Errors.* An unknown name stops before anything is written: `no import theme named 'X'; saved themes: A, B`
    (the themes of the three stores, each name once, text first). `<last used>` before any OK says so. A theme
    that this version cannot read (section "Compatibility") is an error that names it, not "unknown".
- **Tests never touch the real folder.** The app takes the folder from `Startup.themes_dir`; tests pass `None`
  or a temporary folder.

### 17.2a The row in the app

Built in `crates/app/src/dialogs/import_theme.rs` and used by the text window, the Excel window and the
Import Wizard. All of this is **[S]** (Origin's layout of the row was not measured beyond "above the
description", section 10).

- **Place.** The first row inside the window, above the file list: the label `Dialog Theme`, a combo box
  (240 px) and a small button `Manage` (the default font has no arrow glyph that draws) that opens the menu
  `Save`, `Save As...`, `Delete`. In the wizard the row is the top of the Source page. When the application has
  no theme folder (`Startup.themes_dir` is `None`; `--no-themes`) the row is not shown and nothing is read or
  written. `--themes-dir DIR` picks another folder (screenshots and checks without the user's folder).
- **Reading.** The stores are read when a window opens (the list of names and `<last used>`) and again after
  each Save, Save As, Delete or choice (another instance may have changed them), never while drawing a frame.
- **Choosing an entry** fills every option the theme holds and leaves the file list, the file shown in the
  preview, the target sheet and the graph template as they are. The Excel window also leaves the ticked
  sheets. Choosing `<last used>` before any OK gives the defaults.
- **Fresh window.** Opens with `<last used>` when the store has one (the combo says so), else `<default>`.
  File > Import > Comma Delimited (CSV) still starts with the comma delimiter: the preset is applied after the
  theme.
- **OK** records `<last used>` before the files are read, whatever the result of the import; a failure to
  write is logged as a warning and does not stop the import. **Cancel** and closing the window record nothing.
  The Import Wizard records on Finish.
- **Text window, two stores.** A name saved in the text window is written to the `ascii` store (everything but
  the CHI options) and to the `chi` store (the CHI options, with the same name); choosing it reads both (the
  default CHI options when the `chi` store has no such name). `<last used>` and Delete work on both. The list
  shows the names of the `ascii` store.
- **Save** on a saved theme writes over it without asking; on `<default>` or `<last used>` it opens the name
  prompt, like Save As. The prompt is a small window `Save Dialog Theme As` with a name box, `Save` and
  `Cancel`; `Save` is off for an empty name and for a name in angle brackets (the reason is shown), and a note
  says when a theme of that name will be replaced. Enter confirms. Each import window (text, Excel,
  wizard) has a Save As window with an id of its own, so that two of them can be open at once. The wizard draws
  it on the Source page only and closes it when the page changes.
- **Delete** is off for the two built-in entries. After it the combo shows `<default>` and the window is
  filled with the defaults, so that the list and the controls agree.
- **Errors.** A write that fails, a theme that cannot be read (an enum value of a newer version) or one that
  another instance deleted is reported in red under the row; the controls do not change.
- **Not done.** Rename, and an Import Filters Manager (Origin's `.oif`, section 17.1). Moving a theme between
  the text and the Excel stores is not offered.

Tests: `crates/app/src/app/ui_tests/import_themes.rs`.

### 17.3 To measure in Origin [M]

Measure on Windows with `tools/origin-ref` (screenshots of the theme drop-down and its menu into
`re-workspace/scratch/t46/`):

1. The entries of the theme drop-down in impASC: whether `<default>` and `<last used>` exist or some other
   built-in names, and the menu items (Save, Save As, Delete, Rename, Reset?).
2. Which theme a fresh window opens with.
3. Whether a theme stores the file name and the output (target book and sheet), or only the options.
4. Where Origin keeps the themes (per user, per project, a file in the user files folder) and how many a
   window can hold.
5. How the Import Wizard's Import Filter list relates to the Dialog Theme of impASC (the same files or not).
6. Whether OK records `<last used>` only when the import succeeds.
7. Whether the theme name box accepts a typed new name directly (Save As without a prompt).

## Import Filters Manager (T106, measured 2026-11-16, Origin 2016)

Tools > Import Filters Manager... opens Origin's **Filter Manager** (screenshot `re-workspace/scratch/t106`,
local only): a table of the import filters with the columns Filter, Folder (System), Supported (a tick),
File Extensions and Drag&Drop Exclusion (ASCII `*.txt;*.dat;*.asc`, CDF, CSV `*.CSV`, DiademDAT, DSC, DT,
EarthProbe, EDF, Excel `*.xls;*.xlsx;*.xlsm`, Famos, HDF5, HEKA, Igorpro, Image, ISF, JCAMP, JNB, ...) and
the buttons Customize Menu..., Edit, Apply and Cancel.

Sheaf's Filter Manager (`dialogs/filter_manager.rs`) lists the file types its importers read (Filter,
Folder System, Read by, File Extensions: ASCII, CSV, CH Instruments, Excel, Origin project) and, below,
the saved settings of each import window (the Dialog Themes of this file, by window) with Delete. Not
done: Supported and Drag&Drop Exclusion switches, Customize Menu..., Edit (Sheaf's filters are fixed).
