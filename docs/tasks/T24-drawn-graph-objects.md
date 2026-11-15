# T24: Drawn objects on graphs: Line, Arrow, Rectangle, Ellipse

**Status:** done, merged (`542fef7`). Checks pass (fmt, clippy with warnings denied, all tests). On 2026-11-13 the Line, Arrow and Rectangle tools were measured in Origin and their defaults copied (0.5 pt, 10 pt head, LT Gray rectangle fill; spec 17). Front/back ordering within a layer is added (`shape-ordering`, 2026-11-15), with selection-safe undo/redo (including Undo AI); formatting, strict all-target Clippy and 3,199 parallel workspace tests passed (five ignored). This ordering scope is Sheaf's choice and unmeasured. Selected shape endpoint/corner resize handles are added (`shape-resize`, 2026-11-15), as a Sheaf-choice interaction with one undo step per completed drag. Not done: reading drawn objects from `.opj`, attachment to the page or to scales, copy and paste, ordering across object kinds/layers.
**Size:** M. **Branch:** `draw-objects`. **Model:** Sonnet.
**Detail:** audit C "Drawing tools"; T18 backlog row "Drawing tools" (this brief covers lines, arrows,
rectangles and ellipses; pictures and the inset graph stay in the backlog).
**Spec:** `docs/spec/graphing.md`, the section "Drawn objects" (new, after "Free text labels").

## Goal

The Tools bar buttons Line, Arrow, Rectangle and Circle draw objects on a graph as in Origin: drag on
the graph, the tool ends, the object can be selected, moved, deleted and edited, and every renderer
draws it.

## Origin facts

Nothing here was measured (this brief was written on a machine without Origin). From Origin's public
documentation, paraphrased, not measured; measure on the Windows machine with `tools/origin-ref`:

- The Tools toolbar has Line, Arrow, Rectangle and Circle (the last draws an ellipse unless Shift is
  held). One drag makes one object; the tool returns to the Pointer.
- Shift while dragging: a line or arrow snaps to horizontal, vertical or 45 degrees; a rectangle
  becomes a square; an ellipse a circle.
- An object is attached to the page, the layer frame, or the layer and its scales; the default for
  objects drawn inside a layer is the layer (the text object spec in section 16 chose layer frame).
- Double click opens a dialog with tabs for the line (style, width, colour), the arrow (shape, length,
  width, at the begin and/or end) and the fill (pattern, colour; rectangles and ellipses only).
- Delete removes the selected object; it is moved by dragging.

## Where

- `crates/core/src/shape.rs` (new): `Shape`, `ShapeKind`; `Layer::shapes` in `graph.rs`
  (`serde(default, skip_serializing_if = "Vec::is_empty")`).
- `crates/plot/src/shapes.rs` (new): drawing into the layer overlay after the labels;
  `ObjectKind::Shape(usize)` in `scene.rs`; picking in `hit.rs`.
- `crates/app/src/dialogs/shape_object.rs` (new); the drawing tools live in `graph_view.rs`, with small
  additions in `toolbar.rs`, `actions.rs`, `app.rs`, `menus.rs`.
- `crates/io/src/opj/write`: save warning for objects the file cannot hold.

## Steps

1. Model and serde (old files load unchanged).
2. Renderer: line, arrow (head at end, or both ends), rectangle, ellipse; dash, width, colour, fill.
3. Picking: a line by distance, a filled shape by its area, an unfilled one by its outline.
4. App: tools, drag to draw with Shift, select, move, Delete, double click dialog, undo for each step.
5. `.opj` warning; CLI `--arrow` if cheap.
6. Tests: round trip, golden `shapes.svg`, hit test, UI tests.

## Done when

All four kinds can be drawn, moved, edited and deleted in the app with undo; the figure exports show
them; the spec section lists what is measured and what is from documentation; checks pass.

## Don't

Do not attach objects to axis scales yet (section 16 anchors to the layer frame); do not copy Origin's
icons or text.
