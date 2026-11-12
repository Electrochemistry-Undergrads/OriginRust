# FDF fit-function definition file format (Origin 2016)

Summary. Origin 2016 ships its built-in fitting functions as 237 plain-text `.FDF` files in `FitFunc` (the folder has 239 entries; the other two are `NLSF.CNF`, an old fitter configuration, and `ERRFUNC.F`, a stub for an external-library function). An FDF file is INI-like: bracketed section headers followed by `Key=Value` lines, but several sections hold free-form code or formula text instead of keys. Every file is ASCII, CRLF-terminated, no byte-order mark. Section and key names are case-insensitive in practice (59 files use UPPER CASE headers, 178 use Title Case, a few mix them) and a file may write `Key=Value` or `Key = Value`. A programmer can read name, category (from `NLSF.ini`, not from the FDF), variables, parameter names, bounds, initial values and, for 185 of the 237 functions, a plain single-expression formula. The other 52 need extra work: 13 contain if/else logic, 19 use local temporaries, 16 call native routines, use a ternary operator, or are written in broken or non-standard notation, and 4 are external-DLL stubs with no formula at all (see section 9). Observed from: `C:\Program Files\OriginLab\Origin2016\FitFunc\*.FDF` and `C:\Program Files\OriginLab\Origin2016\NLSF.ini` (the owner's copy under `OriginLab\2016\User Files\NLSF.ini` differs only by one space on one line). This document describes the format in the author's words; no file text is reproduced except identifiers, numbers and formulas.

Priority: P0 for the importer fields (names, variables, parameters, bounds, expression formulas, peak flags); P1 for derived parameters, constraints and category lists; P2 for the code sections (initialisation scripts, headers).

## 1. Lexical rules

| Rule | Detail |
|---|---|
| Encoding | ASCII (all 237 files), CRLF line ends. Decode as UTF-8 or Latin-1 safely. |
| Section header | A line consisting of `[` name `]`, optional trailing spaces. Names are matched case-insensitively. Order of sections varies between files. |
| Key/value line | `Key=Value`. Spaces around `=` are optional (`Names = a,b` and `Names=a,b` both occur). Keys are case-insensitive (`Lower bounds`, `Lower Bounds`). Trailing spaces in keys occur (`Function Source `). |
| Inline comment (key sections) | In 111 files a value can be followed by whitespace, `;`, then a comment (`Function Type=Built-in <tab>; source type...`). Strip `\s;.*$` from values of key sections. Do NOT do this inside code/formula sections, where `;` ends a statement. |
| Code comments | In code sections: `//` to end of line and `/* ... */` blocks. A line starting with `#` is a preprocessor line (header sections) or, in one formula, a stray note. |
| List values | Comma-separated. Elements may be padded with spaces. A bounds element may itself contain a parenthesised flag group with a comma (`0.0(X,ON)`), so split on commas that are outside parentheses. A trailing comma occurs once (Weibull). |
| Empty value | `Key=` is legal and means "none" (for example `Initial Values=`, `Number Of Significant Digits=`, `x=`). |
| Numbers | Plain decimals: `0`, `0.0`, `3.1416`, `425`. No exponents were seen in bounds or initial values. |
| Not-set marker | `--` in a bounds or initial-value slot means "no value". |

## 2. Sections

Counts are the number of the 237 files that contain the section (case-insensitive name). "Free text" means the body is code or formulas, not `Key=Value`.

| Section | Files | Kind | Meaning | Importer need |
|---|---:|---|---|---|
| General Information | 237 | keys | Identity and kind of the function. | P0 |
| Fitting Parameters | 237 | keys | Parameter names, labels, bounds, optional initial values. | P0 |
| Independent Variables | 237 | keys | One key per independent variable (value is always empty). | P0 |
| Dependent Variables | 237 | keys | One key per dependent variable (value is always empty). | P0 |
| Formula | 237 | free text | The model. Usually a single assignment `y = expression`; sometimes code. | P0 |
| Controls | 237 | keys | Fitter behaviour switches, peak-parameter flags, multi-peak duplication. | P0 (peak flags), P2 (rest) |
| Constraints | 231 | free text | Validity conditions on parameters, e.g. `t1>0;t2>0;` or `dx != 0`. Often just a placeholder comment. | P1 |
| Constants | 227 | free text | Intended for fixed constants; empty in every file. | ignore |
| Parameters Initialization | 211 | free text (Origin C) | Code that computes starting parameter values from the data. | P2 (see section 6) |
| Initializations | 237 | free text | Code run before fitting; empty or comment only. | ignore |
| After Fitting | 237 | free text | Code run after fitting; empty or comment only. | ignore |
| On Param Change / On Parameter Change | 44 | free text | Code run when a parameter is edited; comment placeholder only. | ignore |
| Compile Function | 62 | keys | Flags `Compile`, `Compile Parameters Initialization`, `On Param Change Scripts Enabled` (also written without spaces). | ignore |
| Origin C Function Header | 42 | free text | Include/pragma lines for the compiled formula. | ignore |
| Origin C Parameter Initialization Header | 45 | free text | Include/pragma/define lines for the initialisation code. | ignore |
| Derived Parameters / DerivedParameters | 45 | free text | Lines `name = expression;` giving quantities computed from the fitted parameters (FWHM, half-life, ...). | P1 |
| Derived Parameter Settings | 13 | keys | `Names=` and `Meanings=` lists naming the derived quantities. | P1 |
| Moments | 16 | free text | Lines `mz0`, `mz1`, `mc2`, `mc3`, `mc4` = zeroth moment (area), centre, and central moments 2-4 of a peak; `N.A.` when not available. | P2 |
| References | 4 | free text | Literature notes. | ignore |
| LabTalk Functions Definition and Initializations | 1 | free text | Empty in the one file that has it. | ignore |

## 3. General Information keys

| Key | Files | Values seen | Meaning |
|---|---:|---|---|
| Function Name | 237 | identifier | Name shown in selection lists (e.g. `Gauss`, `ExpDec1`). Unique. Differs in case and spelling from the file name (`BOLTZMAN.FDF` holds `Boltzmann`). |
| Brief Description | 237 | one line of text | Human description. Not copied into this project's JSON. |
| Function Source | 237 | `fgroup.<Name>` (227), `OPFMFuncs.<Name>` (6), `dscfuncs.<n>` (3), `DSCFIT.2` (1) | Link to the compiled routine that implements the model. `fgroup.*` = built-in function group. Not needed to evaluate an expression formula. |
| Function Type | 237 | `Built-in` (233), `External DLL` (4) | `External DLL` files (DCPDisso, dCpM2ST, M2State, MN2State) have no formula in the file. |
| Function Form | 237 | `Expression` (182), `Equations` (55) | Both forms carry the same statement syntax; no structural difference was found. `Equations` is used by the newer files. |
| Number Of Parameters | 237 | integer 1-12 | Always equals the number of names. |
| Number Of Independent Variables | 237 | 1 (207), 2 (29), 3 (1) | |
| Number Of Dependent Variables | 237 | 1 (234), 2 (3) | |
| Analytical Derivatives for User-Defined | 33 | `Off` / `0` | Whether analytical derivatives are supplied; always off. |
| Function Model | 8 | `Explicit` (5), `Implicit` (3) | Implicit means the formula is `f(x,y,...) = 0` style (Circle, Ellipse, PlaneMod). |
| Path, FunctionPrev | 10, 13 | empty / name | Bookkeeping left by the function builder. Ignore. |

## 4. Fitting Parameters keys

| Key | Files | Syntax | Meaning |
|---|---:|---|---|
| Naming Method | 237 | always `User-Defined` | Parameter names are those in `Names`. |
| Names | 237 | `n1,n2,...` | Parameter identifiers in model order. This order defines parameter indexes used elsewhere (1-based). Names are case-sensitive in formulas (`Z0` vs `z0` mismatch exists in GaussLog). Underscores allowed (`x0_1`). |
| Meanings | 237 | comma list of short labels | One short label per parameter (`offset`, `center`, `width`, `area`). Placeholder `xxxx` in the four external-DLL files. Labels can contain spaces. |
| Lower Bounds | 237 | one element per parameter | See bound syntax below. |
| Upper Bounds | 237 | one element per parameter | Same syntax. Only 9 functions have any upper bound. |
| Initial Values | 58 | one element per parameter | `number(V)` or `number(F)` or `--(V)`; only 7 files give a number (V = starts as a free variable, F = starts fixed; the only F cases are in Goldman). `--(V)` means "no stored value, use the initialisation code". Most files omit the key. |
| Number Of Significant Digits | 106 | list of integers or empty | Display precision; ignore. |
| Unit, Format, CustomDisplay | 6, 5, 5 | comma lists | Display options in a few newer files; ignore. |

### Bound element syntax

An element is `VALUE(FLAG1,FLAG2)` where every part is optional:

* `VALUE` is a number or `--` (none). Example `0.0`.
* `FLAG1` is `X` or `I` (also written `(I, Off)` with a space). [inferred: `X` = bound is exclusive, i.e. strict inequality; `I` = inclusive.]
* `FLAG2` is `ON` or `OFF` in any case. [inferred: whether the bound is enforced by default when the function is selected.]

Observed shapes: `--` (986 times), `0.0(X,ON)`, `0.0(X,OFF)`, `0(I,Off)`, `--(I,Off)`, `--(X,Off)`, and bare flags `(X,Off)` / `(I,...)` with no value. Lower bound values seen: 0 (252 times), 1 (2), 0.5 (2). Upper bound values: 1, 170, 3.1416, 6.2832. 107 functions have at least one lower bound value; for 91 of them at least one is switched on. A bound value with state OFF is a stored suggestion: an importer should keep the number but not enforce it unless the user turns it on. Typical use: peak widths `w` have `0.0(X,ON)` so width must stay positive; `ExpDec1` time constants carry `0.0(X,OFF)` so the constraint is available but not active.

## 5. Variables, formula and constraints

### Variables

`[Independent Variables]` has lines like `x=`; `[Dependent Variables]` has `y=` or `z=` (the value is always empty). The key names are the variable names. Seen: independent `x` (235 files), `y` (27; surface functions), `z` (PlaneMod), `x_1,x_2` (Cole), `x1,x2` (HillBurk); dependent `y` (207), `z` (24), `f` (3; implicit functions), `y1,y2` (GaussianLorentz, LineExp), `v1,v2` (HillBurk). Counts of (independent, dependent): (1,1) 206 functions, (2,1) 27, (1,2) 2, (2,2) 1, (3,1) 1.

### Formula section

The body is Origin C-flavoured statements, not a mathematical markup language. Forms seen:

1. Single assignment, `y = <expr>` (optionally ending in `;`, possibly spanning two or three lines): 185 functions, `formula_kind = "expression"`. The left side is the dependent variable name.
2. Assignment with leading comment lines, for example a note giving the maths in different notation, then the Origin C line. Comments start with `//`.
3. Sequence of statements: `double t = ...;` temporaries followed by `y = ...;` (19 functions).
4. `if` / `else` / `else if` blocks with several assignments to `y` (13 functions) and two functions written with the C ternary operator `cond ? a : b` (ExpGrow3Dec2, ExpGrowDec).
5. A call to a native routine (`nlf_voigt`, `nlf_pearsoniv`, `bessel_i1`, `gamcdf`, `normcdf`, `wblcdf`, `log_gamma`, `SqrWav`).
6. For the four External DLL functions the section holds only a title line.

Expression syntax that must be supported by an expression evaluator:

| Element | Details |
|---|---|
| Operators | `+ - * /` and `^` for power. `^` binds tighter than unary minus in all observed uses, e.g. `-ln(x/xc)^2` means `-(ln(x/xc)^2)` [inferred from intent]. Comparison `< <= == > >=` appears in conditions. |
| Constants | `PI` / `pi` / `Pi` (case varies), `NANUM` (missing value), literal numbers possibly with a trailing dot (`1.`, `3.`). |
| Functions | `exp ln log log10 sqrt abs fabs sin cos tan asin acos atan sinh cosh tanh erf floor gamma pow(a,b)`. `log` is used with the natural-log meaning (LogNormal2D) [inferred]. Function names occur in mixed case in a few files (Sqrt, Exp in non-expression text). |
| Origin-specific calls | `nlf_voigt(x,y0,xc,A,wG,wL)`, `nlf_pearsoniv(x,y0,A,m,v,alpha,lam)`, `bessel_i1`, `gamcdf(x,a,b)` (gamma CDF, shape a, scale b), `normcdf(z)` (standard normal CDF), `wblcdf(x,a,b)`, `log_gamma`, `SqrWav(a,b,x0,T,x)`. |
| Identifiers | Parameter names, the independent variable(s), `double` temporaries, `PI`. |

A formula may also be wrapped by Origin-style comment text that restates the maths; an importer must ignore comment lines first, then treat the rest as code.

### Formula kinds for the importer

Rule used to build `data/fit-functions.json`: after removing comments and preprocessor lines, if the text is exactly one assignment whose left side is a dependent variable and whose right side parses with the grammar above (identifiers limited to parameters, variables, `pi`), the function is `"expression"` and the right-hand side is stored (powers written `^`, `pow(a,b)` rewritten to `a^b`, `PI` to `pi`, `log` to `ln`). Otherwise it is `"procedural"` and `formula` is null; where possible `formula_math` holds a hand-written mathematical statement.

### Constraints section

Lines separated by `;` or newlines, each a relation: `t1>0`, `dx != 0`, `0<r<1`, `A1 > A2 > 0`, `a > b > 0`, `0<=theta<2*pi`, `#d != 1` (a leading `#` marks a fixed parameter in Origin's notation [inferred]). 19 functions have real constraints; the rest contain only a placeholder comment. They restrict admissible parameter values; no importer is required to enforce them, but a fitter should avoid parameter values that violate them (for example `dx = 0` in Boltzmann).

## 6. Parameters Initialization (P2)

211 files have a block of Origin C that sets the starting values from the data. It is not an expression language; it is code that calls Origin's own helper routines on the data curve. Observed helper names: `sort`, `smooth`, `fitpoly`, `peak_pos`, `fwhm`, `area`, `xatymax`, `xatymin`, `yatxmax`, `yatxmin`, `xaty50`, `get_exponent_cuv`, `get_exponent`, `ocmath_xatasymt`, `ocmath_yatasymt`, `fft_complex`, `Curve_yfromX`, `Curve_xfromY`, plus dataset methods `GetMinMax`, `AttachX`, `GetSize`, `Find`. A re-implementation must supply its own initial-guess heuristics (one small routine per function family); the file cannot be executed. Typical recipes are summarised in the catalog under "The 30 functions to implement first". Only 7 files carry a numeric `Initial Values` list (ExpGrow3Dec2, ExpGrowDec, SawtoothWave, SquareWave, SquareWaveMod, Step, Goldman), which are the functions for which no sensible data-driven guess exists.

## 7. Controls keys

| Key | Files | Values | Meaning |
|---|---:|---|---|
| General Linear Constraints | 237 | `Off`/`0` (one `On`/`1`) | Whether general linear constraints are on. |
| Initialization Scripts | 237 | `Off`/`0` | Run the Initializations section. |
| Scripts After Fitting | 236 | `Off`/`0` | Run the After Fitting section. |
| Number Of Duplicates | 96 | `0`, `N/A` | Default number of extra repeated peaks. |
| Duplicate Offset | 100 | `1`,`2`,`3`,`N/A` | [inferred] 1-based index of the first parameter of the block that is repeated per additional peak; parameters before it (the baseline) are shared. |
| Duplicate Unit | 100 | 2-8, `N/A` | [inferred] number of parameters in the repeated block. |
| Peak Center | 39 | 1-based parameter index | Which parameter is the peak centre. Presence of this key is Origin's own "this is a peak function" flag (39 files). |
| Peak Width | 39 | index | Parameter that is the width. |
| Peak Width 2 | 9 | index | Second width, for two-width shapes (Voigt: wL; PsdVoigt2: wL; Bigaussian: w2). |
| Peak Amplitude | 34 | index | Parameter that is the amplitude/area/height. |
| Peak Area | 5 | index | Parameter that is the peak area, used instead of Peak Amplitude by PearsonVII, PearsonIV, GaussMod, ECS and GCAS. |
| Peak Center 2 | 5 | index | Second centre: the y-direction centre of the 2D surface peaks (Gauss2D, Gaussian2D, Lorentz2D, Voigt2D, Voigt2DMod), for which Peak Width 2 is the y width. |
| Generate Curves After Fitting, Generate Peaks After Fitting, Generate Peaks During Fitting, Generate Peaks with Baseline, Paste Parameters to Plot After Fitting, Paste Parameters to Notes Window After Fitting, Paste Parameters to Script Window After Fitting, Generate Residuals After Fitting (also `Generate Residue After Fitting`), Keep Parameters | 5-64 | `Yes`/`No` or `1`/`0` | Default output switches of the fit dialog. External-DLL functions default curves and peaks to No and Keep Parameters to Yes. |
| Curve Point Spacing | 64 | `Uniform on X-Axis Scale` (60), `Same X as Fitting Data` (4) | How the fitted curve is sampled. |
| Enable Parameters Initialization, Compile On Param Change Script, Treat All Numbers As Double | 55 / 55 / 9 | `0`/`1`/`Yes`/`No`/`N/A` | Code-generation switches; ignore. |

Peak flags in the JSON: `is_peak` is true when the file carries `Peak Center` OR the function is listed in the `[Peak Functions]` section of `NLSF.ini` (40 functions). `peak_flag_in_file` and `in_peak_functions_category` give the two sources separately. 17 flagged functions are not in the ini peak list: AsymmetricGaussian, BWF, ConsGaussian, DoniachSunjic, FraserSuzuki, Gauss2D, Gaussian2D, Gaussian_LorenCross, HVL, Lorentz2D, Pulse, Sine, SineDamp, SineSqr, SchulzFlory, Voigt2D, Voigt2DMod. `peak` in the JSON maps `peak_center`, `peak_width`, `peak_amplitude`, `peak_area`, `peak_center_2`, `peak_width_2` to parameter NAMES.

## 8. Derived parameters, moments

`[Derived Parameters]` (36 functions have non-empty content) contains `name = expression;` lines, usually with a trailing `// comment`. Expressions use the fitted parameter names and, rarely, `x`; they are plain maths (`FWHM = sqrt(2*ln(2)) * w`). Example (Gauss): sigma = w/2, FWHM = sqrt(2 ln 2)*w, Height = A/(w*sqrt(pi/2)). `[Derived Parameter Settings]` (13 files) gives `Names` and `Meanings` lists for the derived quantities, in the same style as the fitted parameters. The JSON stores `derived: [{name, formula}]` with the expression rewritten like formulas (null if it did not parse).

`[Moments]` gives five formulas for a peak's statistical moments in terms of parameters; ignore for now.

## 9. Which files need more than the expression

* 185 functions: single-expression formulas; evaluate directly.
* 13 with branching (formula_math provided by hand in the JSON): AsymmetricGaussian, Bigaussian, ExponentialCDF, FraserSuzuki, Power2, Pulse, Pulse1, PWL2, PWL3, SRichards1, SquareWaveMod, Step, WeibullCDF.
* 19 with local temporaries: BiDoseResp, CCE, Carreau, CompInhib, Cross, Extreme, GaussMod, Goldman, GaussianLorentz, HillBurk, Langevin, Logistpk, LangevinMod, LineExp, MixedModelInhib, NoncompInhib, UncompInhib, Weibull, Weibull3. For all of them except the multi-output ones (GaussianLorentz, HillBurk, LineExp) and BiDoseResp the JSON gives `formula_inlined`, which is the final expression with temporaries substituted; the multi-output ones define two dependent variables `y1`,`y2` (or `v1`,`v2`) sharing parameters, meaning a global fit across two data sets. Our importer refuses any file with more than one dependent variable ("several dependent variables (a global fit) are not supported"), in the same way as files with several independent variables. It never imports only the first equation, because the parameters used only by the other equation would be listed but never fitted.
* Parameter names are checked when a function is built, for FDF files as well as typed formulas: a name may not repeat (names are case sensitive), may not equal the independent variable, and may not be `pi` or `e` in any case, because the expression language reads those as constants and the parameter would never take part in the fit.
* Error text for a formula that fails to parse after translation does not carry a source position: the position would point into the translated text, not into the file.
* Native-routine functions (need a library function in the new app; WeibullCDF, listed under branching above, also calls one): Giddings, GammaCDF, LognormalCDF, PearsonIV, Poisson, SquareWave, Voigt.
* Ternary form (needs `if`): ExpGrow3Dec2, ExpGrowDec.
* External DLL, no formula in the file at all (thermodynamic DSC models; skip): DCPDisso, dCpM2ST, M2State, MN2State.
* Source text defective or garbled; do not trust the formula string without the hand-written `formula_math`: BatesWatts, Cole, Freundlich3PMod, FraserSuzuki, GaussLog, LogisticPeak, PolyCtr5, Pulse1. Details are in the `source_defect` field and the catalog.

## 10. NLSF.ini (category lists)

`NLSF.ini` (a copy under `OriginLab\2016\User Files` differs by one space) is a classic INI file. Comment lines start with `;`. It has three kinds of sections:

* `[NLSF Control]` global fitter defaults: default function and group, number of fit points for the curve (`FITNPTS`/`initNfitPts` 60), maximum iterations (`ITERATION=100`), tolerance (`tolerance=1E-9`), derivative step (`derivStepSize=0.005`, min/max `5E-30`/`5E30`), and the same output switches as Controls. [observed; the units of `derivStepSize` are a relative step, inferred].
* `[Category]` the ordered list of categories shown in the function dialog. Lines of the form `Name=fitfunc`, separators written `----------By Form----------=separator`. Commented entries (leading `;`) are hidden. Order: Origin Basic Functions; By Form: Implicit, Convolution, Exponential, Growth/Sigmoidal, Hyperbola, Logarithm, Peak Functions, Piecewise, Polynomial, Power, Rational, Waveform, Surface Fitting, PFW, Baseline; By Field: Chromatography, Electrophysiology, Enzyme Kinetics, Pharmacology, Rheology, Spectroscopy, Statistics, Quick Fit, Multiple Variables; User Defined. Hidden (commented): Asymptotic, Engineering, Pharmacology/Chemistry, Others, NIST.
* One section per category: lines `DisplayName=fdf-file-stem` (case-insensitive stem without extension), plus `Default Function=<display name>` giving the category's default. A function can appear in several categories; `;`-prefixed lines are disabled. Note the display name and the file's `Function Name` can differ in case/spelling (`Boltzmann=Boltzman`).

The first group, `[Origin Basic Functions]`, lists 21 functions (Allometric1, Beta, Boltzmann, dHyperbl, ExpAssoc, ExpDec1-3, ExpGrow1-2, Gauss, GaussAmp, Hyperbl, Logistic, LogNormal, Lorentz, Poisson, Pulse, Rational0, Sine, Voigt) with Gauss as the default; `[Peak Functions]` lists 23 with default Gauss; `[Quick Fit]` lists 11 and `[Baseline]` 15 (for baseline modelling). Surface functions are under `[Surface Fitting]` (22).

## 11. The two non-FDF files

* `NLSF.CNF`: an older, simpler fitter control file with `[NLSF Control]` (defaults: Gauss, 100 initial points, tolerance 0.05, 10 iterations) and an older category list. Superseded by `NLSF.ini`; ignore.
* `ERRFUNC.F`: describes a library-implemented function (`Library Function=errfunc.2`) with a 17-parameter list for a sedimentation-velocity expression and an initialisation script. A different key set from FDF (`Parameters=`, `Description = {..}`, `Initialization script = {..}`). Out of scope.

## 12. Importer recipe

1. Read the file as text; split into sections by `[...]`; lowercase section names.
2. From General Information read `Function Name`, `Function Form`, `Function Type`, variable counts (strip inline `;` comments).
3. From Independent/Dependent Variables take the key names, in order.
4. From Fitting Parameters take `Names` and split bounds and initial values with a parenthesis-aware comma split; interpret each element as `[number|--][(flag,state)]`.
5. Take the Formula body, drop `//` and `/* */` comments and `#` lines, collapse whitespace; if it is a single `dep = rhs` statement, parse `rhs`, else mark procedural.
6. From Controls read `Peak Center`, `Peak Width`, `Peak Width 2`, `Peak Amplitude`, `Peak Area`, `Duplicate Offset`, `Duplicate Unit` and convert 1-based indexes to parameter names.
7. Optional: Derived Parameters (`name = expr;`) and Constraints (split on `;`).
8. Read `NLSF.ini` to assign categories (join on file stem, case-insensitive) and the default function per category.

## 13. Known defects in the source files (observed)

* BatesWatts: formula text writes `x2` where `x^2` is evidently meant.
* Cole: variables declared `x_1`,`x_2`, formula uses `x1`,`x2`.
* Freundlich3PMod: formula uses `c`, the parameter is named `k`.
* Pulse1: formula uses `xc`, the first parameter is named `x0`.
* PolyCtr5: formula text contains an ellipsis.
* GaussLog, LogisticPeak: Mathematica-style bracket notation and juxtaposed products, not valid code.
* FraserSuzuki: the code mixes up the amplitude and the asymmetry parameter (both called `A`) and branches on `x<0` rather than `x<xc`.
* MWChang: `L1` is assigned after it is used.
* Pulse: below `x0` the value is exactly 0 (not `y0`), a discontinuity when `y0` is nonzero.

## 14. Unknowns to check against a running Origin

* Exact meaning of the `X`/`I` and `ON`/`OFF` flags in bounds (inferred above).
* Exact meaning of `Duplicate Offset`/`Duplicate Unit` (inferred: shared-prefix length index and block length).
* Operator precedence of unary minus versus `^` in Origin's expression parser (assumed `^` first).
* Whether `log` means natural or base-10 logarithm in Origin expression code (the evidence suggests natural).
* Whether the hidden categories (Asymptotic etc.) are reachable in the dialog.
* How `Peak Area` is used when both amplitude and area flags exist.

A user-written fit function is refused when a parameter name is repeated, equals the x name, is `pi` or `e` in any case, or does not appear in the formula; each of these would leave a parameter that is never fitted but still counts in the degrees of freedom.
