# T114: Scatter Matrix

**Status:** done 2026-11-16 (Claude macOS "mac-scatter-matrix", graphing.md 35); measured against Origin 2016 and
matched by T313 (2026-11-21): landscape page, 8 %/5 % margins with 87/n % cells, tick labels on every outer side
(All), names as axis titles, blue size-9 squares, a range/4 two-digit tick step. Left: Origin's axis-limit rule,
its triangular layouts, box charts on the diagonal, the other Show Tick and Label modes, ellipse, fit, statistics.
**Size:** M. **Source:** greyed inventory package B4 (Plot > Statistics > Scatter Matrix).
**Spec:** `docs/spec/graphing.md` section 35.

## Goal

Origin's scatter matrix: every pair of the selected columns as a scatter, each in its own layer of one graph,
in an n x n grid (column c on X, row r on Y), with the variables' names (or histograms) on the diagonal.

## Origin facts

OriginLab's pages for Plot > Statistical > Scatter Matrix and the `plot_matrix` X-Function: two or more
columns; Matrix Display Square (default), Upper/Lower Triangular, Mixed; Show in Diagonal Cells None
(default), Box, Histogram, Histogram + Distribution, Distribution, Distribution with fill; Variables in
Diagonal Cells on; Show Tick and Label All; Gap 0 % of the page; confidence ellipse (95 %), linear fit,
Pearson's r, adjusted R², equation, significance marks, grouping and listwise missing values. Not measured
in Origin 2016.

## Where

- `crates/core/src/scatter_matrix.rs` (+ `tests.rs`), `template.rs` (`Template::ScatterMatrix`),
  `project.rs` (errors, `plot_columns`).
- `crates/app/src/dialogs/scatter_matrix.rs`, `app.rs` (`plot_columns`, `show_scatter_matrix`), `menus.rs`,
  `ui_tests/scatter_matrix.rs`.
- `crates/ops` (`--matrix-*`), `crates/mcp` (`matrix_*`), `crates/cli/tests/scatter_matrix.rs`.

## Done when

The menu item plots as documented, the inventory no longer lists it, and fmt, strict Clippy and the full
tests pass.

## Don't

Rebuild histograms or scatter drawing: the cells are ordinary layers. Link the cells' scales (a layer links to
one parent; a cell would need its grid column's X and its grid row's Y).
