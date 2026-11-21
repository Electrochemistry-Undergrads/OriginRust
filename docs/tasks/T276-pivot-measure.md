# T276: Pivot Table measured against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
HANDOFF "Next candidates" B: T124's Pivot Table was built from `wpivot`'s documentation. Origin 2016 run
through `tools/origin-ref/run-labtalk-lines.ps1` on an 8-row table (count, sum with totals, mean
without zeros, percent with totals, label/total sorts, pos 0 and pos 2, mean and max with totals).
Matched already: sheet names, cell values, missing values, default and label/total sorts of Count and
Sum. Changed to match: the value columns' Long Name (`Sum of C` etc.), the label positions of pos 0
and pos 2, the totals' names (`Total_A` column, `Total_B` row), totals that apply the method to the
records (Mean, Min, Max tables) and sort by them, and the normalized row-total column.
**Size:** S. **Spec:** `docs/spec/workbook-model.md` 13n. Probe scripts and outputs:
`re-workspace/scratch/pivot-measure/` (local only).

## Tests

- `pivot::tests::origin_2016_wpivot_cases` (the measured numbers and names);
  `ui_tests::cond_format::pivot_table_counts_into_a_new_sheet_as_one_undo_step` updated.

## Left

- Others merging, extra values and the user-parameter position (still not built).
