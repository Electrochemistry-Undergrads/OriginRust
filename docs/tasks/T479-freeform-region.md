# T479: Freeform regions on graphs

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing 17.12). Origin 2024b's "extract data from regions on a graph"
with freeform regions; not in Origin 2016 (whose Tools bar has a Region drawing tool, not measured), so the rules are Sheaf's own.
**Size:** M. **Source:** gaps doc "Extract data from regions on a graph: freeform regions remain".

## What it does

- Tools bar > **Region** draws a closed freeform outline by dragging (a lasso); the new object is selected and the tool returns to
  the Pointer, one undo step.
- `ShapeKind::Region` keeps its outline as fractions of its rectangle, so it moves, resizes and aligns like a box; it is drawn
  with the object's line and optional fill and saved in `.sheaf` files.
- Extract / Mask / Unmask Data in Region and the All Regions entries work for it (even-odd point-in-polygon).

## Tests

- `crates/core/src/shape.rs`: `a_region_keeps_its_outline_through_moves_and_resizes`.
- `crates/plot/src/region.rs`: `a_freeform_region_takes_the_points_inside_its_outline`.
- `crates/app/src/app/ui_tests.rs`: `the_region_tool_draws_a_lasso_whose_points_can_be_extracted`.
