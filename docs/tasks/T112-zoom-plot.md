# T112: Plot > Specialized > Zoom

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `zoom-plot`).
**Size:** M. **Source:** greyed inventory package B4 (Plot > Specialized > Zoom).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

Origin 2016's Zoom graph: the whole curve in one layer with a marked region, and that region
enlarged in a second layer.

## Steps

1. Measure it in Origin 2016 with tools/origin-ref: the layers and their frames, the initial region,
   how the region is drawn, axes, titles and legends.
2. Build it as a two-layer template whose second layer shows the region of the first.
3. Tests, a graphing.md section, greyed inventory recount.

## Done when

The menu item plots as measured, the inventory no longer lists it, and fmt, strict Clippy and the
full tests pass.
