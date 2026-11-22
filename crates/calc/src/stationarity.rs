//! Stationarity tests: augmented Dickey-Fuller and KPSS (Origin 2024b's Time Series Analysis app, "check
//! stationarity"; not in Origin 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 41).
//!
//! **ADF**: the least-squares regression `dy_t = [a + b t] + g y_{t-1} + sum_{i<=p} d_i dy_{t-i} + e_t`; the statistic
//! is the t value of `g`. With the lag order chosen by AIC, every order `0..=maxlag` is fitted on the same sample (the
//! first `maxlag` differences dropped), the order of smallest `AIC = -2 lnL + 2k` (`lnL = -m/2 (ln 2pi + ln(SSR/m) +
//! 1)`) is kept, and the regression is fitted again on all the points that order allows; `maxlag = ceil(12 (n /
//! 100)^(1/4))`, at most `n/2 - k - 1` (the convention of statsmodels' `adfuller`). The critical values are MacKinnon's
//! (2010) asymptotic ones; no p-value is computed.
//!
//! **KPSS**: the residuals `e` of `y` on a constant (level) or a constant and time (trend), their partial sums `S_t`,
//! and `eta = sum S_t^2 / (n^2 s^2)` with the Newey-West long-run variance `s^2 = (sum e_t^2 + 2 sum_{s<=l} (1 - s /
//! (l + 1)) sum_t e_t e_{t-s}) / n`, `l = ceil(12 (n / 100)^(1/4))` unless given. The p-value is interpolated in
//! Kwiatkowski et al.'s (1992) table (0.10, 0.05, 0.025, 0.01) and held at its ends.
//!
//! The statistics are checked against numpy in `crates/calc/tests/stationarity_reference.rs` (made by
//! `tools/stationarity-ref.py`); the published critical values cannot be checked here.

use crate::stability::invert;

/// The deterministic terms of the ADF regression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdfTerms {
    /// A constant (default).
    #[default]
    Constant,
    /// A constant and a linear trend.
    Trend,
    /// Neither.
    None,
}

/// The null hypothesis of the KPSS test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KpssNull {
    /// Stationary about a level (default).
    #[default]
    Level,
    /// Stationary about a linear trend.
    Trend,
}

/// The ADF result.
#[derive(Debug, Clone, PartialEq)]
pub struct Adf {
    pub statistic: f64,
    pub lags: usize,
    /// Points in the final regression.
    pub nobs: usize,
    /// MacKinnon's asymptotic critical values at 1 %, 5 % and 10 %.
    pub critical: [f64; 3],
}

/// The KPSS result.
#[derive(Debug, Clone, PartialEq)]
pub struct Kpss {
    pub statistic: f64,
    pub lags: usize,
    /// Interpolated in the table; 0.10 means 0.10 or more, 0.01 means 0.01 or less.
    pub p: f64,
    /// The table's critical values at 10 %, 5 %, 2.5 % and 1 %.
    pub critical: [f64; 4],
}

/// Why a test could not be run.
#[derive(Debug, Clone, PartialEq)]
pub enum StationarityError {
    TooShort(usize),
    Missing(usize),
    TooManyLags(usize),
    Singular,
}

impl std::fmt::Display for StationarityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StationarityError::TooShort(n) => write!(f, "{n} values are too few for the test"),
            StationarityError::Missing(i) => write!(f, "value {} is missing: the series must be complete", i + 1),
            StationarityError::TooManyLags(l) => write!(f, "{l} lags are too many for this series"),
            StationarityError::Singular => f.write_str("the regression has no unique solution (a constant series?)"),
        }
    }
}

impl std::error::Error for StationarityError {}

fn check(y: &[f64], min: usize) -> Result<(), StationarityError> {
    if let Some(i) = y.iter().position(|v| !v.is_finite()) {
        return Err(StationarityError::Missing(i));
    }
    if y.len() < min {
        return Err(StationarityError::TooShort(y.len()));
    }
    Ok(())
}

/// `ceil(12 (n / 100)^(1/4))`.
pub fn schwert_lags(n: usize) -> usize {
    (12.0 * (n as f64 / 100.0).powf(0.25)).ceil() as usize
}

/// Least squares of `y` on the columns `x` (rows of regressors): coefficients, their standard errors and SSR.
fn ols(x: &[Vec<f64>], y: &[f64]) -> Result<(Vec<f64>, Vec<f64>, f64), StationarityError> {
    let k = x[0].len();
    let mut xtx = vec![vec![0.0; k]; k];
    let mut xty = vec![0.0; k];
    for (r, yi) in x.iter().zip(y) {
        for i in 0..k {
            xty[i] += r[i] * yi;
            for j in 0..k {
                xtx[i][j] += r[i] * r[j];
            }
        }
    }
    let inv = invert(&xtx).ok_or(StationarityError::Singular)?;
    let beta: Vec<f64> = (0..k).map(|i| (0..k).map(|j| inv[i][j] * xty[j]).sum()).collect();
    let ssr: f64 =
        x.iter().zip(y).map(|(r, yi)| (yi - r.iter().zip(&beta).map(|(a, b)| a * b).sum::<f64>()).powi(2)).sum();
    let m = y.len();
    if m <= k {
        return Err(StationarityError::TooShort(m));
    }
    let s2 = ssr / (m - k) as f64;
    let se = (0..k).map(|i| (s2 * inv[i][i]).sqrt()).collect();
    Ok((beta, se, ssr))
}

fn deterministic(terms: AdfTerms) -> usize {
    match terms {
        AdfTerms::None => 0,
        AdfTerms::Constant => 1,
        AdfTerms::Trend => 2,
    }
}

/// The ADF regression with `p` lagged differences on the last `m` differences: rows `[y_{t-1}, dy_{t-1}.., const,
/// trend]` and the left side `dy_t`.
fn adf_design(y: &[f64], p: usize, m: usize, terms: AdfTerms) -> (Vec<Vec<f64>>, Vec<f64>) {
    let d: Vec<f64> = y.windows(2).map(|w| w[1] - w[0]).collect();
    let start = d.len() - m;
    let mut rows = Vec::with_capacity(m);
    let mut lhs = Vec::with_capacity(m);
    for (j, i) in (start..d.len()).enumerate() {
        let mut r = vec![y[i]];
        r.extend((1..=p).map(|l| d[i - l]));
        if terms != AdfTerms::None {
            r.push(1.0);
        }
        if terms == AdfTerms::Trend {
            // statsmodels' add_trend counts time from 1 within the regression sample.
            r.push((j + 1) as f64);
        }
        rows.push(r);
        lhs.push(d[i]);
    }
    (rows, lhs)
}

/// The augmented Dickey-Fuller test of `y` with `lags` lagged differences, or the AIC choice when `None`.
pub fn adf(y: &[f64], terms: AdfTerms, lags: Option<usize>) -> Result<Adf, StationarityError> {
    check(y, 6)?;
    let n = y.len();
    let nd = n - 1;
    let k_det = deterministic(terms);
    let used = match lags {
        Some(p) => {
            if p + k_det + 2 >= nd.saturating_sub(p) {
                return Err(StationarityError::TooManyLags(p));
            }
            p
        }
        None => {
            let maxlag = schwert_lags(n).min((n / 2).saturating_sub(k_det + 1));
            let m = nd - maxlag;
            let mut best = (f64::INFINITY, 0);
            for p in 0..=maxlag {
                let (x, d) = adf_design(y, p, m, terms);
                let (_, _, ssr) = ols(&x, &d)?;
                let mf = m as f64;
                let ll = -mf / 2.0 * ((2.0 * std::f64::consts::PI).ln() + (ssr / mf).ln() + 1.0);
                let aic = -2.0 * ll + 2.0 * (p + 1 + k_det) as f64;
                if aic < best.0 {
                    best = (aic, p);
                }
            }
            best.1
        }
    };
    let m = nd - used;
    let (x, d) = adf_design(y, used, m, terms);
    let (beta, se, _) = ols(&x, &d)?;
    let critical = match terms {
        AdfTerms::None => [-2.56574, -1.94100, -1.61682],
        AdfTerms::Constant => [-3.43035, -2.86154, -2.56677],
        AdfTerms::Trend => [-3.95877, -3.41049, -3.12705],
    };
    Ok(Adf { statistic: beta[0] / se[0], lags: used, nobs: m, critical })
}

/// The KPSS test of `y` with `lags` autocovariances in the long-run variance (`None`: `ceil(12 (n / 100)^(1/4))`).
pub fn kpss(y: &[f64], null: KpssNull, lags: Option<usize>) -> Result<Kpss, StationarityError> {
    check(y, 4)?;
    let n = y.len();
    let l = lags.unwrap_or_else(|| schwert_lags(n)).min(n - 1);
    let e: Vec<f64> = match null {
        KpssNull::Level => {
            let m = y.iter().sum::<f64>() / n as f64;
            y.iter().map(|v| v - m).collect()
        }
        KpssNull::Trend => {
            let x: Vec<Vec<f64>> = (0..n).map(|t| vec![1.0, (t + 1) as f64]).collect();
            let (b, _, _) = ols(&x, y)?;
            y.iter().enumerate().map(|(t, v)| v - b[0] - b[1] * (t + 1) as f64).collect()
        }
    };
    let mut s = 0.0;
    let eta: f64 = e
        .iter()
        .map(|v| {
            s += v;
            s * s
        })
        .sum::<f64>()
        / (n * n) as f64;
    let mut lr: f64 = e.iter().map(|v| v * v).sum();
    for i in 1..=l {
        let w = 1.0 - i as f64 / (l + 1) as f64;
        lr += 2.0 * w * (i..n).map(|t| e[t] * e[t - i]).sum::<f64>();
    }
    lr /= n as f64;
    if lr <= 0.0 {
        return Err(StationarityError::Singular);
    }
    let statistic = eta / lr;
    let critical = match null {
        KpssNull::Level => [0.347, 0.463, 0.574, 0.739],
        KpssNull::Trend => [0.119, 0.146, 0.176, 0.216],
    };
    let pvals = [0.10, 0.05, 0.025, 0.01];
    let p = if statistic <= critical[0] {
        pvals[0]
    } else if statistic >= critical[3] {
        pvals[3]
    } else {
        let i = (0..3).find(|&i| statistic <= critical[i + 1]).unwrap_or(2);
        let f = (statistic - critical[i]) / (critical[i + 1] - critical[i]);
        pvals[i] + f * (pvals[i + 1] - pvals[i])
    };
    Ok(Kpss { statistic, lags: l, p, critical })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A random walk from a fixed generator, and its differences (white noise).
    fn walk(n: usize) -> Vec<f64> {
        let mut st: u64 = 99;
        let mut y = 0.0;
        (0..n)
            .map(|_| {
                st = st.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                y += (st >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
                y
            })
            .collect()
    }

    #[test]
    fn a_walk_looks_non_stationary_and_its_steps_stationary() {
        let w = walk(200);
        let steps: Vec<f64> = w.windows(2).map(|p| p[1] - p[0]).collect();
        let a = adf(&w, AdfTerms::Constant, None).unwrap();
        assert!(a.statistic > a.critical[1], "a walk: {}", a.statistic);
        let a = adf(&steps, AdfTerms::Constant, None).unwrap();
        assert!(a.statistic < a.critical[0], "white noise: {}", a.statistic);
        let k = kpss(&w, KpssNull::Level, None).unwrap();
        assert!(k.p <= 0.05, "{k:?}");
        let k = kpss(&steps, KpssNull::Level, None).unwrap();
        assert_eq!(k.p, 0.10, "{k:?}");
        assert_eq!(schwert_lags(100), 12);
    }

    #[test]
    fn bad_input_is_refused() {
        assert_eq!(adf(&[1.0, 2.0, 3.0], AdfTerms::Constant, None), Err(StationarityError::TooShort(3)));
        assert_eq!(kpss(&[1.0, f64::NAN, 3.0, 4.0], KpssNull::Level, None), Err(StationarityError::Missing(1)));
        assert_eq!(adf(&walk(20), AdfTerms::Constant, Some(9)), Err(StationarityError::TooManyLags(9)));
        assert_eq!(kpss(&[2.0; 10], KpssNull::Level, None), Err(StationarityError::Singular));
    }
}
