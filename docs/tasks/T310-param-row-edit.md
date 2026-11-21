# T310: Rename and delete a user parameter row

**Status:** done 2026-11-21 (Claude Windows "claude:3"). T309's left item. Right-clicking a user
parameter row's title (left of the label rows) offers Rename Parameter Row..., which opens T309's
window with the current name (an empty or taken name is refused and the window stays open; the values
move with the row), and Delete Parameter Row, which removes the row and its cells from every column.
One undo step each ("Rename parameter row", "Delete parameter row"). Formulas that read the row by name
follow the existing parameter-row recalculation rules.
**Size:** S. **Spec:** feature-parity "Parameter, user parameter, F(x) rows".

## Tests

- `crates/core/src/clipboard.rs`: `a_parameter_row_is_renamed_and_deleted_in_every_column`.
- `crates/app/src/app/ui_tests/sheet_tab.rs`: `a_parameter_row_is_renamed_and_deleted_from_its_title`.
