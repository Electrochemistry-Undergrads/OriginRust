//! Linear quantile regression (Origin's Quantile Regression app, Origin 2019b; not in Origin 2016, so
//! nothing here is measured in Origin). For a quantile `tau` in (0, 1) the coefficients minimize the
//! sum of the check loss `rho(u) = u (tau - [u < 0])` of the residuals `u = y - X b` (Koenker and
//! Bassett 1978); `tau = 0.5` is least absolute deviations.
//!
//! The problem is the linear programme of Koenker's dual, `max y'a` subject to `X'a = (1 - tau) X'1`
//! and `0 <= a <= 1`, solved with the Frisch-Newton interior-point method of Portnoy and Koenker
//! (1997, the `fnb` method of R's `quantreg`). The interior-point answer is then moved to the vertex
//! through the `p` observations with the smallest residuals when that vertex has the smaller loss, so
//! the coefficients are the exact solution of the linear programme (checked against
//! `scipy.optimize.linprog` in `crates/calc/tests/scipy_quantreg.rs`). When the solution is not unique
//! (for example ties in `y`) any optimal vertex may be returned.
//!
//! **Standard errors** (T494; as statsmodels' `QuantReg` and Stata 12's `qreg` compute them, applied to the exact
//! coefficients): the density of the residuals at 0 is a kernel estimate `f = sum K(e_i / h) / (n h)` with the
//! Epanechnikov kernel `K(u) = 3/4 (1 - u^2)` on `|u| <= 1` and the bandwidth
//! `h = min(sd(y), IQR(e) / 1.34) (Phi^-1(tau + hs) - Phi^-1(tau - hs))`, where `hs` is Hall and Sheather's
//! `n^(-1/3) z_0.975^(2/3) (1.5 phi(Phi^-1(tau))^2 / (2 Phi^-1(tau)^2 + 1))^(1/3)`, `sd` divides by `n` and the
//! quartiles interpolate at `(n - 1) p`. Robust (the default): `V = (X'X)^-1 X'DX (X'X)^-1`, `D` holding
//! `(tau / f)^2` for a positive residual and `((1 - tau) / f)^2` otherwise (a residual within 1e-9 of the largest
//! `|y|` counts as 0); iid: `V = tau (1 - tau) / f^2 (X'X)^-1`. `Prob>|t|` is two-sided on `n - p` degrees of
//! freedom. When `tau +- hs` leaves (0, 1) (few points at an extreme quantile) or `f` is 0 the errors are NaN.

use nalgebra::{DMatrix, DVector};
use statrs::distribution::{Continuous, ContinuousCDF, Normal, StudentsT};

/// Why a quantile regression could not be computed.
#[derive(Debug, Clone, PartialEq)]
pub enum QuantRegError {
    BadQuantile(f64),
    TooFewPoints { have: usize, need: usize },
    Singular,
    LengthMismatch,
}

impl std::fmt::Display for QuantRegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuantRegError::BadQuantile(t) => write!(f, "the quantile must lie between 0 and 1 (got {t})"),
            QuantRegError::TooFewPoints { have, need } => {
                write!(f, "not enough points: {have} usable points for {need} coefficients")
            }
            QuantRegError::Singular => f.write_str("the independent variables are linearly dependent (or constant)"),
            QuantRegError::LengthMismatch => f.write_str("the X columns and Y have different lengths"),
        }
    }
}

impl std::error::Error for QuantRegError {}

/// The result of one quantile regression.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantReg {
    pub tau: f64,
    /// The intercept (when fitted) followed by one coefficient per X column.
    pub coefficients: Vec<f64>,
    /// Sum of the check losses of the residuals at the solution.
    pub loss: f64,
    /// Koenker and Machado's pseudo R-square `1 - loss / loss0`, `loss0` the loss of the model with
    /// the intercept alone (the `tau` sample quantile); NaN without an intercept.
    pub pseudo_r2: f64,
    /// Points used (rows with a missing X or Y are left out).
    pub n: usize,
    /// Interior-point iterations.
    pub iterations: usize,
    /// The standard error of each coefficient (T494; NaN when it cannot be estimated).
    pub se: Vec<f64>,
    /// Two-sided `Prob>|t|` of each coefficient, `t = b / se` on `n - p` degrees of freedom.
    pub p_values: Vec<f64>,
    /// The bandwidth `h` of the density estimate behind the errors.
    pub bandwidth: f64,
}

/// How the standard errors are estimated (T494).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeKind {
    /// Heteroskedasticity robust (the sandwich): statsmodels' default.
    #[default]
    Robust,
    /// For identically distributed errors (Stata's default).
    Iid,
}

/// The check loss `rho_tau(u)`.
pub fn check_loss(tau: f64, u: f64) -> f64 {
    if u < 0.0 { u * (tau - 1.0) } else { u * tau }
}

fn total_loss(tau: f64, x: &DMatrix<f64>, y: &DVector<f64>, b: &DVector<f64>) -> f64 {
    (y - x * b).iter().map(|&u| check_loss(tau, u)).sum()
}

/// The loss of the best constant: the `tau` sample quantile (any value between the two order
/// statistics around `n tau` has the same loss; the lower one is used).
fn constant_loss(tau: f64, y: &[f64]) -> f64 {
    let mut s = y.to_vec();
    s.sort_by(f64::total_cmp);
    let k = ((s.len() as f64 * tau).ceil() as usize).clamp(1, s.len()) - 1;
    s.iter().map(|&v| check_loss(tau, v - s[k])).sum()
}

/// Smallest step in (0, inf) that keeps `v + t dv >= 0`; a large number when no component falls.
fn step_bound(v: &DVector<f64>, dv: &DVector<f64>) -> f64 {
    v.iter().zip(dv.iter()).filter(|(_, d)| **d < 0.0).map(|(v, d)| -v / d).fold(1e20, f64::min)
}

/// Quantile regression of `y` on the columns `xs` (each as long as `y`) at quantile `tau`, with an
/// intercept when `intercept` is true. Rows where any value is not finite are left out. Robust standard errors.
pub fn quantile_regression(xs: &[&[f64]], y: &[f64], tau: f64, intercept: bool) -> Result<QuantReg, QuantRegError> {
    quantile_regression_with(xs, y, tau, intercept, SeKind::Robust)
}

/// [`quantile_regression`] with the standard errors of `kind` (T494).
pub fn quantile_regression_with(
    xs: &[&[f64]],
    y: &[f64],
    tau: f64,
    intercept: bool,
    kind: SeKind,
) -> Result<QuantReg, QuantRegError> {
    if !(tau > 0.0 && tau < 1.0) {
        return Err(QuantRegError::BadQuantile(tau));
    }
    if xs.iter().any(|c| c.len() != y.len()) {
        return Err(QuantRegError::LengthMismatch);
    }
    let rows: Vec<usize> = (0..y.len()).filter(|&i| y[i].is_finite() && xs.iter().all(|c| c[i].is_finite())).collect();
    let p = xs.len() + usize::from(intercept);
    let n = rows.len();
    if p == 0 || n < p {
        return Err(QuantRegError::TooFewPoints { have: n, need: p.max(1) });
    }
    let x =
        DMatrix::from_fn(n, p, |r, c| if intercept && c == 0 { 1.0 } else { xs[c - usize::from(intercept)][rows[r]] });
    let yv = DVector::from_iterator(n, rows.iter().map(|&i| y[i]));
    if x.clone().svd(false, false).rank(1e-12 * x.norm().max(1.0)) < p {
        return Err(QuantRegError::Singular);
    }
    let (mut b, iterations) = frisch_newton(&x, &yv, tau)?;
    polish(&x, &yv, tau, &mut b);
    let loss = total_loss(tau, &x, &yv, &b);
    let ys: Vec<f64> = yv.iter().copied().collect();
    let pseudo_r2 = if intercept {
        let l0 = constant_loss(tau, &ys);
        if l0 > 0.0 { 1.0 - loss / l0 } else { f64::NAN }
    } else {
        f64::NAN
    };
    let (se, p_values, bandwidth) = standard_errors(&x, &yv, &b, tau, kind);
    Ok(QuantReg {
        tau,
        coefficients: b.iter().copied().collect(),
        loss,
        pseudo_r2,
        n,
        iterations,
        se,
        p_values,
        bandwidth,
    })
}

/// The `p` sample percentile of ascending `s`, interpolated at `(n - 1) p`.
fn percentile(s: &[f64], p: f64) -> f64 {
    let h = (s.len() - 1) as f64 * p;
    let (i, f) = (h.floor() as usize, h - h.floor());
    if i + 1 < s.len() { s[i] + f * (s[i + 1] - s[i]) } else { s[i] }
}

/// The standard errors, `Prob>|t|` values and bandwidth of the fit `b` (T494; see the module notes).
fn standard_errors(
    x: &DMatrix<f64>,
    y: &DVector<f64>,
    b: &DVector<f64>,
    tau: f64,
    kind: SeKind,
) -> (Vec<f64>, Vec<f64>, f64) {
    let (n, p) = (x.nrows(), x.ncols());
    let none = (vec![f64::NAN; p], vec![f64::NAN; p], f64::NAN);
    let normal = Normal::new(0.0, 1.0).expect("the standard normal");
    let z = normal.inverse_cdf(tau);
    let hs = (n as f64).powf(-1.0 / 3.0)
        * normal.inverse_cdf(0.975).powf(2.0 / 3.0)
        * (1.5 * normal.pdf(z).powi(2) / (2.0 * z * z + 1.0)).powf(1.0 / 3.0);
    if !(tau - hs > 0.0 && tau + hs < 1.0) {
        return none;
    }
    let zero = 1e-9 * y.amax();
    let e: Vec<f64> = (y - x * b).iter().map(|&v| if v.abs() < zero { 0.0 } else { v }).collect();
    let mut sorted = e.clone();
    sorted.sort_by(f64::total_cmp);
    let iqr = percentile(&sorted, 0.75) - percentile(&sorted, 0.25);
    let mean = y.mean();
    let sd = (y.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    let h = sd.min(iqr / 1.34) * (normal.inverse_cdf(tau + hs) - normal.inverse_cdf(tau - hs));
    if h.is_nan() || h <= 0.0 {
        return none;
    }
    let kernel = |u: f64| if u.abs() <= 1.0 { 0.75 * (1.0 - u * u) } else { 0.0 };
    let f = e.iter().map(|v| kernel(v / h)).sum::<f64>() / (n as f64 * h);
    let Some(xtx_inv) = (x.transpose() * x).try_inverse() else { return none };
    if f.is_nan() || f <= 0.0 {
        return (vec![f64::NAN; p], vec![f64::NAN; p], h);
    }
    let v = match kind {
        SeKind::Robust => {
            let d = DVector::from_iterator(
                n,
                e.iter().map(|&r| if r > 0.0 { (tau / f).powi(2) } else { ((1.0 - tau) / f).powi(2) }),
            );
            let xd = DMatrix::from_fn(n, p, |i, j| x[(i, j)] * d[i]);
            &xtx_inv * (x.transpose() * xd) * &xtx_inv
        }
        SeKind::Iid => &xtx_inv * (tau * (1.0 - tau) / (f * f)),
    };
    let se: Vec<f64> = (0..p).map(|j| v[(j, j)].max(0.0).sqrt()).collect();
    let df = (n - p) as f64;
    let t = StudentsT::new(0.0, 1.0, df).ok();
    let p_values = se
        .iter()
        .zip(b.iter())
        .map(|(s, b)| match &t {
            Some(t) if *s > 0.0 => 2.0 * (1.0 - t.cdf((b / s).abs())),
            _ => f64::NAN,
        })
        .collect();
    (se, p_values, h)
}

/// The Frisch-Newton interior-point method on Koenker's dual (`lpfnb`): minimize `c'x` with
/// `c = -y`, `A x = b` (`A = X'`, `b = (1 - tau) X'1`) and `0 <= x <= 1`. Returns the coefficients
/// (minus the dual variable of the equality constraints) and the iterations used.
fn frisch_newton(xm: &DMatrix<f64>, yv: &DVector<f64>, tau: f64) -> Result<(DVector<f64>, usize), QuantRegError> {
    const BETA: f64 = 0.99995;
    const EPS: f64 = 1e-12;
    const MAX_IT: usize = 100;
    let n = xm.nrows();
    let a = xm.transpose(); // p x n
    let c = -yv;
    let u = DVector::from_element(n, 1.0);
    let mut x = DVector::from_element(n, 1.0 - tau);
    let b = &a * &x;
    let mut s = &u - &x;
    let solve = |q: &DMatrix<f64>, rhs: &DVector<f64>| -> Result<DVector<f64>, QuantRegError> {
        q.clone().cholesky().map(|ch| ch.solve(rhs)).ok_or(QuantRegError::Singular)
    };
    let aat = &a * a.transpose();
    let mut y = solve(&aat, &(&a * &c))?;
    let r0 = &c - a.transpose() * &y;
    let mut z = r0.map(|v| v.max(0.0));
    let mut w = &z - &r0;
    let scale = 1.0 + c.amax() * n as f64;
    let gap_of = |x: &DVector<f64>, y: &DVector<f64>, w: &DVector<f64>| c.dot(x) - b.dot(y) + u.dot(w);
    let mut gap = gap_of(&x, &y, &w);
    let mut it = 0;
    while gap.abs() > EPS * scale && it < MAX_IT {
        it += 1;
        let q = DVector::from_iterator(n, (0..n).map(|i| 1.0 / (z[i] / x[i] + w[i] / s[i])));
        let r = &z - &w;
        let aq = DMatrix::from_fn(a.nrows(), n, |row, col| a[(row, col)] * q[col]);
        let qm = &aq * a.transpose();
        let chol = qm.clone().cholesky().ok_or(QuantRegError::Singular)?;
        let mut rhs = &a * q.component_mul(&r);
        let mut dy = chol.solve(&rhs);
        let mut dx = q.component_mul(&(a.transpose() * &dy - &r));
        let mut ds = -&dx;
        let mut dz = DVector::from_iterator(n, (0..n).map(|i| -z[i] * (dx[i] / x[i] + 1.0)));
        let mut dw = DVector::from_iterator(n, (0..n).map(|i| -w[i] * (ds[i] / s[i] + 1.0)));
        let steps = |dx: &DVector<f64>, ds: &DVector<f64>, dz: &DVector<f64>, dw: &DVector<f64>| {
            let fp = step_bound(&x, dx).min(step_bound(&s, ds));
            let fd = step_bound(&w, dw).min(step_bound(&z, dz));
            ((BETA * fp).min(1.0), (BETA * fd).min(1.0))
        };
        let (mut fp, mut fd) = steps(&dx, &ds, &dz, &dw);
        if fp.min(fd) < 1.0 {
            // Mehrotra's corrector.
            let mu0 = z.dot(&x) + w.dot(&s);
            let g = (&z + fd * &dz).dot(&(&x + fp * &dx)) + (&w + fd * &dw).dot(&(&s + fp * &ds));
            let mu = mu0 * (g / mu0).powi(3) / (2.0 * n as f64);
            let dxdz = dx.component_mul(&dz);
            let dsdw = ds.component_mul(&dw);
            let xinv = x.map(|v| 1.0 / v);
            let sinv = s.map(|v| 1.0 / v);
            let xi = mu * (&xinv - &sinv);
            rhs += &a * q.component_mul(&(&dxdz - &dsdw - &xi));
            dy = chol.solve(&rhs);
            dx = q.component_mul(&(a.transpose() * &dy + &xi - &r - &dxdz + &dsdw));
            ds = -&dx;
            dz = DVector::from_iterator(n, (0..n).map(|i| mu * xinv[i] - z[i] - xinv[i] * z[i] * dx[i] - dxdz[i]));
            dw = DVector::from_iterator(n, (0..n).map(|i| mu * sinv[i] - w[i] - sinv[i] * w[i] * ds[i] - dsdw[i]));
            (fp, fd) = steps(&dx, &ds, &dz, &dw);
        }
        x += fp * &dx;
        s += fp * &ds;
        y += fd * &dy;
        w += fd * &dw;
        z += fd * &dz;
        gap = gap_of(&x, &y, &w);
    }
    Ok((-y, it))
}

/// Moves `b` to the vertex through the observations with the smallest residuals when that vertex is
/// at least as good: the linear programme's optimum is such a vertex, and the interior-point answer
/// lies within rounding of it.
fn polish(x: &DMatrix<f64>, y: &DVector<f64>, tau: f64, b: &mut DVector<f64>) {
    let p = x.ncols();
    let r = y - x * &*b;
    let mut order: Vec<usize> = (0..r.len()).collect();
    order.sort_by(|&i, &j| r[i].abs().total_cmp(&r[j].abs()));
    let h = &order[..p];
    let xh = DMatrix::from_fn(p, p, |i, j| x[(h[i], j)]);
    let yh = DVector::from_iterator(p, h.iter().map(|&i| y[i]));
    if let Some(v) = xh.lu().solve(&yh) {
        let (now, then) = (total_loss(tau, x, y, b), total_loss(tau, x, y, &v));
        if then <= now + 1e-9 * now.abs().max(1.0) {
            *b = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_median_of_a_constant_model_is_the_sample_median() {
        let y = [3.0, 1.0, 4.0, 1.5, 9.0, 2.6, 5.0];
        let r = quantile_regression(&[], &y, 0.5, true).unwrap();
        assert_eq!(r.coefficients, vec![3.0]);
        assert_eq!(r.loss, constant_loss(0.5, &y));
        assert_eq!(r.pseudo_r2, 0.0);
    }

    #[test]
    fn standard_errors_match_statsmodels_formulas() {
        // T494: statsmodels 0.15's QuantReg variance (Epanechnikov kernel, Hall-Sheather bandwidth) evaluated at the
        // exact fit from scipy's linprog (re-workspace/scratch/t480/qr_se.py).
        let x: Vec<f64> = (1..=20).map(f64::from).collect();
        let y: Vec<f64> = (1..=20u32)
            .map(|i| {
                let (xi, noise) = (f64::from(i), f64::from((i * 7919) % 13) / 3.0 - 2.0);
                2.0 + 0.5 * xi + noise * (1.0 + xi / 10.0)
            })
            .collect();
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
        let r = quantile_regression(&[&x], &y, 0.25, true).unwrap();
        assert!(close(r.coefficients[0], 1.6) && close(r.coefficients[1], 0.3), "{:?}", r.coefficients);
        assert!(close(r.se[0], 3.9581452343871217) && close(r.se[1], 0.3261196614730868), "{:?}", r.se);
        assert!(
            close(r.p_values[0], 0.6908063434724472) && close(r.p_values[1], 0.3697854143407211),
            "{:?}",
            r.p_values
        );
        assert!(close(r.bandwidth, 10.072413741709758), "{}", r.bandwidth);
        let iid = quantile_regression_with(&[&x], &y, 0.25, true, SeKind::Iid).unwrap();
        assert!(close(iid.se[0], 3.221946701428397) && close(iid.se[1], 0.2689629139991902), "{:?}", iid.se);
        // The median fit is not unique here (scipy finds another vertex of the same loss): the reference uses this one.
        let r = quantile_regression(&[&x], &y, 0.5, true).unwrap();
        assert_eq!(r.coefficients, [2.0, 0.5]);
        assert!(close(r.se[0], 2.3721518776124673) && close(r.se[1], 0.19802341273008708), "{:?}", r.se);
        assert!(close(r.p_values[1], 0.02117876921717891), "{:?}", r.p_values);
        // At 0.9 with 20 points tau + hs passes 1: no errors.
        let r = quantile_regression(&[&x], &y, 0.9, true).unwrap();
        assert!(r.se.iter().chain(&r.p_values).all(|v| v.is_nan()), "{:?}", r.se);
    }

    #[test]
    fn a_line_through_exact_data_is_recovered_at_every_quantile() {
        let x: Vec<f64> = (0..20).map(f64::from).collect();
        let y: Vec<f64> = x.iter().map(|v| 2.0 - 0.5 * v).collect();
        for tau in [0.1, 0.5, 0.9] {
            let r = quantile_regression(&[&x], &y, tau, true).unwrap();
            assert!((r.coefficients[0] - 2.0).abs() < 1e-12 && (r.coefficients[1] + 0.5).abs() < 1e-12, "{r:?}");
            assert!(r.loss < 1e-12);
        }
    }

    #[test]
    fn bad_input_is_refused() {
        let x = [1.0, 2.0, 3.0];
        let y = [1.0, 2.0, 2.5];
        assert_eq!(quantile_regression(&[&x], &y, 1.0, true), Err(QuantRegError::BadQuantile(1.0)));
        assert_eq!(quantile_regression(&[&x[..2]], &y, 0.5, true), Err(QuantRegError::LengthMismatch));
        assert_eq!(quantile_regression(&[&x, &x], &y, 0.5, true), Err(QuantRegError::Singular));
        let one = [f64::NAN, 1.0, f64::NAN];
        assert_eq!(quantile_regression(&[&x], &one, 0.5, true), Err(QuantRegError::TooFewPoints { have: 1, need: 2 }));
    }
}
