# T424: The recent-list writers test fits the lock wait under load

**Status:** done 2026-11-24 (Claude Windows "claude:gmail"). The T419 gate failed once on
`sheaf-io recent::tests::many_writers_at_once_lose_no_entry`: eight threads recording a project at once queued for the
recent-list lock longer than its 60 s wait (83 s) while the full workspace run loaded the machine; alone it takes under
a second. As T411 did for the themes writers, the test now races four writers, which still take the lock in turn.
**Size:** S.

## Tests

- `crates/io/src/recent/tests.rs`: `many_writers_at_once_lose_no_entry` (four writers).
