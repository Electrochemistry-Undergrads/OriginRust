# T392: Nested frequency charts

**Status:** done 2026-11-23 (Claude Windows "claude:1", graphing after 14.27), not measured (Origin 2026 feature).
A treemap or sunburst plot of text columns only counts its rows (every row 1): nested frequency rectangles and
doughnuts, in the app (select the text columns) and with `sheaf plot -t sunburst|treemap` without a numeric `-y`.
**Size:** S. **Source:** HANDOFF C, origin-2026-2027-gaps "Cause-and-effect and nested frequency charts".

## Tests

- `crates/core/src/template.rs`: the treemap and sunburst template tests (text columns only now plot, counted).
- `crates/plot/src/sunburst_tests.rs`: `text_columns_alone_give_a_nested_frequency_doughnut`.
- `crates/cli/tests/sankey.rs`: `treemaps_and_sunbursts_of_text_columns_count_the_rows`.
- `crates/app/src/app/ui_tests.rs`: `a_sunburst_of_selected_text_columns_counts_the_rows`.
