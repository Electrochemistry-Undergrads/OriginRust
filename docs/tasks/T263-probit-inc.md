# T263: Probit axis increment

**Status:** done 2026-11-20 (Claude Windows "claude:2"), measured in Origin 2016.
HANDOFF "Next candidates" A4: Origin's `layer.y.inc` after a rescale of a probit axis is 3, where
Sheaf's `auto_range` said 1. Exported pictures of the same probit axis (data 0.5..80, limits
0.135..84.13 %) with `inc` 3 (automatic), 1 and 2 are identical: labels 2, 3, 4, 5, 6 at every
whole probit and no minor ticks. So Origin keeps 3 (its probability default) as the increment and
does not use it on a probit axis.
**Size:** S. **Source:** HANDOFF.md A4; `re-workspace/scratch/origin-diff/graph-axes/REPORT.md`
case 1 and `graph-axes/probit-inc/` (the three pictures, local only).
**Spec:** `docs/spec/graphing.md` 4 (scale table, probit row).

## Done

- `scales::auto_range` gives `inc` 3 for probit; `ticks_scales::probit` draws every whole probit
  whatever the increment and no minor ticks.

## Tests

- `ticks_scales::tests::a_probit_axis_ignores_its_increment_as_origin_does`,
  `ticks_scales::tests::probit_ticks_are_whole_probits` (no minor ticks).
