# T210: Image menu Levels, Subtract, Crop, Resize (B14 slice, T207 follow-up)

**Status:** done 2026-11-20 (Claude Windows "obj-align"). The formulas are Sheaf's, **not measured**.
**Size:** S. **Source:** greyed inventory package B14.
**Spec:** `docs/spec/matrices.md` 7i.

## Done

- `ImageOp::Levels`, `ImageOp::Crop`, `ImageOp::Resize` and `matrix_image::subtract`, with their
  windows (`ImageForm`) and the menu items.
- Tests: `matrix_image::tests::levels_crop_resize_and_subtract` and
  `matrix_window::image_levels_crop_resize_and_subtract_change_the_active_matrix`.

## Left

- Color to Gray and RGB Split, which need colour images.
