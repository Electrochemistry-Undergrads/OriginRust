//! STL: seasonal and trend decomposition by loess (T498; Cleveland, Cleveland, McRae and Terpenning 1990, the
//! algorithm of their Fortran `stl` as statsmodels' `STL` runs it; Origin 2024b's Time Series Analysis app was not
//! run, so nothing here is measured in Origin; `docs/spec/statistics.md` section 40).
//!
//! With period `np`, the seasonal smoother's length `ns` (odd, at least 3; 7 by default), the trend smoother's `nt`
//! (the smallest odd integer at least `1.5 np / (1 - 1.5 / ns)`) and the low-pass filter's `nl` (the smallest odd
//! integer above `np`), all loess fits of degree 1 at every point:
//!
//! - **Inner loop** (5 passes, 2 when robust): the detrended series is split into its `np` cycle-subseries, each
//!   smoothed by loess of length `ns` and extended by one point at each end; the result goes through moving averages of
//!   lengths `np`, `np` and 3 and a loess of length `nl` (the low-pass), which is taken off to give the seasonal part;
//!   the deseasonalized series smoothed by loess of length `nt` is the trend.
//! - **Outer loop** (robust only, 15 passes): robustness weights from the residuals, `(1 - (r / 6 MAD)^2)^2` (1 below
//!   0.001 of `6 MAD`, 0 above 0.999 of it), multiply the loess weights of the next passes.
//!
//! A loess estimate at `x` takes the tricube weights `(1 - (d / h)^3)^3` of the neighbours within `h`, the distance to
//! the farther end of the window (widened by `floor((len - n) / 2)` when the smoother is longer than the series), and
//! a local line unless the points are too close together. The residual is `y - seasonal - trend`. Checked against
//! statsmodels 0.15 in `stl_matches_statsmodels`.

/// The smoother lengths and passes of an STL decomposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StlSpec {
    pub period: usize,
    /// The seasonal smoother's length (odd, at least 3).
    pub seasonal: usize,
    /// Robustness passes against outliers.
    pub robust: bool,
}

/// The parts of the series and the final robustness weights (all 1 without the robust passes).
#[derive(Debug, Clone, PartialEq)]
pub struct Stl {
    pub seasonal: Vec<f64>,
    pub trend: Vec<f64>,
    pub residual: Vec<f64>,
    pub weights: Vec<f64>,
    /// The trend and low-pass smoother lengths used.
    pub trend_length: usize,
    pub low_pass_length: usize,
}

/// Why the series could not be decomposed.
#[derive(Debug, Clone, PartialEq)]
pub enum StlError {
    BadPeriod(usize),
    BadSeasonal(usize),
    TooShort { n: usize, period: usize },
    Missing(usize),
}

impl std::fmt::Display for StlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StlError::BadPeriod(m) => write!(f, "the period must be at least 2 (got {m})"),
            StlError::BadSeasonal(s) => write!(f, "the seasonal smoother must be odd and at least 3 (got {s})"),
            StlError::TooShort { n, period } => {
                write!(f, "{n} values are too few for period {period}: at least two full periods are needed")
            }
            StlError::Missing(i) => write!(f, "value {} is missing: the series must be complete", i + 1),
        }
    }
}

impl std::error::Error for StlError {}

/// The smallest odd integer at least `x`.
fn odd_at_least(x: f64) -> usize {
    let k = x.ceil() as usize;
    if k.is_multiple_of(2) { k + 1 } else { k }
}

/// The loess estimate at `xs` (from 1) from the points `nleft..=nright` (from 1) of `y[..n]`; `None` when no point
/// has weight. `w` is scratch of at least `nright` values.
#[allow(clippy::too_many_arguments)]
fn est(
    y: &[f64],
    n: usize,
    len: usize,
    deg: usize,
    xs: usize,
    nleft: usize,
    nright: usize,
    w: &mut [f64],
    rw: Option<&[f64]>,
) -> Option<f64> {
    let range = n as f64 - 1.0;
    let mut h = (xs as f64 - nleft as f64).max(nright as f64 - xs as f64);
    if len > n {
        h += ((len - n) / 2) as f64;
    }
    let (h9, h1) = (0.999 * h, 0.001 * h);
    let mut a = 0.0;
    for j in nleft - 1..nright {
        w[j] = 0.0;
        let r = (j as f64 + 1.0 - xs as f64).abs();
        if r <= h9 {
            w[j] = if r <= h1 { 1.0 } else { (1.0 - (r / h).powi(3)).powi(3) };
            if let Some(rw) = rw {
                w[j] *= rw[j];
            }
            a += w[j];
        }
    }
    if a <= 0.0 {
        return None;
    }
    for wj in &mut w[nleft - 1..nright] {
        *wj /= a;
    }
    if h > 0.0 && deg > 0 {
        let a: f64 = (nleft - 1..nright).map(|j| w[j] * (j as f64 + 1.0)).sum();
        let b = xs as f64 - a;
        let c: f64 = (nleft - 1..nright).map(|j| w[j] * (j as f64 + 1.0 - a).powi(2)).sum();
        if c.sqrt() > 0.001 * range {
            let b = b / c;
            for (j, wj) in w.iter_mut().enumerate().take(nright).skip(nleft - 1) {
                *wj *= b * (j as f64 + 1.0 - a) + 1.0;
            }
        }
    }
    Some((nleft - 1..nright).map(|j| w[j] * y[j]).sum())
}

/// Loess of length `len` and degree `deg` at every point of `y[..n]` into `ys` (every point fitted).
fn ess(y: &[f64], n: usize, len: usize, deg: usize, rw: Option<&[f64]>, ys: &mut [f64], scratch: &mut [f64]) {
    if n < 2 {
        ys[0] = y[0];
        return;
    }
    if len >= n {
        for i in 0..n {
            ys[i] = est(y, n, len, deg, i + 1, 1, n, scratch, rw).unwrap_or(y[i]);
        }
        return;
    }
    let nsh = (len + 2) / 2;
    let (mut nleft, mut nright) = (1, len);
    for i in 0..n {
        if i + 1 > nsh && nright != n {
            nleft += 1;
            nright += 1;
        }
        ys[i] = est(y, n, len, deg, i + 1, nleft, nright, scratch, rw).unwrap_or(y[i]);
    }
}

/// The moving average of length `len` of `x[..n]` into `ave[..n - len + 1]`.
fn moving_average(x: &[f64], n: usize, len: usize, ave: &mut [f64]) {
    let flen = len as f64;
    let mut v: f64 = x[..len].iter().sum();
    ave[0] = v / flen;
    for j in 1..n - len + 1 {
        v += x[len + j - 1] - x[j - 1];
        ave[j] = v / flen;
    }
}

/// Decomposes `y` by STL.
pub fn stl(y: &[f64], spec: StlSpec) -> Result<Stl, StlError> {
    let np = spec.period;
    if np < 2 {
        return Err(StlError::BadPeriod(np));
    }
    let ns = spec.seasonal;
    if ns < 3 || ns.is_multiple_of(2) {
        return Err(StlError::BadSeasonal(ns));
    }
    if let Some(i) = y.iter().position(|v| !v.is_finite()) {
        return Err(StlError::Missing(i));
    }
    let n = y.len();
    if n < 2 * np {
        return Err(StlError::TooShort { n, period: np });
    }
    let nt = odd_at_least(1.5 * np as f64 / (1.0 - 1.5 / ns as f64));
    let nl = odd_at_least(np as f64 + 1.0);
    let (inner, outer) = if spec.robust { (2, 15) } else { (5, 0) };
    let mut season = vec![0.0; n];
    let mut trend = vec![0.0; n];
    let mut rw = vec![1.0; n];
    let mut use_rw = false;
    let size = n + 2 * np;
    let mut w0 = vec![0.0; size];
    let mut w1 = vec![0.0; size];
    let mut w2 = vec![0.0; size];
    let mut w4 = vec![0.0; size];
    let mut k = 0;
    loop {
        for _ in 0..inner {
            for i in 0..n {
                w0[i] = y[i] - trend[i];
            }
            // The cycle-subseries, smoothed and extended by one point at each end, into w1.
            for j in 0..np {
                let kk = (n - (j + 1)) / np + 1;
                let sub: Vec<f64> = (0..kk).map(|i| w0[i * np + j]).collect();
                let sub_rw: Vec<f64> = (0..kk).map(|i| rw[i * np + j]).collect();
                let srw = use_rw.then_some(sub_rw.as_slice());
                let mut smooth = vec![0.0; kk + 2];
                let mut scratch = vec![0.0; kk.max(1)];
                ess(&sub, kk, ns, 1, srw, &mut smooth[1..], &mut scratch);
                let nright = ns.min(kk);
                smooth[0] = est(&sub, kk, ns, 1, 0, 1, nright, &mut scratch, srw).unwrap_or(smooth[1]);
                let nleft = (kk as isize - ns as isize + 1).max(1) as usize;
                smooth[kk + 1] = est(&sub, kk, ns, 1, kk + 1, nleft, kk, &mut scratch, srw).unwrap_or(smooth[kk]);
                for (m, v) in smooth.iter().enumerate() {
                    w1[m * np + j] = *v;
                }
            }
            // The low-pass filter of w1: moving averages np, np, 3, then loess nl.
            moving_average(&w1, size, np, &mut w2);
            moving_average(&w2, size - np + 1, np, &mut w0);
            moving_average(&w0, size - 2 * np + 2, 3, &mut w2);
            ess(&w2, n, nl, 1, None, &mut w0, &mut w4);
            for i in 0..n {
                season[i] = w1[np + i] - w0[i];
                w0[i] = y[i] - season[i];
            }
            let r = use_rw.then_some(rw.as_slice());
            ess(&w0, n, nt, 1, r, &mut trend, &mut w2);
        }
        k += 1;
        if k > outer {
            break;
        }
        // Robustness weights from the residuals.
        let mut r: Vec<f64> = (0..n).map(|i| (y[i] - trend[i] - season[i]).abs()).collect();
        let mut sorted = r.clone();
        sorted.sort_by(f64::total_cmp);
        let (m0, m1) = (n / 2, n - n / 2 - 1);
        let cmad = 3.0 * (sorted[m0] + sorted[m1]);
        if cmad == 0.0 {
            rw.iter_mut().for_each(|v| *v = 1.0);
        } else {
            let (c9, c1) = (0.999 * cmad, 0.001 * cmad);
            for (v, ri) in rw.iter_mut().zip(r.iter_mut()) {
                *v = if *ri <= c1 {
                    1.0
                } else if *ri <= c9 {
                    (1.0 - (*ri / cmad).powi(2)).powi(2)
                } else {
                    0.0
                };
            }
        }
        use_rw = true;
    }
    let residual = (0..n).map(|i| y[i] - season[i] - trend[i]).collect();
    Ok(Stl { seasonal: season, trend, residual, weights: rw, trend_length: nt, low_pass_length: nl })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stl_matches_statsmodels() {
        // statsmodels 0.15's STL (re-workspace/scratch/t480/stl_ref.py): a quarterly series, a monthly one, and the
        // monthly one with two outliers by the robust passes (seasonal smoother 9).
        let e: Vec<f64> = (0..120).map(|t| ((t * 7919 + 13) % 101) as f64 / 101.0 - 0.5).collect();
        let pat = [1.0, -2.0, 0.5, 0.5];
        let y4: Vec<f64> = (0..48).map(|t| 10.0 + 0.1 * t as f64 + pat[t % 4] + e[t]).collect();
        let y12: Vec<f64> = (0..120)
            .map(|t| 5.0 + 0.02 * t as f64 + 3.0 * (2.0 * std::f64::consts::PI * t as f64 / 12.0).sin() + e[t])
            .collect();
        let mut y12o = y12.clone();
        y12o[30] += 15.0;
        y12o[77] -= 12.0;
        type Case<'a> = (&'a [f64], StlSpec, [f64; 5], [f64; 5]);
        let cases: [Case; 3] = [
            (
                &y4,
                StlSpec { period: 4, seasonal: 7, robust: false },
                [0.8281933485441194, -2.0717550054819696, 1.0408120411266815, 0.4928646164002656, 0.34514871366953437],
                [9.980215730385423, 10.102306802729858, 12.37697149421729, 14.68372546507237, 14.773823624141329],
            ),
            (
                &y12,
                StlSpec { period: 12, seasonal: 7, robust: false },
                [
                    -0.41327905254893277,
                    1.4840875937248308,
                    -0.01855066435942291,
                    -3.003861636608452,
                    -1.450546062592443,
                ],
                [5.0752350175031085, 5.085898105914603, 6.18308740661475, 7.241917850290835, 7.248126680685113],
            ),
            (
                &y12o,
                StlSpec { period: 12, seasonal: 9, robust: true },
                [
                    -0.5328536516073779,
                    1.398128218675523,
                    -0.13899244524045368,
                    -2.8757668146089093,
                    -1.3457012002455142,
                ],
                [5.143873586143236, 5.151714267537456, 6.2788007006162525, 7.15950206204616, 7.167980764213565],
            ),
        ];
        for (y, spec, seasonal, trend) in cases {
            let s = stl(y, spec).unwrap();
            let n = y.len();
            for (k, i) in [0, 1, n / 2, n - 2, n - 1].into_iter().enumerate() {
                assert!(
                    (s.seasonal[i] - seasonal[k]).abs() < 1e-9,
                    "{spec:?} seasonal {i}: {} vs {}",
                    s.seasonal[i],
                    seasonal[k]
                );
                assert!((s.trend[i] - trend[k]).abs() < 1e-9, "{spec:?} trend {i}: {} vs {}", s.trend[i], trend[k]);
                assert!((s.residual[i] - (y[i] - s.seasonal[i] - s.trend[i])).abs() < 1e-12);
            }
        }
        let s = stl(&y12o, StlSpec { period: 12, seasonal: 9, robust: true }).unwrap();
        assert_eq!((s.weights[30], s.weights[77]), (0.0, 0.0), "the outliers lose their weight");
        assert!((s.weights[0] - 0.9821140126326833).abs() < 1e-9, "{}", s.weights[0]);
        assert_eq!((s.trend_length, s.low_pass_length), (23, 13));
        // Refusals.
        assert_eq!(stl(&y4, StlSpec { period: 1, seasonal: 7, robust: false }).unwrap_err(), StlError::BadPeriod(1));
        assert_eq!(stl(&y4, StlSpec { period: 4, seasonal: 6, robust: false }).unwrap_err(), StlError::BadSeasonal(6));
        assert!(matches!(
            stl(&y4[..7], StlSpec { period: 4, seasonal: 7, robust: false }),
            Err(StlError::TooShort { .. })
        ));
    }
}
