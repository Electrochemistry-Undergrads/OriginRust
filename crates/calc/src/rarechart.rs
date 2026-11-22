//! G and T control charts for rare events (Origin 2027's Quality improvement apps; not in Origin 2016, so nothing
//! here is measured in Origin; `docs/spec/statistics.md` section 35).
//!
//! - **G chart**: each value is the number of opportunities (units, days, cases) between two events, 0 or more.
//!   The counts are taken as geometric on 0, 1, 2, ... with `p = 1 / (gbar + 1)` (`gbar` their mean, the maximum
//!   likelihood estimate). The k-sigma limits are `gbar +- k sqrt(gbar (gbar + 1))` (the lower one at least 0) with
//!   the centre `gbar`; the probability limits are the quantiles at `alpha`, 0.5 and `1 - alpha`, `alpha` the
//!   normal upper tail beyond `k` (0.00135 for k = 3), from the continuous form of the geometric quantile
//!   `max(0, ln(1 - u) / ln(1 - p) - 1)` (rounding it up gives the discrete quantile).
//! - **T chart**: each value is the time between two events, above 0, fitted by an exponential (the mean) or a
//!   Weibull (maximum likelihood) distribution; the limits are its quantiles at `alpha`, 0.5 and `1 - alpha`.
//!
//! Checked against scipy in `crates/calc/tests/rarechart_reference.rs` (made by `tools/rarechart-ref.py`).

use crate::distfit::{Family, estimate, quantile};
use statrs::distribution::{ContinuousCDF, Normal};

/// The limits of the G chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GLimits {
    /// Quantiles of the fitted geometric distribution (default).
    #[default]
    Probability,
    /// `gbar +- k sqrt(gbar (gbar + 1))`.
    Sigma,
}

/// The distribution fitted to the times of the T chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TDistribution {
    /// Weibull by maximum likelihood (default).
    #[default]
    Weibull,
    Exponential,
}

/// A fitted chart.
#[derive(Debug, Clone, PartialEq)]
pub struct RareChart {
    /// The values used (missing values left out).
    pub values: Vec<f64>,
    pub mean: f64,
    /// G chart: `[p]`; T chart: `[mean]` (exponential) or `[shape, scale]` (Weibull).
    pub params: Vec<f64>,
    pub center: f64,
    pub ucl: f64,
    pub lcl: f64,
    /// Indices into `values` above the upper limit and below the lower limit.
    pub above: Vec<usize>,
    pub below: Vec<usize>,
}

/// Why a chart could not be made.
#[derive(Debug, Clone, PartialEq)]
pub enum RareChartError {
    TooFewValues,
    /// A G chart count below 0 or a T chart time not above 0.
    OutOfRange(f64),
    AllZero,
    BadSigma,
    NoFit(String),
}

impl std::fmt::Display for RareChartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RareChartError::TooFewValues => f.write_str("at least 2 values are needed"),
            RareChartError::OutOfRange(v) => write!(f, "the value {v} is out of range (G: 0 or more; T: above 0)"),
            RareChartError::AllZero => f.write_str("every count is 0: events at every opportunity"),
            RareChartError::BadSigma => f.write_str("the number of sigmas must be above 0"),
            RareChartError::NoFit(e) => write!(f, "the distribution could not be fitted: {e}"),
        }
    }
}

impl std::error::Error for RareChartError {}

/// The normal upper tail probability beyond `k` (0.00135 for 3).
pub fn tail(k: f64) -> f64 {
    Normal::new(0.0, 1.0).map_or(f64::NAN, |n| n.sf(k))
}

fn prepared(values: &[f64], k: f64, ok: impl Fn(f64) -> bool) -> Result<Vec<f64>, RareChartError> {
    if !(k > 0.0 && k.is_finite()) {
        return Err(RareChartError::BadSigma);
    }
    let v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if let Some(bad) = v.iter().find(|x| !ok(**x)) {
        return Err(RareChartError::OutOfRange(*bad));
    }
    if v.len() < 2 {
        return Err(RareChartError::TooFewValues);
    }
    Ok(v)
}

fn finish(values: Vec<f64>, params: Vec<f64>, center: f64, ucl: f64, lcl: f64) -> RareChart {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let above = values.iter().enumerate().filter(|(_, v)| **v > ucl).map(|(i, _)| i).collect();
    let below = values.iter().enumerate().filter(|(_, v)| **v < lcl).map(|(i, _)| i).collect();
    RareChart { values, mean, params, center, ucl, lcl, above, below }
}

/// The continuous geometric quantile at `u` for success probability `p` (support 0, 1, 2, ...).
pub fn geometric_quantile(p: f64, u: f64) -> f64 {
    ((-u).ln_1p() / (-p).ln_1p() - 1.0).max(0.0)
}

/// The G chart of the counts `values` with limits at `k` sigma (or the matching probabilities).
pub fn g_chart(values: &[f64], limits: GLimits, k: f64) -> Result<RareChart, RareChartError> {
    let v = prepared(values, k, |x| x >= 0.0)?;
    let g = v.iter().sum::<f64>() / v.len() as f64;
    if g <= 0.0 {
        return Err(RareChartError::AllZero);
    }
    let p = 1.0 / (g + 1.0);
    let (center, ucl, lcl) = match limits {
        GLimits::Sigma => {
            let s = (g * (g + 1.0)).sqrt();
            (g, g + k * s, (g - k * s).max(0.0))
        }
        GLimits::Probability => {
            let a = tail(k);
            (geometric_quantile(p, 0.5), geometric_quantile(p, 1.0 - a), geometric_quantile(p, a))
        }
    };
    Ok(finish(v, vec![p], center, ucl, lcl))
}

/// The T chart of the times `values` with probability limits matching `k` sigma.
pub fn t_chart(values: &[f64], dist: TDistribution, k: f64) -> Result<RareChart, RareChartError> {
    let v = prepared(values, k, |x| x > 0.0)?;
    let family = match dist {
        TDistribution::Weibull => Family::Weibull,
        TDistribution::Exponential => Family::Exponential,
    };
    let params = estimate(family, &v).map_err(|e| RareChartError::NoFit(e.to_string()))?;
    let a = tail(k);
    let q = |u: f64| quantile(family, &params, u);
    let (center, ucl, lcl) = (q(0.5), q(1.0 - a), q(a));
    Ok(finish(v, params, center, ucl, lcl))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g_chart_limits_and_flags() {
        let counts = [10.0, 25.0, 3.0, 40.0, 18.0, 0.0, 300.0, 12.0];
        let c = g_chart(&counts, GLimits::Sigma, 3.0).unwrap();
        let g = 408.0 / 8.0;
        assert!((c.center - g).abs() < 1e-12 && (c.params[0] - 1.0 / (g + 1.0)).abs() < 1e-15);
        assert!((c.ucl - (g + 3.0 * (g * (g + 1.0)).sqrt())).abs() < 1e-9);
        assert_eq!(c.lcl, 0.0);
        assert_eq!((c.above.clone(), c.below.len()), (vec![6], 0));
        let c = g_chart(&counts, GLimits::Probability, 3.0).unwrap();
        // The median of a geometric with mean g is about g ln 2.
        assert!((c.center - (0.5f64.ln() / (-c.params[0]).ln_1p() - 1.0)).abs() < 1e-12);
        assert!(c.lcl < 0.1 && c.ucl > 300.0);
        assert!((tail(3.0) - 0.001_349_898_031_630_094_6).abs() < 1e-13, "{}", tail(3.0));
    }

    #[test]
    fn t_chart_exponential_quantiles() {
        let t = [2.0, 4.0, 6.0, 8.0];
        let c = t_chart(&t, TDistribution::Exponential, 3.0).unwrap();
        assert!((c.params[0] - 5.0).abs() < 1e-12);
        assert!((c.center - 5.0 * 2f64.ln()).abs() < 1e-12);
        assert!((c.ucl + 5.0 * tail(3.0).ln()).abs() < 1e-9);
    }

    #[test]
    fn bad_input_is_refused() {
        assert_eq!(g_chart(&[1.0, -1.0], GLimits::Sigma, 3.0), Err(RareChartError::OutOfRange(-1.0)));
        assert_eq!(g_chart(&[0.0, 0.0], GLimits::Sigma, 3.0), Err(RareChartError::AllZero));
        assert_eq!(g_chart(&[3.0], GLimits::Sigma, 3.0), Err(RareChartError::TooFewValues));
        assert_eq!(t_chart(&[1.0, 0.0], TDistribution::Weibull, 3.0), Err(RareChartError::OutOfRange(0.0)));
        assert_eq!(t_chart(&[1.0, 2.0], TDistribution::Weibull, 0.0), Err(RareChartError::BadSigma));
    }
}
