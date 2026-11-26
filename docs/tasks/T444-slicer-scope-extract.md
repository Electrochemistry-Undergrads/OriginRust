# T444: Graph Slicer layer scope and Extract (rest of T406)

**Status:** done 2026-11-25 (Claude Windows "claude:work", graphing "Graph Slicer"). On a graph of several layers,
the Graph Slicer window has a Scope choice: Page, or one layer, whose plots alone are sliced. Extract applies the
slicer and copies the rows it shows of its sheet to a new workbook, which becomes active (one undo step).
**Size:** S. **Source:** T406 "Left".

## Left

- Several slicer columns, computed plots reading the slicer, the slicer in .opj files. (Plot scope: T467.)

## Tests

- `crates/core/src/slicer.rs`: Extract's rows and names, layer scope, a scope kept across new values.
- `crates/app/src/app/ui_tests/function_graph.rs`: `graph_slicer_shows_the_ticked_values_in_one_undo_step` (Extract).
