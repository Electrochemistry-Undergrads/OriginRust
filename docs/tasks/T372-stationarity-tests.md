# T372: Stationarity tests (ADF, KPSS)

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 41). HANDOFF C (Origin 2024b Pro Time Series
Analysis app, "check stationarity"): Statistics > Stationarity Tests (ADF, KPSS)... and `sheaf analyze stationarity`
give the augmented Dickey-Fuller statistic (lags by AIC or fixed; constant, trend or none) with MacKinnon's asymptotic
critical values, and the KPSS statistic (level or trend) with its table probability, with the 5 % decisions. Not
measured in Origin.
**Size:** S. **Spec:** statistics 41.

## Left

- ADF p-values, finite-sample critical values and automatic KPSS lags: done (T497, checked against statsmodels);
  Phillips-Perron remains (ARIMA: T376).

## Tests

- `crates/calc/tests/stationarity_reference.rs`: a random walk, an AR(1) and a trend, ADF with every term choice
  (AIC and fixed lags) and KPSS (both nulls, default and fixed lags), against numpy (`tools/stationarity-ref.py`).
- `crates/calc/src/stationarity.rs`: a walk and its steps, refusals.
- `crates/core/src/analysis/stationarity.rs`: rows, decisions, too many lags refused.
- `crates/cli/tests/analyze.rs`: `stationarity_tests_run_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_runs_stationarity_tests`.
