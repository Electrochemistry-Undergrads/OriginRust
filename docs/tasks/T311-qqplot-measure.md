# T311: Measure the Q-Q Plot against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", statistics 16.2a). Origin's Q-Q plot puts the ordered
data on X and the expected values on Y: the fitted quantiles at the scores, with the probability plot's
estimates. Its reference line is `y = x` between the expected values' extremes, rounded out to the 1-2-5 step
nearest a seventh of their range. Its band stands round that line on 51 even points, the probability plot's band
at `p = F(x)`. Sheaf's Q-Q plot now does the same (`sheaf_calc::distfit::{qq_line_ends, qq_band}`) and matches
nine Origin runs (five families, three scales).
**Size:** S. **Follows:** T151, T308. **Spec:** statistics 16.2a.

## Left

- Origin's sheet layout and Exchange X-Y Axes (Sheaf's "Data on X" tick does the swap).

## Tests

- `crates/calc/tests/origin_qqplot.rs` (fixture `fixtures/qqplot_origin.rs`): the expected values, the line ends,
  the band grid and the band for nine runs.
- `crates/core/src/analysis/dist.rs`: `origins_documented_options_blom_scores_a_band_and_data_on_x`.
