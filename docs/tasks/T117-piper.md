# T117: Plot > Specialized > Piper

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `piper`).
**Size:** M. **Source:** greyed inventory package B4 (Plot > Specialized > Piper), the last unclaimed
B4 item.
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

A Piper (trilinear) diagram of water chemistry: the cation triangle (Ca, Mg, Na+K), the anion
triangle (Cl, SO4, HCO3+CO3) and the diamond between them, each sample drawn in all three.

## Steps

1. Measure Origin 2016's Piper with tools/origin-ref if automation works; otherwise build from the
   standard Piper construction and Origin's documented behaviour and mark the section unmeasured.
2. Build it: the column roles, the projection into the triangles and the diamond, the grid and corner
   labels, a template and the menu item.
3. Tests, a graphing.md section, greyed inventory recount.

## Done when

The menu item plots, the inventory no longer lists it, and fmt, strict Clippy and the full tests
pass.
