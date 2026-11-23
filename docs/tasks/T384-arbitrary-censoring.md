# T384: Distribution analysis with arbitrary censoring

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 47). HANDOFF C (Origin 2026b "distribution
analysis with arbitrary censoring"): Statistics > Survival Analysis > Distribution Analysis (Arbitrary Censoring)... and
`sheaf analyze lifedist-interval` fit a Weibull, lognormal, exponential or normal distribution by maximum likelihood to
start/end columns holding exact, right-, left- and interval-censored failures, with counts, the fit and a CDF and
survival graph. Not measured in Origin.
**Size:** S. **Spec:** statistics 47.

## Left

- Turnbull nonparametric estimates and probability plots for interval data, a frequency column, life-data regression,
  more distributions.

## Tests

- `crates/calc/tests/interval_censor_reference.rs`: two inspection-interval samples, four distributions, against scipy
  (`tools/interval-censor-ref.py`).
- `crates/calc/src/lifedata.rs`: the arbitrary fit reproduces the right-censored fit of T366; the column rules.
- `crates/core/src/analysis/lifedist.rs`: counts, the fit, the plotted columns, a start after its end refused.
- `crates/cli/tests/analyze.rs`: `distribution_analysis_with_arbitrary_censoring_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_survival_menu_fits_arbitrarily_censored_data`.
