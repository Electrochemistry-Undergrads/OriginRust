# T364: Rare event charts (G and T)

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 35). HANDOFF C (Origin 2027 Pro Quality
improvement apps, "rare events with G/T control charts"): Statistics > Rare Event Chart (G/T)... and `sheaf analyze
rarechart` make a G chart (geometric fit, probability or sigma limits) or a T chart (exponential or Weibull fit,
probability limits) per column, with a result row, the points beyond the limits and a chart. Not in Origin 2016, so
the layout is Sheaf's own.
**Size:** S. **Spec:** statistics 35.

## Left

- Run and trend tests on the charts; the other 2027 Quality additions (nonnormal capability, two-level split-plot
  designs, between-batch stability studies).

## Tests

- `crates/calc/tests/rarechart_reference.rs`: two G and two T series against scipy (`tools/rarechart-ref.py`).
- `crates/calc/src/rarechart.rs`: sigma and probability limits, flags, the exponential T chart, refusals.
- `crates/core/src/analysis/rarechart.rs`: a missing value, a flagged point, the T chart, a 0 time refused.
- `crates/cli/tests/analyze.rs`: `rare_event_charts_run_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_makes_a_rare_event_chart`.
