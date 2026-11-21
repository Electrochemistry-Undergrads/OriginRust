# T280: Color to Gray measured against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
HANDOFF "Next candidates" B for T218. Origin 2016's `imgC2gray` on an imported 2 x 2 PNG (red, green,
blue, (200, 100, 50)) gave levels 64, 159, 32 and 119: `round((2 R + 5 G + B) / 8)`. Sheaf used the
Rec. 709 luminance (0.2126, 0.7152, 0.0722); Image > Conversion > Color to Gray now uses Origin's
weights (without rounding). `img2m` also showed that the image's top-left pixel is matrix row 1,
column 1, as Sheaf's importer has it. RGB Split made a new book; its sheets were not read.
**Size:** S. **Spec:** `docs/spec/matrices.md` (Conversion). Probe: `re-workspace/scratch/gray-measure/`.

## Tests

- `matrix_image::tests::colour_sheets_are_found_by_name_and_turned_to_gray`, the matrix window UI test.
