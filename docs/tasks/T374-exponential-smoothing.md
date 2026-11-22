# T374: Exponential smoothing forecast

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 42). HANDOFF C (Origin 2024b Pro Time Series
Analysis app, "forecast future observations"): Statistics > Exponential Smoothing Forecast... and `sheaf analyze
forecast` smooth each series by simple, Holt or additive or multiplicative Holt-Winters exponential smoothing with
fitted or given constants, and forecast a number of steps, with the one-step forecasts, residuals and a graph. Not
measured in Origin.
**Size:** S. **Spec:** statistics 42.

## Left

- Prediction intervals, damped trends, the ETS likelihood (AIC model choice), ARIMA.

## Tests

- `crates/calc/tests/expsmooth_reference.rs`: the four models at fixed constants against numpy, and the fitted SSE
  against scipy (`tools/expsmooth-ref.py`).
- `crates/calc/src/expsmooth.rs`: the first forecast and the trend of Holt forecasts, simple smoothing by hand, the fit
  beating a grid, refusals.
- `crates/core/src/analysis/expsmooth.rs`: the table, the continued X step and the plotted columns.
- `crates/cli/tests/analyze.rs`: `exponential_smoothing_forecast_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_makes_an_exponential_smoothing_forecast`.
