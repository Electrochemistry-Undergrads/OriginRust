# T476: Plot row ranges in .opj files

**Status:** done 2026-11-26 (Claude Windows "claude:3", opj-format.md 10.5, measured in Origin 2016).
**Size:** S. **Source:** the writer's note "a plot's row range was not written; all rows of its columns are plotted".

## What it does

A plot that shows only some rows of its columns (Sheaf's `Plot::rows`, Origin's plot Begin/End) keeps them in `.opj` files: the curve
record holds the first row and one past the last at `0x07` and `0x0B`, and `0x1E6` is 0 instead of the 3 of a curve of all rows. The
reader keeps a range that differs from the rows of the Y data set; the writer cuts the range to the rows that hold data.

## Measured

`re-workspace/scratch/t476/r1.opj`: a graph of all rows, one with `set %C -b 3` / `-e 8`, one plotted from `[GSrc]1!(1,2)[3:8]`. A file
Sheaf wrote (ignored test `write_plot_rows_project_for_origin`) gave `get %C -b` 3 and `-e` 8 in Origin once `0x1E6` was cleared (1 and 11
before).

## Tests

- `crates/io/src/opj/write/graph_tests.rs`: `a_plot_row_range_is_written_and_read_back`, `what_the_file_cannot_hold_is_named` (the
  range now reads back).
- `crates/io/src/opj/tests.rs`: `local_origin_file_gives_plot_row_ranges` (Origin's own file, local only).
