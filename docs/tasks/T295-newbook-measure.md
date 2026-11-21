# T295: A new workbook measured in Origin

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "New worksheet default shape"
(P0, "not compared"). Origin 2016's `newbook` (`re-workspace/scratch/t295`, local): `wks.nrows` 32,
`wks.ncols` 2, column 1 type 4 (X) and column 2 type 1 (Y), `wks.col1.width` 7. Sheaf already made
A(X) and B(Y); now an empty sheet shows 32 rows (a filled one keeps its data plus 30 spare rows to
type into) and the default column width is 7 characters (was 8; the grid draws a default column at
the same pixel width as before, Column Properties now shows 7).
**Size:** S. **Spec:** feature-parity "New worksheet default shape".

## Tests

- `crates/app/src/grid.rs`: `a_new_sheet_shows_origins_32_rows_and_a_filled_one_its_spare_rows`.
