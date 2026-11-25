# T429: Frequency columns for the Kaplan-Meier estimator and Cox regression

**Status:** done 2026-11-24 (Claude Windows "claude:gmail", statistics 49). T394's "Left" (Origin 2026b's frequency
input for life data): Statistics > Survival Analysis > Kaplan-Meier Estimator... and Cox Proportional Hazards... take
"Last column: frequency (count) of each row", and `sheaf analyze kaplan-meier` and `cox` take `--frequency`; a row
counts as often as its frequency, as in the life-data tools. Not measured in Origin.
**Size:** S. **Spec:** statistics 49.

## Left

- Fractional (case) weights.

## Tests

- `crates/core/src/analysis/lifedist.rs`: `a_cox_frequency_counts_a_row_as_often_as_copies_of_it`.
- `crates/cli/tests/analyze.rs`: `kaplan_meier_runs_from_the_command_line` (frequency against written-out rows, Cox
  case count).
