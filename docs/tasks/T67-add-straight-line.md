# T67: Add Straight Line (B3, slice 1)

**Status:** done 2026-11-15 (branch `straight-line`, Claude Windows, session fe4b37eb; graphing 17.7). Left: dragging a line (Moveable; belongs with the shape drags of T24), the automatic label format does not round to the tick step.
**Size:** M. **Priority:** wave 4 (B3). **Spec:** [graphing](../spec/graphing.md), new section
"Straight lines" next to slope annotations (17.6). **Inventory:** Graph > Add Straight Line... (1).

## Goal

Graph > Add Straight Line... puts a vertical line at an X value or a horizontal line at a Y value
in the active layer, with a label that shows the value, as Origin's `addline` does: a marker at a
potential, a baseline at zero current, a threshold. The line is part of the layer, saved with the
project, drawn in every export, selectable and deletable.

## Origin facts (measured 2026-11-15, Origin 2016, `re-workspace/scratch/t67`, `addline*.lines.txt`)

- Options (X-Function interface, our wording): Type Vertical (default) or Horizontal; At Value (0);
  Color (black); Style Solid (default), Dash, Dot, Dash Dot, Dash Dot Dot, Short Dash, Short Dot,
  Short Dash Dot; Label Display Format (auto = the axis' tick label format; ".2" gave two decimals);
  Label Location Top (default), Bottom, Left, Right; Selectable and Moveable (both on); Name.
- It makes two objects in the layer: the line, named `vline`, `vline1`, `vline2`, ... (`hline`, ...
  for horizontal; or the given name) and its label `<name>Text`. Line width 0.5 pt, attached to the
  layer and its scales.
- A vertical line runs from the Y axis' "from" to its "to" at X = value; a horizontal one from X
  "from" to "to". These ends are fixed when the line is made: after `layer.y.to = 200` the vertical
  line still ended at 125.
- The label's text is the value in the axis' format (`$(vline.x, x)`), in the line's colour, the
  layer's default text size (22 pt on the stock template), not rotated. Its centre sits 5 % of the
  axis span from the line: vertical Top above the frame (y = to + 5 % of the Y span) and Bottom
  below it, Left/Right beside the line half-way up; horizontal Top above the line at the middle of
  the X range and Bottom below it, Left/Right outside the frame's left/right edge at the line's Y.

## Where

- Model: a new `crates/core/src/straight_line.rs` (`StraightLine`), held in `Layer::straight_lines`
  (serde default, skipped while empty; `FORMAT_VERSION` unchanged).
- Drawing: `crates/plot` beside the slope annotations (`plot/src/slope.rs` shows a data-attached
  object with a label), a scene object kind so that hit-testing selects it.
- App: the Graph menu item opens a dialog (`dialogs/straight_line.rs`), OK adds one in one undo step;
  a selected line can be deleted with Delete and opened by double click.
- `.opj`: the writer leaves it out with a warning (ADR 0008); the companion keeps it.
- CLI: `sheaf plot ... --vline X[,LABEL-LOCATION]` and `--hline Y` in the style of `--slope`.

## Steps

1. Model and drawing with Origin's placement; tests for both orientations, the four label locations,
   the format, log axes, a line outside the axis range (not drawn), save/load.
2. Dialog and menu item, selection, Delete, double click; one undo step; UI tests.
3. CLI flags, `.opj` warning, spec section, screenshot/export check.

## Don't

Do not build the other B3 items. Moving the line by dragging (Origin's Moveable) may wait for the
shape-drag owner (T24 `shape-resize`); record it if left.
