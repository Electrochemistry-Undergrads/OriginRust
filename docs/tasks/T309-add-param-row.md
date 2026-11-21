# T309: Add a user parameter row

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Parameter, user parameter, F(x)
rows" (P0, gap "no way to add a new parameter row"). The menu of the sheet's corner (above the row
numbers) has Add User Parameter Row...: a small window asks the row's name; OK gives every column of
the sheet that row, empty and ready to type into (`Worksheet::add_parameter_row`), in one undo step
("Add parameter row"). An empty name or one the sheet already has (any case) is refused and the window
stays open. Sheaf's window; Origin's User Defined Parameters dialog was not compared.
**Size:** S. **Spec:** feature-parity "Parameter, user parameter, F(x) rows".

## Tests

- `crates/core/src/clipboard.rs`: `a_new_parameter_row_reaches_every_column_and_names_are_unique`.
- `crates/app/src/app/ui_tests/sheet_tab.rs`: `a_user_parameter_row_can_be_added_from_the_corner_menu`.

## Left

- Renaming and deleting a parameter row: done in T310.
