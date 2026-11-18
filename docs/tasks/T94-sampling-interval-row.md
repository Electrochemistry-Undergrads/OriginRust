# T94: Sampling Interval label row

**Status:** implemented 2026-11-18 in `codex/t94-sampling-interval-row` after T59's label-row layout landed.
The corner switch and read-only worksheet row follow the recorded Origin 2016 fixture in workbook-model 13i.
Formatting, strict workspace Clippy, Nextest (4,492 passed and
8 skipped), doc tests, inventory and eight Python tooling tests passed.
**Size:** S. **Source:** greyed inventory package B6 (the worksheet corner menu's Sampling Interval), with
T84's sampling interval.
**Spec:** `docs/spec/workbook-model.md` 13d and 13i.

## Goal

The corner menu's Sampling Interval switch shows a label row with each column's sampling interval, as
Origin 2016's.

## Origin facts

The 2016 fixture shows the row below F(x)= after setting B's step to 0.5 s. B shows `0.5 (s)`,
and A and C are empty. The corner switch hides the row. Origin documents the row as read-only.
Workbook-model 13i records Sheaf's saved visibility override, nonzero offset display and remaining
column heading and export work.

## Done when

The switch is enabled and the row shows as measured, saved with the sheet's view switches, fmt/clippy/tests
pass, inventory and spec updated.
