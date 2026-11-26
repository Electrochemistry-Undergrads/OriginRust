# T433: View > Project Browser (Origin 2025b)

**Status:** done 2026-11-24 (Claude Windows "claude:work", workbook-model.md 13s). View > Project Browser opens one
list of the project's books, sheets, columns and graphs, with where each is, its Long Name, Units and Comments. A
search over all of these narrows the list as you type, and a Show filter keeps one kind. Clicking a name previews the
item (a column's first values, a sheet's size, a book's sheets, a graph's layers and plots). Go To or a double click
opens it, as a Find in Project hit does. Ticked items get one field set at once ("Apply to n Ticked", one undo step).
Sheaf's own design (Origin 2016 has no Project Browser).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Project Browser".

## Left

- Folders and matrices in the list, other metadata (user parameters, sheet tab colours), and a preview picture of a
  graph. (Sorting and editing in place: T466.)

## Tests

- `crates/core/src/project_browser.rs`: the list in project order, the search, setting a field on many items
  (items without the field left alone, unchanged ones not counted).
- `crates/app/src/app/ui_tests/function_graph.rs`: `project_browser_sets_units_on_ticked_columns_in_one_step`.
