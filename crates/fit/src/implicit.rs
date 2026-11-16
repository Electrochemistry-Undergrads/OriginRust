//! Nonlinear implicit curve fit: the points `(x_i, y_i)` are fitted by a curve `f(x, y, params) = 0` (a circle, an
//! ellipse) by **orthogonal distance regression**. The quantity minimised is the sum of the squared orthogonal
//! distances from the points to the curve, not the algebraic residual `f` itself (`docs/spec/fitting.md`, section 18:
//! Origin's circle fit equals the geometric fit to 6e-8 and not the algebraic one).
//!
//! The solver is Levenberg-Marquardt on the signed distances with a numerical Jacobian; the distance to a circle is
//! exact, the distance to an ellipse is found by a bracketed search on the angle of the nearest point. Origin uses
//! ODRPACK, whose residual sum of squares and errors differ from the exact distance sum by about 0.1 %; Sheaf reports
//! the exact distances (errors `sqrt(s^2 diag((J^T J)^-1))`, `s^2 = RSS / (n - p)`).
//!
//! R-square follows Origin's number: `1 - RSS / TSS` with `TSS` the sum of squares of the X and Y values together about
//! their common mean (`612.7385` for the measured circle, to 1e-9). The ANOVA table is not reported (Origin's rows hold
//! 0).

use crate::dist::{t_critical, t_two_sided_p};
use crate::report::{FitError, FitReport, FitStatus, Parameter, Statistics};

/// An implicit function of the Implicit category of Origin's fit window that Sheaf fits: Circle and Ellipse. PlaneMod
/// (three variables) is not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplicitFunction {
    /// `(x-xc)^2 + (y-yc)^2 - r^2 = 0`, parameters `xc`, `yc`, `r`.
    Circle,
    /// `((x-xc)/a)^2 + ((y-yc)/b)^2 - 1 = 0` (axes along X and Y), parameters `xc`, `yc`, `a`, `b`.
    Ellipse,
}

/// Names of the implicit functions in the order of Origin's list.
pub const NAMES: [&str; 2] = ["Circle", "Ellipse"];

impl ImplicitFunction {
    /// The function with this name (any case).
    pub fn by_name(name: &str) -> Option<ImplicitFunction> {
        match name.trim().to_ascii_lowercase().as_str() {
            "circle" => Some(ImplicitFunction::Circle),
            "ellipse" => Some(ImplicitFunction::Ellipse),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ImplicitFunction::Circle => "Circle",
            ImplicitFunction::Ellipse => "Ellipse",
        }
    }

    /// The equation as Origin's fit report prints it.
    pub fn equation(self) -> &'static str {
        match self {
            ImplicitFunction::Circle => "f = (x-xc)^2 + (y-yc)^2 - r^2",
            ImplicitFunction::Ellipse => "f = ((x-xc)/a)^2 + ((y-yc)/b)^2 - 1",
        }
    }

    pub fn params(self) -> &'static [&'static str] {
        match self {
            ImplicitFunction::Circle => &["xc", "yc", "r"],
            ImplicitFunction::Ellipse => &["xc", "yc", "a", "b"],
        }
    }

    /// `f(x, y)` for the parameters.
    pub fn eval(self, p: &[f64], x: f64, y: f64) -> f64 {
        match self {
            ImplicitFunction::Circle => (x - p[0]).powi(2) + (y - p[1]).powi(2) - p[2] * p[2],
            ImplicitFunction::Ellipse => ((x - p[0]) / p[2]).powi(2) + ((y - p[1]) / p[3]).powi(2) - 1.0,
        }
    }

    /// Starting values from the data: the algebraic (Kasa) circle for both; the ellipse takes half the ranges.
    pub fn initial(self, x: &[f64], y: &[f64]) -> Vec<f64> {
        let n = x.len().max(1) as f64;
        let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
        let half = |v: &[f64]| {
            let (lo, hi) = v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &t| (a.min(t), b.max(t)));
            ((hi - lo) / 2.0).max(1e-12)
        };
        let ellipse = vec![mx, my, half(x), half(y)];
        // Kasa: x^2 + y^2 = 2 xc x + 2 yc y + c  (least squares in xc, yc, c, on centred coordinates).
        let (mut sxx, mut sxy, mut syy, mut sxz, mut syz) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for (&a, &b) in x.iter().zip(y) {
            let (u, v) = (a - mx, b - my);
            let z = u * u + v * v;
            sxx += u * u;
            sxy += u * v;
            syy += v * v;
            sxz += u * z;
            syz += v * z;
        }
        let det = sxx * syy - sxy * sxy;
        if self == ImplicitFunction::Ellipse || det.abs() < 1e-300 {
            return match self {
                ImplicitFunction::Circle => vec![mx, my, half(x).max(half(y))],
                ImplicitFunction::Ellipse => ellipse,
            };
        }
        let (cu, cv) = ((sxz * syy - syz * sxy) / (2.0 * det), (syz * sxx - sxz * sxy) / (2.0 * det));
        let r2 = (x.iter().zip(y).map(|(&a, &b)| (a - mx - cu).powi(2) + (b - my - cv).powi(2)).sum::<f64>()) / n;
        vec![mx + cu, my + cv, r2.sqrt().max(1e-12)]
    }

    /// The point of the curve nearest to `(px, py)`.
    pub fn foot(self, p: &[f64], px: f64, py: f64) -> (f64, f64) {
        match self {
            ImplicitFunction::Circle => {
                let (dx, dy) = (px - p[0], py - p[1]);
                let d = dx.hypot(dy);
                let r = p[2].abs();
                if d == 0.0 { (p[0] + r, p[1]) } else { (p[0] + r * dx / d, p[1] + r * dy / d) }
            }
            ImplicitFunction::Ellipse => {
                let (a, b) = (p[2].abs(), p[3].abs());
                let at = |t: f64| (p[0] + a * t.cos(), p[1] + b * t.sin());
                let dist2 = |t: f64| {
                    let (fx, fy) = at(t);
                    (fx - px).powi(2) + (fy - py).powi(2)
                };
                // Derivative of the squared distance with respect to the angle.
                let slope = |t: f64| {
                    let (fx, fy) = at(t);
                    (fx - px) * (-a * t.sin()) + (fy - py) * (b * t.cos())
                };
                const SCAN: usize = 90;
                let step = std::f64::consts::TAU / SCAN as f64;
                let best = (0..SCAN)
                    .map(|k| k as f64 * step)
                    .min_by(|s, t| dist2(*s).total_cmp(&dist2(*t)))
                    .expect("the scan has points");
                let (mut lo, mut hi) = (best - step, best + step);
                if slope(lo) < 0.0 && slope(hi) > 0.0 {
                    for _ in 0..200 {
                        let mid = 0.5 * (lo + hi);
                        if slope(mid) < 0.0 {
                            lo = mid;
                        } else {
                            hi = mid;
                        }
                    }
                } else {
                    for _ in 0..200 {
                        let (m1, m2) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
                        if dist2(m1) < dist2(m2) {
                            hi = m2;
                        } else {
                            lo = m1;
                        }
                    }
                }
                at(0.5 * (lo + hi))
            }
        }
    }

    /// The signed orthogonal distance of a point: positive outside the curve (where `f > 0`), negative inside.
    pub fn distance(self, p: &[f64], x: f64, y: f64) -> f64 {
        let (fx, fy) = self.foot(p, x, y);
        let d = (x - fx).hypot(y - fy);
        if self.eval(p, x, y) < 0.0 { -d } else { d }
    }

    /// Points of the curve `f = 0` for a plot, `n` of them with the first repeated at the end (a closed curve).
    pub fn curve(self, p: &[f64], n: usize) -> Vec<(f64, f64)> {
        let n = n.max(3);
        (0..=n)
            .map(|k| {
                let t = std::f64::consts::TAU * (k % n) as f64 / n as f64;
                match self {
                    ImplicitFunction::Circle => (p[0] + p[2].abs() * t.cos(), p[1] + p[2].abs() * t.sin()),
                    ImplicitFunction::Ellipse => (p[0] + p[2].abs() * t.cos(), p[1] + p[3].abs() * t.sin()),
                }
            })
            .collect()
    }
}

/// Settings of an implicit fit.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplicitOptions {
    /// Starting values; empty takes [`ImplicitFunction::initial`].
    pub initial: Vec<f64>,
    pub max_iterations: usize,
    pub tolerance: f64,
    pub confidence_level: f64,
}

impl Default for ImplicitOptions {
    /// Origin's defaults: 400 iterations, tolerance 1e-9, 95 % limits.
    fn default() -> Self {
        ImplicitOptions { initial: Vec::new(), max_iterations: 400, tolerance: 1e-9, confidence_level: 0.95 }
    }
}

/// The result of an implicit fit.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplicitFit {
    /// Parameters, statistics and status as for any fit. `fitted` holds the Y of the nearest curve point of every
    /// input row and `residuals` the Y difference; the distances are in `distances`. NaN for rows left out.
    pub report: FitReport,
    /// The nearest point of the curve of every input row (NaN for rows left out).
    pub feet: Vec<(f64, f64)>,
    /// The signed orthogonal distance of every input row (NaN for rows left out).
    pub distances: Vec<f64>,
}

fn solve_spd(a: &[f64], b: &[f64], n: usize) -> Option<Vec<f64>> {
    let m = nalgebra::DMatrix::from_row_slice(n, n, a);
    let rhs = nalgebra::DVector::from_column_slice(b);
    m.lu().solve(&rhs).map(|v| v.iter().copied().collect())
}

/// Fits `function` to the points; rows with a missing X or Y are left out.
pub fn fit_implicit(
    function: ImplicitFunction,
    x: &[f64],
    y: &[f64],
    opts: &ImplicitOptions,
) -> Result<ImplicitFit, FitError> {
    if x.len() != y.len() {
        return Err(FitError::LengthMismatch { x: x.len(), y: y.len() });
    }
    let used: Vec<usize> = (0..x.len()).filter(|&i| x[i].is_finite() && y[i].is_finite()).collect();
    let (xs, ys): (Vec<f64>, Vec<f64>) = used.iter().map(|&i| (x[i], y[i])).unzip();
    let names = function.params();
    let np = names.len();
    let n = xs.len();
    if n < np + 1 {
        return Err(FitError::TooFewPoints { have: n, need: np + 1 });
    }
    let mut p = if opts.initial.is_empty() { function.initial(&xs, &ys) } else { opts.initial.clone() };
    if p.len() != np {
        return Err(FitError::BadFunction(format!(
            "{} has {np} parameters ({}), not {} starting values",
            function.name(),
            names.join(", "),
            p.len()
        )));
    }
    if !(opts.tolerance > 0.0 && opts.tolerance.is_finite()) {
        return Err(FitError::BadFunction(format!("the tolerance must be a positive number, not {}", opts.tolerance)));
    }
    let residuals = |q: &[f64]| -> Vec<f64> { xs.iter().zip(&ys).map(|(&a, &b)| function.distance(q, a, b)).collect() };
    let sum_sq = |r: &[f64]| r.iter().map(|v| v * v).sum::<f64>();
    let jacobian = |q: &[f64]| -> Vec<Vec<f64>> {
        // Central differences; columns are parameters.
        (0..np)
            .map(|j| {
                let h = 1e-6 * q[j].abs().max(1.0);
                let (mut hi, mut lo) = (q.to_vec(), q.to_vec());
                hi[j] += h;
                lo[j] -= h;
                let (rh, rl) = (residuals(&hi), residuals(&lo));
                rh.iter().zip(&rl).map(|(a, b)| (a - b) / (2.0 * h)).collect()
            })
            .collect()
    };
    let normal = |jac: &[Vec<f64>]| -> Vec<f64> {
        let mut m = vec![0.0; np * np];
        for a in 0..np {
            for b in 0..np {
                m[a * np + b] = jac[a].iter().zip(&jac[b]).map(|(u, v)| u * v).sum();
            }
        }
        m
    };

    let mut r = residuals(&p);
    let mut rss = sum_sq(&r);
    if !rss.is_finite() {
        return Err(FitError::Failed("the starting values give no distances".into()));
    }
    let mut lambda = 1e-3;
    let mut status = FitStatus::MaxIterations;
    let mut iterations = 0;
    for it in 0..opts.max_iterations {
        iterations = it + 1;
        let jac = jacobian(&p);
        let jtj = normal(&jac);
        let g: Vec<f64> = (0..np).map(|a| jac[a].iter().zip(&r).map(|(u, v)| u * v).sum()).collect();
        let mut improved = false;
        for _ in 0..40 {
            let mut m = jtj.clone();
            for a in 0..np {
                m[a * np + a] += lambda * jtj[a * np + a].max(1e-12);
            }
            let rhs: Vec<f64> = g.iter().map(|v| -v).collect();
            if let Some(step) = solve_spd(&m, &rhs, np) {
                let trial: Vec<f64> = p.iter().zip(&step).map(|(a, s)| a + s).collect();
                let rt = residuals(&trial);
                let st = sum_sq(&rt);
                if st.is_finite() && st <= rss {
                    let gain = rss - st;
                    p = trial;
                    r = rt;
                    let small = gain <= opts.tolerance * (rss + opts.tolerance)
                        || step.iter().zip(&p).all(|(s, v)| s.abs() <= 1e-12 * (v.abs() + 1e-12));
                    rss = st;
                    lambda = (lambda * 0.3).max(1e-12);
                    improved = true;
                    if small {
                        status = FitStatus::Converged;
                    }
                    break;
                }
            }
            lambda *= 5.0;
        }
        if !improved {
            // No step reduces the sum of squares: at a minimum to working precision.
            status = FitStatus::Converged;
            break;
        }
        if status == FitStatus::Converged {
            break;
        }
    }
    // The size of an axis or radius has no sign in the curve; report it positive.
    if function == ImplicitFunction::Circle {
        p[2] = p[2].abs();
    } else {
        p[2] = p[2].abs();
        p[3] = p[3].abs();
    }

    let dof = n - np;
    let jac = jacobian(&p);
    let jtj = normal(&jac);
    let reduced = rss / dof as f64;
    let inv = nalgebra::DMatrix::from_row_slice(np, np, &jtj).try_inverse();
    let tcrit = t_critical(opts.confidence_level, dof);
    let mut covariance = vec![f64::NAN; np * np];
    let mut params = Vec::with_capacity(np);
    for (j, name) in names.iter().enumerate() {
        let (se, dep) = match &inv {
            Some(inv) => {
                for k in 0..np {
                    covariance[j * np + k] = reduced * inv[(j, k)];
                }
                let c = reduced * inv[(j, j)];
                let prod = inv[(j, j)] * jtj[j * np + j];
                (
                    if c >= 0.0 { c.sqrt() } else { f64::NAN },
                    if prod.is_finite() && prod != 0.0 { 1.0 - 1.0 / prod } else { f64::NAN },
                )
            }
            None => (f64::NAN, f64::NAN),
        };
        let value = p[j];
        let t = value / se;
        params.push(Parameter {
            name: (*name).to_string(),
            value,
            std_error: se,
            fixed: false,
            t_value: t,
            prob_t: t_two_sided_p(t, dof),
            lcl: value - tcrit * se,
            ucl: value + tcrit * se,
            dependency: dep,
        });
    }
    // Origin's R-square: the total sum of squares is that of the X and Y values together about their common mean.
    let all: Vec<f64> = xs.iter().chain(&ys).copied().collect();
    let mean = all.iter().sum::<f64>() / all.len() as f64;
    let tss: f64 = all.iter().map(|v| (v - mean).powi(2)).sum();
    let r2 = if tss > 0.0 { 1.0 - rss / tss } else { f64::NAN };
    let adj = if tss > 0.0 && dof > 0 { 1.0 - reduced / (tss / (n - 1) as f64) } else { f64::NAN };
    let mut warnings = Vec::new();
    if status == FitStatus::MaxIterations {
        warnings.push("the fit stopped at the iteration limit before converging".to_string());
    }
    if params.iter().any(|q| q.dependency.is_finite() && q.dependency > 0.9999) {
        warnings.push("some parameters are strongly dependent on each other (dependency above 0.9999)".into());
    }
    let mut feet = vec![(f64::NAN, f64::NAN); x.len()];
    let mut distances = vec![f64::NAN; x.len()];
    let mut fitted = vec![f64::NAN; x.len()];
    let mut res = vec![f64::NAN; x.len()];
    for &i in &used {
        let foot = function.foot(&p, x[i], y[i]);
        feet[i] = foot;
        distances[i] = function.distance(&p, x[i], y[i]);
        fitted[i] = foot.1;
        res[i] = y[i] - foot.1;
    }
    let report = FitReport {
        model: function.name().to_string(),
        equation: function.equation().to_string(),
        params,
        stats: Statistics {
            n_points: n,
            dof,
            rss,
            reduced_chi_sqr: reduced,
            r_squared: r2,
            adj_r_squared: adj,
            pearson_r: None,
            iterations: Some(iterations),
        },
        anova: None,
        covariance,
        confidence_level: opts.confidence_level,
        fitted,
        residuals: res,
        status,
        warnings,
    };
    Ok(ImplicitFit { report, feet, distances })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle_points(xc: f64, yc: f64, r: f64, n: usize, noise: f64) -> (Vec<f64>, Vec<f64>) {
        let mut s = 12345u64;
        let mut rnd = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f64 / (1u64 << 31) as f64) - 0.5
        };
        (0..n)
            .map(|k| {
                let t = std::f64::consts::TAU * k as f64 / n as f64;
                (xc + r * t.cos() + noise * rnd(), yc + r * t.sin() + noise * rnd())
            })
            .unzip()
    }

    #[test]
    fn exact_circle_points_are_recovered() {
        let (x, y) = circle_points(2.0, -1.0, 3.0, 12, 0.0);
        let fit = fit_implicit(ImplicitFunction::Circle, &x, &y, &ImplicitOptions::default()).unwrap();
        let v: Vec<f64> = fit.report.params.iter().map(|p| p.value).collect();
        assert!((v[0] - 2.0).abs() < 1e-8 && (v[1] + 1.0).abs() < 1e-8 && (v[2] - 3.0).abs() < 1e-8, "{v:?}");
        assert!(fit.report.stats.rss < 1e-12);
        assert_eq!(fit.report.stats.dof, 9);
    }

    #[test]
    fn exact_ellipse_points_are_recovered() {
        let pts: Vec<(f64, f64)> = ImplicitFunction::Ellipse.curve(&[1.0, 2.0, 4.0, 2.0], 30);
        let (x, y): (Vec<f64>, Vec<f64>) = pts[..30].iter().copied().unzip();
        let fit = fit_implicit(ImplicitFunction::Ellipse, &x, &y, &ImplicitOptions::default()).unwrap();
        let v: Vec<f64> = fit.report.params.iter().map(|p| p.value).collect();
        for (got, want) in v.iter().zip([1.0, 2.0, 4.0, 2.0]) {
            assert!((got - want).abs() < 1e-6, "{v:?}");
        }
    }

    #[test]
    fn the_distance_to_an_ellipse_is_the_nearest_point() {
        let p = [0.0, 0.0, 4.0, 1.0];
        // On the major axis the nearest point of the curve to (6, 0) is (4, 0).
        let (fx, fy) = ImplicitFunction::Ellipse.foot(&p, 6.0, 0.0);
        assert!((fx - 4.0).abs() < 1e-9 && fy.abs() < 1e-9);
        assert!((ImplicitFunction::Ellipse.distance(&p, 6.0, 0.0) - 2.0).abs() < 1e-9);
        assert!((ImplicitFunction::Ellipse.distance(&p, 0.0, 0.5) + 0.5).abs() < 1e-9, "inside is negative");
        // Brute force on a point off the axes.
        let (px, py) = (3.0, 2.0);
        let brute = (0..200000)
            .map(|k| {
                let t = std::f64::consts::TAU * k as f64 / 200000.0;
                (4.0 * t.cos() - px).hypot(t.sin() - py)
            })
            .fold(f64::INFINITY, f64::min);
        assert!((ImplicitFunction::Ellipse.distance(&p, px, py) - brute).abs() < 1e-8);
    }

    #[test]
    fn missing_values_are_left_out_and_too_few_points_are_refused() {
        let (mut x, y) = circle_points(0.0, 0.0, 1.0, 10, 0.01);
        x[3] = f64::NAN;
        let fit = fit_implicit(ImplicitFunction::Circle, &x, &y, &ImplicitOptions::default()).unwrap();
        assert_eq!(fit.report.stats.n_points, 9);
        assert!(fit.distances[3].is_nan());
        let e = fit_implicit(ImplicitFunction::Circle, &x[..3], &y[..3], &ImplicitOptions::default()).unwrap_err();
        assert!(matches!(e, FitError::TooFewPoints { .. }));
    }

    #[test]
    fn the_names_resolve() {
        assert_eq!(ImplicitFunction::by_name(" ellipse "), Some(ImplicitFunction::Ellipse));
        assert_eq!(ImplicitFunction::by_name("PlaneMod"), None);
        assert_eq!(NAMES.len(), 2);
    }
}
