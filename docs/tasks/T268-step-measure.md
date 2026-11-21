# T268: Centred step lines measured in Origin

**Status:** done 2026-11-20 (Claude Windows "claude:3"). HANDOFF next candidates B. Origin 2016 with
`set %C -l N` and exported PNGs (`re-workspace/scratch/t268`, local): line connection 13 is Step
Horizontal Center and 14 Step Vertical Center, both drawn as Sheaf's T252 code draws them. The `.opj`
reader and writer now use 14 for Step Vertical Center (it was written as 12 with a note).
**Size:** S. **Spec:** `docs/spec/graphing.md` (line connection table, "Centred steps").

## Tests

- `crates/io/src/opj/write/graph_tests.rs`: the line connection round trip includes 13 and 14.
