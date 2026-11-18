# T29: PCA, k-means, ROC curve, Kaplan-Meier

**Status:** done on branch `stats-multi` (2026-11-15, not yet merged into main). PCA (spec section 17) and k-means
(section 18) and Kaplan-Meier (section 20) are measured against Origin 2016 with `tools/origin-ref`; ROC (section 19)
is scipy-checked, not Origin (its X-Function is menu-only). Each has a dialog, an enabled menu item, `sheaf analyze`
and a graph (scree, cluster scatter, survival curves with censor marks; ROC curve). Leftovers done on branch
`pca-plots` (2026-11-15): loading plot, score plot and biplot as Origin draws them (spec section 17.1, axes of the
biplot measured in seven runs), Origin's report tables as the layout of the result sheets of PCA, k-means and
Kaplan-Meier (title of each table in the comment row, Origin's labels and order; sections 17.2, 18.2, 20.2), the
confidence limits of the survival curve (Origin's `sfci`: linear, not cut off; section 20.1). Left: the numbers of
Origin's ROC (menu-only X-Function), PCA observation labels, Origin's own k-means graph, the Notes and Input Data
tables. Brief written 2026-11-14 from T18's "Multivariate" row, T14's leftovers and parity
rows "Multivariate analysis" and "Survival, power, ROC", both P2).
**Size:** L (four tools; each can merge on its own, in the order below). **Branch:** `stats-multi`.
**Model:** Sonnet.
**Detail:** audit D (Kaplan-Meier, PCA, k-means, ROC missing).
**Spec:** `docs/spec/statistics.md`: copy the layout of sections 12-16 (what Origin does, what Sheaf
does, fixtures, what was not measured) into new sections 17 (PCA), 18 (k-means), 19 (ROC), 20
(Kaplan-Meier). Section 5 (where results go, ADR 0005) applies unchanged.

## Goal

Statistics > Multivariate Analysis > Principal Component Analysis and K-Means Cluster Analysis,
Statistics > ROC Curve and Statistics > Survival Analysis > Kaplan-Meier Estimator work as tools of the
shared analysis frame: results in report and result sheets as the other statistics tools, plots
(scree, loadings, biplot; ROC curve; survival curve with censor marks) as graphs, `sheaf analyze`
subcommands, recalculation links.

## Origin facts

- Not measured. From Origin's public documentation (paraphrased): PCA on correlation or covariance
  matrix, eigenvalues with percent of variance, loadings and scores, optional scree plot, loading plot
  and biplot; k-means with a chosen cluster count, initial centres, iteration limit, cluster centres
  and memberships with distances; ROC with a state column and a positive value, area under the curve
  with standard error and confidence interval; Kaplan-Meier with time, censor column and censor value,
  survival estimates with standard errors and confidence limits, quartile survival times and, for
  groups, log-rank and related tests.
- Measure each with `tools/origin-ref` on the Windows machine (LabTalk X-Functions: `pca`,
  `kmeans`, `roc`, `kaplanmeier`, names to confirm from Origin's X-Function list) on synthetic data in
  `re-workspace/scratch/t29/`; fixtures as in `crates/calc/tests/origin_*.rs`. Signs of eigenvectors
  are arbitrary: record Origin's sign rule before comparing loadings. Until measured, check against
  scipy/scikit-learn/lifelines and label the section "scipy-checked, not Origin", as sections 15-16.

## Where

- `crates/calc/src/` (look at how `distfit.rs` and `crosstab.rs` are laid out): `pca.rs`
  (symmetric eigen-decomposition; `nalgebra` is already a dependency of `crates/fit`, reuse it rather
  than adding a crate), `kmeans.rs` (deterministic initial centres so results are reproducible),
  `roc.rs` (AUC by the trapezoid rule; the standard error as Origin's, measured 2026-11-15, statistics.md 19.5),
  `survival.rs` (product-limit estimate, Greenwood variance, log-rank).
- `crates/core/src/analysis/statistics.rs`: the `Tool` variants and their specs.
- `crates/app/src/menus.rs` (Statistics menu items now grey) and `crates/app/src/dialogs/` (options).
- `crates/cli/src/args.rs`, `analyze.rs`: `analyze pca`, `kmeans`, `roc`, `kaplan-meier`.

## Steps

For each tool, in the order PCA, ROC, Kaplan-Meier, k-means (owner relevance first: PCA for
spectra/XRD sets):
1. Numerics with doc comments and hand-worked tests.
2. Origin measurement (Windows) or scipy check, fixture, spec section.
3. Tool variant, dialog, menu, CLI, plots.
4. Checks and merge; parity rows.

## Done when

Each of the four menu items opens a working dialog whose report matches Origin (or the reference
library, said in the spec) within fixture tolerances; `sheaf analyze` gives the same numbers; fmt,
clippy and tests pass.

## Don't

- Don't add a large linear-algebra crate; `nalgebra` is in the workspace.
- Don't claim Origin numbers for scipy-checked results.
