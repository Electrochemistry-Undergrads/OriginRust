# T391: Life data regression

**Status:** done 2026-11-23 (Claude Windows "claude:gmail", statistics 48). HANDOFF C (Origin 2026b "regression with
life data"): Statistics > Survival Analysis > Life Data Regression... and `sheaf analyze lifereg` fit Weibull,
lognormal or exponential accelerated failure time models of log time on numeric covariates with right-censored times,
with standard errors and z tests. Not measured in Origin.
**Size:** S. **Spec:** statistics 48.

## Left

- Cox proportional hazards is T408.
- Interval-censored regression, factor covariates, predicted percentiles and residual plots.

## Tests

- `crates/calc/tests/lifereg_reference.rs`: 60 rows with two covariates, three distributions, against numpy/scipy
  (`tools/lifereg-ref.py`).
- `crates/calc/src/lifereg.rs`: without covariates the fit equals the distribution fit of T366; refusals.
- `crates/core/src/analysis/lifedist.rs`: a covariate that shortens life.
- `crates/cli/tests/analyze.rs`: `life_data_regression_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_survival_menu_runs_a_life_data_regression`.
