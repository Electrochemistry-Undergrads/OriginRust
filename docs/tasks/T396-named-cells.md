# T396: Named cells

**Status:** done 2026-11-23 (Claude Windows "claude:1"), not measured (Origin 2022 feature). A data cell's
Define Name... names the cell; formulas of the workbook read the name as the cell's value and depend on its
column (`Workbook::names`, `Project::define_name`, the formula host's variables). Saved in `.sheaf`.
**Size:** S. **Spec:** `docs/spec/expression-language.md`, "Named cells".

## Tests

- `crates/core/src/named.rs`: the name rules; a formula reading a name, its recalculation, redefining, deleting.
- `crates/app/src/app/ui_tests.rs`: `define_name_on_a_cell_lets_formulas_read_it`.

## Left

- Named ranges of several cells (in functions such as `sum`), project-wide names, a name manager, `.opj` and
  Excel names.
