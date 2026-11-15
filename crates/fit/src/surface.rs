//! Built-in surface fit functions `z = f(x, y)` fitted to three worksheet columns (Origin's
//! category "Surface Fitting"). See `docs/spec/fitting.md`, section "Surface fits".
//!
//! They are kept apart from the one-variable table of [`crate::library`] so that the one-variable
//! function lists stay as they are; [`crate::library::get`] also finds them by name and
//! [`crate::nlsf::fit_xyz`] fits them.
//!
//! Starting values follow the rules of the function files where the code is plain: a polynomial
//! fit of `z` against `x` and against `y` for the polynomial surfaces, and for the peak surfaces the
//! peak position, width and height of the slices of the data through the highest point.

use std::sync::Arc;

use crate::nlsf::{FitFunction, InitFn2};

/// One surface function of the table.
struct Entry {
    name: &'static str,
    params: &'static [&'static str],
    /// Right-hand side in the expression language.
    formula: &'static str,
    /// `(parameter, lower, upper)` bounds that are on by default.
    bounds: &'static [(&'static str, Option<f64>, Option<f64>)],
    /// Derived quantities `(name, formula)`.
    derived: &'static [(&'static str, &'static str)],
}

const W: Option<f64> = Some(0.0);

const TABLE: &[Entry] = &[
    Entry { name: "Plane", params: &["z0", "a", "b"], formula: "z0 + a*x + b*y", bounds: &[], derived: &[] },
    Entry {
        name: "Parabola2D",
        params: &["z0", "a", "b", "c", "d"],
        formula: "z0 + a*x + b*y + c*x^2 + d*y^2",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "Poly2D",
        params: &["z0", "a", "b", "c", "d", "f"],
        formula: "z0 + a*x + b*y + c*x^2 + d*y^2 + f*x*y",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "Polynomial2D",
        params: &["z0", "A1", "A2", "A3", "A4", "A5", "B1", "B2", "B3", "B4", "B5"],
        formula: "z0 + A1*x + A2*x^2 + A3*x^3 + A4*x^4 + A5*x^5 + B1*y + B2*y^2 + B3*y^3 + B4*y^4 + B5*y^5",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "Gauss2D",
        params: &["z0", "A", "xc", "w1", "yc", "w2"],
        formula: "z0 + A*exp(-0.5*((x-xc)/w1)^2 - 0.5*((y-yc)/w2)^2)",
        bounds: &[("w1", W, None), ("w2", W, None)],
        derived: &[("FWHMx", "2*sqrt(2*ln(2))*w1"), ("FWHMy", "2*sqrt(2*ln(2))*w2"), ("Volume", "2*pi*A*w1*w2")],
    },
    Entry {
        name: "Gaussian2D",
        params: &["z0", "A", "xc", "w1", "yc", "w2", "theta"],
        formula: "z0 + A*exp(-0.5*((x*cos(theta) + y*sin(theta) - xc*cos(theta) - yc*sin(theta))/w1)^2 \
                  - 0.5*((-x*sin(theta) + y*cos(theta) + xc*sin(theta) - yc*cos(theta))/w2)^2)",
        bounds: &[("w1", W, None), ("w2", W, None)],
        derived: &[("FWHM1", "2*sqrt(2*ln(2))*w1"), ("FWHM2", "2*sqrt(2*ln(2))*w2"), ("Volume", "2*A*pi*w1*w2")],
    },
    Entry {
        name: "Lorentz2D",
        params: &["z0", "A", "xc", "w1", "yc", "w2"],
        formula: "z0 + A/((1 + ((x-xc)/w1)^2)*(1 + ((y-yc)/w2)^2))",
        bounds: &[("w1", W, None), ("w2", W, None)],
        derived: &[("FWHMx", "2*w1"), ("FWHMy", "2*w2"), ("Volume", "A*w1*w2*pi^2")],
    },
    Entry {
        name: "Voigt2D",
        params: &["z0", "A", "xc", "w1", "yc", "w2", "mu"],
        formula: "z0 + A*(mu/((1 + ((x-xc)/w1)^2)*(1 + ((y-yc)/w2)^2)) \
                  + (1-mu)*exp(-0.5*((x-xc)/w1)^2 - 0.5*((y-yc)/w2)^2))",
        bounds: &[("w1", W, None), ("w2", W, None), ("mu", W, Some(1.0))],
        derived: &[("Volume", "A*w1*w2*(mu*pi^2 + (1-mu)*2*pi)")],
    },
    Entry {
        name: "Voigt2DMod",
        params: &["z0", "A", "xc", "w1", "yc", "w2", "mu"],
        formula: "z0 + A*(mu*4/pi^2*w1*w2/((4*(x-xc)^2 + w1^2)*(4*(y-yc)^2 + w2^2)) \
                  + (1-mu)*(4*ln(2))/(pi*w1*w2)*exp(-4*ln(2)/w1^2*(x-xc)^2 - 4*ln(2)/w2^2*(y-yc)^2))",
        bounds: &[("w1", W, None), ("w2", W, None), ("mu", W, Some(1.0))],
        derived: &[],
    },
    Entry {
        name: "Exponential2D",
        params: &["z0", "B", "C", "D"],
        formula: "z0 + B*exp(-x/C)*exp(-y/D)",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "Extreme2D",
        params: &["z0", "B", "C", "D", "E", "F"],
        formula: "z0 + B*y + C*y^2 + D*exp(-exp(-(x - E)/F) - (x - E)/F + 1)",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "LogNormal2D",
        params: &["z0", "B", "C", "D", "E", "F", "G", "H"],
        formula: "z0 + B*exp(-0.5*(ln(x/C)/D)^2) + E*exp(-0.5*(ln(y/F)/G)^2) \
                  + H*exp(-0.5*((ln(x/C)/D)^2 + (ln(y/F)/G)^2))",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "Power2D",
        params: &["z0", "B", "C", "D", "E", "F"],
        formula: "z0 + B*pow(x, C) + D*pow(y, E) + F*pow(x, C)*pow(y, E)",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "Rational2D",
        params: &["z0", "A01", "B01", "B02", "B03", "A1", "A2", "A3", "B1", "B2"],
        formula: "(z0 + A01*x + B01*y + B02*y^2 + B03*y^3)/(1 + A1*x + A2*x^2 + A3*x^3 + B1*y + B2*y^2)",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "RationalTaylor",
        params: &["z0", "A01", "B01", "B02", "C02", "A1", "A2", "B1", "B2", "C2"],
        formula: "(z0 + A01*x + B01*y + B02*y^2 + C02*x*y)/(1 + A1*x + B1*y + A2*x^2 + B2*y^2 + C2*x*y)",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "DoseResp2D",
        params: &["z0", "B", "C", "D", "E", "F"],
        formula: "z0 + B*(1/(1 + (x/C)^(-D)))*(1/(1 + (y/E)^(-F)))",
        bounds: &[("C", W, None), ("E", W, None)],
        derived: &[],
    },
    Entry {
        name: "GaussCum2D",
        params: &["z0", "B", "C", "D", "E", "F"],
        formula: "z0 + 0.25*B*(1 + erf((x - C)/(sqrt(2)*D)))*(1 + erf((y - E)/(sqrt(2)*F)))",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "LogisticCum2D",
        params: &["z0", "B", "C", "D", "E", "F"],
        formula: "z0 + B*(1/(1 + exp(-((x - C)/D))))*(1/(1 + exp(-((y - E)/F))))",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "ExtremeCum2D",
        params: &["z0", "B", "C", "D", "E", "F", "G", "H"],
        formula: "z0 + B*exp(-exp(-(x - C)/D)) + E*exp(-exp(-(y - F)/G)) \
                  + H*exp(-exp(-(x - C)/D) - exp(-(y - F)/G))",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "Chebyshev2D",
        params: &["z0", "A1", "A2", "B1", "B2", "C1"],
        formula: "z0 + A1*cos(acos(x)) + B1*cos(acos(y)) + A2*cos(2*acos(x)) + C1*cos(acos(x))*cos(acos(y)) \
                  + B2*cos(2*acos(y))",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "Cosine2D",
        params: &["z0", "A1", "A2", "B1", "B2", "C1"],
        formula: "z0 + A1*cos(x) + B1*cos(y) + A2*cos(2*x) + C1*cos(x)*cos(y) + B2*cos(2*y)",
        bounds: &[],
        derived: &[],
    },
    Entry {
        name: "Fourier2D",
        params: &["z0", "a", "b", "c", "d", "w1", "w2"],
        formula: "z0 + a*cos(x/w1) + b*sin(x/w1) + c*cos(y/w2) + d*sin(y/w2)",
        bounds: &[],
        derived: &[],
    },
];

/// Names of the surface functions, in table order.
pub fn names() -> Vec<&'static str> {
    TABLE.iter().map(|e| e.name).collect()
}

/// Parameter names of a surface function.
pub fn param_names(name: &str) -> Option<Vec<&'static str>> {
    entry(name).map(|e| e.params.to_vec())
}

/// The formula of a surface function.
pub fn formula(name: &str) -> Option<&'static str> {
    entry(name).map(|e| e.formula)
}

fn entry(name: &str) -> Option<&'static Entry> {
    TABLE.iter().find(|e| e.name.eq_ignore_ascii_case(name))
}

/// The surface function with this name (case does not matter), ready for [`crate::nlsf::fit_xyz`].
pub fn get(name: &str) -> Option<FitFunction> {
    let e = entry(name)?;
    let mut f = FitFunction::from_formula_xy_builtin(e.name, &alias_e(e.formula), e.params).ok()?;
    for (p, lo, hi) in e.bounds {
        f = f.with_bounds(p, *lo, *hi);
    }
    f.init2 = init_for(e.name);
    Some(f)
}

/// The formula with the parameter `E` written `E__` (see `FitFunction::from_formula_xy_builtin`).
fn alias_e(formula: &str) -> String {
    let b: Vec<char> = formula.chars().collect();
    let mut out = String::new();
    for (i, &c) in b.iter().enumerate() {
        out.push(c);
        let word = |k: Option<usize>| k.and_then(|k| b.get(k)).is_some_and(|c| c.is_alphanumeric() || *c == '_');
        if c == 'E' && !word(i.checked_sub(1)) && !word(Some(i + 1)) {
            out.push_str("__");
        }
    }
    out
}

/// Values of the derived quantities (FWHM, volume) of a surface function for fitted parameters.
pub fn derived_values(name: &str, params: &[crate::report::Parameter]) -> Vec<(String, f64)> {
    let Some(e) = entry(name) else { return Vec::new() };
    let host = sheaf_expr::SimpleHost::from_vars(params.iter().map(|p| (p.name.as_str(), p.value)));
    e.derived
        .iter()
        .filter_map(|(n, f)| Some((n.to_string(), sheaf_expr::Expr::parse(f).ok()?.eval(&host).ok()?)))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Starting values
// ---------------------------------------------------------------------------------------------

fn init_for(name: &str) -> Option<InitFn2> {
    let f: InitFn2 = match name {
        "Plane" => Arc::new(|x, y, z| {
            let cx = poly_fit(x, z, 1);
            let cy = poly_fit(y, z, 1);
            vec![cx[0], cx[1], cy[1]]
        }),
        "Parabola2D" => Arc::new(|x, y, z| {
            let cx = poly_fit(x, z, 2);
            let cy = poly_fit(y, z, 2);
            vec![cx[0], cx[1], cy[1], cx[2], cy[2]]
        }),
        "Poly2D" => Arc::new(|x, y, z| {
            let cx = poly_fit(x, z, 2);
            let cy = poly_fit(y, z, 2);
            vec![cx[0], cx[1], cy[1], cx[2], cy[2], 0.0]
        }),
        "Gauss2D" => Arc::new(|x, y, z| peak_start(x, y, z, 0.85, &[])),
        "Gaussian2D" => Arc::new(|x, y, z| {
            let mut v = peak_start(x, y, z, 0.85, &[]);
            v.push(0.0);
            v
        }),
        "Lorentz2D" => Arc::new(|x, y, z| peak_start(x, y, z, 0.5, &[])),
        "Voigt2D" | "Voigt2DMod" => Arc::new(|x, y, z| peak_start(x, y, z, 0.85, &[0.0])),
        "Exponential2D" => Arc::new(|x, y, z| {
            let (xc, yc) = slices(x, y, z);
            let (zmin, zmax) = min_max(z);
            let c = (span_between(&xc, true) - span_between(&xc, false)).abs();
            let d = (span_between(&yc, true) - span_between(&yc, false)).abs();
            // Decay towards the high end of x or y (the slice slopes down) or growth.
            let (z0, b) = if slope_sign(x, z) < 0.0 { (zmin, zmax - zmin) } else { (zmax, zmin - zmax) };
            vec![z0, b, nz(c), nz(d)]
        }),
        "Rational2D" => Arc::new(|x, y, z| {
            let (xc, yc) = slices(x, y, z);
            let c1 = poly_fit_pts(&xc, 1);
            let c2 = poly_fit_pts(&yc, 3);
            let t = if c2[0].abs() < 1e-6 { 1.0 } else { c2[0] };
            let z0 = c1[0] / t;
            let rx: Vec<(f64, f64)> = xc.iter().map(|&(a, b)| (a, 1.0 / b)).collect();
            let ry: Vec<(f64, f64)> = yc.iter().map(|&(a, b)| (a, 1.0 / b)).collect();
            let d1 = poly_fit_pts(&rx, 3);
            let d2 = poly_fit_pts(&ry, 2);
            vec![z0, c1[1], c2[1], c2[2], c2[3], d1[1], d1[2], d1[3], d2[1], d2[2]]
        }),
        _ => return None,
    };
    Some(f)
}

fn nz(v: f64) -> f64 {
    if v.is_finite() && v != 0.0 { v } else { f64::NAN }
}

fn min_max(v: &[f64]) -> (f64, f64) {
    v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &a| (lo.min(a), hi.max(a)))
}

fn slope_sign(x: &[f64], z: &[f64]) -> f64 {
    poly_fit(x, z, 1)[1]
}

/// Least-squares polynomial coefficients (lowest power first) of `z` against `x`; a polynomial
/// that cannot be fitted (too few distinct x) gives zeros.
fn poly_fit(x: &[f64], z: &[f64], order: usize) -> Vec<f64> {
    let pts: Vec<(f64, f64)> = x.iter().copied().zip(z.iter().copied()).collect();
    poly_fit_pts(&pts, order)
}

fn poly_fit_pts(pts: &[(f64, f64)], order: usize) -> Vec<f64> {
    use nalgebra::{DMatrix, DVector};
    let n = pts.len();
    if n <= order {
        return vec![0.0; order + 1];
    }
    // Centre and scale x so that the normal equations stay well conditioned.
    let m = pts.iter().map(|p| p.0).sum::<f64>() / n as f64;
    let s = pts.iter().map(|p| (p.0 - m).abs()).fold(0.0, f64::max).max(1e-300);
    let a = DMatrix::from_fn(n, order + 1, |i, j| ((pts[i].0 - m) / s).powi(j as i32));
    let b = DVector::from_iterator(n, pts.iter().map(|p| p.1));
    let Some(c) = a.svd(true, true).solve(&b, 1e-12).ok() else { return vec![0.0; order + 1] };
    // Expand the polynomial in u = (x - m)/s back to powers of x.
    let mut out = vec![0.0; order + 1];
    for (j, cj) in c.iter().enumerate() {
        // cj * ((x - m)/s)^j = cj/s^j * sum_k C(j,k) x^k (-m)^(j-k)
        let mut binom = 1.0;
        for (k, o) in out.iter_mut().enumerate().take(j + 1) {
            *o += cj / s.powi(j as i32) * binom * (-m).powi((j - k) as i32);
            binom = binom * (j - k) as f64 / (k + 1) as f64;
        }
    }
    out
}

/// The two slice curves of the data through the highest point: `z` against `x` at the `y` of the
/// maximum and `z` against `y` at its `x`; the points of a slice are those with exactly that
/// coordinate when there are at least four, else all points averaged per coordinate. Sorted.
fn slices(x: &[f64], y: &[f64], z: &[f64]) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let imax = (0..z.len()).fold(0, |b, i| if z[i] > z[b] { i } else { b });
    let along = |a: &[f64], b: &[f64]| -> Vec<(f64, f64)> {
        let mut v: Vec<(f64, f64)> = (0..z.len()).filter(|&i| b[i] == b[imax]).map(|i| (a[i], z[i])).collect();
        if v.len() < 4 {
            // Average z per distinct a.
            let mut all: Vec<(f64, f64)> = (0..z.len()).map(|i| (a[i], z[i])).collect();
            all.sort_by(|p, q| p.0.total_cmp(&q.0));
            v.clear();
            let mut i = 0;
            while i < all.len() {
                let mut j = i;
                let mut sum = 0.0;
                while j < all.len() && all[j].0 == all[i].0 {
                    sum += all[j].1;
                    j += 1;
                }
                v.push((all[i].0, sum / (j - i) as f64));
                i = j;
            }
        }
        v.sort_by(|p, q| p.0.total_cmp(&q.0));
        v
    };
    (along(x, y), along(y, x))
}

/// Position of the largest and smallest `z` of a curve, for a span measure.
fn span_between(c: &[(f64, f64)], of_max: bool) -> f64 {
    let pick = |better: &dyn Fn(f64, f64) -> bool| c.iter().fold(c[0], |b, &p| if better(p.1, b.1) { p } else { b }).0;
    if of_max { pick(&|a, b| a > b) } else { pick(&|a, b| a < b) }
}

/// Peak position, full width at half height, baseline and height of a curve (the quantities of
/// Origin's `peak_pos`): the baseline is the lowest value, the height is above it.
fn peak_of(c: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let imax = (0..c.len()).fold(0, |b, i| if c[i].1 > c[b].1 { i } else { b });
    let base = c.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let h = c[imax].1 - base;
    let half = base + h / 2.0;
    let cross = |dir: i64| -> f64 {
        let mut i = imax as i64;
        while i + dir >= 0 && (i + dir) < c.len() as i64 && c[(i + dir) as usize].1 > half {
            i += dir;
        }
        let j = i + dir;
        if j < 0 || j >= c.len() as i64 {
            return c[i as usize].0;
        }
        let (p, q) = (c[i as usize], c[j as usize]);
        p.0 + (half - p.1) / (q.1 - p.1) * (q.0 - p.0)
    };
    let width = (cross(1) - cross(-1)).abs();
    (c[imax].0, width, base, h)
}

/// Starting values `[z0, A, xc, w1, yc, w2, extra...]` of the peak surfaces: centres at the
/// highest point, widths from the slices scaled by `wscale`.
fn peak_start(x: &[f64], y: &[f64], z: &[f64], wscale: f64, extra: &[f64]) -> Vec<f64> {
    let (xc, yc) = slices(x, y, z);
    let (cx, wx, base, h) = peak_of(&xc);
    let (cy, wy, _, _) = peak_of(&yc);
    let mut v = vec![base, h, cx, wx * wscale, cy, wy * wscale];
    v.extend_from_slice(extra);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlsf::{FitOptions, fit_xyz};

    fn grid(n: usize, f: impl Fn(f64, f64) -> f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let (mut x, mut y, mut z) = (vec![], vec![], vec![]);
        for j in 0..n {
            for i in 0..n {
                let (a, b) = (0.5 * i as f64, 0.5 * j as f64);
                x.push(a);
                y.push(b);
                z.push(f(a, b));
            }
        }
        (x, y, z)
    }

    #[test]
    fn every_table_entry_builds() {
        for n in names() {
            let f = get(n).unwrap_or_else(|| panic!("{n} does not build"));
            assert_eq!(f.x2_name.as_deref(), Some("y"));
            assert_eq!(f.params.len(), param_names(n).unwrap().len());
        }
        assert_eq!(names().len(), 22);
    }

    #[test]
    fn plane_recovers_exact_data() {
        let (x, y, z) = grid(8, |a, b| 1.5 + 0.8 * a - 0.6 * b);
        let r = fit_xyz(&get("Plane").unwrap(), &x, &y, &z, &FitOptions::default()).unwrap();
        assert!((r.param("z0").unwrap().value - 1.5).abs() < 1e-8);
        assert!((r.param("a").unwrap().value - 0.8).abs() < 1e-8);
        assert!((r.param("b").unwrap().value + 0.6).abs() < 1e-8);
    }

    #[test]
    fn gauss2d_recovers_exact_data() {
        let (x, y, z) = grid(11, |a, b| {
            0.4 + 3.0 * (-0.5 * ((a - 2.6) / 1.1f64).powi(2) - 0.5 * ((b - 2.2) / 0.9f64).powi(2)).exp()
        });
        // Start where a user would (noise-free data with a poor start can end in a flat spot).
        let mut f = get("Gauss2D").unwrap();
        for (n, v) in [("z0", 0.6), ("A", 2.7), ("xc", 2.65), ("w1", 1.6), ("yc", 2.25), ("w2", 1.3)] {
            f = f.with_initial(n, v);
        }
        let r = fit_xyz(&f, &x, &y, &z, &FitOptions::default()).unwrap();
        for (n, v) in [("z0", 0.4), ("A", 3.0), ("xc", 2.6), ("w1", 1.1), ("yc", 2.2), ("w2", 0.9)] {
            assert!((r.param(n).unwrap().value - v).abs() < 1e-6, "{n}");
        }
    }

    #[test]
    fn one_variable_fit_refuses_a_surface_function() {
        let f = get("Plane").unwrap();
        assert!(crate::nlsf::fit(&f, &[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0, 3.0, 4.0], &FitOptions::default()).is_err());
    }

    #[test]
    fn poly_fit_is_exact_on_a_quadratic() {
        let x: Vec<f64> = (0..9).map(|i| i as f64 * 0.5).collect();
        let z: Vec<f64> = x.iter().map(|a| 2.0 - a + 0.25 * a * a).collect();
        let c = poly_fit(&x, &z, 2);
        assert!((c[0] - 2.0).abs() < 1e-9 && (c[1] + 1.0).abs() < 1e-9 && (c[2] - 0.25).abs() < 1e-9, "{c:?}");
    }
}
