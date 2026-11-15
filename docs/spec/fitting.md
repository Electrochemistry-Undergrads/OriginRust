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

## 11. Compare Models and Rank Models

Source: `fitcmpmodel result1:=[Book]FitNL1 result2:=[Book]FitNL2 aic:=1 bic:=1 ftest:=1 param:=1 statics:=1 rt:=<new name:=cmpA>` after two `nlbegin ... nlfit; nlend output:=1` fits of one dataset (`tools/origin-ref/fitcmp-suite1..3.lines.txt`, generated by `fitcmp-gen-suite1.py` and `fitcmp-gen-suite2.py`; the report sheet is read by `fitcmp-make-fixture.py` into `crates/fit/tests/data/origin_fitcmp.json`: 24 pairs of fits). Test `comparison_matches_origin` takes the residual sum of squares, points and parameters of each model as Origin printed them and compares everything else.

- Free parameters `k` of a model = points minus degrees of freedom. The error variance counts as one more parameter, `k' = k + 1`.
- AICc = `n ln(rss/n) + 2k' + 2k'(k'+1)/(n-k'-1)` (undefined, left empty, when `n - k' - 1` is not positive). Akaike weight `exp(-d/2) / sum exp(-d_i/2)`, `d` = AICc minus the smaller one. The preferred model is the one with the smaller AICc. Matches to 1e-12 (AICc) and 1e-9 (weights).
- BIC = `n ln(rss/n) + k' ln n`; Origin also gives the BIC minus the smaller BIC. A difference of 2 or less is "Inconclusive", otherwise the smaller BIC wins.
- F-test: the model with more degrees of freedom is the null model; `F = |(rss0 - rss1)/df1| / (rss1/df2)` with `df1` the difference of the degrees of freedom and `df2` those of the other model. Prob>F from the upper tail of F(df1, df2); below 0.05 the model with fewer degrees of freedom is preferred, otherwise the null model. Equal degrees of freedom give "No conclusion". Origin's printed Prob>F is off the exact upper tail (scipy and our value agree) by up to 4e-6 relative, and 1.6e-4 for probabilities near 1e-13; the test tolerances are 1e-5 and 1e-3.
- The conclusions list one row per comparison that was switched on, in the order AIC, BIC, F-test; the test compares our verdict with each.

Sheaf: `Project::compare_models(a, b)` fits both requests' models to the same Y column without placing their own report sheets and writes one sheet `FitCompare<n>` (Quantity, Model 1, Model 2, Text columns: points, parameters, degrees of freedom, residual sum of squares, AICc and weights, BIC and difference, F, df1, df2, Prob>F, the three conclusions, warnings). Analysis > Fitting > Compare Models... chooses the Y column and two models (Linear, Polynomial of order 2 or 3, any built-in function; default settings, no weights). `sheaf fit --model M --compare M2`.

Rank Models: Origin's `funcRank` could not be driven by script here (eight attempts with `funsel` trees and `rdRes` outputs gave no result sheet), so Sheaf's ranking is not compared with Origin. It fits each chosen built-in function and orders them by AICc (smallest first; a function whose AICc is undefined, or that cannot be fitted, goes last or is listed as skipped), with Akaike weight, BIC, BIC difference and adjusted R-square, using the criteria that are Origin-verified above. `Project::rank_models`, sheet `FitRank<n>`, Analysis > Fitting > Rank Models... (category list with Select all), `sheaf fit --rank F1,F2,...`.

## 12. Multiple linear regression and linear fit with X error

Multiple Linear Regression, source `fitMR dep:=... indep:=... mrtree:=mrt` (`tools/origin-ref/fitmr-gen.py`; fixture `crates/fit/tests/data/origin_fitmr.json`: free intercept with one, two and three variables, and an intercept held at 0, 1.5 and -2; test `multiple_regression_matches_origin`). Coefficients agree to 1e-9, standard errors, t-values, sums of squares, mean squares, RMSE, R-square, R, F to 1e-8, Prob>|t| and Prob>F to 1e-6. Definitions: degrees of freedom `n - p` with `p` the free coefficients; free intercept: total sum of squares about the mean, `n - 1` degrees of freedom; fixed intercept (at any value): total about zero, `n` degrees of freedom, as in the straight-line fit; ANOVA model row `tss - rss`. Origin's tree has no adjusted R-square or confidence limits; Sheaf computes them (`1 - (rss/dof)/(tss/total_df)`, t quantiles). `sheaf_fit::multi::fit_multiple`; `Project::fit_multiple_regression` adds fitted and residual columns after the Y column and the sheets `FitMR<n>` and `FitMR<n>Stats`. Rows with a missing value in any column are left out. Analysis > Fitting > Multiple Linear Regression... (Y, any number of X columns, fix intercept); `sheaf fit --regress X1,X2 [--fix Intercept=V]`.

Linear Fit with X Error (`FitModel::LinearXError`). Origin's routine could not be reached by script (`nlfit` with an X-error column and `weight:=ins` weights by Y error only), so this is not compared with Origin. `sheaf_fit::xerr` minimises `sum (y - a - b x)^2 / (sy^2 + b^2 sx^2)` with York's iteration and gives York's standard errors multiplied by `sqrt(S/(n-2))` unless the scale switch is off; it reproduces the weighted straight-line fit for zero X error and York's published data set. The X-error column is the X-error column right after the Y column (as for plots), or `FitRequest::x_error`; weighting is none or instrumental (Y errors). Report sheets are `FitLinear<n>` like the straight line; confidence and prediction bands use the parameter covariance. Analysis > Fitting > Linear Fit with X Error... opens the Fit window with the category "Linear Fit with X Error" (no fixed parameters); `sheaf fit --model linearx --xerr COLUMN`.

Storage: these tools only create ordinary worksheets and columns (no analysis-link or other new object), so the `.opj` writer needs no new warning.

## 13. Simulate Curve

Analysis > Fitting > Simulate Curve (Origin's `simcurve` X-Function) makes the curve of a fit function for parameter values that are typed instead of fitted. Task T47. Simulate Surface (two independent variables) waits for matrices and stays grey.

Status of the facts: nothing here is measured in Origin (no Origin on the machine that wrote this). From Origin's public documentation only the shape is taken: a category and function, parameters with initial values, an X range (from, to, number of points), output to a workbook, an optional graph. Every choice below is labelled **Sheaf's choice** and is listed in "To measure on Windows" at the end; none of them may be quoted as Origin's behaviour.

### What is computed

- The function is any entry of the fit library (`sheaf_fit::library`, the list the NLFit window offers; user-defined functions are not offered yet). Values come from `FitFunction::eval`, the evaluator of the fits, so the formula text, the Rust-coded Voigt and GammaCDF and the branch conditionals behave exactly as in a fit. `sheaf_fit::simulate::simulate(function, params, xs)` returns one value per X; a point where the function has no value (a logarithm of a negative number, a pole) is NaN, shown as a missing value. If there is no value at any X the call fails with a message instead of writing an empty column. Parameters must be finite numbers; their number must match the function's.
- Parameter bounds and "fixed" flags of the fit library are ignored: the typed values are used as they are (a width of -1 is the user's request, not an error).
- **Sheaf's choice** (starting values): the parameter grid starts at `initial_params`: the function's own initial value where the library has one, otherwise 1 (what a fit starts from when it has no guess), moved into the parameter's default bounds if those exclude 1 (a lower bound of 3 gives 3). Where 1 makes the function degenerate (division by zero, ln 0, overflow) another value is used: Beta w2 = w3 = 2, BoxLucas2 a1 = 2, Log2P1 a = 0, Logarithm A = 0, MYEGA C = 0.1, SRichards1 d = 0.5, SRichards2 d = 2. Origin's own initial values are not in the data we have (only 39 parameters of 237 functions carry one) and its guess routines need data, so the real defaults must be measured. A test simulates every library function at these values on X = 0.1 to 10 (Bradley on 0.1 to 0.9, where ln(-b ln x) exists) and requires finite values.
- X values (`x_values(from, to, points, spacing)`): linear `from (1 - t) + to t` with `t = i/(n-1)` (not `from + (to - from) t`, whose difference overflows for a range wider than the largest number), or logarithmic `exp(ln from + (ln to - ln from) i/(n-1))`, both with the first and last value exactly the typed bounds. Needs `points` of 2 to 1 000 000, finite bounds, `from != to` (`from > to` gives falling X), and positive bounds for logarithmic spacing. The X values may instead be those of an existing numeric column, row by row (a missing X gives a missing Y; text columns are refused).
- **Sheaf's choice** (defaults of the window and of `SimulateCurveSpec::default()`): function Gauss, X from 1 to 10, 100 points, linear. Chosen so that both spacings are valid without editing; Origin's defaults are not known.

### Where it goes

`Tool::SimulateCurve(SimulateCurveSpec)` of `Project::analyze` writes a new sheet `Simulate<n>` in the workbook of the request's first column, with an X column and a Y column. The tool reads no data except an optional X column, so the request's `columns` only name the workbook (with an X column as source, pass that column).

- **Sheaf's choice** (names): the X column's long name is `X` (the source column's long name, with its unit, when X comes from a column) and its comment `N points from A to B, linear spacing` (or `logarithmic`, or `X of column C`); the Y column's long name is the function name (`Gauss`) and its comment `Simulated Gauss: y0=0, xc=5, w=1, A=1`, the parameters in the function's order. The report notes give the equation and the parameter list. The sheet name prefix `Simulate` and the long names are not Origin's.
- **Sheaf's choice** (output): the tool always writes a new sheet of the workbook of the request's column. The window's Output list offers "New workbook" and "New sheet in <book>" for every workbook (default: the active one, or a new workbook if there is none); with X of a column the sheet goes to that column's workbook and the list is greyed. "New workbook" makes the workbook, runs the tool on a placeholder column, then drops the empty first sheet, all inside one undo step. Origin offers a new workbook or an existing sheet; whether it writes into an existing sheet is not known.
- `plot` draws the curve as a line graph (log10 X axis for logarithmic spacing; axis titles X and the function name) in the same step. One call to `analyze` is one undo step of the document: the sheet and the graph are removed together.
- Recalculate: the tool can be linked (Auto or Manual). The link follows the X column of `SimulateX::OfColumn`; with a typed range nothing changes, so linking only makes sense with a column. The graph belongs to the first run; a repeated run replaces the values in the same columns and draws no second graph.
- Storage: ordinary worksheet columns, a graph and the usual analysis link; the `.opj` writer needs no new warning.

### The window (Analysis > Fitting > Simulate Curve..., `crates/app/src/dialogs/simulate.rs`)

Sheaf's own layout until the window of Origin is measured; every control below is **Sheaf's choice**.

- Category and Function lists (the categories and functions of `sheaf_fit::library::list()`; not shared with the NLFit window's code, which also carries the linear, polynomial and user-defined pages) and the formula under them.
- A parameter grid with one field per parameter, prefilled with `initial_params` of the function (the starting values of the tool, see above); choosing another function resets the values; "Initial values" puts them back. Typed values are used as typed, bounds are not applied.
- X range (From, To, Points, "Logarithmic spacing") or "X of a column" (the numeric columns of the active sheet of the workbook that was active when the window opened; greyed when there are none). Points are not clamped in the field: a count below 2 or above `MAX_POINTS` is refused with the tool's message.
- Output list and "Plot the curve" (off by default, like `SimulateCurveSpec::default()`).
- A live preview of the curve under the controls: the tool's own simulation on at most 400 points (at most 2000 rows of a chosen column), drawn with the fit window's small plot, redrawn when a setting or the project changes. An error that the run would give (equal bounds, log with a non-positive bound, a function with no value anywhere) is shown in red in place of the curve.
- OK runs `Tool::SimulateCurve` through `Document::edit` as one undo step ("Undo Simulate curve"), focuses the new workbook (its new sheet active) and the graph if one was made, and puts the report message on the status line. A refused run changes nothing and keeps the window open with the message under the preview. Cancel, Escape and the close button change nothing. The window does not offer a Recalculate mode yet; the core tool is linkable (`LinkTool::SimulateCurve`).
- `sheaf-app --show simulate[:FUNCTION]` opens the window on the first workbook (for screenshots).

### API

`sheaf_fit::simulate`: `Spacing { Linear, Log }`, `MAX_POINTS`, `x_values`, `initial_params`, `resolve_params(function, &[(name, value)])` (names match exactly, else ignoring case when unambiguous; unknown or repeated names are errors), `simulate`, `describe_params`; `FitError::BadSimulation(String)`. `sheaf_core::analysis`: `SimulateCurveSpec { function, params, x, plot }`, `SimulateX { Range { from, to, points, spacing }, OfColumn(ColumnId) }`, `SimulateSpacing`, `Tool::SimulateCurve`.

### Command line and MCP (`crates/ops/src/simulate.rs`)

`sheaf simulate [INPUT] [--function NAME] [--param NAME=VALUE]... [--from A --to B --points N [--log] | --x-of COLUMN] [--book B --sheet S] [--plot FIGURE [--dpi D]] [--graph] [-o FILE] [--save PROJECT]` and `sheaf simulate --list`. `sheaf-ops` has one `SimulateSpec` and one `run(project, spec)` that call `Tool::SimulateCurve` on the project, so the numbers of the command line, the MCP server and the app are the same function of the same code; there is no second evaluator. All of it is **Sheaf's choice** (nothing is measured in Origin):

- The defaults are those of the tool: Gauss, X from 1 to 10, 100 points, linear. `--param` repeats and takes commas (`-p xc=5,w=2`); a parameter not given keeps the starting value that `--list` prints; names match as in `resolve_params`. A bound is not applied to a typed value.
- `INPUT` (a data file, a `.sheaf`, `.opj` or `.opju` project) is the workbook that gets the new sheet `Simulate<n>`; the first column of the sheet only anchors the request. `--x-of COLUMN` takes the X values of that column (in `--sheet`, default the active sheet) and cannot be combined with `--from`, `--to`, `--points` or `--log`; it needs an input. Without `INPUT` the run makes a project of its own that holds only the new sheet.
- `-o FILE` writes the X and Y columns (with the long names and comments as the sheet has them) as delimited text (tabs for `.tsv` and `.txt`, commas otherwise) or, for `.xlsx`, as an Excel workbook; `--save PROJECT` saves the project with the new sheet (and the graph); `--plot FIGURE` draws the curve (log spacing gives a log X axis) and writes the figure, and `--graph` only adds the graph to the saved project. With neither `-o` nor `--save` the curve is printed as CSV on standard output and the notes (equation, parameters, one-line summary) go to standard error, so a pipe gets only data.
- `--list` prints each function with its parameters and their starting values.
- A bad request (unknown function or parameter, fewer than 2 points, equal ends, a log range with a non-positive end, a function with no value on the range) is an error with a message and writes nothing.
- MCP: `list_simulate_functions {category?}` (pure like `list_fit_models`; `{functions:[{name, category, parameters:[{name, start}]}]}`) and `simulate_curve {function?, params?:{NAME:number}, from?, to?, points?, log?, x_of?, book?, sheet?, plot?, image?, values?}`. It is a dedicated tool, not an `analyze` subcommand: the sheet and the graph are one undo step (`AI: simulate_curve Gauss Book1`), `image` (under the roots, never replacing a file) implies `plot` and is all or nothing like the `fit` plot, and the reply has `book`, `sheet`, `x_column`, `y_column`, `function`, `points`, `graph`, `image`, `notes`, `message` and, up to 1000 points (or when `values` is true), the `x` and `y` arrays. Without a workbook in the session the tool makes one that holds only the new sheet. The option names are those of the command line with `_` for `-`.

### To measure on Windows (`tools/origin-ref`)

1. The window: category and function lists, the parameter grid, the X range controls, the output controls and the graph tick; the default function, X range, number of points and parameter values (and whether the initial values are those of the function file or computed).
2. Output: workbook and sheet names, column names, long names, comments and units of the X and Y columns, whether parameters are recorded, what "create a graph" draws.
3. The numbers: simulated values of Gauss, ExpDec1 and Boltzmann (and Voigt and a branch function such as Bigaussian) against `simcurve` for typed parameters, linear and logarithmic X, compared to 1e-12; store as a fixture in `crates/fit/tests/data/` and add a test.
4. Whether `simcurve` offers a logarithmic spacing and how it names it; behaviour for a function with no value at some X (missing value or error); whether parameter bounds are enforced; whether it applies to user-defined functions; whether the curve is live on a preview graph.

## 14. Surface fits on XYZ worksheet columns (T15 item 7, engine and library)

Scope: `z = f(x, y)` fitted to three worksheet columns. Matrices and 3D plotting are not part of this. The engine pieces are
`nlsf::FitFunction::from_formula_xy` and `x2_name`, `nlsf::fit_xyz` / `Fitter::new_xyz`, the table in `crates/fit/src/surface.rs`
(22 functions; `library::get` also finds them, `library::list` does not list them) and the FDF import of files with two
independent variables (`fdf::FdfFunction::to_fit_function`; three or more are still refused). A one-variable fit of a surface
function, and the reverse, is an error. Points with a missing x, y or z (or weight) are dropped, as for curves. The fitting
algorithm, statistics and report numbers are the ones of section 3 and 4 with no change.

### Measured in Origin 2016 (`tools/origin-ref/nlfit-surface-suite.lines.txt`, made by `nlfit-surface-gen.py`)

* Command: `nlbeginz iz:=(1,2,3) func:=<name> nltree:=tt`, then `nlfit`, `nlend output:=1` (the plain `nlbegin` refuses
  XYZ ranges). The tree holds the parameters (`tt.<name>`, read before `nlfit` they are the initial values) and `tt.niter`.
  The report is the sheet `FitSurface1` (layout as the curve report: parameter table in columns 7 to 11, statistics column 12,
  ANOVA from column 21 or later; `tools/origin-ref/nlfit-surface-make-fixture.py` reads it). It also makes `FitSurfaceCurve1`.
  The statistics are the curve ones: n, DOF, reduced chi-square, RSS, R-square, adjusted R-square, ANOVA, `Succeeded(100)`.
* Data: 11 x 11 grids (9 x 9 for the last case) of `z` plus a fixed sine noise term, made inside Origin; fits of Plane,
  Parabola2D, Poly2D, Gauss2D (also with 5 missing z), Lorentz2D, Voigt2D, Gaussian2D (rotated), Exponential2D, Rational2D and
  Plane with 3 missing x or z values. A missing cell is made with the assignment of `-1.23456789E-300`; the constant `NANUM`
  does nothing in a cell assignment.
* Origin's iterations on these were 3 to 11 (Plane 4, Parabola2D 3, Poly2D 3, Gauss2D 6, Lorentz2D 5, Voigt2D 6, Gaussian2D 7,
  Exponential2D 11, Rational2D 8).
* The 22 functions in Origin's FDF files (category Surface Fitting) and their formulas, parameter names and bounds are in
  `surface.rs`: Plane, Parabola2D, Poly2D, Polynomial2D, Gauss2D, Gaussian2D, Lorentz2D, Voigt2D, Voigt2DMod, Exponential2D,
  Extreme2D, LogNormal2D, Power2D, Rational2D, RationalTaylor, DoseResp, GaussCum, LogisticCum, ExtremeCum, Chebyshev2D, Cosine,
  Fourier2D (our names carry a `2D` suffix for DoseResp, GaussCum, LogisticCum, ExtremeCum and Cosine, which in Origin share the
  name of a one-variable function). Bounds on by default: widths `w1`, `w2` (and `C`, `E` of DoseResp) above 0, `mu` of Voigt in [0, 1].
  A parameter named `E` is a parameter here, not Euler's number (internally written `E__` in the formula).

### Starting values

Origin's rules (from the function files): Plane, Parabola2D, Poly2D start from polynomial fits of z against x and of z against y
(after sort and smoothing; Poly2D has `f = 0`); the peak surfaces take position, width, base and height from two slice curves
interpolated from the data, `w1`, `w2` are the slice widths times 0.85 (Gauss2D, Gaussian2D, Voigt2D) or divided by 2 (Lorentz2D),
`mu = 0`, `theta = 0`; Exponential2D uses the extremes of z and the x and y extent. The slices come from a native
scattered-data interpolation that is not reproduced: ours are the grid lines through the highest point. Measured Origin
starts against ours (Gauss2D: Origin `z0 0.683, A 2.673, xc 2.653, w1 1.637, yc 2.245, w2 1.320`; ours `0.577, 2.76, 2.5, 2.10,
2.0, 1.79`) agree to a few percent for the polynomial surfaces and are rougher for the peak surfaces. From our starts every measured
case reaches Origin's result (test `surface_fits_from_own_starting_values_reach_origin_results`).

### Test and tolerances (`crates/fit/tests/origin_surface.rs`, data `tests/data/origin_surface.json`)

From Origin's starting values our engine takes the same number of iterations in every case but Rational2D and agrees in
parameters, standard errors, t, dependency, statistics and ANOVA to 1e-7 (relative). Rational2D (dependencies near 1) stops at
a slightly different point of its flat valley: values agree to 1e-2, standard errors to 1e-1 relative. From our own starting
values the tolerance is 1e-4 (values) and 1e-3 (statistics).

### Unmeasured

Origin's fits of Polynomial2D, Voigt2DMod, Extreme2D, LogNormal2D, Power2D, RationalTaylor, DoseResp, GaussCum, LogisticCum,
ExtremeCum, Chebyshev2D, Cosine and Fourier2D (our formulas are copied from the files, no run); the starting values of those and of
Exponential2D beyond the case above; the scaling applied by Chebyshev2D and Cosine (the files say x and y are scaled to -1..1 and
0..pi, we apply none); weighted surface fits; fits with fixed parameters, user-set bounds or the simplex algorithm; whether
`nlbeginz` accepts a worksheet range with a column of errors; the dialog and the report sheet layout in the application
(part 2).

## 15. Surface fits (XYZ) in the application, command line and project

Section 14 holds the engine, the 22 functions and what was compared with Origin's `nlbeginz` runs. This section is what
the application does with three worksheet columns. Scope is as there: `z = f(x, y)` on XYZ columns; matrices and 3D
plotting are not part of it.

### Measured in Origin 2016 (raw output in `re-workspace/scratch/t15-surface/`, local only)

Run with `tools/origin-ref/run-labtalk-lines.ps1` on a new book with X, Y, Z columns (an 11 x 11 grid of 121 rows, the
Plane function, `nlbeginz iz:=(1,2,3)`, `nlfit`, `nlend output:=1`):

* The book then holds three sheets: the data sheet (still three columns: no fitted or residual column is added to it),
  `FitSurface1` (the report; layout and statistics in section 14) and `FitSurfaceCurve1`.
* `FitSurfaceCurve1` has 2500 rows and 8 columns. The first three have the long names `FitX`, `FitY`, `FitZ` and the
  designations X, Y, Z (`wks.col.type` 4, 1 and 6). They are the fitted surface on a regular 50 x 50 grid over the data
  range of x and y (for x from 0 to 5: 0, 5/49, 2 x 5/49, ...), x changing fastest and y once per 50 rows; `FitZ` is the
  function at those points. Columns 4 to 8 start with the data x and y, the function at the data points (twice) and a
  column of zeros in the first rows; what the last five columns are exactly was not worked out, and Sheaf does not
  write them.
* The report sheet of a surface fit has the title "Nonlinear Surface Fit", the three input ranges (`[Book1]Sheet1!A`, `B`,
  `C`, as the cell range text `[1*:121*]`), the equation as `z=z0+a*x+b*y;` and otherwise the layout of the curve report
  (section 3 and 4; section 14 lists its statistics).
* `nlfit -d` (open the dialog) after `nlbeginz` does not open a window: Origin's command returned false. The window is
  reached through the menu: `tools/origin-ref/origin-dialog.ps1 -MenuClicks "300,300;K:^a;K:%a;K:F;K:{DOWN} x 6;K:{ENTER}"`
  on a book whose third column is set to Z (Alt+A, F, six Down, Enter lands on Analysis > Fitting > Nonlinear Surface
  Fit...; pictures and control lists in `dlg2` and `dlg3`). What it shows: title "NLFit ()" and the NLFit layout of
  section 9 (Dialog Theme, tabs Settings, Code, Parameters, Bounds, the lower panel with Fit Curve, Residual, Formula,
  Sample Curve, Messages, Function File, Hints, and Fit, Done, Cancel). Function Selection: the Category box holds
  "Surface Fitting" and is grey, the Function box is empty, so the window says "Function is empty. Please specify it to
  continue." and the preview says "No Preview"; Iteration Algorithm (Levenberg Marquardt; the list has a second entry
  "Orthogonal Distance Regression (Pro)"), Description and File Name(.FDF). The Function list holds, in this order,
  Chebyshev2D, Cosine, DoseResp2D, Exponential2D, Extreme2D, ExtremeCum, Fourier2D, Gauss2D, Gaussian2D, GaussCum, LogisticCum,
  LogNormal2D, Lorentz2D, Parabola2D, Plane, Poly2D, Polynomial2D, Power2D, Rational2D, RationalTaylor, Voigt2D, Voigt2DMod
  (so Origin's names have no `2D` suffix for Cosine, ExtremeCum, GaussCum and LogisticCum), then `<New...>` and `<Add...>`. The pages
  of the Settings list are Function Selection, Data Selection, Fitted Curves, Find Z, Advanced, Output (the curve window's
  "Find X/Y" is "Find Z"). Data Selection shows "Multi-Data Fit Mode" (Independent Fit - Consolidated Report, grey) and an
  "Input Data" node whose range text is `[Book1]Sheet1!(A,B,C)`: one X, Y, Z triple. Find Z holds one node "Find Z from XY"
  with a check box. The Fitted Curves, Advanced and Output pages were not read (the clicks did not change the page).

### What Sheaf does

* Request: `FitRequest.surface = Some(SurfaceColumns { x, y })` and `FitRequest.y` is then the Z column. `x` and `y` left
  `None` are the sheet's own: the X column to the left of the Z column (as for curve fits) and the nearest column to its left
  designated Y that is not a fit result (`surface_columns`). Model: a surface function by name (`FitModel::BuiltIn`), a
  formula in `x` and `y` (`FitModel::Formula`, parameters named by the caller) or an `.FDF` file with two independent
  variables. A curve model, a one-variable function, equal X, Y and Z columns, or bands are
  refused, and nothing in the project changes. Rows (all, a range of rows, an X range), the start values, fixed values and
  bounds, weighting (error column or direct weights), scale errors, algorithm, iteration limit, tolerance and the
  confidence level are those of a curve fit; a missing x, y, z or weight leaves the row out. Masked and filtered cells count
  as missing.
* Results (`Project::fit` returns a `FitOutcome` with `surface = true`): sheet `FitSurface1` (the first free number of
  `FitSurface1` and `FitSurface1Stats`) with the parameter table and `FitSurface1Stats` with model, equation, data, rows,
  statistics, ANOVA and derived quantities (FWHM and volume of the peak surfaces), laid out as `FitNL1`; two columns after the Z column
  (fitted Z and residual, marked as fit results, so they are not offered as Y columns); and `FitSurfaceCurve1`, the 50 x 50
  grid above with the columns `FitX`, `FitY`, `FitZ` (X, Y, Z designations). Sheaf adds the fitted and residual columns,
  which Origin's `nlend output:=1` run did not. There is no graph and `FitOutcome::curve_x` and `curve_y` are the grid's X and
  Z columns.
* Saving: the results are ordinary sheets and columns, so the `.sheaf` file holds them (test
  `a_plane_is_fitted_and_the_results_are_placed`). `FitRequest` itself is not stored in a project, and there is no
  recalculation link for any fit result in Sheaf (feature-parity: "analysis and fit results are not linked"), so
  the surface results are static as the curve results are. The `.opj` writer does not write fit result sheets of either
  kind (feature-parity), so the `.opj.sheaf` companion has nothing of its own for them.
* Window: Analysis > Fitting > Nonlinear Surface Fit... (an item of Origin's own menu, measured earlier in
  `gui-look.md`, grey until now) opens the NLFit window with the category "Surface Fitting" (grey, as in Origin) and the function Gauss2D (Origin
  starts with an empty function; Sheaf starts on Gauss2D so that the window can fit at once). With a surface function chosen, Data Selection shows three boxes X, Y and Z
  (the Z box is the column that is fitted; choosing it takes X and Y from the sheet's designations again), the Fitted
  Curves page says where the surface goes instead of the curve options, there are no bands and no other-dataset check
  boxes, and the lower panel's preview says there is nothing to preview. 1 Iteration, Fit until converged, Revert, Fit,
  Done and Cancel work as for curves (section 9) and Fit writes everything above in one undo step ("Fit"). The category and
  function boxes list the 22 functions with their formulas written `z = ...`. The Settings list calls the fourth page "Find Z" as Origin does. Sheaf's single Input Data node is three boxes X, Y and Z;
  the rest is the NLFit layout of section 9 with these differences. Sheaf's function names follow section 14 (`Cosine2D`,
  `GaussCum2D`, `LogisticCum2D`, `ExtremeCum2D`), not Origin's list order or names.
* Command line: `sheaf fit FILE --z COLUMN [--x COLUMN] [--y COLUMN] --model Gauss2D` (also `--formula` in x and y with
  `--params`, `--fdf`, `--init`, `--fix`, `--bounds`, `--weight`, `--rows`, `--x-range`, `--confidence`, `--max-iterations`,
  `--tolerance`, `--save`, `--export`); `--y` is then the Y column, not the fitted one. `--global`, `--regress`, `--compare`,
  `--rank`, `--xerr`, `--bands` and `--plot` are refused with a surface fit; `--list` shows the category "Surface Fitting".
  Tests: `crates/core/tests/surface_fit.rs`, `crates/cli/tests/fit.rs` (`a_surface_fit_takes_z_and_a_surface_function`) and
  the window test `crates/app/src/app/ui_tests/surface_fit.rs`.

### Unmeasured

The Fitted Curves, Advanced and Output pages of Origin's window with a surface function chosen, its Parameters and Bounds tabs, and
what a chosen function changes in the Data Selection page (the function was empty in the run); the "Find Z from XY" tool; what columns 4 to 8 of `FitSurfaceCurve1` hold; whether the grid
size of 50 is a default that the window changes; weighted surface fits and surface fits with fixed or bounded
parameters in Origin; Origin's response to a Z column that is not designated Z; the report sheet's layout in the
application (Sheaf writes its own parameter and statistics sheets, as for curves); the three-variable functions,
Simulate Surface, and matrices.
