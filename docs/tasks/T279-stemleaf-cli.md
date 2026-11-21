# T279: Stem-and-leaf display from the command line and MCP

**Status:** done 2026-11-21 (Claude Windows "claude:3"). T275's left item. `sheaf analyze stemleaf FILE
--y B [--leaf-unit U] [--split] [--max-leaves N] [--trim-outliers]` writes T275's Data/Count/Stem/Leaf
sheet and prints the leaf unit; the MCP `analyze` tool reaches it as `stemleaf` with the same options.
**Size:** S. **Spec:** `docs/spec/statistics.md` 28.

## Tests

- `crates/cli/tests/analyze.rs`: `ttest_anova_normality_and_histogram_run_from_the_command_line` (stemleaf
  calls).
- `crates/mcp/tests/analyze_bridge.rs`: the `stemleaf` parity call.
