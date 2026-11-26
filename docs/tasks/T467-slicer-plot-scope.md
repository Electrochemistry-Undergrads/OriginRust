# T467: Graph Slicer plot scope (rest of T406/T444)

**Status:** done 2026-11-25 (Claude Windows "claude:work", graphing "Graph Slicer"). With a layer scope chosen, the
Graph Slicer window offers All plots or one plot of that layer, and then only that plot is sliced.
**Size:** S. **Source:** T444 "Left".

## Tests

- `crates/core/src/slicer.rs`: `a_slicer_keeps_the_rows_of_the_ticked_values_for_plots_of_its_sheet` (plot scope).
