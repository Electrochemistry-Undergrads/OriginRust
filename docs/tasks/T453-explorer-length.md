# T453: Time Series Explorer intervals of a set length (rest of T416)

**Status:** done 2026-11-25 (Claude Windows "claude:work", graphing "Time Series Explorer"). By length on the explorer
bar splits the series into intervals of a set X length (a day, a week of dates) instead of equal parts. The last
interval may be shorter. The count follows the length, and turning it off goes back to equal parts.
**Size:** S. **Source:** T416 "Left".

## Tests

- `crates/core/src/explorer.rs`: 30 units over 0..100 make 4 intervals, the last one 10 long; too short a length is
  refused; back to equal parts.
- `crates/app/src/app/ui_tests/function_graph.rs`: `time_series_explorer_steps_and_plays_from_the_bar` (By length).
