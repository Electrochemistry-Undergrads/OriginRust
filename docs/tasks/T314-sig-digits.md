# T314: Significant and Scientific displays as Origin shows them

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Numeric display format" (P0,
gap "Significant and Scientific differ from Origin's digit rules"). Measured in Origin 2016 on 41 values
(`re-workspace/scratch/t314`, local) through `wks.col1.subformat`, `digitMode`, `digits` and
`cell(r,c)$`: the plain display with Significant Digits= 1 to 4, Scientific with 0 to 3 decimal places,
with default digits and with significant digits. Sheaf's `Significant(n)` now writes plain text with
zeros kept or the `1.10E+03` form by Origin's rule, and `Scientific(d)` writes `E+02`-style exponents
with two digits at least; both round the exact value with ties away from zero. 326 of the 328 measured
cells match; 0.995 is rounded up by Origin (`1.00`, `1.0E+00`) and not by Sheaf.
**Size:** S. **Spec:** `docs/spec/workbook-model.md` (Numeric display, T314 point).

## Tests

- `crates/core/src/format.rs`: `significant_and_scientific_give_what_origin_2016_shows` (the measured
  table), `fixed_formats`.

## Left

- Scientific with Default Decimal Digits: done in T317.
