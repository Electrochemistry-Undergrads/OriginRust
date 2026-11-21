# T316: Copy Origin 2016's measured Interval Plot and Pareto Chart

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", graphing 14.29). The 2026-11-16 measurement of Origin 2016
(Plot > Statistics > Interval Plot and Pareto Chart - Raw Data) listed the differences for a code slice; this does
them. The Pareto chart now has red bars with a black outline under "Count" from 0 to 1.1 times the total, and no X
title. Its cumulative percent is a thin blue line with blue squares at the bar centres (no (0, 0) start), labelled
above, on a right axis "Cumulative Frequency" from 0 to 110 with ticks every 20 labelled in percent. The interval
plot's Y title is "Range".
**Size:** S. **Follows:** T83. **Spec:** graphing 14.29.

## Left

- The interval plot's two-entry legend ("Mean ± 95% CI" and "Mean").
- 7 significant digits on the cumulative labels (a data-label option Sheaf lacks).
- Pareto Chart - Binned Data, drop lines, count table, "Other", groups.

## Tests

- `crates/plot/src/grouped_tests.rs`: `pareto_counts_sorts_and_accumulates` (points at the centres, titles,
  percent labels; golden `pareto.svg`), `interval_plot_draws_symbols_bars_and_column_names` (golden).
- `crates/core/src/template.rs`: `interval_and_pareto_templates_build_their_plots_and_refuse_a_wrong_selection`.
