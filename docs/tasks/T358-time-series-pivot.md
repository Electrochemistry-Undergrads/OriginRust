# T358: Time Series Pivot

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 32). HANDOFF C (Origin 2025 Time Series
Pivot): Worksheet > Time Series Pivot... and `sheaf analyze timepivot` summarize each chosen column per calendar
period (minute, hour, day, Monday-start week, month, quarter, year) of its date-time X column by sum, mean, count,
minimum, maximum or median, into a sheet with the period label and start and the statistic and count of every
column. Not in Origin 2016, so the rules and layout are Sheaf's own.
**Size:** S. **Spec:** statistics 32.

## Left

- Fiscal calendars, custom intervals, a second period grouping, the period start shown in a date format.

## Tests

- `crates/core/src/analysis/timepivot.rs`: period starts and labels, summaries per day and month.
- `crates/cli/tests/analyze.rs`: `time_series_pivot_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_worksheet_menu_makes_a_time_series_pivot`.
