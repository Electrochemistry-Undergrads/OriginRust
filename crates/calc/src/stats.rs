//! Descriptive statistics (Origin `stats` / `colstats`).

/// Result of [`stats`]. Quantities that need more points than available are NaN
/// (standard deviation, variance, standard error, coefficient of variation: 2 points; skewness:
/// 3; kurtosis: 4).
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    /// Number of valid points.
    pub n: usize,
    /// Number of missing (NaN) points.
    pub missing: usize,
    pub mean: f64,
    /// Sample standard deviation (divisor n - 1).
    pub sd: f64,
    /// Standard error of the mean.
    pub sem: f64,
    pub variance: f64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
    pub q1: f64,
    pub q3: f64,
    pub iqr: f64,
    /// Bias-corrected sample skewness.
    pub skewness: f64,
    /// Bias-corrected sample excess kurtosis.
    pub kurtosis: f64,
    /// Standard deviation divided by the mean.
    pub cv: f64,
}

/// Origin's quartile rule on an ascending sorted slice: the point at the (1-based) position
/// `n p + 1/2` (Hazen rule); where that position is not a multiple of one half it is rounded to
/// the nearest whole position. Positions that fall between two points average them.
/// `p` is 0.25, 0.5 or 0.75; for other `p` the same rule is applied.
pub fn quantile_origin(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    let nf = n as f64;
    let lower = |p: f64| {
        let pos = nf * p + 0.5;
        if (pos * 2.0).fract() != 0.0 { pos.round() } else { pos }
    };
    let pos = if p > 0.5 { nf + 1.0 - lower(1.0 - p) } else { lower(p) };
    let pos = pos.clamp(1.0, nf);
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    (sorted[lo - 1] + sorted[hi - 1]) / 2.0
}

/// The confidence limits of the mean at `level` (for example 0.95): `mean -/+ t * sem` with the quantile
/// of Student's t distribution with n - 1 degrees of freedom, the interval of the one-sample t-test.
/// NaN for fewer than two numbers or a level outside (0, 1).
pub fn mean_confidence_limits(s: &Stats, level: f64) -> (f64, f64) {
    use statrs::distribution::{ContinuousCDF, StudentsT};
    if s.n < 2 || !(level > 0.0 && level < 1.0) || !s.sem.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    let Ok(t) = StudentsT::new(0.0, 1.0, (s.n - 1) as f64) else {
        return (f64::NAN, f64::NAN);
    };
    let crit = t.inverse_cdf(1.0 - (1.0 - level) / 2.0);
    (s.mean - crit * s.sem, s.mean + crit * s.sem)
}

/// Statistics of one row across several columns (Origin `rowstats`). A row without a number gives NaN in
/// every field, the count and the sum included; one number has no standard deviation or standard error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStats {
    pub n: f64,
    pub mean: f64,
    pub sd: f64,
    pub sem: f64,
    pub min: f64,
    pub max: f64,
    pub sum: f64,
}

/// [`RowStats`] of `values`; NaN entries are ignored.
pub fn row_stats(values: &[f64]) -> RowStats {
    let v: Vec<f64> = values.iter().copied().filter(|a| !a.is_nan()).collect();
    let n = v.len();
    if n == 0 {
        return RowStats {
            n: f64::NAN,
            mean: f64::NAN,
            sd: f64::NAN,
            sem: f64::NAN,
            min: f64::NAN,
            max: f64::NAN,
            sum: f64::NAN,
        };
    }
    let nf = n as f64;
    let sum: f64 = v.iter().sum();
    let mean = sum / nf;
    let sd = if n > 1 { (v.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / (nf - 1.0)).sqrt() } else { f64::NAN };
    RowStats {
        n: nf,
        mean,
        sd,
        sem: sd / nf.sqrt(),
        min: v.iter().copied().fold(f64::INFINITY, f64::min),
        max: v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        sum,
    }
}

/// Statistics of a column; NaN entries are counted as missing and ignored.
pub fn stats(data: &[f64]) -> Stats {
    let mut v: Vec<f64> = data.iter().copied().filter(|a| !a.is_nan()).collect();
    let missing = data.len() - v.len();
    let n = v.len();
    let nf = n as f64;
    // Adding +0.0 turns the -0.0 that an empty f64 sum gives into 0.
    let sum: f64 = v.iter().sum::<f64>() + 0.0;
    let mean = if n > 0 { sum / nf } else { f64::NAN };
    let m2: f64 = v.iter().map(|a| (a - mean).powi(2)).sum();
    let variance = if n > 1 { m2 / (nf - 1.0) } else { f64::NAN };
    let sd = variance.sqrt();
    let sem = sd / nf.sqrt();
    let skewness = if n > 2 && sd > 0.0 {
        let s3: f64 = v.iter().map(|a| ((a - mean) / sd).powi(3)).sum();
        nf / ((nf - 1.0) * (nf - 2.0)) * s3
    } else {
        f64::NAN
    };
    let kurtosis = if n > 3 && sd > 0.0 {
        let s4: f64 = v.iter().map(|a| ((a - mean) / sd).powi(4)).sum();
        nf * (nf + 1.0) / ((nf - 1.0) * (nf - 2.0) * (nf - 3.0)) * s4
            - 3.0 * (nf - 1.0) * (nf - 1.0) / ((nf - 2.0) * (nf - 3.0))
    } else {
        f64::NAN
    };
    v.sort_by(|a, b| a.total_cmp(b));
    let (min, max) = if n > 0 { (v[0], v[n - 1]) } else { (f64::NAN, f64::NAN) };
    let median = quantile_origin(&v, 0.5);
    let q1 = quantile_origin(&v, 0.25);
    let q3 = quantile_origin(&v, 0.75);
    Stats {
        n,
        missing,
        mean,
        sd,
        sem,
        variance,
        sum,
        min,
        max,
        median,
        q1,
        q3,
        iqr: q3 - q1,
        skewness,
        kurtosis,
        cv: sd / mean,
    }
}
