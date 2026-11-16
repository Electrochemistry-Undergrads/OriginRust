# T99: Duplicate (Batch Plotting)

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `batch-plot`).
**Size:** M. **Source:** greyed inventory package B11 (Window > Duplicate (Batch Plotting) > With New
Sheets/Books and With New Columns on a graph; the graph page menu's Duplicate (Batch Plotting) >
Duplicate).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

Make copies of a graph with its data replaced, as Origin 2016's batch plotting: one copy per other sheet
or book with the same columns, or one copy per other column of the sheet.

## Origin facts

Measure in Origin 2016: the windows the items open, how sheets/books and columns are chosen, what each
copy plots and how it is named, what the page menu's Duplicate does.

## Done when

The items are enabled and behave as measured (fixtures), one undo step, fmt/clippy/tests pass, inventory
and spec updated.
