# T428: ARIMA coefficient standard errors and order choice by AIC

**Status:** done 2026-11-24 (Claude Windows "claude:gmail", statistics 43). T376's "Left" (Origin 2024b Time Series
Analysis app): Statistics > ARIMA Forecast... and `sheaf analyze arima` give each coefficient's standard error, z and
probability (Gauss-Newton from the exact derivatives of the conditional errors), and `--auto` (the window's "Choose p
and q by AIC") fits every order up to a maximum and keeps the smallest AIC. Not measured in Origin.
**Size:** S. **Spec:** statistics 43.

## Left

- Exact (Kalman filter) maximum likelihood and its standard errors, seasonal ARIMA, a common sample for the AIC
  comparison, stationarity/invertibility constraints.

## Tests

- `crates/calc/tests/arima_se_reference.rs`: five models against complex-step derivatives and two AIC searches against
  scipy fits (`tools/arima-se-ref.py`).
- `crates/calc/src/arima.rs`: the SE of a mean and of an AR(1) coefficient by hand, the search beats every order it
  tried, a singular matrix.
- `crates/core/src/analysis/arima.rs`, `crates/cli/tests/analyze.rs` (`--auto`), `crates/app/src/app/ui_tests.rs`
  (`the_statistics_menu_makes_an_arima_forecast`).
