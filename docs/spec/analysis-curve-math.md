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

Input: two or more curves. Output, as Origin's (section 4): a new sheet `AverageResult1` with "Averaged X",
"Averaged Y" (comment "Average of B, D", the unit of the first curve) and, when ticked, "Standard Deviation"
(a Y-error column of the mean), "Standard Error", "Count", "Minimum" and "Maximum", all off by default.

Choices (defaults in bold, Origin's names):

- Averaged X: Same as Source X (row by row for a shared X, below; otherwise every X value of the curves,
  sorted, no repeats, over the range every curve covers), Common X Range (evenly spaced over the range every
  curve covers), **Full X Range** (evenly spaced from the smallest start to the largest end, each curve
  continued along its end segments), and Sheaf's own: the X values of one chosen curve (counted from 1), or
  `--x any` on the command line (the full range without continuation; a curve counts only where it has data).
- Number of points of the evenly spaced choices: **Auto** (five times the mean number of points per curve,
  at most 1000), or a number.
- Interpolation: **linear**, cubic spline, Akima spline (the same interpolators as Interpolate/Extrapolate;
  splines continue their end cubic in Full X Range and are missing outside a curve's range otherwise).
- Standard deviation: the sample standard deviation (divisor n - 1), missing where fewer than two curves
  contribute; the standard error is SD / sqrt(n). Mean and SD ignore curves that have no value at that X.

Special case, the usual one for repeated scans: if the curves share their X column, they are averaged row by
row in their original order with no interpolation, and the X values may then turn back (a whole cyclic sweep
in one column works). Whether X is shared is decided from the X values as they are in the sheet, row by row: at
each row the curves that have that row must agree on X. A curve with a missing Y at a row, or a column that
ends early, does not hide the shared X; it just has no value at that row, so the count there is lower. Rows
where no curve has an X or a Y are left out.

Otherwise each curve is made increasing in X: a decreasing curve is reversed, and a curve whose X goes up and
then down is sorted by X, as Origin does (measured in Origin 2016 on 2026-11-21, T325,
`re-workspace/scratch/t322/av.lines.txt`: X 0, 1, 2, 3, 2, 1, 0 with Y 0 to 6 against a flat 1 gave the mean
2 at X 0, 1, 2, 3, so the turning curve counted 3 at each X, the mean of its points there). Equal X values
then become their mean. (Before T325 such a curve was refused.) Fewer than two curves, a curve
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

## 4. Measured in Origin (2026-11-13)

Runs of `avecurves`, `intersect` and `polyarea` through `tools/origin-ref/run-labtalk-lines.ps1`
(`re-workspace/scratch/curvemath2/cm*.lines.txt` and `.out.txt`).

Average Multiple Curves (`avecurves`):

- Default Averaged X is Full X Range: evenly spaced X from the smallest start to the largest end of all
  curves, each curve continued past its ends by the straight line of its end segment, so every curve
  counts at every X (a curve 0..5 with y = 2x and a flat curve 0.5..6.5 gave, at X = 6.5, a mean of 7
  from 13 and 1). Common X Range spaces the X values over the range every curve covers. Same as Source
  X needs curves with the same X (two curves with different X were refused).
- Auto number of points: five times the mean number of points per curve, rounded down, at most 1000
  (3 and 3 points gave 15, 7 and 20 gave 65, 6 and 7 gave 30, 1000 and 400 gave 1000, three curves of 250
  gave 1000). Curves with exactly the same X are also regridded (6 and 6 points gave 30 rows).
- The standard deviation uses n - 1; the standard error is SD / sqrt(n).
- Output: a new sheet `AverageResult1` in the input book with "Averaged X", "Averaged Y" (comment "Average
  of B, D"), and when ticked "Standard Deviation", "Standard Error", "Count", "Minimum", "Maximum"; all five
  are off by default.

Sheaf copies these (`CommonX::Auto`, `XRange::Full`, `AverageSpec` with Origin's names and defaults);
Sheaf's row-by-row average of a shared X is offered as Same as Source X, and Sheaf keeps its extra choices
(the X values of one curve; any curve's range without continuation, `--x any`).

Intersect (`intersect`, Method Linear, Sampling Points 1000):

- A crossing is found exactly (curves y = 0, 3, 0, 3 and the line from (0, 3) to (3, 0) gave X = 0.75, 1.5,
  2.25, in increasing X, the same with the curves swapped).
- A touch that does not fall on one of the 1000 sample points is not reported: a zigzag 0, 2, 0, 2, 0 on X
  0..4 against y = 2 gave nothing, against y = 0 gave only its two ends (0, 0) and (4, 0) and not the valley
  at X = 2. Sheaf reports no touching point unless asked, so it gives nothing in the first case and nothing
  in the second (Origin's two end points are not copied).
- Output columns "Intersection X1" (X) and "Intersection Y1" (Y) at the end of the input sheet, without
  comments; a second run adds "Intersection X2" and "Intersection Y2" (measured again on 2026-11-21, T322,
  `re-workspace/scratch/t322`). Sheaf writes them there since T322 for the default output (Next to Input);
  "New sheet" in the window's Output and the touching-points option (which adds a text Type column) still
  write a new sheet.
- `polyarea` writes nothing to the sheet: its result is a number (`polyarea.area` read 12 for the 4 x 3
  rectangle). Sheaf's table of area, centroid and perimeter on a result sheet is its own addition.

Polygon Area (`polyarea`, Area Type Mathematical Area or Absolute Area (default)):

- Mathematical area is the signed shoelace area: +12 for (0,0) (4,0) (4,3) (0,3), -12 for the same
  rectangle clockwise; a last point equal to the first changes nothing. Absolute area is the size of the
  signed area, so a bow-tie (0,0) (2,2) (2,0) (0,2) gives 0 for both. Sheaf's Area is the absolute and its
  Signed Area the mathematical value, as measured.

## Unmeasured

Each item is what Origin has to be asked; Sheaf's current choice is stated above.

1-6. Average: measured, see section 4. Still open: spline interpolation next to a curve's end, Origin's
   Custom X choice and Follow Curve Trace (an X that turns back is sorted, measured in T325), and what Same as Source X does with X values that are not shared.
7. Intersect: more or fewer than two input curves (Origin may accept a reference curve or a range of several);
   the order of the output points; whether touching points and coincident stretches are reported at all, and
   how (we report the two ends of a stretch); the treatment of a point that is a vertex of both curves.
8-9. Intersect output names and Polygon Area sign and self-crossing rule: measured, section 4. Still open:
   whether Origin reports a centroid and perimeter (its X-Function outputs only the area).
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
