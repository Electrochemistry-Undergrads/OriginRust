# T469: Edit an operation's parameters from the operation tree (rest of T443)

**Status:** done 2026-11-26 (Claude Windows "claude:work", analysis-tools.md 23). The operation tree's right-click
menu has Parameters.... It shows every setting of the linked analysis' tool as a field (numbers, switches, choices by
option name). OK runs the operation again with the new settings and replaces its results in place, as one undo step.
**Size:** S. **Source:** T443 "Left".

## Tests

- `crates/core/src/operations.rs`: `a_books_operations_list_by_result_sheet_and_change_mode` (new settings, same result
  columns, another tool refused).
- `crates/app/src/app/ui_tests/notes_window.rs`: `the_operation_tree_lists_a_books_linked_analysis_and_switches_its_mode`
  (Parameters...).
