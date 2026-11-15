# T47: Simulate Curve (Analysis > Fitting > Simulate Curve)

**Status:** done, merged 2026-11-15 (Claude macOS "mac-simcurve"; fitting.md 13): `sheaf_fit::simulate` on the fit evaluator, `Tool::SimulateCurve` (one undo step, linkable), the Simulate Curve window with preview, `sheaf simulate`, MCP `simulate_curve` and `list_simulate_functions`. Reviewed by three agents with each finding checked; four fixed. Left: the Origin measurements in fitting.md 13; user-defined functions; a Recalculate mode in the window; Simulate Surface (matrices). Brief written from
parity row "Simulate curve" (P1, ✗) and the greyed menu item "Simulate Curve..." under Analysis > Fitting.
**Size:** M. **Branch:** `simulate-curve`. **Model:** Sonnet. No Origin needed for the mechanism; the
window layout and defaults are measured once on Windows.
**Spec:** a new section in `docs/spec/fitting.md` ("Simulate Curve").

## Goal

The owner picks any function of the fit library (`sheaf_fit::library`, the same list the NLFit window
offers), types its parameter values (prefilled with the function's initial values), chooses an X range
(from, to, number of points, linear or log spacing, or the X of an existing column) and gets the curve as
a new X/Y pair of columns, optionally plotted, in one undo step. `sheaf simulate` and the MCP server do
the same through `sheaf-ops`.

## Origin facts

- Not measured. From Origin's public documentation (paraphrased): Analysis > Fitting > Simulate Curve
  (X-Function `simcurve`) chooses a category and function, shows the parameters with their initial
  values, an X range (from, to, points) and whether to create a graph; output goes to a new workbook or
  a sheet; the curve also updates live on a preview graph. Measure on Windows with `tools/origin-ref`:
  the window's controls and defaults (number of points, X range defaults, output naming, column long
  names/comments), and compare the simulated values of three functions (Gauss, ExpDec1, Boltzmann) with
  Sheaf's to 1e-12. Record in fitting.md; fixtures in `crates/calc/tests` or `crates/fit/tests`.
- Simulate Surface (two variables) waits for matrices (`originrust-3a`); keep it grey.

## Where

- `crates/fit` (or `crates/calc`): `simulate(function, params, xs) -> Vec<f64>` reusing the library's
  evaluation (find how `FitFunction` evaluates; do not write a second evaluator); X generation (linear,
  log; points >= 2; from != to; log needs positive bounds).
- `crates/core`: an analysis `Tool`/request for Simulate Curve so it is one undo step and can be
  recalculated like the other analysis tools (follow an existing simple tool's pattern); output columns'
  long names (function name, parameters in the comment: Sheaf's choice until measured).
- `crates/app`: enable "Simulate Curve..." (keep "Simulate Surface..." grey); a window in
  `crates/app/src/dialogs/` with category/function choice (reuse the NLFit window's function picker if it
  can be shared), parameter grid, X range, output (new book or the active sheet), "Plot" tick, live preview
  curve if cheap.
- `crates/ops` + CLI + MCP: `sheaf simulate --function Gauss --param y0=0 --param xc=1 ... --from --to
  --points [--log] -o out.csv|--project`; the analysis bridge exposes it to MCP like the other tools.

## Steps

1. Spec section (Sheaf's choices labelled; Origin measurements listed).
2. Simulation function and X generation with tests (each library function evaluates at its own initial
   parameters without NaN on a sensible range; log spacing; error cases).
3. Core tool + app window + UI tests (menu enabled, a simulated Gauss gives the expected columns, one
   undo step, Plot makes a graph).
4. CLI and MCP with tests; parity row to ◐.

## Done when

Analysis > Fitting > Simulate Curve makes a curve from any library function with typed parameters, the
CLI and MCP give the same numbers, fmt, clippy and tests pass.

## Don't

- Don't add a second function evaluator or a second copy of the function list.
- Don't enable Simulate Surface.
- Don't claim Origin's defaults until measured.
