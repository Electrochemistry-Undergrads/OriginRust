# T451: Stratified Cox regression

**Status:** done 2026-11-25 (Claude Windows "claude:gmail", statistics 52). T408's "Left": Statistics > Survival
Analysis > Cox Proportional Hazards... ("A strata column follows the censor column") and `sheaf analyze cox --strata`
fit the stratified model: risk sets within strata, shared coefficients, a baseline survival curve per stratum. Not
measured in Origin.
**Size:** S. **Spec:** statistics 52.

## Left

- Time-varying covariates, factor covariates, residuals, the proportional-hazards check, survival at chosen covariate
  values.

## Tests

- `crates/calc/tests/cox_reference.rs`: two stratified cases (three age bands, Breslow and Efron) against numpy/scipy
  (`tools/cox-ref.py`), every stratum's curve.
- `crates/calc/src/cox.rs`: one stratum is the plain model; the null log-likelihood of two strata by hand.
- `crates/core/src/analysis/lifedist.rs`: `a_stratified_cox_model_draws_a_curve_per_stratum`.
- `crates/cli/tests/analyze.rs`: `cox_regression_runs_from_the_command_line` (text strata).
