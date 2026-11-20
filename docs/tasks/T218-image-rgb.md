# T218: Colour pictures, Color to Gray and RGB Split (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"), **not measured**.
**Size:** S. **Source:** greyed inventory package B14 ("Conversion: Color to Gray... · RGB Split...").
**Spec:** `docs/spec/matrices.md` 7h and 7i.

## Done

- `image_import::decode_picture` keeps the Red, Green and Blue channels of colour pictures. Import
  Image puts them in sheets.
- `matrix_image::rgb_sheets` and `matrix_image::color_to_gray` back Image > Conversion > Color to
  Gray and RGB Split, which make new books.
- Tests: `image_import::tests::a_colour_picture_keeps_three_channels_and_a_gray_one_keeps_one`,
  `matrix_image::tests::colour_sheets_are_found_by_name_and_turned_to_gray` and
  `matrix_window::a_colour_picture_imports_as_rgb_sheets_that_turn_gray_or_split`.

## Left

- Drawing a colour picture as one RGB image.
