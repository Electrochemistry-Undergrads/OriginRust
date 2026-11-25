# T376: ARIMA forecast

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 43). HANDOFF C (Origin 2024b Pro Time Series
Analysis app, "use ARIMA to forecast future observations"): Statistics > ARIMA Forecast... and `sheaf analyze arima`
fit ARIMA(p, d, q) with or without a constant by conditional least squares and forecast with confidence bands from
the psi weights, with predictions, residuals and a graph. Not measured in Origin.
**Size:** S. **Spec:** statistics 43.

## Left

- Coefficient standard errors and the AIC order search are T428. Exact (Kalman filter) maximum likelihood, seasonal
  ARIMA, stationarity/invertibility constraints.

## Tests

- `crates/calc/tests/arima_reference.rs`: AR(2), ARMA(1,1), ARIMA(1,1,1) with drift and ARIMA(0,2,1) against numpy
  recursions and scipy's minimum (`tools/arima-ref.py`).
- `crates/calc/src/arima.rs`: AR(1) errors and forecasts by hand, a drifting walk, refusals.
- `crates/core/src/analysis/arima.rs`: coefficient rows, forecasts and the plotted columns.
- `crates/cli/tests/analyze.rs`: `arima_forecast_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_makes_an_arima_forecast`.
