# T09: Set Column Values dialog in Origin's layout

**Status:** main dialog done, merged (`20c37e1`). Find/search done (2026-11-14, branch
`codex/set-values-function-search`): Find and Function > Search Functions open a searchable list
from the existing Function-menu catalog, with descriptions/examples and insertion at the formula
cursor through Add Function, double click or Enter. Cancel/Escape leave the formula and data
unchanged; Apply remains an undoable worksheet operation. Based on Origin's published search
behavior; the new search-window layout is not measured against installed Origin 2016.
Before Formula Scripts remains read-only; the multi-column variant is optional and still open.
Validation: formatting, strict workspace/all-target Clippy and the full workspace tests pass
on integrated `1500062` plus this change: 2,030 passed, three existing ignored tests. Six new
regressions cover matching, cursor insertion, Apply/undo/redo, cancellation/no results,
Escape/Enter, Unicode cursor positions and single/double-click behavior.
**Size:** M.
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
- Set Multiple Columns Values (Ctrl+Shift+Q) uses the same dialog for several columns; done by T53
  (`docs/tasks/T53-set-multiple-columns.md`), no longer greyed in menus.rs.

## Done when

Screenshot (`--show set-values --screenshot`) matches the spec layout; UI tests for Apply enabling,
Recalculate drop-down and row range; checks pass.
