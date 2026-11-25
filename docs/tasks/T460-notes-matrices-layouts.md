# T460: Sticky notes on matrix books and layout pages (rest of T421)

**Status:** done 2026-11-25 (Claude Windows "claude:work", notes-window.md "Sticky Notes"). Window > Sticky Note works
on matrix books and layout pages. The note's strip sits above the matrix or the page, and Project Explorer marks it.
**Size:** S. **Source:** T421 "Left".

## Tests

- `crates/core/src/sticky.rs`: `notes_pin_to_graphs_books_and_sheets_and_save` (a matrix book and a layout page).
- `crates/app/src/app/ui_tests/notes_window.rs`: `a_matrix_book_takes_a_sticky_note`.
