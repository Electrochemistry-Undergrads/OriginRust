# T419: Process capability with a Box-Cox transformation and Cpm

**Status:** done 2026-11-24 (Claude Windows "claude:gmail", statistics 36). T365's "Left" (Origin 2027 Quality apps,
nonnormal capability): Statistics > Process Capability... and `sheaf analyze capability` gain a Box-Cox model (lambda
by maximum likelihood or given; values and limits transformed, then the normal within and overall indices) and the
Cpm index for a target (normal and Box-Cox models). Not measured in Origin.
**Size:** S. **Spec:** statistics 36.

## Left

- Johnson transformations, confidence intervals of the indices, a rounded ("convenient") lambda, a pooled-deviation
  within estimate, Cpm with one limit.

## Tests

- `crates/calc/tests/capability_boxcox_reference.rs`: seven cases against scipy (`tools/capability-boxcox-ref.py`).
- `crates/calc/src/capability.rs`: lambda 1 equals the normal indices, Cpm by hand, a lower limit of 0 with the log,
  the lambda's likelihood is a maximum, refusals.
- `crates/core/src/analysis/capability.rs`: `normal_and_weibull_capability_with_a_histogram` (Box-Cox part).
- `crates/cli/tests/analyze.rs`: `process_capability_runs_from_the_command_line` (Box-Cox part).
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_computes_box_cox_capability`.
