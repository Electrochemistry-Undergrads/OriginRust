# T509: Percentile lives against a covariate

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 48 "Graph"). Left item of T437 (claude:gmail's); not
measured in Origin.
**Size:** S. **Source:** T437 "Left".

## What it does

Statistics > Survival Analysis > Life Data Regression... can draw the chosen percentile lives against the first
covariate over its range (the others at their means), with their intervals dotted, on a log Y axis; CLI `analyze
lifereg --graph`. Off by default.

## Tests

- `crates/core/src/analysis/lifedist.rs`: `life_regression_percentiles_at_the_means_and_at_given_values` (the graph).
- `crates/cli/tests/analyze.rs`: the life regression test (`--graph`).

## Left

- Choosing the covariate drawn (always the first), interval-censored regression, factor covariates, residual plots.
