# T193: Analysis > Mathematics > Inverse of a matrix (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"). Built from Origin's `minverse`
documentation, **not measured**.
**Size:** S. **Source:** greyed inventory package B14 ("Mathematics: Inverse...").
**Spec:** `docs/spec/matrices.md` 7d.

## Done

- `MatrixSheet::inverse` (Gauss-Jordan with partial pivoting) and `Project::matrix_inverse` (a new
  book). `MatrixCmd::Inverse` is in the matrix bar's new Analysis > Mathematics menu and in the
  workbook and graph Analysis menus.
- Tests: `matrix::inverse_tests` and
  `matrix_window::analysis_mathematics_inverse_writes_the_inverse_to_a_new_matrix_book`.

## Left

- Special Filter.
- Measuring Origin's output (the name of the new book).
