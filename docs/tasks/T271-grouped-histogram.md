# T271: Grouped histogram

**Status:** done 2026-11-21 (Claude Windows "claude:2"), **not measured** (Origin 2026's plot; the
reference install is Origin 2016).
HANDOFF "Next candidates" C, `docs/tasks/origin-2026-2027-gaps.md` (Graph Maker additions).
**Size:** S. **Spec:** `docs/spec/graphing.md` 14.31.

## Done

- `Project::plot_grouped_histogram` (`crates/core/src/grouped_hist.rs`): the Y column split by the
  group column's levels into a new workbook, overlaid in one layer with shared bins or stacked.
- Plot > Statistics > Grouped Histogram... (`dialogs/grouped_hist.rs`, `Cmd::GroupedHistogram`).

## Tests

- `grouped_hist::tests` (core), `ui_tests::plot_statistics_grouped_histogram_makes_one_histogram_per_group`.

## Left

- Several group columns, Separate Graphs, panels and pages, distribution curves, Data Height, Y error.
