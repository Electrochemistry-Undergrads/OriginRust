# T425: Confidence intervals of Pp and Ppk

**Status:** done 2026-11-24 (Claude Windows "claude:gmail", statistics 36). T365's "Left": Statistics > Process
Capability... and `sheaf analyze capability --confidence 95` give the confidence intervals of Pp (chi-square) and Ppk
(Bissell's approximation) for the normal and Box-Cox models. Not measured in Origin.
**Size:** S. **Spec:** statistics 36.

## Left

- Intervals of the within indices (Cp, Cpk: degrees of freedom of R bar / d2) and of Cpm; one-sided bounds; Johnson
  transformations.

## Tests

- `crates/calc/tests/capability_ci_reference.rs`: five cases against scipy (`tools/capability-ci-ref.py`, which also
  simulates the Pp interval's coverage).
- `crates/calc/src/capability.rs`: `the_intervals_widen_with_the_level_and_hold_the_estimates`.
- `crates/core/src/analysis/capability.rs`, `crates/cli/tests/analyze.rs` (`--confidence`),
  `crates/app/src/app/ui_tests.rs` (`the_statistics_menu_computes_box_cox_capability`).
