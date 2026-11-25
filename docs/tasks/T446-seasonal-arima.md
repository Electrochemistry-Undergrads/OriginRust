# T446: Seasonal ARIMA

**Status:** done 2026-11-25 (Claude Windows "claude:gmail", statistics 43). T376's "Left" (Origin 2024b Time Series
Analysis app): Statistics > ARIMA Forecast... (Seasonal P, D, Q and period) and `sheaf analyze arima --sp --sd --sq
--period` fit multiplicative seasonal ARIMA by conditional least squares, with forecasts, bands and coefficient
standard errors. Not measured in Origin.
**Size:** S. **Spec:** statistics 43.

## Left

- Exact (Kalman filter) maximum likelihood, a search over the seasonal orders, stationarity/invertibility constraints.

## Tests

- `crates/calc/tests/sarima_reference.rs`: three seasonal models against numpy/scipy (`tools/sarima-ref.py`).
- `crates/calc/src/arima.rs`: a seasonal random walk with drift forecasts last season plus the drift; the expanded
  polynomials by hand; a period below 2 is refused.
- `crates/cli/tests/analyze.rs` (`arima_forecast_runs_from_the_command_line`), `crates/app/src/app/ui_tests.rs`
  (`the_statistics_menu_makes_an_arima_forecast`).
