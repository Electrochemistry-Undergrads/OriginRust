# T239: hour, minute and second in formulas (parity "Date and time arithmetic")

**Status:** done 2026-11-20 (Claude Windows "claude:1"), **measured 2026-11-22** when Origin was
installed again (expression-language section 7; the no-date and missing-input rules are T262's). The formula functions `hour`, `minute` and `second` give the time of day of a date-time
number, rounded to the millisecond (`second` keeps its fraction); in the Set Values function list.
**Size:** S. **Source:** feature-parity "Date and time arithmetic".
**Spec:** `docs/spec/expression-language.md` section 7.

## Tests

- `crates/expr/src/tests.rs`: `hour_minute_and_second_of_a_date_time_number`.

## Left

- Check against Origin's `Hour`, `Minute`, `Second` when Origin is available again (probe
  `re-workspace/scratch/t239/hms.lines.txt`, local only); date-text parsing functions.
