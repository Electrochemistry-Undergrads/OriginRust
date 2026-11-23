# T401: Measure the Ternary Contour against Origin 2016

**Status:** done 2026-11-23 (Claude Windows "claude:work", matrices 7k). Origin 2016's Ternary Contour (columns X, Y, Z,
Z) has the Contour - Color Fill look: rainbow, level lines and 8 x 5 levels, as Sheaf's already had. Its colour scale
has a light gray box above (white for the plain contour), which Sheaf now copies (`FieldPlot::above_color`).
**Size:** S. **Follows:** T226. **Spec:** matrices 7k.

## Left

- Side labels in the data's units (Origin: 0 to 100 every 25 for percent data; Sheaf: 0 to 1) and the A, B, C side
  titles.

## Tests

- `crates/plot/src/field_tests.rs`: `a_ternary_contour_is_drawn_inside_the_triangle` (the gray box, the ticks).
