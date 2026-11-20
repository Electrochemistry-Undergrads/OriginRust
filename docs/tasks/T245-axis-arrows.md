# T245: Axis arrows (Arrow at Begin / Arrow at End)

**Status:** done 2026-11-20 (Claude Windows "claude:3"), **not measured** (Origin 2016 unusable on
this computer now). The Axis window's Line and Ticks tab gets Arrow at Begin and Arrow at End: a
filled arrowhead past the chosen ends of the axis line, in its colour, sized by the line width
(Sheaf's sizes). `Axis::arrow_begin`, `arrow_end`; the `.opj` writer names them and the companion
keeps them.
**Size:** S. **Source:** feature-parity "Axis line and ticks" (P0) gap "no arrows".
**Spec:** `docs/spec/graphing.md` 21.3 (after At Position =).

## Tests

- `crates/plot/src/offset_axis_tests.rs`: tips past the right, top and bottom ends, none at a
  begin not chosen, moving with an offset axis, the size rule.
- `crates/io/src/opj/write/graph_feature_tests.rs`: the `.opj` note.

## Left

- Measure Origin's arrow shapes, sizes and options; write them to `.opj`.
