# T383: Network plots

**Status:** done 2026-11-22 (Claude Windows "claude:1", graphing 14.33), not measured (Origin 2021b feature).
Plot > Categorical > Network Plot and `sheaf plot -t network [--group S,T] [-y WEIGHT]`: undirected edges of a source
and a target column, counted or weighted, placed by a deterministic force-directed layout; nodes sized by weighted
degree, edges by weight. `PlotKind::Network`, `sheaf_calc::network`.
**Size:** M. **Source:** HANDOFF C, origin-2026-2027-gaps "Network and grouped marginal plots".

## Tests

- `crates/calc/src/network.rs`: summed undirected edges, bad rows, repeatable layout inside the area, groups kept
  together, one node.
- `crates/plot/src/network.rs`: names, edges and no axes in the SVG.
- `crates/cli/tests/sankey.rs`: `a_network_plot_takes_its_ends_and_an_optional_weight`.
- `crates/app/src/app/ui_tests.rs`: `plot_categorical_network_joins_the_sheets_first_two_text_columns`.

## Left

- Directed edges, colouring by group, other layouts, label collision avoidance.
