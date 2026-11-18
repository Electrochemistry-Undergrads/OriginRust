# T92: Worksheet corner menu (label rows, headers, grid lines)

**Status:** the Origin 2016 corner View list was measured in workbook-model 13i on 2026-11-16.
Long Name, Units, Comments, F(x)=, Row Labels, Column Grid and Row Grid landed that day.
T94 added the Sampling Interval row. Column Header and Row Header now collapse the
corresponding screen bands and save their choices per sheet. Parameters, Filter and
Categories remain open. Formatting, strict workspace Clippy, Nextest (4,495 passed and
8 skipped), doc tests, inventory and eight Python tooling tests passed.
**Size:** M. **Source:** greyed inventory package B6 (the worksheet corner menu: Row Labels for Column
Label Rows > Long Name, Units, Comments, F(x)=, Parameters; Column Header, Row Header, Column Grid, Row
Grid). Sparklines and Page Break Preview Lines stay in B6.
**Spec:** `docs/spec/workbook-model.md` (a new section).

## Goal

The worksheet's corner menu shows or hides each label row and the column header, row header and grid
lines of the sheet, as Origin 2016's switches, ticked when shown.

## Origin facts

The 2016 corner menu shows both header switches checked by default. The documented View tab
identifies their screen bands. Workbook-model 13i records the reference and Sheaf's layout,
per-sheet switches and recovery menu.

## Done when

The nine items are enabled, ticked as measured and saved in `.sheaf` (named in the `.opj` save warning if
the writer cannot store them), one undo step each, fmt/clippy/tests pass, inventory and spec updated.
