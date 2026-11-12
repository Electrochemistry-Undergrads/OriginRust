# CH Instruments text exports

**Summary.** CH Instruments (CHI) potentiostat software saves a measurement as a plain text file: a
block of header lines that describe the run, then a column-header line, then numeric rows. This
document says how the importer recognises such a file, which header facts it keeps, how it turns the
rows into worksheet columns, and how a cyclic voltammetry (CV) run with several cycles is split into
separate scans. The scan-splitting rule and the clipping check were taken from the owner's own
Python workflow (his `cv_overlay.py`), so results match the spreadsheets he already has.

The binary `.bin` files of the same program are read by `chi_bin.rs` and described in
`chi-binary-format.md`; they give the same workbooks as the text files.

Sources: the owner's script (observed), public knowledge of the CHI export layout (observed in
many published data sets), and inference where marked. **No real CHI file has been checked yet in
this project**; see "Unknowns".

## 1. File layout

```
Sept. 12, 2026   14:03:22            <- line 1: date and time of the run
Cyclic Voltammetry                    <- line 2: technique name
File: C:\data\run1.bin                <- source data file
Data Source: Experiment
Instrument Model:  CHI627E
Header:                               <- free text the operator typed (may be empty)
Note:

Init E (V) = 1.2                      <- parameter lines: "name = value"
High E (V) = 1.2
Low E (V) = 0
Init P/N = N
Scan Rate (V/s) = 0.001
Segment = 6
Sample Interval (V) = 0.001
Quiet Time (sec) = 2
Sensitivity (A/V) = 1e-3

Results / peak list lines (optional, technique dependent)

Potential/V, Current/A                <- column header line
                                      <- (often one blank line)
1.200, 1.234e-5                       <- data rows
1.199, 1.228e-5
```

Facts the importer relies on:

- Encoding is ASCII or Windows-1252; line ends are CRLF. Read as UTF-8 and fall back to
  Windows-1252 for bytes that are not valid UTF-8.
- **Parameter lines** have the form `name = value`. Everything before the first ` = ` is the name;
  leading and trailing spaces are trimmed from both parts.
- **The column header line** is the last line before the data whose fields (split on comma or tab)
  each have the form `Quantity/Unit` and whose first field is one of the known X quantities below.
  Matching is case-insensitive. Each field gives one column: the text before the last `/` is the
  long name, the text after it is the unit.
- **Data rows** follow the header line. Fields are separated by commas and/or tabs, with optional
  spaces. A row is kept when it has at least two fields and all fields parse as numbers; blank
  lines are skipped. Reading stops at the end of the file. A non-numeric line after the data began
  starts a new block only if it is another column header line (some techniques write several
  segments each with its own header); otherwise it is ignored.

Known first-column quantities and the technique they imply when line 2 is missing or unknown:

| First column | Typical technique (line 2) | Meaning of rows |
|---|---|---|
| `Potential/V` | Cyclic Voltammetry, Linear Sweep Voltammetry, Differential Pulse Voltammetry, Square Wave Voltammetry | current vs. potential |
| `Time/sec` (also `Time/s`) | Amperometric i-t Curve, Chronoamperometry, Chronocoulometry, Open Circuit Potential - Time, Bulk Electrolysis | signal vs. time |
| `Freq/Hz` | A.C. Impedance | impedance vs. frequency |

Other common column names: `Current/A`, `Charge/C`, `Potential/V` (as a Y column in OCP files),
`Z'/ohm`, `Z"/ohm`, `Z/ohm`, `Phase/deg`.

A file is treated as a CHI export when a column header line as defined above is found within the
first 100000 lines. (The first version said 200; a real export of a CV with 3177 segments lists the
peaks of every segment before the column header, which then sits at line 6906.)

## 2. What the importer produces

One workbook per file. The workbook long name is the file name without extension. All parameter
lines are kept as (name, value) pairs in file order and returned with the import result; the
technique name (line 2), date line and instrument model are included as `Technique`, `Date` and
`Instrument Model`.

### 2.1 Techniques other than multi-cycle CV

One sheet named `Data`. Column 1 has designation X; every other column has designation Y. Long
names and units come from the column header line (`Potential/V` gives long name `Potential`, unit
`V`; `Time/sec` gives long name `Time`, unit `s`).

For files whose first column is time and that have a `Current/A` column, the importer can add a
charge column (option `add_charge`, default on): long name `Charge`, unit `C`, the running
trapezoidal integral of current over time starting at 0. This is the "integration of charge" the
owner does for chronoamperometry.

### 2.2 Cyclic voltammetry: splitting into scans

Applies when the first column is `Potential/V`, the file has a `Current/A` column, the technique
line contains "Cyclic Voltammetry" (or is missing), and option `split_scans` is on (default).

Definitions. A **segment** is a run of rows over which the potential moves in one direction. A
**scan** (the owner also calls it a cycle) is two consecutive segments: out and back.

Algorithm, on the potential values `E[0..n)`:

1. Direction of each step: `d[k] = sign(E[k+1] - E[k])` for `k` in `0..n-1`. A zero step takes the
   direction of the previous non-zero step; zero steps before the first non-zero step take the
   direction of the first non-zero step. If every step is zero, there is one scan holding all rows.
2. A **vertex** is every index `k+1` where `d[k+1] != d[k]`, i.e. the row at which the sweep
   reverses. Row `k+1` is the extreme point of the potential.
3. Take every second vertex, starting with the second one (vertices number 2, 4, 6, ...). These rows
   are the cut points. Scans are the half-open row ranges between consecutive cut points, with row 0
   as the first start and `n` as the last end. So the vertex row that closes a scan is the **first
   row of the next scan** and is not repeated.
4. Drop any range with 2 rows or fewer.

Check value from the owner's reference file (`CV1mVlmo.txt`, 7000 rows, start at the high vertex):
scans 1, 2, 3 are rows 1-2200, 2201-4600 and 4601-7000 (1-based, inclusive). The first scan is
shorter because the run started part-way through the window.

Sheet layout for a split CV file: one sheet named `Scans`. For scan number `k` (1-based) the
sheet gets, in this order:

| Column | Designation | Long name | Unit | Comments |
|---|---|---|---|---|
| potential | X | `Potential` | `V` | `Scan k` |
| current | Y | `Current` | `A` | `Scan k` |
| current in mA (only when option `add_milliamp` is on, the default) | Y | `Current` | `mA` | `Scan k` |

The mA column is the A column times 1000 and records the formula text `col(<short name of the A
column>)*1000`. The comment `Scan k` is what the legend of an overlay plot shows.

With `split_scans` off, the file is imported as in 2.1 (one X column and one Y column per data
column), plus the mA column when `add_milliamp` is on.

### 2.3 Current-range clipping warning

When a potentiostat's current range is exceeded the recorded current sits at a constant value. The
importer reports this as a warning (it does not change the data). For each scan (or for the whole
current column when not split), separately for positive and negative current: let `m` be the
largest magnitude; if 20 or more consecutive rows have a value within a relative tolerance of 1e-9
of `m`, add a warning naming the scan, the pinned value in mA, the number of rows, and the potential
(or time) range of the run. A real peak top repeats at most about 5 rows at CHI's 4 significant
digits, so it does not trigger the warning.

## 3. Options

| Option | Meaning | Default |
|---|---|---|
| `split_scans` | Split a CV file into scans as in 2.2 | on |
| `add_milliamp` | Add a current column in mA next to each current column in A | on |
| `add_charge` | For time-based files with current, add the running charge | on |
| `invert_current` | Multiply current by -1 (for files recorded with the cathodic-positive convention) | off |

## 4. Unknowns

- The exact header text of each technique in the owner's CHI software version, and whether his i-t
  files use `Time/sec` or `Time/s`. The importer accepts both. Needs real files from the owner.
- Whether his files contain several data blocks (for example multi-segment chronoamperometry with a
  header per step). The importer handles repeated header lines by appending rows when the header is
  identical, and by starting a new sheet when it differs.
