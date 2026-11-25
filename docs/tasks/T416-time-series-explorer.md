# T416: Plot > Specialized > Time Series Explorer (Origin 2024b)

**Status:** done 2026-11-24 (Claude Windows "claude:work", graphing "Time Series Explorer"). Plot > Specialized > Time
Series Explorer plots the selected columns as a Zoom graph (T112). The upper layer shows the whole series over its
exact X extent, with the region as a full-height band covering one of 10 equal X intervals; the lower layer shows that
interval. The bar above the graph has Previous, Next, Play/Pause (one interval a second, stopping at the last), the
caption "Interval k of n" and the number of intervals (2 to 1000). A run of steps makes one undo step, and the
explorer saves with the project. `sheaf plot -t time-series-explorer` makes one too. Sheaf's own design (Origin 2016
has none).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Time Series Explorer".

## Left

- A detail Y axis that rescales to each interval, overlapping steps, playback speed, picking an interval by clicking
  the upper layer. (Intervals of a set length: T453.)

## Tests

- `crates/core/src/explorer.rs`: the exact X extent, the band of each interval, changing the count, save round trip,
  a plain Zoom graph refused.
- `crates/app/src/app/ui_tests/function_graph.rs`: `time_series_explorer_steps_and_plays_from_the_bar`.
