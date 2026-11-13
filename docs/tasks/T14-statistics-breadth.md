# T14: Statistics breadth

**Status:** items 1 to 4 done, merged (`c44a1a6`) (measured against Origin, `statistics.md` section 10); items 5 and 6 (Holm and Dunnett, cross tabulation, 2D binning, distribution fit, power) open. **Size:** M. **Branch:** `stats-breadth`. **Model:** Sonnet.
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
5. Holm and Dunnett comparisons for ANOVA.
6. Cross tabulation, 2D binning; then distribution fit, probability plot; then power analysis.
   (PCA, k-means, ROC, survival: backlog T18.)

## Method

For each: reference numbers from Origin via `tools/origin-ref/` (see `stats2-*.lines.txt`,
`stats2-make-fixture.py` and `stats2-ref.c` for the pattern), fixture in `crates/calc/tests/`,
tolerance 1e-9 unless Origin prints fewer digits.

## Done when

Each item matches its Origin fixture, appears in the Statistics menu and `sheaf analyze`; checks pass.
