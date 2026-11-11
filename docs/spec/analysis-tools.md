# Analysis tools (Origin 2016 behaviour)

Summary: this file records how the numerical analysis tools of Origin 2016 behave, as measured by
running Origin on inputs we generated (LabTalk scripts in `tools/origin-ref/`, run through COM) and,
for option lists and defaults, from the plain-text X-Function definitions. The Rust implementation
is `crates/calc` (package `sheaf-calc`); its tests in `crates/calc/tests/` hold the Origin numbers
as constants. Everything below is written in our own words. "Observed" means the rule was fitted
to numbers returned by Origin in this project (and the tests check it); "inferred" means we chose a
rule that fits the observations but other rules might too; "unknown" is open.

Conventions: rows are 1-based; missing values are NaN; all comparisons in the tests are relative
(1e-9 to 1e-12 unless noted). Reference scripts build closed-form inputs so the Rust test
regenerates the same data.

Status by tool: integrate, differentiate, normalize, interpolate (linear, cubic spline, Akima),
baseline, peak finding (local maximum) and statistics are done and checked. Smooth: adjacent
averaging, Savitzky-Golay and percentile filter are done and checked; the FFT filter is not
implemented. Cubic B-spline interpolation is not implemented.

## 1. Integrate (`integ1`)

Options (defaults): `type` = `math` or `abs` (math); `baseline` 0/1 (0), a straight line through the
first and last point of the range is subtracted first; the input is an XY range, optionally a row
range such as `col(B)[11:61]`. Outputs: cumulative curve (`oy`), `area`, `x1`, `x2`, `i1`, `i2`,
`y0`, `x0`, `dx`.

Algorithm (observed unless marked):

* Valid points are the rows of the range where both x and y are present. A missing point is simply
  left out; its neighbours are joined. The cumulative column has the same length as the range, with
  a missing value at the skipped row, and is 0 at the first valid point. If the input is a row
  range, Origin writes the cumulative curve compactly from row 1 of the output column.
* Math area: trapezoids `(x[j]-x[j-1]) (y[j]+y[j-1]) / 2`, signed, so descending X gives a negative
  area and a negative `dx`.
* Absolute area: the integral of the absolute value of the piecewise-linear curve. Where a
  segment changes sign the zero crossing is found exactly: the two triangles give
  `h (ya^2 + yb^2) / (2 (|ya| + |yb|))`. This is not the trapezoid rule on `|y|`. The cumulative
  curve uses the same pieces. It uses `|h|`, so the result is positive for descending X.
* Baseline: `y - (ya + (yb - ya)(x - xa)/(xb - xa))` with `(xa, ya)`, `(xb, yb)` the first and last
  valid point; everything below (area, cumulative, peak figures) uses the corrected curve.
* `x1`, `x2`: X of the first and last valid point of the range. `i1` is the first row of the range
  and `i2 = i1 + (number of valid points) - 1` (so a missing value inside lowers `i2`; with a
  missing value and a row range together this is inferred, not observed).
* `y0`, `x0`: the point of largest magnitude `|y|` (after the baseline), with its sign; a tie (a flat
  top, or equal positive and negative extremes) goes to the first row.
* `dx`, the width at half height. Level `lev = y0 / 2`. The direction of the peak is the sign of
  the area computed with `|dx|` steps (positive: a point is "inside" when `y >= lev`; negative:
  when `y <= lev`). Left edge: scan from the first valid point upward for the first inside point
  `j`; if `j` is the first point the edge is `x[0]`, otherwise the edge is the linear
  interpolation of the level between `j-1` and `j`. Right edge: scan from the last point downward
  for the first inside point `j`; if it is not the last point, interpolate between `j` and `j+1`;
  if the last point itself is inside, the line from the peak point to the last point is continued to
  the level (`xr = x0 + (lev - y0)(xlast - x0)/(ylast - y0)`; if `ylast == y0`, `xlast`). So the
  outermost crossings are used (a second lobe above half height widens the result), and the two
  ends are treated differently when the curve does not fall below half height (left: clipped,
  right: extrapolated). `dx = xr - xl`. When the peak is the last point Origin returned the left
  crossing's X itself as the width (observed in five cases, a quirk; implemented as `xr = 2 xl`).
  When the area sign disagrees with the sign of `y0` (for example a positive peak on a negative
  offset, or a negative lobe that dominates the area) the "inside" test is mirrored, which makes
  both ends fall into the clipped / extrapolated branches; this is observed, the reason is not
  known. The reference suites (1 to 6) cover: gaussian, asymmetric, offset, negative, baseline,
  range, descending, uneven X, plateau, two lobes, peak at either end.
* Fewer than two valid points: our function returns `None` (Origin's behaviour is not tested).

Charge from current vs. time is the signed math area of `(time, current)`; with seconds and amps
the result is in coulomb; the helper `charge` just calls the integrator.

Unknown: a range together with missing values (`i2`), behaviour for non-monotonic X, the width when
the area is exactly zero.

## 2. Differentiate (`differentiate`)

Options: `order` 1..9 (1); `smooth` 0/1 (0); `poly` 1..9 (2) and `npts` (20) for the smoothing
derivative.

Plain derivative (observed): slope of each segment `s[i] = (y[i+1]-y[i]) / (x[i+1]-x[i])`; the
result at an interior point is the plain mean `(s[i-1]+s[i])/2`, with no weighting by the spacing,
even for uneven X; at the first and last point it is the slope of the single adjacent segment.
Order n applies this n times (observed for 2 and 3). A missing point is dropped, the result there is
missing, and the neighbours are joined (observed).

Smoothing derivative (observed for order 1 with windows 7/poly 2 and 9/poly 3, and for order 2 with
9/3): for each point a polynomial of order `poly` is fitted by least squares to the `2*(npts/2)+1`
points around it (the window is slid inward at the ends so it always lies inside the data), and the
derivative of the fit at that point is taken. Quirk for order 2: the points of the outer half windows
(the first and last `npts/2` points) are exactly twice the true second derivative of the fit
(observed once; whether the same holds for higher orders is unknown, we only double order 2). The
default `npts` of 20 gives a window of 21 points (inferred from the even-window rule found for
`smooth`).

## 3. Smooth (`smooth`)

Options (defaults): `method` `sg` (Savitzky-Golay); `npts` 5; `polyorder` 2; `boundary` `none`;
`weight` 0 (adjacent averaging); `percent` 50. Window: `2*(npts/2)+1` points, so an even `npts`
acts like the next odd number (observed for 6 with averaging and Savitzky-Golay).

* Missing values are removed, the rest is smoothed as one series, and the missing rows are put back
  as missing (observed for all three methods).
* Adjacent averaging: mean of the window. Weighted: weights `1 - (j/(h+1))^2` for offset `j`, half
  width `h` (5/9, 8/9, 1 for `h = 2`); observed by solving for the weights.
* Savitzky-Golay: local polynomial fit evaluated at the centre.
* Percentile: the window values sorted, element number `ceil(m p / 100)` (1-based) of the `m` sorted
  values, no interpolation. `floor(m p / 100)` (0-based) fits the same data; they differ only when
  `m p / 100` is a whole number (unknown which one Origin uses).
* Boundary `none`: averaging and percentile shrink the window symmetrically (the first and last point
  stay as they are); Savitzky-Golay fits the first (or last) full window and evaluates it at the edge
  point. `reflect`: mirror including the end point (`y[-1] = y[0]`). `repeat`: the end value.
  `periodic`: wrap around. `extrapolate`: the straight line through the two end points. For the
  padded modes the window is full everywhere. All observed.
* FFT filter: the `cutoff` output equals `5/npts` for a 0.1 spacing (that is `1/(2 dx npts)`), and a
  larger `npts` smooths more. A pure sine with a whole number of periods came back with a large smooth
  distortion at both ends (about 0.3 of the amplitude), so the filter is not a simple circular
  low-pass; we did not find the rule (padding / baseline handling) and did not implement it.

## 4. Normalize (`normalize`)

Methods (all observed; statistics over valid points; missing stays missing): `specify` divide by a
value; `range` `(y-min)/(max-min)`; `range100` the same times 100; `snd` `(y-mean)/sd` with the sample
sd; `max`, `min`, `mean`, `median`, `sd` (sample), `norm` (Euclidean norm), `mode`, `sum`: divide by
that statistic. `mode` is all missing when no value occurs twice (observed); with ties we take the
smallest value (unknown). Division by a negative `min` flips the sign (observed). Not implemented:
`ref` (reference column) and `cell`.

## 5. Interpolate / extrapolate (`interp1`, `interp1xy`)

`interp1 ix iy method option boundary sf ox coef`: methods `linear`, `spline`, `bspline`, `akima`;
`option` `extrap` (default), `miss`, `repeat`; spline `boundary` `natural` or `notaknot` (the
X-Function default is not-a-knot). `interp1xy` interpolates onto `npts` (100) equally spaced points
between `xmin` and `xmax` (the data range): the grid is `xmin + (xmax-xmin) i/(npts-1)`; outside the
data it continues the end segment.

Observed: linear extrapolation is the straight line of the end segment; `miss` gives missing outside
`[x_first, x_last]`; `repeat` gives the first or last Y. Cubic spline (natural and not-a-knot) matches
the textbook splines, and outside the data the end cubic continues. Akima: the interior slope at point
`i` (from the third to the third-from-last point) is the classic Akima weighting of the four
neighbouring chord slopes, `(|m[i+1]-m[i]| m[i-1] + |m[i-1]-m[i-2]| m[i]) / (sum of the weights)`.
Origin's end rule differs from the classic one: the slope at the first and last point is the
derivative of the parabola through the three outer points, and at the second and second-to-last
point it is the spacing-weighted mean `(m0 h1 + m1 h0)/(h0 + h1)`; the curve is the cubic Hermite
polynomial and outside the data the end cubic continues. Fits to 1e-14.

The cubic B-spline method is a smoothing spline: with `sf = 0` it did not reproduce the data points
(differences of 0.1 and more) and the `sf` value did not change the result of `interp1` in our run;
not understood, not implemented.

Unsorted X: we sort and drop pairs with a missing value (not compared with Origin).

## 6. Baseline subtraction

Our own tool; Origin does this inside the Peak Analyzer. Constant: `y - c`. Line through two points
`(x1, y1)`, `(x2, y2)`: `y - (y1 + slope (x - x1))`. Anchors: the baseline is the interpolation of the
anchor points (linear, cubic spline or Akima, using section 5) and continues beyond the outer anchors
like the extrapolation of `interp1`. Checked through the interpolation numbers only.

## 7. Peak finding (`pkfind`, method `max`)

Options (defaults): `method` `max` (local maximum); `npts` 2; `dir` both (the default is the third
choice, `b`); `filter` `h` (height) with `value` 0; filters `none`, `h`, `num`. Outputs: the 1-based
row of each peak and its X and Y (`ocenter`, `ocenter_x`, `ocenter_y`).

Observed rules: the window is `npts` points on each side, and the whole window must lie in the data
(the first and last `npts` rows are never peaks). A positive peak must be strictly above the left
neighbours and not below the right ones (a flat top reports its first point; the same for flat
bottoms), and its value must be above 0; a negative peak is the mirror image and must be below 0, so
the ripple minima of a positive curve are not negative peaks. Height filter with value `v`: keep peaks
with `|y| >= v`; with `v = 0` the threshold is 20 % of the largest `|y|` of the data (fits two data
sets; whether the maximum is of `|y|` or of `y` was not separated). Number filter: the `value`
largest `|y|`, reported in row order. If no peak exists Origin stops the script with an error; our
function returns an empty result. The `smooth` option (smoothing before the search) is not
implemented. The window search and derivative methods are not implemented.

## 8. Descriptive statistics (`stats`, `colstats`)

Observed (all fit to 1e-11): `n` and `missing` count valid and NaN points; sample variance and standard
deviation (divisor n-1); `sem = sd/sqrt(n)`; `cv = sd/mean`; skewness is the bias-corrected
`n/((n-1)(n-2)) sum((x-mean)/sd)^3`; kurtosis the bias-corrected excess kurtosis
`n(n+1)/((n-1)(n-2)(n-3)) sum((x-mean)/sd)^4 - 3 (n-1)^2/((n-2)(n-3))`. With too few points the value
is missing: one point gives missing sd, variance, sem, cv, skewness, kurtosis; two points give missing
skewness and kurtosis; three give missing kurtosis. `iqr = q3 - q1`.

Quartiles are not one of the textbook interpolation rules. Observed position (1-based, in the sorted
data) of `q1` for `n = 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,20,21` is
`1,1,1,1.5,2,2,2,2.5,3,3,3,3.5,4,4,4,4.5,5.5,6`, and the position of `q3` is `n + 1 - pos(q1)`. This
is the Hazen position `n p + 1/2`, kept when it is a whole or a half number (a half position averages
the two neighbours) and rounded to the nearest whole position otherwise (`.25` goes down, `.75` up).
Median: the usual one. The `stats` X-function returns mean, sd, n, min, max, sum, missing for the same
numbers (checked for two columns).

## Files

Reference scripts (all in `tools/origin-ref/`): `integ-suite1..6`, `diff-suite1`, `smooth-suite1`,
`smooth-fftprobe`, `normalize-suite1`, `interp-suite1`, `pkfind-suite1`, `pkfind-suite2`,
`colstats-suite1..3`. Run with `run-labtalk-data.ps1`. Raw dumps (git-ignored):
`re-workspace/scratch/sheaf-calc/*.json`.