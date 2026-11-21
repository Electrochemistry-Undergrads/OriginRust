# T300: Normalize Columns measured against Origin's rnormalize

**Status:** done 2026-11-21 (Claude Windows "claude:3"). HANDOFF next candidates B and feature-parity
"Normalize" (P0, gap "no row normalize"). Origin 2016's `rnormalize` (Normalize Columns) normalizes
each column, not rows, so the gap was a misreading. Measured (`re-workspace/scratch/t299`, local): the
results go to the end of the sheet as Normalized1, Normalized2, ... (numbering goes on across runs),
keep the source's designation, and carry the comment "<method> of <column>" with Origin's method
words. Sheaf's Normalize Columns now writes the same; the values already matched.
**Size:** S. **Spec:** `docs/spec/analysis-tools.md` 4 (Where the results go).

## Tests

- `crates/core/src/analysis.rs`: `normalize_columns_writes_what_origins_rnormalize_writes`.

## Left

- The graph's Normalize: Origin's `normalize` names its column "Normalized Y1"; Sheaf uses the
  Normalize Columns names for both.
