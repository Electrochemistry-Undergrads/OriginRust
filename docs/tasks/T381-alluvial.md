# T381: Alluvial diagrams

**Status:** done 2026-11-22 (Claude Windows "claude:1", graphing 14.32), not measured (Origin 2020 feature).
Plot > Categorical > Alluvial Diagram and `sheaf plot -t alluvial [--group A,B,C] [-y WEIGHT]`: the category
columns are stages and every row flows through its cells, counted or weighted; drawn with the Sankey layout of
T379 with each node pinned to its column's stage (`sheaf_calc::sankey::layout_in_stages`, `SankeyStyle::stages`).
**Size:** S. **Source:** HANDOFF C, origin-2026-2027-gaps "Sankey and alluvial diagrams".

## Tests

- `crates/calc/src/sankey.rs`: `fixed_stages_override_the_longest_path`.
- `crates/plot/src/sankey.rs`: counted flows, the same name in two stages as two nodes.
- `crates/cli/tests/sankey.rs`: counts, `--group` with `-y`, too few stages refused.
- `crates/app/src/app/ui_tests.rs`: `plot_categorical_alluvial_takes_the_sheets_text_columns_as_stages`.
