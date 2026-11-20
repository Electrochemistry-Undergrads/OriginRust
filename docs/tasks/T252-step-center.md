# T252: Step Horizontal Center and Step Vertical Center

**Status:** done 2026-11-20 (Claude Windows "claude:3"), not measured (Origin unusable on this computer
now). Origin's line connections Step Horizontal Center (code 13) and Step Vertical Center (code not
documented) as `Connect::StepHorizontalCenter` / `StepVerticalCenter`: drawn, filled, offered in
Plot Details' Connect list, read from and written to `.opj` (13; the vertical one is written as 12 with
a note). Also corrected two stale parity rows: "Spline and step connect" (B-spline, Bezier and the
segments exist) and "Exponential, Sigmoidal, Single Peak shortcuts" (the menu entries exist, T15).
**Size:** S. **Source:** feature-parity "Spline and step connect" (P1); graphing 2.x connection table.
**Spec:** `docs/spec/graphing.md` (line connection table).

## Tests

- `crates/plot/src/plots.rs`: `centred_steps_turn_halfway_between_the_points`.
- `crates/plot/src/fill_inclusive_tests.rs`, `fill_sideline_tests.rs`: the new kinds fill cleanly.
- `crates/io/src/opj/write/graph_tests.rs`: code 13 round trip.

## Left

- Measure both centred steps in Origin (ends, the vertical one's file code).
