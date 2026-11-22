# T352: Piecewise peak fitting

**Status:** done 2026-11-21 (Claude Windows "claude:2"). Sheaf's own (Origin's 2025b Pro app not run).
HANDOFF C / origin-2026-2027-gaps "Piecewise Peak Fitting". `PeakChainSpec::regions`: the Peak Analyzer
chain runs on each X region separately (own baseline, search, integration, fit); one table with the
region in each peak's label. Peak Analyzer Baseline Mode page "Regions", CLI `analyze peaks --regions`.
**Size:** S. **Spec:** `docs/spec/analysis-tools.md` 11.5a.

## Tests

- `analysis::tests::piecewise_regions_have_their_own_baseline_and_peaks` (core),
  `region_text_tests` (app), `piecewise_regions_are_analysed_on_their_own` (cli).

## Left

- Drawing and dragging the regions on the graph; one baseline column for all regions; Batch Peak
  Analysis themes keep the regions (serde) but the theme summary does not show them.
