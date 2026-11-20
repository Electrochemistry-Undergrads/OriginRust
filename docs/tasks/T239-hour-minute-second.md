# T239: hour, minute and second in formulas (parity "Date and time arithmetic")

**Status:** done 2026-11-20 (Claude Windows "claude:1"), **not measured**: Origin 2016 is no longer
installed on the measuring computer (only its crash reporter is left and `Origin.Application` is not
registered). The formula functions `hour`, `minute` and `second` give the time of day of a date-time
number, rounded to the millisecond (`second` keeps its fraction); in the Set Values function list.
**Size:** S. **Source:** feature-parity "Date and time arithmetic".
**Spec:** `docs/spec/expression-language.md` section 7.

## Tests

- `crates/expr/src/tests.rs`: `hour_minute_and_second_of_a_date_time_number`.

## Left

- Check against Origin's `Hour`, `Minute`, `Second` when Origin is available again (probe
  `re-workspace/scratch/t239/hms.lines.txt`, local only); date-text parsing functions.
