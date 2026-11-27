# T497: ADF p-values, finite-sample critical values, automatic KPSS lags

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 41). Left item of T372 (claude:gmail's); Origin
2024b's app was not run, so the reference is statsmodels.
**Size:** S. **Source:** T372 "Left" (the coefficients could not be checked before statsmodels was available).

## What it does

Statistics > Stationarity Tests (ADF, KPSS)... gives the ADF probability (MacKinnon 1994) and MacKinnon's (2010)
critical values for the regression's points (they were asymptotic), and can take the KPSS lags by Hobijn et al.'s
automatic rule (window: KPSS lags Automatic (Hobijn); CLI `--kpss-auto`), as statsmodels' `adfuller` and `kpss`.

## Tests

- `crates/calc/src/stationarity.rs`: `adf_p_values_critical_values_and_kpss_auto_lags_match_statsmodels`.
- `crates/core/src/analysis/stationarity.rs`: `a_walk_and_its_steps`; CLI `stationarity_tests_run_from_the_command_line`;
  app `the_statistics_menu_runs_stationarity_tests`.

## Left

- The Phillips-Perron test.
