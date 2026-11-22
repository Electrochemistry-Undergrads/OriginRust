//! Stability study (shelf life) after ICH Q1E, for Origin 2027's Quality improvement app "model between-batch
//! variation in stability studies" (not in Origin 2016, so nothing here is measured in Origin;
//! `docs/spec/statistics.md` section 38).
//!
//! The response of every batch is a straight line in time. Three analysis-of-covariance models are fitted by least
//! squares: separate intercepts and slopes, separate intercepts with a common slope, and one line. Poolability is
//! tested at `alpha` (0.25 in ICH Q1E): equal slopes by `F = ((SSE_2 - SSE_1) / (g - 1)) / (SSE_1 / (n - 2g))`; if
//! they are not rejected, equal intercepts by `F = ((SSE_3 - SSE_2) / (g - 1)) / (SSE_2 / (n - g - 1))`. The chosen
//! model gives each batch's mean line and its confidence bound `yhat -+ t sqrt(MSE x0' (X'X)^-1 x0)` on the
//! model's residual degrees of freedom: one-sided at the confidence level for one specification limit, two-sided
//! for two. A batch's shelf life is the first time from 0 where a bound reaches a limit; the study's is the smallest.
//! Checked against numpy/scipy in `crates/calc/tests/stability_reference.rs` (made by `tools/stability-ref.py`).

use statrs::distribution::{ContinuousCDF, FisherSnedecor, StudentsT};

/// Which analysis-of-covariance model the poolability tests chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityModel {
    /// Every batch its own intercept and slope.
    Separate,
    /// Separate intercepts, a common slope.
    CommonSlope,
    /// One line for all batches.
    Pooled,
}

impl StabilityModel {
    pub fn name(self) -> &'static str {
        match self {
            StabilityModel::Separate => "separate intercepts and slopes",
            StabilityModel::CommonSlope => "separate intercepts, common slope",
            StabilityModel::Pooled => "one line for all batches",
        }
    }
}

/// The options of the study.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabilityOptions {
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    /// Confidence level of the bounds (0.95).
    pub confidence: f64,
    /// Significance level of the poolability tests (0.25).
    pub alpha: f64,
}

impl Default for StabilityOptions {
    fn default() -> Self {
        StabilityOptions { lower: None, upper: None, confidence: 0.95, alpha: 0.25 }
    }
}

/// The fitted study.
#[derive(Debug, Clone, PartialEq)]
pub struct Stability {
    pub model: StabilityModel,
    /// p-values of equal slopes and of equal intercepts (NaN when not tested).
    pub p_slopes: f64,
    pub p_intercepts: f64,
    /// Per batch: number of points, intercept and slope of its line under the chosen model, shelf life.
    pub batches: Vec<BatchLine>,
    /// The smallest shelf life (infinite when no bound reaches a limit before `HORIZON` times the last time).
    pub shelf_life: f64,
    /// Residual mean square and degrees of freedom of the chosen model, and the t quantile of the bounds.
    pub mse: f64,
    pub df: usize,
    pub t: f64,
    /// `(X'X)^-1` of the chosen model and its parameter vector (`theta`), in the model's own layout.
    inverse: Vec<Vec<f64>>,
    theta: Vec<f64>,
    groups: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BatchLine {
    pub n: usize,
    pub intercept: f64,
    pub slope: f64,
    pub shelf_life: f64,
}

/// The search for a shelf life stops at this many times the last time point.
pub const HORIZON: f64 = 10.0;

/// Why the study could not be fitted.
#[derive(Debug, Clone, PartialEq)]
pub enum StabilityError {
    NoLimits,
    LimitsReversed,
    BadLevel,
    /// A batch with fewer than two different times.
    BatchTooSmall(usize),
    NoResidualDf,
    Singular,
    /// The bound already lies beyond a limit at time 0.
    OutAtStart,
}

impl std::fmt::Display for StabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StabilityError::NoLimits => f.write_str("give a lower or an upper specification limit"),
            StabilityError::LimitsReversed => f.write_str("the lower specification limit must be below the upper"),
            StabilityError::BadLevel => f.write_str("the confidence and significance levels must lie between 0 and 1"),
            StabilityError::BatchTooSmall(b) => write!(f, "batch {} needs at least two different times", b + 1),
            StabilityError::NoResidualDf => f.write_str("too few points for the separate lines model"),
            StabilityError::Singular => f.write_str("the times give no straight line"),
            StabilityError::OutAtStart => f.write_str("the confidence bound is already beyond a limit at time 0"),
        }
    }
}

impl std::error::Error for StabilityError {}

/// Inverts a symmetric positive definite matrix by Gauss-Jordan elimination with partial pivoting.
pub(crate) fn invert(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = a
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut row = r.clone();
            row.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
            row
        })
        .collect();
    let scale = a.iter().flatten().fold(0.0f64, |s, v| s.max(v.abs())).max(1e-300);
    for c in 0..n {
        let p = (c..n).max_by(|&i, &j| m[i][c].abs().total_cmp(&m[j][c].abs()))?;
        if m[p][c].abs() <= 1e-12 * scale {
            return None;
        }
        m.swap(c, p);
        let d = m[c][c];
        for v in &mut m[c] {
            *v /= d;
        }
        let pivot = m[c].clone();
        for (r, rowr) in m.iter_mut().enumerate() {
            let f = rowr[c];
            if r != c && f != 0.0 {
                for (v, p) in rowr.iter_mut().zip(&pivot) {
                    *v -= f * p;
                }
            }
        }
    }
    Some(m.into_iter().map(|r| r[n..].to_vec()).collect())
}

/// The design row of a point of batch `b` at time `t` in a model with `g` batches.
fn row(model: StabilityModel, g: usize, b: usize, t: f64) -> Vec<f64> {
    match model {
        StabilityModel::Separate => {
            let mut r = vec![0.0; 2 * g];
            r[b] = 1.0;
            r[g + b] = t;
            r
        }
        StabilityModel::CommonSlope => {
            let mut r = vec![0.0; g + 1];
            r[b] = 1.0;
            r[g] = t;
            r
        }
        StabilityModel::Pooled => vec![1.0, t],
    }
}

struct Fitted {
    theta: Vec<f64>,
    inverse: Vec<Vec<f64>>,
    sse: f64,
    p: usize,
}

fn fit(model: StabilityModel, g: usize, data: &[(usize, f64, f64)]) -> Result<Fitted, StabilityError> {
    let p = row(model, g, 0, 0.0).len();
    let mut xtx = vec![vec![0.0; p]; p];
    let mut xty = vec![0.0; p];
    for &(b, t, y) in data {
        let r = row(model, g, b, t);
        for i in 0..p {
            xty[i] += r[i] * y;
            for j in 0..p {
                xtx[i][j] += r[i] * r[j];
            }
        }
    }
    let inverse = invert(&xtx).ok_or(StabilityError::Singular)?;
    let theta: Vec<f64> = (0..p).map(|i| (0..p).map(|j| inverse[i][j] * xty[j]).sum()).collect();
    let sse = data
        .iter()
        .map(|&(b, t, y)| {
            let r = row(model, g, b, t);
            let f: f64 = r.iter().zip(&theta).map(|(a, c)| a * c).sum();
            (y - f).powi(2)
        })
        .sum();
    Ok(Fitted { theta, inverse, sse, p })
}

fn f_sf(f: f64, d1: f64, d2: f64) -> f64 {
    FisherSnedecor::new(d1, d2).map_or(f64::NAN, |d| d.sf(f.max(0.0)))
}

impl Stability {
    /// The fitted mean of batch `b` at time `t`.
    pub fn mean(&self, b: usize, t: f64) -> f64 {
        row(self.model, self.groups, b, t).iter().zip(&self.theta).map(|(a, c)| a * c).sum()
    }

    /// The half width of the confidence bound of batch `b`'s mean at time `t`.
    pub fn half_width(&self, b: usize, t: f64) -> f64 {
        let r = row(self.model, self.groups, b, t);
        let q: f64 = (0..r.len()).map(|i| (0..r.len()).map(|j| r[i] * self.inverse[i][j] * r[j]).sum::<f64>()).sum();
        self.t * (self.mse * q).sqrt()
    }
}

/// Fits the study to `(batch, time, response)` points, batches numbered from 0.
pub fn stability(data: &[(usize, f64, f64)], opts: StabilityOptions) -> Result<Stability, StabilityError> {
    if opts.lower.is_none() && opts.upper.is_none() {
        return Err(StabilityError::NoLimits);
    }
    if let (Some(l), Some(u)) = (opts.lower, opts.upper)
        && l >= u
    {
        return Err(StabilityError::LimitsReversed);
    }
    let level_ok = |v: f64| v > 0.0 && v < 1.0;
    if !level_ok(opts.confidence) || !level_ok(opts.alpha) {
        return Err(StabilityError::BadLevel);
    }
    let data: Vec<(usize, f64, f64)> =
        data.iter().copied().filter(|(_, t, y)| t.is_finite() && y.is_finite()).collect();
    let g = data.iter().map(|d| d.0 + 1).max().unwrap_or(0);
    for b in 0..g {
        let ts: Vec<f64> = data.iter().filter(|d| d.0 == b).map(|d| d.1).collect();
        if ts.iter().all(|t| Some(t) == ts.first()) {
            return Err(StabilityError::BatchTooSmall(b));
        }
    }
    let n = data.len();
    if n <= 2 * g {
        return Err(StabilityError::NoResidualDf);
    }
    let gf = g as f64;
    let (model, p_slopes, p_intercepts, chosen) = if g == 1 {
        (StabilityModel::Pooled, f64::NAN, f64::NAN, fit(StabilityModel::Pooled, 1, &data)?)
    } else {
        let m1 = fit(StabilityModel::Separate, g, &data)?;
        let m2 = fit(StabilityModel::CommonSlope, g, &data)?;
        let df1 = (n - 2 * g) as f64;
        let p_slopes = f_sf(((m2.sse - m1.sse) / (gf - 1.0)) / (m1.sse / df1), gf - 1.0, df1);
        if p_slopes < opts.alpha {
            (StabilityModel::Separate, p_slopes, f64::NAN, m1)
        } else {
            let m3 = fit(StabilityModel::Pooled, g, &data)?;
            let df2 = (n - g - 1) as f64;
            let p_int = f_sf(((m3.sse - m2.sse) / (gf - 1.0)) / (m2.sse / df2), gf - 1.0, df2);
            if p_int < opts.alpha {
                (StabilityModel::CommonSlope, p_slopes, p_int, m2)
            } else {
                (StabilityModel::Pooled, p_slopes, p_int, m3)
            }
        }
    };
    let df = n - chosen.p;
    let two_sided = opts.lower.is_some() && opts.upper.is_some();
    let q = if two_sided { 0.5 + opts.confidence / 2.0 } else { opts.confidence };
    let t = StudentsT::new(0.0, 1.0, df as f64).map_or(f64::NAN, |d| d.inverse_cdf(q));
    let mut s = Stability {
        model,
        p_slopes,
        p_intercepts,
        batches: Vec::new(),
        shelf_life: f64::INFINITY,
        mse: chosen.sse / df as f64,
        df,
        t,
        inverse: chosen.inverse,
        theta: chosen.theta,
        groups: g,
    };
    let last = data.iter().map(|d| d.1).fold(0.0, f64::max);
    for b in 0..g {
        // The margin to the nearer limit: positive while both bounds are inside.
        let margin = |t: f64| {
            let (m, h) = (s.mean(b, t), s.half_width(b, t));
            let lo = opts.lower.map_or(f64::INFINITY, |l| m - h - l);
            let hi = opts.upper.map_or(f64::INFINITY, |u| u - (m + h));
            lo.min(hi)
        };
        if margin(0.0) <= 0.0 {
            return Err(StabilityError::OutAtStart);
        }
        let end = HORIZON * last.max(1.0);
        let steps = 2000;
        let mut life = f64::INFINITY;
        let mut a = 0.0;
        for k in 1..=steps {
            let bnd = end * f64::from(k) / f64::from(steps);
            if margin(bnd) <= 0.0 {
                let mut b2 = bnd;
                for _ in 0..200 {
                    let mid = 0.5 * (a + b2);
                    if mid <= a || mid >= b2 {
                        break;
                    }
                    if margin(mid) > 0.0 {
                        a = mid;
                    } else {
                        b2 = mid;
                    }
                }
                life = 0.5 * (a + b2);
                break;
            }
            a = bnd;
        }
        s.batches.push(BatchLine {
            n: data.iter().filter(|d| d.0 == b).count(),
            intercept: s.mean(b, 0.0),
            slope: s.mean(b, 1.0) - s.mean(b, 0.0),
            shelf_life: life,
        });
    }
    s.shelf_life = s.batches.iter().map(|b| b.shelf_life).fold(f64::INFINITY, f64::min);
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_batch_line_and_its_shelf_life() {
        // y = 100 - t exactly plus a small zigzag.
        let data: Vec<(usize, f64, f64)> = (0..7)
            .map(|i| (0, f64::from(i * 3), 100.0 - f64::from(i * 3) + if i % 2 == 0 { 0.2 } else { -0.2 }))
            .collect();
        let s = stability(&data, StabilityOptions { lower: Some(90.0), ..Default::default() }).unwrap();
        assert_eq!(s.model, StabilityModel::Pooled);
        assert!((s.batches[0].slope + 1.0).abs() < 0.05, "{:?}", s.batches[0]);
        // The lower bound meets 90 a little before the line does (t = 10).
        assert!(s.shelf_life < 10.0 && s.shelf_life > 9.0, "{}", s.shelf_life);
        let t = s.shelf_life;
        assert!((s.mean(0, t) - s.half_width(0, t) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn bad_input_is_refused() {
        let d = [(0, 0.0, 1.0), (0, 1.0, 2.0), (0, 2.0, 2.5)];
        let o = StabilityOptions::default();
        assert_eq!(stability(&d, o), Err(StabilityError::NoLimits));
        let o = StabilityOptions { lower: Some(0.0), confidence: 1.0, ..Default::default() };
        assert_eq!(stability(&d, o), Err(StabilityError::BadLevel));
        let o = StabilityOptions { lower: Some(0.0), ..Default::default() };
        assert_eq!(stability(&[(0, 1.0, 1.0), (0, 1.0, 2.0), (0, 1.0, 3.0)], o), Err(StabilityError::BatchTooSmall(0)));
        assert_eq!(stability(&d[..2], o), Err(StabilityError::NoResidualDf));
        let o = StabilityOptions { lower: Some(5.0), ..Default::default() };
        assert_eq!(stability(&d, o), Err(StabilityError::OutAtStart));
    }
}
