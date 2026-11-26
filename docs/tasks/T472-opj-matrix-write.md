# T472: Matrix books written to .opj files

**Status:** done 2026-11-26 (Claude Windows "claude:3", opj-format.md 13.16, measured in Origin 2016).
**Size:** S–M. **Source:** rest of T471; feature-parity "Save project as .opj" ("matrices ... are not written").

## What it does

Saving an `.opj` writes each matrix book in its place among the windows, as Origin 2016 writes one: window and layer headers
with Origin's fixed bytes, the size, the X/Y range as the `X1`/`X2`/`Y1`/`Y2` annotations (only when it is not 1 to 10), one
column record and one data set per sheet (Numeric doubles, row by row, the missing value for empty cells). The data sets take
their places in window order, so graphs after a matrix keep their columns. Matrix sheets with coordinate labels get a warning;
a book name too long for `name@n` is refused.

Fixed on the way: layout pages (not written) no longer take a place in the window positions of the folder tree.

## Verified in Origin

`sheaf-matrix.opj` (written by the ignored test `write_matrix_project_for_origin`) loaded in Origin 2016: both books are matrices,
the cells read back (203, 101, the missing cell, 4 on the second sheet), two sheets, and `m2w` gave the written X 0.5 to 2.5 and
Y 10 to 20.

## Tests

- `crates/io/src/opj/write/feature_tests.rs`: `matrix_books_are_written_and_read_back`,
  `a_matrix_sheet_is_written_as_origin_writes_one`, `a_matrix_with_labels_warns_and_a_long_book_name_is_refused`.
- `crates/io/src/opj/write/graph_tests.rs`: `a_matrix_book_in_front_of_the_workbook_keeps_the_plots_on_their_columns`.
