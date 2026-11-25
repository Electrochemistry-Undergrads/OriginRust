# Notes windows (T154, 2026-11-19, **not measured**)

Origin 2016's File > New > Notes makes a Notes window: a window of free text kept in the project.
It could not be probed (Origin's COM class was unregistered), so this is Sheaf's version of the
documented behaviour.

- **Making one:** File > New > Notes, or the Standard toolbar's New Notes, adds an empty Notes window
  named `Notes1`, `Notes2`, ... (the first free name), one undo step ("New notes").
- **The window:** one text box over the whole window, plain text in a fixed-width font, with a
  scroll bar.
  - The text is written to the project when the box lets go of the keyboard (a click elsewhere, or
    Escape), as one undo step ("Edit notes"). An undo or redo that changes the text while it is being
    typed wins, and the box shows the project's text again.
  - No formatting, no HTML.
- **Like any window:**
  - It is listed in the Project Explorer (type "Notes") and in the Window menu.
  - Rename (short and long name), Duplicate and Delete work; the title is the short name, with the
    long name after it when there is one.
  - It is saved in the `.sheaf` project (`"type": "notes"` in `project.json`). With a Notes window
    active the menu bar is the one without a workbook or graph.
- **Not written:** an Origin project written by File > Export (`.opj`) leaves Notes windows out, with
  a warning. File > Save Window As refuses a Notes window (there is no window file for it yet). The
  AI live context reports the active Notes window's text.
- **Notes sheets (T188, 2026-11-20, not measured):** a sheet tab's right-click Add Notes as Sheet adds a
  Notes sheet after the workbook's last sheet, named `Notes` (then `Notes2`, ...), and shows it, one undo
  step ("Add Notes as Sheet"). It has no columns; the workbook shows the same plain-text box in place of
  the grid (written as one "Edit notes" step when the box lets go of the keyboard). It is renamed,
  duplicated and deleted like any sheet, saved in the `.sheaf` file (the sheet's `notes_text`), and left
  out of an `.opj` with a warning (a workbook of only Notes sheets keeps one empty sheet there).
- **Not done:** Origin's Notes formatting and its HTML mode.

## Sticky Notes (T421, 2026-11-24; Origin 2027's Sticky Notes, Sheaf's own, not measured)

Origin 2027 added searchable, collapsible sticky notes on graph windows, workbooks and single sheets, with colour
choices. Origin 2016 has none.

- **Adding:** Window > Sticky Note on a graph; Window > Sticky Note on Book and Sticky Note on Sheet on a workbook
  (the sheet shown). A new note is empty, yellow and open (one undo step, "Add Sticky Note"); on a window or sheet
  that has one, the command opens it.
- **The strip:** a note is a strip in its colour across the top of its window: a graph's note above the page, a
  workbook's note and then the shown sheet's note above the grid. Open, it has Collapse, its name ("Note", "Book
  note" or "Sheet note"), five colour swatches (Yellow, Green, Blue, Pink, Orange; the note's own outlined), Delete
  Note, and the text box below ("Type a note" when empty). Collapsed, it is one line: Expand, its name and the text's
  first non-blank line ("(empty note)" when there is none), the indicator that the window has a note.
- **Undo:** a run of typing in one note is one step ("Edit Sticky Note"); a colour, collapsing or expanding, and
  Delete Note are a step each.
- **Search:** Find in Project searches the notes' text with the notes windows (the Notes switch), as attribute
  "Sticky Note" of the Graph, Book or Sheet.
- **Model:** `sheaf_core::sticky::StickyNote` (text, colour, collapsed) in `Graph::sticky`, `Workbook::sticky` and
  `Worksheet::sticky`, saved in the `.sheaf` file; a sheet's note moves with the sheet. `Project::add_sticky_note`,
  `edit_sticky_note`, `remove_sticky_note`, `sticky_notes`.
- **Not done:** notes on matrices and layout pages, several notes per window, placing a note inside the window, an
  indicator in Project Explorer, and notes in `.opj` files.

## Linked reports (T439, 2026-11-25; Origin 2019b's linked HTML/Markdown reports, Sheaf's own, not measured)

Origin 2019b builds reports with live links to result values, metadata and figures and exports them as web pages.
Origin 2016's Notes window has no links. In Sheaf a Notes window's text is the report.

- **Links:** `%([Book1]Sheet1,B,3)` (Origin's worksheet cell substitution, with its optional format, `,.2`) is that
  cell; `$(formula)` is the first value of a formula read against the project, such as
  `$(mean([Book1]Sheet1!col(B)))`. Parentheses inside quotes do not end a link. A link that cannot be read shows
  `[link error: <the formula's message>]` in its place. `$` or `%` not followed by `(` is plain text.
- **Figures:** a line that is only `![caption](Graph1)` is graph Graph1 (short name, ignoring case) as a figure.
- **Preview:** the Notes window has Edit and Preview above its text. Preview shows the report resolved, so the
  values follow the project: `# ` lines as headings, `## ` and `### ` lines bold, a figure as a box with
  "Figure: Graph1" (or "Figure: no graph is called X") and its caption in italics.
- **Export Report...:** asks for a file (Web page `.html`, or Markdown `.md`). The figures' graphs are written beside
  it as `<file name>_<graph>.png` at 150 dpi. The HTML has the window's title, `#`/`##`/`###` headings, paragraphs
  split by blank lines (lines joined with line breaks), escaped text and each figure as an image with its caption.
  The Markdown is the resolved text with the figure lines pointing at the PNG files.
- **Not done:** reusing a report over a batch of files, figures drawn inside the Preview, links by fit-report
  name, metadata links, and rich text in the HTML.

