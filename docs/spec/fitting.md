# Nonlinear curve fitting

Summary. `sheaf-fit` fits a model `y = f(x; p)` to points by minimising the (weighted) sum of squared residuals with a Levenberg-Marquardt iteration, with optional bounds and fixed parameters, and returns the same `FitReport` as linear fitting. The built-in functions (206 of Origin's 237 that have one independent variable and a usable formula), user-defined formulas and imported `.FDF` files all become one `FitFunction`. Numbers are compared with Origin 2016 (section 6). Linear and polynomial fits are in `fitting-linear.md`.

Sources: Origin's public documentation on nonlinear curve fit, `NLSF.ini` (defaults), the FDF files (parameter lists, bounds, formulas), and runs of Origin on data we generated (section 6). Everything below is written in our own words.

## 1. The model

A `FitFunction` has: a name, an equation text, parameter names, one independent variable (default name `x`), a model, and an optional routine guessing starting values. The model is either a formula in the expression language (`expression-language.md`; parameters and `x` are variables) or Rust code `(params, x) -> y`. Each parameter has an optional lower bound, an optional upper bound, a starting value (NaN: guess or 1) and a "fixed" flag.

A user-defined function is created with `FitFunction::from_formula(name, "y0 + A*exp(-x/t)", &["y0", "A", "t"])`. A leading `y =` is accepted. A name in the formula that is neither `x` nor a parameter is an error at creation. Bounds, starting values and fixed values are set afterwards (`with_bounds`, `with_initial`, `with_fixed`).

## 2. Defaults

| Setting | Value | Source |
|---|---|---|
| Tolerance | 1e-9 (relative decrease of the sum of squares) | `NLSF.ini` `tolerance` |
| Maximum iterations | 400 | Origin 2016 fit dialog; `NLSF.ini` lists 100, an older default |
| Confidence level | 0.95 | Origin report |
| Scale errors by reduced chi-square | on, except for instrumental weighting | see section 6 |
| Derivatives | numeric, central differences, step 6e-6 times `max(|p|, 1e-3)` | ours (more accurate than Origin's 0.005 relative step, see section 6) |

## 3. Algorithm

Residuals are `r_i = sqrt(w_i) (y_i - f(x_i; p))` over the points with a finite x, y and weight. Weights: none (1), instrumental (`1/err^2`, the column holds the errors), statistical (`1/y`), direct (as given). A point with a non-positive error, or non-positive y for statistical weighting, is dropped with a warning.

Iteration (Marquardt scaling): with `J` the weighted Jacobian of `f` for the free parameters, solve `(J'J + lambda diag(J'J)) d = J'r`, clip `p + d` into the bounds, and accept the step if the sum of squares does not increase (then divide `lambda` by 10) or else multiply `lambda` by 10 and retry. A free parameter that sits on a bound and whose gradient pushes outwards is left out of that step. The fit stops as converged when an accepted step lowers the sum of squares by less than `tolerance` times its value and the largest relative parameter change is below 1e-7, when no step can lower it any more, or when every free parameter is pinned by a bound. If the iteration limit is reached the status is `MaxIterations` and a warning is added. The number of iterations reported is our own count and is not expected to equal Origin's.

## 4. The report

- Degrees of freedom: points minus free parameters (a parameter on a bound still counts as free; a fixed one does not).
- Covariance: `(J'J)^-1` times the reduced chi-square when errors are scaled. Standard error: square root of the diagonal. `t = value / SE`, `Prob>|t|` from the t distribution with the degrees of freedom, confidence limits `value ± t_crit SE`.
- Dependency: `1 - 1/(C_ii (C^-1)_ii)` over the free parameters.
- Reduced chi-square `rss/dof`; R-square `1 - rss/tss`; adjusted R-square `1 - (rss/dof)/(tss/(n-1))`, with `tss = sum w (y - ybar_w)^2` and the weighted mean.
- ANOVA: model df = free parameters, model SS = uncorrected total minus error SS, error row = residual, corrected total df `n-1`, uncorrected total `sum w y^2`; `F` = model MS / error MS; `Prob>F`.
- Fixed parameters have blank (NaN) error, t, probability, limits and dependency.
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

## 6. Comparison with Origin 2016

See the section added after the Origin runs (below).
