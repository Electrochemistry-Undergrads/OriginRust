# T498: STL decomposition

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 40 "STL"). Left item of T370 (claude:gmail's);
Origin 2024b's Time Series Analysis app was not run, so the reference is statsmodels' `STL`.
**Size:** M. **Source:** T370 "Left": STL (loess) decomposition.

## What it does

Statistics > Seasonal Decomposition... can decompose by STL (seasonal-trend decomposition by loess, Cleveland et al.
1990) with a seasonal smoother length and robust passes; CLI `analyze decompose --stl [--stl-seasonal 7] [--robust]`.
The trend, seasonal part (changing from cycle to cycle) and residual go to the sheet as before, with robustness
weights when robust; the seasonal indices are the mean seasonal values by position.

## Tests

- `crates/calc/src/stl.rs`: `stl_matches_statsmodels` (statsmodels 0.15, `re-workspace/scratch/t480/stl_ref.py`).
- `crates/core/src/analysis/decompose.rs`: `indices_rows_and_three_graphs_of_their_own_columns` (the STL columns).
- `crates/cli/tests/analyze.rs`: the decomposition test (`--stl --robust`, an even smoother refused).

## Left

- Smoother degrees 0, jumps for long series, a multiplicative STL through logarithms, MSTL (several periods).
