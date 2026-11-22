# T346: Change point analysis

**Status:** done 2026-11-21 (Claude Windows "claude:gmail", statistics 30). The T342 left item and HANDOFF C:
Origin's 2019b change-point app had no counterpart. Sheaf now finds changes in the mean, the variance or both of a
series (normal costs, MBIC/BIC/AIC/Hannan-Quinn or a typed penalty, a minimum segment length) by PELT, which gives
the exact optimum (checked against an O(n^2) optimal partitioning on 40 runs), as a tool (`Tool::ChangePoint`), from
Statistics > Change Point Analysis... and from `sheaf analyze changepoint` (and the MCP `analyze` tool, which bridges
the command-line tools). Not in Origin 2016, so nothing is measured; the result sheet and graph are Sheaf's own.
**Size:** M. **Spec:** statistics 30.

## Left

- MBIC's short-segment cost term, other distributions (Poisson, exponential, nonparametric), confidence of the
  change locations, binary segmentation or at-most-one-change modes.

## Tests

- `crates/calc/src/changepoint.rs`: mean shifts, a variance change, a calm series, refusals.
- `crates/calc/tests/changepoint_exact.rs`: 40 runs against exact optimal partitioning (`tools/changepoint-ref.py`).
- `crates/core/src/analysis/changepoint.rs`: segment rows and X values with a missing value, refusals.
- `crates/cli/tests/analyze.rs`: `change_point_analysis_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_finds_change_points`.
