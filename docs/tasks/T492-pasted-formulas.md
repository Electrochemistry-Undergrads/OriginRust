# T492: Pasted spreadsheet formulas become cell formulas

**Status:** done 2026-11-26 (Claude Windows "claude:3", workbook-model 13u "Pasted formulas"). Left item of T464
(claude:work's, paused); spreadsheet behaviour, Sheaf's own (Origin 2016 has no cell formulas to measure).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Formula-preserving Excel and Sheets import": pasted spreadsheet
formulas.

## What it does

Edit > Paste (and Paste Transpose) of text whose cells start with `=` makes those cells cell formulas (T448 syntax;
AVERAGE and PI() are read as Sheaf's mean and pi). A formula that cannot be read is pasted as text. Pasted cells lose
their old formulas; the status line counts the formulas.

## Tests

- `crates/core/src/cell_formula.rs`: `pasted_spreadsheet_formulas_become_cell_formulas`.

## Left

- Moving relative references by where the paste lands (pasting text takes it as it is); copying formula cells
  within Sheaf as formulas; other Excel functions (IF, COUNT, STDEV ...).
