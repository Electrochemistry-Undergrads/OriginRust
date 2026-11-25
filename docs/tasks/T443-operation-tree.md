# T443: The workbook operation tree in Object Manager (Origin 2027)

**Status:** done 2026-11-25 (Claude Windows "claude:work", analysis-tools.md "Operation tree"). With a workbook
active, Object Manager lists the book's operations: the analyses run with Recalculate Auto or Manual, grouped by the
sheet holding their results. Each shows its mode, whether it waits for Recalculate, and its settings as hover text,
followed by its input and output columns. Clicking a column opens it. An operation's right-click menu has
Recalculate, Auto, Manual, Stop Following the Input and Go to Results. A waiting operation switched to Auto runs.
Each change is one undo step. Sheaf's own design (Origin 2016 has no operation tree).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Workbook operation tree".

## Left

- Editing an operation's parameters from the tree (reopening its dialog on the run), operations that run once (no
  link), fits and statistics reports, and dragging operations between books.

## Tests

- `crates/core/src/operations.rs`: the book's operations by result sheet, waiting after an input edit, Auto runs it,
  Stop Following keeps the data.
- `crates/app/src/app/ui_tests/notes_window.rs`:
  `the_operation_tree_lists_a_books_linked_analysis_and_switches_its_mode`.
