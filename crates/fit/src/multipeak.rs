//! Sum-of-peaks model for the multi-peak fit: several Gauss or Lorentz peaks on a constant or
//! sloped baseline, with starting values given per peak.
//!
//! The peak shapes use Origin's `Gauss` and `Lorentz` parameterisation (centre `xc`, width `w`,
//! area `A`), so a single peak fitted here agrees with the library functions of the same name:
//!
//! - Gauss: `A / (w sqrt(pi/2)) * exp(-2 ((x - xc) / w)^2)`, FWHM `w sqrt(ln 4)`;
//! - Lorentz: `2 A / pi * w / (4 (x - xc)^2 + w^2)`, FWHM `w`.
//!
//! Parameters are named `y0`, (`m` for a sloped baseline), then `xc1 w1 A1 xc2 w2 A2 ...`.

use std::f64::consts::PI;
use std::sync::Arc;

use crate::nlsf::FitFunction;

/// The shape of every peak of the sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeakShape {
    Gauss,
    Lorentz,
}

/// The baseline under the peaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitBaseline {
    Constant,
    Linear,
}

/// Starting values of one peak.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakStart {
    pub center: f64,
    pub fwhm: f64,
    pub height: f64,
}

impl PeakShape {
    pub fn name(self) -> &'static str {
        match self {
            PeakShape::Gauss => "Gauss",
            PeakShape::Lorentz => "Lorentz",
        }
    }

    /// Value of the peak (without baseline) at `x`.
    pub fn value(self, xc: f64, w: f64, area: f64, x: f64) -> f64 {
        match self {
            PeakShape::Gauss => area / (w * (PI / 2.0).sqrt()) * (-2.0 * ((x - xc) / w).powi(2)).exp(),
            PeakShape::Lorentz => 2.0 * area / PI * w / (4.0 * (x - xc).powi(2) + w * w),
        }
    }

    pub fn fwhm(self, w: f64) -> f64 {
        match self {
            PeakShape::Gauss => w * 4.0_f64.ln().sqrt(),
            PeakShape::Lorentz => w,
        }
    }

    pub fn height(self, w: f64, area: f64) -> f64 {
        self.value(0.0, w, area, 0.0)
    }

    /// The width parameter `w` that gives the FWHM `fwhm`.
    pub fn width_from_fwhm(self, fwhm: f64) -> f64 {
        match self {
            PeakShape::Gauss => fwhm / 4.0_f64.ln().sqrt(),
            PeakShape::Lorentz => fwhm,
        }
    }

    /// The area parameter of a peak with the given height and width parameter.
    pub fn area_from_height(self, w: f64, height: f64) -> f64 {
        match self {
            PeakShape::Gauss => height * w * (PI / 2.0).sqrt(),
            PeakShape::Lorentz => height * PI * w / 2.0,
        }
    }
}

/// Builds the fit function for `starts.len()` peaks. Widths are bounded to be positive.
pub fn multi_peak_function(shape: PeakShape, baseline: FitBaseline, starts: &[PeakStart]) -> FitFunction {
    let sloped = baseline == FitBaseline::Linear;
    let head = if sloped { 2 } else { 1 };
    let mut names: Vec<String> = vec!["y0".to_string()];
    if sloped {
        names.push("m".to_string());
    }
    for i in 1..=starts.len() {
        names.extend([format!("xc{i}"), format!("w{i}"), format!("A{i}")]);
    }
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let n = starts.len();
    let slope = if sloped { " + m*x" } else { "" };
    let equation = match shape {
        PeakShape::Gauss => {
            format!("y = y0{slope} + sum of {n} Gauss peak(s), each A/(w sqrt(pi/2)) exp(-2 ((x-xc)/w)^2)")
        }
        PeakShape::Lorentz => {
            format!("y = y0{slope} + sum of {n} Lorentz peak(s), each 2 A/pi * w / (4 (x-xc)^2 + w^2)")
        }
    };
    let f = Arc::new(move |p: &[f64], x: f64| {
        let mut y = p[0] + if sloped { p[1] * x } else { 0.0 };
        for k in 0..n {
            let o = head + 3 * k;
            y += shape.value(p[o], p[o + 1], p[o + 2], x);
        }
        y
    });
    let mut func = FitFunction::from_closure(&format!("{} multi-peak", shape.name()), &equation, &refs, f);
    for (k, s) in starts.iter().enumerate() {
        let i = k + 1;
        let w = shape.width_from_fwhm(s.fwhm.abs().max(f64::MIN_POSITIVE));
        func = func
            .with_initial(&format!("xc{i}"), s.center)
            .with_initial(&format!("w{i}"), w)
            .with_initial(&format!("A{i}"), shape.area_from_height(w, s.height))
            .with_bounds(&format!("w{i}"), Some(w * 1e-4), None);
    }
    func
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlsf::{FitOptions, fit};

    #[test]
    fn height_and_fwhm_follow_the_shape() {
        let w = 2.0;
        let a = PeakShape::Gauss.area_from_height(w, 3.0);
        assert!((PeakShape::Gauss.height(w, a) - 3.0).abs() < 1e-12);
        let half = PeakShape::Gauss.value(PeakShape::Gauss.fwhm(w) / 2.0, w, a, 0.0);
        assert!((half - 1.5).abs() < 1e-12);
        let a = PeakShape::Lorentz.area_from_height(w, 3.0);
        let half = PeakShape::Lorentz.value(PeakShape::Lorentz.fwhm(w) / 2.0, w, a, 0.0);
        assert!((half - 1.5).abs() < 1e-12);
    }

    #[test]
    fn recovers_two_overlapping_gauss_peaks_on_a_slope() {
        let x: Vec<f64> = (0..400).map(|i| i as f64 * 0.05).collect();
        let starts =
            [PeakStart { center: 8.0, fwhm: 1.5, height: 1.0 }, PeakStart { center: 9.4, fwhm: 1.2, height: 0.7 }];
        let truth = multi_peak_function(PeakShape::Gauss, FitBaseline::Linear, &starts);
        let w1 = PeakShape::Gauss.width_from_fwhm(1.5);
        let w2 = PeakShape::Gauss.width_from_fwhm(1.2);
        let p = [
            0.1,
            0.01,
            8.0,
            w1,
            PeakShape::Gauss.area_from_height(w1, 1.0),
            9.4,
            w2,
            PeakShape::Gauss.area_from_height(w2, 0.7),
        ];
        let y: Vec<f64> = x.iter().map(|&x| truth.eval(&p, x)).collect();
        let guess = multi_peak_function(
            PeakShape::Gauss,
            FitBaseline::Linear,
            &[PeakStart { center: 7.8, fwhm: 1.0, height: 0.9 }, PeakStart { center: 9.6, fwhm: 1.0, height: 0.6 }],
        )
        .with_initial("y0", 0.0)
        .with_initial("m", 0.0);
        let r = fit(&guess, &x, &y, &FitOptions::default()).unwrap();
        assert!((r.param("xc1").unwrap().value - 8.0).abs() < 1e-5);
        assert!((r.param("xc2").unwrap().value - 9.4).abs() < 1e-5);
        assert!((r.param("m").unwrap().value - 0.01).abs() < 1e-6);
    }
}
