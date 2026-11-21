# T302: Interpolate writes where Origin's interp1xy writes

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Interpolate and resample"
(P0, gaps "extrapolation option not exposed" (stale: the window has it) and "new sheet only").
Measured in Origin 2016 (`re-workspace/scratch/t302`, local): `interp1xy` and `interp1` add one Y
column per input at the end of the input's sheet, "Interpolated Y1", "Interpolated Y2", ..., comment
"<Linear|Spline|B-Spline|Akima spline> interp of <column>", no X column; the even grid is the column's
sampling interval. Sheaf's Interpolate (window, CLI, MCP) now does the same instead of writing X and Y
to a new sheet.
**Size:** S. **Spec:** `docs/spec/analysis-tools.md` 5 (Where the results go).

## Tests

- `crates/core/src/analysis.rs`: `interpolate_peaks_and_statistics_write_a_new_sheet` (the Interpolate
  part), `interpolation_onto_the_values_of_a_column`,
  `the_fft_filter_and_the_b_spline_are_accepted_and_bad_factors_are_refused`.
- `crates/app/src/app/ui_tests.rs`: `interpolate_y_from_x_takes_the_x_values_of_a_column`.
- `crates/cli/tests/analyze.rs`: `the_fft_filter_and_the_b_spline_can_be_chosen`.
