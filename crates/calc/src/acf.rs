//! Autocorrelation of a time series (Origin's Time Series Analysis app, 2024b: "inspect autocorrelation"; not in
//! Origin 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 34). The formulas are the
//! textbook ones (Box, Jenkins and Reinsel):
//!
//! - ACF `r_k = sum_{t<n-k} (x_t - m)(x_{t+k} - m) / sum_t (x_t - m)^2`, with Bartlett's standard error
//!   `sqrt((1 + 2 sum_{j<k} r_j^2) / n)` for lag `k` (lag 1: `1/sqrt n`);
//! - PACF by the Durbin-Levinson recursion on the `r_k`, standard error `1/sqrt n`;
//! - Ljung-Box `Q_k = n (n + 2) sum_{j<=k} r_j^2 / (n - j)` with its upper chi-square probability on `k` degrees of
//!   freedom.
//!
//! Checked against an independent numpy/scipy computation (the PACF from solving the Yule-Walker equations of each
//! lag) in `crates/calc/tests/acf_reference.rs`, made by `tools/acf-ref.py`.

use statrs::distribution::{ChiSquared, ContinuousCDF};

/// One lag of the result (lag 0 is not listed: its ACF is 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lag {
    pub lag: usize,
    pub acf: f64,
    /// Bartlett standard error of `acf`.
    pub acf_se: f64,
    pub pacf: f64,
    /// `1 / sqrt n`.
    pub pacf_se: f64,
    /// Ljung-Box statistic over lags 1..=lag and its upper chi-square probability.
    pub ljung_box: f64,
    pub p: f64,
}

/// Why the autocorrelation could not be computed.
#[derive(Debug, Clone, PartialEq)]
pub enum AcfError {
    TooFewPoints(usize),
    Constant,
    NoLags,
}

impl std::fmt::Display for AcfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcfError::TooFewPoints(n) => write!(f, "{n} points are too few: more points than lags are needed"),
            AcfError::Constant => f.write_str("the series does not vary"),
            AcfError::NoLags => f.write_str("the number of lags must be at least 1"),
        }
    }
}

impl std::error::Error for AcfError {}

/// The default number of lags: `min(10 log10 n, n - 1)` rounded down (the R and statsmodels default).
pub fn default_lags(n: usize) -> usize {
    if n < 2 {
        return 0;
    }
    ((10.0 * (n as f64).log10()).floor() as usize).clamp(1, n - 1)
}

/// ACF, PACF and Ljung-Box of `x` (missing values must be removed first) for lags 1..=`lags`.
pub fn autocorrelation(x: &[f64], lags: usize) -> Result<Vec<Lag>, AcfError> {
    if lags == 0 {
        return Err(AcfError::NoLags);
    }
    let n = x.len();
    if n <= lags {
        return Err(AcfError::TooFewPoints(n));
    }
    let m = x.iter().sum::<f64>() / n as f64;
    let c0: f64 = x.iter().map(|v| (v - m) * (v - m)).sum();
    if c0 <= 0.0 {
        return Err(AcfError::Constant);
    }
    let r: Vec<f64> = (0..=lags).map(|k| (0..n - k).map(|t| (x[t] - m) * (x[t + k] - m)).sum::<f64>() / c0).collect();
    // Durbin-Levinson: phi[k][j] for j = 1..=k; the PACF at lag k is phi[k][k].
    let mut pacf = vec![0.0; lags + 1];
    let mut prev: Vec<f64> = Vec::new();
    for k in 1..=lags {
        let num = r[k] - (1..k).map(|j| prev[j - 1] * r[k - j]).sum::<f64>();
        let den = 1.0 - (1..k).map(|j| prev[j - 1] * r[j]).sum::<f64>();
        let phi_kk = if den.abs() > 0.0 { num / den } else { f64::NAN };
        let mut next: Vec<f64> = (1..k).map(|j| prev[j - 1] - phi_kk * prev[k - j - 1]).collect();
        next.push(phi_kk);
        pacf[k] = phi_kk;
        prev = next;
    }
    let nf = n as f64;
    let mut q = 0.0;
    let mut sum_r2 = 0.0;
    Ok((1..=lags)
        .map(|k| {
            let acf_se = ((1.0 + 2.0 * sum_r2) / nf).sqrt();
            sum_r2 += r[k] * r[k];
            q += r[k] * r[k] / (nf - k as f64);
            let ljung_box = nf * (nf + 2.0) * q;
            let p = ChiSquared::new(k as f64).map_or(f64::NAN, |d| d.sf(ljung_box));
            Lag { lag: k, acf: r[k], acf_se, pacf: pacf[k], pacf_se: 1.0 / nf.sqrt(), ljung_box, p }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_alternating_series_has_negative_lag_one_and_ar1_has_a_single_partial() {
        let x: Vec<f64> = (0..20).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let r = autocorrelation(&x, 3).unwrap();
        assert!((r[0].acf + 0.95).abs() < 1e-12, "{}", r[0].acf);
        assert!((r[1].acf - 0.9).abs() < 1e-12);
        assert!((r[0].pacf - r[0].acf).abs() < 1e-15, "the first partial is the first autocorrelation");
        assert!((r[0].acf_se - 1.0 / 20f64.sqrt()).abs() < 1e-15);
        assert!(r[2].p < 1e-6, "strongly autocorrelated");
        assert_eq!(default_lags(100), 20);
        assert_eq!(default_lags(5), 4);
    }

    #[test]
    fn bad_input_is_refused() {
        assert_eq!(autocorrelation(&[1.0, 2.0], 0), Err(AcfError::NoLags));
        assert_eq!(autocorrelation(&[1.0, 2.0], 2), Err(AcfError::TooFewPoints(2)));
        assert_eq!(autocorrelation(&[3.0; 5], 2), Err(AcfError::Constant));
    }
}
