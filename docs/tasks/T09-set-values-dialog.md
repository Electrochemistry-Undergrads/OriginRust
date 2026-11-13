# T09: Set Column Values dialog in Origin's layout

**Status:** done on branch set-values-dialog, not merged (menu line, row range, column arrows, Function menu with descriptions, Recalculate drop-down, Apply grey until changed; Before Formula Scripts box is shown but inert; multi-column variant and the find button left). **Size:** M.
**Branch:** `set-values-dialog`. **Model:** Sonnet.
**Detail:** audit B "Set Column Values". **Spec:** `docs/spec/gui-look.md` (search "Set Values").

## Today (`crates/app/src/dialogs/set_values.rs`)

Title "Set column values: B"; Recalculate as a None/Manual/Auto radio row; rows All/From radios;
buttons OK, Apply, Cancel; Apply always enabled; resizable.

## Origin

- Title `Set Values - [Book1]Sheet1!Col("Signal")`; about 425x335, not resizable.
- Menu line: Formula, wcol, Col(A), Function, Variables, Options. Function menu inserts a function;
  a description pane shows its help (write Sheaf's own text, not Origin's).
- Formula navigation buttons (previous/next column) and a collapse arrow for the "Before Formula
  Scripts" box.
- Recalculate: a drop-down, default Manual. Rows: "From <auto> To <auto>".
- Buttons OK, Cancel, Apply; Apply grey until something changes. Same order fix for the text object
  dialog (`text_object.rs`).
- Set Multiple Columns Values (Ctrl+Shift+Q, greyed in menus.rs) uses the same dialog for several
  columns; optional here.

## Done when

Screenshot (`--show set-values --screenshot`) matches the spec layout; UI tests for Apply enabling,
Recalculate drop-down and row range; checks pass.
