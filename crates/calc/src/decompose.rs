//! Classical seasonal decomposition (Origin 2024b's Time Series Analysis app, "separate trend and seasonality"; not
//! in Origin 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 40).
//!
//! With period `m`: the trend is the centred moving average of `m` points (odd `m`) or of `m + 1` points with half
//! weights at both ends (even `m`), missing for the first and last `floor(m / 2)` points. The detrended series is
//! `y - trend` (additive) or `y / trend` (multiplicative); the seasonal index of each position in the period is the
//! mean of its detrended values, adjusted to sum to 0 (additive) or to average 1 (multiplicative). The residual is
//! `y - trend - seasonal` or `y / (trend seasonal)`. This is the textbook method (and statsmodels'
//! `seasonal_decompose` without trend extrapolation). Checked against numpy in
//! `crates/calc/tests/decompose_reference.rs` (made by `tools/decompose-ref.py`).

/// Additive or multiplicative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecomposeModel {
    #[default]
    Additive,
    Multiplicative,
}

/// The parts of the series, each as long as it; the trend and residual are NaN where the moving average is not
/// defined.
#[derive(Debug, Clone, PartialEq)]
pub struct Decomposition {
    pub trend: Vec<f64>,
    pub seasonal: Vec<f64>,
    pub residual: Vec<f64>,
    /// The seasonal index of each position `0..m` (position `k` is the `k`-th value of the series and every `m`-th
    /// after it).
    pub indices: Vec<f64>,
}

/// Why the series could not be decomposed.
#[derive(Debug, Clone, PartialEq)]
pub enum DecomposeError {
    BadPeriod(usize),
    TooShort { n: usize, period: usize },
    Missing(usize),
    NotPositive(usize),
}

impl std::fmt::Display for DecomposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecomposeError::BadPeriod(m) => write!(f, "the period must be at least 2 (got {m})"),
            DecomposeError::TooShort { n, period } => {
                write!(f, "{n} values are too few for period {period}: at least two full periods are needed")
            }
            DecomposeError::Missing(i) => write!(f, "value {} is missing: the series must be complete", i + 1),
            DecomposeError::NotPositive(i) => {
                write!(f, "value {} is not above 0, which the multiplicative model needs", i + 1)
            }
        }
    }
}

impl std::error::Error for DecomposeError {}

/// Decomposes `y` with period `m`.
pub fn decompose(y: &[f64], m: usize, model: DecomposeModel) -> Result<Decomposition, DecomposeError> {
    if m < 2 {
        return Err(DecomposeError::BadPeriod(m));
    }
    let n = y.len();
    if n < 2 * m {
        return Err(DecomposeError::TooShort { n, period: m });
    }
    if let Some(i) = y.iter().position(|v| !v.is_finite()) {
        return Err(DecomposeError::Missing(i));
    }
    if model == DecomposeModel::Multiplicative
        && let Some(i) = y.iter().position(|v| *v <= 0.0)
    {
        return Err(DecomposeError::NotPositive(i));
    }
    let half = m / 2;
    let weights: Vec<f64> = if m % 2 == 1 {
        vec![1.0 / m as f64; m]
    } else {
        let mut w = vec![1.0 / m as f64; m + 1];
        w[0] = 0.5 / m as f64;
        w[m] = 0.5 / m as f64;
        w
    };
    let mut trend = vec![f64::NAN; n];
    for (i, t) in trend.iter_mut().enumerate().take(n - half).skip(half) {
        *t = weights.iter().enumerate().map(|(k, w)| w * y[i + k - half]).sum();
    }
    let detrended: Vec<f64> = y
        .iter()
        .zip(&trend)
        .map(|(v, t)| match model {
            DecomposeModel::Additive => v - t,
            DecomposeModel::Multiplicative => v / t,
        })
        .collect();
    let mut indices: Vec<f64> = (0..m)
        .map(|k| {
            let vals: Vec<f64> = detrended.iter().skip(k).step_by(m).copied().filter(|v| v.is_finite()).collect();
            vals.iter().sum::<f64>() / vals.len() as f64
        })
        .collect();
    let mean = indices.iter().sum::<f64>() / m as f64;
    for v in &mut indices {
        match model {
            DecomposeModel::Additive => *v -= mean,
            DecomposeModel::Multiplicative => *v /= mean,
        }
    }
    let seasonal: Vec<f64> = (0..n).map(|i| indices[i % m]).collect();
    let residual: Vec<f64> = (0..n)
        .map(|i| match model {
            DecomposeModel::Additive => y[i] - trend[i] - seasonal[i],
            DecomposeModel::Multiplicative => y[i] / (trend[i] * seasonal[i]),
        })
        .collect();
    Ok(Decomposition { trend, seasonal, residual, indices })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_plus_a_pattern_comes_apart_exactly() {
        let pattern = [1.0, -2.0, 0.5, 0.5];
        let y: Vec<f64> = (0..16).map(|i| 3.0 + 0.5 * f64::from(i) + pattern[i as usize % 4]).collect();
        let d = decompose(&y, 4, DecomposeModel::Additive).unwrap();
        assert!(d.trend[0].is_nan() && d.trend[1].is_nan() && d.trend[14].is_nan() && d.trend[15].is_nan());
        for i in 2..14 {
            assert!((d.trend[i] - (3.0 + 0.5 * i as f64)).abs() < 1e-12, "{i}: {}", d.trend[i]);
            assert!(d.residual[i].abs() < 1e-12);
        }
        for (a, b) in d.indices.iter().zip(pattern) {
            assert!((a - b).abs() < 1e-12);
        }
        let y: Vec<f64> = (0..15).map(|i| (10.0 + f64::from(i)) * [1.2, 0.8, 1.0][i as usize % 3]).collect();
        let d = decompose(&y, 3, DecomposeModel::Multiplicative).unwrap();
        assert!((d.indices.iter().sum::<f64>() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn bad_input_is_refused() {
        assert_eq!(decompose(&[1.0; 8], 1, DecomposeModel::Additive), Err(DecomposeError::BadPeriod(1)));
        assert_eq!(
            decompose(&[1.0; 7], 4, DecomposeModel::Additive),
            Err(DecomposeError::TooShort { n: 7, period: 4 })
        );
        let mut y = vec![1.0; 8];
        y[3] = f64::NAN;
        assert_eq!(decompose(&y, 2, DecomposeModel::Additive), Err(DecomposeError::Missing(3)));
        y[3] = 0.0;
        assert_eq!(decompose(&y, 2, DecomposeModel::Multiplicative), Err(DecomposeError::NotPositive(3)));
    }
}
