# T04: Peak Analyzer as Origin's multi-page wizard

**Status:** pages measured and the window rebuilt as Origin's wizard (2026-11-13, analysis-tools.md 11.1); Constant baseline and Origin's Area % added. Voigt added to the Fit Peaks page and `sheaf analyze peaks --fit voigt` (2026-11-15, analysis-tools.md 11.2): a single Voigt fit agrees with Origin 2016's `nlfit` Voigt to 6e-6 relative (Origin's `nlf_Voigt` is an approximation within 2e-6 of the height; Sheaf's is exact). Create and Subtract Baseline goals with Origin's User Defined 2nd-derivative anchors added (measured 2026-11-14 by session 56a09f3a, merged 2026-11-19 from its uncommitted worktree; analysis-tools.md 11.5; the anchor threshold is fitted, exact at 401 points; no command line yet). Left: Origin's Auto integration window rule (measured, not understood), Origin's exact anchor threshold, `--baseline-goal` on the command line, Auto Subtract Baseline, the other baseline modes, anchor editing. A function per peak added the same day (Fit Peaks page list, `--fit-shapes`; Sheaf's own layout, Origin's Fit Control window unseen). **Size:** L. **Branch:** `peak-wizard`.
**Detail:** audit B "Analysis X-Function dialogs" and its re-check note; D "Partial" (baseline).
**Spec:** `docs/spec/analysis-tools.md` (peak chain), `docs/spec/gui-look.md`.

## Goal

Analysis > Peaks and Baseline > Peak Analyzer... opens a wizard with Origin's pages: Goal, Baseline
Mode, Baseline Treatment, Find Peaks, Integrate Peaks (or Fit Peaks), Finish. Today it is one page
(`AnalysisKind::PeakChain` in `crates/app/src/dialogs/analysis.rs`; chain in
`crates/calc/src/peakchain.rs`; UI test "Peak Analyzer" in `crates/app/src/app/ui_tests.rs`).

## Origin facts

- Page list above is from the spec; pages 2+ were **never seen**. An attempt on 2026-11-12 to open the
  Peak Analyzer through `tools/origin-ref/origin-screenshot.ps1` failed (the X-Function ran headless,
  keys did not open the dialog).
- Baseline: Sheaf's calc has constant/line/anchor (`crates/calc/src/baseline.rs`), but the Peak
  Analyzer offers None/EndPoints/Minima only; Origin also has user-defined anchors, ALS, dataset,
  auto baseline.
- Multi-peak fit: Gauss, Lorentz and Voigt (`crates/fit/src/multipeak.rs`, Voigt since 2026-11-15), one
  function per peak or one for all.
  Origin allows per-peak function and Voigt.

## Steps

1. **First** get Origin's pages: try `origin-screenshot.ps1` with the dialog opened by LabTalk
   `pa -d` or the menu id (see `gui-setup*.labtalk.txt`); if that fails, ask the owner (HANDOFF
   "Needs from me") for screenshots of each page. Write what you see into analysis-tools.md.
2. Restructure the dialog as pages with Prev/Next/Finish over the existing chain; keep the chain's
   numerics unchanged (its tests must still pass).
3. Add anchor-point baseline from the existing calc code; Voigt to multipeak (compare one fit with
   Origin's `Voigt` FDF result).

## Done when

Wizard pages match Origin's titles and order; existing peak tests pass; one Voigt fit matches Origin;
checks pass.
