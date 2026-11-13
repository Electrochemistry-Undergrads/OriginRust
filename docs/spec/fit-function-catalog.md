# Fit function catalog (Origin built-in library)

Summary. This catalog covers 237 fit-function definitions, grouped by primary category. Formulas use plain mathematics; `x` is the independent variable, with `y` also used by surface functions. The machine-readable catalog is `data/fit-functions.json`. Priority: P0 = everyday fitting needs (peaks, exponentials, lines and sigmoids), P1 = common, P2 = later. Entries marked [inferred] still need verification.

## Counts

Each function has one primary category in the JSON (first visible category in `NLSF.ini` order, ignoring the aggregate lists Origin Basic Functions, Quick Fit, PFW and Baseline). A function can be listed in several categories in `NLSF.ini`; the membership count is the second column.

| Category | Primary (JSON `category`) | Listed in (membership) |
|---|---:|---:|
| Peak Functions | 23 | 23 |
| Growth/Sigmoidal | 19 | 19 |
| Exponential | 42 | 42 |
| Polynomial | 8 | 8 |
| Power | 18 | 19 |
| Rational | 15 | 15 |
| Logarithm | 5 | 5 |
| Hyperbola | 5 | 5 |
| Waveform | 7 | 7 |
| Piecewise | 2 | 2 |
| Implicit | 3 | 3 |
| Surface Fitting | 22 | 22 |
| Statistics | 9 | 19 |
| Enzyme Kinetics | 5 | 7 |
| Pharmacology | 5 | 8 |
| Rheology | 6 | 6 |
| Electrophysiology | 1 | 9 |
| Multiple Variables | 3 | 3 |
| Asymptotic | 6 | 7 |
| Others | 2 | 3 |
| Engineering | 1 | 4 |
| PFW | 7 | 28 |
| Uncategorized | 23 | 0 |
| Origin Basic Functions | 0 | 21 |
| Quick Fit | 0 | 11 |
| Baseline | 0 | 15 |
| Spectroscopy | 0 | 7 |
| Chromatography | 0 | 6 |
| Convolution | 0 | 2 |
| Pharmacology/Chemistry | 0 | 7 |

Categories `Uncategorized` are functions whose file is not referenced by `NLSF.ini` at all (legacy/hidden definitions). Categories `Asymptotic`, `Others`, `Engineering`, `Pharmacology/Chemistry` and `Spectroscopy`/`Chromatography` exist as sections in `NLSF.ini`; of these, Asymptotic, Others, Engineering and Pharmacology/Chemistry are commented out of the visible `[Category]` list (hidden from the dialog) [observed in NLSF.ini].

## The 30 functions to implement first

All formulas below were taken from the FDF files (observed) and then checked algebraically. "Width" conventions are the usual trap, so they are spelled out. `ln` is the natural logarithm, `pi` = 3.14159..., `erf` is the error function. P0 = needed for the owner's daily workflow, P1 = common.

| # | Name | Pri | Formula | Parameter conventions (what the numbers mean) |
|---|---|---|---|---|
| 1 | Line | P0 | `y = A + B*x` | A = intercept, B = slope. Derived: x-intercept = -A/B. |
| 2 | Poly | P0 | `y = a0 + a1*x + ... + a9*x^9` | 10 coefficients, ascending powers. Degree is chosen by fixing unwanted high coefficients at 0. |
| 3 | Parabola | P0 | `y = A + B*x + C*x^2` | Ascending powers. |
| 4 | Allometric1 | P0 | `y = a * x^b` | a = prefactor, b = exponent. |
| 5 | Power | P1 | `y = x^A` | One parameter, no prefactor. |
| 6 | ExpDec1 | P0 | `y = y0 + A1*exp(-x/t1)` | t1 is a time constant (1/e), NOT a rate. Derived: k = 1/t1, half-life = t1*ln 2. |
| 7 | ExpDec2 | P0 | `y = y0 + A1*exp(-x/t1) + A2*exp(-x/t2)` | Same conventions; derived k1,k2, half-lives. |
| 8 | ExpDec3 | P0 | `y = y0 + A1*exp(-x/t1) + A2*exp(-x/t2) + A3*exp(-x/t3)` | Same. |
| 9 | ExpGro1 | P0 | `y = y0 + A1*exp(x/t1)` | t1 = growth time constant (positive for growth). ExpGrow1 is the same with `x-x0` in the exponent. |
| 10 | ExpAssoc | P0 | `y = y0 + A1*(1-exp(-x/t1)) + A2*(1-exp(-x/t2))` | Rise to a plateau; A1, A2 are the (signed) amplitudes of the two phases. Bounds t1,t2 > 0 on. |
| 11 | Gauss | P0 | `y = y0 + A/(w*sqrt(pi/2)) * exp(-2*((x-xc)/w)^2)` | **A = area** under the peak above y0. **w = 2*sigma** (not the FWHM). sigma = w/2; FWHM = sqrt(2 ln 2)*w = 1.17741*w; height = A/(w*sqrt(pi/2)). Bound w > 0 (on). This is Origin's default peak function. |
| 12 | GaussAmp | P0 | `y = y0 + A*exp(-0.5*((x-xc)/w)^2)` | **A = height** above y0. **w = sigma**. FWHM = 2*sqrt(2 ln 2)*w = 2.35482*w; area = A*w*sqrt(2*pi). |
| 13 | Gaussian | P1 | `y = y0 + A/(w*sqrt(pi/(4 ln 2))) * exp(-4 ln 2*(x-xc)^2/w^2)` | **A = area**. **w = FWHM**. Height = A*sqrt(4 ln 2/pi)/w = 0.939437*A/w. Parameter ORDER is y0, xc, A, w (differs from Gauss). |
| 14 | Lorentz | P0 | `y = y0 + (2*A/pi) * w/(4*(x-xc)^2 + w^2)` | **A = area**, **w = FWHM**. Height = 2*A/(pi*w). Parameter order y0, xc, w, A. |
| 15 | Voigt | P0 | `y = y0 + (2 ln2/pi^1.5)*(A*wL/wG^2) * Integral exp(-t^2)/((sqrt(ln2)*wL/wG)^2 + (2*sqrt(ln2)*(x-xc)/wG - t)^2) dt` over all real t | **A = area**. wG = Gaussian FWHM, wL = Lorentzian FWHM (both > 0). Needs a numeric Voigt (Faddeeva) kernel. Approximate overall FWHM = 0.5346*wL + sqrt(0.2166*wL^2 + wG^2). Parameter order y0, xc, A, wG, wL. |
| 16 | PsdVoigt1 | P0 | `y = y0 + A*( mu*(2/pi)*w/(4*(x-xc)^2+w^2) + (1-mu)*sqrt(4 ln 2)/(sqrt(pi)*w) * exp(-(4 ln 2/w^2)*(x-xc)^2) )` | **A = area**, **w = FWHM shared by both parts**, mu = Lorentzian fraction (mu=1 pure Lorentz, mu=0 pure Gauss). No bounds on mu in the file (usually clamped to [0,1] by users) . Order y0, xc, A, w, mu. |
| 17 | PsdVoigt2 | P0 | same as PsdVoigt1 but Lorentzian part uses wL and Gaussian part uses wG | A = total area; wG, wL = FWHM of each component; mu = Lorentzian fraction. Order y0, xc, A, wG, wL, mu. |
| 18 | Bigaussian | P1 | `y = y0 + H*exp(-(x-xc)^2/(2*w1^2))` if x < xc, else `y0 + H*exp(-(x-xc)^2/(2*w2^2))` | H = height; w1, w2 = left/right sigma. |
| 19 | Asym2Sig | P1 | `y = y0 + A*(1/(1+exp(-(x-xc+w1/2)/w2)))*(1-1/(1+exp(-(x-xc-w1/2)/w3)))` | A = height scale (the true maximum is below A when the edges overlap), w1 = plateau width, w2 / w3 = rise / fall steepness widths. |
| 20 | PearsonVII | P1 | `y = y0 + A*2*Gamma(m)*sqrt(2^(1/m)-1)/(sqrt(pi)*w*Gamma(m-0.5)) * (1 + 4*(2^(1/m)-1)*(x-xc)^2/w^2)^(-m)` | **A = area**, **w = FWHM**, m = shape exponent (bounds 0.5 < m <= 170, m != 0.5). |
| 21 | LogNormal | P1 | `y = y0 + A/(sqrt(2*pi)*w*x) * exp(-(ln(x/xc))^2/(2*w^2))` | A = area, xc = median (bound xc > 0), w = sd of ln x. Requires x > 0. |
| 22 | Boltzmann | P0 | `y = A2 + (A1-A2)/(1+exp((x-x0)/dx))` | A1 = level at low x, A2 = level at high x, x0 = midpoint, dx = width (constraint dx != 0; dx > 0 gives a rising-to-A2 curve when A1 < A2). |
| 23 | Logistic | P1 | `y = A2 + (A1-A2)/(1+(x/x0)^p)` | Sigmoid in x; x0 = midpoint, p = steepness (bound p > 0). Needs x > 0 for non-integer p. |
| 24 | DoseResp | P0 | `y = A1 + (A2-A1)/(1+10^((LOGx0-x)*p))` | x is log10 of dose; LOGx0 = log10 EC50; p = Hill slope. Derived: span = abs(A1-A2), EC50 = 10^LOGx0. |
| 25 | Hill | P1 | `y = Vmax*x^n/(k^n+x^n)` | Bounds Vmax, k, n > 0 (on). k = half-saturation x. |
| 26 | Hyperbl | P1 | `y = P1*x/(P2+x)` | P1 = plateau, P2 = half-plateau x. MichaelisMenten has the same form with names Vmax, Km. |
| 27 | MichaelisMenten | P1 | `y = Vmax*x/(Km+x)` | Bounds > 0. |
| 28 | Sine | P1 | `y = y0 + A*sin(pi*(x-xc)/w)` | **w = half period** (period = 2w), A = amplitude, xc = a zero-crossing (phase) point. Bounds w > 0, A > 0. |
| 29 | SineDamp | P1 | `y = y0 + A*exp(-x/t0)*sin(pi*(x-xc)/w)` | Same as Sine, plus damping time constant t0 > 0 measured from x = 0. |
| 30 | Asymptotic2 | P1 | `y = a - b*exp(-k*x)` | k is a RATE (contrast ExpDec1 where t1 is a time). |

Other points worth knowing for a faithful reimplementation (observed unless marked [inferred]):

* The peak functions Gauss, GaussAmp, Gaussian, Lorentz, Voigt, PsdVoigt1/2, Bigaussian, Asym2Sig, PearsonVII and LogNormal all carry the same three or four control keys naming which parameters are the peak centre, width, amplitude (and a second width for two-width shapes). In Gauss, parameter index 2 = xc, 3 = w, 4 = A (1-based).
* For multi-peak fitting, Origin repeats a block of parameters per peak: `Duplicate Offset` is the 1-based index of the first parameter of the repeating block (2 for all peaks above, so the shared baseline y0 is not repeated) and `Duplicate Unit` is the block length (3 for Gauss/Lorentz/GaussAmp = xc,w,A; 4 for Gaussian/Voigt/PsdVoigt1 etc.) [inferred from the values and parameter lists; confirm by running a 2-peak fit].
* Moments: Gauss declares moments (0th = A, 1st = xc, 2nd central = w^2/4, 3rd = 0, 4th = 3w^4/16); GaussAmp declares 0th = A*sqrt(2*pi)*w; these confirm sigma = w/2 for Gauss and sigma = w for GaussAmp.
* Initial guesses are produced by small procedures that use Origin built-in helpers such as `peak_pos`, `fwhm`, `area`, `xatymax`, `xatymin`, `xaty50`, `yatxmin`, `yatxmax`, `fitpoly`, `get_exponent_cuv`. Typical recipes (own words): peak shapes take the offset from the left edge or a baseline estimate, the centre from the position of the maximum (or minimum if the signed area is negative), the width from the half-maximum width times about 0.85, and the area from numerical integration; Gauss scales that width by 0.85 as well; Lorentz scales the area by 1.57*w; GaussAmp halves the width and rescales A to a height; polynomials start from an ordinary polynomial least-squares fit with every coefficient shrunk by 1%; ExpDec/ExpGro take log-linear estimates of the exponent; sigmoids take the x at half height as the midpoint and set A1/A2 from the data extremes, choosing which is larger by comparing the first and last y; Sine uses an FFT to estimate frequency and amplitude.


## Peak Functions (23)

Single-peak shapes. Most have a centre, width and area/height parameter and carry the peak flags used by multi-peak fitting.

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Asym2Sig | P0 | `y0 + A * (1 / (1 + exp(-(x - xc + w1 / 2) / w2))) * (1 - 1 / (1 + exp(-(x - xc - w1 / 2) / w3)))` | `y0` (offset), `xc` (center), `A` (amplitude), `w1` (full width of half maximum), `w2` (variance of low-energy side), `w3` (variance of high-energy side) | yes | Asymmetric peak built from two logistic (sigmoid) edges: a rising edge of width w2 and a falling edge of width w3, separated by the plateau width w1; A is the height scale. |
| Beta | P1 | `y0 + A * (1 + (w2 + w3 - 2) / (w2 - 1) * ((x - xc) / w1))^(w2 - 1) * (1 - (w2 + w3 - 2) / (w3 - 1) * ((x - xc) / w1))^(w3 - 1)` | `y0` (offset), `xc` (center), `A` (amplitude), `w1` (width), `w2` (width), `w3` (width) | yes |  |
| Bigaussian | P0 | `y = y0 + H*exp(-(x-xc)^2/(2*w1^2)) if x < xc; y = y0 + H*exp(-(x-xc)^2/(2*w2^2)) if x >= xc` | `y0` (base), `xc` (center), `H` (height), `w1` (width), `w2` (width) | yes | Asymmetric peak made of two half-Gaussians with different standard deviations (w1 left of xc, w2 right of xc); H is the peak height above y0. Code-based body. |
| CCE | P1 | `y = y0 + A*( exp(-(x-xc1)^2/(2*w)) + (1 - (1 - tanh(k2*(x-xc2)))/2) * B * exp(-0.5*k3*(abs(x-xc3) + (x-xc3))) )` | `y0` (offset), `xc1` (center), `A` (amplitude), `w` (half width), `k2` (unknown), `xc2` (center), `B` (amplitude), `k3` (unknown), `xc3` (center) | yes | Code-based body. |
| ECS | P1 | `y0 + A / (w * sqrt(2 * pi)) * (exp(-0.5 * ((x - xc) / w)^2) * (1 + a3 / (3 * 2 * 1) * ((x - xc) / w) * (((x - xc) / w)^2 - 3) + a4 / (4 * 3 * 2 * 1) * (((x - xc) / w)^4 - 6 * ((x - xc) / w)^3 + 3) + 10 * a3^2 / (6 * 5 * 4 * 3 * 2 * 1) * (((x - xc) / w)^6 - 15 * ((x - xc) / w)^4 + 45 * ((x - xc) / w)^2 - 15)))` | `y0` (offset), `xc` (center), `A` (area), `w` (half width), `a3` (unknown), `a4` (unknown) | yes |  |
| Extreme | P1 | `y = y0 + A*exp(-exp(-z) - z + 1), z = (x-xc)/w` | `y0` (offset), `xc` (center), `w` (width), `A` (amplitude) | yes | Gumbel-type extreme-value peak shape, A is the height scale. Code-based body. |
| Gauss | P0 | `y0 + A / (w * sqrt(pi / 2)) * exp(-2 * ((x - xc) / w)^2)` | `y0` (offset), `xc` (center), `w` (width), `A` (area) | yes | Gaussian peak parameterised by AREA (A) and a width w equal to twice the standard deviation. FWHM = sqrt(2 ln 2) * w = 1.1774 w. Peak height = A / (w*sqrt(pi/2)). The default peak function. |
| GaussAmp | P0 | `y0 + A * exp(-0.5 * ((x - xc) / w)^2)` | `y0` (offset), `xc` (center), `w` (width), `A` (amplitude) | yes | Gaussian peak parameterised by HEIGHT (A above the offset y0) and w equal to the standard deviation sigma. FWHM = 2*sqrt(2 ln 2)*w = 2.3548 w; area = A*w*sqrt(2*pi). |
| Gaussian | P0 | `y0 + A / (w * sqrt(pi / (4 * ln(2)))) * exp(-4 * ln(2) * (x - xc)^2 / w^2)` | `y0` (base), `xc` (center), `A` (area), `w` (fwhm) | yes | Gaussian peak parameterised by AREA (A) and the full width at half maximum w. Peak height = A*sqrt(4 ln 2 / pi)/w = 0.9394*A/w. |
| GaussMod | P1 | `y = y0 + A/t0 * exp(0.5*(w/t0)^2 - (x-xc)/t0) * (erf(((x-xc)/w - w/t0)/sqrt(2)) + 1)/2` | `y0` (offset), `A` (area), `xc` (center), `w` (width), `t0` (unknown) | yes | Exponentially modified Gaussian (a Gaussian convolved with an exponential tail), the standard chromatography peak shape. Code-based body. |
| GCAS | P1 | `y0 + A / (w * sqrt(2 * pi)) * exp(-((x - xc) / w)^2 / 2) * (1 + abs(a3 / 6 * (((x - xc) / w)^3 - 3 * ((x - xc) / w)) + a4 / 24 * (((x - xc) / w)^4 - 6 * ((x - xc) / w)^3 + 3)))` | `y0` (offset), `xc` (center), `A` (area), `w` (half width), `a3` (unknown), `a4` (unknown) | yes |  |
| Giddings | P1 | `y = y0 + (A/w)*sqrt(xc/x)*I1(2*sqrt(xc*x)/w)*exp(-(x+xc)/w), I1 = modified Bessel function of the first kind, order 1` | `y0` (offset), `xc` (center), `w` (width), `A` (area) | yes | Code-based body. |
| InvsPoly | P1 | `y0 + A / (1 + A1 * (2 * (x - xc) / w)^2 + A2 * (2 * (x - xc) / w)^4 + A3 * (2 * (x - xc) / w)^6)` | `y0` (offset), `xc` (center), `w` (width), `A` (amplitude), `A1` (coefficient), `A2` (coefficient), `A3` (coefficient) | yes |  |
| Laplace | P1 | `y0 + exp(-abs(x - a) / b) / (2 * b)` | `y0` (offset), `a` (location), `b` (shape) | yes | Laplace (double-exponential) peak, centred at a with scale b. |
| Logistpk | P1 | `y = y0 + 4*A*exp(-z)/(1+exp(-z))^2, z = (x-xc)/w` | `y0` (offset), `xc` (center), `w` (width), `A` (amplitude) | yes | Code-based body. |
| LogNormal | P1 | `y0 + A / (sqrt(2 * pi) * w * x) * exp(-ln(x / xc)^2 / (2 * w^2))` | `y0` (offset), `xc` (center), `w` (log standard deviation), `A` (area) | yes | Log-normal distribution shape in x > 0; A is the area, xc the median, w the standard deviation of ln(x). |
| Lorentz | P0 | `y0 + 2 * A / pi * (w / (4 * (x - xc)^2 + w^2))` | `y0` (offset), `xc` (center), `w` (fwhm), `A` (area) | yes | Lorentzian (Cauchy) peak parameterised by AREA (A) and the full width at half maximum w. Peak height = 2*A/(pi*w). |
| PearsonIV | P1 | `y = y0 + A*k*(1+((x-lam)/alpha)^2)^(-m)*exp(-v*atan((x-lam)/alpha)); k is a normalising constant (see the file comment: k = Gamma(m)*\|Gamma(m + i*v/2)/Gamma(m)\|^2/(sqrt(pi)*Gamma(m-1/2))); evaluated by a native routine; the exact normalisation (e.g. a factor 1/alpha) is NOT confirmed` | `y0` (offset), `A` (area), `m` (m), `v` (v), `alpha` (x width), `lam` (x center) | yes | Code-based body. |
| PearsonVII | P1 | `y0 + A * 2 * gamma(m) * sqrt(2^(1 / m) - 1) / (sqrt(pi) * w * gamma(m - 0.5)) * (1 + 4 * (2^(1 / m) - 1) / w^2 * (x - xc)^2)^(-m)` | `y0` (baseline), `xc` (center), `A` (area), `w` (width), `m` (profile shape factor) | yes | Peak with Lorentzian-like tails whose sharpness is controlled by m (m = 1 gives a Lorentzian, large m approaches a Gaussian). A is the area and w the FWHM. |
| PsdVoigt1 | P0 | `y0 + A * (mu * (2 / pi) * (w / (4 * (x - xc)^2 + w^2)) + (1 - mu) * (sqrt(4 * ln(2)) / (sqrt(pi) * w)) * exp(-(4 * ln(2) / w^2) * (x - xc)^2))` | `y0` (offset), `xc` (center), `A` (area), `w` (fwhm), `mu` (profile shape factor) | yes | Pseudo-Voigt: a weighted sum of a Lorentzian and a Gaussian of the SAME FWHM w and the same area A. mu = 1 is pure Lorentzian, mu = 0 pure Gaussian. |
| PsdVoigt2 | P0 | `y0 + A * (mu * (2 / pi) * (wL / (4 * (x - xc)^2 + wL^2)) + (1 - mu) * (sqrt(4 * ln(2)) / (sqrt(pi) * wG)) * exp(-(4 * ln(2) / wG^2) * (x - xc)^2))` | `y0` (offset), `xc` (center), `A` (area), `wG` (gaussian fwhm), `wL` (lorentzian fwhm), `mu` (profile shape factor) | yes | Pseudo-Voigt with independent FWHM values: Lorentzian part uses wL, Gaussian part uses wG; mu is the Lorentzian fraction. Each component is area-normalised, so A is the total area. |
| Voigt | P0 | `y = y0 + (2*ln2/pi^1.5) * (A*wL/wG^2) * Integral_{-inf}^{inf} exp(-t^2)/( (sqrt(ln2)*wL/wG)^2 + (2*sqrt(ln2)*(x-xc)/wG - t)^2 ) dt ; A = area, wG and wL = Gaussian and Lorentzian FWHM; evaluated numerically by a native routine` | `y0` (offset), `xc` (center), `A` (area), `wG` (gaussian width), `wL` (lorentzian width) | yes | Convolution of a Gaussian and a Lorentzian, area A, with separate FWHM values wG and wL. No closed form; needs a numerical Voigt/Faddeeva evaluation. Approximate total FWHM = 0.5346*wL + sqrt(0.2166*wL^2 + wG^2). Code-based body. |
| Weibull3 | P1 | `y = y0 + A*((w2-1)/w2)^((1-w2)/w2) * S^(w2-1) * exp(-S^w2 + (w2-1)/w2), S = (x-xc)/w1 + ((w2-1)/w2)^(1/w2)` | `y0` (offset), `xc` (center), `A` (amplitude), `w1` (width), `w2` (width) | yes | Code-based body. |

## Growth/Sigmoidal (19)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| BiDoseResp | P1 | `y = A1 + (A2-A1)*p/(1+10^((LOGx01-x)*h1)) + (A2-A1)*(1-p)/(1+10^((LOGx02-x)*h2))` | `A1` (bottom), `A2` (top), `LOGx01` (1st ec50), `LOGx02` (2nd ec50), `h1` (slope1), `h2` (slope2), `p` (proportion) |  | Code-based body. |
| BiHill | P1 | `Pm / (1 + (Ka / x)^Ha) / (1 + (x / Ki)^Hi)` | `Pm` (maximum), `Ka` (half-maximal activating), `Ki` (half-maximal inhibitory), `Ha` (activation hill coefficient), `Hi` (inhibitory hill coefficient) |  |  |
| BoltzIV | P1 | `(x - vrev) * gmax / (1 + exp((x - vhalf) / dx))` | `vhalf` (center), `dx` (time const), `gmax` (gmax), `vrev` (vrev) |  | Current-voltage relation of a Boltzmann-gated channel: linear driving force (x - vrev) times conductance gmax times a sigmoid in x. |
| Boltzmann | P0 | `A2 + (A1 - A2) / (1 + exp((x - x0) / dx))` | `A1` (init value), `A2` (final value), `x0` (center), `dx` (time const) |  | Sigmoidal step from level A1 (at low x) to level A2 (at high x) centred at x0 with slope-width dx. |
| DoseResp | P0 | `A1 + (A2 - A1) / (1 + 10^((LOGx0 - x) * p))` | `A1` (bottom asymptote), `A2` (top asymptote), `LOGx0` (center), `p` (hill slope) |  | Four-parameter dose-response sigmoid with x on a logarithmic axis: A1 bottom, A2 top, LOGx0 the log10 midpoint, p the Hill slope. |
| DoubleBoltzmann | P1 | `y0 + A * (frac / (1 + exp((x - x01) / k1)) + (1 - frac) / (1 + exp((x - x02) / k2)))` | `y0` (offset), `A` (span), `frac` (fraction), `x01` (center), `x02` (center), `k1` (slope factor), `k2` (slope factor) |  |  |
| Hill | P0 | `Vmax * x^n / (k^n + x^n)` | `Vmax` (max velocity), `k` (michaelis constant), `n` (cooperative sites) |  | Hill equation, Vmax*x^n/(k^n + x^n); k is the half-saturation x, n the cooperativity. |
| Hill1 | P1 | `START + (END - START) * x^n / (k^n + x^n)` | `START` (start), `END` (end), `k` (michaelis constant), `n` (cooperative sites) |  |  |
| Logistic | P0 | `A2 + (A1 - A2) / (1 + (x / x0)^p)` | `A1` (init value), `A2` (final value), `x0` (center), `p` (power) |  | Sigmoid in x/x0 (a Hill-type curve) from A1 to A2; p is the steepness exponent. Needs x > 0. |
| Logistic5 | P1 | `Amin + (Amax - Amin) / (1 + (x0 / x)^h)^s` | `Amin` (lower asymmetry), `Amax` (upper asymmetry), `x0` (x of half y), `h` (hill slope), `s` (control factor) |  |  |
| MichaelisMenten | P1 | `Vmax * x / (Km + x)` | `Vmax` (maximum velocity), `Km` (michaelis constant) |  | Michaelis-Menten saturation curve (same functional form as Hyperbl). |
| SGompertz | P1 | `a * exp(-exp(-k * (x - xc)))` | `a` (amplitude), `xc` (center), `k` (coefficient) |  |  |
| SLogistic1 | P1 | `a / (1 + exp(-k * (x - xc)))` | `a` (amplitude), `xc` (center), `k` (coefficient) |  |  |
| SLogistic2 | P1 | `a / (1 + (a - y0) / y0 * exp(-4 * Wmax * x / a))` | `y0` (y initial value), `a` (amplitude), `Wmax` (maximun growth rate) |  |  |
| SLogistic3 | P1 | `a / (1 + b * exp(-k * x))` | `a` (amplitude), `b` (coefficient), `k` (coefficient) |  |  |
| SRichards1 | P1 | `y = (a^(1-d) - exp(-k*(x-xc)))^(1/(1-d)) if d < 1; y = (a^(1-d) + exp(-k*(x-xc)))^(1/(1-d)) if d > 1; undefined (NaN) if d == 1` | `a` (top asymptote), `xc` (center), `d` (unknown), `k` (coefficient) |  | Code-based body. |
| SRichards2 | P1 | `a * (1 + (d - 1) * exp(-k * (x - xc)))^(1 / (1 - d))` | `a` (top asymptote), `xc` (center), `d` (unknown), `k` (coefficient) |  |  |
| SWeibull1 | P1 | `A * (1 - exp(-(k * (x - xc))^d))` | `A` (amplitude), `xc` (center), `d` (power), `k` (coefficient) |  |  |
| SWeibull2 | P1 | `a - (a - b) * exp(-(k * x)^d)` | `a` (top asymptote), `b` (unknown), `d` (power), `k` (coefficient) |  |  |

## Exponential (42)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Asymptotic1 | P1 | `a - b * c^x` | `a` (asymptote), `b` (response range), `c` (rate) |  |  |
| BoxLucas1 | P1 | `a * (1 - exp(-b * x))` | `a` (coefficient), `b` (coefficient) |  |  |
| BoxLucas1Mod | P1 | `a * (1 - b^x)` | `a` (coefficient), `b` (coefficient) |  |  |
| BoxLucas2 | P1 | `a1 / (a1 - a2) * (exp(-a2 * x) - exp(-a1 * x))` | `a1` (unknown), `a2` (unknown) |  |  |
| Chapman | P1 | `a * (1 - exp(-b * x))^c` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Exp1P1 | P1 | `exp(x - A)` | `A` (position) |  |  |
| Exp1p2 | P1 | `exp(-A * x)` | `A` (coefficient) |  |  |
| Exp1p2Md | P1 | `B^x` | `B` (position) |  |  |
| Exp1p3 | P1 | `A * exp(-A * x)` | `A` (coefficient) |  |  |
| Exp1P3Md | P1 | `-ln(B) * B^x` | `B` (coefficient) |  |  |
| Exp1P4 | P1 | `1 - exp(-A * x)` | `A` (coefficient) |  |  |
| Exp1P4Md | P1 | `1 - B^x` | `B` (coefficient) |  |  |
| Exp2P | P1 | `a * b^x` | `a` (position), `b` (?) |  |  |
| Exp2PMod1 | P1 | `a * exp(b * x)` | `a` (coefficient), `b` (rate) |  |  |
| Exp2PMod2 | P1 | `exp(a + b * x)` | `a` (coefficient), `b` (rate) |  |  |
| Exp3P1 | P1 | `a * exp(b / (x + c))` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Exp3P1Md | P1 | `exp(a + b / (x + c))` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Exp3P2 | P1 | `exp(a + b * x + c * x^2)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| ExpAssoc | P0 | `y0 + A1 * (1 - exp(-x / t1)) + A2 * (1 - exp(-x / t2))` | `y0` (offset), `A1` (amplitude), `t1` (width), `A2` (amplitude), `t2` (width) |  | Two-phase association (rise to a plateau), y0 + A1*(1-exp(-x/t1)) + A2*(1-exp(-x/t2)). |
| ExpDec1 | P0 | `A1 * exp(-x / t1) + y0` | `y0` (offset), `A1` (amplitude), `t1` (time constant) |  | Single exponential decay toward an offset; t1 is the 1/e time constant (not a rate). |
| ExpDec2 | P0 | `A1 * exp(-x / t1) + A2 * exp(-x / t2) + y0` | `y0` (offset), `A1` (amplitude), `t1` (time constant), `A2` (amplitude), `t2` (time constant) |  | Sum of two exponential decays plus an offset. |
| ExpDec3 | P0 | `A1 * exp(-x / t1) + A2 * exp(-x / t2) + A3 * exp(-x / t3) + y0` | `y0` (offset), `A1` (amplitude), `t1` (time constant), `A2` (amplitude), `t2` (time constant), `A3` (amplitude), `t3` (time constant) |  | Sum of three exponential decays plus an offset. |
| ExpDecay1 | P1 | `y0 + A1 * exp(-(x - x0) / t1)` | `y0` (y offset), `x0` (x offset), `A1` (amplitude), `t1` (time constant) |  |  |
| ExpDecay2 | P1 | `y0 + A1 * exp(-(x - x0) / t1) + A2 * exp(-(x - x0) / t2)` | `y0` (y offset), `x0` (x offset), `A1` (amplitude), `t1` (time constant), `A2` (amplitude), `t2` (time constant) |  |  |
| ExpDecay3 | P1 | `y0 + A1 * exp(-(x - x0) / t1) + A2 * exp(-(x - x0) / t2) + A3 * exp(-(x - x0) / t3)` | `y0` (y offset), `x0` (x offset), `A1` (amplitude), `t1` (time constant), `A2` (amplitude), `t2` (time constant), `A3` (amplitude), `t3` (time constant) |  |  |
| ExpGro1 | P0 | `A1 * exp(x / t1) + y0` | `y0` (offset), `A1` (amplitude), `t1` (time constant) |  | Single exponential growth, y0 + A1*exp(x/t1); t1 is the growth time constant. |
| ExpGro2 | P1 | `A1 * exp(x / t1) + A2 * exp(x / t2) + y0` | `y0` (offset), `A1` (amplitude), `t1` (time constant), `A2` (amplitude), `t2` (time constant) |  |  |
| ExpGro3 | P1 | `A1 * exp(x / t1) + A2 * exp(x / t2) + A3 * exp(x / t3) + y0` | `y0` (offset), `A1` (amplitude), `t1` (time constant), `A2` (amplitude), `t2` (time constant), `A3` (amplitude), `t3` (time constant) |  |  |
| ExpGrow1 | P0 | `y0 + A1 * exp((x - x0) / t1)` | `y0` (y offset), `x0` (x offset), `A1` (amplitude), `t1` (time constant) |  | Single exponential growth with a shift on x, so that the curve equals y0 + A1 at x = x0. |
| ExpGrow2 | P1 | `y0 + A1 * exp((x - x0) / t1) + A2 * exp((x - x0) / t2)` | `y0` (y offset), `x0` (x offset), `A1` (amplitude), `t1` (time constant), `A2` (amplitude), `t2` (time constant) |  |  |
| ExpGrow3Dec2 | P1 | `if x < xc: y = y0 + Ad1 + Ad2 - SUM_i[Ag_i*(1-exp(-xc/tg_i))] + SUM_i[Ag_i*(1-exp(-x/tg_i))] (i=1..3); otherwise y = y0 + Ad1*exp(-(x-xc)/td1) + Ad2*exp(-(x-xc)/td2)` | `y0` (offset), `xc` (center), `Ag1` (growth amplitude), `tg1` (growth time constant), `Ag2` (growth amplitude), `tg2` (growth time constant), `Ag3` (growth amplitude), `tg3` (growth time constant), `Ad1` (decay amplitude), `td1` (decay time constant), `Ad2` (decay amplitude), `td2` (decay time constant) |  | Code-based body. |
| ExpGrowDec | P1 | `if x < xc: y = y0 + Ad + Ag*(exp(-xc/tg) - exp(-x/tg)); otherwise y = y0 + Ad*exp(-(x-xc)/td)` | `y0` (offset), `xc` (center), `Ag` (growth amplitude), `tg` (growth time constant), `Ad` (decay amplitude), `td` (decay time constant) |  | Code-based body. |
| ExpLinear | P1 | `p1 * exp(-x / p2) + p3 + p4 * x` | `p1` (coefficient), `p2` (unknown), `p3` (offset), `p4` (coefficient) |  |  |
| Exponential | P1 | `y0 + A * exp(R0 * x)` | `y0` (offset), `A` (initial value), `R0` (rate) |  |  |
| Langevin | P1 | `y = y0 + C*(coth(x-xc) - 1/(x-xc))` | `y0` (offset), `xc` (center), `C` (amplitude) |  | Code-based body. |
| LangevinMod | P1 | `y = y0 + C*(coth((x-xc)/s) - s/(x-xc))` | `y0` (offset), `xc` (center), `C` (amplitude), `s` (scale) |  | Code-based body. |
| MnMolecular | P1 | `A * (1 - exp(-k * (x - xc)))` | `A` (amplitude), `xc` (center), `k` (rate) |  |  |
| MnMolecular1 | P1 | `A1 - A2 * exp(-k * x)` | `A1` (offset), `A2` (coefficient), `k` (coefficient) |  |  |
| Shah | P1 | `a + b * x + c * r^x` | `a` (offset), `b` (coefficient), `c` (coefficient), `r` (?) |  |  |
| Stirling | P1 | `a + b * (exp(k * x) - 1) / k` | `a` (offset), `b` (coefficient), `k` (coefficient) |  |  |
| YldFert | P1 | `a + b * r^x` | `a` (offset), `b` (coefficient), `r` (coefficient) |  |  |
| YldFert1 | P1 | `a + b * exp(-k * x)` | `a` (offset), `b` (coefficient), `k` (cofficient) |  |  |

## Polynomial (8)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Constant | P1 | `y0 + 0 * x` | `y0` (unknown) |  |  |
| Cubic | P0 | `A + B * x + C * x^2 + D * x^3` | `A` (offset), `B` (cofficient), `C` (cofficient), `D` (cofficient) |  | Cubic polynomial in ascending powers. |
| Line | P0 | `A + B * x` | `A` (yintercept), `B` (slope) |  | Straight line; A is the intercept, B the slope. |
| LineMod | P1 | `a * (x - b)` | `a` (cofficient), `b` (position) |  |  |
| Parabola | P0 | `A + B * x + C * x^2` | `A` (offset), `B` (cofficient), `C` (cofficient) |  | Quadratic polynomial in ascending powers. |
| Poly | P0 | `a0 + a1 * x + a2 * x^2 + a3 * x^3 + a4 * x^4 + a5 * x^5 + a6 * x^6 + a7 * x^7 + a8 * x^8 + a9 * x^9` | `a0` (coefficients), `a1` (?), `a2` (?), `a3` (?), `a4` (?), `a5` (?), `a6` (?), `a7` (?), `a8` (?), `a9` (?) |  | General polynomial of degree up to 9 written in ascending powers; unused high coefficients are meant to be fixed at zero. |
| Poly4 | P1 | `A0 + A1 * x + A2 * x^2 + A3 * x^3 + A4 * x^4` | `A0` (offset), `A1` (cofficient), `A2` (cofficient), `A3` (cofficient), `A4` (cofficient) |  |  |
| Poly5 | P1 | `A0 + A1 * x + A2 * x^2 + A3 * x^3 + A4 * x^4 + A5 * x^5` | `A0` (offset), `A1` (cofficient), `A2` (cofficient), `A3` (cofficient), `A4` (cofficient), `A5` (cofficient) |  |  |

## Power (18)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Allometric1 | P0 | `a * x^b` | `a` (coefficient), `b` (power) |  | Power law with a free prefactor, a*x^b; linear on log-log axes. |
| Allometric2 | P1 | `a + b * x^c` | `a` (offset), `b` (coefficient), `c` (power) |  |  |
| Belehradek | P1 | `a * (x - b)^c` | `a` (coefficient), `b` (position), `c` (power) |  |  |
| BlNeld | P1 | `(a + b * x^f)^(-1 / c)` | `a` (coefficient), `b` (coefficient), `c` (coefficient), `f` (power) |  |  |
| BlNeldSmp | P1 | `(a + b * x)^(-1 / c)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| FarazdaghiHarris | P1 | `(a + b * x^c)^(-1)` | `a` (coefficient), `b` (coefficient), `c` (power) |  |  |
| FreundlichEXT | P1 | `a * x^(b * x^(-c))` | `a` (coefficient), `b` (coefficient), `c` (power) |  |  |
| Gunary | P1 | `x / (a + b * x + c * sqrt(x))` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| LangmuirEXT1 | P1 | `a * b * x^(1 - c) / (1 + b * x^(1 - c))` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| LangmuirEXT2 | P1 | `1 / (a + b * x^(c - 1))` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Pareto | P1 | `1 - 1 / x^A` | `A` (cofficient) |  |  |
| Pow2P1 | P1 | `a * (1 - x^(-b))` | `a` (coefficient), `b` (power) |  |  |
| Pow2P2 | P1 | `a * (1 + x)^b` | `a` (coefficient), `b` (power) |  |  |
| Pow2P3 | P1 | `1 - 1 / (1 + a * x)^b` | `a` (coefficient), `b` (power) |  |  |
| Power | P0 | `x^A` | `A` (power) |  | Pure power x^A with a single exponent (no prefactor). For a prefactor use Allometric1. |
| Power0 | P1 | `y0 + A * abs(x - xc)^P` | `y0` (offset), `xc` (center), `A` (amplitude), `P` (power) |  |  |
| Power1 | P1 | `A * abs(x - xc)^P` | `xc` (center), `A` (amplitude), `P` (power) |  |  |
| Power2 | P1 | `y = A*\|x-xc\|^pl if x < xc; y = A*\|x-xc\|^pu if x > xc; y = 0 if x == xc` | `xc` (center), `A` (amplitude), `pl` (power), `pu` (power) |  | Code-based body. |

## Rational (15)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| BET | P2 | `a * b * x / (1 + (b - 2) * x - (b - 1) * x^2)` | `a` (coefficient), `b` (coefficient) |  |  |
| BETMod | P2 | `x / (a + b * x - (a + b) * x^2)` | `a` (coefficient), `b` (coefficient) |  |  |
| Holliday | P2 | `1.0 / (a + b * x + c * x^2)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Holliday1 | P2 | `a / (1 + b * x + c * x^2)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Nelder | P2 | `(x + a) / (b0 + b1 * (x + a) + b2 * (x + a)^2)` | `a` (unknown), `b0` (unknown), `b1` (unknown), `b2` (unknown) |  |  |
| Rational0 | P2 | `(b + c * x) / (1 + a * x)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  | Rational function (b + c x)/(1 + a x). |
| Rational1 | P2 | `(1 + c * x) / (a + b * x)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Rational2 | P2 | `(b + c * x) / (a + x)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Rational3 | P2 | `(b + x) / (a + c * x)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Rational4 | P2 | `c + b / (x + a)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Rational5 | P2 | `(a + b * x) / (1 + c * x + d * x^2)` | `a` (free term of numerator), `b` (linear coefficient of numerator), `c` (linear coefficient of denominator), `d` (binomial coefficient of denominator) |  |  |
| Reciprocal | P2 | `1 / (a + b * x)` | `a` (coefficient), `b` (coefficient) |  |  |
| Reciprocal0 | P2 | `1 / (1 + A * x)` | `A` (coefficient) |  |  |
| Reciprocal1 | P2 | `1 / (x + A)` | `A` (position) |  |  |
| ReciprocalMod | P2 | `a / (1 + b * x)` | `a` (coefficient), `b` (coefficient) |  |  |

## Logarithm (5)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Bradley | P2 | `a * ln(-b * ln(x))` | `a` (unknown), `b` (unknown) |  |  |
| Log2P1 | P2 | `b * ln(x - a)` | `a` (offset), `b` (cofficient) |  |  |
| Log2P2 | P2 | `ln(a + b * x)` | `a` (offset), `b` (cofficient) |  |  |
| Log3P1 | P2 | `a - b * ln(x + c)` | `a` (coefficient), `b` (coefficient), `c` (coefficient) |  |  |
| Logarithm | P2 | `ln(x - A)` | `A` (center) |  |  |

## Hyperbola (5)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Dhyperbl | P1 | `P1 * x / (P2 + x) + P3 * x / (P4 + x) + P5 * x` | `P1` (unknown), `P2` (unknown), `P3` (unknown), `P4` (unknown), `P5` (unknown) |  |  |
| Hyperbl | P1 | `P1 * x / (P2 + x)` | `P1` (vertical asymptote), `P2` (horizontal asymptote) |  | Rectangular hyperbola P1*x/(P2+x); P1 is the plateau, P2 the x at half plateau. |
| HyperbolaGen | P1 | `a - b / (1 + c * x)^(1 / d)` | `a` (coefficient), `b` (coefficient), `c` (coefficient), `d` (coefficient) |  |  |
| HyperbolaMod | P1 | `x / (T1 * x + T2)` | `T1` (amplitude), `T2` (unknown) |  |  |
| RectHyperbola | P1 | `a * b * x / (1 + b * x)` | `a` (coefficient), `b` (coefficient) |  |  |

## Waveform (7)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| SawtoothWave | P1 | `y0 + A / T * (x - floor((x - x0) / T) * T - x0)` | `x0` (x start), `y0` (offset), `A` (amplitude), `T` (period) |  |  |
| Sine | P0 | `y0 + A * sin(pi * (x - xc) / w)` | `y0` (offset), `xc` (phase shift), `w` (period), `A` (amplitude) | yes | Sine wave; w is HALF the period (period = 2*w), A the amplitude, xc a phase point where the wave crosses y0. |
| SineDamp | P1 | `y0 + A * exp(-x / t0) * sin(pi * (x - xc) / w)` | `y0` (offset), `xc` (phase shift), `w` (period), `t0` (decay constant), `A` (amplitude) | yes | Exponentially damped sine; same conventions as Sine, decay time constant t0 (measured from x = 0, not from xc). |
| SineSqr | P1 | `y0 + A * sin(pi * (x - xc) / w)^2` | `y0` (offset), `xc` (phase shift), `w` (period), `A` (amplitude) | yes |  |
| SquareWave | P1 | `Alternates between level a and level b with period T starting at x0; implemented by a native routine (exact duty cycle and edge convention not stated in the file)` | `a` (high-level), `b` (low-level), `x0` (x start), `T` (period) |  | Code-based body. |
| SquareWaveMod | P1 | `n = floor((x-x0)/T); t = x - n*T; y = a if t <= x0 + duty*T; otherwise y = b` | `a` (high-level), `b` (low-level), `x0` (x start), `duty` (duty ratio), `T` (period) |  | Code-based body. |
| Step | P1 | `y = A if x < x1; y = B if x >= x1` | `A` (level of the first piece), `B` (level of the second piece), `x1` (jump point) |  | Two-level step at x1 (A before, B from x1 on). Code-based body. |

## Piecewise (2)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| PWL2 | P2 | `y = a1 + k1*x if x < xi; y = (a1 + k1*xi) + k2*(x-xi) if x >= xi` | `a1` (intercept), `k1` (slope), `xi` (intersection), `k2` (slope) |  | Code-based body. |
| PWL3 | P2 | `y = a1 + k1*x if x < xi1; y = yi1 + k2*(x-xi1) if xi1 <= x < xi2; y = yi2 + k3*(x-xi2) if x >= xi2; where yi1 = a1 + k1*xi1, yi2 = yi1 + k2*(xi2-xi1)` | `a1` (intercept), `k1` (slope), `xi1` (intersection), `k2` (slope), `xi2` (intersection), `k3` (slope) |  | Code-based body. |

## Implicit (3)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Circle | P2 | `[x,y -> f] (x - xc)^2 + (y - yc)^2 - r^2` | `xc` (x center), `yc` (y center), `r` (radius) |  |  |
| Ellipse | P2 | `[x,y -> f] ((x - xc) / a)^2 + ((y - yc) / b)^2 - 1` | `xc` (x center), `yc` (y center), `a` (major radius), `b` (minor radius) |  |  |
| PlaneMod | P2 | `[x,y,z -> f] sin(theta) * cos(phi) * x + sin(theta) * sin(phi) * y + cos(theta) * z + d` | `theta` (?), `phi` (?), `d` (?) |  |  |

## Surface Fitting (22)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Chebyshev2D | P2 | `[x,y -> z] z0 + A1 * cos(acos(x)) + B1 * cos(acos(y)) + A2 * cos(2 * acos(x)) + C1 * cos(acos(x)) * cos(acos(y)) + B2 * cos(2 * acos(y))` | `z0` (?), `A1` (?), `A2` (?), `B1` (?), `B2` (?), `C1` (?) |  |  |
| Cosine | P2 | `[x,y -> z] z0 + A1 * cos(x) + B1 * cos(y) + A2 * cos(2 * x) + C1 * cos(x) * cos(y) + B2 * cos(2 * y)` | `z0` (?), `A1` (?), `A2` (?), `B1` (?), `B2` (?), `C1` (?) |  |  |
| DoseResp2D | P2 | `[x,y -> z] z0 + B * (1 / (1 + (x / C)^(-D))) * (1 / (1 + (y / E)^(-F)))` | `z0` (?), `B` (?), `C` (?), `D` (?), `E` (?), `F` (?) |  |  |
| Exponential2D | P2 | `[x,y -> z] z0 + B * exp(-x / C) * exp(-y / D)` | `z0` (?), `B` (?), `C` (?), `D` (?) |  |  |
| Extreme2D | P2 | `[x,y -> z] z0 + B * y + C * y^2 + D * exp(-exp(-(x - E) / F) - (x - E) / F + 1)` | `z0` (?), `B` (?), `C` (?), `D` (?), `E` (?), `F` (?) |  |  |
| ExtremeCum | P2 | `[x,y -> z] z0 + B * exp(-exp(-(x - C) / D)) + E * exp(-exp(-(y - F) / G)) + H * exp(-exp(-(x - C) / D) - exp(-(y - F) / G))` | `z0` (?), `B` (?), `C` (?), `D` (?), `E` (?), `F` (?), `G` (?), `H` (?) |  |  |
| Fourier2D | P2 | `[x,y -> z] z0 + a * cos(x / w1) + b * sin(x / w1) + c * cos(y / w2) + d * sin(y / w2)` | `z0` (?), `a` (?), `b` (?), `c` (?), `d` (?), `w1` (?), `w2` (?) |  |  |
| Gauss2D | P2 | `[x,y -> z] z0 + A * exp(-0.5 * ((x - xc) / w1)^2 - 0.5 * ((y - yc) / w2)^2)` | `z0` (z offset), `A` (height), `xc` (x center), `w1` (x width), `yc` (y center), `w2` (y width) | yes |  |
| GaussCum | P2 | `[x,y -> z] z0 + 0.25 * B * (1 + erf((x - C) / (sqrt(2) * D))) * (1 + erf((y - E) / (sqrt(2) * F)))` | `z0` (?), `B` (?), `C` (?), `D` (?), `E` (?), `F` (?) |  |  |
| Gaussian2D | P2 | `[x,y -> z] z0 + A * exp(-0.5 * ((x * cos(theta) + y * sin(theta) - xc * cos(theta) - yc * sin(theta)) / w1)^2 - 0.5 * ((-x * sin(theta) + y * cos(theta) + xc * sin(theta) - yc * cos(theta)) / w2)^2)` | `z0` (z offset), `A` (height), `xc` (x center), `w1` (x width), `yc` (y center), `w2` (y width), `theta` (angle) | yes |  |
| LogisticCum | P2 | `[x,y -> z] z0 + B * (1 / (1 + exp(-((x - C) / D)))) * (1 / (1 + exp(-((y - E) / F))))` | `z0` (?), `B` (?), `C` (?), `D` (?), `E` (?), `F` (?) |  |  |
| LogNormal2D | P2 | `[x,y -> z] z0 + B * exp(-0.5 * (ln(x / C) / D)^2) + E * exp(-0.5 * (ln(y / F) / G)^2) + H * exp(-0.5 * ((ln(x / C) / D)^2 + (ln(y / F) / G)^2))` | `z0` (z offset), `B` (height), `C` (x center), `D` (x width), `E` (height), `F` (y center), `G` (y width), `H` (height) |  |  |
| Lorentz2D | P2 | `[x,y -> z] z0 + A / ((1 + ((x - xc) / w1)^2) * (1 + ((y - yc) / w2)^2))` | `z0` (z offset), `A` (height), `xc` (x center), `w1` (x width), `yc` (y center), `w2` (y width) | yes |  |
| Parabola2D | P2 | `[x,y -> z] z0 + a * x + b * y + c * x^2 + d * y^2` | `z0` (?), `a` (?), `b` (?), `c` (?), `d` (?) |  |  |
| Plane | P2 | `[x,y -> z] z0 + a * x + b * y` | `z0` (?), `a` (?), `b` (?) |  |  |
| Poly2D | P2 | `[x,y -> z] z0 + a * x + b * y + c * x^2 + d * y^2 + f * x * y` | `z0` (?), `a` (?), `b` (?), `c` (?), `d` (?), `f` (?) |  |  |
| Polynomial2D | P2 | `[x,y -> z] z0 + A1 * x + A2 * x^2 + A3 * x^3 + A4 * x^4 + A5 * x^5 + B1 * y + B2 * y^2 + B3 * y^3 + B4 * y^4 + B5 * y^5` | `z0` (?), `A1` (?), `A2` (?), `A3` (?), `A4` (?), `A5` (?), `B1` (?), `B2` (?), `B3` (?), `B4` (?), `B5` (?) |  |  |
| Power2D | P2 | `[x,y -> z] z0 + B * x^C + D * y^E + F * x^C * y^E` | `z0` (offset), `B` (?), `C` (?), `D` (?), `E` (?), `F` (?) |  |  |
| Rational2D | P2 | `[x,y -> z] (z0 + A01 * x + B01 * y + B02 * y^2 + B03 * y^3) / (1 + A1 * x + A2 * x^2 + A3 * x^3 + B1 * y + B2 * y^2)` | `z0` (?), `A01` (?), `B01` (?), `B02` (?), `B03` (?), `A1` (?), `A2` (?), `A3` (?), `B1` (?), `B2` (?) |  |  |
| RationalTaylor | P2 | `[x,y -> z] (z0 + A01 * x + B01 * y + B02 * y^2 + C02 * x * y) / (1 + A1 * x + B1 * y + A2 * x^2 + B2 * y^2 + C2 * x * y)` | `z0` (?), `A01` (?), `B01` (?), `B02` (?), `C02` (?), `A1` (?), `A2` (?), `B1` (?), `B2` (?), `C2` (?) |  |  |
| Voigt2D | P2 | `[x,y -> z] z0 + A * (mu / ((1 + ((x - xc) / w1)^2) * (1 + ((y - yc) / w2)^2)) + (1 - mu) * exp(-0.5 * ((x - xc) / w1)^2 - 0.5 * ((y - yc) / w2)^2))` | `z0` (z offset), `A` (height), `xc` (x center), `w1` (x width), `yc` (y center), `w2` (y width), `mu` (profile shape factor) | yes |  |
| Voigt2DMod | P2 | `[x,y -> z] z0 + A * (mu * 4 / pi^2 * w1 * w2 / ((4 * (x - xc)^2 + w1^2) * (4 * (y - yc)^2 + w2^2)) + (1 - mu) * (4 * ln(2)) / (pi * w1 * w2) * exp(-4 * ln(2) / w1^2 * (x - xc)^2 - 4 * ln(2) / w2^2 * (y - yc)^2))` | `z0` (z offset), `A` (volume), `xc` (x center), `w1` (fwhm for x), `yc` (y center), `w2` (fwhm for y), `mu` (profile shape factor) | yes |  |

## Statistics (9)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| ExponentialCDF | P2 | `y = y0 if x < 0; y = y0 + A*(1 - exp(-x/mu)) if x >= 0` | `y0` (offset), `A` (amplitude), `mu` (mean) |  | Code-based body. |
| GammaCDF | P2 | `y = y0 + A1*P(a, x/b), P = regularized lower incomplete gamma (gamma CDF with shape a, scale b); 0 for x<=0` | `y0` (offset), `A1` (amplitude), `a` (shape), `b` (scale) |  | Code-based body. |
| Gumbel | P2 | `1 - exp(-exp((x - a) / b))` | `a` (location), `b` (shape) |  |  |
| LognormalCDF | P2 | `y = y0 + A*Phi((ln(x) - xc)/w), Phi = standard normal CDF` | `y0` (offset), `A` (amplitude), `xc` (log mean), `w` (log standard deviation) |  | Code-based body. |
| Pareto2 | P2 | `1 - (b / x)^a` | `a` (cofficient), `b` (?) |  |  |
| Poisson | P2 | `y = y0 + exp(-r)*r^x/Gamma(x+1)   (x! generalised through the gamma function)` | `y0` (offset), `r` (mean) |  | Code-based body. |
| Rayleigh | P2 | `1 - exp(-(x / b)^2 / 2)` | `b` (scale) |  |  |
| Weibull | P2 | `y = y0 + (r/a)*((x-u)/a)^(r-1)*exp(-((x-u)/a)^r)` | `y0` (y offset), `a` (scale), `r` (shape), `u` (location) |  | Code-based body. |
| WeibullCDF | P2 | `y = y0 if x <= 0; y = y0 + A1*(1 - exp(-(x/a)^b)) if x > 0   [CDF form inferred: a = scale, b = shape]` | `y0` (offset), `A1` (amplitude), `a` (scale), `b` (shape) |  | Code-based body. |

## Enzyme Kinetics (5)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| CompInhib | P2 | `y = Vmax*x/(Km*(1+Ic/Ki) + x)   [Ic is a fixed, user-supplied inhibitor concentration]` | `Vmax` (maximum enzyme velocity), `Km` (michaelis-menten constant), `Ki` (inhibitor constant), `Ic` (inhibitor concentration) |  | Code-based body. |
| MixedModelInhib | P2 | `y = Vmax_app*x/(Km_app + x), Vmax_app = Vmax/(1+Ic/(Alpha*Ki)), Km_app = Km*(1+Ic/Ki)/(1+Ic/(Alpha*Ki))` | `Vmax` (maximum enzyme velocity), `Km` (michaelis-menten constant), `Ki` (inhibition constant), `Alpha` (constant that determines mechanism), `Ic` (inhibitor concentration) |  | Code-based body. |
| NoncompInhib | P2 | `y = Vmax/(1+Ic/Ki) * x/(Km + x)` | `Vmax` (maximum enzyme velocity), `Km` (michaelis-menten constant), `Ki` (inhibitor constant), `Ic` (inhibitor concentration) |  | Code-based body. |
| SubstrateInhib | P2 | `Vmax * x / (Km + x * (1 + x / Ki))` | `Vmax` (maximum enzyme activity), `Km` (michaelis-menten constant for enzyme activity), `Ki` (dissociation constant) |  |  |
| UncompInhib | P2 | `y = Vmax/(1+Ic/Kia) * x/(Km/(1+Ic/Kia) + x)` | `Vmax` (maximum enzyme velocity), `Km` (michaelis-menten constant), `Kia` (inhibitor constant), `Ic` (inhibitor concentration) |  | Code-based body. |

## Pharmacology (5)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Biphasic | P2 | `Amin + (Amax1 - Amin) / (1 + 10^((x - x0_1) * h1)) + (Amax2 - Amin) / (1 + 10^((x0_2 - x) * h2))` | `Amin` (bottom), `Amax1` (first top asymptote), `Amax2` (second top asymptote), `x0_1` (first median), `x0_2` (second median), `h1` (first slope), `h2` (second slope) |  |  |
| OneSiteBind | P2 | `Bmax * x / (k1 + x)` | `Bmax` (top asymptote), `k1` (median) |  |  |
| OneSiteComp | P2 | `A2 + (A1 - A2) / (1 + 10^(x - logx0))` | `A1` (top asymptote), `A2` (bottom asymptote), `logx0` (center) |  |  |
| TwoSiteBind | P2 | `Bmax1 * x / (k1 + x) + Bmax2 * x / (k2 + x)` | `Bmax1` (first top asymptote), `Bmax2` (second top asymptote), `k1` (first median), `k2` (second median) |  |  |
| TwoSiteComp | P2 | `A2 + (A1 - A2) * (fraction / (1 + 10^(x - logx0_1)) + (1 - fraction) / (1 + 10^(x - logx0_2)))` | `A1` (top asymptote), `A2` (bottom asymptote), `logx0_1` (first center), `logx0_2` (second center), `fraction` (fraction) |  |  |

## Rheology (6)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Bingham | P2 | `y0 + A * x` | `y0` (yield stress), `A` (plastic viscosity) |  |  |
| Carreau | P2 | `y = A2 + (A1-A2)*(1+(t*x)^a)^((n-1)/a)` | `A1` (zero shear viscosity), `A2` (infinite shear viscosity), `t` (time constant), `a` (transition control factor), `n` (power index) |  | Code-based body. |
| Cross | P2 | `y = A2 + (A1-A2)/(1+(t*x)^m)` | `A1` (zero shear viscosity), `A2` (infinite shear viscosity), `t` (time constant), `m` (power index) |  | Code-based body. |
| Herschel | P2 | `y0 + K * x^n` | `y0` (yield stress), `K` (consistency), `n` (flow index) |  |  |
| MYEGA | P2 | `y0 * 10^(K / x * exp(C / x))` | `y0` (infinite temperature viscosity), `K` (amplitude), `C` (rate constant) |  |  |
| VFT | P2 | `10^(A + B / (x - x0))` | `A` (bottom asymptote), `B` (numerator constant), `x0` (x offset) |  |  |

## Electrophysiology (1)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Goldman | P2 | `y = R*T*ln((x + b*Nao)/(Ki + b*Nai))/F, with constants R = 8.314 and F = 96485 (x plays the role of the outside potassium concentration)` | `b` (relative permeabilities of na to k), `Nao` (concentraion of na ions outside), `Nai` (concentraion of na ions inside), `Ki` (concentraion of k ions inside), `T` (temperature (in kelvin)) |  | Code-based body. |

## Multiple Variables (3)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| GaussianLorentz | P2 | `[x -> y1,y2] y1 = y0 + A1/(w1*sqrt(pi/2))*exp(-2*((x-xc)/w1)^2);  y2 = y0 + 2*A2/pi * w2/(4*(x-xc)^2 + w2^2)   (two dependent variables sharing parameters y0, xc)` | `y0` (offset), `xc` (center), `A1` (area), `A2` (area), `w1` (width), `w2` (width) |  | Code-based body. |
| HillBurk | P2 | `[x1,x2 -> v1,v2] v1 = Vm1*x1/(Km1 + x1);  v2 = (1 + Km2*x2)/Vm2   (two independent and two dependent variables; global fit)` | `Vm1` (?), `Km1` (?), `Vm2` (?), `Km2` (?) |  | Code-based body. |
| LineExp | P2 | `[x -> y1,y2] y1 = a1 + b1*x;  y2 = a2 + b2*exp(x/c)   (two dependent variables; global fit)` | `a1` (?), `a2` (?), `b1` (?), `b2` (?), `c` (?) |  | Code-based body. |

## Asymptotic (6)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Asymptotic2 | P1 | `a - b * exp(-k * x)` | `a` (asymptote), `b` (response range), `k` (rate) |  | Approach to a plateau a from below by a decaying exponential, a - b*exp(-k*x); k is a rate. |
| Asymptotic3 | P2 | `a * (1 - exp(-k * (x - xc)))` | `a` (asymptote), `xc` (rate), `k` (origin) |  |  |
| Asymptotic4 | P2 | `a + (I - a) * b^x` | `a` (asymptote), `b` (rate), `I` (intercept) |  |  |
| Asymptotic5 | P2 | `a + (I - a) * exp(-k * x)` | `a` (asymptote), `I` (intercept), `k` (rate) |  |  |
| Asymptotic6 | P2 | `a - exp(-(b + k * x))` | `a` (asymptote), `b` (response range), `k` (rate) |  |  |
| Asymptotic7 | P2 | `I + b * (1 - exp(-k * x))` | `I` (intercept), `b` (response range), `k` (rate) |  |  |

## Others (2)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| MWChang | P2 | `y = Vmax*( k1*x/(1+k1*x) + L1*k2*x/(1+k2*x) )/(L1 + 1), with L1 = ( L*(1+k1*x)/(1+k2*x) )^q` | `Vmax` (coefficient), `k1` (coefficient), `k2` (coefficient), `L` (coefficient), `q` (power) |  | Code-based body. |
| Richards | P2 | `a * (1 + exp(-(l + k * x) / d))^d` | `a` (coefficient), `l` (unknown), `k` (unknown), `d` (unknown) |  |  |

## Engineering (1)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| Pulse | P2 | `y = 0 if x < x0; y = y0 + A*(1-exp(-(x-x0)/t1))^P*exp(-(x-x0)/t2) if x >= x0  (note: the region x<x0 is exactly 0, not y0)` | `y0` (y offset), `x0` (x offset), `A` (amplitude), `t1` (width), `P` (power), `t2` (width) | yes | Code-based body. |

## PFW (7)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| BWF | P2 | `y0 + H * (1 + (x - xc) / (q * w))^2 / (1 + ((x - xc) / w)^2)` | `y0` (base), `xc` (center), `H` (height), `w` (width), `q` (asym) | yes |  |
| ConsGaussian | P2 | `y0 + A * exp(-0.5 * ((x - xc) / (xc * w2 + w1))^2) / sqrt(2 * pi) / (xc * w2 + w1)` | `y0` (baseline), `xc` (center), `A` (area), `w1` (width), `w2` (width) | yes |  |
| DoniachSunjic | P2 | `y0 + H * cos(pi * a / 2 + (1 - a) * atan((x - xc) / w)) / (w^2 + (x - xc)^2)^((1 - a) / 2)` | `y0` (offset), `xc` (center), `H` (height), `w` (fwhm), `a` (asymmetry) | yes |  |
| FraserSuzuki | P2 | `(native routine, no formula in file)` | `y0` (base), `xc` (center), `A` (area), `sig` (unknow) | yes | Source defect: source code is internally inconsistent (parameter A is used as both amplitude and asymmetry, and the branch tests x<0 instead of x<xc); do not implement from this file, use the published Fraser-Suzuki form instead. Code-based body. |
| Gaussian_LorenCross | P2 | `y0 + A / (1 + s * ((x - xc) / w)^2 * exp(0.5 * (1 - s) * ((x - xc) / w)^2))` | `y0` (base), `xc` (center), `A` (amplitude), `w` (width), `s` (shape) | yes |  |
| HVL | P2 | `y0 + A * w * exp(-0.5 * ((x - xc) / w)^2) / (xc * d * sqrt(2 * pi)) / (1 / exp(xc * d / w / w - 1) + 0.5 * (1 + erf((x - xc) / w / sqrt(2))))` | `y0` (base), `xc` (center), `A` (area), `w` (width), `d` (distortion) | yes |  |
| SchulzFlory | P2 | `y0 + A * exp((xc - x) / w) * (x / xc)^(xc / w)` | `y0` (base), `xc` (center), `w` (width), `A` (amplitude) | yes |  |

## Uncategorized (23)

| Function | Pri | Formula | Parameters | Peak | Notes |
|---|---|---|---|---|---|
| AsymmetricGaussian | P2 | `y = y0 + A*exp(-0.5*((x-xc)/w1)^2) if x < xc; y = y0 + A*exp(-0.5*((x-xc)/w2)^2) if x >= xc` | `y0` (offset), `xc` (center), `w1` (width), `w2` (width), `A` (amplitude) | yes | Code-based body. |
| BatesWatts | P2 | `y = p1*p3*x/(1 + p1*x + p2*x^2)   [source text writes "x2"; the single independent variable means x^2 is the only sensible reading - verify]` | `p1` (unknown), `p2` (unknown), `p3` (unknown) |  | Source defect: source text uses "x2". Code-based body. |
| BentHyperblSmp | P2 | `a * (1 + b * (x + c) - (1 + (b * (x + c))^d)^(1 / d))` | `a` (coefficient), `b` (coefficient), `c` (coefficient), `d` (coefficient) |  |  |
| Cole | P2 | `[x_1,x_2 -> y] y = x_1^a * (b + c*x_2)   [source text calls the variables x1 and x2 but declares them x_1 and x_2]` | `a` (power), `b` (offset), `c` (coefficient) |  | Source defect: variable names differ between declaration and formula. Code-based body. |
| DCPDisso | P2 | `(native routine, no formula in file)` | `BL0` (?), `BL1` (?), `n` (?), `Tm` (?), `Hm` (?), `dCp` (?), `Ct` (?) |  | External DLL function; formula not in file. |
| dCpM2ST | P2 | `(native routine, no formula in file)` | `BL0` (?), `BL1` (?), `Tm` (?), `H` (?), `dCp` (?) |  | External DLL function; formula not in file. |
| ExpGrow | P2 | `y0 + A1 * exp((x - x0) / t1) + A2 * exp((x - x0) / t2)` | `y0` (offset), `x0` (center), `A1` (amplitude), `t1` (width), `A2` (amplitude), `t2` (width) |  |  |
| ExpGrowth | P2 | `exp(k * (x - x0))` | `x0` (position), `k` (growth constant) |  |  |
| Freundlich | P2 | `a * x^b` | `a` (amplitude), `b` (width) |  |  |
| Freundlich3P | P2 | `a + b * x^c` | `a` (coefficient), `b` (coefficient), `c` (power) |  |  |
| Freundlich3PMod | P2 | `y = a + b*x^(1/k)   [source text uses the letter c where the parameter is called k - assumed typo]` | `a` (offset), `b` (coefficient), `k` (cofficient) |  | Source defect: formula uses c, parameter is k. Code-based body. |
| FreundlichMod1 | P2 | `a * x^(1 / b)` | `a` (amplitude), `b` (width) |  |  |
| GaussLog | P2 | `[x,y -> z] z = z0 + B*exp(-0.5*( ((x-C)/D)^2 + (ln(y/E)/F)^2 ))   [source text is Mathematica-style with mismatched letter case; parameters are z0,B,C,D,E,F]` | `z0` (?), `B` (?), `C` (?), `D` (?), `E` (?), `F` (?) |  | Source defect: source formula text is garbled. Code-based body. |
| Lanczos3 | P2 | `b1 * exp(-b2 * x) + b3 * exp(-b4 * x) + b5 * exp(-b6 * x)` | `b1` (?), `b2` (?), `b3` (?), `b4` (?), `b5` (?), `b6` (?) |  |  |
| LogisticPeak | P2 | `[x,y -> z] z = z0 + B*y + 4*C*exp(-(x-D)/E)/(1+exp(-(x-D)/E))^2   [source text is garbled (Mathematica-style, juxtaposed products); this is the evident intent - verify]` | `z0` (?), `B` (?), `C` (?), `D` (?), `E` (?) |  | Source defect: source formula text is garbled. Code-based body. |
| M2State | P2 | `(native routine, no formula in file)` | `Tm` (?), `H` (?) |  | External DLL function; formula not in file. |
| MN2State | P2 | `(native routine, no formula in file)` | `Tm` (?), `H` (?), `Hv` (?) |  | External DLL function; formula not in file. |
| Normal | P2 | `y0 + exp(-(x - mu)^2 / (2 * s^2)) / (s * sqrt(2 * pi))` | `mu` (?), `s` (?), `y0` (?) |  |  |
| PolyCtr5 | P2 | `y = y0 + p1*(x-x0) + p2*(x-x0)^2 + p3*(x-x0)^3 + p4*(x-x0)^4 + p5*(x-x0)^5   [source text abbreviates the middle terms with "..."]` | `y0` (offset), `x0` (center), `p1` (coefficient), `p2` (coefficient), `p3` (coefficient), `p4` (coefficient), `p5` (coefficient) |  | Source defect: source formula contains ellipsis. Code-based body. |
| Pulse1 | P2 | `y = 0 if x < xc; y = A*(1-exp(-(x-xc)/t1))^P*exp(-(x-xc)/t2) if x >= xc   [source declares the first parameter as x0 but the body uses xc: same quantity]` | `x0` (center), `A` (amplitude), `t1` (tmie constant), `t2` (time constant), `P` (power) |  | Source defect: formula uses xc, parameter is x0. Code-based body. |
| Reich | P2 | `a1 * exp(-b1 * x) + a2 * exp(-b2 * x)` | `a1` (coefficient), `b1` (rate), `a2` (coefficient), `b2` (rate) |  |  |
| SLogistic4 | P2 | `b + (a - b) / (1 + exp(-k * (x - xc)))` | `a` (amplitude), `b` (coefficient), `xc` (position), `k` (rate) |  |  |
| TimePow | P2 | `a * x^b` | `a` (amplitude), `b` (width) |  |  |
