# T409: Turnbull nonparametric estimate with arbitrary censoring

**Status:** done 2026-11-23 (Claude Windows "claude:gmail", statistics 53). HANDOFF C (Origin 2026b reliability and
survival analysis), the nonparametric estimate left by T384: Statistics > Survival Analysis > Nonparametric Analysis
(Arbitrary Censoring)... and `sheaf analyze turnbull` estimate the failure distribution of exact, right-, left- and
interval-censored rows (optional frequency column) by Turnbull's self-consistency iteration, with one row per Turnbull
interval (probability, survival, cumulative failure) and a survival graph. Not measured in Origin.
**Size:** S. **Spec:** statistics 53.

## Left

- Standard errors and confidence bands, probability plots with interval data, comparing groups (interval-censored
  log-rank tests).

## Tests

- `crates/calc/tests/turnbull_reference.rs`: two cases (12 rows of every kind, 80 rows of inspections, watched units
  and irregular windows) against numpy/scipy (`tools/turnbull-ref.py`).
- `crates/calc/src/turnbull.rs`: Kaplan-Meier with exact and right-censored rows, overlapping windows, the ordering of
  equal ends, refusals.
- `crates/core/src/analysis/lifedist.rs`: `a_turnbull_estimate_reports_the_intervals_and_a_survival_graph`.
- `crates/cli/tests/analyze.rs`: `turnbull_estimate_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_survival_menu_makes_a_turnbull_estimate`.
