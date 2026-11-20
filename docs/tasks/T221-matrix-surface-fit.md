# T221: Nonlinear Surface Fit on a matrix (parity "Surface fit on a matrix")

**Status:** done 2026-11-20 (Claude Windows "obj-align"), **not measured**.
**Size:** S. **Spec:** `docs/spec/matrices.md` 7j.

## Done

- `MatrixCmd::SurfaceFit`: the matrix bar's Analysis > Fitting > Nonlinear Surface Fit... converts the
  active sheet to XYZ columns, then opens the existing surface-fit window on them.
- Test: `surface_fit::a_matrix_is_surface_fitted_through_xyz_columns`.

## Left

- Fitting the matrix directly, as Origin does, so the results are written back next to the matrix.
