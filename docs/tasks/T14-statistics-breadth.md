# T14: Statistics breadth

**Status:** items 1 to 4 done, merged (`c44a1a6`) (measured against Origin, `statistics.md` section 10); item 5: Holm and power analysis merged (`d724c40`, measured against Origin's library), Dunnett added from the textbook definition (calc `dunnett`, one-way ANOVA, `--control`), checked against scipy 1.18.1 only because Origin's library gives no Dunnett statistic or p-value; item 6: 2D binning and discrete frequency done (`twoDBinning`, `discfreqs` measured, statistics.md 13, 14), cross tabulation done and checked against scipy `chi2_contingency` and `fisher_exact`, not Origin (`statistics.md` section 15, with its list of unmeasured Origin details: report layout, level order, percent definitions, Fisher rule, filtered rows, level limit). Distribution fit (normal, lognormal, exponential, gamma, Weibull by maximum likelihood with log-likelihood, AIC, BIC, KS and Anderson-Darling), Probability Plot and Q-Q Plot (a new graph and result sheet) done on `stats-dist` and merged, checked against scipy 1.18.1 only (`statistics.md` section 16; Origin's families, parameterizations, plotting positions and axis scales are unmeasured); that branch's own power and sample size code was dropped for the measured one of `d724c40`. Left: the Origin measurements of sections 15.3 and 16.4 on the Windows machine (no Origin on the machine that built the branches). **Size:** M. **Branch:** `stats-breadth`, then `stats-compare` and `stats-dist` (merged with main's measured versions of Holm, 2D binning and power, which won where both existed). **Model:** Sonnet.
**Detail:** audit D "Missing", "Partial". **Spec:** `docs/spec/statistics.md`.

## Today

- Descriptive statistics table shows N, Missing, Mean, SD, SEM, Sum, Min, Q1, Median, Q3, Max
  (`crates/core/src/analysis.rs`, search the column list near "Q3"), although
  `crates/calc/src/stats.rs` also computes variance, skewness, kurtosis, CV, IQR. No confidence limits.
- Tests present: t-tests, one-/two-way ANOVA, Mann-Whitney, Wilcoxon, Kruskal-Wallis, correlation
  (Pearson, Spearman, Kendall), frequency counts, five normality tests (`crates/calc/src/`).

## Items, in order

1. Descriptive table: add the computed columns as Origin's Statistics on Columns options (variance,
   skewness, kurtosis, CV, IQR, confidence limits of the mean); Origin's defaults decide which show.
2. Statistics on Rows (menu greyed).
3. Outlier tests Grubbs and Dixon.
4. Friedman, sign test, median test, two-sample KS.
5. Holm and Dunnett comparisons for ANOVA (done: Holm measured, Dunnett scipy-checked).
6. Cross tabulation, 2D binning, distribution fit, probability and Q-Q plot, power analysis (all done; the
   distribution fit and plots are scipy-checked, not Origin).
   (PCA, k-means, ROC, survival: backlog T18.)

## Method

For each: reference numbers from Origin via `tools/origin-ref/` (see `stats2-*.lines.txt`,
`stats2-make-fixture.py` and `stats2-ref.c` for the pattern), fixture in `crates/calc/tests/`,
tolerance 1e-9 unless Origin prints fewer digits.

## Done when

Each item matches its Origin fixture, appears in the Statistics menu and `sheaf analyze`; checks pass.
