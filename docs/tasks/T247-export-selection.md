# T247: Export only the selected cells (expASC `select`)

**Status:** done 2026-11-20 (Claude Windows "claude:3"), `select` not measured in Origin. File >
Export > Worksheet as ASCII... gets Selected cells only: the selected columns and rows of the active
sheet, label rows of those columns, row labels with the sheet's row numbers
(`sheaf_io::export::write_selection`). The parity row "Export worksheet as text" was stale (the
options window, semicolon, label rows and full precision exist) and is corrected.
**Size:** S. **Source:** feature-parity "Export worksheet as text" (P1).
**Spec:** `docs/spec/import-ascii.md` 13.1.

## Tests

- `crates/io/src/export.rs`: `a_selection_writes_its_columns_and_rows_with_the_sheets_row_numbers`.
- `crates/app/src/app/ui_tests.rs`: `the_text_export_can_write_only_the_selected_cells`.

## Left

- Measure Origin's `select` (non-contiguous selections); the `sampinterv`, `parameter` and `addxcol`
  options; `sheaf export-data` has no selection.
