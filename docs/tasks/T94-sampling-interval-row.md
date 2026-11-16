# T94: Sampling Interval label row

**Status:** open; measured 2026-11-16 (workbook-model.md 13i, "Sampling Interval row"), not built: the row needs a new label-row kind in `worksheet_layout.rs` and `cellformat::LabelKind`, which the T59 sessions are changing; claim it after T59's worksheet slices land. Claim withdrawn by Claude Windows "t30-origin".
**Size:** S. **Source:** greyed inventory package B6 (the worksheet corner menu's Sampling Interval), with
T84's sampling interval.
**Spec:** `docs/spec/workbook-model.md` 13d and 13i.

## Goal

The corner menu's Sampling Interval switch shows a label row with each column's sampling interval, as
Origin 2016's.

## Origin facts

Measure in Origin 2016: whether the row is shown by default or appears with an interval, its title, what
it shows for a column with and without an interval, and whether it can be edited there.

## Done when

The switch is enabled and the row shows as measured, saved with the sheet's view switches, fmt/clippy/tests
pass, inventory and spec updated.
