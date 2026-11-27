# T437: Predicted percentiles of the life data regression

**Status:** done 2026-11-24 (Claude Windows "claude:gmail", statistics 48). T391's "Left" (Origin 2026b regression
with life data): Statistics > Survival Analysis > Life Data Regression... and `sheaf analyze lifereg` predict
percentile lives (10, 50 and 90 % by default, `--percentiles`) at the covariate means or given values (`--at`), with
delta-method intervals on log time (`--confidence`). Not measured in Origin.
**Size:** S. **Spec:** statistics 48.

## Left

- A percentile-vs-covariate graph, interval-censored regression, factor covariates, residual plots. (Several
  covariate settings at once: T504.)

## Tests

- `crates/calc/tests/lifereg_reference.rs`: `percentiles_match_scipy` (three distributions, two covariate settings,
  four percentiles; `tools/lifereg-ref.py`).
- `crates/calc/src/lifereg.rs`: the exponential median is ln 2 times the mean, its log SE is the intercept's.
- `crates/core/src/analysis/lifedist.rs`: `life_regression_percentiles_at_the_means_and_at_given_values`.
- `crates/cli/tests/analyze.rs`, `crates/app/src/app/ui_tests.rs` (`the_survival_menu_runs_a_life_data_regression`).
