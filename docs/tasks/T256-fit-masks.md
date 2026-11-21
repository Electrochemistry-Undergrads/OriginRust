# T256: Masked cells in the other fits

**Status:** done 2026-11-20 (Claude Windows "claude:3"). Bug fix (T253 family). The main fit reads its Y
column with masks and the worksheet filter (`Worksheet::values_of`); Compare Datasets, Compare and
Rank Models and the global fit (all through `collect_fit_data`), Multiple Linear Regression (Y and
every X) and the implicit circle/ellipse fit read raw numbers, so masked cells and filter-hidden
rows still counted. They now read them as the main fit does.
**Size:** S. **Source:** a check of every fit's column reads after T253.

## Tests

- `crates/core/src/fitting/more.rs`: `masked_cells_are_left_out_of_regression_ranking_and_comparisons`.
- `crates/core/src/fitting/datasets.rs`: `the_implicit_fit_leaves_masked_cells_out`.
