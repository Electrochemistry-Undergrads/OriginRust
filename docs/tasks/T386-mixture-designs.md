# T386: Mixture designs for one or more totals

**Status:** done 2026-11-22 (Claude Windows "claude:2"). Sheaf's own (Origin 2016 has no design of
experiments; Origin 2026b's mixture designs were not run).
HANDOFF C / origin-2026-2027-gaps "Mixture designs with several totals". Simplex lattice {q, m} and
simplex centroid designs, optional axial check blends and overall centroid, scaled from pseudo-components
to each total with lower bounds; replicates and a seeded run order; a design sheet with an empty response.
Statistics > Two-Level Experiments > Mixture Design..., CLI `analyze mixture-design`.
**Size:** M. **Spec:** `docs/spec/statistics.md` 45a.

## Tests

- `mixture::tests` (calc), `doe::tests::a_mixture_design_sheet` (core), `mixture_design_for_two_totals` (cli).

## Left

- Upper bounds (extreme vertices), mixture-process variables, Scheffe model fits of the response.
