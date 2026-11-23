# T399: Measure the Polar Contour against Origin 2016

**Status:** done 2026-11-23 (Claude Windows "claude:work", matrices 7k). Origin 2016's Plot > Contour/Heat Map >
Polar Contour theta(X)r(Y) draws a colour-fill contour without level lines. 0 degrees is at the top and the angles
run clockwise. The radius axis starts at the smallest radius, and the 8 x 5 levels use a reversed rainbow (blue
highest) with gray/black out-of-range boxes. The fill closes the wedge from the last angle back to 0. Sheaf's polar
contour now does all of this.
**Size:** S. **Follows:** T225. **Spec:** matrices 7k.

## Left

- Origin's polar grid lines over the contour and the colour scale title (the Z column name); r(X) theta(Y) not probed.

## Tests

- `crates/plot/src/field_tests.rs`: `a_polar_contour_is_drawn_inside_the_circle` (orientation, palette, lines, radius
  minimum, the closed grid).
