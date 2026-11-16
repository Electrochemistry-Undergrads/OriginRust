# T62: Stack and offset leftovers (Offset Grouped Data menu, Cumulative, Stacked Histograms, waterfall colours)

**Status:** open. Written 2026-11-15 from the owner's priority "features like stacked plots".
**Size:** M (three independent slices). **Branch:** `stack-leftovers`. **Model:** Sonnet. Slice 1
needs no Origin (the Stack tab is measured); slices 2 and 3 are measured on Windows.
**Spec:** `docs/spec/graphing.md` 13 (stacked lines and waterfall; 13.14 Stack tab, measured), 14.12
(stacked columns), ADR 0010.
**Greyed items closed (8):** Graph > Offset Grouped Data in Layer: None, Cumulative, Constant, Auto,
Individual; Plot > Statistics > Stacked Histograms; Plot > Y-offset/Waterfall: Waterfall Y:Color
Mapping, Waterfall Z:Color Mapping.

## Goal

1. **Graph > Offset Grouped Data in Layer** sets the active layer's stack mode from the menu, with
   a tick on the current mode: the same setting as Plot Details > Stack. Cumulative works for line
   and scatter plots.
2. **Stacked Histograms**: one histogram per selected Y column, in panels stacked vertically that
   share the X (bin) axis.
3. **Waterfall with colour mapping**: the waterfall's curves coloured along a colour map, by Y value
   or by each curve's Z value (its parameter), with a colour scale.

## Today

- `Layer.stack: Stack` (`None`, `Auto { gap_percent }`, `Constant { step }`, `Individual`,
  `Waterfall { dx_percent, dy_percent }`) and `Layer.plot_offsets` (`crates/core/src/graph.rs`,
  `stack.rs`; drawing in `crates/plot/src/stack.rs`). Plot Details has the Stack tab (graphing
  13.14). The Graph menu's submenu is `grey_sub("Offset Grouped Data in Layer", ...)` in
  `crates/app/src/menus.rs`, although graphing 13 planned it live ("None, Constant..., Auto... and
  Individual. Cumulative stays disabled until GP-COL-01 exists"; stacked columns exist now).
- Histogram and box plots, the panel builder (`crates/core/src/panels.rs`) and the Stack window
  exist. Colour maps and a colour scale exist for colour-mapped scatter, hexbin and parallel plots.

## Origin facts

- Measured (graphing 13.14): the Stack tab's Offset group is None, Cumulative, Incremental,
  Constant (a box), Auto (Gap (%) 8 when chosen on the tab) with further switches; Cumulative adds
  each plot's Y values point by point to those of the plots before it (graphing 13).
- **Measure on Windows:** whether the menu's Constant and Auto open a prompt or use the tab's
  values; Cumulative on lines with different X values or missing rows; the Stacked Histograms
  template (panels' order, shared bins or per-panel bins, gap, axis titles, legend); both waterfall
  colour-mapping templates (which value colours the line, default palette and levels, the colour
  scale's place). Export one PNG each as fixtures.

## Where

- Slice 1: `menus.rs` (ticked items through an existing or new `GraphOp` that sets `Layer.stack`),
  `Stack::Cumulative` in core with drawing and automatic range in `crates/plot/src/stack.rs` and
  `crates/core/src/autorange.rs`; CLI `--stack cumulative` if `--stack` takes modes.
- Slice 2: a template in `crates/core/src/template.rs` built on the panel builder with one
  histogram per panel; menu item; `sheaf plot -t stacked-histograms`.
- Slice 3: a per-plot or per-layer colour-map setting for waterfall lines reusing the shared colour
  scale; menu items; CLI names.
- `.opj`: write what the writer can (13.14 lists what was not compared), else warn.

## Steps

1. Menu items live with ticks; Cumulative; tests (mode round trip, ticks follow Plot Details, undo,
   golden for Cumulative on three lines). Delete the five items from greyed-items.md.
2. Stacked Histograms from measurements (or Sheaf's labelled choices on macOS); golden; delete the
   item.
3. Waterfall colour mapping; goldens; delete the two items.

## Done when

The stack mode can be changed from the Graph menu; three columns plot as stacked histograms; a
waterfall can be colour mapped; saves and exports keep all three; the checks pass.

## Don't

- Don't change Auto, Constant, Individual or Waterfall placement (measured; `places` in
  `crates/core/src/stack.rs` is shared with the `.opj` writer).
- Don't build 3D Waterfall (B15).
- Don't confuse this with worksheet Stack Columns (a reshape tool) or stacked columns/areas
  (graphing 14.12, done).
