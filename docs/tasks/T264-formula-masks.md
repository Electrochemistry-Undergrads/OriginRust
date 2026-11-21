# T264: Masked cells in formulas, and Origin fixtures for masked and empty rows

**Status:** done 2026-11-20 (Claude Windows "claude:3"), measured in Origin 2016 (HANDOFF next
candidates B). A Set Column Values formula now reads a masked cell as missing, as Origin does
(`col(A)+col(B)` is missing in a masked row); before, Sheaf used the raw value. Fixtures added for two
other measurements Sheaf already matched: descriptive statistics leave a masked row out (n 5 of 6,
sum 82) and Statistics on Rows leaves every field of an all-empty row blank (n too), with the sd and
SEM of a one-value row blank.
**Size:** S. **Source:** HANDOFF.md "Next candidates" B; `re-workspace/scratch/origin-diff/worksheet-formulas`.
**Spec:** `docs/spec/expression-language.md` (Masked cells).

## Tests

- `crates/core/src/formula.rs`: `a_formula_reads_a_masked_cell_as_missing_as_origin`.
- `crates/core/src/analysis.rs`: `masked_rows_and_empty_rows_give_origins_statistics`.

## Left

- Whole-column functions (`mean(col(A))` and the like) now also skip masked cells; Origin was not
  measured for them.
