# T448: Cell formulas and the Formula Bar (Origin 2019 / 2021)

**Status:** done 2026-11-25 (Claude Windows "claude:work", workbook-model.md 13u). Typing `=` and an expression into a
cell (`=A1*10`, `=sum(A1:A5)/B2`, `=Sheet2!A1+1`) keeps a cell formula. The cell shows the value, recalculated in every
edit's undo step, as cell links are. Spreadsheet references are A1-style column-and-row names of the sheet, ranges,
and other sheets of the book. `sum` is the total, as in a spreadsheet. The Formula Bar (View > Formula Bar, off to start
with) above each workbook's grid names the current cell and shows its formula (or its value) for editing; Enter writes it. Typing a value over a
formula cell drops the formula, and a formula that cannot be read is refused. Cell formulas save with the project.
Sheaf's own rules (Origin 2016 has neither).
**Size:** M. **Source:** origin-2026-2027-gaps.md, "Cell formulas and Formula Bar".

## Left

- References that follow rows and columns when they move, absolute/relative `$A$1` references, filling a formula
  down, cell formulas in .opj files, a marker on formula cells, and other workbooks' cells in A1 style.

## Tests

- `crates/core/src/cell_formula.rs`: reference translation, a chain of formula cells following an input, another
  sheet, save round trip, dropping a formula, a bad formula.
- `crates/app/src/app/ui_tests/notes_window.rs`:
  `a_cell_formula_is_typed_shown_in_the_formula_bar_and_follows_its_input`.
