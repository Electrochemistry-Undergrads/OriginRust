//! Upper and lower envelopes of a curve (Origin's Signal Processing > Envelope, `envelope`).
//!
//! Measured rules (see `docs/spec/analysis-tools.md` section 13.2):
//!
//! - A point is an upper extreme when it is the first largest value in the window of `npts`
//!   points on each side of it (itself included): every value on its left is strictly smaller,
//!   every value on its right smaller or equal, so a flat top counts once, at its first point.
//!   The lower extreme is the same with the smallest value. A point whose window does not fit
//!   in the data (fewer than `npts` points on either side) is never an extreme, so the first
//!   and last `npts` points are not.
//! - `npts` is cut to its integer part and a value below 1 (0) acts as 1. A negative value is
//!   refused.
//! - The envelope is the natural cubic spline through the extremes (two extremes give the
//!   straight line through them), evaluated at every X of the curve, outside the first and
//!   last extreme by the end pieces of the spline. Fewer than two extremes of a kind refuse the
//!   whole run, also when only the other kind was asked for.
//! - Data in monotonic X, rising or falling, are used as they are. Data in any other X order
//!   are sorted by X (stable) for the points the spline goes through, while the extremes are
//!   found in the original row order: the `k`-th extreme row is the `k`-th point after sorting.

use std::cmp::Ordering;

/// Which envelope(s) to compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeType {
    Upper,
    Lower,
    Both,
}

/// An envelope request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopeOptions {
    pub kind: EnvelopeType,
    /// Points on each side of an extreme (Origin's `npts`, default 2).
    pub npts: f64,
}

impl Default for EnvelopeOptions {
    fn default() -> Self {
        Self { kind: EnvelopeType::Upper, npts: 2.0 }
    }
}

/// The envelopes: each is the spline value at every input X, plus the extremes used (0-based
/// positions in the order the extremes were searched, which is the input order).
#[derive(Debug, Clone, PartialEq)]
pub struct Envelope {
    pub upper: Option<Vec<f64>>,
    pub lower: Option<Vec<f64>>,
    pub upper_points: Vec<usize>,
    pub lower_points: Vec<usize>,
}

/// Why no envelope came out.
#[derive(Debug, Clone, PartialEq)]
pub enum EnvelopeError {
    /// `npts` is negative or not a number.
    BadNpts,
    /// Fewer than two upper or lower extremes were found.
    TooFewExtremes,
    /// Two extremes with the same X (the spline needs distinct X).
    RepeatedX,
}

/// The 0-based positions of the upper (or lower) extremes of `y` with `n` points on each side.
pub fn extreme_points(y: &[f64], n: usize, upper: bool) -> Vec<usize> {
    let len = y.len();
    let mut out = Vec::new();
    if n > len / 2 || len < n.saturating_mul(2).saturating_add(1) {
        return out;
    }
    let sign = if upper { 1.0 } else { -1.0 };
    for i in n..len - n {
        let v = sign * y[i];
        let left = (i - n..i).all(|j| sign * y[j] < v);
        let right = (i + 1..=i + n).all(|j| sign * y[j] <= v);
        if left && right {
            out.push(i);
        }
    }
    out
}

/// The natural cubic spline through `(xs, ys)` (strictly increasing `xs`, at least two points),
/// evaluated at `at`; outside the nodes the end pieces continue.
pub fn natural_spline(xs: &[f64], ys: &[f64], at: &[f64]) -> Vec<f64> {
    let n = xs.len();
    // Second derivatives m[i], natural ends.
    let mut m = vec![0.0; n];
    if n > 2 {
        let h: Vec<f64> = (0..n - 1).map(|i| xs[i + 1] - xs[i]).collect();
        let k = n - 2;
        let mut diag = vec![0.0; k];
        let mut upper = vec![0.0; k];
        let mut rhs = vec![0.0; k];
        for i in 0..k {
            diag[i] = 2.0 * (h[i] + h[i + 1]);
            upper[i] = h[i + 1];
            rhs[i] = 6.0 * ((ys[i + 2] - ys[i + 1]) / h[i + 1] - (ys[i + 1] - ys[i]) / h[i]);
        }
        // Thomas algorithm (the sub-diagonal of row i is h[i]).
        for i in 1..k {
            let w = h[i] / diag[i - 1];
            diag[i] -= w * upper[i - 1];
            rhs[i] -= w * rhs[i - 1];
        }
        let mut sol = vec![0.0; k];
        sol[k - 1] = rhs[k - 1] / diag[k - 1];
        for i in (0..k - 1).rev() {
            sol[i] = (rhs[i] - upper[i] * sol[i + 1]) / diag[i];
        }
        m[1..=k].copy_from_slice(&sol);
    }
    at.iter()
        .map(|&t| {
            // The piece holding t: the last node at or before it, clamped to the first and last piece.
            let i = match xs.partition_point(|&v| v <= t) {
                0 => 0,
                p => (p - 1).min(n - 2),
            };
            let h = xs[i + 1] - xs[i];
            let a = (xs[i + 1] - t) / h;
            let b = (t - xs[i]) / h;
            a * ys[i] + b * ys[i + 1] + ((a * a * a - a) * m[i] + (b * b * b - b) * m[i + 1]) * h * h / 6.0
        })
        .collect()
}

/// Computes the envelope(s) of the curve `(x, y)` (equal lengths, no missing values).
pub fn envelope(x: &[f64], y: &[f64], opts: &EnvelopeOptions) -> Result<Envelope, EnvelopeError> {
    if opts.npts.is_nan() || opts.npts < 0.0 {
        return Err(EnvelopeError::BadNpts);
    }
    // `as usize` saturates for a huge or infinite value; such a window never fits (too few extremes).
    let n = (opts.npts.floor() as usize).max(1);
    // Data in monotonic X (either direction) are used as they are; any other order is sorted by
    // X for the spline nodes, while the extremes are still searched in the input order.
    let mut order: Vec<usize> = (0..x.len()).collect();
    let rising = x.windows(2).all(|w| w[0] <= w[1]);
    let falling = x.windows(2).all(|w| w[0] >= w[1]);
    if !rising && !falling {
        order.sort_by(|&a, &b| x[a].partial_cmp(&x[b]).unwrap_or(Ordering::Equal));
    }
    let want_upper = matches!(opts.kind, EnvelopeType::Upper | EnvelopeType::Both);
    let want_lower = matches!(opts.kind, EnvelopeType::Lower | EnvelopeType::Both);
    let up = extreme_points(y, n, true);
    let lo = extreme_points(y, n, false);
    if (want_upper && up.len() < 2) || (want_lower && lo.len() < 2) {
        return Err(EnvelopeError::TooFewExtremes);
    }
    let build = |idx: &[usize]| -> Result<Vec<f64>, EnvelopeError> {
        let mut xs: Vec<f64> = idx.iter().map(|&k| x[order[k]]).collect();
        let mut ys: Vec<f64> = idx.iter().map(|&k| y[order[k]]).collect();
        if !rising && falling {
            xs.reverse();
            ys.reverse();
        }
        if xs.windows(2).any(|w| w[1] <= w[0]) {
            return Err(EnvelopeError::RepeatedX);
        }
        Ok(natural_spline(&xs, &ys, x))
    };
    Ok(Envelope {
        upper: if want_upper { Some(build(&up)?) } else { None },
        lower: if want_lower { Some(build(&lo)?) } else { None },
        upper_points: up,
        lower_points: lo,
    })
}

#[cfg(test)]
#[allow(clippy::excessive_precision)]
mod tests {
    use super::*;

    fn close(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len());
        for (u, v) in a.iter().zip(b) {
            assert!((u - v).abs() < 1e-9, "{u} vs {v}");
        }
    }

    #[test]
    fn extremes_take_the_first_of_a_flat_top() {
        let y = [0.0, 1.0, 3.0, 3.0, 1.0, 0.0, 2.0, 1.0, 2.0, 0.0];
        assert_eq!(extreme_points(&y, 1, true), vec![2, 6, 8]);
        assert_eq!(extreme_points(&y, 1, false), vec![5, 7]);
    }

    #[test]
    fn natural_spline_of_three_points() {
        // Natural spline through (4, 1.0237), (12, 1.6936), (23, 2.3894), as printed by Origin.
        let at = [0.0, 4.0, 5.0, 12.0, 29.0];
        let v = natural_spline(&[4.0, 12.0, 23.0], &[1.0237, 1.6936, 2.3894], &at);
        close(&v[..2], &[0.67581339712918687, 1.0237]);
        close(&v[3..], &[1.6936, 2.7439361080904745]);
    }

    #[test]
    fn origin_smooth_curve() {
        let y = [
            0.0, 0.9121, 0.9554, 0.8461, 1.0237, 0.6307, -0.3525, -0.707, -0.3384, -0.2037, -0.0678, 0.8168, 1.6936,
            1.616, 1.2631, 1.2698, 0.8895, -0.0256, -0.2994, 0.2627, 0.6222, 0.8107, 1.6037, 2.3894, 2.1779, 1.6015,
            1.4725, 1.1597, 0.3749, 0.2106,
        ];
        let want_upper = [
            0.6758133971291869,
            0.7613701069078948,
            0.8481396232057417,
            0.9357176771830144,
            1.0237,
            1.1116823228169856,
            1.1992603767942585,
            1.2860298930921055,
            1.3715866028708132,
            1.45552623729067,
            1.5374445275119617,
            1.6169372046949761,
            1.6936,
            1.767145025010874,
            1.837749913005655,
            1.9057086776859504,
            1.971315332753371,
            2.034863891909526,
            2.0966483688560245,
            2.156962777294476,
            2.2161011309264897,
            2.2743574434536757,
            2.332025728577643,
            2.3894,
            2.4467742714223575,
            2.504442556546325,
            2.56269886907351,
            2.6218372227055244,
            2.682151631143976,
            2.7439361080904745,
        ];
        let want_lower = [
            -0.9663818181818182,
            -0.9293272727272728,
            -0.8922727272727273,
            -0.8552181818181818,
            -0.8181636363636363,
            -0.781109090909091,
            -0.7440545454545454,
            -0.707,
            -0.6699454545454545,
            -0.6328909090909091,
            -0.5958363636363636,
            -0.5587818181818182,
            -0.5217272727272727,
            -0.48467272727272726,
            -0.4476181818181818,
            -0.41056363636363635,
            -0.3735090909090909,
            -0.33645454545454545,
            -0.2994,
            -0.26234545454545455,
            -0.22529090909090912,
            -0.18823636363636362,
            -0.15118181818181814,
            -0.11412727272727274,
            -0.07707272727272729,
            -0.0400181818181819,
            -0.0029636363636363905,
            0.03409090909090906,
            0.07114545454545451,
            0.10819999999999996,
        ];
        let x: Vec<f64> = (0..30).map(f64::from).collect();
        let x: &[f64] = &x;
        let r = envelope(x, &y, &EnvelopeOptions { kind: EnvelopeType::Both, npts: 2.0 }).unwrap();
        close(&r.upper.unwrap(), &want_upper);
        close(&r.lower.unwrap(), &want_lower);
    }

    #[test]
    fn origin_shuffled_x() {
        let x = [
            4.0, 9.0, 6.0, 5.0, 14.0, 17.0, 18.0, 1.0, 2.0, 15.0, 10.0, 3.0, 12.0, 7.0, 13.0, 0.0, 16.0, 11.0, 8.0,
            19.0,
        ];
        let y = [
            0.444, -2.921, -1.7, -1.323, 2.498, 1.594, -2.042, 1.783, -2.167, 0.705, -2.24, -2.989, 2.228, -1.743,
            -1.707, 2.895, 2.234, -1.264, 2.769, 0.235,
        ];
        let want_upper = [
            0.444,
            -2.921,
            -0.8452727272727274,
            -0.15001818181818186,
            1.8297629629629628,
            -1.2188531986531985,
            -2.042,
            2.6310000000000002,
            1.7332727272727273,
            0.705,
            -1.5488902356902352,
            1.038018181818182,
            2.228,
            -1.743,
            2.543746127946128,
            3.832436363636363,
            -0.3263164983164982,
            0.5660235690235692,
            -2.7407090909090908,
            -2.8651468013468007,
        ];
        let want_lower = [
            -2.986837823892316,
            2.387464893749611,
            -1.7,
            -3.013428367919236,
            -1.121166648555805,
            1.594,
            2.590261891825575,
            1.783,
            -0.25361891194615804,
            -0.3301190269209199,
            0.5508647473047925,
            -1.9552330959057769,
            -1.9285387163488503,
            0.8487104598990468,
            -1.707,
            3.819618911946158,
            0.5977381081744251,
            -1.264,
            2.769,
            3.5181190269209197,
        ];
        let x: &[f64] = &x;
        let r = envelope(x, &y, &EnvelopeOptions { kind: EnvelopeType::Both, npts: 1.0 }).unwrap();
        close(&r.upper.unwrap(), &want_upper);
        close(&r.lower.unwrap(), &want_lower);
    }

    #[test]
    fn origin_up_then_down_x() {
        let x = [
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 19.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0,
            10.0,
        ];
        let y = [
            1.067, -1.771, 2.646, 1.144, 2.799, 2.362, -1.207, -0.833, -2.004, -2.126, -2.609, -1.192, 0.619, -2.98,
            1.068, -0.973, -1.14, 1.911, -0.116, -1.105,
        ];
        let want_upper = [
            2.493,
            2.2738624916832997,
            2.646,
            3.0181375083167,
            2.799,
            1.6375277469751852,
            0.14352217540228177,
            -0.833,
            -0.6811118651585714,
            0.253754291417166,
            2.5106789260984206,
            1.8139431408787363,
            0.619,
            -0.5759431408787363,
            -1.2726789260984204,
            -0.973,
            0.4841220669771569,
            1.911,
            2.134332313151474,
            1.3870763805721888,
        ];
        let want_lower = [
            -3.619440623512613,
            -1.771,
            0.077440623512613,
            1.144,
            0.9045532462495813,
            -0.1339990657006363,
            -1.207,
            -1.7147090076154212,
            -1.7170510867840711,
            -1.4388676225606853,
            -4.82,
            -4.350302237029986,
            -3.7082417896239885,
            -2.98,
            -2.2517582103760114,
            -1.609697762970014,
            -1.14,
            -0.9095167733178203,
            -0.9077819733107689,
            -1.105,
        ];
        let x: &[f64] = &x;
        let r = envelope(x, &y, &EnvelopeOptions { kind: EnvelopeType::Both, npts: 1.0 }).unwrap();
        close(&r.upper.unwrap(), &want_upper);
        close(&r.lower.unwrap(), &want_lower);
    }

    #[test]
    fn origin_falling_x() {
        let x = [11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0];
        let y = [0.0, 2.0, 1.0, 3.0, 0.5, 4.0, 0.2, 2.0, 1.0, 5.0, 4.0, 3.0];
        let want_upper = [
            1.6138392857142858,
            2.0,
            2.3861607142857144,
            3.0,
            3.841517857142857,
            4.0,
            2.872767857142857,
            2.0,
            2.9174107142857144,
            5.0,
            7.082589285714286,
            8.0,
        ];
        let want_lower = [
            1.5,
            1.2425,
            1.0,
            0.7575,
            0.5,
            0.25249999999999995,
            0.2,
            0.49499999999999994,
            1.0,
            1.505,
            1.8,
            1.6750000000000003,
        ];
        let x: &[f64] = &x;
        let r = envelope(x, &y, &EnvelopeOptions { kind: EnvelopeType::Both, npts: 1.0 }).unwrap();
        close(&r.upper.unwrap(), &want_upper);
        close(&r.lower.unwrap(), &want_lower);
    }

    #[test]
    fn too_few_extremes_refuse_the_run() {
        let x: Vec<f64> = (0..8).map(f64::from).collect();
        let y = [5.0, 3.0, 1.0, 2.0, 4.0, 1.0, 2.0, 6.0];
        let both = EnvelopeOptions { kind: EnvelopeType::Both, npts: 1.0 };
        assert_eq!(envelope(&x, &y, &both), Err(EnvelopeError::TooFewExtremes));
        let lower = EnvelopeOptions { kind: EnvelopeType::Lower, npts: 1.0 };
        assert!(envelope(&x, &y, &lower).is_ok());
        let negative = EnvelopeOptions { kind: EnvelopeType::Lower, npts: -1.0 };
        assert_eq!(envelope(&x, &y, &negative), Err(EnvelopeError::BadNpts));
    }

    #[test]
    fn huge_npts_is_too_few_extremes() {
        let x: Vec<f64> = (0..8).map(f64::from).collect();
        let y = [0.0, 2.0, 1.0, 3.0, 0.5, 4.0, 0.2, 2.0];
        for npts in [f64::INFINITY, 1e30, 4.0] {
            let o = EnvelopeOptions { kind: EnvelopeType::Both, npts };
            assert_eq!(envelope(&x, &y, &o), Err(EnvelopeError::TooFewExtremes), "npts {npts}");
        }
        assert!(extreme_points(&y, usize::MAX, true).is_empty());
    }

    #[test]
    fn origin_first_and_last_y_missing() {
        // Rows 2..11 of the 12-row curve, the first and last Y missing (dropped), as printed by Origin.
        let x: Vec<f64> = (1..=10).map(f64::from).collect();
        let y = [2.0, 1.0, 3.0, 0.5, 4.0, 0.2, 2.0, 1.0, 5.0, 4.0];
        let want_upper = [2.0, 2.075, 3.0, 3.925, 4.0, 2.85, 2.0, 2.925, 5.0, 7.075];
        let want_lower = [1.2425, 1.0, 0.7575, 0.5, 0.25249999999999995, 0.2, 0.49499999999999994, 1.0, 1.505, 1.8];
        let r = envelope(&x, &y, &EnvelopeOptions { kind: EnvelopeType::Both, npts: 1.0 }).unwrap();
        close(&r.upper.unwrap(), &want_upper);
        close(&r.lower.unwrap(), &want_lower);
    }
}
