# T86: Swap Columns and Paste Transpose

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13e): Swap Columns as measured `colswap` (exactly two columns exchange places with all their properties; otherwise refused), Paste Transpose in the Edit menu and the data cell menu by the ordinary paste rules. Left: Paste Transpose measured in Origin (menu-only).
**Size:** S. **Source:** greyed inventory package B5 (the column header's Swap Columns..., Edit > Paste
Transpose and the data cell's Paste Transpose).
**Spec:** `docs/spec/workbook-model.md` (a new section next to the clipboard sections).

## Goal

Swap Columns exchanges the places of two selected columns, as Origin's `colswap`. Paste Transpose
pastes the clipboard's block with rows and columns exchanged at the active cell.

## Origin facts

Measure in Origin 2016: what Swap Columns does with one, two or more selected columns (window or not),
what moves with the columns (names, labels, formulas, designations), and Paste Transpose's placement,
growth of the sheet and handling of text and empty cells.

## Done when

The three items are enabled and behave as measured (fixtures), one undo step each, fmt/clippy/tests
pass, inventory and spec updated.
