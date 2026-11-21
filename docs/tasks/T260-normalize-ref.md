# T260: Normalize with a reference column or a reference cell

**Status:** done 2026-11-20 (Claude Windows "claude:3"), not measured (Origin unusable on this computer).
Origin's `normalize` methods `ref` (scale each column so that its min, max, mean, median or sum equals
the reference column's; OriginLab prints no formula, Sheaf uses `y * stat(ref) / stat(y)`) and `cell`
(divide by one reference cell): `Tool::NormalizeRef` in core (`calc::NormalizeMethod::MatchStat`,
`calc::ref_stat`), the Normalize window's two new methods, and `sheaf analyze normalize --method
ref|cell --ref COLUMN [--stat ...] [--row N]`.
**Size:** M. **Source:** feature-parity "Normalize" (P0) gap.
**Spec:** `docs/spec/analysis-tools.md` 4.

## Tests

- `crates/calc/src/normalize.rs`: `matching_a_reference_statistic_scales_the_data`.
- `crates/core/src/analysis.rs`: `normalize_matches_a_reference_statistic_or_divides_by_a_reference_cell`.
- `crates/app/src/app/ui_tests.rs`: `normalize_can_match_a_reference_columns_mean`.
- `crates/cli/tests/analyze.rs`: `normalize_matches_a_reference_column_or_divides_by_a_reference_cell`
  and two refusals.

## Left

- Row-wise normalize (`rnormalize`); (measured in T269: `ref` scales, `type` defaults to min, now copied);
  Recalculate links; MCP access; the method list's two new entries sit at the end of a long popup
  (a test picks them through `choose_normalize_reference`).
