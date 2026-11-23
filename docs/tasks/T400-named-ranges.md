# T400: Named ranges

**Status:** done 2026-11-23 (Claude Windows "claude:1"), not measured (Origin 2022 feature). Several selected
cells of one column can be named with the data cell's Define Name...; formulas use the name where
`col(A)[first:last]` goes (`mean(Name)`, `total(Name)`, `cumsum(Name)`, `max(Name)`) and depend on its column
(`NamedCell::last`, `Project::define_range`, `sheaf_expr::Host::named_range`). Saved in `.sheaf`.
**Size:** S. **Spec:** `docs/spec/expression-language.md`, "Named cells". Follows T396 (named cells).

## Tests

- `crates/expr/src/tests.rs`: `a_named_range_stands_for_rows_of_a_column`.
- `crates/core/src/named.rs`: `a_named_range_feeds_statistics_and_follows_its_column` (another sheet, recalculation,
  a bare range refused, `.sheaf` serialization).
- `crates/app/src/app/ui_tests.rs`: `define_name_on_selected_cells_of_a_column_names_the_range`.

## Left

- Blocks of several columns, project-wide names, a name manager, `.opj` and Excel names.
