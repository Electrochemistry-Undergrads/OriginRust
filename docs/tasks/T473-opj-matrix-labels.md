# T473: Z labels of matrix sheets in .opj files

**Status:** done 2026-11-26 (Claude Windows "claude:3", opj-format.md 13.16, measured in Origin 2016).
**Size:** S. **Source:** rest of T471/T472.

## What it does

The long name, units and comments of a matrix sheet's values (its Z labels) are read from and written to `.opj` files: Origin
keeps them in the note of the sheet's one column record, `long CR LF units CR LF comments`, as for a workbook column. The X and Y
labels were not found in Origin's files and stay out of the `.opj` with a warning.

## Measured

`re-workspace/scratch/t471/ml.opj`: `r.lname$`, `r.unit$`, `r.comment$` of `range r=1!1` on a new matrix. Origin read the labels
of a written `sheaf-matrix.opj` back with the same properties (`Height`, `nm`, `scan 1`).

## Tests

- `crates/io/src/opj/write/feature_tests.rs`: `the_z_labels_of_a_matrix_are_written_in_its_column_note`,
  `a_matrix_with_x_labels_warns_and_a_long_book_name_is_refused`.
- `crates/io/src/opj/tests.rs`: `local_origin_files_give_their_matrices` (reads `ml.opj`, local only).
