//! Sum-of-peaks model for the multi-peak fit: several Gauss, Lorentz or Voigt peaks on a constant
//! or sloped baseline, with starting values given per peak.
//!
//! The peak shapes use Origin's `Gauss`, `Lorentz` and `Voigt` parameterisation, so a single peak
//! fitted here agrees with the library functions of the same name:
//!
//! - Gauss: `A / (w sqrt(pi/2)) * exp(-2 ((x - xc) / w)^2)`, FWHM `w sqrt(ln 4)`;
//! - Lorentz: `2 A / pi * w / (4 (x - xc)^2 + w^2)`, FWHM `w`;
//! - Voigt: the convolution of a Gauss of FWHM `wG` and a Lorentz of FWHM `wL` with area `A`
//!   ([`crate::library::voigt`]); FWHM `0.5346 wL + sqrt(0.2166 wL^2 + wG^2)`, the derived
//!   parameter of Origin's `Voigt` (Olivero and Longbothum's approximation).
//!
//! Parameters are named `y0`, (`m` for a sloped baseline), then per peak `xc1 w1 A1 xc2 w2 A2 ...`
//! for Gauss and Lorentz, and `xc1 A1 wG1 wL1 ...` (Origin's order) for Voigt.

use std::f64::consts::PI;
use std::sync::Arc;

use crate::library::voigt;
use crate::nlsf::FitFunction;

/// The shape of every peak of the sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PeakShape {
    Gauss,
    Lorentz,
    /// Origin's `Voigt`: four parameters per peak (`xc A wG wL`).
    Voigt,
}

/// One fitted peak, from its parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakSummary {
    pub center: f64,
    pub height: f64,
    pub fwhm: f64,
    pub area: f64,
}

/// FWHM of a Voigt peak from its Gaussian and Lorentzian FWHM, as Origin's `Voigt` derives it.
pub fn voigt_fwhm(wg: f64, wl: f64) -> f64 {
    0.5346 * wl + (0.2166 * wl * wl + wg * wg).sqrt()
}

/// FWHM of a Voigt peak with `wG = wL = 1` (1.637...).
fn voigt_unit_fwhm() -> f64 {
    voigt_fwhm(1.0, 1.0)
}

/// The baseline under the peaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
            PeakShape::Voigt => "Voigt",
        }
    }

    /// Number of parameters of one peak.
    pub fn params_per_peak(self) -> usize {
        match self {
            PeakShape::Gauss | PeakShape::Lorentz => 3,
            PeakShape::Voigt => 4,
        }
    }

    /// Names of the parameters of peak `i` (1-based), in the order of the parameter vector.
    pub fn param_names(self, i: usize) -> Vec<String> {
        match self {
            PeakShape::Gauss | PeakShape::Lorentz => vec![format!("xc{i}"), format!("w{i}"), format!("A{i}")],
            PeakShape::Voigt => vec![format!("xc{i}"), format!("A{i}"), format!("wG{i}"), format!("wL{i}")],
        }
    }

    /// Value of one peak (without baseline) at `x`; `p` holds that peak's parameters in the order
    /// of [`PeakShape::param_names`].
    pub fn eval(self, p: &[f64], x: f64) -> f64 {
        match self {
            PeakShape::Gauss | PeakShape::Lorentz => self.value(p[0], p[1], p[2], x),
            PeakShape::Voigt => voigt(x, 0.0, p[0], p[1], p[2], p[3]),
        }
    }

    /// Centre, height, FWHM and area of one peak from its parameters.
    pub fn summary(self, p: &[f64]) -> PeakSummary {
        match self {
            PeakShape::Gauss | PeakShape::Lorentz => {
                PeakSummary { center: p[0], height: self.height(p[1], p[2]), fwhm: self.fwhm(p[1]), area: p[2] }
            }
            PeakShape::Voigt => {
                PeakSummary { center: p[0], height: self.eval(p, p[0]), fwhm: voigt_fwhm(p[2], p[3]), area: p[1] }
            }
        }
    }

    /// Starting parameters of one peak with the given centre, FWHM and height. A Voigt starts with
    /// equal Gaussian and Lorentzian widths, as Origin's `Voigt` initialisation does.
    fn initial(self, start: &PeakStart) -> Vec<f64> {
        let fwhm = start.fwhm.abs().max(f64::MIN_POSITIVE);
        match self {
            PeakShape::Gauss | PeakShape::Lorentz => {
                let w = self.width_from_fwhm(fwhm);
                vec![start.center, w, self.area_from_height(w, start.height)]
            }
            PeakShape::Voigt => {
                let w = fwhm / voigt_unit_fwhm();
                let unit = voigt(0.0, 0.0, 0.0, 1.0, w, w);
                vec![start.center, start.height / unit, w, w]
            }
        }
    }

    /// Value of a Gauss or Lorentz peak (without baseline) at `x`, from its centre, width
    /// parameter and area. A Voigt has two widths: use [`PeakShape::eval`].
    pub fn value(self, xc: f64, w: f64, area: f64, x: f64) -> f64 {
        match self {
            PeakShape::Gauss => area / (w * (PI / 2.0).sqrt()) * (-2.0 * ((x - xc) / w).powi(2)).exp(),
            PeakShape::Lorentz => 2.0 * area / PI * w / (4.0 * (x - xc).powi(2) + w * w),
            PeakShape::Voigt => voigt(x, 0.0, xc, area, w, w),
        }
    }

    /// FWHM of a Gauss or Lorentz peak from its width parameter (a Voigt with `wG = wL = w`).
    pub fn fwhm(self, w: f64) -> f64 {
        match self {
            PeakShape::Gauss => w * 4.0_f64.ln().sqrt(),
            PeakShape::Lorentz => w,
            PeakShape::Voigt => voigt_fwhm(w, w),
        }
    }

    pub fn height(self, w: f64, area: f64) -> f64 {
        self.value(0.0, w, area, 0.0)
    }

    /// The width parameter `w` that gives the FWHM `fwhm` (a Voigt with `wG = wL = w`).
    pub fn width_from_fwhm(self, fwhm: f64) -> f64 {
        match self {
            PeakShape::Gauss => fwhm / 4.0_f64.ln().sqrt(),
            PeakShape::Lorentz => fwhm,
            PeakShape::Voigt => fwhm / voigt_unit_fwhm(),
        }
    }

    /// The area parameter of a peak with the given height and width parameter.
    pub fn area_from_height(self, w: f64, height: f64) -> f64 {
        match self {
            PeakShape::Gauss => height * w * (PI / 2.0).sqrt(),
            PeakShape::Lorentz => height * PI * w / 2.0,
            PeakShape::Voigt => height / voigt(0.0, 0.0, 0.0, 1.0, w, w),
        }
    }
}

/// Builds the fit function for `starts.len()` peaks of one shape. Widths are bounded to be positive.
pub fn multi_peak_function(shape: PeakShape, baseline: FitBaseline, starts: &[PeakStart]) -> FitFunction {
    multi_peak_function_of(&vec![shape; starts.len()], baseline, starts)
}

/// Builds the fit function for `starts.len()` peaks, peak `k` of shape `shapes[k]` (a shorter list
/// repeats its last shape; an empty one means Gauss). Parameters are named per peak by its shape,
/// so `xc1 w1 A1 xc2 A2 wG2 wL2` for a Gauss and a Voigt.
pub fn multi_peak_function_of(shapes: &[PeakShape], baseline: FitBaseline, starts: &[PeakStart]) -> FitFunction {
    let shapes = shapes_for(shapes, starts.len());
    let sloped = baseline == FitBaseline::Linear;
    let head = if sloped { 2 } else { 1 };
    let mut names: Vec<String> = vec!["y0".to_string()];
    if sloped {
        names.push("m".to_string());
    }
    for (k, shape) in shapes.iter().enumerate() {
        names.extend(shape.param_names(k + 1));
    }
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let n = starts.len();
    let slope = if sloped { " + m*x" } else { "" };
    let same = shapes.windows(2).all(|w| w[0] == w[1]);
    let equation = match shapes.first() {
        Some(PeakShape::Gauss) if same => {
            format!("y = y0{slope} + sum of {n} Gauss peak(s), each A/(w sqrt(pi/2)) exp(-2 ((x-xc)/w)^2)")
        }
        Some(PeakShape::Lorentz) if same => {
            format!("y = y0{slope} + sum of {n} Lorentz peak(s), each 2 A/pi * w / (4 (x-xc)^2 + w^2)")
        }
        Some(PeakShape::Voigt) if same => format!(
            "y = y0{slope} + sum of {n} Voigt peak(s), each a Gauss of FWHM wG convolved with a Lorentz of FWHM wL, area A"
        ),
        _ => format!("y = y0{slope} + sum of {n} peaks: {}", shapes_name(&shapes, ", ")),
    };
    let layout: Vec<(PeakShape, usize)> = shapes
        .iter()
        .scan(head, |o, &shape| {
            let at = *o;
            *o += shape.params_per_peak();
            Some((shape, at))
        })
        .collect();
    let f = {
        let layout = layout.clone();
        Arc::new(move |p: &[f64], x: f64| {
            let mut y = p[0] + if sloped { p[1] * x } else { 0.0 };
            for &(shape, o) in &layout {
                y += shape.eval(&p[o..o + shape.params_per_peak()], x);
            }
            y
        })
    };
    let model = if same { shapes.first().map_or("Gauss", |s| s.name()).to_string() } else { "Mixed".to_string() };
    let mut func = FitFunction::from_closure(&format!("{model} multi-peak"), &equation, &refs, f);
    for (k, (s, shape)) in starts.iter().zip(&shapes).enumerate() {
        let i = k + 1;
        let initial = shape.initial(s);
        for (name, value) in shape.param_names(i).iter().zip(&initial) {
            func = func.with_initial(name, *value);
        }
        func = match shape {
            PeakShape::Gauss | PeakShape::Lorentz => func.with_bounds(&format!("w{i}"), Some(initial[1] * 1e-4), None),
            // A Lorentzian width of 0 is a pure Gauss; a negative one has no meaning.
            PeakShape::Voigt => func.with_bounds(&format!("wG{i}"), Some(initial[2] * 1e-4), None).with_bounds(
                &format!("wL{i}"),
                Some(0.0),
                None,
            ),
        };
    }
    func
}

/// The shape of each of `n` peaks: `shapes` with its last entry repeated (Gauss when empty).
pub fn shapes_for(shapes: &[PeakShape], n: usize) -> Vec<PeakShape> {
    let last = shapes.last().copied().unwrap_or(PeakShape::Gauss);
    (0..n).map(|k| shapes.get(k).copied().unwrap_or(last)).collect()
}

/// The shapes' names joined by `sep`, each once when they are all the same ("Voigt"), otherwise
/// one per peak ("Gauss+Voigt").
pub fn shapes_name(shapes: &[PeakShape], sep: &str) -> String {
    if shapes.windows(2).all(|w| w[0] == w[1]) {
        shapes.first().map_or("Gauss", |s| s.name()).to_string()
    } else {
        shapes.iter().map(|s| s.name()).collect::<Vec<_>>().join(sep)
    }
}

/// The centre, height, FWHM and area of every peak of a fitted multi-peak function, from its
/// parameter vector (`params` in the order of [`multi_peak_function`]'s names).
pub fn peak_summaries(shape: PeakShape, baseline: FitBaseline, params: &[f64]) -> Vec<PeakSummary> {
    let head = if baseline == FitBaseline::Linear { 2 } else { 1 };
    params.get(head..).unwrap_or(&[]).chunks_exact(shape.params_per_peak()).map(|p| shape.summary(p)).collect()
}

/// [`peak_summaries`] for peaks of the given shapes, one per peak (as [`multi_peak_function_of`]).
pub fn peak_summaries_of(shapes: &[PeakShape], baseline: FitBaseline, params: &[f64]) -> Vec<PeakSummary> {
    let mut o = if baseline == FitBaseline::Linear { 2 } else { 1 };
    let mut out = Vec::with_capacity(shapes.len());
    for shape in shapes {
        let Some(p) = params.get(o..o + shape.params_per_peak()) else { break };
        out.push(shape.summary(p));
        o += shape.params_per_peak();
    }
    out
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
