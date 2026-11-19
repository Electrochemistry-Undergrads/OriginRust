# T164: 3D Axis window (B15 slice 6)

**Status:** done 2026-11-19 (Claude Windows "t30-origin"), **not measured** (Origin's 3D axis dialog was
not probed). Format > Axes > Z Axis... is live on 3D layers (greyed elsewhere), and X Axis... / Y Axis...
on a 3D layer open the same window instead of the 2D axis dialog: From, To and Increment of the box's
X, Y and Z axes, each Auto until set, in `Space3dCoords::scales`. Points outside the box are not drawn.
Left: 3D axis titles, tick labels and scale types; the view's rotation.
**Size:** S. **Source:** greyed inventory package B15.
**Spec:** `docs/spec/graphing.md` ("3D Scatter and 3D Trajectory").
