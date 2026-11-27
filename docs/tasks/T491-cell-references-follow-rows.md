# T491: Cell formula references follow inserted and deleted rows

**Status:** done 2026-11-26 (Claude Windows "claude:3", workbook-model 13u). Left item of T462 (claude:work's,
paused); spreadsheet behaviour, Sheaf's own (Origin 2016 has no cell formulas to measure).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Cell formulas and Formula Bar": references that move with rows.

## What it does

Inserting or deleting rows (whole rows or cells of some columns) rewrites the sheet's cell formulas: references to
the cells of those columns below the change move with their rows, ranges grow or shrink, and a reference to a
deleted cell becomes `#REF!` (its formula cell is then empty). `$` does not hold a reference still; sorting rewrites
nothing.

## Tests

- `crates/core/src/cell_formula.rs`: `references_follow_inserted_and_deleted_rows`,
  `formula_cells_keep_reading_their_inputs_when_rows_move`, and the row-move steps of
  `a_cell_formula_follows_its_inputs_and_drops_when_typed_over`.

## Left

- References from other sheets of the book to this sheet's moved rows; references that follow moved or deleted
  columns.
