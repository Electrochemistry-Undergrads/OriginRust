# T262: Date and time parts as Origin gives them (no date, empty cell)

**Status:** done 2026-11-20 (Claude Windows "claude:3"), measured in Origin 2016 (HANDOFF next
candidates A2 and A3). `year`, `month`, `day`, `weekday` return -1 outside Origin's date range
1757583..5373483 (0099-12-31 to 9999-12-31) instead of 4713 BC dates; all seven date/time parts
return -1 for a missing input instead of a missing value; `hour`, `minute`, `second` read a time
alone as that time today, give -1 from 1 up to the first date, and read the fraction of a day before
1899-12-30 backwards, all as measured.
**Size:** S. **Source:** HANDOFF.md "Next candidates" A2, A3; `re-workspace/scratch/origin-diff/worksheet-formulas`.
**Spec:** `docs/spec/expression-language.md` (Date and time parts without a date).

## Tests

- `crates/expr/src/tests.rs`: `date_and_time_parts_follow_origins_range_and_give_minus_one_without_a_date`
  (22 measured rows x 7 functions, and the empty cell).

## Left

- Origin's "today" is the local date; Sheaf uses the UTC date (only matters for time-alone values
  near the date-dependent thresholds around midnight).
