# T370: Seasonal decomposition

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 39). HANDOFF C (Origin 2024b Pro Time Series
Analysis app, "separate trend and seasonality"): Statistics > Seasonal Decomposition... and `sheaf analyze decompose`
split each series into a centred moving-average trend, seasonal indices and a residual (additive or multiplicative),
write the indices and the three parts and draw three graphs. Not measured in Origin.
**Size:** S. **Spec:** statistics 39.

## Left

- STL (loess) decomposition, trend extrapolation to the ends, stationarity tests (ADF, KPSS), ARIMA.

## Tests

- `crates/calc/tests/decompose_reference.rs`: monthly additive, quarterly multiplicative and weekly (odd period)
  series against numpy (`tools/decompose-ref.py`).
- `crates/calc/src/decompose.rs`: an exact line plus pattern, the multiplicative indices, refusals.
- `crates/core/src/analysis/decompose.rs`: indices, the three graphs and their columns.
- `crates/cli/tests/analyze.rs`: `seasonal_decomposition_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_decomposes_a_series`.
