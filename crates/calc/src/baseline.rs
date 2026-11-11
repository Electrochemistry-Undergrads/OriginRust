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
}

/// Subtracts a constant from every value.
pub fn subtract_constant(y: &[f64], level: f64) -> Vec<f64> {
    y.iter().map(|v| v - level).collect()
}

/// Subtracts the straight line through `p1` and `p2`.
pub fn subtract_line(x: &[f64], y: &[f64], p1: (f64, f64), p2: (f64, f64)) -> Vec<f64> {
    let (b, _) = baseline_values(x, &BaselineMethod::Line(p1, p2));
    y.iter().zip(&b).map(|(a, b)| a - b).collect()
}

fn baseline_values(x: &[f64], method: &BaselineMethod) -> (Vec<f64>, ()) {
    let v = match method {
        BaselineMethod::Constant(c) => vec![*c; x.len()],
        BaselineMethod::Line((x1, y1), (x2, y2)) => {
            let slope = if x2 == x1 { 0.0 } else { (y2 - y1) / (x2 - x1) };
            x.iter().map(|xv| y1 + slope * (xv - x1)).collect()
        }
        BaselineMethod::Anchors { points, method, boundary } => {
            let ax: Vec<f64> = points.iter().map(|p| p.0).collect();
            let ay: Vec<f64> = points.iter().map(|p| p.1).collect();
            let o = InterpOptions { method: *method, boundary: *boundary, extrapolation: Extrapolation::Extrapolate };
            interp1(&ax, &ay, x, &o)
        }
    };
    (v, ())
}

/// Returns `(corrected, baseline)`: `y - baseline(x)` and the baseline itself.
pub fn subtract_baseline(x: &[f64], y: &[f64], method: &BaselineMethod) -> (Vec<f64>, Vec<f64>) {
    let n = x.len().min(y.len());
    let (b, _) = baseline_values(&x[..n], method);
    let corrected = y[..n].iter().zip(&b).map(|(a, b)| a - b).collect();
    (corrected, b)
}
