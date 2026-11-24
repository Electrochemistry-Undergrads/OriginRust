# T417: Excel formulas as column formulas

**Status:** done 2026-11-24 (Claude Windows "claude:1", import-excel section 4), not measured (Origin 2026 feature).
An imported Excel column whose data cells hold one formula written for their own row keeps it as its column formula
(`=A2*2` down column B becomes `col(A)*2`, Recalculate Auto, values as stored); other formulas keep their stored values
with a warning naming the column and its first formula. `sheaf_io::excel::formulas`.
**Size:** S. **Source:** HANDOFF C, origin-2026-2027-gaps "Formula-preserving Excel and Sheets import".

## Tests

- `crates/io/src/excel/formulas.rs`: what is translated and what is refused.
- `crates/io/src/excel/tests.rs`: `a_same_row_formula_column_keeps_its_formula` (`basic.xlsx`),
  `formulas_that_are_not_one_same_row_formula_keep_only_their_values` (`formulas.xlsx`, made by
  `tools/make-excel-fixtures.py`).

## Left

- Per-cell formulas (Origin's cell formulas), pasting spreadsheet formulas, more Excel functions (IF, SUM over a row),
  Google Sheets.
