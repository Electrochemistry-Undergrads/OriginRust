# T26: Tests for variance and proportions

**Status:** merged 2026-11-15: calc module with fixtures (`crates/calc/tests/origin_vartest.rs`), core sheets in Origin's report layout, four dialogs, menu, CLI (`analyze vartest | proptest`, `--pooled`), spec section 21 (statistics.md), parity row (partial). Leftovers done on branch `vartest2` (2026-11-15): the two-sample proportion test measured in 20 dialog runs (unpooled and pooled z, all tails, negative and nonzero test difference, small samples, all successes, Fisher's exact test as its own report section, the warning rule), the two-sample variance dialog with intervals and one-sided tails, the significant conclusion sentences of the variance tests, and the Power Analysis group of both proportion dialogs (actual and hypothetical power, implemented in calc, core, dialogs and CLI). Left: Raw and Indexed input forms of the proportion tests, power with a pooled test and unequal sizes beyond the measured runs, flat and missing-value cases in the variance dialogs, the plots of the variance dialogs. **Size:** M. **Branch:** `vartest`.
**Model:** Sonnet (Haiku to run the suites).

**Goal:** Origin's Statistics > Hypothesis Testing has eight entries; Sheaf has the three t-tests. Add
One-Sample Test for Variance, Two-Sample Test for Variance, One-Sample Test for Proportion and Two-Sample
Test for Proportions, measured against Origin. Closes the feature-parity row "Variance and proportion
tests" (`vartest`, P2). (Two-Sample t-Test on Rows is the eighth entry; separate.)

**Origin facts (OriginPro 2016, probes 1-3 and a dialog capture by session 318653a1, read and written down
2026-11-14 evening; raw files in `re-workspace/scratch/vartest/`):**
- Menu Statistics > Hypothesis Testing, in order: One-Sample t-Test..., Pair-Sample t-Test..., Two-Sample
  t-Test..., One-Sample Test for Variance..., Two-Sample Test for Variance..., separator, Two-Sample t-test on
  Rows..., Pair-Sample t-test on Rows..., separator, One-Sample Proportion Test..., Two-Sample Proportion
  Test... (`shots/m1.png`). Screen clicks with the Origin window at 0,0: Statistics `372,43`, Hypothesis
  Testing `420,87`, then the item at x `645` and y `153` (one-sample variance), `175` (two-sample variance),
  `257` (one-sample proportion), `279` (two-sample proportion).
- `vartest1` and `vartest2` exist as X-functions; none of the twelve proportion names tried (`proportion1`,
  `prop1`, `ztest1prop`, `oneprop`, `proptest1`, `ptest1` and their `2`s) does. The dialogs are titled
  `One-Sample Test for Variance: OneSampleTestVar` etc., but `OneSampleTestVar`, `TwoSampleTestVar`,
  `OneSampleProportionTest`, `TwoSampleProportionTest` all fail from LabTalk (`-h` and with arguments,
  probes 2-3), so the proportion tests can only be measured through the dialogs.
- `vartest1 irng:=A var:=2` on probe column A (n 8, `10 + 2 sin(3i) + i/4`): stat 6.3788381990149663, df 7,
  prob 0.99254933044912774, lcl 0.79671917636177081, ucl 7.54950463751445. Statistic and interval are the
  textbook ones in this module (interval of the variance at 95 %), and the two-sided p is
  `2 min(cdf, sf)` (the chi-square median at 7 df is 6.35, so both tails are near 0.5). One case only:
  tails, alpha and the one-sided interval are still unmeasured.
- `vartest2 irng:=(A,B)` (B n 10, `11.5 + 2.5 cos(2i) + i/5`): stat 0.45323022914345851 (= s_A^2 / s_B^2),
  prob 0.30934827784074181 (two-sided). `vartest2` accepts neither `df`, `df1`/`df2` nor `lcl`/`ucl`
  (probe1 lines 28-35 failed; the values printed after them are left over from `vartest1`).
- One-Sample Test for Variance dialog (`d1/first.png`, `first.controls.txt`): Results Log Output, Recalculate
  (None, Auto, Manual), Input; group "Chi-Square Test for Variance": Test Variance (default 1), Null
  Hypothesis (read-only "Variance = 1"), Alternate Hypothesis radios `Variance <> 1` / `> 1` / `< 1`,
  Significance Level 0.05, Confidence Interval(s) tick (off), Confidence Level(s) in % "90 95 99"; Plots
  (Histogram, Box Chart, Arrange Plots of Same Type in One Graph); Output Plot Data, Output Results
  `[<input>]<new>`.
- Still to do in Origin: the report sheet of each dialog (run through the menu, read with `-AfterFile`
  lines `#[Book1]<sheet>` or cell by cell, see `tools/origin-ref/stats3/gen.py`), the two proportion dialogs'
  controls, and the cases of step 2. This session (2026-11-14 evening) had no permission for cargo or
  PowerShell scripts either, so nothing more was run.

Open the dialogs through the menu (`tools/origin-ref/origin-dialog.ps1
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
