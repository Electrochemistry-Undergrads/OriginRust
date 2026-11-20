# T234: Plot > Profile > Contour Profiles, Image Profiles (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"), **not measured**.
**Size:** S. **Source:** greyed inventory package B14 ("Profile: Contour Profiles · Image Profiles").
**Spec:** `docs/spec/matrices.md` 7l.

## Done

- `Project::matrix_profile` (a row or column cut into a workbook and a line graph) and
  `Cmd::Profile`. The window is `Form::Profile` in `dialogs/matrix_ops.rs`.
- Test: `matrix_window::profiles_cut_a_row_or_column_into_a_workbook_and_line_graph`.

## Left

- Origin's interactive profile lines and panels.
- Arbitrary (slanted) cuts.
- Profiles of XYZ contours.
