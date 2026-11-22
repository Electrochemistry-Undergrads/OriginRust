# T356: Binned scatter

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 31). HANDOFF C (Origin 2027 binned scatter):
Statistics > Descriptive Statistics > Binned Scatter... and `sheaf analyze binscatter` cut X into bins of equal counts
(default) or equal width and write each bin's X range and mean, N, Y mean, SD and SE, with a graph of the data and the
bin means. Not in Origin 2016, so the rules and layout are Sheaf's own.
**Size:** S. **Spec:** statistics 31.

## Left

- Error bars (SD, SE or confidence) on the bin means, a fitted line through them, bins by a typed width.

## Tests

- `crates/calc/src/binscatter.rs`: equal counts with the extra points first, equal widths with empty bins and the
  right edge, refusals.
- `crates/core/src/analysis/binscatter.rs`: one row per bin, graph, refusal.
- `crates/cli/tests/analyze.rs`: `binned_scatter_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_descriptive_menu_makes_a_binned_scatter`.
