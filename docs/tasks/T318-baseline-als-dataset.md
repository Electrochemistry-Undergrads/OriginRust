# T318: Peak Analyzer baselines Use Existing Dataset and Asymmetric Least Squares Smoothing

**Status:** done 2026-11-21 (Claude Windows "claude:2"). ALS not measured: Origin 2016 marks it Pro and the
test installation does not open it.
Feature-parity "Baseline modes" (P0) gaps "no dataset baseline, ALS". The Peak Analyzer's Baseline Mode
page offers Use Existing Dataset (a Y column of the project as the anchors; an anchor method of User
Defined in the baseline goals) and Asymmetric Least Squares Smoothing (Eilers and Boelens, with Origin's
four settings) in every goal; the command line has `--baseline als` and the `--als-*` settings.
**Size:** M. **Spec:** `docs/spec/analysis-tools.md` 11.6. Probe: `re-workspace/scratch/baseline-als/`.

## Tests

- `als_follows_the_background_under_a_peak` (calc), `baseline_mode_tests` (app),
  `als_baseline_goal_follows_the_background` (cli).

## Left

- Compare ALS with Origin Pro; XPS, End Points Weighted, Local Baseline, Min&Max.
