# T36: Worksheet cell notes

**Status:** done 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `cell-notes`. Plain text only; no Notes window, rich text or `@NCP`. Spec: gui-look.md "Cell notes (T36)".
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Worksheets: Cell notes, Origin 2022).

## Goal

Attach a note to an individual data cell, show an indicator on the cell and read the note in place
(on hover), edit and delete it; notes follow their cell through sorting and row/column edits and
are saved with the project.

## Origin facts (docs, not measured)

- Origin 2022: "Add Notes to individual Worksheet Cells", similar to Excel and Google Sheets.
  [W:originlab.com/2022]
- "To add a Note, select the worksheet cell and click the Mini Toolbar Add Note button." Note
  contents "display as a popup when you hover on the tab in the upper-right corner of the cell";
  system variable `@NCP = 0` stops the popups. Notes are edited in a Notes window ("Open in Notes
  Window" Mini Toolbar button). Cell notes hold Text or Origin Rich Text. [W:origin-help/insnote-to-wkscell]
- X-Function `insertCmt` adds a note to a selected cell (search snippet, unconfirmed). [W:x-function/ref/insertcmt]
- Not documented (Sheaf's choices, labelled unmeasured): delete, indicator colour, behaviour with
  sort/insert/delete, export, rich text.

## Where

- `crates/core`: notes per sheet keyed by column id and data row (serde default empty, saved in
  `.sheaf`; `.opj` save warns); kept in step by the core row/column operations (insert/delete rows
  and columns, sort, move, duplicate/append id remaps; see how T35 `condformat.rs` and
  `Worksheet::remap_cond_format_columns` do it).
- `crates/app/src/grid.rs` `body_cell`: a small tab in the upper-right corner of a noted cell and a
  hover popup with the note; cell right-click menu: Add Note / Edit Note / Delete Note; a small note
  editor (plain text) dialog; undo for each change.
- `crates/app/src/menus.rs`, `app.rs` actions.

## Done when

Notes can be added, read on hover, edited and deleted from the cell menu, survive save/reload,
sorting and row/column edits, undo works, and core + UI tests cover these; the spec section
(gui-look.md) records Origin's facts and Sheaf's choices.

## Don't

- Don't change conditional formatting, frozen panes, filters, masks or Go To.
- Don't add rich text or a Notes window; plain text only (left in Status).
