//! FIR filter design and application as Origin's FIR Filter app (OriginPro, Origin 2027, File
//! Exchange; `docs/spec/analysis-tools.md` 17). Nothing here is measured in Origin: the app's page
//! lists the controls and no formulas, so the numbers follow `scipy.signal` (`firwin`, `firls`,
//! `kaiserord`, `get_window`; `crates/calc/tests/scipy_fir.rs`, made by `tools/iir-fir-ref.py`).
//!
//! Documented by Origin: response types low pass, high pass, band pass and band stop; the methods
//! Least Squares and Window; the filter order (an integer; for the Window method a Minimum that
//! computes it with the Kaiser window); the 17 windows (Bartlett, Bartlett-Hanning, Blackman,
//! Blackman-Harris, Bohman, Chebyshev, Flat Top, Gaussian, Hamming, Hanning, Kaiser, Nuttall,
//! Parzen, Rectangle, Taylor, Triangular, Tukey); Scale Passband (the passband centre has gain
//! 1); frequencies in Hz or normalized to the Nyquist frequency; passband and stopband weights
//! (Least Squares); the magnitude specification in dB or linear (Window, minimum order);
//! Forward-Backward Filtering.
//!
//! Sheaf's choices, unmeasured: the filter has `order + 1` coefficients (as MATLAB's `fir1`); high
//! pass and band stop need an even order; the Window method is `firwin` (ideal response times the
//! window), Least Squares is `firls` (constant desired gain per band, weights per band, both
//! generalized to an even number of coefficients); the minimum order is `kaiserord` on the
//! smaller of the passband deviation and the stopband deviation with the cutoffs half way between
//! each pass and stop frequency, and the window of that design is the selected window (Kaiser
//! with beta from the attenuation when Kaiser is selected); window parameters default as
//! MATLAB's (Chebyshev 100 dB, Gaussian alpha 2.5, Tukey 0.5, Taylor 4 bands and 30 dB), Kaiser
//! beta 5; one-way filtering removes the delay (the output is aligned with the input, the ends
//! extended by odd reflection of `order` points), forward-backward filtering is `filtfilt` (odd
//! extension of `3 * numtaps` points, which needs more points than that).

use std::f64::consts::PI;

use nalgebra::{DMatrix, DVector};
use rustfft::{FftPlanner, num_complex::Complex};
use serde::{Deserialize, Serialize};

use crate::iir::{FilterError, FilterResponse, FreqUnit, cutoffs, odd_ext, pass_stop};

/// The largest filter order Sheaf designs.
pub const MAX_FIR_ORDER: usize = 1000;

/// Origin's Method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FirMethod {
    LeastSquares,
    #[default]
    Window,
}

/// Origin's Window list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FirWindow {
    Bartlett,
    BartlettHanning,
    Blackman,
    BlackmanHarris,
    Bohman,
    Chebyshev,
    FlatTop,
    Gaussian,
    #[default]
    Hamming,
    Hanning,
    Kaiser,
    Nuttall,
    Parzen,
    Rectangle,
    Taylor,
    Triangular,
    Tukey,
}

impl FirWindow {
    pub const ALL: [FirWindow; 17] = [
        FirWindow::Bartlett,
        FirWindow::BartlettHanning,
        FirWindow::Blackman,
        FirWindow::BlackmanHarris,
        FirWindow::Bohman,
        FirWindow::Chebyshev,
        FirWindow::FlatTop,
        FirWindow::Gaussian,
        FirWindow::Hamming,
        FirWindow::Hanning,
        FirWindow::Kaiser,
        FirWindow::Nuttall,
        FirWindow::Parzen,
        FirWindow::Rectangle,
        FirWindow::Taylor,
        FirWindow::Triangular,
        FirWindow::Tukey,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FirWindow::Bartlett => "Bartlett",
            FirWindow::BartlettHanning => "Bartlett-Hanning",
            FirWindow::Blackman => "Blackman",
            FirWindow::BlackmanHarris => "Blackman-Harris",
            FirWindow::Bohman => "Bohman",
            FirWindow::Chebyshev => "Chebyshev",
            FirWindow::FlatTop => "Flat Top",
            FirWindow::Gaussian => "Gaussian",
            FirWindow::Hamming => "Hamming",
            FirWindow::Hanning => "Hanning",
            FirWindow::Kaiser => "Kaiser",
            FirWindow::Nuttall => "Nuttall",
            FirWindow::Parzen => "Parzen",
            FirWindow::Rectangle => "Rectangle",
            FirWindow::Taylor => "Taylor",
            FirWindow::Triangular => "Triangular",
            FirWindow::Tukey => "Tukey",
        }
    }

    /// What the window parameter means, if the window has one.
    pub fn param_label(self) -> Option<&'static str> {
        match self {
            FirWindow::Chebyshev => Some("Sidelobe attenuation (dB)"),
            FirWindow::Gaussian => Some("Alpha"),
            FirWindow::Kaiser => Some("Beta"),
            FirWindow::Taylor => Some("Sidelobe level (dB)"),
            FirWindow::Tukey => Some("Taper fraction"),
            _ => None,
        }
    }

    /// The default of the window parameter.
    pub fn default_param(self) -> f64 {
        match self {
            FirWindow::Chebyshev => 100.0,
            FirWindow::Gaussian => 2.5,
            FirWindow::Kaiser => 5.0,
            FirWindow::Taylor => 30.0,
            FirWindow::Tukey => 0.5,
            _ => 0.0,
        }
    }
}

/// Origin's Unit of the magnitude specification (Window, minimum order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MagnitudeUnit {
    #[default]
    Db,
    /// The passband ripple and the stopband "attenuation" are deviations from 1 and 0.
    Linear,
}

/// The options of the FIR Filter dialog.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FirSpec {
    pub response: FilterResponse,
    pub method: FirMethod,
    /// The filter order; `None` is Minimum (Window method only).
    pub order: Option<usize>,
    pub window: FirWindow,
    /// The parameter of the window (Chebyshev, Gaussian, Kaiser, Taylor, Tukey); `None` is the default.
    pub window_param: Option<f64>,
    pub scale_passband: bool,
    pub unit: FreqUnit,
    pub sample_frequency: Option<f64>,
    /// Cutoff (Window method with a fixed order); the first of two for the band responses.
    pub cutoff: Option<f64>,
    pub cutoff2: Option<f64>,
    /// Pass and stop frequencies (Least Squares; Window with the minimum order).
    pub pass: Option<f64>,
    pub pass2: Option<f64>,
    pub stop: Option<f64>,
    pub stop2: Option<f64>,
    /// Least Squares weights of the passbands and the stopbands.
    pub pass_weight: f64,
    pub stop_weight: f64,
    pub magnitude_unit: MagnitudeUnit,
    pub passband_ripple: f64,
    pub stopband_attenuation: f64,
    pub forward_backward: bool,
}

impl Default for FirSpec {
    fn default() -> Self {
        FirSpec {
            response: FilterResponse::LowPass,
            method: FirMethod::Window,
            order: Some(20),
            window: FirWindow::Hamming,
            window_param: None,
            scale_passband: true,
            unit: FreqUnit::Hz,
            sample_frequency: None,
            cutoff: None,
            cutoff2: None,
            pass: None,
            pass2: None,
            stop: None,
            stop2: None,
            pass_weight: 1.0,
            stop_weight: 1.0,
            magnitude_unit: MagnitudeUnit::Db,
            passband_ripple: 1.0,
            stopband_attenuation: 60.0,
            forward_backward: false,
        }
    }
}

/// A designed FIR filter.
#[derive(Debug, Clone, PartialEq)]
pub struct FirFilter {
    pub coefficients: Vec<f64>,
    /// The cutoffs or the band edges as used, normalized (Nyquist = 1).
    pub edges: Vec<f64>,
}

impl FirFilter {
    pub fn order(&self) -> usize {
        self.coefficients.len() - 1
    }

    /// Magnitude of the frequency response at `w` (normalized, 1 is Nyquist).
    pub fn magnitude(&self, w: f64) -> f64 {
        self.amplitude_complex(w).norm()
    }

    fn amplitude_complex(&self, w: f64) -> Complex<f64> {
        self.coefficients
            .iter()
            .enumerate()
            .fold(Complex::new(0.0, 0.0), |a, (k, h)| a + Complex::from_polar(*h, -PI * w * k as f64))
    }

    /// Filters `y`. One way: the delay of the filter (`order / 2` points, rounded down) is
    /// removed, the ends extended by odd reflection. Forward and backward: `filtfilt`.
    pub fn apply(&self, y: &[f64], forward_backward: bool) -> Result<Vec<f64>, FilterError> {
        let h = &self.coefficients;
        let numtaps = h.len();
        if forward_backward {
            let pad = 3 * numtaps;
            if y.len() <= pad {
                return Err(FilterError::TooShort { need: pad, got: y.len() });
            }
            let ext = odd_ext(y, pad);
            let forward = causal(h, &ext, ext[0]);
            let mut rev: Vec<f64> = forward.into_iter().rev().collect();
            let start = rev[0];
            rev = causal(h, &rev, start);
            rev.reverse();
            Ok(rev[pad..rev.len() - pad].to_vec())
        } else {
            let pad = numtaps - 1;
            if y.len() <= pad {
                return Err(FilterError::TooShort { need: pad, got: y.len() });
            }
            let ext = odd_ext(y, pad);
            let delay = pad / 2;
            // out[i] = sum_k h[k] ext[i + pad + delay - k]
            Ok((0..y.len()).map(|i| h.iter().enumerate().map(|(k, hk)| hk * ext[i + pad + delay - k]).sum()).collect())
        }
    }
}

/// `y[n] = sum_k h[k] x[n - k]` with the signal before the start equal to `before`.
fn causal(h: &[f64], x: &[f64], before: f64) -> Vec<f64> {
    (0..x.len())
        .map(|n| h.iter().enumerate().map(|(k, hk)| hk * if n >= k { x[n - k] } else { before }).sum())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Windows (symmetric, `scipy.signal.get_window(..., fftbins=False)`).

fn bessel_i0(x: f64) -> f64 {
    let (mut term, mut sum) = (1.0, 1.0);
    let q = x * x / 4.0;
    for k in 1..500 {
        term *= q / (k as f64 * k as f64);
        sum += term;
        if term < 1e-17 * sum {
            break;
        }
    }
    sum
}

fn cosine_sum(m: usize, a: &[f64]) -> Vec<f64> {
    (0..m)
        .map(|i| {
            let fac = -PI + 2.0 * PI * i as f64 / (m as f64 - 1.0);
            a.iter().enumerate().map(|(k, ak)| ak * (k as f64 * fac).cos()).sum()
        })
        .collect()
}

/// The `m`-point symmetric window; `param` is the window's parameter (see [`FirWindow::param_label`]).
pub fn window(kind: FirWindow, m: usize, param: Option<f64>) -> Result<Vec<f64>, FilterError> {
    if m == 0 {
        return Err(FilterError::BadOrder("a window needs at least one point".into()));
    }
    if m == 1 {
        return Ok(vec![1.0]);
    }
    let p = param.unwrap_or_else(|| kind.default_param());
    let mf = m as f64;
    let idx = || (0..m).map(|i| i as f64);
    Ok(match kind {
        FirWindow::Rectangle => vec![1.0; m],
        FirWindow::Bartlett => idx()
            .map(|n| if n <= (mf - 1.0) / 2.0 { 2.0 * n / (mf - 1.0) } else { 2.0 - 2.0 * n / (mf - 1.0) })
            .collect(),
        FirWindow::BartlettHanning => idx()
            .map(|n| {
                let fac = (n / (mf - 1.0) - 0.5).abs();
                0.62 - 0.48 * fac + 0.38 * (2.0 * PI * fac).cos()
            })
            .collect(),
        FirWindow::Blackman => cosine_sum(m, &[0.42, 0.50, 0.08]),
        FirWindow::BlackmanHarris => cosine_sum(m, &[0.35875, 0.48829, 0.14128, 0.01168]),
        FirWindow::Nuttall => cosine_sum(m, &[0.3635819, 0.4891775, 0.1365995, 0.0106411]),
        FirWindow::FlatTop => cosine_sum(m, &[0.21557895, 0.41663158, 0.277263158, 0.083578947, 0.006947368]),
        FirWindow::Hamming => cosine_sum(m, &[0.54, 0.46]),
        FirWindow::Hanning => cosine_sum(m, &[0.5, 0.5]),
        FirWindow::Bohman => {
            let mut w = vec![0.0; m];
            for (i, wi) in w.iter_mut().enumerate().take(m - 1).skip(1) {
                let fac = (-1.0 + 2.0 * i as f64 / (mf - 1.0)).abs();
                *wi = (1.0 - fac) * (PI * fac).cos() + (PI * fac).sin() / PI;
            }
            w
        }
        FirWindow::Gaussian => {
            if !(p.is_finite() && p > 0.0) {
                return Err(FilterError::BadMagnitude("the Gaussian window needs an alpha above 0".into()));
            }
            // alpha is the reciprocal of the standard deviation in units of half the width
            let sigma = (mf - 1.0) / (2.0 * p);
            idx().map(|n| (-(n - (mf - 1.0) / 2.0).powi(2) / (2.0 * sigma * sigma)).exp()).collect()
        }
        FirWindow::Kaiser => {
            if !(p.is_finite() && p >= 0.0) {
                return Err(FilterError::BadMagnitude("the Kaiser window needs a beta of 0 or more".into()));
            }
            let alpha = (mf - 1.0) / 2.0;
            idx().map(|n| bessel_i0(p * (1.0 - ((n - alpha) / alpha).powi(2)).max(0.0).sqrt()) / bessel_i0(p)).collect()
        }
        FirWindow::Parzen => {
            let q = (mf - 1.0) / 4.0;
            idx()
                .map(|i| {
                    let n = i - (mf - 1.0) / 2.0;
                    let r = n.abs() / (mf / 2.0);
                    if n.abs() <= q { 1.0 - 6.0 * r * r + 6.0 * r * r * r } else { 2.0 * (1.0 - r).powi(3) }
                })
                .collect()
        }
        FirWindow::Triangular => {
            let half = m.div_ceil(2);
            let first: Vec<f64> = (1..=half)
                .map(|n| if m.is_multiple_of(2) { (2.0 * n as f64 - 1.0) / mf } else { 2.0 * n as f64 / (mf + 1.0) })
                .collect();
            let mut w = first.clone();
            let skip = if m.is_multiple_of(2) { 0 } else { 1 };
            w.extend(first.iter().rev().skip(skip));
            w
        }
        FirWindow::Tukey => {
            if p <= 0.0 {
                vec![1.0; m]
            } else if p >= 1.0 {
                cosine_sum(m, &[0.5, 0.5])
            } else {
                let width = (p * (mf - 1.0) / 2.0).floor() as usize;
                (0..m)
                    .map(|n| {
                        let nf = n as f64;
                        if n <= width {
                            0.5 * (1.0 + (PI * (-1.0 + 2.0 * nf / p / (mf - 1.0))).cos())
                        } else if n >= m - width - 1 {
                            0.5 * (1.0 + (PI * (-2.0 / p + 1.0 + 2.0 * nf / p / (mf - 1.0))).cos())
                        } else {
                            1.0
                        }
                    })
                    .collect()
            }
        }
        FirWindow::Chebyshev => {
            if !(p.is_finite() && p > 0.0) {
                return Err(FilterError::BadMagnitude(
                    "the Chebyshev window needs a sidelobe attenuation above 0 dB".into(),
                ));
            }
            let order = mf - 1.0;
            let beta = ((1.0 / order) * (10f64.powf(p.abs() / 20.0)).acosh()).cosh();
            let pk: Vec<f64> = (0..m)
                .map(|k| {
                    let x = beta * (PI * k as f64 / mf).cos();
                    if x > 1.0 {
                        (order * x.acosh()).cosh()
                    } else if x < -1.0 {
                        (2.0 * (m % 2) as f64 - 1.0) * (order * (-x).acosh()).cosh()
                    } else {
                        (order * x.acos()).cos()
                    }
                })
                .collect();
            let mut data: Vec<Complex<f64>> = if m % 2 == 1 {
                pk.iter().map(|&v| Complex::new(v, 0.0)).collect()
            } else {
                pk.iter().enumerate().map(|(k, &v)| Complex::from_polar(v, PI / mf * k as f64)).collect()
            };
            FftPlanner::new().plan_fft_forward(m).process(&mut data);
            let w: Vec<f64> = data.iter().map(|c| c.re).collect();
            let w: Vec<f64> = if m % 2 == 1 {
                let n = m.div_ceil(2);
                let mut out: Vec<f64> = w[1..n].iter().rev().copied().collect();
                out.extend_from_slice(&w[..n]);
                out
            } else {
                let n = m / 2 + 1;
                let mut out: Vec<f64> = w[1..n].iter().rev().copied().collect();
                out.extend_from_slice(&w[1..n]);
                out
            };
            let top = w.iter().copied().fold(f64::MIN, f64::max);
            w.iter().map(|v| v / top).collect()
        }
        FirWindow::Taylor => {
            if !(p.is_finite() && p > 0.0) {
                return Err(FilterError::BadMagnitude("the Taylor window needs a sidelobe level above 0 dB".into()));
            }
            let nbar = 4usize;
            let b = 10f64.powf(p / 20.0);
            let a = b.acosh() / PI;
            let nb = nbar as f64;
            let s2 = nb * nb / (a * a + (nb - 0.5).powi(2));
            let fm: Vec<f64> = (1..nbar)
                .map(|mi| {
                    let m2 = (mi * mi) as f64;
                    let sign = if mi % 2 == 1 { 1.0 } else { -1.0 };
                    let numer =
                        sign * (1..nbar).map(|j| 1.0 - m2 / s2 / (a * a + (j as f64 - 0.5).powi(2))).product::<f64>();
                    let denom =
                        2.0 * (1..nbar).filter(|&j| j != mi).map(|j| 1.0 - m2 / (j * j) as f64).product::<f64>();
                    numer / denom
                })
                .collect();
            let w = |n: f64| {
                1.0 + 2.0
                    * fm.iter()
                        .enumerate()
                        .map(|(i, f)| f * (2.0 * PI * (i + 1) as f64 * (n - mf / 2.0 + 0.5) / mf).cos())
                        .sum::<f64>()
            };
            let scale = 1.0 / w((mf - 1.0) / 2.0);
            idx().map(|n| w(n) * scale).collect()
        }
    })
}

// ---------------------------------------------------------------------------------------------
// Design.

fn check_taps(response: FilterResponse, numtaps: usize) -> Result<(), FilterError> {
    if !(2..=MAX_FIR_ORDER + 1).contains(&numtaps) {
        return Err(FilterError::BadOrder(format!(
            "the filter order must be from 1 to {MAX_FIR_ORDER}, not {}",
            numtaps as i64 - 1
        )));
    }
    if matches!(response, FilterResponse::HighPass | FilterResponse::BandStop) && numtaps.is_multiple_of(2) {
        return Err(FilterError::BadOrder(format!(
            "a {} filter needs an even order (the gain at the Nyquist frequency), not {}",
            response.label(),
            numtaps - 1
        )));
    }
    Ok(())
}

/// The frequency at which Scale Passband sets the gain to 1.
fn scale_frequency(response: FilterResponse, edges: &[f64]) -> f64 {
    match response {
        FilterResponse::LowPass | FilterResponse::BandStop => 0.0,
        FilterResponse::HighPass => 1.0,
        FilterResponse::BandPass => 0.5 * (edges[0] + edges[1]),
    }
}

fn scale_to_unit(h: &mut [f64], w: f64) -> Result<(), FilterError> {
    let alpha = 0.5 * (h.len() as f64 - 1.0);
    let s: f64 = h.iter().enumerate().map(|(n, v)| v * (PI * w * (n as f64 - alpha)).cos()).sum();
    if !(s.is_finite() && s.abs() > 1e-300) {
        return Err(FilterError::NotFinite);
    }
    h.iter_mut().for_each(|v| *v /= s);
    Ok(())
}

/// Window-method design of `numtaps` coefficients (`scipy.signal.firwin`). `cutoff` is one
/// (low pass, high pass) or two (band pass, band stop) normalized frequencies.
pub fn firwin(
    numtaps: usize,
    cutoff: &[f64],
    response: FilterResponse,
    win: FirWindow,
    param: Option<f64>,
    scale: bool,
) -> Result<Vec<f64>, FilterError> {
    check_taps(response, numtaps)?;
    let pass_zero = matches!(response, FilterResponse::LowPass | FilterResponse::BandStop);
    let pass_nyquist = (cutoff.len() % 2 == 1) ^ pass_zero;
    let mut edges: Vec<f64> = Vec::new();
    if pass_zero {
        edges.push(0.0);
    }
    edges.extend_from_slice(cutoff);
    if pass_nyquist {
        edges.push(1.0);
    }
    let alpha = 0.5 * (numtaps as f64 - 1.0);
    let sinc = |x: f64| if x == 0.0 { 1.0 } else { (PI * x).sin() / (PI * x) };
    let mut h: Vec<f64> = (0..numtaps)
        .map(|n| {
            let m = n as f64 - alpha;
            edges.chunks(2).map(|b| b[1] * sinc(b[1] * m) - b[0] * sinc(b[0] * m)).sum()
        })
        .collect();
    let w = window(win, numtaps, param)?;
    for (v, wv) in h.iter_mut().zip(&w) {
        *v *= wv;
    }
    if scale {
        let (left, right) = (edges[0], edges[1]);
        let sf = if left == 0.0 {
            0.0
        } else if right == 1.0 {
            1.0
        } else {
            0.5 * (left + right)
        };
        scale_to_unit(&mut h, sf)?;
    }
    Ok(h)
}

/// Least squares design (`scipy.signal.firls` for an odd `numtaps`, extended to even): `bands`
/// are normalized `(low, high)` pairs, with a constant `desired` gain and a `weight` each.
pub fn firls(numtaps: usize, bands: &[(f64, f64)], desired: &[f64], weight: &[f64]) -> Result<Vec<f64>, FilterError> {
    let odd = numtaps % 2 == 1;
    let nb = if odd { (numtaps - 1) / 2 + 1 } else { numtaps / 2 };
    let nu: Vec<f64> = (0..nb).map(|i| if odd { i as f64 } else { i as f64 + 0.5 }).collect();
    let integral = |alpha: f64, a: f64, b: f64| {
        if alpha.abs() < 1e-14 { b - a } else { ((alpha * b).sin() - (alpha * a).sin()) / alpha }
    };
    let mut q = DMatrix::<f64>::zeros(nb, nb);
    let mut rhs = DVector::<f64>::zeros(nb);
    for (bi, &(lo, hi)) in bands.iter().enumerate() {
        let (a, b) = (PI * lo, PI * hi);
        for m in 0..nb {
            for n in 0..nb {
                q[(m, n)] += weight[bi] * 0.5 * (integral(nu[m] - nu[n], a, b) + integral(nu[m] + nu[n], a, b));
            }
            rhs[m] += weight[bi] * desired[bi] * integral(nu[m], a, b);
        }
    }
    let sol = q.lu().solve(&rhs).ok_or(FilterError::NotFinite)?;
    let mut h = vec![0.0; numtaps];
    if odd {
        let mid = (numtaps - 1) / 2;
        h[mid] = sol[0];
        for m in 1..nb {
            h[mid - m] = sol[m] / 2.0;
            h[mid + m] = sol[m] / 2.0;
        }
    } else {
        let half = numtaps / 2;
        for m in 0..nb {
            h[half - 1 - m] = sol[m] / 2.0;
            h[half + m] = sol[m] / 2.0;
        }
    }
    if h.iter().any(|v| !v.is_finite()) {
        return Err(FilterError::NotFinite);
    }
    Ok(h)
}

/// Kaiser's beta for a stopband attenuation in dB (`scipy.signal.kaiser_beta`).
pub fn kaiser_beta(a: f64) -> f64 {
    if a > 50.0 {
        0.1102 * (a - 8.7)
    } else if a > 21.0 {
        0.5842 * (a - 21.0).powf(0.4) + 0.07886 * (a - 21.0)
    } else {
        0.0
    }
}

/// The number of coefficients and the Kaiser beta for a ripple in dB and a transition width
/// (normalized, Nyquist = 1) (`scipy.signal.kaiserord`).
pub fn kaiserord(ripple_db: f64, width: f64) -> Result<(usize, f64), FilterError> {
    let a = ripple_db.abs();
    if a.is_nan() || a < 8.0 {
        return Err(FilterError::BadMagnitude(format!(
            "the deviation of {a:.3} dB is too small for the Kaiser order formula (8 dB or more)"
        )));
    }
    let numtaps = (a - 7.95) / 2.285 / (PI * width) + 1.0;
    if !numtaps.is_finite() || numtaps > (MAX_FIR_ORDER + 1) as f64 {
        return Err(FilterError::BadOrder(format!(
            "the minimum order is about {}, above the largest ({MAX_FIR_ORDER})",
            if numtaps.is_finite() { format!("{:.0}", numtaps - 1.0) } else { "infinite".into() }
        )));
    }
    Ok((numtaps.ceil() as usize, kaiser_beta(a)))
}

impl FirSpec {
    /// Designs the filter. `auto_fs` is the sample frequency from the X column, used when the
    /// unit is Hz and no sample frequency is given.
    pub fn design(&self, auto_fs: Option<f64>) -> Result<FirFilter, FilterError> {
        let r = self.response;
        match (self.method, self.order) {
            (FirMethod::LeastSquares, None) => Err(FilterError::BadOrder(
                "the Least Squares method needs a filter order (Minimum is for the Window method)".into(),
            )),
            (FirMethod::LeastSquares, Some(order)) => {
                let ps = pass_stop(
                    r,
                    self.unit,
                    self.sample_frequency,
                    auto_fs,
                    [self.pass, self.pass2],
                    [self.stop, self.stop2],
                )?;
                let ok = match r {
                    FilterResponse::LowPass => ps.wp[0] < ps.ws[0],
                    FilterResponse::HighPass => ps.ws[0] < ps.wp[0],
                    FilterResponse::BandPass => ps.ws[0] < ps.wp[0] && ps.wp[0] < ps.wp[1] && ps.wp[1] < ps.ws[1],
                    FilterResponse::BandStop => ps.wp[0] < ps.ws[0] && ps.ws[0] < ps.ws[1] && ps.ws[1] < ps.wp[1],
                };
                if !ok {
                    return Err(FilterError::BadFrequency(
                        "the pass and stop frequencies are out of order for this response type".into(),
                    ));
                }
                if !(self.pass_weight > 0.0
                    && self.stop_weight > 0.0
                    && self.pass_weight.is_finite()
                    && self.stop_weight.is_finite())
                {
                    return Err(FilterError::BadMagnitude("the passband and stopband weights must be above 0".into()));
                }
                let numtaps = order + 1;
                check_taps(r, numtaps)?;
                let (wp, ws) = (&ps.wp, &ps.ws);
                let (pw, sw) = (self.pass_weight, self.stop_weight);
                let (bands, desired, weight): (Vec<(f64, f64)>, Vec<f64>, Vec<f64>) = match r {
                    FilterResponse::LowPass => (vec![(0.0, wp[0]), (ws[0], 1.0)], vec![1.0, 0.0], vec![pw, sw]),
                    FilterResponse::HighPass => (vec![(0.0, ws[0]), (wp[0], 1.0)], vec![0.0, 1.0], vec![sw, pw]),
                    FilterResponse::BandPass => {
                        (vec![(0.0, ws[0]), (wp[0], wp[1]), (ws[1], 1.0)], vec![0.0, 1.0, 0.0], vec![sw, pw, sw])
                    }
                    FilterResponse::BandStop => {
                        (vec![(0.0, wp[0]), (ws[0], ws[1]), (wp[1], 1.0)], vec![1.0, 0.0, 1.0], vec![pw, sw, pw])
                    }
                };
                let mut h = firls(numtaps, &bands, &desired, &weight)?;
                if self.scale_passband {
                    scale_to_unit(&mut h, scale_frequency(r, wp))?;
                }
                let mut edges = wp.clone();
                edges.extend(ws.iter());
                Ok(FirFilter { coefficients: h, edges })
            }
            (FirMethod::Window, Some(order)) => {
                let wn = cutoffs(r, self.unit, self.sample_frequency, auto_fs, self.cutoff, self.cutoff2)?;
                if wn.len() == 2 && wn[0] >= wn[1] {
                    return Err(FilterError::BadFrequency(
                        "the first cutoff frequency must be below the second".into(),
                    ));
                }
                let h = firwin(order + 1, &wn, r, self.window, self.window_param, self.scale_passband)?;
                Ok(FirFilter { coefficients: h, edges: wn })
            }
            (FirMethod::Window, None) => {
                let ps = pass_stop(
                    r,
                    self.unit,
                    self.sample_frequency,
                    auto_fs,
                    [self.pass, self.pass2],
                    [self.stop, self.stop2],
                )?;
                let ok = match r {
                    FilterResponse::LowPass => ps.wp[0] < ps.ws[0],
                    FilterResponse::HighPass => ps.ws[0] < ps.wp[0],
                    FilterResponse::BandPass => ps.ws[0] < ps.wp[0] && ps.wp[0] < ps.wp[1] && ps.wp[1] < ps.ws[1],
                    FilterResponse::BandStop => ps.wp[0] < ps.ws[0] && ps.ws[0] < ps.ws[1] && ps.ws[1] < ps.wp[1],
                };
                if !ok {
                    return Err(FilterError::BadFrequency(
                        "the pass and stop frequencies are out of order for this response type".into(),
                    ));
                }
                let (dev_p, dev_s) = match self.magnitude_unit {
                    MagnitudeUnit::Db => {
                        if !(self.passband_ripple > 0.0 && self.stopband_attenuation > 0.0) {
                            return Err(FilterError::BadMagnitude(
                                "the passband ripple and the stopband attenuation must be above 0 dB".into(),
                            ));
                        }
                        let g = 10f64.powf(self.passband_ripple / 20.0);
                        ((g - 1.0) / (g + 1.0), 10f64.powf(-self.stopband_attenuation / 20.0))
                    }
                    MagnitudeUnit::Linear => {
                        if !(self.passband_ripple > 0.0
                            && self.passband_ripple < 1.0
                            && self.stopband_attenuation > 0.0
                            && self.stopband_attenuation < 1.0)
                        {
                            return Err(FilterError::BadMagnitude(
                                "the linear passband ripple and stopband attenuation must be between 0 and 1".into(),
                            ));
                        }
                        (self.passband_ripple, self.stopband_attenuation)
                    }
                };
                let a = -20.0 * dev_p.min(dev_s).log10();
                let cut: Vec<f64> = ps.wp.iter().zip(&ps.ws).map(|(p, s)| 0.5 * (p + s)).collect();
                let width = ps.wp.iter().zip(&ps.ws).map(|(p, s)| (p - s).abs()).fold(f64::INFINITY, f64::min);
                let (mut numtaps, beta) = kaiserord(a, width)?;
                if matches!(r, FilterResponse::HighPass | FilterResponse::BandStop) && numtaps.is_multiple_of(2) {
                    numtaps += 1;
                }
                let param = if self.window == FirWindow::Kaiser { Some(beta) } else { self.window_param };
                let h = firwin(numtaps, &cut, r, self.window, param, self.scale_passband)?;
                Ok(FirFilter { coefficients: h, edges: cut })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_are_symmetric_and_peak_at_one() {
        for kind in FirWindow::ALL {
            for m in [8usize, 9, 31, 32] {
                let w = window(kind, m, None).unwrap();
                assert_eq!(w.len(), m);
                for i in 0..m {
                    assert!((w[i] - w[m - 1 - i]).abs() < 1e-9, "{kind:?} {m} not symmetric at {i}");
                }
                assert!(w.iter().all(|v| v.is_finite()), "{kind:?}");
            }
        }
        let rect = window(FirWindow::Rectangle, 5, None).unwrap();
        assert_eq!(rect, vec![1.0; 5]);
        let hann = window(FirWindow::Hanning, 5, None).unwrap();
        assert!((hann[2] - 1.0).abs() < 1e-15 && hann[0].abs() < 1e-15);
    }

    #[test]
    fn low_pass_window_design_has_unit_dc_gain_and_stops_high_frequencies() {
        let spec = FirSpec { order: Some(40), cutoff: Some(0.3), unit: FreqUnit::Normalized, ..FirSpec::default() };
        let f = spec.design(None).unwrap();
        assert_eq!(f.coefficients.len(), 41);
        assert!((f.magnitude(0.0) - 1.0).abs() < 1e-12);
        assert!(f.magnitude(0.8) < 0.01);
    }

    #[test]
    fn high_pass_and_band_stop_need_an_even_order() {
        let hp = FirSpec {
            response: FilterResponse::HighPass,
            order: Some(21),
            unit: FreqUnit::Normalized,
            cutoff: Some(0.4),
            ..FirSpec::default()
        };
        assert!(hp.design(None).unwrap_err().to_string().contains("even order"));
        let ok = FirSpec { order: Some(20), ..hp };
        let f = ok.design(None).unwrap();
        assert!((f.magnitude(1.0) - 1.0).abs() < 1e-9);
        assert!(f.magnitude(0.0) < 0.01);
    }

    #[test]
    fn least_squares_meets_a_simple_specification() {
        let spec = FirSpec {
            method: FirMethod::LeastSquares,
            order: Some(60),
            unit: FreqUnit::Normalized,
            pass: Some(0.2),
            stop: Some(0.3),
            ..FirSpec::default()
        };
        let f = spec.design(None).unwrap();
        assert!((f.magnitude(0.1) - 1.0).abs() < 0.01);
        assert!(f.magnitude(0.5) < 0.01);
        let odd = FirSpec { order: Some(61), ..spec };
        assert!((odd.design(None).unwrap().magnitude(0.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn minimum_order_uses_the_kaiser_formula() {
        let spec = FirSpec {
            order: None,
            unit: FreqUnit::Normalized,
            pass: Some(0.2),
            stop: Some(0.3),
            stopband_attenuation: 60.0,
            passband_ripple: 0.1,
            ..FirSpec::default()
        };
        let f = spec.design(None).unwrap();
        assert!(f.order() > 10);
        assert!(f.magnitude(0.3) < 10f64.powf(-55.0 / 20.0), "{}", f.magnitude(0.3));
        assert!((f.magnitude(0.2) - 1.0).abs() < 0.02);
    }

    #[test]
    fn the_one_way_output_is_aligned_with_the_input() {
        let spec = FirSpec { order: Some(30), cutoff: Some(0.3), unit: FreqUnit::Normalized, ..FirSpec::default() };
        let f = spec.design(None).unwrap();
        let x: Vec<f64> = (0..200).map(|i| (2.0 * PI * 0.05 * i as f64).sin()).collect();
        let y = f.apply(&x, false).unwrap();
        for i in 50..150 {
            assert!((y[i] - x[i]).abs() < 0.02, "{i}: {} vs {}", y[i], x[i]);
        }
        let z = f.apply(&x, true).unwrap();
        for i in 100..150 {
            assert!((z[i] - x[i]).abs() < 0.02);
        }
        assert!(f.apply(&x[..80], true).is_err());
    }
}
