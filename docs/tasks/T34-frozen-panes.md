# T34: Worksheet frozen panes

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `frozen-panes`.
**Size:** L. **Source:** `origin-2026-2027-gaps.md` (Worksheets: Worksheet frozen panes, Origin 2023).

## Goal

Keep the first n data rows and/or the first n columns of a worksheet visible while the rest of the
data area scrolls, as Origin 2023's View: Freeze/Unfreeze Panes.

## Origin facts (docs, not measured)

- Origin 2023 highlights: "Set up Frozen Panes", "Freeze first n rows or columns". [W:originlab.com/2023]
- Menu "View: Freeze/Unfreeze Panes" (a toggle), shortcut Ctrl+Shift+F, Mini Toolbar button
  "Freeze Panes". Select a row, a column or a single cell whose index is between 1 and 10; to
  unfreeze, click the upper-left cell and "Unfreeze Panes", or the View toggle.
  [W:origin-help/freeze-rows-columns]
- Frozen row and column headers are shaded darker. [W:origin-help/freeze-rows-columns]
- LabTalk `wks.frozen$` (string): `wks.frozen$="C 3"` freezes columns A to C and rows 1 to 3, the
  same as selecting cell D4 and clicking Freeze Panes; `wks.frozen$=""` unfreezes. [W:labtalk/ref/wks-obj]
- So the frozen part is everything above and left of the selected cell; selecting a whole row
  freezes rows only, a whole column freezes columns only (inferred; check on Windows).
- Not stated: whether label rows count, whether the state is per sheet and saved with the project,
  the divider look. Sheaf's choices go in `docs/spec/gui-look.md` (or the worksheet spec) labelled
  as unmeasured.

## Where

- `crates/core` worksheet model: frozen rows/columns per sheet (serde default 0, saved in `.sheaf`;
  `.opj` save warns if the writer cannot store it).
- `crates/app/src/grid.rs` (`show`, the `TableBuilder` body/header, scroll offsets, hit-testing,
  selection, keyboard navigation and reveal/scroll-to): draw frozen rows under the header and frozen
  columns beside the row header, kept in place while the rest scrolls.
- `crates/app/src/menus.rs` (workbook View menu, Ctrl+Shift+F), `crates/app/src/app.rs` action.

## Done when

Freeze/Unfreeze from the View menu and Ctrl+Shift+F work from a cell, row or column selection (index
1-10), frozen cells stay visible while scrolling both ways, selection, editing, drag-select, keyboard
navigation and Go To still work, the state saves in `.sheaf`, and UI tests cover these.

## Don't

- Don't change filter, mask, Go To or formula behaviour; frozen rows are counted in displayed
  (filtered) rows as the grid shows them.
