# T83: Interval Plot and Pareto (B4 slice)

**Status:** Done 2026-11-16 (Claude macOS, branch `interval-pareto`; graphing 14.29). Built from Origin's documentation on a Origin 2016 measurements recorded 2026-11-16 (Claude Windows, docs only, graphing 14.29): the interval statistic matches; left for a code slice: the two-entry legend and "Range" title, Pareto points at bar centres without (0, 0), point labels, 1.1 x total count axis aligned with a 0..110 percent axis, "Cumulative Frequency", blue line/squares. T316 (2026-11-21) copied Origin 2016's measured Pareto
(colours, axes 0..1.1 x total and 0..110 %, points at the bar centres, labels, titles) and the interval plot's Y
title "Range".
machine without Origin: every choice the documentation does not give is Sheaf's and **unmeasured** (14.29).
**Size:** M. **Branch:** `interval-pareto`.
**Spec:** [graphing.md 14.29](../spec/graphing.md). **Inventory:** Plot > Statistics > Interval Plot and Pareto (2 greyed
items, both `B4`).

## Goal

Plot > Statistics > Interval Plot (template `interval-plot`: the mean of each selected Y column with an error bar) and Pareto
(template `pareto`: the counts of the distinct values of one column as columns, most frequent first, with the cumulative
percent on a right axis of 0 to 100). Both plot straight from the selection, without Origin's dialogs.

## Origin facts (documentation only, not measured; paraphrased)

- Interval Plot: a box chart with the box hidden, so only the mean shows as a solid circle (template BoxInterval).
  [W: docs.originlab.com/origin-help/interval-plot] The default error statistic is not documented there.
- Pareto: Origin has a binned (`plot_paretobin`) and a raw-data (`plot_paretoraw`, template ParetoRaw) command. Bars sorted
  from most to least significant; the cumulative percent is a line with symbols in a new layer on a right axis that always
  runs 0 to 100. [W: docs.originlab.com/origin-help/create-pareto-chart, .../paretochart-rawdata,
  docs.originlab.com/x-function/ref/plot_paretoraw]

## Steps

1. Core: `Template::IntervalPlot`/`Pareto`, `Plot.stat` (`StatPlot`, `IntervalError`), `Project::plot_interval`/`plot_pareto`,
   `indexed::counts_descending`.
2. Plot: mean and t/SD/SE half width, counts and running percent, category axes (`plot/src/data.rs`, `layout.rs`).
3. App: both menu items, selection rule and status warnings, Plot Details > Error Bar for interval plots.
4. CLI/MCP: `sheaf plot -t interval-plot [--interval-error ci|sd|se] [--interval-level P]`, `-t pareto -y COLUMN`.
5. `.opj` export writes plain plots with a save warning; `.sheaf` round-trips the field.
6. Tests (core, plot `grouped_tests.rs` with goldens, app, cli `tests/interval_pareto.rs`, io); docs.

## Done when

Both items are enabled and plot as 14.29 says; fmt, strict Clippy and the tests pass; `tools/greyed-count.py --check` passes.

## Don't

Touch Column Scatter (T79), Histogram + Probabilities, marginal plots, 2D kernel density, the QC chart, Scatter Matrix or
stock plots; claim Origin-measured behaviour.
