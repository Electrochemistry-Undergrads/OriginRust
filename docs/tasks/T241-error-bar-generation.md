# T241: Add Error Bars generates errors (Percent, Standard Deviation, Square Root)

**Status:** done 2026-11-20 (Claude Windows "claude:3"), **not measured** (Origin 2016 is not usable on
this computer now). Graph > Add Error Bars' Source of Errors gets Origin's three generated sources:
Percent of Data (%), Standard Deviation of Data with Scaling Factor, Square Root of Data, besides
Sheaf's existing-column chooser. OK writes a new Y Error column right of the plot's Y column (comment
naming the source, the Set Column Values formula kept) and sets it as the plot's Y errors, in one
undo step, as OriginLab's help describes.
**Size:** S. **Source:** feature-parity "Error bars" (P0) gap.
**Spec:** `docs/spec/graphing.md` 14.13a.

## Tests

- `crates/app/src/app/ui_tests/error_bars.rs`:
  `error_bars_are_generated_into_a_new_y_error_column_as_origin_does` (Percent 5 by default: the
  column right of Y, designation, comment, formula, values, the plot's link; Standard Deviation x 2
  on the other plot; one undo step). The chooser's tests now pick Existing Column first.

## Left

- Measure Origin's dialog: defaults, the new column's name and long name, Percent of negative
  values, rounding; X errors; the Plus/Minus direction at creation.
