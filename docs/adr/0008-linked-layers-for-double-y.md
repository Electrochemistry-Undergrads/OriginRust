# ADR 0008: A second Y axis is a linked layer

Status: accepted (2026-11-12)

## Context

The owner needs figures with two Y axes over one X axis (current and charge against time, capacity
and coulombic efficiency against cycle number). Until now a Sheaf layer had one X and one Y axis,
and a graph with several layers drew each layer on its own.

Origin 2016 makes a double-Y graph from two layers on one page: the second layer is linked to the
first, covers exactly the first layer's frame, uses the first layer's X scale and has its own Y
scale, shown on the right. The same mechanism gives its "add right Y layer" command, graphs with
three or four Y axes, a top X axis with another scale, stacked panels that share X, and insets.
Origin 2023 later added two Y scales inside one layer; Origin 2016 does not have that form.
Details are in docs/spec/graphing.md section 12.

Two models were considered:

1. **Linked layers.** A layer may be linked to an earlier layer. While linked, its frame is read
   as fractions of the parent's frame and its X (or Y) axis is the parent's axis. Each axis says
   on which edge it is drawn (normal, opposite or hidden).
2. **An axis choice per plot.** One layer with a second Y axis, and each plot says whether it
   belongs to the left or the right axis.

## Decision

Sheaf uses linked layers (option 1), as Origin 2016 does.

- `Layer.link: Option<LayerLink { parent, x, y }>` with `ScaleLink::None` or `Straight`.
  A link is valid only when the parent comes earlier and is not linked itself; an invalid link is
  ignored and the layer is drawn on its own.
- `Axis.side` (`Normal`, `Opposite`, `Hidden`), `Axis.color_follows_plot`,
  `Axis.title_color_follows_plot` and `Legend.scope` (`Layer` or `Graph`).
- The renderer lays out a layer and the layers linked to it as one group: a shared frame, a shared
  X axis whose automatic range covers every linked layer's data, and page fitting over the labels
  of the whole group.
- The feel of a per-plot choice is offered on top of the model: `Project::add_right_y_layer`,
  `Project::move_plot_to_layer`, the `--y2` option of `sheaf plot`, and later a "left axis / right
  axis" choice per plot in the app, all of which create the right layer the first time.
- All new fields have serde defaults and are not written while they have their default value, so
  older project files load and draw unchanged, and graphs that do not use the feature are saved
  as before.

## Consequences

- Origin project files map one to one: two layer records, each with its own axes, curves, titles
  and legend, plus a link. The `.opj` reader only has to recognise the link and which edges each
  layer shows; a writer writes two layers. With a per-plot model the reader would have to merge
  layers and the writer split them, which works only for the plain case and loses a top X axis
  with its own scale, link formulas, the second layer's own text and legend, and offset axes.
- The same mechanism will carry 3 and 4 Y axes, a top X axis with another scale (for example
  d-spacing over 2-theta on an XRD pattern, once custom links exist), stacked panels that share X
  and insets.
- It matches what the owner knows from Origin 2016: layer icons 1 and 2, Layer Contents per layer,
  the Plot Details tree by layer, and the "add Right-Y layer" button.
- The renderer and the geometry function work per group instead of per layer, and the desktop app
  will need an "active layer" for its dialogs. Until then its graph settings dialog edits the
  first layer only.
- A legend that lists every layer is drawn after all layers, so later layers never cover it.
- Differences from Origin 2016 that follow from this design (shared X range from all linked data,
  colours that increment within each layer, one legend for the graph) are listed in graphing.md
  section 12.8.
