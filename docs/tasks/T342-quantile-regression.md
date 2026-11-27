# T342: Quantile regression

**Status:** done 2026-11-21 (Claude Windows "claude:gmail", statistics 29). HANDOFF C (Origin 2026-2027 gaps):
Origin's Quantile Regression app (2019b) had no counterpart. Sheaf now fits linear quantile regressions exactly
(interior point, then the optimal vertex; checked against `scipy.optimize.linprog` to 1e-9 on 20 fits), as a tool
(`Tool::QuantReg`), from Statistics > Quantile Regression... and from `sheaf analyze quantreg` (and the MCP `analyze` tool, which bridges the command-line tools, as `quantreg`). Not in Origin 2016,
so nothing is measured; the result sheet and graph are Sheaf's own.
**Size:** M. **Spec:** statistics 29.

## Left

- Standard errors of the coefficients: done (T494, kernel robust or iid, with Prob>|t|); confidence limits
  (rank inversion), bootstrap errors, tests between quantiles remain.
- Change-point analysis (the other half of the 2019b app pair).

## Tests

- `crates/calc/src/quantreg.rs`: constant model = sample median, exact lines, refusals.
- `crates/calc/tests/scipy_quantreg.rs`: 20 fits against `scipy.optimize.linprog` (fixture from `tools/quantreg-ref.py`).
- `crates/core/src/analysis/quantreg.rs`: one row per quantile, graph, refusals.
- `crates/cli/tests/analyze.rs`: `quantile_regression_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_runs_a_quantile_regression`.
