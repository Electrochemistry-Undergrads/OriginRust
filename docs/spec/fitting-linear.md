# Linear and polynomial fitting

What `sheaf_fit::linear` (`fit_linear`, `fit_polynomial`) computes, and how each rule was
established against Origin 2016. Everything here was found by running Origin on data we generated
ourselves; nothing is copied from Origin's documentation or code.

## How the numbers were obtained

Origin's "Linear Fit" and "Polynomial Fit" dialogs could not be driven from a script (the
`FitLinear` and `FitPolynomial` X-Functions only open the dialog). Three scriptable sources were
used instead, in a hidden Origin through COM (`tools/origin-ref/run-labtalk-chunks.ps1`, scripts
`fitlin-suite.labtalk.txt` and `fitlin-edge-cases.labtalk.txt`):

| Source | What it gives |
| --- | --- |
| `fitlr` (straight line; options `fixint`/`intercept`, `fixslope`/`slope`) | intercept `a`, slope `b`, their errors `aerr`, `berr`, Pearson's `r`, point count |
| `fitpoly` (polynomial of any order; `fixint`/`intercept`) | coefficients and errors as vectors, `RSqCOD`, `AdjRSq`, point count |
| `nlbegin`, `nlfit`, `nlend output:=1` with the built-in `Line` and `Poly4` functions, and the tree variable (`ChiSqr`, `SSR`, `COD`, `r`, `Dof`) | the full report sheet: value, standard error, t, Prob>|t|, dependency, points, degrees of freedom, reduced chi-square, residual sum of squares, R-square, adjusted R-square, ANOVA; supports instrumental and statistical weights |

The straight-line and polynomial numbers from the three sources agree with each other and with a
plain weighted least-squares solution to 1e-9 or better wherever the problem is well conditioned.
The values are stored in `crates/fit/tests/fixtures/linear_origin.rs` (generated from the raw
dumps; raw dumps stay in the git-ignored scratch folder) and compared in
`crates/fit/tests/origin_linear.rs`. Tolerances: 1e-9 relative for `fitlr` and `fitpoly`, 1e-6
relative for the `nlfit` report (its iteration stops at a tolerance of 1e-9).

The test inputs: a noisy line (`2 + 0.5 x + 0.3 sin 3x`, x = 1..10), a line through the origin,
fixed intercept and fixed slope on the noisy line, instrumental weights (`err = 0.1 + 0.05 x`),
statistical weights, the noisy line with a missing x and a missing y, x from 100000 to 100010, a
noisy cubic (x = 0..11) fitted with orders 2 to 5, an exact line, a ninth-order fit of
`exp(x) + 0.01 cos 11x` on 21 points, and a second-order fit with the intercept fixed at 1.

## Definitions (all confirmed against the sources above unless marked)

Let `n` be the number of usable points, `p` the number of free parameters, `w_i` the weights
(1 when unweighted), `rss = sum w_i (y_i - f(x_i))^2`.

- **Missing values.** A point with NaN in x, y or its weight is dropped. `n_points` counts the
  rest; `fitted` and `residuals` have the length of the input, NaN at dropped points. (Origin:
  point count and degrees of freedom checked with missing x and y.)
- **Degrees of freedom** `n - p`; a held-fixed parameter is not counted in `p`.
- **Weights.** "Instrumental" uses `w = 1 / err^2` from a y-error column; "Statistical" uses
  `w = 1 / |y|` (checked with a negative y: Origin's result equals the `1/|y|` fit; for positive y
  this is the usual `1/y`); "Direct" uses the given weights. A zero or infinite weight
  (error of zero, statistical weight of a zero y) makes Origin's fit fail (status -204 and -205
  respectively) and `fit_*` returns an error too. A missing (NaN) weight drops the point.
  "Direct" weighting is not offered by the `nlbegin` option used here, so it was not compared
  with Origin; it is the same code path as the others.
- **Standard errors** are `sqrt(diag(C))` with `C = (X'WX)^-1 * rss / (n - p)`: the errors are
  scaled by the reduced chi-square, also when instrumental weights are given (verified on the
  weighted fit: the unscaled values would be 1.8 times larger). `scale_errors_by_reduced_chi_sqr =
  false` drops the factor; Origin's scripting interfaces have no switch for this, so that option
  is not compared with Origin (the dialog's "Use Reduced Chi-Sqr" check box is the likely
  counterpart).
- **t-value** `value / error`; **Prob>|t|** is the two-sided probability with `n - p` degrees of
  freedom (verified against the `nlfit` report to 1e-6).
- **Confidence limits** `value +- t_crit * error`, with `t_crit` the two-sided critical value at
  the chosen level (default 0.95). Not compared with Origin (the `nlfit` report sheet does not
  list limits); the formula is the standard one, and the 95 % critical value for 8 degrees of
  freedom is tested against the table value.
- **R-square (COD)** `1 - rss / tss`, **adjusted** `1 - (rss / (n - p)) / (tss / total_df)`.
  - All parameters free: `tss` is the sum about the weighted mean of y,
    `sum w (y - ybar_w)^2`, `ybar_w = sum w y / sum w`; `total_df = n - 1`. Verified for
    unweighted, weighted and polynomial fits (adjusted value equals the usual
    `1 - (1 - R2)(n - 1)/(n - p - 1)`).
  - **Intercept held fixed (at any value, not only zero):** `tss = sum w y^2`, the total about
    zero (not about the fixed intercept), and `total_df = n`. Verified with the intercept
    fixed at 0 and at 1.5 (straight line) and at 1 (second-order polynomial). This makes R-square
    for a fit through the origin much larger than the corrected definition would give
    (0.99924 against 0.99654 on the test data).
  - **Slope held fixed:** `tss` about the weighted mean, as for a free fit; verified through
    Pearson's r (below). The adjusted R-square for this case was not available from Origin and is
    computed with the same formula (`total_df = n - 1`, error degrees of freedom `n - 1`).
- **Pearson's r** (straight line only): the square root of R-square with the sign of the slope.
  For a free line this is the ordinary correlation coefficient. With the intercept held fixed,
  Origin's `fitlr` returns `sqrt` of the uncorrected R-square above (for the intercept at 0 this
  is `sum xy / sqrt(sum x^2 sum y^2)`); with the slope held fixed it returns the square root of
  the corrected R-square. Weighted fits: the `nlfit` `r` equals `sqrt(COD)` of the weighted
  R-square. If R-square is negative the square root is NaN here (no Origin value seen).
- **ANOVA.** Error row: `df = n - p`, `SS = rss`, `MS = rss / (n - p)`. Total row: `SS = tss`,
  `df = total_df` as above. Model row: `SS = tss - rss`, `df = total_df - (n - p)`, which is the
  number of free parameters other than a free intercept; `MS = SS / df`, `F = MS(model) /
  MS(error)`, `Prob>F` from the F distribution. For a free straight line `F` equals the square
  of the slope's t-value. The `nlfit` report shows the uncorrected model and total rows instead
  (model df equals the number of parameters); its error row and corrected total row agree with
  ours to 1e-6, which is what the tests compare, and the model row and `F` are derived from them
  in the test. **Not verified:** the Linear Fit dialog's own ANOVA table was not seen. With a
  fixed slope the model has no degrees of freedom and `F`, `Prob>F` are NaN. The unused
  `uncorrected_total_ss` field is NaN.
- **Dependency** is not filled in for linear fits (NaN). Origin's `nlfit` report shows it for
  the same fit (0.7857 for the noisy line), which can be computed from the covariance matrix if a
  later screen wants it.
- **Exact data.** On a perfect line Origin returns standard errors of exactly 0, `r` = 1 and a
  fit status of "Succeeded(101)". We return errors of the order of 1e-16 (the residual sum of
  squares is rounding noise); the test accepts 1e-12 absolute.
- **No degrees of freedom** (as many parameters as points). Origin returns zero errors and blank
  probabilities (`nlfit`, status 102) or leaves the errors unset (`fitlr`, `fitpoly`). We return
  NaN for errors, t, probability and confidence limits, NaN for adjusted R-square and a warning.

## Numerics

- The weighted design matrix is decomposed by QR; there are no normal equations. If the R factor
  has a diagonal entry below 1e-13 of the largest, the fit is reported as singular (for example
  all x equal).
- With every coefficient free, the basis is `u^k`, `u = (x - mean_w) / spread`, which stays exact
  to rounding for x near 1e5. The coefficients are converted to powers of x for the report. The
  converted coefficients and their errors suffer cancellation when `mean / spread` is large;
  fitted values and the residual sum of squares are evaluated in the `u` basis and do not.
- The fitted curve that the app and `sheaf fit` draw (the `FitCurve` sheet and the graph line) is
  evaluated in that solved basis too, through `fit_linear_with_curve` and
  `fit_polynomial_with_curve`, which return a `FittedCurve` next to the report. Evaluating the
  converted coefficients with Horner's rule instead was off by 16 times the y scale for x = 1e5..1e5+19
  and order 5. The reported coefficients are unchanged.
- With a fixed coefficient the free powers of `x / max|x|` are used.
- Badly scaled x (100000 to 100010): for the straight line our numbers equal Origin's to 1e-7 or
  better. For a second-order fit on the same data **Origin itself is inaccurate**: an exact
  rational-arithmetic solution gives R-square 0.9001044458823787, ours 0.90010444588238 (1e-14),
  Origin's `fitpoly` 0.9001044512709 (6e-9 off), and Origin's coefficients differ from the exact
  ones in the seventh digit. The test compares ours with the exact value and with Origin to 1e-7.
- Ninth-order polynomial on 21 points in [-1, 1]: the test accepts coefficient differences of 1e-6 of the
  error estimate, errors to 1e-6 relative and R-square to 1e-9 (the observed differences were
  not recorded more finely; they are below these limits).
- Orders 1 to 9 are supported; any higher order works if there are enough points and the
  problem is not singular, but numbers for orders above 9 were not compared with Origin.

## API

```rust
let rep = fit_linear(&x, &y, &LinearOptions::default())?;           // Intercept, Slope
let rep = fit_polynomial(&x, &y, 3, &PolyOptions::default())?;      // Intercept, B1, B2, B3
let (rep, curve) = fit_polynomial_with_curve(&x, &y, 3, &PolyOptions::default())?; // curve.eval(x)
```

- `LinearOptions { fix_intercept, fix_slope, weighting, confidence_level (0.95),
  scale_errors_by_reduced_chi_sqr (true) }`. `PolyOptions` has the same without `fix_slope`.
- `Weighting::Instrumental(errors)` carries the y-error column (one value per input row);
  there is no separate argument. `Weighting::Direct(weights)` likewise.
- Both fixed parameters of a line is an error (nothing to fit).
- The report's `status` is `Exact`, `iterations` is `None`, `pearson_r` is set for orders 1 only.

## Our own rules for odd inputs (Origin not run on these)

- A confidence level that is not strictly between 0 and 1 is refused with an error.
- When y does not vary (the total sum of squares is zero or only rounding noise), R-square,
  adjusted R-square and Pearson's r are NaN and a warning is added. Origin's behaviour was not
  checked.
- The confidence limits are value plus or minus the Student t critical value times the standard
  error, and the ANOVA F equals the squared t of a straight line's slope: these are checked against
  our own formulas only, not against an Origin dialog report.

## What was not checked against Origin

- The Linear Fit and Polynomial Fit dialogs' own report sheets (only `fitlr`, `fitpoly` and
  `nlfit` could be driven). If the dialog's ANOVA or R-square for fixed parameters differs from
  `fitlr`/`fitpoly`, only the corresponding fields would change.
- Confidence limits, the unscaled-error option, "Direct" weighting, adjusted R-square with a fixed
  slope, and the dependency column.
- Fixed parameters inside `nlfit` (the tree key that fixes a parameter was not found), so the
  fixed-parameter definitions rest on `fitlr` and `fitpoly` only.
