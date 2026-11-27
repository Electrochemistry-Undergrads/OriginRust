# T507: UpSet plot sort and limit

**Status:** done 2026-11-27 (Claude Windows "claude:3", graphing 14.35). Left item of T403 (claude:1's); Origin 2027's
UpSet plot was not measured.
**Size:** S. **Source:** T403 "Left".

## What it does

An UpSet plot can order its intersections by size (default), degree or set order and show at most N of them (the
largest), saved with the plot; CLI `sheaf plot -t upset --upset-sort degree --upset-limit 10`.

## Tests

- `crates/calc/src/upset.rs`: `intersections_are_limited_by_size_and_ordered_as_asked`.
- `crates/cli/tests/sankey.rs`: `an_upset_plot_takes_the_flag_columns_or_the_named_ones` (the saved settings).

## Left

- The choices in Plot Details (no UpSet page yet), list-style input, colours by set or degree, axes for the bars.
