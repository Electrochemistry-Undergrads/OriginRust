# T49: Parallel Plot and Parallel Index

**Status:** done 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `parallel-plot`. Spec: graphing.md 14.25. Not done: Parallel Sets, curvature, colour map choice, axis reordering, Origin's real defaults.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Violin and parallel-coordinate plots,
Origin 2019; the parallel half; violin is T37).

## Goal

Plot > Categorical > Parallel Plot / Parallel Index draws each row as a polyline across one vertical
axis per selected column, coloured by a numeric column (Parallel Plot) or by a categorical group
column (Parallel Index), with per-axis or shared scaling.

## Origin facts (docs, not measured)

- Parallel Plot: Plot > Categorical: Parallel Plot (also Plot > 2D: Parallel Plot); at least two
  columns, all treated as Y; colour scale legend; numeric colour usually mapped by a column (By Points
  tab); template `Parallel.otpu`. [W:origin-help/parallel-plot]
- Parallel Index: Plot > Categorical: Parallel Index; at least three columns plus a grouping column of
  categorical data that creates the legend and assigns plot colour. [W:origin-help/parallel-index]
- Axes dialog: "Rescale Each Axis Independently": checked, each axis has its own From/To; cleared,
  all axes share one range from the variable with the greatest range. Axis title default `%(?Y)`
  (Long Name or Short Name plus Unit). Dragging a column onto the plot adds a last axis.
  [W:origin-help/axes_dialog_parallel_coordinates]
- Plot Details Parallel tab: Style, Width, Transparency 0-100 (0 opaque); Curvature 0-100 (used by
  alluvial/sets). [W:origin-help/pd-dialog-parallel-tab]
- Not documented (Sheaf's choices, unmeasured): defaults of Rescale (Sheaf: on), width, transparency,
  straight lines for Parallel Plot/Index, colour map (Sheaf reuses the shared colour ramp), axis tick
  layout, missing values (the polyline breaks at a missing cell). Parallel Sets left.

## Where

- `crates/core`: `Template::Parallel`, `Template::ParallelIndex` / plot kind with per-plot settings
  (rescale, width, transparency, colour column or group column); `.opj` warns.
- `crates/plot`: one vertical axis per column (ticks, title from long name + unit), polylines through
  the per-axis normalised values, colour scale (reuse T45's colour scale bar) or group legend.
- `crates/app`: Plot > Categorical menu items (enable/add), Plot Details Parallel tab; CLI
  `-t parallel` / `-t parallel-index`, MCP.

## Done when

Both plot types draw from the menu/CLI with correct per-axis and shared scaling, colouring, axis
titles, legend/colour scale, missing values; tests and graphing.md section.

## Don't

- Don't change the colour-mapped scatter or hexbin colours, or other plot kinds' output.
