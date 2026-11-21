# T306: Drag a sheet tab to move the sheet

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Multiple sheets per book" (P0,
gap "not by dragging a tab"). A sheet tab can be dragged onto another tab of the same workbook: the sheet
moves to that tab's position (`Workbook::reorder_sheets`), stays the active sheet, and the move is one
undo step ("Move worksheet"). Clicking, double-clicking (rename) and the right-click menu work as
before. Not measured against Origin (its tabs move the same way by dragging).
**Size:** S. **Spec:** feature-parity "Multiple sheets per book".

## Tests

- `crates/app/src/app/ui_tests/sheet_tab.rs`: `dragging_a_sheet_tab_onto_another_moves_the_sheet`.
