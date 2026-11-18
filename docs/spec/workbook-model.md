# Workbook / worksheet / column data model

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
4. If there is no X column, the X values are the row index or the column's sampling interval (start value and step; section 13d). A column with an interval plots at it even when an X column stands to its left.
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

### Observed in Origin (black-box runs, 2026-11-12) [F]

Runs went through `tools/origin-ref` (`run-labtalk-lines.ps1`) on inputs we made ourselves.

- **Date numbers.** `date(2026,11,11)` is 2461355, `date(1970,1,1)` is 2440587, `date(1900,1,1)` is 2415020 and `date(2000,1,1)` is 2451544. So a date at midnight is a whole number, equal to the number of days since 1970-01-01 plus 2440587 (the astronomical day number minus one). A date with a time is that whole number plus the day fraction: 2026-11-11 13:45:10 is 2461355.5730324076. A cell holding 2461000.5 shows as 11/21/2025 (the whole part picks the day; .5 is noon of that day). Sheaf stores exactly this number, so that a later `.opj` writer produces what Origin reads.
- **Time of day.** `time(13,45,10)` is 0.57303240740740735, the fraction of a day. A Time column holds only the fraction. Fractional seconds are kept (13:45:10.250 imports as 0.5730353009).
- **Format codes in LabTalk.** `wks.col1.format` is 3 for Time, 4 for Date and 7 for Text & Numeric (the default of an imported text column). `wks.col1.subformat` picks the display pattern.
- **Date subformats** (value 2026-11-11 13:45:10 unless noted): 1 `11/11/2026` (month, day, year without leading zeros: 2026-02-05 shows `2/5/2026`), 2 `Wednesday, November 11, 2026`, 3 `Nov 11`, 4 `11/11`, 5 `11`, 6 `Wed`, 7 `W`, 8 `2026`, 9 `26`, 10 `11/11/2026 13:45`, 11 `11/11/2026 13:45:10`, 12 `261111`, 13 `261111 13:45`, 14 `261111 13:45:10`, 15 `261111 1345`, 16 `261111 134510`, 17 `Nov`, 18 `N`, 19 `Q4`, 20 `11-11-2026` (`2-5-2026` for February 5). Time of day is 24-hour in the date-time patterns.
- **Time subformats** (value 13:45:10 as a fraction of a day): 1 `13:45`, 2 `13`, 3 `13:45:10`, 4 `13:45:10.00`, 5 `01 PM`, 6 `01:45 PM`, 7 `45:10`, 8 `45:10.00`, 9 `1345`, 10 `134510`, 11 `13:45:10.000`, 12 `000` (**the elapsed days, not milliseconds**), 13 `000:13`, 14 `000:13:45`, 15 `000:13:45:10`, 16 `000:13:45:10.00`, 17 `000:13:45:10.000000`; 18 is the custom display. The Display list of the dialog names them `HH:mm`, `HH`, `HH:mm:ss`, `HH:mm:ss.##`, `hh pm`, `hh:mm pm`, `mm:ss`, `mm:ss.##`, `HHmm`, `HHmmss`, `HH:mm:ss.###`, `DDD`, `DDD:HH`, `DDD:HH:mm`, `DDD:HH:mm:ss`, `DDD:HH:mm:ss.##`, `DDD:HH:mm:ss.######`, Custom Display (measured 2026-11-13).
- **Display rounds to the last digit shown** (measured 2026-11-13 on Time and Date columns; the earlier note, that Origin truncates, was wrong): a time is rounded to the step of the pattern (whole seconds for every pattern without decimals, also `HH:mm` and `mm:ss`; hundredths, thousandths, microseconds for `.##`, `.###`, `.######`) and then cut into fields. 0.0004 days (34.56 s) is `00:00:35` as `HH:mm:ss`, `00:00:00` as `HH:mm` and `00:00:34.56` with hundredths; 0.00068981 days (59.6 s) is `00:01` as `HH:mm`; 0.000001 days (0.0864 s) is `00:00:00.09`; 0.99999999 days is `24:00:00`, `24:00`, `23:59:59.999` (milliseconds) and `000:24:00:00` (the hours do not wrap and the day count does not advance). In a Date column with a time the date comes from the whole part alone: 2461355.9999999 shows `11/11/2026 24:00:00`, and 2461355.5730440 shows `11/11/2026 13:45:11`. A missing cell shows `--`.
- **Time columns, hours, sign and elapsed days** (measured 2026-11-13, `wks.col1.format` 3). While the number is below 240 days (239.9999 gives `5759:59:51`; 240 gives `00:00:00`) the patterns with `HH` count **all** the hours: 1.0625 days is `25:30` as `HH:mm`, 2.5 is `60:00:00`, 100.5 is `2412:00:00`, `2530` as `HHmm`; from 240 days on, only the time of day is shown (1000.5 and 2461355.5 are `12:00:00`). The 12-hour patterns and the minute patterns wrap (1.0625 is `01 AM` and, as `mm:ss`, `30:00`). The `DDD` patterns count the whole days at any size (2461355.5 is `2461355:12` as `DDD:HH`; 1.0625 is `001:01`); the hours there reach 24 but the days do not carry. **A negative number is written like its magnitude behind a minus sign**: -0.0625 is `-01:30`, -1.0625 is `-25:30`, -1.0625 as `DDD:HH` is `-001:01`, -2.5 is `-60:00:00`, -0.5 as `hh pm` is `-12 PM`, -1000.5 is `-12:00:00`; the sign stays when the rest rounds to zero (-1E-9 is `-00:00:00`) and is missing only for 0.
- **Odd corner**: `date(1,1,1)` returned 2451910 (year 1 taken as 2001), so very old dates through that function are not a reliable reference.

### What Sheaf implements (crates/core/src/datetime.rs, format.rs)

- Cells stay `f64`. `NumberFormat` gained `Date(pattern)`, `Time(pattern)` and `DateTime(date pattern, time pattern)`; project files written before this change still load because the old variants are unchanged.
- Date patterns: `M/d/yyyy`, `M-d-yyyy`, `dd/MM/yyyy`, `dd-MM-yyyy`, `dd.MM.yyyy`, `yyyy-MM-dd`, `dd-MMM-yyyy`. Time patterns: `HH:mm`, `HH:mm:ss`, `HH:mm:ss.fff`. Added 2026-11-13 (column-formats): the other measured date subformats as patterns (`dddd, MMMM d, yyyy`, `MMM d`, `M/d`, `d`, `ddd`, weekday letter, `yyyy`, `yy`, `yyMMdd`, `MMM`, month letter, quarter `Q4`; 19 in all, Origin's list has 20 and its date-time subformats 10-16 are covered by Date and time) and the 17 measured time subformats as `TimePattern`s in the order and with the labels of Origin's list (`HH:mm` ... `DDD:HH:mm:ss.######`; the pattern that an earlier build called `fff` and showed as milliseconds is Origin's `DDD`, project files that hold it as `millis` still load). `format_time` (`crates/core/src/datetime.rs`) rounds to the step of the pattern, counts the hours below 240 days and writes the minus sign as measured above; each rule has a test with the measured text.
- Display types in Column Properties: Numeric, Text, Text & Numeric, Time, Date, Month, Day of Week (and Sheaf's Date and time). `NumberFormat::Month(style)` and `Weekday(style)` show a number as a name. **Measured in Origin (2026-11-13, `wks.col1.format` 5 and 6, `wks.col1.subformat` 1 to 3, text read with `cell(r,c)$`):**
  - Month: the whole part of the number counts (cut toward zero: 1.5 and 6.9 are January and June), 1 is January and 12 is December, and the cycle repeats (13 is January, 100 and 1E6 and 1E9 are April, 2147483647 is July). 0, 0.5, every negative number, and anything from 2147483648 up show an empty cell.
  - Day of Week: the whole part counts, **0 and 7 are Sunday, 1 is Monday, 2 Tuesday, up to 6 Saturday**, repeating every 7 (8 is Monday, 14 Sunday, 100 Tuesday, 1E6 Monday). This is not the numbering of the calendar weekday (the date 2461355 would be a Monday here, though 2026-11-11 is a Wednesday): the column only reads the number. -0.5 and -0.9999 cut to 0 and show Sunday; -1 and below, and 2147483648 and up, show an empty cell. (The earlier Sheaf rule, 1 = Sunday, was wrong.)
  - Name styles: `subformat` 1 = abbreviation (`Jan`, `Mon`), 2 = full name (`January`, `Monday`), 3 = first letter (`J`, `M`, so Tuesday and Thursday both give `T`). The Display list of the Column Properties dialog is the three samples of the style, not words: for Month `Jan | January | J`, for Day of Week `Mon | Monday | M`; the first is the default.
  - The Format list of the dialog holds, in this order, Numeric, Text, Time, Date, Month, Day of Week, Text & Numeric; the Plot Designation list X, Y, Z, X Error, Y Error, Label, Disregard, Group, Subject.
  Sheaf shows the same samples in the Display list (`NameStyle::sample`).
- Numeric display in Column Properties (measured in Origin 2026-11-13; `wks.col1.subformat`, `wks.col1.digitMode`, `wks.col1.digits`, text read with `cell(r,c)$`, and the dialog's combo boxes read item by item). The **Display** list is `Decimal:1000` (plain, `subformat` 1), `Scientific:1E3` (2), `Engineering:1K` (3), `Decimal:1,000` (thousands separators, 4) and Custom; the **Digits** list is `Default Decimal Digits`, `Set Decimal Places=` and `Significant Digits=` (`digitMode` 0, 1, 2). So the name `Decimal:1000` is the plain display and the comma form is `Decimal:1,000` (Sheaf had them the other way round). The digits apply to every display:
  - **Engineering** has no exponent form with a capital E. The mantissa runs from 1 up to 1000 and the exponent is a multiple of 3, written as an SI letter: `a` 1E-18, `f`, `p`, `n`, `µ` (micro sign, U+00B5), `m`, none, `k`, `M`, `G`, `T`, `P`, `E` 1E18. There is no `y`, `z`, `Z` or `Y`. Outside that range the exponent is written in **lower case, without plus, in steps of 3**: 1.5E-19 shows `150.0e-21` (one digit), 1.5E-24 `1.5e-24`, 1.5E21 `1.5e21`, 1.5E24 `1.5e24`, 1.5E25 `15.0e24`, 1.5E30 `1.5e30`. Negative numbers: `-15.0k`.
  - **Set Decimal Places=** n: n digits after the point of the mantissa, zeros kept (15000 with 1 is `15.0k`, 0.0025 with 2 is `2.50m`, 47 with 1 is `47.0`, 0 with 1 is `0.0`; with 0 digits 0.0025 is `3m`). Ties round up (2.5 gives 3, 12.5 gives 13), unlike C's printf.
  - **Default Decimal Digits**: 5 digits after the point, trailing zeros dropped: 15000 is `15k`, 0.0025 `2.5m`, 1234567 `1.23457M`, 12345.678 `12.34568k`, 0.1 `100m`, 0 `0`.
  - **Significant Digits=** n counts the digits of the mantissa: 15000 with 3 is `15.0k`, 1234567 `1.23M`, 12345.678 with 2 is `12k`. When the integer part of the mantissa has more digits than n, the text becomes `1.2E+02` (capital E, sign, two exponent digits; 123456789012 with 2 is `1.2E+02G`, 15 with 1 is `2E+01`, 999.9 with 2 is `1.0E+03`), except that a power of ten with exactly one digit too many stays plain (100 with 2 digits is `100`, 10 with 1 is `10`; 110 and 101 with 2 digits give `1.1E+02` and `1.0E+02`); 999.9 with 3 digits is `1000`.
  - The letter is chosen from the value rounded to about six significant digits and the mantissa is not corrected afterwards: 999.96 with 1 decimal is `1000.0` (not `1.0k`), 999999 with 1 decimal is `1000.0k`, 0.99999 is `1000.0m`, but 999999.5 and 999999.96 are `1.0M` (7 digits or more round up to the next letter). Two edge cases of this are not matched exactly: 9.99999E-19 shows `1.0a` and an Engineering value just below 1E6 with significant digits shows `1.000M` (one digit more than the others).
  - The "Significant Digits=" mode of a plain Decimal:1000 column shows the same text without the letters (15 with 3 is `15.0`, 1100 with 3 is `1.10E+03`, 1000 with 3 is `1000`); Sheaf's `Significant` display (which drops trailing zeros and writes `1.23E3`) was not changed to this yet.
  Sheaf: `NumberFormat::EngineeringSi(d)` (and the older `Engineering(d)`, which now means the same), `EngineeringSig(n)` and `EngineeringAuto`; the Display list of Sheaf's dialog folds Origin's two lists into one. The scaling power of ten is parsed from its decimal representation, rather than computed with `powi`, so a value such as `1e-24` normalizes to exactly `1` and keeps the requested significant digits (`1e-24` with one digit, `1.0e-24` with two).
- Switching Format between Numeric, Text and Text & Numeric converts the cells (`Column::convert_storage`): numbers become the text they show, text is read as numbers where it can be.
- Column short name (measured in Origin 2026-11-13, Column Properties box and `wks.col1.name$` alike): at most 18 characters, a longer name is **cut to 18 without a message** (`abcdefghijklmnopqrstuvwxyz` becomes `abcdefghijklmnopqr`); letters, digits and dots stay and every other character (space, underscore, hyphen, slash, brackets, `%`, `#`, `$`, `@`, `*`, `,`, `:`) is dropped (`a b_c-d.e` becomes `abcd.e`); a first digit is allowed; a name that another column of the sheet has (any case) is refused and the column keeps its name. Sheaf: `column::clean_short_name`, used by the Column Properties dialog (it shows the cleaned name, and still says so when the name is empty or taken). The importer's rules for header cells are in import-ascii.md section 3. Book short names (measured in Origin 2016 on 2026-11-15 with `win -r`, `re-workspace/scratch/bookname`): only ASCII
letters and digits stay (space, `_`, `-`, `.`, `/`, `#`, brackets are dropped, so `a.b` becomes `ab`, unlike a column
name), a name that is then empty or starts with a digit gets an `A` in front (`1abc` becomes `A1abc`, `___` becomes
`A`), and the result is cut to 13 characters without a message (`12345678901234` becomes `A123456789012`). A name
another window has brings up a message (not measured further). Sheaf: `project::clean_window_name` and
`Project::rename_window` (refuses a name in use, any case, and changes `[Old]` in column formulas to the new name),
used by the Short Name field of the window's Properties (rename) window; graphs follow the same rule (unmeasured).
- `.opj` writer: display formats are not stored (it counts them in `formats_dropped`); the new formats are kept only in the `.opj.sheaf` companion / project file.
- The calendar is the proleptic Gregorian one; no time zone, no leap seconds. Display rounds to the millisecond and then truncates to the unit shown, like Origin.
- Typing a date or time into a date column sets the number; the formula engine sees plain numbers, so `col(A) - col(A)[1]` gives days (times 1440 for minutes). Origin's `date()` and `time()` functions are not in the expression language yet.
- A date column as X gives date tick labels (see `docs/spec/graphing.md` for ticks): steps are round times (1 to 30 s, 1 to 30 min, 1 to 12 h, 1 to 7 days, then 1-2-5 days); a step of a day or more shows the date, a shorter one the time of day, with the date added when the axis crosses midnight. Calendar months and years are not tick steps yet.

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
- Recalculation behaviour, measured in Origin and as built in Sheaf, is in section 10a.
- Saved formulas can be stored/loaded by name; there is a shipped list of examples (`formula.txt`: power operator, conditional operator `?:`, string concatenation `+`, `%()` substitution, user function definitions, moving subranges, 3-sigma band masks). [F]

Example formulas relevant to the owner: `col(B)/0.0707` (current density with area), `col(B)-mean(col(B)[1:10])`, `col(A)*1000` (V to mV), `i` (row index), `(i-1)*0.01` (time from sampling).

## 10a. Recalculating column formulas (P0)

What Origin does, from its documentation and from black-box runs (a hidden Origin driven through `tools/origin-ref/`, LabTalk `csetvalue col:=col(B) formula:="col(A)*10" recalculate:=1`, where 0 is None, 1 is Auto and 2 is Manual; the probe scripts were throw-away and are not kept):

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
2. Auto and Manual formulas read local columns or explicit cross-sheet/book references such as `[Book1]Sheet2!col(A)` and `2!wcol(1)` (expression-language.md, Cross-sheet and cross-book columns). Reads are matched by short name, quoted long name or column position within the selected sheet. Workbook short names (preferred), unique display titles and sheet names ignore ASCII case; sheet positions are 1-based. Ambiguous book titles are refused. This qualified formula syntax is Sheaf's extension of Origin's documented range notation; installed Origin 2016 requires range variables.
3. Every change made through a `Document` edit (cell edit, import, analysis result, delete or rename of a column, set column values) is followed, inside the same undo step, by a project-wide pass over Auto and Manual columns. Data, names, label metadata, display formats and parameter edits invalidate readers. Structural edits re-resolve references and invalidate changed bindings; unrelated new books do not mark Manual formulas pending. Undo and redo restore whole snapshots, so remote sources and their dependents come back together.
4. The pass runs formula columns in dependency order across books and sheets. An Auto column runs when one of its inputs changed or was itself just recalculated; a Manual column is marked Pending instead and keeps its values. The Recalculate command runs one column now, then local and remote columns that read it.
5. Only the rows of the stored range are written (default: as many rows as the longest input). Rows outside it keep their values.
6. A formula that cannot run is marked Broken and keeps its old values: an input column was deleted or renamed, the formula text does not parse, the row limit is exceeded, or text sits outside the rows. It runs again on the next pass once the problem is gone (renaming the column back repairs it).
7. Setting a formula that would close a loop through other recalculating columns is refused with a message naming the chain (`B -> C -> B`) and changes nothing. A formula that reads its own column runs once and the column becomes mode None, as in Origin. A loop that arrives some other way (a project file edited by hand) marks the columns in it Broken.
8. Cells of an Auto or Manual column are locked against hand edits in the app; the model itself does not forbid writing the data (analysis may), and such a write is overwritten at the next recalculation of that column.
9. The worksheet header shows a small word for these columns: `auto` or `manual` (green) when current, `pending` (yellow), `broken` (red). Origin draws a lock icon in those colours; the words are a stand-in until the Origin-style grid exists. The Set Column Values window has a Recalculate choice (None, Auto, Manual; it opens on Manual for a column without a recalculating formula, as Origin does) and the column menu has Recalculate.
10. Speed: a cell edit with two chained Auto columns of 1,000,000 rows each took about 0.13 seconds in a debug build (including the undo bookkeeping), measured in `crates/core/tests/recalc.rs`.

Cross-sheet links and cycle checks, positional rebinding, removed sources, Undo/Redo and native/companion persistence have dedicated `cross_sheet_formulas` regressions. `.opj` recalculation records remain limited to same-sheet inputs; cross-sheet Auto/Manual mode survives in the companion, with the existing omission warning.

Not done: recalculation of analysis and fit results from their source, `Before Formula` scripts, and a command-line option for the mode (the CLI has no Set Column Values command).

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

### 13.1 Measured with Origin (`wsort`, task T10)

Run through `tools/origin-ref/run-labtalk-lines.ps1` on a book whose first column holds 3, --, 1, 2, 1, 2, 1 (rows 1 to 7) next to a column of row labels, and on a text column with a, b, B, C and a gap.
`wsort bycol:=1 c1:=<column> descending:=0|1` sorts every column of the sheet by that column.

- A missing value (and the empty text) counts as the **smallest** value: first when ascending, last when descending.
- Text compares without regard to case ("b" and "B" are equal keys).
- The order of equal keys is **not stable** in Origin (ascending: keys 1 at rows 3, 5, 7 came out as rows 7, 3, 5; keys 2 kept their order); it cannot be copied, so Sheaf keeps equal keys in their order.
- `Project::sort_sheet`, `sort_range`, `sort_columns` and the nested sort (`Project::sort_nested`) follow this.

Selected-range behavior follows the published [Sorting Worksheet Data](https://docs.originlab.com/origin-help/wks-sort-data/) reference. Sort Range and Sort Columns > Ascending/Descending reorder the selected data together by its leftmost column. **Each Column Ascending/Descending** sorts each column independently. A rectangular cell selection or row-header selection limits the sorted rows; rows outside it and unselected columns keep their values. Selecting whole columns retains the full-column behavior. Label cells do not enable sorting of the data beneath them.

Sort Columns > Custom and the column-header context menu now open the existing nested-sort dialog, as do Sort Range and Sort Worksheet > Custom. Range/column Custom captures the selected row interval when opened; whole-sheet Custom still sorts the whole sheet. Core methods `sort_range_in_rows`, `sort_columns_in_rows`, and `sort_nested_in_rows` clamp intervals to the worksheet length and preserve masks with their cells; the original full-range API methods remain. Filters are reapplied after the final row order, and edits undo/redo as one step. Regression tests cover simple, independent and nested sorting, masks, untouched outside data, the new menu wiring, row-header selections, and preventing sorts from a label selection. This selection work is based on the public reference, not a new installed-Origin probe; the measured comparison/tie rules above remain unchanged. Custom categorical ordering (including first-appearance order) is not modeled.

### 13.1a Mask, measured with Origin (task T10)

`wcellmask irng:=<range>` masks the cells of a range. `stats` on a column with one of five cells masked reports n = 4 (the masked value is left out). In a graph (line + symbol, five points, the third masked, exported to PNG) the masked point stays plotted: its symbol is drawn **red**, and so are the two line segments that meet at it. Sheaf: `Column::mask` (a set of rows), Column > Mask > Apply / Remove / Swap on the selected cells; analysis, statistics and fits read `Column::numeric_unmasked`; the mask moves with its rows in sorts, inserts and deletes; the plot draws masked points red (`draw_masked`). Masking from the graph: Data > Mask Data Points (graphing.md 20, T48). The `.opj` reader and writer keep masks since 2026-11-15 (opj-format 3.2); the mask colour, Disable Masking and Hide Masked Points are not in the file (Origin keeps them as settings).

**Change Color, Disable Masking, Mask Cells by Condition (measured 2026-11-15, Origin 2016, `re-workspace/scratch/t10m`):**
- Column > Mask holds Apply, Remove, Swap, Change Color and Disable Masking; Mask Cells by Condition... is its own item
  after the Mask submenu. Change Color has no dialog: each use moves the mask colour, a global setting (`@MC`, 1 at
  first), to the next entry of the colour list and wraps after index 23 back to 1, so black (0) never comes up
  (Origin's own menu script). Exported graphs with `@MC` = 1, 2, 3, 4, 5 drew the masked symbol and its two segments
  red, green, blue, cyan and magenta: the indices of the colour list (graphing 1.5).
- Disable Masking is a global switch (`@MM`); the menu then reads Enable Masking. With it on, `stats` counts the masked
  cell (n 5 instead of 4) and the graph draws the masked point as an ordinary black point; the mask itself stays.
- Mask Cells by Condition... is `colmask` (inputs `irng`, `cond`, `nSD`, `val`, `abs`, `keep`). Dialog: Input,
  Condition (Outliers by Std. Deviation, >, >=, <, <=, =; the first by default), Multiple of SD (1 to 6, default 1),
  for the other conditions Value and Use Absolute Values to Test, and Keep Existing Mask (off). 46 runs on our data:
  the outlier test masks `|y - mean| > k s` with the sample standard deviation (divisor n - 1) of all the numbers
  (one set separates n - 1 from n; `>=` fits as well); `cond` 1 to 5 are the five comparisons in the dialog's order;
  with Use Absolute Values `|y|` is compared with `|value|` (`|y| > -1` leaves 0 and 1 unmasked); without Keep Existing
  Mask the column's old mask is replaced, with it the new cells are added.
Sheaf: `Project::mask_color` (saved with the project, Origin keeps it as a setting) and `cycle_mask_color`; Disable
Masking is one project switch (`Project::set_masking_disabled`) copied into every column after each edit, so columns
made later follow it (`Column::active_mask` is what analysis, plots, the categorical tools and conditional formatting
read); `sheetops::MaskCondition` and `Worksheet::mask_by_condition`. The worksheet still shows
masked cells in its mask colours while masking is disabled (Origin's sheet colours were not measured).
- Hide masked points (Origin's Mask toolbar switch `@MP`, measured 2026-11-15 with the same graph): the masked point
  and the two segments that meet at it disappear, leaving a gap as for a missing value; the axis range stayed as it
  was (the axes were not rescaled in that run). Sheaf: `Project::hide_masked`, Column > Mask > Hide Masked Points
  (Show Masked Points while on; Sheaf has no Mask toolbar, so the switch sits in this submenu); graphs treat hidden
  masked rows as missing values, analysis is unchanged. The worksheet shows a masked cell with red text on a light red fill; Origin's own cell colour was not measured.

### 13.2 Worksheet operations in Sheaf (T10)

- Edit > Insert and Delete (also the cell and row-number menus): with whole columns selected, Insert adds one new Y column before each selected column's block (as many as are selected) and Delete removes the columns; with rows or a block of cells selected, Insert adds blank cells above the selection and Delete removes the selected cells, moving the cells below, in the columns the selection covers (all columns for whole rows). Columns that recalculate from a formula are left alone. This follows how Origin documents the commands; the GUI result was not measured.
- Column > Move Columns (To First, Left, Right, To Last) keeps the selected columns in their order; Hide Selected / Unhide All set `Column::hidden`; the grid skips hidden columns and the cursor keys jump over them. Column Width (Column Properties, characters, default 8) is `Column::width`.
- Column > Reverse Order reverses each selected column on its own (`colreverse`). Worksheet > Sort Columns by Label orders the columns by Long Name, Units, Comments or Short Name.
- Column > Filter (Add or Remove Filter, Enable/Disable Filter, Reapply Filter): `Worksheet::filter` holds a condition in the column-formula language and the rows that passed when it was applied. Hidden rows stay in the data; the grid shows only the rows that pass, with their own row numbers; analysis and fits read the sheet through `Worksheet::values_of`, which turns hidden rows (and masked cells) into missing values. Sorts and row inserts and deletes apply the filter again; editing a cell does not (as the spec above says, Origin re-runs a filter on demand). Presets: above average, below average, within 3 sigma of the mean, not missing. Not done: one filter per column, date/text filters, and the installed-Origin measurement of the filter itself. The old destructive Worksheet Query stays.
- Worksheet > Transpose, Stack Columns, Unstack Columns, Split Columns, Reduce Rows (`reshape.rs`, one window each) write a **new sheet** of the workbook, so the source stays and one undo removes the result. Measured with `wtranspose iw:=<range>`: three columns of two rows become two columns of three rows (new column j is old row j), the columns are named A, B, ... and the label rows (long name, unit) of the first columns stay in place; Origin's default output replaces the sheet, and `ow:=<new>` makes a new sheet named after the function. Sheaf always makes a new sheet. Stack (a `Source` text column and a `Value` column), Unstack (key column and value column, one column per key), Split (pieces of n rows) and Reduce (every n rows by average, sum, minimum, maximum, median or first) are Sheaf's reading of the Origin commands; their option names could not be found by scripting, so their exact output layout was not compared.
- Column > Set as Categorical (`Column::categorical`, a toggle) and the designations Group (G) and Subject (S) in Set As and Column Properties. Origin accepts the column type codes 7, 8 and 9 in `wks.colN.type` (set through scripting; which code is which was not read back as a name), so the codes of Group and Subject in a file are not known: the `.opj` writer stores them as unused and names them in the save warnings. Categorical columns are offered first as the key of Unstack Columns; no analysis tool reads Group or Subject yet.
- The `.opj` writer does not store hidden columns or column widths: `Writer::finish` names them in the save warnings, so the `.sheaf` companion keeps them.

### 13.2a Plots follow worksheet filters (T10, 2026-11-14)

The published [Data Filter reference](https://docs.originlab.com/origin-help/wks-datafilter/)
says filter-hidden rows are excluded from plotting by default and that edited data requires
Reapply Filter. Sheaf now follows the stored filter for plotting as it already did for the
worksheet and analysis. This change uses the public reference, not a new installed-Origin probe.

`Project::plot_row_filters` collects active filters on a plot's X, Y and auxiliary source columns;
when sources come from different worksheets, Sheaf requires a row to pass each source filter.
That intersection is Sheaf's explicit rule; the cross-sheet case was not measured in Origin.
`PlotData::rows` skips hidden rows without compacting or renumbering the underlying data. Lines
and filled areas join the remaining points, while real missing values still break them. Labels,
error bars and masks retain their original source-row references. Hidden masked cells disappear;
visible masked cells keep the existing red drawing. Automatic axis ranges and automatic stack
offsets ignore hidden values. Histogram, box and pie summaries use only visible source values;
stacked totals and bubble/color scales do likewise. Radar polygons close to the first visible
source point even when their first worksheet row is hidden. The Data Reader cannot pick or step
to filtered rows and stays within the plot's row range.

Disabling/removing a filter restores the full graph; filter edits undo/redo with the worksheet.
The same plot data feeds the graph window and image/vector figure exports. The `.opj` writer
still warns that it cannot store worksheet filters; the `.sheaf` companion retains them. Numeric data, formula evaluation, filter
persistence and the existing explicit Reapply behavior are unchanged. Sheaf still has one filter
per worksheet, lacks the Ignore Hidden Rows override, and retains original category tick/radar
spoke positions rather than compressing their labels. Date/text filters and independently hidden
rows remain separate work. Graph gadgets retain their existing input behavior; only the Data
Reader was updated here.

### 13.3 Cell selection and editing (owner-requested behavior, 2026-11-13)

A click in a yellow label cell selects only that label, outlined in black; only the grey
column-letter header selects the entire column. Labels have their own selection state, separate
from data ranges, so Copy/Cut/Paste/Clear on a label never change the data below it. Long Name,
Units, Comments and user-parameter cells accept typing or F2; Enter/Tab commit and move,
Shift reverses the direction, and Escape cancels. Arrow keys move between label cells; Down
after the last label enters data row 1. Formula labels continue to use Set Column Values.
Structural Insert/Delete and Mask are data-range operations, absent from the label-cell context menu.

The same grid handles newly created and imported workbooks, including initially empty cells.
Regression UI tests exercise both creation paths with mouse selection, typing, Enter/Tab,
clipboard operations, Delete and undo, and assert that editing labels leaves the data unchanged.
The cell editors retain Tab themselves so egui's widget-focus traversal cannot steal the grid's
focus after committing. Each new edit uses a fresh editor state with the cursor after the initial
typed text, including Unicode; stale cursor positions must not turn typing `a`, then `b`, into
`ba`. Navigation within an ongoing edit retains its normal cursor/selection state.
Filtered-table keyboard follow-up (2026-11-14): arrows, Home/End, Page Up/Down,
Enter/Tab and leaving an edit skip filter-hidden rows and hidden columns. Ctrl+Down/End
search within the target column's data before falling back to the current visible cell;
spare entry rows remain reachable. Enter/Tab and their Shift versions walk only visible
cells within a selected block, retaining source row numbers and wrapping in the existing
row/column order. Go to and leaving the last label row also land on visible cells. A filter
that hides the active cell moves it to a visible cell within the selection or to the next
visible entry row; an editor that becomes hidden is cancelled. New/imported-table UI
regressions verify movement, edit commits, block traversal and unchanged hidden values.
This is Sheaf keyboard consistency, not a new Origin shortcut/default measurement.

This selection behavior follows the owner's report, not a new measurement of Origin.

### 13.4 Clipboard text preservation (T07, 2026-11-14)

Worksheet Copy and Cut encode tab-separated fields. Text containing a comma, tab, line break,
carriage return or double quote is quoted, with embedded quotes doubled. This keeps a single
column's commas from becoming extra columns on Paste and preserves embedded whitespace instead
of replacing it with spaces. Copy including label rows uses the same field encoding; those rows
remain plain table rows when pasted into data cells. Direct label-cell copy/paste stays plain text.

Paste accepts tab-separated or comma-separated external text, choosing tabs outside quoted fields
first. Quoted single fields and multiline fields are supported; quotes within ordinary text or
without a complete quoted field stay literal. Core round-trip tests and UI tests on both new and imported tables cover
field boundaries, Unicode, Copy/Cut, Paste, and Undo/Redo. These are Sheaf data-preservation rules;
The Origin 2016 measurements and remaining transport differences are recorded below.

### 13.4a Origin 2016 clipboard defaults (T07 follow-up, 2026-11-14)

Measured through COM under `Global\SheafOriginReference`, using synthetic columns and the real
Copy menu commands (57634, full precision 38798, including labels 38799, labels only 38801,
formula only 38800). Raw scripts, `.tsv` clipboard text and JSON are local-only in
`re-workspace/scratch/t07-origin-copy-defaults/`; `probe3.log` includes a formula created by
`csetvalue`, not merely assigned data. Only text owned by the temporary Origin process was
recorded. Clipboard data was retained in memory for restoration, and no project was saved.

- **Copy:** displayed digits. With default General and a Decimal column (`format=1`,
  `subformat=1`, `digitMode=1`, `digits=2`), 1.2345678901234567 copied as `1.23457` and
  2.345678901234567 as `2.35`. Numeric missing cells copied as `--`, while empty cells of a
  Text column copied as empty fields. Cell-range and whole-column copies produced the same text.
- **Full precision:** 15 significant digits, including `1.23456789012346`, `123456789.123457`,
  `1.23456789012346E-8` and `0.3` for the stored input 0.30000000000000004. Missing numeric
  values still copied as `--`. The existing export formatter matches these measured values.
- **Including labels:** Long Name, Units, Comments, **F(x)**, user parameters, then data. The
  F(x) row was present even when empty; with the real formula `i*2`, its fourth-row cell
  contained that formula. Parameters Temperature then Sample followed creation order. Label-only
  and formula-only menu commands independently confirmed those rows.
- **Transport:** tabs separate columns; CRLF separates rows; there is no final line terminator.
  A comma within a Text cell was unquoted. `@CWACC` and `@PLO` both started at 0 in this
  instance; changing `@PLO` to 1 did not change the tested 2016 output. No rule for other
  versions or configurations is inferred.

**Sheaf fixes:** Copy/Cut now represent stored missing Numeric and Text & Numeric cells with
`--` (empty text remains empty), and Copy including label rows retains the empty F(x) row.
Core Origin regressions and UI tests on both new/imported worksheets cover these changes and
numeric paste/Undo. Parameter rows retain Sheaf's displayed alphabetical order (the existing
model decision); missing cells beyond the stored column length retain their existing empty-field
behavior because that case was not measured independently.

**Transport:** Windows now has separate external text and private lossless data (13.4b).
Other platforms retain the existing lossless text fallback; parameter order and quoting remain
explicit differences. The published
[Origin precision reference](https://docs.originlab.com/quick-help/full-precision-copy/)
describes full-precision worksheet-to-worksheet copying even when external text uses displayed
digits. The current
[copying reference](https://docs.originlab.com/origin-help/copypaste-wks-data/)
describes a newer label-selection default, which is not substituted for the measured 2016 result.

### 13.4b Windows worksheet clipboard formats (T07)

On Windows, worksheet Copy/Cut publish Unicode text using the current displayed number format;
Copy Full Precision publishes 15 significant digits. Both use tabs and CRLF without a final
newline. Copy including label rows uses displayed data and the rows measured above. Ctrl+Alt+C
and Ctrl+Shift+C keep their precision/label behavior when the native window bridge sends a Copy
event instead of a key event. Origin's `@CWACC` preference is not imported.

The same clipboard transaction publishes `OriginRust.Sheaf.Worksheet.v1`, a private lossless
table. A receiving Sheaf instance reads both forms under one clipboard lock, verifies that the
private external text exactly matches the actual Unicode clipboard contents, and uses the full
data for worksheet paste. The incoming paste event may contain the raw text or the window
bridge's one-pass CRLF-to-LF normalization. Literal CR-CR-LF sequences and final-newline differences
are preserved/checked. A changed clipboard, absent/private-invalid data or an external text-only
copy uses the incoming text. The format is independent of project/column identifiers and remains
readable after the writing process exits. Label-cell and text-editor copies remain plain text.

The private bytes are `SHEAFWK1`, two little-endian u64 UTF-8 byte lengths (external then full
table), and those UTF-8 strings. Length arithmetic, version and UTF-8 are checked; padding after
the declared data is ignored. The total private payload is capped at 128 MiB. If native writing
or payload allocation fails, or on non-Windows systems, Copy keeps the existing lossless text
fallback. Graph Copy Page continues using its bitmap/metafile adapter.

An empty external text cell uses one LF so that the window bridge emits a paste event; its
private data contains an explicitly quoted empty cell, so pasting clears a destination cell.
Trailing empty single-column text rows are likewise quoted internally. Copy remains a read-only
project action; paste and cut retain their existing Undo/Redo boundaries.

Validation: Origin fixtures cover displayed and 15-digit text; core tests cover empty text rows;
pure payload tests cover versions, lengths, Unicode and event normalization; UI tests cover both
new/imported tables, exact numeric bits, modifiers, labels, empty cells, Cut/Paste and Undo.
The manual example `cargo run -p sheaf-app --example worksheet_clipboard_probe` verifies the real
Windows formats. A second `read` process recovered exact data after the writer exited, and an
`external` process verified that replacing the clipboard with the same Unicode-only text removes
the private data. The local scratch wrapper preserved/restored the prior clipboard data object.

Remaining differences: other platforms use the fallback; quoting protects separators rather than
matching Origin's unquoted commas; parameter rows follow the current alphabetical display order;
number display details remain those of the column formatter. Formatted/OLE tables, links and
automatic label detection on paste are separate features.

### 13.4c Edit > Copy variants (T88, measured 2026-11-16, Origin 2016)

The workbook's Edit > Copy submenu (Origin 2016: Copy Ctrl+C, Copy (full precision) Ctrl+Alt+C, Copy
(including label rows) Ctrl+Shift+C, Copy (formula) Ctrl+J, Copy (label rows) Ctrl+Shift+J, Copy (formula
and label rows) Ctrl+Alt+J, Copy All Ctrl+Shift+A). Measured by pressing the keys with whole columns
selected and reading the clipboard (`tools/origin-ref/origin-dialog.ps1 -NoDialog` with `@KEY` and
`@CLIP`, `re-workspace/scratch/t88`, local only; fixture in `crates/core/src/clipboard.rs`). Columns B
(Long Name Bee, Units mA, Comments note, no formula; 10 20 30) and C (Long Name Double, F(x) `col(B)*2`;
20 40 60); rows end in CRLF, no line break after the last:

| Item | Text | Sheaf |
|---|---|---|
| Copy (formula) | a tab, then `col(B)*2` (the F(x)= row) | `CopyPart::Formula` |
| Copy (label rows) | Long Name, Units, Comments, then F(x)= | `CopyPart::LabelRows` (then the user parameter rows) |
| Copy (formula and label rows) | F(x)= first, then Long Name, Units, Comments | `CopyPart::FormulaAndLabels` |
| Copy All | F(x)=, Long Name, Units, Comments, then the values | `CopyPart::All` (of the selected columns) |
| Copy (including label rows) | Long Name, Units, Comments, F(x)=, then the values | unchanged (13.4) |

- With a block of cells selected, Origin greys the four (and Copy (including label rows)); with whole
  columns they are enabled. Sheaf refuses the four unless whole columns or the whole sheet are selected
  ("Select whole columns first"); Copy including label rows stays enabled for a block, as before.
- Keys: Ctrl+J is Copy formula in a workbook and Copy Page in a graph (the key goes to the command that can
  run, as Ctrl+G); Ctrl+Alt+J and Ctrl+Shift+A as Origin. Ctrl+Shift+J stays View > Full Screen in Sheaf,
  so Copy label rows has no key (which one Origin runs in a workbook was not checked).
- Where the user parameter rows go in Origin's text was not measured (Sheaf puts them after the other
  label rows, as Copy including label rows does).

## 13b. Cell formats (T59)

Manual formatting of data cells and label cells: bold, italic, underline, font, size, text colour,
fill colour, horizontal alignment (`crates/core/src/cellformat.rs`, `Worksheet.cell_formats`).
Basic worksheet toolbar/font/row-growth behaviour was measured in Origin 2016 on Windows
(2026-11-15, GUI 3 Cell formatting). The model rules, dialog scope, conditional precedence and
`.opj` storage below remain Sheaf's choices unless explicitly stated otherwise.

- **Storage.** An ordered list of ranges. A range is column ids plus either a data-row span
  (first row, optional last row; no last row runs to the end of the data and beyond) or label rows
  (long name, units, comments, F(x)=, a parameter), and a style whose unset properties change
  nothing. Whole-column formatting has no row span: one record however many rows. Empty lists are
  not saved; old files load without formats.
- **Overlap.** Later ranges win property by property. A transparent colour means "no fill" or
  "default text colour", an empty font name "default font", size 0 "default size" and alignment
  Auto "left for text, right for numbers"; each lets a later range undo an earlier one.
- **Applying.** Appends a range, merging into the last one when it has the same target, or the
  same style on the same rows of other columns or on touching row spans of the same columns.
  Earlier ranges wholly inside the new one lose the properties it sets.
- **Clear Formats** cuts ranges over the selected cells (data cells and label cells are separate
  targets). Selections: a block that runs to the last data row is open-ended; columns, rows and
  the whole sheet are cells of those columns/rows; a row filter does not matter (hidden rows in
  the span are formatted too).
- **Toolbar/keys.** Underline and fill use the existing range fields. Origin 2016 measurements
  (GUI 3, 2026-11-16) confirm per-cell U and Ctrl+B/I/U, Fill Color/None, and data-only whole-column
  underline distinct from label underline. Sheaf uses one Undo per key or palette change, leaves
  source columns/formulas alone and rejects stale requests and effective no-ops.
- **Custom toolbar colours.** Worksheet text/fill Custom... accepts opaque RGB bytes, with a
  preview and explicit OK/Enter or Cancel/Escape. GUI 3 records the native chooser and target
  measurements. This one-off dialog starts at the current manual colour (white for no fill),
  a Sheaf policy; drafts do not mutate the project. Existing range/request fields preserve
  independent labels/data, compact whole columns, other styles and `.sheaf`/companion saves.
  Stale selection/generation/revision and effective no-ops create no history entry.
- **Reusable custom wells.** The toolbar's numeric text/fill chooser and Format Cells
  share the graph/editor profile pool (graphing7.12; GUI3). Add saves only a present
  opaque draft colour; a stored choice seeds the numeric toolbar draft or optional
  dialog colour. Existing OK/Apply commits formats; Cancel discards uncommitted
  changes while retaining preferences. Removing a well changes no draft or applied
  format. Existing range/label/whole-column/history/stale and save/export paths
  remain unchanged. This is a documented reusable-chooser contract with explicit
  Sheaf menu/profile policies, not a new Native2016 measurement. No model fields
  or project palette are added; unused wells stay profile-only.
- **Classic presets.** The worksheet Font Color and Fill Color menus include all 24 native
  Classic RGB colours (GUI 3, measurements 2026-11-16). The additional 16 names are grouped in a
  submenu, a Sheaf UI choice. They reuse the same captured range changes and persistence as
  basic/custom colours, including stale-target rejection and independent data/label styles.
- **Dialog scope.** Set Data Style > More... reuses the selected-range Format Cells window.
  Origin 2016 has no Default data-style command; Rich Text/Wrap Text/Float remain separate,
  unsupported text controls (B5). Its Worksheet Properties Format tab instead has category
  defaults (Data > Standard, plus headers and label categories). F4 now opens a separate
  category-font dialog with Data family/size and Long Name/Units/Comments sizes (GUI 3,
  measurements 2026-11-16). Other categories remain disabled.
- **Data font defaults.** Optional `Worksheet.font_defaults.data` holds a family/point size;
  old files omit the empty field. Resolution fills missing manual family/size from that default,
  preserving every other manual/conditional property and keeping labels independent. One sparse
  setting reaches future columns and arbitrarily remote rows without per-cell records. Clone,
  duplicate and project/companion save carry it with the worksheet. The existing batch ReplaceSheet
  path starts with default formatting, matching its existing range-format reset policy. The grid
  cache includes defaults even when no manual ranges exist. Print/image and Excel reuse effective
  style resolution. `.opj` warns and requires its full `.sheaf` companion for these fonts.
  Excel writes current values and blank cells inside the used rectangle; it cannot retain Sheaf's
  future-cell inheritance, and import still does not read cell styles. Category labels, Standard/
  header defaults and persistent row-height changes remain outside this slice.
- **Label size defaults.** Optional `font_defaults.long_name_size_pt`, `units_size_pt` and
  `comments_size_pt` store independent sparse point sizes below manual label ranges. Old sheets
  retain their serialized shape. Each setting covers existing and future columns without
  per-column records, leaves data and unsupported formula/parameter categories independent,
  and uses the existing 4..96 pt bounds. The F4 dialog keeps category drafts and commits all
  changed categories in one Undo; Cancel/Escape discard unapplied drafts. These draft rules
  and retained manual-label precedence are Sheaf policy, not measured Native parity. Native
  category sizes/new columns and specific-over-bulk priority were measured (GUI3); family and
  manual-label precedence remain open. Print/image/grid use existing effective label resolution
  and derived geometry; empty printed labels remain under T27's existing omission policy.
  Project and OPJ-companion saves preserve the fields; unsupported native OPJ formatting warns.
  Excel writes effective sizes only for exported label rows, reports omitted category styles,
  and leaves data blanks unstyled when only label defaults exist. Future Excel label inheritance
  is not preserved. Bulk/Standard/header defaults remain disabled.
- **Label family defaults (documented contract).** Optional `long_name_family`, `units_family`
  and `comments_family` strings extend the existing `font_defaults` object without changing
  its point-size fields. A missing family uses the existing automatic Arial face. Family-only
  edits do not invent size overrides; clearing Automatic removes only the family field. Named
  manual label faces and sizes retain independent precedence. Unknown installed faces use the
  same renderer fallback as Data. Existing/future columns, cloning, native project and OPJ
  companions retain these sparse category settings; `.opj` warns about unsupported formatting.
  Excel writes effective families for requested label rows and warns when family-only labels
  are omitted, leaving data cells/blanks independent. The F4 chooser reuses installed-font
  search and retained category drafts. The official Format-tab manual supports the control
  contract (GUI3); Native family mutation/future inheritance/manual/Automatic parity remains
  unverified after excluded ineffective attempts and a later COM registration refusal. These
  inheritance/reset rules are Sheaf policies. Data, unsupported categories and T27 extents stay
  unchanged.
- **Vertical alignment.** The 2016 selected-A1 Format Cells and Worksheet Properties Data
  Format/Size probes (GUI 3, 2026-11-16) found no vertical text-alignment control in those
  inspected routes. Other routes and mutation semantics remain unestablished; this probe
  does not justify adding a new per-range field. Existing placement remains in use.
  The documentation scope audit in GUI 3 distinguishes the later Data Info/tooltip
  table controls from worksheet ranges; it does not supply the missing worksheet route.
- **Conditional formats.** Conditional fill and text colour override manual ones; bold, italic,
  underline, font, size and alignment stay manual.
- **Maintenance.** Row insert/delete shift, grow, shrink or drop spans of the affected columns
  (a range that covers other columns is split first). Sorts and other row permutations move
  formats with their rows, like cell notes (whole-column formats are unchanged; rows an order
  drops lose theirs, rows beyond the data stay). Column delete and analysis result retirement
  drop ids; duplicate sheet/window and append rename ids. Undo and redo are ordinary document
  steps. After a reorder, records with the same columns and style whose spans touch or overlap
  are merged (a permutation of a fully formatted span is one record again; a partly formatted
  column keeps one record per run of formatted rows). When ranges of different styles overlap,
  only neighbouring records are merged so later-wins order holds. A row removal in place (filter)
  puts an open-ended tail at the new row count, so rows appended later are covered as before.
- **Lookup.** Drawing resolves cells through a per-revision index: per column a sorted list of
  non-overlapping row stretches with their resolved style, found by binary search, so a lookup
  costs O(log n) whatever the number of records.
- **Size.** Font sizes are clamped to 4..96 pt (`clamp_cell_pt`) in the grid, print, PDF and
  image export alike. Origin's measured default is Arial 9 pt. The grid converts points to
  logical pixels by 4/3; print scales by its existing default font over 9 pt. Installed faces
  and derived row geometry keep enlarged cells visible in grid/print/image output. Geometry
  uses format intervals, not one record per data row. Derived heights can shrink with the font;
  Origin retains enlarged row heights, which the current model cannot persist.
- **Files.** `.sheaf` and the `.opj` companion keep everything. The `.opj` writer holds none and
  warns ("N sheet(s) have cell formatting ..."). Excel export combines resolved manual styles
  with each column's numeric/date/time code. Fonts use the worksheet default Arial 9 pt,
  explicit sizes use the same 4..96 pt clamp, and bold/italic/underline, RGB text/fill and
  horizontal alignment are retained. Number values stay numeric and mixed/text cells stay text;
  pre-1900 dates retain the existing plain-number fallback and warning, with their cell style.
  Long-name/units/comments styles follow the label rows actually requested by the export;
  short names use the default style. Blank styled cells inside the existing data rectangle are
  included. Open-ended ranges do not create extra rows; explicit formatting outside that rectangle
  and omitted label-row formatting generate warnings. Future-cell styling is not exported.
  Conditional colours use the existing effective precedence as a static snapshot, with a warning
  that rules are not written. Transparent resets remain resets; partial alpha is written as
  opaque RGB with a warning. These are Sheaf export choices, not new Origin measurements.
  The Excel importer still reads data/column number formats rather than cell styles.

## 13c. Fill Column With patterns (T74, measured 2026-11-15, Origin 2016)

Column > Fill Column With (and the column header's Fill Column with) > A set of Numbers..., A set of
Date/Time values..., Arbitrary set of Text&Numeric values... are Origin's X-Functions `patternN`,
`patternD`, `patternT`. Measured by running them from LabTalk (`tools/origin-ref/run-labtalk-lines.ps1`,
`re-workspace/scratch/t74/f1` and `f2`, local only) and copied in `sheaf_core::fill_pattern`
(tests `crates/core/src/fill_pattern/tests.rs` hold the measured cases):

- **Run**: from `from` toward `to` by `inc`, `to` included when a step lands on it (1 to 10 by 3 is 1, 4,
  7, 10; 1 to 2 by 0.25 has five values). An increment that points away from `to` writes nothing (5 to 1
  by 1), by -1 it counts down.
- **Repeat mode**: each value `onerepeat` times in a row, the run `seqrepeat` times (1 to 3, 2 and 3 gives
  1, 1, 2, 2, 3, 3 three times, 18 rows); the set size is not used.
- **Random mode**: `size` values drawn from the run with replacement (1 to 4, size 7 gave 4 3 3 1 2 2 1).
  Random dates have no size: they fill as many rows as the sheet has (9 there).
- **Dates and times**: the unit is a fixed length: a week is 7 days, a **month 31 days** (Jan 31 by one
  month: Jan 31, Mar 3, Apr 3, May 4, Jun 4) and a **year 365 days** (2024-01-01 plus a year is
  2024-12-31); seconds, minutes and hours as fractions of a day. The column becomes a Date column (Origin
  format 4) or, for times, a Time column (format 3); a text fill leaves Text & Numeric (format 7).
- **Rows**: the fill writes from the first row; rows below it keep their values (1 to 3 into a column of 1
  to 8 leaves 4 to 8). Sheaf also drops the column's formula, since the values no longer follow it.
- **Text items**: separated by white space; double quotes keep an item with spaces together (the hint of
  Origin's window, `Boston "New York" Chicago`; Origin's own quote handling could not be checked from a
  script). A number item is stored as a number.
- **Window** (Sheaf's layout, Origin's window not opened): From, To, Increment (with the unit list for dates;
  Date or Time), Mode Repeat or Random, the repeat counts or the set size. Defaults as the X-Functions: 1 to
  10 by 1, each once, the run once, a set of 10; dates from today for ten days by a day; times 00:00 to
  12:00 by an hour. OK fills every selected column, one undo step ("Fill column"); a refused fill (no
  value, no text item) keeps the window open with the reason.
- Row Numbers, Uniform and Normal Random Numbers in the same submenu are T63 group H.

## 13d. Sampling interval (T84, measured 2026-11-16, Origin 2016)

Column > Set Sampling Interval... and Show X Column... (and the same two items of the column header's
menu) are Origin's X-Functions `colint` (Initial X Value, default 1; X Increment, default 1; Units; Long
Name) and `colshowx` (Clear Sampling Interval, default off). Measured from LabTalk
(`tools/origin-ref/run-labtalk-lines.ps1`, `re-workspace/scratch/t84/s1`-`s3`, local only) and copied in
`sheaf_core::sampling` (tests `crates/core/src/sampling/tests.rs`):

- **The interval** gives row `i` (from 0) the X value `x0 + i * inc`, with an X long name and units
  (`wks.col.xinit`, `xinc`, `xname$`, `xunits$`; `xof()` gave 0 to 2 for x0 = 0, inc = 0.5). An initial
  value and increment of 1 clear it (Origin's own hint in the window). Several selected columns get the
  same interval.
- **Plots** of the column use the interval even when an X column stands to its left: a scatter of B with
  x0 = 0, inc = 0.5 drew X from 0 to 2 (axis -0.5 to 2.5), not the X column's 1 to 5. Sheaf gives such a
  plot no X column and draws it at the interval; the X axis title is the interval's long name and units
  (Sheaf's choice; Origin's title not measured). A negative increment works (100, 98, 96).
- **Show X Column** inserts a new column directly left of the column, named as a new column (the next
  free letter: E, then F), with the interval's long name and units and as many rows as the column (3 for
  a 3-row column in a 5-row sheet). Without Clear Sampling Interval it is a **Disregard** column (type 2)
  and the interval stays, so plots still use the interval; with it, it is an **X** column (type 4) and the
  interval is cleared, so the column now plots against it (axis 95 to 101 for 100, 98, 96). Showing it
  again inserts another copy. A column without an interval shows nothing: Origin's `colshowx` fails; Sheaf
  keeps the window open with "column B has no sampling interval to show" and changes nothing.
- **Windows** (Sheaf's layout; Origin's windows not opened): Set Sampling Interval has the four fields
  filled from the first selected column's interval (1, 1 and empty without one) and Origin's hint line; Show
  X Column has the one check box. OK changes every selected column, one undo step each ("Set sampling
  interval", "Show X column").
- **Files**: the interval is saved with the column in `.sheaf` (`"sampling": {"x0", "inc", "long_name",
  "units"}`). The `.opj` written here does not hold it: the save warning says "N column(s) have a sampling
  interval ...", and the companion keeps it for Sheaf. Not read from Origin's `.opj` files yet. Sheaf's
  analysis tools (fits, statistics) still take the X column or the row number, not the interval.

## 13e. Swap Columns and Paste Transpose (T86, 2026-11-16)

- **Swap Columns...** (the column header's menu) is Origin 2016's `colswap`, measured from LabTalk
  (`re-workspace/scratch/t86/w1`, local only; `crates/core/tests/swap_transpose.rs`): with exactly two
  columns selected they exchange places, each taking everything of its own along (B and D of A..E give A,
  D, C, B, E; D keeps its long name, units, Y-error designation and width, B its width of 12). With one or
  three columns selected `colswap` fails and nothing changes; Sheaf says "Select exactly two columns to
  swap". One undo step ("Swap columns"). Origin opened no window from LabTalk; Sheaf opens none.
- **Paste Transpose** (Edit menu and the data cell's menu) pastes the clipboard's block with its rows as
  columns and its columns as rows, at the active cell, by the rules of an ordinary paste (section 13.3:
  columns added at the right when needed, an empty column takes the type of what is pasted, a number
  column with data keeps its type, columns a formula keeps up to date are left alone). Short rows of the
  text give empty cells. One undo step ("Paste transpose"); the next ordinary paste is not transposed.
  Sheaf's rule, not measured in Origin (a menu-only command that a script cannot reach with the
  clipboard).

## 13f. Column data range: Set As Begin, Set As End, Reset to Full Range (T87, measured 2026-11-16)

Edit > Set As Begin, Set As End and Reset to Full Range (workbook), and the data cell's Set as Begin and
Set as End. Measured in Origin 2016 with the LabTalk commands behind them (`set col(B) -b n`, `-e n`,
`get col(B) -b`; `re-workspace/scratch/t87/r1` and a screenshot, local only) and copied in
`sheaf_core::data_range`:

- A column's **data range** is its rows from a begin to an end; the values outside stay stored (a dump of
  the sheet still read them, and setting begin 1 and end 6 brought them back).
- The **worksheet shows the cells outside blank** (B begun at row 3: rows 1 and 2 empty; C ended at row 4:
  rows 5 and 6 empty).
- **Plots and statistics use only the rows inside**: B (10 to 60) from row 2 to 4 gave `stats` n = 3, mean
  30, and its scatter's X axis went from 1.5 to 4.5. Sheaf's plots leave the rows out;
  Statistics on Columns and Rows now also use the saved ranges (T126/T127 below). Other analysis readers remain open.
- **Sheaf's commands**: Set As Begin makes the top selected row the first row of every selected column's
  range, Set As End the bottom selected row the last; a begin after the end drops the end (and the other
  way round). Reset to Full Range clears the range of every column of the active sheet. One undo step each
  ("Set as begin", "Set as end", "Reset to full range"). Which columns Origin's menu commands change was
  not checked in its window.
- **Files**: saved with the column in `.sheaf` (`"data_range": {"begin", "end"}`, rows from 0). The `.opj`
  written here does not hold it: the save warning names it, and the companion keeps it. The graph's own
  Data > Reset to Full Range (display range of a plot) is a different command and stays grey.

### 13f.1 Descriptive Statistics on Columns (T126, 2026-11-17)

The descriptive column reader intersects the existing resolved request rows with
that Y column's saved Begin/End range, before calling the unchanged statistics
calculator. The T87 accepted 2016 fixture above is the numeric reference: values
10..60 with rows 2..4 yield N=3, mean30, SD10 and sum90. No new Native run or COM
retry was made. The [current FAQ-1101](https://docs.originlab.com/quick-help/set-as-begin/)
describes clearing/deleting and 2021/2022 menu changes; those are not substituted
for the accepted non-destructive 2016 column-range fixture.

Explicit row/XBetween requests intersect the saved Y range using original row
indices. Each selected column has its own range; a different X column's saved
range does not restrict univariate Y statistics. Outside rows are excluded and
do not increase Missing. Existing mask/filter/missing handling within the
intersection, units, report layout, all optional statistics and output history
stay unchanged. Empty intersections use existing empty statistics (N=0,
Missing=0, sum0, undefined values NaN), an explicit Sheaf edge policy. Bounds are
clamped with a saturating inclusive-end conversion, including persisted ranges
beyond the available data. Source data and range metadata remain untouched.

This T126 slice changes only Statistics on Columns; the row reader follows in
T127 below. Inferential tests, distribution/categorical tools, fitting and other
analysis readers remain T87 leftovers. No shared input-loader, Core model, IO/schema, grid or menu changes.

### 13f.2 Statistics on Rows (T127, 2026-11-17)

The [official rowstats reference](https://docs.originlab.com/x-function/ref/rowstats/)
describes seven row quantities within an input range. The [Statistics on Rows
guide](https://docs.originlab.com/origin-help/statistics-on-rows/) excludes missing
observations. With the existing T87 saved-range contract, Sheaf now admits a
contributor at its original worksheet row only when both the resolved request span
and that column's own Begin/End range contain the row. This per-column intersection
is explicit Sheaf policy, not a fresh Native2016 rowstats measurement; no COM retry
or repair was made. Each input's range is resolved once, not once per output cell.

No rows are compressed or shifted. Existing mask/filter handling, numerical
formulas, output length/order/units/comments, seven quantity switches and insertion
behind the last selected column remain. An unselected X column's saved range does
not remove another contributor. A row without eligible numbers retains the
existing all-missing rowstats outputs, including Number of Points and Sum (unlike
T126's empty column summary zeros); a single number has no SD or SEM. Stored source
values and range metadata stay fixed. Only row_statistics and its call change;
the shared input loader, T126 reader, other tools, model and IO/schema stay fixed.

## 13g. Copy Columns to (T89, measured 2026-11-16, Origin 2016)

The column header's Copy Columns to... is Origin's `colcopy` (Source Column(s); Destination Column(s),
default `<new>!<new>`; Copy Data, Copy Format, Ignore Hidden Rows, Long Name, Units, Comments, Parameters,
User-Defined Parameters, all on). Measured from LabTalk (`re-workspace/scratch/t89/c1`, `c2`, local only)
and copied in `sheaf_core::copy_columns`:

- **No destination**: a new book; the copies are named A, B, ... from its first column and keep their
  designation (a Y-error column stays one), long name, units, comments and display format (3 decimals
  stayed 3). The formula is not copied, the values are.
- **Into a column** (`orng:=[Dst]Sheet1!col(B)`): two copies overwrote B and filled C (a new column after
  the last, a Y-error column as its source); B kept its short name and took the copy's long name. The
  sheet grows to the copies' rows.
- **`<new>` column**: added after the last column with the next letter (D). **`<new>!<new>`** in a book:
  a new sheet (Sheet2).
- Sheaf's window (layout Sheaf's; Origin's not opened): New book / New sheet in a workbook / Columns of a
  sheet (workbook, sheet, and the first column overwritten or `<new>` after the last), and the check boxes
  Copy Data, Ignore Hidden Rows, Copy Format (display format and width), Long Name, Units, Comments and
  User-Defined Parameters.
  An unticked part leaves the destination's own (Sheaf's choice, not measured). One undo step ("Copy
  columns"). System parameter rows remain open.

Ignore Hidden Rows starts checked. It copies only rows shown by each source sheet's active
filter and compacts them in their original order. Clearing the option copies all stored rows.
The option applies when Copy Data is on. Sheaf preserves source data and filters, keeps missing
cells and storage types, and stops a short column after its last stored row when compacting.
The existing row order is captured before any destination column is overwritten. These
details use Sheaf's row model. The filter-copy contract and default come from the
[Origin `colcopy` reference](https://docs.originlab.com/x-function/ref/colcopy/).

## 13h. Sheet tab names: Sheet Name to Show, Duplicate Without Data (T90, measured 2026-11-16, Origin 2016)

Measured in Origin 2016 by right-clicking a sheet tab (`tools/origin-ref/origin-dialog.ps1 -NoDialog`,
`@RCLICK`/`@CLICK`/`@SHOT`, `re-workspace/scratch/t90`, local only) and copied in `sheaf_core::sheet_tab`:

- A sheet has a short name and a **long name** (`wks.longname$`). Sheaf keeps it in `Worksheet::long_name`
  (saved in `.sheaf`; Name and Comments... has a Long Name field).
- Name and Comments also edits plain-text, multiline worksheet comments. OK saves the draft
  with the sheet's names in one undo step. Cancel discards it. The tab tooltip shows the
  names and the nonempty comments. Sheaf preserves comments when duplicating a sheet or
  replacing its imported data. The editor and tab hover are documented for Origin 2019b
  and earlier. [Origin worksheet Name and Comments](https://docs.originlab.com/quick-help/add-worksheet-comment-name/)
- **Sheet Name to Show** has three items in Origin 2016: Long Name if available, Short Name (ticked for a
  new sheet) and Long Name (the greyed inventory had listed "Both", which Origin 2016 does not have). The
  choice is **per sheet**: Long Name on Sheet1 showed "Potential scan" on its tab and left Sheet2's tab
  "Sheet2" although Sheet2 had a long name. Long Name if available falls back to the short name. Sheaf also
  shows the short name for Long Name on a sheet without a long name (Origin's empty case not measured).
  One undo step ("Sheet name to show").
- **Duplicate Without Data** adds the copy after the last sheet, named as a new sheet (the first free
  SheetN: Sheet3 next to Sheet1 and Sheet2), with the long name followed by " 1" ("Potential scan 1"; a
  further copy takes " 2" in Sheaf), and makes it active. It keeps the columns with their short names,
  designations, long names, units and formulas (`i*2` stayed, the cells were empty). Sheaf also keeps
  formats, widths, user parameters, conditional and cell formats, and drops values, masks, notes and the
  filter. One undo step ("Duplicate sheet without data").
- The `.opj` writer reports sheet names, tab display choices and comments it cannot represent.
  The `.opj.sheaf` companion preserves them. Native `.sheaf` files store the comments directly.
  Older files without this field load with empty comments.
- **Navigate...** (T93) opens "Navigate Worksheets - Book1" in Origin 2016: a table of the sheets (Index,
  Creation Index, Name with the short name, Columns, Size(KB), Comments, Type "Flat Sheet", Source), the
  active sheet selected, an Actions menu and Apply Reorder (greyed), OK and Close. Sheaf's window has Index,
  Name, Long Name, Columns, Rows and Comments. The Comments cell previews the first line and shows the
  full saved text on hover. Clicking an index or name picks a sheet, OK or a double click goes to it,
  and Close or Escape closes. Name and Comments is the editing route. Actions and Apply Reorder remain open.

## 13i. Worksheet corner menu (T92, measured 2026-11-16, Origin 2016)

Right-clicking the corner above the row numbers in Origin 2016 gives a short menu (Save Template As...,
View, Add New Sheet, Add Graph..., and more behind the chevron). Its **View** submenu, expanded, lists
the switches (screenshots `re-workspace/scratch/t92`, local only): Long Name, Units, Comments,
Parameters, Sampling Interval, Sparklines, Filter, F(x)= (Ctrl+Shift+U), Categories | Row Labels for
Column Label Rows | Column Header, Row Header | Column Grid, Row Grid, Page Break Preview Lines. On a new
sheet Long Name, Units, Comments, F(x)=, Row Labels, both headers and both grids are ticked. (The greyed
inventory had listed Row Labels for Column Label Rows as a submenu of the label rows; in Origin 2016 it is
one switch, the titles left of the label rows.)

Sheaf's corner menu is that list (`sheaf_core::sheet_view`, saved per sheet in `.sheaf`; Sheaf's choice,
the per-book or per-sheet scope was not checked): Long Name, Units, Comments and F(x)= hide or show their
label row; Row Labels hides the row titles; Column Grid and Row Grid the vertical and horizontal lines
between data cells. One undo step each ("Worksheet view"). Grey: Parameters (Origin's system rows),
Sparklines, Filter, Categories and Page Break Preview Lines. User parameter rows show whenever a column has one. The `.opj`
written here does not hold the switches (the save warning names the sheets).

**Column Header and Row Header (T92).** Origin's View-tab illustration distinguishes the
top short-name band from the left gutter containing the label titles and data row numbers.
Sheaf's Column Header switch collapses the top band, and Row Header collapses the gutter.
The label cells and data cells remain in the grid. Row Labels controls the label titles
when the gutter is visible. These switches default to on in older projects, are saved per
sheet, and each creates one undo step. When the corner is hidden, the sheet tab's View
submenu offers both switches. The recovery menu and exact geometry are Sheaf choices.
[Origin Worksheet Properties View tab](https://docs.originlab.com/origin-help/wksproperties-dialog-view-tab/)

**Sampling Interval row (T94, measured 2026-11-16):** after `colint` gave B x0 = 0, inc =
0.5, units s, Origin 2016 showed a "Sampling Interval" label row on its own (the switch turned on), below
F(x)=, with "0.5 (s)" in B (the increment and the units in parentheses) and empty cells in A and C; the
corner switch hides it again. The column heads also changed: B became "B(Y#)" with a small icon, A "A(X1)"
and C "C(Y1)" (designation numbers appear once a column has its own X).

Sheaf shows the row after F(x)= when the sheet has a sampling interval. The corner switch
saves an explicit show or hide choice per sheet and supports Undo and Redo. Legacy sheets use
automatic visibility. A row explicitly shown on a sheet without intervals has empty cells.
Zero-start intervals display the increment with optional units as in the measured case.
[Origin's metadata guide](https://docs.originlab.com/labtalk/guide/accessing-metadata/)
describes the read-only row and its offset and increment. Sheaf displays nonzero starts in
the single-line form `x0 = 2, dx = 0.5 (s)` and uses round-trippable numeric text. Cells without
an interval are empty. The displayed value can be copied. Edit the underlying metadata via
Set Sampling Interval. Column heading icons and print/export support are follow-up work.

## 13j. Find in Project (T95, measured 2026-11-16, Origin 2016)

Edit > Find in Project... (F3; "Find in Project" on a graph's Edit menu) opens Origin's **Find** window
(screenshots `re-workspace/scratch/t95`, local only): Current Path, Find What, Within (Entire Project), Case
Sensitive, Match Whole Word, Allow Wildcards (* and ?), Types to be Included > Metadata: Folder, Graph,
Book, Sheet, Column, Notes and Select..., then Data in Cells (1st Matched in Column), Text Objects on
Graphs, Notes Content; a result list (No., Range, Element, Attribute, Folder) and "Found: N". Defaults
(Select Metadata): Graph, Book, Sheet and Notes with Long Name, Short Name and Comments; Column with Long
Name, Short Name, Comments, Parameters and User Parameters, **not** Units, F(x), Categories, Sampling
Intervals, Filters, Info Variables; Data in Cells off, Text Objects on Graphs on. Origin keeps the settings
between sessions.

Measured hits (book with B "Current density", mA; a line graph of B): "density" gave
`[Graph1]Layer1!YL` Graph / Axis Title, `[Graph1]Layer1!Legend` Graph / Legend and
`[Book1]Sheet1!B"Current density"` Column / Long Name (text objects as drawn); "Graph" gave `[Graph1]`
Graph / Short Name; "30" with Data in Cells gave column B once, attribute Data Cells.

Sheaf (`crate::project_find` in the app, window `dialogs/find_project.rs`): the same fields except Within
(always the whole project), Folder and Notes (Sheaf has no notes windows); Select... shows the column's
Units and F(x) switches; axis titles are searched as drawn (`sheaf_plot::axis_title`), legends with `%(n)`
replaced by the plots' names, and text labels by their text. Clicking a hit brings its window forward (and
its sheet, with the column selected); the window stays open and keeps its settings for the session.
Double-clicking a hit in Origin was not measured.

## 13k. Export > Worksheet as Image (T97, measured 2026-11-16, Origin 2016)

File > Export > Worksheet as Image... is Origin's `expWks` (Image Type AI, BMP, CGM, DXF, EMF, EPS, GIF,
JPG, PCX, PDF, PNG, PSD, TGA, TIF, WMF; Select Sheet(s) Current Sheet / Sheets in Book / Sheets in Folder
/ ... / Specified; File Name(s); Path; Overwrite Existing Auto Rename / Replace / Skip / Ask; Use Tools
Options; Keep Original Size; Image Size and Image Settings). Measured from LabTalk
(`re-workspace/scratch/t97`, local only): the picture of a sheet holds the **label rows filled light
yellow** (long name and units centred) and the **cells** (numbers right-aligned), **without** the column
heads, the row numbers or grid lines, and runs down all the sheet's rows (691 x 1863 px for three columns
of the default 32-row sheet, about 300 dpi). With no file name the script wrote ".png"; a token such as
`<short>` in the name was not replaced.

Sheaf: one picture of the active sheet (its data rows, not empty spare rows), drawn from the printed table
(`sheaf_core::sheet_print`) with the heads, row numbers and grid left out (`sheaf_plot::sheet_image_svg`).
The save window offers PNG (300 dpi), PDF and SVG; the name defaults to `<book long name or name>_<sheet>.png`
in the project's folder (Sheaf's choice). Not done: the other image types, several sheets at once, the size
and overwrite options.

## 13l. Re-Import (T98, measured 2026-11-16, Origin 2016)

File > Import > Re-Import Directly (Ctrl+4) and Re-Import... (workbook File menu; a graph's has none) are
Origin's `reimport` (`dialog` Re-Import Directly / Re-Import by Changing Options; ReImport Selected Range
Only). Measured from LabTalk (`re-workspace/scratch/t98`, local only; `impasc` of x,y,z with three rows,
a comment and units typed on B and B plotted, then the file replaced by x,y,z,w with four rows):

- The values were replaced (four rows) and the graph followed (axes rescaled).
- The book kept its three columns: the file's fourth column was **not** added.
- B kept the comment and units the user typed (the file has none) and its long name from the header.

Sheaf (`Project::replace_book_data`, app `reimport_book`): the book's source file (`Workbook::source`,
importer ascii, chi or excel) is read again with that importer's default options (Origin reuses the
import settings; Sheaf does not store them yet) and each column takes the new values and the labels the
file gives, keeping its id, so graphs keep plotting it; extra columns of the file are reported and not
added; a column the file no longer has is emptied (not measured). One undo step ("Re-import"). Re-Import...
picks a file (starting at the source) and reads it the same way. A book made in Sheaf or opened from a
project has no source and the items are refused. Not done: the stored import options, Selected Range Only.

## 13m. Folders: View Windows, Window > Folders, folder Properties (T100, measured 2026-11-16, Origin 2016)

Measured with screenshots (`re-workspace/scratch/t100`, local only; Book1 in the root folder, Book2 in a
folder Sub made with `pe_mkdir`/`pe_cd`):

- **View Windows** (the explorer folder's menu; Sheaf's View menu too) offered None and **Windows in
  Active Folder** (ticked, Origin's default) on the root folder's menu. With it, selecting Sub showed only
  Book2's window (Book1 was hidden), activated Book2 and added " - /Sub/" to the main title; the root
  showed only Book1. The Window menu's window list then listed only the folder's windows.
- **Window > Folders** lists every folder by path ("1 /UNTITLED/", "2 /UNTITLED/Sub/"), the active one
  ticked; picking one makes it the active folder.
- **Properties...** of a folder opens "Folder Properties - Sub": << Previous / Next >> / Window..., tabs
  Properties and User Tree; Name, Comments; Page Info: Type Folder, Size, Contains 1 Windows, Created,
  Modified, Location /UNTITLED/; OK, Cancel, Apply.
- The root folder's menu: Find... F3, Append Project..., Save Project As..., Show All Windows, Hide All
  Windows, New Window, New Folder, View Windows, Send Graphs to PowerPoint..., Slide Show of Graphs...,
  Properties... (Alt+Enter); a subfolder's adds Delete Folder, Rename (F2), Restricted in Viewer.

Sheaf: `Project::view_windows` (saved in `.sheaf`, one undo step to change) with None, Windows in Active
Folder (default) and Active Folder and Subfolders (the third as the inventory names it; Origin's root
menu showed two). The workspace draws only the shown windows and the Window menu lists only them; when
the folder changes and the active window is not shown, the folder's most recent window comes forward.
Window > Folders lists the folders as Origin. A folder's Properties... edits its Name (not the root's) and
Comments (`Folder::comments`) and shows Type, Contains and Location, one undo step. Not done: the main
title's folder path, Size/Created/Modified, Previous/Next, User Tree, Restricted in Viewer.

## 13n. Pivot Table (T124, 2026-11-17, not measured)

Origin facts (X-Function `wpivot`, docs.originlab.com/x-function/ref/wpivot, Origin 2015 SR0 and
later; Origin COM was unavailable on the Windows machine on 2026-11-17, so nothing was measured):
`row` (one or more columns), `col`, `data`; `method` 0 count (default), 1 sum, 2 mean, 3 min, 4 max;
`total` adds a Row Total column (sum across each row), `sum` a Column Total row; `sort_total` 0 row
labels ascending, 1 descending by row totals (default), 2 ascending by row totals, 3 row labels
descending, 4 none; `sort_col` 0 column labels ascending (default), 1 descending, 2 ascending by
column totals, 3 descending by column totals, 4 none; `normalize` 0 none, 1 fraction, 2 percent of
each column's total; `zero` (default 1) shows missing combinations as zeros; `pos` (default 1)
puts each column's value in 0 Long Name, 1 Comments, 2 appended to the Long Name, 3 a user
parameter row; `dir`/`vmode`/`threshold`/`label` merge small values into "Others"; `exrow`/`excol`
add extra values; the output defaults to a new sheet of the input book.

Sheaf (`crates/core/src/pivot.rs`, `crates/app/src/dialogs/pivot.rs`), choices **unmeasured**:

- Worksheet > Pivot Table... opens the window with the selected columns (or the sheet's first
  columns) as row source, column source and data, then `wpivot`'s defaults. OK writes a new sheet
  `Pivot1`, `Pivot2`, ... of the same workbook, shows it, and is one undo step ("Pivot table").
- Keys: a cell's number or text; numbers sort before texts, numbers by value, texts by characters
  (`0` and `-0` are one key). Several row sources give one row per combination, one label column
  each, compared column by column.
- Source rows: rows hidden by the filter are left out, and so are rows with an empty or masked cell
  in any row source or the column source. A masked, text or empty data cell is missing: Count still
  counts the row; the other methods skip it. A combination with no value is missing (0 with "Show
  Missing as 0", else empty).
- Totals sum the cells shown (after normalizing). Sorting by totals uses the totals before
  normalizing; equal totals keep the labels' ascending order; "None" keeps the order of first
  appearance. Normalize divides each column by its total (a zero total gives empty cells).
- Output columns: the label columns (Long Name the source column's name; the first is X), then one
  Y column per column key with its label in the chosen position (Long Name, Comments, or after the
  data column's Long Name, or the method's name for Count), then Row Total (its name in the same
  position). The Column Total row has "Column Total" in the first label column; its Row Total cell
  is the grand total.
- **Left out** (shown disabled): Combine Direction/Others, extra row and column values, the user
  parameter position.

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
