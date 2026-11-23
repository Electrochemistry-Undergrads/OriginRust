# T385: Measure the Smith Chart against Origin 2016

**Status:** done 2026-11-22 (Claude Windows "claude:work", graphing 37). Origin 2016's Plot > Specialized > Smith
Chart opens no window and takes X/Y as a normalised resistance/reactance. It draws a black line over a grid of
solid gray circles and arcs at 0.2, 0.5, 1, 2, 5 and 20, with dotted light-gray ones halfway between them in
gamma. The resistances are labelled along the axis and the reactances ("0.5j", "-2.0j") round the rim. Sheaf
now draws the same grid and labels, and a line plot.
**Size:** S. **Follows:** T115. **Spec:** graphing 37.

## Left

- Origin's page (6848 x 4784) and layer proportions; the legend at the right.

## Tests

- `crates/plot/src/smith.rs`: `a_smith_layer_draws_the_reflection_coefficients_over_its_grid` (grid count, labels).
