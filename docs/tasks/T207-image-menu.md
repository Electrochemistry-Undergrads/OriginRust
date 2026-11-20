# T207: The Image menu on a matrix (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"). The formulas are Sheaf's,
**not measured**.
**Size:** M. **Source:** greyed inventory package B14 (Image menu: Adjustments, Arithmetic, Conversion,
Geometric Transform, Spatial Filters).
**Spec:** `docs/spec/matrices.md` 7i.

## Done

- `sheaf_core::matrix_image::ImageOp` holds Brightness, Contrast, Gamma, Math, To Binary, Rotate,
  Flip, Smooth, Sharpen and Edge Detection.
- The Image menu in the matrix bar and the workbook bar sends `Cmd::Image`. The windows are
  `ImageForm` in `dialogs/matrix_ops.rs`. Each operation is applied in place as one undo step.
- Tests: `matrix_image::tests` (two) and
  `matrix_window::the_image_menu_adjusts_and_filters_the_active_matrix_in_place`.

## Left

- Levels, Subtract (one image from another), Color to Gray, RGB Split, Crop and Resize.
- Colour images.
- Measuring Origin's formulas.
