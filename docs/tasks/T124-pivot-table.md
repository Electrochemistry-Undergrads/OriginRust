# T124: Worksheet > Pivot Table...

**Status:** done 2026-11-17 (Claude Windows, scheduled session "cond-heatmap", branch `pivot-table`; workbook-model.md 13n), not measured (Origin COM unavailable); measured and corrected in T276 (2026-11-21). Left: Combine/Others, extra values, user-parameter position, a check against Origin 2016 (it has `wpivot`).
**Size:** M. **Source:** greyed inventory package B7 (its last item).
**Spec:** `docs/spec/workbook-model.md` (a new section next to the reshaping tools).

## Goal

Summarize a worksheet into a new sheet: one row per distinct value (or combination) of the row
source columns, one column per distinct value of the column source, each cell counting the rows or
summing/averaging/taking the min/max of a data column.

## Origin facts (X-Function `wpivot`, docs.originlab.com/x-function/ref/wpivot; Origin 2015 SR0 and later, so Origin 2016 has it; not measured: Origin COM is unavailable on this machine now)

- `row` (one or more columns), `col`, `data` (not used by count); `method` 0 count (default), 1 sum,
  2 mean, 3 min, 4 max.
- `total` adds a Row Total column (sum across each row), `sum` a Column Total row; `sort_total` 0
  row labels ascending, 1 descending by row totals (default), 2 ascending by row totals, 3 row
  labels descending, 4 none; `sort_col` 0 column labels ascending (default), 1 descending, 2
  ascending by column totals, 3 descending by column totals, 4 none.
- `normalize` 0 none, 1 fraction, 2 percent of each column's total; `zero` (default 1) shows
  missing combinations as zeros; `pos` (default 1 Comments) where a column's value is written:
  0 Long Name, 1 Comments, 2 appended to the Long Name, 3 a user parameter row.
- `dir`/`vmode`/`threshold`/`label` merge small values into an "Others" row or column; `exrow`/
  `excol` add extra values; output `rd` defaults to a new sheet of the input book.

## Steps

1. Core: new `crates/core/src/pivot.rs` (pure table builder + `Project::pivot_table`), tests.
2. App: a Pivot Table window, the menu item, a UI test.
3. Spec section, inventory, index.

## Done when

The item builds the table in a new sheet as one undo step; Others merging, extra values and the
user-parameter position may be left (stated in Status, shown disabled). fmt, strict Clippy and the
full tests pass.
