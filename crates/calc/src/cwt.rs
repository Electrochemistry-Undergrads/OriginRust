//! The continuous wavelet transform as Origin's Analysis > Signal Processing > Wavelet > Continuous
//! Wavelet (`cwt`) computes it (`docs/spec/analysis-tools.md` 19.6).
//!
//! The rule, found by fitting Origin's impulse and signal responses to 1e-14 (all three wavelet
//! types, every scale tried):
//!
//! * The mother wavelet is sampled at `N = 1024` evenly spaced points over `[-R, R]` (`R = 8` for
//!   Morlet and Mexican hat, `R = 5` for the derivative of Gaussian), `step = 2R / (N - 1)`, and its
//!   running integral `ip[i] = step * sum_{k <= i} psi(x_k)` is formed.
//! * For a scale `a` (cut to its integer part, `a >= 1`) the filter has `floor(2R a) + 1` taps,
//!   `f[i] = ip[floor(i / (a step))]`, reversed.
//! * The coefficients are `-sqrt(a)` times the first difference of the convolution of the signal
//!   with the filter, the middle `n` values (`floor((taps - 2) / 2)` skipped). The difference of an
//!   integrated wavelet is what puts every response half a sample after the impulse.
//! * Morlet: `exp(-x^2/2) cos(w x)`, `w` the wave number cut to an integer, 5 to 20. Mexican hat:
//!   `2 / (sqrt(3) pi^(1/4)) (1 - x^2) exp(-x^2/2)`. DGauss of order `m` (1 to 8): the `m`-th
//!   derivative of `exp(-x^2)` scaled to unit L2 norm, with the sign `-` for `m` = 1, 2 (mod 4)
//!   and `+` for `m` = 3, 0 (mod 4): `sigma_m H_m(x) exp(-x^2) / sqrt(sqrt(pi/2) (2m-1)!!)`, `H_m`
//!   the physicists' Hermite polynomial.
//!
//! Not reproduced (no script route in Origin): the pseudo-frequency output, the coefficient
//! matrix and the image plot. Origin writes the coefficient sheet only.

use std::f64::consts::PI;

/// Points of the sampled mother wavelet.
const GRID: usize = 1024;

/// The mother wavelet of Origin's `type` (0, 1, 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CwtWavelet {
    #[default]
    Morlet,
    /// Derivative of Gaussian.
    DGauss,
    /// Mexican hat.
    MexHat,
}

impl CwtWavelet {
    pub const ALL: [CwtWavelet; 3] = [CwtWavelet::Morlet, CwtWavelet::DGauss, CwtWavelet::MexHat];

    pub fn name(self) -> &'static str {
        match self {
            CwtWavelet::Morlet => "Morlet",
            CwtWavelet::DGauss => "DGauss",
            CwtWavelet::MexHat => "MexHat",
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        Self::ALL.get(i).copied()
    }

    /// `morlet`, `dgauss`, `mexhat` (any case), or the index `0`, `1`, `2`.
    pub fn parse(s: &str) -> Option<Self> {
        let t = s.trim().to_ascii_lowercase();
        if let Ok(i) = t.parse::<usize>() {
            return Self::from_index(i);
        }
        Self::ALL.into_iter().find(|w| w.name().eq_ignore_ascii_case(&t))
    }

    /// The allowed whole values of the parameter: Morlet wave numbers 5 to 20, DGauss orders 1 to 8,
    /// none for the Mexican hat.
    pub fn param_range(self) -> Option<(u32, u32)> {
        match self {
            CwtWavelet::Morlet => Some((5, 20)),
            CwtWavelet::DGauss => Some((1, 8)),
            CwtWavelet::MexHat => None,
        }
    }

    /// Half the support of the sampled wavelet.
    fn half_support(self) -> f64 {
        match self {
            CwtWavelet::DGauss => 5.0,
            _ => 8.0,
        }
    }
}

/// What `cwt` refuses.
#[derive(Debug, Clone, PartialEq)]
pub enum CwtError {
    /// No data.
    Empty,
    /// A missing or infinite value in the signal.
    NotFinite,
    /// A scale below 1 (or not a number): Origin refuses the whole run.
    ScaleBelowOne(f64),
    /// No scales given.
    NoScales,
    /// A scale above [`MAX_SCALE`]: the filter would be too long to compute.
    ScaleTooLarge(f64),
    /// The wave number or order is outside the allowed range: `(wavelet, value, low, high)`.
    BadParameter(CwtWavelet, f64, u32, u32),
}

impl std::fmt::Display for CwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CwtError::Empty => write!(f, "the signal is empty"),
            CwtError::NotFinite => write!(f, "the signal has a missing or infinite value"),
            CwtError::ScaleBelowOne(a) => write!(f, "scales must be at least 1, got {a}"),
            CwtError::ScaleTooLarge(a) => write!(f, "scales above {MAX_SCALE} are refused, got {a}"),
            CwtError::NoScales => write!(f, "there is no scale to transform with"),
            CwtError::BadParameter(w, v, lo, hi) => {
                let what = if *w == CwtWavelet::Morlet { "wave number" } else { "order" };
                write!(f, "the {} {what} must be between {lo} and {hi}, got {v}", w.name())
            }
        }
    }
}

impl std::error::Error for CwtError {}

/// The wave number or order as Origin uses it: the integer part, checked against the range.
fn whole_param(wavelet: CwtWavelet, param: f64) -> Result<u32, CwtError> {
    let Some((lo, hi)) = wavelet.param_range() else { return Ok(0) };
    let v = param.floor();
    if !(v >= f64::from(lo) && v <= f64::from(hi)) {
        return Err(CwtError::BadParameter(wavelet, param, lo, hi));
    }
    Ok(v as u32)
}

/// Physicists' Hermite polynomial `H_m(x)`.
fn hermite(m: u32, x: f64) -> f64 {
    let (mut a, mut b) = (1.0, 2.0 * x);
    if m == 0 {
        return a;
    }
    for k in 1..m {
        let c = 2.0 * x * b - 2.0 * f64::from(k) * a;
        a = b;
        b = c;
    }
    b
}

/// The mother wavelet `psi(x)`.
fn psi(wavelet: CwtWavelet, m: u32, norm: f64, x: f64) -> f64 {
    match wavelet {
        CwtWavelet::Morlet => (-x * x / 2.0).exp() * (f64::from(m) * x).cos(),
        CwtWavelet::MexHat => 2.0 / (3f64.sqrt() * PI.powf(0.25)) * (1.0 - x * x) * (-x * x / 2.0).exp(),
        CwtWavelet::DGauss => {
            let sign = if m % 4 == 1 || m % 4 == 2 { -1.0 } else { 1.0 };
            sign * hermite(m, x) * (-x * x).exp() / norm
        }
    }
}

/// `sqrt(sqrt(pi/2) (2m-1)!!)`, the L2 norm of the `m`-th derivative of `exp(-x^2)`.
fn dgauss_norm(m: u32) -> f64 {
    let double_factorial: f64 = (1..=m).map(|k| f64::from(2 * k - 1)).product();
    ((PI / 2.0).sqrt() * double_factorial).sqrt()
}

/// The integrated mother wavelet on its grid, and its grid step.
fn integrated(wavelet: CwtWavelet, m: u32) -> (Vec<f64>, f64) {
    let r = wavelet.half_support();
    let step = 2.0 * r / (GRID - 1) as f64;
    let norm = if wavelet == CwtWavelet::DGauss { dgauss_norm(m) } else { 1.0 };
    let mut sum = 0.0;
    let ip = (0..GRID)
        .map(|i| {
            sum += psi(wavelet, m, norm, -r + i as f64 * step);
            step * sum
        })
        .collect();
    (ip, step)
}

/// The largest scale accepted. Origin's upper limit was not measured; this bound only keeps the
/// filter (`16 a` taps) and the `O(n taps)` convolution from exhausting memory or time.
pub const MAX_SCALE: f64 = 10_000.0;

/// The coefficients of `signal` at the scales in `scales` (one vector of `signal.len()` values per
/// scale, in the order given). A scale is cut to its integer part, as Origin does; any scale below 1
/// refuses the whole run.
pub fn cwt(signal: &[f64], scales: &[f64], wavelet: CwtWavelet, param: f64) -> Result<Vec<Vec<f64>>, CwtError> {
    if signal.is_empty() {
        return Err(CwtError::Empty);
    }
    if signal.iter().any(|v| !v.is_finite()) {
        return Err(CwtError::NotFinite);
    }
    if scales.is_empty() {
        return Err(CwtError::NoScales);
    }
    if let Some(&a) = scales.iter().find(|a| !(**a >= 1.0 && a.is_finite())) {
        return Err(CwtError::ScaleBelowOne(a));
    }
    if let Some(&a) = scales.iter().find(|a| **a > MAX_SCALE) {
        return Err(CwtError::ScaleTooLarge(a));
    }
    let m = whole_param(wavelet, param)?;
    let (ip, step) = integrated(wavelet, m);
    let width = 2.0 * wavelet.half_support();
    let n = signal.len();
    Ok(scales
        .iter()
        .map(|&a| {
            let a = a.floor();
            let taps = (a * width).floor() as usize + 1;
            // f reversed: f[taps - 1 - i] = ip[floor(i / (a step))]
            let mut f = vec![0.0; taps];
            for i in 0..taps {
                let j = ((i as f64 / (a * step)).floor() as usize).min(GRID - 1);
                f[taps - 1 - i] = ip[j];
            }
            // g[j] = f[j] - f[j - 1], so that diff(conv(x, f))[k] = sum_i x[i] g[k + 1 - i]
            let g: Vec<f64> = (0..=taps)
                .map(|j| {
                    let hi = if j < taps { f[j] } else { 0.0 };
                    let lo = if j > 0 { f[j - 1] } else { 0.0 };
                    hi - lo
                })
                .collect();
            let skip = (taps - 2) / 2;
            let scale = -a.sqrt();
            (0..n)
                .map(|t| {
                    let k = skip + t;
                    // i runs over the samples with 0 <= k + 1 - i <= taps
                    let i_lo = (k + 1).saturating_sub(taps);
                    let i_hi = (k + 1).min(n - 1);
                    let mut s = 0.0;
                    for i in i_lo..=i_hi {
                        s += signal[i] * g[k + 1 - i];
                    }
                    scale * s
                })
                .collect()
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_response_is_centred_half_a_sample_after_the_impulse() {
        let mut x = vec![0.0; 64];
        x[32] = 1.0;
        let r = &cwt(&x, &[4.0], CwtWavelet::MexHat, 5.0).unwrap()[0];
        for d in 0..8 {
            assert!((r[32 + 1 + d] - r[32 - d]).abs() < 1e-12, "{d}");
        }
    }

    #[test]
    fn scales_and_parameters_are_cut_to_integers_and_checked() {
        let x: Vec<f64> = (0..40).map(|i| (i as f64 * 0.3).sin()).collect();
        let a = cwt(&x, &[2.9], CwtWavelet::Morlet, 5.9).unwrap();
        let b = cwt(&x, &[2.0], CwtWavelet::Morlet, 5.0).unwrap();
        assert_eq!(a, b);
        assert_eq!(cwt(&x, &[0.99], CwtWavelet::Morlet, 5.0), Err(CwtError::ScaleBelowOne(0.99)));
        assert!(cwt(&x, &[2.0], CwtWavelet::Morlet, 4.0).is_err());
        assert!(cwt(&x, &[2.0], CwtWavelet::Morlet, 21.0).is_err());
        assert!(cwt(&x, &[2.0], CwtWavelet::DGauss, 0.0).is_err());
        assert!(cwt(&x, &[2.0], CwtWavelet::DGauss, 9.0).is_err());
        assert!(cwt(&x, &[2.0], CwtWavelet::DGauss, 8.9).is_ok());
        assert_eq!(cwt(&x, &[1e12], CwtWavelet::Morlet, 5.0), Err(CwtError::ScaleTooLarge(1e12)));
        assert_eq!(cwt(&x, &[f64::MAX], CwtWavelet::Morlet, 5.0), Err(CwtError::ScaleTooLarge(f64::MAX)));
        assert_eq!(cwt(&[], &[2.0], CwtWavelet::MexHat, 5.0), Err(CwtError::Empty));
    }
}
