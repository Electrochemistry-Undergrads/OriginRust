# T423: Interactive formula references

**Status:** done 2026-11-24 (Claude Windows "claude:1", gui-look "Interactive formula references"), not measured
(Origin 2025b feature). While Set Column Values is open, a new selection in a sheet is inserted into the formula at
its cursor: `col(B)`, `col(B)[3]` or `col(B)[2:5]`, with `Sheet2!` or `[Book2]Sheet1!` in front when elsewhere.
**Size:** S. **Source:** HANDOFF C, origin-2026-2027-gaps "Interactive formula references".

## Tests

- `crates/app/src/app/ui_tests.rs`: `choosing_a_column_or_a_cell_while_setting_values_inserts_its_reference`.

## Left

- Dragging over several columns (a block), the same in the Before Formula Scripts box and other formula boxes.
