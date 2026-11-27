//! ARIMA(p, d, q) by conditional sum of squares (Origin 2024b's Time Series Analysis app, "use ARIMA to forecast";
//! not in Origin 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 43).
//!
//! The series is differenced `d` times to `w`. With mean `mu` (when the constant is on, else 0) the model is
//! `w_t - mu = sum_i phi_i (w_{t-i} - mu) + e_t + sum_j theta_j e_{t-j}`. The conditional sum of squares takes the
//! errors before `t = p` as 0 and sums `e_t^2` over `t = p..`; it is minimized by Nelder-Mead from `mu = mean(w)`,
//! `phi = theta = 0` (no stationarity or invertibility constraint). `sigma^2 = SS / m` over the `m` summed errors and
//! `AIC = m ln(2 pi sigma^2) + m + 2 (k + 1)` for `k` coefficients. Forecasts set future errors to 0 and undo the
//! differences; their standard errors are `sigma sqrt(sum_{j<h} psi_j^2)` from the psi weights of
//! `phi(B) (1 - B)^d y = theta(B) e`. Checked against numpy and scipy in `crates/calc/tests/arima_reference.rs`
//! (made by `tools/arima-ref.py`).
//!
//! **Standard errors** (T428): the Gauss-Newton covariance `sigma^2 (J'J)^-1`, `J` the derivatives of the summed
//! conditional errors with respect to the coefficients (exact, by differentiating the error recursion); `z = b / SE`
//! with normal two-sided probabilities. **Order choice** (T428): every `p <= P`, `q <= Q` (with `d` and the constant
//! as given) is fitted and the smallest AIC kept; each AIC sums its own `n - d - p` errors.
//!
//! **Seasonal ARIMA** (T446): with a period `s` and seasonal orders `(P, D, Q)` the series is also differenced `D`
//! times at lag `s`, and the AR and MA polynomials are the products `(1 - sum phi_i B^i)(1 - sum Phi_j B^(js))` and
//! `(1 + sum theta_i B^i)(1 + sum Theta_j B^(js))`, written out lag by lag; the errors before the AR polynomial's
//! last lag `p + P s` are 0. Forecasts undo the seasonal and the ordinary differences in turn, the bands use the psi
//! weights of the full polynomials, and the standard errors take `J` by central differences (the ordinary model keeps
//! the exact derivatives). The coefficients are `[mu?, phi.., theta.., Phi.., Theta..]`.
//!
//! **Exact maximum likelihood** (T495, `Method::Ml`): the differenced series minus its mean is an ARMA process in
//! state-space form (Harvey's: state length `r = max(p', q' + 1)` for the full lags `p'`, `q'`; transition with the AR
//! lags in its first column and ones above the diagonal; disturbance loading `(1, theta'..)`), started at its
//! stationary covariance (`P = T P T' + R R'`, by doubling) and run through the Kalman filter with `sigma^2 = 1`. The
//! innovations `v_t` and their variances `F_t` give `-2 ln L = n ln(2 pi sigma^2) + sum ln F_t + n` with `sigma^2 =
//! sum v_t^2 / F_t / n` concentrated out; it is minimized by Nelder-Mead from the CSS fit (the lag coefficients from 0
//! when that fit is not stationary; a non-stationary AR part has no likelihood). `AIC = -2 ln L + 2 (k + 1)`. The
//! residuals are the innovations (one-step prediction errors) of every differenced value, the forecasts come from the
//! filtered state (their standard errors from the psi weights, as for CSS), and the standard errors from the inverse
//! of the Hessian of `-ln L` by central differences. Checked against statsmodels' `ARIMA` (state space, the same
//! likelihood) on the differenced series in `exact_likelihood_matches_statsmodels`.

use nalgebra::{DMatrix, DVector};

/// How the coefficients are estimated (T495).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Method {
    /// Conditional sum of squares (the errors before the first full lag are 0).
    #[default]
    Css,
    /// Exact Gaussian maximum likelihood (Kalman filter).
    Ml,
}

/// The seasonal part of an order (T446): `(P, D, Q)` at period `s`; all 0 for none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Seasonal {
    pub p: usize,
    pub d: usize,
    pub q: usize,
    pub period: usize,
}

/// The model order and whether it has a constant (mean of the differenced series).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArimaOrder {
    pub p: usize,
    pub d: usize,
    pub q: usize,
    pub constant: bool,
    /// The seasonal orders (T446).
    pub seasonal: Seasonal,
}

impl ArimaOrder {
    /// The number of coefficients: the mean (if any), `phi_1..p`, `theta_1..q`, then `Phi_1..P`, `Theta_1..Q`.
    pub fn coefficients(&self) -> usize {
        usize::from(self.constant) + self.p + self.q + self.seasonal.p + self.seasonal.q
    }

    /// The last lag of the full AR polynomial, before which the errors are 0.
    fn ar_lags(&self) -> usize {
        self.p + self.seasonal.p * self.seasonal.period
    }

    /// The values the differences take off the front of the series.
    fn lost(&self) -> usize {
        self.d + self.seasonal.d * self.seasonal.period
    }

    fn is_seasonal(&self) -> bool {
        self.seasonal.p + self.seasonal.q + self.seasonal.d > 0
    }
}

/// A fitted model.
#[derive(Debug, Clone, PartialEq)]
pub struct Arima {
    pub order: ArimaOrder,
    /// `[mu?, phi.., theta..]`.
    pub params: Vec<f64>,
    pub ss: f64,
    pub sigma2: f64,
    pub aic: f64,
    /// Errors of the differenced series (0 before `t = p`).
    pub residuals: Vec<f64>,
    /// In-sample one-step predictions of `y` (NaN for the first `d + p` values).
    pub fitted: Vec<f64>,
    pub forecast: Vec<f64>,
    pub forecast_se: Vec<f64>,
    /// Standard errors of `params` (T428; NaN when `J'J` is singular).
    pub se: Vec<f64>,
    /// How it was fitted (T495).
    pub method: Method,
    /// The log-likelihood: exact for `Method::Ml`, the conditional one for CSS.
    pub loglik: f64,
}

/// Why the model could not be fitted.
#[derive(Debug, Clone, PartialEq)]
pub enum ArimaError {
    Missing(usize),
    TooShort(usize),
    Empty,
    NoConvergence,
    /// A seasonal order with a period below 2 (T446).
    BadPeriod(usize),
}

impl std::fmt::Display for ArimaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArimaError::Missing(i) => write!(f, "value {} is missing: the series must be complete", i + 1),
            ArimaError::TooShort(n) => write!(f, "{n} values are too few for this order"),
            ArimaError::Empty => f.write_str("the model has nothing to fit: give p, q or a constant"),
            ArimaError::NoConvergence => f.write_str("the fit did not give finite errors"),
            ArimaError::BadPeriod(s) => write!(f, "the seasonal period must be at least 2 (got {s})"),
        }
    }
}

impl std::error::Error for ArimaError {}

/// The levels of differencing: `y`, then each ordinary difference, then each seasonal one; the last is the modelled
/// series, and each level's lag is 1 or the period.
fn levels(y: &[f64], order: &ArimaOrder) -> Vec<(Vec<f64>, usize)> {
    let mut out = vec![(y.to_vec(), 0)];
    let s = order.seasonal.period;
    for lag in std::iter::repeat_n(1, order.d).chain(std::iter::repeat_n(s, order.seasonal.d)) {
        let last = &out.last().expect("a level").0;
        let next =
            if last.len() > lag { (lag..last.len()).map(|t| last[t] - last[t - lag]).collect() } else { Vec::new() };
        out.push((next, lag));
    }
    out
}

fn difference(y: &[f64], order: &ArimaOrder) -> Vec<f64> {
    levels(y, order).pop().expect("a level").0
}

fn split(order: &ArimaOrder, params: &[f64]) -> (f64, Vec<f64>, Vec<f64>) {
    let c = usize::from(order.constant);
    let mu = if order.constant { params[0] } else { 0.0 };
    (mu, params[c..c + order.p].to_vec(), params[c + order.p..c + order.p + order.q].to_vec())
}

/// The mean and the full AR and MA lag coefficients (index `k - 1` for lag `k`): `w_t - mu = sum ar_k (w_{t-k} - mu) +
/// e_t + sum ma_k e_{t-k}`, the seasonal factors multiplied in.
fn expand(order: &ArimaOrder, params: &[f64]) -> (f64, Vec<f64>, Vec<f64>) {
    let (mu, phi, theta) = split(order, params);
    if !order.is_seasonal() {
        return (mu, phi, theta);
    }
    let c = usize::from(order.constant) + order.p + order.q;
    let big_phi = &params[c..c + order.seasonal.p];
    let big_theta = &params[c + order.seasonal.p..c + order.seasonal.p + order.seasonal.q];
    let s = order.seasonal.period;
    // Polynomials in B with the constant term first: AR (1 - ...)(1 - ...), MA (1 + ...)(1 + ...).
    let poly = |small: &[f64], big: &[f64], sign: f64| -> Vec<f64> {
        let a: Vec<f64> = std::iter::once(1.0).chain(small.iter().map(|v| sign * v)).collect();
        let mut b = vec![0.0; big.len() * s + 1];
        b[0] = 1.0;
        for (j, v) in big.iter().enumerate() {
            b[(j + 1) * s] = sign * v;
        }
        let mut out = vec![0.0; a.len() + b.len() - 1];
        for (i, x) in a.iter().enumerate() {
            for (j, z) in b.iter().enumerate() {
                out[i + j] += x * z;
            }
        }
        out
    };
    let ar: Vec<f64> = poly(&phi, big_phi, -1.0)[1..].iter().map(|v| -v).collect();
    let ma: Vec<f64> = poly(&theta, big_theta, 1.0)[1..].to_vec();
    (mu, ar, ma)
}

/// The conditional errors of the differenced series and their sum of squares, for `params`.
pub fn css(y: &[f64], order: ArimaOrder, params: &[f64]) -> (Vec<f64>, f64) {
    let w = difference(y, &order);
    let (mu, phi, theta) = expand(&order, params);
    let mut e = vec![0.0; w.len()];
    let mut ss = 0.0;
    for t in order.ar_lags()..w.len() {
        let mut v = w[t] - mu;
        for (i, ph) in phi.iter().enumerate() {
            v -= ph * (w[t - i - 1] - mu);
        }
        for (j, th) in theta.iter().enumerate() {
            if t > j {
                v -= th * e[t - j - 1];
            }
        }
        e[t] = v;
        ss += v * v;
    }
    (e, ss)
}

/// Standard errors from central-difference derivatives of the conditional errors (the seasonal model, T446).
fn numeric_standard_errors(y: &[f64], order: ArimaOrder, params: &[f64], sigma2: f64) -> Vec<f64> {
    let k = params.len();
    let from = order.ar_lags();
    let cols: Vec<Vec<f64>> = (0..k)
        .map(|i| {
            let step = 1e-6 * params[i].abs().max(1.0);
            let (mut up, mut dn) = (params.to_vec(), params.to_vec());
            up[i] += step;
            dn[i] -= step;
            let (eu, ed) = (css(y, order, &up).0, css(y, order, &dn).0);
            eu[from..].iter().zip(&ed[from..]).map(|(a, b)| (a - b) / (2.0 * step)).collect()
        })
        .collect();
    let jtj: Vec<Vec<f64>> =
        (0..k).map(|a| (0..k).map(|b| cols[a].iter().zip(&cols[b]).map(|(x, z)| x * z).sum()).collect()).collect();
    match invert(jtj) {
        Some(inv) => (0..k).map(|i| (sigma2 * inv[i][i]).sqrt()).collect(),
        None => vec![f64::NAN; k],
    }
}

/// The standard errors of `params` (T428): `sigma2 (J'J)^-1` with `J` the exact derivatives of the summed conditional
/// errors; NaN when `J'J` is singular.
pub fn standard_errors(y: &[f64], order: ArimaOrder, params: &[f64], sigma2: f64) -> Vec<f64> {
    if order.is_seasonal() {
        return numeric_standard_errors(y, order, params, sigma2);
    }
    let k = params.len();
    let w = difference(y, &order);
    let (mu, phi, theta) = split(&order, params);
    let (e, _) = css(y, order, params);
    let c = usize::from(order.constant);
    // d[t][i]: the derivative of e_t with respect to coefficient i (0 before t = p, as e is).
    let mut d = vec![vec![0.0; k]; w.len()];
    let mut jtj = vec![vec![0.0; k]; k];
    for t in order.p..w.len() {
        let mut row = vec![0.0; k];
        if order.constant {
            row[0] = -1.0 + phi.iter().sum::<f64>();
        }
        for i in 0..order.p {
            row[c + i] = -(w[t - i - 1] - mu);
        }
        for j in 0..order.q {
            if t > j {
                row[c + order.p + j] = -e[t - j - 1];
            }
        }
        for (j, th) in theta.iter().enumerate() {
            if t > j {
                for (r, prev) in row.iter_mut().zip(&d[t - j - 1]) {
                    *r -= th * prev;
                }
            }
        }
        for a in 0..k {
            for b in 0..k {
                jtj[a][b] += row[a] * row[b];
            }
        }
        d[t] = row;
    }
    match invert(jtj) {
        Some(inv) => (0..k).map(|i| (sigma2 * inv[i][i]).sqrt()).collect(),
        None => vec![f64::NAN; k],
    }
}

/// The inverse of a symmetric positive definite matrix by Gauss-Jordan elimination with pivoting; `None` when singular.
fn invert(mut a: Vec<Vec<f64>>) -> Option<Vec<Vec<f64>>> {
    let k = a.len();
    let scale = a.iter().flatten().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut inv: Vec<Vec<f64>> = (0..k).map(|i| (0..k).map(|j| f64::from(u8::from(i == j))).collect()).collect();
    for col in 0..k {
        let piv = (col..k).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[piv][col].abs() <= 1e-13 * scale.max(f64::MIN_POSITIVE) {
            return None;
        }
        a.swap(col, piv);
        inv.swap(col, piv);
        let p = a[col][col];
        for j in 0..k {
            a[col][j] /= p;
            inv[col][j] /= p;
        }
        for i in 0..k {
            if i != col {
                let f = a[i][col];
                if f != 0.0 {
                    for j in 0..k {
                        a[i][j] -= f * a[col][j];
                        inv[i][j] -= f * inv[col][j];
                    }
                }
            }
        }
    }
    Some(inv)
}

/// Fits every ARIMA(`p`, `d`, `q`) with `p <= max_p`, `q <= max_q` and keeps the one with the smallest AIC (T428);
/// orders too long for the series, without coefficients or without finite errors are passed over. With a period, the
/// seasonal `P` and `Q` of `seasonal` are maxima too: every `P' <= P`, `Q' <= Q` joins the search, `D` fixed (T452).
pub fn fit_auto(
    y: &[f64],
    d: usize,
    max_p: usize,
    max_q: usize,
    constant: bool,
    seasonal: Seasonal,
    h: usize,
) -> Result<Arima, ArimaError> {
    fit_auto_with(y, d, max_p, max_q, constant, seasonal, h, Method::Css)
}

/// [`fit_auto`] by `method` (T495).
#[allow(clippy::too_many_arguments)]
pub fn fit_auto_with(
    y: &[f64],
    d: usize,
    max_p: usize,
    max_q: usize,
    constant: bool,
    seasonal: Seasonal,
    h: usize,
    method: Method,
) -> Result<Arima, ArimaError> {
    let mut best: Option<Arima> = None;
    for sp in 0..=seasonal.p {
        for sq in 0..=seasonal.q {
            let s = Seasonal { p: sp, q: sq, ..seasonal };
            for p in 0..=max_p {
                for q in 0..=max_q {
                    match fit_with(y, ArimaOrder { p, d, q, constant, seasonal: s }, h, method) {
                        Ok(m) => {
                            if best.as_ref().is_none_or(|b| m.aic < b.aic) {
                                best = Some(m);
                            }
                        }
                        Err(e @ (ArimaError::Missing(_) | ArimaError::BadPeriod(_))) => return Err(e),
                        Err(_) => {}
                    }
                }
            }
        }
    }
    best.ok_or(ArimaError::TooShort(y.len()))
}

/// Forecasts `h` steps of `y` and their standard errors for `params` and `sigma2`.
pub fn forecast(y: &[f64], order: ArimaOrder, params: &[f64], sigma2: f64, h: usize) -> (Vec<f64>, Vec<f64>) {
    let n = difference(y, &order).len();
    let (w, _) = forecast_differenced(y, order, params, h);
    let fc = undo_differences(&levels(y, &order), w[n..].to_vec());
    (fc, psi_standard_errors(&order, params, sigma2, h))
}

/// The differenced series followed by its `h` CSS forecasts, and the errors (0 for the future).
fn forecast_differenced(y: &[f64], order: ArimaOrder, params: &[f64], h: usize) -> (Vec<f64>, Vec<f64>) {
    let (e, _) = css(y, order, params);
    let stack = levels(y, &order);
    let mut w = stack.last().expect("a level").0.clone();
    let (mu, phi, theta) = expand(&order, params);
    let n = w.len();
    let mut errs = e;
    for k in 0..h {
        let t = n + k;
        let mut v = mu;
        for (i, ph) in phi.iter().enumerate() {
            v += ph * (w[t - i - 1] - mu);
        }
        for (j, th) in theta.iter().enumerate() {
            if t > j {
                v += th * errs[t - j - 1];
            }
        }
        w.push(v);
        errs.push(0.0);
    }
    (w, errs)
}

/// Forecasts of the differenced series put back on the level of `y`, the differences undone level by level (seasonal
/// ones first): `x_t = dx_t + x_{t - lag}`.
fn undo_differences(stack: &[(Vec<f64>, usize)], mut fc: Vec<f64>) -> Vec<f64> {
    for lvl in (0..stack.len() - 1).rev() {
        let lag = stack[lvl + 1].1;
        let mut known = stack[lvl].0.clone();
        let base = known.len();
        for dv in &fc {
            let back = known.len() - lag;
            known.push(dv + known[back]);
        }
        fc = known[base..].to_vec();
    }
    fc
}

/// The forecast standard errors `sigma sqrt(sum_{j<h} psi_j^2)` from the psi weights of
/// `phi(B)(1 - B)^d (1 - B^s)^D y = theta(B) e` with the full polynomials.
fn psi_standard_errors(order: &ArimaOrder, params: &[f64], sigma2: f64, h: usize) -> Vec<f64> {
    let (_, phi, theta) = expand(order, params);
    let mut ar = vec![1.0];
    for ph in &phi {
        ar.push(-ph);
    }
    for lag in std::iter::repeat_n(1, order.d).chain(std::iter::repeat_n(order.seasonal.period, order.seasonal.d)) {
        let mut next = vec![0.0; ar.len() + lag];
        for (i, a) in ar.iter().enumerate() {
            next[i] += a;
            next[i + lag] -= a;
        }
        ar = next;
    }
    let mut psi = vec![1.0];
    for j in 1..h {
        let mut v = theta.get(j - 1).copied().unwrap_or(0.0);
        for i in 1..ar.len().min(j + 1) {
            v -= ar[i] * psi[j - i];
        }
        psi.push(v);
    }
    let mut acc = 0.0;
    psi.iter()
        .take(h)
        .map(|p| {
            acc += p * p;
            (sigma2 * acc).sqrt()
        })
        .collect()
}

/// The Kalman filter of an ARMA process (T495): the innovations, their variances in units of `sigma^2`, the state
/// predicted for the next value and the transition. `None` when the AR part is not stationary.
struct Filtered {
    v: Vec<f64>,
    f: Vec<f64>,
    state: DVector<f64>,
    transition: DMatrix<f64>,
}

fn kalman(w: &[f64], mu: f64, ar: &[f64], ma: &[f64]) -> Option<Filtered> {
    let r = ar.len().max(ma.len() + 1);
    let tm = DMatrix::from_fn(r, r, |i, j| {
        if j == 0 {
            ar.get(i).copied().unwrap_or(0.0)
        } else if i + 1 == j {
            1.0
        } else {
            0.0
        }
    });
    let rv = DVector::from_fn(r, |i, _| if i == 0 { 1.0 } else { ma.get(i - 1).copied().unwrap_or(0.0) });
    let rr = &rv * rv.transpose();
    // The stationary covariance sum_j T^j R R' T'^j, doubling the terms each pass.
    let mut p = rr.clone();
    let mut a = tm.clone();
    for _ in 0..64 {
        p = &p + &a * &p * a.transpose();
        a = &a * &a;
        if !a.amax().is_finite() || p.iter().any(|v| !v.is_finite()) {
            return None;
        }
        if a.amax() < 1e-15 {
            break;
        }
    }
    if a.amax() > 1e-10 {
        return None;
    }
    let mut st = DVector::zeros(r);
    let (mut v, mut f) = (Vec::with_capacity(w.len()), Vec::with_capacity(w.len()));
    for &x in w {
        let ft = p[(0, 0)];
        if ft.is_nan() || ft <= 0.0 {
            return None;
        }
        let vt = x - mu - st[0];
        let k = p.column(0) / ft;
        st += &k * vt;
        let row = p.row(0).clone_owned();
        p -= &k * row;
        st = &tm * &st;
        p = &tm * &p * tm.transpose() + &rr;
        v.push(vt);
        f.push(ft);
    }
    Some(Filtered { v, f, state: st, transition: tm })
}

/// `-2 ln L` without its constant, `n ln(sum v^2 / F / n) + sum ln F`, for `params` (T495); infinite when the AR part
/// is not stationary.
fn ml_objective(w: &[f64], order: &ArimaOrder, params: &[f64]) -> f64 {
    let (mu, ar, ma) = expand(order, params);
    let Some(k) = kalman(w, mu, &ar, &ma) else { return f64::INFINITY };
    let n = w.len() as f64;
    let s: f64 = k.v.iter().zip(&k.f).map(|(v, f)| v * v / f).sum();
    let lf: f64 = k.f.iter().map(|f| f.ln()).sum();
    let out = n * (s / n).ln() + lf;
    if out.is_finite() { out } else { f64::INFINITY }
}

/// Standard errors from the inverse of the Hessian of `-ln L = objective / 2` by central differences (T495).
fn hessian_standard_errors(f: &dyn Fn(&[f64]) -> f64, x: &[f64]) -> Vec<f64> {
    let k = x.len();
    let h: Vec<f64> = x.iter().map(|v| 1e-4 * v.abs().max(1.0)).collect();
    let at = |moves: &[(usize, f64)]| {
        let mut p = x.to_vec();
        for (i, s) in moves {
            p[*i] += s;
        }
        f(&p)
    };
    let f0 = f(x);
    let mut half = vec![vec![0.0; k]; k];
    for i in 0..k {
        half[i][i] = (at(&[(i, h[i])]) - 2.0 * f0 + at(&[(i, -h[i])])) / (h[i] * h[i]) / 2.0;
        for j in 0..i {
            let v = (at(&[(i, h[i]), (j, h[j])]) - at(&[(i, h[i]), (j, -h[j])]) - at(&[(i, -h[i]), (j, h[j])])
                + at(&[(i, -h[i]), (j, -h[j])]))
                / (4.0 * h[i] * h[j])
                / 2.0;
            half[i][j] = v;
            half[j][i] = v;
        }
    }
    match invert(half) {
        Some(inv) => (0..k).map(|i| if inv[i][i] > 0.0 { inv[i][i].sqrt() } else { f64::NAN }).collect(),
        None => vec![f64::NAN; k],
    }
}

/// The exact maximum likelihood fit from the CSS coefficients `start` (T495).
fn fit_ml(y: &[f64], order: ArimaOrder, h: usize, mut start: Vec<f64>, scale: &[f64]) -> Result<Arima, ArimaError> {
    let w = difference(y, &order);
    let nw = w.len();
    let k = order.coefficients();
    let objective = |p: &[f64]| ml_objective(&w, &order, p);
    if !objective(&start).is_finite() {
        // A non-stationary CSS fit: the lag coefficients start at 0.
        start[usize::from(order.constant)..].iter_mut().for_each(|v| *v = 0.0);
    }
    if !objective(&start).is_finite() {
        return Err(ArimaError::NoConvergence);
    }
    let (mut best, mut val) = nelder_mead(&objective, &start, scale);
    for _ in 0..10 {
        let small: Vec<f64> = scale.iter().map(|s| s * 0.1).collect();
        let (p, v) = nelder_mead(&objective, &best, &small);
        let improved = v < val - 1e-12 * val.abs().max(1.0);
        if v < val {
            best = p;
            val = v;
        }
        if !improved {
            break;
        }
    }
    let (mu, ar, ma) = expand(&order, &best);
    let kf = kalman(&w, mu, &ar, &ma).ok_or(ArimaError::NoConvergence)?;
    let n = nw as f64;
    let sigma2 = kf.v.iter().zip(&kf.f).map(|(v, f)| v * v / f).sum::<f64>() / n;
    let lf: f64 = kf.f.iter().map(|f| f.ln()).sum();
    let loglik = -0.5 * (n * (2.0 * std::f64::consts::PI * sigma2).ln() + lf + n);
    let aic = -2.0 * loglik + 2.0 * (k + 1) as f64;
    let ss = kf.v.iter().map(|v| v * v).sum();
    let lost = order.lost();
    let mut fitted = vec![f64::NAN; y.len()];
    for (t, v) in kf.v.iter().enumerate() {
        fitted[t + lost] = y[t + lost] - v;
    }
    let mut st = kf.state.clone();
    let mut wf = Vec::with_capacity(h);
    for _ in 0..h {
        wf.push(mu + st[0]);
        st = &kf.transition * &st;
    }
    let forecast = undo_differences(&levels(y, &order), wf);
    let forecast_se = psi_standard_errors(&order, &best, sigma2, h);
    let se = hessian_standard_errors(&objective, &best);
    Ok(Arima {
        order,
        params: best,
        ss,
        sigma2,
        aic,
        residuals: kf.v,
        fitted,
        forecast,
        forecast_se,
        se,
        method: Method::Ml,
        loglik,
    })
}

/// Nelder-Mead from `start` with initial steps `scale`.
fn nelder_mead(f: &dyn Fn(&[f64]) -> f64, start: &[f64], scale: &[f64]) -> (Vec<f64>, f64) {
    let k = start.len();
    let mut pts: Vec<Vec<f64>> = vec![start.to_vec()];
    for i in 0..k {
        let mut p = start.to_vec();
        p[i] += scale[i];
        pts.push(p);
    }
    let mut vals: Vec<f64> = pts.iter().map(|p| f(p)).collect();
    for _ in 0..20_000 {
        let mut order: Vec<usize> = (0..=k).collect();
        order.sort_by(|&i, &j| vals[i].total_cmp(&vals[j]));
        pts = order.iter().map(|&i| pts[i].clone()).collect();
        vals = order.iter().map(|&i| vals[i]).collect();
        let spread = pts.iter().flat_map(|p| p.iter().zip(&pts[0]).map(|(a, b)| (a - b).abs())).fold(0.0, f64::max);
        if spread < 1e-12 && (vals[k] - vals[0]).abs() <= 1e-15 * vals[0].abs().max(1e-300) {
            break;
        }
        let centroid: Vec<f64> = (0..k).map(|d| pts[..k].iter().map(|p| p[d]).sum::<f64>() / k as f64).collect();
        let along = |t: f64| centroid.iter().zip(&pts[k]).map(|(c, w)| c + t * (c - w)).collect::<Vec<f64>>();
        let r = along(1.0);
        let fr = f(&r);
        if fr < vals[0] {
            let e = along(2.0);
            let fe = f(&e);
            if fe < fr {
                pts[k] = e;
                vals[k] = fe;
            } else {
                pts[k] = r;
                vals[k] = fr;
            }
        } else if fr < vals[k - 1] {
            pts[k] = r;
            vals[k] = fr;
        } else {
            let c = if fr < vals[k] { along(0.5) } else { along(-0.5) };
            let fc = f(&c);
            if fc < vals[k].min(fr) {
                pts[k] = c;
                vals[k] = fc;
            } else {
                for i in 1..=k {
                    pts[i] = pts[i].iter().zip(&pts[0]).map(|(p, b)| b + 0.5 * (p - b)).collect();
                    vals[i] = f(&pts[i]);
                }
            }
        }
    }
    let best = (0..=k).min_by(|&i, &j| vals[i].total_cmp(&vals[j])).unwrap_or(0);
    (pts[best].clone(), vals[best])
}

/// Fits ARIMA(`order`) to `y` by conditional sum of squares and forecasts `h` steps.
pub fn fit(y: &[f64], order: ArimaOrder, h: usize) -> Result<Arima, ArimaError> {
    fit_with(y, order, h, Method::Css)
}

/// Fits ARIMA(`order`) to `y` by `method` (T495) and forecasts `h` steps.
pub fn fit_with(y: &[f64], order: ArimaOrder, h: usize, method: Method) -> Result<Arima, ArimaError> {
    if let Some(i) = y.iter().position(|v| !v.is_finite()) {
        return Err(ArimaError::Missing(i));
    }
    let k = order.coefficients();
    if k == 0 {
        return Err(ArimaError::Empty);
    }
    if order.is_seasonal() && order.seasonal.period < 2 {
        return Err(ArimaError::BadPeriod(order.seasonal.period));
    }
    let nw = y.len().saturating_sub(order.lost());
    if nw < order.ar_lags() + k + 2 {
        return Err(ArimaError::TooShort(y.len()));
    }
    let w = difference(y, &order);
    let mean = w.iter().sum::<f64>() / nw as f64;
    let sd = (w.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / nw as f64).sqrt().max(1e-12);
    let mut start = Vec::with_capacity(k);
    let mut scale = Vec::with_capacity(k);
    if order.constant {
        start.push(mean);
        scale.push(0.1 * sd);
    }
    let lags = order.p + order.q + order.seasonal.p + order.seasonal.q;
    start.extend(std::iter::repeat_n(0.0, lags));
    scale.extend(std::iter::repeat_n(0.1, lags));
    let objective = |p: &[f64]| {
        let ss = css(y, order, p).1;
        if ss.is_finite() { ss } else { f64::INFINITY }
    };
    let (mut best, mut val) = nelder_mead(&objective, &start, &scale);
    // Restarts from the best point until it stops improving.
    for _ in 0..10 {
        let small: Vec<f64> = scale.iter().map(|s| s * 0.1).collect();
        let (p, v) = nelder_mead(&objective, &best, &small);
        let improved = v < val * (1.0 - 1e-14);
        best = p;
        val = v.min(val);
        if !improved {
            break;
        }
    }
    let (residuals, ss) = css(y, order, &best);
    if !ss.is_finite() {
        return Err(ArimaError::NoConvergence);
    }
    if method == Method::Ml {
        return fit_ml(y, order, h, best, &scale);
    }
    let m = (nw - order.ar_lags()) as f64;
    let sigma2 = ss / m;
    let aic = m * (2.0 * std::f64::consts::PI * sigma2).ln() + m + 2.0 * (k + 1) as f64;
    // One-step predictions of y: the differenced prediction w_t - e_t, put back on the level of y.
    let mut fitted = vec![f64::NAN; y.len()];
    let lost = order.lost();
    for t in order.ar_lags()..nw {
        // The one-step error of the differenced series is that of y.
        fitted[t + lost] = y[t + lost] - residuals[t];
    }
    let (fc, fse) = forecast(y, order, &best, sigma2, h);
    let se = standard_errors(y, order, &best, sigma2);
    let loglik = -0.5 * (m * (2.0 * std::f64::consts::PI * sigma2).ln() + m);
    Ok(Arima {
        order,
        params: best,
        ss,
        sigma2,
        aic,
        residuals,
        fitted,
        forecast: fc,
        forecast_se: fse,
        se,
        method: Method::Css,
        loglik,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(n: usize) -> Vec<f64> {
        (0..n).map(|t| ((t * 7919 + 13) % 101) as f64 / 101.0 - 0.5).collect()
    }

    #[test]
    fn exact_likelihood_matches_statsmodels() {
        // T495: statsmodels 0.15 ARIMA (state space, lbfgs to gtol 1e-12, cov_type "approx": the numerical Hessian) on
        // the differenced series (re-workspace/scratch/t480/arima_ml.py). The likelihood is flat near the optimum, so
        // the coefficients agree to 1e-4 while the log-likelihood agrees to 1e-6.
        let e = noise(150);
        let mut arma = Vec::new();
        for t in 0..120 {
            let prev_w = if t > 0 { arma[t - 1] - 5.0 } else { 0.0 };
            let prev_e = if t > 0 { e[t - 1] } else { 0.0 };
            arma.push(5.0 + 0.6 * prev_w + e[t] + 0.3 * prev_e);
        }
        let (mut walk, mut level, mut prev_e) = (Vec::new(), 0.0, 0.0);
        for &et in &e {
            level += et - 0.4 * prev_e;
            prev_e = et;
            walk.push(level);
        }
        let mut sar: Vec<f64> = Vec::new();
        for t in 0..120 {
            let mut v = 2.0 + e[t];
            if t >= 1 {
                v += 0.5 * (sar[t - 1] - 2.0);
            }
            if t >= 4 {
                v += 0.3 * (sar[t - 4] - 2.0);
            }
            if t >= 5 {
                v -= 0.15 * (sar[t - 5] - 2.0);
            }
            sar.push(v);
        }
        let cases: [(&[f64], ArimaOrder, &[f64], f64, f64, f64, &[f64], &[f64]); 3] = [
            (
                &arma,
                ArimaOrder { p: 1, d: 0, q: 1, constant: true, ..Default::default() },
                &[4.971850641566094, 0.35151569233489866, 0.03655414017989958],
                0.06570096207235528,
                -6.995957538482822,
                21.991915076965643,
                &[0.03728110936181515, 0.19794520480553998, 0.20296843205928067],
                &[4.796784031196819, 4.910311980817415, 4.95021883662766],
            ),
            (
                &walk,
                ArimaOrder { p: 0, d: 1, q: 1, constant: false, ..Default::default() },
                &[-0.7308985033725176],
                0.06952717248950069,
                -13.18688215720757,
                30.37376431441514,
                &[0.048042724484854506],
                &[-0.2418393190021595; 3],
            ),
            (
                &sar,
                ArimaOrder { p: 1, d: 0, q: 0, constant: true, seasonal: Seasonal { p: 1, d: 0, q: 0, period: 4 } },
                &[1.979679786462697, 0.16222713316351178, -0.04033776247742877],
                0.06132918555036888,
                -2.8015485330400924,
                13.603097066080185,
                &[0.02594517526826237, 0.10973948846777637, 0.11307074017730381],
                &[1.9468273249891352, 1.9772139480329751, 1.9995611932207524],
            ),
        ];
        for (y, order, params, sigma2, llf, aic, bse, fc) in cases {
            let m = fit_with(y, order, 3, Method::Ml).unwrap();
            let all = format!("{order:?}: {m:?}");
            assert_eq!(m.method, Method::Ml);
            assert!(m.params.iter().zip(params).all(|(a, b)| (a - b).abs() < 1e-4), "params {all}");
            assert!((m.sigma2 - sigma2).abs() < 1e-4 * sigma2, "sigma2 {all}");
            assert!((m.loglik - llf).abs() < 1e-6, "loglik {all}");
            assert!((m.aic - aic).abs() < 1e-5, "aic {all}");
            assert!(m.se.iter().zip(bse).all(|(a, b)| (a - b).abs() < 2e-3 * b), "se {:?} {bse:?}", m.se);
            assert!(m.forecast.iter().zip(fc).all(|(a, b)| (a - b).abs() < 1e-4), "forecast {all}");
            // The residuals are the innovations of every differenced value.
            assert_eq!(m.residuals.len(), y.len() - order.d);
        }
        // A non-stationary series still fits (the start is pulled back to 0); CSS stays the default.
        assert_eq!(
            fit(&arma, ArimaOrder { p: 1, d: 0, q: 1, constant: true, ..Default::default() }, 1).unwrap().method,
            Method::Css
        );
    }

    #[test]
    fn ar1_recursions_and_a_random_walk_forecast() {
        let order = ArimaOrder { p: 1, d: 0, q: 0, constant: true, ..Default::default() };
        let y = [1.0, 2.0, 1.5, 3.0];
        let (e, ss) = css(&y, order, &[2.0, 0.5]);
        // e_1 = (2 - 2) - 0.5 (1 - 2) = 0.5; e_2 = (1.5 - 2) - 0.5 (2 - 2) = -0.5; e_3 = 1 - 0.5 (-0.5) = 1.25.
        assert_eq!(e, [0.0, 0.5, -0.5, 1.25]);
        assert!((ss - (0.25 + 0.25 + 1.5625)).abs() < 1e-15);
        let (fc, se) = forecast(&y, order, &[2.0, 0.5], 1.0, 3);
        assert!((fc[0] - 2.5).abs() < 1e-15 && (fc[1] - 2.25).abs() < 1e-15);
        assert!((se[1] - 1.25f64.sqrt()).abs() < 1e-15);
        // ARIMA(0,1,0) with drift: forecasts go on by the mean step, errors grow as sqrt(h).
        let walk = [0.0, 1.0, 3.0, 4.0, 6.0, 7.0, 9.0];
        let rw = fit(&walk, ArimaOrder { p: 0, d: 1, q: 0, constant: true, ..Default::default() }, 2).unwrap();
        assert!((rw.params[0] - 1.5).abs() < 1e-9, "{:?}", rw.params);
        assert!((rw.forecast[1] - 12.0).abs() < 1e-8);
        assert!((rw.forecast_se[1] / rw.forecast_se[0] - 2f64.sqrt()).abs() < 1e-12);
        assert!((rw.fitted[1] - 1.5).abs() < 1e-9, "{:?}", rw.fitted);
    }

    #[test]
    fn the_standard_error_of_a_mean_is_sigma_over_root_n_and_auto_finds_white_noise() {
        // ARIMA(0,0,0) with a mean: e_t = y_t - mu, J = -1, so SE = sigma / sqrt(n).
        let y = [3.0, 5.0, 4.0, 6.0, 2.0, 4.0];
        let order = ArimaOrder { p: 0, d: 0, q: 0, constant: true, ..Default::default() };
        let m = fit(&y, order, 1).unwrap();
        assert!((m.se[0] - (m.sigma2 / 6.0).sqrt()).abs() < 1e-12, "{:?}", m.se);
        // AR(1) without a constant: J_t = -y_{t-1}, SE = sigma / sqrt(sum y_{t-1}^2).
        let ar = ArimaOrder { p: 1, d: 0, q: 0, constant: false, ..Default::default() };
        let se = standard_errors(&y, ar, &[0.5], 2.0);
        let s: f64 = y[..5].iter().map(|v| v * v).sum();
        assert!((se[0] - (2.0 / s).sqrt()).abs() < 1e-12);
        // White noise around 10: auto picks a model no worse by AIC than every one it tried.
        let noise: Vec<f64> = (0..60).map(|i| 10.0 + (f64::from(i) * 2.399).sin()).collect();
        let auto = fit_auto(&noise, 0, 2, 2, true, Seasonal::default(), 1).unwrap();
        for p in 0..=2 {
            for q in 0..=2 {
                let m = fit(&noise, ArimaOrder { p, d: 0, q, constant: true, ..Default::default() }, 1).unwrap();
                assert!(auto.aic <= m.aic, "({p},{q}): {} < {}", m.aic, auto.aic);
            }
        }
        assert!(invert(vec![vec![1.0, 2.0], vec![2.0, 4.0]]).is_none());
    }

    #[test]
    fn a_seasonal_random_walk_forecasts_the_last_season_again() {
        // (0,0,0)(0,1,0)4 without a constant: the forecast repeats the last season, the bands grow every season.
        let y = [1.0, 5.0, 2.0, 8.0, 2.0, 6.0, 3.0, 9.0, 3.0, 7.0, 4.0, 10.0];
        let order = ArimaOrder {
            constant: true,
            seasonal: Seasonal { d: 1, period: 4, ..Default::default() },
            ..Default::default()
        };
        let m = fit(&y, order, 8).unwrap();
        // Seasonal differences are all 1: the drift is 1, the residuals 0.
        assert!((m.params[0] - 1.0).abs() < 1e-9 && m.ss < 1e-12, "{:?} {}", m.params, m.ss);
        assert!((m.forecast[0] - 4.0).abs() < 1e-8 && (m.forecast[5] - 9.0).abs() < 1e-8, "{:?}", m.forecast);
        assert!((m.fitted[4] - 2.0).abs() < 1e-8, "{:?}", m.fitted);
        // The expanded polynomials of (1,0,1)(1,0,1)3: AR lags 1, 3, 4 and MA lags 1, 3, 4.
        let o = ArimaOrder {
            p: 1,
            q: 1,
            seasonal: Seasonal { p: 1, q: 1, period: 3, ..Default::default() },
            ..Default::default()
        };
        let (_, ar, ma) = expand(&o, &[0.5, 0.2, 0.4, 0.3]);
        assert_eq!(ar, [0.5, 0.0, 0.4, -0.2]);
        assert!(
            (ma[0] - 0.2).abs() < 1e-15 && ma[1] == 0.0 && (ma[2] - 0.3).abs() < 1e-15 && (ma[3] - 0.06).abs() < 1e-15
        );
        let bad =
            ArimaOrder { p: 1, seasonal: Seasonal { p: 1, period: 1, ..Default::default() }, ..Default::default() };
        assert_eq!(fit(&y, bad, 1), Err(ArimaError::BadPeriod(1)));
    }

    #[test]
    fn auto_searches_the_seasonal_orders_up_to_the_given_ones() {
        // A seasonal AR(1) at period 4 around 10, with a little noise from an LCG.
        let mut st: u64 = 99;
        let mut y = vec![10.0; 4];
        for t in 4..120 {
            st = st.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let e = ((st >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 0.5;
            let v = 10.0 + 0.8 * (y[t - 4] - 10.0) + e;
            y.push(v);
        }
        let seasonal = Seasonal { p: 1, q: 1, period: 4, ..Default::default() };
        let m = fit_auto(&y, 0, 1, 1, true, seasonal, 1).unwrap();
        assert_eq!(m.order.seasonal.p, 1, "the seasonal AR term is found: {:?}", m.order);
        // No order in the search beats it.
        for sp in 0..=1 {
            for sq in 0..=1 {
                for p in 0..=1 {
                    for q in 0..=1 {
                        let s = Seasonal { p: sp, q: sq, period: 4, ..Default::default() };
                        if let Ok(o) =
                            fit(&y, ArimaOrder { p, q, constant: true, seasonal: s, ..Default::default() }, 1)
                        {
                            assert!(m.aic <= o.aic, "({p},{q})({sp},{sq}) {} < {}", o.aic, m.aic);
                        }
                    }
                }
            }
        }
        let bad = Seasonal { p: 1, period: 1, ..Default::default() };
        assert_eq!(fit_auto(&y, 0, 1, 1, true, bad, 1).unwrap_err(), ArimaError::BadPeriod(1));
    }

    #[test]
    fn bad_input_is_refused() {
        let none = ArimaOrder { p: 0, d: 0, q: 0, constant: false, ..Default::default() };
        assert_eq!(fit(&[1.0; 10], none, 1), Err(ArimaError::Empty));
        let ar2 = ArimaOrder { p: 2, d: 1, q: 0, constant: true, ..Default::default() };
        assert_eq!(fit(&[1.0, 2.0, 3.0, 4.0], ar2, 1), Err(ArimaError::TooShort(4)));
        assert_eq!(fit(&[1.0, f64::NAN, 3.0], ar2, 1), Err(ArimaError::Missing(1)));
    }
}
