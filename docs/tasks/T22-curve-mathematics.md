# T22: Curve mathematics: Average Multiple Curves, Intersect, Polygon Area

**Status:** done, merged (`734a1ff`) (builder "math1"): numerics, core tools, links, dialogs, menu, CLI, spec and tests. Nothing in this brief was measured in Origin (the build machine is macOS); every Origin behaviour below is from Origin's public documentation, paraphrased, and is listed under "Unmeasured" in the spec. On 2026-11-13 (Windows) Origin's avecurves, intersect and polyarea were measured (spec section 4): the average now uses Origin's defaults (Full X Range, Auto points, Origin's column names, optional columns off), Intersect writes Origin's column names, Polygon Area already matched. On 2026-11-14 Level Crossing was added (analysis-tools.md 14, measured against Origin's `levelcrossing`); Cumulative Sum is dropped because Origin has no such tool (only the `cumsum()` function of Set Column Values).
**Size:** M. **Branch:** `math-curves`. **Model:** Sonnet.
**Detail:** audit D "Mathematics: ... curve intersect, average curves, polygon area" (backlog row of T18, now covered here).
**Spec:** `docs/spec/analysis-curve-math.md` (new, written with this task); `docs/spec/xfunction-index.md` lists the X-functions `avecurves`, `intersect`, `polyarea`; menu in `docs/spec/ui-command-map.md` (Mathematics).

## Goal

Three greyed items of Analysis > Mathematics work as real tools in the analysis dialog frame, from the
menu, from the command line and through Recalculate: **Average Multiple Curves...**, **Intersect...**
and **Polygon Area...**. The typical uses for CV data: the mean (with standard deviation) of the repeated
scans of one voltammogram, the potential where two curves cross (for example forward and reverse sweep, or
a curve and a reference), and the area inside a CV loop.

## Origin facts

Nothing measured. From Origin's public documentation (paraphrased, to be measured on the Windows machine with
`tools/origin-ref`):

- Average Multiple Curves (`avecurves`) interpolates the input curves onto a common set of X values and
  gives the mean Y, optionally with the standard deviation and the number of curves that contributed, in
  new columns; the X values come from a chosen reference curve, from evenly spaced points, or from the
  union of all X values. Linear interpolation is the default.
- Intersect (`intersect`) finds where two curves cross, treating each as a polyline, and writes X and Y of
  each crossing; curves need not be functions of X.
- Polygon Area (`polyarea`) treats the curve as a closed polygon and reports the area (and centroid);
  the polygon is closed from the last point back to the first.

## Where

- `crates/calc/src/curves.rs` (new): `average_curves`, `intersections`, `polygon_area`; tests with
  hand-worked values.
- `crates/core/src/analysis.rs`: `Tool::AverageCurves`, `Tool::Intersect`, `Tool::PolygonArea` and their
  specs; the work is in `crates/core/src/analysis/curves.rs` (new). `crates/core/src/analysis_link.rs`:
  three `LinkTool` variants so the results follow their input.
- `crates/app/src/dialogs/analysis.rs`: three `AnalysisKind`s (small, additive); the option controls are in
  `crates/app/src/dialogs/math_curves.rs` (new). `crates/app/src/menus.rs`: the three entries.
- `crates/cli/src/analyze.rs`, `args.rs`: `average`, `intersect`, `polygon-area` subcommands of `analyze`.

## Steps

1. Numerics in sheaf-calc with doc comments and tests (square, triangle, circle, figure eight; crossing,
   touching, overlapping, parallel, disjoint; averages with equal and unequal X, ranges, descending X).
2. Core tools and recalculation links; UI tests in core.
3. Dialogs, menu, CLI (From a graph, Average Multiple Curves and Intersect start with the plots of the active layer); screenshot of each dialog and a figure of the average with SD error bars.
4. Spec file with the decisions and the list of what an Origin run has to confirm.

## Done when

The three tools run from the menu, the CLI and Recalculate Auto/Manual; checks pass; the spec names every
unmeasured behaviour.

## Don't

Do not copy Origin's help text. Do not claim a behaviour was measured. Do not touch the other Mathematics
items (Simple Column Math, XYZ Surface Area, Median Filter, ...); they stay grey.
