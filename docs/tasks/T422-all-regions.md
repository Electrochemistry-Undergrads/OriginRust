# T422: Data in all regions

**Status:** done 2026-11-24 (Claude Windows "claude:1", graphing 17.12), not measured (Origin 2024b feature). With two
or more drawn rectangles or ellipses in a layer, their menu has Extract Data in All Regions, Mask Data in All Regions
and Unmask Data in All Regions: the points inside any of them, each once. `sheaf_plot::region::points_in_shapes`,
`region_shapes`.
**Size:** S. **Source:** HANDOFF C, the rest of T355 and T413.

## Tests

- `crates/plot/src/region.rs`: `points_inside_any_of_several_regions_are_counted_once`.
- `crates/app/src/app/ui_tests.rs`: `a_drawn_rectangle_extracts_the_data_points_inside_it` (the entries appear with a
  second region; mask and extract both regions).

## Left

- Freeform regions, deleting the points.
