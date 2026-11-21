# T319: Pareto Chart - Binned Data

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", graphing 14.29). Measured Origin 2016's `plot_paretobin`.
It sorts the categories by their counts, most first and equal counts in sheet order, writes them to a sheet
ParetoBinN (Data, Count, Cumulative Frequency) and draws the raw-data Pareto chart from it. In Sheaf, Plot > Statistics
> Pareto with a category and a count column selected does the same (`Project::plot_pareto_binned`).
**Size:** S. **Follows:** T83, T316. **Spec:** graphing 14.29.

## Left

- Combine Smaller Values (By Percent / By Count into "Other") and Recalculate.
- A menu item of its own (Origin has "Pareto Chart-Binned Data" beside "Pareto Chart-Raw Data").

## Tests

- `crates/plot/src/grouped_tests.rs`: `a_binned_pareto_sorts_the_counts_into_a_new_sheet_as_origin`.
