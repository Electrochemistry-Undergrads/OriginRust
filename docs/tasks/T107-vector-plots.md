# T107: Vector XYAM and Vector XYXY

**Status:** done 2026-11-16 (Claude Windows "t30-origin", graphing.md 30): both measured (XYAM's System angle unit is radians, multiplier, head sizes; XYXY's X,Y,X,Y designation and axes taking in the ends) and built as new plot kinds. Left: Plot Details Vector tab, Head/Midpoint positions, Origin's exact multiplier rule, legend arrows.
**Size:** M. **Source:** greyed inventory package B4 (Plot > Specialized > Vector XYAM, Vector XYXY).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

Vector plots as Origin 2016's: XYAM draws an arrow at each (X, Y) with the angle and magnitude of two
more columns; XYXY draws an arrow from (X1, Y1) to (X2, Y2).

## Origin facts

Measure in Origin 2016: which columns each takes from a selection, where the arrow starts, the angle unit
and direction, how the magnitude maps to length, the arrow head, the default colour and width, and the
axis ranges.

## Done when

Both items are enabled and draw as measured (fixtures), saved in `.sheaf`, fmt/clippy/tests pass,
inventory and spec updated.
