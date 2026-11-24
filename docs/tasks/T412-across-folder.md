# T412: Add Column to All Sheets by folder and position

**Status:** done 2026-11-24 (Claude Windows "claude:1", workbook-model 13q), not measured (Origin 2026b feature).
Column > Add Column to All Sheets... can reach every sheet of every workbook in this workbook's project folder and its
subfolders, and After Column puts the new column right after a named column of each sheet (else at the end).
`AcrossScope::Folder`, `AcrossColumn::after`, `Project::folder_of`.
**Size:** S. **Source:** HANDOFF C, the rest of T359.

## Tests

- `crates/core/src/across.rs`: `a_folder_scope_reaches_its_books_and_the_column_can_follow_a_named_one`.
- `crates/app/src/app/ui_tests.rs`: `add_column_to_all_sheets_reaches_every_sheet_of_the_workbook` (folder and After
  Column).

## Left

- Picking the sheets one by one: done in T418.
