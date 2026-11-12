//! Interpolation and extrapolation (Origin `interp1`, `interp1xy`): linear, cubic spline
//! (natural or not-a-knot ends), Akima and the cubic B-spline.
//!
//! The cubic B-spline of Origin is a smoothing spline (Dierckx fit) whose smoothing factor `sf`
//! is the allowed sum of squared residuals; `sf = 0` means an automatic factor that we could not
//! reproduce. Here the B-spline method is the interpolating cubic (not-a-knot) spline with
//! Origin's straight-line continuation outside the data; a smoothing factor above zero is not
//! implemented and gives NaN (callers reject it first). See `docs/spec/analysis-tools.md`.

/// Interpolation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterpMethod {
    #[default]
    Linear,
    CubicSpline,
    Akima,
    /// Cubic B-spline of Origin (see the module comment for what is and is not implemented).
    BSpline,
}

/// End condition of the cubic spline (Origin default: not-a-knot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplineBoundary {
    Natural,
    #[default]
    NotAKnot,
}

/// Behaviour outside the X range of the data (`interp1` option `option`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Extrapolation {
    /// Continue the end segment (a straight line, or the end cubic).
    #[default]
    Extrapolate,
    /// NaN outside the range.
    Missing,
    /// The first or last Y value.
    Repeat,
}

/// Options of [`interp1`] and [`interp1xy`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InterpOptions {
    pub method: InterpMethod,
    pub boundary: SplineBoundary,
    pub extrapolation: Extrapolation,
    /// Smoothing factor of the B-spline method (`sf`): 0 interpolates; values above 0 are not
    /// implemented. Ignored by the other methods.
    pub smoothing: f64,
}

/// Slopes at the knots of a cubic Hermite representation.
fn spline_slopes(x: &[f64], y: &[f64], boundary: SplineBoundary) -> Vec<f64> {
    let n = x.len();
    let h: Vec<f64> = (0..n - 1).map(|i| x[i + 1] - x[i]).collect();
    let m: Vec<f64> = (0..n - 1).map(|i| (y[i + 1] - y[i]) / h[i]).collect();
    if n == 2 {
        return vec![m[0], m[0]];
    }
    if n == 3 && boundary == SplineBoundary::NotAKnot {
        // a single parabola through the three points
        let s0 = m[0] - h[0] * (m[1] - m[0]) / (h[0] + h[1]);
        let s1 = (m[0] * h[1] + m[1] * h[0]) / (h[0] + h[1]);
        let s2 = m[1] + h[1] * (m[1] - m[0]) / (h[0] + h[1]);
        return vec![s0, s1, s2];
    }
    // tridiagonal system a[i] s[i-1] + b[i] s[i] + c[i] s[i+1] = d[i]
    let mut a = vec![0.0; n];
    let mut b = vec![0.0; n];
    let mut c = vec![0.0; n];
    let mut d = vec![0.0; n];
    for i in 1..n - 1 {
        a[i] = h[i];
        b[i] = 2.0 * (h[i - 1] + h[i]);
        c[i] = h[i - 1];
        d[i] = 3.0 * (h[i] * m[i - 1] + h[i - 1] * m[i]);
    }
    match boundary {
        SplineBoundary::Natural => {
            b[0] = 2.0;
            c[0] = 1.0;
            d[0] = 3.0 * m[0];
            a[n - 1] = 1.0;
            b[n - 1] = 2.0;
            d[n - 1] = 3.0 * m[n - 2];
        }
        SplineBoundary::NotAKnot => {
            b[0] = h[1];
            c[0] = h[0] + h[1];
            d[0] = ((h[0] + 2.0 * c[0]) * h[1] * m[0] + h[0] * h[0] * m[1]) / c[0];
            a[n - 1] = h[n - 2] + h[n - 3];
            b[n - 1] = h[n - 3];
            let k = a[n - 1];
            d[n - 1] = (h[n - 2] * h[n - 2] * m[n - 3] + (2.0 * k + h[n - 2]) * h[n - 3] * m[n - 2]) / k;
        }
    }
    // Thomas algorithm
    for i in 1..n {
        let w = a[i] / b[i - 1];
        b[i] -= w * c[i - 1];
        d[i] -= w * d[i - 1];
    }
    let mut s = vec![0.0; n];
    s[n - 1] = d[n - 1] / b[n - 1];
    for i in (0..n - 1).rev() {
        s[i] = (d[i] - c[i] * s[i + 1]) / b[i];
    }
    s
}

/// Akima slopes with Origin's end rule: parabola through the three outer points at the end
/// points, the length-weighted mean of the two adjacent chord slopes at the next points.
fn akima_slopes(x: &[f64], y: &[f64]) -> Vec<f64> {
    let n = x.len();
    let h: Vec<f64> = (0..n - 1).map(|i| x[i + 1] - x[i]).collect();
    let m: Vec<f64> = (0..n - 1).map(|i| (y[i + 1] - y[i]) / h[i]).collect();
    if n == 2 {
        return vec![m[0], m[0]];
    }
    let mut s = vec![0.0; n];
    s[0] = m[0] - h[0] * (m[1] - m[0]) / (h[0] + h[1]);
    s[n - 1] = m[n - 2] + h[n - 2] * (m[n - 2] - m[n - 3]) / (h[n - 2] + h[n - 3]);
    if n >= 3 {
        s[1] = (m[0] * h[1] + m[1] * h[0]) / (h[0] + h[1]);
        s[n - 2] = (m[n - 3] * h[n - 2] + m[n - 2] * h[n - 3]) / (h[n - 2] + h[n - 3]);
    }
    for i in 2..n.saturating_sub(2) {
        let w1 = (m[i + 1] - m[i]).abs();
        let w2 = (m[i - 1] - m[i - 2]).abs();
        s[i] = if w1 + w2 > 0.0 { (w1 * m[i - 1] + w2 * m[i]) / (w1 + w2) } else { (m[i - 1] + m[i]) / 2.0 };
    }
    s
}

/// Interpolates the points `(x, y)` at the positions `at`. Pairs with a missing value are
/// dropped; unsorted data are sorted by x. Fewer than two valid points give all NaN. Positions
/// that are NaN give NaN.
pub fn interp1(x: &[f64], y: &[f64], at: &[f64], opts: &InterpOptions) -> Vec<f64> {
    let mut pts: Vec<(f64, f64)> =
        x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(&a, &b)| (a, b)).collect();
    let bspline = opts.method == InterpMethod::BSpline;
    // Origin needs four points for the B-spline; the smoothing fit is not implemented.
    if pts.len() < if bspline { 4 } else { 2 } || (bspline && opts.smoothing > 0.0) {
        return vec![f64::NAN; at.len()];
    }
    if pts.windows(2).any(|w| w[1].0 < w[0].0) {
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    let xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = pts.iter().map(|p| p.1).collect();
    let n = xs.len();
    let slopes = match opts.method {
        InterpMethod::Linear => None,
        InterpMethod::CubicSpline => Some(spline_slopes(&xs, &ys, opts.boundary)),
        InterpMethod::BSpline => Some(spline_slopes(&xs, &ys, SplineBoundary::NotAKnot)),
        InterpMethod::Akima => Some(akima_slopes(&xs, &ys)),
    };
    at.iter()
        .map(|&t| {
            if t.is_nan() {
                return f64::NAN;
            }
            if t < xs[0] || t > xs[n - 1] {
                match opts.extrapolation {
                    Extrapolation::Missing => return f64::NAN,
                    Extrapolation::Repeat => return if t < xs[0] { ys[0] } else { ys[n - 1] },
                    Extrapolation::Extrapolate => {
                        if bspline {
                            // straight line through the two nearest data points
                            let (a, b) = if t < xs[0] { (0, 1) } else { (n - 2, n - 1) };
                            return ys[a] + (ys[b] - ys[a]) / (xs[b] - xs[a]) * (t - xs[a]);
                        }
                    }
                }
            }
            // segment k with xs[k] <= t <= xs[k+1] (end segments for outside points)
            let k = xs.partition_point(|&v| v <= t).saturating_sub(1).min(n - 2);
            let hk = xs[k + 1] - xs[k];
            let mk = (ys[k + 1] - ys[k]) / hk;
            let u = t - xs[k];
            match &slopes {
                None => ys[k] + mk * u,
                Some(s) => {
                    let c2 = (3.0 * mk - 2.0 * s[k] - s[k + 1]) / hk;
                    let c3 = (s[k] + s[k + 1] - 2.0 * mk) / (hk * hk);
                    ys[k] + u * (s[k] + u * (c2 + u * c3))
                }
            }
        })
        .collect()
}

/// Interpolates onto `npts` equally spaced points between `xmin` and `xmax` (default: the range
/// of the data), like Origin's `interp1xy` (default `npts` is 100). Returns `(x, y)`; outside the
/// data range the end segments are continued.
pub fn interp1xy(
    x: &[f64],
    y: &[f64],
    npts: usize,
    xmin: Option<f64>,
    xmax: Option<f64>,
    opts: &InterpOptions,
) -> (Vec<f64>, Vec<f64>) {
    let finite = || x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(a, _)| *a);
    let lo = xmin.unwrap_or_else(|| finite().fold(f64::INFINITY, f64::min));
    let hi = xmax.unwrap_or_else(|| finite().fold(f64::NEG_INFINITY, f64::max));
    let gx: Vec<f64> = match npts {
        0 => Vec::new(),
        1 => vec![lo],
        _ => (0..npts).map(|i| lo + (hi - lo) * i as f64 / (npts - 1) as f64).collect(),
    };
    let o = InterpOptions { extrapolation: Extrapolation::Extrapolate, ..*opts };
    let gy = interp1(x, y, &gx, &o);
    (gx, gy)
}
