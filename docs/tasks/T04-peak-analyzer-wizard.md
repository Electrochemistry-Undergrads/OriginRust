# T04: Peak Analyzer as Origin's multi-page wizard

**Status:** open; blocked in part on seeing Origin's pages. **Size:** L. **Branch:** `peak-wizard`.
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
- Multi-peak fit: Gauss and Lorentz only (`crates/fit/src/multipeak.rs`), one shape for all peaks.
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
