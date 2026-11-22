//! Baseline subtraction: a constant, a straight line through two points, or a curve through
//! user-given anchor points (interpolated with the interpolation tools).
//!
//! These are our own tools (Origin offers them inside the Peak Analyzer and the Baseline tool);
//! the interpolation is the part that was checked against Origin.

use crate::interp::{Extrapolation, InterpMethod, InterpOptions, SplineBoundary, interp1};

/// How the baseline is defined.
#[derive(Debug, Clone, PartialEq)]
pub enum BaselineMethod {
    /// A constant level.
    Constant(f64),
    /// The straight line through two points `(x, y)`.
    Line((f64, f64), (f64, f64)),
    /// A curve through anchor points, interpolated with `method` (the line or curve continues
    /// beyond the first and last anchor).
    Anchors { points: Vec<(f64, f64)>, method: InterpMethod, boundary: SplineBoundary },
    /// Asymmetric Least Squares Smoothing ([`als_baseline`]).
    Als(AlsOptions),
}

/// The settings of Origin's Peak Analyzer baseline mode Asymmetric Least Squares Smoothing (the
/// names of its ALS page). Origin 2016 offers the mode in Origin Pro only, so the defaults are
/// Sheaf's choice from the typical values the page names, not measured.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AlsOptions {
    /// Asymmetric Factor, between 0 and 1: the weight of the points on the peak side of the
    /// baseline (close to 0 for positive peaks, close to 1 for negative ones).
    pub asymmetric: f64,
    /// Threshold, between 0 and 1: a point counts as being on the peak side when its distance from
    /// the baseline is more than this fraction of the largest such distance.
    pub threshold: f64,
    /// Smoothing Factor, above 0: the base-10 logarithm of the penalty on the baseline's second
    /// differences (larger is smoother).
    pub smoothing: f64,
    /// Number of Iterations, at least 1.
    pub iterations: usize,
}

impl Default for AlsOptions {
    fn default() -> Self {
        AlsOptions { asymmetric: 0.001, threshold: 0.05, smoothing: 5.0, iterations: 10 }
    }
}

impl AlsOptions {
    /// Why the settings cannot be used, if they cannot.
    pub fn check(&self) -> Result<(), String> {
        if !(self.asymmetric > 0.0 && self.asymmetric < 1.0) {
            return Err("the asymmetric factor must be between 0 and 1".into());
        }
        if !(0.0..1.0).contains(&self.threshold) {
            return Err("the ALS threshold must be at least 0 and below 1".into());
        }
        if !(self.smoothing > 0.0 && self.smoothing <= 15.0) {
            return Err("the ALS smoothing factor must be above 0 (at most 15)".into());
        }
        if self.iterations == 0 || self.iterations > 1000 {
            return Err("the number of ALS iterations must be 1 to 1000".into());
        }
        Ok(())
    }
}

/// The Asymmetric Least Squares baseline (Eilers and Boelens 2005, the algorithm Origin's Peak
/// Analyzer page gives): the curve `z` that minimises `sum w_i (y_i - z_i)^2 + lambda sum (second
/// difference of z)^2` with `lambda = 10^smoothing`. The first pass weighs every point 1; after each
/// pass the points on the peak side beyond the threshold get the small weight and the others the
/// large one (`asymmetric` and `1 - asymmetric`), and the baseline is solved again, `iterations`
/// passes in all. Points that are not numbers get weight 0. The X spacing is not used (as in the
/// method). Fewer than three numbers give the data back.
pub fn als_baseline(y: &[f64], o: &AlsOptions) -> Vec<f64> {
    let n = y.len();
    let finite = y.iter().filter(|v| v.is_finite()).count();
    if n < 3 || finite < 3 {
        return y.to_vec();
    }
    let lambda = 10f64.powf(o.smoothing);
    let yy: Vec<f64> = y.iter().map(|v| if v.is_finite() { *v } else { 0.0 }).collect();
    let mut w: Vec<f64> = y.iter().map(|v| if v.is_finite() { 1.0 } else { 0.0 }).collect();
    // lambda D'D, D the (n - 2) x n second-difference matrix: a symmetric band of width 2.
    let (mut d0, mut d1, mut d2) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let c = [1.0, -2.0, 1.0];
    for k in 0..n - 2 {
        for a in 0..3 {
            d0[k + a] += lambda * c[a] * c[a];
            if a + 1 < 3 {
                d1[k + a] += lambda * c[a] * c[a + 1];
            }
            if a + 2 < 3 {
                d2[k + a] += lambda * c[a] * c[a + 2];
            }
        }
    }
    let positive = o.asymmetric < 0.5;
    let (small, large) = if positive { (o.asymmetric, 1.0 - o.asymmetric) } else { (1.0 - o.asymmetric, o.asymmetric) };
    let mut z = yy.clone();
    for pass in 0..o.iterations.max(1) {
        z = solve_band(&d0, &d1, &d2, &w, &yy);
        if pass + 1 == o.iterations.max(1) {
            break;
        }
        // Distance on the peak side (above for positive peaks, below for negative ones).
        let side = |i: usize| if positive { yy[i] - z[i] } else { z[i] - yy[i] };
        let far = (0..n).filter(|&i| y[i].is_finite()).map(side).fold(0.0f64, f64::max);
        for i in 0..n {
            if y[i].is_finite() {
                w[i] = if side(i) > o.threshold * far && side(i) > 0.0 { small } else { large };
            }
        }
    }
    z
}

/// Solves `(diag(w) + B) z = w y` with `B` symmetric of band width 2 (`d0` the diagonal, `d1` and
/// `d2` the first and second upper diagonals) by a banded Cholesky factorisation.
fn solve_band(d0: &[f64], d1: &[f64], d2: &[f64], w: &[f64], y: &[f64]) -> Vec<f64> {
    let n = d0.len();
    // L has the diagonal l0 and the sub-diagonals l1 (row i, column i - 1) and l2 (column i - 2).
    let (mut l0, mut l1, mut l2) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    for i in 0..n {
        if i >= 2 {
            l2[i] = d2[i - 2] / l0[i - 2];
        }
        if i >= 1 {
            let mut v = d1[i - 1];
            if i >= 2 {
                v -= l2[i] * l1[i - 1];
            }
            l1[i] = v / l0[i - 1];
        }
        let diag = d0[i] + w[i] - l1[i] * l1[i] - l2[i] * l2[i];
        l0[i] = diag.max(f64::MIN_POSITIVE).sqrt();
    }
    let mut u = vec![0.0; n];
    for i in 0..n {
        let mut v = w[i] * y[i];
        if i >= 1 {
            v -= l1[i] * u[i - 1];
        }
        if i >= 2 {
            v -= l2[i] * u[i - 2];
        }
        u[i] = v / l0[i];
    }
    let mut z = vec![0.0; n];
    for i in (0..n).rev() {
        let mut v = u[i];
        if i + 1 < n {
            v -= l1[i + 1] * z[i + 1];
        }
        if i + 2 < n {
            v -= l2[i + 2] * z[i + 2];
        }
        z[i] = v / l0[i];
    }
    z
}

/// Subtracts a constant from every value.
pub fn subtract_constant(y: &[f64], level: f64) -> Vec<f64> {
    y.iter().map(|v| v - level).collect()
}

/// Subtracts the straight line through `p1` and `p2`.
pub fn subtract_line(x: &[f64], y: &[f64], p1: (f64, f64), p2: (f64, f64)) -> Vec<f64> {
    let (b, _) = baseline_values(x, y, &BaselineMethod::Line(p1, p2));
    y.iter().zip(&b).map(|(a, b)| a - b).collect()
}

fn baseline_values(x: &[f64], y: &[f64], method: &BaselineMethod) -> (Vec<f64>, ()) {
    let v = match method {
        BaselineMethod::Als(o) => als_baseline(&y[..x.len().min(y.len())], o),
        BaselineMethod::Constant(c) => vec![*c; x.len()],
        BaselineMethod::Line((x1, y1), (x2, y2)) => {
            let slope = if x2 == x1 { 0.0 } else { (y2 - y1) / (x2 - x1) };
            x.iter().map(|xv| y1 + slope * (xv - x1)).collect()
        }
        BaselineMethod::Anchors { points, method, boundary } => {
            let ax: Vec<f64> = points.iter().map(|p| p.0).collect();
            let ay: Vec<f64> = points.iter().map(|p| p.1).collect();
            let o = InterpOptions {
                method: *method,
                boundary: *boundary,
                extrapolation: Extrapolation::Extrapolate,
                ..Default::default()
            };
            interp1(&ax, &ay, x, &o)
        }
    };
    (v, ())
}

/// Returns `(corrected, baseline)`: `y - baseline(x)` and the baseline itself.
pub fn subtract_baseline(x: &[f64], y: &[f64], method: &BaselineMethod) -> (Vec<f64>, Vec<f64>) {
    let n = x.len().min(y.len());
    let (b, _) = baseline_values(&x[..n], &y[..n], method);
    let corrected = y[..n].iter().zip(&b).map(|(a, b)| a - b).collect();
    (corrected, b)
}
