# T284: Image > Spatial Filters > Median

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
Origin 2016's Image menu has a median filter (`imgMedian d:=`); Sheaf's Spatial Filters had Smooth,
Sharpen and Edge Detection only. Measured on a 5 x 5 image with an outlier: a d x d window, edge
cells repeated, even windows reaching one cell further forward, the lower middle value for an even
count, default d = 2. `ImageOp::Median(d)` reproduces every measured cell.
**Size:** S. **Spec:** `docs/spec/matrices.md` 7i. Probe: `re-workspace/scratch/median-measure/`.

## Tests

- `matrix_image::tests::median_matches_origin_2016`.
