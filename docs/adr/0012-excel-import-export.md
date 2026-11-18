# ADR 0012: Excel import and export on `calamine` and `rust_xlsxwriter`

Status: accepted (2026-11-14). Task T30.

## Context

The owner keeps much of their lab data in Excel workbooks (cyclic voltammetry overlays, ICP-OES
results). Sheaf has to read `.xls`, `.xlsx` and `.xlsm` files and write `.xlsx`, without Excel being
installed and without COM. Three facts decide how:

- The lock file has `zip` (used by the project file and the `.opju` reader) but no spreadsheet code.
  An `.xlsx` is a zip of XML parts, so a reader for it could be written on `zip` and an XML parser.
  The legacy `.xls` is not: it is a compound file (OLE) holding BIFF records, with its own text
  encodings and number formats. Writing and testing that is weeks of work.
- A reader must give the value Excel stored for a formula cell, tell a date from a number by the
  cell's number format, and know the 1900 and 1904 date systems. These are many small rules where an
  existing, widely used crate has already met the files that exist.
- Writing an `.xlsx` that Excel opens without a repair prompt needs correct content types, styles,
  shared strings and relationships. That is mechanical but easy to get subtly wrong.

## Decision

Two new dependencies, both in `crates/io/Cargo.toml` only:

| Crate | Version | Licence | Use |
|---|---|---|---|
| `calamine` | 0.38 | MIT | Reads `.xls`, `.xlsx`, `.xlsm` (and `.xlsb`, `.ods`, which the file picker does not offer) with the cached value of formula cells, date detection from the cell's number format, sheet visibility and sheet type. |
| `rust_xlsxwriter` | 0.102 | MIT OR Apache-2.0 | Writes `.xlsx`: numbers, text, date number formats, sheet names, column widths. |

Both are used with `default-features = false`. No `chrono`, `serde`, `ryu` or other optional feature
is switched on; dates are converted by `sheaf_core::datetime`.

Their own dependencies enter the lock file: `atoi_simd`, `byteorder`, `codepage`, `core_detect`,
`debug_unsafe`, `encoding_rs`, `fast-float2`, `multiversion_no_op` and a second `quick-xml` (0.42, next
to the 0.41 that the GUI stack already pulls in). `zip` 9.5.1 is shared with the project file. Licences
of all of these are MIT, Apache-2.0, Unlicense or BSD-3-Clause (`encoding_rs`, the Unicode tables),
which the project's MIT OR Apache-2.0 licence can carry. No C code and no network access at run time.
One of them has a build script: `encoding_rs` 0.8.44 has a small `build.rs` that asks the compiler's
version (through `rustversion`, which the lock file holds already) whether to set a cfg flag; it
generates no code and reads nothing else. The others (`calamine`, `rust_xlsxwriter`, `atoi_simd`,
`byteorder`, `codepage`, `core_detect`, `debug_unsafe`, `fast-float2`, `multiversion_no_op`,
`quick-xml`) have none.

**Binary size.** Measured on macOS (arm64) with `cargo build --release -p sheaf-cli` (thin LTO) at the same commit with and without the two crates: 13,897,152 bytes without, 14,779,184 bytes with, so about 0.86 MB more (6 %) in the command-line binary. The desktop application links the same code and gains about the same. `calamine` also carries the `.xlsb` and `.ods` readers, which cannot be left out by a feature.

**What is not read.** The reader takes cell values and nothing else: no charts, drawings or
pictures, no cell styles beyond the number format of date cells (xlsx only), no pivot tables, no
conditional formats, no data validation, no comments or notes, no hyperlinks, no defined names or VBA
(`.xlsm` is read as `.xlsx`; macros are not run or kept). A merged range gives its top-left value; the
other cells of the range are empty and the shape of the merge is not kept. Hidden rows and columns are
read like any other. A sheet that holds only a chart gives no data and is reported. The writer writes
values and number formats only: no styles, no charts, no formulas, no column formulas of the
worksheet (their values are written).

**Damaged and hostile files.** `calamine` panics on some damaged `.xls` files and builds a dense
table over the area from the first to the last used cell, so a 1.5 kB `.xlsx` with values in `A1` and
`XFD1048576` asks for gigabytes. The reader therefore catches a panic around every `calamine` call and
returns an error, reads `.xlsx` sheets cell by cell through `calamine`'s cell reader and refuses a
sheet above 40 million cells before the table is built, and refuses a zip whose parts unpack to far
more than they weigh (`zip`, already a dependency, reads the declared sizes). Limits are in
`docs/spec/import-excel.md` section 8.

**Where the code sits.**

- `crates/io/src/excel.rs` (reader, options, previews), `excel/write.rs` (writer), `excel/dates.rs`
  (serial numbers and format codes). Callers see Sheaf types only; no `calamine` or
  `rust_xlsxwriter` type is public.
- The stage "rows of cells to a sheet with long names, units, comments, short names and parameter
  rows" was moved out of `ascii::parse_block` into one function, `ascii::assemble`, that both the text
  and the Excel import call, with the header-line detection `ascii::find_layout`, and the row and
  column selection helpers. The text import's behaviour is unchanged (its whole test suite runs
  unmodified), so Excel header rows follow the same rules by construction.
- `import_file` reads an Excel file by its extension and first bytes (a text file named `.xls` stays
  text), so the app's open and drop routes and the command line get Excel without change; the sheet choice and header rows come through
  `import_file_with_excel` and `batch::import_batch_with_excel`.

## Alternatives considered

- **A hand-written `.xlsx` reader on `zip` and `quick-xml`.** No new crate, but it would not read
  `.xls`, which old lab files still are, and it would have to re-learn every quirk (shared strings,
  inline strings, date detection by format code, the 1904 flag, cached formula values, hidden sheets).
  Rejected; it remains the fallback if either crate becomes a problem for `.xlsx` alone.
- **`umya-spreadsheet`, `xlsx-rs` and similar read/write crates.** They load and keep the whole
  document model (styles, charts); this is more than the importer needs and slower on big sheets.
- **`rust_xlsxwriter` features (`serde`, `chrono`, `constant_memory`, `zlib`).** Not needed: rows are
  written from Sheaf's own columns, dates are numbers already, and exports are far below the sizes
  where constant memory matters.
- **Convert through a command-line tool (LibreOffice, Python `openpyxl`).** Adds a run-time
  requirement the task forbids.
- **Open Excel as a live window (`File > Open Excel`).** Needs Excel itself; out of scope, the menu
  item stays grey.

## Consequences

- Two more crates to keep up to date, and a second `quick-xml` in the lock file until the GUI stack
  moves to 0.42.
- Test fixtures that the writer cannot make (a legacy `.xls`, a 1904 workbook, merged cells, hidden
  sheets, chart-only sheets, a formula with and without a stored value) are generated by
  `tools/make-excel-fixtures.py` (Python `openpyxl` and `xlwt`, not needed to build or test) and
  committed under `crates/io/tests/fixtures/excel/`, each below 10 kB and made up.
- Origin's own Excel behaviour is not measured. `docs/spec/import-excel.md` separates what the
  documentation says, what Sheaf chose, and the measurements still to be made on the Windows machine.
