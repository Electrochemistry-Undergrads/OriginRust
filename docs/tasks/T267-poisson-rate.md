# T267: Poisson rate tests

**Status:** slices 1 and 2 done 2026-11-20 (Claude Windows "claude:2"), **not measured** (Origin 2026b Pro
feature; OriginLab prints no formulas and the reference install, Origin 2016, lacks the tests).
HANDOFF "Next candidates" C. Origin 2026b's One-Sample and Two-Sample Poisson Rate Test, built from
the help pages' dialog description and the textbook formulas.
**Size:** M (two slices). **Spec:** `docs/spec/statistics.md` 27.

## Done in slice 1

- `crates/calc/src/poisson.rs`: `poisson_one`, `poisson_two`, the exact (chi-square) and normal
  intervals of a rate, the interval of a difference. Checked against SciPy (`poisson`, `chi2`,
  `binom`).
- `Tool::PoissonTest(PoissonTestSpec)` in core (Summarized input), report sheet `PoissonRate<n>`.
- CLI `sheaf analyze poissontest`.

## Done in slice 2

- The two dialogs (`AnalysisKind::PoissonOne`, `PoissonTwo` in `dialogs/analysis.rs`) and their menu
  items. Test: `ui_tests::the_hypothesis_testing_menu_runs_the_variance_and_proportion_tests`.

## Tests

- `poisson::tests` (calc), `statistics::tests::poisson_rate_tests_need_no_data_and_report_both_methods`
  (core), `variance_and_proportion_tests_run_from_the_command_line` (cli).

## Left

- Raw and Indexed input, the plots of Origin's Graphical Summary report, power and sample size.
