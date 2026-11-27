# T415: Power and sample size of the proportion and variance tests

**Status:** done 2026-11-24 (Claude Windows "claude:gmail", statistics 12.1). HANDOFF feature-parity row "Survival,
power, ROC" ("no power tools for proportions or variances"): Statistics > Power and Sample Size gains One-Proportion
Test, Two-Proportion Test, One-Sample Variance Test and Two-Sample Variance Test, and `sheaf analyze power --test
one-proportion|two-proportions|one-variance|two-variances` gives powers for sample sizes or sample sizes for powers,
for the tests of statistics 21. Not measured in Origin.
**Size:** S. **Spec:** statistics 12.1.

## Left

- Measuring Origin's PSS proportion and variance tools (their X-Functions refuse script calls). (Exact binomial
  powers, pooled two-proportion tests and unequal group sizes: T500.)

## Tests

- `crates/calc/tests/power_rate_reference.rs`: 64 cases against scipy (`tools/power-rate-ref.py`, which also simulates
  the variance tests).
- `crates/calc/src/power.rs`: no effect gives power alpha; refusals.
- `crates/cli/tests/analyze.rs`: `power_of_proportion_and_variance_tests_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `power_of_a_proportion_test_from_the_menu`.
