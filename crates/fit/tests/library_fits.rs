//! Every built-in function that has a starting-value routine must find its way from the guess to
//! the true parameters on clean, slightly noisy data (noise is a fixed sine formula, not random).
//!
//! The whole table is run at three y scales: 1 (the size most cases are written in), 1e-3 and
//! 1e-6 (the owner's currents run from 1e-6 to 1e-2). A case lists the parameters that scale
//! with y (offsets, amplitudes, areas, heights); at scale `s` their true values and the noise are
//! multiplied by `s`, the others stay. `Power` (`y = x^A`) has no prefactor, so y cannot be scaled
//! through a parameter; instead its x range is scaled by `s^(1/A)`, which scales y by `s`.

use sheaf_fit::library;
use sheaf_fit::nlsf::{FitOptions, fit};
use sheaf_fit::report::FitStatus;

struct Case {
    name: &'static str,
    truth: &'static [f64],
    /// Names of the parameters that scale with y.
    y_params: &'static [&'static str],
    /// Parameters held fixed at their true value.
    fixed: &'static [&'static str],
    x0: f64,
    x1: f64,
    n: usize,
    noise: f64,
    /// Relative tolerance on each parameter (absolute for parameters whose truth is 0).
    tol: f64,
}

const fn case(
    name: &'static str,
    truth: &'static [f64],
    y_params: &'static [&'static str],
    x0: f64,
    x1: f64,
    n: usize,
    noise: f64,
    tol: f64,
) -> Case {
    Case { name, truth, y_params, fixed: &[], x0, x1, n, noise, tol }
}

const POLY_FIXED: &[&str] = &["a4", "a5", "a6", "a7", "a8", "a9"];

const CASES: &[Case] = &[
    case("Line", &[1.5, -0.7], &["A", "B"], 0.0, 10.0, 40, 0.01, 0.05),
    case("Parabola", &[1.0, -2.0, 0.5], &["A", "B", "C"], -3.0, 5.0, 40, 0.01, 0.05),
    case("Cubic", &[1.0, -2.0, 0.5, 0.1], &["A", "B", "C", "D"], -3.0, 5.0, 40, 0.01, 0.05),
    // Poly has ten coefficients; the six highest are held at zero, as a user would do.
    Case {
        name: "Poly",
        truth: &[1.0, -2.0, 0.5, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        y_params: &["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9"],
        fixed: POLY_FIXED,
        x0: -3.0,
        x1: 5.0,
        n: 40,
        noise: 0.01,
        tol: 0.05,
    },
    case("Allometric1", &[2.5, 0.6], &["a"], 1.0, 20.0, 40, 0.005, 0.05),
    case("Power", &[2.5], &[], 0.5, 5.0, 40, 0.005, 0.05),
    case("Gauss", &[0.5, 5.0, 1.8, 6.0], &["y0", "A"], 0.0, 10.0, 100, 0.01, 0.03),
    case("Gauss", &[2.0, 4.0, 1.2, -5.0], &["y0", "A"], 0.0, 10.0, 100, 0.01, 0.03),
    case("GaussAmp", &[0.2, 5.0, 1.0, 3.0], &["y0", "A"], 0.0, 10.0, 100, 0.01, 0.03),
    // Gaussian is (y0, xc, A, w): A is the area and w the FWHM.
    case("Gaussian", &[0.5, 5.0, 6.0, 2.0], &["y0", "A"], 0.0, 10.0, 100, 0.01, 0.03),
    case("Bigaussian", &[0.2, 5.0, 3.0, 1.0, 1.6], &["y0", "H"], 0.0, 10.0, 150, 0.01, 0.05),
    case("Lorentz", &[0.3, 5.0, 1.2, 4.0], &["y0", "A"], 0.0, 10.0, 200, 0.005, 0.03),
    case("PsdVoigt1", &[0.2, 5.0, 4.0, 1.5, 0.4], &["y0", "A"], 0.0, 10.0, 200, 0.005, 0.05),
    case("Voigt", &[0.2, 5.0, 4.0, 1.2, 0.8], &["y0", "A"], 0.0, 10.0, 200, 0.005, 0.06),
    case("ExpDec1", &[0.5, 3.0, 2.0], &["y0", "A1"], 0.0, 10.0, 50, 0.005, 0.05),
    case("ExpDec2", &[0.5, 2.0, 0.7, 1.5, 4.0], &["y0", "A1", "A2"], 0.0, 20.0, 80, 0.002, 0.1),
    case("ExpDec3", &[0.5, 1.5, 0.5, 1.5, 3.0, 2.0, 9.0], &["y0", "A1", "A2", "A3"], 0.0, 40.0, 150, 0.0005, 0.2),
    case("ExpGro1", &[0.5, 0.4, 3.0], &["y0", "A1"], 0.0, 8.0, 50, 0.002, 0.05),
    // ExpGrow1 is (y0, x0, A1, t1); x0 and A1 trade off against each other, so x0 is held at 0.
    Case {
        name: "ExpGrow1",
        truth: &[0.5, 0.0, 0.4, 3.0],
        y_params: &["y0", "A1"],
        fixed: &["x0"],
        x0: 0.0,
        x1: 8.0,
        n: 50,
        noise: 0.002,
        tol: 0.05,
    },
    case("ExpAssoc", &[3.0, -1.5, 1.0, -1.0, 4.0], &["y0", "A1", "A2"], 0.0, 20.0, 80, 0.002, 0.15),
    case("Boltzmann", &[0.5, 4.0, 5.0, 0.8], &["A1", "A2"], 0.0, 10.0, 60, 0.01, 0.05),
    case("Boltzmann", &[4.0, 0.5, 5.0, 0.8], &["A1", "A2"], 0.0, 10.0, 60, 0.01, 0.05),
    case("Logistic", &[0.5, 4.0, 5.0, 3.0], &["A1", "A2"], 0.5, 10.0, 60, 0.01, 0.05),
    case("Hill", &[3.0, 4.0, 2.0], &["Vmax"], 0.0, 15.0, 60, 0.01, 0.05),
    case("DoseResp", &[0.2, 3.0, 1.0, 1.5], &["A1", "A2"], -1.0, 3.0, 60, 0.01, 0.05),
    case("Sine", &[1.0, 0.8, 1.6, 2.0], &["y0", "A"], 0.0, 12.0, 120, 0.01, 0.03),
    case("Asymptotic1", &[5.0, 4.0, 0.7], &["a", "b"], 0.0, 10.0, 50, 0.005, 0.05),
    case("Exponential", &[0.5, 0.4, 0.3], &["y0", "A"], 0.0, 8.0, 50, 0.002, 0.05), // Functions added with their own starting values (second batch).
    case("Poly4", &[1.0, -2.0, 0.5, 0.1, -0.01], &["A0", "A1", "A2", "A3", "A4"], -3.0, 5.0, 60, 0.001, 0.1),
    case(
        "Poly5",
        &[1.0, -2.0, 0.5, 0.1, -0.01, 0.002],
        &["A0", "A1", "A2", "A3", "A4", "A5"],
        -3.0,
        5.0,
        60,
        0.001,
        0.15,
    ),
    case("Constant", &[2.0], &["y0"], 0.0, 10.0, 20, 0.01, 0.05),
    case("LineMod", &[0.8, 2.0], &["a"], 0.0, 10.0, 40, 0.01, 0.05),
    Case {
        name: "Power1",
        truth: &[0.0, 2.5, 0.6],
        y_params: &["A"],
        fixed: &["xc"],
        x0: 0.5,
        x1: 20.0,
        n: 40,
        noise: 0.005,
        tol: 0.05,
    },
    case("PearsonVII", &[0.2, 5.0, 4.0, 1.5, 2.0], &["y0", "A"], 0.0, 10.0, 200, 0.005, 0.06),
    case("PsdVoigt2", &[0.2, 5.0, 4.0, 1.5, 1.2, 0.4], &["y0", "A"], 0.0, 10.0, 200, 0.002, 0.1),
    case("Asym2Sig", &[0.2, 5.0, 3.0, 2.0, 0.6, 0.9], &["y0", "A"], 0.0, 10.0, 200, 0.005, 0.1),
    case("AsymmetricGaussian", &[0.2, 5.0, 1.0, 1.6, 3.0], &["y0", "A"], 0.0, 10.0, 150, 0.01, 0.05),
    case("Logistpk", &[0.2, 5.0, 0.8, 3.0], &["y0", "A"], 0.0, 10.0, 150, 0.01, 0.05),
    case("LogNormal", &[0.2, 4.0, 0.3, 3.0], &["y0", "A"], 0.5, 10.0, 200, 0.005, 0.05),
    Case {
        name: "ExpDecay1",
        truth: &[0.5, 0.0, 3.0, 2.0],
        y_params: &["y0", "A1"],
        fixed: &["x0"],
        x0: 0.0,
        x1: 10.0,
        n: 50,
        noise: 0.005,
        tol: 0.05,
    },
    Case {
        name: "ExpDecay2",
        truth: &[0.5, 0.0, 2.0, 0.7, 1.5, 4.0],
        y_params: &["y0", "A1", "A2"],
        fixed: &["x0"],
        x0: 0.0,
        x1: 20.0,
        n: 80,
        noise: 0.002,
        tol: 0.1,
    },
    Case {
        name: "ExpDecay3",
        truth: &[0.5, 0.0, 1.5, 0.5, 1.5, 3.0, 2.0, 9.0],
        y_params: &["y0", "A1", "A2", "A3"],
        fixed: &["x0"],
        x0: 0.0,
        x1: 40.0,
        n: 150,
        noise: 0.0005,
        tol: 0.2,
    },
    case("ExpGro2", &[0.5, 0.3, 2.0, 0.3, 6.0], &["y0", "A1", "A2"], 0.0, 8.0, 80, 0.002, 0.15),
    Case {
        name: "ExpGrow2",
        truth: &[0.5, 0.0, 0.3, 2.0, 0.3, 6.0],
        y_params: &["y0", "A1", "A2"],
        fixed: &["x0"],
        x0: 0.0,
        x1: 8.0,
        n: 80,
        noise: 0.002,
        tol: 0.15,
    },
    case("Asymptotic2", &[5.0, 4.0, 0.7], &["a", "b"], 0.0, 10.0, 50, 0.005, 0.05),
    case("BoxLucas1", &[5.0, 0.7], &["a"], 0.0, 10.0, 50, 0.005, 0.05),
    case("Exp2P", &[1.5, 1.3], &["a"], 0.0, 8.0, 40, 0.005, 0.05),
    case("SLogistic1", &[4.0, 5.0, 1.2], &["a"], 0.0, 10.0, 60, 0.01, 0.05),
    case("SLogistic4", &[4.0, 0.5, 5.0, 1.2], &["a", "b"], 0.0, 10.0, 60, 0.01, 0.05),
    case("SLogistic4", &[0.5, 4.0, 5.0, 1.2], &["a", "b"], 0.0, 10.0, 60, 0.01, 0.05),
    case("SGompertz", &[4.0, 4.0, 0.8], &["a"], 0.0, 12.0, 60, 0.01, 0.05),
    case("Hill1", &[0.2, 3.0, 4.0, 2.0], &["START", "END"], 0.0, 15.0, 60, 0.01, 0.05),
    case("MichaelisMenten", &[3.0, 2.0], &["Vmax"], 0.0, 20.0, 50, 0.01, 0.05),
    case("OneSiteBind", &[2.0, 1.5], &["Bmax"], 0.0, 15.0, 50, 0.01, 0.05),
    case("Hyperbl", &[2.0, 3.0], &["P1"], 0.0, 20.0, 50, 0.01, 0.05),
    case("RectHyperbola", &[3.0, 0.4], &["a"], 0.0, 20.0, 50, 0.01, 0.05),
];

fn run_table(scale: f64) {
    for c in CASES {
        let mut f = library::get(c.name).unwrap();
        assert_eq!(f.params.len(), c.truth.len(), "{}", c.name);
        let names: Vec<String> = f.params.iter().map(|p| p.name.clone()).collect();
        let truth: Vec<f64> = names
            .iter()
            .zip(c.truth)
            .map(|(n, &t)| if c.y_params.contains(&n.as_str()) { t * scale } else { t })
            .collect();
        for (n, &t) in names.iter().zip(&truth) {
            if c.fixed.contains(&n.as_str()) {
                f = f.with_fixed(n, t);
            }
        }
        // Power has no prefactor: scale x so that y scales by `scale`.
        let xs = if c.name == "Power" { scale.powf(1.0 / c.truth[0]) } else { 1.0 };
        let x: Vec<f64> = (0..c.n).map(|i| xs * (c.x0 + (c.x1 - c.x0) * i as f64 / (c.n - 1) as f64)).collect();
        let y: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, &x)| f.eval(&truth, x) + scale * c.noise * (37.0 * (i + 1) as f64).sin())
            .collect();
        let label = format!("{} at y scale {scale:e}", c.name);
        let r = fit(&f, &x, &y, &FitOptions::default()).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(r.status, FitStatus::Converged, "{label}");
        let mut got: Vec<f64> = r.params.iter().map(|p| p.value).collect();
        let mut want = truth.clone();
        // The phases of a multi-exponential are interchangeable: compare them ordered by time
        // constant (the parameters after y0 come as pairs of amplitude and time constant).
        if matches!(c.name, "ExpDec2" | "ExpDec3" | "ExpAssoc") {
            for v in [&mut got, &mut want] {
                let mut pairs: Vec<(f64, f64)> = v[1..].chunks(2).map(|c| (c[0], c[1])).collect();
                pairs.sort_by(|a, b| a.1.total_cmp(&b.1));
                for (k, (a, t)) in pairs.into_iter().enumerate() {
                    v[1 + 2 * k] = a;
                    v[2 + 2 * k] = t;
                }
            }
        }
        for (k, (&g, &t)) in got.iter().zip(&want).enumerate() {
            let err = if t == 0.0 { g.abs() } else { ((g - t) / t).abs() };
            assert!(err < c.tol, "{label}, {}: got {g}, true {t}", names[k]);
        }
    }
}

#[test]
fn guesses_lead_to_the_true_parameters() {
    run_table(1.0);
}

#[test]
fn guesses_work_at_milli_scale() {
    run_table(1e-3);
}

#[test]
fn guesses_work_at_micro_scale() {
    run_table(1e-6);
}

/// A voltammetric peak of 2e-5 A on a 3e-6 A baseline. The Gaussian guess once put the width
/// where the area belongs, and the fit ran off to xc = -1.5e6 yet reported Converged.
#[test]
fn gaussian_fits_a_small_current_peak_like_gauss_and_gaussamp() {
    let x: Vec<f64> = (0..200).map(|i| i as f64 / 199.0).collect();
    let y: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, &x)| 3e-6 + 2e-5 * (-0.5 * ((x - 0.45) / 0.04).powi(2)).exp() + 2e-7 * (37.0 * (i + 1) as f64).sin())
        .collect();
    for name in ["Gaussian", "Gauss", "GaussAmp"] {
        let f = library::get(name).unwrap();
        let r = fit(&f, &x, &y, &FitOptions::default()).unwrap();
        assert_eq!(r.status, FitStatus::Converged, "{name}");
        let xc = r.param("xc").unwrap().value;
        assert!((xc - 0.45).abs() < 0.005, "{name}: xc {xc}");
        assert!(r.stats.r_squared > 0.99, "{name}: R2 {}", r.stats.r_squared);
    }
}

#[test]
fn every_function_with_a_guess_has_a_case() {
    // The table must cover every function that has a starting-value routine.
    let with_guess: Vec<String> = library::list()
        .into_iter()
        .filter(|i| library::get(&i.name).is_some_and(|f| f.init.is_some()))
        .map(|i| i.name)
        .collect();
    for name in &with_guess {
        assert!(CASES.iter().any(|c| c.name == name), "{name} has a guess routine but no case");
    }
}
