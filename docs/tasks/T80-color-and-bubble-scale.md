# T80: New Color Scale, Add Color Scale, Add Bubble Scale (B3, slice 4)

**Status:** done 2026-11-16 (branch `color-scale`, Claude macOS, no Origin; graphing 27). Left: dragging a scale, a properties dialog, a Map link to a chosen plot, a CLI flag, scales for the other plot kinds.
**Size:** M. **Spec:** [graphing](../spec/graphing.md) section 27 and 14.16. **Inventory:** Graph > New Color Scale (1); Graph legend: Add Color Scale, Add Bubble Scale (2).

## Goal

Enable the three greyed items. A colour scale (the bar hexbin and sunburst already draw) and a bubble scale
(Origin's size legend, measured in 14.16) become objects of the layer: added from the Graph menu or the legend's
context menu in one undo step, drawn in every export, selectable and deletable, saved in `.sheaf`.

## Origin facts

Only the bubble size legend is measured (graphing 14.16). The rest is from Origin's help (colorscale, bubblescale
pages, [W]); defaults, placement and enabling rules are undocumented, so Sheaf's choices are marked unmeasured in
graphing 27.

## Where

- Model: `crates/core/src/layer_scale.rs`, `Layer::scales` (serde default, skipped while empty).
- Drawing: `crates/plot/src/layer_scale.rs` (reuses `hexbin::draw_color_bar`, `sunburst::scale_ticks`,
  `plots::z_range`), `ObjectKind::Scale`, room in `layout::label_edges`.
- App: `Cmd::NewColorScale` with `MenuContext::can_color_scale`, `menus::graph_legend_menu(layer, can_color,
  can_bubble)`, `GraphOp::AddScale` / `DeleteScale`, scale context menu, selection JSON.
- `.opj`: the writer warns and leaves them out (ADR 0008).

## Don't

Add Legend Title, New Table, Function Graph, display range/markers, Copy/Paste Format (T72), New Bubble Scale.
