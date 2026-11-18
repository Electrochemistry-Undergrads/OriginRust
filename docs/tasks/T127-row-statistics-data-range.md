# T127: Row statistics respects column Begin/End ranges (T87 follow-up)

**Status:** done, 2026-11-17; row descriptive reader only, other T87 readers remain open.

Fmt, strict workspace/all-target Clippy, all4,469 workspace tests (eight ignored),
inventory271 and eight Python tests pass on integrated T128 source with all933
source/fixture/asset hashes frozen. Four Core regressions cover seven quantities,
per-column ranges, request intersections, original row alignment, ragged/missing/
masked/filtered inputs, empty/extreme ranges and reset. Two App regressions exercise
actual range/reset/row-statistics menus, unchanged source and whole-project
Undo/Redo. A fresh owned desktop row-statistics menu produces the independently
expected original-row means/SD/units; full source metadata/data stay fixed, Undo
restores the complete worksheet, and restored export pixels match exactly.
The app/guard close successfully and no global input is used. The pre-interruption
focused Core pass is historical; this combined full pass supplies current credit.

**Size:** S. **Priority:** wave 5 (B5/T87). **Spec:** workbook-model13f, statistics rowstats paragraph.

## Goal and reference

Only use a selected column's saved Begin/End rows when calculating Statistics on
Rows. T87 already stores these ranges; T126 fixes the column descriptive reader.
The official [rowstats reference](https://docs.originlab.com/x-function/ref/rowstats/)
describes seven quantities across rows of an input range. The [Statistics on Rows
guide](https://docs.originlab.com/origin-help/statistics-on-rows/) excludes missing
values. T87's accepted 2016 fixture establishes a non-destructive column data range.
Intersecting each contributor's saved range with the requested row span is explicit
Sheaf policy under those contracts, not a fresh Native2016 rowstats measurement.
No Native COM retry or repair is attempted.

## Where and behavior

- Only `core/analysis.rs::row_statistics` and its call gain the source Worksheet.
  Resolve each contributor's range once; fill cells in original row and input order.
- A value contributes when its original request span and saved range both contain
  that row. Existing mask/filter/missing handling and the numerical calculator stay.
- Keep output length and alignment, seven quantities, optional selection, source
  units/comments, insertion behind the last selected column and history unchanged.
- A row with no eligible numbers retains the existing missing outputs, including
  Number of Points and Sum. One eligible number has no SD or SEM. Do not substitute
  the column descriptive reader's empty-table N=0/Sum=0 policy.
- Stored values/range metadata are read-only. Each selected input has its own range;
  unrelated unselected X-column bounds do not remove other contributors.

## Checks and completion

Add Core integration tests for all seven quantities, staggered/disjoint/beyond-data
ranges, original row alignment, request intersection, ragged columns, masks/filters,
empty/one-value rows, source preservation and reset. Add UI menu/history tests and
inspect a fresh owned desktop result and saved source/Undo. Run fmt, strict
workspace/all-target Clippy, full workspace tests and inventory checks before push.
Update the task index/spec and release only this claim after all actors terminate.

## Not included

No shared input loader, T126 column reader, inferential/distribution/categorical/
ANOVA/fitting or other analysis tools. No model/IO/schema, grid/fonts/default families,
clipboard, App dispatch or other active owner's modules. Other T87 readers remain open.
