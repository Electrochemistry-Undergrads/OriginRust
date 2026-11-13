# T10: Worksheet operations

**Status:** items 1-8 done on branch `worksheet-ops` (spec workbook-model 13.1-13.2). Left: Mask Change Color, Disable Masking, by condition and from the graph, `.opj` read of masks; one filter per column, date/text filters, plots skipping filtered rows; Stack/Unstack/Split/Reduce option layouts not compared with Origin; Group/Subject file codes unknown. Earlier: (`sheet-menus`, merged 2026-11-13, added drag auto-scroll and typing into a block in `grid.rs`). **Size:** L, one commit per
operation. **Branch:** `sheet-ops`. **Model:** Sonnet.
**Detail:** audit E "Missing" and "Partial"; A "Greyed" (Column and Worksheet menus).
**Spec:** `docs/spec/workbook-model.md`, `docs/spec/ui-command-map.md` (Column, Worksheet menus).

## Model facts

- `Column` in `crates/core/src/column.rs`; `Designation` is X, Y, Z, YError, XError, Label, Disregard
  (no Group/Subject, no categorical flag). No mask field. Missing = NaN only.
- Sort: `Project::sort_sheet` (single key, whole sheet) and filter `Project::filter_sheet`
  (deletes rows) in `crates/core/src/analysis.rs`.
- Undo is a full snapshot per step, cap 100 (`crates/core/src/history.rs`).
- Greyed menu entries live in `crates/app/src/menus.rs` (search the labels below).

## Operations, in priority order

1. Insert rows/columns at a position; delete rows (Edit > Insert, Delete; grid right-click).
2. Move columns (Column > Move Columns: to first/last/left/right).
3. Hide/Unhide columns (Column > Hide/Unhide Columns); column width (Column Properties Width is
   greyed in `dialogs/column_props.rs`).
4. Sort Column (one column only), Sort Range, nested sort (custom), sort by label.
5. Mask: a per-cell mask set on Column (and `.opj` read keeps it: the reader skips masks today,
   `crates/io/src/opj/`); masked points excluded from analysis and drawn in red.
6. Non-destructive filter (Column > Filter) replacing the deleting filter, with presets.
7. Transpose, Stack/Unstack, Split Columns, Reduce Rows (Worksheet menu).
8. Set as Categorical; Group/Subject designations.

Each: measure Origin's result on a small sheet with `tools/origin-ref/run-labtalk.ps1` where the
behaviour is not obvious (sort stability, missing values in sorts, masked points in statistics).

## Done when

Each finished operation: menu item live, undo works, UI test, `.sheaf` and `.opj` save keep the new
state or name it in the save warnings; checks pass.
