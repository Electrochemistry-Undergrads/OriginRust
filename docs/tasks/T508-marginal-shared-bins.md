# T508: Grouped marginal histograms share their bins

**Status:** done 2026-11-27 (Claude Windows "claude:3", graphing 32). Left item of T373 (claude:1's); Origin 2021b's
grouped marginal plots were not measured.
**Size:** S. **Source:** T373 "Left".

## What it does

`sheaf plot -t marginal-histograms -y Y --group G` (and `Project::plot_marginal_groups`) bins every group's
histogram in a margin on the same edges: the automatic bins of all the groups' values of that margin, written into
each group's histogram settings, so the groups compare bin for bin.

## Tests

- `crates/core/src/marginal.rs`: `grouped_marginal_plots_have_a_scatter_and_a_histogram_or_box_per_group`.

## Left

- A group column in the Plot > Statistics windows; kernel density margins.
