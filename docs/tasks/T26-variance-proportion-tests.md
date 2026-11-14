# T26: Tests for variance and proportions

**Status:** started 2026-11-14 by a session that could not build, run Origin or commit (no permission for
cargo, PowerShell scripts or git writes). Done so far, all untracked in the main checkout: the calc module
`crates/calc/src/vartest.rs` (not yet in `lib.rs`, never compiled), the Origin probe
`re-workspace/scratch/vartest/probe1.lines.txt`. Nothing measured yet. **Size:** M. **Branch:** `vartest`.
**Model:** Sonnet (Haiku to run the suites).

**Goal:** Origin's Statistics > Hypothesis Testing has eight entries; Sheaf has the three t-tests. Add
One-Sample Test for Variance, Two-Sample Test for Variance, One-Sample Test for Proportion and Two-Sample
Test for Proportions, measured against Origin. Closes the feature-parity row "Variance and proportion
tests" (`vartest`, P2). (Two-Sample t-Test on Rows is the eighth entry; separate.)

**Origin facts:** none measured. X-function names `vartest1` and `vartest2` are from memory; the proportion
tests' names are unknown (probe1 tries ten guesses with `-h`, an unknown name returns `[False]`).
Find out with probe1, or open the dialogs through the menu (`tools/origin-ref/origin-dialog.ps1
-MenuClicks`, as for the Peak Analyzer in T04) and screenshot each page.

**Where:**
- calc: `crates/calc/src/vartest.rs`: `vartest_one`, `vartest_two` (return `VarTest`),
  `proportion_one`, `proportion_two` (return `ProportionTest` with both the normal and the exact p-value,
  Wald and Clopper-Pearson intervals, so the core can show whichever Origin shows). Add `pub mod vartest;`
  and a `pub use` line in `crates/calc/src/lib.rs`. Its unit tests use closed forms only (chi-square with
  2 df, F(2, 2), binomial ends); compile and run them first, fix what does not build.
- core: `crates/core/src/analysis.rs` next to `TTestSpec`/`Tool::TTest` (name, sheet name "VarTest",
  "PropTest", and the list near `Tool::TTest(_) | ... Tool::SignedRank(_)`); the runner in
  `crates/core/src/analysis/statistics.rs` beside `fn ttest` (copy its result-sheet layout: the
  statistics table, then the test table, then the interval). The proportion tests take counts typed in the
  dialog (Origin's dialog asks for successes and sample size, to be confirmed), not columns, unless Origin
  also accepts a column of 0/1 or of categories: measure.
- app: `crates/app/src/menus.rs` "&Hypothesis Testing" submenu (four `item(...)` lines after the t-tests,
  Origin's order and labels from the menu); `crates/app/src/dialogs/analysis.rs` `AnalysisKind` (new
  variants, title, id string, description, tool building, options UI beside `AnalysisKind::TTest`).
- CLI: `crates/cli/src/args.rs` and `crates/cli/src/analyze.rs` beside `ttest` (`sheaf analyze vartest`,
  `sheaf analyze proptest`). Note that T25 is moving the CLI's command work into a shared `sheaf-ops`
  crate (branch `mcp-server`); start after it is merged, or put the work where it went.
- spec: a new section of `docs/spec/statistics.md` with every measurement; `docs/spec/feature-parity.md`
  row "Variance and proportion tests".

**Steps:**
1. Run probe1 (`tools/origin-ref/run-labtalk-lines.ps1 -Script re-workspace/scratch/vartest/probe1.lines.txt`):
   which names exist, what outputs `vartest1`/`vartest2` take (`df1`/`df2` or `df`, `lcl`/`ucl`).
2. Measure, on the `stats-ttest.lines.txt` columns A (n 8) and B (n 10) and a few more data sets: all tails,
   alpha 0.05 and 0.1, a hypothesized variance or ratio other than 1, n = 2, a flat sample, missing
   values. For proportions: 0 and all successes, small and large n, every method and interval Origin
   offers. Copy Origin: the two-sided p rule of the chi-square and F test, the one-sided interval (the
   module assumes `ttest1`'s `1 - 2 alpha` rule), which exact two-sided p (sum of less likely counts or
   twice the smaller tail), which interval (Wald, Wilson, Clopper-Pearson), pooled or not.
3. Change `vartest.rs` to what Origin does; turn the numbers into `crates/calc/tests/origin_vartest.rs`.
4. Wire core, app, CLI; UI test opening each dialog and running it; a CLI test.
5. Checks, merge, parity count, this Status.

**Done when:** the four menu items run, the numbers agree with Origin's to 1e-6 relative on the measured
cases, checks pass.

**Don't:** copy Origin's help text into the dialog; guess an Origin rule without writing "unmeasured" in
the spec; leave the module's header comment saying "not yet measured" after measuring.
