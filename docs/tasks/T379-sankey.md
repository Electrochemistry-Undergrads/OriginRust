# T379: Sankey diagrams

**Status:** done 2026-11-22 (Claude Windows "claude:1", graphing 14.32), not measured (Origin 2020 feature).
Plot > Categorical > Sankey Diagram and `sheaf plot -t sankey [--group SOURCE,TARGET]`: flows of a value column
between the nodes of a source and a target column, stages by longest path, nodes sized by flow, curved bands in the
source's colour; `PlotKind::Sankey`, `SankeyStyle`, `sheaf_calc::sankey`.
**Size:** M. **Source:** HANDOFF C, origin-2026-2027-gaps "Sankey and alluvial diagrams".

## Tests

- `crates/calc/src/sankey.rs`: stages, summed links, band widths, nodes filled by their bands, cycles and bad rows.
- `crates/core/src/template.rs`: `a_sankey_plot_takes_two_text_columns_and_the_flow`.
- `crates/plot/src/sankey.rs`: bands, names, no axes in the SVG.
- `crates/cli/tests/sankey.rs`; `crates/app/src/app/ui_tests.rs`: `plot_categorical_sankey_uses_the_sheets_text_columns_and_the_flow`.

## Left

- Alluvial diagrams, crossing reduction, value labels, Plot Details controls, `.opj` round trip.
