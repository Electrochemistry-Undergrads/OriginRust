# T393: Monte Carlo simulation

**Status:** done 2026-11-23 (Claude Windows "claude:2"). Sheaf's own (Origin 2027's app was not run).
HANDOFF C / origin-2026-2027-gaps "Monte Carlo simulation workflow". Inputs with distributions (normal, uniform,
lognormal, triangular, constant) drawn with a seed, an output formula, the draws in a new sheet, summary and
percentiles, specification limits (share outside, Cp, Cpk) and Spearman sensitivity per input.
Statistics > Monte Carlo Simulation..., CLI `analyze monte-carlo`.
**Size:** M. **Spec:** `docs/spec/statistics.md` 50.

## Tests

- `montecarlo::tests` (calc: parsing, draws and seeds, percentiles, capability, sensitivity),
  `montecarlo::tests::a_simulation_writes_draws_summary_capability_and_sensitivity` (core),
  `monte_carlo_summarises_the_output` (cli).

## Left

- Optimizing controllable inputs against tolerances; correlated inputs; an output histogram graph.
