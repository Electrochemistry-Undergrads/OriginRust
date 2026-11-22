# T363: Autocorrelation

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 34). HANDOFF C (Origin 2024b Pro Time Series
Analysis app, "inspect autocorrelation"): Statistics > Autocorrelation... and `sheaf analyze acf` compute the ACF with
Bartlett bounds, the PACF (Durbin-Levinson) with `+-z/sqrt n` bounds and the Ljung-Box Q and probability per lag,
write one row per lag and draw the ACF and PACF graphs. Not in Origin 2016, so the layout is Sheaf's own.
**Size:** S. **Spec:** statistics 34.

## Left

- The rest of the Time Series Analysis app: stationarity tests (ADF, KPSS), trend and seasonal decomposition, ARIMA
  fitting and forecasting.

## Tests

- `crates/calc/tests/acf_reference.rs`: three series (AR(1), seasonal, white noise) against numpy/scipy, with the
  PACF from solving the Yule-Walker equations (`tools/acf-ref.py`).
- `crates/calc/src/acf.rs`: an alternating series, the default lag count, refusals.
- `crates/core/src/analysis/acf.rs`: default lags with a missing value, bounds, both graphs, refusals.
- `crates/cli/tests/analyze.rs`: `autocorrelation_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_computes_the_autocorrelation`.
