# T254: Masked factor and subject cells in the ANOVAs

**Status:** done 2026-11-20 (Claude Windows "claude:3"). Bug fix, T253 follow-up. In the two-way
(indexed layout), three-way and repeated-measures ANOVA a masked factor (level) or subject cell, or a
row the worksheet filter hides, now leaves its row out (`statistics::shown_level`), as the
categorical tools already did; such a row with data is counted among the rows left out, and a
filtered row is not reported.
**Size:** S. **Source:** T253 "Left".

## Tests

- `crates/core/src/analysis/statistics.rs`: `the_two_way_anova_leaves_out_masked_cells_and_filtered_rows`
  (a masked level cell).
- `crates/core/src/analysis/anova3.rs`, `rmanova.rs`: `masked_data_cells_are_left_out` (a masked
  factor or subject cell).
