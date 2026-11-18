# T107: Vector XYAM and Vector XYXY

**Status:** both vector plot kinds landed 2026-11-16 (graphing.md 30). The archived Origin
2016 captures show right-pointing legend arrows for both kinds. Sheaf now draws those
samples using the plot's arrow style. Plot Details Vector tab, Head/Midpoint positions and
Origin's exact multiplier rule remain open. Workspace verification pending.
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
