# Analysis tools (Origin behaviour)

Summary: this file records how the numerical analysis tools of Origin behave, as measured by
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
averaging, Savitzky-Golay, percentile filter and the FFT filter are done and checked. Cubic
B-spline interpolation: done and checked, both for a smoothing factor above 0 and for Origin's default
factor 0 (an automatic factor from the data, found 2026-11-14, see section 5).

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
  point. `reflect`: mirror including the end point (`y[-1] = y[0]`), except for the percentile
  filter, which mirrors without the first point at the start (`y[-1] = y[1]`) and with the last point
  at the end (`y[n] = y[n-1]`; measured at 50 %, 12.3). `repeat`: the end value.
  `periodic`: wrap around. `extrapolate`: the straight line through the two end points. For the
  padded modes the window is full everywhere. All observed.
* FFT filter (`method:=fft`; `npts` is not a window length but sets the cutoff). All rules were
  found by running Origin on unit impulses (which give the whole matrix of the linear filter) and on
  pseudo-random inputs, and reproduce 47 Origin outputs to 1e-12 (n from 2 to 5000, X spacing 0.037
  to 1, uneven and descending X, a missing value; `origin_smooth_fft.rs`). The filter is linear in y.
  1. (observed) Origin sorts the points by X, filters, and returns the values in the original row
     order. Missing values are removed and put back, as for the other methods.
  2. (observed) A straight baseline is subtracted before and added back after the transform. It is
     the line from the mean of the first `w` points, placed at the first X, to the mean of the last
     `w` points, placed at the last X, with `w = 1` for `n <= 10` and `w = floor(n / 100)` limited to
     10..100 for larger `n` (the cap of 100 was missed at first: Origin outputs at n = 20000 and
     50000 match only with `w` = 100, the other values of `w` between 1 and 5000 that were tried do
     not; n / 100 itself was confirmed up to n = 5000) (the script-only option `baseline` has the values `two_ends_1` (default, this
     rule), `two_ends` and `none`; only the default was measured).
  3. (observed) The rest goes through a plain FFT of length `n`: no padding to a power of two (the
     script option `pad` is 0 by default), no windowing in the time domain. Write `R` for the X range,
     `dx = R / (n - 1)`, `df = 1 / R` (bin spacing; note `R`, not `n dx`) and the cutoff
     `fc = 1 / (2 dx npts)` (the `cutoff` output of `smooth`, as measured earlier).
  4. (observed) Weighting per bin `k >= 1`: a cosine part (real part of the spectrum) is kept when
     `(k + 1) df <= fc` and a sine part (imaginary part) when `k df <= fc`; the sine part of bin 1 is
     always kept. A kept part is multiplied by `1 - (k df / fc)^2` (by 1 for `k = 1`). So the filter
     is a parabolic low-pass whose cosine side is cut one bin earlier than its sine side, which is
     why it is not a symmetric circulant filter. The mean (bin 0) is kept; the Nyquist bin of an even
     `n` is dropped. Even at the largest `npts` the result keeps the mean and the first sine term, so
     a line is not reproduced exactly (the baseline handles only its two ends).
  5. (observed) When `(k + 1) df` equals `fc` exactly, whether the bin is kept depends on floating
     point rounding in Origin. All 20 such cases we generated agree with computing `dx` as
     `(x_last - x_first) / (n - 1)`, `df = 1 / R`, `fc = 1 / dx / (2 npts)` and comparing
     `(k + 1) * df <= fc` in double precision (inferred: other expressions might agree on these
     20 cases too).
  6. (inferred) The explanation why bins differ in sine and cosine is unknown; the rule above is an
     exact description of the matrix of the filter for every input we tried, not a derivation.

* Lowess, Loess and binomial (`method:=lw`, `le`, `bm`; run 2026-11-13, 30 columns, scripts
  `smooth-local-suite1/2.labtalk.txt`, fixtures `origin_smooth_local.rs`, all to 1e-12). The script options of
  `smooth` are `iy method interval order hint weight prop span npts boundary polyorder percent cutoff pad filter
  baseline oy`.
  1. (observed) Lowess is a local straight-line fit, Loess a local parabola, both evaluated at the point, with
     tricube weights `(1 - u^3)^3`, `u = |x - x_i| / h`, `h` the largest distance among the `q` points nearest in X
     (the point itself included). There are no robustness iterations. `polyorder`, `weight` and `boundary` have no
     effect on either; for uneven X the fit uses the X values.
  2. (observed) `span` (default 0.1) is a proportion: `q = ceil(span n)` with `n` the number of valid points (10.1 gives
     11, 20.0 gives 20, 25.5 gives 26, 5.0 gives 5; span 1 uses all points; a span so small that `q` is not larger than
     the polynomial degree leaves the data unchanged). With `prop:=0` the window is `npts` points instead and `span`
     is ignored (`npts` 7 and 20 checked). `prop` defaults to 1 (a hidden option; the dialog shows Span). `interval`
     and `order` do not change Lowess.
  3. (observed) A missing Y or X is removed first, so `n` and the window count only valid points; the row comes back as
     missing.
  4. (observed) Binomial: the filter `1 2 1 / 4` applied `order` times (script option `order`, default 1; `npts` has no
     effect). With `boundary:=none` the first and last point stay as they are in every pass and the window of the
     other points is always three points. `repeat`, `periodic` and `extrap` pad as for the other methods; `reflect` here
     mirrors around the end point, `y[-1] = y[1]` (unlike adjacent averaging and Savitzky-Golay, where `y[-1] = y[0]`).
     Orders 1 to 50 and every boundary were checked.
  Sheaf: `SmoothMethod::Lowess/Loess { span: LocalSpan }`, `Binomial { order }`; dialog fields Span (Lowess, Loess) and
  Order (Binomial); `sheaf analyze smooth --method lowess|loess|binomial --span S --order N`.

## 4. Normalize (`normalize`)

Methods (all observed; statistics over valid points; missing stays missing): `specify` divide by a
value; `range` `(y-min)/(max-min)`; `range100` the same times 100; `snd` `(y-mean)/sd` with the sample
sd; `max`, `min`, `mean`, `median`, `sd` (sample), `norm` (Euclidean norm), `mode`, `sum`: divide by
that statistic. `mode` is all missing when no value occurs twice (observed); with ties we take the
smallest value (unknown). Division by a negative `min` flips the sign (observed).

`ref` and `cell` (T260 from OriginLab's X-Function page; measured in Origin 2016 on 2026-11-20, T269,
`re-workspace/scratch/t269/norm.out.txt`). `ref` (method 11, variables `refcol` and `type`) "normalize[s] the input data by
matching the statistic specified by the type variable to that of a reference column"; `type` offers
min, max, mean, median and sum (`type` 0 to 4, default 0 = min, measured). The page prints no
formula; Origin scales each column, `y * stat(ref) / stat(y)` (measured: A = 1, 2, 3, 4, 6 against
B = 2, 4, ..., 10 gives A times 2 for min and median, 10/6 for max, 1.875 for mean and sum), so its
statistic equals the reference's (a statistic of 0, or none, in the
reference or in a column is refused), and the new column takes the reference's units. `cell` (method
12, variable `cell`) divides by one reference cell (measured: A / B[2] = A / 4); an empty, masked or zero cell is refused. The
reference is read as analysis reads data (masked cells and filter-hidden rows left out). Both are
in the Normalize window (Method: "Match a statistic of a reference column", "Divide by a reference
cell", with the reference column, Statistic or Row) and `sheaf analyze normalize --method ref|cell
--ref COLUMN [--stat min|max|mean|median|sum] [--row N]` (same sheet); the statistic starts on min, as
Origin's `type`. They do not take part in
Recalculate Auto/Manual.

**Where the results go (T300, measured in Origin 2016 on 2026-11-21, `re-workspace/scratch/t299`).**
Origin's Normalize Columns (`rnormalize`) normalizes each column of the range (not each row): with
A = 1 2 3 4 (X), B = 2 4 6 0, C = 5 1 9 2 the default method ([0, 1], method 1) gives 0 1/3 2/3 1,
1/3 2/3 1 0 and 0.5 0 1 0.125. It appends the results at the end of the sheet, named Normalized1,
Normalized2, ... (a second run goes on after the highest number: Normalized3), each with the
designation of its source (the column from A is an X column) and the comment "<method> of <short
name>": method 0 "Divided by 1" (the value), 1 "Normalize to [0, 1]", 2 "Transfer to N(0, 1)", 3 to 10
"Divided by Max", "Min", "Mean", "Median", "SD", "Norm", "Mode", "Sum"; methods 11 and 12 failed in the
script. A constant column under [0, 1] gives an empty column; a column without a mode gives an empty
one. Origin's single-curve `normalize` also appends its column at the end, named "Normalized Y1". Sheaf's
Normalize Columns (and the graph's Normalize) writes as `rnormalize` does (`normalize_method_text`,
`next_normalized_index` in `crates/core/src/analysis.rs`); the reference and cell methods still write
beside their source.

## 5. Interpolate / extrapolate (`interp1`, `interp1xy`)

`interp1 ix iy method option boundary sf ox coef`: methods `linear`, `spline`, `bspline`, `akima`;
`option` `extrap` (default), `miss`, `repeat`; spline `boundary` `natural` or `notaknot` (the
X-Function default is not-a-knot). `interp1xy` interpolates onto `npts` (100) equally spaced points
between `xmin` and `xmax` (the data range): the grid is `xmin + (xmax-xmin) i/(npts-1)`; outside the
data it continues the end segment.

**Where the results go (T302, measured in Origin 2016 on 2026-11-21, `re-workspace/scratch/t302`).**
`interp1xy` writes one Y column per input at the end of the input's sheet, long name "Interpolated Y1"
(then Y2, ... after the highest number already there), comment "Linear interp of B" (methods 0 to 3:
"Linear", "Spline", "B-Spline", "Akima spline"), and no X column: the even grid is the column's
sampling interval (`wks.col3.xinit` 0, `xinc` 0.5 for 9 points over 0..4). `interp1` (onto the
values of a column) writes the same Y column at the end, with no sampling interval. Sheaf writes the
same since T302 (`interp_method_text`, `next_label_index` in `crates/core/src/analysis.rs`); before,
it wrote X and Y to a new sheet.

Observed: linear extrapolation is the straight line of the end segment; `miss` gives missing outside
`[x_first, x_last]`; `repeat` gives the first or last Y. Cubic spline (natural and not-a-knot) matches
the textbook splines, and outside the data the end cubic continues. Akima: the interior slope at point
`i` (from the third to the third-from-last point) is the classic Akima weighting of the four
neighbouring chord slopes, `(|m[i+1]-m[i]| m[i-1] + |m[i-1]-m[i-2]| m[i]) / (sum of the weights)`.
Origin's end rule differs from the classic one: the slope at the first and last point is the
derivative of the parabola through the three outer points, and at the second and second-to-last
point it is the spacing-weighted mean `(m0 h1 + m1 h0)/(h0 + h1)`; the curve is the cubic Hermite
polynomial and outside the data the end cubic continues. Fits to 1e-14.

Cubic B-spline (`method:=bspline`). What was found (all with our own inputs, 11 points
x = 0..10, impulses, polynomials, random numbers; scripts `interp-bspline.labtalk.txt`):

* (observed) The curve inside the data range is a Dierckx smoothing spline (FITPACK `curfit`, the
  routine behind NAG e02be that Origin's documentation names): with the `sf` value `s` as the allowed
  sum of squared residuals it reproduces Origin's output to 1e-14 for `sf` = 0.05, 0.3, 0.5, 1, 2
  and 1e-6 on 11 and 24 points (even and uneven spacing) and for 5 on 200 noisy points (54 knots).
  The knots are chosen by Dierckx's rule (start with no interior knot, add knots where the
  residual is largest until the residual sum falls below `s`, then tune the smoothness penalty until
  the residual sum is within 0.1 % of `s`), not uniformly. `crates/calc/src/dierckx.rs` is our port;
  it also agrees with SciPy's `splrep` on 300 random cases (checked once, not kept as a test). A very
  small `sf` ends with one knot per data point (the interpolating spline); a large one with a single
  cubic polynomial.
* (observed) `sf = 0` (the default) is not an interpolating spline: Origin chooses the factor from
  the data and then runs the same Dierckx fit. The rule (found 2026-11-14 by matching the Dierckx fit
  to Origin's curve, which pins the factor to about 1e-9, then decoding it from unit impulses and all
  pairs `e_i + e_j`, `e_i - e_j` on 5, 8 and 11 points; scripts `re-workspace/scratch/t16/gen1.py` to
  `gen4.py`, raw outputs `probe1-4.out.txt` there):
  1. Drop rows with a missing X or Y. Keep the rows in worksheet order (do not sort, do not merge
     repeated X yet).
  2. With `m` rows, take `L = min(5, m - 1)` lags and the starts `i = 0 .. m - L - 1` (the SAME
     `m - L` starts for every lag, so the last rows enter fewer pairs: this is why the factor is not
     symmetric when the data are reversed).
  3. For each lag `d = 1 .. L`: `h_d` = mean over the starts of `(x[i+d] - x[i])^2`, `g_d` = mean over
     the starts of `(y[i+d] - y[i])^2 / 2` (a semivariogram against squared distance).
  4. Fit `g = a + b h` to the `L` points by ordinary least squares; the factor is `|a|`, the nugget,
     an estimate of the noise variance. The absolute value matters: the intercept is negative for
     some data (an impulse near the end), and Origin then uses its magnitude.
  On equally spaced X this is `|sum_i sum_d (89 - 5 d^2) (y[i+d] - y[i])^2| / (340 (m - 5))` for
  `m >= 6`: a unit impulse inside long data gets `1/(m - 5)`, one at the first row half of that.
  The factor scales with the square of Y, ignores a constant added to Y and the scale of X, and
  changes when the rows are reordered (checked: reversed data, unsorted X). With the factor, the
  Dierckx fit on the sorted points (repeated X merged to their mean, as for `sf > 0`) reproduced
  Origin's curve on all 236 data sets measured (impulses, pairs, random numbers, noisy sine, uneven
  and unsorted X, repeated X, missing values, 5 to 200 points) to 6e-15. A factor of 0 (constant
  Y) gives the interpolating spline. With 4 points any factor gives the interpolating cubic, so
  nothing there is measurable. Not measured: equal `h_d` at every lag (impossible for distinct X
  seen so far; Sheaf then uses `a` = mean of `g`), an Origin fit whose knot search stops early.
* (observed) Outside the data range with the default option Origin continues with the straight
  line through the two nearest data points (not the end slope of the curve), whatever `sf` is.
  `miss` and `repeat` work as for the other methods.
* (observed) Repeated X values: Origin replaces them by one point with the mean Y (checked for
  `sf` = 0.2 on x = 0 1 2 2 3 4 4; the output equals the fit of the merged points to 1e-9). We do the
  same for the B-spline method. With the default factor the merge happens after the factor is
  computed on the unmerged rows (measured, data set `dupx` of probe 3). Fewer than four different
  X values give no result.
* Our implementation (`InterpMethod::BSpline`): `smoothing` above 0 is the Dierckx smoothing spline
  inside the data; 0 computes Origin's factor (`sheaf_calc::automatic_smoothing`, the rule above) and
  fits with it; both continue outside the data with Origin's straight line. At least four different
  X values. A negative or missing factor is refused. Tests: `crates/calc/tests/origin_bspline.rs`
  and `origin_bspline_auto.rs` (16 Origin data sets at 15 positions each, 1e-10).

Unsorted X and missing values: pairs with a missing value are dropped and the points sorted, as in Origin (measured for the B-spline, where the automatic factor is computed before sorting; section 5).

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
function returns an empty result. The other search methods and smoothing before the search are
part of the peak chain (section 11); only the local maximum method is checked against `pkfind`.

Options of `pkfind` (`iy smooth dir method npts option height width filter value hwidth fheight ocenter oleft
oright` and the `_x`, `_y` outputs of each). Runs of 2026-11-13 (scripts `pkfind-feet-suite1/2.lines.txt`, fixtures
`origin_peaks_options.rs`, 45 calls, rows compared exactly):

* (observed) `smooth:=m` smooths the data first with a Savitzky-Golay filter of order 2, window `2*(m/2)+1` points and
  the end points extrapolated by the straight line through the two end points (the other four boundary choices were
  tried and fail for m = 7 to 25); `m` 0 and 1 mean none, 2 and 3 leave the data as they are (a parabola through
  three points). The search and the feet use the smoothed curve; the height filter and `ocenter_y` use the data.
* (observed) Feet (`oleft`, `oright`, 1-based rows): the foot of a peak on one side is the point of smallest `|y|`
  between the peak and its neighbouring peak in the output list (or the data end), peak included; the point nearer
  the peak wins ties. So between two positive peaks it is the valley (a small dip on the way is passed over: the
  lowest point counts, not the first), between a positive and a negative peak it is the zero crossing (the point
  with the smaller `|y|` of the two sides), and the outer feet of the first and last peak lie at the lowest point
  towards the data end. The neighbour is taken after the height and number filters and after the direction choice, so
  with `dir:=p` the zero crossing of a negative lobe between two positive peaks is passed and the foot lies at its
  nearer crossing.
* (observed) `fheight` (default 0, off): the walk away from the peak stops at the first point with `|y|` below it;
  that point is the foot (the walk is still limited by the neighbouring peak and by `hwidth`).
* (observed) `hwidth`: not given means no limit. Given, the search from a peak covers the points up to the first one
  whose X distance from the peak reaches `hwidth` (that point included: `hwidth` 0.25 on a 0.1 grid searches three
  points on each side, 0.35 four, 1.0 ten), but always at least two points on each side (0, 0.05, 0.1, 0.15 all give
  two; `npts` plays no part). Explicit `hwidth:=0` therefore differs from leaving it out. At a distance that is an exact
  grid multiple (0.2) Origin's rounding decides per peak between two and three points (not reproduced).
* (observed) `filter:=h value:=0` keeps every peak, while leaving `value` out gives the 20 % rule used so far.
* `method` `win` (with `option`, `height`, `width`), `first`, `second` and `residue` ("residual after 1st derivative")
  were run on the same curves and give peak lists unlike the local maximum method (on the ripple curve `first` finds 30,
  `second` 40 and `residue` exactly the four gaussians; on the clean curve `first` finds 15 including shoulders, `win`
  7); their rules were not worked out, Sheaf's Peak Analyzer methods (section 11) are not calibrated against them.
  An explicit `option:=r height:=10 width:=5` call returned an error for both curves.
* (observed 2026-11-14, `re-workspace/scratch/pkm`, probes 1 and 2: 15 data sets, 360 calls; rules still not found,
  these facts narrow them down) `npts` changes nothing for `first`, `second`, `residue` and `win` (1, 2, 3 and 5 give
  the same rows), and neither does the X spacing (0.5 and 1.0 give the same rows). On a single unit impulse in the
  middle of N points, `first` reports the impulse and the points `floor(N/10)` rows on either side (N = 21, 41, 81,
  101: offsets 2, 4, 8, 10), so its derivative window grows with the length of the data; `second` reports offsets
  2, 4, 6 on both sides for N = 41 (and an uneven set for N = 81, 101); `residue` reports every second row within
  4 rows for N = 41; `win` reports only the impulse. The rows reported by `first` and `second` include points whose
  value is 0 or tiny (gaussian tails), so no sign or height test applies, and on a gaussian the side points lie 10
  rows away for N = 41, not `floor(N/10)`. `win` behaves like the local maximum with a fixed window of about two
  points, but finds nothing on a gaussian raised by 5 (it measures height from a baseline). `residue` on a cubic
  returned row 1 repeated for every row (a degenerate result). Tried and failed against all 15 sets: zero crossings
  of a Savitzky-Golay derivative (windows 3 to 41, orders 1 to 5, five edge modes, three picking rules) and of a
  slope or span derivative with half window `floor(N/10)` (three edge rules, five picking rules). Origin's own
  `smooth` (FFT, adjacent averaging, Savitzky-Golay defaults) does not give these rows either.
Sheaf: `PeakOptions { smooth_points, foot_height, max_half_width }`, `PeakResult { left, right }`; the Find Peaks
window has Smoothing points, Foot height and a limit for the distance of the feet; the result sheet has Left foot and
Right foot columns (X values). `sheaf analyze peaks` keeps its own smoothing of the peak chain.

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

Sheaf's Statistics on Columns (`Tool::Statistics(StatisticsSpec)`, window Statistics > Descriptive
Statistics > Statistics on Columns, `sheaf analyze stats [--all] [--add iqr,variance,skewness,kurtosis,cv,limits]
[--level 0.95]`) writes one row per column. The default is N, Missing, Mean, SD, SEM, Sum, Minimum, Q1, Median,
Q3, Maximum; the other quantities `colstats` returns (IQR, Variance, Skewness, Kurtosis, CV) are options, as
are the lower and upper confidence limits of the mean at a chosen level (default 95 %), which are
`mean -/+ t(1 - (1 - level)/2, n - 1) SEM`, the interval of the one-sample t-test (`ttest1` intervals were
checked in `statistics.md` section 1). Which quantities Origin's dialog ticks by default was not measured
(its `DescStats` window cannot be driven from a script); the default stays the eleven columns Sheaf always
had. Not done: mode, geometric mean, trimmed mean and the other quantities of the Origin dialog.

## 9. Curve math: subtract a reference curve and simple math (`subtract_ref`, `mathtool`)

Origin has two menu items for this: Analysis > Mathematics > Subtract Reference Data (`subtract_ref`)
and Analysis > Mathematics > Simple Math (`mathtool`). `subtract_ref` is `mathtool` fixed to the
operator Subtract and the operand Reference Data. Inputs of `mathtool` (from its X-Function file):
the data curve `iy1` (X and Y), `operator` (add, sub, div, mul, pow; default add), `operand`
(Constant or Reference Data), the reference curve `iy2`, the constant `const`, `common` ("use common
range only", off by default) and `rescale` (rescales the source graph; a graph action, not
implemented). The output `oy` is an X and a Y column.

Observed (Origin, scripts `curvemath-suite1..3.lines.txt`; the Rust test is
`crates/calc/tests/origin_curvemath.rs`, relative tolerance 1e-10):

- Operations are `y op operand` with the data on the left: `y + r`, `y - r`, `y * r`, `y / r`,
  `y ^ r`. With a constant the operand is the same number for every row.
- The reference is interpolated onto the X values of the data with straight lines between its
  points (the default, and the only choice offered). Outside the reference's X range the first or
  last segment of the reference is continued as a straight line (so a data point left of the
  reference gets the straight-line extension, not the end value, and no missing value). Checked
  with data reaching 1.5 below and 1.5 above the reference.
- Without "common range only" the output has one row per data row, at the data's X values and in
  the data's order; the data need not be sorted.
- With "common range only" the result covers X from the larger of the two minima to the smaller of
  the two maxima (both ends included). Its X values are the sorted union of the data's and the
  reference's X values inside that range, and both curves are interpolated linearly onto them (so
  the data are also interpolated, not only the reference). Checked with unsorted data and a
  reference on a different grid.
- Division by zero (by the constant 0, or by a reference value that is exactly 0 at that X) gives a
  missing value. For the constant 0 Origin refuses to run the X-Function at all.
- A reference sorted by X is handled as expected, including two reference points with the same X
  (the segment left of them uses the first, the segment right of them the second). An unsorted
  reference was probed too, and Origin's result there is odd: inside the range it equals the
  sorted result, but outside the range it continued the ends differently (constant below the
  lowest X, a different slope above the highest). We do not reproduce this; we sort the reference
  and continue its end segments, which is what Origin does for a sorted reference. Not compared:
  missing values inside the data or the reference, a reference with fewer than two points.

Sheaf: `sheaf_calc::curve_math_ref`, `curve_math_const` and `CurveOp` (add, subtract, multiply,
divide, power; non-finite results become missing values). The project operation is
`Tool::CurveMath(CurveMathSpec)` of `Project::analyze` (ADR 0005), one undo step named "Subtract
reference" or "Curve math". The reference is any numeric column of the project (another sheet or
workbook too); its X is the nearest X column to its left, or row numbers. Where results go, which
follows ADR 0005 and differs a little from Origin's X and Y output pair: without common range, a
new Y column directly behind each source column (rows outside a chosen row range are missing),
named "<name> minus <reference name>" and so on, with unit of the source for add and subtract and
for scaling by a constant, and no unit otherwise. With common range only, a new sheet `Math1` with
an X column and a Y column per input. The reference needs at least two valid points; an empty
common range and division by the constant 0 are refused and change nothing. Origin's power
operator is included; the owner asked only for the four basic ones.

Command line: `sheaf analyze subtract-ref FILE --y B --ref COLUMN [--ref-file F] [--ref-book B]
[--ref-sheet S] [--common]` and `sheaf analyze math --op add|subtract|multiply|divide|power
(--value N | --ref COLUMN ...)`. The reference column is searched in the same sheet unless a file,
book or sheet is named; a file named with `--ref-file` is added to the project as a workbook (and
so appears in `--save`). Window: Analysis > Subtract Reference Data... and Simple Math..., with a
list of all other numeric Y columns of the project as references.

### 9.1 Simple Column Math (`vmathtool`)

Analysis > Mathematics > Simple Column Math combines a column with a constant or another column row
by row, with no X values. Inputs (from its X-Function file): `ix1`, `operator` (add, sub, div, mul,
pow; default add), `operand` (Constant or Reference Data; Constant first), `ix2`, `constant`
(default 10) and the output `ox`, whose default is the input column itself.

Observed (Origin, `re-workspace/scratch/vmath/vmath.out.txt`; the Rust test is
`column_math_matches_origin_vmathtool` in `crates/calc/src/curvemath.rs`):

- Without an output range the result replaces the values of the input column.
- Row `i` of the data goes with row `i` of the operand column. Rows past the end of the operand
  column count as 0: with a five-row operand, row 6 of `A - B` is `A`, of `A ^ B` is 1 and of
  `A / B` missing.
- A missing operand inside its length, or division by 0, gives a missing value; so does a
  negative number to the power 0.5.
- The result has one row per data row (a longer operand adds nothing).
- Division by the constant 0 is refused (the X-Function fails and nothing changes).

Sheaf: `sheaf_calc::column_math`, `Tool::ColumnMath(ColumnMathSpec)` (undo step "Column math").
The window starts on Add with the constant 10 and its Output on the input columns themselves
(Existing columns), as Origin; new columns beside the input or a new sheet can be chosen instead.
Command line: `sheaf analyze column-math FILE --y B [--op ...] (--value N | --ref COLUMN)`, results
beside the input. Not compared: a missing value in the data raised to the power 0.

## 10. Integrating several ranges at once (`integ1` per range)

Origin's Integrate reports for one curve and one range: `area`, `x0`/`y0` (position and signed
height of the largest-magnitude point after the baseline), `dx` (width at half height, measured as
in section 1), the first and last point of the range (`x1`, `x2`, `i1`, `i2`). It does not report
a separate absolute area; that comes from running it with `type:=abs`. Sheaf's range table is
those same numbers per range, with the absolute area added, so it reuses the checked numerics of
section 1 for every row (there is no new Origin fixture: each row is one `integ1` run on the range's
rows).

Sheaf: `Tool::IntegrateRanges(IntegrateRangesSpec)`. Input: a list of ranges, each an X interval
(the first unbroken run of rows inside it, as for the row choice of ADR 0005) or a row range, and
the end-point-baseline switch, which is applied to each range separately (the line through that
range's own first and last point). The request's own row range is applied first: X intervals are
searched inside it and row ranges are cut to it, so the two sweeps of a cyclic voltammogram are
separated with a row range, since an X interval always finds the first pass. Result: a table with
one row per input column and range, labelled "<column label>, range N", and the numbers First row,
Last row (worksheet rows), X start, X end (X of the first and last point used), Area, Absolute
area, Peak X, Peak height, Width at half height; also written as the new sheet `Integrate1`. The
area unit is the product of the Y and X units (current times time is charge). No running-integral
columns are added. A range with fewer than two points, or no row inside it, is an error and
nothing changes. One undo step, named "Integrate ranges".

Typical uses: the anodic and cathodic peak of a cyclic voltammogram (one row range per sweep) and
the charge of each cycle. Not done: choosing the ranges by dragging on a graph, anchor-point
baselines per peak, and splitting a curve into peaks automatically (Origin's Peak Analyzer).

Command line: `sheaf analyze integrate FILE --ranges 0.2:0.5,0.6:0.9 [--row-ranges 1:200,201:400]
[--baseline]`; `--absolute` and `--no-cumulative` do not apply and are refused with a message.
Window: Analysis > Integrate Ranges... (a list of X intervals with Add range and Remove).

## 11. Peak chain (Sheaf's Peak Analyzer)

Origin's Peak Analyzer is a wizard with the goals create baseline, subtract baseline, find peaks,
integrate peaks and fit peaks. Sheaf runs the same goals in one tool, `Tool::PeakChain`
(`crates/calc/src/peakchain.rs`, `crates/fit/src/multipeak.rs`, the glue in `crates/core/src/analysis.rs`).
What was measured in Origin: the local-maximum search (section 7) and `integ1` (section 1), used here
as the reference for the area, centre, height and half-height width of each peak. The wizard itself
was not driven, so the rules for baselines, limits and the other search methods below are Sheaf's own
choices, written to behave like the wizard's defaults; they are marked "own".

Steps, in order:

1. Baseline (own): none, a constant, the straight line through the first and last point, anchor
   points joined by linear or spline interpolation (the `baseline` module), or an automatic mode that
   takes the lowest point of each of n equal stretches of the (lightly smoothed) data as an anchor and
   joins the anchors by straight lines. The baseline is subtracted; all later numbers use the
   corrected curve.
2. Search (own, except local maximum): methods local maximum (section 7, `npts` neighbours), window
   search (the largest point of a window of `npts` points each side), first derivative (plus-to-minus
   sign change of a Savitzky-Golay derivative) and second derivative (a negative minimum of the second
   derivative, which also finds a shoulder that has no maximum of its own). Optional Savitzky-Golay
   smoothing before the search; the reported row is the largest raw point next to the smoothed
   maximum. Direction positive, negative or both. Filters: height (0 means 20 % of the largest
   magnitude, as in `pkfind`), a number of highest peaks, and a minimum full width at half height.
3. Limits (own): walking outwards from the peak point, a peak ends where the corrected curve reaches
   zero (the baseline), found by linear interpolation between two points, but never beyond the valley
   (the point nearest the baseline) between it and the neighbouring peak, so overlapping peaks share
   the valley as their common limit and their areas add up to the area under both. With the half
   height option the limits are the points where the curve falls to half the peak height.
4. Result per peak: row of the peak point, centre (X of the peak point), height above the baseline,
   area (trapezoids between the two limits, including the interpolated end pieces), area percent (share
   of the sum of all areas), FWHM (interpolated half-height crossings inside the limits), left and right
   limit (X). A negative peak has negative height and area.
   Data rules (own, not compared with Origin): rows where X or Y is missing are skipped in every step
   (the baseline and corrected curve are missing there, an area is the integral over the valid rows,
   and the end-points line goes through the first and last valid rows), so the integration columns and
   the fit columns agree. X must run one way, rising or falling; otherwise the tool refuses with "X is
   not sorted" and changes nothing. The area of a positive peak is positive whichever way X runs
   (Origin's `integ1` gives a negative area for falling X, section 1; the table here follows the
   fit columns instead, so the two sets of columns have one sign).
5. Multi-peak fit (own model, Origin's functions): a sum of Gauss, Lorentz or Voigt peaks plus a
   constant or sloped baseline, parameters `y0`, `m`, then `xc`, `w`, `A` per peak (Voigt: `xc`, `A`,
   `wG`, `wL`, Origin's order), in Origin's parameterisation (Gauss `A/(w sqrt(pi/2))
   exp(-2((x-xc)/w)^2)`, FWHM `w sqrt(ln 4)`; Lorentz `2A/pi * w/(4(x-xc)^2+w^2)`, FWHM `w`; Voigt
   section 11.2). Starting values come from the found peaks (centre, FWHM,
   height above the baseline), with widths bounded to stay positive. The fit uses the nonlinear engine
   of `crates/fit`. The report has centre, height, FWHM, area and area percent per peak, plus a line of
   R squared, reduced chi-square and iterations. The fit uses the data, not the corrected curve, so
   the baseline is fitted along with the peaks.

Output (ADR 0005): a new sheet `PeakAnalysis1` with one row per peak and column, headers Row, Center,
Height, then (with integration) Area, Area %, FWHM, Left limit, Right limit, then (with a fit) Fit
center, Fit height, Fit FWHM, Fit area, Fit area %. A baseline column and, with a fit, a fitted-curve
column are added beside each source column. No peaks found, or a baseline that cannot be made, is an
error and changes nothing. One undo step named "Analyze peaks".

Checked against Origin (`crates/calc/tests/origin_peakchain.rs`, script
`tools/origin-ref/peaks-integ.lines.txt`): generated data (401 points, slope 0.01, one separate peak,
two overlapping peaks, a small ripple). Sheaf found three peaks and its limits; Origin's `integ1` then
integrated the corrected curve over exactly those rows. Agreement: peak position and height exactly,
area within 2e-4 absolute (Sheaf also integrates the sliver up to the interpolated baseline crossing,
below 1.1e-4 here), width at half height within 1e-9 where Origin's half-height crossing lies inside
the rows. For the second peak the valley side never reaches half height inside its limits; Origin
extends the width beyond the rows and Sheaf stops at the limit, so that width is not compared. Not
checked against Origin: the choice of limits itself, the automatic baseline, the derivative methods,
and the multi-peak fit (it is tested against closed-form data in `multipeak.rs` and in the core and
CLI tests, not against Origin's Peak Analyzer fit).

Not done: anchor points chosen on a graph, a second-derivative-zero baseline, the wizard's
interactive preview, separate widths or shapes per peak, constraints, and fitting inside a row range
other than the request's range.

Command line: `sheaf analyze peaks FILE [--method local-max|window|first-derivative|second-derivative]
[--smooth N] [--min-width W] [--baseline none|end-points|minima|anchors] [--baseline-points N]
[--anchors X1:Y1,X2:Y2] [--limits baseline|half-height] [--integrate] [--fit gauss|lorentz|voigt]
[--fit-shapes F1,F2,...]
[--fit-baseline constant|linear]`, together with the existing `--points`, `--min-height`, `--count`,
`--direction`. Without any chain option the plain peak finder of section 7 runs. Window: Analysis >
Analyze Peaks... (command line `--show analysis:peak-chain`).

### 11.1 Origin's Peak Analyzer wizard (measured 2026-11-13)

Opened through Analysis > Peaks and Baseline > Peak Analyzer > Open Dialog... on a graph
(`tools/origin-ref/origin-dialog.ps1 -MenuClicks`, pictures and control lists in
`re-workspace/scratch/t04/d2..d4`). The menu Peaks and Baseline holds Multiple Peak Fit..., Peak Analyzer
(a submenu: `<Last used>`, `QuickPeaks (System)`, Open Dialog...) and Batch Peak Analysis Using Theme....

The window "Peak Analyzer" (about 400 x 600) has Dialog Theme, a page tree on black, Prev, Next, Finish
and Cancel, the X-Function name of the page with a one-line description, and the page's settings. The
page list follows the Goal:

| Goal | Pages after Goal |
|---|---|
| Integrate Peaks (default) | Baseline Mode, Baseline Treatment, Find Peaks, Integrate Peaks, Finish |
| Find Peaks | Baseline Mode, Baseline Treatment, Find Peaks, Finish |
| Fit Peaks (Pro) | Baseline Mode, Baseline Treatment, Find Peaks, Fit Peaks (Pro), Finish |
| Create Baseline | Baseline Mode, Create Baseline, Finish |
| Subtract Baseline | Baseline Mode, Subtract Baseline, Finish |

Next is greyed on the last page before Finish.

- Goal (`pa_goal`, "Select spectrum data and Goal"): Recalculate (Manual), Goal, Input.
- Baseline Mode (`pa_basemode`): Constant (default; Y = the Minimum (default), Maximum, Mean or Median of
  the data, or Custom), User Defined (anchor points found by 2nd Derivative (default), 1st and 2nd
  Derivative, a script or an existing dataset; smoothing window Auto; Number of Points to Find; Snap to
  Spectrum), Use Existing Dataset, XPS, None (Y=0), End Points Weighted, Straight Line, Asymmetric Least
  Squares Smoothing.
- Baseline Treatment (`pa_basetreat`): Auto Subtract Baseline (off), Auto Rescale (off), Subtract Now,
  Undo, Rescale.
- Find Peaks (`pa_peaks`): Current Number of Peaks, Enable Auto Find (on), Find, Add, Modify/Del, Clear
  All, Save..., Load..., Peaks Info..., Snap to Spectrum (off); Peak Finding Settings: Direction Both
  (Positive, Negative), Method Local Maximum (Window Search, 1st Derivative, 2nd Derivative (search Hidden
  peaks), Residual after 1st Derivative (search Hidden peaks)), Smooth Derivative None or Savitzky-Golay,
  smoothing None (FFT Filter, Adjacent-Averaging, Savitzky-Golay, Quadratic Savitzky-Golay), Local Points
  and Size Option (Percent of Raw Data); Peak Filtering: Method By Height (None, By Number), Threshold
  Height (%) 20 with Auto; Labels and Markers: center label X of Peaks (Y, Indices, (X,Y)), rotated, Above,
  with a center marker.
- Integrate Peaks (`pa_int`, "Calculate the peak area"): Number of Ranges to Integrate (All Peaks),
  Integration Window Width Auto (Adjust on Preview Graph, Fix Width For All Peaks; left and right half
  width), Integrate From Baseline (or Y=0), Show Integrated Area (on), Quantities: Peak Area, Percent
  Area, Row Index, Beginning X, Ending X, Peak Center, Peak Height, FWHM (on), Curve Area, Left Half
  Width, Right Half Width, Peak Centroid (off).

Results of Integrate Peaks with every default (`re-workspace/scratch/t04/r-*.out`, d10): a sheet
`Integration_Result1` with Index, Area, AreaIntgP(%) (comment "Integral Result of B"), Row Index, Beginning X,
Ending X, FWHM, Center, Height. On 0.2 + 0.01 x + exp(-(x-5)^2/0.5) + 0.6 exp(-(x-12)^2/0.8), x = 0..20 step
0.05: the Constant baseline is the minimum 0.2; peak 1 row 101, X 0.3 to 6.8, area 1.48386, 35.292 %,
FWHM 1.220641, height 1.05; peak 2 row 241, X 6.8 to 14.15, area 1.72099, 40.932 %, FWHM 1.713769, height 0.72.
AreaIntgP is the share of the area of the whole baseline-corrected curve (4.2045 here), not of the sum of
the peaks. The Auto integration window of a lone Gaussian exp(-(x-10)^2/(2 s^2)) on the same X ran from
10 - d to 10 + d with d = 1.75, 3.75 and 8 for s = 0.5, 1 and 2 (3.5 s, 3.75 s and 4 s; no rule found yet).

Sheaf (2026-11-13): the Peak Analyzer window shows Origin's page tree for the goal, Prev and Next, and the
pages Goal (Integrate, Find and Fit Peaks; Create and Subtract Baseline greyed), Baseline Mode (Constant with
Minimum by default, Maximum, Mean, Median or Custom; User Defined with Sheaf's anchors; None (Y=0); Straight
Line through the end points; the other modes greyed), Baseline Treatment (greyed: the baseline is always
measured from, never subtracted from the data), Find Peaks, Integrate Peaks and Fit Peaks; OK is Finish.
Center, Row, Height and FWHM match Origin to 1e-9 on the data above, Area % follows Origin's definition,
and the limit between two peaks (the valley, 6.8) matches; the outer limits do not (Sheaf integrates to
where the corrected curve meets the baseline or the end of the data: 0 and 20 here, Origin 0.3 and 14.15).

### 11.2 Voigt peaks (measured 2026-11-15)

The Fit Peaks page (`sheaf analyze peaks --fit voigt`) offers Origin's `Voigt` besides Gauss and
Lorentz: a Gauss of FWHM `wG` convolved with a Lorentz of FWHM `wL`, area `A`. Sheaf evaluates it
exactly through the Faddeeva function (`sheaf_fit::library::voigt`; scipy's `wofz` gives the same
digits). Origin's function record (`VOIGT5.FDF`, `docs/spec/data/fit-functions.json`) gives the
parameter order `y0 xc A wG wL` and the derived FWHM `0.5346 wL + sqrt(0.2166 wL^2 + wG^2)`, which
Sheaf reports as the peak's FWHM; the height is the value at the centre. A peak starts at the found
centre with `wG = wL` (as Origin's initialisation, which gave `wG = wL = 1.421` on the probe below)
chosen so that the start has the found FWHM and height; `wL` is bounded below by 0 (a Gauss) and `wG`
by 1e-4 of its start.

Measured in Origin 2016 (`re-workspace/scratch/t04v/v1` and `v2`, `tools/origin-ref/run-labtalk-lines.ps1`):

- `nlf_Voigt` is a numerical approximation: at six points (0.2 + Voigt(xc 5, A 4, wG 1.2, wL 0.8) at x
  = 0, 2, 4.5, 5; a nearly Lorentzian and a nearly Gaussian peak) it differs from the exact value by at
  most 1.7e-6 of the peak height (2.042942100 against 2.042945314 at the centre).
- `nlbegin func:=Voigt; nlfit` on 201 points made by `nlf_Voigt` plus a fixed ripple (x = 0 to 10,
  kept as `crates/fit/tests/data/origin_voigt_probe.tsv`) gave y0 0.199772863732606, xc
  4.99999967995525, A 4.00225088185448, wG 1.19892425718239, wL 0.801615491802901, reduced
  chi-square 7.8673486183435684e-05 in 8 iterations. Sheaf's multi-peak fit of the same points with
  one Voigt peak agrees to 6e-6 relative (xc to 2.3e-9) and the reduced chi-square to 2e-7, the size of
  the difference between Origin's approximate and Sheaf's exact function. The standard errors of xc
  and A are 3.4e-4 and 5.3e-3 relative apart (Origin's derivatives of its approximate function would
  explain it; not measured further). Test: `crates/fit/tests/multipeak_voigt.rs`.

**A function per peak** (Sheaf's own layout; Origin's Fit Control window, where the Pro version sets a
function per peak, was not opened): the Fit Peaks page's "Function per peak" list (Add Peak Function,
one entry per peak in the order found, "Same as above" or Gauss, Lorentz, Voigt) and
`sheaf analyze peaks --fit F --fit-shapes F1,F2,...` give peak *k* its own function; peaks beyond the
list take the Function (`--fit`). Each peak keeps its own parameters (`xc1 w1 A1 xc2 A2 wG2 wL2` for a
Gauss and a Voigt), the note names them in order ("Gauss+Voigt fit of 2 peaks") and the fitted
column's comment says the same. `--fit-shapes` needs `--fit`.

Not done: constraints between peaks, and Origin's own Peak Analyzer Fit Peaks (Pro) page, which was
not opened (11.1).

### 11.3 Multiple Peak Fit (T128, 2026-11-17, not measured)

Origin facts (docs.originlab.com/origin-help/fit-multipeakfit-tool and x-function/ref/nlfitpeaks;
Origin COM was unavailable, so nothing was measured): Analysis: Peak and Baseline: Multiple Peak Fit
opens the `nlfitpeaks` dialog (input: the active data plot; the peak function); OK activates the
graph (from a worksheet a graph is made first) and opens Get Points: the cursor moves with the mouse
or arrow keys, a double-click or Enter picks a peak centre, the dialog shows X, Y and the number
picked. **Fit** fits with the picked centres as starting points and adds a results table to the
graph; **Open NLFit** opens the NLFit dialog initialised from them. No baseline treatment; Use Peak
Marker fits existing peak markers.

Sheaf (`crates/app/src/dialogs/multipeak.rs`, the chain of 11 with given centres), choices
**unmeasured**:

- From a graph the active data plot is the input; from a worksheet with one Y column selected, a
  line graph of it is made first (its own undo step "Plot"). Peak Function: Gauss (default, as
  Origin's tutorial picks), Lorentz or Voigt.
- Get Points uses the graph's Data Reader (turned on while picking): a click pins a point; a
  double-click outside the Get Points window, or Enter, picks the pinned point of the input plot.
  The window lists the centres (X, Y); Remove Last undoes a pick; a point already picked is not
  picked again. Cancel or closing turns the Data Reader off and changes nothing.
- Fit runs the Peak Analyzer chain with no baseline and the picked X as the peaks
  (`analyze_peaks_at`: each centre snaps to the nearest data point, two centres on one point give one
  peak, heights and FWHM are measured above the lowest point of the data), then the multi-peak fit
  of 11 with a constant `y0` (start: the lowest Y) and per-peak starts from those heights and widths.
  Results go where the Peak Analyzer's go (the result sheet, one row per peak in X order, the fit
  curve column beside the data, then a column "Peak k of ..." per peak: that peak on the fitted
  baseline `y0`), and the curves are drawn on the graph as lines: each peak in blue, green, orange
  (repeating), then the fitted sum on top in the fit tools' red (220, 30, 30), width at least 1.5.
  One undo step ("Multiple peak fit"). The Peak Analyzer does not add the per-peak columns
  (`PeakFitSpec.peak_curves`, off there).
- **Left out:** Open NLFit and Use Peak Marker (shown disabled), the results table on the graph,
  arrow-key stepping specific to Get Points (the Data Reader's Left/Right
  keys step the pinned point).

### 11.4 Peak Analyzer themes and batch peak analysis (T169, 2026-11-19, not measured)

Origin keeps Peak Analyzer settings as dialog themes and its Peaks and Baseline menu has Batch Peak
Analysis Using Theme... (11.1). Neither window was measured; Sheaf's are its own (**unmeasured**):

- **Save as Theme** (the Peak Analyzer window, below Prev/Next): a Theme name and Save as Theme store
  the window's current settings (the `PeakChainSpec` that OK would run: goal, baseline, peak search,
  integration and fit) as a named theme in `peak-themes.json` in the settings folder
  (`sheaf_io::peak_themes`). Names are trimmed and white space collapsed; a theme of the same name
  (letter case ignored) is replaced in its place. A theme this version cannot read is kept in the file
  and hides no other. Loading a theme back into the window is not offered yet.
- **Batch Peak Analysis Using Theme...** (Analysis > Peaks and Baseline): a window with the saved themes
  and the curves it runs on: the selected numeric columns of the active workbook, else the columns a plot
  of the active sheet would show; on a graph the active layer's curves. OK runs Peak Analyzer with the
  theme on all of them at once, the results in one new sheet as an ordinary Peak Analyzer run on several
  columns, one undo step ("Batch peak analysis"). Not done: Origin's per-dataset summary options, running
  on several books or files (use File > Batch Processing, workbook-model 13p).

### 11.5 Create Baseline and Subtract Baseline goals, User Defined anchors (measured 2026-11-14)

Measured by clicking Origin's Peak Analyzer through the goals with `tools/origin-ref/origin-dialog.ps1`
(`re-workspace/scratch/t04base`: `run-base.sh` runs a curve through Create Baseline with Origin's defaults and
reads the anchors back as the rows where the written baseline changes slope; `fit_rule.py`, `fit2.py`, `fit3.py`
try candidate rules):

- Pages: Goal, Baseline Mode, Create Baseline (`pa_basecreate`) or Subtract Baseline (`pa_basesubtr`), Finish.
  For these goals Baseline Mode lists User Defined (default), XPS, End Points Weighted and Asymmetric Least
  Squares Smoothing. User Defined: Snap to Spectrum (off); Anchor Points Finding Method 2nd Derivative (default),
  1st Derivative and 2nd Derivative, Use Script to Search, Use Existing Dataset; Smoothing Window Size 3 and
  Threshold 0.05, both Auto (the same shown values for 201, 401 and 801 points); Enable Auto Find (on); Number of
  Points to Find; Find, Add, Modify/Del, Clear All, Save..., Load..., Anchor Points Info....
- Create Baseline page: Connect by Interpolation, Snap to Spectrum (off), anchor Add and Modify/Del, Number of
  Baseline Points (the input's, "Same as" ticked), Interpolation Method Line (default), Spline or BSpline, the
  Baseline output and a Subtract button. Subtract Baseline page: Subtract, Subtracted Data and Baseline (ticked)
  outputs.
- Output, in the input sheet after its columns: Create Baseline writes `Baseline_Data X1` and
  `Baseline_Data Y1` (comment "Baseline of B"); Subtract Baseline writes `Subtracted_Data X1`,
  `Subtracted_Data Y1` ("Subtracted from B"), `Baseline_Data X1`, `Baseline_Data Y1` ("Baseline to Subtract
  for B"). The anchors are data points; the baseline is the straight line through them at every X of the data.
- Anchors, X = 0..20 step 0.05 (401 rows), two Gaussian peaks at 5 and 12 on 0.2 + 0.01 x: rows 0, 32, 64,
  165, 197, 304, 336, 400; the same on the curved baseline 0.5 + 0.003 (x - 7)^2; one peak at 10 on that curve:
  0, 41, 82, 123, 164, 276, 317, 400; the curve without a peak: only the two ends. Steps 0.1 and 0.025 (201 and
  801 rows) with the two peaks: 0, 15, 30, 82, 97, 155, 170, 200 and 0, 65, 130, 328, 393, 603, 668, 800.
- The rule these fit: the candidates are the rows where the second derivative is small compared with its
  largest size, leaving out single rows (a peak's inflection point is not a candidate); with C candidates
  and the step C / 8 (rounded down, 8 = Number of Points to Find), the candidates number 0, step, ..., 6 step
  are anchors, and the last row is always one. A curve whose curvature is the same everywhere has no
  candidate. The threshold is about 2.5 % of the largest |second derivative| at 401 points but about 1.3 % at
  201 and 3.3 % at 801 points, although the page shows the same Auto values, so the exact test is not known.

Sheaf (`sheaf_calc::second_derivative_anchors`, `ChainBaseline::SecondDerivative`, `Tool::PeakBaseline`): the
candidates are the rows where |second derivative| (central differences) is at most 2.6 % of its largest value,
runs of one row left out. This gives Origin's anchors for the straight-baseline and the one-peak cases above and
for the curve without a peak; on the curved baseline three anchors are 1 to 4 rows away, and at 201 and 801
points a few rows away. Those anchors lie on flat baseline, so the baseline hardly changes. The Peak Analyzer's
Create Baseline and Subtract Baseline goals are offered with User Defined (2nd Derivative, Number of Points to
Find 8, or Sheaf's own lowest point of each stretch) and Line, Spline or BSpline; the columns are named as
Origin's (`Baseline_Data Y<n>`, `Subtracted_Data Y<n>`), without Origin's copies of X. The other methods, Snap
to Spectrum, anchor editing, XPS, End Points Weighted and Asymmetric Least Squares are greyed. User Defined in
the peak goals uses the same anchors with straight lines. Command line: `sheaf analyze peaks FILE --baseline-goal
create|subtract [--baseline second-derivative] [--baseline-points N]`.

## 12. Data manipulation tools and Median Filter (Sheaf's own rules, not measured, except 12.1 to 12.3)

Analysis > Data Manipulation has Subtract Straight Line, Reduce Duplicate X Data (12.1), Reduce to Evenly Spaced X and Curve
Translate X; Analysis > Mathematics has Median Filter. Nothing in this section was run in Origin:
there was no Origin on the machine where it was written. What is known comes from Origin's public
documentation (paraphrased: what each item is for) and from two earlier measurements that the
tools reuse. Everything else is Sheaf's choice, listed under "Not measured" so that the Windows
machine (`tools/origin-ref`) can settle it. The numerics are in `crates/calc/src/manip.rs`, the
tools in `crates/core/src/analysis/manip.rs`.

Measured earlier and reused:

- Linear interpolation between data points and missing values outside the data (`calc::interp1`
  with the `miss` option), observed in section 5. These tools return missing values outside the
  data; they do not use the end-segment continuation that section 5 also observed.
- Subtracting a reference curve (`subtract_ref`, section 9) was measured. The two-point line
  subtraction of Subtract Straight Line (`calc::subtract_line`, section 6) is Sheaf's own and was
  not compared with Origin; only its interpolation arithmetic is covered by section 5.
- The percentile filter's boundary rule (section 3): with boundary `none` the window shrinks
  symmetrically and the first and last point keep their value; `repeat` and `reflect` fill the
  window. Missing values are taken out, the rest is smoothed as one series and the missing rows are
  put back. The median filter copies this on the assumption that Origin's median filter behaves like
  the percentile filter at 50 % (see "Not measured").

Rules in Sheaf (`Tool::SubtractLine`, `ReduceX`, `Translate`, `MedianFilter` of `Project::analyze`;
each is one undo step named "Subtract straight line", "Reduce to evenly spaced X", "Translate X",
"Translate Y" and "Median filter"; all four can be run with Recalculate Auto or Manual, section 9's
link mechanism):

- Subtract Straight Line: the line goes through two points given by X. A point's Y is the curve's
  own value at that X (straight lines between data points) unless a Y is given. Equal X values, an X
  outside the data without a given Y, and a curve with fewer than two valid points are refused with
  nothing changed. The result is a new Y column behind each input named "<name> minus line".
- Reduce to Evenly Spaced X: Origin's `reduce_ex`, measured (12.2).
- Curve Translate X: adds a constant to X, or moves the point at X = `at` to X = `to` (shift
  `to - at`); the result is a new sheet with the shifted X and the unchanged Y. Curve Translate Y
  (Sheaf's own item, next to X): adds a constant to Y, or shifts so that the curve's value at X = `at`
  (between data points by straight lines) becomes `to`; the result is a new Y column behind each
  input named "<name> shifted". A point outside the X range is refused.
- Median Filter: Origin's percentile filter at 50 % (12.3). The window has `2 (npts / 2) + 1`
  points (an even `npts` acts like the next odd one; at least 1; the default is 5), centred on each
  point. Edge choices: shrink (default), repeat the end value, reflect as Origin's percentile filter.
  The result is a new Y column behind each input named "Median filtered <name>".

Not measured (to be measured in Origin; nothing here may be quoted as Origin's behaviour):

1. X-Function names, read from the interface lists of Origin's X-Function files with
   `tools/origin-ref/xf-args.py` (names, labels, defaults and choices only): Subtract Straight Line is
   `subtract_line` (no arguments: an interactive tool on the graph), Curve Translate is `curve translate`
   (interactive: `ntype` Vertical Translate (default) or Horizontal Translate, a line to move the curve
   to and a marker on the curve), Reduce to Evenly Spaced X is `reduce_ex` (12.2). Origin's
   Mathematics menu has no median filter for curves: `medianflt2` filters matrices; a 1D median is a
   method of Smooth.
2. Subtract Straight Line: how Origin picks the two points (on a graph, by data cursor, or by
   typed values), whether a point's Y is the data value or the cursor's, and whether the result
   replaces the data or goes to a new column.
3. Reduce to Evenly Spaced X: measured, see 12.2.
4. Curve Translate: whether Origin moves data in place or writes a copy, how its "move a point to
   a value" mode reads a Y at an X between data points, and whether it works on the graph
   interactively. Curve Translate Y is not an Origin item as far as we know; its numbers are Sheaf's.
5. Median Filter: measured, see 12.3 (missing values follow the smoothing rule measured in
   section 3; the default window of 5 is Sheaf's, Origin has no median item for curves).
6. The Recalculate behaviour of these items in Origin (which of None, Manual, Auto they offer),
   the long names, units and comments of the output columns, and the row-range handling.

### 12.1 Reduce Duplicate X Data (measured in Origin)

Analysis > Data Manipulation > Reduce Duplicate X Data is Origin's X-Function `reducedup` (method,
`tol`, `oy`, `xcount`). Measured on 2026-11-13 with `tools/origin-ref/run-labtalk-lines.ps1`
(scripts and outputs `re-workspace/scratch/reducedup/reducedup*.lines.txt` and `.out.txt`):

- Pairs with a missing X or Y are left out; the rest are sorted by X.
- Neighbours whose X values differ by no more than `tol` times the X range form one group, as a
  chain: 0, 0.3, 0.6, 0.9 with `tol` 0.035 of a range of 10 (limit 0.35) are one group although 0.9
  is 0.9 from the first. The default `tol` groups X values 1e-8 apart in a range of 1 and not 9e-8
  apart; Sheaf uses 1e-8. The comparison is plain floating point (10.3 - 10.1 is just above 0.2 and
  does not group at a limit of 0.2).
- A group's X is its lower middle X after sorting (the median of an odd group, the lower of the two
  middle values of an even one); a point alone keeps its own Y.
- Methods mean (default), median, min, max, sum, sd (n - 1 divisor), se (sd / sqrt n) and rms; rms
  gives the mean of the group, not the root mean square.
- Output by default: two new columns at the end of the input sheet, long names `ReduceDupX1` and
  `ReduceDupY1` (the number counts up with each run), the Y column's comment "Duplicates replaced
  with Mean of B" (the method's word: Mean, Median, Min, Max, Sum, SD, SE, RMS). `xcount:=<new>` adds
  `DupX1` (the X of each group of two or more points) and `DupXCount1` (its count), no comments.

Sheaf (`Tool::ReduceDup`, undo step "Reduce duplicate X data", `calc::reduce_duplicates`) copies
all of this; since T321 the columns go to the end of the input sheet as in Origin, numbered on after the
`ReduceDupX<n>` columns already there (before, they went to a new sheet named "ReduceDup"). The window offers the method, the tolerance and "Output
the duplicate X values and their counts". Recalculate works as for the other tools of this section.

### 12.2 Reduce to Evenly Spaced X (measured in Origin)

Origin's `reduce_ex` (dialog and runs on 2026-11-13, `re-workspace/scratch/needs7/reduce_ex/` and
`reduceex1..4.lines.txt`/`.out.txt`). The dialog has Recalculate (Manual), Input, SubGroup (Every N
Points, By Sampling Interval, By Number of Intervals, By Frequency), Resample Factor (5 with Auto
ticked; Auto gave 5 for 5 to 10000 points), Sampling Interval (0.25), Interval Number (10), Resample
Frequency (8), X Start and X End (auto), Sampling Method (First Point, Y Mean (default), Random Y (X
Average)), Even Sampling Interval Tolerance (5 %), Plot FFT for Preview, and Output `(<autoX>,<new>)`.

- Every N Points: each run of N points gives one point. Y Mean gives the mean X and mean Y of the
  run, First Point the run's first point. A shorter last run keeps the place of a full run: 12 points
  0..11 every 5 gave X 2, 7, 12 (the last run is 10 and 11). When the X steps are not even (10 % off
  was; 4 % off was not), the curve is first interpolated by straight lines onto as many evenly spaced
  X values from the first to the last X; a missing value counts as a gap and does the same (the
  row is left out and the 11 other points regridded).
- The interval modes use the raw points. There are round((X End - X Start) / interval) intervals
  `[start + k dx, start + (k + 1) dx)` (11 / 2 gave 6, 11 / 2.5 gave 4); a point at X End or beyond
  is in none, even inside the last interval. By Number of Intervals uses (End - Start) / n, By
  Frequency an interval of 1 / f. Y Mean gives the interval centre and the mean Y, First Point the
  lower edge and the Y of the first point in it; an empty interval keeps its X with a missing Y.
- The output columns are named `ReduceEXX1` and `ReduceEXY1`; the Y comment is "Y Mean(X Average)
  Stats Reduction of B" or "Y First(X First) Stats Reduction of B".

Sheaf (`calc::reduce_evenly`, `Tool::ReduceX`) copies these rules, names and comments, with three
differences: Random Y is not offered; the columns go to a new sheet (as for every curve result of this
section) and always include X, where Origin's default writes only a Y column with an implicit even X;
in a run whose steps are uneven but within the tolerance, Origin's X of the short last run is not the
rule above (0, 1.03, ... 10 every 3 gave 10.103 for the run 9, 10; Sheaf gives 10).
Command line: `sheaf analyze reduce-x FILE --y B [--every N | --step DX | --intervals N | --frequency F]
[--method mean|first] [--x-min X] [--x-max X] [--tolerance PCT]`.

Command line: `sheaf analyze reduce-dup FILE --y B [--method mean|median|min|max|sum|sd|se|rms]
[--tol T] [--count]`; window `--show analysis:reduce-dup`.

Command line: `sheaf analyze subtract-line FILE --y B --x1 X --x2 X [--y1 V] [--y2 V]`,
`sheaf analyze reduce-x` (12.2), `sheaf analyze translate FILE --y B [--axis x|y] (--by D | --at X --to V)`
and `sheaf analyze median-filter FILE --y B [--points N] [--edge shrink|repeat|reflect]`. Windows:
`--show analysis:subtract-line`, `reduce-x`, `translate-x`, `translate-y`, `median-filter`.

### 12.3 Median Filter (measured in Origin)

Origin 2016 has no median filter for curves; its median of a curve is Smooth's percentile filter at
50 % (`smooth method:=pf percent:=50`). Measured on 2026-11-15 with
`tools/origin-ref/run-labtalk-lines.ps1` (`re-workspace/scratch/t23m/m1`): 40 points with ties and
outliers, windows 1, 4, 5, 7 and 8, boundaries none, reflect and repeat; the sheet is
`crates/calc/tests/fixtures/origin_median.tsv`.

- An even `npts` acts like the next odd one: 4 gives the result of 5, 8 that of 9 (as section 3 found
  for averaging). `npts:=1` returns the data.
- `none`: the window shrinks symmetrically at the ends, so the first and last point keep their value.
- `repeat`: the end value fills the window.
- `reflect` is not symmetric: before the start the data are mirrored without the first point
  (`y[-1] = y[1]`, `y[-2] = y[2]`), past the end with the last point repeated (`y[n] = y[n-1]`). The
  second row of the probe tells the two rules apart at the start (the mirror with `y[-1] = y[0]` gives
  7.8108 where Origin gives 7.0353) and the next-to-last row at the end. Sheaf's Smooth followed the
  symmetric rule for the percentile filter too, and was corrected; its averaging and Savitzky-Golay
  reflection, measured in section 3, is unchanged.

Sheaf's Median Filter (`calc::median_filter`) and Smooth's percentile filter reproduce all eight
Origin columns exactly (a median is one of the values): `crates/calc/tests/origin_median.rs`.

## 13. Convolution and Correlation (`conv`, `corr1`)

Analysis > Signal Processing > Convolution and Correlation. Inputs (from the X-Function files):
`conv`: the signal `ix`, the `response`, `interval` (`<auto>`), `norm` (off), `wrap` (off),
`circular` (Linear or Circular) and the output `oy` (X and Y); `corr1`: `ix1`, `ix2` (the same
column for an autocorrelation), `interval`, `type` (Linear or Circular), `norm` (off), `oy`.

Observed (Origin, `re-workspace/scratch/signal/sig1.out.txt` and `sig2.out.txt`; Rust tests
`convolution_matches_origin_conv` and `correlation_matches_origin_corr1` in
`crates/calc/src/signal.rs`, 1e-12). Origin computes by FFT, so its numbers carry noise of about
1e-15 (`2.4999999999999996`, values like `2e-16` where the exact result is 0); Sheaf sums directly.

- Convolution, linear: the full convolution of the signal (n points) and the response (m), n + m - 1
  values. X is `k * interval`, k from 0. With `<auto>` the interval is the X step of the signal's
  sheet (X 0, 0.1, 0.2, 0.3 gave 0, 0.1, ..., 0.4); without X it is 1. Checked with a longer
  response than signal too.
- Convolution, circular: both padded with zeros to N = max(n, m); N values,
  `c[k] = sum_j s[j] r[(k - j) mod N]`.
- Normalize: the response is divided by its Euclidean norm (1 / sqrt(1.3125) = 0.8729 for
  1, 0.5, 0.25).
- Wrap response: no difference found with a three-point response, linear or circular; not
  offered in Sheaf.
- Correlation, linear: both padded with zeros to N = max(n, m); lags -(N-1) to N-1 (2N - 1 values,
  so a leading 0 when the lengths differ), value `sum_j a[j + k] b[j]`; lag 0 is the dot product.
  The X is the lag times the interval, 1 by default (the X spacing of the sheet is not used).
- Correlation, circular: N values at lags 0 to N-1, but in Origin's order value k is
  `sum_j a[(j - k - 1) mod N] b[j]`, that is the plain circular correlation read from the far end.
  Checked on two data sets and both argument orders.
- Normalize (correlation): divided by sqrt(sum a^2 * sum b^2).
- Decimation (`decimate`) is section 13.4.

Sheaf: `sheaf_calc::convolve`, `correlate`; `Tool::Convolution(ConvolutionSpec)` and
`Tool::SignalCorrelation(SignalCorrelationSpec)`. The results go to a new sheet `Convolution<n>`
or `Correlation<n>` with an X (the input's X name and unit, or "Lag") and a Y column per input.
Trailing missing values are dropped; a missing value inside a signal is refused. The windows list
every numeric Y column of the project, the inputs included, as the response or second signal.
Command line: `sheaf analyze convolution FILE --y B --response C [--circular] [--normalize]
[--interval D]` and `sheaf analyze signal-correlation FILE --y B --with C [--circular] [--normalize]
[--interval D]`.

### 13.1 Hilbert Transform (`hilbert`)

Analysis > Signal Processing > Hilbert Transform. Inputs: `ix`, `hil` (output the transform, on),
`ansig` (output the analytic signal; on in the X-Function file although its help says off) and the
report sheet `rd` (a new sheet of the input book). Observed (`re-workspace/scratch/hilbert/hb.out.txt`,
eight and seven points; Rust test `hilbert_matches_origin`, 1e-12): the transform is the standard
discrete one, the imaginary part of the analytic signal made by FFT (positive frequencies doubled,
negative removed, zero and, for an even length, the middle frequency kept); the analytic signal is a
complex column whose real part is the input. Sheaf writes the transform only, in a new column beside
each input (no complex columns); `Tool::Hilbert`, `sheaf analyze hilbert FILE --y B`.

### 13.2 Envelope (`envelope`)

Analysis > Signal Processing > Envelope. Options (from the X-Function file): `iy` (the input XY
range), `type` (`upper` Upper Envelope, the default, `lower`, `both`), `method` (one choice, By Spline
of Extreme Pts; giving `method:=spline` or `0` made Origin refuse the call, so it is left out),
`npts` (default 2) and the report sheet `rd` (without it Origin makes a new workbook named after the
input). Observed (`re-workspace/scratch/envelope/p1..p11.out.txt`, 12 to 30 points; Rust tests
`origin_smooth_curve`, `origin_shuffled_x`, `origin_up_then_down_x`, `origin_falling_x` in
`calc/src/envelope.rs`, 1e-9, and `an_envelope_goes_to_a_new_sheet_with_its_own_x` in core):

- Extreme points: a point is an upper extreme when it is the first largest value of the window of
  `npts` points on each side of it (itself included): the values on its left are strictly smaller,
  those on its right smaller or equal, so a flat top counts once, at its first point; the lower
  extreme is the same with the smallest value (p2, p3: flat tops and flat bottoms of two and three
  points). A point whose window does not fit in the data is never an extreme, so the first and last
  `npts` points are not (p2: `npts` 5 left out a peak at row 5 that `npts` 3 kept). `npts` is cut to its
  integer part and 0 acts as 1 (p4, p5: 0, 1.2 and 1.7 gave the output of 1); a negative value is
  refused; `npts` so large that nothing is an extreme is refused.
- The envelope is the natural cubic spline through the extremes, evaluated at every X of the
  input; before the first and after the last extreme the end pieces of the spline continue (p1:
  three upper extremes at 4, 12, 23 gave 0.6758 at X 0 and 2.7439 at X 29; with two extremes the
  spline is the straight line through them). At least two extremes of each kind asked for are
  needed: with `both`, a single lower extreme refuses the call even though the upper envelope would
  be fine; `upper` alone needs only two upper extremes, whatever the lower ones are (p2, p3).
- Rows with a missing X or Y are dropped before anything else: the report has fewer rows and the
  extremes are searched in the shortened data, so the neighbours of the gap become neighbours (p11:
  a missing Y at row 6 gave 11 rows; a missing X likewise; missing first and last Y gave 10).
- X order: data whose X rises or falls monotonically are used as they are (a falling X gave the
  same curve per row as the rising one, p8). For any other order the extremes are still searched
  in the row order of the data, but the spline goes through the points of the X-sorted data at those
  positions, so one position swapped between rows (X 5 before 4) swaps the point used (p3, p9: three
  shuffles matched this to 1e-14). The report keeps the input row order.
- Report sheet: for `both`, four columns `Envelope X 1`, `Envelope Y 1` (upper), `Envelope X 2`,
  `Envelope Y 2` (lower); with `upper` or `lower` alone only that pair, and the lower pair keeps the
  numbers 2 (p6). The X columns repeat the input X. The comments read `Upper Envelope of "Name"`,
  `Lower Envelope of "Name"` with the long name of the input in quotes, or the short name without
  quotes when it has no long name (p5, p6). No units.

Sheaf: `sheaf_calc::envelope` (`extreme_points`, `natural_spline`), `Tool::Envelope(EnvelopeSpec)`, the
curves of each input (X and Y of each envelope) in a new sheet named `Envelope...`; the Envelope
window has Type, Method (fixed) and the points on each side. Command line: `sheaf analyze envelope
FILE --y B [--type upper|lower|both] [--npts N]`. Not measured or not built: other methods (Origin has
only the spline one in this version), repeated X values at two extremes (refused by Sheaf, not
tried in Origin), the default workbook name of a call without `rd`, non-numeric input, several input columns (each
pair is named Envelope X 1/Y 1 again) and row ranges.

### 13.3 Coherence (`cohere`)

Analysis > Signal Processing > Coherence. Options (from the X-Function file): `ix1` and `ix2` (the two
signals), `interval`, `fftlen` (combo 256, 512, 1024; default 256), `win` (rect, welch, tri, bartlett,
hanning, hamming, blackman, gauss, kaiser), `winlen` (default 256), `overlap` (default 128), `alpha`
(Gaussian) and `beta` (Kaiser), and `oy` (an X and a Y column). Observed (probe files in
`re-workspace/scratch/coherence/`: `c1` to `c5` with the `.lines.txt` scripts and `.out.txt`
printouts; 64 points of two correlated random signals, a few 20 point signals for missing values;
Rust tests in `calc/coherence.rs`, all to 1e-9, in practice 1e-15):

- The result is the magnitude-squared coherence `|Sxy|^2 / (Sxx Syy)` with the spectra summed over
  the segments (Welch's method, no averaging factor needed). Rows: `fftlen / 2 + 1` (odd lengths:
  `fftlen / 2` rounded down plus 1; 15 gave 8, 31 gave 16) with the frequency `k / (fftlen * interval)`.
- Segments of `winlen` points start every `winlen - overlap` points; only whole segments are used
  (64 points, window 20, overlap 7: four segments, the partial fifth dropped). Each is multiplied by
  the window, zero padded to `fftlen`, transformed; no detrending. Data shorter than one segment is one
  zero padded segment, so the coherence is exactly 1 (the default 256/256/128 on 64 points gave 129 rows
  of 1).
- Windows are the symmetric ones of the FFT tool (Welch, triangular, Bartlett, Hanning, Hamming,
  Blackman, 16 points checked), the Gaussian `exp(-(alpha (i - c) / c)^2 / 2)` with `c = (winlen - 1) / 2`
  (alpha 1, 2.5 and 3; alpha 0 is flat), and the Kaiser `I0(beta sqrt(1 - (2i/(winlen-1) - 1)^2)) /
  I0(beta)` (beta 4; beta 0 is flat).
- Signals of unequal length: the shorter one is padded with zeros to the longer (41 against 64 points,
  both ways round). A missing value counts as 0 (inside or at the start; dropping them or compacting
  the column gave other numbers).
- The interval defaults to 1 and is not taken from the X column, even as an XY range or with
  `interval:=<auto>` (the frequency step stayed 1 / fftlen for an X step of 0.5). Only a given interval
  scales the frequencies.
- Where a spectrum is zero (a signal of zeros) the coherence is missing (NaN in the output). A signal
  with itself gives 1.
- Refused: `winlen` above `fftlen`, `winlen` of 1, `overlap` of `winlen` or more, a negative overlap.
  `winlen` of 2, overlap 0 and overlap `winlen - 1` work.

Sheaf: `sheaf_calc::coherence`, `Tool::Coherence(CoherenceSpec)`: each input column against a second
column (chosen in the window; may be an input), the frequency and the coherence in a new sheet. The
second column is cut to the same rows as the input range. Command line: `sheaf analyze coherence
FILE --y B --with C [--fft-len N] [--win-len N] [--overlap N] [--window rectangle|welch|triangular|
bartlett|hanning|hamming|blackman|gaussian|kaiser] [--alpha A] [--beta B] [--interval D]`.

Not measured or not built: the Origin result graph, the output into existing columns, the
interval taken from X (Sheaf follows what Origin does, 1), other detrending, and the phase or the
cross spectrum.
### 13.4 Decimation (`decimate`)

Analysis > Signal Processing > Decimation. Options (from the X-Function file): `iy` (X and Y
range), `d` (Resample Factor, default 2; smaller factors work),
`filter` (`none` No Filter, `ave` Moving Average Filter, `fir` FIR Filter; default none), `order`
(FIR order, default 30), `plot` (preview, no data), `oy` (new X and Y columns, "Decimated X1" and
"Decimated Y1" by default) and `newsize` (an output value: assigning it from LabTalk is refused).
Observed (`re-workspace/scratch/decimation/p1..p11.out.txt`; p1/p2/p3 which factors are refused on
10 to 200 points, p4 the three filters, p5 and p6 impulse responses, p7 to p9 the FIR order, p11 the
fixture; earlier `re-workspace/scratch/signal/sig1.out.txt` and `sig2.out.txt`; Rust tests in
`crates/calc/src/decimate.rs`, 1e-12):

- Result: `ceil(n / d)` points, point `k` is row `k * d` of the input, X and Y both (X values need
  not be even). Factor 1 copies the data.
- Refused (the X-Function returns false, no output): fewer than 2 points; a factor of 3 or more with
  fewer than `10 * d` points (30 points allow 3, 29 do not; 59 points allow 5, not 6; 120 allow 12,
  119 do not); a FIR order below 8 or above 60 (8 and 60 accepted, 7 and 61 refused, at 5 to 200
  points; the points do not matter). Factors 1 and 2 are accepted from 2 points up. This is why the
  earlier 11 and 12 point probes refused 3, 4 and 5, and why the values seen after the refused
  calls were the leftovers of the call before.
- `ave`: the triangle with weights `1, 2, .., d + 1, .., 2, 1` over `(d + 1)^2` centred on the kept
  row (impulse responses, d = 2 and 3, 60 points: 1/9, 2/9, 3/9 and 1/16, 2/16, 3/16, 4/16), that
  is the moving average of `d + 1` points applied twice; rows before the start count as 0 (the
  first value of the ramp 1..20 at d = 2 is 1.111, the middle ones are exact). The filter is not
  centred at the end: the kept row whose window reaches past the last row loses the weight of the
  missing rows (60 points, d = 3: the last value has weights 1, 2, 3, 3, 2, 1 over 16 on the last
  six rows, a total of 12/16, not 1, 2, 3, 4, 3, 2, 1). Equivalent: the second moving average reads
  the first only up to the last row, `a[t] = 0` for `t >= n`. Checked on 61 to 100 points with
  factors 3 to 7, where the last kept row is anywhere in a window.
- `fir`: the same triangle with half width `order` instead of `d`, whatever the factor (impulse
  response for d = 1, 2, 3, 5 at order 30: peak `31 / 961`; order 10: peak `11 / 121`), same start
  and end rule. Checked against Origin for orders 8 to 12, 50, 58 to 60 and the default 30 on up to
  100 points. The name FIR is Origin's; the filter it computes is this triangle, there is no
  designed low pass in the numbers.
- `fir` with `d` times the X step at 0.5 or below (probes p12 to p18; fresh books, the first call,
  default order and `order:=30`, and an output sheet made beforehand all agree, so the earlier p4
  values were not a leftover): Origin does not return the triangle. For 60 points at X step 0.5,
  `d = 1`, the values start 3.8274402148869662, 5.6599095268221635, 7.2131135270130722 (the
  triangle: 4.999, 5.297, 5.570). The kernel is symmetric over the same 61 rows, sums to 1, has a
  non-negative frequency response above 1 in places, and a centre weight of 5/31 at step 0.5, 13/31
  at 0.25, 33/31 at 0.1 (31/961 = 1/31 at step 1 and above); it depends on `d` times the step (`d = 2`
  at 0.25 gives the same values as `d = 1` at 0.5 at every other row). Above 0.5 (0.501, 0.51, up to
  2, `d` 1 to 3) the result is the triangle exactly. The moving average has no such case. The
  oscillating kernel is not worked out, so Sheaf refuses FIR there (an error, no values) instead of
  returning the triangle; the dialog and command line show the same message.
- Not measured: missing values inside the input (the probes could not make a blank cell through
  LabTalk; Sheaf refuses them, as in section 13), and `plot`.

Sheaf: `sheaf_calc::decimate` (`DecimateOptions`, `DecimateFilter`, `DecimateError`),
`Tool::Decimate(DecimateSpec)`. The result goes to a new sheet `Decimate<n>` with the kept X (the
input's X name with "Decimated " in front, same unit) and Y per input; the Y comment says which point
and filter. Origin's refusals are reported as errors with the same limits. Window: Analysis > Signal
Processing > Decimation... (Resample Factor, Filter Type, Order). Command line: `sheaf analyze
decimate FILE --y B [--factor D] [--filter none|ave|fir] [--order N]`.

## 14. Level crossing (`levelcrossing`)

Origin has the `levelcrossing` X-Function (Data Manipulation) but no menu item for it; Sheaf puts it
under Analysis > Mathematics > Level Crossing. Inputs (from its X-Function file): the curve `iy`,
`level` (0), `type` (Any, Left, Right: the row given for a crossing between two rows), `pts` (1: both
crossings of a pair closer than this many rows are dropped) and the outputs: X of each crossing
(`oy`, an X column), `direction` (1 up, -1 down, 0 "derivative zero"), `xindex` (a Label column)
and `yval` (the level).

Observed (Origin, `re-workspace/scratch/levelx/lx.out.txt` and `re-workspace/scratch/lcross/lc2.out.txt`,
`lc3.out.txt`, `lc4.out.txt`; Rust tests `level_crossings_match_origin` and `more_cases_match_origin` in
`crates/calc/src/levelcross.rs`):

- A data point exactly on the level is a crossing at its own X and row. Its direction is the sign
  of the steps before and after it when they agree, else 0: a touch (1, 0, 1 at level 0) and each
  point of a run on the level (-1, 0, 0, 3) have direction 0. At the first and the last point the one
  step there decides (0, 1, ... gives 1; ..., 0, 1 at level 1 gives 1).
- Between two points on opposite sides the crossing is interpolated linearly, direction of the step
  in row order (also with falling X: X 10, 8, 7, 3, 0 and Y 0, 2, 0, 2, 0 at level 1 give 9, 7.5, 5, 1.5
  with 1, -1, 1, -1).
- A missing Y is skipped and its neighbours are joined: 0, 2, missing, 0, 2 at level 1 gives 0.5, 2 and
  3.5 (the crossing at 2 is not lost).
- The row (1-based) for an interpolated crossing: Left the row before, Right the row after, Any the
  row whose X is nearest, counting rows whose Y is missing (the crossing at 2 above is row 3, the
  missing row), and on a tie the one with the smaller X (0.5 between X 0 and 1 is row 1; 9 between
  X 10 and 8 is row 2).
- `pts`: with indices 2, 4, 8, 9 and `pts` 3 all four were dropped; with 1, 2, 3, 4, 7 and `pts` 2
  only 7 was left. Sheaf takes the crossings in pairs in order (first with second, third with
  fourth); a sliding comparison would fit these two cases as well.
- No crossing: empty outputs (Origin still wrote the level once into `yval`; Sheaf writes nothing).
- Default output (no output columns named): only the X values, in a new column at the end of the input
  sheet with long name `CrossingX1` (then `CrossingX2`, ... for later runs on that sheet), the comment
  `X values of Y= 1.5 Level Crossing of B` (the level as typed: 1, 0.25, -3) and no units. The column
  is added even when the curve never reaches the level. Origin writes the direction, index and Y values
  only into columns that the script names.

Sheaf: `sheaf_calc::level_crossings`, `Tool::LevelCrossing(LevelCrossingSpec)`; the `CrossingX<n>`
column as Origin, beside its input (ADR 0005) instead of at the end of the sheet. The switch "Also output
directions, X indices and Y crossing values" (`details`) adds `Directions<n>`, `X Indices<n>` (rows of
the sheet) and `Y Crossing Values<n>` (with the input's units) beside it; these names are Sheaf's, from
the labels of Origin's outputs. Origin's index for a row range that does not start at row 1 was not
measured. Command line: `sheaf analyze level-crossing FILE --y B [--level L] [--row any|left|right]
[--min-gap N] [--details]`.

## 15. FFT (`fft1`)

Analysis > Signal Processing > FFT > FFT. Options (from the X-Function file): `interval` (`<Auto>`),
`win` (Rectangle, Welch, Triangular, Bartlett, Hanning, Hamming, Blackman), `correct` (None,
Amplitude, Power; Amplitude by default), `norma` (normalize, off), `shift` (on), `unwrap` (on),
`factor` (-1 Electrical Engineering, +1 Science), `st` (auto, one-sided, two-sided), `norm` (MSA,
SSA, TISA) and the report sheet. Origin's result sheet has Frequency, Complex, Real, Imaginary,
Magnitude, Amplitude, Phase, Power and dB; Sheaf writes the same without the complex column.

Observed (Origin, `re-workspace/scratch/fft/f1..f4.out.txt`, 5, 7, 8 and 9 points; Rust tests in
`crates/calc/src/fft.rs`, relative 1e-9):

- Real and imaginary parts are the plain DFT `X_k = sum x_n e^(-2 pi i k n / N)` (Science: the
  imaginary part and the phase change sign). Magnitude `|X|`; Normalize divides real, imaginary and
  magnitude (only) by N.
- One-sided (the default for real data): rows k = 0 to N/2 (integer division), frequency
  `k / (N * interval)`. Amplitude `2|X|/N`, except `|X|/N` in the first and the last row (also for an
  odd N, where the last row is not the Nyquist frequency). Power (MSA) `2|X|^2/N^2`, except
  `|X|^2/N^2` in the first row and `4|X|^2/N^2` in the last row for an odd N (5, 7 and 9 points).
- Two-sided: all N rows, amplitude `|X|/N` and power `|X|^2/N^2` everywhere; with Shift the rows
  run from `-(N/2 - 1)` (even N) to the Nyquist frequency, else from 0 to `(N-1)/(N*interval)`.
- SSA is MSA times N, TISA is SSA times the interval. dB is `20 log10(amplitude)`.
- Phase in degrees from atan2, unwrapped in row order (after the shift): a step of more than 180
  degrees is corrected by multiples of 360, so 0 can become 360 in a shifted two-sided result.
- Windows, symmetric over N points (c = (N-1)/2): Welch `1 - ((n-c)/((N+1)/2))^2`, Triangular
  `1 - |n-c|/(N/2)`, Bartlett `1 - |n-c|/c`, Hanning `0.5 - 0.5 cos(2 pi n/(N-1))`, Hamming
  `0.54 - 0.46 cos(...)`, Blackman `0.42 - 0.5 cos(...) + 0.08 cos(4 pi n/(N-1))`. Correction
  Amplitude multiplies by `N / sum w`, Power by `sqrt(N / sum w^2)`.
- `<Auto>` interval: called from LabTalk with a column (`col(B)`, a range `[Book]Sheet!B`) Origin
  used 1 although the sheet has an X column with step 0.5; an XY range `(1,2)` is read as real and
  imaginary parts, not as X and Y. Sheaf's Auto takes the mean X step of the input's X column (1
  without one), as Convolution's measured `<auto>` does; the dialog's own behaviour is not measured.

Sheaf: `sheaf_calc::fft_spectrum`, `Tool::Fft(FftSpec)`; a new sheet `FFT<n>` with the eight columns
per input (Amplitude in the input's unit, Phase in degrees). Trailing missing values are dropped, a
missing value inside is refused. Command line: `sheaf analyze fft FILE --y B [--interval D]
[--window ...] [--correction none|amplitude|power] [--normalize] [--two-sided] [--no-shift]
[--no-unwrap] [--science] [--power msa|ssa|tisa]`. Not built: complex input, IFFT, 2D FFT, the
graphs Origin makes with the result.

### 15.1 IFFT (`ifft1`)

Analysis > Signal Processing > FFT > IFFT. Options: the input `ix` (real and imaginary parts in
two columns, or one), `interval` (`<Auto>`), `shift` (on), `win`, `correct`, `factor`, `unwrap`.
Observed (`re-workspace/scratch/fft/i1.out.txt`, the two-sided unshifted FFT of eight points; Rust
test `inverse_fft_matches_origin`):

- The result sheet has Time, Real, Imaginary, Amplitude (`|x|`) and Phase (degrees, unwrapped).
  `x_n = (1/N) sum X_k e^(2 pi i k n/N)`; Science uses the other sign (the signal comes out in
  reverse order after the first point).
- The interval is the step of the spectrum; the time step is `1 / (N * interval)` (interval 1
  gave 0.125 for eight points, 0.25 gave 0.5).
- Shift on (the default) treats the input as shifted the way FFT writes it and puts it back in FFT
  order first; on an unshifted spectrum this gives a rotated (complex) result, as Origin's does.
- Only the real part given: the imaginary part is 0.
- The default window gave the same numbers as Rectangle; windows are not offered in Sheaf's IFFT.
- Unwrap: a step of exactly -180 degrees gains 360 (0, 180, 0 became 0, 180, 360) but +180 is kept;
  Sheaf's FFT uses the same rule. A portability regression (2026-11-14) showed Linux
  returning -179.99999999999997 for a half-turn. The shared unwrap routine reduces steps
  modulo 360 and treats values within `8 * f64::EPSILON * 180` degrees of a half-turn as
  the positive tie. This is a floating-point guard, not an altered Origin fixture: the
  existing FFT/IFFT expectations stay unchanged, and adjacent representable values plus
  genuinely distinct nearby steps are regression-tested.

Sheaf: `sheaf_calc::ifft_spectrum`, `Tool::Ifft(IfftSpec)`; the input columns are real parts, the
imaginary part a chosen column read row by row (the window suggests the column right after the
input, as in an FFT result sheet). Auto takes the X step of the input (the frequency step of an FFT
result). Command line: `sheaf analyze ifft FILE --y Real [--imaginary Imag] [--interval D]
[--no-shift] [--no-unwrap] [--science]`.

### 15.2 FFT Filters (`fft_filters`)

Analysis > Signal Processing > FFT Filters. Options (from the X-Function file): `filter` (0 Low
Pass, 1 High Pass, 2 Band Pass, 3 Band Block, 4 Threshold, 5 Low Pass Parabolic), `cutoff`, `freq1`,
`freq2`, `threshold`, `pass`, `stop` (all automatic by default) and `offset` (keep the DC offset, on;
high pass, band pass and band block). Observed (`re-workspace/scratch/fftfilt/ff1..ff4.out.txt`, 10,
16, 20, 32 and 40 points; the gain of every frequency was read off by dividing the FFTs; Rust test
`fft_filters_match_origin` on nine of the cases):

- Each frequency `f = k * step` (both signs, `step = 1 / (N * interval)`, the interval from the X
  column: X step 0.5 doubled the frequencies) is kept or removed, and the real part of the inverse
  transform is the result. Low pass keeps `f <= cutoff`; high pass `f >= cutoff`; band pass
  `freq1 <= f <= freq2` (`6 * 0.05` lies just above 0.3 in floating point and was removed, so `f`
  is the product, not `k / (N * interval)`).
- Band block removes `freq1 <= f < freq2 + step`: one frequency more than band pass keeps whenever
  freq2 is not itself a frequency (0.12 to 0.28 removed 0.15 to 0.30 of 20 points; 0.1 to 0.3 of 16
  points removed up to 0.3125).
- Keep DC offset: the zero frequency stays in high pass, band pass and band block; off, it is removed
  too.
- Threshold keeps the frequencies whose magnitude `|X|` (unscaled FFT) is at least the threshold.
- Low Pass Parabolic weights `1` up to `pass`, `1 - ((f - pass) / (stop - pass))^2` up to `stop`, 0
  above. In two cases the one frequency at a quarter of the sampling rate came out a little
  different from this (0.31 and 0.434 instead of 0.4375, with a small imaginary part); not copied.
- Automatic values: cutoff (low and high pass), freq1, pass = a quarter of the Nyquist frequency;
  freq2, stop = three quarters; threshold = a tenth of the largest `|X|`, zero frequency included
  (designed spectra: largest 100 removed up to 9 and kept 10; largest 50 removed up to 4).

Sheaf: `sheaf_calc::fft_filter`, `Tool::FftFilter(FftFilterSpec)`, a new column beside each input
(or a new sheet or existing columns, the Output node). Command line: `sheaf analyze fft-filter FILE
--y B [--type low-pass|high-pass|band-pass|band-block|threshold|parabolic] [--cutoff F] [--freq1 F
--freq2 F] [--threshold T] [--pass F --stop F] [--no-offset] [--interval D]`.

## 16. Trace Interpolation (`interp1trace`)

Analysis > Mathematics > Trace Interpolation. The X-Function has `iy` (the XY range), `method`
(`linear`, `spline` Cubic Spline, `bspline` Cubic B-Spline; default linear), `npts` (number of
points, default 100) and `oy` (new X and Y columns). Observed (probe files in
`re-workspace/scratch/trace-interp/`: `p1`, `p2`, `p3`, `p4`, `p5`, `q1..q3`, `r1`, `s1..s3`, `t1..t3`
and `p_nan3`, each a `.lines.txt` script and its `.out.txt`; Rust tests in `calc/src/trace.rs` hold
Origin's printed numbers):

- The result has `npts` points evenly spaced along the path of the curve, the first and last being
  the first and last points of the (cleaned) data. The path parameter is the cumulative chord
  length. Linear joins the points with straight lines; Cubic Spline is a natural cubic spline of X
  and of Y against the path length; Cubic B-Spline is the interpolating cubic spline with
  not-a-knot ends (equal to scipy's `make_interp_spline`; the two spline kinds agree to 1e-15 with
  the computation, all three methods on a loop of eight points).
- Origin cleans the data first, and this is what makes a loop with straight X or Y runs look odd:
  rows with a missing X or Y are dropped; then neighbouring points with the same X are replaced by
  one point at the mean of X and Y; then neighbouring points with the same Y likewise (X first;
  (3,0),(3,2) became (3,1) and only then (0,0),(1,0) became (0.5,0); a merge by Y that makes a new
  equal X is not merged again). Only neighbours merge: the same X far apart in the path stays.
  "The same" has a tolerance of 1e-8 times the range of that column (5e-8 apart in a range of 9
  merged, 1e-7 did not; 5e-6 in a range of 504 merged, 1e-5 did not; the range is not the data's
  magnitude: shifting the data by 1000 changed nothing).
- No result (the sheet is not made) for `npts` of 0 or 1, for fewer than two points after the
  cleaning (a constant Y column becomes one point), and for the B-spline with fewer than four
  points. Cubic Spline with two points is the line, with three it is the natural spline. A
  non-integer `npts` (5.7) is cut to 5.
- A constant-X (vertical) curve fails in Origin too (measured 2026-11-21, T326); Sheaf merges it to one
  point and refuses it. Not measured: whether the tolerance compares with the previous point or the
  first of a run; and an X range given with a row range.
- Output (measured 2026-11-21, T326, `re-workspace/scratch/t326`): `InterpolatedX1` (X) and
  `InterpolatedY1` (Y, comment "Linear trace interp of B"; "Spline" and "B-Spline" for the other
  methods) at the end of the input sheet; each run goes on with 2, 3, ...

Sheaf: `sheaf_calc::trace_interp`, `Tool::TraceInterp(TraceInterpSpec)`: since T326 the X and Y of each
input go to the end of the input sheet as in Origin (names and comments as above); "New sheet" in the
window's Output still writes them to a sheet of their own. Command line:
`sheaf analyze trace-interp FILE --y B [--method linear|spline|bspline] [--points N]`.

## 17. IIR Filter and FIR Filter (`dfilter`, the FIR Filter app; T55)

Nothing in this section was measured in Origin: no Origin was available, and neither tool has a
reference script in `tools/origin-ref/` yet. The facts below are from Origin's help pages and the File
Exchange page of the FIR app, quoted as paraphrase [W:x-function/ref/dfilterpro,
W:x-function/ref/dfilterapply, W:origin-help/iir-filter-dialog, W:origin-help/iir-filter-algorithm,
W:tutorials/iir-filter, fileExchange fid=1077]. Everything Origin does not say is Sheaf's choice, labelled
"Sheaf's choice, unmeasured". The numbers of the designs and the outputs are checked against
`scipy.signal` (`crates/calc/tests/scipy_filters.rs`, from `tools/iir-fir-ref.py`, scipy 1.18.1), not
against Origin.

### 17.1 IIR Filter (OriginPro, Analysis > Signal Processing > IIR Filter)

Origin facts (docs, not measured):

- The X-Function is `dfilter` (Pro), brief "Create and apply an IIR filter"; the command line example
  is `dfilter method:=cheby1 order:=5 sample:=1000 wn:=50 filtfilt:=1 coef:=[<input>]<new>;`. (The
  name `iirfilter` of the T55 brief is not an Origin X-Function.) `dfilterapply` applies a designed
  filter given as an SOS matrix or as numerator and denominator coefficients.
- Variables and defaults: `iy` input signal; `type` response type `low`, `high`, `bandpass`, `bandstop`
  (default low); `method` `butter`, `cheby1`, `cheby2`, `elliptic` (default Butterworth); `order`
  10; `unit` `hz` or `normalize` (default Hz); `sample` Fs, `<auto>`; `wn` cutoff (low and high pass),
  `wn1` and `wn2` (band pass and band stop), all `<auto>`; for the minimum order `wp`, `ws` (or `wp1`,
  `wp2`, `ws1`, `ws2`); `rp` passband ripple 1 dB; `rs` stopband attenuation 80 dB; `filtfilt`
  forward-backward filtering, off; outputs `sos`, `zpk`, `abcd` (state space), `coef` (numerator and
  denominator) and the filtered signal `oy`. Normalized frequency is `f / (Fs / 2)`, so 1 is the Nyquist
  frequency. Fs is "auto-detected from the associated X column if one exists".
- The dialog: Response Type, Method (Butterworth, Chebyshev Type I, Chebyshev Type II, Elliptic; Bessel is
  not offered), Filter Order with a Minimum checkbox (unchecked, an integer; the tutorial unchecks it to
  set 4, which suggests Minimum is on by default), Frequency Specification (Unit, Sample Frequency,
  Pass, Stop and Cutoff frequencies depending on response, method and unit), Magnitude Specification
  (dB), Forward-Backward Filtering, Output Results (SOS matrix, zeros/poles/gain, state space,
  transfer function coefficients, output signal), and preview panels (filter specification, magnitude
  and phase response, impulse response, the filter applied to the data with cutoffs that can be dragged).
  The tutorial's output is a new filtered column and an "SOS Matrix" worksheet.
- The algorithm: a normalized analog low pass prototype (Butterworth, Chebyshev I or II, elliptic),
  the frequency transform to the response type (low pass `s/w0`, high pass `w0/s`, band pass and band stop
  with `w0 = sqrt(w1 w2)` and `B = w2 - w1`), then the bilinear transform. SOS is a gain times `L`
  biquads (`L x 6`, three numerator and three denominator coefficients). Forward-backward filtering
  "results in zero phase distortion".
- Not documented: pre-warping, the order estimation formulas, how `rp` and `rs` enter each prototype,
  initial conditions, padding and edge handling (one way or forward-backward), whether one-way
  filtering is a direct form or an SOS cascade, how Fs is computed from the X column, unevenly spaced
  X, missing values.

Sheaf (`sheaf_calc::iir`: `IirSpec`, `IirFilter`, `iir_filter`, `iir_order`, `sosfilt`, `sosfiltfilt`):

- Methods, responses and options as above with Origin's defaults (Butterworth, low pass, order 10, ripple
  1 dB, attenuation 80 dB, Hz, no forward-backward filtering). Bessel is not built, as Origin lists no
  Bessel filter. The order is limited to 200.
- Sheaf's choice, unmeasured: the frequency pre-warping `tan(pi f / Fs)` (scipy's design chain: analog
  prototype, `lp2lp`/`lp2hp`/`lp2bp`/`lp2bs`, bilinear transform; zeros, poles and gain, numerator and
  denominator, agree with scipy to 1e-11 for all four methods and responses).
- Sheaf's choice, unmeasured: what the cutoff frequency (Fc, `wn`) is depends on the method, as in scipy.
  Butterworth: the frequency where the gain is down by 3 dB. Chebyshev I and elliptic: the passband edge,
  where the gain is down by the passband ripple `rp` (the gain at the edge is `10^(-rp/20)`). Chebyshev II:
  the stopband edge, where the gain is down by the stopband attenuation `rs` (so with `rs` 80 dB the gain at
  Fc is already -80 dB, and the passband is narrower than a Butterworth filter with the same Fc). The dialog
  says so under the frequencies; the column comment prints "Fc" whatever the method. Origin does not say.
- Sheaf's choice, unmeasured: the passband gain follows scipy. Butterworth, Chebyshev II, and odd order
  Chebyshev I and elliptic low pass filters have gain 1 at DC; an even order Chebyshev I or elliptic low
  pass filter has gain `10^(-rp/20)` at DC (0.891 for `rp` = 1 dB), not 1. The passband gain of Chebyshev I and
  elliptic filters ripples between `10^(-rp/20)` and 1. Nothing rescales the output to unit gain. Origin's
  convention is not documented and not measured.
- Sheaf's choice, unmeasured: designs of any order up to 200 keep a finite, non-zero gain: the gain is the
  real part of a ratio of products of the zeros and poles, taken with a scaled running product so that it
  does not overflow where the products pass 1e308 (order 100 band pass at 0.3 to 0.6 of Nyquist; checked
  against scipy up to orders 190). A design whose gain would be 0, infinite or NaN is an error ("the filter
  design gave values that are not finite"; a low pass at 0.95 of Nyquist of order 181 or more
  is one, where scipy raises an OverflowError). The checks are in `high_order_iir_designs_match_scipy`.
- Sheaf's choice, unmeasured: the minimum order and the natural frequencies follow scipy's `buttord`,
  `cheb1ord`, `cheb2ord` and `ellipord` (order equal to scipy's on the test cases; the cutoffs of a band
  stop filter agree to 1e-4 only, because scipy moves the pass edges with `fminbound` and Sheaf with a
  golden section search). The passband ripple and stopband attenuation are the specification of the
  minimum order; for a fixed order only Chebyshev I (`rp`), Chebyshev II (`rs`) and elliptic (both) use
  them. Sheaf's default Minimum is off (the X-Function default is the fixed order 10).
- Sheaf's choice, unmeasured: automatic frequencies are a quarter of the Nyquist frequency (low and
  high pass cutoff, band pass and band stop Fc1) and three quarters (Fc2); for the minimum order low
  pass `0.2`/`0.3`, high pass `0.3`/`0.2`, band pass `0.3`, `0.5` against `0.2`, `0.6` and band stop
  `0.2`, `0.6` against `0.3`, `0.5` (pass/stop, fractions of the Nyquist frequency).
- Sheaf's choice, unmeasured: Fs is `1 / (mean X step)` (the same step as the FFT filters, 1 without
  an X column); unevenly spaced X is not checked. Frequencies at or above Nyquist, band edges out of
  order, a stopband attenuation not above the ripple, and an order outside 1 to 200 are errors.
- Sheaf's choice, unmeasured: one-way filtering is `sosfilt` with zero initial state (a signal that
  starts away from zero shows the start-up transient of the filter); forward-backward filtering is
  `sosfiltfilt` (odd extension of `3 * ntaps` points at both ends, the step response as initial
  state), which needs more than `3 * ntaps` points (`ntaps = 2 * sections + 1`, an error otherwise).
  The two agree with scipy's `sosfilt` and `sosfiltfilt` on the cases of the test (1e-9 of the range).
- Sheaf's choice, unmeasured: the second-order sections are paired by the pole closest to the unit
  circle last, each with the nearest zeros, the gain in the first section. This is not scipy's `zpk2sos`
  pairing, so the sections are compared through their response and their output.
- Not built: the state space output, the SOS / ZPK / coefficient worksheets, `dfilterapply`, `dfilterfcf`,
  the preview panels. Calc has the zeros, poles, gain, coefficients and sections (`IirFilter`).
- Missing values: as the other signal tools, trailing blanks are dropped and a blank inside the signal is
  an error.

Sheaf: `Tool::IirFilter(IirSpec)`; a new column beside each input (or a new sheet or the existing
columns, the Output node) named "<name> IIR filtered", the unit of the input, the comment "Butterworth
low pass IIR filter of B (order 4, Fc 50 Hz, forward-backward)". The tool can follow its input
(Recalculate Auto and Manual, `LinkTool::IirFilter`) and a project saved as `.opj` names such a result in
the save warnings, as for Smooth. Window: Analysis > Signal Processing > IIR Filter... (X-Function name
`dfilter`). Command line: `sheaf analyze iir-filter FILE --y B [--type low-pass|high-pass|band-pass|band-stop]
[--method butterworth|chebyshev1|chebyshev2|elliptic] [--order N | --minimum] [--unit hz|normalized] [--fs HZ]
[--cutoff F] [--cutoff2 F] [--pass F --pass2 F --stop F --stop2 F] [--ripple DB] [--attenuation DB]
[--forward-backward]` (alias `iir`; MCP `iir_filter`).

### 17.2 FIR Filter (the FIR Filter app of Origin 2027, Analysis > Signal Processing)

Origin facts (File Exchange page of "FIR_Filter.opx" v1.00, OriginPro, Origin 2027; docs, not measured; no
page on docs.originlab.com was found):

- "Create and apply an FIR (Finite Impulse Response) digital filter." Response Type: lowpass, highpass,
  bandpass, bandstop. Method: Least Squares (linear phase) or Window (window-based; no Parks-McClellan).
  Filter Order: an integer; for the Window method a Minimum checkbox computes it "using the Kaiser
  window". Window (Window method): Bartlett, Bartlett-Hanning, Blackman, Blackman-Harris, Bohman,
  Chebyshev, Flat Top, Gaussian, Hamming, Hanning, Kaiser, Nuttall, Parzen, Rectangle, Taylor,
  Triangular, Tukey, or a window given as data. Scale Passband: normalizes the coefficients so that the
  passband centre has magnitude 1 (0 dB). Frequency Specification: Hz or normalized (0, 1) with
  `f / (Fs / 2)`, Fs detected from the X column (Auto) or given; pass, stop and cutoff frequencies by
  response, method and unit. Magnitude Specification: passband and stopband weights (Least Squares);
  dB or linear, passband ripple and stopband attenuation (Window with the minimum order).
  Forward-Backward Filtering: filtering "in both forward and backward directions". Outputs: the
  coefficients (transfer function) and the output signal; previews as for the IIR dialog.
- Not documented: defaults (order, window, response, Fs, ripple, attenuation), the Kaiser beta and width
  parameters, the Least Squares algorithm in detail (band edges, transition band), alignment or
  delay compensation of the one-way output, edge handling, whether Hamming or Hanning is the default.
  The option names mirror MATLAB's `firls`, `fir1` and `kaiserord`; nothing documents the formulas.

Sheaf (`sheaf_calc::fir`: `FirSpec`, `FirFilter`, `firwin`, `firls`, `kaiserord`, `window`):

- Sheaf's choice, unmeasured: the filter has `order + 1` coefficients (as `fir1`); high pass and band stop
  need an even order (an error otherwise); the order is limited to 1000.
- Sheaf's choice, unmeasured: the Window method is `scipy.signal.firwin` (ideal response times the
  symmetric window); the Least Squares method is `scipy.signal.firls` with a constant desired gain in
  each band (1 in the passbands, 0 in the stopbands), the pass weight in the passbands and the stop weight
  in the stopbands, the transition bands left out, generalized to an even number of coefficients
  (which scipy does not offer; the checks against the weighted least squares optimum, solved in numpy by
  quadrature, are `even_tap_least_squares_matches_the_weighted_optimum`). Window values, `firwin` and `firls`
  agree with scipy to 1e-10 (`firls` 1e-8), also for non-default window parameters and for the band and
  weight mapping of every response type (`least_squares_designs_of_every_response_match_scipy_firls`) and
  the minimum-order assembly with Kaiser (`minimum_order_window_designs_match_scipy`, dB and linear).
- A window whose parameter overflows (Kaiser beta of a few hundred, Chebyshev attenuation of thousands of dB,
  a Tukey fraction that is not a number) is an error ("the filter design gave values that are not finite")
  with Scale Passband on or off; before, the unscaled design wrote an empty column.
- Sheaf's choice, unmeasured: the minimum order is `kaiserord` with the ripple `A = -20 log10(min(dp, ds))`
  of the smaller of the passband deviation `dp = (10^(rp/20) - 1) / (10^(rp/20) + 1)` and the stopband
  deviation `ds = 10^(-rs/20)` (the linear unit takes both as deviations), the transition width the
  smaller gap between a pass and a stop frequency, the cutoffs half way between them; below 8 dB the
  formula is refused. The window of that design is the selected one, with Kaiser's beta from the
  attenuation when Kaiser is selected.
- Sheaf's choice, unmeasured: window parameters default as MATLAB's (Chebyshev 100 dB, Gaussian alpha 2.5,
  Tukey 0.5, Taylor 30 dB with 4 bands), Kaiser beta 5; the default method is Window with Hamming and order
  20, Scale Passband on (low pass and band stop at 0 Hz, high pass at Nyquist, band pass at the centre),
  ripple 1 dB, attenuation 60 dB, Fs from the X column as for the IIR filter, automatic cutoffs a quarter
  (three quarters) of the Nyquist frequency, automatic pass and stop frequencies as for the IIR minimum order.
- Sheaf's choice, unmeasured: one-way output is aligned with the input (the delay of `order / 2` points,
  rounded down, removed; the ends extended by an odd reflection of `order` points; an error if the signal
  has `order` points or fewer); forward-backward output is `scipy.signal.filtfilt` of the coefficients (odd
  extension of `3 * (order + 1)` points, so more points than that). Both agree with numpy and scipy to 1e-10.
- Not built: a window given as data, the coefficient output, the previews. Missing values as for the IIR filter.

Sheaf: `Tool::FirFilter(FirSpec)`, a new column beside each input named "<name> FIR filtered" with the
comment "Hamming window low pass FIR filter of B (order 100, Fc 50 Hz)" (Least Squares: "least squares ...
(order 80, pass 50 Hz, stop 100 Hz)"); Recalculate and `.opj` warnings as the IIR filter. Window:
Analysis > Signal Processing > FIR Filter..., placed under IIR Filter (Sheaf's choice: Origin ships the
filter as an app, so its menu place is not known). Command line: `sheaf analyze fir-filter FILE --y B
[--type ...] [--method least-squares|window] [--order N | --minimum] [--window NAME] [--window-param P]
[--no-scale] [--unit hz|normalized] [--fs HZ] [--cutoff F] [--cutoff2 F] [--pass F --pass2 F --stop F
--stop2 F] [--pass-weight W] [--stop-weight W] [--magnitude-unit db|linear] [--ripple R] [--attenuation A]
[--forward-backward]` (alias `fir`; MCP `fir_filter`).

## 18. XYZ Surface Area (`xyzarea`; T91)

Analysis > Mathematics > XYZ Surface Area... gives the area of the surface through scattered (X, Y, Z) points. Measured in Origin 2016 on 2026-11-16 (`tools/origin-ref/t91/xyzarea.lines.txt`, cases in `xyzarea-cases.json`, raw output in `re-workspace/scratch/t91/xyzarea.out.txt`); `xyzarea` runs from a script.

- The X-Function: `xyzarea iz:=(1,2,3) area:=aa` (input XYZ range of three columns; output `area`, a number). It writes **no sheet and no report**: only the number, returned in the variable (the Messages Log was not read). The window was not opened.
- **Method (checked with scipy, not Origin's code): the Delaunay triangulation of the points projected on the XY plane, the surface area is the sum of the areas of the 3D triangles** `0.5 |(p2 - p1) x (p3 - p1)|`. Origin equals `scipy.spatial.Delaunay` + that sum to 1e-15 relative in every case that has a unique triangulation:

  | Case (z) | Points | Origin | Delaunay sum |
  |---|---|---|---|
  | plane `2x + 3y`, 5 by 5 grid on 0 to 1 | 25 | 3.7416573867739404 (exact sqrt(14)) | same |
  | paraboloid `x^2 + y^2`, 6 by 6 grid | 36 | 1.8561937972253828 | 1.8561937972253826 |
  | the same, 11 by 11 grid | 121 | 1.8602226321628541 | 1.8602226321628539 |
  | one triangle (0,0,0), (1,0,0), (0,1,2) | 3 | 1.1180339887498949 | sqrt(5)/2 |
  | flat unit square | 4 | 1 | 1 |
  | `sin(3x) y + x` on 25 random points of the unit square (seed 7) | 25 | 1.1947142916930125 | 1.1947142916930125 |

  (The grid cases are cocircular: both diagonals of every cell give the same area for these symmetric surfaces, so they do not show which diagonal Origin takes.)
- **Duplicate (x, y) points are merged by averaging their Z**: the points (0,0,0), (1,0,1), (0,1,2), (1,1,3) and (1,1,5) gave 3.09557356477856, which is the area of the square surface with z(1,1) = 4 (1.2247449 + 1.8708287); a triangulation that kept z = 3 gives 2.4494897 (what scipy returns) and z = 5 gives 3.774 or 3.992.
- Too few or degenerate points (two points; four collinear points in XY): no result, the output variable is left unset (the `area` argument keeps its previous value) and the call returns false; the message was not read.
- Not measured: the window (title, controls: the input range selector, the Output field), whether the result also goes to the Results Log or a sheet when run from the menu, the number of decimals shown, the diagonal choice on cocircular points, tolerance for nearly duplicate points, missing values in the input, the maximum point count, and a Z column not designated Z.

**Built (T91).** `sheaf_calc::xyzarea` (Delaunay by Bowyer-Watson, checked against the convex hull area; `tests/origin_xyzarea.rs` reproduces the seven measured areas to 1e-12), tool `Tool::XyzArea`, window and `sheaf analyze xyz-area`.

**Sheaf's choice** until the window is measured: Analysis > Mathematics > XYZ Surface Area... uses the XYZ columns of the active sheet (X, Y, Z designations as the surface fits do, fitting.md 14), runs the Delaunay triangulation (`sheaf-calc`, new module, one triangulation also usable by the gridding work that other packages plan) and writes a small results sheet or log line with the area, the number of points used and of triangles, and the number of merged duplicates; `sheaf analyze xyz-area` prints the number; rows with a missing X, Y or Z are skipped and counted.

## 19. Wavelets (`dwt`, `idwt`, `wtdenoise`, `wtsmooth`, `cwt`; T96)

Analysis > Signal Processing > Wavelet holds five items, each one an X-Function of OriginPro 2016 (names from the
X-Function files in `Program Files/OriginLab/Origin2016/X-Functions/Signal Processing/Wavelet`, the labels from
their descriptions): **Decompose...** = `dwt` (one level of the 1D discrete wavelet transform), **Reconstruction...** =
`idwt`, **Denoise...** = `wtdenoise`, **Smooth...** = `wtsmooth`, **Continuous Wavelet...** = `cwt`. The folder also has
`mdwt` (Multi-Scale DWT, several levels into a report sheet), `dwt2`, `idwt2` (2D) and `cw_evaluate`; none of these is
in the menu list of the inventory, so T96 does not build them. All five scripted fine (`run-labtalk-safe.ps1`, each
called with named arguments); the windows were opened with `origin-dialog.ps1 -Command "<name> -d;"`. Measured on
2026-11-16 in Origin 2016; generators and a fixture file (`t96-origin.json`, inputs and Origin outputs) are in
`tools/origin-ref/t96/`, raw output in `re-workspace/scratch/t96/` (local only). PyWavelets 1.8 was used only to
identify filters ("pywt-checked"), never as the source of a number.

### 19.1 Wavelets and extension modes (all five items)

- **21 wavelets, in this order** (the combo box and the integer `type` 0 to 20): Haar, DB2 to DB10, Bior1.1, Bior1.3,
  Bior1.5, Bior2.2, Bior2.4, Bior2.6, Bior2.8, Bior3.1, Bior3.3, Bior3.5, Bior3.7. No Symlets, Coiflets or other
  families. Default Haar. In the other four items the option strings are `haar, db2..db10, b11, b13, b15, b22, b24,
  b26, b28, b31, b33, b35, b37` (`type:=db2` and `type:=b13` work as well as the number).
- **The filters are the usual ones**, with PyWavelets' names `haar`, `db2`..`db10`, `bior1.1`..`bior3.7` (Origin's DBn has
  2n taps, so DB2 has 4; filter lengths L: Haar 2, DBn 2n, Bior1.1 2, 1.3 6, 1.5 10, 2.2 6, 2.4 10, 2.6 14, 2.8 18, 3.1 4,
  3.3 8, 3.5 12, 3.7 16). All 21 decompositions below equal `dec_lo`/`dec_hi` of the wavelet to 1e-15.
- **Extension mode** ("Extension Mode", in Reconstruction "Boundary"): `Periodic` (default) or `Zero-padded`.

### 19.2 Decompose (`dwt`)

`dwt ix:=<Y column> type:=<0..20> ext:=<0 periodic, 1 zero-padded> ca:=<new> cd:=<new>`. The X column of the book is
ignored. One level only. Two new columns are appended at the right end of the sheet: the approximation (long name `DWT_CA1`,
comment `Approx Coeff of DB2 dwt on B`) and the detail (`DWT_CD1`, `Detail Coeff of DB2 dwt on B`); a second call counts up
(`DWT_CA2`, ...). The comment names the wavelet in the combo's spelling and the input column's short name.

- With `x[m]` the signal and `lo`, `hi` the decomposition filters (pywt `dec_lo`, `dec_hi`, taps `0..L-1`):
  **`ca[k] = sum_j lo[j] x[2k+1-j]`, `cd[k] = sum_j hi[j] x[2k+1-j]`.**
- **Periodic**: indices modulo `n`, `k = 0..n/2-1`, so `n/2` coefficients each. An odd `n` is refused (the call returns
  false, nothing is written; `n = 21` refused, `n = 16` works).
- **Zero-padded**: `x = 0` outside `0..n-1`, `k = 0..floor((n+L-1)/2)-1`, so `floor((n+L-1)/2)` coefficients each (n = 16:
  Haar 8, DB2 9, DB3 10, DB4 11, ..., DB10 17; n = 21 works: Bior1.3 gives 13). This is `pywt.dwt(x, w, mode='zero')`
  exactly (all 21 wavelets, 1e-15).
- Periodic is **not** pywt's `periodization` (that is a circular shift of it); the formula above matches all 21 wavelets to
  1e-15 (`tools/origin-ref/t96/a3.py`).

### 19.3 Reconstruction (`idwt`)

`idwt ca:=<column> cd:=<column> type:=<n> ext:=<n> ox:=<new>`. Output: one new column (long name `IDWT_Result1`, comment
`DB2 idwt of C, D`). With `rl`, `rh` the reconstruction filters (pywt `rec_lo`, `rec_hi`), `N = len(ca)` and
`m = 2k + j - (L-2)`: **`y[m] += ca[k] rl[j] + cd[k] rh[j]`** for `k = 0..N-1`, `j = 0..L-1`.

- **Periodic**: `m` wraps modulo `2N`; output length `2N`. **Zero-padded**: terms with `m` outside `0..2N-L+1` are dropped; output
  length `2N - L + 2` (`N = 9`, DB2: 16 values, the length `dwt` took in). Checked against Origin for DB2 on unit coefficients
  (`ca` or `cd` one at a time, every position, both modes, 1e-16, `idwt1.out`); other wavelets are not checked on their own,
  only through decompose-then-reconstruct in the model.
- Unequal lengths (DB2): the output length follows `ca`; a shorter `cd` counts as zeros (periodic, 5 and 3 values give 10), a
  longer `cd` is cut to `ca` (zero-padded, 5 and 6 values give 8, the same as 5 and 5).
- Coefficients not made by `dwt` are accepted (random `ca` and `cd` of 5 values worked).

### 19.4 Denoise (`wtdenoise`)

`wtdenoise iy:=(Xcol,Ycol) type:=<name> ext:=periodic|pad0 thrt:=custom|sqtwolog level:=<1..7> thrd:=<percent>`. Defaults Haar,
periodic, custom, level 1, 50. Output: two new columns, `Denoised X1` (a copy of the input X) and `Denoised Y1` (comment
`1 level(s) 50% threshold DB2 denoise of B`), counting up on the next call. The result has the input's length.

- **Levels allowed** (a larger value returns false and writes nothing): periodic `floor(log2 n) - 1` (n = 16: 3, 17: 3, 32: 4,
  64: 5, 100: 5, 128: 6), zero-padded `floor(log2 n)` (16: 4, 17: 4, 32: 5, 64: 6, 100: 6, 128: 7), the same for Haar, DB2 and
  DB4. **Haar with zero-padded is refused at every level when `n` is a power of two** (16, 32, 64, 128) and works for n = 17
  and 100; not explained.
- **`custom` is hard thresholding of the detail coefficients of levels 1 to `level` with one common threshold
  `T = thrd/100 * max |d|`, the maximum taken over all those levels; coefficients with `|d| >= T` stay unchanged, the others
  become 0; the approximation is never touched.** Equal to 1e-15 to the model (decompose `level` times as in 19.2, threshold, rebuild as in 19.3) in the cases of the fixture
(128 noisy points, levels 1 to 4, `thrd` 0, 25, 50, 100, five wavelets, periodic); `thrd = 0` returns the input. At exactly 100%
only the maximum coefficient stays (`>=`): three of the four levels kept it, level 3 did not (rounding of `100/100 * max`); Sheaf
keeps `>=` and accepts that difference.
- **`sqtwolog` ignores `thrd`.** It is the same hard rule on levels 1 to `level` with one common threshold `T` that does **not**
  scale with the data (the same signal times 0.1, 1 and 10 gives the same `T`) and depends only on `n`. Measured by
  dropped-below and kept-above brackets on designed signals (db2, periodic, level 1; `sqtwolog_threshold_brackets` in the
  fixture): n = 128: 1.861 to 1.908; 256: 2.0526 to 2.0600; 512: 2.1847 to 2.1959; 1024: 2.3269 to 2.3302; 2048: 2.4377 to
  2.4593; the brackets for n = 24, 32, 48, 64, 96 are wider (1.6 to 2.2) and all contain 1.88. For n >= 256, `T = 0.6592 sqrt(2 ln(n/2))` fits all four brackets (the factor must lie in 0.6590 to 0.6594); for n <= 128
  `T` is about 1.88 and flat. **The rule behind `T` is not found** (it is not the universal threshold `sigma sqrt(2 ln n)`: no
  `sigma` from the data enters). Sheaf's choice until it is found: the brackets as a table for the lengths measured, the fit
  above beyond 2048, and a note in the dialog's description line.
- **Zero-padded denoise** (db4, level 2, `custom` 50, 128 points) equals the model built on the zero-padded `dwt` of 19.2 with each level's reconstruction cut back to the length it came from (67 samples at level 1: the 68 that `idwt` gives lose the last one), to 4e-16.
- **Boundary samples.** The first and last `L - 2` samples of the output differ from the model in some cases: six designed
  signals (db2, periodic, level 1, `thrd` 50: one coefficient `d[0] = 10` kept; `d[0] = 3` dropped next to a kept 10; with
  approximation `a[0] = 5`) differ at samples 0, 1, 14, 15 of 16 by up to 0.9 (`denoise_boundary` in the fixture), while the same
  signals with `thrd` 1 (everything kept) come back exactly. The 24 noisy-signal cases do not show it. Not resolved; the model is
  right for samples `L-2 .. n-L+1`, the fixture decides the rest.

### 19.5 Smooth (`wtsmooth`)

`wtsmooth iy:=(Xcol,Ycol) type:=<name> ext:=periodic|pad0 cutoff:=<percent>`; defaults Haar, periodic, 20. Output: `WT Smoothed X1`
(the input X) and `WT Smooth Y1`, comment `20% cutoff DB2 smooth of B`. There is no level input: **the signal is decomposed to the
deepest allowed level** (19.4; n = 128 periodic: 6 levels, 126 detail coefficients). The detail coefficients are laid out as
`[cD_J, ..., cD_2, cD_1]` (coarsest first) and **the last `K` of them are set to 0, `K = floor(cutoff/100 * N)`**, `N` the number
of detail coefficients: the finest level goes first and, inside a level, the last coefficients go first (n = 128, cutoff 20:
`K = 25`, the last 25 of level 1; cutoff 50: `K = 63`, all of level 1 but its first; cutoff 70: all of level 1 and the last 24 of
level 2; cutoff 90: levels 1 to 3 and the last of level 4). The approximation stays; the signal is rebuilt (19.3). Cutoff 100 leaves
the approximation only. It is a position rule, not a size rule: large coefficients at the end go and small ones at the start stay
(db2, periodic, n = 128, noisy signal, cutoff 0, 5, 10, 20, 30, 50, 70, 90, 100, equal to 1e-15, except as listed next).

- n = 128 with cutoff 0 and 1 gave `K = 2`, not 0 or 1 (Haar, DB2, DB4, Bior1.3); n = 64 and n = 100 with cutoff 0 return the input.
- Zero-padded DB2, n = 128, cutoff 0: `K = 3` (found from the fixture by trying `K`; the periodic corner is 2). Sheaf reproduces both for n = 128 and cutoff 0 and 1.
- Zero-padded: depth `floor(log2 n)`, `N` the sum of the coefficient counts (n = 128, DB2: depth 7, N = 140; cutoff 50 gave
  `K = 70`, cutoff 100 gave 140), but n = 100 zero-padded at 50 zeroed exactly the 51 finest coefficients, not
  `floor(0.5 * 112)`, and periodic n = 100 (odd length 25 at level 2) has no exact fit yet. Open.

### 19.6 Continuous Wavelet (`cwt`)

`cwt ix1:=<signal column> ix2:=<scale column> type:=<0 Morlet, 1 DGauss, 2 MexHat> param:=<wave number> upf:=<0|1> pf:=<new>
rcoef:=<range> mcoef:=<matrix> plot:=<0|1>`. Defaults: Morlet, wave number 5 (the combo offers 5, 6, 7, 8, 9, 10, 15, 20 and is
editable; 3 was refused, 5 and 8 work), `upf` 0, `plot` 1. Output `rcoef` defaults to a **new sheet `CWTResults1` in the input's
book: one row per signal sample, one column per scale in the order of the scale column, real numbers, no X column**; the first column
is named `Coefficient`, the next ones `A`, `B`, ... (long names empty), the comment of every column is `cwt of A, B` (the signal's
and the scale column's names). A script call made no graph and no matrix (`plot:=1` and `mcoef:=<new>` did nothing), and
`pf:=<new>` with `upf:=1` wrote no column; with `upf:=1` the coefficients are the same.

**What the numbers are** (formula found and verified against 43 Origin runs (fixture keys `cwt` and `cwt_more` of `t96-origin.json`, test
`crates/calc/tests/origin_cwt.rs`, agreement 1e-14). The runs: unit impulses at several rows and amplitudes, a generic signal of 65 and 64
samples, signals of 1, 2, 3, 17, 30, 31 samples, Morlet wave numbers 5 to 20, DGauss orders 1 to 8, MexHat, scales 1 to 12.3.

- Origin's coefficients are computed the way the classic discrete CWT on a sampled, integrated wavelet is: the mother wavelet `psi`
  is sampled at `N = 1024` points `x_i = -R + i step`, `step = 2R / (N - 1)`, with `R = 8` for Morlet and MexHat and `R = 5` for DGauss,
  and `ip[i] = step * sum_{k <= i} psi(x_k)` (running sum times the step).
- A scale `a` is **cut to its integer part** (scale 1.5 gives exactly the column of scale 1, with `sqrt(1)`; 2.9 gives 2, 12.3 gives 12).
  Any scale below 1 refuses the whole run (0.5, 0.9 and 0.99999 each did; the output sheet is not made).
- The filter has `floor(2R a) + 1` taps `f[i] = ip[floor(i / (a step))]`, `i = 0 .. 2R a`, applied reversed. The coefficients are
  `-sqrt(a)` times the first difference of `conv(signal, f)`, keeping the middle `n` values (the first `floor((taps - 2) / 2)` of the
  difference skipped). The first difference of an integrated wavelet is why every impulse response is centred half a sample after the
  impulse (the earlier measurement). Shift invariance, linearity, zero padding at the ends and signals shorter than the filter all follow.
- Morlet: `psi = exp(-x^2/2) cos(w x)` with `w` the wave number cut to an integer. Origin allows 5 to 20 (1 to 4 and 21, 25, 50, 100 refused;
  non-integer values such as 5.5, 6.5, 19.9 are accepted and give the column of 5, 6, 19).
- MexHat: `psi = 2 / (sqrt(3) pi^(1/4)) (1 - x^2) exp(-x^2/2)`; the parameter is not used.
- DGauss of order `m` (the `param`, cut to an integer, 1 to 8; 0 and 9 to 12 refused, 8.5 gives 8, 1.5 gives 1):
  `psi = sigma_m H_m(x) exp(-x^2) / sqrt(sqrt(pi/2) (2m-1)!!)`, `H_m` the physicists' Hermite polynomial, `sigma_m = -1` for `m` = 1, 2 (mod 4)
  and `+1` for `m` = 3, 0 (mod 4), i.e. the `m`-th derivative of `exp(-x^2)` scaled to unit L2 norm with a sign that follows that pattern
  (all eight orders checked).
- `upf:=1` gives exactly the same coefficients (CM against CMP, difference 0); it only adds the pseudo-frequency output, which a script
  call does not write, so it is not measured. The earlier note that the coefficients differ was wrong.
- Columns come in the order of the scale column; the sheet has `n` rows and one column per scale, no X column.

Sheaf: `sheaf_calc::cwt::cwt`, `Tool::ContinuousWavelet`, `sheaf analyze cwt --with <scale column> --wavelet morlet|dgauss|mexhat --param N`.
Differences kept: the short names are A, B, ... (Origin's first column is named `Coefficient`), the coefficient matrix, the image
plot and the pseudo-frequency are not made, and the scale column must be a Y column of the project.

### 19.7 The windows

Titles `Decompose: dwt`, `Reconstruction: idwt`, `Denoise: wtdenoise`, `Smooth: wtsmooth`, `Continuous Wavelet: cwt`: plain
X-Function windows (a Dialog Theme row on Denoise and Smooth, a one-line Description, the fields, OK and Cancel, a status line
under the fields: `Please select a data range for input.` on Decompose and Reconstruction, `Please select an XYRange of input
data` on Denoise and Smooth, `You must have numeric data to run this analysis` on Continuous Wavelet). Fields, in order:

| Window | Fields (default) |
|---|---|
| Decompose | Recalculate (Manual); Input (range); Wavelet Type && Order (Haar, 21 entries); Extension Mode (Periodic, Zero-padded); Approx Coefficient (`<new>`); Detail Coefficient (`<new>`) |
| Reconstruction | Recalculate; Approximation Coefficients; Detail Coefficients; Wavelet Type (Haar); Boundary (Periodic, Zero-padded); Output (`<new>`) |
| Denoise | Recalculate; Input; Wavelet Type (Haar); Extension Mode (Periodic); Output (`(<new>,<new>)`); Threshold Type (custom, sqtwolog); Thresholding Level (1 to 7, 1); Threshold of every Level (%) (50, slider 0 to 100); a preview pane on the right with Auto Preview and Preview buttons |
| Smooth | Recalculate; Input; Wavelet Type; Extension Mode; Output (`(<new>,<new>)`); Cutoff (%) (20, slider 0 to 100); the same preview pane |
| Continuous Wavelet | Recalculate; Discrete Signal; Scale; Wavelet Type (Morlet, DGauss, MexHat); Wave Number (5, editable list); Use Pseudo-Frequency (unchecked); Coefficient (checked, `[<input>]<new>!1`); Coefficient Matrix (unchecked) |

The Decompose label reads `Wavelet Type && Order` (two ampersands, as in the file). The menu itself was not opened; the item order
is the inventory's (greyed-items.md).

### 19.8 Not measured

The comment spelling for Haar in Decompose, what `Recalculate` does for these five, the preview pane's drawing, the Results Log
lines, missing values in the input (a blank cell in Denoise: the call succeeded, the output was not read), the menu itself, the
image plot and the coefficient matrix of `cwt` (no script route; the window's "Coefficient Matrix" check and the plot made after
OK), the pseudo-frequency output, `mdwt`, `dwt2`, `idwt2`, missing values in the `cwt` signal, several signal columns in one `cwt` call, and the open
rules of 19.4 (boundary samples, `sqtwolog` threshold) and 19.5 (non-power-of-two lengths, `K` at cutoff 0 and 1).

## 20. Deconvolution and STFT (`deconv`, `stft`; T57, 2026-11-19; measured against Origin 2016 in T292)

Origin 2016 has Analysis > Signal Processing > Deconvolution... and STFT.... Neither could be run
(Origin's COM class was unregistered), so this follows Origin's documented definitions. The numbers
below are Sheaf's choices, checked against numpy, not against Origin.

### 20.1 Deconvolution

- **Inputs:** the signal columns and one Response column (any numeric Y column of the project, the
  signal itself included), as Convolution (section 13). Trailing missing values are dropped; a missing
  value inside is refused.
- **Method** (`sheaf_calc::deconvolve`; measured in T292, 2026-11-21): the signal's spectrum divided by
  the response's.
  - **Linear** (the default, Origin's `circular:=linear`): signal and response padded with zeros to
    `2N - 1` points; the first `N - m + 1` values are kept. Deconvolving a linear convolution
    `[1, 3, 5, 3, 0, 0]` by `[1, 1]` gives `[1, 2, 3, 0, 0]`; a signal that is no exact convolution gives
    Origin's numbers too (`[2, 5, ..., 17]` by `[1, 1]`: `6.5, -1.5, 9.5, 1.5, 12.5`).
  - **Circular** (`--circular`, the window's Type): the FFT over the signal's `N` points, `N` values; it
    undoes a circular convolution exactly (Origin's numbers to 1e-12).
- **Options:**
  - Normalize (on by default in the window, as Origin's `norm:=1`; `--normalize` in the CLI) divides the
    response by its Euclidean norm first, as Convolution's `norm`: the result is scaled by that norm.
  - Sampling interval: Auto is the X step of the input, as Convolution.
- **Refused:** a response longer than the signal, and a response whose spectrum has a zero (a value
  at most 1e-12 of its largest), because nothing can be divided by it. A constant response as long
  as the signal has zeros at every frequency but 0, so it is refused. (Origin writes missing values
  there instead.)
- **Output:** a new sheet "Deconvolution" with X (0, interval, 2 interval, ...) and Y for each input. As
  Origin 2016, the Y column has no Long Name and the Comments "deconvolution of B, C".
- **Command line:** `sheaf analyze deconvolution <file> --y <col> --response <col> [--normalize]
  [--circular] [--interval d]`. MCP reaches it through the analyze bridge.

### 20.2 STFT

- **Method:** segments of Window Length points start every Window Length - Overlap points while
  they fit in the signal; a remainder at the end is left out (`sheaf_calc::stft`).
  - Each segment is multiplied by the window. The windows are FFT's (section 15), symmetric over the
    segment's points.
  - The segment is padded with zeros to FFT Length and transformed.
- **Output values:**
  - Amplitude (the default) is one-sided, `2|X| / sum(w)`, and `|X| / sum(w)` at 0 and at the
    Nyquist frequency. A sine of amplitude A filling a segment shows a peak of about A.
  - dB is `20 log10(amplitude)`, missing where the amplitude is 0.
- **Defaults (Origin 2016's, T292):** Hanning window, Window Length 256, FFT Length 256, Overlap 128,
  dB, and the sampling interval from the input's X (Auto).
- **Measured in Origin 2016 (T292).** `stft` on 1024 samples of a unit sine of period 16 gave a 129 x 7
  matrix (frequencies 0..0.5 by segment centres 127.5..895.5) whose amplitudes are exactly Sheaf's
  (peak 0.99999653, the DC bin 3.0042e-6 and the Nyquist bin 1.1841e-7 not doubled: the symmetric
  Hanning with `sum(w)` = 127.5), and dB = 20 log10 of them. `correct:=none` gives `2|X| / N` instead
  (not offered). Origin pads a short window with zeros on both sides (the amplitudes are the same).
- **Refused:** a window under 2 points, an overlap not less than the window, an FFT length under the
  window, a signal shorter than the window, and more than 2000 segments.
- **Output:** Origin writes a matrix (rows the frequencies, columns the segments) and an image plot;
  Sheaf still writes a new sheet "STFT" (a frequency column and a column per segment), with the same
  numbers.
  - The X column is "Frequency", 0 to the Nyquist frequency in `fft_len / 2 + 1` rows.
  - Then there is one Y column per segment. Its long name is "<input> at <X name> = t", where t is
    the segment's centre: its first X value plus `(start + (window_len - 1) / 2)` samples, times the
    interval. Its comment is "STFT amplitude of <short>" (or "STFT dB of ...").
- **Command line:** `sheaf analyze stft <file> --y <col> [--window hanning] [--window-len 64]
  [--fft-len 64] [--overlap 32] [--output amplitude|db] [--interval d]`.

### 20.3 Not measured

Not checked: Origin's dialog controls and defaults for both tools, its deconvolution length and
wrap-around rules, its STFT window list, scaling, time axis and matrix layout, and missing values.
Measure with `tools/origin-ref/` once Origin's COM class is registered again.

## 21. CV Charge (T296, Sheaf's own; the owner's cyclic voltammetry work)

Origin 2016 has no cyclic voltammetry tool: the charge of a CV peak is an `integ1` over chosen rows,
divided by the scan rate by hand. Sheaf's Analysis > Mathematics > CV Charge... (CLI `sheaf analyze
cv-charge FILE --y COL --scan-rate V [--baseline]`) does both in one step (`sheaf_calc::cv_charge`,
`Tool::CvCharge`):

- **Input:** X the potential, Y the current, over the chosen rows (choose the rows of a peak for a peak
  charge). Missing points are skipped.
- **Sweeps:** the curve is cut at every turn of the potential: a run where the potential rises is an
  anodic sweep, a falling one cathodic. The turning point belongs to both sweeps; steps where the
  potential does not change stay in the sweep.
- **Charge:** `Q = integral of I dt = sum of (I_k + I_k+1) / 2 |E_k+1 - E_k| / v` per sweep, with `v` the
  scan rate (Scan rate, potential units per second; 0.1 by default). An anodic (oxidation) current
  gives a positive charge in either sweep, a cathodic one a negative charge. The unit is the current's
  times seconds (A gives C, mA mC, uA uC).
- **Baseline:** "Subtract the line through each sweep's ends" removes the straight line through the
  sweep's first and last point (current against potential) before summing: with the rows of a peak
  chosen, the peak's charge above a sloping background.
- **Output:** a results sheet `CVCharge<n>`: one row per sweep (label "<input>, sweep k (anodic)" or
  "(cathodic)") with First row, Last row, E start, E end and Charge, then the total of all anodic and of
  all cathodic sweeps; a note gives the scan rate. One undo step ("CV charge").
- **Not built:** picking the peak's feet on the graph (the parity row's "foot picking").

## Files

`tools/iir-fir-ref.py` makes the scipy numbers of section 17 (not Origin).

Reference scripts (all in `tools/origin-ref/`): `curvemath-suite1..3` (run with
`run-labtalk-lines.ps1`, they print the result sheet), `integ-suite1..6`, `diff-suite1`, `smooth-suite1`,
`smooth-fftprobe`, `smooth-fft-grid`, `smooth-fft-edge`, `smooth-fft-big`, `interp-bspline`, `normalize-suite1`, `interp-suite1`, `pkfind-suite1`, `pkfind-suite2`,
`colstats-suite1..3`. Run with `run-labtalk-data.ps1`. Raw dumps (git-ignored):
`re-workspace/scratch/sheaf-calc/*.json`.