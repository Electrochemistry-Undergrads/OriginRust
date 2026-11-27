# T495: ARIMA by exact maximum likelihood

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 43 "Exact maximum likelihood"). Left item of T376
and T446; Origin 2024b's Time Series Analysis app was not run, so the reference is statsmodels.
**Size:** M. **Source:** origin-2026-2027-gaps.md, "Time series modeling and forecasting": exact-likelihood ARIMA.

## What it does

Statistics > ARIMA Forecast... has Method: Conditional sum of squares (as before) or Exact maximum likelihood (CLI
`analyze arima --method ml`). The exact likelihood comes from the Kalman filter of the differenced series in
state-space form, started at its stationary covariance with sigma^2 concentrated out, maximized by Nelder-Mead from
the CSS fit; standard errors from the numerical Hessian; forecasts from the filtered state. The table adds the
log-likelihood. Seasonal orders and the AIC search work with either method.

## Tests

- `crates/calc/src/arima.rs`: `exact_likelihood_matches_statsmodels` (three models against statsmodels 0.15,
  `re-workspace/scratch/t480/arima_ml.py`).
- `crates/core/src/analysis/arima.rs`: `a_drifting_walk_and_its_forecast_bands`.
- `crates/cli/tests/analyze.rs`: `arima_forecast_runs_from_the_command_line`; app
  `the_statistics_menu_makes_an_arima_forecast` (the Log-likelihood row).

## Left

- The integrated part in the state (statsmodels' default diffuse start), stationarity and invertibility constraints
  on CSS fits, exact forecast variances from the state (the bands use the psi weights).
