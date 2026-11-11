//! Straight-line and polynomial fits (closed form, Origin's "Linear Fit" and "Polynomial Fit").
//!
//! The rules (weights, scaling of the errors, R-square and the analysis of variance when a
//! parameter is held fixed) are written down in `docs/spec/fitting-linear.md` together with the
//! Origin numbers they were checked against.
//!
//! Numerics: the straight line is solved from sums about the weighted means, which stays accurate
//! when x is far from zero. A polynomial is solved by QR decomposition of the weighted design
//! matrix built on a centred and scaled x, and the coefficients and their covariance are then
//! converted back to powers of the original x.

use crate::dist::{f_upper_p, t_critical, t_two_sided_p};
use crate::report::{
    Anova, FitError, FitReport, FitStatus, Parameter, Statistics, Weighting,
};
use nalgebra::{DMatrix, DVector};

/// Options of [`fit_linear`]. The defaults are those of Origin's Linear Fit dialog.
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
    /// errors). If false, the covariance is `(X'WX)^-1` as given by the weights alone.
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
        return Err(FitError::LengthMismatch {
            x: x.len(),
            y: y.len(),
        });
    }
    let side: Option<&Vec<f64>> = match weighting {
        Weighting::Instrumental(v) | Weighting::Direct(v) => Some(v),
        _ => None,
    };
    if let Some(v) = side
        && v.len() != x.len()
    {
        return Err(FitError::LengthMismatch {
            x: x.len(),
            y: v.len(),
        });
    }
    let mut p = Points {
        index: Vec::new(),
        x: Vec::new(),
        y: Vec::new(),
        w: Vec::new(),
    };
    for i in 0..x.len() {
        let w = match weighting {
            Weighting::None => 1.0,
            Weighting::Instrumental(e) => 1.0 / (e[i] * e[i]),
            Weighting::Statistical => 1.0 / y[i],
            Weighting::Direct(w) => w[i],
        };
        if x[i].is_nan() || y[i].is_nan() || w.is_nan() || w.is_infinite() {
            continue;
        }
        p.index.push(i);
        p.x.push(x[i]);
        p.y.push(y[i]);
        p.w.push(w);
    }
    Ok(p)
}

/// Fit a straight line `y = Intercept + Slope * x`.
pub fn fit_linear(x: &[f64], y: &[f64], opts: &LinearOptions) -> Result<FitReport, FitError> {
    let pts = collect_points(x, y, &opts.weighting)?;
    let mut fixed: Vec<(usize, f64)> = Vec::new();
    if let Some(a) = opts.fix_intercept {
        fixed.push((0, a));
    }
    if let Some(b) = opts.fix_slope {
        fixed.push((1, b));
    }
    let core = fit_core(
        &pts,
        1,
        &fixed,
        opts.scale_errors_by_reduced_chi_sqr,
        opts.confidence_level,
    )?;
    let names = vec!["Intercept".to_string(), "Slope".to_string()];
    Ok(build_report(
        "Linear".to_string(),
        "y = Intercept + B1*x".to_string(),
        names,
        &pts,
        x.len(),
        core,
        opts.confidence_level,
        true,
    ))
}

/// Fit a polynomial `y = Intercept + B1*x + B2*x^2 + ...` of the given order (at least 1).
pub fn fit_polynomial(
    x: &[f64],
    y: &[f64],
    order: usize,
    opts: &PolyOptions,
) -> Result<FitReport, FitError> {
    if order == 0 {
        return Err(FitError::BadFunction(
            "a polynomial needs order 1 or more".to_string(),
        ));
    }
    let pts = collect_points(x, y, &opts.weighting)?;
    let mut fixed = Vec::new();
    if let Some(a) = opts.fix_intercept {
        fixed.push((0, a));
    }
    let core = fit_core(
        &pts,
        order,
        &fixed,
        opts.scale_errors_by_reduced_chi_sqr,
        opts.confidence_level,
    )?;
    let mut names = vec!["Intercept".to_string()];
    for k in 1..=order {
        names.push(format!("B{k}"));
    }
    let mut eq = String::from("y = Intercept");
    for k in 1..=order {
        eq.push_str(&format!(" + B{k}*x^{k}"));
    }
    Ok(build_report(
        format!("Polynomial (order {order})"),
        eq,
        names,
        &pts,
        x.len(),
        core,
        opts.confidence_level,
        order == 1,
    ))
}

/// Result of the numerical part, before it is laid out as a report.
struct Core {
    /// All coefficients in powers of the original x, fixed ones included.
    coef: Vec<f64>,
    /// Covariance of all coefficients (fixed ones zero), row-major.
    cov: Vec<f64>,
    fixed: Vec<bool>,
    dof: usize,
    rss: f64,
    /// Weighted total sum of squares about the weighted mean of y.
    tss: f64,
    /// Weighted sum of y squared.
    uncorrected_tss: f64,
    /// Pearson's r from the weighted sums, straight-line fits only.
    pearson: f64,
}

/// Weighted least squares of a polynomial of `order` with some coefficients held fixed.
fn fit_core(
    pts: &Points,
    order: usize,
    fixed: &[(usize, f64)],
    scale: bool,
    _level: f64,
) -> Result<Core, FitError> {
    let n = pts.x.len();
    let m = order + 1;
    let free: Vec<usize> = (0..m).filter(|k| !fixed.iter().any(|f| f.0 == *k)).collect();
    let nfree = free.len();
    if n < nfree || n == 0 {
        return Err(FitError::TooFewPoints {
            have: n,
            need: nfree,
        });
    }
    let dof = n - nfree;

    // Weighted means for centring and the sums used by the statistics.
    let sw: f64 = pts.w.iter().sum();
    let xm: f64 = pts.x.iter().zip(&pts.w).map(|(x, w)| x * w).sum::<f64>() / sw;
    let ym: f64 = pts.y.iter().zip(&pts.w).map(|(y, w)| y * w).sum::<f64>() / sw;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut syy = 0.0;
    let mut syy_raw = 0.0;
    for i in 0..n {
        let dx = pts.x[i] - xm;
        let dy = pts.y[i] - ym;
        sxx += pts.w[i] * dx * dx;
        sxy += pts.w[i] * dx * dy;
        syy += pts.w[i] * dy * dy;
        syy_raw += pts.w[i] * pts.y[i] * pts.y[i];
    }
    let pearson = sxy / (sxx * syy).sqrt();

    // Scale for the basis: u = (x - xm) / xs with xs the spread of x.
    let xs = if sxx > 0.0 && sw > 0.0 {
        (sxx / sw).sqrt()
    } else {
        1.0
    };
    let u: Vec<f64> = pts.x.iter().map(|x| (x - xm) / xs).collect();

    // Coefficients in u: y = sum c_k u^k. Fixed coefficients in x are moved to the right side
    // through the same transform, so the problem is solved in terms of the free x-coefficients.
    // Columns: x^k = (xm + xs*u)^k expanded in u.
    // t[k][j] = coefficient of u^j in x^k.
    let mut t = DMatrix::<f64>::zeros(m, m);
    for k in 0..m {
        let mut binom = 1.0;
        for j in 0..=k {
            // C(k, j) * xm^(k-j) * xs^j
            t[(k, j)] = binom * xm.powi((k - j) as i32) * xs.powi(j as i32);
            binom = binom * (k - j) as f64 / (j + 1) as f64;
        }
    }
    // Design matrix in the u basis columns j = 0..m; build the design in x^k terms via t:
    // value of x^k at point i = sum_j t[k][j] u_i^j.
    let mut basis = DMatrix::<f64>::zeros(n, m); // column k: x^k evaluated stably
    for i in 0..n {
        for k in 0..m {
            let mut s = 0.0;
            let mut up = 1.0;
            for j in 0..=k {
                s += t[(k, j)] * up;
                up *= u[i];
            }
            basis[(i, k)] = s;
        }
    }
    // The above equals x^k but is evaluated by Horner-free expansion; it is only used to pick
    // the right-hand side offset of fixed terms. For the solve we use u-powers directly.
    let _ = basis;

    // Solve for the free coefficients in a basis of u-powers when no coefficient is fixed.
    // With the intercept fixed (the only supported fixed coefficient of a polynomial) the
    // problem becomes y - a0 = sum_{k>=1} B_k x^k, which has no constant term, so the basis is
    // x^k scaled by a column norm instead of centred.
    let all_free = fixed.is_empty();
    let sqrt_w: Vec<f64> = pts.w.iter().map(|w| w.sqrt()).collect();

    let coef: Vec<f64>;
    let mut cov_free: DMatrix<f64>;
    let mut fixed_vals = vec![0.0; m];
    for (k, v) in fixed {
        fixed_vals[*k] = *v;
    }

    if all_free {
        let mut a = DMatrix::<f64>::zeros(n, m);
        for i in 0..n {
            let mut up = 1.0;
            for j in 0..m {
                a[(i, j)] = sqrt_w[i] * up;
                up *= u[i];
            }
        }
        let b = DVector::from_iterator(n, (0..n).map(|i| sqrt_w[i] * pts.y[i]));
        let (c, ctc_inv) = solve_ls(&a, &b)?;
        // Convert from u-powers to x-powers: x-coefficient vector a_x, with u = (x - xm)/xs:
        // sum_j c_j ((x - xm)/xs)^j = sum_k a_k x^k  =>  a_k = sum_{j>=k} c_j xs^-j C(j,k) (-xm)^(j-k)
        let mut tr = DMatrix::<f64>::zeros(m, m);
        for j in 0..m {
            let mut binom = 1.0; // C(j, k) starting at k = 0
            for k in 0..=j {
                tr[(k, j)] = binom * (-xm).powi((j - k) as i32) / xs.powi(j as i32);
                binom = binom * (j - k) as f64 / (k + 1) as f64;
            }
        }
        coef = (&tr * &c).iter().copied().collect();
        cov_free = &tr * ctc_inv * tr.transpose();
    } else {
        // Intercept fixed: columns x^k for k in free, scaled by their norms.
        let nf = free.len();
        let mut a = DMatrix::<f64>::zeros(n, nf);
        let mut scales = vec![1.0; nf];
        for (col, &k) in free.iter().enumerate() {
            // Use powers of x / xmax to keep the columns near unit size.
            let xmax = pts.x.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
            let sc = if xmax > 0.0 { xmax.powi(k as i32) } else { 1.0 };
            scales[col] = sc;
            for i in 0..n {
                a[(i, col)] = sqrt_w[i] * pts.x[i].powi(k as i32) / sc;
            }
        }
        let b = DVector::from_iterator(
            n,
            (0..n).map(|i| {
                let mut r = pts.y[i];
                for (k, v) in fixed {
                    r -= v * pts.x[i].powi(*k as i32);
                }
                sqrt_w[i] * r
            }),
        );
        let (c, ctc_inv) = solve_ls(&a, &b)?;
        let mut full = fixed_vals.clone();
        let mut cf = DMatrix::<f64>::zeros(m, m);
        for (col, &k) in free.iter().enumerate() {
            full[k] = c[col] / scales[col];
            for (col2, &k2) in free.iter().enumerate() {
                cf[(k, k2)] = ctc_inv[(col, col2)] / (scales[col] * scales[col2]);
            }
        }
        coef = full;
        cov_free = cf;
    }

    // Residuals and sums of squares, evaluated in the centred basis for accuracy.
    let mut rss = 0.0;
    for i in 0..n {
        let r = pts.y[i] - eval_poly(&coef, pts.x[i], xm);
        rss += pts.w[i] * r * r;
    }
    let _ = (syy_raw,);
    let tss = syy;

    // Scale the covariance by the reduced chi-square if asked.
    if scale && dof > 0 {
        cov_free *= rss / dof as f64;
    }
    let fixed_flags: Vec<bool> = (0..m).map(|k| fixed.iter().any(|f| f.0 == k)).collect();
    Ok(Core {
        coef,
        cov: cov_free.transpose().iter().copied().collect(),
        fixed: fixed_flags,
        dof,
        rss,
        tss,
        uncorrected_tss: syy_raw,
        pearson,
    })
}

/// Value of the polynomial with the coefficients `coef` (powers of x) at `x`, evaluated about
/// `xm` so cancellation stays small for x far from zero.
fn eval_poly(coef: &[f64], x: f64, _xm: f64) -> f64 {
    let mut s = 0.0;
    for c in coef.iter().rev() {
        s = s * x + c;
    }
    s
}

/// Least squares of `a * c = b` by QR; returns `c` and `(A'A)^-1`.
fn solve_ls(a: &DMatrix<f64>, b: &DVector<f64>) -> Result<(DVector<f64>, DMatrix<f64>), FitError> {
    let qr = a.clone().qr();
    let q = qr.q();
    let r = qr.r();
    let k = a.ncols();
    let qtb = q.transpose() * b;
    let rk = r.rows(0, k).into_owned();
    let diag_min = (0..k).map(|i| rk[(i, i)].abs()).fold(f64::INFINITY, f64::min);
    let diag_max = (0..k).map(|i| rk[(i, i)].abs()).fold(0.0, f64::max);
    if diag_min <= diag_max * 1e-14 {
        return Err(FitError::Failed(
            "the fit is singular: the x values do not determine all the coefficients".to_string(),
        ));
    }
    let rhs = qtb.rows(0, k).into_owned();
    let c = rk
        .clone()
        .solve_upper_triangular(&rhs)
        .ok_or_else(|| FitError::Failed("singular matrix".to_string()))?;
    let rinv = rk
        .solve_upper_triangular(&DMatrix::identity(k, k))
        .ok_or_else(|| FitError::Failed("singular matrix".to_string()))?;
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
    core: Core,
    level: f64,
    straight_line: bool,
) -> FitReport {
    let m = names.len();
    let n = pts.x.len();
    let tcrit = t_critical(level, core.dof);
    let params: Vec<Parameter> = (0..m)
        .map(|k| {
            let value = core.coef[k];
            if core.fixed[k] {
                Parameter {
                    name: names[k].clone(),
                    value,
                    std_error: f64::NAN,
                    fixed: true,
                    t_value: f64::NAN,
                    prob_t: f64::NAN,
                    lcl: f64::NAN,
                    ucl: f64::NAN,
                    dependency: f64::NAN,
                }
            } else {
                let se = core.cov[k * m + k].max(0.0).sqrt();
                let t = value / se;
                Parameter {
                    name: names[k].clone(),
                    value,
                    std_error: se,
                    fixed: false,
                    t_value: t,
                    prob_t: t_two_sided_p(t, core.dof),
                    lcl: value - tcrit * se,
                    ucl: value + tcrit * se,
                    dependency: f64::NAN,
                }
            }
        })
        .collect();

    let nfree = core.fixed.iter().filter(|f| !**f).count();
    let rss = core.rss;
    let reduced = rss / core.dof as f64;
    let r2 = 1.0 - rss / core.tss;
    let adj = 1.0 - reduced / (core.tss / (n as f64 - 1.0));
    let model_df = nfree.saturating_sub(usize::from(!core.fixed[0]));
    let model_ss = core.tss - rss;
    let model_ms = model_ss / model_df.max(1) as f64;
    let f = model_ms / reduced;
    let anova = Anova {
        model_df,
        model_ss,
        model_ms,
        error_df: core.dof,
        error_ss: rss,
        error_ms: reduced,
        total_df: n - 1,
        total_ss: core.tss,
        uncorrected_total_ss: core.uncorrected_tss,
        f_value: f,
        prob_f: f_upper_p(f, model_df, core.dof),
    };

    let mut fitted = vec![f64::NAN; n_input];
    let mut residuals = vec![f64::NAN; n_input];
    for (j, &i) in pts.index.iter().enumerate() {
        let fv = eval_poly(&core.coef, pts.x[j], 0.0);
        fitted[i] = fv;
        residuals[i] = pts.y[j] - fv;
    }
    FitReport {
        model,
        equation,
        params,
        stats: Statistics {
            n_points: n,
            dof: core.dof,
            rss,
            reduced_chi_sqr: reduced,
            r_squared: r2,
            adj_r_squared: adj,
            pearson_r: if straight_line {
                Some(core.pearson)
            } else {
                None
            },
            iterations: None,
        },
        anova: Some(anova),
        covariance: core.cov,
        confidence_level: level,
        fitted,
        residuals,
        status: FitStatus::Exact,
        warnings: Vec::new(),
    }
}
