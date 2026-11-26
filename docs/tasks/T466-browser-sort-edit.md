# T466: Project Browser sorting and editing in place (rest of T433)

**Status:** done 2026-11-25 (Claude Windows "claude:work", workbook-model.md 13s). A click on a Project Browser column
header sorts the list by it, and a second click reverses it. A double click on a Long Name, Units or Comments cell
edits it in place, and Enter writes it as one undo step.
**Size:** S. **Source:** T433 "Left".

## Tests

- `crates/app/src/dialogs/project_browser.rs`: `a_header_click_sorts_by_its_column_and_again_reverses`.
- `crates/app/src/app/ui_tests/function_graph.rs`: `project_browser_edits_units_in_place`.
