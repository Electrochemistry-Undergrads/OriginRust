# Nonlinear curve fitting

Summary. `sheaf-fit` fits a model `y = f(x; p)` to points by minimising the (weighted) sum of squared residuals with a Levenberg-Marquardt iteration, with optional bounds and fixed parameters, and returns the same `FitReport` as linear fitting. The built-in functions (206 of Origin's 237 that have one independent variable and a usable formula), user-defined formulas and imported `.FDF` files all become one `FitFunction`. Numbers are compared with Origin (section 6): same iteration counts, differences below 5e-10.

Sources: Origin's public documentation on nonlinear curve fit, the FDF files (parameter lists, bounds, formulas), and runs of Origin on data we generated (section 6). Everything below is written in our own words.

## 1. The model

A `FitFunction` has: a name, an equation text, parameter names, one independent variable (default name `x`), a model, and an optional routine guessing starting values. The model is either a formula in the expression language (`expression-language.md`; parameters and `x` are variables) or Rust code `(params, x) -> y`. Each parameter has an optional lower bound, an optional upper bound, a starting value (NaN: guess or 1) and a "fixed" flag.

A user-defined function is created with `FitFunction::from_formula(name, "y0 + A*exp(-x/t)", &["y0", "A", "t"])`. A leading `y =` is accepted. A name in the formula that is neither `x` nor a parameter is an error at creation. Bounds, starting values and fixed values are set afterwards (`with_bounds`, `with_initial`, `with_fixed`).

## 2. Defaults

| Setting | Value | Source |
|---|---|---|
| Tolerance | 1e-9 | read from a running Origin (`tolerance` of the fit tree); how it is applied: section 3 |
| Maximum iterations | 400 | read from a running Origin (`maxIter` of the fit tree) |
| Confidence level | 0.95 | Origin report |
| Scale errors by reduced chi-square | on, also with instrumental weights; `FitOptions::scale_errors`, the NLFit window's "Scale Error with sqrt(Reduced Chi-Sqr)" box and `--no-scale-errors`. Without weights it is always on, whatever the switch says | section 6; the "always on without weights" rule is from Origin's own fit code (it forces the scaling when a data set has no weights), not measured through LabTalk |
| Derivatives | numeric, central differences, step 6e-6 times `max(|p|, 1e-3)`, taken across bounds | ours; the differences from Origin in every compared number stay below 5e-10 |

## 3. Algorithm

Residuals are `r_i = sqrt(w_i) (y_i - f(x_i; p))` over the points with a finite x, y and weight. Weights: none (1), instrumental (`1/err^2`, the column holds the errors), statistical (`1/|y|`, the same rule as the linear fits, where Origin gave the `1/|y|` result for a negative y; a y of exactly zero is refused with an error that names the cause), direct (as given; a weight of zero or below leaves the point out, so it does not count in the degrees of freedom, and if too few points keep a positive weight the fit is refused). Direct weighting is the instrumental case with the error `1/sqrt(w)`: Origin's `nlbegin` offers only `none`, `ins` and `stat`, so direct weights were checked as Origin's instrumental fit of `err = 1/sqrt(w)` (section 6). Before fitting, a lower bound above the upper bound, a bound that is not a number, or a fixed parameter whose value is not finite is refused with an error; a sum of squares that overflows although the model is defined is reported as an overflow. A point with a non-positive error is dropped with a warning (what Origin does with such a point was not checked).

Starting values: the parameter's own starting value, else the function's guess, else 1. A starting value outside its bounds is first moved onto the nearest bound; Origin does the same (its first iterate from the moved point is reproduced to 14 digits).

Iteration, as reconstructed from Origin's iterates (section 6). With `J` the weighted Jacobian of `f` for the free parameters, `g = J'r`, `A = J'J` and a damping factor `lambda`:

1. Solve `(A + lambda diag(A)) d = g` for all free parameters, including those resting on a bound.
2. Cut every component of `p + d` back to its bounds. This is not an active-set method: the parameter on the bound still shapes the step of the others, which is why Origin's result with an active bound is usually not the constrained minimum (section 6).
3. If the sum of squares of the new point is not larger than the old one, accept it and divide `lambda` by 10. Otherwise multiply `lambda` by 10 and solve again.
4. `lambda` starts at 1e-3 and is carried from one iteration to the next.

The fit has converged after an accepted step when `old - new <= tolerance * (old + new)` for the sums of squares (the first iteration that meets this ends the fit and its step is kept), or when no step lowers the sum of squares any more (`lambda` above 1e14). If the iteration limit is reached the status is `MaxIterations` (Origin: `Failed(-200)`) and a warning is added. The iteration count equals Origin's in all compared fits (counting the iteration that detects convergence), when started from the same values.

Stepping. `Fitter::new` prepares the fit, `step` makes one iteration of the above and returns a `StepOutcome` (iteration number, sum of squares, reduced chi-square, `lambda`, converged, parameters), `finish` builds the report. `fit` is a loop of at most `max_iterations` steps, so n steps equal a fit limited to n iterations, bit for bit. The NLFit-style dialog runs a fit limited to one iteration from the values in its Parameters table, so `lambda` restarts at 1e-3 at every press of 1 Iteration or Fit (not compared with Origin).

## 4. The report

- Degrees of freedom: points minus free parameters (a parameter on a bound still counts as free; a fixed one does not).
- Covariance: `(J'J)^-1` times the reduced chi-square when errors are scaled. Standard error: square root of the diagonal. `t = value / SE`, `Prob>|t|` from the t distribution with the degrees of freedom, confidence limits `value ± t_crit SE`.
- Dependency: `1 - 1/(C_ii (C^-1)_ii)` over the free parameters.
- Reduced chi-square `rss/dof`; R-square `1 - rss/tss`; adjusted R-square `1 - (rss/dof)/(tss/(n-1))`, with `tss = sum w (y - ybar_w)^2` and the weighted mean.
- ANOVA: model df = free parameters, model SS = uncorrected total minus error SS, error row = residual, corrected total df `n-1`, uncorrected total `sum w y^2`; `F` = model MS / error MS; `Prob>F`.
- Fixed parameters: the error is 0 (and the dependency 0 in nonlinear fits), as in Origin's report sheet, which stores 0 for both; t, Prob>|t| and the limits are blank (NaN).
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

Gaussian has parameters (y0, xc, A, w) with `A` the area and `w` the FWHM: its guess is `A = area`, `w = FWHM`. Every guess was checked against its function's parameter order and conventions (FWHM or sigma, area or height).

A test fits every such function (every name with a guess, 57 cases; a test also fails if a function with a guess has no case) from its guess on noisy generated data and requires the true parameters back within a few percent. The whole table runs at y scales 1, 1e-3 and 1e-6: offsets, amplitudes and areas and the noise are multiplied by the scale. Power (`y = x^A`) has no prefactor, so there its x range is scaled by `scale^(1/A)`, which scales y by `scale`. Poly is fitted with its six highest coefficients held at zero and ExpGrow1 with `x0` held at 0 (it trades off against `A1`). The phases of ExpDec2, ExpDec3 and ExpAssoc are compared ordered by time constant, because the fit may swap them.

### FDF import

`fdf::parse_fdf(text)` reads the sections of an FDF file (see `fdf-format.md`) and `to_fit_function()` builds the function. The formula body is translated: comments and `#` lines dropped, `double t = ...;` temporaries substituted as parenthesised text, `if / else if / else` turned into nested conditionals (a branch that leaves `y` unassigned makes the file untranslatable), `log` to `ln` in the formula (natural logarithm in the Origin C formula text; no function in the library uses `log` there) and to `log10` in a derived-parameter line (those are LabTalk, where `log(100)` is 2: checked in Origin), `fabs` to `abs`, `PI` to `pi`, `NANUM` to `na()`. A formula that calls a routine the expression language lacks (`nlf_voigt`, `gamcdf`, ...) fails with a message when the function is built. Bounds are enforced only if the file's state flag is ON. A numeric initial value with `(F)` makes the parameter fixed. Starting-value code is not executed.

On this machine, of the 237 installed files all parse and 188 build into a fit function (the test prints the count and the reasons for the others: 31 have two or three independent variables, the remainder call native routines, have defective text or leave a branch unassigned).

## 6. Verified against Origin

Method. A LabTalk script (`tools/origin-ref/nlfit-suite.lines.txt`, run by `run-labtalk-lines.ps1`) generates data in a hidden Origin (a smooth curve plus `a*sin(37 i) + b*sin(101.3 i)` as noise, `i` the row number), runs `nlbegin ... nlfit; nlend output:=1;`, and reads the fit report sheet and the fit tree. The data as Origin held them and Origin's numbers are stored in `crates/fit/tests/data/origin_nlsf.json`; `crates/fit/tests/origin_nlsf.rs` fits the same data and compares. In the report sheet the parameter rows give value, standard error, t, probability and dependency; the statistics block gives points, degrees of freedom, reduced chi-square, residual sum of squares, R-square, adjusted R-square and the status text; the ANOVA block gives the model, error, uncorrected total and corrected total rows. Calling `nlend output:=1` from COM showed no dialog in these runs. Why an earlier attempt by a previous agent produced no output was not determined.

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

Not verified: functions other than the six above (the other library functions are checked only against their own noisy data), instrumental weights with a dropped point, confidence limits (computed from the t distribution; they were not read from Origin's report), fits with more than 100 points, behaviour when the model is undefined at the starting point, the scale-errors switch turned off (no LabTalk route to it was found: the fit tree of `nlbegin` has no node for it, and guesses at about 50 node names changed nothing; the unscaled errors are the scaled ones divided by the square root of the reduced chi-square, as Origin's help describes), instrumental or direct weights in a global fit with a dropped point.

Direct weighting (test `direct_weighting_and_unscaled_errors_follow_origin_instrumental_fit`): the Gauss case with instrumental errors, refitted with direct weights `1/err^2`, reproduces Origin's values and standard errors and its iteration count. A direct weight of zero or below leaves the point out (Origin's fit code excludes such weights; negative ones are an error code there), in the nonlinear and in the linear fits.

Quick-fit functions (test `quick_fit_functions_match_origin`, `tools/origin-ref/nlfit-quick-gen.py`): ExpGrow1 with `x0` held at 0, Logistic, Hill and DoseResp agree with Origin like the first 21 cases, with the same iteration counts (4, 4, 8, 4), from Origin's starting values and from ours.

## 10. Several datasets: independent and global fits

Source: `nlbegin iy:=(1,2:3) func:=Gauss mode:=global nltree:=tt`, `tt.s_xc = 1` (share the parameter `xc`), `tt.f_y0_2 = 1` (fix the copy of dataset 2), then `nlfit` and `nlend output:=1` (`tools/origin-ref/nlfit-global-gen.py`; raw output `re-workspace/scratch/fit-breadth/nlfit-global.out`, local only). `mode:=auto` and `cons` (Independent Fit, consolidated report) fit each dataset alone; `iy:=((1,2,5),(1,3,6))` with `weight:=ins` gives each dataset its error column. What Origin does, as measured on 6 cases (2 and 3 datasets, none, one or two shared parameters, instrumental weights, fixed parameters):

- One fit of the stacked data. Parameter names: dataset 1 keeps the plain name (`y0`), dataset k is `y0_k`; a shared parameter is one parameter. Degrees of freedom: all points minus the free parameters (shared once; fixed ones not counted): 40 points, 7 free parameters, 33.
- Covariance, standard errors, t, dependency: from the stacked problem, scaled by the reduced chi-square of all points. With nothing shared the values equal the single fits (the errors do not: their reduced chi-square is that of all the data).
- Statistics: residual sum of squares of all points and of each dataset (Origin lists both); R-square of each dataset against its own mean; the R-square and adjusted R-square of all points use the mean of all points and `n - 1`.
- ANOVA: model degrees of freedom = free parameters; the corrected total has `n - (number of datasets)` degrees of freedom and the sum of the datasets' sums about their own means; uncorrected total and F as in a single fit.
- The iteration count is not in the report (`tt.niter` reads 0 in global mode), so only results are compared: values, errors, t, dependency, statistics, ANOVA agree within 1e-6 relative (test `global_fits_match_origin`; the smallest differences seen are well below that).

`sheaf_fit::global::fit_global` stacks the datasets into one problem for the ordinary engine through a composite function whose x is the position in the stack, so bounds, fixed parameters, weights, damping and stopping are the single-fit ones; `GlobalFit::combined` is the stacked report (composite parameter names, ANOVA corrected total as above) and `GlobalFit::datasets` the view of each dataset in the function's parameter order (the dataset's own point count, residual sum of squares and R-square; the degrees of freedom and reduced chi-square of the whole fit; adjusted R-square blank as in Origin). A dataset's copy of a parameter can have its own start value or fixed value (`ParamOverride`).

`Project::fit_multi(requests, shared)`: `None` is independent fits (each as `Project::fit`); `Some(names)` a global fit (nonlinear models only). Every column still gets its fitted and residual columns, `FitNL<k>` sheets (values, errors and limits of the parameters it uses, with the whole fit's degrees of freedom), curve and graph entry; a global fit adds `FitNLGlobal<k>` (the composite table with Fixed and Shared columns) and its `Stats` sheet. All requests share the first one's model, settings, weighting, iteration limit and tolerance. The weight or error column of weighted fits is each column's own Y-error column (instrumental) or the one chosen column (direct). Nothing new is stored in the project beyond ordinary sheets and columns, so the `.opj` writer needs no new warning. Not offered: independent fits consolidated into one report sheet, replica fits, per-dataset start values and bounds in the window (the table applies to all datasets), unequal x per dataset is allowed.

Quick fits (Analysis > Fitting > Exponential Fit, Sigmoidal Fit, Single Peak Fit). Origin's `Fitexp`, `Fitsigmoidal` and `Fitpeak` X-functions are NLFit with a preset (read from the installed definitions; the X-functions return false when called through COM without their dialog, so the numbers come from `nlbegin` with the same functions, section 6): Exponential offers ExpDec1 (default), ExpDec2, ExpDec3 and ExpGrow1; Sigmoidal offers Boltzmann (default), Logistic, Hill and DoseResp; Single Peak offers Gauss (default) and Lorentz. The category cannot be changed. Everything else keeps NLFit's defaults (tolerance 1e-9, 400 iterations, no weights; the peak-analyzer style baseline and peak table of Origin's Single Peak Fit are not offered).

## 9. The NLFit window: buttons and layout

Source: OriginLab's online help on the NLFit dialog and its buttons (read 2026-11-12; the wording here is ours). Not tested in Origin itself: the dialog cannot be opened headless, and `nlbegin`/`nlfit`/`nlend` have no button equivalents beyond what section 6 already compared (an iteration-limited `nlfit` followed by `nlend output:=1` writes a report whose status is Failed(-200); that is what Fit and Done write when the limit is hit).

What the buttons do in Origin:

- **1 Iteration**: one Levenberg-Marquardt step from the values now in the Parameters tab; the new values go back into the tab, the message goes to the Messages tab; the dialog stays open.
- **Fit until converged** (an icon button): iterates until the tolerance or the iteration limit (Advanced page, Fit Control) is reached; the dialog stays open and the button turns into OK.
- **Fit** (text button): runs whatever iterations are still needed, writes the report and the output sheets, and closes the dialog.
- **Done**: takes the values as they are, with no further iteration, converged or not, writes the results and closes.
- **Cancel**: closes the dialog; nothing is written. The help does not say whether the iterated parameter values survive anywhere; since the results sheets are only made by Fit and Done, Sheaf keeps nothing.
- When the iteration limit is hit, Fit ends there and the report carries the status "not converged"; the help suggests raising the limit and fitting again from the dialog.
- Layout: tabs Settings / Code / Parameters / Bounds (upper panel), a button row, and a lower panel with the tabs Fit Curve, Formula, Sample Curve, Message, Function File, Residual and Hints; two arrow buttons show or hide each panel.

Sheaf's window (`crates/app/src/dialogs/fit.rs`):

- Same tabs; Settings has the list Function Selection, Data Selection, Fitted Curves, Find X/Y, Advanced, Output. Category and Function drop-downs live on Function Selection, the weights, column and rows on Data Selection.
- Buttons: 1 Iteration, Fit until converged and Revert (Revert is ours) keep the window open and work on the Parameters table on a copy of the project; Fit and Done write the results through one `Document::edit` ("Fit", one undo step) and close; Cancel closes without writing.
- Fit writes with the table values as starting values and the dialog's iteration limit. Done writes with `max_iterations = 0` (the engine makes no step and reports the starting values) unless the table equals the values at which the last trial converged, in which case it runs normally and one pass confirms the convergence. A line or polynomial has no iterations; Fit and Done are the same.
- Fitted Curves: Number of Points (default 1000), X range (the input data range, or a custom Min/Max), Plot on Source Graph, Update Legend on Source Graph (drops hand-written legend text of the layers that receive the curve so the legend lists it), and the bands. Not offered: plot type, stacking with the residual-vs-independents plot, preview on the source graph, plotting in the report table.
- Find X/Y: a page with a note only. Output: a page describing where results go; Origin's choices of sheet names, tables and arrangement are not offered.
- Lower panel (fold with the "Hide lower panel" button): Fit Curve (data as dots and the curve at the table values, drawn with the egui painter, made by a fit with no iterations on a copy), Residual, Formula, Messages. Not offered: Sample Curve, Function File, Hints.
- Weights (Data Selection): None, error bars (1/err^2), 1/|y| and Direct with a weight column chosen from the sheet; the box "Scale Error with sqrt(Reduced Chi-Sqr)" is on the Advanced page (on by default, ignored without weights). "Also fit" lists the other Y columns of the sheet; with one ticked a choice appears: Independent Fit or Global Fit (the Parameters tab then has a Shared column). Quick fits open the same window with a title of their own, the category locked and the function list cut to their functions. Section 10.
- Not offered: Dialog Theme row, the icon buttons for Simplex, Initialize Parameters, Calculate Chi-Square, 100 iterations.
