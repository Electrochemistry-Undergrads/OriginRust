# T304: A sampling interval is its column's X everywhere

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Sampling interval" (P0, gap
"analysis tools do not use it"), and a follow-up to T302 (Interpolate's results, which have an interval,
now sit in sheets that have an X column). Origin writes a column with an interval as belonging to no X
column (T303) and draws it at its interval beside an X column (T84). Sheaf's analysis input now takes
the interval as the X of such a column (X name: the interval's long name, else "X"; its units), and a
plot whose Y column has an interval uses it even when the plot names an X column, with the interval's
title on the X axis.
**Size:** S. **Spec:** `docs/spec/workbook-model.md` 13d.

## Tests

- `crates/core/src/analysis.rs`: `a_sampling_interval_is_the_x_of_its_column_even_beside_an_x_column`.
- Existing: `crates/plot/src/sampling_tests.rs`, `crates/io/tests/sampling_interval.rs`.
