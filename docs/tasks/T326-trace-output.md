# T326: Trace Interpolation writes where Origin writes

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Interpolate onto given X, trace"
(P1). Measured in Origin 2016 (`re-workspace/scratch/t326`, local): `interp1trace` fails on a vertical
curve, as Sheaf's refusal does; it writes `InterpolatedX1` (X) and `InterpolatedY1` (Y, comment
"Linear trace interp of B", "Spline" or "B-Spline" for the other methods) at the end of the input
sheet, and later runs go on with 2, 3. Sheaf's Trace Interpolation now does the same for the default
output; "New sheet" in the window still makes a sheet of its own (with the same names).
**Size:** S. **Spec:** `docs/spec/analysis-tools.md` 16.

## Tests

- `crates/core/src/analysis.rs`: `a_trace_interpolation_goes_to_the_end_of_the_sheet_with_its_own_x`.
- `crates/cli/tests/analyze.rs`: `trace_interp_resamples_a_loop_into_a_new_sheet`.
- `crates/app/src/app/ui_tests.rs`: `trace_interpolation_opens_from_mathematics_and_writes_a_sheet_with_its_own_x`.
