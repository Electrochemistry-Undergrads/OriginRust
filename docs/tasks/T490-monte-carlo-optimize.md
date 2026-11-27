# T490: Monte Carlo Simulation optimizes controllable inputs

**Status:** done 2026-11-26 (Claude Windows "claude:3", statistics 51). Left item of T393 (claude:2's, idle);
Origin 2027's Monte Carlo app has this step, Sheaf's own design (the app was not run).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Monte Carlo simulation workflow": the optimization step.

## What it does

Statistics > Monte Carlo Simulation (Randomize Inputs)... has Optimize centres of (`name = low:high, ...`, up to 4
inputs) and a goal: Highest Cpk, or Mean at target with the least spread. The inputs named move their centres
within their ranges with the same random numbers to the best setting (a grid, then a compass search); the output
and the moved inputs at that setting are added to the table and the sheet, with a note comparing it with the
setting before. CLI: `analyze monte-carlo ... --optimize NAME=LOW:HIGH [--aim T]`.

## Tests

- `crates/calc/src/montecarlo.rs`: `inputs_move_to_a_centre_with_the_same_draws`,
  `controls_are_read_and_the_box_search_finds_the_lowest_point`.
- `crates/core/src/analysis/montecarlo.rs`: `the_optimisation_moves_controls_to_the_best_setting`.
- `crates/app/src/dialogs/multivariate_options.rs`: `the_monte_carlo_window_reads_controls_and_the_goal`.
- `crates/cli/tests/analyze.rs`: `monte_carlo_summarises_the_output` (with `--optimize` and `--aim`).

## Left

- Spreads (tolerances) of inputs as controls, with a cost; correlated inputs; an output histogram graph.
