# T347: Best Subset Selection

**Status:** done 2026-11-21 (Claude Windows "claude:2"). Sheaf's own (Origin's 2025b Pro app not run).
HANDOFF C / origin-2026-2027-gaps "Best Subset Selection". Every subset of up to 15 candidate predictors is
regressed with an intercept on the common complete rows and scored by R-square, adjusted R-square,
Mallows Cp, AIC and BIC; the table with the preferred subsets marked goes to a new sheet.
**Size:** M. **Spec:** `docs/spec/fitting.md` 12a.

## Tests

- `subset::tests` (fit), `best_subsets::every_subset_goes_to_a_new_sheet_with_the_preferred_ones_marked`
  (core), `best_subsets_ranks_every_subset_of_the_regressors` (cli).

## Left

- Forced-in predictors, a limit on the subset size, the best m per size only; compare with Origin Pro's app.
