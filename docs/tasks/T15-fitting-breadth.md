# T15: Fitting breadth

**Status:** items 1-3 done, merged 2026-11-13; items 4-6 done, merged (`e525575`) (item 5: Rank Models is Sheaf's AICc ranking, Origin's `funcRank` could not be scripted; item 6: Linear Fit with X Error is not compared with Origin, same reason); item 7 (surface fits) open. The NLFit window has Origin's tabs, lower panel and Fit/Done/Cancel since `nlfit-origin` (merged 2026-11-13; fitting.md section 9). **Size:** L. **Branch:** `fit-breadth`. **Model:** Sonnet.
**Detail:** audit D "Missing", "Partial". **Spec:** `docs/spec/fitting.md`, `fit-function-catalog.md`,
`fdf-format.md`, `fitting-linear.md`.

## Today

- Engine `crates/fit/src/nlsf.rs` (Levenberg-Marquardt, stepping `Fitter`), library
  `crates/fit/src/library.rs` (206 one-variable functions; Origin 237 incl. 22 surface and 3
  multi-variable), FDF import `crates/fit/src/fdf.rs` (rejects several dependent variables; reaches
  188 of 237 files).
- Weighting: `FitWeighting` in `crates/core/src/fitting.rs` is None/Instrumental/Statistical; the
  engine already supports direct weights (`crates/fit/src/report.rs`). `scale_errors` (default true)
  and the linear fit's `scale_errors_by_reduced_chi_sqr` have no switch in the UI.

## Items, in order

1. Expose Direct weighting and the "scale errors with sqrt(reduced chi-sqr)" switch in the NLFit
   and Linear Fit dialogs.
2. Multi-dataset fit: independent and global (shared parameters) over several Y columns.
3. Quick fits from greyed menu items: Exponential, Sigmoidal, Single Peak Fit (they are NLFit with
   a preset function and default settings: measure Origin's defaults).
4. Simplex algorithm (algorithm combo today shows only Levenberg Marquardt).
5. Compare Models / Rank Models (AIC, BIC, F-test as in Origin's report).
6. Multiple Linear Regression; Linear Fit with X Error.
7. Surface fits and functions of 2-3 variables (needs matrices: after `originrust-3a`'s matrix work).

## Origin facts already measured

Fit curve has 1000 points; live iteration limit 400; 21 fits match Origin's NLFit to 5e-10 with the
same iteration counts; fixed-parameter standard error: Origin's sheet stores 0 (branch `sheet-menus`
changes Sheaf's report to 0). Origin's bound handling can stop above the true constrained minimum;
Sheaf copies that on purpose. Not compared with Origin: each "1 Iteration" press restarts the
damping (lambda). Use `tools/origin-ref/nlfit-make-fixture.py` and
`nlfit-suite*.lines.txt` for new references.

## Done when

Each item matches Origin on a fixture and is reachable from the menu; checks pass.
