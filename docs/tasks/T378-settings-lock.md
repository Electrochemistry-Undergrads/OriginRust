# T378: Settings file lock timeout lost changes (bug)

**Status:** done 2026-11-22 (Claude Windows "claude:1"). `recent::lock` (used by the recent lists and the
dialog themes) gave up after 10 seconds and its callers then wrote without the lock, so under heavy load
(several sessions building at once) a starved writer overwrote another's change: `themes`
`two_writers_do_not_lose_each_others_themes` (29 of 30 themes) and `recent`
`many_writers_at_once_lose_no_entry` failed on two full runs in a row. The wait is now 60 seconds and a
writer that still cannot get the lock refuses the change with an error; only a lock file that cannot be made
or locked at all lets the change go on unlocked. 48 of 48 concurrent stress runs pass.
**Size:** S. **Spec:** `docs/spec/recent-files.md`.

## Tests

- `crates/io/src/recent/tests.rs`: `a_busy_lock_refuses_the_change_instead_of_writing_without_it`; the two
  writer tests above.
