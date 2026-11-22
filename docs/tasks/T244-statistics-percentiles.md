# T244: Percentiles in Statistics on Columns (parity "Moments, quantiles")

**Status:** done 2026-11-22 (Claude Windows "claude:1"), **not measured**: Origin was not installed. The
Percentiles (%) box of Statistics on Columns, `sheaf analyze stats --percentiles` and the MCP `statistics`
option `percentiles` add one column per percentile (`P5`, `P97.5`, ...), by Origin's quartile rule applied
to any p (`StatisticsSpec::percentiles`).
**Size:** S. **Spec:** `docs/spec/analysis-tools.md`, Statistics on Columns.

## Tests

- `crates/core/src/analysis.rs`: P25/P50 equal Q1/Median, P5/P97.5 by the rule, out-of-range refused.
- `crates/app/src/dialogs/analysis.rs`: the box's parser. `crates/cli/tests/analyze.rs`: `--percentiles`.

## Left

- Compare Origin's Custom Percentile output (rule and column names) when Origin is installed again.
