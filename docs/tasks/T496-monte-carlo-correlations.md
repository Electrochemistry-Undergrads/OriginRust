# T496: Monte Carlo correlated inputs and output histogram

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 51). Left items of T393/T490; Origin 2027's Monte
Carlo app was not run, so this is Sheaf's own.
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Monte Carlo simulation workflow".

## What it does

Statistics > Monte Carlo Simulation (Randomize Inputs)... takes correlations between inputs (`a, b = 0.6; ...`, CLI
`--correlation a,b=0.6`), drawn through a Gaussian copula so each input keeps its distribution, and draws a histogram
of the output (and of the output at the optimum) with the specification limits.

## Tests

- `crates/calc/src/montecarlo.rs`: `correlated_inputs_keep_their_distributions_and_correlate`.
- `crates/core/src/analysis/montecarlo.rs`: `a_simulation_writes_draws_summary_capability_and_sensitivity`,
  `the_optimisation_moves_controls_to_the_best_setting` (the histogram columns).
- `crates/app/src/dialogs/multivariate_options.rs`: `the_monte_carlo_window_reads_controls_and_the_goal`.
- `crates/cli/tests/analyze.rs`: `monte_carlo_summarises_the_output`.

## Left

- Spreads of inputs as controls; rank correlations as given (Iman-Conover).
