# T119: Graph > Add Function Graph...

**Status:** done 2026-11-16 (Claude Windows "t30-origin", graphing.md 41), **not measured** (Origin COM unregistered): the Add Function Graph window (y(x), From, To, Points), the points in a new FuncN workbook, a line plot in the active layer. Left: measure Origin's window and defaults and match them; a curve that follows the X axis range.
**Size:** S. **Source:** greyed inventory package B3 (Graph > Add Function Graph...).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

Graph > Add Function Graph... adds the curve of a function y = f(x) over an X range to the active layer
of the graph.

## Steps

1. Measure Origin 2016's dialog and result with tools/origin-ref if automation works; otherwise build
   from Origin's documented behaviour and mark the section unmeasured.
2. Build it: a dialog for the expression (in Sheaf's expression language, variable x), the X range and
   the number of points; the curve's values in a new worksheet; a line plot in the active layer.
3. Tests, a graphing.md section, greyed inventory recount.

## Done when

The menu item adds the curve, the inventory no longer lists it, and fmt, strict Clippy and the full
tests pass.
