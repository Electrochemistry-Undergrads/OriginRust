# T349: CV Charge with the peak's feet picked on the graph

**Status:** done 2026-11-21 (Claude Windows "claude:gmail", analysis-tools 21). The parity row "CV charge with
sloped baseline" (P0, the owner's cyclic voltammetry work) said the feet could not be picked on the graph. With a
graph active, Analysis > Mathematics > CV Charge now turns the Data Reader on: a double-click on a data point of the
active plot (or Enter) picks a foot, the scan rate is set in the small window, and Calculate runs CV Charge over the
rows between the feet with the end-point baseline, so the line through the feet is the sloped baseline. One undo
step; the results sheet is the one of T296. From a workbook the typed window stays.
**Size:** S. **Spec:** analysis-tools 21.

## Left

- Drawing the baseline and the integrated area on the graph; free (not snapped) feet.

## Tests

- `crates/app/src/app/ui_tests.rs`: `cv_charge_on_a_graph_picks_the_feet_of_the_peak` (feet picked in either
  order, charge = trapezoid sum above the line through the feet, first row, undo label).
