# T52: Slope annotation

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `slope-annotation`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Slope Annotation, Origin 2026b).

## Goal

A graph tool that marks a slope between two data points: pick two points of a plot, and a segment with
a label showing the slope is drawn and stays linked to those data points (recomputed when the data or
axes change). Useful for Tafel slopes on a log axis.

## Origin facts (docs, thin; not measured)

- 2026b, "Graph Annotation Improvements": "New Slope Annotation tool". [W:originlab.com/2026b]
- Tools toolbar entry: "Add a slope Annotation object". [W:origin-help/tools-toolbar]
- Release-note summary (search result, not opened): the slope is computed between two selected points
  from their coordinate values (ORG-32493).
- Everything else is Sheaf's own choice (unmeasured): what is drawn (segment between the two points plus
  a dashed rise/run triangle), label text (`slope = 1.234` with 4 significant digits; on a log10 axis the
  slope is per decade, e.g. `slope = 118.3 mV/dec` style using the axes' units when both have units —
  document), snapping to the nearest data point of the picked plot, the link to rows (row indices), label
  position (midpoint, offset perpendicular), style (plot colour, 1 pt, dashed legs).

## Where

- `crates/core/src/graph.rs`: a `SlopeAnnotation { plot, row_a, row_b, legs, label offset, style }` on
  the layer (serde default empty); `Graph::remap_columns` if it stores ColumnIds; `.opj` warns.
- `crates/plot`: draw segment, legs and label in data coordinates through the layer transform (log axes:
  slope computed in the transformed coordinates, i.e. per decade); hit-test object so it can be
  selected/deleted.
- `crates/app`: a Tools-toolbar / Graph-toolbar "Slope" tool (see `toolbar.rs`, `graph_view.rs` and the
  Data Reader `reader.rs` for point picking): click two points of a plot; Delete removes it; undo; a
  Properties dialog (legs on/off, decimals). CLI `sheaf plot --slope ROW_A,ROW_B` (optional), MCP option.

## Done when

Slopes can be added with the tool in the app, recompute when data change, show the right value on
linear and log axes (tests), save in `.sheaf`; spec section in graphing.md.

## Don't

- Don't change drawn shapes (T24, shape-ordering session), Data Reader behaviour or other annotations.
