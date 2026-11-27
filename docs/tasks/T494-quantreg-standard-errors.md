# T494: Quantile Regression standard errors

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 29). Left item of T342 (claude:gmail's); Origin's
Quantile Regression app (2019b) was not run, so the method is statsmodels' and Stata 12's.
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Quantile regression and change-point analysis": no standard errors.

## What it does

Each coefficient of Statistics > Quantile Regression... gets its standard error and Prob>|t|: a kernel estimate of the
residual density at 0 (Epanechnikov, Hall-Sheather bandwidth), robust (sandwich, the default) or iid. Option
Standard errors (Robust / iid / None), CLI `analyze quantreg --se robust|iid|none`.

## Tests

- `crates/calc/src/quantreg.rs`: `standard_errors_match_statsmodels_formulas` (statsmodels 0.15's formulas at the same
  coefficients, `re-workspace/scratch/t480/qr_se.py`).
- `crates/core/src/analysis/quantreg.rs`: `one_row_per_quantile_and_a_graph_of_the_lines`.
- `crates/cli/tests/analyze.rs`: `quantile_regression_runs_from_the_command_line`.

## Left

- Confidence limits by rank inversion (R's default for small samples), bootstrap errors, tests between quantiles.
