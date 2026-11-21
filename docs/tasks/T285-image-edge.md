# T285: Image Edge Detection measured against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
HANDOFF "Next candidates" B for T207. Origin 2016's `imgEdge` gives a binary edge map: a cell is
an edge when its filter response, in 0..255 levels, is above the threshold `t`. A threshold sweep
on steps of 10, 20, 40 and 80 levels showed the Sobel and Prewitt responses equal the step (the
gradient over the kernel weight, 4 and 3). Sheaf's Edge Detection gave the continuous Sobel
magnitude; it is now Origin's binary map with Sobel/Prewitt Horizontal, Vertical or both and a
threshold, in a window. Laplace and compass filters (Origin's default is Laplace horizontal) are not
built.
**Size:** S. **Spec:** `docs/spec/matrices.md` 7i. Probe: `re-workspace/scratch/edge-measure/`.

## Tests

- `matrix_image::tests::edge_detection_thresholds_match_origin_2016`, the spatial-filter test.
