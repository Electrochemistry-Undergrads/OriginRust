//! Simulate Curve: the values of a fit function at chosen X values for typed parameters, and the
//! X values themselves (linear or logarithmic spacing). See `docs/spec/fitting.md`, section 13.
//!
//! The values come from [`FitFunction::eval`], the evaluator the fits use; there is no second one.

use serde::{Deserialize, Serialize};

use crate::nlsf::FitFunction;
use crate::report::FitError;

/// The most points a simulated curve may have.
pub const MAX_POINTS: usize = 1_000_000;

/// How [`x_values`] spaces the points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Spacing {
    /// Equal steps.
    #[default]
    Linear,
    /// Equal ratios; both bounds must be positive.
    Log,
}

fn bad(message: String) -> FitError {
    FitError::BadSimulation(message)
}

/// `points` X values from `from` to `to`, both included. `from` may be larger than `to` (X then
/// falls). Needs at least 2 points, finite bounds that differ, and positive bounds for log spacing.
pub fn x_values(from: f64, to: f64, points: usize, spacing: Spacing) -> Result<Vec<f64>, FitError> {
    if !(from.is_finite() && to.is_finite()) {
        return Err(bad("the X range needs finite 'from' and 'to' values".into()));
    }
    if from == to {
        return Err(bad(format!("'from' and 'to' are both {from}; the X range needs two different values")));
    }
    if points < 2 {
        return Err(bad(format!("{points} points are too few; a curve needs at least 2")));
    }
    if points > MAX_POINTS {
        return Err(bad(format!("{points} points are too many; the most is {MAX_POINTS}")));
    }
    if spacing == Spacing::Log && (from <= 0.0 || to <= 0.0) {
        return Err(bad(format!("logarithmic spacing needs positive bounds, not {from} and {to}")));
    }
    let last = (points - 1) as f64;
    let mut xs: Vec<f64> = match spacing {
        // Not `from + (to - from) * t`: the difference overflows for a range wider than `f64::MAX`.
        Spacing::Linear => (0..points)
            .map(|i| {
                let t = i as f64 / last;
                from * (1.0 - t) + to * t
            })
            .collect(),
        Spacing::Log => {
            let (a, b) = (from.ln(), to.ln());
            (0..points).map(|i| (a + (b - a) * (i as f64 / last)).exp()).collect()
        }
    };
    // The ends are exactly the bounds that were typed.
    xs[0] = from;
    xs[points - 1] = to;
    Ok(xs)
}

/// Starting values that replace the default 1 where 1 makes the function degenerate (a division by
/// zero, a logarithm of zero, an overflow): `(function, parameter, value)`. Sheaf's own choice.
const START_OVERRIDES: &[(&str, &str, f64)] = &[
    ("Beta", "w2", 2.0),
    ("Beta", "w3", 2.0),
    ("BoxLucas2", "a1", 2.0),
    ("Log2P1", "a", 0.0),
    ("Logarithm", "A", 0.0),
    ("MYEGA", "C", 0.1),
    ("SRichards1", "d", 0.5),
    ("SRichards2", "d", 2.0),
];

fn start_value(function: &FitFunction, param: &crate::nlsf::ParamSpec) -> f64 {
    if param.initial.is_finite() {
        return param.initial;
    }
    START_OVERRIDES
        .iter()
        .find(|(f, p, _)| function.name.eq_ignore_ascii_case(f) && *p == param.name)
        .map_or(1.0, |&(_, _, v)| v)
}

/// The parameter values a simulation starts from: the function's own initial value where it has
/// one, otherwise 1 (what a fit starts from without a guess; a few functions that are degenerate
/// at 1 have another value), moved into the bounds of the parameter if they exclude it. The values
/// are in the order of `function.params`.
pub fn initial_params(function: &FitFunction) -> Vec<f64> {
    function
        .params
        .iter()
        .map(|p| {
            let mut v = start_value(function, p);
            if let Some(lo) = p.lower {
                v = v.max(lo);
            }
            if let Some(hi) = p.upper {
                v = v.min(hi);
            }
            v
        })
        .collect()
}

/// Parameter values by name: the starting values of [`initial_params`] with each `(name, value)`
/// put in its place. A name matches exactly, or ignoring case when that is unambiguous. An unknown
/// name, a repeated one or a value that is not finite is an error.
pub fn resolve_params(function: &FitFunction, given: &[(String, f64)]) -> Result<Vec<f64>, FitError> {
    let mut values = initial_params(function);
    let mut set = vec![false; values.len()];
    let known = || function.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ");
    for (name, value) in given {
        let exact = function.params.iter().position(|p| p.name == *name);
        let index = exact.or_else(|| {
            let mut hits = function.params.iter().enumerate().filter(|(_, p)| p.name.eq_ignore_ascii_case(name));
            match (hits.next(), hits.next()) {
                (Some((i, _)), None) => Some(i),
                _ => None,
            }
        });
        let Some(i) = index else {
            return Err(bad(format!("{} has no parameter '{name}'; its parameters are {}", function.name, known())));
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

/// The function's values at `xs` for the parameter values `params` (in the order of
/// `function.params`). A point where the function has no value (for example a logarithm of a
/// negative number) is NaN; if there is no value at all the call fails, so that a wrong range or
/// parameter is reported rather than written as an empty column.
pub fn simulate(function: &FitFunction, params: &[f64], xs: &[f64]) -> Result<Vec<f64>, FitError> {
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
    let ys: Vec<f64> = xs.iter().map(|&x| if x.is_finite() { function.eval(params, x) } else { f64::NAN }).collect();
    if !xs.is_empty() && !ys.iter().any(|y| y.is_finite()) {
        return Err(bad(format!(
            "{} has no value at any of the {} X values with these parameters",
            function.name,
            xs.len()
        )));
    }
    Ok(ys)
}

/// A one-line description of a parameter set for a column comment, for example `y0=0, xc=5`.
pub fn describe_params(function: &FitFunction, params: &[f64]) -> String {
    function.params.iter().zip(params).map(|(p, v)| format!("{}={}", p.name, v)).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library;

    #[test]
    fn linear_x_values_include_both_ends() {
        let xs = x_values(0.0, 1.0, 5, Spacing::Linear).unwrap();
        assert_eq!(xs, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
        let down = x_values(10.0, 0.0, 3, Spacing::Linear).unwrap();
        assert_eq!(down, vec![10.0, 5.0, 0.0]);
    }

    #[test]
    fn a_linear_range_wider_than_the_largest_number_keeps_every_x_finite() {
        let xs = x_values(-1e308, 1e308, 5, Spacing::Linear).unwrap();
        assert!(xs.iter().all(|x| x.is_finite()), "{xs:?}");
        assert_eq!(xs[0], -1e308);
        assert_eq!(xs[4], 1e308);
        assert!(xs[1] < xs[2] && xs[2] < xs[3], "{xs:?}");
        assert!(xs[2].abs() < 1e293, "{xs:?}");
    }

    #[test]
    fn log_x_values_have_equal_ratios() {
        let xs = x_values(1.0, 1000.0, 4, Spacing::Log).unwrap();
        assert_eq!((xs[0], xs[3]), (1.0, 1000.0));
        for (got, want) in xs.iter().zip([1.0, 10.0, 100.0, 1000.0]) {
            assert!((got / want - 1.0).abs() < 1e-12, "{got} vs {want}");
        }
    }

    #[test]
    fn x_range_errors_say_what_is_wrong() {
        let msg = |r: Result<Vec<f64>, FitError>| r.unwrap_err().to_string();
        assert!(msg(x_values(0.0, 1.0, 1, Spacing::Linear)).contains("too few"));
        assert!(msg(x_values(0.0, 1.0, 0, Spacing::Linear)).contains("too few"));
        assert!(msg(x_values(2.0, 2.0, 10, Spacing::Linear)).contains("different"));
        assert!(msg(x_values(f64::NAN, 2.0, 10, Spacing::Linear)).contains("finite"));
        assert!(msg(x_values(0.0, f64::INFINITY, 10, Spacing::Linear)).contains("finite"));
        assert!(msg(x_values(0.0, 1.0, MAX_POINTS + 1, Spacing::Linear)).contains("too many"));
        assert!(msg(x_values(0.0, 10.0, 5, Spacing::Log)).contains("positive"));
        assert!(msg(x_values(-1.0, 10.0, 5, Spacing::Log)).contains("positive"));
        assert!(x_values(0.0, 1.0, MAX_POINTS, Spacing::Linear).is_ok());
    }

    #[test]
    fn simulate_matches_the_formula_for_gauss() {
        let f = library::get("Gauss").unwrap();
        let p = resolve_params(&f, &[("y0".into(), 1.0), ("xc".into(), 5.0), ("A".into(), 4.0), ("w".into(), 2.0)])
            .unwrap();
        let xs = [3.0, 5.0, 6.5];
        let ys = simulate(&f, &p, &xs).unwrap();
        for (x, y) in xs.iter().zip(&ys) {
            let want =
                1.0 + 4.0 / (2.0 * (std::f64::consts::PI / 2.0).sqrt()) * (-2.0 * ((x - 5.0) / 2.0).powi(2)).exp();
            assert!((y - want).abs() < 1e-12, "{y} vs {want}");
        }
    }

    #[test]
    fn simulate_matches_exp_dec1_and_boltzmann() {
        let f = library::get("ExpDec1").unwrap();
        let ys = simulate(&f, &[0.5, 2.0, 1.5], &[0.0, 1.5]).unwrap();
        assert!((ys[0] - 2.5).abs() < 1e-15);
        assert!((ys[1] - (0.5 + 2.0 * (-1.0f64).exp())).abs() < 1e-15);
        let b = library::get("Boltzmann").unwrap();
        let ys = simulate(&b, &[0.0, 10.0, 3.0, 1.0], &[3.0, 100.0]).unwrap();
        assert!((ys[0] - 5.0).abs() < 1e-12);
        assert!((ys[1] - 10.0).abs() < 1e-12);
    }

    #[test]
    fn simulate_gives_the_same_values_as_the_fit_evaluator() {
        // Voigt is computed by Rust code, Gauss by a formula; both go through FitFunction::eval.
        for name in ["Voigt", "Gauss", "GammaCDF"] {
            let f = library::get(name).unwrap();
            let p = initial_params(&f);
            let xs = x_values(0.5, 8.0, 7, Spacing::Linear).unwrap();
            let ys = simulate(&f, &p, &xs).unwrap();
            for (x, y) in xs.iter().zip(ys) {
                assert_eq!(y.to_bits(), f.eval(&p, *x).to_bits(), "{name} at {x}");
            }
        }
    }

    #[test]
    fn simulate_refuses_wrong_parameters_and_empty_results() {
        let f = library::get("Gauss").unwrap();
        assert!(simulate(&f, &[1.0, 2.0], &[1.0]).unwrap_err().to_string().contains("4 parameters"));
        assert!(simulate(&f, &[1.0, 2.0, 3.0, f64::NAN], &[1.0]).unwrap_err().to_string().contains("not a finite"));
        // A logarithm of negative x has no value anywhere on a negative range.
        let g = crate::nlsf::FitFunction::from_formula("Log", "a*ln(x)", &["a"]).unwrap();
        assert!(simulate(&g, &[1.0], &[-3.0, -2.0]).unwrap_err().to_string().contains("no value"));
        // Partly defined: the undefined points are missing values.
        let ys = simulate(&g, &[1.0], &[-1.0, 1.0, std::f64::consts::E]).unwrap();
        assert!(ys[0].is_nan());
        assert!((ys[2] - 1.0).abs() < 1e-15);
        assert_eq!(simulate(&g, &[1.0], &[]).unwrap(), Vec::<f64>::new());
    }

    #[test]
    fn named_parameters_start_from_the_initial_values() {
        let f = library::get("Gauss").unwrap();
        let none = resolve_params(&f, &[]).unwrap();
        assert_eq!(none, initial_params(&f));
        let one = resolve_params(&f, &[("XC".into(), 7.0)]).unwrap();
        assert_eq!(one[1], 7.0);
        assert_eq!(one[0], none[0]);
        let unknown = resolve_params(&f, &[("zz".into(), 1.0)]).unwrap_err().to_string();
        assert!(unknown.contains("no parameter 'zz'") && unknown.contains("y0, xc, w, A"), "{unknown}");
        assert!(resolve_params(&f, &[("A".into(), 1.0), ("A".into(), 2.0)]).is_err());
        assert!(resolve_params(&f, &[("A".into(), f64::NAN)]).is_err());
    }

    #[test]
    fn typed_values_are_not_clamped_to_the_default_bounds() {
        let f = library::get("Gauss").unwrap();
        let p = resolve_params(&f, &[("w".into(), -1.0)]).unwrap();
        assert_eq!(p[2], -1.0);
        assert!(simulate(&f, &p, &[1.0, 2.0]).is_ok());
    }

    #[test]
    fn initial_values_respect_bounds_and_explicit_starts() {
        let f = library::get("Gauss").unwrap().with_bounds("w", Some(3.0), None).with_initial("y0", -2.0);
        let p = initial_params(&f);
        assert_eq!(p, vec![-2.0, 1.0, 3.0, 1.0]);
    }

    #[test]
    fn every_library_function_simulates_at_its_initial_values() {
        let mut bad = Vec::new();
        let xs = x_values(0.1, 10.0, 50, Spacing::Linear).unwrap();
        // ln(-b ln x) is defined only on one side of x = 1.
        let below_one = x_values(0.1, 0.9, 50, Spacing::Linear).unwrap();
        for info in library::list() {
            let f = library::get(&info.name).unwrap();
            let p = initial_params(&f);
            let xs = if info.name == "Bradley" { &below_one } else { &xs };
            match simulate(&f, &p, xs) {
                Ok(ys) if ys.iter().all(|y| y.is_finite()) => {}
                Ok(ys) => bad.push(format!(
                    "{}: {} of {} values are not finite",
                    info.name,
                    ys.iter().filter(|y| !y.is_finite()).count(),
                    ys.len()
                )),
                Err(e) => bad.push(format!("{}: {e}", info.name)),
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
