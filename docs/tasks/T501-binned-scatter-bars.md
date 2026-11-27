# T501: Binned scatter error bars, line and bin width

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 31). Left item of T356 (claude:gmail's); Origin
2027's binned scatter was not run, so this is Sheaf's own.
**Size:** S. **Source:** T356 "Left".

## What it does

Statistics > Descriptive Statistics > Binned Scatter... draws SD, SE or 95 % confidence error bars on the bin means,
the least-squares line of the data, and takes bins of a given width (edges at its multiples); CLI `analyze binscatter
--width W --error sd|se|ci --fit`. It also fixes the graph, whose plots pointed at the label and table columns of the
result sheet instead of the data and bin means. Analysis graph plans can now carry Y error columns (`Look::y_errors`).

## Tests

- `crates/calc/src/binscatter.rs`: `bins_of_a_width_error_bars_and_the_line`.
- `crates/core/src/analysis/binscatter.rs`: `one_row_per_bin_and_a_graph` (the plotted columns, error bars, line).
- `crates/cli/tests/analyze.rs`: the binned scatter test (`--width`, `--error ci`, `--fit`).

## Left

- Binned regression with control variables (Stata's binscatter), confidence bands of the line.
