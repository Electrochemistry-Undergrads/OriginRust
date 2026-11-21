# T317: Scientific with default digits

**Status:** done 2026-11-21 (Claude Windows "claude:3"). T314's left item. `NumberFormat::ScientificAuto`
is Origin's Scientific display with Default Decimal Digits as measured in T314: six significant digits,
the mantissa's trailing zeros dropped, `E` with a sign and at least two exponent digits (`1E+02`,
`1.2345E+04`, `9.999E+01`). Column Properties lists it as "Scientific (default digits)"; the `.xlsx`
writer gives it the number format `0.#####E+00`; the `.opj` writer names it with the other display
formats it does not hold.
**Size:** S. **Spec:** `docs/spec/workbook-model.md` (Numeric display).

## Tests

- `crates/core/src/format.rs`: `scientific_with_default_digits_keeps_six_significant_digits_without_trailing_zeros`.
