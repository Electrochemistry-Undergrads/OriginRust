//! Linear and polynomial fits compared with numbers produced by Origin on data we generated
//! ourselves (`tools/origin-ref/fitlin-suite.labtalk.txt`). The numbers are stored in
//! `fixtures/linear_origin.rs`; these tests need no Origin installation.
//!
//! Tolerances: quantities Origin computes in closed form (`fitlr`, `fitpoly`) are compared to
//! 1e-9 relative. Numbers read from the report of the iterative `nlfit` are compared to 1e-6
//! relative, because Origin stops that iteration at a tolerance of 1e-9 and so its own numbers are
//! only that exact. The tolerance is written at every comparison.

use sheaf_fit::linear::{LinearOptions, PolyOptions, fit_linear, fit_polynomial};
use sheaf_fit::{FitError, FitReport, Weighting};

#[path = "fixtures/linear_origin.rs"]
mod fixtures;
use fixtures::*;

/// Closed-form tolerance (fitlr and fitpoly numbers).
const EXACT: f64 = 1e-9;
/// Tolerance for numbers taken from the iterative nlfit report.
const ITER: f64 = 1e-6;

fn close(actual: f64, expected: f64, rel: f64, what: &str) {
    let tol = rel * expected.abs() + 1e-300;
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: got {actual:e}, Origin {expected:e}, allowed difference {tol:e}"
    );
}

fn close_abs(actual: f64, expected: f64, abs: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= abs,
        "{what}: got {actual:e}, Origin {expected:e}, allowed difference {abs:e}"
    );
}

fn line(x: &[f64], y: &[f64]) -> FitReport {
    fit_linear(x, y, &LinearOptions::default()).unwrap()
}

/// Compare the parameter rows of a report with the rows of an nlfit report sheet
/// (value, standard error, t, Prob>|t|).
fn check_nl_params(rep: &FitReport, nl: &[[f64; 5]], rel: f64) {
    for (i, row) in nl.iter().enumerate() {
        let p = &rep.params[i];
        close(p.value, row[0], rel, &format!("{} value", p.name));
        close(p.std_error, row[1], rel, &format!("{} error", p.name));
        close(p.t_value, row[2], rel, &format!("{} t", p.name));
        close(p.prob_t, row[3], rel, &format!("{} Prob>|t|", p.name));
    }
}

/// Compare statistics with an nlfit report sheet: points, degrees of freedom, reduced chi-square,
/// residual sum of squares, R-square, adjusted R-square.
fn check_nl_stats(rep: &FitReport, nl: &[f64; 6], rel: f64) {
    assert_eq!(rep.stats.n_points as f64, nl[0]);
    assert_eq!(rep.stats.dof as f64, nl[1]);
    close(rep.stats.reduced_chi_sqr, nl[2], rel, "reduced chi-square");
    close(rep.stats.rss, nl[3], rel, "residual sum of squares");
    close(rep.stats.r_squared, nl[4], rel, "R-square");
    close(rep.stats.adj_r_squared, nl[5], rel, "adjusted R-square");
}

/// The corrected total and error rows of an nlfit ANOVA must agree with ours, and the model row
/// of a straight line or polynomial is derived from them (the nlfit model row is uncorrected).
fn check_nl_anova(rep: &FitReport, nl: &[[f64; 5]; 4], rel: f64) {
    let a = rep.anova.as_ref().unwrap();
    assert_eq!(a.error_df as f64, nl[1][0]);
    close(a.error_ss, nl[1][1], rel, "error SS");
    close(a.error_ms, nl[1][2], rel, "error MS");
    assert_eq!(a.total_df as f64, nl[3][0]);
    close(a.total_ss, nl[3][1], rel, "corrected total SS");
    let model_ss = nl[3][1] - nl[1][1];
    let model_df = nl[3][0] - nl[1][0];
    assert_eq!(a.model_df as f64, model_df);
    close(a.model_ss, model_ss, rel, "model SS");
    close(a.f_value, model_ss / model_df / nl[1][2], rel, "F");
}

#[test]
fn noisy_line_matches_fitlr_and_fitpoly() {
    let rep = line(&BOOK1_X, &BOOK1_Y);
    let (i, s) = (&rep.params[0], &rep.params[1]);
    assert_eq!((i.name.as_str(), s.name.as_str()), ("Intercept", "Slope"));
    close(i.value, C1_A, EXACT, "intercept (fitlr)");
    close(s.value, C1_B, EXACT, "slope (fitlr)");
    close(i.std_error, C1_AERR, EXACT, "intercept error (fitlr)");
    close(s.std_error, C1_BERR, EXACT, "slope error (fitlr)");
    close(rep.stats.pearson_r.unwrap(), C1_R, EXACT, "r (fitlr)");
    assert_eq!(rep.stats.n_points as f64, C1_PTS);
    close(i.value, C1P_C1, EXACT, "intercept (fitpoly)");
    close(s.std_error, C1P_E2, EXACT, "slope error (fitpoly)");
    close(rep.stats.adj_r_squared, C1P_ADJRSQ, EXACT, "adjusted R-square (fitpoly)");
    close(rep.stats.r_squared, C1P_RSQCOD, EXACT, "R-square (fitpoly)");
    check_nl_params(&rep, &BOOK1_NL_PARAMS, ITER);
    check_nl_stats(&rep, &BOOK1_NL_STATS, ITER);
    check_nl_anova(&rep, &BOOK1_NL_ANOVA, ITER);
    close(rep.stats.rss, N1_SSR, ITER, "residual sum of squares (nltree)");
    close(rep.stats.reduced_chi_sqr, N1_CHISQR, ITER, "reduced chi-square (nltree)");
    assert_eq!(rep.stats.dof as f64, N1_DOF);
    assert!(rep.stats.iterations.is_none());
}

#[test]
fn confidence_limits_use_the_t_distribution() {
    let rep = line(&BOOK1_X, &BOOK1_Y);
    let p = &rep.params[1];
    // 95 % limit with 8 degrees of freedom: t = 2.306004135204166 (standard table value).
    let half = 2.306_004_135_204_166 * p.std_error;
    close(p.value - p.lcl, half, 1e-9, "slope lower limit");
    close(p.ucl - p.value, half, 1e-9, "slope upper limit");
    assert_eq!(rep.confidence_level, 0.95);
    let wide = fit_linear(&BOOK1_X, &BOOK1_Y, &LinearOptions { confidence_level: 0.99, ..Default::default() }).unwrap();
    assert!(wide.params[1].ucl - wide.params[1].lcl > p.ucl - p.lcl);
}

#[test]
fn line_through_the_origin_uses_the_uncorrected_total() {
    let opts = LinearOptions { fix_intercept: Some(0.0), ..Default::default() };
    let rep = fit_linear(&BOOK2_X, &BOOK2_Y, &opts).unwrap();
    assert!(rep.params[0].fixed);
    assert_eq!(rep.params[0].std_error, 0.0, "Origin stores 0 for the error of a fixed parameter");
    assert_eq!(rep.params[0].value, 0.0);
    close(rep.params[1].value, C2_B, EXACT, "slope (fitlr)");
    close(rep.params[1].std_error, C2_BERR, EXACT, "slope error (fitlr)");
    close(rep.stats.pearson_r.unwrap(), C2_R, EXACT, "r (fitlr)");
    close(rep.stats.r_squared, C2P_RSQCOD, EXACT, "R-square (fitpoly)");
    close(rep.stats.adj_r_squared, C2P_ADJRSQ, EXACT, "adjusted R-square (fitpoly)");
    assert_eq!(rep.stats.dof, 9);
    let a = rep.anova.unwrap();
    assert_eq!((a.model_df, a.error_df, a.total_df), (1, 9, 10));
    // The total sum of squares is the sum of y squared.
    let syy: f64 = BOOK2_Y.iter().map(|y| y * y).sum();
    close(a.total_ss, syy, 1e-12, "total SS about zero");
}

#[test]
fn fixed_intercept_other_than_zero_also_uses_the_uncorrected_total() {
    let opts = LinearOptions { fix_intercept: Some(1.5), ..Default::default() };
    let rep = fit_linear(&BOOK3_X, &BOOK3_Y, &opts).unwrap();
    close(rep.params[1].value, C3B_B, EXACT, "slope (fitlr)");
    close(rep.params[1].std_error, C3B_BERR, EXACT, "slope error (fitlr)");
    close(rep.stats.pearson_r.unwrap(), C3B_R, EXACT, "r (fitlr)");
    close(rep.stats.r_squared, C3P_RSQCOD, EXACT, "R-square (fitpoly)");
    close(rep.stats.adj_r_squared, C3P_ADJRSQ, EXACT, "adjusted R-square (fitpoly)");
    // The polynomial entry point with the same fixed intercept gives the same report.
    let poly =
        fit_polynomial(&BOOK3_X, &BOOK3_Y, 1, &PolyOptions { fix_intercept: Some(1.5), ..Default::default() }).unwrap();
    close(poly.params[1].value, C3P_C2, EXACT, "slope (fitpoly)");
    close(poly.params[1].std_error, C3P_E2, EXACT, "slope error (fitpoly)");
    assert_eq!(poly.stats, rep.stats);
}

#[test]
fn fixed_slope_uses_the_corrected_total() {
    let opts = LinearOptions { fix_slope: Some(0.5), ..Default::default() };
    let rep = fit_linear(&BOOK3_X, &BOOK3_Y, &opts).unwrap();
    assert!(rep.params[1].fixed);
    close(rep.params[0].value, C3_A, EXACT, "intercept (fitlr)");
    close(rep.params[0].std_error, C3_AERR, EXACT, "intercept error (fitlr)");
    close(rep.stats.pearson_r.unwrap(), C3_R, EXACT, "r (fitlr)");
    assert_eq!(rep.stats.dof, 9);
    let a = rep.anova.unwrap();
    assert_eq!((a.model_df, a.total_df), (0, 9));
    assert!(a.f_value.is_nan() && a.prob_f.is_nan());
}

#[test]
fn instrumental_weighting_matches_nlfit() {
    let opts = LinearOptions { weighting: Weighting::Instrumental(BOOK4_E.to_vec()), ..Default::default() };
    let rep = fit_linear(&BOOK4_X, &BOOK4_Y, &opts).unwrap();
    check_nl_params(&rep, &BOOK4_NL_PARAMS, ITER);
    check_nl_stats(&rep, &BOOK4_NL_STATS, ITER);
    check_nl_anova(&rep, &BOOK4_NL_ANOVA, ITER);
    close(rep.stats.pearson_r.unwrap(), N4_R, ITER, "r (nltree)");
}

#[test]
fn unscaled_errors_differ_by_the_square_root_of_the_reduced_chi_square() {
    let weighting = Weighting::Instrumental(BOOK4_E.to_vec());
    let scaled =
        fit_linear(&BOOK4_X, &BOOK4_Y, &LinearOptions { weighting: weighting.clone(), ..Default::default() }).unwrap();
    let raw = fit_linear(
        &BOOK4_X,
        &BOOK4_Y,
        &LinearOptions { weighting, scale_errors_by_reduced_chi_sqr: false, ..Default::default() },
    )
    .unwrap();
    let factor = scaled.stats.reduced_chi_sqr.sqrt();
    for k in 0..2 {
        close(scaled.params[k].std_error, raw.params[k].std_error * factor, 1e-12, "scaled versus unscaled error");
        close(scaled.params[k].value, raw.params[k].value, 1e-12, "value");
    }
}

#[test]
fn statistical_weighting_matches_nlfit() {
    let opts = LinearOptions { weighting: Weighting::Statistical, ..Default::default() };
    let rep = fit_linear(&BOOK5_X, &BOOK5_Y, &opts).unwrap();
    check_nl_params(&rep, &BOOK5_NL_PARAMS, ITER);
    check_nl_stats(&rep, &BOOK5_NL_STATS, ITER);
    check_nl_anova(&rep, &BOOK5_NL_ANOVA, ITER);
}

#[test]
fn direct_weights_equal_instrumental_weights_made_by_hand() {
    let w: Vec<f64> = BOOK4_E.iter().map(|e| 1.0 / (e * e)).collect();
    let direct =
        fit_linear(&BOOK4_X, &BOOK4_Y, &LinearOptions { weighting: Weighting::Direct(w), ..Default::default() })
            .unwrap();
    check_nl_params(&direct, &BOOK4_NL_PARAMS, ITER);
}

#[test]
fn missing_values_drop_the_point_and_keep_input_order() {
    assert!(BOOK6_Y[3].is_nan() && BOOK6_X[6].is_nan());
    let rep = line(&BOOK6_X, &BOOK6_Y);
    assert_eq!(rep.stats.n_points, 8);
    assert_eq!(rep.stats.dof, 6);
    close(rep.params[0].value, C6_A, EXACT, "intercept (fitlr)");
    close(rep.params[1].value, C6_B, EXACT, "slope (fitlr)");
    close(rep.params[0].std_error, C6_AERR, EXACT, "intercept error (fitlr)");
    close(rep.params[1].std_error, C6_BERR, EXACT, "slope error (fitlr)");
    close(rep.stats.pearson_r.unwrap(), C6_R, EXACT, "r (fitlr)");
    close(rep.stats.adj_r_squared, C6P_ADJRSQ, EXACT, "adjusted R-square (fitpoly)");
    check_nl_params(&rep, &BOOK6_NL_PARAMS, ITER);
    check_nl_stats(&rep, &BOOK6_NL_STATS, ITER);
    assert_eq!(rep.fitted.len(), 10);
    assert_eq!(rep.residuals.len(), 10);
    for k in 0..10 {
        if k == 3 || k == 6 {
            assert!(rep.fitted[k].is_nan() && rep.residuals[k].is_nan(), "row {k}");
        } else {
            let want = rep.params[0].value + rep.params[1].value * BOOK6_X[k];
            close(rep.fitted[k], want, 1e-12, "fitted value");
            close(rep.residuals[k], BOOK6_Y[k] - want, 1e-9, "residual");
        }
    }
}

#[test]
fn a_missing_weight_drops_the_point_too() {
    let mut e = BOOK4_E.to_vec();
    e[2] = f64::NAN;
    let rep =
        fit_linear(&BOOK4_X, &BOOK4_Y, &LinearOptions { weighting: Weighting::Instrumental(e), ..Default::default() })
            .unwrap();
    assert_eq!(rep.stats.n_points, 9);
    assert!(rep.fitted[2].is_nan());
}

#[test]
fn badly_scaled_x_stays_accurate() {
    // x from 100000 to 100010. Origin's own numbers are good to about 1e-8 here.
    let rep = line(&BOOK7_X, &BOOK7_Y);
    close(rep.params[1].value, C7_B, 1e-7, "slope (fitlr)");
    close(rep.params[1].std_error, C7_BERR, 1e-7, "slope error (fitlr)");
    close(rep.params[0].value, C7_A, 1e-7, "intercept (fitlr)");
    close(rep.params[0].std_error, C7_AERR, 1e-7, "intercept error (fitlr)");
    close(rep.stats.pearson_r.unwrap(), C7_R, 1e-9, "r (fitlr)");
    close(rep.stats.r_squared, C7P_RSQCOD, 1e-9, "R-square (fitpoly)");
    // The straight line through the points, evaluated in the stable basis: the residuals must
    // sum to zero to rounding error and the fit must be better than any perturbed line.
    let sum: f64 = rep.residuals.iter().sum();
    assert!(sum.abs() < 1e-10, "residuals sum to {sum:e}");
    let quad = fit_polynomial(&BOOK7_X, &BOOK7_Y, 2, &PolyOptions::default()).unwrap();
    // For the second-order fit Origin's own numbers are only good to about 1e-8 (its R-square
    // differs from the exact value in the ninth digit, its coefficients in the seventh), so they
    // are compared to 1e-7. The exact values below were computed from the same data with exact
    // rational arithmetic (Python fractions), not by Origin; ours agree with them to 1e-12.
    close(quad.stats.r_squared, C7Q_RSQCOD, 1e-7, "order 2 R-square (fitpoly)");
    close(quad.stats.adj_r_squared, C7Q_ADJRSQ, 1e-7, "order 2 adjusted R-square (fitpoly)");
    let exact_r2 = 0.900_104_445_882_378_7;
    close(quad.stats.r_squared, exact_r2, 1e-12, "order 2 R-square (exact)");
    close(quad.stats.adj_r_squared, 1.0 - (1.0 - exact_r2) * 10.0 / 8.0, 1e-12, "order 2 adjusted R-square (exact)");
    // The coefficients of x^k for x near 1e5 depend on cancellation; compare the fitted curve
    // instead of the coefficients, which is what matters for use.
    let exact = [-279_648.497_328_700_2, 5.591_564_394_816_763, -2.795_069_433_734_193_4e-5];
    for (k, x) in BOOK7_X.iter().enumerate() {
        let want = exact[0] + exact[1] * x + exact[2] * x * x;
        close_abs(quad.fitted[k], want, 1e-6, "order 2 fitted value");
        let origin = C7Q_C1 + C7Q_C2 * x + C7Q_C3 * x * x;
        close_abs(quad.fitted[k], origin, 1e-1, "order 2 fitted value (Origin coefficients)");
    }
    close(quad.params[1].value, exact[1], 1e-7, "order 2 B1");
}

#[test]
fn polynomials_match_fitpoly() {
    type Case<'a> = (usize, f64, f64, &'a [f64], &'a [f64]);
    let cases: [Case; 4] = [
        (2, C8P2_ADJRSQ, C8P2_RSQCOD, &[C8P2_C1, C8P2_C2, C8P2_C3], &[C8P2_E1, C8P2_E2, C8P2_E3]),
        (3, C8P3_ADJRSQ, C8P3_RSQCOD, &[C8P3_C1, C8P3_C2, C8P3_C3, C8P3_C4], &[C8P3_E1, C8P3_E2, C8P3_E3, C8P3_E4]),
        (
            4,
            C8P4_ADJRSQ,
            C8P4_RSQCOD,
            &[C8P4_C1, C8P4_C2, C8P4_C3, C8P4_C4, C8P4_C5],
            &[C8P4_E1, C8P4_E2, C8P4_E3, C8P4_E4, C8P4_E5],
        ),
        (
            5,
            C8P5_ADJRSQ,
            C8P5_RSQCOD,
            &[C8P5_C1, C8P5_C2, C8P5_C3, C8P5_C4, C8P5_C5, C8P5_C6],
            &[C8P5_E1, C8P5_E2, C8P5_E3, C8P5_E4, C8P5_E5, C8P5_E6],
        ),
    ];
    for (order, adj, cod, coef, err) in cases {
        let rep = fit_polynomial(&BOOK8_X, &BOOK8_Y, order, &PolyOptions::default()).unwrap();
        let tag = format!("order {order}");
        assert_eq!(rep.params.len(), order + 1);
        assert_eq!(rep.params[0].name, "Intercept");
        assert_eq!(rep.params[order].name, format!("B{order}"));
        for k in 0..=order {
            close(rep.params[k].value, coef[k], EXACT, &format!("{tag} coefficient {k}"));
            close(rep.params[k].std_error, err[k], EXACT, &format!("{tag} error {k}"));
        }
        close(rep.stats.adj_r_squared, adj, EXACT, &format!("{tag} adjusted R-square"));
        close(rep.stats.r_squared, cod, EXACT, &format!("{tag} R-square"));
        assert_eq!(rep.stats.n_points, 12);
        assert_eq!(rep.stats.dof, 12 - order - 1);
        assert!(rep.stats.pearson_r.is_none());
    }
}

#[test]
fn order_four_matches_the_nlfit_report() {
    let rep = fit_polynomial(&BOOK8_X, &BOOK8_Y, 4, &PolyOptions::default()).unwrap();
    check_nl_params(&rep, &BOOK8_NL_PARAMS, ITER);
    check_nl_stats(&rep, &BOOK8_NL_STATS, ITER);
    check_nl_anova(&rep, &BOOK8_NL_ANOVA, ITER);
}

#[test]
fn ninth_order_polynomial_agrees_with_fitpoly() {
    let rep = fit_polynomial(&BOOK10_X, &BOOK10_Y, 9, &PolyOptions::default()).unwrap();
    let coef = [C10P_C1, C10P_C2, C10P_C3, C10P_C4, C10P_C5, C10P_C6, C10P_C7, C10P_C8, C10P_C9, C10P_C10];
    let err = [C10P_E1, C10P_E2, C10P_E3, C10P_E4, C10P_E5, C10P_E6, C10P_E7, C10P_E8, C10P_E9, C10P_E10];
    // The design matrix of a ninth-order fit is ill conditioned (condition number about 1e6 on
    // this data), so Origin's coefficients and ours agree only to roughly 1e-6 of their size.
    for k in 0..10 {
        close_abs(rep.params[k].value, coef[k], 1e-6 * err[k].max(1e-3), &format!("coefficient {k}"));
        close(rep.params[k].std_error, err[k], 1e-6, &format!("error {k}"));
    }
    close(rep.stats.r_squared, C10P_RSQCOD, 1e-9, "R-square");
    close(rep.stats.adj_r_squared, C10P_ADJRSQ, 1e-9, "adjusted R-square");
}

#[test]
fn polynomial_with_fixed_intercept() {
    let rep = fit_polynomial(&BOOK11_X, &BOOK11_Y, 2, &PolyOptions { fix_intercept: Some(1.0), ..Default::default() })
        .unwrap();
    assert!(rep.params[0].fixed);
    close(rep.params[1].value, C11P_C2, EXACT, "B1");
    close(rep.params[1].std_error, C11P_E2, EXACT, "B1 error");
    close(rep.params[2].value, C11P_C3, EXACT, "B2");
    close(rep.params[2].std_error, C11P_E3, EXACT, "B2 error");
    close(rep.stats.r_squared, C11P_RSQCOD, EXACT, "R-square");
    close(rep.stats.adj_r_squared, C11P_ADJRSQ, EXACT, "adjusted R-square");
    assert_eq!(rep.stats.dof, 10);
}

#[test]
fn exact_line_gives_zero_errors_and_r_one() {
    let rep = line(&BOOK9_X, &BOOK9_Y);
    close(rep.params[0].value, C9_A, 1e-12, "intercept");
    close(rep.params[1].value, C9_B, 1e-12, "slope");
    close_abs(rep.params[0].std_error, C9_AERR, 1e-12, "intercept error");
    close_abs(rep.params[1].std_error, C9_BERR, 1e-12, "slope error");
    close(rep.stats.pearson_r.unwrap(), C9_R, 1e-12, "r");
    close(rep.stats.r_squared, N9_COD, 1e-12, "R-square");
    close_abs(rep.stats.rss, N9_SSR, 1e-20, "residual sum of squares");
    // Probabilities of a perfect fit are zero.
    close_abs(rep.params[1].prob_t, 0.0, 1e-12, "slope Prob>|t|");
}

#[test]
fn shape_of_the_report() {
    let rep = line(&BOOK1_X, &BOOK1_Y);
    assert_eq!(rep.model, "Linear");
    assert_eq!(rep.covariance.len(), 4);
    assert_eq!(rep.covariance[1], rep.covariance[2]);
    close(rep.covariance[0].sqrt(), rep.params[0].std_error, 1e-12, "covariance diagonal");
    assert!(rep.params.iter().all(|p| p.dependency.is_nan()));
    assert_eq!(rep.status, sheaf_fit::FitStatus::Exact);
    assert!(rep.warnings.is_empty());
    let a = rep.anova.unwrap();
    close(a.model_ss + a.error_ss, a.total_ss, 1e-12, "sums of squares add up");
    assert_eq!((a.model_df, a.error_df, a.total_df), (1, 8, 9));
    // For a straight line the F value is the square of the slope's t value.
    close(a.f_value, rep.params[1].t_value.powi(2), 1e-9, "F equals t squared");
    close(a.prob_f, rep.params[1].prob_t, 1e-9, "Prob>F equals Prob>|t|");
    for (k, (f, r)) in rep.fitted.iter().zip(&rep.residuals).enumerate() {
        close(f + r, BOOK1_Y[k], 1e-12, "fitted plus residual");
    }
}

#[test]
fn refusals_and_edge_cases() {
    let x = [1.0, 2.0, 3.0];
    assert!(matches!(
        fit_linear(&x, &[1.0, 2.0], &LinearOptions::default()),
        Err(FitError::LengthMismatch { x: 3, y: 2 })
    ));
    assert!(matches!(
        fit_linear(&[1.0], &[1.0], &LinearOptions::default()),
        Err(FitError::TooFewPoints { have: 1, need: 2 })
    ));
    // Every point but one is missing.
    assert!(matches!(
        fit_linear(&x, &[1.0, f64::NAN, f64::NAN], &LinearOptions::default()),
        Err(FitError::TooFewPoints { have: 1, .. })
    ));
    // All x equal: the slope is not determined.
    assert!(matches!(
        fit_linear(&[2.0, 2.0, 2.0], &[1.0, 2.0, 4.0], &LinearOptions::default()),
        Err(FitError::Failed(_))
    ));
    assert!(matches!(fit_polynomial(&x, &[1.0, 2.0, 3.0], 0, &PolyOptions::default()), Err(FitError::BadFunction(_))));
    // Order 3 needs four points.
    assert!(matches!(
        fit_polynomial(&x, &[1.0, 2.0, 3.0], 3, &PolyOptions::default()),
        Err(FitError::TooFewPoints { have: 3, need: 4 })
    ));
    // Both parameters fixed leaves nothing to fit.
    assert!(matches!(
        fit_linear(
            &x,
            &[1.0, 2.0, 3.0],
            &LinearOptions { fix_intercept: Some(0.0), fix_slope: Some(1.0), ..Default::default() }
        ),
        Err(FitError::BadFunction(_))
    ));
    // Statistical weighting with a y of zero: Origin's fit fails (status -205), so do we.
    assert!(matches!(
        fit_linear(
            &[1.0, 2.0, 3.0, 4.0, 5.0],
            &[1.0, 2.0, 0.0, 4.0, 5.0],
            &LinearOptions { weighting: Weighting::Statistical, ..Default::default() }
        ),
        Err(FitError::Failed(_))
    ));
    // A zero error cannot give a weight (Origin: status -204).
    assert!(matches!(
        fit_linear(
            &x,
            &[1.0, 2.0, 3.5],
            &LinearOptions { weighting: Weighting::Instrumental(vec![0.1, 0.0, 0.1]), ..Default::default() }
        ),
        Err(FitError::Failed(_))
    ));
    // As many points as parameters: the line is exact but the errors are undefined.
    let two = fit_linear(&[1.0, 3.0], &[2.0, 6.0], &LinearOptions::default()).unwrap();
    assert_eq!(two.stats.dof, 0);
    assert!(two.params[1].std_error.is_nan());
    assert!(!two.warnings.is_empty());
    close(two.params[1].value, 2.0, 1e-12, "slope through two points");
}

#[test]
fn statistical_weighting_uses_the_absolute_value_of_y() {
    // Origin's nlfit on this data (tools/origin-ref/fitlin-edge-cases.labtalk.txt, fourth
    // workbook) gave intercept -2.4e-9 and slope 0.30000000086, which is the weighted fit with
    // weights 1 / |y|. Origin's iteration stops at 1e-9, so the comparison is absolute 1e-7.
    let rep = fit_linear(
        &[1.0, 2.0, 3.0, 4.0, 5.0],
        &[1.0, 2.0, -0.5, 4.0, 5.0],
        &LinearOptions { weighting: Weighting::Statistical, ..Default::default() },
    )
    .unwrap();
    close_abs(rep.params[0].value, 0.0, 1e-7, "intercept");
    close_abs(rep.params[1].value, 0.3, 1e-7, "slope");
    close_abs(rep.params[1].std_error, 0.835_151_170_434_117_8, 1e-6, "slope error");
}
