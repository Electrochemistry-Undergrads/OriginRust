# T88: Copy formula, Copy label rows, Copy formula and label rows, Copy All

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13.4c): Origin's clipboard text measured for all four (and the existing Copy including label rows), whole columns only as Origin greys them for a block, Ctrl+J / Ctrl+Alt+J / Ctrl+Shift+A. Left: where Origin puts user parameter rows; which command Ctrl+Shift+J runs in Origin's workbook.
**Size:** S. **Source:** greyed inventory package B5 (Edit > Copy submenu on a workbook).
**Spec:** `docs/spec/workbook-model.md`, clipboard sections (13.4).

## Goal

The four greyed items of the workbook's Edit > Copy submenu copy what their names say, as Origin 2016
puts it on the clipboard: the column formulas, the label rows, both, and the whole sheet.

## Origin facts

Measure in Origin 2016 (menu driven, clipboard text captured): the text each item puts on the clipboard
for a selected block and for whole columns, which label rows and in what order, how a formula and an
empty formula appear, and what Copy All takes (the sheet regardless of selection, with or without label
rows).

## Done when

The four items are enabled and copy as measured (fixtures), fmt/clippy/tests pass, inventory and spec
updated.
