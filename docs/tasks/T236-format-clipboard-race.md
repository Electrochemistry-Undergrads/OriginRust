# T236: Format clipboard file under concurrent writes (bug)

**Status:** done 2026-11-20 (Claude Windows "claude:1"). The test
`format_clip_ui::storage::tests::concurrent_replacements_leave_one_complete_typed_record` failed under load
(9 of 160 runs with 16 copies at once): while one thread renamed a new `format-clipboard.json` over the old
one, Windows refused the reader's open with "access denied", so `load` gave nothing. `load` now tries the
open again, and `save` the rename, for up to one second while Windows says access denied (as `sheaf-io`'s
`write_atomically` does after T230). 480 of 480 stress runs pass.
**Size:** S. **Source:** the T232 full run.

## Tests

- The existing test; the stress run is `re-workspace/scratch/wb-templates/t236_stress.sh` (local only).
