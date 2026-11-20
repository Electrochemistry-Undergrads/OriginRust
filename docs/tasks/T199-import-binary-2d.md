# T199: File > Import > Binary 2D Array (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"). Built from Origin's `impBin2D` options,
**not measured**.
**Size:** S. **Source:** greyed inventory package B14 ("Import: Binary 2D Array...").
**Spec:** `docs/spec/matrices.md` 7f.

## Done

- `sheaf_io::bin2d` reads eight number types in either byte order after a header. The options
  window is `dialogs/import_bin2d.rs`, and it makes a new matrix book.
- Tests: `bin2d::tests` (two) and
  `matrix_window::a_binary_2d_array_is_imported_into_a_matrix_book_named_after_the_file`.

## Left

- Import filters and several frames from one file.
- Measuring Origin's defaults.
