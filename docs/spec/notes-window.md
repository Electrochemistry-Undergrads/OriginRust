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
- **Not done:** Add Notes as Sheet, Origin's Notes formatting and its HTML mode.
