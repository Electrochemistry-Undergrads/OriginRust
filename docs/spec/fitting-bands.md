# Confidence and prediction bands, iteration controls

Written in our own words from the Origin 2016 documentation, the fit report and the Origin C
header that ships with Origin, and checked with black-box runs (see "What was verified").

## What the bands are

For a fitted curve `f(x; p)` with parameter covariance `C` (already multiplied by the reduced
chi-square, `s2 = RSS / dof`) and gradient `g = df/dp` at `x`:

- the **confidence band** is `f(x) +- t * sqrt(g' C g)`. It holds the true curve with the chosen
  probability.
- the **prediction band** is `f(x) +- t * sqrt(s2 + g' C g)`. It holds a new observation with the
  chosen probability and is always wider.

`t` is the two-sided Student value for the chosen confidence level and the degrees of freedom of
the fit (`dof = n - number of free parameters`). The default level is 95 %, as in Origin's
"Confidence Level for Curves". It is a separate setting from the level of the parameter limits.

For a straight line this is Origin's closed form
`t * RMSE * sqrt(1/W + (x - xbar)^2 / Sxx)` for the confidence band and the same with `1 +` under
the root for the prediction band (`W` the sum of weights, `RMSE = sqrt(s2)`). With the intercept
held fixed the curve is `value + slope x`, so the confidence half width is `t |x| SE(slope)` and the
prediction half width `t sqrt(x^2 SE(slope)^2 + s2)`.

The bands always use the covariance scaled by the reduced chi-square, whatever the "scale errors
with reduced chi-square" option of the parameter report is: the option changes the parameter errors
but not the bands (the library rescales an unscaled covariance for this).

## How they are computed

- Straight line and polynomial: from the covariance of the solved (centred and scaled) basis, so
  that x far from zero does not cancel digits (`FittedCurve::bands`).
- Nonlinear fits (built-in functions, typed formulas, .FDF): the gradient is taken by central
  differences of the model with a step of `6e-6 * max(|p|, 1e-3)`; fixed parameters have a zero
  gradient (`bands::nl_bands`). A nonlinear fit of a line gives the same bands as the linear fit.
- A value is NaN where there are no degrees of freedom or the variance is negative from rounding.

## Where they go

- `FitRequest::bands` (`None`, `Confidence`, `Prediction`, `Both`), `band_level` (0.95) and
  `plot_bands`. The columns are added to the `FitCurveN` sheet after X and the fitted Y, in the
  order lower confidence limit, upper confidence limit, lower prediction limit, upper prediction
  limit, named `LCL of <column> (95%)`, `UCL ...`, `LPL ...`, `UPL ...`.
- With `plot_bands` (default on) each band limit is also added as a line plot to the graphs that
  receive the fitted curve, in the curve's colour, dashed for confidence and dotted for prediction
  limits, labelled for the legend.
- Command line: `sheaf fit --bands none|confidence|prediction|both --level 95`.
- Fit window: a "Bands" choice and a "Band confidence level" field.

## Iteration controls

`FitRequest::max_iterations` (default 400) and `tolerance` (default 1e-9) are passed to the
Levenberg-Marquardt engine. Linear and polynomial fits are solved directly and ignore them. Zero
iterations or a tolerance that is not a positive number are refused and change nothing. Command
line: `--max-iterations N --tolerance T`. Fit window: "Maximum iterations" and "Tolerance 1e-"
followed by the number of digits.

## Starting values

Functions that gained their own guess: Poly4, Poly5, Constant, LineMod, Power1, PearsonVII,
PsdVoigt2, Asym2Sig, AsymmetricGaussian, Logistpk, LogNormal, ExpDecay1/2/3, ExpGro2, ExpGrow2,
Asymptotic2, BoxLucas1, Exp2P, SLogistic1, SLogistic4, SGompertz, Hill1, MichaelisMenten,
OneSiteBind, Hyperbl and RectHyperbola. Recipes: polynomials from an ordinary least-squares fit;
peaks from the peak guess (centre, width at half height, area); exponentials from a line through
`ln|y - y0|`; logistic steepness from the width between the 25 % and 75 % levels (`2 ln 3 / k`);
Gompertz from a line through `ln(-ln(y / a))`; hyperbolas from the line of `1/y` against `1/x`
(kept only when it gives positive values). Each is in the fit table of
`crates/fit/tests/library_fits.rs`, run at y scales 1, 1e-3 and 1e-6.

## What was verified

- `crates/fit/tests/origin_bands.rs` holds the straight-line numbers of an Origin 2016 run on data
  we generated (`tools/origin-ref/bands-linear.lines.txt`): Origin supplied the regression, RSS,
  degrees of freedom, the Student quantiles (95 % and 90 %) and the mean and spread of x, and the
  test compares the bands with the closed form evaluated from those numbers at five x values
  (tolerance 1e-6, because Origin's `tinv` agrees with the exact quantile to about 3e-7).
- **Not verified against Origin:** the band columns themselves. They cannot be read from a script
  without changing the owner's dialog settings, so the closed form from the Origin C source stands
  in. Nonlinear bands are checked against the analytic gradient of an exponential, against the
  linear bands for a line, and for ordering, not against an Origin run.
