# T199: File > Import > Binary 2D Array (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"); measured against Origin 2016's `impBin2D`
and matched by T294 (2026-11-21): Origin's options and defaults, rows from the file size, a partial last
row of zeros, transpose, import into the active matrix sheet.
**Size:** S. **Source:** greyed inventory package B14 ("Import: Binary 2D Array...").
**Spec:** `docs/spec/matrices.md` 7f.

## Done

- `sheaf_io::bin2d` reads eight number types in either byte order after a header. The options
  window is `dialogs/import_bin2d.rs`, and it makes a new matrix book.
- Tests: `bin2d::tests` (two) and
  `matrix_window::a_binary_2d_array_is_imported_with_origins_options_into_the_active_sheet_or_a_new_book`.

## Left

- Import filters and several frames from one file.
- With a workbook active, Origin writes into its worksheet (Sheaf makes a new matrix book).
