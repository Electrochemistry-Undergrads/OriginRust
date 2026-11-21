# T266: Measure the axis "At Position =" and arrows in Origin

**Status:** done 2026-11-20 (Claude Windows "claude:3"). HANDOFF next candidates B (measure the
unmeasured). Origin 2016 through `tools/origin-ref/run-labtalk-lines.ps1` and exported PNGs
(`re-workspace/scratch/t266`, local): "At Position =" is `layer.x.posType = 2` with
`layer.x.position`; T243's drawing matched (axis at the value, labels and title on the normal side,
edge left bare, labels inside at the top edge) except that a value outside the other axis' range is
drawn outside the frame, not clamped: fixed. Axis arrows (T245) have no LabTalk property in Origin
2016, so they stay unmeasured.
**Size:** S. **Spec:** `docs/spec/graphing.md` 21.3.

## Tests

- `crates/plot/src/offset_axis_tests.rs`: `an_axis_at_a_position_stands_where_the_other_axis_has_that_value`
  (above and below the frame).

## Left

- Axis arrows: a screenshot of Origin's Axis window (Line and Ticks) to see whether 2016 has them.
