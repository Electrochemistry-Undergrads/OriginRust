# T75: Grouped Columns (indexed data) and Grouped Box Charts (B4 slice)

**Status:** Done 2026-11-16 (Claude macOS, branch `grouped-indexed`; graphing 14.28). Built from Origin's documentation on a
machine without Origin: every choice the documentation does not give is Sheaf's and **unmeasured** (14.28).
**Size:** M. **Branch:** `grouped-indexed`.
**Spec:** [graphing.md 14.28](../spec/graphing.md). **Inventory:** Plot > Column/Bar/Pie > Grouped Columns (indexed data) and
Plot > Statistics > Grouped Box Charts (2 greyed items, both `B4`).

## Goal

Two plots of *indexed data* (one numeric data column whose rows are labelled by the values of one or two group columns):
Plot > Column/Bar/Pie > Grouped Columns (indexed data) (template `grouped-columns`) and Plot > Statistics > Grouped Box
Charts (template `grouped-box`). Both plot straight from the selected columns, without Origin's dialog.

## Origin facts (documentation only, not measured; paraphrased)

- The X-Functions `plot_gindexed` and `plot_gboxindexed`, available since Origin 9.1 SR0 (so in 2016); Origin's items open a
  dialog (data form, statistics, themes, panels). [W: docs.originlab.com/x-function/ref/plot_gindexed,
  docs.originlab.com/x-function/ref/plot_gboxindexed, docs.originlab.com/origin-help/grouped-column-index-data,
  docs.originlab.com/origin-help/groupedboxcharts-indexeddata]
- One or more group columns; their order is the hierarchy (the first on the axis, the next nested). Groups sort alphabetically
  by default. The result is one grouped plot; with two group columns the X tick labels are a two-row table. The fill colour is
  indexed by the subgroup by default.

## Steps

1. Core: `Template::GroupedColumns`/`GroupedBox`, `Plot.indexed`, `Project::plot_indexed`, level helpers (`core/src/indexed.rs`).
2. Plot: means with SD/SE error bars, dodged boxes per level, category X axis (`plot/src/data.rs`, `layout.rs`, `plots.rs`).
3. App: both Plot menu items, selection rule and status warning, Plot Details > Error Bar (None, SD, SE).
4. CLI/MCP: `sheaf plot -t grouped-columns|grouped-box -y DATA --group G1[,G2] [--group-error none|sd|se]`.
5. `.opj` export writes plain plots with a save warning; `.sheaf` round-trips the field.
6. Tests (core, plot, app `ui_tests/grouped_indexed.rs`, cli `tests/grouped_indexed.rs`, io `graph_feature_tests.rs`); docs.

## Done when

Both items are enabled and plot as 14.28 says; fmt, strict Clippy and the tests pass; `tools/greyed-count.py --check` passes.

## Don't

Touch Interval Plot, Line Series, Column Scatter or other statistics plots; claim Origin-measured behaviour.
