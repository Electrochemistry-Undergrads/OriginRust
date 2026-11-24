# T406: Graph > Graph Slicer (Origin 2025, base slice)

**Status:** done 2026-11-23 (Claude Windows "claude:work", graphing "Graph Slicer"). Graph > Graph Slicer... picks a
category column of the graph's source sheet and ticks the values to show. Every plot from that sheet then draws only
the matching rows (page scope), without filtering the worksheet. It is one undo step and saves with the project.
Remove Slicer takes it off. Sheaf's own design (Origin 2016 has no slicer).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Graph Slicer".

## Left

- Layer and plot scope, several slicer columns, extracting the selected subset to a sheet, computed plots (histograms,
  box charts) reading the slicer, the slicer in .opj files.

## Tests

- `crates/core/src/slicer.rs`: the kept rows, other sheets unaffected, the worksheet untouched, save round trip.
- `crates/plot/src/grouped_tests.rs`: `a_graph_slicer_leaves_out_the_rows_of_unticked_values`.
- `crates/app/src/app/ui_tests/function_graph.rs`: `graph_slicer_shows_the_ticked_values_in_one_undo_step`.
