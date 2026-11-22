# T328: Measure the Peak Analyzer's End Points Weighted baseline

**Status:** measured 2026-11-21 (Claude Windows "claude:2"), not built: docs only.
T318 left item / feature-parity "Baseline modes" (P0). Origin 2016's End Points Weighted (End Points(%)
default 12) interpolates straight lines through data points of the two end stretches and continues the
outer line to the ends. On smooth data the stretches' rows are known for 5, 12, 20 and 30 % (401 and 201
rows); on noisy data Origin keeps only some rows, in adjacent pairs, by a rule not found.
**Size:** S. **Spec:** `docs/spec/analysis-tools.md` 11.7. Probe: `re-workspace/scratch/baseline-als/run-epw.sh`.

## Left

- Find the anchor rule on noisy data (more probes: noise amplitude, End Points(%), point count), then
  build the mode.
