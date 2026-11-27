# T488: Time Series Explorer Rescale Y

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing "Time Series Explorer"). Left item of T416/T453 (claude:work's,
unclaimed); Origin 2024b's Time Series Explorer, Sheaf's own design.
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Time Series Explorer": a detail Y axis per interval remains.

## What it does

A Rescale Y tick on the explorer bar fits the detail layer's Y axis to the data of the interval shown (with the axis's rescale margin);
off, the detail keeps the whole Y range. One undo step; saved with the project.

## Tests

- `crates/plot/src/explorer_rescale_tests.rs`: `rescale_y_fits_the_detail_axis_to_the_interval`.
- `crates/core/src/explorer.rs`: saved with the project.
- `crates/app/src/app/ui_tests/function_graph.rs`: `time_series_explorer_steps_and_plays_from_the_bar` (the tick, one undo step).
