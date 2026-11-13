//! Straight-line and polynomial fits (closed form, Origin's "Linear Fit" and "Polynomial Fit").
//!
//! The rules (weights, scaling of the errors, R-square and the analysis of variance when a
//! parameter is held fixed) are written down in `docs/spec/fitting-linear.md` together with the
//! Origin numbers they were checked against.
//!
//! Numerics: the weighted design matrix is decomposed by QR, never through the normal equations.
//! When every coefficient is free the basis is `u^k` with `u = (x - mean) / spread`, which stays
//! accurate when x is far from zero (for example 100000 to 100010); the coefficients and their
//! covariance are then converted to powers of the original x. When a coefficient is held fixed,
//! the free powers of `x / max|x|` are used. Fitted values and the residual sum of squares are
//! always evaluated in the basis that was solved, not from the converted coefficients, so the
//! cancellation in the conversion does not reach them.

use crate::bands::Bands;
use crate::dist::{f_upper_p, t_critical, t_two_sided_p};
use crate::report::{Anova, FitError, FitReport, FitStatus, Parameter, Statistics, Weighting};
use nalgebra::{DMatrix, DVector};

/// Options of [`fit_linear`]. The defaults are those of Origin's Linear Fit dialog.
///
/// Instrumental weighting carries the y-error values inside [`Weighting::Instrumental`], one per
/// input row, so there is no separate error argument.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearOptions {
    /// Hold the intercept at this value.
    pub fix_intercept: Option<f64>,
    /// Hold the slope at this value.
    pub fix_slope: Option<f64>,
    pub weighting: Weighting,
    /// Confidence level of the confidence limits.
    pub confidence_level: f64,
    /// Multiply the covariance by the reduced chi-square (the usual least-squares standard
    /// errors; Origin's default). If false, the covariance is `(X'WX)^-1` from the weights alone,
    /// which is only meaningful when the weights are true inverse variances.
    pub scale_errors_by_reduced_chi_sqr: bool,
}

impl Default for LinearOptions {
    fn default() -> Self {
        LinearOptions {
            fix_intercept: None,
            fix_slope: None,
            weighting: Weighting::None,
            confidence_level: 0.95,
            scale_errors_by_reduced_chi_sqr: true,
        }
    }
}

/// Options of [`fit_polynomial`].
#[derive(Debug, Clone, PartialEq)]
pub struct PolyOptions {
    /// Hold the intercept at this value.
    pub fix_intercept: Option<f64>,
    pub weighting: Weighting,
    pub confidence_level: f64,
    pub scale_errors_by_reduced_chi_sqr: bool,
}

impl Default for PolyOptions {
    fn default() -> Self {
        PolyOptions {
            fix_intercept: None,
            weighting: Weighting::None,
            confidence_level: 0.95,
            scale_errors_by_reduced_chi_sqr: true,
        }
    }
}

/// The usable points: indices kept, with x, y and weight.
struct Points {
    index: Vec<usize>,
    x: Vec<f64>,
    y: Vec<f64>,
    w: Vec<f64>,
}

fn collect_points(x: &[f64], y: &[f64], weighting: &Weighting) -> Result<Points, FitError> {
    if x.len() != y.len() {
        return Err(FitError::LengthMismatch { x: x.len(), y: y.len() });
    }
    let side: Option<&Vec<f64>> = match weighting {
        Weighting::Instrumental(v) | Weighting::Direct(v) => Some(v),
        _ => None,
    };
    if let Some(v) = side
        && v.len() != x.len()
    {
        return Err(FitError::LengthMismatch { x: x.len(), y: v.len() });
    }
    let mut p = Points { index: Vec::new(), x: Vec::new(), y: Vec::new(), w: Vec::new() };
    for i in 0..x.len() {
        let w = match weighting {
            Weighting::None => 1.0,
            Weighting::Instrumental(e) => 1.0 / (e[i] * e[i]),
            // Origin weights by 1 / |y| (checked with a negative y); a y of zero is refused.
            Weighting::Statistical => 1.0 / y[i].abs(),
            Weighting::Direct(w) => w[i],
        };
        if x[i].is_nan() || y[i].is_nan() || w.is_nan() {
            continue;
        }
        if x[i].is_infinite() || y[i].is_infinite() {
            return Err(FitError::Failed(
                "x and y must be finite numbers (use a missing value to leave a point out)".to_string(),
            ));
        }
        if w <= 0.0 || w.is_infinite() {
            return Err(FitError::Failed(
                "weights must be positive and finite (an error of zero or a statistical \
                 weight from a y that is not positive cannot be used)"
                    .to_string(),
            ));
        }
        p.index.push(i);
        p.x.push(x[i]);
        p.y.push(y[i]);
        p.w.push(w);
    }
    Ok(p)
}

/// The confidence level must lie strictly between 0 and 1; otherwise the limits would be NaN.
fn check_level(level: f64) -> Result<(), FitError> {
    if level > 0.0 && level < 1.0 {
        Ok(())
    } else {
        Err(FitError::Failed(format!("the confidence level must be strictly between 0 and 1, not {level}")))
    }
}

/// Fit a straight line `y = Intercept + Slope * x`.
pub fn fit_linear(x: &[f64], y: &[f64], opts: &LinearOptions) -> Result<FitReport, FitError> {
    fit_linear_with_curve(x, y, opts).map(|(report, _)| report)
}

/// Like [`fit_linear`], and also returns the fitted line as a curve evaluated in the solved basis.
pub fn fit_linear_with_curve(x: &[f64], y: &[f64], opts: &LinearOptions) -> Result<(FitReport, FittedCurve), FitError> {
    check_level(opts.confidence_level)?;
    let pts = collect_points(x, y, &opts.weighting)?;
    let fixed = [opts.fix_intercept, opts.fix_slope];
    let solved = solve(&pts, 1, &fixed, opts.scale_errors_by_reduced_chi_sqr)?;
    let curve = FittedCurve::new(solved.basis.clone(), solved.dof, solved.rss);
    let report = build_report(
        "Linear".to_string(),
        "y = Intercept + Slope*x".to_string(),
        vec!["Intercept".to_string(), "Slope".to_string()],
        &pts,
        x.len(),
        solved,
        opts.confidence_level,
        true,
    );
    Ok((report, curve))
}

/// Fit a polynomial `y = Intercept + B1*x + B2*x^2 + ...` of the given order (at least 1).
pub fn fit_polynomial(x: &[f64], y: &[f64], order: usize, opts: &PolyOptions) -> Result<FitReport, FitError> {
    fit_polynomial_with_curve(x, y, order, opts).map(|(report, _)| report)
}

/// Like [`fit_polynomial`], and also returns the fitted polynomial as a curve evaluated in the
/// solved (centred or scaled) basis, which is accurate where the power-of-x coefficients are not.
pub fn fit_polynomial_with_curve(
    x: &[f64],
    y: &[f64],
    order: usize,
    opts: &PolyOptions,
) -> Result<(FitReport, FittedCurve), FitError> {
    if order == 0 {
        return Err(FitError::BadFunction("a polynomial needs order 1 or more".to_string()));
    }
    check_level(opts.confidence_level)?;
    let pts = collect_points(x, y, &opts.weighting)?;
    let mut fixed = vec![None; order + 1];
    fixed[0] = opts.fix_intercept;
    let solved = solve(&pts, order, &fixed, opts.scale_errors_by_reduced_chi_sqr)?;
    let curve = FittedCurve::new(solved.basis.clone(), solved.dof, solved.rss);
    let mut names = vec!["Intercept".to_string()];
    let mut eq = String::from("y = Intercept");
    for k in 1..=order {
        names.push(format!("B{k}"));
        eq.push_str(&format!(" + B{k}*x^{k}"));
    }
    let report = build_report(
        format!("Polynomial (order {order})"),
        eq,
        names,
        &pts,
        x.len(),
        solved,
        opts.confidence_level,
        order == 1,
    );
    Ok((report, curve))
}

/// The fitted polynomial as a function of x, evaluated in the basis the fit was solved in
/// (centred and scaled x), not from the coefficients in powers of x, which lose digits when x is
/// far from zero relative to its spread.
#[derive(Debug, Clone)]
pub struct FittedCurve {
    basis: Basis,
    dof: usize,
    /// Reduced chi-square: the weighted residual sum of squares over the degrees of freedom.
    reduced_chi_sqr: f64,
}

impl FittedCurve {
    fn new(basis: Basis, dof: usize, rss: f64) -> Self {
        FittedCurve { basis, dof, reduced_chi_sqr: if dof > 0 { rss / dof as f64 } else { f64::NAN } }
    }

    pub fn eval(&self, x: f64) -> f64 {
        self.basis.eval(x)
    }

    /// Confidence and prediction bands of the fitted line or polynomial at the given x values,
    /// at confidence `level` (0.95 for 95 %). See [`crate::bands`] for the formulas. The bands
    /// always use the covariance scaled by the reduced chi-square, whatever the option for the
    /// parameter errors was, as Origin does.
    pub fn bands(&self, xs: &[f64], level: f64) -> Result<Bands, FitError> {
        check_level(level)?;
        let mean: Vec<f64> = xs.iter().map(|&x| self.basis.eval(x)).collect();
        let var: Vec<f64> = xs.iter().map(|&x| self.basis.gradient_variance(x) * self.reduced_chi_sqr).collect();
        Ok(Bands::from_variances(xs, &mean, &var, self.reduced_chi_sqr, self.dof, level))
    }
}

/// How the solved coefficients are turned back into curve values.
#[derive(Debug, Clone)]
enum Basis {
    /// `y = sum c_j u^j` with `u = (x - centre) / spread`; every coefficient was free.
    Centred { centre: f64, spread: f64, c: Vec<f64>, cov: Vec<f64> },
    /// `y = fixed part + sum c_j (x / scale)^k_j` over the free powers `k_j`.
    Scaled { scale: f64, free: Vec<usize>, c: Vec<f64>, fixed: Vec<(usize, f64)>, cov: Vec<f64> },
}

impl Basis {
    /// `g' C g` for the gradient `g` of the curve value with respect to the solved coefficients and
    /// their covariance `C` before any scaling by the reduced chi-square.
    fn gradient_variance(&self, x: f64) -> f64 {
        let (g, cov): (Vec<f64>, &Vec<f64>) = match self {
            Basis::Centred { centre, spread, c, cov } => {
                let u = (x - centre) / spread;
                (std::iter::successors(Some(1.0), |p| Some(p * u)).take(c.len()).collect(), cov)
            }
            Basis::Scaled { scale, free, cov, .. } => {
                let t = x / scale;
                (free.iter().map(|&k| t.powi(k as i32)).collect(), cov)
            }
        };
        let m = g.len();
        let mut v = 0.0;
        for i in 0..m {
            for j in 0..m {
                v += g[i] * cov[i + j * m] * g[j];
            }
        }
        v
    }

    fn eval(&self, x: f64) -> f64 {
        match self {
            Basis::Centred { centre, spread, c, .. } => {
                let u = (x - centre) / spread;
                c.iter().rev().fold(0.0, |s, cj| s * u + cj)
            }
            Basis::Scaled { scale, free, c, fixed, .. } => {
                let t = x / scale;
                let mut s: f64 = fixed.iter().map(|(k, v)| v * x.powi(*k as i32)).sum();
                for (cj, k) in c.iter().zip(free) {
                    s += cj * t.powi(*k as i32);
                }
                s
            }
        }
    }
}

/// Result of the numerical part, before it is laid out as a report.
struct Solved {
    /// All coefficients in powers of the original x, fixed ones included.
    coef: Vec<f64>,
    /// Covariance of all coefficients (fixed ones zero), row-major, after the optional scaling.
    cov: Vec<f64>,
    fixed: Vec<bool>,
    basis: Basis,
    dof: usize,
    /// Weighted residual sum of squares.
    rss: f64,
    /// Total sum of squares: about the weighted mean of y, or about zero when the intercept is
    /// held fixed.
    tss: f64,
    /// Degrees of freedom of the total sum of squares.
    total_df: usize,
    /// Weighted sum of y squared: the scale against which `tss` is judged to be zero.
    tss_scale: f64,
}

/// Weighted least squares of a polynomial of `order`; `fixed[k]` holds coefficient k at a value.
fn solve(pts: &Points, order: usize, fixed: &[Option<f64>], scale_by_chi: bool) -> Result<Solved, FitError> {
    let n = pts.x.len();
    let m = order + 1;
    let free: Vec<usize> = (0..m).filter(|k| fixed[*k].is_none()).collect();
    let nfree = free.len();
    if nfree == 0 {
        return Err(FitError::BadFunction("at least one parameter must be left free".to_string()));
    }
    if n < nfree {
        return Err(FitError::TooFewPoints { have: n, need: nfree });
    }
    let dof = n - nfree;
    let sw: f64 = pts.w.iter().sum();
    let sqrt_w: Vec<f64> = pts.w.iter().map(|w| w.sqrt()).collect();
    let fixed_list: Vec<(usize, f64)> = (0..m).filter_map(|k| fixed[k].map(|v| (k, v))).collect();

    // Weighted mean of y, and (when every coefficient is free) of x.
    let ym: f64 = pts.y.iter().zip(&pts.w).map(|(y, w)| y * w).sum::<f64>() / sw;
    let (basis, coef, cov_unscaled) = if fixed_list.is_empty() {
        let xm: f64 = pts.x.iter().zip(&pts.w).map(|(x, w)| x * w).sum::<f64>() / sw;
        let sxx: f64 = pts.x.iter().zip(&pts.w).map(|(x, w)| w * (x - xm) * (x - xm)).sum();
        let spread = if sxx > 0.0 { (sxx / sw).sqrt() } else { 1.0 };
        let mut a = DMatrix::<f64>::zeros(n, m);
        for i in 0..n {
            let u = (pts.x[i] - xm) / spread;
            let mut up = 1.0;
            for j in 0..m {
                a[(i, j)] = sqrt_w[i] * up;
                up *= u;
            }
        }
        let b = DVector::from_iterator(n, (0..n).map(|i| sqrt_w[i] * pts.y[i]));
        let (c, cov_u) = solve_qr(&a, &b)?;
        // Powers of u to powers of x: coefficient k of x is
        // sum over j >= k of c_j * C(j, k) * (-centre)^(j-k) / spread^j.
        let mut t = DMatrix::<f64>::zeros(m, m);
        for j in 0..m {
            let mut binom = 1.0;
            for k in 0..=j {
                t[(k, j)] = binom * (-xm).powi((j - k) as i32) / spread.powi(j as i32);
                binom = binom * (j - k) as f64 / (k + 1) as f64;
            }
        }
        let coef: Vec<f64> = (&t * &c).iter().copied().collect();
        let cov = &t * &cov_u * t.transpose();
        (
            Basis::Centred { centre: xm, spread, c: c.iter().copied().collect(), cov: cov_u.iter().copied().collect() },
            coef,
            cov,
        )
    } else {
        let xmax = pts.x.iter().fold(0.0_f64, |acc, x| acc.max(x.abs()));
        let scale = if xmax > 0.0 { xmax } else { 1.0 };
        let mut a = DMatrix::<f64>::zeros(n, nfree);
        for i in 0..n {
            let t = pts.x[i] / scale;
            for (col, &k) in free.iter().enumerate() {
                a[(i, col)] = sqrt_w[i] * t.powi(k as i32);
            }
        }
        let b = DVector::from_iterator(
            n,
            (0..n).map(|i| {
                let off: f64 = fixed_list.iter().map(|(k, v)| v * pts.x[i].powi(*k as i32)).sum();
                sqrt_w[i] * (pts.y[i] - off)
            }),
        );
        let (c, cov_c) = solve_qr(&a, &b)?;
        let mut coef = vec![0.0; m];
        let mut cov = DMatrix::<f64>::zeros(m, m);
        for (col, &k) in free.iter().enumerate() {
            coef[k] = c[col] / scale.powi(k as i32);
            for (col2, &k2) in free.iter().enumerate() {
                cov[(k, k2)] = cov_c[(col, col2)] / (scale.powi(k as i32) * scale.powi(k2 as i32));
            }
        }
        for (k, v) in &fixed_list {
            coef[*k] = *v;
        }
        (
            Basis::Scaled {
                scale,
                free: free.clone(),
                c: c.iter().copied().collect(),
                fixed: fixed_list.clone(),
                cov: cov_c.iter().copied().collect(),
            },
            coef,
            cov,
        )
    };

    let mut rss = 0.0;
    for i in 0..n {
        let r = pts.y[i] - basis.eval(pts.x[i]);
        rss += pts.w[i] * r * r;
    }

    // Origin measures the total about zero when the intercept is held fixed, and about the
    // weighted mean otherwise.
    let intercept_fixed = fixed[0].is_some();
    let (tss, total_df) = if intercept_fixed {
        let t: f64 = pts.y.iter().zip(&pts.w).map(|(y, w)| w * y * y).sum();
        (t, n)
    } else {
        let t: f64 = pts.y.iter().zip(&pts.w).map(|(y, w)| w * (y - ym) * (y - ym)).sum();
        (t, n - 1)
    };

    let mut cov = cov_unscaled;
    if scale_by_chi {
        cov *= if dof > 0 { rss / dof as f64 } else { f64::NAN };
    }
    Ok(Solved {
        coef,
        cov: cov.transpose().iter().copied().collect(),
        fixed: (0..m).map(|k| fixed[k].is_some()).collect(),
        basis,
        dof,
        rss,
        tss,
        total_df,
        tss_scale: pts.y.iter().zip(&pts.w).map(|(y, w)| w * y * y).sum(),
    })
}

/// Least squares of `a * c = b` by QR; returns `c` and `(A'A)^-1`.
fn solve_qr(a: &DMatrix<f64>, b: &DVector<f64>) -> Result<(DVector<f64>, DMatrix<f64>), FitError> {
    let singular =
        || FitError::Failed("the fit is singular: the x values do not determine all the coefficients".to_string());
    let k = a.ncols();
    let qr = a.clone().qr();
    let r = qr.r();
    let rk = r.rows(0, k).into_owned();
    let diag: Vec<f64> = (0..k).map(|i| rk[(i, i)].abs()).collect();
    let diag_min = diag.iter().copied().fold(f64::INFINITY, f64::min);
    let diag_max = diag.iter().copied().fold(0.0, f64::max);
    if diag_min.is_nan() || diag_min <= diag_max * 1e-13 {
        return Err(singular());
    }
    let qtb = qr.q().transpose() * b;
    let rhs = qtb.rows(0, k).into_owned();
    let c = rk.clone().solve_upper_triangular(&rhs).ok_or_else(singular)?;
    let rinv = rk.solve_upper_triangular(&DMatrix::identity(k, k)).ok_or_else(singular)?;
    let cov = &rinv * rinv.transpose();
    Ok((c, cov))
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    model: String,
    equation: String,
    names: Vec<String>,
    pts: &Points,
    n_input: usize,
    solved: Solved,
    level: f64,
    straight_line: bool,
) -> FitReport {
    let m = names.len();
    let n = pts.x.len();
    let dof = solved.dof;
    let tcrit = t_critical(level, dof);
    let params: Vec<Parameter> = (0..m)
        .map(|k| {
            let value = solved.coef[k];
            if solved.fixed[k] {
                Parameter {
                    name: names[k].clone(),
                    value,
                    std_error: 0.0,
                    fixed: true,
                    t_value: f64::NAN,
                    prob_t: f64::NAN,
                    lcl: f64::NAN,
                    ucl: f64::NAN,
                    dependency: f64::NAN,
                }
            } else {
                let var = solved.cov[k * m + k];
                let se = if var.is_nan() { f64::NAN } else { var.max(0.0).sqrt() };
                let t = value / se;
                Parameter {
                    name: names[k].clone(),
                    value,
                    std_error: se,
                    fixed: false,
                    t_value: t,
                    prob_t: t_two_sided_p(t, dof),
                    lcl: value - tcrit * se,
                    ucl: value + tcrit * se,
                    dependency: f64::NAN,
                }
            }
        })
        .collect();

    let rss = solved.rss;
    let reduced = if dof > 0 { rss / dof as f64 } else { f64::NAN };
    // A total sum of squares that is zero (or only rounding noise relative to the data) means y
    // does not vary: R-square is undefined. Origin's behaviour here was not checked.
    let degenerate = solved.tss.is_nan() || solved.tss <= solved.tss_scale * 1e-24;
    let (r2, adj) = if degenerate {
        (f64::NAN, f64::NAN)
    } else {
        (1.0 - rss / solved.tss, 1.0 - reduced / (solved.tss / solved.total_df as f64))
    };
    let model_df = solved.total_df - dof;
    let model_ss = solved.tss - rss;
    let (model_ms, f, prob_f) = if model_df > 0 && dof > 0 {
        let ms = model_ss / model_df as f64;
        let f = ms / reduced;
        (ms, f, f_upper_p(f, model_df, dof))
    } else {
        (f64::NAN, f64::NAN, f64::NAN)
    };
    let anova = Anova {
        model_df,
        model_ss,
        model_ms,
        error_df: dof,
        error_ss: rss,
        error_ms: reduced,
        total_df: solved.total_df,
        total_ss: solved.tss,
        uncorrected_total_ss: f64::NAN,
        f_value: f,
        prob_f,
    };

    let mut fitted = vec![f64::NAN; n_input];
    let mut residuals = vec![f64::NAN; n_input];
    for (j, &i) in pts.index.iter().enumerate() {
        let fv = solved.basis.eval(pts.x[j]);
        fitted[i] = fv;
        residuals[i] = pts.y[j] - fv;
    }
    // Pearson's r: the square root of R-square with the sign of the slope. For a free
    // straight line this is the usual correlation coefficient; Origin reports the same
    // definition when a parameter is held fixed.
    let pearson_r = if straight_line {
        let sign = if solved.coef[1] < 0.0 { -1.0 } else { 1.0 };
        Some(sign * r2.sqrt())
    } else {
        None
    };
    let mut warnings = Vec::new();
    if degenerate {
        warnings
            .push("the y values do not vary, so R-square, adjusted R-square and Pearson's r are undefined".to_string());
    }
    if dof == 0 {
        warnings.push("as many free parameters as points: the errors and probabilities are undefined".to_string());
    }
    FitReport {
        model,
        equation,
        params,
        stats: Statistics {
            n_points: n,
            dof,
            rss,
            reduced_chi_sqr: reduced,
            r_squared: r2,
            adj_r_squared: adj,
            pearson_r,
            iterations: None,
        },
        anova: Some(anova),
        covariance: solved.cov,
        confidence_level: level,
        fitted,
        residuals,
        status: FitStatus::Exact,
        warnings,
    }
}
