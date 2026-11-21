# T325: Average Multiple Curves sorts a turning X as Origin

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Average multiple curves" (P1,
gap "Origin sorts a turning X, Sheaf refuses it"). Measured in Origin 2016 (`re-workspace/scratch/t322`,
local): `avecurves` on a curve with X 0, 1, 2, 3, 2, 1, 0 (Y 0 to 6) and a flat curve 1 on X 0 to 3 gave
the mean 2 at X 0, 1, 2, 3: the turning curve is sorted by X and its points at equal X averaged. Sheaf now
does the same instead of refusing the curve (`monotonic_increasing` in `crates/calc/src/curves.rs`).
**Size:** S. **Spec:** `docs/spec/analysis-curve-math.md` 1.

## Tests

- `crates/calc/src/curves.rs`: `a_turning_curve_is_sorted_by_x_and_equal_x_averaged_as_origin`.
- `crates/core/src/analysis/curves.rs`: `average_needs_two_curves_and_sorts_a_turning_x`.
- `crates/cli/tests/curves.rs`: `average_on_a_cycle_is_sorted_by_x_or_averaged_row_by_row_on_a_shared_x`.
