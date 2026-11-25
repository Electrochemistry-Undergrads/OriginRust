# T427: Exponential smoothing with a damped trend and prediction intervals

**Status:** done 2026-11-24 (Claude Windows "claude:gmail", statistics 42). T374's "Left" (Origin 2024b Time Series
Analysis app): Statistics > Exponential Smoothing Forecast... and `sheaf analyze forecast` gain a damped trend (the
factor given with `--phi` or fitted in [0.8, 0.98] with `--damped`) and prediction intervals at a chosen level for the
simple, Holt and additive Holt-Winters models (the additive-error state space variances), drawn on the graph. Not
measured in Origin.
**Size:** S. **Spec:** statistics 42.

## Left

- The ETS likelihood and AIC model choice, multiplicative-error models, simulated intervals for the multiplicative
  season.

## Tests

- `crates/calc/tests/expsmooth_pi_reference.rs`: five fixed cases and two fitted dampings against numpy/scipy
  (`tools/expsmooth-pi-ref.py`, which also simulates each interval variance).
- `crates/calc/src/expsmooth.rs`: phi 1 is the linear trend, damped steps shrink by phi, the simple-smoothing variance by
  hand, refusals.
- `crates/core/src/analysis/expsmooth.rs`, `crates/cli/tests/analyze.rs`
  (`exponential_smoothing_forecast_runs_from_the_command_line`), `crates/app/src/app/ui_tests.rs`
  (`the_statistics_menu_makes_an_exponential_smoothing_forecast`).
