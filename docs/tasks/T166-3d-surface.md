# T166: 3D Surface from XYZ columns (B15 slice 7)

**Status:** done 2026-11-19 (Claude Windows "t30-origin"), **not measured**. Plot > 3D Surface > Color Map
Surface, Color Fill Surface, Wire Frame and Wire Surface are live for worksheet X, Y and Z columns on
T155's fixed view: a Delaunay triangulation over X and Y (`sheaf_calc::xyzarea::delaunay`), triangles
drawn back to front with black mesh lines. Left: X/Y Constant with Base (Colormap Surface with
Projection: T168), 3D Ternary Colormap Surface, surfaces from matrices (B14), a colour scale for the map.
**Size:** M. **Source:** greyed inventory package B15.
**Spec:** `docs/spec/graphing.md` ("3D Scatter and 3D Trajectory").
