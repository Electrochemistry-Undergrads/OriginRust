# T240: Timing flakes in the MCP link tests (tests only)

**Status:** done 2026-11-20 (Claude Windows "claude:1"). `crates/mcp/tests/link.rs` assumed that 100 ms
was enough for a call to reach Sheaf's queue and that a job is in the executor's log once its reply
arrived. Under load neither held: `the_owner_replacing_the_project_tells_the_assistant_once` failed 28 of
384 times with 48 copies at once (the replace closed a connection still saying hello, or came before the
call), and `a_cancelled_call_that_waited_behind_another_never_runs` ran a cancelled call. The test
executor can now hold jobs without running them (`Executor::hold`, `held_queued`), so those tests wait
until the call is queued; `ran_at_least(n)` waits for the log entry that follows a reply. Stress after
the change: 0 of 576 and 0 of 384. No change to the link itself.
**Size:** S. **Source:** the T236 full run.

## Left

- `a_full_tcp_host_delivers_busy_repeatedly_and_a_silent_rejected_peer_cannot_hold_accept` can fail
  with "connection aborted" when 16 copies of the whole link test binary run at once (1 of 64);
  not seen in ordinary runs.
- Stress scripts: `re-workspace/scratch/wb-templates/t240_*.sh` (local only).
