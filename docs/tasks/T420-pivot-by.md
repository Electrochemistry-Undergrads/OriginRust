# T420: Time Series Pivot second grouping

**Status:** done 2026-11-24 (Claude Windows "claude:1", statistics 32), not measured (Origin 2025 feature). Within
each period the values can be grouped again by the hour of the day, the day of the week or the month of the year
(`--by hour|weekday|month`, the options window's "Within each period, by"): one result column per group with values,
for example "Mean of v (Mon)". `TimePivotSpec::by`, `PivotBy`.
**Size:** S. **Source:** HANDOFF C, the rest of T358 and T404.

## Tests

- `crates/core/src/analysis/timepivot.rs`: `a_second_grouping_gives_a_column_per_group_with_values`.
- `crates/cli/tests/analyze.rs`: `time_series_pivot_runs_from_the_command_line` (`--by`).
- `crates/app/src/app/ui_tests.rs`: `the_worksheet_menu_makes_a_time_series_pivot` (the option is shown).

## Left

- Other second groupings (minute of hour, day of month), N columns per group.
