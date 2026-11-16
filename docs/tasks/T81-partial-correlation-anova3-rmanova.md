# T81: Partial Correlation, Three-Way ANOVA, Repeated Measures ANOVA (B8 slice)

**Status:** partial correlation done (calc, core, dialog, menu, `sheaf analyze partial-corr`, Origin fixtures `crates/calc/tests/origin_t81.rs`); three-way and repeated measures ANOVA open. Origin 2016 measured 2026-11-16 (branch `stats-anova3`, Claude Windows; `docs/spec/statistics.md` 24-26).
**Size:** L. **Branch:** `stats-anova3`. **Spec:** `docs/spec/statistics.md` sections 24 (partial correlation), 25 (three-way ANOVA),
26 (repeated measures ANOVA).
**Inventory:** Statistics > Descriptive Statistics > Partial Correlation Coefficient...; Statistics > ANOVA > Three-Way ANOVA...;
Statistics > ANOVA > Repeated Measures ANOVA... (3 items, tagged `T81`).

## Goal

Three statistics tools that work as Origin 2016's, with their reports, dialogs, command-line and MCP entries:

1. **Partial correlation** (`pcorrcoef`): Pearson partial correlation of two or more variables controlling for one or more others.
2. **Three-way ANOVA** (`ANOVAThreeWay`): type III table, descriptive statistics, the seven means comparisons, Levene, power.
3. **Repeated measures ANOVA**: Origin 2016 has two items, **One-Way Repeated Measures ANOVA** and **Two-Way Repeated Measures
   ANOVA** (spec 26); replace Sheaf's one greyed "Repeated Measures ANOVA..." by those two labels.

## Origin facts (measured; numbers in `tools/origin-ref/stats6/t81-origin.compact.json`)

- **Partial correlation (spec 24).** `r = -P[x,y]/sqrt(P[x,x] P[y,y])` from the inverse correlation matrix of (x, y, controls); t
  test with `df = n - 2 - k`, halved when one-tailed; pairwise deletion uses the n of the pair (x, y) only, listwise the complete
  rows; diagonal "--" and df 0. Refused: no controls, one variable, constant or duplicate control, variable also a control,
  perfect correlation of the pair, n = 3 with one control. Report: Notes, Input Data (variables and controlling variables),
  Descriptive Statistics, Partial Correlations (Partial Corr., p-value, df blocks; p and df only with "Show Significance", `*`
  with "Flag Significant"), footer lines. Defaults: pairwise, two-tailed, significance off, flag off. Pearson only.
- **Three-way ANOVA (spec 25).** Type III sums of squares for balanced and unbalanced designs; rows A, B, C, A*B, A*C, B*C, A*B*C,
  Model, Error, Total. Dialog tabs Input / Model / Descriptive Statistics / Means Comparison (Tukey, Bonferroni, Dunn-Sidak,
  Fisher LSD, Scheffe, Holm-Bonferroni, Holm-Sidak) / Power Analysis / Mean Plot / Output. Report sheet `ANOVA3Way1` in the input
  book.
- **Repeated measures (spec 26).** One-way: sphericity-corrected table (assumed, Greenhouse-Geisser, Huynh-Feldt, lower bound),
  Mauchly's test, epsilons, multivariate tests, between-subjects table, descriptive statistics with confidence limits, means
  comparisons with Dunnett. Two-way (both factors repeated, one number per subject and cell): separate error term per effect,
  the same sphericity rows, Interactions off by default. Result sheets `ANOVAOneWayRM1` and `ANOVATwoWayRM1` in the input book.
- Every number in the three sections was also re-computed with numpy and agrees (partial correlation, type III sums of
  squares, repeated measures sums of squares, Mauchly, Greenhouse-Geisser, lower bound); Huynh-Feldt is capped at 1.

## Where

- New `crates/calc/src/partcorr.rs`, `anova3.rs`, `rmanova.rs` (reuse `anova2.rs` dummy-variable regression, `anova.rs`
  comparison code and `correlation.rs`; no change to the existing functions' behaviour).
- `crates/core/src/analysis/`: new modules `partcorr.rs`, `anova3.rs`, `rmanova.rs` with the `Tool` variants added in
  `analysis.rs` (additive) and the three report layouts; the runner reads factor columns itself as the two-way runner does
  (`two_way_anova` in `analysis/statistics.rs`).
- `crates/app/src/menus.rs` (the three greyed items; two repeated measures items), `crates/app/src/dialogs/analysis.rs`
  (dialogs in Origin's tab layout; the rest local to new files), `crates/ops/src` (`analyze partcorr | anova3 | rmanova1 |
  rmanova2`), `crates/cli`, `crates/mcp`.
- Fixtures: `crates/calc/tests/origin_t81.rs` from the compact JSON (data and numbers; tolerance 1e-5 relative for printed
  5-digit values); scripts to regenerate: `tools/origin-ref/stats6/` (`gp.py`, `gp2.py`, `g3.py`, `t81-make-compact.py`).

## Steps

1. [done] `partcorr`: the calc function (pairwise and listwise), the tool, report, dialog, CLI. Test the 9 measured runs.
2. `anova3`: type III table by dummy-variable regression for balanced and unbalanced data; descriptive blocks; means comparison
   blocks (reuse the two-way ones: cells for the interactions); Levene; power (noncentral F; check the two measured values);
   report and dialog (all seven tabs; Mean Plot only with the graphs, as the two-way tool does: leave unchecked until built).
3. `rmanova1`: within and between tables, Mauchly, epsilons, corrections, multivariate tests, descriptive limits, means
   comparisons (Dunnett on the control level); `rmanova2` with its own error term per effect.
4. Menu: enable the items, delete them from `greyed-items.md` and run `python3 tools/greyed-count.py --write`.
5. Tests, spec sections updated with what the Rust does where it differs, README row status.

## Done when

The three items are gone from `greyed-items.md` and enabled; the Origin runs of the compact JSON reproduce to the printed
digits; unmeasured behaviours are labelled "Sheaf's own, unmeasured" in the spec; fmt, strict clippy and
`cargo test --workspace` pass.

## Don't

- Do not touch Probability/Q-Q plots, cross tabulation, Dunnett (one-way), distribution fit, or other B8 packages.
- Do not use type I or II sums of squares for unbalanced three-way data (Origin gives type III).
- Do not copy Origin's code, help text or report wording beyond the labels the report needs; the OriginC sources in the
  Origin install folder are Origin's code and were not read.
- Do not claim an item enabled if it lacks a part of Origin's dialog that the spec lists (Mean Plot graphs, Raw layout and
  missing values are unmeasured: say so in the item's tooltip or the spec).
- Run the Origin dialog automation only through `tools/origin-ref/origin-dialog.ps1`; the Results Log window may open
  floating and cover menu clicks (click its close button first, as the scripts in the scratch folder did).
