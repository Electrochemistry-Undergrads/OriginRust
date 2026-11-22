//! Right-censored life-data fits against scipy, made by `tools/lifedata-ref.py` (T366; none of these numbers come
//! from Origin). Tolerance 1e-8 relative for the parameters, log-likelihoods, medians and means; plotting positions
//! exact to 1e-15.

#[path = "fixtures/lifedata_reference.rs"]
mod fixtures;

use sheaf_calc::lifedata::{LifeDist, fit_right_censored, plotting_positions};

fn close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() <= 1e-8 * b.abs().max(1.0), "{what}: {a} vs {b}");
}

#[test]
fn censored_fits_match_scipy() {
    for s in fixtures::SAMPLES {
        for (dist, params, ll, median, mean) in s.fits {
            let d = match *dist {
                "weibull" => LifeDist::Weibull,
                "lognormal" => LifeDist::Lognormal,
                "exponential" => LifeDist::Exponential,
                _ => LifeDist::Normal,
            };
            let f = fit_right_censored(s.t, s.censored, d).unwrap();
            let what = format!("{} {dist}", s.name);
            assert_eq!(f.params.len(), params.len(), "{what}");
            for (a, b) in f.params.iter().zip(*params) {
                close(*a, *b, &format!("{what} parameter"));
            }
            close(f.log_likelihood, *ll, &format!("{what} log-likelihood"));
            close(f.median, *median, &format!("{what} median"));
            close(f.mean, *mean, &format!("{what} mean"));
        }
        let p = plotting_positions(s.t, s.censored);
        assert_eq!(p.len(), s.positions.len(), "{}", s.name);
        for (a, b) in p.iter().zip(s.positions) {
            assert!(a.0 == b.0 && (a.1 - b.1).abs() < 1e-15, "{}: {a:?} vs {b:?}", s.name);
        }
    }
}
