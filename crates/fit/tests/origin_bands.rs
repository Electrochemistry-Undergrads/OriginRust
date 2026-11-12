//! Confidence and prediction bands (`docs/spec/fitting-bands.md`).
//!
//! The straight-line numbers below were produced by Origin 2016 with
//! `tools/origin-ref/bands-linear.lines.txt` on data we generated: x = 1..12 and
//! y = 2 + 0.5 x + 0.3 sin(3 x). Origin supplied the regression (intercept, slope), the residual
//! sum of squares and degrees of freedom, the mean and spread of x and the Student t quantiles;
//! the band half widths are Origin's closed form for a line evaluated from those numbers (the form
//! is written in the Origin C source shipped with Origin 2016). The bands themselves cannot be read
//! from a script without changing the owner's dialog settings.

use sheaf_fit::library;
use sheaf_fit::linear::{LinearOptions, PolyOptions, fit_linear_with_curve, fit_polynomial_with_curve};
use sheaf_fit::nlsf::{FitOptions, fit};
use sheaf_fit::report::Weighting;
use sheaf_fit::{bands, dist};

// Origin 2016 output (see the script).
const A: f64 = 2.031_994_928_083_539;
const B: f64 = 0.493_324_262_056_351_5;
const N: f64 = 12.0;
const SSR: f64 = 0.616_395_762_241_221_7;
const DOF: usize = 10;
const RMSE: f64 = 0.248_273_188_693_669;
const XBAR: f64 = 6.5;
const SXX: f64 = 143.0;
// Origin's tinv agrees with the exact quantile to about 3e-7, hence the tolerance of 1e-6 below.
const T95: f64 = 2.228_138_548_533_349;
const T90: f64 = 1.812_461_039_797_34;

fn data() -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (1..=12).map(f64::from).collect();
    let y: Vec<f64> = x.iter().map(|x| 2.0 + 0.5 * x + 0.3 * (3.0 * x).sin()).collect();
    (x, y)
}

const XS: [f64; 5] = [0.5, 3.3, 6.5, 9.9, 14.0];

#[test]
fn straight_line_bands_match_the_origin_closed_form() {
    let (x, y) = data();
    let (report, curve) = fit_linear_with_curve(&x, &y, &LinearOptions::default()).unwrap();
    assert!((report.params[0].value - A).abs() < 1e-12 && (report.params[1].value - B).abs() < 1e-12);
    assert!((report.stats.rss - SSR).abs() < 1e-12);
    for (level, t_origin) in [(0.95, T95), (0.90, T90)] {
        let b = curve.bands(&XS, level).unwrap();
        assert!((b.t - t_origin).abs() < 1e-6, "t at {level}: {} vs {t_origin}", b.t);
        for (i, &xs) in XS.iter().enumerate() {
            let fit = A + B * xs;
            let conf = t_origin * RMSE * (1.0 / N + (xs - XBAR).powi(2) / SXX).sqrt();
            let pred = t_origin * RMSE * (1.0 + 1.0 / N + (xs - XBAR).powi(2) / SXX).sqrt();
            let tol = 1e-6;
            assert!((b.fit[i] - fit).abs() < tol, "fit at {xs}");
            assert!((b.conf_lower[i] - (fit - conf)).abs() < tol, "LCL at {xs}: {} vs {}", b.conf_lower[i], fit - conf);
            assert!((b.conf_upper[i] - (fit + conf)).abs() < tol, "UCL at {xs}");
            assert!((b.pred_lower[i] - (fit - pred)).abs() < tol, "LPL at {xs}");
            assert!((b.pred_upper[i] - (fit + pred)).abs() < tol, "UPL at {xs}");
        }
    }
    assert_eq!(report.stats.dof, DOF);
}

#[test]
fn a_line_with_x_far_from_zero_keeps_its_bands() {
    // x from 100000 to 100011: the covariance in powers of x would cancel completely; the bands
    // are computed in the centred basis and equal the shifted case.
    let (x, y) = data();
    let xs: Vec<f64> = x.iter().map(|v| v + 100_000.0).collect();
    let (_, shifted) = fit_linear_with_curve(&xs, &y, &LinearOptions::default()).unwrap();
    let (_, plain) = fit_linear_with_curve(&x, &y, &LinearOptions::default()).unwrap();
    let at = [3.3, 6.5, 14.0];
    let shifted_at: Vec<f64> = at.iter().map(|v| v + 100_000.0).collect();
    let b1 = shifted.bands(&shifted_at, 0.95).unwrap();
    let b2 = plain.bands(&at, 0.95).unwrap();
    for i in 0..at.len() {
        assert!((b1.conf_upper[i] - b2.conf_upper[i]).abs() < 1e-7, "{i}");
        assert!((b1.pred_lower[i] - b2.pred_lower[i]).abs() < 1e-7, "{i}");
    }
}

#[test]
fn fixed_intercept_line_uses_the_slope_error_only() {
    // With the intercept held at a value the curve is value + slope x: the confidence half width
    // is t |x| SE(slope) and the prediction half width t sqrt(x^2 SE^2 + s2) (Origin C source).
    let (x, y) = data();
    let opts = LinearOptions { fix_intercept: Some(2.0), ..Default::default() };
    let (report, curve) = fit_linear_with_curve(&x, &y, &opts).unwrap();
    let se = report.params[1].std_error;
    let s2 = report.stats.reduced_chi_sqr;
    let b = curve.bands(&XS, 0.95).unwrap();
    let t = dist::t_critical(0.95, report.stats.dof);
    for (i, &xs) in XS.iter().enumerate() {
        let conf = t * xs.abs() * se;
        let pred = t * (xs * xs * se * se + s2).sqrt();
        assert!((b.conf_upper[i] - b.fit[i] - conf).abs() < 1e-10, "conf at {xs}");
        assert!((b.fit[i] - b.pred_lower[i] - pred).abs() < 1e-10, "pred at {xs}");
    }
}

#[test]
fn bands_do_not_depend_on_the_error_scaling_option() {
    // Origin: the confidence and prediction bands are not affected by "Use Reduced Chi^2".
    let (x, y) = data();
    let err: Vec<f64> = x.iter().map(|v| 0.1 + 0.02 * v).collect();
    let weighting = Weighting::Instrumental(err);
    let on = LinearOptions { weighting: weighting.clone(), ..Default::default() };
    let off = LinearOptions { weighting, scale_errors_by_reduced_chi_sqr: false, ..Default::default() };
    let (r_on, c_on) = fit_linear_with_curve(&x, &y, &on).unwrap();
    let (r_off, c_off) = fit_linear_with_curve(&x, &y, &off).unwrap();
    assert!(r_on.params[1].std_error != r_off.params[1].std_error);
    let (b_on, b_off) = (c_on.bands(&XS, 0.95).unwrap(), c_off.bands(&XS, 0.95).unwrap());
    assert_eq!(b_on, b_off);
}

#[test]
fn polynomial_bands_follow_the_closed_form_of_a_quadratic() {
    // Independent check: the variance of a quadratic fitted to x = 1..12 from the normal
    // equations (X'X)^-1 written out with nalgebra, scaled by the reduced chi-square.
    use nalgebra::{DMatrix, DVector};
    let (x, y) = data();
    let (report, curve) = fit_polynomial_with_curve(&x, &y, 2, &PolyOptions::default()).unwrap();
    let design = DMatrix::from_fn(12, 3, |i, j| x[i].powi(j as i32));
    let inv = (design.transpose() * &design).try_inverse().unwrap();
    let b = curve.bands(&XS, 0.95).unwrap();
    let t = dist::t_critical(0.95, 9);
    let s2 = report.stats.reduced_chi_sqr;
    for (i, &xs) in XS.iter().enumerate() {
        let g = DVector::from_vec(vec![1.0, xs, xs * xs]);
        let var = s2 * (g.transpose() * &inv * &g)[(0, 0)];
        assert!((b.conf_upper[i] - b.fit[i] - t * var.sqrt()).abs() < 1e-8, "conf at {xs}");
        assert!((b.pred_upper[i] - b.fit[i] - t * (var + s2).sqrt()).abs() < 1e-8, "pred at {xs}");
    }
}

#[test]
fn nonlinear_bands_of_a_line_equal_the_linear_bands() {
    let (x, y) = data();
    let (_, curve) = fit_linear_with_curve(&x, &y, &LinearOptions::default()).unwrap();
    let want = curve.bands(&XS, 0.95).unwrap();
    let f = library::get("Line").unwrap();
    let report = fit(&f, &x, &y, &FitOptions::default()).unwrap();
    let got = bands::nl_bands(&f, &report, &XS, 0.95, true).unwrap();
    for i in 0..XS.len() {
        for (a, b) in [
            (got.conf_lower[i], want.conf_lower[i]),
            (got.conf_upper[i], want.conf_upper[i]),
            (got.pred_lower[i], want.pred_lower[i]),
            (got.pred_upper[i], want.pred_upper[i]),
        ] {
            assert!((a - b).abs() < 1e-5 * (1.0 + b.abs()), "{a} vs {b}");
        }
    }
}

#[test]
fn nonlinear_bands_of_an_exponential_use_the_analytic_gradient() {
    // y = a exp(-x/t) fitted to noisy data; the band half width is t sqrt(g' C g) with the
    // analytic gradient g = (exp(-x/t), a x / t^2 exp(-x/t)).
    let x: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let y: Vec<f64> =
        x.iter().enumerate().map(|(i, x)| 3.0 * (-x / 2.0).exp() + 0.02 * (7.0 * i as f64).sin()).collect();
    let f = sheaf_fit::nlsf::FitFunction::from_formula("E", "a*exp(-x/t)", &["a", "t"])
        .unwrap()
        .with_initial("a", 2.0)
        .with_initial("t", 1.0);
    let report = fit(&f, &x, &y, &FitOptions::default()).unwrap();
    let (a, t) = (report.params[0].value, report.params[1].value);
    let c = &report.covariance;
    let b = bands::nl_bands(&f, &report, &x, 0.95, true).unwrap();
    let tc = dist::t_critical(0.95, report.stats.dof);
    for (i, &xi) in x.iter().enumerate() {
        let g = [(-xi / t).exp(), a * xi / (t * t) * (-xi / t).exp()];
        let var = g[0] * g[0] * c[0] + 2.0 * g[0] * g[1] * c[1] + g[1] * g[1] * c[3];
        let half = tc * var.sqrt();
        assert!((b.conf_upper[i] - b.fit[i] - half).abs() < 1e-6 * (1.0 + half), "x = {xi}");
        let pred = tc * (var + report.stats.reduced_chi_sqr).sqrt();
        assert!((b.pred_upper[i] - b.fit[i] - pred).abs() < 1e-6 * (1.0 + pred), "x = {xi}");
    }
}

#[test]
fn level_ordering_and_fixed_parameters() {
    let x: Vec<f64> = (0..40).map(|i| i as f64 * 0.25).collect();
    let y: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, x)| 1.0 + 2.0 * (-0.5 * ((x - 5.0) / 1.5).powi(2)).exp() + 0.02 * (13.0 * i as f64).sin())
        .collect();
    let f = library::get("Gauss").unwrap().with_fixed("y0", 1.0);
    let report = fit(&f, &x, &y, &FitOptions::default()).unwrap();
    let narrow = bands::nl_bands(&f, &report, &x, 0.80, true).unwrap();
    let wide = bands::nl_bands(&f, &report, &x, 0.99, true).unwrap();
    for i in 0..x.len() {
        assert!(narrow.conf_upper[i] <= wide.conf_upper[i] && narrow.conf_lower[i] >= wide.conf_lower[i]);
        assert!(wide.pred_upper[i] >= wide.conf_upper[i] && wide.pred_lower[i] <= wide.conf_lower[i]);
        assert!(wide.conf_lower[i] <= wide.fit[i] && wide.fit[i] <= wide.conf_upper[i]);
    }
    assert!(bands::nl_bands(&f, &report, &x, 1.0, true).is_err());
    assert!(bands::nl_bands(&f, &report, &x, 0.0, true).is_err());
}

#[test]
fn an_unscaled_covariance_is_rescaled_for_the_bands() {
    let x: Vec<f64> = (0..30).map(|i| i as f64 * 0.2).collect();
    let y: Vec<f64> =
        x.iter().enumerate().map(|(i, x)| 3.0 * (-x / 2.0).exp() + 0.05 * (7.0 * i as f64).sin()).collect();
    let f = sheaf_fit::nlsf::FitFunction::from_formula("E", "a*exp(-x/t)", &["a", "t"])
        .unwrap()
        .with_initial("a", 2.0)
        .with_initial("t", 1.0);
    let scaled = fit(&f, &x, &y, &FitOptions::default()).unwrap();
    let unscaled = fit(&f, &x, &y, &FitOptions { scale_errors: Some(false), ..Default::default() }).unwrap();
    let b1 = bands::nl_bands(&f, &scaled, &x, 0.95, true).unwrap();
    let b2 = bands::nl_bands(&f, &unscaled, &x, 0.95, false).unwrap();
    for i in 0..x.len() {
        assert!((b1.conf_upper[i] - b2.conf_upper[i]).abs() < 1e-6, "{i}");
    }
}
