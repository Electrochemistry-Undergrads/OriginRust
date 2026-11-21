# T321: Reduce Duplicate X Data writes into the input sheet

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Data reduction" (P1, gap
"reducedup writes a new sheet where Origin adds columns to the input sheet"; Origin's behaviour was
measured earlier, analysis-tools 12.1). `ReduceDupX<n>` (X) and `ReduceDupY<n>` (Y), and with the count
option `DupX<n>` and `DupXCount<n>`, now go to the end of the input sheet; the numbers go on after the
columns of earlier runs (a second run makes `ReduceDupX2`...). Also corrects the P1 row "Before-formula
script, multi-column set" after T307.
**Size:** S. **Spec:** `docs/spec/analysis-tools.md` 12.1.

## Tests

- `crates/core/src/analysis/manip.rs`: `reduce_duplicates_writes_origins_columns_at_the_end_of_the_sheet`.
- `crates/app/src/app/ui_tests.rs`: `reduce_duplicate_x_data_runs_from_the_menu_into_origins_columns`.
