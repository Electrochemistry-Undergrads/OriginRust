# T367: hour, minute, second measured; T262's date bounds re-checked

**Status:** done 2026-11-22 (Claude Windows "claude:1"). Origin 2016 runs again, so the T239 functions
were measured on a worksheet column; Sheaf already matched (new test
`hour_minute_and_second_match_origins_worksheet_column`). A separate probe of the bounds (from -1e6 to
3e7, bisected to the unit) found what T262 had already built: date window 1757583 to 5373483, time of a
number below 1 read as that time today (so -703784 gives -1 on 2026-11-22), times before 1899-12-30 read
backwards, -1 for a missing cell. No code change beyond the test; HANDOFF A.2 and A.3 marked done.
**Size:** S. **Spec:** `docs/spec/expression-language.md` section 7.
