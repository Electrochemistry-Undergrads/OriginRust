# T201: Plot > Contour/Heat Map > Gray Scale Map (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"), **not measured**.
**Size:** S. **Source:** greyed inventory package B14 ("Contour/Heat Map: Gray Scale Map").
**Spec:** `docs/spec/matrices.md` 7g.

## Done

- `FieldTemplate::GrayScale`: heat-map cells in the black-to-white palette. It is in the
  Contour/Heat Map menus of matrix windows and workbooks, and in the command line's template list.
- Test: `field_tests::golden_gray_scale_map_of_a_matrix` (golden `matrix-grayscale.svg`).

## Left

- Measuring Origin's Gray Scale Map (levels, lines, scale) and matching it.
