# T377: Random forest

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", statistics 44). HANDOFF C (Origin 2026b Pro Random Forest
app, "train ensembles of decision trees for predictive modeling"): Statistics > Random Forest (Machine Learning)...
and `sheaf analyze randomforest` grow a seeded forest of CART trees for a class (text) or value response, with the
out-of-bag error and predictions, predictor importance, a confusion matrix for classes and an importance graph. Not
measured in Origin.
**Size:** M. **Spec:** statistics 44.

## Left

- Prediction of new rows from a saved model, class probabilities, permutation importance, class weights, partial
  dependence plots, categorical (text) predictors.

## Tests

- `crates/calc/tests/forest_reference.rs`: a classification tree, a depth-limited regression tree and two seeded
  forests agree exactly with an independent Python implementation (`tools/forest-ref.py`).
- `crates/calc/src/forest.rs`: the one useful split, reproducibility and learning, refusals.
- `crates/core/src/analysis/forest.rs`: a text response classified, a number regressed, the plotted columns.
- `crates/cli/tests/analyze.rs`: `random_forest_runs_from_the_command_line`.
- `crates/app/src/app/ui_tests.rs`: `the_statistics_menu_grows_a_random_forest` (an imported text column is a Label
  column, which the window now includes).
