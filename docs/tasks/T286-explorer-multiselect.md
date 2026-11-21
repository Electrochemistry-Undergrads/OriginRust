# T286: Several windows highlighted in the Project Explorer

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Project Explorer" (P0) gap
"no free multi-select". In the window list a plain click highlights one row, Ctrl+click adds or
removes a row, Shift+click highlights the run from the last clicked row (which stays the start of
the run), and Ctrl+A highlights every window of the folder (or of Favorites). Delete, and the row
menu's Show, Hide and Delete, act on every highlighted window when the clicked row is one of them; a
right-click or a drag on a row that is not highlighted takes it alone. Dragging a highlighted row onto
a folder moves them all. Rename, Duplicate, Export and Properties act on the clicked row. The usual
Windows list behaviour; not measured against Origin.
**Size:** S. **Spec:** `docs/spec/ui-command-map.md` (Ctrl+A), feature-parity "Project Explorer".

## Tests

- `crates/app/src/panels/explorer.rs`: `ctrl_and_shift_clicks_pick_windows_as_windows_lists_do`.
- `crates/app/src/app/ui_tests.rs`: `ctrl_and_shift_clicks_in_the_explorer_pick_several_windows`,
  `the_explorer_keys_f2_delete_and_ctrl_a_work_on_the_highlighted_window`.

## Left

- Search and filters in the Explorer.
