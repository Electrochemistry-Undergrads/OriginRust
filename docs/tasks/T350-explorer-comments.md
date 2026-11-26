# T350: Comments column in the Project Explorer

**Status:** done 2026-11-25 (Claude Windows "claude:3"). T249's left item. The Project Explorer's window list
has a fifth column, Comments, holding the first line of each window's Comments (Window Properties, T249);
a click on its head sorts the list by it like the other columns. The full text stays the row's tooltip.
Origin's Explorer has such a column; its widths and order were not measured.
**Size:** S. **Spec:** feature-parity "Project Explorer".

## Tests

- `crates/app/src/panels/explorer.rs`: `the_list_cells_hold_name_long_name_short_name_type_and_comments`.
