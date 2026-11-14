# Curve mathematics: Average Multiple Curves, Intersect, Polygon Area

Summary: how Sheaf's three Analysis > Mathematics curve tools behave. **Nothing here was measured in
Origin.** The build machine runs macOS and has no Origin; the rules come from the generally known
mathematics (shoelace formula, segment intersection, linear interpolation) and from the paraphrased
public description of the X-functions `avecurves`, `intersect` and `polyarea`. Every rule that depends on
what Origin does is listed in "Unmeasured" at the end; those are the checks for a run of
`tools/origin-ref` on the Windows machine. This file is written in our own words and does not copy Origin's
help text. Task: `docs/tasks/T22-curve-mathematics.md`.

Code: numerics in `crates/calc/src/curves.rs` (tests with hand-worked values), tools in
`crates/core/src/analysis/curves.rs`, recalculation links in `crates/core/src/analysis_link.rs`, option
controls in `crates/app/src/dialogs/math_curves.rs`, command line in `crates/cli/src/analyze.rs`
(`sheaf analyze average | intersect | polygon-area`). Menu: Analysis > Mathematics (`ui-command-map.md`);
X-function names in `xfunction-index.md`.

Common rules: a point with a missing or non-finite X or Y is dropped before anything else. The tools work on
the chosen Y columns and the X column each one uses; the row range of the analysis window (all rows, a row
interval, or an X interval) applies to all chosen columns. All three can be run with Recalculate Auto or
Manual (the result follows its input, replaced in place, in the same undo step as the edit).

## 1. Average Multiple Curves (`avecurves`)

Input: two or more curves. Output (a new sheet by default): X, `Mean` (Y), `SD` (a Y-error column of the
mean, optional) and `N` (the number of curves at that X, optional, a Disregard column). The long name of the
mean column is "Mean" with the unit of the first curve; its comment lists the averaged columns.

Choices (defaults in bold):

- X values: **every X value of the curves** (union of the points inside the range, sorted, no repeats),
  evenly spaced (N points, default 100, at least 2), or the X values of one chosen curve (counted from 1 among
  the input columns) that lie in the range.
- X range: **covered by every curve** (largest start to smallest end) or covered by any curve (smallest start
  to largest end; a curve is never extrapolated, so at the ends fewer curves contribute and `N` says how many).
- Interpolation: **linear**, cubic spline, Akima spline (the same interpolators as Interpolate/Extrapolate;
  splines return missing outside a curve's own range).
- Standard deviation: the sample standard deviation (divisor n - 1), missing where fewer than two curves
  contribute. Mean and SD ignore curves that have no value at that X.

Special case, the usual one for repeated scans: if the curves share their X column, they are averaged row by
row in their original order with no interpolation, and the X values may then turn back (a whole cyclic sweep
in one column works). Whether X is shared is decided from the X values as they are in the sheet, row by row: at
each row the curves that have that row must agree on X. A curve with a missing Y at a row, or a column that
ends early, does not hide the shared X; it just has no value at that row, so the count there is lower. Rows
where no curve has an X or a Y are left out.

Otherwise each curve's X must be monotonic: increasing, or decreasing (reversed internally). Neighbouring equal
X values become their mean. A curve whose X goes up and then down is refused with a message that names the
curve and the point where it turns back; the user chooses the rows of one sweep. Fewer than two curves, a curve
with fewer than two different X values, curves with no common X (intersection range), a missing reference
curve and fewer than two points are refused with a message.

## 2. Intersect (`intersect`)

Input: exactly two curves (anything else is refused). Each is a polyline through its points in row order; the
curves need not be functions of X, so the forward and reverse sweep of a cyclic voltammogram, or a curve and a
reference, work.

Output: X of each meeting point (long name "Intersection X (name of the X column)") and Y (long name
"Intersection Y (name of the first curve)"), in order along the first curve. The note says how many points were
found, or that the curves do not meet. All pairs of segments are tested (cost: product of the lengths, with a
bounding-box rejection first).

- Crossing: the first curve passes from one side of the second to the other. Listed always.
- Touching: the curves meet but the first does not pass to the other side, including the two ends of a stretch
  where they coincide (a coincident stretch is reported by its two ends only, however many vertices lie inside it, and whatever
  happens after it). Listed only
  when "also list the points where the curves touch without crossing" is on; a third column "Type" then says
  Crossing or Touching.
- A meeting point that is a vertex of either curve is reported once.
- Parallel and disjoint curves give no points and no error.

## 3. Polygon Area (`polyarea`)

Input: one or more curves; each is taken as a closed polygon, the last point joined to the first (a last point
equal to the first is not counted twice). Fewer than three different points in a column refuses the whole run, with a message that names that column (no
partial table).

Output: a table with one row per input column: `Area` (absolute value), `Signed area` (positive when the curve
runs counterclockwise with Y up), `Centroid X`, `Centroid Y`, `Perimeter` (length of the closed boundary),
`Self-intersecting` (0 or 1) and `Points`. Units: area is the Y unit times the X unit; centroid and perimeter
use X and Y units. The centroid is missing when the signed area is zero (also for a figure eight whose lobes
cancel).

Formula: signed area = 1/2 times the sum of (x_i y_(i+1) - x_(i+1) y_i) around the closed list; the centroid is
the sum of (x_i + x_(i+1)) times that cross term over 6 times the signed area, and the same with y. Sums are
taken relative to the first point to keep precision. For a polygon that crosses itself the parts that run the
other way count as negative, so the area is not the total enclosed; a note says so.

## Unmeasured

Each item is what Origin has to be asked; Sheaf's current choice is stated above.

1. Average: which X values Origin uses by default, and whether "all X values" is a sorted union, the longest
   curve's X, or something else; the evenly spaced default point count.
2. Average: the exact interpolation Origin applies by default, and whether splines are offered with the same
   names; behaviour of spline interpolation next to a curve's end.
3. Average: how Origin treats curves that do not cover the whole X range (we use the intersection by default
   and never extrapolate), and the first/last point tolerance.
4. Average: whether the SD column uses divisor n - 1 (we do), whether Origin writes the standard error or the
   SD, and what it writes where fewer than two curves contribute.
5. Average: how Origin handles an X that turns back (we refuse), repeated X values (we take their mean) and
   curves with the same X (we average row by row).
6. Average: names, order, designations and units of the output columns, and whether the count column exists
   by default.
7. Intersect: more or fewer than two input curves (Origin may accept a reference curve or a range of several);
   the order of the output points; whether touching points and coincident stretches are reported at all, and
   how (we report the two ends of a stretch); the treatment of a point that is a vertex of both curves.
8. Intersect: the output columns, names, and whether X and Y are interleaved or on separate sheets.
9. Polygon Area: whether the area is signed or absolute and what Origin does with a self-crossing polygon; the
   closing rule (last point to first); whether the centroid and perimeter are reported; the area unit.
10. Polygon Area: input of a single polyline given as an X and a Y column versus a selection of several
    curves; behaviour with fewer than three points.
11. How Recalculate (Auto, Manual) treats these tools in Origin, and whether results go to a new sheet or next
    to the input by default.
12. The dialog layout and wording (ours is the shared analysis window frame with the options of each tool).

## Origin's X-Function interfaces (read 2026-11-13)

Read from the interface lists of Origin's X-Function files with `tools/origin-ref/xf-args.py` (names,
labels, defaults and choices only; the numbers of these tools are still to be measured):

- `avecurves`: Method Average (default) or Concatenate; Averaged X Same as Source X, Common X Range,
  Full X Range (the stored default), Custom or Follow Curve Trace; Tolerance for Common X Values 1e-8;
  Number of Points, X Minimum and X Maximum auto; Interpolate Linear (default), Spline or B-Spline; Sort X
  None, Ascending (default) or Descending; optional Std Dev, Std Err, N, Minimum, Maximum, SD times 2
  and SD times 3 columns (all off).
- `intersect`: two XY ranges; Method Linear (default), Cubic Spline or Cubic B-Spline; Sampling Points
  1000; the intersection points go to new X and Y columns.
- `levelcrossing` (Data Manipulation): Crossing Level 0; outputs the crossing X values, directions, X
  indices and Y crossing values; Nearest Type Any (default), Left or Right; Duration Points 1.
- `polyarea` has no argument list in its file that the script can read.
