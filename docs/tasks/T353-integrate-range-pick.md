# T353: Integrate Ranges picked on the graph

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", analysis-tools Integrate Ranges). The parity row
"Per-peak integration" (P0) said there was no picking on a graph. With a graph active, Analysis > Mathematics >
Integrate Ranges now turns the Data Reader on: a double-click on a data point of the active plot (or Enter) picks a
limit, every two limits make one range, an optional tick subtracts the line through each range's ends, and Integrate
runs Integrate Ranges over the picked rows as one undo step. From a workbook the typed window stays.
**Size:** S. **Spec:** analysis-tools (Integrate Ranges paragraph).

## Left

- Drawing the picked ranges on the graph while picking; free (not snapped) limits; Analyze Peaks' own limits
  moved on the graph.

## Tests

- `crates/app/src/app/ui_tests.rs`: `integrate_ranges_on_a_graph_picks_the_limits` (limits in either order, two
  ranges, areas, undo label).
