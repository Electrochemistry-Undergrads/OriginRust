# T282: Image Resize and Crop measured against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
HANDOFF "Next candidates" B for T210. Origin 2016's `imgResize` defaults to nearest neighbour
(`interpolate:=normal`): a 3 x 3 image to 5 x 5 repeats cells 0 0 1 1 2, to 6 x 6 doubles each.
Sheaf's Resize was bilinear only; it now offers Nearest (the default, Origin's mapping) and
Bilinear. Origin's `resample` and `bicubic` are LEADTOOLS filters that fit no plain formula, so they
are not copied. `imgCrop` counts x and y from 0 at the top left; Sheaf's 1-based rows and columns
select the same cells.
**Size:** S. **Spec:** `docs/spec/matrices.md` 7i. Probe: `re-workspace/scratch/resize-measure/`.

## Tests

- `matrix_image::tests` (nearest 3 to 5), the matrix window UI test (bilinear).
