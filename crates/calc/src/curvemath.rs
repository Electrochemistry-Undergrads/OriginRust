//! Curve arithmetic (Origin `mathtool` and `subtract_ref`): a data curve combined with a constant
//! or with a reference curve.
//!
//! The reference is interpolated onto the X values of the data with straight lines between its
//! points; outside its X range the first or last segment is continued as a straight line. With
//! "common range only" the result is restricted to the X range both curves cover, and both curves
//! are evaluated on the merged X values of the two. See `docs/spec/analysis-tools.md`.

use crate::interp::{Extrapolation, InterpMethod, InterpOptions, SplineBoundary, interp1};

/// The arithmetic operation, with the data curve on the left: `y op operand`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveOp {
    Add,
    Subtract,
    Multiply,
    /// Division by zero gives a missing value, not infinity.
    Divide,
    /// `y` raised to the operand; a result that is not a finite number is a missing value.
    Power,
}

impl CurveOp {
    /// Applies the operation. Results that are not finite (division by zero, negative base with a
    /// fractional exponent, overflow) are NaN, which is a missing value in a worksheet.
    pub fn apply(self, y: f64, operand: f64) -> f64 {
        let v = match self {
            CurveOp::Add => y + operand,
            CurveOp::Subtract => y - operand,
            CurveOp::Multiply => y * operand,
            CurveOp::Divide if operand == 0.0 => f64::NAN,
            CurveOp::Divide => y / operand,
            CurveOp::Power => y.powf(operand),
        };
        if v.is_finite() { v } else { f64::NAN }
    }
}

/// Result of [`curve_math_ref`]: the X values and the combined Y values, one per point.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveMathResult {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// Applies `op` with a constant operand to every y value.
pub fn curve_math_const(y: &[f64], op: CurveOp, operand: f64) -> Vec<f64> {
    y.iter().map(|&v| op.apply(v, operand)).collect()
}

/// Origin's Simple Column Math (`vmathtool`) with a column as the operand: row `i` of `y` combined
/// with row `i` of `operand`, without X values or interpolation. Rows past the end of `operand`
/// use 0 (as Origin: `6 - nothing` is 6, `6 ^ nothing` is 1, `6 / nothing` is missing); a missing
/// value in either column inside its length gives a missing value. The result has `y.len()` rows.
pub fn column_math(y: &[f64], op: CurveOp, operand: &[f64]) -> Vec<f64> {
    y.iter()
        .enumerate()
        .map(|(i, &v)| {
            let r = operand.get(i).copied().unwrap_or(0.0);
            if v.is_nan() || r.is_nan() { f64::NAN } else { op.apply(v, r) }
        })
        .collect()
}

fn linear() -> InterpOptions {
    InterpOptions {
        method: InterpMethod::Linear,
        boundary: SplineBoundary::NotAKnot,
        extrapolation: Extrapolation::Extrapolate,
        smoothing: 0.0,
    }
}

fn finite_range(x: &[f64], y: &[f64]) -> Option<(f64, f64, usize)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut n = 0;
    for (a, b) in x.iter().zip(y) {
        if a.is_finite() && b.is_finite() {
            lo = lo.min(*a);
            hi = hi.max(*a);
            n += 1;
        }
    }
    (n > 0).then_some((lo, hi, n))
}

/// Combines the data curve `(x, y)` with the reference curve `(rx, ry)`: `y op reference(x)`.
///
/// Without `common_only` the result has one point per data point, at the data's X values and in
/// the data's order (a data point whose X or Y is missing gives a missing value). With
/// `common_only` it covers only the X range that both curves span, at the sorted, merged X values
/// of both curves inside that range. Returns `None` when the reference has fewer than two valid
/// points, or, for `common_only`, when the curves share no X range.
pub fn curve_math_ref(
    x: &[f64],
    y: &[f64],
    rx: &[f64],
    ry: &[f64],
    op: CurveOp,
    common_only: bool,
) -> Option<CurveMathResult> {
    let (rlo, rhi, rn) = finite_range(rx, ry)?;
    if rn < 2 {
        return None;
    }
    let opts = linear();
    if !common_only {
        let reference = interp1(rx, ry, x, &opts);
        let out = y.iter().zip(&reference).map(|(&v, &r)| op.apply(v, r)).collect();
        return Some(CurveMathResult { x: x.to_vec(), y: out });
    }
    let (dlo, dhi, dn) = finite_range(x, y)?;
    if dn < 2 {
        return None;
    }
    let (lo, hi) = (rlo.max(dlo), rhi.min(dhi));
    if lo > hi {
        return None;
    }
    let mut grid: Vec<f64> = x
        .iter()
        .zip(y)
        .filter(|(a, b)| a.is_finite() && b.is_finite())
        .map(|(a, _)| *a)
        .chain(rx.iter().zip(ry).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(a, _)| *a))
        .filter(|v| *v >= lo && *v <= hi)
        .collect();
    grid.sort_by(f64::total_cmp);
    grid.dedup();
    let data = interp1(x, y, &grid, &opts);
    let reference = interp1(rx, ry, &grid, &opts);
    let out = data.iter().zip(&reference).map(|(&v, &r)| op.apply(v, r)).collect();
    Some(CurveMathResult { x: grid, y: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_math_matches_origin_vmathtool() {
        // Origin: A = 1..6, B = {2, 0, missing, -1, 5} (five rows); vmathtool A op B.
        let nan = f64::NAN;
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [2.0, 0.0, nan, -1.0, 5.0];
        let same = |got: Vec<f64>, want: [f64; 6]| {
            for (g, w) in got.iter().zip(want) {
                assert!(g == &w || (g.is_nan() && w.is_nan()), "{got:?} != {want:?}");
            }
        };
        same(column_math(&a, CurveOp::Divide, &b), [0.5, nan, nan, -4.0, 1.0, nan]);
        same(column_math(&a, CurveOp::Power, &b), [1.0, 1.0, nan, 0.25, 3125.0, 1.0]);
        same(column_math(&a, CurveOp::Subtract, &b), [-1.0, 2.0, nan, 5.0, 0.0, 6.0]);
        // A shorter than the operand: one row per row of A.
        assert_eq!(column_math(&[1.0, 2.0, 3.0], CurveOp::Multiply, &[5.0, 6.0, 7.0, 8.0, 9.0]), [5.0, 12.0, 21.0]);
        // A negative base to the power 0.5 is missing.
        let r = curve_math_const(&[-8.0, 4.0, 9.0], CurveOp::Power, 0.5);
        assert!(r[0].is_nan() && r[1] == 2.0 && r[2] == 3.0);
    }

    #[test]
    fn reference_is_interpolated_linearly_and_continued_outside() {
        let x = [0.0, 1.0, 2.0, 5.0];
        let y = [1.0, 1.0, 1.0, 1.0];
        let r = curve_math_ref(&x, &y, &[1.0, 3.0], &[10.0, 30.0], CurveOp::Subtract, false).unwrap();
        assert_eq!(r.x, x);
        // reference is 10 x: 0 at 0, 10 at 1, 20 at 2, 50 at 5
        assert_eq!(r.y, vec![1.0, -9.0, -19.0, -49.0]);
    }

    #[test]
    fn division_by_zero_and_bad_powers_are_missing() {
        assert!(CurveOp::Divide.apply(1.0, 0.0).is_nan());
        assert!(CurveOp::Power.apply(-8.0, 0.5).is_nan());
        assert_eq!(CurveOp::Power.apply(3.0, 2.0), 9.0);
        assert_eq!(curve_math_const(&[1.0, f64::NAN], CurveOp::Add, 2.0)[0], 3.0);
        assert!(curve_math_const(&[1.0, f64::NAN], CurveOp::Add, 2.0)[1].is_nan());
    }

    #[test]
    fn common_range_merges_the_x_values_of_both_curves() {
        let x = [5.0, 1.0, 9.0, 3.0, 7.0, 4.0, 6.0];
        let y: Vec<f64> = x.iter().map(|v| 10.0 * v).collect();
        let r = curve_math_ref(&x, &y, &[8.5, 1.5, 3.5], &[72.25, 2.25, 12.25], CurveOp::Subtract, true).unwrap();
        assert_eq!(r.x, vec![1.5, 3.0, 3.5, 4.0, 5.0, 6.0, 7.0, 8.5]);
        assert!((r.y[0] - 12.75).abs() < 1e-12);
    }

    #[test]
    fn too_little_reference_gives_none() {
        assert!(curve_math_ref(&[1.0, 2.0], &[1.0, 2.0], &[1.0], &[1.0], CurveOp::Add, false).is_none());
        assert!(curve_math_ref(&[1.0, 2.0], &[1.0, 2.0], &[5.0, 6.0], &[1.0, 1.0], CurveOp::Add, true).is_none());
    }
}
