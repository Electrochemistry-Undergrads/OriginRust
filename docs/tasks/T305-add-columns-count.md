# T305: Add New Columns asks how many

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Add, delete, move columns" (P0,
gap "no count dialog or swap"; Swap Columns already existed, T86). Column > Add New Columns..., the
column header's Add New Columns... and Ctrl+D open "Add New Columns" with "Number of columns to add"
(1 to 1000, starting at 1); OK adds that many Y columns at the end of the active sheet in one undo step
("Add column" / "Add columns"). The Standard toolbar's Add New Columns button adds one column without
asking, as Origin's button does. The window's layout is Sheaf's (Origin's dialog is not scriptable).
**Size:** S. **Spec:** feature-parity "Add, delete, move columns".

## Tests

- `crates/app/src/app/ui_tests.rs`: `add_new_columns_asks_how_many_and_the_toolbar_adds_one` and the
  updated Ctrl+D, Undo and column-menu tests.
- `crates/app/src/toolbar.rs`: `every_button_with_a_command_sends_the_command_of_the_menu_entry_it_is_named_after`
  (the button's one-column command is the documented exception).
