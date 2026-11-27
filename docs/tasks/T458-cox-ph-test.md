# T458: Cox proportional-hazards test

**Status:** done 2026-11-25 (Claude Windows "claude:gmail", statistics 52). T408's "Left" (the proportional-hazards
check): Cox Proportional Hazards... ("Test proportional hazards") and `sheaf analyze cox --ph-test` report Grambsch and
Therneau's test on the scaled Schoenfeld residuals against the rank of the failure times, per covariate and global. Not
measured in Origin.
**Size:** S. **Spec:** statistics 52.

## Left

- The exact score form of the test (R's cox.zph since 3.0), a plot of the scaled residuals against time. (The
  Kaplan-Meier, identity and log time transforms: T503.)

## Tests

- `crates/calc/tests/cox_reference.rs`: the test of all six reference cases against numpy/scipy (`tools/cox-ref.py`).
- `crates/core/src/analysis/lifedist.rs` (`cox_residuals_and_a_curve_at_given_values`), `crates/cli/tests/analyze.rs`
  (`--ph-test`).
