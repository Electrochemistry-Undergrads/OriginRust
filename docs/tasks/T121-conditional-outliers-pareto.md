# T121: Conditional Formatting > Conditional Outliers... and Conditional Pareto...

**Status:** claimed 2026-11-17 (Claude Windows, scheduled session "cond-heatmap", branch `cond-outliers`).
**Size:** S. **Source:** greyed inventory package B7 (T35 leftovers, after T120).
**Spec:** `docs/spec/gui-look.md` "Conditional formatting" (new subsections).

## Goal

Two more rule kinds for Worksheet > Conditional Formatting, built on T35/T120's rule list:
Conditional Outliers colours the cells that a box chart would show as outliers; Conditional Pareto
colours the most or least frequent items of the range.

## Origin facts (docs.originlab.com/origin-help/conditional-format, not measured: Origin 2016 has none)

- Outliers: **IQR** with a **Coef** combo (default 1.5): fences Q1 - Coef x IQR and Q3 + Coef x IQR
  (3 gives the outer fences); **SD**: as the box chart's "Whisker Range = SD", factor Coef.
- Pareto: counts all items in the range and ranks them from most to least frequent. Rules: Most
  Frequent N Items (top N by count), Most Frequent N% (items whose cumulative frequency is <= N%),
  Least Frequent N Items, Least Frequent N% (cumulative frequency >= 1 - N%).
- Range and Name as the other rules. Colours, default N and SD default Coef not documented.

## Steps

1. Core: `Rule::Outliers` and `Rule::Pareto` in `crates/core/src/condformat.rs` (new module), quartiles
   by Origin's box-chart rule (`sheaf_calc::stats::quantile_origin`); tests.
2. App: windows in `dialogs/cond_format.rs`, menu items enabled, UI tests.
3. XLSX snapshot colours, spec subsections, inventory, index.

## Done when

Both items open their windows and colour the cells live, survive save/reload, are removed by Remove
Conditional Format; fmt, strict Clippy and the full tests pass.
