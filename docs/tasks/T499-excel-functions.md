# T499: IF, comparisons and more functions in Excel cell formulas

**Status:** done 2026-11-27 (Claude Windows "claude:3", import-excel "Other formulas kept as cell formulas",
workbook-model 13u "Pasted formulas"). Left item of T464 (claude:work's, paused).
**Size:** S. **Source:** T464 "Left": other Excel functions (IF ...).

## What it does

Excel cell formulas imported from `.xlsx` and spreadsheet formulas pasted as text read the comparisons `= <> < > <=
>=` and IF, MEDIAN, STDEV(.S/.P), STDEVP, VAR(.S), POWER, TRUNC and SIGN (import also INT and MOD with Excel's
meaning). A pasted formula's commas inside parentheses no longer split the pasted text.

## Tests

- `crates/io/src/excel/formulas.rs`: `cell_formulas_read_comparisons_and_more_functions`.
- `crates/core/src/cell_formula.rs`: `pasted_spreadsheet_formulas_become_cell_formulas`.
- `crates/core/src/clipboard.rs`: `a_pasted_formula_keeps_the_commas_of_its_arguments`.

## Left

- COUNT (numbers only), AND/OR/NOT, text functions and `&`, VLOOKUP and the other lookups, TRUE/FALSE.
