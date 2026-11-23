//! Accelerated failure time regression against numpy and scipy, made by `tools/lifereg-ref.py` (T391; none of these
//! numbers come from Origin). Coefficients, scale and log-likelihood to 1e-7 relative; standard errors (from
//! finite-difference Hessians on both sides) to 1e-5.

#[path = "fixtures/lifereg_reference.rs"]
mod fixtures;

use sheaf_calc::lifedata::LifeDist;
use sheaf_calc::lifereg::life_regression;

fn close(a: f64, b: f64, tol: f64, what: &str) {
    if b.is_nan() {
        assert!(a.is_nan(), "{what}: {a} vs NaN");
    } else {
        assert!((a - b).abs() <= tol * b.abs().max(1.0), "{what}: {a} vs {b}");
    }
}

#[test]
fn life_regression_matches_scipy() {
    let x: Vec<Vec<f64>> = fixtures::X.iter().map(|r| r.to_vec()).collect();
    for (name, b, se, scale, scale_se, ll) in fixtures::FITS {
        let dist = match *name {
            "weibull" => LifeDist::Weibull,
            "lognormal" => LifeDist::Lognormal,
            _ => LifeDist::Exponential,
        };
        let f = life_regression(&x, fixtures::T, fixtures::FAILED, dist).unwrap();
        for i in 0..3 {
            close(f.coefficients[i], b[i], 1e-7, &format!("{name} coefficient {i}"));
            close(f.se[i], se[i], 1e-5, &format!("{name} se {i}"));
        }
        close(f.scale, *scale, 1e-7, &format!("{name} scale"));
        close(f.scale_se, *scale_se, 1e-5, &format!("{name} scale se"));
        close(f.log_likelihood, *ll, 1e-9, &format!("{name} log-likelihood"));
    }
}
