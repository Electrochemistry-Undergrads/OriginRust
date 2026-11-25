# T462: Formula cells move with their rows (rest of T448)

**Status:** done 2026-11-25 (Claude Windows "claude:work", workbook-model.md 13u). A cell formula moves with its row
when rows are inserted, deleted or sorted, through the same row hooks as cell notes. A formula on a deleted row goes
with it. The formula's references are not rewritten.
**Size:** S. **Source:** T448 "Left".

## Left

- References that follow moved rows and columns (rewriting A1 references), as a spreadsheet does.

## Tests

- `crates/core/src/cell_formula.rs`: `a_cell_formula_follows_its_inputs_and_drops_when_typed_over` (insert and delete).
