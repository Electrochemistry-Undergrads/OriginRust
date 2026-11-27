# T502: Random forest permutation importance

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 44). Left item of T377 (claude:gmail's); Origin
2026b's Random Forest app was not run.
**Size:** S. **Source:** T377 "Left".

## What it does

Statistics > Random Forest gives each predictor's permutation importance (Breiman's: the mean increase of each tree's
out-of-bag error when the predictor is shuffled among that tree's out-of-bag rows) next to the impurity importance, in
the table, on the sheet and in a note. The shuffles use their own seeded stream, so the forest itself is unchanged.

## Tests

- `crates/calc/src/forest.rs`: `a_forest_is_reproducible_and_learns`,
  `permutation_importance_of_a_regression_follows_the_signal`.
- `crates/core/src/analysis/forest.rs`: `a_text_response_is_classified_and_a_number_regressed`.

## Left

- Scaled (z) permutation importance, conditional importance, the rest of T377's list.
