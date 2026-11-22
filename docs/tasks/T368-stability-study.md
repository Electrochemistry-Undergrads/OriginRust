# T368: Stability study (ICH Q1E shelf life)

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 38). HANDOFF C (Origin 2027 Pro Quality
improvement apps, "model between-batch variation in stability studies"): Statistics > Stability Study... and `sheaf
analyze stability` fit per-batch lines, choose separate lines, a common slope or one line by the ICH Q1E poolability
tests at 0.25, and give each batch's and the study's shelf life where a confidence bound of the mean meets a
specification limit, with a graph. Not measured in Origin.
**Size:** S. **Spec:** statistics 38.

## Left

- A random-batch (mixed model) analysis, transformed responses, the ICH extrapolation limits, several factors
  (strength, package).

## Tests

- `crates/calc/tests/stability_reference.rs`: four studies covering all three models and one or two limits, against
  numpy/scipy (`tools/stability-ref.py`).
- `crates/calc/src/stability.rs`: one batch with a known line, refusals.
- `crates/core/src/analysis/stability.rs`: sorted numeric batches, pooling, shelf life and the plotted columns.
- `crates/cli/tests/analyze.rs`: `stability_study_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_runs_a_stability_study`.
