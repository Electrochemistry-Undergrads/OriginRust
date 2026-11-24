# T355: Extract Data in Region

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", graphing 17.12). HANDOFF C (Origin 2024b "extract data
from regions on a graph"): a drawn rectangle's or ellipse's right-click menu has Extract Data in Region, which copies
the layer's data points inside the object (through the axis scales) to a new workbook, X, Y and source row per plot,
as one undo step. Not in Origin 2016, so the layout is Sheaf's own.
**Size:** S. **Spec:** graphing 17.12.

## Left

- Freeform (lasso) regions, several regions at once, stacked-plot offsets, deleting the points in the region (masking
  them: done in T413).

## Tests

- `crates/plot/src/region.rs`: rectangle through linear and log axes, ellipse corners, lines refused, empty region.
- `crates/app/src/app/ui_tests.rs`: `a_drawn_rectangle_extracts_the_data_points_inside_it` (menu entry only on
  boxes, the new workbook's columns, undo label).
