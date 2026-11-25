# T408: Cox proportional hazards

**Status:** done 2026-11-23 (Claude Windows "claude:gmail", statistics 52). HANDOFF C (Origin 2026b reliability and
survival analysis), the Cox regression left by T391: Statistics > Survival Analysis > Cox Proportional Hazards... and
`sheaf analyze cox` fit the proportional hazards model on numeric covariates with right-censored times (Breslow
ties), with standard errors, z tests, hazard ratios and their intervals, the likelihood-ratio, Wald and score tests
and the baseline survival at the covariate means as a graph. Not measured in Origin.
**Size:** S. **Spec:** statistics 52.

## Left

- Efron ties are T445, strata T451. Time-varying covariates, factor covariates, residuals (martingale, Schoenfeld) and the
  proportional-hazards check; survival curves at chosen covariate values.

## Tests

- `crates/calc/tests/cox_reference.rs`: 70 rows with ties, three covariates and one, against numpy/scipy
  (`tools/cox-ref.py`).
- `crates/calc/src/cox.rs`: refusals (no covariates, too few events, separation).
- `crates/core/src/analysis/lifedist.rs`: `a_cox_model_reports_hazard_ratios_and_a_survival_graph`.
- `crates/cli/tests/analyze.rs`: `cox_regression_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_survival_menu_fits_a_cox_model`.
