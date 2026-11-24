# T413: Mask data in a region

**Status:** done 2026-11-24 (Claude Windows "claude:1", graphing 17.12), not measured (Origin 2024b feature). A drawn
rectangle's or ellipse's menu has Mask Data in Region and Unmask Data in Region: the Y cells of the layer's data points
inside the object (found as for Extract Data in Region, T355) are masked or unmasked, one undo step each.
**Size:** S. **Source:** HANDOFF C, the rest of T355.

## Tests

- `crates/app/src/app/ui_tests.rs`: `a_drawn_rectangle_extracts_the_data_points_inside_it` (mask, then unmask, the
  four points inside; undo labels).

## Left

- Freeform regions, several regions at once, deleting the points.
