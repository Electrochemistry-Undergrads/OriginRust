# T421: Sticky Notes on graphs, workbooks and sheets (Origin 2027)

**Status:** done 2026-11-24 (Claude Windows "claude:work", notes-window.md "Sticky Notes"). Window > Sticky Note
(a graph), Sticky Note on Book and Sticky Note on Sheet (a workbook) pin a coloured note to the window or to the shown
sheet. The note shows as a strip at the top of the window, with the text, five colours, Collapse and Delete Note.
Collapsed, it is a one-line indicator showing the note's first line. Typing is one undo step, and so is each other
change. Notes save with the project and Find in Project finds them with the notes (attribute "Sticky Note").
Sheaf's own design (Origin 2016 has none).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Sticky Notes".

## Left

- Several notes per window, a note's place inside the window, notes in .opj files. (The Project Explorer mark: T450;
  matrices and layout pages: T460.)

## Tests

- `crates/core/src/sticky.rs`: graph, book and sheet notes, the first line, adding again opens, save round trip,
  removal.
- `crates/app/src/project_find.rs`: `sticky_notes_are_found_with_the_notes`.
- `crates/app/src/app/ui_tests/function_graph.rs`: `sticky_notes_pin_to_a_sheet_take_text_and_collapse`.
