//! Maximum likelihood with exact, right-, left- and interval-censored rows against scipy, made by
//! `tools/interval-censor-ref.py` (T384; none of these numbers come from Origin). The log-likelihood at scipy's
//! parameters agrees to 1e-10; Sheaf's maximum is no lower than scipy's (1e-9 relative) and its parameters agree to
//! 1e-5.

#[path = "fixtures/interval_censor_reference.rs"]
mod fixtures;

use sheaf_calc::lifedata::{LifeDist, Obs, fit_arbitrary, log_likelihood_arbitrary};

#[test]
fn arbitrary_censoring_matches_scipy() {
    for s in fixtures::SAMPLES {
        let obs: Vec<Obs> = s.rows.iter().map(|(a, b)| Obs::from_columns(*a, *b).unwrap()).collect();
        for (name, params, ll) in s.fits {
            let dist = match *name {
                "weibull" => LifeDist::Weibull,
                "lognormal" => LifeDist::Lognormal,
                "exponential" => LifeDist::Exponential,
                _ => LifeDist::Normal,
            };
            let what = format!("{} {name}", s.name);
            let at_ref = log_likelihood_arbitrary(dist, params, &obs);
            assert!((at_ref - ll).abs() <= 1e-10 * ll.abs(), "{what}: likelihood {at_ref} vs {ll}");
            let f = fit_arbitrary(&obs, dist).unwrap();
            assert!(f.log_likelihood >= ll - 1e-9 * ll.abs(), "{what}: maximum {} vs scipy {ll}", f.log_likelihood);
            for (a, b) in f.params.iter().zip(*params) {
                assert!((a - b).abs() <= 1e-5 * b.abs().max(1.0), "{what}: {:?} vs {params:?}", f.params);
            }
        }
    }
}
