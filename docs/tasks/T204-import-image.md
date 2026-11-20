# T204: File > Import > Image into a matrix book (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"), **not measured**.
**Size:** S. **Source:** greyed inventory package B14 ("Import: Image...").
**Spec:** `docs/spec/matrices.md` 7h.

## Done

- `sheaf_io::image_import` decodes PNG, JPEG, GIF and BMP to gray levels. Its `image` dependency is
  the version already in the lockfile, with decoders only. `matrix_view::import_image` makes the
  matrix book, and File > Import > Image... is live.
- Tests: `image_import::tests` and `matrix_window::an_imported_picture_becomes_a_matrix_of_gray_levels`.

## Left

- Colour (RGB) images, TIFF, and the Image menu (Adjustments, Conversion, Geometric Transform,
  Spatial Filters).
