# T504: Percentile lives at several covariate settings

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 48). Left item of T437 (claude:gmail's); not
measured in Origin.
**Size:** S. **Source:** T437 "Left".

## What it does

Statistics > Survival Analysis > Life Data Regression... predicts the percentile lives at several covariate settings
at once: the window's "at covariates" takes settings separated by `;`, the CLI `--at` and repeated `--also-at`. Each
setting gets its own Percent, Percentile life, Lower and Upper columns (numbered) and note.

## Tests

- `crates/core/src/analysis/lifedist.rs`: `life_regression_percentiles_at_the_means_and_at_given_values`.
- `crates/cli/tests/analyze.rs`: the life regression test (`--also-at`).

## Left

- A percentile-against-covariate graph, interval-censored regression, factor covariates, residual plots.
