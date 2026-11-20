# T225: Plot > Contour/Heat Map > Polar Contour (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"), **not measured**.
**Size:** S. **Source:** greyed inventory package B14 ("Polar Contour theta(X) r(Y)" and
"r(X) theta(Y)").
**Spec:** `docs/spec/matrices.md` 7k.

## Done

- `FieldTemplate::new_polar_graph` and `Project::plot_xyz_polar`: a contour field in the polar layer.
  The field renderer already maps vertices through `LayerGeometry::data_to_page`, so the contour
  follows the polar layer. The two menu items send `Cmd::PolarContour`.
- Tests: `field_tests::a_polar_contour_is_drawn_inside_the_circle` and
  `matrix_window::polar_contour_items_plot_three_selected_columns_in_a_polar_layer`. The PNG was
  checked by eye: concentric bands.

## Left

- Wrapping the triangulation round 360 degrees.
- Curved band edges.
- Ternary Contour.
- Measuring Origin.
