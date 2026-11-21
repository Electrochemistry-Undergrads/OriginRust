# T272: Custom colours file under concurrent writes (bug, flaky test)

**Status:** done 2026-11-21 (Claude Windows "claude:2"). The test
`custom_colors::tests::concurrent_replacements_always_expose_one_complete_collection` failed once in a
full nextest run (T271) and passed alone. `custom_colors.rs` has the same pattern T236 fixed for the
format clipboard: while one thread renames a new file over the old one, Windows can refuse a reader's
open (or another writer's rename) with "access denied", and `load` then gave an empty list. `load` now
retries the open, and `save` the rename, for up to one second while Windows says access denied.
After the fix, 160 of 160 runs pass with 16 copies at once (the old code was not stress-run for a
before figure).
**Size:** S. **Source:** the T271 full run.

## Tests

- The existing test; stressed with 16 concurrent copies of the test binary, 10 rounds.
