# T380: Two-level factorial designs and effects

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 45). HANDOFF C (Origin 2027 Pro Quality
improvement apps, "run two-level split-plot experiments"), base slice: Statistics > Two-Level Experiments > Factorial
Design... / `sheaf analyze factorial-design` make full and fractional two-level designs (standard generators,
resolution, replicates, centre points, seeded run order) in a new sheet, and Factorial Effects... / `sheaf analyze
factorial-effects` estimate main and two-factor interaction effects with t tests or Lenth's margin, listing aliases,
with an effects graph. Not measured in Origin.
**Size:** M. **Spec:** statistics 45.

## Left

- Split-plot designs (hard-to-change factors, whole-plot and subplot error), fold-over, Plackett-Burman, response
  surface designs, half-normal and interaction plots, three-factor interactions.

## Tests

- `crates/calc/tests/doe_reference.rs`: replicated 2^3, unreplicated 2^4, 2^(5-1) and 2^(4-1) with aliases against
  numpy and scipy (`tools/doe-ref.py`).
- `crates/calc/src/doe.rs`: generators, resolution, replicates and centre points, an exact model, aliasing.
- `crates/core/src/analysis/doe.rs`: the design sheet and the effects with the plotted columns.
- `crates/cli/tests/analyze.rs`: `factorial_design_and_effects_run_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_designs_and_analyzes_a_two_level_experiment`.
