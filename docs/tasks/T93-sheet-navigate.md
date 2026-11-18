# T93: Sheet tab Navigate...

**Status:** sheet listing and activation landed 2026-11-16 (workbook-model 13h). The Comments
column now shows the worksheet's saved T90 comments with full text on hover. Actions and
Apply Reorder remain open. Formatting, strict workspace Clippy, Nextest (4,517 passed and
8 skipped), doc tests, inventory and eight Python tooling tests passed.
**Size:** S. **Source:** greyed inventory package B6 (the sheet tab's Navigate...).
**Spec:** `docs/spec/workbook-model.md` (with the sheet tab section 13h).

## Goal

Navigate... on a sheet tab lists the workbook's sheets and activates the one picked, as Origin 2016's.

## Origin facts

Measure in Origin 2016: what Navigate... shows (a list or a window, the names it uses, the order) and how
a sheet is picked.

## Done when

The item is enabled and behaves as measured, fmt/clippy/tests pass, inventory and spec updated.
