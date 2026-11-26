# T471: Matrix books read from .opj files

**Status:** done 2026-11-26 (Claude Windows "claude:3", opj-format.md 13.16, measured in Origin 2016).
**Size:** S–M. **Source:** feature-parity "Open .opj" row ("skips matrices"); matrices.md stage 3.

## What it does

Opening an `.opj` gives its matrix books: name, long name (and comments), one sheet per layer with its name, size
(columns at `0x2B`, rows at `0x52` of the layer header), numbers (the data set named after the window, `@2` and on for
later sheets, row by row) and X/Y range (annotations `X1`, `X2`, `Y1`, `Y2` as text; 1 to 10 without them). Before, every
matrix window was counted in "window(s) ... were not imported", and a project of only matrices got a misleading
"no workbook could be matched" warning.

Not read: the labels of the matrix coordinates, display formats, palettes. The writer is a later task.

## Measured

`re-workspace/scratch/t471`: `mj.opj` (two books, two sheets, `m2w` XYZ of the default coordinates), `m1.opj` (`matrix -ps XY`),
`mh.opj` (Origin's `2D Gaussian.ogm` sample, X 11 to 20). A sweep over the 1183 local `.opj` files read 13 matrix books with no
warning.

## Tests

- `crates/io/src/opj/tests.rs`: `matrix_books_are_read_with_their_sheets_sizes_and_coordinates`,
  `an_empty_matrix_has_missing_cells_and_no_fallback_workbook`, `local_origin_files_give_their_matrices` (Origin's own files,
  local only).
