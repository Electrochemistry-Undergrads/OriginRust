# T434: Name Manager

**Status:** done 2026-11-24 (Claude Windows "claude:1", expression-language "Named cells"), not measured (Origin 2022
feature). Worksheet > Name Manager... lists the active workbook's names with what each stands for (`Sheet1!B2`,
`Sheet1!B2:B4`, `Sheet1!A1:C3`) and deletes one with Delete, one undo step. `Workbook::name_reference`.
**Size:** S. **Source:** HANDOFF C, the rest of T396, T400 and T432.

## Tests

- `crates/core/src/named.rs`: `a_name_says_what_it_stands_for`.
- `crates/app/src/app/ui_tests.rs`: `define_name_on_selected_cells_of_a_column_names_the_range` (the manager lists the
  range and the block, Delete, undo label, Close).

## Left

- Renaming and redefining in the manager, project-wide names.
