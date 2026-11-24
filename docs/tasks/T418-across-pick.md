# T418: Add Column to chosen sheets

**Status:** done 2026-11-24 (Claude Windows "claude:1", workbook-model 13q), not measured (Origin 2026b feature).
Column > Add Column to All Sheets... has "Only the ticked sheets": a ticked list of every sheet of every workbook
(this workbook's sheets ticked at first); OK adds the column to the ticked sheets only. `AcrossScope::Sheets`.
**Size:** S. **Source:** HANDOFF C, the rest of T359 and T412.

## Tests

- `crates/core/src/across.rs`: `chosen_sheets_get_the_column_and_the_others_do_not`.
- `crates/app/src/app/ui_tests.rs`: `add_column_to_all_sheets_reaches_every_sheet_of_the_workbook` (an unticked
  sheet keeps its columns).

## Left

- Nothing.
