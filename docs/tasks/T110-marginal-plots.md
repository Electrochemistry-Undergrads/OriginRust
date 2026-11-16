# T110: Marginal Histograms and Marginal Box Charts

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `marginal-plots`).
**Size:** M. **Source:** greyed inventory package B4 (Plot > Statistics > Marginal Histograms,
Marginal Box Charts).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

Origin 2016's marginal plots: a scatter of X and Y with the distribution of each drawn along its
axis, as histograms (Marginal Histograms) or box charts (Marginal Box Charts), in linked layers
beside the scatter.

## Steps

1. Measure both in Origin 2016 with tools/origin-ref: the columns they take, the layers and their
   frames, bins and box settings, colours, axes, titles and legends.
2. Build them as multi-layer templates, reusing the existing histogram and box plot kinds in layers
   linked to the scatter's X and Y scales.
3. Tests, a graphing.md section, greyed inventory recount.

## Done when

Both menu items plot as measured, the inventory no longer lists them, and fmt, strict Clippy and the
full tests pass.
