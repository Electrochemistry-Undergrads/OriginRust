# T126: Statistics on Columns respects the column data range (T87 follow-up)

**Status:** done, 2026-11-17; descriptive column reader only. Other T87 analysis readers remain open.

Fmt, strict workspace/all-target Clippy, all 4,458 workspace tests (eight ignored),
inventory272 and eight Python tests pass with all929 source/fixture/asset hashes
frozen. Five Core regressions cover the accepted N3/mean30 fixture, per-column and
explicit row/XBetween intersections, both empty directions, extreme persisted
bounds and mask/filter/missing handling. Two App regressions exercise range,
reset and Statistics menus plus complete-project Undo/Redo. A fresh owned desktop
Statistics menu run saves N3/mean30/SD10/sum90; the entire source worksheet stays
unchanged, Undo removes the report, and restored source-export pixels match
exactly. The owned app and guard close successfully; no global input is used.
An initial test-only API compile pilot is excluded; production scope stayed fixed.

**Size:** S. **Priority:** wave 5 (B5/T87 follow-up). **Spec:** workbook-model 13f.

## Goal

Use only the saved Begin/End rows for descriptive Statistics on Columns. T87 already
implements the five range-edit commands, persistence and plotting; do not duplicate them.
Its accepted Origin2016 fixture (B = 10..60, rows 2..4) gives N = 3 and mean = 30.

## Where

Only `core/analysis.rs::statistics_table` and its `Tool::Statistics` call site change.
Intersect the existing requested row span with that input Y column's saved data range.
Keep mask/filter handling, numerical formulas, units, report sheets and document history.
No shared input-loader or other Tool changes. Add Core integration regressions and an
App UI test module/registry; update T87, this brief, workbook-model13f and the task index.

## Contract and checks

- Rows outside the saved range are excluded, not counted as missing. Existing masked,
  filtered or missing values inside the intersection retain their current handling.
- Explicit row or XBetween requests intersect the Y range. An unrelated X column's
  own data range does not restrict univariate Y statistics.
- An empty intersection uses existing empty descriptive statistics: N = 0, Missing = 0,
  Sum = 0 and undefined quantities NaN. This edge is Sheaf policy, not measured Native parity.
- Each selected Y uses its own range. No stored source values or range metadata change.
- Tests cover the accepted fixture, explicit intersections, both disjoint directions,
  absent/beyond-end ranges, masks/missing values, per-column ranges, units, untouched
  source and existing output/history behavior. Exercise real range and Statistics menus.
- Run fmt, strict workspace/all-target Clippy, full workspace tests and inventory check.

## Reference boundaries

The existing T87 measurement is the primary numeric reference; no new Native probe or
COM repair is attempted. Origin's current [FAQ-1101](https://docs.originlab.com/quick-help/set-as-begin/)
describes clearing/deleting rows and menu changes in 2021/2022. Do not replace the accepted
2016 non-destructive range fixture with that newer UI behavior. Historical Origin
[Worksheet documentation](https://www.originlab.com/pdfs/3_Worksheet.pdf) also documents
display-range analysis; it does not settle all modern GUI details.

## Not included

Statistics on Rows, inferential/distribution/crosstab/ANOVA tools, fitting and other
analysis readers remain T87 leftovers. No model, IO/schema, grid/layout, worksheet
fonts/default families, clipboard, graph interaction, menu or App dispatch changes.
