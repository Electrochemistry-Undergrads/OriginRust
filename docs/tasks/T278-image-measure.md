# T278: Image menu measured against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), partly measured.
HANDOFF "Next candidates" B for T207/T210's Image menu. Origin 2016's `img*` X-functions work on 8-bit
images: a matrix goes through `m2img` (its range scaled to 0..255) and back through `img2m`
(16-bit). Measured on a 5 x 5 ramp: Brightness (adds a% of 255, saturating), Gamma (Sheaf's formula),
To Binary (a Low..High band), Flip and Rotate 90 (Sheaf already matched). Changed: Brightness is a
percentage of the range, clamped to it; To Binary has Low and High. Contrast and Sharpen follow
LEADTOOLS routines with no formula Sheaf could confirm, so they stay Sheaf's. Sheaf keeps working on
the numbers in place.
**Size:** S. **Spec:** `docs/spec/matrices.md` 7i. Probes: `re-workspace/scratch/image-measure/`.

## Tests

- `matrix_image::tests::adjustments_arithmetic_and_binary_work_cell_by_cell`,
  `matrix_window::the_image_menu_adjusts_and_filters_the_active_matrix_in_place`.
