//! The built-in fit functions: names, categories, parameters with their default bounds, formulas
//! and routines that guess starting values from the data. See `docs/spec/fitting.md`.
//!
//! The table is `data/functions.json`, trimmed from `docs/spec/data/fit-functions.json` (names,
//! parameter names, bounds, formulas). Most formulas are plain expressions; a few that Origin
//! writes with branches use the conditional operator (`x < xc ? a : b`), and two (Voigt and
//! GammaCDF) are computed by Rust code. Functions with several independent variables, external
//! routines or source text that cannot be trusted are not in the table.

use std::sync::{Arc, OnceLock};

use nalgebra::Complex;
use serde::Deserialize;
use sheaf_expr::{Expr, SimpleHost};

use crate::nlsf::{FitFunction, InitFn, Model, ModelFn, ParamSpec};
use crate::report::Parameter;

#[derive(Debug, Clone, Deserialize)]
struct RawParam {
    name: String,
    lower: Option<f64>,
    upper: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawDerived {
    name: String,
    formula: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawEntry {
    name: String,
    category: String,
    priority: String,
    formula: Option<String>,
    params: Vec<RawParam>,
    #[serde(default)]
    derived: Vec<RawDerived>,
}

/// What the library knows about one function, without building it.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionInfo {
    pub name: String,
    /// Category in Origin's function dialog, for example `Peak Functions`.
    pub category: String,
    /// `P0` for the functions used daily, `P1` common, `P2` the rest.
    pub priority: String,
    pub params: Vec<String>,
    /// The formula, or `None` for the two functions computed by Rust code.
    pub formula: Option<String>,
    /// Derived quantities such as `FWHM`, as `(name, formula)` over the fitted parameters.
    pub derived: Vec<(String, String)>,
}

fn entries() -> &'static Vec<RawEntry> {
    static E: OnceLock<Vec<RawEntry>> = OnceLock::new();
    E.get_or_init(|| {
        serde_json::from_str(include_str!("../data/functions.json")).expect("data/functions.json is valid")
    })
}

/// Every function in the library, in the order of the table.
pub fn list() -> Vec<FunctionInfo> {
    entries()
        .iter()
        .filter(|e| e.formula.is_some() || closure_for(&e.name).is_some())
        .map(|e| FunctionInfo {
            name: e.name.clone(),
            category: e.category.clone(),
            priority: e.priority.clone(),
            params: e.params.iter().map(|p| p.name.clone()).collect(),
            formula: e.formula.clone(),
            derived: e.derived.iter().map(|d| (d.name.clone(), d.formula.clone())).collect(),
        })
        .collect()
}

/// The built-in function with this name (case does not matter), ready to fit: bounds that are
/// switched on by default are set, and the starting-value routine is attached where there is one.
pub fn get(name: &str) -> Option<FitFunction> {
    let Some(e) = entries().iter().find(|e| e.name.eq_ignore_ascii_case(name)) else {
        // Surface functions z = f(x, y) are kept in their own table and not listed with these.
        return crate::surface::get(name);
    };
    let params: Vec<ParamSpec> = e
        .params
        .iter()
        .map(|p| ParamSpec { name: p.name.clone(), initial: f64::NAN, lower: p.lower, upper: p.upper, fixed: false })
        .collect();
    let (model, equation) = match (&e.formula, closure_for(&e.name)) {
        (Some(f), _) => (Model::Formula(Expr::parse(f).ok()?), format!("y = {f}")),
        (None, Some((f, eq))) => (Model::Closure(f), eq.to_string()),
        (None, None) => return None,
    };
    Some(FitFunction {
        name: e.name.clone(),
        equation,
        x_name: "x".to_string(),
        params,
        model,
        init: init_for(&e.name),
        x2_name: None,
        init2: None,
    })
}

/// Values of a function's derived quantities (FWHM, half-life, ...) for fitted parameters.
pub fn derived_values(name: &str, params: &[Parameter]) -> Vec<(String, f64)> {
    let Some(e) = entries().iter().find(|e| e.name.eq_ignore_ascii_case(name)) else {
        return Vec::new();
    };
    let host = SimpleHost::from_vars(params.iter().map(|p| (p.name.as_str(), p.value)));
    e.derived
        .iter()
        .filter_map(|d| {
            let v = Expr::parse(&d.formula).ok()?.eval(&host).ok()?;
            Some((d.name.clone(), v))
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Functions computed by Rust code
// ---------------------------------------------------------------------------------------------

fn closure_for(name: &str) -> Option<(ModelFn, &'static str)> {
    match name {
        "Voigt" => Some((
            Arc::new(|p: &[f64], x: f64| voigt(x, p[0], p[1], p[2], p[3], p[4])),
            "y = y0 + A*(2*sqrt(ln2)/(sqrt(pi)*wG)) * Re w(u + i*a), u = 2*sqrt(ln2)*(x-xc)/wG, a = sqrt(ln2)*wL/wG (area A; wG and wL are the Gaussian and Lorentzian FWHM)",
        )),
        "GammaCDF" => Some((
            Arc::new(|p: &[f64], x: f64| {
                if x <= 0.0 || p[2] <= 0.0 || p[3] <= 0.0 {
                    p[0]
                } else {
                    p[0] + p[1] * statrs::function::gamma::gamma_lr(p[2], x / p[3])
                }
            }),
            "y = y0 + A1*P(a, x/b), P = regularized lower incomplete gamma function",
        )),
        _ => None,
    }
}

/// Voigt profile with area `area`, Gaussian FWHM `wg` and Lorentzian FWHM `wl`, plus an offset.
pub fn voigt(x: f64, y0: f64, xc: f64, area: f64, wg: f64, wl: f64) -> f64 {
    if wg.is_nan() || wg <= 0.0 || wl.is_nan() || wl < 0.0 {
        return f64::NAN;
    }
    let ln2s = std::f64::consts::LN_2.sqrt();
    let u = 2.0 * ln2s * (x - xc) / wg;
    let a = ln2s * wl / wg;
    y0 + area * (2.0 * ln2s / (std::f64::consts::PI.sqrt() * wg)) * faddeeva(Complex::new(u, a)).re
}

/// The Faddeeva function `w(z) = exp(-z^2) erfc(-i z)` for `Im z >= 0`, by Weideman's rational
/// approximation (40 terms; about 1e-13 accurate) and a short asymptotic series far from zero.
pub fn faddeeva(z: Complex<f64>) -> Complex<f64> {
    use std::f64::consts::PI;
    if z.norm_sqr() > 900.0 {
        let zi = Complex::new(1.0, 0.0) / z;
        let z2 = zi * zi;
        let series = Complex::new(1.0, 0.0) + z2 * (0.5 + z2 * (0.75 + z2 * (1.875 + z2 * 6.5625)));
        return Complex::new(0.0, 1.0 / PI.sqrt()) * zi * series;
    }
    let (l, coef) = weideman_coefficients();
    let il = Complex::new(0.0, 1.0);
    let lc = Complex::new(*l, 0.0);
    let zz = (lc + il * z) / (lc - il * z);
    // Horner evaluation of sum_n coef[n] * zz^n.
    let mut p = Complex::new(0.0, 0.0);
    for c in coef.iter().rev() {
        p = p * zz + Complex::new(*c, 0.0);
    }
    let d = lc - il * z;
    p * 2.0 / (d * d) + Complex::new(1.0 / PI.sqrt(), 0.0) / d
}

const WEIDEMAN_N: usize = 40;

fn weideman_coefficients() -> &'static (f64, Vec<f64>) {
    static C: OnceLock<(f64, Vec<f64>)> = OnceLock::new();
    C.get_or_init(|| {
        let n = WEIDEMAN_N;
        let m = 2 * n;
        let m2 = 2 * m;
        let l = (n as f64 / 2f64.sqrt()).sqrt();
        // f at the sample points theta_k = k*pi/M, k = -M+1 .. M-1, with a leading zero.
        let mut f = vec![0.0; m2];
        for (idx, fk) in f.iter_mut().enumerate().skip(1) {
            let k = idx as f64 - m as f64;
            let theta = k * std::f64::consts::PI / m as f64;
            let t = l * (theta / 2.0).tan();
            *fk = (-t * t).exp() * (l * l + t * t);
        }
        // Real part of the discrete Fourier transform of the half-shifted sequence.
        let mut coef = Vec::with_capacity(n);
        for nn in 1..=n {
            let mut s = 0.0;
            for (idx, fk) in f.iter().enumerate() {
                let j = idx as f64 - m as f64;
                s += fk * (-2.0 * std::f64::consts::PI * j * nn as f64 / m2 as f64).cos();
            }
            coef.push(s / m2 as f64);
        }
        (l, coef)
    })
}

// ---------------------------------------------------------------------------------------------
// Starting values
// ---------------------------------------------------------------------------------------------

/// Sorts the points by x.
fn sorted(x: &[f64], y: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut idx: Vec<usize> = (0..x.len()).collect();
    idx.sort_by(|&a, &b| x[a].total_cmp(&x[b]));
    (idx.iter().map(|&i| x[i]).collect(), idx.iter().map(|&i| y[i]).collect())
}

fn min_max(v: &[f64]) -> (f64, f64) {
    v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &a| (lo.min(a), hi.max(a)))
}

/// Straight line through the points by least squares: `(intercept, slope)`.
fn line_fit(x: &[f64], y: &[f64]) -> (f64, f64) {
    let n = x.len() as f64;
    let sx: f64 = x.iter().sum();
    let sy: f64 = y.iter().sum();
    let sxx: f64 = x.iter().map(|a| a * a).sum();
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| a * b).sum();
    let d = n * sxx - sx * sx;
    if d.abs() < 1e-300 {
        return (sy / n.max(1.0), 0.0);
    }
    let slope = (n * sxy - sx * sy) / d;
    ((sy - slope * sx) / n, slope)
}

/// Polynomial coefficients (ascending powers) of the given degree by least squares.
fn poly_fit(x: &[f64], y: &[f64], degree: usize) -> Vec<f64> {
    use nalgebra::{DMatrix, DVector};
    let k = degree + 1;
    if x.len() < k {
        return vec![f64::NAN; k];
    }
    // Scale x to keep the normal equations reasonable.
    let (lo, hi) = min_max(x);
    let c = 0.5 * (lo + hi);
    let s = (0.5 * (hi - lo)).max(1e-300);
    let a = DMatrix::from_fn(x.len(), k, |i, j| ((x[i] - c) / s).powi(j as i32));
    let b = DVector::from_column_slice(y);
    let Some(sol) = a.svd(true, true).solve(&b, 1e-12).ok() else {
        return vec![f64::NAN; k];
    };
    // Convert back from the scaled variable: sum_j sol_j ((x-c)/s)^j.
    let mut out = vec![0.0; k];
    for (j, &sj) in sol.iter().enumerate() {
        // ((x-c)/s)^j = s^-j * sum_i C(j,i) x^i (-c)^(j-i)
        let mut binom = 1.0;
        for (i, o) in out.iter_mut().enumerate().take(j + 1) {
            *o += sj * s.powi(-(j as i32)) * binom * (-c).powi((j - i) as i32);
            binom = binom * (j - i) as f64 / (i + 1) as f64;
        }
    }
    out
}

/// What the data say about a single peak on a flat baseline.
struct PeakGuess {
    xc: f64,
    /// Full width at half height.
    fwhm: f64,
    baseline: f64,
    /// Signed height above the baseline (negative for a downward peak).
    height: f64,
    /// Signed area between the curve and the baseline.
    area: f64,
}

fn peak_guess(x: &[f64], y: &[f64]) -> PeakGuess {
    let (x, y) = sorted(x, y);
    let n = x.len();
    let (ymin, ymax) = min_max(&y);
    let mut sorted_y = y.clone();
    sorted_y.sort_by(f64::total_cmp);
    let median = sorted_y[n / 2];
    let up = (ymax - median) >= (median - ymin);
    let baseline = if up { ymin } else { ymax };
    let ipk = (0..n).max_by(|&a, &b| ((y[a] - baseline).abs()).total_cmp(&(y[b] - baseline).abs())).unwrap_or(0);
    let height = y[ipk] - baseline;
    let half = baseline + height / 2.0;
    // Walk outwards from the peak to the first point below half height on each side, and
    // interpolate where the curve crosses half height.
    let crossing = |dir: isize| -> Option<f64> {
        let mut i = ipk as isize;
        loop {
            let j = i + dir;
            if j < 0 || j >= n as isize {
                return None;
            }
            let (yi, yj) = (y[i as usize], y[j as usize]);
            let below = |v: f64| if height >= 0.0 { v < half } else { v > half };
            if below(yj) {
                let t = (half - yi) / (yj - yi);
                return Some(x[i as usize] + t * (x[j as usize] - x[i as usize]));
            }
            i = j;
        }
    };
    let span = x[n - 1] - x[0];
    let fwhm = match (crossing(-1), crossing(1)) {
        (Some(a), Some(b)) => b - a,
        (Some(a), None) => 2.0 * (x[ipk] - a),
        (None, Some(b)) => 2.0 * (b - x[ipk]),
        (None, None) => span / 4.0,
    };
    let fwhm = if fwhm.is_finite() && fwhm > 0.0 { fwhm } else { span / 4.0 };
    let area: f64 = (1..n).map(|i| 0.5 * (x[i] - x[i - 1]) * ((y[i] - baseline) + (y[i - 1] - baseline))).sum();
    PeakGuess { xc: x[ipk], fwhm, baseline, height, area }
}

/// Fits `y = y0 + A exp(m x)` the way a quick estimate does: the asymptote `y0` is put just
/// beyond the end of the data where the curve flattens, then `ln|y - y0|` is fitted by a straight
/// line. Returns `(y0, A, m)`. `decay` says which end flattens (the last point for a decay).
fn exponential_guess(x: &[f64], y: &[f64], decay: bool) -> (f64, f64, f64) {
    let (x, y) = sorted(x, y);
    let n = x.len();
    let (first, last) = (y[0], y[n - 1]);
    let (flat, other) = if decay { (last, first) } else { (first, last) };
    let y0 = flat - 0.02 * (other - flat);
    let mut xs = Vec::new();
    let mut ls = Vec::new();
    for i in 0..n {
        let d = (y[i] - y0).abs();
        if d > 0.0 && d.is_finite() {
            xs.push(x[i]);
            ls.push(d.ln());
        }
    }
    if xs.len() < 2 {
        return (y0, 1.0, if decay { -1.0 } else { 1.0 });
    }
    let (b, m) = line_fit(&xs, &ls);
    let sign = if other - y0 >= 0.0 { 1.0 } else { -1.0 };
    (y0, sign * b.exp(), m)
}

/// Level of x where y crosses the middle between its smallest and largest value.
fn x_at_half(x: &[f64], y: &[f64]) -> f64 {
    x_at_fraction(x, y, 0.5)
}

/// Level of x where y first crosses `lo + fraction * (hi - lo)`, with `lo` and `hi` the smallest and
/// largest y; the middle of the x range if it never does.
fn x_at_fraction(x: &[f64], y: &[f64], fraction: f64) -> f64 {
    let (x, y) = sorted(x, y);
    let (lo, hi) = min_max(&y);
    x_at_level(&x, &y, lo + fraction * (hi - lo))
}

/// Level of x where the (x-sorted) curve first crosses the y level `mid`.
fn x_at_level(x: &[f64], y: &[f64], mid: f64) -> f64 {
    for i in 1..x.len() {
        let (a, b) = (y[i - 1] - mid, y[i] - mid);
        if a == 0.0 {
            return x[i - 1];
        }
        if a * b < 0.0 {
            return x[i - 1] + (x[i] - x[i - 1]) * a / (a - b);
        }
    }
    0.5 * (x[0] + x[x.len() - 1])
}

/// Width in x between the 25 % and 75 % levels of a sigmoid (positive); zero if it is not defined.
fn quartile_width(x: &[f64], y: &[f64]) -> f64 {
    (x_at_fraction(x, y, 0.75) - x_at_fraction(x, y, 0.25)).abs()
}

/// Steepness `k` of a logistic curve `1 / (1 + exp(-k (x - xc)))` from the width between its
/// quarter and three-quarter levels (which is `2 ln 3 / k`), positive when y rises with x.
fn logistic_k(x: &[f64], y: &[f64]) -> f64 {
    let w = quartile_width(x, y);
    let (_, ys) = sorted(x, y);
    let k = if w > 0.0 { 2.0 * 3f64.ln() / w } else { 1.0 };
    if ys[0] > ys[ys.len() - 1] { -k } else { k }
}

/// Starting values of `V x / (K + x)`: the straight line of `1/y` against `1/x` (Lineweaver-Burk),
/// kept only if it gives a positive `V` and `K`; otherwise the largest y and the half-way x.
fn hyperbola_guess(x: &[f64], y: &[f64]) -> (f64, f64) {
    let (ix, iy): (Vec<f64>, Vec<f64>) =
        x.iter().zip(y).filter(|(a, b)| **a > 0.0 && **b > 0.0).map(|(a, b)| (1.0 / a, 1.0 / b)).unzip();
    if ix.len() >= 2 {
        let (c, m) = line_fit(&ix, &iy);
        if c > 0.0 && m > 0.0 && c.is_finite() && m.is_finite() {
            return (1.0 / c, m / c);
        }
    }
    let (_, hi) = min_max(y);
    (hi, x_at_half(x, y).abs().max(1e-300))
}

/// Smallest and largest-x end levels: `(low-x end, high-x end)` of y, taken as the extreme values
/// in the direction of the overall trend.
fn sigmoid_levels(x: &[f64], y: &[f64]) -> (f64, f64) {
    let (x, y) = sorted(x, y);
    let n = x.len();
    let (lo, hi) = min_max(&y);
    let _ = x;
    if y[0] > y[n - 1] { (hi, lo) } else { (lo, hi) }
}

/// Coefficients (ascending powers, `degree + 1` of them) of a least-squares polynomial, or the
/// lower-order fit padded with zeros when there are too few points.
fn poly_guess(x: &[f64], y: &[f64], degree: usize) -> Vec<f64> {
    let d = degree.min(x.len().saturating_sub(1));
    let mut c = poly_fit(x, y, d);
    c.resize(degree + 1, 0.0);
    c
}

fn init_for(name: &str) -> Option<InitFn> {
    let f: InitFn = match name {
        "Line" => Arc::new(|x, y| {
            let (a, b) = line_fit(x, y);
            vec![a, b]
        }),
        "Parabola" => Arc::new(|x, y| poly_fit(x, y, 2)),
        "Cubic" => Arc::new(|x, y| poly_fit(x, y, 3)),
        "Poly" => Arc::new(|x, y| {
            let mut c = poly_fit(x, y, 2.min(x.len().saturating_sub(1)));
            c.resize(10, 0.0);
            c
        }),
        "Allometric1" => Arc::new(|x, y| {
            let (lx, ly): (Vec<f64>, Vec<f64>) =
                x.iter().zip(y).filter(|(a, b)| **a > 0.0 && **b > 0.0).map(|(a, b)| (a.ln(), b.ln())).unzip();
            if lx.len() < 2 {
                return vec![1.0, 1.0];
            }
            let (a, b) = line_fit(&lx, &ly);
            vec![a.exp(), b]
        }),
        "Power" => Arc::new(|x, y| {
            let (mut num, mut den) = (0.0, 0.0);
            for (a, b) in x.iter().zip(y) {
                if *a > 0.0 && *b > 0.0 {
                    num += a.ln() * b.ln();
                    den += a.ln() * a.ln();
                }
            }
            vec![if den > 0.0 { num / den } else { 1.0 }]
        }),
        "Gauss" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            let w = 0.85 * g.fwhm;
            vec![g.baseline, g.xc, w, g.height * w * (std::f64::consts::PI / 2.0).sqrt()]
        }),
        "GaussAmp" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            vec![g.baseline, g.xc, g.fwhm / 2.354_820_045, g.height]
        }),
        "Gaussian" => Arc::new(|x, y| {
            // Parameters are (y0, xc, A, w) with A the area and w the FWHM.
            let g = peak_guess(x, y);
            vec![g.baseline, g.xc, g.area, g.fwhm]
        }),
        "Lorentz" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            vec![g.baseline, g.xc, g.fwhm, std::f64::consts::FRAC_PI_2 * g.height * g.fwhm]
        }),
        "Voigt" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            let wg = 0.85 * g.fwhm;
            vec![g.baseline, g.xc, g.area, wg, wg]
        }),
        "PsdVoigt1" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            vec![g.baseline, g.xc, g.area, g.fwhm, 0.5]
        }),
        "Bigaussian" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            let w = g.fwhm / 2.354_820_045;
            vec![g.baseline, g.xc, g.height, w, w]
        }),
        "ExpDec1" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, true);
            vec![y0, a, -1.0 / m]
        }),
        "ExpDec2" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, true);
            let t = -1.0 / m;
            vec![y0, a / 2.0, 0.9 * t, a / 2.0, 1.1 * t]
        }),
        "ExpDec3" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, true);
            let t = -1.0 / m;
            vec![y0, a / 3.0, 0.8 * t, a / 3.0, t, a / 3.0, 1.2 * t]
        }),
        "ExpGro1" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, false);
            vec![y0, a, 1.0 / m]
        }),
        "ExpGrow1" => Arc::new(|x, y| {
            // Parameters y0, x0, A1, t1. The offset x0 and the amplitude A1 are interchangeable,
            // so x0 starts at zero; hold it fixed if it should stay there.
            let (y0, a, m) = exponential_guess(x, y, false);
            vec![y0, 0.0, a, 1.0 / m]
        }),
        "Exponential" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, false);
            vec![y0, a, m]
        }),
        "Asymptotic1" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, true);
            vec![y0, -a, m.exp()]
        }),
        "ExpAssoc" => Arc::new(|x, y| {
            // Rise to a plateau: decay of the distance to the plateau.
            let (p, a, m) = exponential_guess(x, y, true);
            let t = -1.0 / m;
            // y = p + a exp(-x/t) = (p + a) - a (1 - exp(-x/t)).
            vec![p + a, -a / 2.0, t, -a / 2.0, t]
        }),
        "Boltzmann" => Arc::new(|x, y| {
            let (lo, hi) = sigmoid_levels(x, y);
            let (xl, xh) = min_max(x);
            vec![lo, hi, x_at_half(x, y), (xh - xl) / 20.0]
        }),
        "Logistic" => Arc::new(|x, y| {
            let (lo, hi) = sigmoid_levels(x, y);
            vec![lo, hi, x_at_half(x, y), 3.0]
        }),
        "Hill" => Arc::new(|x, y| {
            let (_, hi) = min_max(y);
            vec![hi, x_at_half(x, y), 1.0]
        }),
        "DoseResp" => Arc::new(|x, y| {
            let (lo, hi) = sigmoid_levels(x, y);
            vec![lo, hi, x_at_half(x, y), 1.0]
        }),
        "Sine" => Arc::new(sine_guess),
        "Poly4" => Arc::new(|x, y| poly_guess(x, y, 4)),
        "Poly5" => Arc::new(|x, y| poly_guess(x, y, 5)),
        "Constant" => Arc::new(|_, y| vec![y.iter().sum::<f64>() / y.len().max(1) as f64]),
        "LineMod" => Arc::new(|x, y| {
            // y = a (x - b): slope a and the x where the line crosses zero.
            let (c0, c1) = line_fit(x, y);
            if c1 != 0.0 && c1.is_finite() { vec![c1, -c0 / c1] } else { vec![0.0, 0.0] }
        }),
        "Power1" => Arc::new(|x, y| {
            // y = A |x - xc|^P with xc starting at zero.
            let (lx, ly): (Vec<f64>, Vec<f64>) =
                x.iter().zip(y).filter(|(a, b)| **a > 0.0 && **b > 0.0).map(|(a, b)| (a.ln(), b.ln())).unzip();
            if lx.len() < 2 {
                return vec![0.0, 1.0, 1.0];
            }
            let (a, b) = line_fit(&lx, &ly);
            vec![0.0, a.exp(), b]
        }),
        "PearsonVII" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            vec![g.baseline, g.xc, g.area, g.fwhm, 1.5]
        }),
        "PsdVoigt2" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            vec![g.baseline, g.xc, g.area, g.fwhm, g.fwhm, 0.5]
        }),
        "Asym2Sig" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            vec![g.baseline, g.xc, g.height, 0.6 * g.fwhm, 0.15 * g.fwhm, 0.15 * g.fwhm]
        }),
        "AsymmetricGaussian" => Arc::new(|x, y| {
            let g = peak_guess(x, y);
            let w = g.fwhm / 2.354_820_045;
            vec![g.baseline, g.xc, w, w, g.height]
        }),
        "Logistpk" => Arc::new(|x, y| {
            // Peak value A at xc; the full width at half height is 3.5255 w.
            let g = peak_guess(x, y);
            vec![g.baseline, g.xc, g.fwhm / 3.525_494, g.height]
        }),
        "LogNormal" => Arc::new(|x, y| {
            // The mode is xc exp(-w^2) and the relative width about 2.355 w.
            let g = peak_guess(x, y);
            let w = if g.xc > 0.0 { (g.fwhm / (2.354_820_045 * g.xc)).clamp(0.01, 2.0) } else { 0.3 };
            vec![g.baseline, g.xc.abs().max(1e-300) * (w * w).exp(), w, g.area]
        }),
        "ExpDecay1" => Arc::new(|x, y| {
            // y0, x0, A1, t1; x0 and A1 are interchangeable, so x0 starts at zero.
            let (y0, a, m) = exponential_guess(x, y, true);
            vec![y0, 0.0, a, -1.0 / m]
        }),
        "ExpDecay2" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, true);
            let t = -1.0 / m;
            vec![y0, 0.0, a / 2.0, 0.9 * t, a / 2.0, 1.1 * t]
        }),
        "ExpDecay3" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, true);
            let t = -1.0 / m;
            vec![y0, 0.0, a / 3.0, 0.8 * t, a / 3.0, t, a / 3.0, 1.2 * t]
        }),
        "ExpGro2" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, false);
            let t = 1.0 / m;
            vec![y0, a / 2.0, 0.9 * t, a / 2.0, 1.1 * t]
        }),
        "ExpGrow2" => Arc::new(|x, y| {
            let (y0, a, m) = exponential_guess(x, y, false);
            let t = 1.0 / m;
            vec![y0, 0.0, a / 2.0, 0.9 * t, a / 2.0, 1.1 * t]
        }),
        "Asymptotic2" => Arc::new(|x, y| {
            // y = a - b exp(-k x)
            let (y0, a, m) = exponential_guess(x, y, true);
            vec![y0, -a, -m]
        }),
        "BoxLucas1" => Arc::new(|x, y| {
            // y = a (1 - exp(-b x)): the plateau and the rate of the approach to it.
            let (y0, _, m) = exponential_guess(x, y, true);
            vec![y0, -m]
        }),
        "Exp2P" => Arc::new(|x, y| {
            // y = a b^x: a straight line through ln y.
            let (xs, ls): (Vec<f64>, Vec<f64>) =
                x.iter().zip(y).filter(|(_, b)| **b > 0.0).map(|(a, b)| (*a, b.ln())).unzip();
            if xs.len() < 2 {
                return vec![1.0, 1.0];
            }
            let (c0, c1) = line_fit(&xs, &ls);
            vec![c0.exp(), c1.exp()]
        }),
        "SLogistic1" => Arc::new(|x, y| {
            let (_, hi) = min_max(y);
            vec![hi, x_at_half(x, y), logistic_k(x, y)]
        }),
        "SLogistic4" => Arc::new(|x, y| {
            // y = b + (a - b) / (1 + exp(-k (x - xc))): b is the level at small x, a at large x.
            let (b, a) = sigmoid_levels(x, y);
            vec![a, b, x_at_half(x, y), logistic_k(x, y).abs()]
        }),
        "SGompertz" => Arc::new(|x, y| {
            // y = a exp(-exp(-k (x - xc))): a straight line through ln(-ln(y / a)) against x.
            let (_, hi) = min_max(y);
            let a = hi * 1.02;
            let (xs, ys) = sorted(x, y);
            let xc = x_at_level(&xs, &ys, a / std::f64::consts::E);
            let (px, pz): (Vec<f64>, Vec<f64>) = xs
                .iter()
                .zip(&ys)
                .filter(|(_, v)| **v > 0.05 * a && **v < 0.95 * a)
                .map(|(u, v)| (*u, (-(v / a).ln()).ln()))
                .unzip();
            let k = if px.len() >= 2 { -line_fit(&px, &pz).1 } else { 1.0 };
            vec![a, xc, if k > 0.0 && k.is_finite() { k } else { 1.0 }]
        }),
        "Hill1" => Arc::new(|x, y| {
            // START + (END - START) x^n / (k^n + x^n); n from the ratio of the 75 % and 25 % levels.
            let (start, end) = sigmoid_levels(x, y);
            let (x25, x75) = (x_at_fraction(x, y, 0.25), x_at_fraction(x, y, 0.75));
            let n = if x25 > 0.0 && x75 > x25 { (9f64.ln() / (x75 / x25).ln()).clamp(0.2, 20.0) } else { 1.0 };
            vec![start, end, x_at_half(x, y).abs().max(1e-300), n]
        }),
        "MichaelisMenten" | "OneSiteBind" | "Hyperbl" => Arc::new(|x, y| {
            let (v, k) = hyperbola_guess(x, y);
            vec![v, k]
        }),
        "RectHyperbola" => Arc::new(|x, y| {
            // y = a b x / (1 + b x): a is the plateau and 1/b the half-saturation x.
            let (v, k) = hyperbola_guess(x, y);
            vec![v, 1.0 / k]
        }),
        _ => return None,
    };
    Some(f)
}

/// Starting values for `y0 + A sin(pi (x - xc) / w)`: the offset is the mean, the amplitude half
/// the range and the half-period comes from the strongest frequency of a discrete Fourier
/// transform of the (assumed evenly spaced) points.
fn sine_guess(x: &[f64], y: &[f64]) -> Vec<f64> {
    let (x, y) = sorted(x, y);
    let n = x.len();
    let mean = y.iter().sum::<f64>() / n as f64;
    let (lo, hi) = min_max(&y);
    let amp = 0.5 * (hi - lo);
    let step = (x[n - 1] - x[0]) / (n - 1).max(1) as f64;
    if n < 4 || step <= 0.0 {
        return vec![mean, 0.0, 1.0, amp];
    }
    let mut best = (1usize, 0.0f64);
    for k in 1..=n / 2 {
        let (mut re, mut im) = (0.0, 0.0);
        for (j, v) in y.iter().enumerate() {
            let ang = -2.0 * std::f64::consts::PI * (k * j) as f64 / n as f64;
            re += (v - mean) * ang.cos();
            im += (v - mean) * ang.sin();
        }
        let a = re * re + im * im;
        if a > best.1 {
            best = (k, a);
        }
    }
    let freq = best.0 as f64 / (step * n as f64);
    let w = 1.0 / (2.0 * freq);
    let ipk = (0..n).max_by(|&a, &b| y[a].total_cmp(&y[b])).unwrap_or(0);
    let xc = x[ipk] - 0.5 * w;
    let xc = xc - 2.0 * w * (xc / (2.0 * w)).floor();
    vec![mean, xc, w, amp]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faddeeva_known_values() {
        // w(0) = 1; on the real axis Re w(x) = exp(-x^2).
        let w0 = faddeeva(Complex::new(0.0, 0.0));
        assert!((w0.re - 1.0).abs() < 1e-12 && w0.im.abs() < 1e-12);
        for x in [0.3, 1.0, 2.5, 5.0] {
            let w = faddeeva(Complex::new(x, 0.0));
            assert!((w.re - (-x * x).exp()).abs() < 1e-11, "x = {x}: {}", w.re);
        }
        // w(i y) = exp(y^2) erfc(y): at y = 1 this is 0.4275835761558070.
        let w = faddeeva(Complex::new(0.0, 1.0));
        assert!((w.re - 0.427_583_576_155_807).abs() < 1e-11, "{}", w.re);
        // Far away the asymptotic series takes over: w(40 i) ~ 1/(sqrt(pi) 40).
        let far = faddeeva(Complex::new(0.0, 40.0));
        assert!(
            (far.re - 1.0 / (std::f64::consts::PI.sqrt() * 40.0) * (1.0 - 0.5 / 1600.0 + 0.75 / 1600.0 / 1600.0)).abs()
                < 1e-11
        );
    }

    #[test]
    fn voigt_area_and_limits() {
        // Area under the profile is A (midpoint sum over a wide window; Lorentzian tails removed
        // by using a small wL).
        let (wg, wl) = (1.0, 0.01);
        let h = 0.002;
        let n = 20000;
        let mut s = 0.0;
        for i in 0..n {
            let x = -20.0 + (i as f64 + 0.5) * h;
            s += voigt(x, 0.0, 0.0, 3.0, wg, wl) * h;
        }
        assert!((s - 3.0).abs() < 2e-3, "area {s}");
        // With no Lorentzian part it is a Gaussian of FWHM wg.
        let g = voigt(0.4, 0.5, 0.0, 2.0, 1.0, 0.0);
        let sigma = 1.0 / (2.0 * (2.0 * std::f64::consts::LN_2).sqrt());
        let want = 0.5 + 2.0 / (sigma * (2.0 * std::f64::consts::PI).sqrt()) * (-0.16 / (2.0 * sigma * sigma)).exp();
        assert!((g - want).abs() < 1e-10, "{g} vs {want}");
    }

    #[test]
    fn every_formula_in_the_table_parses_and_evaluates() {
        let all = list();
        assert!(all.len() > 190, "{}", all.len());
        for info in &all {
            let f = get(&info.name).unwrap_or_else(|| panic!("{} cannot be built", info.name));
            let p = vec![1.3; f.params.len()];
            let v = f.eval(&p, 0.7);
            // A value, not an error: NaN is allowed (a branch or a domain edge), a panic is not.
            let _ = v;
        }
        for name in ["Gauss", "Lorentz", "ExpDec1", "ExpDec2", "Boltzmann", "Line", "Voigt", "PsdVoigt1", "Sine"] {
            assert!(get(name).is_some(), "{name}");
        }
    }

    #[test]
    fn lookup_ignores_case_and_gives_derived_values() {
        let f = get("gauss").unwrap();
        assert_eq!(f.name, "Gauss");
        assert_eq!(f.params[2].lower, Some(0.0));
        let params: Vec<Parameter> = ["y0", "xc", "w", "A"]
            .iter()
            .zip([0.0, 1.0, 2.0, 3.0])
            .map(|(n, v)| Parameter {
                name: n.to_string(),
                value: v,
                std_error: f64::NAN,
                fixed: false,
                t_value: f64::NAN,
                prob_t: f64::NAN,
                lcl: f64::NAN,
                ucl: f64::NAN,
                dependency: f64::NAN,
            })
            .collect();
        let d = derived_values("Gauss", &params);
        let fwhm = d.iter().find(|(n, _)| n == "FWHM").unwrap().1;
        assert!((fwhm - 2.0 * (2.0 * std::f64::consts::LN_2).sqrt()).abs() < 1e-12);
    }

    fn named(pairs: &[(&str, f64)]) -> Vec<Parameter> {
        pairs
            .iter()
            .map(|(n, v)| Parameter {
                name: n.to_string(),
                value: *v,
                std_error: f64::NAN,
                fixed: false,
                t_value: f64::NAN,
                prob_t: f64::NAN,
                lcl: f64::NAN,
                ucl: f64::NAN,
                dependency: f64::NAN,
            })
            .collect()
    }

    fn derived(name: &str, pairs: &[(&str, f64)], what: &str) -> f64 {
        derived_values(name, &named(pairs)).iter().find(|(n, _)| n == what).unwrap().1
    }

    #[test]
    fn ec20_and_ec80_use_base_ten_logarithms() {
        // Logistic: y = A2 + (A1 - A2) / (1 + (x/x0)^p) falls to 80 % of its way at x0 * 0.25^(1/p).
        let l = [("A1", 0.0), ("A2", 1.0), ("x0", 5.0), ("p", 2.0)];
        assert!((derived("Logistic", &l, "EC20") - 2.5).abs() < 1e-12);
        assert!((derived("Logistic", &l, "EC50") - 5.0).abs() < 1e-12);
        assert!((derived("Logistic", &l, "EC80") - 10.0).abs() < 1e-12);
        // DoseResp: x is a base-10 logarithm, EC values are 10^(LOGx0 + log10(0.25 or 4)/p).
        let d = [("A1", 0.0), ("A2", 1.0), ("LOGx0", 1.0), ("p", 2.0)];
        assert!((derived("DoseResp", &d, "EC20") - 5.0).abs() < 1e-12);
        assert!((derived("DoseResp", &d, "EC80") - 20.0).abs() < 1e-12);
        // The derived values agree with the model: A1 is the level at small x, so
        // y = 1 - 1/(1 + (x/5)^2) is 0.2 at EC20 and 0.8 at EC80.
        let f = get("Logistic").unwrap();
        let p: Vec<f64> = l.iter().map(|x| x.1).collect();
        assert!((f.eval(&p, 2.5) - 0.2).abs() < 1e-12);
        assert!((f.eval(&p, 10.0) - 0.8).abs() < 1e-12);
    }

    #[test]
    fn logistic5_ec50_is_the_half_level() {
        let l = [("Amin", 0.0), ("Amax", 1.0), ("x0", 5.0), ("h", 2.0), ("s", 0.7)];
        let ec50 = derived("Logistic5", &l, "EC50");
        let f = get("Logistic5").unwrap();
        let p: Vec<f64> = l.iter().map(|x| x.1).collect();
        assert!((f.eval(&p, ec50) - 0.5).abs() < 1e-12, "{ec50}");
    }

    #[test]
    fn polynomial_fit_helper_is_exact_for_a_polynomial() {
        let x: Vec<f64> = (0..8).map(|i| 10.0 + i as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| 2.0 - 0.5 * v + 0.1 * v * v).collect();
        let c = poly_fit(&x, &y, 2);
        assert!((c[0] - 2.0).abs() < 1e-7 && (c[1] + 0.5).abs() < 1e-8 && (c[2] - 0.1).abs() < 1e-9, "{c:?}");
    }
}
