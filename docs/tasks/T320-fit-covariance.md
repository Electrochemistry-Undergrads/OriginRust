# T320: Covariance and correlation matrices in the fit report

**Status:** done 2026-11-21 (Claude Windows "claude:b284", fitting section 4). The parity row "Fit report" said the
covariance and correlation matrices were not written. Origin's NLFit has both as Fit Parameters options; Origin 2016's
default report (`nlend output:=1`) has neither, so both are off by default here too. When ticked (NLFit window
Advanced > Quantities, CLI `fit --covariance --correlation`, `FitRequest::covariance_matrix` / `correlation_matrix`),
the parameter sheet gets one column per parameter for each matrix after "Fixed" (long name = parameter, comment
"Covariance" or "Correlation"). Linear, polynomial, nonlinear, surface and per-dataset global fits all write them.
**Size:** S. **Spec:** fitting 4.

## Measured

Origin 2016, Gauss fit of 20 points (`re-workspace/scratch/t320/probe1.lines.txt`): report columns have no matrix
tables; the nltree dump (`tree.txt`) has values, errors and dependencies but no matrix. The test checks Sheaf's
covariance diagonal against Origin's standard errors and the correlation matrix against Origin's Dependency
(`1 - 1/(R^-1)_ii`), both to 1e-6.

## Left

- Origin's report layout (row headers in a report tree) is not copied; the global fit's combined table has no
  matrices.

## Tests

- `crates/fit/src/report.rs`: `correlation_divides_by_the_standard_deviations_and_blanks_fixed_parameters`.
- `crates/core/src/fitting.rs`: `covariance_and_correlation_matrices_match_origin_2016_and_are_off_by_default`.
- `crates/cli/tests/fit.rs`: `covariance_and_correlation_matrices_go_to_the_parameter_sheet`.
- `crates/mcp/tests/executor.rs`: `the_fit_tool_adds_the_covariance_and_correlation_matrices_on_request` (MCP `fit` takes `covariance_matrix`, `correlation_matrix`).
- `crates/app/src/app/ui_tests.rs`: `the_fit_window_offers_direct_weighting_and_the_scale_errors_switch` (the boxes).
