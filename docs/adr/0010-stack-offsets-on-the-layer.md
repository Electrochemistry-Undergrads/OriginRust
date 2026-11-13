# ADR 0010: Stacked curves are a layer setting; curve offsets are stored on the layer

Status: accepted (2026-11-12)

## Context

The owner stacks XRD patterns with Origin's "Stacked Lines by Y Offsets" (eleven such graphs in
one project). Origin moves each curve of a layer's plot group by an offset that only changes the
drawing. The group has one mode at a time: None, Cumulative, Constant, Auto or Individual. In
Origin the template uses Auto: each curve sits one gap (10 % of the tallest curve) above the
one before it, computed again from the data. In Individual mode every curve has its own X and Y
offset. Origin's 2D waterfall moves the curves by a layer-wide shift. Details are in
docs/spec/graphing.md section 13.

Questions to decide:

1. Are offsets display-only, or are the data shifted?
2. Where does the mode live?
3. Where do the per-curve numbers (Individual offsets, and which curve a fitted curve moves with)
   live: on each `Plot`, or on the layer?

## Decision

1. **Display only.** The renderer adds the offsets when it draws and when it computes the
   automatic axis range. Worksheets, analysis and fits always see the stored values.
2. **One mode per layer**: `Layer.stack: Stack` with `None`, `Auto { gap_percent }`,
   `Constant { step }`, `Individual` and `Waterfall { dx_percent, dy_percent }`. The modes exclude
   each other, as in Origin's dialog and Graph menu. Auto offsets are not stored; they are
   computed at every drawing, so they follow data changes as in Origin.
3. **Per-curve numbers on the layer, found by the curve's Y column**:
   `Layer.plot_offsets: Vec<PlotOffset { column, x, y, follows }>`.
   - `x` and `y` are used in Individual mode and kept while another mode is active.
   - `follows` makes a fitted curve take the place of its data curve instead of a place of its
     own in the stack. The fit code sets it when it draws a fit over its data.
4. Hidden plots keep their place in the stack (Origin's behaviour), so hiding a curve does not
   move the others.
5. Both fields have serde defaults and are not written while they are empty, so older files load
   unchanged and graphs that do not stack are written exactly as before.

## Why the per-curve numbers are not fields of `Plot`

- `Plot` is built in many places: the project, the fit code, the desktop app and the `.opj`
  reader. A field on `Plot` would make every one of them deal with stacking, and the `.opj`
  reader is being changed on another branch at the same time.
- An entry found by its Y column stays with its curve when plots are reordered, removed or added;
  a list indexed by plot position would not.
- The cost: two plots of the same column in one layer share one entry. That case is rare, and
  the two curves then simply move together.

## Consequences

- The `.opj` reader can bring the owner's stacked graphs in exactly as Origin drew them: it reads
  each curve's stored Y offset (curve record `0x1CA`) and writes an entry for that curve's Y
  column with mode Individual (graphing spec section 13.10).
- The app's future Offsets controls read and write the entry of the selected plot's column; when
  the user switches to Individual it first copies the offsets in effect
  (`sheaf_plot::stack_offsets`), so the picture does not jump.
- The waterfall is a simple 2D form drawn only for a layer on its own. In a group of linked
  layers (a double-Y graph) it is drawn like `None`, and the command line refuses that
  combination.
- Cumulative offsets (stacked columns and areas) are left for GP-COL-01; they would be one more
  `Stack` variant.
