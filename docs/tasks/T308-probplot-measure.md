# T308: Measure the Probability Plot against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", statistics 16.2a). Origin's `plot_prob`, driven
through its window, has five score methods (Blom default, Benard, Hazen, Van der Waerden, Kaplan-Meier).
It estimates by maximum likelihood with the normal SDs over n - 1, and draws its reference line between half
the first percent and 100 minus that. Its confidence band at 27 fixed percents follows a measured formula per
family: normal and lognormal `k sigma sqrt(1/n + z^2/(2n - 3))`, exponential `exp(-+k/sqrt(n))`, Weibull from
the observed information, gamma from the Fisher information. Sheaf now computes all of these
(`sheaf_calc::distfit::{prob_plot_params, confidence_band, quantile}`, `Scores::KaplanMeier`) and matches
Origin to 1e-12 (Weibull 1.4e-8, gamma 1e-8).
**Size:** M. **Follows:** T151. **Spec:** statistics 16.2a.

## Left

- Origin's sheet layout (Percentiles, Reference Line and Additional Table columns), Exchange X-Y Axes and
  fixed parameters.
- Origin's Q-Q plot: done by T311.

## Tests

- `crates/calc/tests/origin_probplot.rs` (fixture `fixtures/probplot_origin.rs`): Origin's estimates and band
  for eight runs (all families, n 10 and 20, levels 95 and 90), and its score methods.
- `crates/core/src/analysis/dist.rs`: `origins_documented_options_blom_scores_a_band_and_data_on_x`.
