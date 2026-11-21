# T270: Run chart

**Status:** done 2026-11-21 (Claude Windows "claude:2"), **not measured** (Origin 2025b's chart; the
reference install is Origin 2016).
HANDOFF "Next candidates" C, `docs/tasks/origin-2026-2027-gaps.md` (Graph Maker additions).
**Size:** S. **Spec:** `docs/spec/graphing.md` 14.30.

## Done

- `crates/core/src/runchart.rs`: `run_chart_data` and `Project::plot_run_chart` (workbook and graph,
  dashed reference line). Plot > Statistics > Run Chart... (`dialogs/run_chart.rs`, `Cmd::RunChart`).

## Tests

- `runchart::tests` (core), `ui_tests::plot_statistics_run_chart_makes_its_workbook_and_graph`.

## Left

- Subgroups by a reference column and "Each Subgroup in Row"; the statistics table on the graph;
  output embedded in a report sheet; Origin's Graph Maker.
