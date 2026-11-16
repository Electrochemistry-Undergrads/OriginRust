# T111: QC (X bar R) Chart

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `qc-chart`).
**Size:** M. **Source:** greyed inventory package B4 (Plot > Statistics > QC (X bar R) Chart).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

Origin 2016's X bar R chart: the means and ranges of subgroups of a column, each in its own layer,
with the centre line and the upper and lower control limits.

## Steps

1. Measure it in Origin 2016 with tools/origin-ref: the columns it takes, how it chooses the
   subgroup size, the layers, the lines and their labels, colours, axes and titles, and the
   control-limit constants.
2. Build it: the subgroup statistics and limits in `sheaf-calc` or `sheaf-core`, a template with
   the two layers, and the menu item enabled.
3. Tests, a graphing.md section, greyed inventory recount.

## Done when

The menu item plots as measured, the inventory no longer lists it, and fmt, strict Clippy and the
full tests pass.
