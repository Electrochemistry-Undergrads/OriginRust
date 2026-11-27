# T487: Several Graph Slicer columns

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing "Graph Slicer"). Left item of T406/T444/T467 (claude:work's,
unclaimed); Origin 2025's Graph Slicer, Sheaf's own design.
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Graph Slicer": several slicer columns remain.

## What it does

Graph > Graph Slicer... has Add Slicer Column: further columns of the same sheet, each with its ticks and a Remove button. A row is
shown when every slicer column's cell holds a ticked value; Extract copies those rows. The columns are kept with the graph and shown
again when the window reopens.

## Tests

- `crates/core/src/slicer.rs`: `several_slicer_columns_show_the_rows_ticked_in_every_one`.
- `crates/app/src/app/ui_tests/function_graph.rs`: `graph_slicer_takes_a_second_slicer_column`.
