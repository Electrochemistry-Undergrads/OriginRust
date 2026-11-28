# T510: Directed network plots

**Status:** done 2026-11-27 (Claude Windows "claude:3", graphing 14.33 "Directed"). Left item of T383 (claude:1's);
Origin 2021b's network plot was not measured.
**Size:** S. **Source:** T383 "Left".

## What it does

A network plot can draw its edges as arrows from source to target (`sheaf plot -t network --directed`, saved with the
plot as `SankeyStyle::directed`); pairs linked both ways get two arrows side by side.

## Tests

- `crates/calc/src/network.rs`: `edges_are_undirected_and_summed_and_bad_rows_left_out` (the arcs).
- `crates/plot/src/network.rs`: `the_svg_has_an_edge_per_pair_a_node_per_name_and_the_names` (shafts and heads).
- `crates/cli/tests/sankey.rs`: the network test (`--directed` saved).

## Left

- A Plot Details switch for it; colouring by group, other layouts, label collision avoidance.
