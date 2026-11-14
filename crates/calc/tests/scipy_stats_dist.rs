//! Distribution fit and probability plot against numbers from scipy (scipy-checked; none of these numbers come
//! from Origin).
//!
//! The inputs and the scipy numbers are in `fixtures/scipy_stats_dist.rs`, made by `tools/stats-dist-ref.py`.
//!
//! Tolerances:
//! - closed forms (normal, lognormal and exponential estimates, log-likelihood from the estimates, AIC, BIC,
//!   probability plot quantiles and line, the Anderson-Darling sum): 1e-9 relative;
//! - the gamma and Weibull shapes are the root of one equation. scipy's gamma fit solves it too and agrees to
//!   1e-9; scipy's Weibull fit is a general optimizer that stops 1e-5 to 2.5e-5 (relative) away from the maximum
//!   (its log-likelihood is lower by a hair, never higher than ours), so against it the Weibull parameters are
//!   compared with 1e-4. Against the same equation solved with `scipy.optimize.brentq` to machine precision (the
//!   `params` of the fixture) they agree to 1e-9, and everything computed from the parameters (log-likelihood,
//!   AIC, BIC, D, A2) is compared with the numbers scipy gives for those parameters;
//! - the Kolmogorov-Smirnov p-value is compared to 1e-7 (the matrix method of Marsaglia, Tsang and Wang
//!   against scipy's exact Simard-L'Ecuyer algorithm).

#[path = "fixtures/scipy_stats_dist.rs"]
mod fixtures;

use fixtures::*;
use sheaf_calc::*;

fn family(code: usize) -> Family {
    [Family::Normal, Family::Lognormal, Family::Exponential, Family::Gamma, Family::Weibull][code]
}

fn near(what: &str, got: f64, want: f64, rel: f64, abs: f64) {
    assert!((got - want).abs() <= rel * want.abs() + abs, "{what}: got {got:?}, scipy gave {want:?}");
}

#[test]
fn fits_match_scipy() {
    assert!(FITS.len() >= 25);
    for c in FITS {
        let fam = family(c.family);
        let what = |s: &str| format!("{} {} {s}", c.name, fam.name());
        let r = dist_fit(fam, c.data).unwrap_or_else(|e| panic!("{}: {e}", what("fit")));
        assert_eq!(r.n, c.data.len());
        assert_eq!(r.params.len(), c.params.len());
        let scipy_rel = if fam == Family::Weibull { 1e-4 } else { 1e-9 };
        for ((got, want), scipy) in r.params.iter().zip(c.params).zip(c.params_scipy) {
            near(&what("parameter"), *got, *want, 1e-9, 0.0);
            near(&what("parameter (scipy fit)"), *got, *scipy, scipy_rel, 0.0);
        }
        near(&what("log-likelihood"), r.log_likelihood, c.log_likelihood, 1e-9, 1e-9);
        near(&what("AIC"), r.aic, c.aic, 1e-9, 1e-9);
        near(&what("BIC"), r.bic, c.bic, 1e-9, 1e-9);
        near(&what("KS D"), r.ks_d, c.ks_d, 1e-9, 1e-12);
        near(&what("KS p"), r.ks_p, c.ks_p, 1e-7, 1e-9);
        near(&what("A2"), r.ad, c.ad, 1e-9, 1e-9);
        match fam {
            Family::Normal | Family::Lognormal => assert!((0.0..=1.0).contains(&r.ad_p), "{}", what("AD p")),
            _ => assert!(r.ad_p.is_nan()),
        }
    }
}

#[test]
fn kolmogorov_p_above_140_matches_scipy() {
    assert!(KS_P.len() >= 60);
    for c in KS_P {
        let got = 1.0 - kolmogorov_cdf(c.n, c.d);
        if c.n <= KS_EXACT_MAX_N {
            // exact (or below 1e-7, where the series is used). scipy itself uses the Pelz-Good approximation
            // in part of this range (about five digits), hence 1e-5.
            near(&format!("KS p (n={}, d={})", c.n, c.d), got, c.p, 1e-5, 1e-5);
        } else {
            // the asymptotic series: off by 0.0034 at most just above 1000, 0.0011 at 10000
            assert!((got - c.p).abs() < 0.0025, "KS p (n={}, d={}): got {got}, scipy gave {}", c.n, c.d, c.p);
        }
    }
    // the case of the review: n = 141, D = 0.05 (the old series gave 0.864 against scipy's 0.855)
    near("n=141, d=0.05", 1.0 - kolmogorov_cdf(141, 0.05), 0.854845693009663, 1e-5, 1e-5);
    near("n=200, d=0.05", 1.0 - kolmogorov_cdf(200, 0.05), 0.6802625495871704, 1e-5, 1e-5);
}

#[test]
fn the_estimates_solve_the_likelihood_equations() {
    // The maximum: the score of both parameters is zero, and no neighbouring shape has a higher likelihood.
    for c in FITS.iter().filter(|c| matches!(family(c.family), Family::Gamma | Family::Weibull)) {
        let fam = family(c.family);
        let r = dist_fit(fam, c.data).unwrap();
        for factor in [0.999_999, 1.000_001] {
            let p = [r.params[0] * factor, r.params[1]];
            assert!(dist_log_likelihood(fam, &p, c.data) <= r.log_likelihood + 1e-12, "{} {}", c.name, fam.name());
        }
        // The scipy estimate is never better than ours.
        assert!(dist_log_likelihood(fam, c.params_scipy, c.data) <= r.log_likelihood + 1e-9);
    }
}

#[test]
fn probability_plots_match_scipy() {
    assert!(PROBPLOTS.len() >= 6);
    for c in PROBPLOTS {
        let fam = family(c.family);
        let shape = if c.shape.is_nan() { None } else { Some(c.shape) };
        let r = probplot(fam, c.data, shape).unwrap();
        assert_eq!(r.theoretical.len(), c.theoretical.len());
        for (i, ((a, b), (o, w))) in
            r.theoretical.iter().zip(c.theoretical).zip(r.ordered.iter().zip(c.ordered)).enumerate()
        {
            near(&format!("{} quantile {i}", c.name), *a, *b, 1e-9, 1e-12);
            assert_eq!(o, w, "{} ordered {i}", c.name);
        }
        near(&format!("{} slope", c.name), r.slope, c.slope, 1e-9, 1e-12);
        near(&format!("{} intercept", c.name), r.intercept, c.intercept, 1e-9, 1e-9);
        near(&format!("{} r", c.name), r.r, c.r, 1e-9, 1e-12);
    }
}

#[test]
fn a_probability_plot_without_a_shape_uses_the_fitted_one() {
    let c = PROBPLOTS.iter().find(|c| c.name == "gamma25").unwrap();
    let r = probplot(Family::Gamma, c.data, None).unwrap();
    let fitted = dist_fit(Family::Gamma, c.data).unwrap().params[0];
    assert_eq!(r.shape, Some(fitted));
    // scipy's shape differs from ours by 1e-8 at most, so the quantiles agree to 1e-6.
    for (a, b) in r.theoretical.iter().zip(c.theoretical) {
        near("gamma quantile", *a, *b, 1e-6, 0.0);
    }
}

#[test]
fn fits_refuse_data_the_family_cannot_take() {
    assert!(matches!(dist_fit(Family::Normal, &[1.0]), Err(FitError::TooFew { .. })));
    assert!(matches!(dist_fit(Family::Normal, &[2.0, 2.0, 2.0]), Err(FitError::NoSpread)));
    assert!(matches!(dist_fit(Family::Lognormal, &[1.0, -2.0, 3.0]), Err(FitError::NotPositive(_))));
    assert!(matches!(dist_fit(Family::Gamma, &[1.0, 0.0, 3.0]), Err(FitError::NotPositive(_))));
    assert!(dist_fit(Family::Exponential, &[0.0, 1.0, 3.0]).is_ok());
    assert!(matches!(dist_fit(Family::Weibull, &[4.0, 4.0]), Err(FitError::NoSpread)));
    // Missing values are ignored.
    let r = dist_fit(Family::Normal, &[1.0, f64::NAN, 3.0, 5.0]).unwrap();
    assert_eq!(r.n, 3);
}

/// Runs `fit` on a thread and fails (instead of hanging the test run) if it does not return in 20 seconds.
fn fit_with_timeout(fam: Family, data: Vec<f64>) -> Result<DistFit, FitError> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(dist_fit(fam, &data));
    });
    rx.recv_timeout(std::time::Duration::from_secs(20)).expect("the fit did not return")
}

#[test]
fn numbers_that_differ_by_rounding_only_have_no_spread() {
    // The gamma statistic ln(mean) - mean(ln x) is zero or slightly negative here; the shape equation used to
    // loop for ever (first two) or the fit panicked in statrs (third).
    let cases: [Vec<f64>; 4] = [
        vec![1.0, 1.0000000000000002, 1.0],
        vec![0.1 + 0.2, 0.3, 0.3],
        vec![1.0, 1.0 + 1e-12, 1.0, 1.0 + 2e-12],
        vec![5.0, 5.0 + 1e-9, 5.0, 5.0 + 3e-9, 5.0],
    ];
    for data in cases {
        assert_eq!(fit_with_timeout(Family::Gamma, data.clone()), Err(FitError::NoSpread), "{data:?}");
        // the other families neither hang nor panic
        for fam in Family::ALL {
            let _ = fit_with_timeout(fam, data.clone());
        }
    }
    assert!(dist_estimate(Family::Gamma, &[1.0, 1.0000000000000002, 1.0]).is_err());
}

#[test]
fn a_gamma_with_a_modest_spread_is_still_fitted() {
    // A coefficient of variation of 1e-3: shape about 1e6, well inside the accepted range.
    let data: Vec<f64> = (0..20).map(|i| 100.0 * (1.0 + 1e-3 * ((i % 5) as f64 - 2.0))).collect();
    let r = fit_with_timeout(Family::Gamma, data).unwrap();
    assert!(r.params[0] > 1e5 && r.params[0] < 1e8, "{:?}", r.params);
    assert!(r.log_likelihood.is_finite() && r.ks_d.is_finite());
}

#[test]
fn extreme_magnitudes_give_a_result_or_an_error_never_a_panic() {
    let cases: [Vec<f64>; 6] = [
        vec![0.0, 1e-200],
        vec![1e-200, 2e-200, 3e-200],
        vec![1e300, 1e-300, 5.0],
        vec![1e200, 2e200, 3e200],
        vec![1e308, 1.7e308, 1.2e308],
        vec![f64::MIN_POSITIVE, 2.0 * f64::MIN_POSITIVE, 5e-324],
    ];
    for data in cases {
        for fam in Family::ALL {
            // an error is fine (no spread, degenerate); a fit must be finite
            if let Ok(r) = fit_with_timeout(fam, data.clone()) {
                assert!(r.params.iter().all(|p| p.is_finite()), "{} {data:?}: {:?}", fam.name(), r.params);
                assert!(r.log_likelihood.is_finite() && r.ks_d.is_finite() && r.ad.is_finite());
            }
        }
    }
    // The ones with a sensible answer are fitted: the normal SD no longer underflows or overflows.
    let r = dist_fit(Family::Normal, &[1e-200, 2e-200, 3e-200]).unwrap();
    assert!((r.params[1] / 1e-200 - (2.0_f64 / 3.0).sqrt()).abs() < 1e-12, "{:?}", r.params);
    let r = dist_fit(Family::Normal, &[1e200, 2e200, 3e200]).unwrap();
    assert!((r.params[1] / 1e200 - (2.0_f64 / 3.0).sqrt()).abs() < 1e-12, "{:?}", r.params);
    // cdf with a bad SD is NaN, not a panic
    assert!(dist_cdf(Family::Normal, &[0.0, 0.0], 1.0).is_nan());
    assert!(dist_cdf(Family::Gamma, &[2.0, 0.0], 1.0).is_nan());
}

#[test]
fn the_anderson_darling_p_is_the_p_of_the_normality_test() {
    // The fit's A2 uses the maximum likelihood SD (divisor n); the formula for the p-value is for the sample SD
    // (divisor n - 1), so the p-value is taken from A2 with that SD and equals the Normality Test's.
    for c in FITS.iter().filter(|c| matches!(family(c.family), Family::Normal | Family::Lognormal) && c.data.len() >= 8)
    {
        let fam = family(c.family);
        let r = dist_fit(fam, c.data).unwrap();
        let x: Vec<f64> =
            if fam == Family::Lognormal { c.data.iter().map(|v| v.ln()).collect() } else { c.data.to_vec() };
        let t = anderson_darling(&x).unwrap();
        near(&format!("{} {} AD p", c.name, fam.name()), r.ad_p, t.p, 1e-9, 1e-12);
        // while the reported A2 is the one against the maximum likelihood fit (scipy-checked above)
        assert!((r.ad - t.statistic).abs() > 1e-6 * t.statistic, "{}: A2 should be the ML one", c.name);
    }
}
