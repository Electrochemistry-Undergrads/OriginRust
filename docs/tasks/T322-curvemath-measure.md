# T322: Intersect writes into the input sheet; curve tools checked against Origin

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Level crossing and curve
intersect" and "Polygon area" (P1, "not measured"; most was measured on 2026-11-13, analysis-curve-math 4).
Measured again in Origin 2016 (`re-workspace/scratch/t322`, local): `intersect` adds "Intersection X1"
(X) and "Intersection Y1" (Y) at the end of the input sheet, without comments, and a second run adds X2
and Y2; `polyarea` writes no column, only a number. Sheaf's Intersect now writes its columns there for
the default output, numbered on across runs. Fixed on the way: when a linked result is run again
(Auto/Manual recalculation, Apply), a numbered label such as `Intersection X1`, `Normalized1` or
`Interpolated Y1` keeps its number instead of taking the next one (`same_numbered_label` in
`analysis_link.rs`), which also affected T300, T302 and T321.
**Size:** S. **Spec:** `docs/spec/analysis-curve-math.md` 4.

## Tests

- `crates/core/src/analysis/curves.rs`: `intersect_lists_the_crossings`.
- `crates/core/src/analysis_link.rs`: `a_numbered_label_keeps_its_number_when_the_run_is_replaced`.
- `crates/app/src/app/ui_tests.rs`: `the_mathematics_menu_finds_the_intersection_of_two_curves`,
  `an_auto_intersect_follows_an_edit_of_its_input`.
- `crates/cli/tests/curves.rs`: `intersect_needs_two_curves_and_lists_the_point`.
