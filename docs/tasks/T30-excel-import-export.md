# T30: Excel import and export (xls, xlsx, xlsm)

**Status:** done, merged 2026-11-14 night (macOS session "mac-excel", no Origin; ADR 0012, spec
`docs/spec/import-excel.md`). `sheaf_io::excel` reads xls/xlsx/xlsm through `calamine` (ASCII header-row
logic shared, not copied; cached formula values; 1900/1904 dates; size and zip guards; damaged files are
errors, not panics) and writes xlsx through `rust_xlsxwriter`; File > Import > Excel and Export >
Worksheet as Excel, drag-and-drop, `sheaf import/info/export/export-data`, MCP `import_files`. Synthetic
fixtures from `tools/make-excel-fixtures.py`. Reviewed by three agents with each finding checked; 12 fixed.
Origin measured on Windows (2026-11-14/15, Origin 2016 `impExcel`, probes p1 to p11 in
`re-workspace/scratch/t30/`; import-excel.md 2) and copied where Sheaf differed: hidden sheets are read
(`--skip-hidden-sheets` leaves them out), the book long name is the file name with its extension,
leading empty columns are kept (Excel's column letters), and dates among text keep their dates in a
Text & Numeric column; booleans, error cells, merged cells, text among numbers, 1904 dates and date
numbers already matched (fixture `origin-kinds.xlsx` compares Sheaf's numbers with Origin's). Kept as
Sheaf's own: header rows detected (Origin reads none), dates before 1900-03-01 as Excel shows them
(Origin's are one day early). Left (import-excel.md 10): Origin's import window, export (not
reachable from LabTalk in Origin 2016), display of Excel format codes, user parameter rows, `.xlsb`/`.ods` tests (step 6 done: three
workbooks on the Windows machine agree cell by cell with `openpyxl`, import-excel.md 10); MCP `import_files` has no switch to leave hidden sheets out
and offers only the books/sheets layouts for Excel; Open Excel stays grey (out of scope).
**Size:** L (import M, export S, Open Excel window out of scope). **Branch:** `excel-io`. **Model:** Sonnet.
**Detail:** audit A (File menu greyed items) and E (import/export); parity rows "Excel import" (P1),
"Write xlsx" (P1), "Open Excel, sample project, Close" (P1).
**Spec:** new `docs/spec/import-excel.md`; the header-row and naming rules to reuse are in
`docs/spec/import-ascii.md` sections 3 (header handling), 10 (import mode and naming) and 13 (expASC).

## Goal

File > Import > Excel (XLS, XLSX, XLSM)... reads one or more sheets of a workbook into Sheaf books, with
long name, units and comments taken from header rows as the ASCII import does, and the workbook Export
submenu writes the active book (or sheet) as `.xlsx`. `sheaf import` and `sheaf export` gain the same.
The owner keeps much of their lab data in Excel workbooks (CV overlays, ICP-OES results), so the import
must read their cells, numbers, dates and text faithfully even when the sheet also holds charts and
formulas.

## Origin facts

- Not measured yet. From Origin's public documentation (paraphrased): File > Import > Excel uses the
  Excel data connector in recent versions (older: the `impExcel` X-Function). One book per Excel file,
  one sheet per Excel sheet (all sheets by default, or a chosen set); header lines chosen as main header
  and subheader rows, which map to Long Name, Units, Comments; formula cells import their cached value;
  date cells become Date columns; a partial import selects rows and columns. File > Open Excel (Ctrl+E)
  opens the file as a live Excel window inside Origin, which needs Excel itself: **out of scope**, leave
  the item grey.
- Measure on the Windows machine with `tools/origin-ref` (one Origin at a time, global lock) before
  claiming any of the above: run `impExcel` (or the connector's LabTalk equivalent) on synthetic
  `.xlsx` files written by the new exporter or by Python `openpyxl` in `re-workspace/scratch/t26/`:
  header rows, empty leading rows, merged cells, text in a numeric column, dates and times, a formula
  cell, a hidden sheet, a sheet with a chart only, more than 1000 columns. Record each in
  `import-excel.md` with the run's output file name.
- Export: Origin's worksheet export to Excel is `expXLS`/the connector's save; measure the sheet names,
  header rows (Long Name, Units, Comments) and how missing values are written, the same way.

## Where

- Dependencies (ADR first, number after the MCP branch's 0011, so 0012): `calamine` (MIT) reads
  xls, xlsx, xlsm and ods with cached formula values and date detection; `rust_xlsxwriter`
  (MIT/Apache-2.0) writes xlsx. `zip` and `quick-xml` are already in the lock file (crates/io's `.opju`
  reader); a hand-written xlsx reader on them is the fallback if the ADR rejects new crates, but it would
  not read legacy `.xls`.
- `crates/io/src/excel.rs` (new): `read_workbook(path, &ExcelOptions) -> Vec<SheetData>` and
  `write_xlsx(book, path, &ExcelExportOptions)`; reuse the header-row assignment of `crates/io/src/ascii.rs`
  (find the function that maps header lines to Long Name/Units/Comments by name, do not copy it).
- `crates/io/src/batch.rs`: an Excel source next to ASCII so batch import modes (new books, new sheets,
  new columns) apply.
- `crates/app/src/menus.rs` `file_menu`: wire `"Excel (XLS, XLSX, XLSM)..."` under Import and a new
  "Worksheet as Excel..." under Export; keep "Open &Excel..." and New > Excel grey. A dialog for sheet
  choice and header rows in `crates/app/src/dialogs/` (copy the pattern of `import.rs`, the ASCII
  import options, and reuse its header-row controls); export options next to `export_sheet.rs`.
- `crates/cli/src/args.rs` and the import/export commands: `.xls`/`.xlsx`/`.xlsm` detected by extension,
  `--sheets`, header-row flags matching the ASCII ones.
- Drag-and-drop and File > Open routing in the app: an Excel file dropped on the window imports.

## Steps

1. ADR 0012 (dependencies, licences, binary size, what is not read: charts, styles, pivot tables).
2. Reader with tests on small `.xlsx` files generated in the test itself by the writer (round trip) and
   one committed hand-made fixture per edge case (each under 10 kB, synthetic, never the owner's files).
3. Writer: one sheet per Sheaf sheet, header rows as Origin writes them (once measured), missing values
   as empty cells, text columns as text, dates as Excel dates.
4. Menus, dialog, CLI, drag-and-drop; UI test that the Import item is enabled and a dropped `.xlsx`
   makes a book.
5. Origin measurements (Windows) and the spec; parity rows "Excel import" and "Write xlsx" to ◐ or ✓.
6. Local-only check: if the owner's Excel workbooks are present (git-ignored, read-only), import each
   and compare the cell values with `openpyxl` (`data_only=True`) in a scratch script; never commit
   their contents.

## Done when

An `.xlsx` with header rows, numbers, text, dates and formula cells imports into the expected columns
and labels from the menu, drag-and-drop and `sheaf import`; a book exports to `.xlsx` that Excel and
`openpyxl` read back with the same values; fmt, clippy and tests pass; the spec lists what was measured.

## Don't

- Don't depend on Excel being installed, and don't use COM to Excel.
- Don't commit the owner's workbooks or numbers from them.
- Don't claim Origin's header-row defaults until measured; until then use the ASCII import's defaults
  and say so in the spec.
