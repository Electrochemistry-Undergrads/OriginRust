# T296: CV Charge (anodic and cathodic sweeps, scan-rate division)

**Status:** done 2026-11-21 (Claude Windows "claude:2"). Sheaf's own tool (Origin 2016 has none; its way
is `integ1` and a hand division), for the owner's cyclic voltammetry work.
**Source:** feature-parity "CV charge with sloped baseline" (P0) gaps "no anodic and cathodic split, no
scan-rate division". **Size:** S. **Spec:** `docs/spec/analysis-tools.md` 21.

## Done

- `sheaf_calc::cv_charge`: the sweeps between turns of the potential, each sweep's charge
  `sum (I_k + I_k+1)/2 |dE| / v`, optional end-point baseline per sweep.
- `Tool::CvCharge(CvChargeSpec)` (results sheet `CVCharge<n>`, a row per sweep and the anodic and
  cathodic totals), Analysis > Mathematics > CV Charge..., `sheaf analyze cv-charge --scan-rate`.

## Tests

- `integrate::cv_tests` (calc), `analysis::tests::cv_charge_gives_each_sweep_and_the_totals` (core),
  `cv_charge_splits_the_sweeps` (cli).

## Left

- Picking the peak's feet on the graph.
