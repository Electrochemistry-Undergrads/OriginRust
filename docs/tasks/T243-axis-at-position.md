# T243: Axis Position "At Position =" (an axis at zero)

**Status:** done 2026-11-20 (Claude Windows "claude:3"). The Axis window's Line and Ticks tab gets
Origin's Axis Position "At Position =" (Origin lists it, graphing 21.3; its drawing was not
measured): the axis line, ticks, labels and title stand where the other axis has the given value,
following that axis' range. `Axis::at_position`; the `.opj` writer names it and the companion keeps
it. Also corrected the parity rows "Opposite and central axes" (no axis at zero) and "Axis line and
ticks" (tick colour and offset exist).
**Size:** S. **Source:** feature-parity "Opposite and central axes" (P0).
**Spec:** `docs/spec/graphing.md` 21.3.

## Tests

- `crates/plot/src/offset_axis_tests.rs`: an X axis at Y = 0 a quarter up a -1..3 range, following a
  new range, clamped to the edge, measured from the top on the opposite side; a Y axis at an X value
  and its hit box.
- `crates/app/src/app/ui_tests.rs`: the Axis Position list, the X = field, back to Right, one undo step.
- `crates/io/src/opj/write/graph_feature_tests.rs`: the `.opj` note.

## Left

- Measure Origin's drawing for values outside the range, and read/write `posType` for At Position in
  `.opj`; axis end arrows.
