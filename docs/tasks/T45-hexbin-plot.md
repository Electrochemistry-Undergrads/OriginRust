# T45: Hexbin plot

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `hexbin`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Hexbin, binned scatter and UpSet plots,
Origin 2027; the hexbin part only).

## Goal

Plot > Contour/Heat Map > Hexbin draws XY data as hexagonal bins coloured by the number of points in
each bin, with Origin's bin controls and a colour scale.

## Origin facts (docs, not measured)

- New graph types in 2027: "Hexbin Plot" and "Binned Scatter Plot". [W:Origin 2027 page]
- Menu "Plot > Contour, Heatmap: Hexbin", template `Hexbin.OTPU`; input XY or XYZ columns, a virtual
  matrix or a matrix; "the color represents the number of data points within each bin".
  Bin controls per axis: Min Center, Max Center, Bin Spacing, Number of Bins. "Prefer Regular
  Hexagons" is checked by default and only works for square (1:1) layer frames; "Derive Y from X"
  (default) or "Derive X from Y" picks the axis computed from the other. [W:origin-help/hexbin-plot]
- Not documented (Sheaf's choices, unmeasured): default number of bins, colour map (Sheaf reuses the
  colour-mapped scatter ramp in `crates/plot/src/plots.rs`), empty bins (not drawn), orientation
  (pointy-top), colour scale placement and labels, XYZ/matrix input (left), Binned Scatter (left; no
  Origin page found).

## Where

- `crates/calc`: hexagonal binning (centres from Min/Max Center and spacing, nearest-centre
  assignment on the offset-row lattice, counts).
- `crates/core`: `Template::Hexbin` / plot kind with bin settings (serde defaults), `.opj` warns.
- `crates/plot`: hexagons in data coordinates (through the layer transform), colour by count, a colour
  scale bar beside the layer (Sheaf's own: gradient + count ticks).
- `crates/app/src/menus.rs`: enable "Hexbin" in the Contour/Heat Map submenu (leave the other contour
  items greyed); Plot Details bin controls; CLI `-t hexbin`, MCP.

## Done when

Hexbin graphs draw from the menu/CLI with counts matching a reference binning (numpy) in calc tests,
regular hexagons on square frames, bin controls in Plot Details, a colour scale; spec section.

## Don't

- Don't touch the unmerged matrices/heatmap/contour worktree (`originrust-3a`) or change the
  colour-mapped scatter's colours.
