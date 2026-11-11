//! Every built-in function that has a starting-value routine must find its way from the guess to
//! the true parameters on clean, slightly noisy data (noise is a fixed sine formula, not random).

use sheaf_fit::library;
use sheaf_fit::nlsf::{FitOptions, fit};
use sheaf_fit::report::FitStatus;

struct Case {
    name: &'static str,
    truth: &'static [f64],
    x0: f64,
    x1: f64,
    n: usize,
    noise: f64,
    /// Relative tolerance on each parameter (absolute for parameters whose truth is 0).
    tol: f64,
}

const CASES: &[Case] = &[
    Case { name: "Line", truth: &[1.5, -0.7], x0: 0.0, x1: 10.0, n: 40, noise: 0.01, tol: 0.05 },
    Case { name: "Parabola", truth: &[1.0, -2.0, 0.5], x0: -3.0, x1: 5.0, n: 40, noise: 0.01, tol: 0.05 },
    Case { name: "Allometric1", truth: &[2.5, 0.6], x0: 1.0, x1: 20.0, n: 40, noise: 0.005, tol: 0.05 },
    Case { name: "Gauss", truth: &[0.5, 5.0, 1.8, 6.0], x0: 0.0, x1: 10.0, n: 100, noise: 0.01, tol: 0.03 },
    Case { name: "Gauss", truth: &[2.0, 4.0, 1.2, -5.0], x0: 0.0, x1: 10.0, n: 100, noise: 0.01, tol: 0.03 },
    Case { name: "GaussAmp", truth: &[0.2, 5.0, 1.0, 3.0], x0: 0.0, x1: 10.0, n: 100, noise: 0.01, tol: 0.03 },
    Case { name: "Lorentz", truth: &[0.3, 5.0, 1.2, 4.0], x0: 0.0, x1: 10.0, n: 200, noise: 0.005, tol: 0.03 },
    Case { name: "PsdVoigt1", truth: &[0.2, 5.0, 4.0, 1.5, 0.4], x0: 0.0, x1: 10.0, n: 200, noise: 0.005, tol: 0.05 },
    Case { name: "Voigt", truth: &[0.2, 5.0, 4.0, 1.2, 0.8], x0: 0.0, x1: 10.0, n: 200, noise: 0.005, tol: 0.06 },
    Case { name: "ExpDec1", truth: &[0.5, 3.0, 2.0], x0: 0.0, x1: 10.0, n: 50, noise: 0.005, tol: 0.05 },
    Case { name: "ExpDec2", truth: &[0.5, 2.0, 0.7, 1.5, 4.0], x0: 0.0, x1: 20.0, n: 80, noise: 0.002, tol: 0.1 },
    Case { name: "ExpGro1", truth: &[0.5, 0.4, 3.0], x0: 0.0, x1: 8.0, n: 50, noise: 0.002, tol: 0.05 },
    Case { name: "ExpAssoc", truth: &[3.0, -1.5, 1.0, -1.0, 4.0], x0: 0.0, x1: 20.0, n: 80, noise: 0.002, tol: 0.15 },
    Case { name: "Boltzmann", truth: &[0.5, 4.0, 5.0, 0.8], x0: 0.0, x1: 10.0, n: 60, noise: 0.01, tol: 0.05 },
    Case { name: "Boltzmann", truth: &[4.0, 0.5, 5.0, 0.8], x0: 0.0, x1: 10.0, n: 60, noise: 0.01, tol: 0.05 },
    Case { name: "Logistic", truth: &[0.5, 4.0, 5.0, 3.0], x0: 0.5, x1: 10.0, n: 60, noise: 0.01, tol: 0.05 },
    Case { name: "Hill", truth: &[3.0, 4.0, 2.0], x0: 0.0, x1: 15.0, n: 60, noise: 0.01, tol: 0.05 },
    Case { name: "DoseResp", truth: &[0.2, 3.0, 1.0, 1.5], x0: -1.0, x1: 3.0, n: 60, noise: 0.01, tol: 0.05 },
    Case { name: "Sine", truth: &[1.0, 0.8, 1.6, 2.0], x0: 0.0, x1: 12.0, n: 120, noise: 0.01, tol: 0.03 },
    Case { name: "Asymptotic1", truth: &[5.0, 4.0, 0.7], x0: 0.0, x1: 10.0, n: 50, noise: 0.005, tol: 0.05 },
    Case { name: "Exponential", truth: &[0.5, 0.4, 0.3], x0: 0.0, x1: 8.0, n: 50, noise: 0.002, tol: 0.05 },
];

#[test]
fn guesses_lead_to_the_true_parameters() {
    for c in CASES {
        let f = library::get(c.name).unwrap();
        assert_eq!(f.params.len(), c.truth.len(), "{}", c.name);
        let x: Vec<f64> = (0..c.n).map(|i| c.x0 + (c.x1 - c.x0) * i as f64 / (c.n - 1) as f64).collect();
        let y: Vec<f64> =
            x.iter().enumerate().map(|(i, &x)| f.eval(c.truth, x) + c.noise * (37.0 * (i + 1) as f64).sin()).collect();
        let r = fit(&f, &x, &y, &FitOptions::default()).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert_eq!(r.status, FitStatus::Converged, "{}", c.name);
        for (p, &t) in r.params.iter().zip(c.truth) {
            let err = if t == 0.0 { p.value.abs() } else { ((p.value - t) / t).abs() };
            assert!(err < c.tol, "{} {}: got {}, true {t}", c.name, p.name, p.value);
        }
    }
}
