# T438: Probability plot for arbitrarily censored data

**Status:** done 2026-11-24 (Claude Windows "claude:gmail", statistics 47). T384's "Left" (Origin 2026b distribution
analysis with arbitrary censoring): Statistics > Survival Analysis > Distribution Analysis (Arbitrary Censoring)... and
`sheaf analyze lifedist-interval` add a probability plot: the Turnbull estimate's percent failed at the upper end of each
interval on the fitted distribution's probability axis, with the fitted line. Not measured in Origin.
**Size:** S. **Spec:** statistics 47.

## Left

- Confidence bands on the plot, other plotting positions (midpoints), several groups on one plot.

## Tests

- `crates/app/src/app/ui_tests.rs`: `the_survival_menu_fits_arbitrarily_censored_data` (two graphs, the Turnbull points
  rise); the points themselves are T409's checked estimate.
