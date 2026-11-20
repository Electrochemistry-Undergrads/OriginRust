# T253: Two-way, three-way and repeated-measures ANOVA leave masked cells out

**Status:** done 2026-11-20 (Claude Windows "claude:3"). Bug fix. The two-way, three-way and
repeated-measures ANOVA read the raw numbers of the data columns, so masked cells and rows hidden by
the worksheet filter still counted. They now read them as every other analysis tool does
(`Worksheet::values_of`: masked and filtered cells are missing values). The parity row "Masking" named
the two-way case.
**Size:** S. **Source:** feature-parity "Masking" (P1) gap.

## Tests

- `crates/core/src/analysis/statistics.rs`: `the_two_way_anova_leaves_out_masked_cells_and_filtered_rows`
  (a masked cell and a filtered row give the table of the same row deleted).
- `crates/core/src/analysis/anova3.rs`, `rmanova.rs`: `masked_data_cells_are_left_out`.

## Left

- Masked or filtered factor (level) cells: only the data column's mask counts; a masked level cell with
  data still counts (as the categorical tools, which drop it, do not yet share one rule here).
