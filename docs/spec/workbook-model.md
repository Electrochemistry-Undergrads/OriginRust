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
  - The "Significant Digits=" mode of a plain Decimal:1000 column shows the same text without the letters (15 with 3 is `15.0`, 1100 with 3 is `1.10E+03`, 1000 with 3 is `1000`); Since T314 Sheaf's `Significant` display follows it (see the next point).
  - **Measured again on a wider list (T314, Origin 2016, 2026-11-21, `re-workspace/scratch/t314`)**: plain
    display with Significant Digits= n: with `e` the exponent of the value, plain text with `n - 1 - e`
    decimals and zeros kept while `-3 <= e < n` (15 with 3 is `15.0`, 0.00123 with 3 `0.00123`, 0.0999 with
    1 `0.10`, 9.99 with 2 `10.0`); a power of ten with `e == n` stays plain (100 with 2, 10 with 1, 1000 with
    3); otherwise Scientific with n significant digits (`1.10E+03`, `1.2E+02`, `1.23E-04` for 0.000123,
    `1E-05`); 0 is `0`, `0.0`, `0.00` by n. Scientific with Set Decimal Places= d: the mantissa with d
    decimals and `E`, a sign and at least two exponent digits (`1.25E+02`, `5.0E-01`, `0E+00`,
    `1.0E+03` for 999 with 1); with Significant Digits= n it equals d = n - 1; with Default Decimal
    Digits up to 6 significant digits with trailing zeros dropped (`1E+02`, `1.2345E+04`, `3.14159E+00`).
    Rounding is that of the exact binary value with ties away from zero: 125 with 2 digits is
    `1.3E+02`, 12.5 is `13`, 1.5 with 1 is `2`, while 0.95 with 1 is `0.9` and 2.5E-7 `2E-07` (both stored
    just below the tie). Sheaf's `NumberFormat::Significant` and `Scientific` copy this
    (`format_significant`, `format_scientific`); not matched: 0.995, which Origin rounds up (`1.00` with
    2 significant digits, `1.0E+00` in Scientific with 1 decimal) although it is stored below the tie.
    Scientific with Default Decimal Digits is `NumberFormat::ScientificAuto` (T317), "Scientific (default digits)"
    in Column Properties.
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
rows remain separate work. Only the Data Reader was updated here; the graph gadgets followed in
T209 (2026-11-20): every gadget (Integrate, Rise Time, Intersect, Quick Peaks, Sigmoidal Fit, FFT,
Statistics, Quick Fit, Vertical Cursor, Differentiate, Interpolate, Cluster) reads only the plot's
rows inside its row range that pass its worksheet filters and are not masked in X or Y, joined in
order as the plot's line joins them, the rows the analysis tools (and the Integrate gadget's New
Output) read. The Data Reader keeps the worksheet's row numbers.

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
  category-font dialog with Standard, Data, Column Labels, Column Header,
  Row Header, Long Name, Units, Comments and F(x)= family and size controls (GUI 3).
- **Data font defaults.** Optional `Worksheet.font_defaults.data` holds independent family
  and point-size overrides. An empty Data category is omitted. Resolution fills missing manual
  family and size from Data and then Standard, preserving other manual and conditional
  properties and keeping label overrides independent. One sparse
  setting reaches future columns and arbitrarily remote rows without per-cell records. Clone,
  duplicate and project/companion save carry it with the worksheet. The existing batch ReplaceSheet
  path starts with default formatting, matching its existing range-format reset policy. The grid
  cache includes defaults even when no manual ranges exist. Print/image and Excel reuse effective
  style resolution. `.opj` warns and requires its full `.sheaf` companion for these fonts.
  Excel writes current values and blank cells inside the used rectangle; it cannot retain Sheaf's
  future-cell inheritance, and import still does not read cell styles.
- **Label size defaults.** Optional `font_defaults.long_name_size_pt`, `units_size_pt` and
  `comments_size_pt` store independent sparse point sizes below manual label ranges. Old sheets
  retain their serialized shape. Each setting covers existing and future columns without
  per-column records, leaves data and other label categories independent,
  and uses the existing 4..96 pt bounds. The F4 dialog keeps category drafts and commits all
  changed categories in one Undo; Cancel/Escape discard unapplied drafts. These draft rules
  and retained manual-label precedence are Sheaf policy, not measured Native parity. Native
  category sizes/new columns and specific-over-bulk priority were measured (GUI3); family and
  manual-label precedence remain open. Print/image/grid use existing effective label resolution
  and derived geometry; empty printed labels remain under T27's existing omission policy.
  Project and OPJ-companion saves preserve the fields; unsupported native OPJ formatting warns.
  Excel writes effective sizes only for exported label rows, reports omitted category styles,
  and leaves data blanks unstyled when only label defaults exist. Future Excel label inheritance
  is not preserved.
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
  inheritance/reset rules are Sheaf policies.
- **Column Labels font defaults (T59, 2026-11-21).** The sparse
  `font_defaults.column_labels` entry supplies family and point size independently
  for Long Name, Units, Comments, formula and parameter label cells, including
  future columns. Each manual property wins over the corresponding specific
  category property, which wins over the bulk property. Starting from Automatic
  family or Default size inherits the next level. Project files and OPJ companions
  preserve the entry. Excel writes the resolved fonts for the three supported
  metadata rows and warns when styled label rows are omitted, including formula
  or parameter rows. Grid, print and image export use the worksheet style resolver.
  See GUI 3 for the Origin 2016 size probe and the documented family control.
- **Standard font (T59).** Sparse `font_defaults.standard` supplies the base family and
  point size for the worksheet. Each manual property wins over its category's setting,
  and each category inherits missing parts independently. The default size of 9 pt
  leaves the existing base row heights and widths intact. Grid, print and images use
  the Standard size to scale their base geometry. Excel exports effective cell fonts
  and reports omitted row and column heading fonts. Native projects and OPJ companions
  preserve the setting. See GUI 3 for the Origin 2016 size and family probes.
- **Formula label font (T59).** Sparse `formula_family` and `formula_size_pt`
  override Column Labels and Standard for F(x)= cells, including new columns.
  The fields do not change the stored formula, its evaluated values or the row
  caption. Grid, print, images and project saves use the existing label-font
  path. Excel reports when it omits the styled formula label row.
- **Data colors (T59).** Optional `data_colors.text_color` and `data_colors.fill`
  hold Worksheet Properties defaults for data cells. They fill missing manual
  properties, and conditional colors retain their priority over both. A manual
  transparent fill continues to clear a fill. New rows and columns inherit the
  settings without range records. The grid, print and images use the resolved
  colors. Native projects and OPJ companions preserve the settings. Excel
  writes effective colors for cell values and blanks inside the used rectangle
  and reports that future cells take Excel defaults.
- **Column Labels colors (T59).** Optional `column_label_colors.text_color`
  and `column_label_colors.fill` supply defaults for Long Name, Units, Comments,
  F(x)= and parameter cells, including labels in new columns. Manual label
  properties take precedence. A manual transparent fill uses the ordinary
  label background. The grid, print, worksheet images, project files and OPJ
  companions retain the colors. Excel writes the effective colors on selected
  metadata rows, including their blank cells, and reports omitted styles.
- **Individual label colors (T59).** `label_colors` holds optional text and
  fill overrides for Long Name, Units, Comments and F(x)=. Each property
  resolves from manual cell formatting, then the individual row, then
  Column Labels. New columns inherit the same defaults. Export and project
  persistence follow the Column Labels color rules.
- **Standard colors (T59).** Sparse `standard_colors` provides text color
  for data and column label cells and fill color for data cells. Data,
  Column Labels and individual label properties override Standard
  independently. Heading colors use their own palettes. Conditional and
  manual colors resolve before category defaults. Grid, print and worksheet images
  share the resolution rules. Excel writes a snapshot of the used data
  rectangle and selected label rows and reports omitted styles.
- **Heading colors (T59).** Optional `label_colors.column_header` and
  `label_colors.row_header` store text and fill independently of the heading
  fonts. The column fill also covers the select-all corner. Row colors cover
  numbers and label captions. The grid and print use these styles, worksheet
  images crop the headings, and Excel reports their omission.
- **Missing display (T59).** Optional `view.show_missing_as_blank` controls the
  numeric missing marker in the grid, print and worksheet images. With the
  field absent, the grid hides numeric markers and printed or exported images
  show them. The display option preserves stored values and literal text.
  Native projects and OPJ companions carry the worksheet view setting.
- **Worksheet size (T59).** Optional `view.allocated_rows` stores the grid
  capacity set in Worksheet Properties. Each column still owns its actual
  data length. With no explicit count, sheets use automatic spare rows.
  The grid includes any data extending beyond the allocation. Row and
  column reductions use the existing deletion paths.
  Project saves preserve the allocation. Optional `view.auto_add_rows`
  controls whether a larger viewport increases the allocation. New
  sheets start automatic, and older explicit counts stay fixed until
  Auto Add Rows is selected. Automatic viewport growth is a view
  adjustment and leaves the data and undo history intact.
- **Fit Sheet to Window (T59).** The worksheet stores the Miscellaneous
  switch and the last fitted body row height and column width. The grid
  gives displayed rows and columns equal shares of the space left by the
  headers and labels. Turning Fit off keeps those sizes. Subsequent
  explicit Size measurements replace them, and the worksheet image and
  PDF paths use the saved sizes.
- **Header measurements (T59).** Sparse `view.header_measurements`
  stores the worksheet's explicit column heading height and row heading
  width in grid percentages. Automatic measurements use the worksheet's
  font and label visibility. The stored measurements follow Standard
  font scaling and are independent of the View heading switches. The
  same group saves the top gap in points when nonzero. Its height is
  independent of the Standard font. Print and worksheet images use
  their own geometry.
- **Body measurements (T59).** Sparse `view.cell_measurements` stores
  explicit All Rows Height and All Columns Width percentages. Row height
  applies to data rows, including sparse future rows. An All Columns Width
  edit clears individual width settings and supplies the width of new
  columns. A later Standard point-size edit restores automatic body sizes.
  The grid, print and worksheet images use these settings.
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
- **Row Numbers, Uniform Random Numbers, Normal Random Numbers** (T63 group H, 2026-11-19, documented, **not
  measured**: Origin's COM class was unregistered). They have no window: they fill every selected column at once,
  one undo step ("Fill column"), from the Column menu or the column header's Fill Column with submenu.
  - Row Numbers writes 1, 2, 3, ... and the random items write uniform numbers on [0, 1) (as `rnd()`) or normal
    numbers with mean 0 and SD 1 (as `grnd()`). Each column gets its own random numbers.
  - They write every row of the sheet (its longest column), and 32 rows into a sheet without rows (Origin's new
    sheet has 32). Text cells and the column's formula go; the number format stays.
  - Not measured: Origin's row count for a partly filled sheet and for a selected cell range, and its generator.

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
  "units"}`). Since T303 the `.opj` reader and writer keep the first X and the step where Origin does (the
  column record's `0x1D2`/`0x1DA`, opj-format 4.3); only the interval's X long name and units are not in
  the `.opj` written (the save warning names them, the companion keeps them).
- **Analysis and every plot (T304)**: analysis tools take a column with an interval at its interval, not
  at the sheet's X column or the row number (Integrate of y = x on 0..10 with x0 = 0, inc = 0.5 gives 25);
  a plot whose Y column has an interval draws it there even when the plot names an X column (as plots
  made by analysis tools or Add Plot can), with the interval's title on the X axis. Origin writes such a
  column as belonging to no X column (T303).
- **From X Column** (T228, Sheaf's button, **not measured**; Origin's `vinc` / `vinc_check` X-Functions
  are for internal use): when the first selected column has an X column, Set Sampling Interval has a
  "From X Column" button. It checks the X values (up to the last number) the way `vinc` does: the mean
  step is (last - first) / (count - 1), and the column is evenly spaced when no step differs from it by
  more than 1e-6 of the step (`sheaf_core::sampling::check_increment`, Sheaf's tolerance). Evenly spaced,
  the button fills Initial X Value with the first value and X Increment with the mean step, and Units and
  Long Name with the X column's when it has them; otherwise the window says "X column A is not evenly
  spaced: a step differs from the mean step M by D" (or "needs at least two numbers and no gaps" for
  fewer than two numbers, a missing value or a zero step) and changes nothing. The X column stays.
- **Origin's `vinc` and `vinc_check`, measured (T371, Origin 2016, 2026-11-22, probes
  `re-workspace/scratch/t371`, local only).** `vinc` gives the mean step and the **sample** standard
  deviation of the steps (steps 1, 1.04, 0.96: mean 1, sd 0.04; steps 1, 2: mean 1.5, sd 0.7071). `vinc_check`
  (default tolerance 0.05) passes when the **largest** deviation of a step from the mean step is at most
  tolerance x |mean step|: steps 1, 1, 1, 1.045, 0.955 (sd 0.032) fail at 0.044 and pass at 0.046; steps 10,
  10.4, 9.6 and 0.1, 0.104, 0.096 pass at 0.05 and fail at 0.039 (relative). It then returns the mean step
  (negative for falling values); otherwise it fails and returns 0, also for a single value. A missing cell is
  read as about 0 (steps 1, -1, 3 for 0, 1, missing, 3). `check_increment` is the same test, with the sample
  standard deviation as `sd` and `ORIGIN_TOLERANCE` = 0.05; Sheaf refuses a missing value instead, and From X
  Column keeps the 1e-6 tolerance because the interval then replaces the X values exactly.

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

- **Insert** and **Add** (T212, Sheaf's rule, not measured here) make a sheet with columns A(X) and
  B(Y), as File > New > Worksheet does, so that it can be typed into at once (it used to have no
  columns, hence no cells). One undo step ("Insert sheet", "Add sheet").
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
- **Activate This Sheet in All Books in Folder** (T277, Origin 2027's "activate the same sheet in all
  workbooks of a folder"; the release note gives no more, so the rule is Sheaf's): in every workbook of
  the book's Explorer folder (not its subfolders) that has a sheet of the clicked sheet's name, in any
  case, that sheet becomes the active one. Books without such a sheet are left alone, and the status bar
  says how many changed and how many have no sheet of that name. Like clicking a tab, it is a view change,
  not an undo step.
- **Navigate...** (T93) opens "Navigate Worksheets - Book1" in Origin 2016: a table of the sheets (Index,
  Creation Index, Name with the short name, Columns, Size(KB), Comments, Type "Flat Sheet", Source), the
  active sheet selected, an Actions menu and Apply Reorder (greyed), OK and Close. Sheaf's window has Index,
  Name, Long Name, Columns, Rows and Comments. The Comments cell previews the first line and shows the
  full saved text on hover. Clicking an index or name picks a sheet, OK or a double click goes to it,
  and Close or Escape closes. Name and Comments is the editing route.
- **Actions (T180, not measured; Origin's help lists the items).** The Actions menu above the list and a
  row's right-click menu hold the same items and act on the selected sheets. A click picks one sheet,
  Ctrl+click adds or removes one (the last stays) and Shift+click selects the run from the last click in
  the shown order; a right-click on an unselected row picks it first. Each item is one undo step:
  - Activate (one sheet) goes to it, as OK does.
  - Duplicate puts a copy with data right after each sheet ("Duplicate sheets"; the copy is named
    `<name>_<n>` and becomes active). Duplicate without Data adds T90's copies after the last sheet.
  - Clear all Data empties the sheets' columns, keeping columns, labels and formulas ("Clear worksheet
    data").
  - Delete removes the sheets and their columns from plots ("Delete sheets"); it is greyed when every
    sheet is selected, since a workbook keeps one. The active sheet stays active when it is kept.
  - Move Before and Move to Behind move each selected sheet one place towards the first or the last
    sheet; a selected block moves together and stops at the ends. Move to End puts the selected sheets
    last in their order ("Move worksheets"); the moved sheets stay selected.
  - Set Current Order as Sheet Order is Apply Reorder.
  - Move to > New Book (T184) moves the sheets, in their order, to one new workbook with the old
    one's long name ("Move sheets to new book"); their columns keep their identities, so graphs of them
    still plot them. It is greyed when every sheet is selected. Duplicate to New Book copies the
    sheets with their data and new columns into one new workbook ("Duplicate sheets to new book").
  - Exclude from Plotting dialogs (T184) marks the sheets (`Worksheet::exclude_from_plotting`), ticked
    when all are marked; again clears it. Plot Setup and Layer Contents then do not list the sheets'
    columns; plots already made stay. The `.opj` written here does not hold the mark (its save warns).
  - Move to also lists the project's other workbooks (T191, Origin's Move to Selected Book): the
    sheets go after that workbook's last sheet, the first of them active; a name the workbook already
    has gets the next free number (`Sheet1` becomes `Sheet11`). Columns keep their identities, so
    graphs follow; greyed when every sheet is selected ("Move sheets to book").
  - Not done: Origin's Excel-export exclusion.
    [Origin Navigate Worksheets](https://docs.originlab.com/origin-help/nav-wks-dialog/)
- Clicking a Navigate Worksheets header in Sheaf previews its ascending order. Click it again for
  descending order. Index refers to the workbook order before applying. Apply Reorder commits the
  displayed permutation in one undo step and keeps the active worksheet. Equal values keep their
  workbook order. Finish any other open dialog before applying the reorder, since it may refer to
  the original sheet indices. Origin 2016's unsorted capture shows the disabled Apply Reorder
  button. The documentation describes sorting by headers and committing the current order.
  [Origin Navigate Worksheets](https://docs.originlab.com/origin-help/nav-wks-dialog/)

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
Filter, Categories and Page Break Preview Lines (Sparklines: T175, below). User parameter rows show whenever a column has one. The `.opj`
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

**Sparklines (T175, 2026-11-19, not measured).** Origin draws a sparkline, a small graph of a column,
in the Sparklines label row; its column header item Add or Update Sparklines... opens the `sparklines`
X-Function's options. Sheaf has no dialog and no graph object in the cell:

- Column header > **Add or Update Sparklines...** marks the selected columns (`Column::sparkline`) and
  turns on the sheet's Sparklines row (`SheetView::sparklines`), as one undo step ("Add or Update
  Sparklines").
- The **Sparklines** corner switch shows or hides the row ("Worksheet view"); a new sheet has it off.
  It sits after Sampling Interval.
- In the row, each marked column draws its finite values against their row number as a 1 px line in
  the text colour, from the first value at the left to the last at the right and from the smallest at
  the bottom to the largest at the top, inside a 3 px margin. Long columns are thinned to about one
  point per pixel. Unmarked columns' cells stay empty. The row is 32 px high (twice a text row), Sheaf's
  choice. The line follows the data as it changes ("Update" needs no action).
- The cells are read-only: editing them does nothing, and Cut, Copy, Paste and Clear on them only say
  that sparklines are drawn from the column's values. Printing and worksheet images leave the row out.
- `.opj` does not hold the marks; the save warning names the sheet when the row is on.

**Page Break Preview Lines (T185, 2026-11-19, not measured).** The corner menu's switch and the workbook
View menu's item (ticked when on) show dashed blue lines in the grid where the printed pages break: below
the last row of each page and right of the last column of each band of columns, as File > Print would cut
the sheet on the project's page setup (the same pagination, `SheetTable::page_breaks`, real font metrics);
nothing after the last row or column. Off on a new sheet, saved per sheet, one undo step ("Worksheet view").
The lines are on screen only. Origin's colour, line style and which setup it uses were not measured.

**Categories row (T187, 2026-11-20, not measured).** The corner menu's Categories switch shows a
read-only label row titled "Categories", last of the label rows: for a categorical column (Column > Set as
Categorical) its distinct non-empty cell texts in the order they first appear, joined by ", "; empty for
other columns. Off on a new sheet, saved per sheet, one undo step ("Worksheet view"). Typing or pasting into
it is refused with a status message (Copy works). Origin's row text and order (sorted or as set in the
category editor) were not measured.

**Graph sheets (T190, 2026-11-20, not measured).** A sheet tab's right-click Add Graph as Sheet... opens a
list of the project's graphs; OK adds a sheet after the last one, named after the graph (`Graph1`, then
`Graph12`, ... when the name is taken), that shows that graph in place of the grid, live and as its window
does (clicks, menus and edits act on the graph), one undo step ("Add Graph as Sheet"). The graph window stays
in the project (Origin embeds the graph; that was not measured, so Sheaf keeps both). Deleting the graph
leaves the sheet with a note. The sheet has no columns; it is saved in `.sheaf` (the sheet's `graph_sheet`,
following the graph's id on Append), dropped from workbook templates, and left out of an `.opj` with a
warning (as Notes sheets are).

**Graphs in cells (T194, 2026-11-20, not measured).** A data cell's right-click Insert Graph... opens the list
of the project's graphs; OK puts that graph in the cell, one undo step ("Insert Graph"). The cell shows a
picture of the graph fitted into it on white (drawn again whenever the project changes); a double click
opens the graph window instead of editing the cell. One graph per cell (inserting again replaces it); the
cell's Remove Graph takes it out ("Remove Graph"). The graph follows its column when columns get new ids
and goes with a deleted column; it is saved in `.sheaf` (the sheet's `cell_graphs`), dropped from workbook
templates, not written to `.opj`, and not printed or exported with the sheet. Rows inserted or deleted
above do not move it. The row number's Insert Graph... stays grey. Origin's sizing and embedding were not
measured.

**Sparklines in cells (T200, 2026-11-20, not measured).** A data cell's right-click Insert Sparklines...
lists the numeric columns of the cell's sheet (the cell's own column first chosen); OK makes the cell draw
that column's values as a small line on white, as the Sparklines row does (T175: finite values, first to
last, filling the cell), one undo step ("Insert Sparklines"). One sparkline per cell; Remove Sparkline
takes it out. It follows both columns when they get new ids and goes when either is deleted; saved in
`.sheaf` (the sheet's `cell_sparklines`), not written to `.opj`, not printed. Origin's dialog (it can make
several at once and pick rows) was not measured.

**Pictures in cells (T202, 2026-11-20, not measured).** A data cell's right-click Insert Images from Files...
opens the system file picker for a PNG file; the picture is kept in the project (the sheet's `cell_images`,
the file's bytes written as base64 text in `project.json`, so window files and templates carry it too) and
drawn fitted into the cell on white, one undo step ("Insert Image"). A file that is not a readable PNG is
refused with a status message. One picture per cell; Remove Image takes it out ("Remove Image"). It
follows its column when columns get new ids and goes with a deleted column; it is not written to `.opj`
and not printed. Only PNG (Origin also reads JPEG, BMP and others); Origin's sizing options were not
measured.

**Arrows in cells and the row number's Insert Graph (T205, 2026-11-20, not measured).** A data cell's
right-click Insert Arrow > Arrow draws a black arrow across the cell from left to right (head at the right
end); Double Arrow puts heads at both ends; one undo step ("Insert Arrow"). One arrow per cell (inserting
again replaces it); Remove Arrow takes it out. It follows its column like the other cell objects, is
saved in `.sheaf` (the sheet's `cell_arrows`), not written to `.opj` and not printed. A row number's
right-click Insert Graph... puts the picked graph in that row's first visible cell (as a data cell's Insert
Graph..., T194). Origin's arrow direction, style and row behaviour were not measured.

## 13j0. Cell links: Paste Link, Paste Link Transpose, Remove Links (T211, 2026-11-20, not measured)

Model and reasons: [ADR 0014](../adr/0014-cell-links.md). Origin's link storage and paste rules were not
measured. In a workbook, Edit > Paste Link (Ctrl+Alt+V) and Paste Link Transpose (also on a data cell's
right-click menu) link cells to the block last copied in Sheaf (Copy; a Cut leaves nothing to link to):
from the clicked cell down and right, cell for cell, or with rows and columns swapped for Transpose. A
linked cell takes its source's value at once and whenever the source changes, inside the same undo step
(numbers exactly, text as text, a missing value as an empty cell); plots, formulas and analysis see the
value. Cells past the sheet's last column and cells of formula columns are left out (the status line says
how many). Remove Links (Edit menu, data cell, column header) drops the links of the selected cells and
keeps their values; typing into a linked cell replaces its value and ends its link. A linked cell shows a
small blue corner at its bottom left. Links follow their columns through re-ids and go with a deleted
column; a sheet copied on its own (Duplicate) loses them. They are saved in `.sheaf` (the sheet's
`cell_links`); `.opj` and other exports write the values. The graph window's Paste Link stays grey.

## 13j. Find in Project (T95, measured 2026-11-16, Origin 2016)

Edit > Find in Project... (F3; "Find in Project" on a graph's Edit menu) opens Origin's **Find** window
(screenshots `re-workspace/scratch/t95`, local only): Current Path, Find What, Within (Entire Project), Case
Sensitive, Match Whole Word, Allow Wildcards (* and ?), Types to be Included > Metadata: Folder, Graph,
Book, Sheet, Column, Notes and Select..., then Data in Cells (1st Matched in Column), Text Objects on
Graphs, Notes Content; a result list (No., Range, Element, Attribute, Folder) and "Found: N". Defaults
(Select Metadata): Folder with Name and Comments; Graph, Book, Sheet and Notes with Long Name, Short
Name and Comments; Column with Long Name, Short Name, Comments, Parameters and User Parameters, **not**
Units, F(x), Categories, Sampling
Intervals, Filters, Info Variables; Data in Cells off, Text Objects on Graphs on. Origin keeps the settings
between sessions.

Measured hits (book with B "Current density", mA; a line graph of B): "density" gave
`[Graph1]Layer1!YL` Graph / Axis Title, `[Graph1]Layer1!Legend` Graph / Legend and
`[Book1]Sheet1!B"Current density"` Column / Long Name (text objects as drawn); "Graph" gave `[Graph1]`
Graph / Short Name; "30" with Data in Cells gave column B once, attribute Data Cells.

Sheaf (`crate::project_find` in the app, window `dialogs/find_project.rs`) searches supported window
types and their contents. Select... shows the sheet's
Comments switch and the column's Units and F(x) switches; axis titles are searched as drawn
(`sheaf_plot::axis_title`), legends with `%(n)`
replaced by the plots' names, and text labels by their text. Clicking a hit brings its window forward (and
its sheet, with the column selected); the window stays open and keeps its settings for the session.
Double-clicking a hit in Origin was not measured.

The archived Origin 2016 Select Metadata capture has Sheet > Comments checked by default, as also
documented under Sheet metadata in [Finding in Origin Project](https://docs.originlab.com/origin-help/finding-project/).
Sheaf searches the full saved worksheet comment by default. Select... can turn Comments off while
retaining sheet name matches. Each matching comment gives one `[Book]Sheet` row with Element **Sheet**
and Attribute **Comments**, using the existing sheet navigation. This result row and click behavior
follow Sheaf's existing sheet hits. They were not measured separately in Origin 2016.

Within defaults to **Entire Project**, as seen in the Origin 2016 capture. **Active Folder**
restricts the search to books and graphs in the selected Project Explorer folder.
**Active Folder and Subfolders** includes windows below that folder. These two scopes
follow the [Origin Find documentation](https://docs.originlab.com/origin-help/finding-project/#within).
Current Path shows the Explorer folder. The scope choice stays selected for the session,
and each Find uses the folder and membership at the time of the search. Sheaf uses a window's
own folder for its results and treats windows without explicit membership as root windows.
Clicking a hit that View Windows hides first activates the window's folder so the result
can be shown.

The archived Select Metadata capture shows Folder Name and Comments checked. Sheaf includes
folder names and full comments by default within the selected scope. Clear Folder to omit both,
or clear Folder Comments under Select... to keep only name matches. A folder hit shows its path
and activates that folder when clicked. Notes and folder info variables remain open.

The archived Origin 2016 results show Folder after Attribute, with the root's short name
`UNTITLED` in graph, legend, column and data cell rows. Sheaf fills the column with the
containing folder's name for each window and its contents, using the project root for
windows without explicit subfolder membership. For a folder metadata hit, Sheaf displays
that folder's own name and uses its full path in Range. The archived results measured
root window hits. The nested-folder and folder metadata display rules follow Sheaf's
Project Explorer membership.

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
in the project's folder (Sheaf's choice). The raster-format follow-up below adds JPEG, TIFF, BMP, GIF,
TGA, PCX and PSD. Section 13k.2 adds EPS and EMF.

### 13k.1 Additional worksheet raster formats (2026-11-17)

The existing 2016 expWks list above and the [official expWks reference](https://docs.originlab.com/x-function/ref/expwks/)
support the added types. File filters and extension dispatch now expose JPG/JPEG,
TIF/TIFF, BMP, GIF, TGA, PCX and PSD. The new sheet_image_bytes facade routes the
unchanged worksheet SVG through existing graph raster encoders; PNG/PDF/SVG keep
their prior functions. No new Native measurement, COM retry or codec/layout/font
change is claimed. SVG remains an existing Sheaf addition to the measured list.

Defaults are explicit Sheaf choices: fixed300dpi in the App, graph RasterOptions
defaults (24-bit colour, JPEG quality90, TIFF LZW), opaque formats flattened onto
white, palette GIF and flattened RGB PSD without editable layers. Resolution is
written where the existing encoder supports it. The existing400M-pixel cap applies
before allocation. Side caps are30,000 for PSD,65,535 for JPEG/GIF/TGA and a common
conservative65,534 for PCX (its even padded plane-row length must fit u16). Invalid
resolution or unsupported formats return errors before a target file is written.
App extensions are case-insensitive, aliases retain the requested path, and a
missing extension still adds .png. Unknown vector formats stay unsupported.

Original worksheet layout, labels/styles, file-name/folder default and overwrite
behavior remain. The existing whole-sheet picture still includes retained source
values outside a column's Begin/End display range; whether Origin expWks renders
those values is unmeasured. This format slice does not change that rendering policy.
### 13k.2 EPS and EMF worksheet export (2026-11-21)

The worksheet image save dialog also offers Encapsulated PostScript and Enhanced
Metafile. Both writers receive the same worksheet scene used by SVG, with its
labels, cells and original size in points. EPS uses the existing PostScript font
mapping, which substitutes unsupported characters. EMF stores text in UTF-16.
The list and layout of the original images were measured in 13k. This follow-up
implements the two file types documented by [Origin's expWks reference](https://docs.originlab.com/x-function/ref/expwks/).

Remaining export types are AI, CGM, DXF and WMF. Multi-sheet export and size,
depth, quality, compression and overwrite controls are also open.

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

## 13n. Pivot Table (T124, 2026-11-17; measured against Origin 2016 in T276, 2026-11-21)

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

**Measured in Origin 2016 (T276, 2026-11-21)**, `wpivot` on Region (A: East, West, East, North, West,
East, North, East), Product (B: pen, pen, ink, pen, ink, pad, ink, pen) and Sales (C: 5, 3, 2, 7, 4, 1, 6,
8), read cell by cell (`re-workspace/scratch/pivot-measure/`); Sheaf now matches every item:

- The sheet is `Pivot1`, `Pivot2`, ... of the input book; counts, sums, means, maxima and missing
  combinations (0, or `--` with `zero:=0`) as above.
- **Names.** The value columns' method label is `Count`, or `Sum of C`, `Mean of C`, `Max of C`,
  `Min of C` (the data column's name). pos 1 (default): label column Long Name = the row source's name;
  value columns Long Name = the method label, Comments = the key. pos 0: the label column's name moves to
  Comments (Long Name empty); value columns Long Name = the key, Comments = the method label. pos 2:
  label column as pos 0; value columns Long Name = the data column's own Long Name + `_` + the key
  (`_ink` when it has none), Comments = the method label.
- **Totals.** The row-total column is named `Total_<row source>` (`Total_A`) in the key's place, with
  the method label in the other; the column-total row's label is `Total_<column source>` (`Total_B`).
  Every total applies the method to all records of its row, column or the table: a Mean table's North
  total is the mean of North's records (6.5), not the sum of its cells, and the grand total is the mean
  of all values (4.5); for Count and Sum this is the same as adding the cells.
- **Sorting by totals** uses those totals: a Max table puts East (8) above North (7).
- **Normalize** (percent, with totals): each column, the row-total column included, is divided by its
  own total (East's total 16 of 36 is 44.44 %), and the column-total row is 100.

## 13o. User workbook templates: Save Template As on a workbook (T160, 2026-11-19, not measured)

Origin saves a workbook as a template (`.otw`/`.otwu`) from File > Save Template As..., and File > New >
From Template lists workbook templates by file name (T103 measured the submenu: `Assays.ogw` and the like).
Nothing about what an Origin workbook template keeps was measured (Origin COM was unavailable); Sheaf's
choices below are **unmeasured**, and Origin's own template files are never read or written.

- **Saving** (File > Save Template As... on a workbook; greyed without a template folder): the same name
  prompt as a graph template (graphing 24), filled with the book's short name; OK writes
  `<name>.sheafbooktemplate` in the same templates folder. Graph and workbook templates have names of
  their own: a workbook template never replaces a graph template of the same name, and the reverse. The
  file is a `.sheaf` archive with the marker `"template": "workbook"` holding one workbook without values.
- **What is kept** (`sheaf_core::BookTemplate::from_workbook`): the book's long name and every sheet (name,
  long name, comments, tab label, view switches, frozen panes, category fonts, row filter, conditional
  formats, cell formats), and per column the short and long names, units, comments, user parameter rows,
  designation, number format, storage kind (numeric, text, text & numeric), width, hidden and categorical
  flags, sampling interval and the Set Column Values formula with its recalc mode and rows. **Dropped**:
  every value, masks, cell notes, Set As Begin/End ranges, analysis marks and fit-result flags, the import
  source and the comparison source of a report sheet.
- **New from a template** (File > New > From Template, after the graph templates and a separator; More...
  picks either kind of file): a new workbook built like the template with fresh ids and a free `Book<n>`
  name, shown in front, one undo step ("New workbook from template"). Formula columns stay empty until
  their inputs change.
- **Managing**: Plot > User Templates > Manage... lists the workbook templates under their own heading,
  with Rename (refused onto another workbook template's name) and Delete.
- **Analysis templates** (File > Save Workbook As Analysis Template... on a workbook, T163, 2026-11-19,
  **unmeasured**: OriginLab's Analysis Template help pages returned 404 on 2026-11-19 and Origin COM was
  unavailable): the same prompt, titled "Save As Analysis Template", writes a workbook template as above
  that also keeps the book's recalculating analysis runs (Recalculate Auto or Manual, `AnalysisLink`)
  whose input, result and target columns are all in the book (`BookTemplate::analysis_from_workbook`);
  runs that read or write another book are left out. Values are cleared as for a workbook template; the
  result columns keep their Auto/Manual mark. It is listed with the workbook templates. A new book from
  it gets the runs with its own column ids: data put into an input column runs an Auto run at once (in
  the same undo step, results written into the template's result columns) and marks a Manual run
  pending. Origin's `.ogw`/`.ogwu` files are never read or written.
- Not done: Origin's analysis-template options (unknown), CLI and MCP access, Origin `.otw` files.
- Tests: `crates/core/src/book_template.rs`, `crates/io/src/user_templates/tests.rs`,
  `crates/app/src/app/ui_tests/user_templates.rs`.

## 13p. File > Batch Processing... (T165, 2026-11-19, not measured)

Origin's Batch Processing (`batchProcess`) runs an analysis template on many data files. Its dialog and
options were not measured (Origin COM unavailable, OriginLab's help pages for it not read); Sheaf's
window is its own, **unmeasured**:

- File > Batch Processing... (workbook and graph windows; greyed without a template folder) opens
  "Batch Processing": a Template list (the saved workbook templates, T160/T163), the Data files list with
  Add Files... (system picker, several at once; a file already listed is not added twice), Remove and
  Clear, OK (enabled with a template and at least one file) and Cancel.
- OK reads each file with the automatic importer and its default options (as File > Import) and makes
  one workbook per readable file from the template (`Project::new_workbook_from_template_with_data`):
  a free `Book<n>` name, the long name the file's name, the file's first sheet filling, in order, the
  data columns of the book's first sheet (columns that are neither an analysis run's results nor
  formula columns), each taking the file's values and the long name, units and comments the file gives
  while the template's designation and format stay; file columns beyond them are left out. Then the
  book's analysis runs run on the new data (Auto and Manual alike) and formula columns recalculate.
- All books are one undo step ("Batch processing"); the last book comes to the front. Files that cannot
  be read are skipped and named in the status line with any analysis problem; when no file can be read,
  the window stays open with the error.
- Summary (T233, **unmeasured**): Summary Sheet lists "(none)" and the sheets of the chosen template
  (it goes back to none when another template does not have the sheet); Dataset Identifier (default
  "Dataset"; empty is "Dataset") and Delete Intermediate Books are enabled with a sheet. With a sheet,
  after the books are made (formula columns brought up to date) a new workbook, long name "Batch
  Summary", one sheet "Summary", holds for each book in file order every row of its sheet of that
  name (as many rows as its longest column), each row led by the file name in a first text column
  (designation X) named by the identifier. The other columns follow the template sheet: long name
  (the short name when it has none), units, comments and designation; values only (no formulas), text
  cells kept. With Delete Intermediate Books the books made are deleted with their analysis runs once
  the summary is made. All of it is the one undo step; the summary comes to the front and the status
  line names it.
- Without a window (T235): `sheaf batch FILES... -t TEMPLATE.sheafbooktemplate [-o project]
  [--append] [--summary SHEET [--identifier NAME] [--delete-books] [--summary-csv FILE]]` does the same
  run (`Project::batch_process`, shared with the window), reading the files with the usual read
  options; an unreadable file is skipped with a warning, a summary sheet the template lacks is an
  error before anything is written.
- Not done: import filters saved in the template, the graph-window variant that runs a graph template,
  Origin's own summary layout and identifier choices, Minimize/Hide for intermediate books. MCP:
  `batch_process` (T238, mcp.md).
- Tests: `crates/core/src/book_template.rs`, `crates/app/src/app/ui_tests/user_templates.rs`.

## 13q. Add Column to All Sheets (T359, 2026-11-22, not measured)

Origin 2026b can add or insert a matching column with its formula in several related sheets of a workbook, folder
or project; Origin 2016 cannot, so this is Sheaf's own. Column > Add Column to All Sheets... asks for a Long Name,
Units, an optional Set Column Values formula with its Recalculate mode (None, Auto, Manual) and the sheets: every
sheet of this workbook (default) or every sheet of every workbook. OK adds one Y column at the end of each of those
sheets (`Project::add_column_across_sheets`, `sheaf_core::across`); the formula runs in each sheet against that
sheet's own columns (`col(B)*2` reads each sheet's B). A formula that fails in any sheet adds nothing and the window
names the sheet. One undo step ("Add column to all sheets").

T412: the sheets can also be "every sheet of every workbook in this folder": the workbooks of the project folder
holding this workbook (the first subfolder that lists it, else the root, which holds every workbook) and of its
subfolders (`AcrossScope::Folder`, `Project::folder_of`). After Column (a short name, letter case ignored, else a long
name) puts the new column right after that column in each sheet that has one; other sheets get it at the end.

T418: "Only the ticked sheets" lists every sheet of every workbook ("Book1 / Sheet1"), starting with this workbook's
sheets ticked; OK adds the column to the ticked ones only (`AcrossScope::Sheets`), in book and sheet order. With none
ticked the window says there are no sheets to add the column to.

## 13r. Column > Flash Fill (T430, 2026-11-24; Origin 2026's Flash Fill and Smart Formula, Sheaf's own, not measured)

Origin 2026 infers a text rule from an example, previews it, and fills static values or keeps a calculation. Origin
2016 has no Flash Fill.

- **Examples and sources:** the column's non-empty cells are the examples; the sources are the columns to its left
  that hold any text or number, read as text: a number as its digits, a date or time as the sheet shows it in its
  column's format (T493; before, a date was read as its day number).
- **Rules:** a rule joins pieces: fixed text, a whole source cell, a word of one (a run of letters and digits,
  counted from the start or the end) or its first or last one to four characters (T468: `Jo` of `John`, `17` of
  `2017`, written `left(token(...), n)` and `right(token(...), n)`), each as is, upper case or lower case. One rule
  must build every example. The search tries longer pieces first, then prefers a cell to a word, a word to an
  initial and an initial to fixed text, and a word counted from the nearer end (from the start when equal). So
  `John Smith` -> `Smith, J.` gives `token(col(A)$, -1) + ", " + left(token(col(A)$, 1), 1) + "."` and
  `Mary Ann Lee` gives `Lee, M.`; a second example settles a choice (`a b` -> `b` with `c d e` -> `d` is word 2).
  The search gives up after 2000 steps; then, as when nothing fits, the message asks for another example.
- **The window "Flash Fill":** the rule as a formula ("cannot be written as a formula" when its fixed text holds a
  `"`), and a preview of the first 8 rows (sources | result, the examples marked). Fill Values writes the results,
  the examples as typed; a row with no source text stays empty. A number or date column stays one when every other
  result reads as a number (a date or time in a date column) as if typed into it (T493: `2017` from `run 2017 a`
  stays a number, `1999-12-31` stays a date); a Text & Numeric column takes each result as typed; otherwise the
  column becomes text. Keep Formula sets the formula as the
  column's formula, recalculated automatically (Set Column Values' Auto). Either is one undo step ("Flash Fill");
  Cancel and Esc change nothing.
- **Not done:** arithmetic on numbers and dates (results are built from text pieces only), substrings that are not
  the first or last characters of words, conditional rules, suggestions while typing, picking the source columns, and
  a number result in Keep Formula (the formula gives text).

## 13s. View > Project Browser (T433, 2026-11-24; Origin 2025b's Project Browser, Sheaf's own, not measured)

Origin 2025b added a Project Browser that searches a project-wide list of sheets, columns, graphs and folders,
previews matches and edits the metadata of several at once. Origin 2016 has none.

- **The list:** every workbook, then its sheets, each followed by its columns, in project order, then every graph.
  Columns: a tick, Kind, Name (short name), Where (`[Book1]` for a sheet, `[Book1]Sheet1` for a column), Long
  Name, Units, Comments (first line). At most 500 rows are drawn; the count above the list says how many match.
- **Search and Show:** the search (ignoring case, as you type) keeps the rows where any of Name, Where, Long Name,
  Units or Comments holds it (and Folder, T486); Show keeps one kind (All, Book, Sheet, Column, Graph, Matrix, Matrix Sheet). Tick All Shown ticks the rows
  the list shows; Clear Ticks unticks every row.
- **Preview and Go To:** clicking a name makes it the previewed item: a column's row count and first 5 values, a
  sheet's columns and rows, a book's sheets, a graph's layers and plots. Go To, or a double click on a name, opens
  the item as a Find in Project hit does (its folder, window, sheet and column); the browser stays open.
- **Set:** Set [Long Name | Units | Comments] to [text], Apply to n Ticked: n counts the ticked items that have the
  field (columns have all three, sheets a Long Name and Comments, books and graphs a Long Name). One undo step,
  "Set Units" etc.
- **Sorting (T466):** a click on a column header sorts the list by that column, ignoring case and keeping project
  order among equal entries (a triangle marks it); a second click reverses it.
- **Editing in place (T466):** a double click on a Long Name, Units or Comments cell of an item that has the field
  opens a text box with the whole text; Enter writes it (one undo step, "Set Units" etc.), leaving the box otherwise
  changes nothing.
- **Folders and matrices (T486, 2026-11-26):** a Folder column after Where gives the project folder of the item's
  window (`Raw/Day 2`; empty in the root folder), searched like the others and sortable. Every matrix book follows the
  graphs, each followed by its sheets (`[MBook1]` in Where); Show has Matrix and Matrix Sheet. A matrix book has a Long
  Name; a matrix sheet's Long Name, Units and Comments are those of its values (the Z labels), so they are set and
  edited like a column's. The preview says a matrix book's sheets and a sheet's rows x columns; Go To opens the book.
- **Not done:** other metadata, and a picture preview of a graph.

## 13t. File > Import > Auto Refresh (T442, 2026-11-25; Origin 2019b Data Connectors / 2026 periodic refresh, Sheaf's own, not measured)

Origin keeps a linked source in a workbook and reloads it on a configurable, pausable timer. Origin 2016 has only
Re-Import (13l, T98). Sheaf adds a timer to Re-Import Directly.

- **The window "Auto Refresh"** (File > Import, on an imported workbook, below Re-Import...): the source file; Check
  every [N] seconds (5 to 86400; 60 for a book without a refresh); Only when the file has changed (on); Paused (off);
  OK, Stop Refreshing (when the book refreshes), Cancel. OK and Stop Refreshing are one undo step ("Auto Refresh").
  A book that was not imported is refused (the command is greyed, as Re-Import is).
- **The timer:** each refreshing, unpaused book is checked N seconds after the setting is made or the window opens,
  then every N seconds. With "only when changed" the file is read again only when its modification time differs
  from the last read; otherwise every time. A read is Re-Import Directly (values replaced, labels kept, graphs
  follow) and one "Re-import" undo step; the status line says what was read.
- **Failure:** a file that cannot be read pauses the book's refresh (an "Auto Refresh" step) with a message, so a
  missing file is not tried again until the user unpauses it.
- **Saved:** the setting is in the workbook (`Workbook::refresh`) in the `.sheaf` file; the timer runs while the
  project is open.

## 13u. Cell formulas and the Formula Bar (T448, 2026-11-25; Origin 2019 / 2021, Sheaf's own rules, not measured)

Origin 2019 calculates individual cells alongside column formulas and Origin 2021 edits their expressions in a
Formula Bar. Origin 2016 has neither.

- **Entering:** a cell typed (or set from the Formula Bar) as `=` and an expression keeps a cell formula; the
  expression is checked first, and one that cannot be read is refused with a message (the cell keeps what it had).
  It is one undo step, "Cell Formula". A cell link on the cell ends (T211).
- **The expression:** a column formula (expression-language.md) in which a spreadsheet reference names a cell:
  `B3` is column B (short name, capital letters) row 3, `A1:A5` rows 1 to 5 of A, `Sheet2!B3` a cell of another
  sheet of the workbook. A reference needs the sheet to have that column, is not inside quotes and is not followed by
  `(` (so `LOG10(x)` stays a call). `sum(...)` is the total of its range, as in a spreadsheet (`total`; in column
  formulas `sum` is Origin's running sum). The value is the first row's result of the formula read in the cell's
  sheet; a text result makes a text cell.
- **Recalculation:** inside every edit's undo step, right after the cell links, every cell formula of the project is
  worked out again; formulas that read formula cells settle in up to ten passes. A formula that fails leaves its cell
  empty.
- **Typing over:** typing a value into a formula cell drops the formula; the value typed stays.
- **`$` and Fill Down (T455):** `$A$1`, `A$1` and `$A1` name the same cell as `A1` (a `$` after `)`, before
  anything but a capital letter or digit, or in quotes is not a marker, so `col(A)$` and `$(...)` keep their
  meaning). The Formula Bar has Fill Down when the current cell has a formula: the formula goes to every cell below
  it down to the sheet's last row with data, each copy's row numbers moved by its distance except those with `$`
  before the row (`A1*$A$1` in row 1 becomes `A3*$A$1` in row 3); a reference to a column the sheet does not have
  is left as typed. One undo step, "Fill Down".
- **The Formula Bar:** View > Formula Bar (on a workbook) shows or hides it for every workbook; it is off to start
  with (Sheaf's choice, so the grid keeps its place). It is a strip above each workbook's grid (below its sticky
  notes): the current cell's name (`B1`, when exactly one cell is selected), `fx`, and a text box with `=` and the
  cell's formula, or the cell as shown. Enter writes the text as if typed into the cell; leaving the box otherwise,
  or Esc, changes nothing.
- **Saved:** the formulas are on the sheet (`Worksheet::cell_formulas`) in the `.sheaf` file.
- **Moving rows (T462):** inserting, deleting and sorting rows move a formula cell with its row (as cell notes move);
  a deleted row's formula goes with it.
- **Pasted formulas (T492, as a spreadsheet takes pasted text):** a pasted cell whose text starts with `=` becomes a
  cell formula when it can be read, with the spreadsheet names that differ replaced (`AVERAGE(` is `mean(`, `PI()` is
  `pi`; SUM, MIN, MAX, SQRT ... are the same, any case); one that cannot be read is pasted as its text, as if typed.
  The column's type is decided by the other pasted cells. Every pasted cell loses the formula it had, as a cell typed
  over does; all formulas are worked out once after the paste, so a pasted formula reads cells pasted with it. The
  status line counts the cell formulas. Pasting takes the text as it is: references are not moved for where it lands.
- **References follow rows (T491, as a spreadsheet does):** inserting or deleting rows of some columns (whole rows or
  cells shifted) rewrites every formula of the sheet whose references name those columns of this sheet (plain `B3`
  or `ThisSheet!B3`): a reference at or below the insertion moves down by the rows inserted, one below deleted rows
  moves up; a range (`A2:A5`) grows when rows are inserted after its first row and up to its last, and shrinks by its
  deleted rows; a reference to a deleted cell, or a range all of whose rows were deleted, becomes `#REF!`, so the
  formula no longer runs and its cell is empty. `$` keeps its place in the text but does not hold the reference (it
  only matters in Fill Down). Sorting rewrites no reference.
- **Not done:** references from other sheets that follow this sheet's rows, references that follow moved columns,
  filling right or into a chosen range, A1 references to other workbooks, a marker on formula cells, and cell
  formulas in `.opj` files.

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
