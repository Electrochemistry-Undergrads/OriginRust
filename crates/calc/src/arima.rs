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
/// orders too long for the series, without coefficients or without finite errors are passed over.
pub fn fit_auto(
    y: &[f64],
    d: usize,
    max_p: usize,
    max_q: usize,
    constant: bool,
    seasonal: Seasonal,
    h: usize,
) -> Result<Arima, ArimaError> {
    let mut best: Option<Arima> = None;
    for p in 0..=max_p {
        for q in 0..=max_q {
            match fit(y, ArimaOrder { p, d, q, constant, seasonal }, h) {
                Ok(m) => {
                    if best.as_ref().is_none_or(|b| m.aic < b.aic) {
                        best = Some(m);
                    }
                }
                Err(e @ ArimaError::Missing(_)) => return Err(e),
                Err(_) => {}
            }
        }
    }
    best.ok_or(ArimaError::TooShort(y.len()))
}

/// Forecasts `h` steps of `y` and their standard errors for `params` and `sigma2`.
pub fn forecast(y: &[f64], order: ArimaOrder, params: &[f64], sigma2: f64, h: usize) -> (Vec<f64>, Vec<f64>) {
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
    // Undo the differences level by level (seasonal ones first): x_t = dx_t + x_{t - lag}.
    let mut fc: Vec<f64> = w[n..].to_vec();
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
    // Psi weights of phi(B)(1 - B)^d (1 - B^s)^D y = theta(B) e with the full polynomials.
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
    let se = psi
        .iter()
        .take(h)
        .map(|p| {
            acc += p * p;
            (sigma2 * acc).sqrt()
        })
        .collect();
    (fc, se)
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

/// Fits ARIMA(`order`) to `y` and forecasts `h` steps.
pub fn fit(y: &[f64], order: ArimaOrder, h: usize) -> Result<Arima, ArimaError> {
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
    Ok(Arima { order, params: best, ss, sigma2, aic, residuals, fitted, forecast: fc, forecast_se: fse, se })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn bad_input_is_refused() {
        let none = ArimaOrder { p: 0, d: 0, q: 0, constant: false, ..Default::default() };
        assert_eq!(fit(&[1.0; 10], none, 1), Err(ArimaError::Empty));
        let ar2 = ArimaOrder { p: 2, d: 1, q: 0, constant: true, ..Default::default() };
        assert_eq!(fit(&[1.0, 2.0, 3.0, 4.0], ar2, 1), Err(ArimaError::TooShort(4)));
        assert_eq!(fit(&[1.0, f64::NAN, 3.0], ar2, 1), Err(ArimaError::Missing(1)));
    }
}
