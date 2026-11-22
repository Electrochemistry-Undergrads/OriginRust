//! Change-point analysis (the change-point half of Origin's 2019b statistics apps; not in Origin 2016, so
//! nothing here is measured in Origin). The series is split into segments by minimizing the sum of the
//! segment costs plus a penalty per change, solved exactly with PELT (Killick, Fearnhead and Eckley 2012);
//! the costs and penalties are those of R's `changepoint` package (twice the negative normal
//! log-likelihood; MBIC's penalty, SIC/BIC, AIC and Hannan-Quinn), see `docs/spec/statistics.md` section 30.
//!
//! - Mean: a change in mean with a common variance, estimated robustly from the first differences
//!   (`sigma = MAD(diff) / (0.6745 sqrt 2)`, or the ordinary standard deviation of the differences over
//!   sqrt 2 when the MAD is 0); cost `sum (x - mean)^2 / sigma^2`.
//! - Variance: a change in variance about the mean of the whole series; cost `n (ln(2 pi) + ln s2 + 1)`,
//!   `s2 = sum (x - mean_all)^2 / n` of the segment.
//! - Mean and variance: both change; the same cost with `s2` about the segment's own mean.
//!
//! The answer equals that of exact optimal partitioning (checked against an O(n^2) dynamic programme in
//! `crates/calc/tests/changepoint_exact.rs`, made by `tools/changepoint-ref.py`).

/// What may change between segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChangeKind {
    #[default]
    Mean,
    Variance,
    MeanVariance,
}

impl ChangeKind {
    /// Parameters that change at a change point (`changepoint`'s `diffparam`).
    pub fn parameters(self) -> f64 {
        match self {
            ChangeKind::Mean | ChangeKind::Variance => 1.0,
            ChangeKind::MeanVariance => 2.0,
        }
    }
}

/// The penalty per change.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Penalty {
    /// The penalty of `changepoint`'s default MBIC, `(p + 2) ln n` (its extra cost term for short segments
    /// is not added).
    #[default]
    Mbic,
    /// Schwarz (SIC/BIC): `p ln n`.
    Bic,
    /// Akaike: `2 p`.
    Aic,
    /// Hannan-Quinn: `2 p ln ln n`.
    HannanQuinn,
    /// A value of one's own (at least 0).
    Manual(f64),
}

impl Penalty {
    /// The penalty value for `n` points.
    pub fn value(self, kind: ChangeKind, n: usize) -> f64 {
        let p = kind.parameters();
        let n = n as f64;
        match self {
            Penalty::Mbic => (p + 2.0) * n.ln(),
            Penalty::Bic => p * n.ln(),
            Penalty::Aic => 2.0 * p,
            Penalty::HannanQuinn => 2.0 * p * n.ln().ln(),
            Penalty::Manual(v) => v,
        }
    }
}

/// Why change points could not be found.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangePointError {
    TooFewPoints {
        have: usize,
        need: usize,
    },
    BadPenalty(f64),
    BadMinSegment,
    /// The mean model needs some variation to scale the cost.
    ConstantSeries,
}

impl std::fmt::Display for ChangePointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangePointError::TooFewPoints { have, need } => {
                write!(f, "not enough points: {have} usable points, at least {need} needed")
            }
            ChangePointError::BadPenalty(v) => write!(f, "the penalty must be a number of at least 0 (got {v})"),
            ChangePointError::BadMinSegment => {
                f.write_str("the minimum segment length must be at least 1 (2 when the variance changes)")
            }
            ChangePointError::ConstantSeries => f.write_str("the series does not vary, so there is nothing to split"),
        }
    }
}

impl std::error::Error for ChangePointError {}

/// One segment of the answer, as positions in the series given (missing values left out).
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    /// First and last position, 0-based inclusive.
    pub start: usize,
    pub end: usize,
    pub mean: f64,
    /// Standard deviation of the segment (n - 1 in the denominator; NaN for one point).
    pub sd: f64,
}

/// The result of a change-point analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangePoints {
    /// Positions where a new segment starts (0-based; the first segment's start, 0, is not listed).
    pub changes: Vec<usize>,
    pub segments: Vec<Segment>,
    /// The penalty per change used.
    pub penalty: f64,
    /// Sum of the segment costs plus the penalties.
    pub objective: f64,
    /// The common standard deviation of the mean model (NaN for the others).
    pub sigma: f64,
}

/// Prefix sums for O(1) segment costs.
struct Sums {
    s: Vec<f64>,
    s2: Vec<f64>,
    /// Sums of squares about the whole series' mean (variance model).
    c2: Vec<f64>,
}

impl Sums {
    fn new(x: &[f64]) -> Sums {
        let mean = x.iter().sum::<f64>() / x.len() as f64;
        let mut s = vec![0.0; x.len() + 1];
        let mut s2 = vec![0.0; x.len() + 1];
        let mut c2 = vec![0.0; x.len() + 1];
        for (i, v) in x.iter().enumerate() {
            s[i + 1] = s[i] + v;
            s2[i + 1] = s2[i] + v * v;
            c2[i + 1] = c2[i] + (v - mean) * (v - mean);
        }
        Sums { s, s2, c2 }
    }

    /// Sum of squares about the segment's own mean, of positions `a..b` (b exclusive).
    fn ss(&self, a: usize, b: usize) -> f64 {
        let n = (b - a) as f64;
        let s = self.s[b] - self.s[a];
        (self.s2[b] - self.s2[a] - s * s / n).max(0.0)
    }
}

/// The robust common standard deviation of the mean model.
pub fn mean_sigma(x: &[f64]) -> f64 {
    let d: Vec<f64> = x.windows(2).map(|w| w[1] - w[0]).collect();
    if d.is_empty() {
        return f64::NAN;
    }
    let median = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        let m = v.len();
        if m % 2 == 1 { v[m / 2] } else { 0.5 * (v[m / 2 - 1] + v[m / 2]) }
    };
    let mut dd = d.clone();
    let md = median(&mut dd);
    let mut dev: Vec<f64> = d.iter().map(|v| (v - md).abs()).collect();
    let mad = median(&mut dev) / 0.6745;
    if mad > 0.0 {
        return mad / std::f64::consts::SQRT_2;
    }
    let m = d.iter().sum::<f64>() / d.len() as f64;
    let var = d.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / (d.len().max(2) - 1) as f64;
    var.sqrt() / std::f64::consts::SQRT_2
}

/// Finds the change points of `x` (missing values must be removed first). `min_segment` is the smallest
/// number of points in a segment (at least 1, at least 2 when the variance changes).
pub fn change_points(
    x: &[f64],
    kind: ChangeKind,
    penalty: Penalty,
    min_segment: usize,
) -> Result<ChangePoints, ChangePointError> {
    let need_min = if kind == ChangeKind::Mean { 1 } else { 2 };
    if min_segment < need_min {
        return Err(ChangePointError::BadMinSegment);
    }
    let n = x.len();
    if n < 2 * min_segment.max(1) {
        return Err(ChangePointError::TooFewPoints { have: n, need: 2 * min_segment.max(1) });
    }
    let beta = penalty.value(kind, n);
    if !(beta.is_finite() && beta >= 0.0) {
        return Err(ChangePointError::BadPenalty(beta));
    }
    let owned = Sums::new(x);
    let sums = &owned;
    let mut sigma = f64::NAN;
    let total_var = sums.c2[n] / n as f64;
    if total_var <= 0.0 {
        return Err(ChangePointError::ConstantSeries);
    }
    // A segment variance is kept above this floor, so a flat stretch cannot drive the cost to minus infinity.
    let floor = total_var * 1e-12;
    let cost: Box<dyn Fn(usize, usize) -> f64 + '_> = match kind {
        ChangeKind::Mean => {
            sigma = mean_sigma(x);
            if sigma.is_nan() || sigma <= 0.0 {
                return Err(ChangePointError::ConstantSeries);
            }
            let s2 = sigma * sigma;
            Box::new(move |a, b| sums.ss(a, b) / s2)
        }
        ChangeKind::Variance => Box::new(move |a, b| {
            let m = (b - a) as f64;
            let v = ((sums.c2[b] - sums.c2[a]) / m).max(floor);
            m * ((2.0 * std::f64::consts::PI).ln() + v.ln() + 1.0)
        }),
        ChangeKind::MeanVariance => Box::new(move |a, b| {
            let m = (b - a) as f64;
            let v = (sums.ss(a, b) / m).max(floor);
            m * ((2.0 * std::f64::consts::PI).ln() + v.ln() + 1.0)
        }),
    };
    // PELT: f[t] is the best objective of positions 0..t, last[t] where its last segment starts.
    let m = min_segment;
    let mut f = vec![f64::INFINITY; n + 1];
    let mut last = vec![0usize; n + 1];
    f[0] = -beta;
    // A start s is pruned at the first time t where F(s) + C(s, t) > F(t); it is dropped once t is a usable
    // start for every later end (from t + m on), which keeps the answer exact with a minimum length.
    let mut candidates: Vec<(usize, Option<usize>)> = vec![(0, None)];
    for t in m..=n {
        candidates.retain(|(_, pruned)| pruned.is_none_or(|p| t < p + m));
        let mut best = f64::INFINITY;
        let mut arg = 0;
        for &(s, _) in &candidates {
            if t - s < m || !f[s].is_finite() {
                continue;
            }
            let v = f[s] + cost(s, t) + beta;
            if v < best {
                best = v;
                arg = s;
            }
        }
        f[t] = best;
        last[t] = arg;
        for (s, pruned) in &mut candidates {
            if pruned.is_none() && t - *s >= m && f[*s].is_finite() && f[*s] + cost(*s, t) > best {
                *pruned = Some(t);
            }
        }
        candidates.push((t, None));
    }
    let mut changes = Vec::new();
    let mut t = n;
    while t > 0 {
        let s = last[t];
        if s > 0 {
            changes.push(s);
        }
        t = s;
    }
    changes.reverse();
    let mut bounds = vec![0];
    bounds.extend(changes.iter().copied());
    bounds.push(n);
    let segments = bounds
        .windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let k = (b - a) as f64;
            let mean = (sums.s[b] - sums.s[a]) / k;
            let sd = if b - a > 1 { (sums.ss(a, b) / (k - 1.0)).sqrt() } else { f64::NAN };
            Segment { start: a, end: b - 1, mean, sd }
        })
        .collect();
    Ok(ChangePoints { changes, segments, penalty: beta, objective: f[n], sigma })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Roughly normal noise of unit variance from a fixed linear congruential generator.
    fn noise(n: usize, seed: u64) -> Vec<f64> {
        let mut state = seed;
        let mut uniform = move || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..n).map(|_| (0..12).map(|_| uniform()).sum::<f64>() - 6.0).collect()
    }

    fn steps() -> Vec<f64> {
        noise(90, 7)
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let level = if i < 30 {
                    0.0
                } else if i < 60 {
                    5.0
                } else {
                    2.0
                };
                level + 0.3 * e
            })
            .collect()
    }

    #[test]
    fn two_mean_shifts_are_found_where_they_are() {
        let r = change_points(&steps(), ChangeKind::Mean, Penalty::Mbic, 1).unwrap();
        assert_eq!(r.changes, vec![30, 60]);
        assert_eq!(r.segments.len(), 3);
        assert!((r.segments[1].mean - 5.0).abs() < 0.1, "{:?}", r.segments[1]);
        let r = change_points(&steps(), ChangeKind::MeanVariance, Penalty::Mbic, 2).unwrap();
        assert_eq!(r.changes, vec![30, 60]);
    }

    #[test]
    fn a_variance_change_is_found_and_a_steady_series_has_none() {
        let x: Vec<f64> = noise(100, 3).iter().enumerate().map(|(i, e)| e * if i < 50 { 0.2 } else { 2.0 }).collect();
        let r = change_points(&x, ChangeKind::Variance, Penalty::Mbic, 2).unwrap();
        assert_eq!(r.changes.len(), 1);
        assert!((48..=52).contains(&r.changes[0]), "{:?}", r.changes);
        let calm = noise(100, 11);
        assert!(change_points(&calm, ChangeKind::Mean, Penalty::Mbic, 1).unwrap().changes.is_empty());
    }

    #[test]
    fn bad_input_is_refused() {
        assert_eq!(
            change_points(&[1.0], ChangeKind::Mean, Penalty::Bic, 1).unwrap_err(),
            ChangePointError::TooFewPoints { have: 1, need: 2 }
        );
        assert_eq!(
            change_points(&steps(), ChangeKind::Variance, Penalty::Bic, 1).unwrap_err(),
            ChangePointError::BadMinSegment
        );
        assert_eq!(
            change_points(&steps(), ChangeKind::Mean, Penalty::Manual(-1.0), 1).unwrap_err(),
            ChangePointError::BadPenalty(-1.0)
        );
        assert_eq!(
            change_points(&[2.0; 10], ChangeKind::Mean, Penalty::Bic, 1).unwrap_err(),
            ChangePointError::ConstantSeries
        );
    }
}
