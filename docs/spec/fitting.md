# Nonlinear curve fitting

Summary. `sheaf-fit` fits a model `y = f(x; p)` to points by minimising the (weighted) sum of squared residuals with a Levenberg-Marquardt iteration, with optional bounds and fixed parameters, and returns the same `FitReport` as linear fitting. The built-in functions (206 of Origin's 237 that have one independent variable and a usable formula), user-defined formulas and imported `.FDF` files all become one `FitFunction`. Numbers are compared with Origin 2016 (section 6): same iteration counts, differences below 5e-10. Linear and polynomial fits are in `fitting-linear.md`.

Sources: Origin's public documentation on nonlinear curve fit, the FDF files (parameter lists, bounds, formulas), and runs of Origin on data we generated (section 6). Everything below is written in our own words.

## 1. The model

A `FitFunction` has: a name, an equation text, parameter names, one independent variable (default name `x`), a model, and an optional routine guessing starting values. The model is either a formula in the expression language (`expression-language.md`; parameters and `x` are variables) or Rust code `(params, x) -> y`. Each parameter has an optional lower bound, an optional upper bound, a starting value (NaN: guess or 1) and a "fixed" flag.

A user-defined function is created with `FitFunction::from_formula(name, "y0 + A*exp(-x/t)", &["y0", "A", "t"])`. A leading `y =` is accepted. A name in the formula that is neither `x` nor a parameter is an error at creation. Bounds, starting values and fixed values are set afterwards (`with_bounds`, `with_initial`, `with_fixed`).

## 2. Defaults

| Setting | Value | Source |
|---|---|---|
| Tolerance | 1e-9 | read from a running Origin 2016 (`tolerance` of the fit tree); how it is applied: section 3 |
| Maximum iterations | 400 | read from a running Origin 2016 (`maxIter` of the fit tree) |
| Confidence level | 0.95 | Origin report |
| Scale errors by reduced chi-square | always on, also with instrumental weights | section 6 (the instrumental case differs from the fit without scaling by exactly the square root of the reduced chi-square) |
| Derivatives | numeric, central differences, step 6e-6 times `max(|p|, 1e-3)`, taken across bounds | ours; the differences from Origin in every compared number stay below 5e-10 |

## 3. Algorithm

Residuals are `r_i = sqrt(w_i) (y_i - f(x_i; p))` over the points with a finite x, y and weight. Weights: none (1), instrumental (`1/err^2`, the column holds the errors), statistical (`1/y`), direct (as given). A point with a non-positive error, or non-positive y for statistical weighting, is dropped with a warning (what Origin does with such a point was not checked).

Starting values: the parameter's own starting value, else the function's guess, else 1. A starting value outside its bounds is first moved onto the nearest bound; Origin does the same (its first iterate from the moved point is reproduced to 14 digits).

Iteration, as reconstructed from Origin's iterates (section 6). With `J` the weighted Jacobian of `f` for the free parameters, `g = J'r`, `A = J'J` and a damping factor `lambda`:

1. Solve `(A + lambda diag(A)) d = g` for all free parameters, including those resting on a bound.
2. Cut every component of `p + d` back to its bounds. This is not an active-set method: the parameter on the bound still shapes the step of the others, which is why Origin's result with an active bound is usually not the constrained minimum (section 6).
3. If the sum of squares of the new point is not larger than the old one, accept it and divide `lambda` by 10. Otherwise multiply `lambda` by 10 and solve again.
4. `lambda` starts at 1e-3 and is carried from one iteration to the next.

The fit has converged after an accepted step when `old - new <= tolerance * (old + new)` for the sums of squares (the first iteration that meets this ends the fit and its step is kept), or when no step lowers the sum of squares any more (`lambda` above 1e14). If the iteration limit is reached the status is `MaxIterations` (Origin: `Failed(-200)`) and a warning is added. The iteration count equals Origin's in all compared fits (counting the iteration that detects convergence), when started from the same values.

## 4. The report

- Degrees of freedom: points minus free parameters (a parameter on a bound still counts as free; a fixed one does not).
- Covariance: `(J'J)^-1` times the reduced chi-square when errors are scaled. Standard error: square root of the diagonal. `t = value / SE`, `Prob>|t|` from the t distribution with the degrees of freedom, confidence limits `value ± t_crit SE`.
- Dependency: `1 - 1/(C_ii (C^-1)_ii)` over the free parameters.
- Reduced chi-square `rss/dof`; R-square `1 - rss/tss`; adjusted R-square `1 - (rss/dof)/(tss/(n-1))`, with `tss = sum w (y - ybar_w)^2` and the weighted mean.
- ANOVA: model df = free parameters, model SS = uncorrected total minus error SS, error row = residual, corrected total df `n-1`, uncorrected total `sum w y^2`; `F` = model MS / error MS; `Prob>F`.
- Fixed parameters have blank (NaN) error, t, probability, limits and dependency here. Origin's report sheet stores 0 for the error and the dependency of a fixed parameter and leaves t and Prob>|t| empty; the application should show a fixed parameter's error as blank.
- Warnings: a parameter at a bound, iteration limit reached, singular covariance matrix, dependency above 0.9999, points dropped.

## 5. Library, starting values, FDF import, user functions

### Library

`crates/fit/data/functions.json` holds, for each function with one independent variable: name, category, priority, parameter names with the bounds Origin switches on by default (a bound with state OFF in the file is not enforced), the formula, derived quantities (for example `FWHM = sqrt(2 ln 2) w`). It is trimmed from `docs/spec/data/fit-functions.json` and adds formulas written from the mathematics for functions that Origin writes with branches (Bigaussian, AsymmetricGaussian, Pulse, PWL2, PWL3, Step, ExpGrowDec, Power2, WeibullCDF, ExponentialCDF, SRichards1, BiDoseResp, Poisson, LognormalCDF; branches use `cond ? a : b`). Voigt (through the Faddeeva function, Weideman's rational approximation, checked against `exp(-x^2)` on the real axis and against the area of the profile) and GammaCDF are Rust code. Not available: functions with two or three independent variables, the four external-routine functions, Giddings, PearsonIV, SquareWave, SquareWaveMod, ExpGrow3Dec2, and those whose source text is defective (BatesWatts, Cole, Freundlich3PMod, FraserSuzuki, MWChang, PolyCtr5, Pulse1, GaussLog, LogisticPeak).

`library::get(name)` ignores case and returns a ready `FitFunction`; `library::list()` describes all; `library::derived_values` evaluates the derived quantities.

### Starting values

Origin's files compute starting values with helper routines that are not in the files (peak position, width estimates, exponent estimation); the files only show which helpers each function uses and some scale factors. Ours are our own routines with the same intent:

- Peak functions: baseline is the smallest value (largest for a downward peak), the centre the x of the largest distance from the baseline, FWHM from the interpolated half-height crossings, area by the trapezoid rule. Gauss `w = 0.85 FWHM` (Origin applies the same factor), `A = height * w sqrt(pi/2)`; GaussAmp `w = FWHM/2.355`, `A = height`; Lorentz `w = FWHM`, `A = pi/2 * height * w`; Voigt `wG = wL = 0.85 FWHM`, `A = area`; PsdVoigt1 `w = FWHM`, `mu = 0.5`, `A = area`; Bigaussian `w1 = w2 = FWHM/2.355`.
- Exponentials: the asymptote is put just beyond the flat end of the data (2 % of the span), then `ln|y - y0|` is fitted by a line. ExpDec1 `t1 = -1/slope`; ExpDec2 splits the amplitude in two and uses `0.9 t` and `1.1 t` (as Origin's file does); ExpDec3 uses `0.8 t, t, 1.2 t`; ExpAssoc starts both phases equal.
- Sigmoids: the end levels are the data extremes ordered by the trend, the centre is where the data cross the middle level, Boltzmann `dx = range/20`, Logistic `p = 3`, Hill `Vmax = max(y)`, `n = 1`, DoseResp `p = 1`.
- Line, Parabola, Cubic, Poly: least squares. Allometric1 and Power: fit in logarithms.
- Sine: offset is the mean, amplitude half the range, half-period from the strongest Fourier component, centre from the position of the maximum.

A test fits every such function from its guess on noisy generated data and requires the true parameters back within a few percent.

### FDF import

`fdf::parse_fdf(text)` reads the sections of an FDF file (see `fdf-format.md`) and `to_fit_function()` builds the function. The formula body is translated: comments and `#` lines dropped, `double t = ...;` temporaries substituted as parenthesised text, `if / else if / else` turned into nested conditionals (a branch that leaves `y` unassigned makes the file untranslatable), `log` to `ln` (natural logarithm in FDF bodies), `fabs` to `abs`, `PI` to `pi`, `NANUM` to `na()`. A formula that calls a routine the expression language lacks (`nlf_voigt`, `gamcdf`, ...) fails with a message when the function is built. Bounds are enforced only if the file's state flag is ON. A numeric initial value with `(F)` makes the parameter fixed. Starting-value code is not executed.

On this machine, of the 237 installed files all parse and 188 build into a fit function (the test prints the count and the reasons for the others: 31 have two or three independent variables, the remainder call native routines, have defective text or leave a branch unassigned).

## 6. Verified against Origin 2016

Method. A LabTalk script (`tools/origin-ref/nlfit-suite.lines.txt`, run by `run-labtalk-lines.ps1`) generates data in a hidden Origin (a smooth curve plus `a*sin(37 i) + b*sin(101.3 i)` as noise, `i` the row number), runs `nlbegin ... nlfit; nlend output:=1;`, and reads the fit report sheet and the fit tree. The data as Origin held them and Origin's numbers are stored in `crates/fit/tests/data/origin_nlsf.json`; `crates/fit/tests/origin_nlsf.rs` fits the same data and compares. In the report sheet the parameter rows give value, standard error, t, probability and dependency; the statistics block gives points, degrees of freedom, reduced chi-square, residual sum of squares, R-square, adjusted R-square and the status text; the ANOVA block gives the model, error, uncorrected total and corrected total rows. Calling `nlend output:=1` from COM showed no dialog in these runs; the earlier failed attempts were probably runs that waited for the lock held by another Origin run.

Cases (21): Gauss, Lorentz, ExpDec1, ExpDec2, Boltzmann, Line; Gauss with `y0` fixed; Gauss with an instrumental-weight column; ExpDec1 with statistical weights; eleven fits with an active lower or upper bound (one or two bounds, the start inside or outside the bound, a bound together with a fixed parameter, a bound on the weakly coupled centre of a Gauss); ExpDec2 stopped after 3 iterations.

Result with Origin's own starting values (test `engine_matches_origin`): the same number of iterations in all 21 fits; parameter values, standard errors, t, dependency, reduced chi-square, residual sum of squares, R-square, adjusted R-square, ANOVA sums of squares and F agree within a relative 5e-10 in the worst case. The test allows 1e-7. `Prob>|t|` agrees within an absolute 1e-9 plus 1e-6 relative.

Result with our own starting-value routines (test `own_starting_values_reach_origin_results`; fits with bounds and the iteration-limited fit are left out because their result depends on the path): parameters within 1e-5 and statistics within 1e-4 relative. The real differences are smaller; the looser limits allow for both programs stopping a few digits short of the exact minimum.

Findings that shaped the engine:

- Origin's first iterates were reproduced to 14 digits only with the damping `lambda diag(J'J)` (Marquardt scaling) at 1e-3 and divided by 10 after each accepted step; ExpDec1 iterations 2 and 3 matched with this and not with a constant 1e-3 or with halving.
- Bounded fits. With a parameter held on its bound, Origin's final point is often not the constrained minimum: for ExpDec1 with `t1 <= 1.5` Origin stops at a sum of squares of 1.809, while the fit with `t1` fixed at 1.5 reaches 0.396. It happens even for a straight line with the intercept bounded. Cutting the whole step back to the bounds, with the bounded parameter still in the step calculation, reproduces Origin's iterates and final point, so the engine does the same; a result that differs from the constrained minimum follows Origin and is not a bug here. (When the damping is lowered the next trial overshoots, is rejected, and the damping returns to 1e-3; this is why the damping looks constant in such fits.) A bound on a parameter that is nearly independent of the others (the centre of a Gauss) gives the true constrained minimum in both programs.
- Stopping. The first iteration whose fall in the sum of squares is below `1e-9 * (old + new)` ends the fit. Testing against `1e-9 * old` stopped one iteration late in three fits, and against an absolute 1e-9 one iteration early in one.
- Scaling. With instrumental weights Origin's standard errors equal the unscaled ones times the square root of the reduced chi-square (the test weights give a reduced chi-square of 0.70).
- A fixed parameter does not count in the degrees of freedom; the ANOVA model degrees of freedom are the number of free parameters.
- A parameter on a bound keeps its standard error, t and dependency and counts as free in the degrees of freedom (27 for 30 points and 3 parameters in the `t1 <= 1.5` fit).
- Origin's own starting values for the compared functions (for example Gauss `y0 = 1.205, xc = 5, w = 0.898, A = 3.708` on our test data) are stored in the fixture as `origin_initial`; our starting-value routines were not made to reproduce them.

Not verified: functions other than the six above (the other library functions are checked only against their own noisy data), instrumental weights with a dropped point, direct weighting, confidence limits (computed from the t distribution; they were not read from Origin's report), fits with more than 100 points, behaviour when the model is undefined at the starting point, global or multi-dataset fits (not implemented).
