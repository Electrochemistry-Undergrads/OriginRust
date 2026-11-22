# T348: Subtract Straight Line picked on the graph

**Status:** done 2026-11-21 (Claude Windows "claude:gmail", analysis-tools 12). The parity row "Subtract straight
line" (P0) said the points were typed only, with no on-graph pick. On a graph, Analysis > Data Manipulation >
Subtract Straight Line now works as Origin's interactive `subtract_line`: the Data Reader goes on, a double-click on
a data point of the active plot (or Enter) picks an end of the line, and Subtract writes the curve minus the line
through the two picked points as a new Y column and draws it on the graph, one undo step ("Subtract straight
line"). From a workbook the typed window stays. Origin's own picking (screen point or data point) and whether it
replaces the data were not measured (it needs Origin's GUI).
**Size:** S. **Spec:** analysis-tools 12.

## Left

- Free (not snapped) points, a preview of the line while picking, replacing the data in place as Origin may do.

## Tests

- `crates/app/src/app/ui_tests.rs`: `subtract_straight_line_on_a_graph_picks_the_two_points` (picking, result
  values, drawn plot, undo, Cancel).
