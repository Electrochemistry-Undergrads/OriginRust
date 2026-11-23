# T395: Measure Graph > New Table against Origin 2016

**Status:** done 2026-11-23 (Claude Windows "claude:work", graphing "Tables"). Origin 2016's Graph > New Table window
("Add a linked table to a graph or a layout") has Layer, Number of Columns 5 and Number of Rows 8 (each 1 to 9),
Table Title, Show Column Labels (off) and Table Name. Sheaf's New Table now starts with 8 rows of 5 cells and
grows to at most 9 rows and 9 columns.
**Size:** S. **Follows:** T136. **Spec:** graphing "Tables".

## Left

- Origin's table is linked to a worksheet; its title, column labels and look (Origin's OK left no table in the probe).

## Tests

- `crates/app/src/app/ui_tests/graph_table.rs`: `a_new_table_is_made_and_edited_cell_by_cell` (8 x 5 start).
