# T394: Frequency column for the life-data tools

**Status:** done 2026-11-23 (Claude Windows "claude:gmail", statistics 49). HANDOFF C (Origin 2026b "an optional
frequency input" for life data): Distribution Overview Plot, Distribution Analysis (Arbitrary Censoring) and Life Data
Regression take a last column of counts ("Last column: frequency (count) of each row"; `--frequency` on the CLI); a row
of count k is k copies of it. Not measured in Origin.
**Size:** S. **Spec:** statistics 49.

## Left

- Frequency columns for the Kaplan-Meier estimator and Cox regression are T429. Fractional weights (case weights).

## Tests

- `crates/core/src/analysis/lifedist.rs`: `a_frequency_counts_a_row_as_often_as_copies_of_it` (overview and arbitrary
  censoring equal the written-out rows; a fractional count refused).
- `crates/cli/tests/analyze.rs`: `life_data_frequency_columns_count_rows`.
