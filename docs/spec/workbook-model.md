# Workbook / worksheet / column data model (clean-room spec)

Source tags: **[F]** installed file, **[W]** public OriginLab documentation (docs.originlab.com, paraphrased), **[I]** inference, **[?]** unknown, check against running Origin.

## Summary

- A project holds folders; folders hold windows (workbooks, graphs, matrix books, layouts, notes). A workbook holds worksheets (sheets); a sheet holds columns; a column holds cells of one data type.
- Every window and every column has a **short name** (restricted, used in formulas) and an optional **long name** (free text, used as a display label).
- A column carries label rows above the data: long name, units, comments, parameters, user parameters, plus a formula row and sparkline row. These are text attributes of the column, not data cells.
- Each column has a **plot designation** (X, Y, Z, X error, Y error, Label, Disregard, Group, Subject) that decides how plots and analysis interpret it. A Y column finds its X by position.
- Numbers are stored as 64-bit doubles by default; "missing" is a reserved double value shown as "--".
- Calculated columns come from "Set Column Values": a formula applied to a row range, optionally re-evaluated automatically when inputs change.
- Priority P0 for the owner: workbook/sheet/column naming, long name + units rows, X/Y designations, missing values, Set Column Values with simple formulas (scale, offset, subtract baseline, current density = I / area).

---

## 1. Containment (P0)

| Level | Notes | Source |
|---|---|---|
| Project | One document. Contains a tree of folders (Project Explorer). Windows live in exactly one folder. Saved as .opj / .opju. | [W] |
| Folder | Nestable; one root. A folder holds windows only, name unique among its siblings (inference). | [W][I] |
| Window | Types: workbook, matrix book, graph, layout, notes. Each has a short name, a long name, and a comment. | [W] |
| Workbook | One or more sheets (worksheets and/or matrix sheets in a matrix book). Up to 4096 sheets. | [W] origin-limitation |
| Worksheet | A grid of columns x rows plus label rows. | [W] |
| Column | One-dimensional array of cells with a format, a designation and label rows. | [W] |

Cells in a sheet are addressed by column and row. Different columns may have different lengths; the "length" of a sheet is the longest column (inference).

## 2. Names and limits (P0)

| Item | Rule | Source |
|---|---|---|
| Book (window) short name | at most 13 characters, letters and digits only (underscore is tolerated in practice **[?]**), must be unique in the project; auto-generated names look like Book1, Book2. | [W] restrictions-on-object-name, origin-limitation |
| Book long name | free text, up to 30,000 characters (an older source says 5,506). Not required to be unique. | [W] |
| Worksheet (sheet) name | the quick-help page says 32 characters incl. spaces; the limits page says 64. Use 64 on read, 32 to be safe on write. Default names Sheet1, Sheet2. Sheet label, comment up to 30,000 characters. | [W] (conflict, **[?]**) |
| Column short name | at most 18 characters, letters/digits, must start with a letter or digit, no spaces or special chars; unique inside a sheet. | [W] |
| Column long name, units, comments, parameter cells | free text up to 30,000 characters each. Long name need not be unique. | [W] |
| Book short name + column short name | sum must be at most 23 chars (legacy rule from old "Book_Col" notation). | [W] |
| Rows per column | up to 1,000,000,000 (limited by memory). | [W] |
| Columns per sheet | up to 65,504. | [W] |
| User-defined parameter rows | up to 128. | [W] |
| Text cell | up to 799 characters (text format); Text and Numeric format up to 3799 chars. | [W] |
| ASCII import | up to 18,002 columns, 200,000,000 rows. | [W] |

Short names of columns: by default Origin auto-assigns them by "spreadsheet cell notation": A, B, ... Z, AA, AB, ... (like a spreadsheet). In that mode the names follow the column position and cannot be edited; turning the notation off (sheet property) allows arbitrary short names such as "Time" or "Potential". New columns added get the next unused name. If a requested name exists, a generic name is used. [W basic-worksheet-column-operation] Since Origin 2017 short names are alphabetic by default.

## 3. New worksheet default shape (P0)

Default workbook template ("Origin" prototype): one sheet named Sheet1, two columns: A with designation X, B with designation Y, a visible block of about 30 empty rows (empty columns have no data, length 0). Label rows shown by default: Long Name, Units, Comments, plus the F(x) formula row (inference from the GUI). **[I]**, shape not read from a file. The default new-column designation is Y. Number format default: numeric double, "Default Decimal Digits" display. Check template contents with running Origin **[?]**.

Template files for new windows live in the user files folder (e.g. `OriginLab/2016/User Files/*.otw`); the `Origin.ini` entries `[Data] Double=1` (double precision data, 0 = single precision) is the only data-model setting there. [F]

## 4. Column label rows (P0)

| Row | Label code (LabTalk) | Meaning |
|---|---|---|
| Long Name | L | display name; used as axis title on plots by default |
| Units | U | measurement unit text; combined with long name for axis titles "Long Name (Units)" |
| Comments | C | free text |
| Parameters (system) | P1, P2 ... | column parameters (e.g. analysis results stamped on the column) |
| User parameters | D1, D2 ... (user-named rows) | user-defined named rows; first can be renamed; max 128 |
| Sampling interval | (row) | shows the X increment of a column with no explicit X (P1) |
| F(x) formula row | F | holds the Set Column Values formula text |
| Short name | G? (set via name) | see section 2 |
| Sparklines | (row) | tiny line chart per column (P2) |

The row labels `L`, `U`, `C`, `P1`, `D1` come from the `[L]`/`[U]` bracket syntax like `col(1)[L]$` [W]. Which label rows are displayed is a sheet property (`wks.labels()` takes a string of codes such as `LUCP1D1`). [W]

## 5. Plot designations (P0)

| Designation | Short symbol (list) | LabTalk numeric type code | Meaning |
|---|---|---|---|
| X | X | 4 | independent values |
| Y | Y | 1 | dependent values plotted against X |
| Z | Z | 6 | third dimension (3D, colour map, contour); uses X on its left only |
| X Error | xEr± (M) | 7 | error bar size in X |
| Y Error | yEr± (E) | 3 | error bar size in Y; attaches to the Y column immediately to its left |
| Label | L | 5 | text/number labels for points of a Y column (legacy approach) |
| Disregard | N | 2 | never plotted, even if selected |
| Group | G | 8 | category for statistics |
| Subject | S | 9 | subject id for repeated measures |

[W] wkscol-setdesignation and worksheet column options. A value above 9 is invalid.

How a Y column finds its X [W]:
1. If the sheet has exactly one X column, that X is used for all Y columns on either side of it.
2. If there are several X columns, each X covers the Y and Z columns to its right, up to the next X.
3. In a non-adjacent selection, a selected X column is used; if none is selected, search to the left of the Y first, then to the right (Z only searches left).
4. If there is no X column, the X values are the row index or the column's sampling interval (start value and step), P1.
5. X error columns associate with the nearest X on the left, else the right.

The header shows the designation in parentheses after the short name, e.g. "A(X)", "B(Y)"; Disregard shows only the short name. Pattern shorthand for setting many columns: `XY` repeated, `3(XY)`; leftover columns become Y. [W] The default designation list in the import filters is "XY" (CSV) [F].

## 6. Column data formats (P0)

| Code (worksheet -f, LabTalk) | Code (Origin C / Python) | Format | Notes |
|---|---|---|---|
| 1 | 0 | Numeric | numbers only; text typed in becomes missing |
| 2 | 1 | Text | any text; not usable in calculations; 49 chars default, up to 799 |
| 3 | 2 | Time | stored as day fraction |
| 4 | 3 | Date | stored as day count |
| 5 | 4 | Month | 1-12 or names (abbreviations OK if unique) |
| 6 | 5 | Day of Week | three display options (full name, abbreviation, first letter) |
| 7 | 9 | Text & Numeric | default for every designation except Label; text counted as missing in calculations; up to 3799 chars |

[W] The two numbering schemes differ; the Rust model should use its own enum and map when reading files. The legacy import filters both write format code 9 = Text & Numeric. [F] Extra display-only kinds: Color (cell fill from index 1-24 or HTML code), Binary (only 1/0). [W]

Display options for Numeric and Text & Numeric:

| Option | Meaning | Default |
|---|---|---|
| Display mode: Decimal | normal notation; switches to scientific automatically for values below 1e-3 or above 1e6 (thresholds adjustable) | default |
| Scientific | exponent form such as 1E3 | |
| Engineering | metric suffix (1k, 1.0M), case sensitive | |
| Custom | user-defined formats saved in the project | |
| Digits mode | default digits from the global setting; or set decimal places (n digits after the point); or significant digits | default digits; if blank, 5 decimal places for double |
| digits value | LabTalk `digits`, `digitMode`; -3 = "free format" in the old worksheet command | |

Display never changes the stored value. A cell too narrow for the number is rounded for display before "###" is shown (since 2020b). [W]

Date/time: a Date/Time column stores a double: whole part = day count in a Julian-day-based system (valid range 0100-01-01 to 9999-12-31), fraction = time of day; the zero point is noon on 4713-BC Jan 1 with a 12-hour offset in the default system; newer systems exist (not in 2016). Precision about 0.0001 s. Display tokens: M MM MMM MMMM, d dd ddd dddd, D (weekday 1-7), y yy yyyy, q, h hh H HH, m mm, s ss, # to #### fractional seconds, t tt (AM/PM), DDD for day count (IRIG-style); literals in single quotes; custom strings up to 60 chars. [W] (P0 only if the owner imports time-stamped logs.)

## 7. Internal data types (numeric columns) (P1)

| Code (`-fi` / numerictype) | Name | Bytes | Range / notes |
|---|---|---|---|
| 1 | double | 8 | about +-1.7e308, 15 digits; the only type that supports missing values; DEFAULT |
| 2 | real | 4 | +-3.4e38, 7 digits |
| 3 | short | 2 | -32768..32767 |
| 4 | long | 4 | -2147483648..2147483647 |
| 5 | char | 1 | -128..127 |
| 6 | byte | 1 | 0..255 |
| 7 | ushort | 2 | 0..65535 |
| 8 | ulong | 4 | 0..4294967295 |
| 9 | complex | 16 | two doubles |

[W] colproperties-dialog and number-in-origin. Recommendation: implement double (and text) first; others read-only/convert on load.

## 8. Missing values (P0)

- Reserved double -1.23456789E-300 means "missing". Arithmetic with it gives missing. Shown as "--" in the sheet (system variable @JM=2 shows blank instead). [W]
- Sources of a missing value: function NA(); division by zero in an expression; text typed into a numeric column. Skipped or cleared cells are left blank (empty) rather than filled with the missing value in current versions (system variable @CDB). [W]
- In the formula language the literal `1/0` evaluates to missing; the shipped example `col(A)==1/0 ? mean(col(A)) : col(A)` replaces missing values with the column mean. [F] `mformula.txt`-neighbour file `formula.txt`.
- Export option writes missing as "--" by default (expASC `missing`=1). [F]
- Rust: represent as NaN-or-sentinel internally; recommend `Option<f64>` / NaN and map the sentinel on file load. **[I]**

## 9. Range and reference notation (P0/P1)

| Form | Meaning | Source |
|---|---|---|
| `[BookName]SheetName!CellRange` | general data range; BookName is short or long name; if several columns share a long name the leftmost is used | [W] wkscol-cellrange-names |
| `[Book1]Sheet1!Col(A)` | whole column A by short name | [W] |
| `[Book1]Sheet1!Col("Potential")` | by long name | [W] |
| `Col(A)`, `col(1)`, `wcol(n)` | in a formula: column by name or by index; `wcol(n)` refers to the n-th column of the sheet (also the current one in Set Values `wcol(j)`) | [W] |
| `col(A)[i]` | the value at row i (i = current row index, 1-based) | [W][F] |
| `col(A)[3:10]`, `col(A)[i-3:i+4]` | subrange of rows | [F] `formula.txt` (moving SD example) |
| `cell(r,c)` | one cell by row and column number | [F] mformula.txt (matrix version `cell(i,j)`) |
| `col(A)$` | the text version of a column (string) | [F] |
| `col(A)[L]$`, `col(A)[C]$`, `col(1)[U]$` | label-row text of a column | [W] |
| `x`, `y` in column formulas | shorthand for the column's X and own values (Fill-from-X examples) | [F] |
| `This`, `This_n` | current column; n-th cell (only with spreadsheet cell notation on) | [W] |
| `i` | current row index; `j` current column index; `pi` the constant | [W] |
| `Book1_A` style | legacy notation; avoid | [I] |

Ranges may be several columns `(1,3,4)` or `Col(2):Col(4)`. [W]

## 10. Set Column Values (P0)

Dialog behavior [W wks-setcolval-menuopt-dialog, from-set-vals-dialog; F formula.txt]:

| Control | Meaning | Default |
|---|---|---|
| Row (i) From / To | rows written. If a part of a column was selected when opened, the range is pre-filled; whole column = auto (all rows, including later appended ones). Editing the formula in the F(x) row resets the range to the full column. | auto (full column) |
| Col (j) From / To | only when several columns are set together | |
| Column Formula | one single-line expression producing the value of each row; may use functions, conditional operator, variables | |
| Before Formula Scripts | multi-line script run once before the expression; used to define variables (e.g. read a scalar into a double, compute statistics) that the expression then uses. Cell notation `This_n` is not allowed here. | empty |
| Recalculate | None (compute once, no link), Auto (re-run when input columns change), Manual (user triggers re-run). Auto/Manual show a lock icon on the column: green = up to date, yellow = update pending, red = broken. | dialog opens on Manual by default (FAQ), system variables can change it |
| Apply / OK | run, keep dialog open / run and close | |
| Keep Text | keep text in the target rather than converting to missing during calc | off |
| Formula Text | custom display text in the F(x) cell instead of the formula | |
| Variables v1-v4 | four numbers stored in the sheet, usable as `page.info.SYSTEM.FORMULA.V1` | |

Rules:
- Evaluation is row by row, with `i` going over the row range; the expression is vectorised: `col(A)*2` means for every row. A formula that returns a whole dataset (see "dataset" functions) fills the column in one go.
- If the formula references the column being set, Auto/Manual recalculation reverts to None. [W]
- Missing in, missing out. Text in a numeric operation counts as missing (Text & Numeric format). [W]
- Whole-column functions (`mean`, `total`, `max`, `stddev`, `min`, `count`, `median`) return a scalar that is then broadcast to every row.
- Before-script uses LabTalk (variables, `range` definitions, X-function calls); the Rust app can offer a simple variable-assignment list instead (P1).
- Recalculation behaviour, measured in Origin 2016 and as built in Sheaf, is in section 10a.
- Saved formulas can be stored/loaded by name; there is a shipped list of examples (`formula.txt`: power operator, conditional operator `?:`, string concatenation `+`, `%()` substitution, user function definitions, moving subranges, 3-sigma band masks). [F]

Example formulas relevant to the owner: `col(B)/0.0707` (current density with area), `col(B)-mean(col(B)[1:10])`, `col(A)*1000` (V to mV), `i` (row index), `(i-1)*0.01` (time from sampling).

## 10a. Recalculating column formulas (P0)

What Origin 2016 does, from its documentation and from black-box runs (a hidden Origin driven through `tools/origin-ref/`, LabTalk `csetvalue col:=col(B) formula:="col(A)*10" recalculate:=1`, where 0 is None, 1 is Auto and 2 is Manual; the probe scripts were throw-away and are not kept):

- **[M] Formula text.** The formula of a column reads back as `wks.col2.formula$`. The mode could not be read back through any property name we tried, so the mode itself is taken from the documentation (section 10).
- **[M] Auto follows its source.** After the whole source column was assigned (`col(A) = {5,6,7,8}`) the Auto column had the new values, but only after a short pause: Origin recalculates on its idle loop, not inside the assignment. The test runs showed the old value right after the assignment and the new one about a second later.
- **[?] Sources that shrink (Sheaf's choice, not probed in Origin).** An Auto or Manual formula without a row range covers the whole column, so when its sources get shorter (a shorter re-import) the result is cut to the same length, and when every source is empty the result is emptied. A formula with an explicit row range leaves the rows outside it alone.
- **[M] Chain A to B to C.** With B = col(A)*10 (Auto) and C = col(B)+1 (Auto), changing A updated B and then C (B was 60 and C was 61 after A[2] became 6). Dependency order, not column order, decides.
- **[M] Manual does not follow.** A Manual column kept its old values after the source changed, and a None column also kept them. Writing the formula again with `csetvalue` is what updates it from a script; we found no script command that only recalculates, and no script access to the pending (yellow) state. The pending mark and the Recalculate command come from the documentation (green: up to date, yellow: update pending, red: broken).
- **[M] Hand edits.** In a column set to Auto or Manual, assigning a value to a cell from a script was ignored (the cell kept the formula result); in a None column the assignment was kept. This matches the lock icon: a column with a recalculating formula is protected. In Sheaf the grid does not open an editor on such a column and the action says why.
- **[M] Cell assignment from a script did not trigger Auto** in our runs (`col(A)[2] = 20`), while a whole-column assignment did. Edits in the worksheet window are the documented case for Auto; we could not drive those from a script, so Sheaf recalculates after every kind of change.
- **[M] Renaming a source column** left the formula text of the dependent column unchanged (`col(A)*10` stayed). Whether it then keeps working could not be settled from the script, so Sheaf's rule below is our choice.
- **[W] A formula that reads its own column** reverts to None (section 10).
- **[?] Cycles (B reads C, C reads B) and deleting a source column** could not be reproduced by script in the time spent. Sheaf's rules below are our choice.

Sheaf's rules (the `RecalcMode` and `FormulaState` of a column; `Project::set_column_values_with`, `refresh_formulas`, `recalculate`, `recalculate_pending`, `recalculate_since`):

1. A formula column stores its formula, its mode (None, Auto, Manual) and its row range (a column set on "all rows" has none and follows the length of its inputs). Projects without these keys load as mode None. The CH Instruments importer's `col*1000` formulas stay None: they are one-time values.
2. Auto and Manual formulas read columns of the same sheet. The reads are found from the parsed formula (`Expr::columns`) and matched by short name, long name or position, exactly as evaluation matches them.
3. Every change made through a `Document` edit (cell edit, import, analysis result, delete or rename of a column, set column values) is followed, inside the same undo step, by a pass over the sheets that hold Auto or Manual columns. The changed columns are found by comparing the project before and after (data, short name, long name). Undo and redo restore whole snapshots, so source and dependents come back together.
4. The pass runs the formula columns in dependency order. An Auto column runs when one of its inputs changed or was itself just recalculated; a Manual column is marked Pending instead and keeps its values. The Recalculate command runs one column now, then the columns that read it.
5. Only the rows of the stored range are written (default: as many rows as the longest input). Rows outside it keep their values.
6. A formula that cannot run is marked Broken and keeps its old values: an input column was deleted or renamed, the formula text does not parse, the row limit is exceeded, or text sits outside the rows. It runs again on the next pass once the problem is gone (renaming the column back repairs it).
7. Setting a formula that would close a loop through other recalculating columns is refused with a message naming the chain (`B -> C -> B`) and changes nothing. A formula that reads its own column runs once and the column becomes mode None, as in Origin. A loop that arrives some other way (a project file edited by hand) marks the columns in it Broken.
8. Cells of an Auto or Manual column are locked against hand edits in the app; the model itself does not forbid writing the data (analysis may), and such a write is overwritten at the next recalculation of that column.
9. The worksheet header shows a small word for these columns: `auto` or `manual` (green) when current, `pending` (yellow), `broken` (red). Origin draws a lock icon in those colours; the words are a stand-in until the Origin-style grid exists. The Set Column Values window has a Recalculate choice (None, Auto, Manual; it opens on Manual for a column without a recalculating formula, as Origin does) and the column menu has Recalculate.
10. Speed: a cell edit with two chained Auto columns of 1,000,000 rows each took about 0.13 seconds in a debug build (including the undo bookkeeping), measured in `crates/core/tests/recalc.rs`.

Not done: formulas that read other sheets or workbooks, recalculation of analysis and fit results from their source, `Before Formula` scripts, and a command-line option for the mode (the CLI has no Set Column Values command).

## 11. Built-in functions usable in column formulas

Signature shorthand: `x` number or column, `v` vector (column), `s` string. Meanings paraphrased; names confirmed from [W] lt-supported-oc-functions, [F] formula.txt / condition.txt examples, otherwise the standard LabTalk function set from Origin documentation (not re-verified, mark **[I]**).

**Math (P0).** `abs(x)` absolute value; `sqrt(x)`; `exp(x)`; `ln(x)` natural log; `log(x)` base-10 log; `log10(x)` same; `pow(x,y)` and operator `^`; `int(x)` truncate to integer; `round(x,n)` round to n decimals; `ceil(x)`, `floor(x)`; `mod(x,y)` remainder (operator `%` in LabTalk is modulus too **[?]**); `sign(x)`; `max(a,b)`/`min(a,b)` of two numbers; `sum`, `fact(n)` factorial; `gamma(x)`; `erf(x)`; `asinh acosh atanh`; `degrees(a)`, `radians(a)`; `distance(x1,y1,x2,y2)`; `na()` missing value; `isna(x)` test for missing; constant `pi`.

**Trigonometric (P0).** `sin cos tan asin acos atan atan2(y,x) sinh cosh tanh` (radians; use `radians()` for degrees). **[I]**

**Statistics over a column (P0).** `mean(v)`, `total(v)` (sum), `min(v)`, `max(v)`, `stddev(v)` sample, `stddevp(v)` population, `sd(v)` **[I]**, `median(v[,interp])`, `count(v[,mode])` number of elements (modes: size / numeric / missing), `var(v)` **[I]**, `sem(v)` **[I]**, `rank`, `sum` with ranges such as `mean(col(A)[1:10])`. Also running/cumulative forms: `cumsum(v)`, `diff(v)` **[I]**, `smooth` and others are X-functions, not formula functions.

**Conditional and logical (P0).** `cond ? a : b` ternary; comparison `== != < > <= >=`; boolean `&& || !` and the words `and`, `or`, `not`; `if(c,a,b)` (older form **[?]**); `lookup(v, key)` style table functions P2. Division by zero `1/0` yields missing and is commonly used to produce a missing value. [F]

**String (P1).** `left(s,n) right(s,n) mid(s,start,len) len(s) upper(s) lower(s) trim(s,mode) find(s1,s2[,start])` (case-sensitive, 1-based position) `search(s1,s2[,start])` (case-insensitive) `compare(s1,s2[,case])` `exact(s1,s2)` `replace(s,pos,len,new)` `substitute(s,old,new[,n])` `char(code) code(s) format(x,fmt)`; concatenation `+`; `%()` text substitution evaluates a string; `col(A)$` gives a column as text. [W]

**Date and time (P1).** `now()` current date-time; `today()` current date; `year(d) month(d) day(d[,mode]) hour(d) minute(d) second(d) quarter(d)`; `weekday(d[,mode])` day of week number; `weeknum(d,opt)` week 1-53; `monthname(d,n)`; `yearname(d,n)`; date arithmetic is plain addition (1 = one day); `date(y,m,d)` and `time(h,m,s)` constructors **[I]**. Examples exist in `dateFilterCondition.txt` (this week, last month, quarter, between 9 and 18 hours). [F]

**Random numbers (P1).** `rnd()` uniform in [0,1); `grnd()` normal with mean 0 and sd 1; `rnd(seed)` seeding **[I]**. The Fill-column options remember mean/sigma for normal (default 0/1) and min/max for uniform (default 0/1) in Origin.ini `[FillColumn]`. [F]

**Row/index helpers (P0).** `i` row index; `x` the X value of the column; `col(A)[i-1]` neighbour (undefined row gives missing) **[I]**; `wcol(j)`; `ndata`/`dmin dmax` helpers for matrices. [F]

**Not in a formula but common column operations (X-functions, names only, from `X-Functions/Data Manipulation/Worksheet` [F]):** colcopy, colhide, colint, colmask, colmove, colreverse, colshowx, colsplit, colswap, filltext, wautofill, wcellmask, wclear, wcolwidth, wcopy, wdeldup, wdelrows, wextract, wjoinbycol, wmergexy, wproperties, wreducerows, wrcopy, wreplace, wsort, wsplit, wstackcol, wunstackcol, wtranspose, wxt, wpivot, merge_book, addsheet, newbook, newsheet; also `reduce_ex`, `xy_resample`, `vfind`, `vshift`, `copydata`, `rank`. Utilities: get_wks_sel, get_plot_sel.

Function list caveat: signatures for items marked [I] were not confirmed against the installed help; implement the confirmed ones first.

## 12. Data masking (P1)

Masking marks cells as excluded from plots and analysis without deleting them. Mask is a per-cell flag (a colour-gray display). Mask by selecting cells and "Mask", or by X-functions `colmask` / `wcellmask` over a range; formulas in "masked" examples select rows by condition. Masked cells remain in the column, are skipped by fits and statistics, and are still visible in the sheet. [W][F names]. Details (stored as a separate bit array; effect on exported data) **[?]**.

## 13. Sorting and filtering (P1)

- Sort: single column, several columns, a selected block, or the whole sheet; ascending or descending; the row relationship across columns is preserved. Nested (multi-key) sort: ordered list of columns, each with its own order. X-function `wsort` (`descending`, `bycol`, `nestcols`, `order` options). [W]
- Filter: a column-based data filter hides rows that fail a condition; types: numeric, text, date/time. Conditions are stored as small formula lists in `numFilterCondition.txt` (above average, below average, within 3-sigma band, within 95 percent interval) and `dateFilterCondition.txt` (this week, last month, quarter, specific month, weekday, hour range). Filters are re-run on demand (`wks.runfilter`). Hidden rows are still in the data; plots ignore them. [F][W]
- Sheet/column properties that matter for round-tripping: column width, hidden columns, frozen header, `Spreadsheet cell notation` flag (auto short names), `Data protection` on import (read-only, sort-only). [W]

## 14. Suggested Rust data model (P0)

```
Project { folders: Tree<Folder> }
Folder  { name, windows: Vec<Window> }
Workbook { short_name (<=13), long_name, sheets: Vec<Sheet> }
Sheet   { name, comment, columns: Vec<Column>, label_rows_visible, cell_notation: bool }
Column  { short_name (<=18), long_name, units, comment, params: Vec<String>,
          user_params: Vec<(String,String)>, designation: Desig,
          format: ColFormat, display: DisplayOpts, data: ColData,
          formula: Option<ColFormula>, sampling: Option<(f64,f64)>, mask: bitset }
ColData = F64(Vec<f64>) | Text(Vec<String>) | Mixed(Vec<Cell>)   // Cell = Num | Text | Empty
Desig   = X | Y | Z | XErr | YErr | Label | Disregard | Group | Subject
ColFormula { before_script, expr, row_from, row_to, recalc: None|Auto|Manual }
```
Empty cell and "missing" (NaN) should be distinct states, both displayed as blank / "--" respectively.

## Unknowns that need a check against running Origin

1. Exact default new-sheet shape (row count shown, label rows shown, number of columns).
2. Whether book short names allow underscores; behavior when a derived name collides.
3. Sheet name limit (32 or 64).
4. Precise semantics of Recalculate Auto on dependent-column cycles and when columns are appended later; whether the row range "auto" includes appended rows.
5. Which formula functions exist under each name (items tagged [I]), the exact meaning of `%` and of `sd` vs `stddev`, and behavior of missing values inside `mean`, `total`, `stddev` (expected: skipped).
6. Format code numbering between LabTalk `wks.col.format` (1..) and Origin C (0..) in this version.
7. Mask storage and its propagation to plots and exports.
8. Sparklines and sampling interval details (P2).
