# T432: Named blocks

**Status:** done 2026-11-24 (Claude Windows "claude:1", expression-language "Named cells"), not measured (Origin 2022
feature). Define Name... on cells of several adjacent columns names the block; statistics (`total`, `mean`, `count`,
`sd`, `max`, `min`, ...) take every cell of it, and a formula using it depends on all its columns.
`NamedCell::last_column`, `Project::define_block`, `sheaf_expr::Host::named_block`.
**Size:** S. **Source:** HANDOFF C, the rest of T400.

## Tests

- `crates/expr/src/tests.rs`: `a_named_block_takes_every_cell_of_its_columns_in_statistics`.
- `crates/core/src/named.rs`: `a_named_block_feeds_statistics_and_follows_all_its_columns`.
- `crates/app/src/app/ui_tests.rs`: `define_name_on_selected_cells_of_a_column_names_the_range` (a two-column block).

## Left

- Project-wide names, `.opj` and Excel names (a name manager: done in T434).
