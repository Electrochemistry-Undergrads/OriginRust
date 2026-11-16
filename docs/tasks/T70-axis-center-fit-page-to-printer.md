# T70: Axis Center and Fit Page To Printer (B3, slice 2)

**Status:** in progress 2026-11-15 (branch `b3-small`, Claude Windows, session fe4b37eb).
**Size:** S. **Priority:** wave 4 (B3). **Spec:** graphing (axis context menu; page sizing next to
Fit Page to Layers, 15.7d). **Inventory:** Graph axis: Center; Fit Page To Printer [Graph].

## Origin facts (measured 2026-11-15, Origin 2016, scratch `re-workspace/scratch/t70`)

- **Axis right-click menu:** Hide Axis, Hide Grids, Rescale To Show All (Ctrl+R), Axis Zoom In,
  Axis Zoom Out, Center. **Center** keeps the axis span and moves the range so that the value under
  the right-click becomes its middle: X 0..11 right-clicked at about 5.42 gave -0.08..10.92; Y
  -25..125 right-clicked at about 55 gave -20..130. Only that axis changes.
- **Format menu (graph):** Page Properties... (F2), Layer Properties..., Plot Properties..., Snap
  Layers to Grid, Snap Objects to Grid, **Fit Page To Layers...** (it opens the `pfit2l` window,
  "Fit Page to Layers: pfit2l", the same as the Graph menu item), **Fit Page To Printer**, Axes,
  Axis Tick Labels, Axis Titles, Programming Control..., Object Properties....
- **Fit Page To Printer** acts at once: the stock page 6432 x 4923 dots (10.72 x 8.205 in at 600
  dpi) became 6400 x 4800 (10.667 x 8.0 in): the default printer's printable area on landscape
  Letter (11 x 8.5 in less 1/6 in left and right and 1/4 in top and bottom). The layers kept their
  frames in percent of the page, so they shrink or grow with it.
- **Legend > Add Legend Title** is not in Origin 2016's legend menu (Categorical Values..., Point by
  Point..., Data Plots, Text Color Follows Plot, Reverse Order, Show Legend for Visible Plots Only,
  Hide Legend for Fitted Curves, Reconstruct Legend, Update Legend..., Arrange in Vertical/Horizontal);
  it belongs to a later Origin and cannot be measured here: it stays `B3`.

## Where

- Center: `crates/app/src/menus.rs` axis context menu (`grey("Center")` near Axis Zoom In/Out,
  `GraphOp::AxisZoom`); the right-click position in axis units comes from the graph view
  (`graph_view.rs`, the existing axis zoom path). Pure range arithmetic in core or app with tests.
- Fit Page To Printer: `menus.rs` Format menu `grey("Fit Page To Printer")`; page size from the
  default printer (`crates/app/src/winprint.rs` already reads `GetDeviceCaps` for printing) with the
  project's Page Setup paper and the page's orientation; elsewhere the Page Setup paper less the
  measured edges (12 pt left/right, 18 pt top/bottom). Frames unchanged. One undo step.
- Also fix T60's Format > "Fit Page To Layers": Origin's item has dots and opens the window.

## Done when

Both items do what Origin's do, with UI tests (Center on X and Y, one undo step; Fit Page To
Printer page size and unchanged frames, one undo step; Format > Fit Page To Layers... opens the
window), inventory updated, spec written.
