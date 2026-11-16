# T91: Simulate Surface, Compare Datasets, Nonlinear Implicit Curve Fit, XYZ Surface Area (B8 slice)

**Status:** done 2026-11-16 (Claude Windows, branch `fit-surface-more`). Origin 2016 measured the same day
(`docs/spec/fitting.md` 16-18, `docs/spec/analysis-tools.md` 18). **Built:** XYZ Surface Area (calc `xyzarea`, `Tool::XyzArea`, window, `sheaf analyze xyz-area`); Simulate Surface (`sheaf_fit::simsurface`, `Tool::SimulateSurface`, window, `sheaf simulate-surface`, MCP `simulate_surface`); Nonlinear Implicit Curve Fit (`sheaf_fit::implicit`, `Project::fit_implicit`, window, `sheaf fit-implicit`; Circle and Ellipse, orthogonal distance regression); Compare Datasets (`sheaf_fit::compare_data`, `Project::compare_datasets`, window, `sheaf compare-datasets`). **Open:** PlaneMod (three-variable implicit function), MCP tools for the implicit fit and Compare Datasets, Origin's Compare Datasets report (needs two fits made in the NLFit window).
**Size:** L. **Spec:** `docs/spec/fitting.md` sections 16 (Simulate Surface), 17 (Compare Datasets), 18 (Nonlinear
Implicit Curve Fit); `docs/spec/analysis-tools.md` section 18 (XYZ Surface Area).
**Inventory:** Analysis > Fitting > Simulate Surface...; Compare Datasets...; Nonlinear Implicit Curve Fit...;
Analysis > Mathematics > XYZ Surface Area... (4 items, tagged `T91`). Wavelets, peaks, Probability/Q-Q and the other
statistics tools are not in this slice.

## Goal

Four tools that do what Origin 2016's do, in the order of the steps below (each can be a separate commit and each
enables its own menu item only when it works):

1. **XYZ Surface Area**: area of the surface through scattered XYZ points.
2. **Simulate Surface**: XYZ data of any of the 22 surface functions on a grid, with optional noise.
3. **Nonlinear Implicit Curve Fit**: fit of Circle, Ellipse (and PlaneMod, a three-variable surface case) by orthogonal
   distance regression.
4. **Compare Datasets**: two fits of one model compared; the report is not measured (see Origin facts).

## Origin facts (measured 2026-11-16; raw output `re-workspace/scratch/t91/`, local only, scripts `tools/origin-ref/t91/`)

- **XYZ Surface Area** (`xyzarea iz:=(1,2,3) area:=aa`, scriptable): the sum of the 3D triangle areas of the Delaunay
  triangulation of the XY projection; equals scipy's Delaunay sum to 1e-15 on a plane, two paraboloid grids, one
  triangle, a flat square and 25 random points. Duplicate (x, y) points are merged with the mean of their Z (checked
  by hand). Under three usable points (two points, collinear points): no result. No sheet or report, only the number.
  The window was not opened.
- **Simulate Surface** (`simsurface`, not scriptable; window read through the menu): Gauss2D default; 22 functions with
  their own parameter values and X/Y ranges (table in fitting.md 16.1), 20 by 20 points, noise 10 % in the four
  functions starting on -1 to 1 (Gauss2D, Gaussian2D, Voigt2D, Voigt2DMod) and 0 elsewhere; Result Data
  `(<new>,<new>,<new>)` makes a new book `Sheet1` with X, Y, Z columns (long names `Simulated X1/Y1/Z1`, function name
  as comment, X changing fastest); noise is `noise/100 * mean(z) * g` on Z only, `g` a fixed standard-normal-like sequence
  (not reproducible by us). Values at noise 0 are exactly the function.
- **Nonlinear Implicit Curve Fit** (the NLFit window with Category Implicit, Circle/Ellipse/PlaneMod, default algorithm
  Orthogonal Distance Regression (Pro); scriptable as `nlbegin ... func:=Circle`): the parameters are the geometric
  (orthogonal distance) fit of a circle, not the algebraic fit of `f = 0`; Origin's residual sum of squares and errors are
  ODRPACK's, about 0.1 % from the exact distance sum. Full numbers in fitting.md 18.
- **Compare Datasets** (`fitcmpdata`, not scriptable, window measured): defaults Recalculate Manual, Fit Parameters and
  Fit Statistics ticked, names Data1/Data2, Results `<new>`; the report-picker lists only fits made in the NLFit window,
  so the **test and the report layout are unmeasured** and Sheaf starts from its own pooled-fit F-test (fitting.md 17).

## Where

- `crates/fit/src/`: new modules, no edits to the existing engine except `pub mod` lines: `simsurface.rs` (grid, noise,
  per-function starting values; reuse `surface.rs` evaluation and `library`), `implicit.rs` (orthogonal-distance
  Gauss-Newton/LM on the point-to-curve distance for Circle and Ellipse; generic implicit `f(x, y) = 0` via the
  distance along the gradient for other functions), `compare_data.rs` (pooled fit and the F-test; reuse `global.rs` and
  `compare.rs`). `crates/calc/src/xyzarea.rs` (Delaunay triangulation, duplicate merging, area).
- `crates/core/src/analysis.rs` (additive `Tool` variants) and new modules under `crates/core/src/analysis/`.
- `crates/app/src/menus.rs` (the four `grey(...)` items become actions), new dialog files under
  `crates/app/src/dialogs/` (Simulate Surface in the layout of fitting.md 16.1, Compare Datasets as 17, the XYZ Surface
  Area window, implicit fit as a category of the existing NLFit window); `crates/ops/src` (`sheaf simulate --surface`,
  `sheaf fit --implicit`, `sheaf analyze xyz-area`, `sheaf fit --compare-datasets`), `crates/mcp`.

## Steps

1. Measure the rest where cheap: the XYZ Surface Area window and its output (Results Log), the Ellipse implicit fit,
   Simulate Surface into a matrix; for Compare Datasets run two fits in the NLFit window (button Fit, then Done), then the
   dialog, and dump the report. Record in the same spec sections.
2. XYZ Surface Area: calc function and fixture `crates/calc/tests/origin_xyzarea.rs` (the six cases of the spec table
   and the duplicate case, tolerance 1e-12), tool, menu item, CLI.
3. Simulate Surface: library values per function (the table), grid, noise, tool and window; test that every function at
   its starting values on its range is finite where Origin's is (compare fitting.md 16.1 values to 1e-12 on Gauss2D,
   Plane, Lorentz2D where the formula is already Origin-verified in fitting.md 14).
4. Implicit fit: circle fixture from `tools/origin-ref/t91/circle-data.json` (parameters 1e-6, statistics 2e-3, label "ODRPACK,
   Origin within 0.2 %"), Ellipse checked against scipy and labelled "scipy-checked, not Origin" unless measured.
5. Compare Datasets as in fitting.md 17 (Sheaf's layout, labelled) or Origin's if measured in step 1.
6. Each tool: one undo step, `.sheaf` storage in ordinary sheets (no `.opj` warning needed unless a new object type),
   inventory lines deleted, `python tools/greyed-count.py --write` (revert its change to `HANDOFF.md`).

## Done when

The four menu items are enabled and do what the spec says, with Origin fixtures where measured and scipy-checked tests
elsewhere; fmt, strict clippy, `cargo test --workspace` and `python3 tools/greyed-count.py --check` pass; the four
inventory lines are removed; the README row says done.

## Don't

- Do not describe Sheaf's own Compare Datasets report or the noise generator as Origin's.
- Do not use `simsurface` or `fitcmpdata` as scripts (they return false); the window is the only way.
- Do not treat the algebraic least-squares fit of `f = 0` as the implicit fit; Origin fits orthogonal distances.
- Do not edit `HANDOFF.md` beyond your own claim; keep edits to `analysis.rs`, `menus.rs`, `dialogs/analysis.rs` and
  `crates/ops/src/*.rs` additive.
