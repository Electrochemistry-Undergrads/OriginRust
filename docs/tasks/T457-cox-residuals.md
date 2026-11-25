# T457: Cox residuals and survival at given covariates

**Status:** done 2026-11-25 (Claude Windows "claude:gmail", statistics 52). T408's "Left": Cox Proportional Hazards...
and `sheaf analyze cox` write martingale and deviance residuals per case and Schoenfeld residuals per failure
(`--residuals`), and draw the survival at given covariate values (`--at`) beside the curve at the means. Not measured
in Origin.
**Size:** S. **Spec:** statistics 52.

## Left

- The proportional-hazards test on the scaled Schoenfeld residuals, score and dfbeta residuals, residual plots.

## Tests

- `crates/calc/tests/cox_reference.rs`: the residuals of all six reference cases against numpy (`tools/cox-ref.py`).
- `crates/calc/src/cox.rs`: Schoenfeld residuals sum to the score (0), Breslow martingale residuals to 0, the curve at
  given covariates.
- `crates/core/src/analysis/lifedist.rs`: `cox_residuals_and_a_curve_at_given_values`.
- `crates/cli/tests/analyze.rs`: `cox_regression_runs_from_the_command_line` (`--at`, `--residuals`).
