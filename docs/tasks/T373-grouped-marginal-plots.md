# T373: Grouped marginal plots

**Status:** done 2026-11-22 (Claude Windows "claude:1", graphing 32), not measured (Origin 2021b feature;
Origin 2016 has no grouped version). `sheaf plot -t marginal-histograms|marginal-box-charts --group G` draws a
scatter per group with each group's histogram (half transparent) or box (side by side) in the margins, in the
group's colour; `Project::plot_marginal_groups`, `PlotDecor::marginal_slot`. With one Y column, every
`--group` plot now titles its Y axis with that column's name instead of the first group's value.
**Size:** S. **Source:** HANDOFF C, origin-2026-2027-gaps "Network and grouped marginal plots".

## Tests

- `crates/core/src/marginal.rs`: `grouped_marginal_plots_have_a_scatter_and_a_histogram_or_box_per_group`.
- `crates/cli/tests/plot_by_group.rs`: `grouped_marginal_box_charts_have_a_box_per_group_in_each_margin`.

## Left

- A group column in the Plot > Statistics windows; kernel density margins. (Shared bins across groups: T508; network
  plots: T383.)
