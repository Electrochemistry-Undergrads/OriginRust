# T362: Manhattan plot

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 33). HANDOFF C (Origin 2017 Pro app Manhattan
Plot): Statistics > Manhattan Plot... and `sheaf analyze manhattan` lay each group's (chromosome's) positions end to
end, plot `-log10 p` per point as one scatter per group with a significance line, and write a per-group table. Not in
Origin 2016, so the layout is Sheaf's own.
**Size:** S. **Spec:** statistics 33.

## Left

- Group names at the group centres on the X axis (alternating colours, labels of the hits and a gap between groups:
  T505).

## Tests

- `crates/core/src/analysis/manhattan.rs`: sorting numeric groups, offsets, p outside (0, 1] left out, refusals.
- `crates/cli/tests/analyze.rs`: `manhattan_plot_runs_from_the_command_line` (three and two columns).
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_makes_a_manhattan_plot`.
