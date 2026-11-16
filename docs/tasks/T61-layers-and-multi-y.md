# T61: Adding layers, insets, the layer icon menu and graphs with three or more Y axes

**Status:** open. Written 2026-11-15 from the owner's priority "features like stacked plots" and
parity rows "Double-Y, multiple Y axes" (P0, partial) and "Inset, pictures in graphs".
**Size:** L (three slices). **Branch:** `layers-multi-y`. **Model:** Sonnet. Slice 1 needs no
Origin; the multi-Y templates' placement is measured on Windows.
**Spec:** `docs/spec/graphing.md` 1.3 (layer), 12 (double-Y and linked layers), ADR 0009.
**Greyed items closed (20):** Toolbars > Graph: Add Layer (Bottom-X Left-Y), (Top-X Left-Y),
(Bottom-X Right-Y), (Top-X Right-Y), Add Inset Graph; Graph > New Layer(Axes) and graph page > New
Layer(Axes): Horizontal Axis Layer..., Vertical Axis Layer...; Plot > Multi-Y: 3Ys Y-YY, 3Ys Y-Y-Y,
4Ys Y-YYY, 4Ys YY-YY, Multiple Y Axes...; Format > Layer Properties... [Graph]; layer icon menu:
Show Active Layer Only, Set Active Layer By Layer Icon Only, Delete Layer, Double click to open
Plot Setup, Layer Properties....

## Goal

1. **Add and delete layers on an existing graph.** The four Add Layer buttons add an empty layer
   over the active one with the named axes shown, linked to it as Origin links it (a right-Y layer
   shares the parent's X). Add Inset Graph adds a small layer inside the frame. Delete Layer removes
   a layer and re-points links. Then plots are added to the new layer with Layer Contents / Plot
   Setup, which exist.
2. **Layer icon menu and Layer Properties.** The greyed entries of the layer icon's menu work;
   Format > Layer Properties... opens Plot Details at the active layer.
3. **Three and four Y axes.** Plot > Multi-Y > 3Ys Y-YY, 3Ys Y-Y-Y, 4Ys Y-YYY, 4Ys YY-YY plot the
   selected Y columns each on its own Y axis (the letters around the dash say how many axes sit
   left and right of the frame), each axis in its curve's colour as Double-Y does. Multiple Y
   Axes... opens a window to choose plot type and axis side per column. In the app, a plot can be
   moved to the left or right axis (the CLI has `--y2`; parity row "Double-Y" names this gap).

## Today

- `Graph::layers`, `Layer.link` (`LayerLink { parent, x, y }`, the parent's index must be smaller),
  `Template::DoubleY` (`crates/core/src/template.rs`): the right axis is a linked layer.
- The layer icons, the active layer, Hide Other Layers (`GraphOp::HideOtherLayers`) and the layer
  icon menu exist (`crates/app/src/menus.rs`, `graph_view.rs`); Delete Layer is grey.
- An axis is drawn at the frame's edge; check whether an axis can be drawn offset from the frame
  (needed for the second axis on one side) before designing slice 3.
- The `.opj` writer writes several layers; links are kept in the companion (ADR 0008, 0009).

## Origin facts

- From Origin's documentation (paraphrased), not measured: the four Add Layer buttons add a linked
  layer with the named axes; the Multi-Y templates draw extra Y axes offset outwards from the frame,
  each with its own scale, in its curve's colour; Multiple Y Axes... is the `plotmyaxes` window.
- **Measure on Windows:** for each Add Layer button and inset, the new layer's frame, which axes
  and titles show, and the link (X, Y, dimension); for each Multi-Y template with 3 and 4 Y
  columns, the frame, each axis's offset from the frame (in % of the frame), colours, legend and
  how more columns than axes are assigned; the Multiple Y Axes window's controls; what "Horizontal
  Axis Layer..." and "Vertical Axis Layer..." open (they are in Sheaf's menu from the menu study;
  if Origin 2016 does not have them, record that and move them to B3); what Show Active Layer Only
  and Set Active Layer By Layer Icon Only change. Export a PNG per template as fixtures.

## Where

- `crates/core/src/graph.rs`: `add_layer(kind)`, `delete_layer(i)` (re-index links, refuse or
  re-parent children: measure Origin, else re-parent to the deleted layer's parent), an axis offset
  if missing; new `GraphOp`s in `crates/app/src/actions.rs`.
- `crates/core/src/template.rs`: `ThreeY_Y_YY`-style templates built as DoubleY is; CLI names
  `-t 3ys-y-yy`, `3ys-y-y-y`, `4ys-y-yyy`, `4ys-yy-yy` (through `sheaf_ops::args`, so MCP gets them).
- `crates/plot`: draw an axis offset from the frame; margins grow for it (`margin.rs`).
- `crates/app`: toolbar buttons (`Bar::Graph` in `toolbar.rs`), menus, the Multiple Y Axes window,
  a per-plot "Plot on: left / right / layer n" choice in Plot Details or Layer Contents.
- `crates/io/src/opj/write`: the new layers are ordinary layers; axis offsets go to the companion
  unless the format's field is known.

## Steps

1. Add layer (four kinds), inset, delete layer, layer icon menu items, Layer Properties; model
   tests (links after delete, undo), UI tests, golden image of a graph with an inset.
2. Per-plot axis choice in the app; axis offset drawing with tests.
3. The four Multi-Y templates and the Multiple Y Axes window; CLI and MCP; golden images.
4. Origin pass (Windows) for everything under "Measure"; record in graphing 12; fix differences.
   Delete the 20 items from greyed-items.md as each slice enables them.

## Done when

A second, third and fourth Y axis can be made from the Plot menu and by adding layers to a finished
graph; an inset can be added; a layer can be deleted; all of it survives save and reopen and exports
on every backend; the checks pass.

## Don't

- Don't arrange existing layers in grids or merge graphs here (T60).
- Don't break Double-Y: its goldens and its measured colours (graphing 12.5) stay.
- Don't allow a link to a later layer (`parent` must stay smaller than the layer's index).
