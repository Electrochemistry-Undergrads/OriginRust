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
B-spline interpolation: the smoothing spline for a smoothing factor above 0 is done and matches
Origin; the default factor 0 is implemented as the interpolating spline and does NOT match Origin
inside the data (Origin's automatic factor was not found, see section 5).

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
* (observed) `sf = 0` (the default) is not an interpolating spline: Origin then uses an automatic
  factor that depends on the data in a way we did not find. Measured values of this factor for
  unit impulses on x = 0..10: 1/12, 0.1245, 19/120, 0.1799, 0.1843, 1/6, 1/12, 0.0422, 1/120, 9/680,
  3/170 for the impulse at x = 0, 1, ..., 10; it is proportional to the square of the data scale and
  for other data (sin x, random numbers) it is 4 % to 14 % of the sum of squares of y. It is not a
  quadratic form in y, not the residual of a polynomial fit, and it does not come from limiting the
  number of knots. Consequently Origin's default B-spline moves up to 0.27 away from the data points
  of a unit impulse and about 0.2 from sin x on 11 points. Data that already lie on a cubic
  polynomial (for example y = x^2) are reproduced exactly.
* (observed) Outside the data range with the default option Origin continues with the straight
  line through the two nearest data points (not the end slope of the curve), whatever `sf` is.
  `miss` and `repeat` work as for the other methods.
* (observed) Repeated X values: Origin replaces them by one point with the mean Y (checked for
  `sf` = 0.2 on x = 0 1 2 2 3 4 4; the output equals the fit of the merged points to 1e-9). We do the
  same for the B-spline method. For the default factor the curve at a repeated X then differs from
  the mean, which is the same automatic smoothing that is not understood. Fewer than four different
  X values give no result.
* Our implementation (`InterpMethod::BSpline`): `smoothing` above 0 is the Dierckx smoothing spline
  inside the data; 0 is the interpolating (not-a-knot) cubic spline, which agrees with Origin's
  default only for data that lie on a cubic polynomial; both continue outside the data with Origin's
  straight line. At least four different X values. A negative or missing factor is refused. The
  dialog and the command line say that factor 0 is not Origin's automatic smoothing.

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
5. Multi-peak fit (own model, Origin's functions): a sum of Gauss or Lorentz peaks plus a constant or
   sloped baseline, parameters `y0`, `m`, then `xc`, `w`, `A` per peak, in Origin's parameterisation
   (Gauss `A/(w sqrt(pi/2)) exp(-2((x-xc)/w)^2)`, FWHM `w sqrt(ln 4)`; Lorentz
   `2A/pi * w/(4(x-xc)^2+w^2)`, FWHM `w`). Starting values come from the found peaks (centre, FWHM,
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
[--anchors X1:Y1,X2:Y2] [--limits baseline|half-height] [--integrate] [--fit gauss|lorentz]
[--fit-baseline constant|linear]`, together with the existing `--points`, `--min-height`, `--count`,
`--direction`. Without any chain option the plain peak finder of section 7 runs. Window: Analysis >
Analyze Peaks... (command line `--show analysis:peak-chain`).

## 12. Data manipulation tools and Median Filter (Sheaf's own rules, not measured)

Analysis > Data Manipulation has Subtract Straight Line, Reduce to Evenly Spaced X and Curve
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
- Reduce to Evenly Spaced X: new X values from the lowest to the highest X of the data (or a chosen
  range), either by number of points (first and last included, at least 2) or by step (the last value
  is kept when the range is a whole number of steps within rounding). Y is interpolated by straight
  lines (new X outside the data are missing) or averaged: each new X owns the bin one step wide
  centred on it, the lower edge belongs to the bin, an empty bin is missing. The result is a new sheet
  with an X column and a Y column per input. Points with a missing X or Y are ignored.
- Curve Translate X: adds a constant to X, or moves the point at X = `at` to X = `to` (shift
  `to - at`); the result is a new sheet with the shifted X and the unchanged Y. Curve Translate Y
  (Sheaf's own item, next to X): adds a constant to Y, or shifts so that the curve's value at X = `at`
  (between data points by straight lines) becomes `to`; the result is a new Y column behind each
  input named "<name> shifted". A point outside the X range is refused.
- Median Filter: the window has an odd number of points (at least 1; the default is 5), centred on
  each point. Edge choices: shrink (default), repeat the end value, reflect including the end point.
  Even windows are refused. The result is a new Y column behind each input named "Median filtered
  <name>".

Not measured (to be measured in Origin; nothing here may be quoted as Origin's behaviour):

1. Origin's X-Function names and options for the four items (the window title shows the tool alone).
2. Subtract Straight Line: how Origin picks the two points (on a graph, by data cursor, or by
   typed values), whether a point's Y is the data value or the cursor's, and whether the result
   replaces the data or goes to a new column.
3. Reduce to Evenly Spaced X: which interpolation and averaging choices Origin offers, its bin
   definition and edge rule for averaging, how it chooses the number of points or the step by
   default, what a new X outside the data gets (missing or extrapolated), whether the last grid
   point is kept when the range is not a whole number of steps, and the output layout.
4. Curve Translate: whether Origin moves data in place or writes a copy, how its "move a point to
   a value" mode reads a Y at an X between data points, and whether it works on the graph
   interactively. Curve Translate Y is not an Origin item as far as we know; its numbers are Sheaf's.
5. Median Filter: whether Origin's window must be odd, what an even window does, its boundary
   rule at the ends (we assume the percentile filter's), the handling of missing values (we assume
   the smoothing rule), and the default window size.
6. The Recalculate behaviour of these items in Origin (which of None, Manual, Auto they offer),
   the long names, units and comments of the output columns, and the row-range handling.

Command line: `sheaf analyze subtract-line FILE --y B --x1 X --x2 X [--y1 V] [--y2 V]`,
`sheaf analyze reduce-x FILE --y B [--step S | --points N] [--method interpolate|average]
[--x-min X] [--x-max X]`, `sheaf analyze translate FILE --y B [--axis x|y] (--by D | --at X --to V)`
and `sheaf analyze median-filter FILE --y B [--points N] [--edge shrink|repeat|reflect]`. Windows:
`--show analysis:subtract-line`, `reduce-x`, `translate-x`, `translate-y`, `median-filter`.

## Files

Reference scripts (all in `tools/origin-ref/`): `curvemath-suite1..3` (run with
`run-labtalk-lines.ps1`, they print the result sheet), `integ-suite1..6`, `diff-suite1`, `smooth-suite1`,
`smooth-fftprobe`, `smooth-fft-grid`, `smooth-fft-edge`, `smooth-fft-big`, `interp-bspline`, `normalize-suite1`, `interp-suite1`, `pkfind-suite1`, `pkfind-suite2`,
`colstats-suite1..3`. Run with `run-labtalk-data.ps1`. Raw dumps (git-ignored):
`re-workspace/scratch/sheaf-calc/*.json`.