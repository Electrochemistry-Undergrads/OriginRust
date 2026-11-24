//! Turnbull's estimate against an independent numpy/scipy computation, made by `tools/turnbull-ref.py` (T409; none
//! of these numbers come from Origin). The intervals must match exactly, the log-likelihood to 1e-9 and the
//! probabilities and survival to 1e-6 (the self-consistency iteration converges slowly along flat directions).

#[path = "fixtures/turnbull_reference.rs"]
mod fixtures;

use sheaf_calc::lifedata::Obs;
use sheaf_calc::turnbull::turnbull;

#[test]
fn turnbull_matches_numpy_and_scipy() {
    for c in fixtures::CASES {
        let obs: Vec<Obs> = c.start.iter().zip(c.end).map(|(a, b)| Obs::from_columns(*a, *b).unwrap()).collect();
        let t = turnbull(&obs).unwrap();
        assert!(t.converged, "{}", c.name);
        assert_eq!(t.lower, c.lower, "{}", c.name);
        assert_eq!(t.upper, c.upper, "{}", c.name);
        assert_eq!(t.lower_closed, c.lower_closed, "{}", c.name);
        assert!(
            (t.log_likelihood - c.log_likelihood).abs() < 1e-9,
            "{}: {} vs {}",
            c.name,
            t.log_likelihood,
            c.log_likelihood
        );
        for j in 0..t.probability.len() {
            assert!(
                (t.probability[j] - c.probability[j]).abs() < 1e-6,
                "{} p{j}: {:?} vs {:?}",
                c.name,
                t.probability,
                c.probability
            );
            assert!((t.survival[j] - c.survival[j]).abs() < 1e-6, "{} s{j}", c.name);
        }
    }
}
