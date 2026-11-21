# T294: Measure Binary 2D Array import against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", matrices 7f). Origin's `impBin2D` window has
Number of Columns (128), Byte Length (2), Signed/Unsigned (Unsigned), Big-Endian, Header Bytes, Transpose
after import and Output (the active matrix sheet). The rows come from the file size, rounded up, and a row
the file ends inside is all zeros. Sheaf's options window, reader and target now follow it.
**Size:** S. **Follows:** T199. **Spec:** matrices 7f.

## Left

- With a workbook active Origin writes into its worksheet; Sheaf makes a new matrix book.
- Origin's binary import filters and templates, Results Log Output, several frames from one file.

## Tests

- `crates/io/src/bin2d.rs`: Origin's defaults, every measured type, header, partial row, transpose,
  refusals.
- `crates/app/src/app/ui_tests/matrix_window.rs`:
  `a_binary_2d_array_is_imported_with_origins_options_into_the_active_sheet_or_a_new_book`.
