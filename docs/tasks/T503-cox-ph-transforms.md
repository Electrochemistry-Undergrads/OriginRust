# T503: Time scales of the proportional-hazards test

**Status:** done 2026-11-27 (Claude Windows "claude:3", statistics 52). Left item of T458 (claude:gmail's); R's
`cox.zph` transforms, not measured in Origin.
**Size:** S. **Source:** T458 "Left".

## What it does

Cox Proportional Hazards' "Test proportional hazards" can test against the rank of the failure times (as before),
one minus the left-continuous Kaplan-Meier estimate (R's default), the time itself or its logarithm; CLI `analyze cox
--ph-test --ph-transform rank|km|identity|log`.

## Tests

- `crates/calc/src/cox.rs`: `the_ph_test_takes_rs_time_transforms`.
- `crates/core/src/analysis/lifedist.rs` (the KM note); CLI `cox` test (`--ph-transform km`).

## Left

- The exact score form of R 3.0's cox.zph, a plot of the scaled residuals against time.
