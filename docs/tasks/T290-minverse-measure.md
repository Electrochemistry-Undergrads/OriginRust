# T290: Measure Matrix Inverse against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", matrices 7d). Origin's `minverse` puts its
result in a new MBookN without a long name, in one sheet "Inversed1" on the default ranges. Singular and
non-square matrices get the Moore-Penrose pseudoinverse (cut-off `max(m, n) * eps * s_max`) and a
"not full rank" warning. A missing cell takes part as Origin's missing number, in effect 0. Sheaf now
does all of this (`sheaf_calc::minverse`), with a full-rank rule that reproduces the 25 measured cases.
**Size:** S. **Follows:** T193. **Spec:** matrices 7d.

## Left

- Origin's exact full-rank test (not documented; Sheaf's rule matches every measured case).
- Origin's 3e-301 trace of the missing number in the result (Sheaf gives 0).

## Tests

- `crates/calc/src/minverse.rs`: Origin's values for full-rank, singular, wide, tall and badly scaled
  matrices, the full-rank test on every measured case, the cut-off, and missing cells.
- `crates/core/src/matrix.rs`: `inverse_tests::the_inverse_of_a_matrix_as_origins_minverse`.
