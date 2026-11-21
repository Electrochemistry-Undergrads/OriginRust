# T291: Axis title rotation and offset

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Axis titles" (P0) gap "no
rotation or offset". Measured in Origin 2016 (`re-workspace/scratch/t291`, local): `yl.rotate` 90 and
`xb.rotate` 0 by default; a rotation turns the title about its middle; `xb.x`/`xb.y`/`yl.y` move it
in axis units and it stays there. The Axis window's Title tab gets Rotate (deg.) (Default or a value,
replacing the greyed box) and Offset (pt) right and down; Sheaf turns the title about its middle and
then moves it. The `.opj` reader keeps a non-default angle; the writer writes angle and offset.
**Size:** S. **Spec:** `docs/spec/graphing.md` 3.4 (Rotation and position).

## Tests

- `crates/plot/src/offset_axis_tests.rs`: `a_turned_title_keeps_its_middle_and_moves_by_its_offset`.
- `crates/io/src/opj/write/graph_tests.rs`: `a_turned_title_keeps_its_angle_in_the_opj`.

## Left

- Dragging a title with the mouse in the graph window (it would set the offset); reading a moved
  title's position from `.opj` (Origin keeps it in axis units).
