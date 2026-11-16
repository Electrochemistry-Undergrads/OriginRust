# T92: Worksheet corner menu (label rows, headers, grid lines)

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `corner-menu`).
**Size:** M. **Source:** greyed inventory package B6 (the worksheet corner menu: Row Labels for Column
Label Rows > Long Name, Units, Comments, F(x)=, Parameters; Column Header, Row Header, Column Grid, Row
Grid). Sparklines and Page Break Preview Lines stay in B6.
**Spec:** `docs/spec/workbook-model.md` (a new section).

## Goal

The worksheet's corner menu shows or hides each label row and the column header, row header and grid
lines of the sheet, as Origin 2016's switches, ticked when shown.

## Origin facts

Measure in Origin 2016 (screenshots after each switch): the default state of each item, whether a switch
belongs to the sheet or the book, what hiding a label row does to its cells and to the labels used by
plots, and what the header and grid switches draw.

## Done when

The nine items are enabled, ticked as measured and saved in `.sheaf` (named in the `.opj` save warning if
the writer cannot store them), one undo step each, fmt/clippy/tests pass, inventory and spec updated.
