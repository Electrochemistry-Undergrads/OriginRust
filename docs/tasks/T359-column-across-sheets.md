# T359: Add Column to All Sheets

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", workbook-model 13q). HANDOFF C (Origin 2026b "insert
columns across sheets"): Column > Add Column to All Sheets... adds one column (long name, units, optional formula and
Recalculate mode) at the end of every sheet of the workbook or of every workbook, the formula computed from each
sheet's own columns, as one undo step. Not in Origin 2016, so the rules are Sheaf's own.
**Size:** S. **Spec:** workbook-model 13q.

## Left

- Picking the sheets one by one (folder scope and inserting after a named column: done in T412).

## Tests

- `crates/core/src/across.rs`: per-sheet formula values, project scope, a failing formula.
- `crates/app/src/app/ui_tests.rs`: `add_column_to_all_sheets_reaches_every_sheet_of_the_workbook`.
