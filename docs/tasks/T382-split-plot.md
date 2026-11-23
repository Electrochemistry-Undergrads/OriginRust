# T382: Two-level split-plot designs

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 46). HANDOFF C (Origin 2027 Pro Quality
improvement apps, "run two-level split-plot experiments"), on T380: Statistics > Two-Level Experiments > Split-Plot
Design... / `sheaf analyze split-plot-design` make designs whose hard-to-change factors are set once per whole plot
(replicated whole plots, two-stage randomization), and Split-Plot Effects... / `sheaf analyze split-plot-effects` test
whole-plot terms against the whole-plot error and the others against the subplot error, with Lenth's margin per
stratum when it has no degrees of freedom. Not measured in Origin.
**Size:** S. **Spec:** statistics 46.

## Left

- REML for unbalanced plots, fractional and split-split-plot designs, variance components, residual plots.

## Tests

- `crates/calc/tests/splitplot_reference.rs`: two designs with whole-plot random effects against numpy and scipy
  (`tools/splitplot-ref.py`).
- `crates/calc/src/doe.rs`: whole-plot factors constant within plots, an exact model with plot effects, refusals.
- `crates/core/src/analysis/doe.rs`: the design sheet and the strata.
- `crates/cli/tests/analyze.rs`: `split_plot_design_and_effects_run_from_the_command_line` (its column names avoid the
  short names A, B, C, which the CLI matches first).
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_analyzes_and_designs_a_split_plot_experiment`.
