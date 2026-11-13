//! Distribution fit, probability plot, noncentral t and F distributions, and power calculations against numbers
//! from scipy (scipy-checked; none of these numbers come from Origin).
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
//! - noncentral t and F: 1e-9 absolute plus 1e-9 relative;
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
fn noncentral_t_matches_scipy() {
    assert!(NCT.len() > 600);
    let mut compared = 0;
    for c in NCT {
        let what = format!("nct(t={}, df={}, ncp={})", c.t, c.df, c.ncp);
        if !c.cdf.is_nan() {
            near(&format!("{what} cdf"), nct_cdf(c.t, c.df, c.ncp), c.cdf, 1e-9, 1e-9);
            compared += 1;
        }
        if !c.sf.is_nan() {
            near(&format!("{what} sf"), nct_sf(c.t, c.df, c.ncp), c.sf, 1e-9, 1e-9);
            compared += 1;
        }
    }
    assert!(compared > 1000);
}

#[test]
fn noncentral_t_tails_add_up_where_scipy_gives_nothing() {
    for c in NCT.iter().filter(|c| c.cdf.is_nan()) {
        let (lower, upper) = (nct_cdf(c.t, c.df, c.ncp), nct_sf(c.t, c.df, c.ncp));
        assert!((lower + upper - 1.0).abs() < 1e-9, "t={} df={} ncp={}: {lower} + {upper}", c.t, c.df, c.ncp);
        assert!((0.0..=1.0).contains(&lower));
    }
}

#[test]
fn noncentral_f_matches_scipy() {
    assert!(NCF.len() > 250);
    let mut compared = 0;
    for c in NCF {
        let what = format!("ncf(x={}, df1={}, df2={}, ncp={})", c.x, c.df1, c.df2, c.ncp);
        if c.cdf.is_nan() {
            continue;
        }
        compared += 1;
        near(&format!("{what} cdf"), ncf_cdf(c.x, c.df1, c.df2, c.ncp), c.cdf, 1e-9, 1e-9);
        near(&format!("{what} sf"), ncf_sf(c.x, c.df1, c.df2, c.ncp), c.sf, 1e-9, 1e-9);
    }
    assert!(compared > 200);
}

fn design(code: u8, ratio: f64) -> TTestDesign {
    match code {
        0 => TTestDesign::OneSample,
        1 => TTestDesign::Paired,
        _ => TTestDesign::TwoSample { ratio },
    }
}

fn tail(code: u8) -> Tail {
    if code == 0 { Tail::Two } else { Tail::Upper }
}

#[test]
fn t_test_power_matches_scipy() {
    assert!(T_POWER.len() > 150);
    for c in T_POWER {
        let r = t_power(design(c.design, c.ratio), c.effect, c.n1, c.alpha, tail(c.tail)).unwrap();
        let what = format!("design {} d={} n={} alpha={} tail {}", c.design, c.effect, c.n1, c.alpha, c.tail);
        assert_eq!(r.n2, c.n2, "{what}");
        near(&format!("{what} df"), r.df, c.df, 1e-12, 0.0);
        near(&format!("{what} ncp"), r.ncp, c.ncp, 1e-12, 0.0);
        near(&format!("{what} critical"), r.critical, c.critical, 1e-9, 1e-12);
        near(&format!("{what} power"), r.power, c.power, 1e-9, 1e-9);
    }
}

#[test]
fn t_test_sample_sizes_match_scipy() {
    assert!(T_SIZE.len() > 60);
    for c in T_SIZE {
        let r = t_sample_size(design(c.design, 1.0), c.effect, c.target, c.alpha, tail(c.tail)).unwrap();
        let what = format!("design {} d={} power={} tail {}", c.design, c.effect, c.target, c.tail);
        assert_eq!(r.n1, c.n, "{what}");
        near(&format!("{what} power"), r.power, c.power, 1e-9, 1e-9);
        assert!(r.power >= c.target);
    }
}

#[test]
fn anova_power_matches_scipy() {
    assert!(ANOVA_POWER.len() > 100);
    for c in ANOVA_POWER {
        let r = anova_power(c.groups, c.effect, c.n, c.alpha).unwrap();
        let what = format!("k={} f={} n={} alpha={}", c.groups, c.effect, c.n, c.alpha);
        near(&format!("{what} df1"), r.df1, c.df1, 1e-12, 0.0);
        near(&format!("{what} df2"), r.df2, c.df2, 1e-12, 0.0);
        near(&format!("{what} ncp"), r.ncp, c.ncp, 1e-12, 0.0);
        near(&format!("{what} critical"), r.critical, c.critical, 1e-9, 1e-12);
        near(&format!("{what} power"), r.power, c.power, 1e-9, 1e-9);
    }
}

#[test]
fn anova_sample_sizes_match_scipy() {
    for c in ANOVA_SIZE {
        let r = anova_sample_size(c.groups, c.effect, c.target, c.alpha).unwrap();
        let what = format!("k={} f={} power={}", c.groups, c.effect, c.target);
        assert_eq!(r.n, c.n, "{what}");
        near(&format!("{what} power"), r.power, c.power, 1e-9, 1e-9);
    }
}

#[test]
fn anova_effect_size_of_group_means() {
    // Means 1, 2, 3 about their mean 2: sqrt((1 + 0 + 1) / 3) = 0.8165, over SD 2.
    let f = anova_effect_size(&[1.0, 2.0, 3.0], 2.0).unwrap();
    assert!((f - (2.0_f64 / 3.0).sqrt() / 2.0).abs() < 1e-15);
    assert_eq!(anova_effect_size(&[1.0, 1.0], 1.0), Err(PowerError::BadSpread));
    assert_eq!(anova_effect_size(&[1.0], 1.0), Err(PowerError::TooFewGroups));
}

#[test]
fn bad_input_is_refused() {
    assert!(matches!(t_power(TTestDesign::OneSample, 0.5, 10, 0.0, Tail::Two), Err(PowerError::BadLevel(_))));
    assert!(matches!(t_power(TTestDesign::OneSample, 0.0, 10, 0.05, Tail::Two), Err(PowerError::BadEffect)));
    assert!(matches!(t_power(TTestDesign::OneSample, 0.5, 1, 0.05, Tail::Two), Err(PowerError::TooSmall { .. })));
    assert!(matches!(
        t_power(TTestDesign::TwoSample { ratio: 0.0 }, 0.5, 10, 0.05, Tail::Two),
        Err(PowerError::BadRatio)
    ));
    assert!(matches!(t_sample_size(TTestDesign::OneSample, 0.5, 1.0, 0.05, Tail::Two), Err(PowerError::BadLevel(_))));
    assert_eq!(t_sample_size(TTestDesign::OneSample, 1e-6, 0.9, 0.05, Tail::Two), Err(PowerError::TooLarge));
    assert!(matches!(anova_power(1, 0.2, 10, 0.05), Err(PowerError::TooFewGroups)));
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
