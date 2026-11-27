# T464: Excel cell formulas import as cell formulas (rest of T417, with T448)

**Status:** done 2026-11-25 (Claude Windows "claude:work", import-excel.md "Other formulas kept as cell formulas").
In an imported Excel column whose formulas are not one same-row formula (T417 keeps those as a column formula), each
cell's readable formula becomes a cell formula (T448). References are mapped to the imported columns and data rows,
`$` markers are kept, ranges stay in one column, and SUM, AVERAGE, MIN, MAX and T417's functions are read. Unreadable
ones keep their stored values with a warning. Cell formulas now follow a workbook's new column ids when it joins a
project, and a one-row range (`A1:A1`) stays a range.
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Formula-preserving Excel and Sheets import" (per-cell formulas).

## Left

- Pasted spreadsheet formulas: done (T492); other Excel functions (IF, VLOOKUP, COUNT ...), references to other sheets, text
  formulas.

## Tests

- `crates/io/src/excel/tests.rs`: `formulas_that_are_not_one_same_row_formula_become_cell_formulas` (the formulas,
  the warning, Excel's values from the cell formulas after the book joins a project).
