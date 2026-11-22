# T366: Distribution Overview plot (right-censored life data)

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 37). HANDOFF C (Origin 2026b "Distribution
Overview plot for right-censored data"): Statistics > Survival Analysis > Distribution Overview Plot... and `sheaf
analyze lifedist` fit a Weibull, lognormal, exponential or normal distribution by maximum likelihood to time and censor
columns, write the parameters, log-likelihood, AIC, mean and median, and draw four graphs: a probability plot with the
modified Kaplan-Meier points, the density, the survival with the Kaplan-Meier points, and the hazard. Not measured in
Origin.
**Size:** S. **Spec:** statistics 37.

Also fixed here: the graphs of the change point (T346), Manhattan (T362), autocorrelation (T363), rare event chart
(T364) and process capability (T365) tools plotted table columns instead of their own data; their tests now check the
plotted columns.

## Left

- Arbitrary (interval, left) censoring, a frequency column, confidence bands on the probability plot, more
  distributions (gamma, log-logistic, three-parameter forms), several groups, regression with life data.

## Tests

- `crates/calc/tests/lifedata_reference.rs`: two censored samples, four distributions, against scipy
  (`tools/lifedata-ref.py`), plus the plotting positions.
- `crates/calc/src/lifedata.rs`: exponential, uncensored normal and Weibull identities, positions, refusals.
- `crates/core/src/analysis/lifedist.rs`: a missing time, the exponential fit, the four graphs and their columns.
- `crates/cli/tests/analyze.rs`: `distribution_overview_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_survival_menu_makes_a_distribution_overview`.
