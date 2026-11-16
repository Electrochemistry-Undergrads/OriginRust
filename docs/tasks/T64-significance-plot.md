# T64: Multiple comparisons plot (significance brackets)

**Status:** done 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `significance-plot`. Spec: statistics.md 23. Left: Two-Way ANOVA, paired t-test and the one-sample Significance Plot, Origin measurements (everything is from documents), picking/editing brackets, a mini toolbar.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Analysis: Multiple Comparisons and Significance
Plots, Origin 2027).

## Goal

From a one-way ANOVA report with a means comparison (and from a two-sample t-test report), make a graph
of the groups (bar of means, box, violin or scatter) with error bars (SD, SEM or 95 % CI) and brackets
between compared groups labelled with asterisks or p-values, optionally only for significant pairs.

## Origin facts (docs, not measured)

- Origin 2027: "Multiple Comparisons Plot" made directly from One-Way ANOVA, Two-Way ANOVA, Two-Sample and
  paired t-Test reports, launched from an icon in the report sheet; "Significance Plot" from a One-Sample
  t-Test. Plot types bar, box, violin, scatter; error bars SD, SEM or 95 % CI; customizable brackets and
  annotations, can sit outside the plot area; option to show only p-values meeting a threshold; a mini
  toolbar customizes asterisk bracket labels. [W:Origin 2027 product page]
- Older free app "Paired Comparison Plot": marks "Letters on Top", "Asterisk with Bracket" or "P-Value with
  Bracket"; "Exclude n.s. in Group". [W:app/paired-comparison-plot]
- Not documented (Sheaf's choices, unmeasured): star thresholds (`*` p < 0.05, `**` < 0.01, `***` < 0.001,
  `****` < 0.0001, `ns` otherwise — the common convention), default plot type (bar of means + SD),
  bracket stacking (shortest spans lowest, each new level 6 % of the data range above the tallest element
  beneath it), default "significant only" (on), Two-Way ANOVA and one-sample t-test (left).

## Where

- `crates/core/src/analysis/statistics.rs` one-way ANOVA means-comparison results and the two-sample
  t-test report (read the stored comparison p-values; don't change their computation).
- New plot construction: a graph from a report (groups' data + comparisons) with a bracket annotation
  type (layer-level, linked to the report's group order); `crates/plot` bracket drawing (lines + label,
  stacking); app: a "Multiple Comparisons Plot" button/context entry on the ANOVA/t-test report sheet and
  a small options dialog (plot type, error bar, label style asterisks/p-values, significant only,
  threshold); CLI `sheaf analyze anova ... --comparison-plot OUT.png` or a `sheaf plot` option; MCP.
- `docs/spec/statistics.md` / graphing.md section.

## Done when

The plot is made from one-way ANOVA (Tukey etc.) and two-sample t-test results with correct groups,
error bars, brackets and labels (tests vs the report's p-values), from the app, CLI and MCP; spec.

## Don't

- Don't change ANOVA/t-test results or other sessions' T53-T57 scope.
