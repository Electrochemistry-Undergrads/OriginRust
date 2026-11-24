# T404: Time Series Pivot intervals and fiscal years

**Status:** done 2026-11-23 (Claude Windows "claude:1", statistics 32), not measured (Origin 2025 feature).
Worksheet > Time Series Pivot and `sheaf analyze timepivot` take groups of N periods (`--every`, Every) and a fiscal
year start month (`--fiscal-start`, Fiscal year starts in) for months, quarters and years; fiscal quarters and years
are written `FY2027 Q1` and `FY2027`. `TimePivotSpec::every`, `TimePivotSpec::fiscal_start`.
**Size:** S. **Source:** HANDOFF C, origin-2026-2027-gaps "Time Series Pivot" (the rest of T358).

## Tests

- `crates/core/src/analysis/timepivot.rs`: `custom_intervals_count_from_midnight_the_first_day_or_the_year`,
  `fiscal_years_start_in_their_month_and_are_named_by_their_end`.
- `crates/cli/tests/analyze.rs`: `time_series_pivot_runs_from_the_command_line` (`--every`, `--fiscal-start`).
- `crates/app/src/app/ui_tests.rs`: `the_worksheet_menu_makes_a_time_series_pivot` (the new options).

## Left

- A second grouping (for example hour of day within each month), custom intervals that do not start at a unit.
