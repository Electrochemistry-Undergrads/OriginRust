# T275: Stem-and-leaf plot

**Status:** done 2026-11-21 (Claude Windows "claude:3"). HANDOFF next candidates C (Origin 2026's
"Stem and Leaf Plot" app). Statistics > Descriptive Statistics > Stem-and-Leaf Plot... writes the
lines of the display (Data, Count, Stem, Leaf) to a new sheet, with the app page's options: Leaf Unit
(auto or given), Auto Split Stem, Max Number of Leaves per Stem (50), Auto Trim Outliers (1.5 IQR,
Low and High lines). Not measured (the app is not in Origin 2016): the result layout and the
automatic leaf unit are Sheaf's own.
**Size:** M. **Spec:** `docs/spec/statistics.md` 28.

## Tests

- `crates/calc/src/stemleaf.rs`: `a_classic_display_with_empty_stems`,
  `negatives_get_minus_zero_and_stems_split_at_five`,
  `outliers_go_to_their_own_lines_and_long_lines_are_capped`.
- `crates/core/src/analysis/statistics.rs`: `stem_and_leaf_writes_count_stem_and_leaf_lines_per_column`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_makes_a_stem_and_leaf_plot`.

## Left

- A graph of the display (Origin's app draws the display as a text graph); the CLI and MCP tools.
