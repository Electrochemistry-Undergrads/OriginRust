# T03: Analysis dialogs: Recalculate, Output, smoothing methods, pkFind options

**Status:** done, merged (`859c78e`): items 1 to 4 and Auto Apply. Left: Recalculate only links Smooth, Integrate and Find Peaks (the other tools run once); pkFind methods `win`, `first`, `second`, `residue` were measured but their rules are not reproduced; `hwidth` at an exact grid multiple (0.2 on a 0.1 grid) differs per peak; no saved dialog themes; the Output choice is offered for the six column-writing tools only; Cancel keeps what Apply or Auto Apply already made.
**Size:** M. **Branch:** `analysis-frame`. **Model:** Sonnet.
**Detail:** audit B "Analysis X-Function dialogs", D "Partial".
**Spec:** `docs/spec/gui-look.md` around the X-Function dialog (search "Recalculate"),
`docs/spec/ui-command-map.md` (Smoothing), `docs/spec/xfunction-index.md` (smooth, pkFind),
`docs/spec/analysis-tools.md`.

## Goal

The shared analysis-tool frame and its tools behave like Origin's X-Function dialogs.

## Items

1. **Recalculate:** the combo in `crates/app/src/dialogs/analysis.rs` (search `"Recalculate"`) shows
   only None; Manual/Auto disabled; choice not stored. Origin's default is **Manual**. Store the mode on
   the result (ADR 0005 analysis results) and recalculate when the source changes (Auto) or on
   request (Manual), reusing the formula recalculation design of column formulas.
2. **Output node:** greyed text today; Origin lets you pick the output range (new columns / new
   sheet / existing columns). Implement the choices ADR 0005 already supports.
3. **Smoothing methods:** add Lowess, Loess and Binomial to `crates/calc/src/smooth.rs` and the dialog.
   Get reference numbers from Origin (`smooth` X-function, `method:=lowess|loess|binomial`) with
   `tools/origin-ref/run-labtalk.ps1`, commit as fixtures like `smooth-suite1.labtalk.txt`.
4. **Find Peaks options:** Sheaf uses "Neighbours compared" and keep rules; Origin's pkFind has
   smoothing points, direction, height/width thresholds, foot height, max half width and "residual
   after 1st derivative" (spec only, not run). Measure pkFind with `pkfind-suite*.labtalk.txt` as a
   template, then add the options to `crates/calc/src/peaks.rs`.

## Done when

Recalculate Manual/Auto works for at least Smoothing, Integrate and Find Peaks with UI tests; the
three smoothing methods match Origin fixtures; pkFind options match measured cases; checks pass.

## Don't

The Peak Analyzer wizard is T04, not here.
