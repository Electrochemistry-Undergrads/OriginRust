//! Fourier self-deconvolution (FSD, Kauppinen, Moffatt, Mantsch and Cameron, Appl. Spectrosc. 35,
//! 271 (1981)): narrows the bands of a spectrum so that overlapping peaks separate. Sheaf's own tool
//! (T341): Origin 2016 has none, and Origin's later one is not documented.
//!
//! The straight line through the curve's first and last point is removed, the rest is Fourier
//! transformed, multiplied by `exp(2 pi g |t|)` (the inverse of the transform of a Lorentzian band of
//! half width `g` = FWHM / 2) and by an apodization that ends at `L`, transformed back, and the line is
//! added again. `L` is chosen so that the apodization alone makes a line `FWHM / K` wide: a Lorentzian
//! band of the given width comes out about `K` times narrower, with the same area.

use rustfft::{FftPlanner, num_complex::Complex};
use serde::{Deserialize, Serialize};

/// The apodization (window on the interferogram) that ends the deconvolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Apodization {
    /// None (a box): the narrowest lines, with the strongest side lobes.
    Boxcar,
    /// Triangle: `1 - s`.
    Triangle,
    /// Triangle squared: `(1 - s)^2`, the usual choice for FSD (small side lobes).
    #[default]
    TriangleSquared,
}

impl Apodization {
    pub const ALL: [Apodization; 3] = [Apodization::Boxcar, Apodization::Triangle, Apodization::TriangleSquared];

    /// The name in the window and the command line.
    pub fn label(self) -> &'static str {
        match self {
            Apodization::Boxcar => "Boxcar",
            Apodization::Triangle => "Triangle",
            Apodization::TriangleSquared => "Triangle squared",
        }
    }

    fn weight(self, s: f64) -> f64 {
        if !(0.0..1.0).contains(&s) {
            return 0.0;
        }
        match self {
            Apodization::Boxcar => 1.0,
            Apodization::Triangle => 1.0 - s,
            Apodization::TriangleSquared => (1.0 - s) * (1.0 - s),
        }
    }

    /// The FWHM of the line the apodization makes on its own, times its length `L` (the line is the
    /// cosine transform of the weight over 0..1; found numerically).
    fn width_factor(self) -> f64 {
        let n = 4000;
        let shape = |f: f64| -> f64 {
            (0..n)
                .map(|i| {
                    let s = (i as f64 + 0.5) / n as f64;
                    self.weight(s) * (2.0 * std::f64::consts::PI * f * s).cos()
                })
                .sum::<f64>()
                / n as f64
        };
        let top = shape(0.0);
        let (mut lo, mut hi) = (0.0, 2.0);
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if shape(mid) > top / 2.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        2.0 * lo
    }
}

/// The settings of [`fourier_self_deconvolution`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FsdOptions {
    /// Full width at half maximum of the bands to narrow, in X units (Lorentzian).
    pub fwhm: f64,
    /// Resolution enhancement factor K: how many times narrower the bands become (above 1).
    pub enhancement: f64,
    pub apodization: Apodization,
}

impl Default for FsdOptions {
    fn default() -> Self {
        FsdOptions { fwhm: 1.0, enhancement: 2.0, apodization: Apodization::TriangleSquared }
    }
}

/// Fourier self-deconvolution of `y` against evenly spaced `x` (see the module text). The result has
/// one value per point. Errors: fewer than 8 points, values that are not numbers, X not evenly
/// spaced, a width or factor out of range.
pub fn fourier_self_deconvolution(x: &[f64], y: &[f64], o: &FsdOptions) -> Result<Vec<f64>, String> {
    let n = x.len().min(y.len());
    if n < 8 {
        return Err("Fourier self-deconvolution needs at least 8 points".into());
    }
    if x[..n].iter().chain(&y[..n]).any(|v| !v.is_finite()) {
        return Err("Fourier self-deconvolution needs numbers in every row (no missing values)".into());
    }
    let h = (x[n - 1] - x[0]) / (n - 1) as f64;
    if h == 0.0 || x[..n].windows(2).any(|w| ((w[1] - w[0]) - h).abs() > 1e-6 * h.abs().max(1e-300) + 1e-9 * h.abs()) {
        return Err(
            "Fourier self-deconvolution needs evenly spaced X (Reduce to Evenly Spaced X or Interpolate first)".into(),
        );
    }
    if !(o.fwhm.is_finite() && o.fwhm > 0.0) {
        return Err("the band width (FWHM) must be above 0".into());
    }
    if !(o.enhancement.is_finite() && o.enhancement > 1.0 && o.enhancement <= 20.0) {
        return Err("the enhancement factor K must be above 1 (at most 20)".into());
    }
    let h = h.abs();
    // The line through the ends, removed so that the periodic transform has no jump.
    let (y0, y1) = (y[0], y[n - 1]);
    let line = |i: usize| y0 + (y1 - y0) * i as f64 / (n - 1) as f64;
    let m = (2 * n).next_power_of_two();
    let mut buf: Vec<Complex<f64>> =
        (0..m).map(|i| Complex::new(if i < n { y[i] - line(i) } else { 0.0 }, 0.0)).collect();
    let mut planner = FftPlanner::new();
    planner.plan_fft_forward(m).process(&mut buf);
    let g = o.fwhm / 2.0;
    let length = o.apodization.width_factor() * o.enhancement / o.fwhm;
    for (k, v) in buf.iter_mut().enumerate() {
        let t = k.min(m - k) as f64 / (m as f64 * h);
        let w = o.apodization.weight(t / length);
        *v *= if w == 0.0 { 0.0 } else { w * (2.0 * std::f64::consts::PI * g * t).exp() };
    }
    planner.plan_fft_inverse(m).process(&mut buf);
    Ok((0..n).map(|i| buf[i].re / m as f64 + line(i)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lorentz(x: f64, c: f64, fwhm: f64) -> f64 {
        let g = fwhm / 2.0;
        g * g / ((x - c) * (x - c) + g * g)
    }

    fn fwhm_of(x: &[f64], y: &[f64]) -> f64 {
        let (p, top) = y.iter().copied().enumerate().fold((0, f64::MIN), |a, (i, v)| if v > a.1 { (i, v) } else { a });
        let half = top / 2.0;
        let cross = |range: Box<dyn Iterator<Item = usize>>| -> f64 {
            let mut prev = p;
            for i in range {
                if y[i] < half {
                    let t = (half - y[i]) / (y[prev] - y[i]);
                    return x[i] + t * (x[prev] - x[i]);
                }
                prev = i;
            }
            f64::NAN
        };
        cross(Box::new(p + 1..y.len())) - cross(Box::new((0..p).rev()))
    }

    #[test]
    fn a_lorentzian_band_becomes_k_times_narrower_with_its_area() {
        let x: Vec<f64> = (0..1024).map(|i| i as f64 * 0.05).collect();
        let y: Vec<f64> = x.iter().map(|&v| 0.3 + 0.01 * v + lorentz(v, 25.0, 2.0)).collect();
        for apod in Apodization::ALL {
            let o = FsdOptions { fwhm: 2.0, enhancement: 2.5, apodization: apod };
            let d = fourier_self_deconvolution(&x, &y, &o).unwrap();
            let base: Vec<f64> = x.iter().zip(&d).map(|(&v, &b)| b - 0.3 - 0.01 * v).collect();
            let w = fwhm_of(&x, &base);
            assert!((w - 0.8).abs() < 0.08, "{apod:?}: width {w}");
            let area_in: f64 = x.iter().map(|&v| lorentz(v, 25.0, 2.0)).sum();
            let area_out: f64 = base.iter().sum();
            assert!((area_out / area_in - 1.0).abs() < 0.02, "{apod:?}: area {area_out} vs {area_in}");
            let top =
                base.iter().copied().enumerate().fold((0, f64::MIN), |a, (i, v)| if v > a.1 { (i, v) } else { a });
            assert_eq!(top.0, 500, "the centre stays at x = 25");
        }
    }

    #[test]
    fn two_overlapping_bands_separate() {
        let x: Vec<f64> = (0..1024).map(|i| i as f64 * 0.05).collect();
        let y: Vec<f64> = x.iter().map(|&v| lorentz(v, 24.5, 2.0) + lorentz(v, 25.5, 2.0)).collect();
        let maxima =
            |v: &[f64]| (1..v.len() - 1).filter(|&i| v[i] > v[i - 1] && v[i] >= v[i + 1] && v[i] > 0.3).count();
        assert_eq!(maxima(&y), 1, "the input shows one band");
        let d = fourier_self_deconvolution(&x, &y, &FsdOptions { fwhm: 2.0, enhancement: 3.0, ..Default::default() })
            .unwrap();
        assert_eq!(maxima(&d), 2, "two bands after FSD");
    }

    #[test]
    fn bad_input_is_explained() {
        let x: Vec<f64> = (0..20).map(f64::from).collect();
        let y = vec![1.0; 20];
        assert!(fourier_self_deconvolution(&x[..5], &y[..5], &FsdOptions::default()).is_err());
        let mut uneven = x.clone();
        uneven[7] = 7.5;
        assert!(fourier_self_deconvolution(&uneven, &y, &FsdOptions::default()).unwrap_err().contains("evenly spaced"));
        let bad_k = FsdOptions { enhancement: 1.0, ..Default::default() };
        assert!(fourier_self_deconvolution(&x, &y, &bad_k).is_err());
        let flat = fourier_self_deconvolution(&x, &y, &FsdOptions::default()).unwrap();
        assert!(flat.iter().all(|v| (v - 1.0).abs() < 1e-9), "a flat line stays flat");
    }
}
