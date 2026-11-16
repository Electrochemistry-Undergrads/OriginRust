# T60: Merge Graph Windows and Layer Management

**Status:** open. Written 2026-11-15 from the owner's priority "features like stacked plots" and
parity row "Layer arrangement, merge" (now P0).
**Size:** L (two halves: merge, then Layer Management). **Branch:** `merge-graphs`. **Model:**
Sonnet. Both windows are measured on Windows; the merge itself can be built first without Origin.
**Spec:** `docs/spec/graphing.md` 15 (multi-panel graphs; add "15.8 Merge Graph Windows" and "15.9
Layer Management"), ADR 0009 (linked layers).
**Greyed items closed (7):** Graph > Layer Management..., Merge Graph Windows..., Fit Layer to
Graph, Fit Page to Layers...; Format > Fit Page To Layers [Graph]; context menus: graph page >
Layer Management..., layer icon > Layer Management....

## Goal

1. **Merge Graph Windows.** A person has several finished graphs (say five CVs, or XRD, Raman and a
   CV) and makes one figure of them: Graph > Merge Graph Windows... lists the graph windows, takes a
   grid (rows and columns; one column for a vertical stack), gaps and margins, and builds a new graph
   page whose layers are copies of the source graphs' layers placed in the grid, each keeping its
   plots, axes, legend, labels and style. The source graphs stay (or are removed, by a check box).
2. **Layer Management.** On any multi-layer graph: a window with the layer list and a preview, to
   arrange the layers in a grid, set each layer's position and size, and link a layer's axes to
   another layer.
3. Fit Layer to Graph and Fit Page to Layers: the active layer fills the page inside margins; the
   page shrinks or grows to the layers' bounding box.

## Today

- `crates/core/src/panels.rs`: `PanelLayout`, `PanelOrder` (`TopDown` is documented there as the
  order of "Origin's 4 Panel and 9 Panel templates, Merge Graph"), `PanelPage`; applying a layout
  writes the layers' frames, links and label settings. Plot > Multi-Panel and the Stack window
  (`crates/app/src/dialogs/stack_panels.rs`, graphing 15.7a) build panel graphs from worksheet
  columns only, never from existing graphs.
- A layer's place is its frame; `Layer.link: Option<LayerLink { parent, x, y }>` (ADR 0009).
- Plot Details > layer > Size edits one layer's frame. CLI: `sheaf plot --panels`.
- No command copies a layer from one graph to another.

## Origin facts

- Origin's X-Function is `merge_graph` (xfunction-index.md, P1). Layer Management is in the Graph
  menu, the page menu and the layer icon menu (gui-look.md 4).
- **Not measured, measure on Windows:** the Merge Graph Windows window (which graphs it offers,
  grid controls, gap and margin units and defaults, page size choices, how text and symbols are
  scaled when a full-page graph becomes a small panel, what happens to legends, whether layers of a
  multi-layer source keep their links); the Layer Management window (its tabs, the Arrange and
  Size/Position controls, the Link choices, what Apply does); Fit Layer to Graph and Fit Page to
  Layers (margins used). Take PNG exports of a 2 x 1 and a 3 x 1 merge of the stock Line graph for
  fixtures.
- Until measured, Sheaf's choices (label them): panels are filled row by row from the top left; text
  and symbol sizes are kept in points (no scaling) with a "Scale text with panel" switch; a vertical
  merge offers "Link X axes" as the Stack window does.

## Where

- `crates/core/src/merge.rs` (new): `merge_graphs(project, sources, layout, options) -> Graph`,
  pure and undoable: deep-copies layers (plots keep their column references), re-indexes
  `LayerLink.parent`, places frames with `PanelLayout`. Fit functions beside it or in `panels.rs`.
- `crates/app/src/dialogs/merge_graphs.rs` and `layer_management.rs` (new); preview drawn by
  `sheaf_plot` at small size.
- `crates/ops`, `crates/cli`, `crates/mcp`: `sheaf merge-graphs PROJECT --graph A --graph B --rows 2
  --cols 1 [-o out.png]` and the MCP twin, sharing the core function.
- `crates/io`: nothing new for `.sheaf`; `.opj` already writes multi-layer graphs (links go to the
  companion).

## Steps

1. Core merge with tests: two single-layer graphs into 2 x 1 and 1 x 2; a double-Y source keeps its
   link; three sources into a 2 x 2 grid leave one cell empty; sources unchanged; one undo step;
   golden image of a three-panel vertical stack with linked X.
2. Merge Graph Windows window, menu item live; CLI and MCP; UI test and a `--screenshot` look.
3. Layer Management: list and preview, Arrange, Size/Position, Link; each Apply one undo step.
   Fit Layer to Graph, Fit Page to Layers. Enable the remaining items; delete all seven from
   greyed-items.md.
4. Origin pass (Windows): measure the "not measured" list, record it in graphing 15.8 and 15.9,
   change what differs (scaling of text is the part most likely to differ).

## Done when

Three existing graphs become one vertically stacked figure with shared X in a few clicks and from
the CLI; a layer can be moved, resized, re-arranged and linked from Layer Management; exports and
`.sheaf`/`.opj` round trips keep the result; the checks pass.

## Don't

- Don't change the Stack window or the 2/4/9 Panel commands; they build from columns and are
  measured (15.7a).
- Don't add or delete single layers here (T61 owns `Add Layer`, `Delete Layer`, insets).
- Don't move the source graphs' data or change the source windows unless "remove sources" is ticked.
- Don't read Origin's merged `.opj` to learn the layout; measure the window and the exports.
