# T116: Plot > Statistics > 2D Kernel Density

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `kde2d`).
**Size:** M. **Source:** greyed inventory package B4 (Plot > Statistics > 2D Kernel Density...).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

A 2D kernel density plot: the bivariate Gaussian kernel density estimate of an X-Y point set drawn as
a colour map over the plane, with the points on top.

## Steps

1. Measure Origin 2016's dialog and plot with tools/origin-ref if automation works; otherwise build
   from Origin's documented behaviour and mark the section unmeasured (as T113 to T115).
2. Build it: the 2D density in `sheaf-calc` (bandwidth per axis from `kde::bandwidth`), a plot kind
   drawing the density grid with the shared colour map, a template and the menu item.
3. Tests, a graphing.md section, greyed inventory recount.

## Done when

The menu item plots, the inventory no longer lists it, and fmt, strict Clippy and the full tests
pass.
