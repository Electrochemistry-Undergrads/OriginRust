# T226: Plot > Contour/Heat Map > Ternary Contour (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"), **not measured**.
**Size:** S. **Source:** greyed inventory package B14 ("Contour/Heat Map: Ternary Contour").
**Spec:** `docs/spec/matrices.md` 7k.

## Done

- `FieldSource::Ternary { a, b, c, z }` normalizes each point and grids it as scattered XYZ.
  `FieldTemplate::new_ternary_graph` and `Project::plot_ternary_contour` build the graph, and
  `Cmd::TernaryContour` takes four selected columns.
- Tests: `field_tests::a_ternary_contour_is_drawn_inside_the_triangle` and
  `matrix_window::ternary_contour_plots_four_selected_columns_in_a_ternary_layer`. The PNG was
  checked by eye.

## Left

- A smooth edge along the triangle's sides.
- Measuring Origin.
