# T40: Graphical Summary (descriptive report with normality test and charts)

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `graph-summary`.
**Size:** L. **Source:** `origin-2026-2027-gaps.md` (Analysis: Graphical Summary, Origin 2026).

## Goal

Statistics > Descriptive Statistics > Graphical Summary makes, for each input column (and each
group when group columns are given), a report of descriptive statistics, confidence intervals and
an Anderson-Darling normality test, with diagnostic charts, in Origin's "Separate Graphs" layout.

## Origin facts (docs, not measured)

- Menu "Statistics: Descriptive Statistics: Graphical Summary". Inputs: one or more columns (or a
  Data Table), optional Groups column(s), Confidence Interval (%) level, Graph Arrangement:
  Separate Graphs, Per Variable, Per Group, All in One. [W:origin-help/graphical-summary]
- Separate Graphs: per variable-group pair a graph with three plots (histogram with distribution
  curve, confidence-interval plot, probability plot) and three tables ("Description Statistics",
  "##% Confidence Intervals", "Normality Test"), titled by variable and group.
  [W:origin-help/graphical-summary; blog.originlab.com/graphical-summary]
- Per Variable / Per Group: histogram with curve, box chart with outliers, CI plot, panels per
  group/variable, one summary table; All in One: all subsets in one graph. (Left for later.)
- The descriptive table shows mean and median with their confidence intervals; summary statistics
  include the Anderson-Darling normality test (2026 release). [W:blog; press release]
- Not stated: full statistic list, default confidence level (Sheaf: 95 %), output destination,
  the curve (Sheaf: normal with the sample mean and SD), the A-D p-value method (Sheaf: the
  existing `sheaf-calc` normality A-D, as the Normality Test tool reports it).

## Where

- Reuse `crates/calc` (descriptive statistics, `normality.rs` Anderson-Darling, quantiles, CI for
  mean (t), median (order-statistic/binomial) and SD (chi-square)), existing histogram, probability
  plot and box/CI plot code from T14 (statistics.md 16) and T12.
- `crates/core` analysis tool + report sheet + graph construction (see how the Normality Test,
  Descriptive Statistics and probability plot tools build their outputs), Statistics menu item and
  options dialog in `crates/app`, `sheaf analyze graphsummary` in ops/CLI (MCP reaches it through
  the bridge).
- `docs/spec/statistics.md` new section.

## Done when

Separate Graphs output for ungrouped and grouped input: report tables with the statistics, CIs
and A-D values (checked against scipy/R-style reference values in calc tests), one graph per
variable-group with histogram + normal curve, CI plot and probability plot; menu, dialog and CLI;
tests; spec records Origin's facts and Sheaf's choices. Other arrangements left in Status.

## Don't

- Don't change the existing Normality Test, Descriptive Statistics, distribution fit or ANOVA
  tools' results (other sessions measure them against Origin).
