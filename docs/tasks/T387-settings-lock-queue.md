# T387: Settings lock waits in the system's queue (bug, follows T378)

**Status:** done 2026-11-23 (Claude Windows "claude:1"). The daily push's test of main at 13369191 (1772 s on a
loaded machine) failed `themes::tests::two_writers_do_not_lose_each_others_themes`: a writer did not get the
settings lock within the 60 s of T378, so its save was refused. The lock was taken by polling `try_lock` every
10 ms, which let one writer lose to the others again and again. A busy lock is now waited for with the system's
blocking lock (writers are served in turn) on a helper thread, with the same 60 s limit. 96 of 96 runs pass
with 32 copies of the writer tests at once (slowest 23 s).
**Size:** S. **Spec:** `docs/spec/recent-files.md`.
