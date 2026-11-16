# T93: Sheet tab Navigate...

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13h): Origin's Navigate Worksheets window measured; Sheaf's lists the sheets and goes to the picked one. Left: Actions menu, Apply Reorder.
**Size:** S. **Source:** greyed inventory package B6 (the sheet tab's Navigate...).
**Spec:** `docs/spec/workbook-model.md` (with the sheet tab section 13h).

## Goal

Navigate... on a sheet tab lists the workbook's sheets and activates the one picked, as Origin 2016's.

## Origin facts

Measure in Origin 2016: what Navigate... shows (a list or a window, the names it uses, the order) and how
a sheet is picked.

## Done when

The item is enabled and behaves as measured, fmt/clippy/tests pass, inventory and spec updated.
