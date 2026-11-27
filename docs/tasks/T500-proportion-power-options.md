# T500: Exact, pooled and unequal-group powers of the proportion tests

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 12.1 "Options"). Left item of T415
(claude:gmail's); Origin's PSS proportion tools could not be measured, so the references are scipy and statsmodels.
**Size:** S. **Source:** T415 "Left".

## What it does

Statistics > Power and Sample Size > One-Proportion Test can use the exact binomial test (Method), Two-Proportion Test
the pooled z test (Standard error), and the two-sample proportion and variance tests a ratio of group sizes; CLI
`analyze power --exact`, `--pooled`, `--ratio R`.

## Tests

- `crates/calc/src/power.rs`: `exact_pooled_and_unequal_proportion_powers_match_scipy_and_statsmodels`.
- `crates/cli/tests/analyze.rs`: `power_of_proportion_and_variance_tests_runs_from_the_command_line`.

## Left

- Measuring Origin's PSS tools; exact (Fisher) two-proportion powers; continuity corrections.
