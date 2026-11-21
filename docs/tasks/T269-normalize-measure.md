# T269: Normalize ref and cell measured in Origin

**Status:** done 2026-11-20 (Claude Windows "claude:3"). HANDOFF next candidates B. Origin 2016's
`normalize method:=ref refcol:= type:=0..4` and `method:=cell cell:=` (`re-workspace/scratch/t269`,
local): `ref` scales each column, `y * stat(ref) / stat(y)`, as T260 computes; `type` 0..4 are min,
max, mean, median, sum, default min; `cell` divides by the cell. The Normalize window and `sheaf
analyze normalize --method ref` now start on min (they started on mean).
**Size:** S. **Spec:** `docs/spec/analysis-tools.md` 4.

## Tests

- `crates/core/src/analysis.rs`: `normalize_ref_and_cell_give_origins_numbers` (Origin's six columns).
