//! Stationarity tests: augmented Dickey-Fuller and KPSS (Origin 2024b's Time Series Analysis app, "check
//! stationarity"; not in Origin 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 41).
//!
//! **ADF**: the least-squares regression `dy_t = [a + b t] + g y_{t-1} + sum_{i<=p} d_i dy_{t-i} + e_t`; the statistic
//! is the t value of `g`. With the lag order chosen by AIC, every order `0..=maxlag` is fitted on the same sample (the
//! first `maxlag` differences dropped), the order of smallest `AIC = -2 lnL + 2k` (`lnL = -m/2 (ln 2pi + ln(SSR/m) +
//! 1)`) is kept, and the regression is fitted again on all the points that order allows; `maxlag = ceil(12 (n /
//! 100)^(1/4))`, at most `n/2 - k - 1` (the convention of statsmodels' `adfuller`). The critical values are MacKinnon's
//! (2010) finite-sample ones at the regression's points `T`, `b0 + b1 / T + b2 / T^2 + b3 / T^3` (T497; they were
//! the asymptotic `b0` before), and the p-value is MacKinnon's (1994) response surface `Phi(sum c_i tau^i)` (the
//! small-tau coefficients up to his `tau*`, the large-tau ones above; 0 below his smallest tau, 1 above his largest),
//! as statsmodels' `adfuller` reports them.
//!
//! **KPSS**: the residuals `e` of `y` on a constant (level) or a constant and time (trend), their partial sums `S_t`,
//! and `eta = sum S_t^2 / (n^2 s^2)` with the Newey-West long-run variance `s^2 = (sum e_t^2 + 2 sum_{s<=l} (1 - s /
//! (l + 1)) sum_t e_t e_{t-s}) / n`, `l = ceil(12 (n / 100)^(1/4))` unless given. The p-value is interpolated in
//! Kwiatkowski et al.'s (1992) table (0.10, 0.05, 0.025, 0.01) and held at its ends. Automatic lags (T497, Hobijn et
//! al. 1998, statsmodels' `nlags="auto"`): with `c = floor(n^(2/9))` autocovariances `g_i = 2 sum_t e_t e_{t-i} / n`,
//! `s0 = sum e^2 / n + sum_{i<=c} g_i`, `s1 = sum_{i<=c} i g_i`, the lags are `floor(1.1447 (s1 / s0)^(2/3) n^(1/3))`,
//! at most `n - 1`.
//!
//! The statistics are checked against numpy in `crates/calc/tests/stationarity_reference.rs` (made by
//! `tools/stationarity-ref.py`); the p-values, critical values and automatic lags against statsmodels 0.15 in
//! `adf_p_values_critical_values_and_kpss_auto_lags_match_statsmodels`.

use crate::stability::invert;
use statrs::distribution::{ContinuousCDF, Normal};

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
    /// MacKinnon's (2010) critical values at 1 %, 5 % and 10 % for the regression's points (T497).
    pub critical: [f64; 3],
    /// MacKinnon's (1994) approximate p-value of the statistic (T497).
    pub p: f64,
}

/// MacKinnon's (1994) approximate p-value of an ADF statistic `tau` (T497; one I(1) series).
pub fn adf_p_value(tau: f64, terms: AdfTerms) -> f64 {
    // (small-tau coefficients, large-tau coefficients, tau*, smallest tau, largest tau), scaled as published.
    let (small, large, star, min, max): ([f64; 3], [f64; 4], f64, f64, f64) = match terms {
        AdfTerms::Constant => (
            [2.1659, 1.4412, 3.8269 * 1e-2],
            [1.7339, 9.3202 * 1e-1, -1.2745 * 1e-1, -1.0368 * 1e-2],
            -1.61,
            -18.83,
            2.74,
        ),
        AdfTerms::Trend => (
            [3.2512, 1.6047, 4.9588 * 1e-2],
            [2.5261, 6.1654 * 1e-1, -3.7956 * 1e-1, -6.0285 * 1e-2],
            -2.89,
            -16.18,
            0.7,
        ),
        AdfTerms::None => (
            [0.6344, 1.2378, 3.2496 * 1e-2],
            [0.4797, 9.3557 * 1e-1, -0.6999 * 1e-1, 3.3066 * 1e-2],
            -1.04,
            -19.04,
            f64::INFINITY,
        ),
    };
    if tau > max {
        return 1.0;
    }
    if tau < min {
        return 0.0;
    }
    let c: &[f64] = if tau <= star { &small } else { &large };
    let z = c.iter().rev().fold(0.0, |acc, b| acc * tau + b);
    Normal::new(0.0, 1.0).expect("the standard normal").cdf(z)
}

/// MacKinnon's (2010) critical values at 1 %, 5 % and 10 % of the ADF statistic for a regression on `nobs` points
/// (T497).
pub fn adf_critical_values(terms: AdfTerms, nobs: usize) -> [f64; 3] {
    let table: [[f64; 4]; 3] = match terms {
        AdfTerms::None => {
            [[-2.56574, -2.2358, -3.627, 0.0], [-1.941, -0.2686, -3.365, 31.223], [-1.61682, 0.2656, -2.714, 25.364]]
        }
        AdfTerms::Constant => [
            [-3.43035, -6.5393, -16.786, -79.433],
            [-2.86154, -2.8903, -4.234, -40.04],
            [-2.56677, -1.5384, -2.809, 0.0],
        ],
        AdfTerms::Trend => [
            [-3.95877, -9.0531, -28.428, -134.155],
            [-3.41049, -4.3904, -9.036, -45.374],
            [-3.12705, -2.5856, -3.925, -22.38],
        ],
    };
    let x = 1.0 / nobs as f64;
    table.map(|b| b.iter().rev().fold(0.0, |acc, c| acc * x + c))
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
    let statistic = beta[0] / se[0];
    Ok(Adf {
        statistic,
        lags: used,
        nobs: m,
        critical: adf_critical_values(terms, m),
        p: adf_p_value(statistic, terms),
    })
}

/// The KPSS residuals: `y` less its mean (level) or its least-squares line (trend).
fn kpss_residuals(y: &[f64], null: KpssNull) -> Result<Vec<f64>, StationarityError> {
    let n = y.len();
    Ok(match null {
        KpssNull::Level => {
            let m = y.iter().sum::<f64>() / n as f64;
            y.iter().map(|v| v - m).collect()
        }
        KpssNull::Trend => {
            let x: Vec<Vec<f64>> = (0..n).map(|t| vec![1.0, (t + 1) as f64]).collect();
            let (b, _, _) = ols(&x, y)?;
            y.iter().enumerate().map(|(t, v)| v - b[0] - b[1] * (t + 1) as f64).collect()
        }
    })
}

/// The KPSS lags of Hobijn et al. (1998) for `y` (T497; statsmodels' `nlags="auto"`, see the module notes).
pub fn kpss_auto_lags(y: &[f64], null: KpssNull) -> Result<usize, StationarityError> {
    check(y, 4)?;
    let e = kpss_residuals(y, null)?;
    let n = e.len();
    let nf = n as f64;
    let cov = nf.powf(2.0 / 9.0) as usize;
    let mut s0 = e.iter().map(|v| v * v).sum::<f64>() / nf;
    let mut s1 = 0.0;
    for i in 1..=cov.min(n - 1) {
        let g = (i..n).map(|t| e[t] * e[t - i]).sum::<f64>() / (nf / 2.0);
        s0 += g;
        s1 += i as f64 * g;
    }
    let s = s1 / s0;
    let lags = 1.1447 * (s * s).powf(1.0 / 3.0) * nf.powf(1.0 / 3.0);
    Ok(if lags.is_finite() && lags > 0.0 { (lags as usize).min(n - 1) } else { 0 })
}

/// The KPSS test of `y` with `lags` autocovariances in the long-run variance (`None`: `ceil(12 (n / 100)^(1/4))`).
pub fn kpss(y: &[f64], null: KpssNull, lags: Option<usize>) -> Result<Kpss, StationarityError> {
    check(y, 4)?;
    let n = y.len();
    let l = lags.unwrap_or_else(|| schwert_lags(n)).min(n - 1);
    let e = kpss_residuals(y, null)?;
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

    #[test]
    fn adf_p_values_critical_values_and_kpss_auto_lags_match_statsmodels() {
        // T497: statsmodels 0.15's adfuller (autolag AIC), mackinnonp and kpss(nlags="auto")
        // (re-workspace/scratch/t480/stat_ref.py).
        let close = |a: f64, b: f64, tol: f64| (a - b).abs() <= tol * b.abs().max(1e-300);
        for (tau, c, ct, n) in [
            (-20.0, 0.0, 0.0, 0.0),
            (-5.0, 2.2193154713956276e-05, 0.00020574728263882532, 1.0570487880413703e-06),
            (-3.0, 0.034894400275345266, 0.1320809847799973, 0.0026637350127542685),
            (-1.7, 0.4311124768686236, 0.7508711887741886, 0.08441908940680137),
            (-1.0, 0.7532643012005655, 0.9441147109023218, 0.28810611212633064),
            (0.5, 0.9848730963065522, 0.996851911498776, 0.824879195252956),
            (3.0, 1.0, 1.0, 0.9998068584192428),
        ] {
            for (terms, want) in [(AdfTerms::Constant, c), (AdfTerms::Trend, ct), (AdfTerms::None, n)] {
                let got = adf_p_value(tau, terms);
                assert!(got == want || close(got, want, 1e-10), "{tau} {terms:?}: {got} vs {want}");
            }
        }
        let e: Vec<f64> = (0..200).map(|t| ((t * 7919 + 13) % 101) as f64 / 101.0 - 0.5).collect();
        let mut walk = Vec::new();
        let mut level = 0.0;
        for v in &e {
            level += v;
            walk.push(level);
        }
        let mut ar: Vec<f64> = Vec::new();
        for (t, v) in e.iter().enumerate() {
            ar.push(v + if t > 0 { 0.5 * ar[t - 1] } else { 0.0 });
        }
        let trend: Vec<f64> = e.iter().enumerate().map(|(t, v)| 0.05 * t as f64 + v).collect();
        type AdfRow = (AdfTerms, f64, f64, usize, usize, [f64; 3]);
        let adfs: [(&[f64], [AdfRow; 3], [(KpssNull, f64, usize); 2]); 3] = [
            (
                &walk,
                [
                    (
                        AdfTerms::Constant,
                        -3.5975153137353026,
                        0.0058087076754134905,
                        5,
                        194,
                        [-3.4645146202692527, -2.8765564361715534, -2.5747745328940375],
                    ),
                    (
                        AdfTerms::Trend,
                        -5.227094834548966,
                        7.783359814229448e-05,
                        7,
                        192,
                        [-4.006711675454599, -3.433608194523564, -3.1406263010660807],
                    ),
                    (
                        AdfTerms::None,
                        -1.4534859761043104,
                        0.1365233723259192,
                        5,
                        194,
                        [-2.5773611127643745, -1.9424696688463448, -1.6155195657809531],
                    ),
                ],
                [(KpssNull::Level, 1.0857913558997527, 9), (KpssNull::Trend, 0.05164389019685711, 8)],
            ),
            (
                &ar,
                [
                    (
                        AdfTerms::Constant,
                        -5.011051347641449,
                        2.110219858100462e-05,
                        13,
                        186,
                        [-3.466005071659723, -2.8772078537639385, -2.5751221620996647],
                    ),
                    (
                        AdfTerms::Trend,
                        -4.988698988156388,
                        0.0002157222515360818,
                        13,
                        186,
                        [-4.008285142312431, -3.434362538624019, -3.1410680056243683],
                    ),
                    (
                        AdfTerms::None,
                        -4.975037052854765,
                        1.1827496314497832e-06,
                        13,
                        186,
                        [-2.577865268817204, -1.9425364994337093, -1.6154665497285408],
                    ),
                ],
                [(KpssNull::Level, 0.036007287620418396, 4), (KpssNull::Trend, 0.03602273252490094, 4)],
            ),
            (
                &trend,
                [
                    (
                        AdfTerms::Constant,
                        0.13515553569964503,
                        0.9683809646357376,
                        4,
                        195,
                        [-3.464337030867007, -2.876478799035722, -2.574733103221565],
                    ),
                    (
                        AdfTerms::Trend,
                        -4.363827585395453,
                        0.002499251093281504,
                        4,
                        195,
                        [-4.005961859943694, -3.433248624251926, -3.1404157270014665],
                    ),
                    (
                        AdfTerms::None,
                        1.064077383869922,
                        0.9244718358150754,
                        15,
                        184,
                        [-2.5779982171550095, -1.942554162032804, -1.615452613190392],
                    ),
                ],
                [(KpssNull::Level, 2.1052882440065117, 9), (KpssNull::Trend, 0.03092125509130415, 14)],
            ),
        ];
        for (y, rows, kp) in adfs {
            for (terms, stat, p, lags, nobs, crit) in rows {
                let a = adf(y, terms, None).unwrap();
                assert_eq!((a.lags, a.nobs), (lags, nobs), "{terms:?}");
                assert!(close(a.statistic, stat, 1e-9), "{terms:?} {} vs {stat}", a.statistic);
                assert!(close(a.p, p, 1e-8), "{terms:?} p {} vs {p}", a.p);
                for (g, w) in a.critical.iter().zip(crit) {
                    assert!(close(*g, w, 1e-12), "{terms:?} critical {g} vs {w}");
                }
            }
            for (null, stat, lags) in kp {
                let l = kpss_auto_lags(y, null).unwrap();
                assert_eq!(l, lags, "{null:?}");
                let k = kpss(y, null, Some(l)).unwrap();
                assert!(close(k.statistic, stat, 1e-9), "{null:?} {} vs {stat}", k.statistic);
            }
        }
    }

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
