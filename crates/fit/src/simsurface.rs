//! Simulate Surface: the Z values of a surface fit function on a grid of X and Y for typed parameters, with
//! optional noise (Analysis > Fitting > Simulate Surface; `docs/spec/fitting.md`, section 16, measured in Origin
//! 2016 on 2026-11-16).
//!
//! The values come from [`FitFunction::eval2`], the evaluator of the surface fits (`surface.rs`); there is no
//! second one. [`SAMPLES`] holds what Origin's window starts each of the 22 functions from: the parameter
//! values, the X and Y range and the noise level. The grid is `x1 + (x2 - x1) i/(n - 1)` with X changing
//! fastest. Noise is added to Z only, as `noise/100 * mean(z) * g`; **Sheaf's choice**: `g` comes from a seeded
//! generator (standard normal by Box-Muller), so a run is repeatable but not the sequence of Origin, which is a
//! fixed sequence that was not reproduced.

use crate::nlsf::FitFunction;
use crate::report::FitError;

/// The most points a simulated surface may have.
pub const MAX_POINTS: usize = 4_000_000;

/// Origin's default number of points in X and in Y.
pub const ORIGIN_POINTS: usize = 20;

/// What Origin's Simulate Surface window shows for one function (fitting.md 16.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Origin's name of the function, as its list shows it.
    pub origin_name: &'static str,
    /// The name in `surface::get` (five differ from Origin's by a `2D` suffix).
    pub library_name: &'static str,
    /// The parameter values, in the order of the function's parameters.
    pub params: &'static [f64],
    /// The range of X and of Y (the same for both).
    pub range: (f64, f64),
    /// The noise level in percent.
    pub noise: f64,
}

const WIDE: (f64, f64) = (-10.0, 10.0);
const UNIT: (f64, f64) = (-1.0, 1.0);

/// The 22 functions in the order of Origin's Function list, each with its own window values.
pub const SAMPLES: [Sample; 22] = [
    Sample {
        origin_name: "Chebyshev2D",
        library_name: "Chebyshev2D",
        params: &[0.0, 1.0, 2.0, 1.0, 2.0, 1.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Cosine",
        library_name: "Cosine2D",
        params: &[0.0, 1.0, 1.0, 1.0, 1.0, 3.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "DoseResp2D",
        library_name: "DoseResp2D",
        params: &[0.0, 1.0, 2.0, 2.0, 2.0, 2.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Exponential2D",
        library_name: "Exponential2D",
        params: &[0.0, 1.0, 2.0, 2.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Extreme2D",
        library_name: "Extreme2D",
        params: &[0.0, 1.0, 1.0, 1.0, 1.0, 2.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "ExtremeCum",
        library_name: "ExtremeCum2D",
        params: &[0.0, 1.0, 1.0, 2.0, 1.0, 1.0, 2.0, 1.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Fourier2D",
        library_name: "Fourier2D",
        params: &[0.0, 1.0, 1.0, 1.0, 1.0, 2.0, 2.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Gauss2D",
        library_name: "Gauss2D",
        params: &[0.0, 1.0, 0.0, 0.3, 0.0, 0.3],
        range: UNIT,
        noise: 10.0,
    },
    Sample {
        origin_name: "Gaussian2D",
        library_name: "Gaussian2D",
        params: &[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        range: UNIT,
        noise: 10.0,
    },
    Sample {
        origin_name: "GaussCum",
        library_name: "GaussCum2D",
        params: &[0.0, 1.0, 1.0, 2.0, 1.0, 2.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "LogisticCum",
        library_name: "LogisticCum2D",
        params: &[0.0, 1.0, 1.0, 2.0, 1.0, 2.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "LogNormal2D",
        library_name: "LogNormal2D",
        params: &[0.0, 1.0, 2.0, 2.0, 1.0, 2.0, 2.0, 1.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Lorentz2D",
        library_name: "Lorentz2D",
        params: &[0.0, 1.0, 0.0, 0.3, 0.0, 0.3],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Parabola2D",
        library_name: "Parabola2D",
        params: &[0.0, 2.0, 2.0, 2.0, 2.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample { origin_name: "Plane", library_name: "Plane", params: &[0.0, 1.0, 2.0], range: WIDE, noise: 0.0 },
    Sample {
        origin_name: "Poly2D",
        library_name: "Poly2D",
        params: &[0.0, 2.0, 2.0, -2.0, -2.0, 1.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Polynomial2D",
        library_name: "Polynomial2D",
        params: &[0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Power2D",
        library_name: "Power2D",
        params: &[0.0, 1.0, 2.0, 1.0, 2.0, 1.0],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Rational2D",
        library_name: "Rational2D",
        params: &[100.0, -0.02, 0.03, -4e-4, -1e-7, -0.001, 5e-5, -1e-8, 0.001, -1e-5],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "RationalTaylor",
        library_name: "RationalTaylor",
        params: &[100.0, -0.007, 0.007, -4e-4, 4e-5, -0.001, 5e-5, 8e-4, 1e-5, 3e-6],
        range: WIDE,
        noise: 0.0,
    },
    Sample {
        origin_name: "Voigt2D",
        library_name: "Voigt2D",
        params: &[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        range: UNIT,
        noise: 10.0,
    },
    Sample {
        origin_name: "Voigt2DMod",
        library_name: "Voigt2DMod",
        params: &[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        range: UNIT,
        noise: 10.0,
    },
];

/// The function Origin's window starts on.
pub const DEFAULT_FUNCTION: &str = "Gauss2D";

fn bad(message: String) -> FitError {
    FitError::BadSimulation(message)
}

/// Origin's names of the 22 functions, in the order of its list.
pub fn names() -> Vec<&'static str> {
    SAMPLES.iter().map(|s| s.origin_name).collect()
}

/// The window values of a function, by Origin's name or by the library's (case does not matter).
pub fn sample(name: &str) -> Option<&'static Sample> {
    SAMPLES
        .iter()
        .find(|s| s.origin_name.eq_ignore_ascii_case(name))
        .or_else(|| SAMPLES.iter().find(|s| s.library_name.eq_ignore_ascii_case(name)))
}

/// The surface function for a name Origin's or the library's, with Origin's name for labels.
pub fn function(name: &str) -> Result<(&'static Sample, FitFunction), FitError> {
    let s = sample(name).ok_or_else(|| {
        bad(format!("there is no surface function named '{name}'; the functions are {}", names().join(", ")))
    })?;
    let f = crate::surface::get(s.library_name)
        .ok_or_else(|| bad(format!("the surface function {} is missing from the library", s.library_name)))?;
    Ok((s, f))
}

/// The parameter values of `sample` with each `(name, value)` put in its place (a name matches exactly or
/// ignoring case; an unknown name, a repeated one or a value that is not finite is an error).
pub fn resolve_params(sample: &Sample, function: &FitFunction, given: &[(String, f64)]) -> Result<Vec<f64>, FitError> {
    let mut values = sample.params.to_vec();
    let mut set = vec![false; values.len()];
    let known = || function.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ");
    for (name, value) in given {
        let index = function
            .params
            .iter()
            .position(|p| p.name == *name)
            .or_else(|| function.params.iter().position(|p| p.name.eq_ignore_ascii_case(name)));
        let Some(i) = index else {
            return Err(bad(format!(
                "{} has no parameter '{name}'; its parameters are {}",
                sample.origin_name,
                known()
            )));
        };
        if !value.is_finite() {
            return Err(bad(format!("the value of parameter {name} is not a finite number")));
        }
        if set[i] {
            return Err(bad(format!("parameter {name} is given twice")));
        }
        values[i] = *value;
        set[i] = true;
    }
    Ok(values)
}

/// `n` values from `a` to `b`, both included, exactly the typed bounds at the ends (`a + (b - a) i/(n - 1)`).
pub fn axis(a: f64, b: f64, n: usize, what: &str) -> Result<Vec<f64>, FitError> {
    if !(a.is_finite() && b.is_finite()) {
        return Err(bad(format!("the {what} range needs finite bounds")));
    }
    if a == b {
        return Err(bad(format!("the {what} bounds are both {a}; the range needs two different values")));
    }
    if n < 2 {
        return Err(bad(format!("{n} {what} points are too few; a surface needs at least 2")));
    }
    let last = (n - 1) as f64;
    let mut v: Vec<f64> = (0..n).map(|i| a + (b - a) * (i as f64 / last)).collect();
    v[n - 1] = b;
    Ok(v)
}

/// The XYZ rows of the surface: every X for the first Y, then every X for the next Y. Returns the X, Y and Z
/// columns, each `xs.len() * ys.len()` long. A point where the function has no value is NaN; if there is no
/// value at all the call fails, so that a wrong parameter is reported rather than written as an empty column.
pub fn simulate_surface(
    function: &FitFunction,
    params: &[f64],
    xs: &[f64],
    ys: &[f64],
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>), FitError> {
    if params.len() != function.params.len() {
        return Err(bad(format!(
            "{} has {} parameters ({}), not {}",
            function.name,
            function.params.len(),
            function.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "),
            params.len()
        )));
    }
    if let Some(i) = params.iter().position(|v| !v.is_finite()) {
        return Err(bad(format!("the value of parameter {} is not a finite number", function.params[i].name)));
    }
    let n = xs.len().checked_mul(ys.len()).filter(|n| *n <= MAX_POINTS).ok_or_else(|| {
        bad(format!("{} by {} points are too many; the most is {MAX_POINTS} in all", xs.len(), ys.len()))
    })?;
    let (mut gx, mut gy, mut gz) = (Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n));
    for &y in ys {
        for &x in xs {
            gx.push(x);
            gy.push(y);
            gz.push(function.eval2(params, x, y));
        }
    }
    if !gz.iter().any(|z| z.is_finite()) {
        return Err(bad(format!("{} has no value at any of the {n} points with these parameters", function.name)));
    }
    Ok((gx, gy, gz))
}

/// A standard normal sequence (Box-Muller on a splitmix64 stream) for the noise; repeatable for a seed.
struct Normal {
    state: u64,
    spare: Option<f64>,
}

impl Normal {
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in (0, 1).
    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    fn next(&mut self) -> f64 {
        if let Some(s) = self.spare.take() {
            return s;
        }
        let (u, v) = (self.uniform(), self.uniform());
        let r = (-2.0 * u.ln()).sqrt();
        self.spare = Some(r * (std::f64::consts::TAU * v).sin());
        r * (std::f64::consts::TAU * v).cos()
    }
}

/// Adds noise to `z` in place: `noise/100 * mean(z) * g` with `g` standard normal from `seed` (Origin's form,
/// fitting.md 16.2; the mean is that of the values that are finite, and its sign does not matter because `g`
/// is symmetric). Missing values stay missing. A level of 0 changes nothing.
pub fn add_noise(z: &mut [f64], percent: f64, seed: u64) -> Result<(), FitError> {
    if !percent.is_finite() || percent < 0.0 {
        return Err(bad(format!("the noise level {percent} is not a percentage of 0 or more")));
    }
    if percent == 0.0 {
        return Ok(());
    }
    let finite: Vec<f64> = z.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return Ok(());
    }
    let mean = finite.iter().sum::<f64>() / finite.len() as f64;
    let scale = percent / 100.0 * mean.abs();
    let mut g = Normal { state: seed, spare: None };
    for v in z.iter_mut() {
        // One g per point, also where Z is missing, so that a point keeps its noise when another goes missing.
        let k = g.next();
        if v.is_finite() {
            *v += scale * k;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_the_22_functions_with_their_parameter_counts() {
        assert_eq!(names().len(), 22);
        for s in &SAMPLES {
            let f = crate::surface::get(s.library_name)
                .unwrap_or_else(|| panic!("{} is not in the library", s.library_name));
            assert_eq!(s.params.len(), f.params.len(), "{}", s.origin_name);
            assert!(s.noise == 0.0 || s.range == UNIT, "only the unit-range peaks have noise");
        }
        // The noise is on in the four peak functions that start on -1 to 1, off in the others.
        let noisy: Vec<&str> = SAMPLES.iter().filter(|s| s.noise > 0.0).map(|s| s.origin_name).collect();
        assert_eq!(noisy, ["Gauss2D", "Gaussian2D", "Voigt2D", "Voigt2DMod"]);
        assert_eq!(sample("cosine").unwrap().library_name, "Cosine2D");
        assert_eq!(sample("Cosine2D").unwrap().origin_name, "Cosine");
        assert!(sample("nothing").is_none());
    }

    #[test]
    fn the_grid_includes_both_bounds_and_runs_x_fastest() {
        let xs = axis(-1.0, 1.0, 20, "X").unwrap();
        assert_eq!((xs[0], xs[19]), (-1.0, 1.0));
        assert!((xs[1] - (-0.894736842105263)).abs() < 1e-15, "{}", xs[1]);
        let (f, p) = {
            let (s, f) = function("Plane").unwrap();
            (f, s.params.to_vec())
        };
        let (gx, gy, gz) = simulate_surface(&f, &p, &[0.0, 1.0, 2.0], &[10.0, 20.0]).unwrap();
        assert_eq!(gx, [0.0, 1.0, 2.0, 0.0, 1.0, 2.0]);
        assert_eq!(gy, [10.0, 10.0, 10.0, 20.0, 20.0, 20.0]);
        // Plane at the window values z0 0, a 1, b 2: z = x + 2 y.
        assert_eq!(gz, [20.0, 21.0, 22.0, 40.0, 41.0, 42.0]);
    }

    /// Origin's values at noise 0 for the Gauss2D default (20 by 20 on -1 to 1): fitting.md 16.2.
    #[test]
    fn gauss2d_at_its_window_values_matches_origins_numbers() {
        let (s, f) = function("Gauss2D").unwrap();
        let xs = axis(s.range.0, s.range.1, ORIGIN_POINTS, "X").unwrap();
        let (_, _, z) = simulate_surface(&f, s.params, &xs, &xs).unwrap();
        assert_eq!(z.len(), 400);
        assert!((z[0] - 1.4945338524781e-05).abs() < 1e-17, "{}", z[0]);
        // (x, y) = (-1 + 2*10/19, -1) is not on the grid; the grid's second X is -0.894736842105263.
        let by_hand = (-0.5 * ((xs[1] / 0.3).powi(2) + (xs[0] / 0.3).powi(2))).exp();
        assert!((z[1] - by_hand).abs() < 1e-15);
    }

    #[test]
    fn lorentz2d_and_plane_at_their_window_values_equal_the_formula() {
        let (s, f) = function("Lorentz2D").unwrap();
        let (xs, ys) = (axis(-10.0, 10.0, 5, "X").unwrap(), axis(-10.0, 10.0, 3, "Y").unwrap());
        let (gx, gy, gz) = simulate_surface(&f, s.params, &xs, &ys).unwrap();
        // z0 + A / (1 + ((x-xc)/w1)^2) / (1 + ((y-yc)/w2)^2), the form measured in fitting.md 14.
        let formula = crate::surface::formula("Lorentz2D").unwrap();
        assert!(formula.contains("w1") && formula.contains("w2"));
        for ((x, y), z) in gx.iter().zip(&gy).zip(&gz) {
            let want = 1.0 / ((1.0 + (x / 0.3).powi(2)) * (1.0 + (y / 0.3).powi(2)));
            assert!((z - want).abs() < 1e-12, "({x}, {y}): {z} vs {want}");
        }
    }

    #[test]
    fn every_function_has_values_on_its_window_range() {
        // Chebyshev2D is defined on -1 to 1 only (acos) and LogNormal2D for x, y > 0 (a log), so their window range
        // of -10 to 10 has numbers at 4 and 100 of the 400 points, as the formulas of the fits say; every other
        // function has numbers everywhere (Rational2D and RationalTaylor may have a pole or two).
        let mut report = Vec::new();
        for s in &SAMPLES {
            let (_, f) = function(s.origin_name).unwrap();
            let xs = axis(s.range.0, s.range.1, ORIGIN_POINTS, "X").unwrap();
            let (_, _, z) = simulate_surface(&f, s.params, &xs, &xs).unwrap();
            let finite = z.iter().filter(|v| v.is_finite()).count();
            let wanted = match s.origin_name {
                "Chebyshev2D" => 4,
                "LogNormal2D" => 100,
                _ => 390,
            };
            if finite < wanted {
                report.push(format!("{}: {finite} of {} values are numbers, wanted {wanted}", s.origin_name, z.len()));
            }
        }
        assert!(report.is_empty(), "{report:?}");
    }

    #[test]
    fn parameters_are_resolved_by_name_and_bad_ones_refused() {
        let (s, f) = function("Plane").unwrap();
        assert_eq!(resolve_params(s, &f, &[("B".into(), 5.0)]).unwrap(), [0.0, 1.0, 5.0]);
        assert!(resolve_params(s, &f, &[("q".into(), 5.0)]).unwrap_err().to_string().contains("no parameter 'q'"));
        assert!(resolve_params(s, &f, &[("a".into(), 1.0), ("a".into(), 2.0)]).is_err());
        assert!(resolve_params(s, &f, &[("a".into(), f64::NAN)]).is_err());
        assert!(
            axis(0.0, 0.0, 5, "X").is_err() && axis(0.0, 1.0, 1, "Y").is_err() && axis(f64::NAN, 1.0, 5, "X").is_err()
        );
        assert!(function("Nope").is_err());
    }

    #[test]
    fn noise_is_a_percentage_of_the_mean_and_repeatable() {
        let base: Vec<f64> = (0..2000).map(|i| 2.0 + (i % 3) as f64).collect(); // mean 3
        let mean = base.iter().sum::<f64>() / base.len() as f64;
        let mut a = base.clone();
        add_noise(&mut a, 10.0, 7).unwrap();
        let mut b = base.clone();
        add_noise(&mut b, 10.0, 7).unwrap();
        assert_eq!(a, b, "the same seed gives the same noise");
        let mut c = base.clone();
        add_noise(&mut c, 10.0, 8).unwrap();
        assert_ne!(a, c);
        let resid: Vec<f64> = a.iter().zip(&base).map(|(x, y)| x - y).collect();
        let sd = (resid.iter().map(|r| r * r).sum::<f64>() / resid.len() as f64).sqrt();
        assert!((sd - 0.1 * mean).abs() < 0.05 * 0.1 * mean * 4.0, "sd {sd} against {}", 0.1 * mean);
        // 100 % is exactly ten times 10 %, row by row (Origin's property too).
        let mut d = base.clone();
        add_noise(&mut d, 100.0, 7).unwrap();
        for i in 0..base.len() {
            assert!((d[i] - base[i] - 10.0 * (a[i] - base[i])).abs() < 1e-12);
        }
        let mut z = vec![1.0, 2.0];
        add_noise(&mut z, 0.0, 1).unwrap();
        assert_eq!(z, [1.0, 2.0]);
        assert!(add_noise(&mut z, -1.0, 1).is_err());
        let mut gap = vec![1.0, f64::NAN, 3.0];
        add_noise(&mut gap, 50.0, 1).unwrap();
        assert!(gap[1].is_nan() && gap[0] != 1.0);
    }
}
