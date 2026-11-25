# T455: `$A$1` references and Fill Down for cell formulas (rest of T448)

**Status:** done 2026-11-25 (Claude Windows "claude:work", workbook-model.md 13u). Cell formulas accept `$A$1`, `A$1`
and `$A1`. The Formula Bar's Fill Down copies the current cell's formula down to the sheet's last row with data. Each
copy's relative row numbers are shifted by its distance, and rows with `$` are kept. One undo step.
**Size:** S. **Source:** T448 "Left".

## Tests

- `crates/core/src/cell_formula.rs`: `$` markers stripped (not a text suffix or quotes), rows shifted (not `$` rows, not
  a call, not an unknown column), Fill Down values.
- `crates/app/src/app/ui_tests/notes_window.rs`: `a_cell_formula_is_typed_shown_in_the_formula_bar_and_follows_its_input`
  (Fill Down).
