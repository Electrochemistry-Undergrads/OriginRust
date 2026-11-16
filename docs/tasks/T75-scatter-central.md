# T75: Scatter Central (B4, slice 3)

**Status:** in progress 2026-11-15 (branch `scatter-central`, Claude Windows, session fe4b37eb).
**Size:** S-M. **Inventory:** Plot > Symbol > Scatter Central (1).

## Origin facts (measured 2026-11-15, Origin 2016, scratch `re-workspace/scratch/t71`)

- Plot > Symbol > Scatter Central makes a scatter graph (symbols, no line, `get %C -l` 0) whose axes
  cross in the middle of the layer: `layer.x.position = -50` and `layer.y.position = -50` with
  `posType = 1` (percent), against 0 and 0 for the plain Scatter. The X axis line, ticks and tick
  labels stand half-way up the frame, the Y axis half-way across; the frame is drawn as a box round
  the layer; the Y axis title stands at the top end of the Y axis line. Ranges as for Scatter (X 0..10,
  Y -10..90 for 9 points of x²).
- The Y axis position is T61's `Axis::offset_pct` (graphing 21.3: "% From Left", negative inward,
  -50 = middle, in percent of the X axis length); for the X axis the same in percent of the Y axis
  length ("% From Bottom").

## Where

`crates/core/src/graph.rs` `Axis::offset_pct` (today documented as used by the Y axis only), the plot
layout of axes (`crates/plot`, where T61 moves the Y axis line, ticks, labels and title with the
offset; do the same for X), hit-testing of the moved X axis, a template `scatter-central` in
`core/template.rs` with the frame box shown as in the picture, the menu item, tests, graphing spec.
